use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn, Let, Mut, If, Else, Elif, Match, For, While, Loop,
    Break, Continue, Return, Type, Trait, Impl, Pub,
    Import, As, Var, Extern, True, False, Nil, In, And, Or, Not,
    Where, MutKw, Self_,

    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StrLiteral(String),
    CharLiteral(char),
    BoolLiteral(bool),

    // Identifiers
    Ident(String),

    // Operators
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, BangEq, Lt, Gt, LtEq, GtEq,
    PlusEq, MinusEq, StarEq, SlashEq,
    Arrow, FatArrow, Pipe, DotDot,
    AndAnd, OrOr, Bang,

    // Delimiters
    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,
    Comma, Colon, Semicolon, Dot,
    Hash, At,

    // Special
    Newline,
    Eof,
    Underscore,
}

impl Token {
    pub fn is_binop(&self) -> bool {
        matches!(self, Token::Plus | Token::Minus | Token::Star | Token::Slash
            | Token::Percent | Token::EqEq | Token::BangEq
            | Token::Lt | Token::Gt | Token::LtEq | Token::GtEq
            | Token::AndAnd | Token::OrOr | Token::And | Token::Or)
    }

    pub fn is_assign(&self) -> bool {
        matches!(self, Token::Eq | Token::PlusEq | Token::MinusEq
            | Token::StarEq | Token::SlashEq)
    }

    pub fn precedence(&self) -> u8 {
        match self {
            Token::OrOr | Token::Or => 1,
            Token::AndAnd | Token::And => 2,
            Token::EqEq | Token::BangEq => 3,
            Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => 4,
            Token::Plus | Token::Minus => 5,
            Token::Star | Token::Slash | Token::Percent => 6,
            Token::Pipe => 7,
            _ => 0,
        }
    }
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input,
            chars: input.chars().peekable(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn line(&self) -> usize { self.line }
    pub fn col(&self) -> usize { self.col }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += 1;
        Some(c)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' { return; }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) -> Option<()> {
        // We've already consumed /*
        loop {
            match self.advance()? {
                '*' if self.peek() == Some('/') => { self.advance(); return Some(()); }
                '\n' => {}  // already tracked in advance
                _ => {}
            }
        }
    }

    fn read_string(&mut self, delim: char) -> Result<String, String> {
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err("unterminated string literal".to_string()),
                Some(c) if c == delim => return Ok(s),
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('\'') => s.push('\''),
                        Some('{') => s.push('{'),  // escaping interpolation
                        Some(c) => { s.push('\\'); s.push(c); }
                        None => return Err("unterminated escape sequence".to_string()),
                    }
                }
                Some('{') => {
                    s.push('\x01'); // marker for interpolation start (skip the { itself)
                }
                Some(c) => s.push(c),
            }
        }
    }

    fn read_number(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);
        let mut is_float = false;
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '_' {
                s.push(c);
                self.advance();
            } else if c == '.' {
                // Check it's not .. (range operator)
                let mut chars = self.chars.clone();
                chars.next();
                if chars.peek() == Some(&'.') {
                    break;
                }
                is_float = true;
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s.retain(|c| c != '_');
        if is_float {
            Token::FloatLiteral(s.parse().unwrap_or(0.0))
        } else {
            Token::IntLiteral(s.parse().unwrap_or(0))
        }
    }

    fn read_ident(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "mut" => Token::MutKw,
            "if" => Token::If,
            "else" => Token::Else,
            "match" => Token::Match,
            "for" => Token::For,
            "while" => Token::While,
            "loop" => Token::Loop,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "return" => Token::Return,
            "type" => Token::Type,
            "trait" => Token::Trait,
            "impl" => Token::Impl,
            "pub" => Token::Pub,
            "import" => Token::Import,
            "as" => Token::As,
            "var" => Token::Var,
            "extern" => Token::Extern,
            "true" => Token::True,
            "false" => Token::False,
            "nil" => Token::Nil,
            "in" => Token::In,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            "elif" => Token::Elif,
            "self" => Token::Self_,
            "where" => Token::Where,
            _ => Token::Ident(s),
        }
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        loop {
            self.skip_whitespace();

            let c = match self.advance() {
                Some(c) => c,
                None => return Ok(Token::Eof),
            };

            return match c {
                // Newlines
                '\n' => {
                    // Skip consecutive newlines
                    while self.peek() == Some('\n') {
                        self.advance();
                    }
                    Ok(Token::Newline)
                }

                // Comments
                '/' if self.peek() == Some('/') => {
                    self.skip_line_comment();
                    continue;
                }
                '/' if self.peek() == Some('*') => {
                    self.advance(); // consume *
                    self.skip_block_comment().ok_or("unterminated block comment".to_string())?;
                    continue;
                }

                // Single-character operators
                '+' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::PlusEq) }
                    else { Ok(Token::Plus) }
                }
                '-' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::MinusEq) }
                    else if self.peek() == Some('>') { self.advance(); Ok(Token::Arrow) }
                    else { Ok(Token::Minus) }
                }
                '*' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::StarEq) }
                    else { Ok(Token::Star) }
                }
                '/' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::SlashEq) }
                    else { Ok(Token::Slash) }
                }
                '%' => Ok(Token::Percent),
                '=' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::EqEq) }
                    else if self.peek() == Some('>') { self.advance(); Ok(Token::FatArrow) }
                    else { Ok(Token::Eq) }
                }
                '!' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::BangEq) }
                    else { Ok(Token::Bang) }
                }
                '<' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::LtEq) }
                    else { Ok(Token::Lt) }
                }
                '>' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::GtEq) }
                    else { Ok(Token::Gt) }
                }
                '|' => {
                    if self.peek() == Some('|') { self.advance(); Ok(Token::OrOr) }
                    else if self.peek() == Some('>') { self.advance(); Ok(Token::Pipe) }
                    else { Ok(Token::Pipe) }
                }
                '&' => {
                    if self.peek() == Some('&') { self.advance(); Ok(Token::AndAnd) }
                    else { Ok(Token::And) }  // bitwise & used as logical
                }

                // Delimiters
                '(' => Ok(Token::LParen),
                ')' => Ok(Token::RParen),
                '{' => Ok(Token::LBrace),
                '}' => Ok(Token::RBrace),
                '[' => Ok(Token::LBracket),
                ']' => Ok(Token::RBracket),
                ',' => Ok(Token::Comma),
                ':' => {
                    if self.peek() == Some(':') {
                        return Err(":: operator not supported, use . or module.path".to_string());
                    }
                    Ok(Token::Colon)
                }
                ';' => Ok(Token::Semicolon),
                '.' => {
                    if self.peek() == Some('.') {
                        self.advance();
                        if self.peek() == Some('=') {
                            return Err("..= is not supported, use ..".to_string());
                        }
                        Ok(Token::DotDot)
                    } else {
                        Ok(Token::Dot)
                    }
                }
                '#' => Ok(Token::Hash),
                '@' => Ok(Token::At),

                // Literals
                '"' => {
                    let s = self.read_string('"')?;
                    Ok(Token::StrLiteral(s))
                }
                '\'' => {
                    match self.advance() {
                        Some(c) if c != '\\' => {
                            if self.peek() == Some('\'') {
                                self.advance();
                                Ok(Token::CharLiteral(c))
                            } else {
                                Err("unterminated char literal".to_string())
                            }
                        }
                        Some('\\') => {
                            let c = match self.advance() {
                                Some('n') => '\n',
                                Some('t') => '\t',
                                Some('r') => '\r',
                                Some('\\') => '\\',
                                Some('\'') => '\'',
                                Some(c) => return Err(format!("invalid escape \\{}", c)),
                                None => return Err("unterminated char literal".to_string()),
                            };
                            if self.peek() == Some('\'') {
                                self.advance();
                                Ok(Token::CharLiteral(c))
                            } else {
                                Err("unterminated char literal".to_string())
                            }
                        }
                        _ => Err("empty char literal".to_string()),
                    }
                }

                // Digits
                c if c.is_ascii_digit() => Ok(self.read_number(c)),
                // Identifiers
                c if c.is_alphabetic() => Ok(self.read_ident(c)),
                // Underscore (pattern wildcard)
                '_' => Ok(Token::Underscore),
                other => Err(format!("unexpected character '{}'", other)),
            };
        }
    }

    /// Collect all tokens, ignoring newlines. Newlines separate statements in block parsing.
    pub fn tokenize(&mut self) -> Result<Vec<(Token, usize, usize)>, String> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let here = (self.line, self.col);
            match &token {
                Token::Eof => {
                    tokens.push((token, self.line, self.col));
                    return Ok(tokens);
                }
                _ => {
                    tokens.push((token, here.0, here.1));
                }
            }
        }
    }
}
