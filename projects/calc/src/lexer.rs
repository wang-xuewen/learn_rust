// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 zongge —— 非商业使用免费；商业使用（含商业培训）须事先书面授权，见仓库根目录 LICENSE。

//! 词法分析器（Lexer / Tokenizer）：`&str` → `Vec<Token>`。
//!
//! 设计要点（都是 Rust 的常用套路）：
//!
//! 1. **借用而非拷贝**：`Lexer<'a>` 持有 `&'a str` 生命周期参数，不复制输入。
//! 2. **迭代器风格**：实现 [`Iterator`]，用 `Peekable` 实现「向前看一个字符」。
//!    有了 `tokenize` 的 `collect()`，错误处理也自然变成 `Result<Vec<_>, _>`。
//! 3. **零宽度的哨兵**：迭代结束前吐出一个 `Eof` token，语法分析器不用到处判空。

use std::iter::Peekable;
use std::str::CharIndices;

use crate::error::CalcError;
use crate::token::{Token, TokenKind};

/// 词法分析器：把输入字符串切成 token 流。
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    /// 原始输入，只借用不拥有。
    input: &'a str,

    // 为什么 chars 没提 input，却和 input 绑定
    // 它们没有用名字互相引用，但都用同一个 'a 参数。这个 'a 是「整个 Lexer 结构体所借用的那块字符串的寿命」。
    // CharIndices<'a> 的 <'a> 不是凭空写的，它表示「这个迭代器借用的字符串的寿命」。
    // 结构体中 input 和 chars 共享同一个 'a'，所以虽然代码里没有写 chars 引用 input，
    // 但生命周期参数 'a 已经把两者在编译期牢牢绑在一起——这就是 Rust 用「生命周期参数」
    // 而非「字段互相引用」来表达「多个借用指向同一块内存且同寿」的方式。

    /// 带下标的字符迭代器；`Peekable` 让我们能「偷看」下一个字符而不消耗它。
    chars: Peekable<CharIndices<'a>>,
    /// 是否已经吐出过 `Eof`，避免无限循环。
    eof_emitted: bool,
}

impl<'a> Lexer<'a> {
    /// 基于输入串创建词法分析器。
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            eof_emitted: false,
        }
    }

    /// 扫描一个数字字面量：`123`、`1.5`、`.5`、`1e9`、`2.5e-3`。
    fn scan_number(&mut self, first: char, start: usize) -> Result<Token, CalcError> {
        let mut text = String::from(first);
        let mut seen_dot = first == '.';
        let mut seen_exp = false;

        while let Some(&(_, ch)) = self.chars.peek() {
            match ch {
                c if c.is_ascii_digit() => {
                    text.push(c);
                    self.chars.next();
                }
                '.' if !seen_dot && !seen_exp => {
                    seen_dot = true;
                    text.push(ch);
                    self.chars.next();
                }
                'e' | 'E' if !seen_exp => {
                    seen_exp = true;
                    text.push(ch);
                    self.chars.next();
                    // 指数部分可以带符号：1e-9
                    if let Some(&(_, sign)) = self.chars.peek() {
                        if sign == '+' || sign == '-' {
                            text.push(sign);
                            self.chars.next();
                        }
                    }
                }
                _ => break,
            }
        }

        match text.parse::<f64>() {
            Ok(value) => Ok(Token::new(TokenKind::Number(value), start)),
            Err(err) => Err(CalcError::InvalidNumber {
                text,
                pos: start,
                reason: err.to_string(),
            }),
        }
    }

    /// 扫描标识符（变量名 / 函数名）：字母或 `_` 开头，后接字母、数字、`_`。
    fn scan_ident(&mut self, first: char, start: usize) -> Token {
        let mut name = String::from(first);
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }
        Token::new(TokenKind::Ident(name), start)
    }
}

/// 一次产出一个 token；出错时产出 `Err`，调用方用 `collect::<Result<Vec<_>, _>>()` 收敛。
// <'_> 里的 '_ 是匿名生命周期占位符，意思是「这里有一个生命周期参数，但我懒得（也不需要）给它起名字，编译器你自己推断去」。
// 为什么这里用 '_ 而不是 'a
// 关键看 impl 块内部需不需要引用这个生命周期,
// 在 impl Iterator for Lexer<'_>（lexer.rs:100）里，next 的签名是 
// fn next(&mut self) -> Option<Self::Item> —— 全程只用到 self，
// 没有任何地方需要把生命周期写出来。既然用不到名字，就没必要起名，用 _ 更简洁。
impl Iterator for Lexer<'_> {
    type Item = Result<Token, CalcError>;
// todo
    fn next(&mut self) -> Option<Self::Item> {
        if self.eof_emitted {
            return None;
        }

        while let Some((pos, ch)) = self.chars.next() {
            let kind = match ch {
                // 空白是分隔符，直接丢弃
                c if c.is_whitespace() => continue,
                // 数字：以数字开头，或以 `.` 开头（支持 `.5`）
                c if c.is_ascii_digit() => return Some(self.scan_number(c, pos)),
                '.' => return Some(self.scan_number(ch, pos)),
                // 标识符：变量名 / 函数名
                c if c.is_alphabetic() || c == '_' => return Some(Ok(self.scan_ident(c, pos))),
                // 运算符与标点
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '^' => TokenKind::Caret,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                ',' => TokenKind::Comma,
                '=' => TokenKind::Assign,
                // 其它一律不认识
                _ => return Some(Err(CalcError::UnexpectedChar { ch, pos })),
            };
            return Some(Ok(Token::new(kind, pos)));
        }

        // 字符耗尽：补一个哨兵，然后迭代器结束
        self.eof_emitted = true;
        Some(Ok(Token::new(TokenKind::Eof, self.input.len())))
    }
}

/// 便捷入口：把整段输入切成 token 列表（末尾一定带一个 [`TokenKind::Eof`]）。
///
/// # Errors
///
/// 遇到非法字符或格式非法的数字时返回 [`CalcError`]。
pub fn tokenize(input: &str) -> Result<Vec<Token>, CalcError> {
    // collect() 不是 Lexer 自己声明的方法，而是 Iterator trait 自带的方法。
    // Lexer 一旦实现了 Iterator（提供了 next()），
    // 就能自动获得 collect()、map()、filter() 等一整套方法

    // 这里有两个细节：
    // 因为 type Item = Result<Token, CalcError>，所以 collect() 默认收集成 Vec<Result<Token, CalcError>>。
    // 但 tokenize 的返回类型是 Result<Vec<Token>, CalcError>。
    // 这里利用了一个很方便的特性：Result 也实现了 FromIterator，
    // 所以 collect() 能把 Vec<Result<_>> 自动「短路」收敛成 Result<Vec<_>> —— 一旦出现 Err，
    // 整个收集结果就是 Err，否则把所有 Ok 里的 Token 聚成 Vec。这就是注释里说的「错误处理也自然变成 Result<Vec<_>, _>」。

    // Result 类型也实现了 FromIterator trait
    // 意味着：你可以把一个 Result 的迭代器直接 .collect() 成一个 Result<集合, 错误>——全 Ok 就成功，
    // 遇到第一个 Err 就整体失败并短路。 这是 Rust 里把“逐个可能失败的操作”聚合成“整体成功或失败”的常用惯用法。
    
    // Lexer 结构体实现了Iterator trait。此处就会调用Iterator的collect的方法
    // Rust 的方法解析顺序是：先看实例自身的方法，没有就去看它实现的 trait 上的方法。
    // Lexer 没有声明 collect，于是回退到 Iterator trait —— 找到就调用。
    Lexer::new(input).collect()

    // Iterator 中 next方法的实现方式会影响collect返回结果吗
    // 会，而且 next() 的实现完全决定了 collect() 的结果——collect() 本身没有任何逻辑，
    // 它就是「反复调 next()，把每次拿到的值存起来，直到 next() 返回 None」。
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 只取 token 类型，忽略位置，方便断言。
    fn kinds(input: &str) -> Vec<TokenKind> {
        tokenize(input)
            .expect("tokenize 失败")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn tokenize_simple_expression() {
        assert_eq!(
            kinds("1 + 2"),
            vec![
                TokenKind::Number(1.0),
                TokenKind::Plus,
                TokenKind::Number(2.0),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_ignores_whitespace() {
        assert_eq!(
            kinds("  ( 1\t*\n2 ) "),
            vec![
                TokenKind::LParen,
                TokenKind::Number(1.0),
                TokenKind::Star,
                TokenKind::Number(2.0),
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_number_forms() {
        assert_eq!(
            kinds("1 1.5 .5 1e3 2.5e-3"),
            vec![
                TokenKind::Number(1.0),
                TokenKind::Number(1.5),
                TokenKind::Number(0.5),
                TokenKind::Number(1000.0),
                TokenKind::Number(0.0025),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_ident_and_call() {
        assert_eq!(
            kinds("sqrt(x)"),
            vec![
                TokenKind::Ident("sqrt".to_string()),
                TokenKind::LParen,
                TokenKind::Ident("x".to_string()),
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_records_position() {
        let tokens = tokenize("1 + $").expect_err("`$` 应该被拒绝");
        assert_eq!(tokens, CalcError::UnexpectedChar { ch: '$', pos: 4 });
    }

    #[test]
    fn tokenize_rejects_bad_number() {
        let err = tokenize("1e").expect_err("`1e` 不是合法数字");
        assert!(matches!(err, CalcError::InvalidNumber { pos: 0, .. }));
    }

    #[test]
    fn empty_input_only_yields_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn lexer_iterator_terminates() {
        // 迭代完 Eof 之后必须是 None，否则 `for` 循环会死循环
        let mut lexer = Lexer::new("1");
        let _ = lexer.next();
        let _ = lexer.next(); // Eof
        assert!(lexer.next().is_none());
    }
}
