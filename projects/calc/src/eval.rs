//! 求值器：遍历 AST 算出 `f64` 结果。
//!
//! 这里演示几个 Rust 抽象手段：
//!
//! - **trait**：[`Evaluate`] 定义「可求值」这一能力，`Expr` 实现它。
//!   以后若新增节点类型（比如数组、矩阵），只要再实现一次即可，调用方不用改。
//! - **`HashMap`**：变量表（名字 → 值）。
//! - **函数指针 `fn(&[f64]) -> Result<...>`**：把「内置函数表」做成数据，
//!   加一个函数只要往表里加一行，不用改 `match`。

use std::collections::HashMap;
use std::f64::consts;

use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::error::CalcError;

/// 上一次计算结果的变量名。
pub const ANS: &str = "ans";

/// 函数的参数个数约束。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    /// 恰好 `n` 个参数。
    Exact(usize),
    /// 至少 `n` 个参数（可变参数函数，如 `max(1, 2, 3)`）。
    AtLeast(usize),
}

impl Arity {
    /// 判断参数个数是否满足约束。
    #[must_use]
    pub const fn matches(self, count: usize) -> bool {
        match self {
            Self::Exact(n) => count == n,
            Self::AtLeast(n) => count >= n,
        }
    }

    /// 人类可读描述，用于错误信息。
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Exact(1) => "1",
            Self::Exact(2) => "2",
            Self::AtLeast(1) => "至少 1",
            Self::Exact(_) | Self::AtLeast(_) => "若干",
        }
    }
}

/// 一个内置函数。
#[derive(Debug, Clone, Copy)]
pub struct Function {
    /// 函数名。
    pub name: &'static str,
    /// 参数个数约束。
    pub arity: Arity,
    /// 实现体：函数指针，因此整个表可以是 `static` 的。
    pub apply: fn(&[f64]) -> Result<f64, CalcError>,
    /// 一句话说明，供 `:funcs` 展示。
    pub help: &'static str,
}

// ---- 内置函数实现：每个都是普通 fn，统一签名 fn(&[f64]) -> Result<f64, CalcError> ----

fn f_sqrt(a: &[f64]) -> Result<f64, CalcError> {
    require_non_negative("sqrt", a[0])?;
    Ok(a[0].sqrt())
}

fn f_cbrt(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].cbrt())
}

fn f_abs(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].abs())
}

fn f_sin(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].sin())
}

fn f_cos(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].cos())
}

fn f_tan(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].tan())
}

fn f_asin(a: &[f64]) -> Result<f64, CalcError> {
    require_in_unit_range("asin", a[0])?;
    Ok(a[0].asin())
}

fn f_acos(a: &[f64]) -> Result<f64, CalcError> {
    require_in_unit_range("acos", a[0])?;
    Ok(a[0].acos())
}

fn f_atan(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].atan())
}

fn f_atan2(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].atan2(a[1]))
}

fn f_ln(a: &[f64]) -> Result<f64, CalcError> {
    require_positive("ln", a[0])?;
    Ok(a[0].ln())
}

fn f_log2(a: &[f64]) -> Result<f64, CalcError> {
    require_positive("log2", a[0])?;
    Ok(a[0].log2())
}

fn f_log10(a: &[f64]) -> Result<f64, CalcError> {
    require_positive("log10", a[0])?;
    Ok(a[0].log10())
}

fn f_exp(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].exp())
}

fn f_floor(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].floor())
}

fn f_ceil(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].ceil())
}

fn f_round(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a[0].round())
}

fn f_pow(a: &[f64]) -> Result<f64, CalcError> {
    // 负底数 + 小数指数在实数域无解，显式报错而不是静默返回 NaN
    if a[0] < 0.0 && a[1].fract() != 0.0 {
        return Err(CalcError::DomainError {
            name: "pow".to_string(),
            reason: "负底数不支持小数指数".to_string(),
        });
    }
    Ok(a[0].powf(a[1]))
}

fn f_min(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a.iter().copied().fold(f64::INFINITY, f64::min))
}

fn f_max(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a.iter().copied().fold(f64::NEG_INFINITY, f64::max))
}

fn f_sum(a: &[f64]) -> Result<f64, CalcError> {
    Ok(a.iter().sum())
}

// ---- 定义域检查小工具 ----

fn require_positive(name: &str, value: f64) -> Result<f64, CalcError> {
    if value > 0.0 {
        Ok(value)
    } else {
        Err(CalcError::DomainError {
            name: name.to_string(),
            reason: "参数必须大于 0".to_string(),
        })
    }
}

fn require_non_negative(name: &str, value: f64) -> Result<f64, CalcError> {
    if value >= 0.0 {
        Ok(value)
    } else {
        Err(CalcError::DomainError {
            name: name.to_string(),
            reason: "参数不能为负数".to_string(),
        })
    }
}

fn require_in_unit_range(name: &str, value: f64) -> Result<f64, CalcError> {
    if (-1.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err(CalcError::DomainError {
            name: name.to_string(),
            reason: "参数必须落在 [-1, 1]".to_string(),
        })
    }
}

/// 内置函数表。加新函数只需在这里加一行。
static FUNCTIONS: &[Function] = &[
    Function {
        name: "sqrt",
        arity: Arity::Exact(1),
        apply: f_sqrt,
        help: "平方根",
    },
    Function {
        name: "cbrt",
        arity: Arity::Exact(1),
        apply: f_cbrt,
        help: "立方根",
    },
    Function {
        name: "abs",
        arity: Arity::Exact(1),
        apply: f_abs,
        help: "绝对值",
    },
    Function {
        name: "sin",
        arity: Arity::Exact(1),
        apply: f_sin,
        help: "正弦（弧度）",
    },
    Function {
        name: "cos",
        arity: Arity::Exact(1),
        apply: f_cos,
        help: "余弦（弧度）",
    },
    Function {
        name: "tan",
        arity: Arity::Exact(1),
        apply: f_tan,
        help: "正切（弧度）",
    },
    Function {
        name: "asin",
        arity: Arity::Exact(1),
        apply: f_asin,
        help: "反正弦",
    },
    Function {
        name: "acos",
        arity: Arity::Exact(1),
        apply: f_acos,
        help: "反余弦",
    },
    Function {
        name: "atan",
        arity: Arity::Exact(1),
        apply: f_atan,
        help: "反正切",
    },
    Function {
        name: "atan2",
        arity: Arity::Exact(2),
        apply: f_atan2,
        help: "atan2(y, x)",
    },
    Function {
        name: "ln",
        arity: Arity::Exact(1),
        apply: f_ln,
        help: "自然对数",
    },
    Function {
        name: "log2",
        arity: Arity::Exact(1),
        apply: f_log2,
        help: "以 2 为底的对数",
    },
    Function {
        name: "log10",
        arity: Arity::Exact(1),
        apply: f_log10,
        help: "常用对数",
    },
    Function {
        name: "exp",
        arity: Arity::Exact(1),
        apply: f_exp,
        help: "e 的幂",
    },
    Function {
        name: "floor",
        arity: Arity::Exact(1),
        apply: f_floor,
        help: "向下取整",
    },
    Function {
        name: "ceil",
        arity: Arity::Exact(1),
        apply: f_ceil,
        help: "向上取整",
    },
    Function {
        name: "round",
        arity: Arity::Exact(1),
        apply: f_round,
        help: "四舍五入",
    },
    Function {
        name: "pow",
        arity: Arity::Exact(2),
        apply: f_pow,
        help: "pow(x, y) 幂运算",
    },
    Function {
        name: "min",
        arity: Arity::AtLeast(1),
        apply: f_min,
        help: "最小值（可变参数）",
    },
    Function {
        name: "max",
        arity: Arity::AtLeast(1),
        apply: f_max,
        help: "最大值（可变参数）",
    },
    Function {
        name: "sum",
        arity: Arity::AtLeast(1),
        apply: f_sum,
        help: "求和（可变参数）",
    },
];

/// 按名字查找内置函数。
#[must_use]
pub fn lookup_function(name: &str) -> Option<&'static Function> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

/// 遍历所有内置函数（供 REPL 的 `:funcs` 使用）。
pub fn functions() -> impl Iterator<Item = &'static Function> {
    FUNCTIONS.iter()
}

/// 求值上下文：保存变量表。
///
/// 内置常量（`pi` / `e` / `tau`）也是普通变量，只是预先塞进表里，
/// 这样求值逻辑不用为「常量」单独分支。
#[derive(Debug, Clone)]
pub struct Context {
    vars: HashMap<String, f64>,
}

impl Context {
    /// 创建一个只含内置常量的上下文。
    #[must_use]
    pub fn new() -> Self {
        Self {
            vars: HashMap::from([
                ("pi".to_string(), consts::PI),
                ("e".to_string(), consts::E),
                ("tau".to_string(), consts::TAU),
            ]),
        }
    }

    /// 写入（或覆盖）一个变量。
    pub fn set(&mut self, name: impl Into<String>, value: f64) {
        self.vars.insert(name.into(), value);
    }

    /// 读取变量值；不存在时返回 `None`。
    #[must_use]
    pub fn get(&self, name: &str) -> Option<f64> {
        self.vars.get(name).copied()
    }

    /// 删除变量，返回被删除的值。
    pub fn remove(&mut self, name: &str) -> Option<f64> {
        self.vars.remove(name)
    }

    /// 只读访问整张变量表。
    #[must_use]
    pub fn vars(&self) -> &HashMap<String, f64> {
        &self.vars
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// “可求值”这一能力。
///
/// 定义成 trait 而不是直接给 `Expr` 写 `fn eval`，
/// 是为了让「求值」成为一个可以替换 / 扩展的抽象（例如以后加一个常量折叠的 visitor）。
pub trait Evaluate {
    /// 在给定上下文中求值。
    ///
    /// # Errors
    ///
    /// 变量未定义、函数未知、参数个数不对、除零或超出定义域时返回 [`CalcError`]。
    fn evaluate(&self, ctx: &Context) -> Result<f64, CalcError>;
}

impl Evaluate for Expr {
    fn evaluate(&self, ctx: &Context) -> Result<f64, CalcError> {
        match self {
            Expr::Number(value) => Ok(*value),
            Expr::Var(name) => ctx
                .get(name)
                .ok_or_else(|| CalcError::UnknownVariable { name: name.clone() }),
            Expr::Unary { op, expr } => {
                let value = expr.evaluate(ctx)?;
                match op {
                    UnaryOp::Neg => Ok(-value),
                    UnaryOp::Pos => Ok(value),
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                // `?` 把子表达式的错误直接向上传播 —— 不要 unwrap
                let a = lhs.evaluate(ctx)?;
                let b = rhs.evaluate(ctx)?;
                match op {
                    BinaryOp::Add => Ok(a + b),
                    BinaryOp::Sub => Ok(a - b),
                    BinaryOp::Mul => Ok(a * b),
                    BinaryOp::Div | BinaryOp::Rem => {
                        if b == 0.0 {
                            Err(CalcError::DivisionByZero)
                        } else {
                            match op {
                                BinaryOp::Div => Ok(a / b),
                                _ => Ok(a % b),
                            }
                        }
                    }
                    BinaryOp::Pow => f_pow(&[a, b]),
                }
            }
            Expr::Call { name, args } => {
                let func = lookup_function(name)
                    .ok_or_else(|| CalcError::UnknownFunction { name: name.clone() })?;
                if !func.arity.matches(args.len()) {
                    return Err(CalcError::WrongArity {
                        name: name.clone(),
                        expected: func.arity.describe().to_string(),
                        got: args.len(),
                    });
                }
                // 迭代器 + collect 直接把 Vec<Result<..>> 收敛成 Result<Vec<..>>
                let values: Result<Vec<f64>, CalcError> =
                    args.iter().map(|arg| arg.evaluate(ctx)).collect();
                (func.apply)(&values?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// 解析并求值，取 `f64` 结果。
    fn value_of(input: &str) -> f64 {
        let stmt = parse(input).expect("解析失败");
        match stmt {
            crate::ast::Stmt::Expr(expr) => expr.evaluate(&Context::new()).expect("求值失败"),
            other => panic!("期望表达式，实际是 {other:?}"),
        }
    }

    #[test]
    fn arithmetic_basics() {
        assert_eq!(value_of("1 + 2"), 3.0);
        assert_eq!(value_of("7 - 9"), -2.0);
        assert_eq!(value_of("6 * 7"), 42.0);
        assert_eq!(value_of("1 / 4"), 0.25);
        assert_eq!(value_of("7 % 3"), 1.0);
        assert_eq!(value_of("2 ^ 10"), 1024.0);
    }

    #[test]
    fn precedence_and_associativity() {
        assert_eq!(value_of("1 + 2 * 3"), 7.0);
        assert_eq!(value_of("(1 + 2) * 3"), 9.0);
        assert_eq!(value_of("10 - 2 - 3"), 5.0); // 左结合
        assert_eq!(value_of("2 ^ 3 ^ 2"), 512.0); // 右结合：2^(3^2)
        assert_eq!(value_of("-2 ^ 2"), -4.0);
    }

    #[test]
    fn unary_minus() {
        assert_eq!(value_of("-3"), -3.0);
        assert_eq!(value_of("-(1 + 2)"), -3.0);
        assert_eq!(value_of("3 * -2"), -6.0);
    }

    #[test]
    fn builtin_constants() {
        assert_eq!(value_of("pi"), consts::PI);
        assert_eq!(value_of("2 * pi"), consts::TAU);
        assert_eq!(value_of("e"), consts::E);
    }

    #[test]
    fn function_calls() {
        assert_eq!(value_of("sqrt(16)"), 4.0);
        assert_eq!(value_of("pow(2, 0.5)"), 2.0f64.sqrt());
        assert_eq!(value_of("abs(0 - 5)"), 5.0);
        assert_eq!(value_of("max(1, 9, 3)"), 9.0);
        assert_eq!(value_of("min(1, 9, 3)"), 1.0);
        assert_eq!(value_of("sum(1, 2, 3, 4)"), 10.0);
        assert_eq!(value_of("round(2.5)"), 3.0);
    }

    #[test]
    fn nested_calls() {
        assert_eq!(value_of("sqrt(pow(3, 2) + pow(4, 2))"), 5.0);
    }

    #[test]
    fn division_by_zero_is_an_error() {
        let stmt = parse("1 / 0").expect("解析成功");
        let err = match stmt {
            crate::ast::Stmt::Expr(expr) => {
                expr.evaluate(&Context::new()).expect_err("除零应该报错")
            }
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert_eq!(err, CalcError::DivisionByZero);
    }

    #[test]
    fn modulo_by_zero_is_an_error() {
        let stmt = parse("1 % 0").expect("解析成功");
        let err = match stmt {
            crate::ast::Stmt::Expr(expr) => {
                expr.evaluate(&Context::new()).expect_err("模零应该报错")
            }
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert_eq!(err, CalcError::DivisionByZero);
    }

    #[test]
    fn unknown_variable_is_an_error() {
        let stmt = parse("nope + 1").expect("解析成功");
        let err = match stmt {
            crate::ast::Stmt::Expr(expr) => expr.evaluate(&Context::new()).expect_err("未定义变量"),
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert_eq!(
            err,
            CalcError::UnknownVariable {
                name: "nope".to_string()
            }
        );
    }

    #[test]
    fn unknown_function_is_an_error() {
        let stmt = parse("foo(1)").expect("解析成功");
        let err = match stmt {
            crate::ast::Stmt::Expr(expr) => expr.evaluate(&Context::new()).expect_err("未知函数"),
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert_eq!(
            err,
            CalcError::UnknownFunction {
                name: "foo".to_string()
            }
        );
    }

    #[test]
    fn wrong_arity_is_an_error() {
        let stmt = parse("sqrt(1, 2)").expect("解析成功");
        let err = match stmt {
            crate::ast::Stmt::Expr(expr) => expr.evaluate(&Context::new()).expect_err("参数个数错"),
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert_eq!(
            err,
            CalcError::WrongArity {
                name: "sqrt".to_string(),
                expected: "1".to_string(),
                got: 2,
            }
        );
    }

    #[test]
    fn domain_error_for_sqrt_of_negative() {
        let stmt = parse("sqrt(-1)").expect("解析成功");
        let err = match stmt {
            crate::ast::Stmt::Expr(expr) => expr.evaluate(&Context::new()).expect_err("定义域错误"),
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert!(matches!(err, CalcError::DomainError { .. }));
    }

    #[test]
    fn negative_base_with_fractional_exponent_is_a_domain_error() {
        let stmt = parse("(-8) ^ (1/3)").expect("解析成功");
        let err = match stmt {
            crate::ast::Stmt::Expr(expr) => expr.evaluate(&Context::new()).expect_err("定义域错误"),
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert!(matches!(err, CalcError::DomainError { .. }));
    }

    #[test]
    fn context_variables() {
        let mut ctx = Context::new();
        ctx.set("x", 10.0);
        let expr = match parse("x * 2").expect("解析成功") {
            crate::ast::Stmt::Expr(expr) => expr,
            other => panic!("期望表达式，实际是 {other:?}"),
        };
        assert_eq!(expr.evaluate(&ctx).expect("求值成功"), 20.0);
        assert_eq!(ctx.get("x"), Some(10.0));
        assert_eq!(ctx.remove("x"), Some(10.0));
        assert_eq!(ctx.get("x"), None);
    }

    #[test]
    fn arity_helper() {
        assert!(Arity::Exact(2).matches(2));
        assert!(!Arity::Exact(2).matches(3));
        assert!(Arity::AtLeast(1).matches(5));
        assert_eq!(Arity::AtLeast(1).describe(), "至少 1");
    }

    #[test]
    fn function_table_lookup() {
        assert!(lookup_function("sqrt").is_some());
        assert!(lookup_function("不存在的函数").is_none());
        assert_eq!(functions().count(), 21);
    }
}
