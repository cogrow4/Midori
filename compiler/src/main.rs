use std::process::Command;
use std::path::Path;
use std::fs;
use std::time::SystemTime;

mod lexer;
mod ast;
mod parser;
mod types;
mod codegen;

const VERSION: &str = "0.1.0";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "build" => {
            if args.len() < 3 {
                eprintln!("error: expected file argument");
                std::process::exit(1);
            }
            compile(&args[2], false, &[]);
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("error: expected file argument");
                std::process::exit(1);
            }
            let binary_args: Vec<String> = args[3..].to_vec();
            compile(&args[2], true, &binary_args);
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("error: expected project name");
                std::process::exit(1);
            }
            new_project(&args[2]);
        }
        "dev" => {
            if args.len() < 3 {
                eprintln!("error: expected file argument");
                std::process::exit(1);
            }
            dev_watch(&args[2]);
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        "version" | "--version" | "-v" => {
            print_version();
        }
        s if s.ends_with(".mi") => {
            // Bare "midori example.mi arg1 arg2" passes args to binary
            let binary_args: Vec<String> = args[2..].to_vec();
            compile(&args[1], true, &binary_args);
        }
        _ => {
            eprintln!("error: unknown command '{}'. Use 'midori help'", args[1]);
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("Midori Programming Language v{}", VERSION);
    println!("A beautiful, developer-friendly compiled language.");
    println!();
    println!("Usage:");
    println!("  midori <file.mi> [args...]      Compile and run with args");
    println!("  midori run <file.mi> [args...]   Compile and run with args");
    println!("  midori build <file.mi>           Compile to binary");
    println!("  midori dev <file.mi>             Watch for changes and re-run");
    println!("  midori new <project>             Create new project");
    println!("  midori help                      Show this help");
    println!("  midori version                   Show version");
    println!();
    println!("Examples:");
    println!("  midori hello.mi                  Run a script");
    println!("  midori build hello.mi            Build a binary");
    println!("  midori fileinfo.mi test.txt      Run with arguments");
}

fn print_version() {
    println!("Midori v{}", VERSION);
}

fn compile(input_file: &str, run: bool, binary_args: &[String]) {
    let source = match fs::read_to_string(input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read '{}': {}", input_file, e);
            std::process::exit(1);
        }
    };

    let lex = lexer::Lexer::new(&source);
    let mut parser = match parser::Parser::new(lex) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    // Type check
    let mut checker = types::TypeChecker::new();
    if let Err(e) = checker.check_program(&program) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }

    // Generate C code
    let mut codegen = codegen::Codegen::new();
    let c_code = codegen.generate(&program);

    // Write C file
    let input_path = Path::new(input_file);
    let stem = input_path.file_stem().unwrap().to_str().unwrap();
    let c_file = format!("{}.c", stem);

    if let Err(e) = fs::write(&c_file, &c_code) {
        eprintln!("error: could not write C file: {}", e);
        std::process::exit(1);
    }

    // Write runtime files alongside generated C (embedded in binary)
    let runtime_h_src = include_str!("runtime/runtime.h");
    let runtime_c_src = include_str!("runtime/runtime.c");
    let _ = fs::write("midori_runtime.h", runtime_h_src);
    let _ = fs::write("midori_runtime.c", runtime_c_src);

    let output = if run { String::new() } else { stem.to_string() };

    let mut cmd = Command::new("cc");
    cmd.arg("-o").arg(if output.is_empty() { stem } else { &output });
    cmd.arg(&c_file);
    cmd.arg("midori_runtime.c");
    cmd.arg("-I.");
    cmd.arg("-lm");

    // ponytail: using cc (gcc/clang) for backend; replace with self-hosted later

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not execute cc: {}", e);
            std::process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("error: C compilation failed");
        std::process::exit(1);
    }

    // Clean up intermediate files
    let _ = fs::remove_file("midori_runtime.h");
    let _ = fs::remove_file("midori_runtime.c");
    if !std::env::var("MIDORI_KEEP_C").is_ok() {
        let _ = fs::remove_file(&c_file);
    }
    eprintln!("✓ Compiled: {}", stem);

    if run {
        let mut run_cmd = Command::new(format!("./{}", stem));
        run_cmd.args(binary_args);
        let run_status = match run_cmd.status() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: could not run binary: {}", e);
                std::process::exit(1);
            }
        };

        if !run_status.success() {
            std::process::exit(run_status.code().unwrap_or(1));
        }

        // Clean up binary
        let _ = fs::remove_file(stem);
    }
}

fn dev_watch(input_file: &str) {
    let path = Path::new(input_file);
    if !path.exists() {
        eprintln!("error: file '{}' not found", input_file);
        std::process::exit(1);
    }

    eprintln!("👀 Watching {} for changes...", input_file);
    let mut last_mtime = fs::metadata(input_file)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::now());

    compile(input_file, true, &[]);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Ok(meta) = fs::metadata(input_file) {
            if let Ok(mtime) = meta.modified() {
                if mtime != last_mtime {
                    last_mtime = mtime;
                    eprintln!("\n🔄 Change detected, recompiling...");
                    compile(input_file, true, &[]);
                    eprintln!("👀 Watching {} for changes...", input_file);
                }
            }
        }
    }
}

fn new_project(name: &str) {
    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("error: directory '{}' already exists", name);
        std::process::exit(1);
    }

    match fs::create_dir_all(dir) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("error: could not create project: {}", e);
            std::process::exit(1);
        }
    }

    // Create main.mi
    let main_content = "fn main() {\n    println(\"Hello, Midori!\")\n}\n";
    fs::write(dir.join("main.mi"), main_content).unwrap();

    println!("✓ Created new Midori project '{}'", name);
    println!("  cd {} && midori run main.mi", name);
}
