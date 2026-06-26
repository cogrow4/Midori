# Midori

A compiled programming language with Pythonic syntax and C-style braces. Compiles through C to native code via Clang/GCC.

```midori
fn greet(name: Str) -> Str {
    "Hello, {name}!"
}

fn main() {
    println(greet("Midori"))
}
```

## Quick Start

```sh
# Install
git clone https://github.com/cogrow4/Midori.git
cd Midori
./install.sh

# Run a file
midori hello.mi

# Build a binary
midori build hello.mi

# Create a new project
midori new my-project
```

## Features

| Status | Feature |
|--------|---------|
| ✅ | Functions, variables (`let`/`var`/`mut`), type inference |
| ✅ | `if`/`elif`/`else`, `while`, `for` (range & iter), `loop`/`break`/`continue` |
| ✅ | `match` expressions with pattern matching |
| ✅ | Structs, enums, traits with `impl` blocks |
| ✅ | Method dispatch — `obj.method(args)` |
| ✅ | String interpolation — `"Hello, {name}!"` |
| ✅ | Cli toolchain — `midori`, `midori run`, `midori build`, `midori dev` (watch mode) |
| ✅ | String ops, file I/O, math builtins, `os_args`, string builder |
| ✅ | Recursion, first-class functions, block-to-expr conversion |
| ✅ | Source-aware error messages with caret highlighting |
| ✅ | C backend — compiles through C, links with `cc` + `-lm` |
| 🚧 | Standard library |
| 🚧 | Error handling (try/catch, Result, Option) |
| 🚧 | Module system |
| 🚧 | Generics |
| 🚧 | Self-hosting compiler |

## Commands

```
midori <file.mi> [args...]    Compile and run with args
midori run <file.mi> [args...] Compile and run with args
midori build <file.mi>         Compile to binary
midori dev <file.mi>           Watch for changes and re-run
midori new <project>           Create new project scaffold
midori help                    Show help
midori version                 Show version
```

## Project Structure

```
compiler/        Rust compiler source
  src/           Lexer, parser, AST, type checker, codegen
  src/runtime/   Runtime library (C, embedded at compile time)
  tests/         Test suite
website/         Project website (localhost:8080)
examples/        Example .mi programs
install.sh       Build & install script
```

## Status

Pre-release. The core language works — all examples compile and run. Method dispatch, string interpolation, match expressions, and range-based for loops are functional. Known gaps in stdlib, error handling, and module system are tracked for production readiness.

## License

MIT
