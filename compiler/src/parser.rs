use crate::ast::*;
use crate::lexer::{Lexer, Token};

pub struct Parser {
    tokens: Vec<(Token, usize, usize)>,
    pos: usize,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Result<Self, String> {
        let tokens = lexer.tokenize()?;
        Ok(Parser { tokens, pos: 0 })
    }

    /// Parse an expression from a string snippet (used for string interpolation).
    fn parse_expr_str(&self, text: &str) -> Result<Expr, String> {
        let mut lexer = Lexer::new(text);
        let tokens = lexer.tokenize()?;
        let mut p = Parser { tokens, pos: 0 };
        p.parse_expr()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].0
    }

    fn peek_n(&self, n: usize) -> &Token {
        if self.pos + n < self.tokens.len() {
            &self.tokens[self.pos + n].0
        } else {
            &Token::Eof
        }
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos].0;
        self.pos += 1;
        t
    }

    fn expect(&mut self, tok: Token) -> Result<(), String> {
        let line = self.tokens[self.pos].1;
        let col = self.tokens[self.pos].2;
        if self.peek() != &tok {
            return Err(format!("expected {:?}, got {:?} at {}:{}", tok, self.peek(), line, col));
        }
        self.advance();
        Ok(())
    }

    fn check(&self, tok: &Token) -> bool {
        self.peek() == tok
    }

    fn check_one_of(&self, toks: &[Token]) -> bool {
        toks.iter().any(|t| self.peek() == t)
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    fn line(&self) -> usize { if self.pos < self.tokens.len() { self.tokens[self.pos].1 } else { 0 } }
    fn col(&self) -> usize { if self.pos < self.tokens.len() { self.tokens[self.pos].2 } else { 0 } }

    // --- Top-level parsing ---

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.check(&Token::Eof) {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(Program { stmts })
    }

    fn skip_newlines(&mut self) {
        while self.check(&Token::Newline) {
            self.advance();
        }
    }

    fn parse_newlines(&mut self) {
        // Expect at least one newline or end of block
        // But we're lenient - skip any number
        self.skip_newlines();
    }

    // --- Statement parsing ---

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        self.skip_newlines();

        match self.peek().clone() {
            Token::Import => self.parse_import(),
            Token::Pub => {
                self.advance(); // consume pub
                match self.peek() {
                    Token::Fn => self.parse_fn(true),
                    _ => Err(format!("expected fn after pub at {}:{}", self.line(), self.col())),
                }
            }
            Token::Fn => self.parse_fn(false),
            Token::Let => self.parse_let(),
            Token::Var => {
                self.advance(); // var
                // var(Type) name = expr  or  var name = expr
                let type_expr = if self.eat(&Token::LParen) {
                    let te = Some(self.parse_type()?);
                    self.expect(Token::RParen)?;
                    te
                } else {
                    None
                };
                let name = match self.advance() {
                    Token::Ident(s) => s.clone(),
                    t => return Err(format!("expected variable name, got {:?}", t)),
                };
                self.expect(Token::Eq)?;
                let value = self.parse_expr()?;
                Ok(Stmt::Let { name, mutable: true, type_expr, value })
            }
            Token::Extern => {
                self.advance(); // extern
                self.expect(Token::Fn)?;
                let name = match self.advance() {
                    Token::Ident(s) => s.clone(),
                    t => return Err(format!("expected function name, got {:?}", t)),
                };
                self.expect(Token::LParen)?;
                let params = self.parse_fn_params()?;
                self.expect(Token::RParen)?;
                let return_type = if self.eat(&Token::Arrow) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                Ok(Stmt::Extern { name, params, return_type })
            }
            Token::Type => self.parse_type_def(),
            Token::Impl => self.parse_impl(),
            Token::Return => {
                self.advance();
                if self.check(&Token::Newline) || self.check(&Token::Eof) || self.check(&Token::RBrace) {
                    Ok(Stmt::Return(None))
                } else {
                    let expr = self.parse_expr()?;
                    Ok(Stmt::Return(Some(expr)))
                }
            }
            Token::If => {
                self.advance(); // 'if'
                let stmt = self.parse_if_tail(false)?;
                Ok(stmt)
            }
            Token::Elif => {
                self.advance(); // 'elif'
                let stmt = self.parse_if_tail(true)?;
                Ok(stmt)
            }
            Token::While => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block_stmts()?;
                self.expect(Token::RBrace)?;
                Ok(Stmt::While(cond, body))
            }
            Token::For => {
                self.advance();
                let name = match self.advance() {
                    Token::Ident(s) => s.clone(),
                    t => return Err(format!("expected identifier in for, got {:?}", t)),
                };
                self.expect(Token::In)?;
                let iter = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block_stmts()?;
                self.expect(Token::RBrace)?;
                Ok(Stmt::For(name, iter, body))
            }
            Token::Loop => {
                self.advance();
                self.expect(Token::LBrace)?;
                let body = self.parse_block_stmts()?;
                self.expect(Token::RBrace)?;
                Ok(Stmt::Loop(body))
            }
            Token::Break => {
                self.advance();
                let val = if self.check(&Token::Newline) || self.check(&Token::RBrace) || self.check(&Token::Eof) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                Ok(Stmt::Break(val))
            }
            Token::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
            _ => {
                let expr = self.parse_expr()?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_block_stmts(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    fn parse_import(&mut self) -> Result<Stmt, String> {
        self.advance(); // import
        let path = self.parse_ident_path()?;
        let alias = if self.eat(&Token::As) {
            match self.advance() {
                Token::Ident(s) => Some(s.clone()),
                t => return Err(format!("expected identifier after as, got {:?}", t)),
            }
        } else {
            None
        };
        Ok(Stmt::Expr(Expr::Import(path, alias)))
    }

    fn parse_ident_path(&mut self) -> Result<String, String> {
        let mut path = match self.advance() {
            Token::Ident(s) => s.clone(),
            t => return Err(format!("expected identifier, got {:?}", t)),
        };
        while self.eat(&Token::Dot) {
            match self.advance() {
                Token::Ident(s) => { path = format!("{}.{}", path, s); }
                t => return Err(format!("expected identifier after ., got {:?}", t)),
            }
        }
        Ok(path)
    }

    /// Parse the tail of an if/elif statement: cond { body } [else if/elif cond { body }]* [else { body }]
    fn parse_if_tail(&mut self, _is_elif: bool) -> Result<Stmt, String> {
        let cond = self.parse_expr()?;
        self.expect(Token::LBrace)?;
        let then = self.parse_block_stmts()?;
        self.expect(Token::RBrace)?;
        // Handle else / elif / else if continuation
        let else_opt = if self.eat(&Token::Else) {
            if self.eat(&Token::If) {
                // `else if cond { ... }`
                let nested = self.parse_if_tail(false)?;
                Some(vec![nested])
            } else if self.eat(&Token::Elif) {
                // `else elif cond { ... }` (unusual but valid)
                let nested = self.parse_if_tail(true)?;
                Some(vec![nested])
            } else {
                // `else { ... }`
                self.expect(Token::LBrace)?;
                let body = self.parse_block_stmts()?;
                self.expect(Token::RBrace)?;
                Some(body)
            }
        } else if self.eat(&Token::Elif) {
            // `elif cond { ... }` (Python-style without `else`)
            let nested = self.parse_if_tail(true)?;
            Some(vec![nested])
        } else {
            None
        };
        Ok(Stmt::If(cond, then, else_opt))
    }

    fn parse_fn(&mut self, pub_visible: bool) -> Result<Stmt, String> {
        self.advance(); // fn
        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            t => return Err(format!("expected fn name, got {:?}", t)),
        };
        self.expect(Token::LParen)?;
        let params = self.parse_fn_params()?;
        self.expect(Token::RParen)?;
        let return_type = if self.eat(&Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(Token::LBrace)?;
        let body_stmts = self.parse_block_stmts()?;
        self.expect(Token::RBrace)?;
        // Convert body block to expression (last expression is return value)
        let body = self.block_to_expr(body_stmts);
        Ok(Stmt::Fn { name, pub_visible, params, return_type, body })
    }

    fn parse_fn_params(&mut self) -> Result<Vec<FnParam>, String> {
        let mut params = Vec::new();
        if self.check(&Token::RParen) { return Ok(params); }
        params.push(self.parse_fn_param()?);
        while self.eat(&Token::Comma) {
            if self.check(&Token::RParen) { break; }
            params.push(self.parse_fn_param()?);
        }
        Ok(params)
    }

    fn parse_fn_param(&mut self) -> Result<FnParam, String> {
        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            t => return Err(format!("expected parameter name, got {:?}", t)),
        };
        let type_expr = if self.eat(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let default = if self.eat(&Token::Eq) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        Ok(FnParam { name, type_expr, default })
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.advance(); // let
        let mutable = self.eat(&Token::MutKw);
        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            t => return Err(format!("expected variable name, got {:?}", t)),
        };
        let type_expr = if self.eat(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let value;
        if self.eat(&Token::Eq) {
            value = self.parse_expr()?;
        } else {
            // Must come from a match or if expression
            // try to parse expression directly
            value = self.parse_expr()?;
        }
        Ok(Stmt::Let { name, mutable, type_expr, value })
    }

    fn parse_type_def(&mut self) -> Result<Stmt, String> {
        self.advance(); // type
        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            t => return Err(format!("expected type name, got {:?}", t)),
        };
        let params = if self.eat(&Token::LBracket) {
            let mut ps = Vec::new();
            match self.advance() {
                Token::Ident(s) => ps.push(s.clone()),
                t => return Err(format!("expected type parameter, got {:?}", t)),
            }
            while self.eat(&Token::Comma) {
                match self.advance() {
                    Token::Ident(s) => ps.push(s.clone()),
                    t => return Err(format!("expected type parameter, got {:?}", t)),
                }
            }
            self.expect(Token::RBracket)?;
            ps
        } else {
            vec![]
        };
        self.expect(Token::LBrace)?;
        self.skip_newlines();
        let mut variants = Vec::new();
        // Single variant with named fields: type Person { name: Str, age: Int }
        // Or multiple variants (enum): type Option[T] { Some(T), None }
        if !self.check(&Token::RBrace) {
            // Try to parse as struct fields first
            if self.peek_n(1) == &Token::Colon || self.peek_n(1) == &Token::Comma {
                // Struct-style
                let mut fields = Vec::new();
                loop {
                    self.skip_newlines();
                    if self.check(&Token::RBrace) { break; }
                    let fname = match self.advance() {
                        Token::Ident(s) => s.clone(),
                        t => return Err(format!("expected field name, got {:?}", t)),
                    };
                    self.expect(Token::Colon)?;
                    let ftype = self.parse_type()?;
                    fields.push(TypeField { name: fname, type_expr: ftype });
                    if !self.eat(&Token::Comma) { break; }
                }
                variants.push(TypeVariant { name: name.clone(), fields });
            } else {
                // Enum-style: type Option[T] { Some(value: T), None }
                loop {
                    self.skip_newlines();
                    if self.check(&Token::RBrace) { break; }
                    let vname = match self.advance() {
                        Token::Ident(s) => s.clone(),
                        t => return Err(format!("expected variant name, got {:?}", t)),
                    };
                    let mut fields = Vec::new();
                    if self.eat(&Token::LParen) {
                        loop {
                            if self.check(&Token::RParen) { break; }
                            let fname = match self.advance() {
                                Token::Ident(s) => s.clone(),
                                t => return Err(format!("expected field name, got {:?}", t)),
                            };
                            self.expect(Token::Colon)?;
                            let ftype = self.parse_type()?;
                            fields.push(TypeField { name: fname, type_expr: ftype });
                            if !self.eat(&Token::Comma) { break; }
                        }
                        self.expect(Token::RParen)?;
                    }
                    variants.push(TypeVariant { name: vname, fields });
                    if !self.eat(&Token::Comma) { break; }
                }
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Type { name, params, variants })
    }

    fn parse_impl(&mut self) -> Result<Stmt, String> {
        self.advance(); // impl
        let type_name = match self.advance() {
            Token::Ident(s) => s.clone(),
            t => return Err(format!("expected type name in impl, got {:?}", t)),
        };
        self.skip_newlines();
        // Check for "for" to implement a trait
        if self.eat(&Token::For) {
            let trait_name = type_name;
            let type_name = match self.advance() {
                Token::Ident(s) => s.clone(),
                t => return Err(format!("expected type name after for, got {:?}", t)),
            };
            self.expect(Token::LBrace)?;
            self.skip_newlines();
            let mut methods = Vec::new();
            while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
                if self.check(&Token::Fn) {
                    methods.push(self.parse_fn(false)?);
                } else {
                    return Err(format!("expected fn in impl, got {:?}", self.peek()));
                }
                self.skip_newlines();
            }
            self.expect(Token::RBrace)?;
            // Treat impl Trait for Type as statements
            // For now, just store the methods under the type name
            Ok(Stmt::Impl { type_name: format!("{} for {}", trait_name, type_name), methods })
        } else {
            self.expect(Token::LBrace)?;
            self.skip_newlines();
            let mut methods = Vec::new();
            while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
                if self.check(&Token::Fn) {
                    methods.push(self.parse_fn(false)?);
                } else {
                    return Err(format!("expected fn in impl, got {:?}", self.peek()));
                }
                self.skip_newlines();
            }
            self.expect(Token::RBrace)?;
            Ok(Stmt::Impl { type_name, methods })
        }
    }

    fn parse_trait(&mut self) -> Result<Stmt, String> {
        self.advance(); // trait
        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            t => return Err(format!("expected trait name, got {:?}", t)),
        };
        self.expect(Token::LBrace)?;
        self.skip_newlines();
        let mut methods = Vec::new();
        while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
            if self.check(&Token::Fn) {
                self.advance(); // fn
                let mname = match self.advance() {
                    Token::Ident(s) => s.clone(),
                    t => return Err(format!("expected method name, got {:?}", t)),
                };
                self.expect(Token::LParen)?;
                let params = self.parse_fn_params()?;
                self.expect(Token::RParen)?;
                let return_type = if self.eat(&Token::Arrow) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                methods.push(TraitMethod { name: mname, params, return_type });
                self.skip_newlines();
            } else {
                return Err(format!("expected fn in trait, got {:?}", self.peek()));
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Trait { name, methods })
    }

    // --- Expression parsing (Pratt parser) ---

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // Handle newlines after prefix
            self.skip_newlines();

            // Check for postfix operators: calls, fields, method calls
            match self.peek() {
                Token::LParen => {
                    // Function call
                    self.advance();
                    let args = self.parse_call_args()?;
                    self.expect(Token::RParen)?;
                    lhs = Expr::Call(Box::new(lhs), args);
                    continue;
                }
                Token::LBracket => {
                    // Index
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    lhs = Expr::Index(Box::new(lhs), Box::new(idx));
                    continue;
                }
                Token::Dot => {
                    self.advance();
                    let name = match self.advance() {
                        Token::Ident(s) => s.clone(),
                        t => return Err(format!("expected field/method name, got {:?}", t)),
                    };
                    // Check if it's a method call
                    if self.check(&Token::LParen) {
                        self.advance();
                        let args = self.parse_call_args()?;
                        self.expect(Token::RParen)?;
                        lhs = Expr::MethodCall(Box::new(lhs), name, args);
                    } else {
                        lhs = Expr::Field(Box::new(lhs), name);
                    }
                    continue;
                }
                Token::Pipe => {
                    self.advance();
                    let rhs = self.parse_expr_bp(0)?;
                    lhs = Expr::Pipe(Box::new(lhs), Box::new(rhs));
                    continue;
                }
                _ => {}
            }

            // Binary operator
            if let Some((l_bp, r_bp)) = self.infix_binding_power() {
                if l_bp < min_bp { break; }
                let op = self.advance().clone();
                let op = self.token_to_binop(&op);
                let rhs = self.parse_expr_bp(r_bp)?;
                lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs));
                continue;
            }

            // Assignment
            if self.peek().is_assign() {
                let op = self.advance().clone();
                let assign_op = self.token_to_assignop(&op);
                let rhs = self.parse_expr()?;
                lhs = Expr::Assign(Box::new(lhs), assign_op, Box::new(rhs));
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        self.skip_newlines();
        let tok = self.advance().clone();
        match tok {
            // Literals
            Token::IntLiteral(n) => Ok(Expr::Int(n)),
            Token::FloatLiteral(f) => Ok(Expr::Float(f)),
            Token::True => Ok(Expr::Bool(true)),
            Token::False => Ok(Expr::Bool(false)),
            Token::Nil => Ok(Expr::Nil),
            Token::StrLiteral(s) => {
                if s.contains('\x01') {
                    let parts: Vec<&str> = s.split('\x01').collect();
                    let prefix = parts[0].to_string();
                    let mut interp = Vec::new();
                    for i in 1..parts.len() {
                        let part = parts[i];
                        if let Some(pos) = part.find('}') {
                            let expr_text = &part[..pos];
                            let expr = if expr_text.is_empty() {
                                Expr::Nil
                            } else {
                                self.parse_expr_str(expr_text)?
                            };
                            interp.push(expr);
                            let suffix = part[pos+1..].to_string();
                            if !suffix.is_empty() {
                                interp.push(Expr::Str(suffix, vec![]));
                            }
                        }
                    }
                    Ok(Expr::Str(prefix, interp))
                } else {
                    Ok(Expr::Str(s, vec![]))
                }
            }
            Token::CharLiteral(c) => Ok(Expr::Char(c)),

            // Unary operators
            Token::Minus => {
                let rhs = self.parse_expr_bp(7)?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(rhs)))
            }
            Token::Bang => {
                let rhs = self.parse_expr_bp(7)?;
                Ok(Expr::Unary(UnOp::Not, Box::new(rhs)))
            }

            // Groups and blocks
            Token::LParen => {
                if self.check(&Token::RParen) {
                    self.advance();
                    return Ok(Expr::Tuple(vec![]));
                }
                let first = self.parse_expr()?;
                if self.eat(&Token::Comma) {
                    // Tuple
                    let mut exprs = vec![first];
                    loop {
                        if self.check(&Token::RParen) { break; }
                        exprs.push(self.parse_expr()?);
                        if !self.eat(&Token::Comma) { break; }
                    }
                    self.expect(Token::RParen)?;
                    if exprs.len() == 2 && !self.check(&Token::Arrow) {
                        // Could be a tuple or function param - treat as tuple if not followed by ->
                        Ok(Expr::Tuple(exprs))
                    } else {
                        Ok(Expr::Tuple(exprs))
                    }
                } else if self.eat(&Token::Arrow) {
                    // Anonymous function: (x: Int) -> x + 1
                    // First item was actually a param
                    let param_name = match &first {
                        Expr::Ident(s) => s.clone(),
                        _ => return Err("expected parameter name".to_string()),
                    };
                    let param_type = if self.check(&Token::Colon) {
                        // Already consumed, rebuild
                        self.parse_fn_param_in_parens(param_name)?
                    } else {
                        FnParam { name: param_name, type_expr: None, default: None }
                    };
                    let mut params = vec![param_type];
                    self.expect(Token::RParen)?;
                    let return_type = None;
                    let body = self.parse_expr()?;
                    Ok(Expr::Fn(params, return_type, Box::new(body)))
                } else {
                    self.expect(Token::RParen)?;
                    Ok(first)
                }
            }
            Token::LBrace => {
                let stmts = self.parse_block_stmts()?;
                self.expect(Token::RBrace)?;
                Ok(Expr::Block(stmts))
            }
            Token::LBracket => {
                let mut exprs = Vec::new();
                if !self.check(&Token::RBracket) {
                    loop {
                        exprs.push(self.parse_expr()?);
                        if !self.eat(&Token::Comma) { break; }
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(exprs))
            }

            // Anonymous function: fn(x) { x * 2 }
            Token::Fn => {
                self.expect(Token::LParen)?;
                let params = self.parse_fn_params()?;
                self.expect(Token::RParen)?;
                let return_type = if self.eat(&Token::Arrow) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                if self.check(&Token::LBrace) {
                    self.advance();
                    let stmts = self.parse_block_stmts()?;
                    self.expect(Token::RBrace)?;
                    let body = self.block_to_expr(stmts);
                    Ok(Expr::Fn(params, return_type, Box::new(body)))
                } else {
                    let body = self.parse_expr()?;
                    Ok(Expr::Fn(params, return_type, Box::new(body)))
                }
            }

            // If expression
            Token::If => {
                let cond = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                let then = self.parse_block_stmts()?;
                self.expect(Token::RBrace)?;
                let else_expr = if self.eat(&Token::Else) {
                    if self.eat(&Token::If) {
                        // else if - parse as nested if
                        let cond = self.parse_expr()?;
                        self.expect(Token::LBrace)?;
                        let then = self.parse_block_stmts()?;
                        self.expect(Token::RBrace)?;
                        let else_inner = if self.eat(&Token::Else) {
                            if self.check(&Token::If) {
                                // another else if
                                self.pos -= 1; // back up
                                None // handled by recursion
                            } else {
                                self.expect(Token::LBrace)?;
                                let body = self.parse_block_stmts()?;
                                self.expect(Token::RBrace)?;
                                Some(Box::new(Expr::Block(body)))
                            }
                        } else { None };
                        Some(Box::new(Expr::If(Box::new(cond),
                            Box::new(self.block_to_expr(then)),
                            else_inner)))
                    } else {
                        self.expect(Token::LBrace)?;
                        let body = self.parse_block_stmts()?;
                        self.expect(Token::RBrace)?;
                        Some(Box::new(Expr::Block(body)))
                    }
                } else {
                    None
                };
                Ok(Expr::If(Box::new(cond),
                    Box::new(self.block_to_expr(then)),
                    else_expr))
            }

            // Match expression
            Token::Match => {
                let expr = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                self.skip_newlines();
                let mut arms = Vec::new();
                while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
                    arms.push(self.parse_match_arm()?);
                    self.skip_newlines();
                }
                self.expect(Token::RBrace)?;
                Ok(Expr::Match(Box::new(expr), arms))
            }

            // Identifier or type constructor
            Token::Ident(s) => {
                // Check for struct literal: Person { name: "Midori", age: 1 }
                if self.check(&Token::LBrace) && !self.check(&Token::Eof)
                    && self.peek_n(1) != &Token::RBrace && self.peek_n(1) != &Token::Newline
                    && self.peek_n(2) == &Token::Colon {
                    // Could be struct literal - check if next token is field:value
                    self.advance(); // consume {
                    let mut fields = Vec::new();
                    loop {
                        self.skip_newlines();
                        if self.check(&Token::RBrace) { break; }
                        let fname = match self.advance() {
                            Token::Ident(n) => n.clone(),
                            t => return Err(format!("expected field name, got {:?}", t)),
                        };
                        self.expect(Token::Colon)?;
                        let val = self.parse_expr()?;
                        fields.push((fname, val));
                        if !self.eat(&Token::Comma) { break; }
                    }
                    self.expect(Token::RBrace)?;
                    Ok(Expr::StructLit(s, fields))
                } else {
                    Ok(Expr::Ident(s))
                }
            }

            Token::Import => {
                let path = self.parse_ident_path()?;
                let alias = if self.eat(&Token::As) {
                    match self.advance() {
                        Token::Ident(s) => Some(s.clone()),
                        t => return Err(format!("expected identifier after as, got {:?}", t)),
                    }
                } else {
                    None
                };
                Ok(Expr::Import(path, alias))
            }

            Token::Eof => Err("unexpected end of file".to_string()),
            t => Err(format!("unexpected token {:?} at {}:{}", t, self.line(), self.col())),
        }
    }

    fn parse_fn_param_in_parens(&mut self, name: String) -> Result<FnParam, String> {
        // We've already consumed the : after the name
        let type_expr = Some(self.parse_type()?);
        let default = if self.eat(&Token::Eq) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        Ok(FnParam { name, type_expr, default })
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if self.check(&Token::RParen) { return Ok(args); }
        loop {
            args.push(self.parse_expr()?);
            if !self.eat(&Token::Comma) { break; }
            if self.check(&Token::RParen) { break; }
        }
        Ok(args)
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, String> {
        let pattern = self.parse_pattern()?;
        if self.eat(&Token::If) {
            // Guard clause - skip for now but consume
            // For now just parse and discard the guard expression
            // This is a simplification
            self.parse_expr()?;
        }
        match self.peek() {
            Token::FatArrow | Token::Arrow => {
                self.advance();
            }
            _ => {
                // Try => as arrow
                self.expect(Token::FatArrow)?;
            }
        }
        let body = self.parse_expr()?;
        Ok(MatchArm { pattern, body: Box::new(body) })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        self.skip_newlines();
        match self.peek().clone() {
            Token::IntLiteral(n) => { self.advance(); Ok(Pattern::Int(n)) }
            Token::FloatLiteral(f) => { self.advance(); Ok(Pattern::Float(f)) }
            Token::True | Token::False => {
                let b = matches!(self.advance(), Token::True);
                Ok(Pattern::Bool(b))
            }
            Token::StrLiteral(s) => { self.advance(); Ok(Pattern::Str(s)) }
            Token::CharLiteral(c) => { self.advance(); Ok(Pattern::Char(c)) }
            Token::Nil => { self.advance(); Ok(Pattern::Wild) }
            Token::Ident(s) => {
                let name = match self.advance() { Token::Ident(s) => s.clone(), _ => unreachable!() };
                if self.check(&Token::LParen) {
                    // Struct or enum variant pattern
                    self.advance();
                    let mut patterns = Vec::new();
                    loop {
                        if self.check(&Token::RParen) { break; }
                        patterns.push(self.parse_pattern()?);
                        if !self.eat(&Token::Comma) { break; }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Pattern::Tuple(patterns))
                } else if self.check(&Token::LBrace) {
                    // Struct pattern with named fields
                    self.advance();
                    let mut fields = Vec::new();
                    loop {
                        self.skip_newlines();
                        if self.check(&Token::RBrace) { break; }
                        let fname = match self.advance() {
                            Token::Ident(n) => n.clone(),
                            t => return Err(format!("expected field name, got {:?}", t)),
                        };
                        self.expect(Token::Colon)?;
                        let pat = self.parse_pattern()?;
                        fields.push((fname, pat));
                        if !self.eat(&Token::Comma) { break; }
                    }
                    self.expect(Token::RBrace)?;
                    Ok(Pattern::Struct(name, fields))
                } else if name.as_str() == "_" {
                    Ok(Pattern::Wild)
                } else {
                    // Binding pattern or wildcard
                    Ok(Pattern::Ident(name))
                }
            }
            Token::Underscore => { self.advance(); Ok(Pattern::Wild) }
            _ => Err(format!("unexpected token in pattern {:?}", self.peek())),
        }
    }

    fn parse_type(&mut self) -> Result<TypeExpr, String> {
        match self.peek() {
            Token::LParen => {
                self.advance();
                let mut types = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        types.push(self.parse_type()?);
                        if !self.eat(&Token::Comma) { break; }
                    }
                }
                self.expect(Token::RParen)?;
                if self.eat(&Token::Arrow) {
                    // Function type
                    let ret = self.parse_type()?;
                    Ok(TypeExpr::Fn(types, Box::new(ret)))
                } else if types.len() == 1 {
                    Ok(types.into_iter().next().unwrap())
                } else {
                    Ok(TypeExpr::Tuple(types))
                }
            }
            Token::Ident(s) => {
                let name = match self.advance() { Token::Ident(s) => s.clone(), _ => unreachable!() };
                if self.check(&Token::LBracket) {
                    // Generic type
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        args.push(self.parse_type()?);
                        if !self.eat(&Token::Comma) { break; }
                    }
                    self.expect(Token::RBracket)?;
                    Ok(TypeExpr::Generic(name, args))
                } else {
                    Ok(TypeExpr::Named(name))
                }
            }
            Token::Underscore => {
                self.advance();
                Ok(TypeExpr::Infer)
            }
            t => Err(format!("expected type, got {:?}", t)),
        }
    }

    fn infix_binding_power(&self) -> Option<(u8, u8)> {
        let prec = self.peek().precedence();
        if prec == 0 { None } else { Some((prec, prec + 1)) }
    }

    fn token_to_binop(&self, tok: &Token) -> BinOp {
        match tok {
            Token::Plus => BinOp::Add,
            Token::Minus => BinOp::Sub,
            Token::Star => BinOp::Mul,
            Token::Slash => BinOp::Div,
            Token::Percent => BinOp::Mod,
            Token::EqEq => BinOp::Eq,
            Token::BangEq => BinOp::Neq,
            Token::Lt => BinOp::Lt,
            Token::Gt => BinOp::Gt,
            Token::LtEq => BinOp::Le,
            Token::GtEq => BinOp::Ge,
            Token::AndAnd | Token::And => BinOp::And,
            Token::OrOr | Token::Or => BinOp::Or,
            _ => panic!("unexpected operator {:?}", tok),
        }
    }

    fn token_to_assignop(&self, tok: &Token) -> AssignOp {
        match tok {
            Token::Eq => AssignOp::Set,
            Token::PlusEq => AssignOp::Add,
            Token::MinusEq => AssignOp::Sub,
            Token::StarEq => AssignOp::Mul,
            Token::SlashEq => AssignOp::Div,
            _ => panic!("unexpected assign op {:?}", tok),
        }
    }

    fn block_to_expr(&self, stmts: Vec<Stmt>) -> Expr {
        if stmts.is_empty() {
            return Expr::Nil;
        }
        let mut stmts = stmts;
        let last = stmts.pop().unwrap();
        match &last {
            Stmt::Expr(e) => {
                if stmts.is_empty() {
                    return e.clone();
                }
                stmts.push(last);
                Expr::Block(stmts)
            }
            // Convert statement-level control flow to expressions for implicit returns
            Stmt::If(cond, then_body, else_body) if stmts.is_empty() => {
                Expr::If(
                    Box::new(cond.clone()),
                    Box::new(Expr::Block(then_body.clone())),
                    else_body.as_ref().map(|b| {
                        // Recursively convert else-body to expression (handles elif chains)
                        Box::new(self.block_to_expr(b.clone()))
                    }),
                )
            }
            Stmt::While(..) | Stmt::For(..) | Stmt::Loop(..) if stmts.is_empty() => {
                Expr::Block(vec![last])
            }
            _ => {
                stmts.push(last);
                Expr::Block(stmts)
            }
        }
    }
}
