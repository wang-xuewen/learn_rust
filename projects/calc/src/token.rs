//! Token（词法单元）的定义。
//!
//! 词法分析器把输入的 `&str` 切成一个个带类型的 token，
//! 语法分析器只认识 token，不认识字符 —— 这样两层职责清晰，
//! 也让「空白、注释」这类噪音提前消失。

use std::fmt;

/// Token 的类型。
///
/// 注意 Rust 的枚举可以携带数据（`Number(f64)`、`Ident(String)`），
/// 这和 C 的整数枚举完全不是一个东西 —— 它是「和类型 / sum type」。
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// 数字字面量，如 `3.14`、`1e6`。
    Number(f64),
    /// 标识符：变量名或函数名，如 `pi`、`sqrt`。
    Ident(String),

    /// `+`
    Plus,
    /// `-`（既可以是二元减，也可以是一元负号，由语法分析阶段决定）
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `^`（幂运算）
    Caret,

    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`（函数参数分隔符）
    Comma,
    /// `=`（变量赋值）
    Assign,

    /// 输入结束的哨兵。加上它可以让语法分析器少写很多 `Option` 判空。
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Ident(name) => write!(f, "{name}"),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Percent => write!(f, "%"),
            Self::Caret => write!(f, "^"),
            Self::LParen => write!(f, "("),
            Self::RParen => write!(f, ")"),
            Self::Comma => write!(f, ","),
            Self::Assign => write!(f, "="),
            Self::Eof => write!(f, "表达式结束"),
        }
    }
}

/// 一个 token = 类型 + 它在输入串中的位置。
///
/// 记录位置（`pos`）是为了让错误信息能指出「第几个字符出错」，
/// 这是编译器 / 解释器最基本的体验要求。
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// token 的类型。
    pub kind: TokenKind,
    /// 起始字节下标（从 0 开始）。
    pub pos: usize,
}

impl Token {
    /// 构造一个新 token。
    #[must_use]
    pub const fn new(kind: TokenKind, pos: usize) -> Self {
        Self { kind, pos }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_number_and_ident() {
        assert_eq!(TokenKind::Number(42.0).to_string(), "42");
        assert_eq!(TokenKind::Ident("sqrt".to_string()).to_string(), "sqrt");
        assert_eq!(TokenKind::Caret.to_string(), "^");
        assert_eq!(TokenKind::Eof.to_string(), "表达式结束");
    }

    #[test]
    fn token_keeps_position() {
        let token = Token::new(TokenKind::Plus, 7);
        assert_eq!(token.pos, 7);
        assert_eq!(token.kind, TokenKind::Plus);
    }
}
