// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 zongge —— 非商业使用免费；商业使用（含商业培训）须事先书面授权，见仓库根目录 LICENSE。

//! 会话层：一次输入一行文本，返回一个结果。
//!
//! 它把「词法 → 语法 → 求值」串起来，并在 REPL 里保留变量状态。
//! CLI 和未来的 GUI / Web 前端都只需要依赖这一层，不关心内部细节。

use crate::ast::Stmt;
use crate::error::CalcError;
use crate::eval::{Context, Evaluate, ANS};
use crate::parser::parse;

/// 一行输入的执行结果。
///
/// 用枚举而不是「返回 `f64` + 靠副作用打印」，
/// 是为了让调用方（CLI / 测试 / 未来的 API）自己决定怎么展示。
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// 表达式求值成功，得到该值。
    Value(f64),
    /// 赋值语句执行成功：变量 `name` 现在是 `value`。
    Assignment {
        /// 被赋值的变量名。
        name: String,
        /// 赋给它的值。
        value: f64,
    },
    /// 空行或注释行，没有产生结果。
    Nothing,
}

/// 保存变量与状态的会话。
///
/// # 例子
///
/// ```
/// use calc::{Output, Session};
///
/// let mut session = Session::new();
/// // 表达式的结果会自动写入 `ans`
/// assert_eq!(session.eval_line("6 * 7").unwrap(), Output::Value(42.0));
/// assert_eq!(session.eval_line("ans / 6").unwrap(), Output::Value(7.0));
///
/// // 赋值语句不写入 `ans`
/// session.eval_line("x = 10").unwrap();
/// assert_eq!(session.eval_line("x * 2").unwrap(), Output::Value(20.0));
/// ```
#[derive(Debug, Clone, Default)]
pub struct Session {
    ctx: Context,
}

impl Session {
    /// 创建一个新会话（内置常量已就绪）。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 只读访问上下文（例如 REPL 打印 `:vars`）。
    #[must_use]
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// 可变访问上下文。
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.ctx
    }

    /// 执行一行输入。
    ///
    /// - 空行或以 `#` 开头的注释行返回 [`Output::Nothing`]；
    /// - 表达式返回 [`Output::Value`]，并把结果写入 `ans`；
    /// - 赋值语句返回 [`Output::Assignment`]，**不**写入 `ans`。
    ///
    /// # Errors
    ///
    /// 词法 / 语法 / 求值错误都会原样返回 [`CalcError`]。
    pub fn eval_line(&mut self, line: &str) -> Result<Output, CalcError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(Output::Nothing);
        }

        match parse(line)? {
            Stmt::Expr(expr) => {
                let value = expr.evaluate(&self.ctx)?;
                self.ctx.set(ANS, value);
                Ok(Output::Value(value))
            }
            Stmt::Assign { name, expr } => {
                let value = expr.evaluate(&self.ctx)?;
                self.ctx.set(name.clone(), value);
                Ok(Output::Assignment { name, value })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_and_comment_lines_produce_nothing() {
        let mut session = Session::new();
        assert_eq!(session.eval_line("").expect("空行"), Output::Nothing);
        assert_eq!(session.eval_line("   ").expect("空白行"), Output::Nothing);
        assert_eq!(
            session.eval_line("# 注释").expect("注释行"),
            Output::Nothing
        );
    }

    #[test]
    fn expression_result_is_stored_in_ans() {
        let mut session = Session::new();
        assert_eq!(
            session.eval_line("2 ^ 5").expect("求值成功"),
            Output::Value(32.0)
        );
        assert_eq!(session.context().get(ANS), Some(32.0));
        assert_eq!(
            session.eval_line("ans + 8").expect("求值成功"),
            Output::Value(40.0)
        );
    }

    #[test]
    fn assignment_does_not_touch_ans() {
        let mut session = Session::new();
        session.eval_line("5").expect("求值成功");
        let output = session.eval_line("x = 100").expect("赋值成功");
        assert_eq!(
            output,
            Output::Assignment {
                name: "x".to_string(),
                value: 100.0,
            }
        );
        assert_eq!(session.context().get("x"), Some(100.0));
        assert_eq!(session.context().get(ANS), Some(5.0));
    }

    #[test]
    fn variables_persist_between_lines() {
        let mut session = Session::new();
        session.eval_line("rate = 0.05").expect("赋值成功");
        session.eval_line("principal = 1000").expect("赋值成功");
        assert_eq!(
            session.eval_line("principal * rate").expect("求值成功"),
            Output::Value(50.0)
        );
    }

    #[test]
    fn errors_are_propagated_and_state_is_unchanged() {
        let mut session = Session::new();
        let err = session.eval_line("1 / 0").expect_err("除零");
        assert_eq!(err, CalcError::DivisionByZero);
        assert_eq!(session.context().get(ANS), None);
    }

    #[test]
    fn parse_errors_are_propagated() {
        let mut session = Session::new();
        assert!(matches!(
            session.eval_line("1 +"),
            Err(CalcError::UnexpectedEof { .. })
        ));
    }
}
