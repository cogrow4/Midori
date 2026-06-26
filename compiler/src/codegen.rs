use crate::ast::*;
use std::collections::HashMap;

pub struct Codegen {
    output: String,
    indent: usize,
    var_count: usize,
    current_fn: Option<String>,
    struct_types: HashMap<String, Vec<(String, String)>>,
    enum_types: HashMap<String, Vec<(String, Vec<(String, String)>)>>,
    fn_types: HashMap<String, String>,
    var_types: HashMap<String, String>, // variable name -> C type
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            output: String::new(),
            indent: 0,
            var_count: 0,
            current_fn: None,
            struct_types: HashMap::new(),
            enum_types: HashMap::new(),
            fn_types: HashMap::new(),
            var_types: HashMap::new(),
        }
    }
    pub fn generate(&mut self, program: &Program) -> String {
        self.collect_types(program);

        // Collect function return types
        for stmt in &program.stmts {
            if let Stmt::Fn { name, return_type, .. } = stmt {
                let rt = self.mi_ctype(return_type.as_ref().map(|t| self.te_string(t)).as_deref());
                self.fn_types.insert(name.clone(), rt);
            }
        }


        let mut out = String::new();
        out.push_str("#include \"midori_runtime.h\"\n\n");

        // Emit struct definitions in dependency order (topological sort)
        let struct_order = self.sorted_structs();
        for name in &struct_order {
            if let Some(fields) = self.struct_types.get(name) {
                out.push_str(&format!("typedef struct {{\n"));
                for (fname, ftype) in fields {
                    out.push_str(&format!("    {} {};\n", ftype, fname));
                }
                out.push_str(&format!("}} {};\n\n", name));
            }
        }

        // Emit enum definitions
        for (name, variants) in &self.enum_types {
            out.push_str(&format!("typedef enum {{\n"));
            for (i, (vname, _)) in variants.iter().enumerate() {
                out.push_str(&format!("    {}_TAG_{},\n", name, vname.to_uppercase()));
            }
            out.push_str(&format!("}} {}_Tag;\n\n", name));
            out.push_str(&format!("typedef struct {{\n"));
            out.push_str(&format!("    {}_Tag tag;\n", name));
            out.push_str(&format!("    union {{\n"));
            for (vname, fields) in variants {
                if fields.is_empty() {
                    out.push_str(&format!("        int __empty_{}_;\n", vname));
                } else {
                    for (fname, ftype) in fields {
                        out.push_str(&format!("        {} {}_{};\n", ftype, vname, fname));
                    }
                }
            }
            out.push_str(&format!("    }} data;\n"));
            out.push_str(&format!("}} {};\n\n", name));
        }

        // Forward declarations (skip main — it gets special handling)
        for stmt in &program.stmts {
            if let Stmt::Fn { name, params, return_type, .. } = stmt {
                if name == "main" { continue; }
                let rt = self.mi_ctype(return_type.as_ref().map(|t| self.te_string(t)).as_deref());
                let ps: Vec<String> = params.iter()
                    .map(|p| self.mi_ctype(p.type_expr.as_ref().map(|t| self.te_string(t)).as_deref()))
                    .collect();
                if ps.is_empty() {
                    out.push_str(&format!("{} {}();\n", rt, name));
                } else {
                    out.push_str(&format!("{} {}({});\n", rt, name, ps.join(", ")));
                }
            }
        }
        out.push_str("\n");

        // Generate code
        self.output = out;
        for stmt in &program.stmts {
            self.gen_stmt(stmt);
        }

        self.output.clone()
    }

    /// Topological sort of structs by dependency.
    /// Struct A depends on B if any field type of A is a known struct name.
    /// Result: dependencies before dependents (Kahn's algorithm).
    fn sorted_structs(&self) -> Vec<String> {
        // deps[A] = structs A depends on (must be defined first)
        let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
        for name in self.struct_types.keys() {
            deps.entry(name.as_str()).or_default();
            dependents.entry(name.as_str()).or_default();
        }
        for (name, fields) in &self.struct_types {
            for (_, ft) in fields {
                let ft = ft.trim();
                if self.struct_types.contains_key(ft) && ft != name {
                    deps.get_mut(name.as_str()).unwrap().push(ft);
                    dependents.get_mut(ft).unwrap().push(name.as_str());
                }
            }
        }
        // Kahn's: nodes with in_degree 0 have no pending deps, emit first
        let mut in_degree: HashMap<&str, usize> = deps.iter()
            .map(|(n, d)| (*n, d.len()))
            .collect();
        let mut queue: Vec<&str> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(n, _)| *n)
            .collect();
        let mut sorted = Vec::new();
        while let Some(n) = queue.pop() {
            sorted.push(n.to_string());
            for m in &dependents[n] {
                let deg = in_degree.get_mut(m).unwrap();
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    queue.push(m);
                }
            }
        }
        if sorted.len() != self.struct_types.len() {
            // ponytail: cycle detected, fallback to arbitrary order
            return self.struct_types.keys().cloned().collect();
        }
        sorted
    }

    fn collect_types(&mut self, program: &Program) {
        for stmt in &program.stmts {
            if let Stmt::Type { name, variants, .. } = stmt {
                if variants.len() == 1 && variants[0].name == *name {
                    let fields: Vec<(String, String)> = variants[0].fields.iter()
                        .map(|f| (f.name.clone(), self.mi_ctype(Some(&self.te_string(&f.type_expr)))))
                        .collect();
                    self.struct_types.insert(name.clone(), fields);
                } else {
                    let vars: Vec<(String, Vec<(String, String)>)> = variants.iter()
                        .map(|v| {
                            let fields: Vec<(String, String)> = v.fields.iter()
                                .map(|f| (f.name.clone(), self.mi_ctype(Some(&self.te_string(&f.type_expr)))))
                                .collect();
                            (v.name.clone(), fields)
                        }).collect();
                    self.enum_types.insert(name.clone(), vars);
                }
            }
        }
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn emit_line(&mut self, s: &str) {
        for _ in 0..self.indent { self.output.push_str("    "); }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn te_string(&self, te: &TypeExpr) -> String {
        match te {
            TypeExpr::Named(n) => n.clone(),
            TypeExpr::Generic(n, args) => {
                if args.is_empty() { return n.clone(); }
                format!("{}_T_{}", n, args.iter().map(|a| self.te_string(a)).collect::<Vec<_>>().join("_"))
            }
            TypeExpr::Fn(_, _) => "Fn".to_string(),
            TypeExpr::Tuple(ts) => format!("Tuple{}", ts.len()),
            TypeExpr::Infer => "Unknown".to_string(),
        }
    }

    fn mi_ctype(&self, midori_type: Option<&str>) -> String {
        match midori_type {
            None | Some("Unknown") => "mi_int".to_string(),
            Some("Int") => "mi_int".to_string(),
            Some("Float") => "mi_float".to_string(),
            Some("Bool") => "mi_bool".to_string(),
            Some("Str") => "mi_str".to_string(),
            Some("Char") => "mi_char".to_string(),
            Some("Nil") => "mi_nil_t".to_string(),
            Some(s) if self.struct_types.contains_key(s) => s.to_string(),
            Some(s) if self.enum_types.contains_key(s) => s.to_string(),
            Some(s) => {
                if s.starts_with('[') || s.starts_with("Array") {
                    "mi_array".to_string()
                } else {
                    s.to_string()
                }
            }
        }
    }

    fn c_literal(&self, midori_type: Option<&str>) -> String {
        match midori_type {
            Some("Int") | None => "0",
            Some("Float") => "0.0",
            Some("Bool") => "0",
            Some("Str") => "mi_str_empty()",
            Some("Char") => "'\\0'",
            Some("Nil") => "",
            _ => "{0}",
        }.to_string()
    }

    fn infer_ctype(&self, expr: &Expr) -> String {
        match expr {
            Expr::Int(_) => "mi_int".to_string(),
            Expr::Float(_) => "mi_float".to_string(),
            Expr::Bool(_) => "mi_bool".to_string(),
            Expr::Str(_, _) => "mi_str".to_string(),
            Expr::Char(_) => "mi_char".to_string(),
            Expr::Nil => "mi_nil_t".to_string(),
            Expr::Ident(name) => {
                self.var_types.get(name).cloned().unwrap_or("mi_int".to_string())
            }
            Expr::Call(callee, _) => {
                if let Expr::Ident(name) = callee.as_ref() {
                    match name.as_str() {
                        "range" | "len" | "len_str" => "mi_int".to_string(),
                        "str" | "str_bool" | "str_char" | "read_file" => "mi_str".to_string(),
                        "str_float" => "mi_str".to_string(),
                        "print" | "println" => "mi_nil_t".to_string(),
                        "os_args" => "mi_array".to_string(),
                        _ => self.fn_types.get(name).cloned().unwrap_or("mi_int".to_string())
                    }
                } else {
                    "mi_int".to_string()
                }
            }
            Expr::Array(_) => "mi_array".to_string(),
            Expr::StructLit(name, _) => name.clone(),
            _ => "mi_int".to_string()
        }
    }

    fn resolve_method_call(&self, receiver_ctype: &str, method: &str, receiver_c: &str, args: &[String]) -> String {
        let type_name = match receiver_ctype {
            "mi_str" => Some("Str"),
            "mi_array" => Some("Array"),
            s if self.struct_types.contains_key(s) => Some(s),
            _ => None,
        };
        match (type_name, method) {
            (Some("Str"), "len") => format!("len_str({})", receiver_c),
            (Some("Str"), "is_empty") => format!("(len_str({}) == 0)", receiver_c),
            (Some("Array"), "len") => format!("len({})", receiver_c),
            (Some("Array"), "is_empty") => format!("(len({}) == 0)", receiver_c),
            (Some(tn), _) => {
                let self_arg = if tn == "mi_array" {
                    format!("&{}", receiver_c)
                } else {
                    receiver_c.to_string()
                };
                let all_args = if args.is_empty() {
                    self_arg
                } else {
                    format!("{}, {}", self_arg, args.join(", "))
                };
                format!("{}__{}({})", tn, method, all_args)
            }
            (None, _) => format!("__method_{}({}{})", method, receiver_c,
                if args.is_empty() { String::new() } else { format!(", {}", args.join(", ")) })
        }
    }
    fn fresh_var(&mut self, prefix: &str) -> String {
        let n = self.var_count;
        self.var_count += 1;
        format!("{}_{}", prefix, n)
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(e) => {
                let c = self.gen_expr(e);
                self.emit_line(&format!("{};", c));
            }
            Stmt::Let { name, mutable: _, type_expr, value } => {
                // Special case: tuple value without explicit type annotation
                if type_expr.is_none() {
                    if let Expr::Tuple(items) = value {
                        if items.len() > 1 {
                            let mut fields = Vec::new();
                            for (i, item) in items.iter().enumerate() {
                                let ct = self.infer_ctype(item);
                                fields.push(format!("{} f{}", ct, i));
                            }
                            let mut vals = Vec::new();
                            for item in items {
                                vals.push(self.gen_expr(item));
                            }
                            let struct_type = format!("struct {{ {} }}", fields.join("; "));
                            self.var_types.insert(name.clone(), struct_type);
                            self.emit_line(&format!("struct {{ {} }} {} = {{", fields.join("; "), name));
                            self.indent += 1;
                            for (i, v) in vals.iter().enumerate() {
                                let sep = if i < vals.len() - 1 { "," } else { "" };
                                self.emit_line(&format!("{}{}", v, sep));
                            }
                            self.indent -= 1;
                            self.emit_line("};");
                            return;
                        }
                    }
                }
                let val = self.gen_expr(value);
                let ct = if let Some(te) = type_expr {
                    self.mi_ctype(Some(&self.te_string(te)))
                } else {
                    self.infer_ctype(value)
                };
                self.var_types.insert(name.clone(), ct.clone());
                self.emit_line(&format!("{} {} = {};", ct, name, val));
            }
            Stmt::Fn { name, params, return_type, body, .. } => {
                self.current_fn = Some(name.clone());
                let is_main = name == "main";
                let ret_str = return_type.as_ref().map(|t| self.te_string(t));
                let rt = if is_main { "int".to_string() } else { self.mi_ctype(ret_str.as_deref()) };
                let ps: Vec<String> = params.iter()
                    .map(|p| {
                        let pt_str = p.type_expr.as_ref().map(|t| self.te_string(t));
                        let pt = self.mi_ctype(pt_str.as_deref());
                        format!("{} {}", pt, p.name)
                    })
                    .collect();
                // Track parameter types for Index codegen
                for p in params {
                    let pt_str = p.type_expr.as_ref().map(|t| self.te_string(t));
                    let pt = self.mi_ctype(pt_str.as_deref());
                    self.var_types.insert(p.name.clone(), pt);
                }
                if is_main {
                    self.emit_line("int main(int argc, char** argv) {");
                    self.emit_line("mi_init_args(argc, argv);");
                } else if ps.is_empty() {
                    self.emit_line(&format!("{} {}() {{", rt, name));
                } else {
                    self.emit_line(&format!("{} {}({}) {{", rt, name, ps.join(", ")));
                }
                self.indent += 1;
                self.gen_block_body(body, rt.as_str() != "void");
                if is_main && !body_has_return(body) {
                    self.emit_line("return 0;");
                }
                self.indent -= 1;
                self.emit_line("}");
                self.emit_line("");
                // ponytail: void functions get implicit return; non-void must have explicit in user code
            }
            Stmt::Return(opt) => {
                if let Some(e) = opt {
                    let val = self.gen_expr(e);
                    self.emit_line(&format!("return {};", val));
                } else if self.current_fn.as_deref() == Some("main") {
                    self.emit_line("return 0;");
                } else {
                    self.emit_line("return;");
                }
            }
            Stmt::If(cond, then, else_opt) => {
                let cc = self.expr_as_condition(cond);
                self.emit_line(&format!("if ({}) {{", cc));
                self.indent += 1;
                for s in then { self.gen_stmt(s); }
                self.indent -= 1;
                if let Some(els) = else_opt {
                    // Check for else if
                    if els.len() == 1 {
                        if let Stmt::If(..) = &els[0] {
                            self.emit_line("} else ");
                            self.gen_stmt(&els[0]);
                            return;
                        }
                    }
                    self.emit_line("} else {");
                    self.indent += 1;
                    for s in els { self.gen_stmt(s); }
                    self.indent -= 1;
                }
                self.emit_line("}");
            }
            Stmt::While(cond, body) => {
                let cc = self.expr_as_condition(cond);
                self.emit_line(&format!("while ({}) {{", cc));
                self.indent += 1;
                for s in body { self.gen_stmt(s); }
                self.indent -= 1;
                self.emit_line("}");
            }
            Stmt::For(name, iter, body) => {
                let iv = self.fresh_var("_i");
                match iter {
                    Expr::Call(callee, args) if matches!(callee.as_ref(), Expr::Ident(f) if f == "range") => {
                        let start = if args.len() >= 2 { self.gen_expr(&args[0]) } else { "0".to_string() };
                        let end = if args.len() >= 2 { self.gen_expr(&args[1]) } else if args.len() == 1 { self.gen_expr(&args[0]) } else { "0".to_string() };
                        self.emit_line(&format!("for (mi_int {} = {}; {} < {}; {}++) {{", iv, start, iv, end, iv));
                        self.indent += 1;
                        self.emit_line(&format!("mi_int {} = {};", name, iv));
                        for s in body { self.gen_stmt(s); }
                        self.indent -= 1;
                        self.emit_line("}");
                    }
                    Expr::Array(items) => {
                        let arr_var = self.fresh_var("_arr");
                        let items_s: Vec<String> = items.iter().map(|i| self.gen_expr(i)).collect();
                        self.emit_line(&format!("mi_int {}_data[] = {{}};", arr_var));
                        self.emit_line(&format!("mi_array {} = mi_array_new(sizeof(mi_int), {});", arr_var, items.len()));
                        for (i, val) in items_s.iter().enumerate() {
                            let tv = self.fresh_var("_tv");
                            self.emit_line(&format!("mi_int {} = {};", tv, val));
                            self.emit_line(&format!("mi_array_set(&{}, {}, &{});", arr_var, i, tv));
                        }
                        self.emit_line(&format!("for (mi_int {} = 0; {} < {}.len; {}++) {{", iv, iv, arr_var, iv));
                        self.indent += 1;
                        self.emit_line(&format!("mi_int {} = mi_array_get_int(&{}, {});", name, arr_var, iv));
                        for s in body { self.gen_stmt(s); }
                        self.indent -= 1;
                        self.emit_line("}");
                        self.emit_line(&format!("mi_array_free(&{});", arr_var));
                    }
                    _ => {
                        let iter_s = self.gen_expr(iter);
                        self.emit_line(&format!("for (mi_int {} = 0; {} < {}; {}++) {{", iv, iv, iter_s, iv));
                        self.indent += 1;
                        self.emit_line(&format!("mi_int {} = {};", name, iv));
                        for s in body { self.gen_stmt(s); }
                        self.indent -= 1;
                        self.emit_line("}");
                    }
                }
            }
            Stmt::Loop(body) => {
                self.emit_line("for (;;) {");
                self.indent += 1;
                for s in body { self.gen_stmt(s); }
                self.indent -= 1;
                self.emit_line("}");
            }
            Stmt::Break(_) => self.emit_line("break;"),
            Stmt::Continue => self.emit_line("continue;"),
            Stmt::Type { .. } => {} // emitted earlier
            Stmt::Impl { type_name, methods } => {
                let prefix = type_name.replace(" for ", "__");
                for m in methods {
                    if let Stmt::Fn { name, params, return_type, body, .. } = m {
                        let fn_name = format!("{}__{}", prefix, name);
                        let ret_str = return_type.as_ref().map(|t| self.te_string(t));
                        let rt = self.mi_ctype(ret_str.as_deref());
                        let ps: Vec<String> = params.iter()
                            .map(|p| {
                                let pt_str = p.type_expr.as_ref().map(|t| self.te_string(t));
                                let pt = self.mi_ctype(pt_str.as_deref());
                                format!("{} {}", pt, p.name)
                            })
                            .collect();
                        if ps.is_empty() {
                            self.emit_line(&format!("{} {}() {{", rt, fn_name));
                        } else {
                            self.emit_line(&format!("{} {}({}) {{", rt, fn_name, ps.join(", ")));
                        }
                        self.indent += 1;
                        self.gen_block_body(body, rt.as_str() != "void");
                        self.indent -= 1;
                        self.emit_line("}");
                        self.emit_line("");
                    }
                }
            }
            Stmt::Trait { .. } => {} // Traits are interface definitions; no C codegen for v1
            Stmt::Extern { name, params, return_type } => {
                let rt = self.mi_ctype(return_type.as_ref().map(|t| self.te_string(t)).as_deref());
                let ps: Vec<String> = params.iter()
                    .map(|p| {
                        let pt = self.mi_ctype(p.type_expr.as_ref().map(|t| self.te_string(t)).as_deref());
                        format!("{} {}", pt, p.name)
                    })
                    .collect();
                if ps.is_empty() {
                    self.emit_line(&format!("{} {}();", rt, name));
                } else {
                    self.emit_line(&format!("{} {}({});", rt, name, ps.join(", ")));
                }
            }
            Stmt::Empty => {}
        }
    }
    fn gen_block_body(&mut self, body: &Expr, needs_return: bool) {
        match body {
            Expr::Block(stmts) => {
                let len = stmts.len();
                for (i, s) in stmts.iter().enumerate() {
                    if i == len - 1 && needs_return {
                        if let Stmt::Expr(e) = s {
                            let v = self.gen_expr(e);
                            self.emit_line(&format!("return {};", v));
                        } else if let Stmt::If(cond, then_body, else_body) = s {
                            let cc = self.expr_as_condition(cond);
                            self.emit_line(&format!("if ({}) {{", cc));
                            self.indent += 1;
                            if then_body.len() == 1 {
                                if let Stmt::Expr(e) = &then_body[0] {
                                    let v = self.gen_expr(e);
                                    self.emit_line(&format!("return {};", v));
                                } else {
                                    for s2 in then_body { self.gen_stmt(s2); }
                                }
                            } else {
                                for s2 in then_body { self.gen_stmt(s2); }
                            }
                            self.indent -= 1;
                            if let Some(els) = else_body {
                                if els.len() == 1 && matches!(&els[0], Stmt::If(..)) {
                                    self.emit_line("} else ");
                                    self.gen_stmt(&els[0]);
                                } else {
                                    self.emit_line("} else {");
                                    self.indent += 1;
                                    for s2 in els {
                                        if let Stmt::Expr(e) = s2 {
                                            let v = self.gen_expr(e);
                                            self.emit_line(&format!("return {};", v));
                                        } else {
                                            self.gen_stmt(s2);
                                        }
                                    }
                                    self.indent -= 1;
                                    self.emit_line("}");
                                }
                            } else {
                                self.emit_line("}");
                            }
                        } else {
                            self.gen_stmt(s);
                        }
                    } else {
                        self.gen_stmt(s);
                    }
                }
            }
            Expr::If(cond, then, else_opt) => {
                let cc = self.expr_as_condition(cond);
                let t = self.gen_expr(then);
                if let Some(el) = else_opt {
                    let e = self.gen_expr(el);
                    self.emit_line(&format!("if ({}) {{ return {}; }} else {{ return {}; }}", cc, t, e));
                } else {
                    self.emit_line(&format!("if ({}) {{ {}; }}", cc, t));
                }
            }
            Expr::Match(expr, arms) => {
                let m = self.gen_expr(expr);
                let rv = self.fresh_var("_mr");
                self.emit_line(&format!("mi_int {} = 0;", rv));
                for (i, arm) in arms.iter().enumerate() {
                    let cond = self.gen_pattern_cond(&arm.pattern, &m);
                    let body = self.gen_expr(&arm.body);
                    if i == 0 {
                        self.emit_line(&format!("if ({}) {{", cond));
                    } else {
                        self.emit_line(&format!("}} else if ({}) {{", cond));
                    }
                    self.indent += 1;
                    if needs_return {
                        self.emit_line(&format!("return {};", body));
                    } else {
                        self.emit_line(&format!("{} = {};", rv, body));
                    }
                    self.indent -= 1;
                }
                if !arms.is_empty() {
                    self.emit_line("}");
                }
                if !needs_return {
                    self.emit_line(&format!("return {};", rv));
                }
            }
            Expr::Call(..) | Expr::MethodCall(..) => {
                let v = self.gen_expr(body);
                if needs_return {
                    self.emit_line(&format!("return {};", v));
                } else {
                    self.emit_line(&format!("{};", v));
                }
            }
            _ => {
                if needs_return {
                    let v = self.gen_expr(body);
                    self.emit_line(&format!("return {};", v));
                } else {
                    let v = self.gen_expr(body);
                    self.emit_line(&format!("{};", v));
                }
            }
        }
    }
    fn gen_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Int(n) => format!("{}", n),
            Expr::Float(f) => {
                if f.is_infinite() { return "MI_INF".to_string(); }
                if f.is_nan() { return "MI_NAN".to_string(); }
                format!("{}", f)
            }
            Expr::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
            Expr::Str(s, interps) => {
                if interps.is_empty() {
                    format!("mi_str_lit(\"{}\")", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
                } else {
                    let prefix = format!("mi_str_lit(\"{}\")", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
                    let mut result = prefix;
                    for expr in interps {
                        let val = self.gen_expr(expr);
                        // ponytail: wrap non-string expressions with str()
                        let ct = self.infer_ctype(expr);
                        let val = if ct == "mi_str" { val } else { format!("str({})", val) };
                        result = format!("mi_str_concat({}, {})", result, val);
                    }
                    result
                }
            }
            Expr::Char(c) => format!("'{}'", c.escape_default()),
            Expr::Nil => "mi_nil()".to_string(),

            Expr::Ident(name) => name.clone(),

            Expr::Field(obj, field) => {
                let o = self.gen_expr(obj);
                // Numeric field names (tuple access .0, .1) map to f0, f1
                if let Ok(_) = field.parse::<i64>() {
                    format!("{}.f{}", o, field)
                } else {
                    format!("{}.{}", o, field)
                }
            }
            Expr::Index(arr, idx) => {
                let i = self.gen_expr(idx);
                let is_array = match arr.as_ref() {
                    Expr::Ident(name) => self.var_types.get(name).map(|t| t.starts_with("mi_array")).unwrap_or(false),
                    _ => false,
                };
                match arr.as_ref() {
                    Expr::Str(s, _) => {
                        if let Expr::Int(n) = idx.as_ref() {
                            let c = s.as_bytes().get(*n as usize).copied().unwrap_or(0);
                            format!("((mi_char){})", c)
                        } else {
                            format!("(mi_char)(mi_str_lit(\"{}\").data[{}])", s.replace('"', "\\\"").replace('\n', "\\n"), i)
                        }
                    }
                    _ if is_array => {
                        let a = self.gen_expr(arr);
                        format!("mi_array_get_int(&{}, {})", a, i)
                    }
                    _ => {
                        let a = self.gen_expr(arr);
                        format!("mi_str_at({}, {})", a, i)
                    }
                }
            }
            Expr::Call(callee, args) => {
                let callee_str = self.gen_expr(callee);
                let args_s: Vec<String> = args.iter().map(|a| self.gen_expr(a)).collect();
                let fn_name = match callee.as_ref() {
                    Expr::Ident(n) => match n.as_str() {
                        "sin" | "cos" | "tan" | "sqrt" | "pow" | "exp" | "log" | "log10"
                        | "abs" | "floor" | "ceil" | "round" | "atan2" => format!("mi_{}", n),
                        _ => callee_str,
                    },
                    _ => callee_str,
                };
                format!("{}({})", fn_name, args_s.join(", "))
            }
            Expr::MethodCall(obj, method, args) => {
                let o = self.gen_expr(obj);
                let args_s: Vec<String> = args.iter().map(|a| self.gen_expr(a)).collect();
                let receiver_ctype = self.infer_ctype(obj);
                self.resolve_method_call(&receiver_ctype, method, &o, &args_s)
            }

            Expr::Unary(op, rhs) => {
                let r = self.gen_expr(rhs);
                match op {
                    UnOp::Neg => format!("(-{})", r),
                    UnOp::Not => format!("(!{})", r),
                }
            }
            Expr::Binary(lhs, op, rhs) => {
                let l = self.gen_expr(lhs);
                let r = self.gen_expr(rhs);
                let is_str = self.is_string_expr(lhs) || self.is_string_expr(rhs);
                if is_str {
                    // String operations use runtime functions
                    match op {
                        BinOp::Add => format!("mi_str_concat({}, {})", l, r),
                        BinOp::Eq => format!("mi_str_eq({}, {})", l, r),
                        BinOp::Neq => format!("mi_str_ne({}, {})", l, r),
                        BinOp::Lt => format!("mi_str_lt({}, {})", l, r),
                        BinOp::Le => format!("mi_str_le({}, {})", l, r),
                        BinOp::Gt => format!("mi_str_gt({}, {})", l, r),
                        BinOp::Ge => format!("mi_str_ge({}, {})", l, r),
                        _ => format!("({} {} {})", l, self.binop_to_c_op(op), r),
                    }
                } else {
                    format!("({} {} {})", l, self.binop_to_c_op(op), r)
                }
            }
            Expr::Assign(target, op, value) => {
                let t = self.gen_expr(target);
                let v = self.gen_expr(value);
                let c_op = match op {
                    AssignOp::Set => "=",
                    AssignOp::Add => "+=",
                    AssignOp::Sub => "-=",
                    AssignOp::Mul => "*=",
                    AssignOp::Div => "/=",
                };
                format!("{} {} {}", t, c_op, v)
            }

            Expr::If(cond, then, else_opt) => {
                let cc = self.expr_as_condition(cond);
                let t = self.gen_expr(then);
                if let Some(el) = else_opt {
                    let e = self.gen_expr(el);
                    format!("({} ? {} : {})", cc, t, e)
                } else {
                    format!("({} ? {} : mi_nil())", cc, t)
                }
            }

            Expr::Match(expr, arms) => {
                let m = self.gen_expr(expr);
                let result_var = self.fresh_var("_match");
                self.emit_line(&format!("mi_int {} = 0;", result_var));
                // Generate if-else chain for match arms
                for (i, arm) in arms.iter().enumerate() {
                    let cond = self.gen_pattern_cond(&arm.pattern, &m);
                    let body = self.gen_expr(&arm.body);
                    if i == 0 {
                        self.emit_line(&format!("if ({}) {{", cond));
                    } else {
                        self.emit_line(&format!("}} else if ({}) {{", cond));
                    }
                    self.indent += 1;
                    self.emit_line(&format!("{} = {};", result_var, body));
                    self.indent -= 1;
                }
                if !arms.is_empty() {
                    self.emit_line("}");
                }
                result_var
            }

            Expr::Block(stmts) => {
                let last_is_expr = stmts.last().map_or(false, |s| matches!(s, Stmt::Expr(_)));
                if stmts.len() == 1 && last_is_expr {
                    // Single expression block: use the expression value directly
                    if let Stmt::Expr(e) = &stmts[0] {
                        self.gen_expr(e)
                    } else {
                        unreachable!()
                    }
                } else if stmts.is_empty() {
                    "0".to_string()
                } else {
                    // Multi-statement block: use temp variable
                    let rv = self.fresh_var("_block");
                    self.emit_line(&format!("mi_int {} = 0;", rv));
                    for (i, s) in stmts.iter().enumerate() {
                        if i == stmts.len() - 1 {
                            if let Stmt::Expr(e) = s {
                                let v = self.gen_expr(e);
                                self.emit_line(&format!("{} = {};", rv, v));
                            } else {
                                self.gen_stmt(s);
                            }
                        } else {
                            self.gen_stmt(s);
                        }
                    }
                    rv
                }
            }

            Expr::Fn(params, _return_type, body) => {
                // Generate anonymous function as nested function or lambda
                // For now, just generate the body inline
                self.gen_expr(body)
            }

            Expr::Pipe(lhs, rhs) => {
                let l = self.gen_expr(lhs);
                match rhs.as_ref() {
                    Expr::Ident(fname) => format!("{}({})", fname, l),
                    Expr::Call(callee, args) => {
                        let callee_str = self.gen_expr(callee);
                        let args_s: Vec<String> = args.iter().map(|a| self.gen_expr(a)).collect();
                        let mut all_args = vec![l];
                        all_args.extend(args_s);
                        format!("{}({})", callee_str, all_args.join(", "))
                    }
                    _ => format!("({})({})", self.gen_expr(rhs), l),
                }
            }
            Expr::Array(items) => {
                if items.is_empty() {
                    "mi_array_new(sizeof(mi_int), 0)".to_string()
                } else {
                    let elem_var = self.fresh_var("_arr");
                    let items_s: Vec<String> = items.iter().map(|i| self.gen_expr(i)).collect();
                    let elem_type = "mi_int";
                    self.emit_line(&format!("mi_int {}_data[] = {{}};", elem_var));
                    self.emit_line(&format!("mi_array {} = mi_array_new(sizeof({}), {});", elem_var, elem_type, items.len()));
                    for (i, val) in items_s.iter().enumerate() {
                        let tv = self.fresh_var("_tv");
                        self.emit_line(&format!("mi_int {} = {};", tv, val));
                        self.emit_line(&format!("mi_array_set(&{}, {}, &{});", elem_var, i, tv));
                    }
                    elem_var
                }
            }
            Expr::Tuple(items) => {
                if items.is_empty() { "0".to_string() }
                else if items.len() == 1 { self.gen_expr(&items[0]) }
                else {
                    let mut field_types = Vec::new();
                    for item in items {
                        field_types.push(self.infer_ctype(item));
                    }
                    let mut field_vals = Vec::new();
                    for item in items {
                        field_vals.push(self.gen_expr(item));
                    }
                    let decls: Vec<String> = field_types.iter().enumerate()
                        .map(|(i, t)| format!("{} f{}", t, i)).collect();
                    let vals = field_vals.join(", ");
                    format!("((struct {{ {} }}){{ {} }})", decls.join("; "), vals)
                }
            }
            Expr::StructLit(name, fields) => {
                let fs: Vec<String> = fields.iter()
                    .map(|(fn_, fv)| format!(".{} = {}", fn_, self.gen_expr(fv)))
                    .collect();
                format!("({}){{{}}}", name, fs.join(", "))
            }

            Expr::Import(_, _) => "/* import */".to_string(),
        }
    }

    fn expr_as_condition(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Binary(l, BinOp::And, r) => format!("({} && {})", self.expr_as_condition(l), self.expr_as_condition(r)),
            Expr::Binary(l, BinOp::Or, r) => format!("({} || {})", self.expr_as_condition(l), self.expr_as_condition(r)),
            Expr::Unary(UnOp::Not, r) => format!("!({})", self.expr_as_condition(r)),
            _ => {
                let v = self.gen_expr(expr);
                format!("({})", v)
            }
        }
    }

    fn gen_pattern_cond(&mut self, pattern: &Pattern, matched: &str) -> String {
        match pattern {
            Pattern::Wild => "1".to_string(),
            Pattern::Int(n) => format!("({} == {})", matched, n),
            Pattern::Float(f) => format!("({} == {})", matched, f),
            Pattern::Bool(b) => format!("({} == {})", matched, if *b { "1" } else { "0" }),
            Pattern::Str(s) => format!("mi_str_eq({}, mi_str_lit(\"{}\"))", matched, s),
            Pattern::Char(c) => format!("({} == '{}')", matched, c),
            Pattern::Ident(name) => {
                if name == "_" {
                    "1".to_string()
                } else {
                    // Binding pattern - bind and match
                    // For now, just match and ignore the binding
                    "1".to_string()
                }
            }
            Pattern::Binding(name, inner) => {
                // Binding pattern
                self.gen_pattern_cond(inner, matched)
            }
            Pattern::Tuple(patterns) => {
                // Tuple destructuring - simplified
                "1".to_string()
            }
            Pattern::Or(p1, p2) => {
                format!("({} || {})",
                    self.gen_pattern_cond(p1, matched),
                    self.gen_pattern_cond(p2, matched))
            }
            Pattern::Range(p1, p2) => {
                let l = self.gen_pattern_cond(p1, matched);
                let r = self.gen_pattern_cond(p2, matched);
                format!("(({} >= {}) && ({} <= {}))", matched, l, matched, r)
            }
            Pattern::Struct(_, _) => "1".to_string(),
        }
    }

    fn escape_c_string(&self, s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
    }

    /// Heuristic: check if an expression produces a string value
    fn is_string_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Str(_, _) => true,
            Expr::Call(callee, _) => {
                if let Expr::Ident(name) = callee.as_ref() {
                    matches!(name.as_str(), "str" | "str_char" | "str_bool" | "str_float" | "read_file")
                } else {
                    false
                }
            }
            Expr::Binary(l, BinOp::Add, r) => self.is_string_expr(l) || self.is_string_expr(r),
            Expr::If(_, t, _) => self.is_string_expr(t),
            Expr::MethodCall(_, _, _) => true, // methods return unknown, could be string
            _ => false,
        }
    }

    fn binop_to_c_op(&self, op: &BinOp) -> &'static str {
        match op {
            BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*",
            BinOp::Div => "/", BinOp::Mod => "%",
            BinOp::Eq => "==", BinOp::Neq => "!=",
            BinOp::Lt => "<", BinOp::Gt => ">",
            BinOp::Le => "<=", BinOp::Ge => ">=",
            BinOp::And => "&&", BinOp::Or => "||",
        }
    }
}
fn body_has_return(body: &Expr) -> bool {
    match body {
        Expr::Block(stmts) => stmts.iter().any(|s| matches!(s, Stmt::Return(_))),
        Expr::If(_, then, else_opt) => {
            body_has_return(then) || else_opt.as_ref().map(|e| body_has_return(e)).unwrap_or(false)
        }
        _ => false,
    }
}
