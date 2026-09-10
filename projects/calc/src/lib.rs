// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 zongge —— 非商业使用免费；商业使用（含商业培训）须事先书面授权，见仓库根目录 LICENSE。

//! `calc` —— 一个命令行表达式计算器，也是 Rust 练手项目 #1。
//!
//! 数据流：
//!
//! ```text
//! &str ──词法分析(lexer)──▶ Vec<Token> ──语法分析(parser)──▶ AST ──求值(eval)──▶ f64
//! ```
//!
//! 每个阶段都返回 [`Result`]，错误统一收敛到 [`CalcError`]，库里没有 `unwrap` / `panic`。
//!
//! # 快速开始
//!
//! ```
//! use calc::{eval, Output, Session};
//!
//! // 1) 一次性求值
//! assert_eq!(eval("1 + 2 * (3 - 1)").unwrap(), 5.0);
//!
//! // 2) 带状态的会话：支持变量与 `ans`
//! let mut session = Session::new();
//! session.eval_line("x = 2 + 3").unwrap();
//! assert_eq!(session.eval_line("sqrt(x * 5)").unwrap(), Output::Value(5.0));
//! ```
//!
//! # 模块一览
//!
//! | 模块 | 职责 | 主要练习点 |
//! | --- | --- | --- |
//! | [`lexer`] | 字符流 → token | 生命周期 `&'a str`、迭代器、`Peekable` |
//! | [`token`] | token 定义 | 带数据的枚举、`Display` |
//! | [`parser`] | token → AST | 递归、优先级爬升、`Box` |
//! | [`ast`] | 语法树节点 | 递归数据结构、`Display` |
//! | [`eval`] | AST → `f64` | trait、函数指针、`HashMap`、`?` |
//! | [`session`] | 一行输入 → 结果 | 状态管理、枚举建模 |
//! | [`error`] | 统一错误类型 | `thiserror`、`Result` |
//! | [`fmt`] | 结果格式化 | `String` 处理 |

// 单元测试里为了方便可以使用 unwrap / expect / panic，库代码仍然禁止
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]

pub mod ast;
pub mod error;
pub mod eval;
pub mod fmt;
pub mod lexer;
pub mod parser;
pub mod session;
pub mod token;

pub use crate::ast::{BinaryOp, Expr, Stmt, UnaryOp};
pub use crate::error::CalcError;
pub use crate::eval::{functions, lookup_function, Arity, Context, Evaluate, Function, ANS};
pub use crate::fmt::format_number;
pub use crate::lexer::{tokenize, Lexer};
pub use crate::parser::{parse, Parser};
pub use crate::session::{Output, Session};
pub use crate::token::{Token, TokenKind};

/// 便捷函数：用空上下文一次性求值一个表达式。
///
/// # Errors
///
/// 词法 / 语法 / 求值错误，或输入为空时返回 [`CalcError`]。
///
/// # 例子
///
/// ```
/// use calc::eval;
///
/// assert_eq!(eval("(1 + 2) * 3").unwrap(), 9.0);
/// assert!(eval("1 / 0").is_err());
/// ```
pub fn eval(input: &str) -> Result<f64, CalcError> {
    let mut session = Session::new();
    match session.eval_line(input)? {
        Output::Value(value) => Ok(value),
        Output::Assignment { value, .. } => Ok(value),
        Output::Nothing => Err(CalcError::EmptyExpression),
    }
}
