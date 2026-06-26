use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Named(String),
    Generic(String, Vec<TypeExpr>),
    Fn(Vec<TypeExpr>, Box<TypeExpr>),
    Tuple(Vec<TypeExpr>),
    Infer,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, Le, Ge,
    And, Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Neg, Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Set, Add, Sub, Mul, Div,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String, Vec<Expr>),  // string with interpolation expressions
    Char(char),
    Nil,

    Ident(String),
    Field(Box<Expr>, String),        // expr.name
    Index(Box<Expr>, Box<Expr>),     // expr[index]
    Call(Box<Expr>, Vec<Expr>),      // expr(args)
    MethodCall(Box<Expr>, String, Vec<Expr>), // expr.method(args)

    Unary(UnOp, Box<Expr>),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Assign(Box<Expr>, AssignOp, Box<Expr>),

    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Match(Box<Expr>, Vec<MatchArm>),
    Block(Vec<Stmt>),

    Fn(Vec<FnParam>, Option<TypeExpr>, Box<Expr>),       // anonymous fn
    Pipe(Box<Expr>, Box<Expr>),                           // lhs |> rhs

    Array(Vec<Expr>),
    Tuple(Vec<Expr>),
    StructLit(String, Vec<(String, Expr)>),

    Import(String, Option<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wild,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Char(char),
    Ident(String),
    Binding(String, Box<Pattern>),
    Tuple(Vec<Pattern>),
    Struct(String, Vec<(String, Pattern)>),
    Or(Box<Pattern>, Box<Pattern>),
    Range(Box<Pattern>, Box<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    pub name: String,
    pub type_expr: Option<TypeExpr>,
    pub default: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Let {
        name: String,
        mutable: bool,
        type_expr: Option<TypeExpr>,
        value: Expr,
    },
    Fn {
        name: String,
        pub_visible: bool,
        params: Vec<FnParam>,
        return_type: Option<TypeExpr>,
        body: Expr,
    },
    Return(Option<Expr>),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    For(String, Expr, Vec<Stmt>),
    Loop(Vec<Stmt>),
    Break(Option<Expr>),
    Continue,
    Type {
        name: String,
        params: Vec<String>,
        variants: Vec<TypeVariant>,
    },
    Impl {
        type_name: String,
        methods: Vec<Stmt>,  // fn stmts
    },
    Trait {
        name: String,
        methods: Vec<TraitMethod>,
    },
    Extern {
        name: String,
        params: Vec<FnParam>,
        return_type: Option<TypeExpr>,
    },
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeVariant {
    pub name: String,
    pub fields: Vec<TypeField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeField {
    pub name: String,
    pub type_expr: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_type: Option<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

// Pretty printing for debugging
impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeExpr::Named(n) => write!(f, "{}", n),
            TypeExpr::Generic(n, args) => {
                write!(f, "{}[", n)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, "]")
            }
            TypeExpr::Fn(params, ret) => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            TypeExpr::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            TypeExpr::Infer => write!(f, "_"),
        }
    }
}
