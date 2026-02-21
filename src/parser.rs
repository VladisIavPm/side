use crate::lexer::Token;
use crate::ast::*;
use crate::value::Value;
use crate::error::ParseError;
use std::mem::discriminant;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    current_line: usize, // добавляем номер текущей строки
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            current_line: 1, // начинаем с 1
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            // здесь можно увеличивать current_line при встрече перевода строки,
            // но пока оставим как есть
            Some(token)
        } else {
            None
        }
    }

    fn consume(&mut self) -> Result<Token, ParseError> {
        self.next().ok_or(ParseError::UnexpectedToken {
            line: self.current_line,
            token: Token::Error,
        })
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        let token = self.consume()?;
        if discriminant(&token) == discriminant(&expected) {
            Ok(())
        } else {
            Err(ParseError::Expected {
                line: self.current_line,
                expected: format!("{:?}", expected),
                found: token,
            })
        }
    }

    fn peek_is(&self, token: &Token) -> bool {
        self.peek().map_or(false, |t| discriminant(t) == discriminant(token))
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        match self.peek() {
            Some(Token::Link) => Ok(Item::Decl(self.parse_link()?)),
            Some(Token::Proc) => Ok(Item::Decl(self.parse_proc()?)),
            Some(Token::Form) => Ok(Item::Decl(self.parse_form()?)),
            _ => Ok(Item::Stmt(self.parse_stmt()?)),
        }
    }

    fn parse_link(&mut self) -> Result<Decl, ParseError> {
        self.expect(Token::Link)?;
        let path = match self.consume()? {
            Token::String(s) => s,
            tok => {
                return Err(ParseError::Expected {
                    line: self.current_line,
                    expected: "string".to_string(),
                    found: tok,
                })
            }
        };
        let alias = if self.peek_is(&Token::As) {
            self.next();
            match self.consume()? {
                Token::Identifier(name) => Some(name),
                tok => {
                    return Err(ParseError::Expected {
                        line: self.current_line,
                        expected: "identifier".to_string(),
                        found: tok,
                    })
                }
            }
        } else {
            None
        };
        Ok(Decl::Link { path, alias })
    }

    fn parse_proc(&mut self) -> Result<Decl, ParseError> {
        self.expect(Token::Proc)?;
        let name = match self.consume()? {
            Token::Identifier(s) => s,
            tok => {
                return Err(ParseError::Expected {
                    line: self.current_line,
                    expected: "identifier".to_string(),
                    found: tok,
                })
            }
        };
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if !self.peek_is(&Token::RParen) {
            loop {
                match self.consume()? {
                    Token::Identifier(p) => params.push(p),
                    tok => {
                        return Err(ParseError::Expected {
                            line: self.current_line,
                            expected: "identifier".to_string(),
                            found: tok,
                        })
                    }
                }
                if self.peek_is(&Token::RParen) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
        }
        self.expect(Token::RParen)?;
        self.expect(Token::Start)?;
        let body = self.parse_block()?;
        self.expect(Token::End)?;
        Ok(Decl::Proc { name, params, body })
    }

    fn parse_form(&mut self) -> Result<Decl, ParseError> {
        self.expect(Token::Form)?;
        let name = match self.consume()? {
            Token::Identifier(s) => s,
            tok => {
                return Err(ParseError::Expected {
                    line: self.current_line,
                    expected: "identifier".to_string(),
                    found: tok,
                })
            }
        };
        self.expect(Token::Start)?;
        let mut fields = Vec::new();
        while !self.peek_is(&Token::End) {
            fields.push(self.parse_field_decl()?);
        }
        self.expect(Token::End)?;
        Ok(Decl::Form { name, fields })
    }

    fn parse_field_decl(&mut self) -> Result<FieldDecl, ParseError> {
        let mutable = match self.peek() {
            Some(Token::Set) => {
                self.next();
                true
            }
            Some(Token::Fix) => {
                self.next();
                false
            }
            _ => {
                return Err(ParseError::Expected {
                    line: self.current_line,
                    expected: "set or fix".to_string(),
                    found: self.peek().unwrap().clone(),
                })
            }
        };
        let name = match self.consume()? {
            Token::Identifier(s) => s,
            tok => {
                return Err(ParseError::Expected {
                    line: self.current_line,
                    expected: "identifier".to_string(),
                    found: tok,
                })
            }
        };
        let initial = if self.peek_is(&Token::Assign) {
            self.next();
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(FieldDecl { name, mutable, initial })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek() {
            Some(Token::Set) => self.parse_set_stmt(true),
            Some(Token::Fix) => self.parse_set_stmt(false),
            Some(Token::Log) => self.parse_log_stmt(),
            Some(Token::Check) => self.parse_if_stmt(),
            Some(Token::Loop) => self.parse_loop_stmt(),
            Some(Token::Stop) => self.parse_stop_stmt(),
            Some(Token::Wait) => self.parse_wait_stmt(),
            Some(Token::Give) => self.parse_return_stmt(),
            Some(Token::Trap) => self.parse_trap_stmt(),
            _ => self.parse_assign_or_expr_stmt(),
        }
    }

    fn parse_set_stmt(&mut self, mutable: bool) -> Result<Stmt, ParseError> {
        self.next();
        let name = match self.consume()? {
            Token::Identifier(s) => s,
            tok => {
                return Err(ParseError::Expected {
                    line: self.current_line,
                    expected: "identifier".to_string(),
                    found: tok,
                })
            }
        };
        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;
        Ok(Stmt::Set { name, value, mutable })
    }

    fn parse_log_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Log)?;
        let expr = self.parse_expression()?;
        Ok(Stmt::Log(expr))
    }

    fn parse_block_until(&mut self, terminators: &[Token]) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while !self.is_at_end() {
            if let Some(token) = self.peek() {
                if terminators.iter().any(|t| discriminant(token) == discriminant(t)) {
                    break;
                }
            }
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Check)?;
        let condition = self.parse_expression()?;
        self.expect(Token::Start)?;
        let then_block = self.parse_block_until(&[Token::Else, Token::End])?;
        let else_block = if self.peek_is(&Token::Else) {
            self.next();
            self.expect(Token::Start)?;
            Some(self.parse_block_until(&[Token::End])?)
        } else {
            None
        };
        self.expect(Token::End)?;
        Ok(Stmt::If { condition, then_block, else_block })
    }

    fn parse_loop_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Loop)?;
        let condition = self.parse_expression()?;
        self.expect(Token::Start)?;
        let body = self.parse_block()?;
        self.expect(Token::End)?;
        Ok(Stmt::Loop { condition, body })
    }

    fn parse_stop_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Stop)?;
        Ok(Stmt::Break)
    }

    fn parse_wait_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Wait)?;
        let seconds = self.parse_expression()?;
        Ok(Stmt::Wait(seconds))
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Give)?;
        let value = if !self.peek_is(&Token::End) && !self.peek_is(&Token::Else) && !self.peek_is(&Token::Start) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Stmt::Return(value))
    }

    fn parse_trap_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Trap)?;
        self.expect(Token::Start)?;
        let try_block = self.parse_block_until(&[Token::Else, Token::End])?;
        self.expect(Token::Else)?;
        self.expect(Token::Start)?;
        let catch_block = self.parse_block_until(&[Token::End])?;
        self.expect(Token::End)?;
        Ok(Stmt::Trap { try_block, catch_block })
    }

    fn parse_assign_or_expr_stmt(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expression()?;
        if self.peek_is(&Token::Assign) {
            self.next();
            let value = self.parse_expression()?;
            match expr {
                Expr::Variable(_) | Expr::Field { .. } | Expr::Index { .. } => {
                    Ok(Stmt::Assign { target: expr, value })
                }
                _ => Err(ParseError::InvalidLValue { line: self.current_line }),
            }
        } else {
            Ok(Stmt::ExprStmt(expr))
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while !self.peek_is(&Token::End) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    // Выражения --------------------------------------------------------------

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;
        while self.peek_is(&Token::Or) {
            self.next();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        while self.peek_is(&Token::And) {
            self.next();
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;
        while let Some(op) = self.peek_comparison_op() {
            self.next();
            let right = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn peek_comparison_op(&self) -> Option<BinOp> {
        match self.peek() {
            Some(Token::Equal) => Some(BinOp::Eq),
            Some(Token::NotEqual) => Some(BinOp::Ne),
            Some(Token::Less) => Some(BinOp::Lt),
            Some(Token::LessEqual) => Some(BinOp::Le),
            Some(Token::Greater) => Some(BinOp::Gt),
            Some(Token::GreaterEqual) => Some(BinOp::Ge),
            _ => None,
        }
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;
        while let Some(op) = self.peek_term_op() {
            self.next();
            let right = self.parse_factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn peek_term_op(&self) -> Option<BinOp> {
        match self.peek() {
            Some(Token::Plus) => Some(BinOp::Add),
            Some(Token::Minus) => Some(BinOp::Sub),
            _ => None,
        }
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        while let Some(op) = self.peek_factor_op() {
            self.next();
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn peek_factor_op(&self) -> Option<BinOp> {
        match self.peek() {
            Some(Token::Star) => Some(BinOp::Mul),
            Some(Token::Slash) => Some(BinOp::Div),
            Some(Token::Percent) => Some(BinOp::Rem),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.peek_is(&Token::Not) {
            self.next();
            let expr = self.parse_unary()?;
            Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
            })
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.consume()?;
        match token {
            Token::Whole(n) => Ok(Expr::Literal(Value::Whole(n))),
            Token::Fraction(f) => Ok(Expr::Literal(Value::Fraction(f))),
            Token::String(s) => Ok(Expr::Literal(Value::String(s))),
            Token::True => Ok(Expr::Literal(Value::Bool(true))),
            Token::False => Ok(Expr::Literal(Value::Bool(false))),
            Token::None => Ok(Expr::Literal(Value::None)),
            Token::LBracket => {
                let mut elements = Vec::new();
                if !self.peek_is(&Token::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if self.peek_is(&Token::RBracket) {
                            break;
                        }
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::List(elements))
            }
            Token::Identifier(name) => self.parse_suffix(Expr::Variable(name)),
            Token::LParen => {
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                self.parse_suffix(expr)
            }
            Token::New => {
                let name = match self.consume()? {
                    Token::Identifier(s) => s,
                    tok => {
                        return Err(ParseError::Expected {
                            line: self.current_line,
                            expected: "identifier".to_string(),
                            found: tok,
                        })
                    }
                };
                let expr = Expr::New { name };
                self.parse_suffix(expr)
            }
            Token::Entry => {
                let prompt = if matches!(self.peek(), Some(Token::String(_))) {
                    match self.consume()? {
                        Token::String(s) => Some(Box::new(Expr::Literal(Value::String(s)))),
                        _ => unreachable!(),
                    }
                } else {
                    None
                };
                let expr = Expr::Entry { prompt };
                self.parse_suffix(expr)
            }
            _ => Err(ParseError::UnexpectedToken {
                line: self.current_line,
                token,
            }),
        }
    }

    fn parse_suffix(&mut self, mut expr: Expr) -> Result<Expr, ParseError> {
        loop {
            if self.peek_is(&Token::LParen) {
                self.next();
                let mut args = Vec::new();
                if !self.peek_is(&Token::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if self.peek_is(&Token::RParen) {
                            break;
                        }
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RParen)?;
                match expr {
                    Expr::Variable(name) => {
                        expr = Expr::Call { name, args };
                    }
                    _ => {
                        return Err(ParseError::Expected {
                            line: self.current_line,
                            expected: "function name".to_string(),
                            found: Token::LParen,
                        })
                    }
                }
            } else if self.peek_is(&Token::Dot) {
                self.next();
                let field = match self.consume()? {
                    Token::Identifier(s) => s,
                    tok => {
                        return Err(ParseError::Expected {
                            line: self.current_line,
                            expected: "identifier".to_string(),
                            found: tok,
                        })
                    }
                };
                expr = Expr::Field {
                    object: Box::new(expr),
                    field,
                };
            } else if self.peek_is(&Token::LBracket) {
                self.next();
                let index = self.parse_expression()?;
                self.expect(Token::RBracket)?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }
}