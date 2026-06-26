use std::collections::HashMap;
use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub enum MiType {
    Int,
    Float,
    Bool,
    Str,
    Char,
    Nil,
    Fn(Vec<MiType>, Box<MiType>),
    Struct(String, Vec<(String, MiType)>),
    Enum(String, Vec<(String, Vec<(String, MiType)>)>),
    Generic(String),
    TypeVar(usize),
    Array(Box<MiType>),
    Tuple(Vec<MiType>),
    Unknown,
}

pub struct TypeChecker {
    pub globals: HashMap<String, (MiType, bool)>, // name -> (type, mutable)
    pub functions: HashMap<String, MiType>,
    pub types: HashMap<String, MiType>,
    methods: HashMap<String, HashMap<String, MiType>>, // type_name -> {method_name -> Fn type}
    locals: Vec<HashMap<String, (MiType, bool)>>,
    errors: Vec<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        // Built-in functions
        globals.insert("print".to_string(), (MiType::Fn(vec![MiType::Str], Box::new(MiType::Nil)), false));
        globals.insert("println".to_string(), (MiType::Fn(vec![MiType::Str], Box::new(MiType::Nil)), false));
        globals.insert("len".to_string(), (MiType::Fn(vec![MiType::Array(Box::new(MiType::Unknown))], Box::new(MiType::Int)), false));
        globals.insert("str".to_string(), (MiType::Fn(vec![MiType::Int], Box::new(MiType::Str)), false));
        globals.insert("int".to_string(), (MiType::Fn(vec![MiType::Str], Box::new(MiType::Int)), false));
        globals.insert("len_str".to_string(), (MiType::Fn(vec![MiType::Str], Box::new(MiType::Int)), false));
        globals.insert("range".to_string(), (MiType::Fn(vec![MiType::Int], Box::new(MiType::Array(Box::new(MiType::Int)))), false));
        // Math library
        globals.insert("pi".to_string(), (MiType::Fn(vec![], Box::new(MiType::Float)), false));
        globals.insert("e".to_string(), (MiType::Fn(vec![], Box::new(MiType::Float)), false));
        globals.insert("sin".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("cos".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("tan".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("sqrt".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("pow".to_string(), (MiType::Fn(vec![MiType::Float, MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("exp".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("log".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("log10".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("abs".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("floor".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("ceil".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("round".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Float)), false));
        globals.insert("atan2".to_string(), (MiType::Fn(vec![MiType::Float, MiType::Float], Box::new(MiType::Float)), false));
        // File I/O
        globals.insert("read_file".to_string(), (MiType::Fn(vec![MiType::Str], Box::new(MiType::Str)), false));
        globals.insert("write_file".to_string(), (MiType::Fn(vec![MiType::Str, MiType::Str], Box::new(MiType::Bool)), false));
        globals.insert("file_exists".to_string(), (MiType::Fn(vec![MiType::Str], Box::new(MiType::Bool)), false));
        // String builder
        globals.insert("sb_new".to_string(), (MiType::Fn(vec![], Box::new(MiType::Unknown)), false));
        globals.insert("sb_append".to_string(), (MiType::Fn(vec![MiType::Unknown, MiType::Str], Box::new(MiType::Nil)), false));
        globals.insert("sb_build".to_string(), (MiType::Fn(vec![MiType::Unknown], Box::new(MiType::Str)), false));
        // CLI args
        globals.insert("os_args".to_string(), (MiType::Fn(vec![], Box::new(MiType::Array(Box::new(MiType::Str)))), false));

        // String conversions
        globals.insert("str_bool".to_string(), (MiType::Fn(vec![MiType::Bool], Box::new(MiType::Str)), false));
        globals.insert("str_float".to_string(), (MiType::Fn(vec![MiType::Float], Box::new(MiType::Str)), false));
        globals.insert("str_char".to_string(), (MiType::Fn(vec![MiType::Char], Box::new(MiType::Str)), false));
        TypeChecker {
            globals,
            functions: HashMap::new(),
            types: HashMap::new(),
            methods: HashMap::new(),
            locals: vec![HashMap::new()],
            errors: Vec::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), String> {
        // First pass: collect function signatures, type definitions, and method signatures
        for stmt in &program.stmts {
            self.collect_signature(stmt);
        }

        // Second pass: check all statements
        for stmt in &program.stmts {
            self.check_stmt(stmt, 0, 0)?;
        }

        if !self.errors.is_empty() {
            Err(self.errors.join("\n"))
        } else {
            Ok(())
        }
    }

    fn collect_signature(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Fn { name, params, return_type, .. } => {
                let param_types = params.iter().map(|p| {
                    p.type_expr.as_ref().map(|t| self.type_expr_to_type(t))
                        .unwrap_or(MiType::Unknown)
                }).collect();
                let ret = return_type.as_ref().map(|t| self.type_expr_to_type(t))
                    .unwrap_or(MiType::Unknown);
                self.functions.insert(name.clone(), MiType::Fn(param_types, Box::new(ret)));
            }
            Stmt::Extern { name, params, return_type } => {
                let param_types = params.iter().map(|p| {
                    p.type_expr.as_ref().map(|t| self.type_expr_to_type(t))
                        .unwrap_or(MiType::Unknown)
                }).collect();
                let ret = return_type.as_ref().map(|t| self.type_expr_to_type(t))
                    .unwrap_or(MiType::Unknown);
                self.functions.insert(name.clone(), MiType::Fn(param_types, Box::new(ret)));
            }
            Stmt::Type { name, variants, .. } => {
                if variants.len() == 1 && variants[0].name == *name {
                    // Struct
                    let fields: Vec<(String, MiType)> = variants[0].fields.iter()
                        .map(|f| (f.name.clone(), self.type_expr_to_type(&f.type_expr)))
                        .collect();
                    self.types.insert(name.clone(), MiType::Struct(name.clone(), fields));
                } else {
                    // Enum
                    let v: Vec<(String, Vec<(String, MiType)>)> = variants.iter()
                        .map(|v| {
                            let fields: Vec<(String, MiType)> = v.fields.iter()
                                .map(|f| (f.name.clone(), self.type_expr_to_type(&f.type_expr)))
                                .collect();
                            (v.name.clone(), fields)
                        }).collect();
                    self.types.insert(name.clone(), MiType::Enum(name.clone(), v));
                }
            }
            Stmt::Impl { type_name, methods } => {
                let mut sigs = HashMap::new();
                for m in methods {
                    if let Stmt::Fn { name, params, return_type, .. } = m {
                        let param_types: Vec<MiType> = params.iter().map(|p| {
                            p.type_expr.as_ref().map(|t| self.type_expr_to_type(t))
                                .unwrap_or(MiType::Unknown)
                        }).collect();
                        let ret = return_type.as_ref().map(|t| self.type_expr_to_type(t))
                            .unwrap_or(MiType::Unknown);
                        sigs.insert(name.clone(), MiType::Fn(param_types, Box::new(ret)));
                    }
                }
                self.methods.insert(type_name.clone(), sigs);
            }
            _ => {}
        }
    }

    fn type_expr_to_type(&self, te: &TypeExpr) -> MiType {
        match te {
            TypeExpr::Named(n) => {
                match n.as_str() {
                    "Int" => MiType::Int,
                    "Float" => MiType::Float,
                    "Bool" => MiType::Bool,
                    "Str" => MiType::Str,
                    "Char" => MiType::Char,
                    "Nil" => MiType::Nil,
                    _ => self.types.get(n).cloned().unwrap_or(MiType::Unknown),
                }
            }
            TypeExpr::Generic(n, args) => {
                let elem = self.type_expr_to_type(&args[0]);
                match n.as_str() {
                    "Array" | "List" => MiType::Array(Box::new(elem)),
                    _ => MiType::Unknown,
                }
            }
            TypeExpr::Fn(params, ret) => {
                MiType::Fn(
                    params.iter().map(|p| self.type_expr_to_type(p)).collect(),
                    Box::new(self.type_expr_to_type(ret)),
                )
            }
            TypeExpr::Tuple(ts) => MiType::Tuple(ts.iter().map(|t| self.type_expr_to_type(t)).collect()),
            TypeExpr::Infer => MiType::Unknown,
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt, line: usize, col: usize) -> Result<MiType, String> {
        match stmt {
            Stmt::Expr(e) => self.check_expr(e, line, col),

            Stmt::Let { name, mutable, type_expr, value } => {
                let val_type = self.check_expr(value, line, col)?;
                if let Some(te) = type_expr {
                    let ann_type = self.type_expr_to_type(te);
                    if !self.types_compatible(&val_type, &ann_type) {
                        return Err(self.err(&format!("expected {}, got {}", ann_type, val_type), line, col));
                    }
                }
                self.locals.last_mut().unwrap().insert(name.clone(), (val_type, *mutable));
                Ok(MiType::Nil)
            }

            Stmt::Fn { name, params, return_type, body, .. } => {
                self.locals.push(HashMap::new());
                for param in params {
                    let pt = param.type_expr.as_ref()
                        .map(|t| self.type_expr_to_type(t))
                        .unwrap_or(MiType::Unknown);
                    self.locals.last_mut().unwrap().insert(param.name.clone(), (pt.clone(), false));
                    // Check default value type if present
                    if let Some(default) = &param.default {
                        let dt = self.check_expr(default, line, col)?;
                        if !self.types_compatible(&dt, &pt) {
                            self.errors.push(self.err(&format!(
                                "parameter '{}': default value type {} does not match declared type {}",
                                param.name, dt, pt), line, col));
                        }
                    }
                }
                let body_type = self.check_expr(body, line, col)?;
                if let Some(rt) = return_type {
                    let ret_type = self.type_expr_to_type(rt);
                    if !self.types_compatible(&body_type, &ret_type) {
                        self.errors.push(self.err(&format!(
                            "function '{}': expected return type {}, got body type {}",
                            name, ret_type, body_type), line, col));
                    }
                }
                self.locals.pop();
                Ok(MiType::Nil)
            }

            Stmt::Extern { name, params, return_type } => {
                // Validate parameter types
                for param in params {
                    if let Some(te) = &param.type_expr {
                        self.type_expr_to_type(te);
                    }
                }
                if let Some(rt) = return_type {
                    self.type_expr_to_type(rt);
                }
                Ok(MiType::Nil)
            }

            Stmt::Return(opt) => {
                if let Some(e) = opt {
                    self.check_expr(e, line, col)
                } else {
                    Ok(MiType::Nil)
                }
            }

            Stmt::If(cond, then, else_opt) => {
                let cond_type = self.check_expr(cond, line, col)?;
                if !self.types_compatible(&cond_type, &MiType::Bool) {
                    return Err(self.err(&format!("if condition must be Bool, got {}", cond_type), line, col));
                }
                self.locals.push(HashMap::new());
                let mut then_type = MiType::Nil;
                for s in then {
                    then_type = self.check_stmt(s, line, col)?;
                }
                self.locals.pop();
                let mut else_type = MiType::Nil;
                if let Some(else_stmts) = else_opt {
                    self.locals.push(HashMap::new());
                    for s in else_stmts {
                        else_type = self.check_stmt(s, line, col)?;
                    }
                    self.locals.pop();
                }
                // Unify then/else types
                if then_type == else_type {
                    Ok(then_type)
                } else if then_type == MiType::Unknown {
                    Ok(else_type)
                } else if else_type == MiType::Unknown {
                    Ok(then_type)
                } else if else_type == MiType::Nil {
                    Ok(then_type)
                } else if then_type == MiType::Nil {
                    Ok(else_type)
                } else {
                    Ok(then_type) // prefer then type when both are non-Nil and differ
                }
            }

            Stmt::While(cond, body) => {
                let cond_type = self.check_expr(cond, line, col)?;
                if !self.types_compatible(&cond_type, &MiType::Bool) {
                    return Err(self.err(&format!("while condition must be Bool, got {}", cond_type), line, col));
                }
                self.locals.push(HashMap::new());
                for s in body {
                    self.check_stmt(s, line, col)?;
                }
                self.locals.pop();
                Ok(MiType::Nil)
            }

            Stmt::For(name, iter, body) => {
                let iter_type = self.check_expr(iter, line, col)?;
                // Infer element type from iterable
                let elem_type = match &iter_type {
                    MiType::Array(elem) => *elem.clone(),
                    MiType::Str => MiType::Char,
                    MiType::Tuple(ts) if !ts.is_empty() => ts[0].clone(),
                    MiType::Unknown => MiType::Unknown,
                    _ => return Err(self.err(&format!("cannot iterate over type {}", iter_type), line, col)),
                };
                self.locals.push(HashMap::new());
                self.locals.last_mut().unwrap().insert(name.clone(), (elem_type, false));
                for s in body {
                    self.check_stmt(s, line, col)?;
                }
                self.locals.pop();
                Ok(MiType::Nil)
            }

            Stmt::Loop(body) => {
                self.locals.push(HashMap::new());
                for s in body {
                    self.check_stmt(s, line, col)?;
                }
                self.locals.pop();
                Ok(MiType::Nil)
            }

            Stmt::Break(v) => {
                if let Some(e) = v {
                    self.check_expr(e, line, col)?;
                }
                Ok(MiType::Nil)
            }

            Stmt::Continue => Ok(MiType::Nil),

            Stmt::Type { .. } => Ok(MiType::Nil),

            Stmt::Impl { type_name: _, methods } => {
                for m in methods {
                    if let Stmt::Fn { .. } = m {
                        self.check_stmt(m, line, col)?;
                    }
                }
                Ok(MiType::Nil)
            }

            Stmt::Trait { .. } => Ok(MiType::Nil),

            Stmt::Empty => Ok(MiType::Nil),
        }
    }

    fn check_pattern(&self, pattern: &Pattern, expected: &MiType, line: usize, col: usize) -> Result<(), String> {
        match pattern {
            Pattern::Wild => Ok(()),
            Pattern::Int(_) => {
                if !self.types_compatible(&MiType::Int, expected) {
                    Err(self.err(&format!("expected Int pattern, matching {}", expected), line, col))
                } else { Ok(()) }
            }
            Pattern::Float(_) => {
                if !self.types_compatible(&MiType::Float, expected) {
                    Err(self.err(&format!("expected Float pattern, matching {}", expected), line, col))
                } else { Ok(()) }
            }
            Pattern::Bool(_) => {
                if !self.types_compatible(&MiType::Bool, expected) {
                    Err(self.err(&format!("expected Bool pattern, matching {}", expected), line, col))
                } else { Ok(()) }
            }
            Pattern::Str(_) => {
                if !self.types_compatible(&MiType::Str, expected) {
                    Err(self.err(&format!("expected Str pattern, matching {}", expected), line, col))
                } else { Ok(()) }
            }
            Pattern::Char(_) => {
                if !self.types_compatible(&MiType::Char, expected) {
                    Err(self.err(&format!("expected Char pattern, matching {}", expected), line, col))
                } else { Ok(()) }
            }
            Pattern::Ident(_) => Ok(()), // binds the value, any type ok
            Pattern::Binding(_, pat) => self.check_pattern(pat, expected, line, col),
            Pattern::Tuple(pats) => {
                match expected {
                    MiType::Tuple(ts) if ts.len() == pats.len() => {
                        for (p, t) in pats.iter().zip(ts.iter()) {
                            self.check_pattern(p, t, line, col)?;
                        }
                        Ok(())
                    }
                    MiType::Unknown => Ok(()),
                    _ => Err(self.err(&format!("expected tuple pattern with {} fields, got {}", pats.len(), expected), line, col)),
                }
            }
            Pattern::Struct(name, fields) => {
                match expected {
                    MiType::Struct(sname, def_fields) if sname == name => {
                        for (fname, fpat) in fields {
                            if let Some((_, ft)) = def_fields.iter().find(|(n, _)| n == fname) {
                                self.check_pattern(fpat, ft, line, col)?;
                            } else {
                                return Err(self.err(&format!("struct '{}' has no field '{}'", name, fname), line, col));
                            }
                        }
                        Ok(())
                    }
                    MiType::Unknown => Ok(()),
                    _ => Err(self.err(&format!("expected struct '{}', got {}", name, expected), line, col)),
                }
            }
            Pattern::Or(p1, p2) => {
                self.check_pattern(p1, expected, line, col)?;
                self.check_pattern(p2, expected, line, col)?;
                Ok(())
            }
            Pattern::Range(p1, p2) => {
                self.check_pattern(p1, expected, line, col)?;
                self.check_pattern(p2, expected, line, col)?;
                Ok(())
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern, matched_type: &MiType) {
        match pattern {
            Pattern::Ident(name) => {
                self.locals.last_mut().unwrap()
                    .insert(name.clone(), (matched_type.clone(), false));
            }
            Pattern::Binding(name, pat) => {
                self.locals.last_mut().unwrap()
                    .insert(name.clone(), (matched_type.clone(), false));
                self.bind_pattern(pat, matched_type);
            }
            Pattern::Tuple(pats) => {
                if let MiType::Tuple(ts) = matched_type {
                    for (pat, t) in pats.iter().zip(ts.iter()) {
                        self.bind_pattern(pat, t);
                    }
                }
            }
            Pattern::Struct(_, fields) => {
                if let MiType::Struct(_, def_fields) = matched_type {
                    for (fname, fpat) in fields {
                        if let Some((_, ft)) = def_fields.iter().find(|(n, _)| n == fname) {
                            self.bind_pattern(fpat, ft);
                        }
                    }
                }
            }
            Pattern::Or(p1, p2) => {
                self.bind_pattern(p1, matched_type);
                self.bind_pattern(p2, matched_type);
            }
            _ => {} // Wild, Int, Float, Bool, Str, Char, Range — no binding
        }
    }

    fn check_expr(&mut self, expr: &Expr, line: usize, col: usize) -> Result<MiType, String> {
        match expr {
            Expr::Int(_) => Ok(MiType::Int),
            Expr::Float(_) => Ok(MiType::Float),
            Expr::Bool(_) => Ok(MiType::Bool),
            Expr::Str(_, _) => Ok(MiType::Str),
            Expr::Char(_) => Ok(MiType::Char),
            Expr::Nil => Ok(MiType::Nil),

            Expr::Ident(name) => {
                // Check locals (most recent first), then globals
                for scope in self.locals.iter().rev() {
                    if let Some((t, _)) = scope.get(name) {
                        return Ok(t.clone());
                    }
                }
                if let Some((t, _)) = self.globals.get(name) {
                    return Ok(t.clone());
                }
                if self.functions.contains_key(name) {
                    return Ok(self.functions[name].clone());
                }
                Err(self.err(&format!("undefined variable '{}'", name), line, col))
            }

            Expr::Field(obj, field) => {
                let obj_type = self.check_expr(obj, line, col)?;
                match &obj_type {
                    MiType::Struct(_, fields) => {
                        for (n, t) in fields {
                            if n == field { return Ok(t.clone()); }
                        }
                        Err(self.err(&format!("struct has no field '{}'", field), line, col))
                    }
                    MiType::Tuple(ts) => {
                        // Field access on tuple: f0, f1, etc.
                        if let Some(idx_str) = field.strip_prefix('f') {
                            if let Ok(idx) = idx_str.parse::<usize>() {
                                if idx < ts.len() {
                                    return Ok(ts[idx].clone());
                                }
                            }
                        }
                        Err(self.err(&format!("tuple has no field '{}'", field), line, col))
                    }
                    _ => Err(self.err(&format!("cannot access field '{}' on type {}", field, obj_type), line, col)),
                }
            }

            Expr::Index(arr, idx) => {
                let arr_type = self.check_expr(arr, line, col)?;
                let idx_type = self.check_expr(idx, line, col)?;
                if !self.types_compatible(&idx_type, &MiType::Int) {
                    return Err(self.err(&format!("index must be Int, got {}", idx_type), line, col));
                }
                match &arr_type {
                    MiType::Array(elem) => Ok(*elem.clone()),
                    MiType::Str => Ok(MiType::Char),
                    MiType::Tuple(ts) => {
                        // Try constant index - we don't have const eval, so just return the first element type
                        ts.first().cloned().ok_or_else(|| {
                            self.err("cannot index empty tuple", line, col)
                        })
                    }
                    _ => Err(self.err(&format!("cannot index type {}", arr_type), line, col)),
                }
            }

            Expr::Call(callee, args) => {
                let callee_type = self.check_expr(callee, line, col)?;
                let arg_types: Result<Vec<MiType>, String> =
                    args.iter().map(|a| self.check_expr(a, line, col)).collect();
                let arg_types = arg_types?;
                match &callee_type {
                    MiType::Fn(param_types, ret_type) => {
                        let name_hint = if let Expr::Ident(n) = callee.as_ref() { Some(n.as_str()) } else { None };
                        let is_range = name_hint == Some("range");
                        if !is_range && param_types.len() != arg_types.len() {
                            return Err(self.err(&format!(
                                "expected {} arguments, got {}", param_types.len(), arg_types.len()), line, col));
                        }
                        if is_range && (arg_types.len() < 1 || arg_types.len() > 2) {
                            return Err(self.err(&format!(
                                "expected 1 or 2 arguments, got {}", arg_types.len()), line, col));
                        }
                        for (i, (pt, at)) in param_types.iter().zip(arg_types.iter()).enumerate() {
                            if !self.types_compatible(at, pt) {
                                return Err(self.err(&format!(
                                    "argument {}: expected {}, got {}", i + 1, pt, at), line, col));
                            }
                        }
                        Ok(*ret_type.clone())
                    }
                    _ => {
                        // Maybe it's a type constructor (enum variant or struct)
                        let type_name = match &callee_type {
                            MiType::Struct(n, _) => Some(n.clone()),
                            MiType::Enum(n, _) => Some(n.clone()),
                            _ => {
                                // Check if callee is a type name (via Expr::Ident)
                                if let Expr::Ident(name) = callee.as_ref() {
                                    if self.types.contains_key(name) {
                                        Some(name.clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                        };
                        if let Some(tn) = type_name {
                            if let Some(MiType::Enum(_, variants)) = self.types.get(&tn) {
                                // Check if it's a variant with the same name as the type
                                if let Some((_, fields)) = variants.iter().find(|(n, _)| n.as_str() == tn.as_str()) {
                                    if fields.len() == arg_types.len() {
                                        for (i, ((_, ft), at)) in fields.iter().zip(arg_types.iter()).enumerate() {
                                            if !self.types_compatible(at, ft) {
                                                return Err(self.err(&format!(
                                                    "field {}: expected {}, got {}", i + 1, ft, at), line, col));
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(callee_type)
                        } else {
                            Err(self.err(&format!("cannot call non-function type {}", callee_type), line, col))
                        }
                    }
                }
            }

            Expr::MethodCall(obj, method, args) => {
                let obj_type = self.check_expr(obj, line, col)?;
                let arg_types: Result<Vec<MiType>, String> =
                    args.iter().map(|a| self.check_expr(a, line, col)).collect();
                let arg_types = arg_types?;

                // Find the type name for method lookup
                let type_name = self.type_name(&obj_type);
                if let Some(ref tn) = type_name {
                    if let Some(methods) = self.methods.get(tn) {
                        if let Some(fn_type) = methods.get(method) {
                            if let MiType::Fn(param_types, ret_type) = fn_type {
                                let method_params = &param_types[1..];
                                if method_params.len() != arg_types.len() {
                                    return Err(self.err(&format!(
                                        "method '{}': expected {} arguments, got {}",
                                        method, method_params.len(), arg_types.len()), line, col));
                                }
                                if !self.types_compatible(&obj_type, &param_types[0]) {
                                    return Err(self.err(&format!(
                                        "method '{}': receiver type mismatch, expected {}, got {}",
                                        method, param_types[0], obj_type), line, col));
                                }
                                for (i, (pt, at)) in method_params.iter().zip(arg_types.iter()).enumerate() {
                                    if !self.types_compatible(at, pt) {
                                        return Err(self.err(&format!(
                                            "method '{}' argument {}: expected {}, got {}",
                                            method, i + 1, pt, at), line, col));
                                    }
                                }
                                return Ok(*ret_type.clone());
                            }
                        }
                        return Err(self.err(&format!("type '{}' has no method '{}'", tn, method), line, col));
                    }
                }

                // No methods found — check built-in methods
                match (&obj_type, method.as_str()) {
                    (MiType::Str, "len") => Ok(MiType::Int),
                    (MiType::Str, "is_empty") => Ok(MiType::Bool),
                    (MiType::Array(_), "len") => Ok(MiType::Int),
                    (MiType::Array(_), "is_empty") => Ok(MiType::Bool),
                    _ => {
                        if let Some(ref tn) = type_name {
                            Err(self.err(&format!("type '{}' has no method '{}'", tn, method), line, col))
                        } else {
                            Err(self.err(&format!("type {} has no methods", obj_type), line, col))
                        }
                    }
                }
            }

            Expr::Unary(op, rhs) => {
                let rhs_type = self.check_expr(rhs, line, col)?;
                match op {
                    UnOp::Neg => {
                        match &rhs_type {
                            MiType::Int | MiType::Float => Ok(rhs_type),
                            _ => Err(self.err(&format!("cannot negate type {}", rhs_type), line, col)),
                        }
                    }
                    UnOp::Not => {
                        match &rhs_type {
                            MiType::Bool => Ok(MiType::Bool),
                            _ => Err(self.err(&format!("cannot apply 'not' to type {}", rhs_type), line, col)),
                        }
                    }
                }
            }

            Expr::Binary(lhs, op, rhs) => {
                let lhs_type = self.check_expr(lhs, line, col)?;
                let rhs_type = self.check_expr(rhs, line, col)?;
                match op {
                    BinOp::Add => {
                        match (&lhs_type, &rhs_type) {
                            (MiType::Int, MiType::Int) => Ok(MiType::Int),
                            (MiType::Float, MiType::Float) => Ok(MiType::Float),
                            (MiType::Int, MiType::Float) | (MiType::Float, MiType::Int) => Ok(MiType::Float),
                            (MiType::Str, MiType::Str) => Ok(MiType::Str),
                            _ => Err(self.err(&format!(
                                "cannot apply + to {} and {}", lhs_type, rhs_type), line, col)),
                        }
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        match (&lhs_type, &rhs_type) {
                            (MiType::Int, MiType::Int) => Ok(MiType::Int),
                            (MiType::Float, MiType::Float) => Ok(MiType::Float),
                            (MiType::Int, MiType::Float) | (MiType::Float, MiType::Int) => Ok(MiType::Float),
                            _ => Err(self.err(&format!(
                                "cannot apply arithmetic to {} and {}", lhs_type, rhs_type), line, col)),
                        }
                    }
                    BinOp::Eq | BinOp::Neq => {
                        // Allow comparison of any two compatible types
                        if self.types_compatible(&lhs_type, &rhs_type) || lhs_type == rhs_type {
                            Ok(MiType::Bool)
                        } else {
                            Err(self.err(&format!(
                                "cannot compare {} and {}", lhs_type, rhs_type), line, col))
                        }
                    }
                    BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        match (&lhs_type, &rhs_type) {
                            (MiType::Int, MiType::Int)
                            | (MiType::Float, MiType::Float)
                            | (MiType::Int, MiType::Float)
                            | (MiType::Float, MiType::Int)
                            | (MiType::Str, MiType::Str) => Ok(MiType::Bool),
                            _ => Err(self.err(&format!(
                                "cannot compare {} and {}", lhs_type, rhs_type), line, col)),
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        match (&lhs_type, &rhs_type) {
                            (MiType::Bool, MiType::Bool) => Ok(MiType::Bool),
                            _ => Err(self.err(&format!(
                                "cannot apply logical operator to {} and {}", lhs_type, rhs_type), line, col)),
                        }
                    }
                }
            }

            Expr::Assign(target, _op, value) => {
                let val_type = self.check_expr(value, line, col)?;
                let target_type = self.check_expr(target, line, col)?;
                // Check target is an assignable expression
                match target.as_ref() {
                    Expr::Ident(name) => {
                        // Check mutability
                        let mut found_mut = false;
                        for scope in self.locals.iter().rev() {
                            if let Some((_, mutable)) = scope.get(name) {
                                if !mutable {
                                    return Err(self.err(&format!("cannot assign to immutable variable '{}'", name), line, col));
                                }
                                found_mut = true;
                                break;
                            }
                        }
                        if !found_mut {
                            if let Some((_, mutable)) = self.globals.get(name) {
                                if !mutable {
                                    return Err(self.err(&format!("cannot assign to immutable global '{}'", name), line, col));
                                }
                            } else {
                                return Err(self.err(&format!("undefined variable '{}' in assignment", name), line, col));
                            }
                        }
                    }
                    Expr::Field(_, _) | Expr::Index(_, _) => {} // OK
                    _ => return Err(self.err("invalid assignment target", line, col)),
                }
                if !self.types_compatible(&val_type, &target_type) {
                    return Err(self.err(&format!(
                        "cannot assign {} to variable of type {}", val_type, target_type), line, col));
                }
                Ok(val_type)
            }

            Expr::If(cond, then, else_opt) => {
                let cond_type = self.check_expr(cond, line, col)?;
                if !self.types_compatible(&cond_type, &MiType::Bool) {
                    return Err(self.err(&format!("if condition must be Bool, got {}", cond_type), line, col));
                }
                let then_type = self.check_expr(then, line, col)?;
                if let Some(el) = else_opt {
                    let else_type = self.check_expr(el, line, col)?;
                    // Unify then/else types
                    if then_type == else_type {
                        Ok(then_type)
                    } else if then_type == MiType::Unknown {
                        Ok(else_type)
                    } else if else_type == MiType::Unknown {
                        Ok(then_type)
                    } else if then_type == MiType::Nil {
                        Ok(else_type)
                    } else if else_type == MiType::Nil {
                        Ok(then_type)
                    } else {
                        Err(self.err(&format!(
                            "if branches have incompatible types: then={}, else={}", then_type, else_type), line, col))
                    }
                } else {
                    Ok(then_type)
                }
            }

            Expr::Match(expr, arms) => {
                let match_type = self.check_expr(expr, line, col)?;
                // Check patterns
                for arm in arms {
                    self.check_pattern(&arm.pattern, &match_type, line, col)?;
                }
                // Check arm bodies and unify types
                let mut arm_types = Vec::new();
                for arm in arms {
                    // Enter scope for pattern bindings
                    self.locals.push(HashMap::new());
                    self.bind_pattern(&arm.pattern, &match_type);
                    let body_type = self.check_expr(&arm.body, line, col)?;
                    self.locals.pop();
                    arm_types.push(body_type);
                }
                // Unify all arm types
                if arm_types.is_empty() {
                    Ok(MiType::Nil)
                } else {
                    let mut result = arm_types[0].clone();
                    for t in &arm_types[1..] {
                        if result != *t {
                            if result == MiType::Unknown {
                                result = t.clone();
                            } else if *t != MiType::Unknown && *t != MiType::Nil {
                                // Types differ — prefer first non-Nil non-Unknown
                                if result == MiType::Nil {
                                    result = t.clone();
                                }
                                // else keep result, don't error — lenient
                            }
                        }
                    }
                    Ok(result)
                }
            }

            Expr::Block(stmts) => {
                self.locals.push(HashMap::new());
                let mut last_type = MiType::Nil;
                for stmt in stmts {
                    last_type = self.check_stmt(stmt, line, col)?;
                }
                self.locals.pop();
                Ok(last_type)
            }

            Expr::Fn(params, return_type, body) => {
                self.locals.push(HashMap::new());
                for param in params {
                    let pt = param.type_expr.as_ref()
                        .map(|t| self.type_expr_to_type(t))
                        .unwrap_or(MiType::Unknown);
                    self.locals.last_mut().unwrap().insert(param.name.clone(), (pt, false));
                }
                let body_type = self.check_expr(body, line, col)?;
                let ret_type = return_type.as_ref()
                    .map(|t| self.type_expr_to_type(t))
                    .unwrap_or(body_type.clone());
                if let Some(rt) = return_type {
                    let declared_ret = self.type_expr_to_type(rt);
                    if !self.types_compatible(&body_type, &declared_ret) {
                        self.errors.push(self.err(&format!(
                            "anonymous function: expected return type {}, got {}",
                            declared_ret, body_type), line, col));
                    }
                }
                let param_types: Vec<MiType> = params.iter().map(|p| {
                    p.type_expr.as_ref().map(|t| self.type_expr_to_type(t))
                        .unwrap_or(MiType::Unknown)
                }).collect();
                self.locals.pop();
                Ok(MiType::Fn(param_types, Box::new(ret_type)))
            }

            Expr::Pipe(lhs, rhs) => {
                let lhs_type = self.check_expr(lhs, line, col)?;
                let rhs_type = self.check_expr(rhs, line, col)?;
                match &rhs_type {
                    MiType::Fn(params, ret_type) => {
                        if params.len() != 1 {
                            return Err(self.err(&format!(
                                "pipe requires single-argument function, got {} parameters", params.len()), line, col));
                        }
                        if !self.types_compatible(&lhs_type, &params[0]) {
                            return Err(self.err(&format!(
                                "pipe: expected argument type {}, got {}", params[0], lhs_type), line, col));
                        }
                        Ok(*ret_type.clone())
                    }
                    _ => Err(self.err(&format!(
                        "pipe requires function on right side, got {}", rhs_type), line, col)),
                }
            }

            Expr::Array(items) => {
                if items.is_empty() {
                    Ok(MiType::Array(Box::new(MiType::Unknown)))
                } else {
                    let mut elem_types = Vec::new();
                    for item in items {
                        let t = self.check_expr(item, line, col)?;
                        elem_types.push(t);
                    }
                    // Unify all element types
                    let mut unified = elem_types[0].clone();
                    for t in &elem_types[1..] {
                        if *t != unified {
                            if unified == MiType::Unknown {
                                unified = t.clone();
                            } else if *t != MiType::Unknown {
                                return Err(self.err(&format!(
                                    "array elements must have the same type, got {} and {}", unified, t), line, col));
                            }
                        }
                    }
                    Ok(MiType::Array(Box::new(unified)))
                }
            }

            Expr::Tuple(items) => {
                let types: Result<Vec<MiType>, String> =
                    items.iter().map(|i| self.check_expr(i, line, col)).collect();
                Ok(MiType::Tuple(types?))
            }

            Expr::StructLit(name, fields) => {
                // Clone the type definition first to avoid borrow conflicts
                let type_def = self.types.get(name).cloned();
                match type_def {
                    Some(MiType::Struct(_, def_fields)) => {
                        // Check all required fields are provided
                        for (df_name, df_type) in &def_fields {
                            match fields.iter().find(|(n, _)| n == df_name) {
                                Some((_, val)) => {
                                    let val_type = self.check_expr(val, line, col)?;
                                    if !self.types_compatible(&val_type, df_type) {
                                        return Err(self.err(&format!(
                                            "field '{}': expected {}, got {}", df_name, df_type, val_type), line, col));
                                    }
                                }
                                None => {
                                    return Err(self.err(&format!(
                                        "missing field '{}' in struct literal for '{}'", df_name, name), line, col));
                                }
                            }
                        }
                        // Warn about extra fields
                        for (fname, _) in fields {
                            if !def_fields.iter().any(|(n, _)| n == fname) {
                                self.errors.push(self.err(&format!(
                                    "struct '{}' has no field '{}'", name, fname), line, col));
                            }
                        }
                        Ok(MiType::Struct(name.clone(), def_fields.clone()))
                    }
                    Some(MiType::Enum(_, variants)) => {
                        // Enum variant construction via StructLit syntax
                        // e.g., Option { value: 42 } (single variant with same name as type)
                        // or just pass-through for enum types
                        if variants.len() == 1 && variants[0].0 == *name {
                            let (_, def_fields) = &variants[0];
                            for (fname, ftype) in def_fields {
                                match fields.iter().find(|(n, _)| n == fname) {
                                    Some((_, val)) => {
                                        let val_type = self.check_expr(val, line, col)?;
                                        if !self.types_compatible(&val_type, ftype) {
                                            return Err(self.err(&format!(
                                                "field '{}': expected {}, got {}", fname, ftype, val_type), line, col));
                                        }
                                    }
                                    None => {
                                        return Err(self.err(&format!(
                                            "missing field '{}' in variant '{}'", fname, name), line, col));
                                    }
                                }
                            }
                            Ok(MiType::Enum(name.clone(), variants.clone()))
                        } else {
                            Err(self.err(&format!(
                                "type '{}' is an enum with multiple variants; use a named variant", name), line, col))
                        }
                    }
                    _ => Err(self.err(&format!("undefined type '{}'", name), line, col)),
                }
            }

            Expr::Import(_, _) => Ok(MiType::Nil),
        }
    }

    /// Get the type name string for method lookup
    fn type_name(&self, t: &MiType) -> Option<String> {
        match t {
            MiType::Struct(n, _) => Some(n.clone()),
            MiType::Enum(n, _) => Some(n.clone()),
            MiType::Int => Some("Int".to_string()),
            MiType::Float => Some("Float".to_string()),
            MiType::Bool => Some("Bool".to_string()),
            MiType::Str => Some("Str".to_string()),
            MiType::Char => Some("Char".to_string()),
            MiType::Array(_) => Some("Array".to_string()),
            MiType::Tuple(_) => Some("Tuple".to_string()),
            MiType::Nil => Some("Nil".to_string()),
            _ => None,
        }
    }

    fn types_compatible(&self, a: &MiType, b: &MiType) -> bool {
        if a == b { return true; }
        if matches!(a, MiType::Unknown) || matches!(b, MiType::Unknown) { return true; }
        match (a, b) {
            (MiType::Int, MiType::Float) | (MiType::Float, MiType::Int) => true,
            (MiType::Array(ta), MiType::Array(tb)) => self.types_compatible(ta, tb),
            (MiType::Tuple(ta), MiType::Tuple(tb)) if ta.len() == tb.len() => {
                ta.iter().zip(tb.iter()).all(|(x, y)| self.types_compatible(x, y))
            }
            (MiType::Struct(na, _), MiType::Struct(nb, _)) => na == nb,
            (MiType::Enum(na, _), MiType::Enum(nb, _)) => na == nb,
            _ => false,
        }
    }

    fn err(&self, msg: &str, line: usize, col: usize) -> String {
        if line == 0 && col == 0 {
            msg.to_string()
        } else {
            format!("{} at {}:{}", msg, line, col)
        }
    }
}

impl std::fmt::Display for MiType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiType::Int => write!(f, "Int"),
            MiType::Float => write!(f, "Float"),
            MiType::Bool => write!(f, "Bool"),
            MiType::Str => write!(f, "Str"),
            MiType::Char => write!(f, "Char"),
            MiType::Nil => write!(f, "Nil"),
            MiType::Fn(params, ret) => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            MiType::Struct(name, _) => write!(f, "{}", name),
            MiType::Enum(name, _) => write!(f, "{}", name),
            MiType::Generic(name) => write!(f, "{}", name),
            MiType::TypeVar(n) => write!(f, "T{}", n),
            MiType::Array(elem) => write!(f, "[{}]", elem),
            MiType::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            MiType::Unknown => write!(f, "?"),
        }
    }
}
