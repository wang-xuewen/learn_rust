//! 语法分析器（Parser）：token 流 → AST。
//!
//! 采用 **Pratt 解析（precedence climbing / 优先级爬升）**：
//! 比「为每个优先级写一个函数」的递归下降更短，且加新运算符只要改一张优先级表。
//!
//! 语法（由松到紧）：
//!
//! ```text
//! stmt     := ident '=' expr | expr
//! expr     := 由 binding power 驱动的二元表达式
//! primary  := NUMBER | IDENT '(' args ')' | IDENT | '(' expr ')' | ('-'|'+') primary
//! args     := [ expr (',' expr)* ]
//! ```

use std::iter::Peekable;
use std::vec::IntoIter;

use crate::ast::{BinaryOp, Expr, Stmt, UnaryOp};
use crate::error::CalcError;
use crate::lexer::tokenize;
use crate::token::{Token, TokenKind};

/// 前缀运算符（一元 `+` / `-`）的操作数绑定力。
/// 取值在「乘法(3,4)」与「幂(7,6)」之间，于是 `-2 ^ 2` 解析成 `-(2 ^ 2)`。
const PREFIX_BINDING_POWER: u8 = 5;

/// 语法分析器：消费 token 流，产出 AST。
#[derive(Debug, Clone)]
pub struct Parser {
    /// `Peekable` 让我们「看一眼下一个 token」而不消耗它。
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    /// 用已切好的 token 列表构造分析器。
    #[must_use]
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into_iter().peekable(),
        }
    }

    /// 便捷入口：解析一整行输入，返回语句 AST。
    ///
    /// # Errors
    ///
    /// 词法或语法错误时返回 [`CalcError`]。
    pub fn parse(input: &str) -> Result<Stmt, CalcError> {
        Self::new(tokenize(input)?).parse_stmt()
    }

    /// 解析一条语句：赋值语句或表达式。
    ///
    /// # Errors
    ///
    /// 语法不合法时返回 [`CalcError`]。
    pub fn parse_stmt(&mut self) -> Result<Stmt, CalcError> {
        // 预读两个 token 判断是不是 `ident = ...`；
        // `Peekable` 只支持看 1 个，这里克隆一份迭代器做 2 格预读（Token: Clone，代价很小）。
        let assign_target = match (self.peek_nth(0), self.peek_nth(1)) {
            (
                Some(Token {
                    kind: TokenKind::Ident(name),
                    ..
                }),
                Some(Token {
                    kind: TokenKind::Assign,
                    ..
                }),
            ) => Some(name.clone()),
            _ => None,
        };

        if let Some(name) = assign_target {
            self.bump(); // ident
            self.bump(); // '='
            let expr = self.parse_expr_bp(0)?;
            self.expect_eof()?;
            return Ok(Stmt::Assign { name, expr });
        }

        let expr = self.parse_expr_bp(0)?;
        self.expect_eof()?;
        Ok(Stmt::Expr(expr))
    }

    /// Pratt 解析核心：`min_bp` 是「本层能接受的最低绑定力」。
    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, CalcError> {
        // 1) 先解析一个「前缀表达式」（primary）
        let token = match self.bump() {
            Some(token) => token,
            None => {
                return Err(CalcError::UnexpectedEof {
                    expected: "表达式".to_string(),
                })
            }
        };

        let mut lhs = match token.kind {
            TokenKind::Number(value) => Expr::Number(value),
            TokenKind::Ident(name) => {
                // 标识符后面紧跟 `(` 说明是函数调用
                if matches!(
                    self.peek(),
                    Some(Token {
                        kind: TokenKind::LParen,
                        ..
                    })
                ) {
                    self.bump();
                    Expr::Call {
                        name,
                        args: self.parse_args()?,
                    }
                } else {
                    Expr::Var(name)
                }
            }
            TokenKind::LParen => {
                let inner = self.parse_expr_bp(0)?;
                self.expect(TokenKind::RParen)?;
                inner
            }
            TokenKind::Minus => Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(self.parse_expr_bp(PREFIX_BINDING_POWER)?),
            },
            TokenKind::Plus => Expr::Unary {
                op: UnaryOp::Pos,
                expr: Box::new(self.parse_expr_bp(PREFIX_BINDING_POWER)?),
            },
            TokenKind::Eof => {
                return Err(CalcError::UnexpectedEof {
                    expected: "表达式".to_string(),
                })
            }
            other => {
                return Err(CalcError::UnexpectedToken {
                    expected: "表达式".to_string(),
                    found: other.to_string(),
                    pos: token.pos,
                })
            }
        };

        // 2) 再看后面能不能继续「吃掉」中缀运算符
        loop {
            let op = match self.peek() {
                Some(Token {
                    kind: TokenKind::Plus,
                    ..
                }) => BinaryOp::Add,
                Some(Token {
                    kind: TokenKind::Minus,
                    ..
                }) => BinaryOp::Sub,
                Some(Token {
                    kind: TokenKind::Star,
                    ..
                }) => BinaryOp::Mul,
                Some(Token {
                    kind: TokenKind::Slash,
                    ..
                }) => BinaryOp::Div,
                Some(Token {
                    kind: TokenKind::Percent,
                    ..
                }) => BinaryOp::Rem,
                Some(Token {
                    kind: TokenKind::Caret,
                    ..
                }) => BinaryOp::Pow,
                _ => break,
            };

            let (left_bp, right_bp) = op.binding_power();
            if left_bp < min_bp {
                break; // 优先级不够紧，交回上层处理
            }
            self.bump(); // 消费运算符
            let rhs = self.parse_expr_bp(right_bp)?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    /// 解析函数实参列表（调用方已消费掉左括号）。
    fn parse_args(&mut self) -> Result<Vec<Expr>, CalcError> {
        let mut args = Vec::new();
        if matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::RParen,
                ..
            })
        ) {
            self.bump();
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr_bp(0)?);
            match self.peek() {
                Some(Token {
                    kind: TokenKind::Comma,
                    ..
                }) => {
                    self.bump();
                }
                _ => break,
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }

    /// 偷看第 `n` 个 token（0 是下一个），不消耗。
    fn peek_nth(&self, n: usize) -> Option<Token> {
        self.tokens.clone().nth(n)
    }

    /// 偷看下一个 token，不消耗。
    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    /// 消费并返回下一个 token。
    fn bump(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    /// 期望下一个 token 是指定类型，否则报错。
    fn expect(&mut self, expected: TokenKind) -> Result<Token, CalcError> {
        match self.bump() {
            Some(token) if token.kind == expected => Ok(token),
            // 遇到哨兵说明「输入结束了」，报 UnexpectedEof 比报 UnexpectedToken 更准确
            Some(Token {
                kind: TokenKind::Eof,
                ..
            }) => Err(CalcError::UnexpectedEof {
                expected: expected.to_string(),
            }),
            Some(token) => Err(CalcError::UnexpectedToken {
                expected: expected.to_string(),
                found: token.kind.to_string(),
                pos: token.pos,
            }),
            None => Err(CalcError::UnexpectedEof {
                expected: expected.to_string(),
            }),
        }
    }

    /// 确认输入已经结束（只剩 `Eof`）。
    fn expect_eof(&mut self) -> Result<(), CalcError> {
        match self.bump() {
            None
            | Some(Token {
                kind: TokenKind::Eof,
                ..
            }) => Ok(()),
            Some(token) => Err(CalcError::UnexpectedToken {
                expected: TokenKind::Eof.to_string(),
                found: token.kind.to_string(),
                pos: token.pos,
            }),
        }
    }
}

/// 便捷入口：解析一行输入为语句 AST。
///
/// # Errors
///
/// 词法或语法错误时返回 [`CalcError`]。
pub fn parse(input: &str) -> Result<Stmt, CalcError> {
    Parser::parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 解析并回显，用字符串断言 AST 结构（含括号），比层层 match 直观。
    fn ast_of(input: &str) -> String {
        parse(input).expect("解析失败").to_string()
    }

    #[test]
    fn parse_single_number() {
        assert_eq!(ast_of("42"), "42");
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(ast_of("1 + 2 * 3"), "(1 + (2 * 3))");
        assert_eq!(ast_of("1 * 2 + 3"), "((1 * 2) + 3)");
    }

    #[test]
    fn addition_is_left_associative() {
        assert_eq!(ast_of("1 - 2 - 3"), "((1 - 2) - 3)");
        assert_eq!(ast_of("8 / 4 / 2"), "((8 / 4) / 2)");
    }

    #[test]
    fn power_is_right_associative() {
        assert_eq!(ast_of("2 ^ 3 ^ 2"), "(2 ^ (3 ^ 2))");
    }

    #[test]
    fn power_binds_tighter_than_unary_minus() {
        // 与 Python 一致：-2 ^ 2 == -(2 ^ 2) == -4
        assert_eq!(ast_of("-2 ^ 2"), "(-(2 ^ 2))");
        assert_eq!(ast_of("2 ^ -1"), "(2 ^ (-1))");
    }

    #[test]
    fn parentheses_override_precedence() {
        assert_eq!(ast_of("(1 + 2) * 3"), "((1 + 2) * 3)");
    }

    #[test]
    fn parse_unary_operators() {
        assert_eq!(ast_of("-x"), "(-x)");
        assert_eq!(ast_of("+3"), "(+3)");
        assert_eq!(ast_of("--1"), "(-(-1))");
    }

    #[test]
    fn parse_function_call_with_multiple_args() {
        assert_eq!(ast_of("pow(2, 10)"), "pow(2, 10)");
        assert_eq!(ast_of("sqrt(2)"), "sqrt(2)");
    }

    #[test]
    fn parse_assignment_stmt() {
        assert_eq!(
            parse("x = 1 + 2").expect("解析失败"),
            Stmt::Assign {
                name: "x".to_string(),
                expr: Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::Number(1.0)),
                    rhs: Box::new(Expr::Number(2.0)),
                },
            }
        );
    }

    #[test]
    fn empty_input_is_an_error() {
        let err = parse("").expect_err("空输入应报错");
        assert_eq!(
            err,
            CalcError::UnexpectedEof {
                expected: "表达式".to_string()
            }
        );
    }

    #[test]
    fn unbalanced_parenthesis_reports_position() {
        let err = parse("(1 + 2").expect_err("缺少右括号");
        assert!(matches!(err, CalcError::UnexpectedEof { .. }));
    }

    #[test]
    fn trailing_token_is_reported() {
        let err = parse("1 2").expect_err("多余 token");
        assert!(matches!(err, CalcError::UnexpectedToken { pos: 2, .. }));
    }

    #[test]
    fn unexpected_operator_reports_position() {
        let err = parse("1 + * 2").expect_err("运算符位置错误");
        assert!(matches!(err, CalcError::UnexpectedToken { pos: 4, .. }));
    }
}
