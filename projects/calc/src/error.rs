//! 统一的错误类型。
//!
//! 库里所有可能失败的操作都返回 [`CalcError`]，不允许 `unwrap` / `panic`：
//! 「可恢复的错误」用 `Result` 交给调用方决定（打印？重试？忽略？），
//! 只有「程序 bug」才 panic。
//!
//! 这里用 [`thiserror::Error`] 派生 `std::error::Error` + `Display`，
//! 它生成的就是手写实现的那套代码，省掉模板代码但语义完全一致。

use thiserror::Error;

/// 计算器可能出现的全部错误。
///
/// 每个变体都携带足够的上下文（名字、位置、期望值），
/// 这样上层（CLI / REPL）可以直接打印出对人类友好的提示。
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CalcError {
    /// 词法分析阶段：遇到了不属于本语言字母表的字符。
    #[error("无法识别的字符 `{ch}`（位置 {pos}）")]
    UnexpectedChar {
        /// 出错的字符。
        ch: char,
        /// 该字符在输入串中的字节下标（从 0 开始）。
        pos: usize,
    },

    /// 词法分析阶段：数字字面量看起来合法但无法解析成 `f64`（如 `1e`、`1.2.3`）。
    #[error("数字 `{text}` 格式非法（位置 {pos}）：{reason}")]
    InvalidNumber {
        /// 扫描到的原始文本。
        text: String,
        /// 起始字节下标。
        pos: usize,
        /// 来自标准库 `parse::<f64>()` 的原因说明。
        reason: String,
    },

    /// 语法分析阶段：期望某种 token，实际遇到了另一种。
    #[error("语法错误：期望 {expected}，实际是 `{found}`（位置 {pos}）")]
    UnexpectedToken {
        /// 期望看到的内容（人类可读描述）。
        expected: String,
        /// 实际看到的 token 文本。
        found: String,
        /// 出错位置。
        pos: usize,
    },

    /// 语法分析阶段：表达式还没写完输入就结束了（如 `(1 + 2`）。
    #[error("语法错误：表达式意外结束，期望 {expected}")]
    UnexpectedEof {
        /// 期望看到的内容。
        expected: String,
    },

    /// 求值阶段：引用了未定义的变量或常量。
    #[error("未知的变量或常量 `{name}`（可用 `:vars` 查看已定义的变量）")]
    UnknownVariable {
        /// 变量名。
        name: String,
    },

    /// 求值阶段：调用了不存在的函数。
    #[error("未知的函数 `{name}`（可用 `:funcs` 查看支持的函数）")]
    UnknownFunction {
        /// 函数名。
        name: String,
    },

    /// 求值阶段：函数参数个数不匹配。
    #[error("函数 `{name}` 需要 {expected} 个参数，实际传入 {got} 个")]
    WrongArity {
        /// 函数名。
        name: String,
        /// 期望的元数描述，如「2」「至少 1」。
        expected: String,
        /// 实际参数个数。
        got: usize,
    },

    /// 求值阶段：除数为 0。
    #[error("除数为 0")]
    DivisionByZero,

    /// 求值阶段：数学函数收到定义域外的参数（如 `sqrt(-1)`、`ln(0)`）。
    #[error("函数 `{name}` 的参数超出定义域：{reason}")]
    DomainError {
        /// 函数名。
        name: String,
        /// 具体原因。
        reason: String,
    },

    /// 输入是空行或纯注释，没有可求值的表达式。
    #[error("表达式为空")]
    EmptyExpression,
}
