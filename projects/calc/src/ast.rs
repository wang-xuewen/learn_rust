// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 zongge —— 非商业使用免费；商业使用（含商业培训）须事先书面授权，见仓库根目录 LICENSE。

//! 抽象语法树（AST）定义。
//!
//! 语法分析器把 token 流组装成树；求值器遍历这棵树算出结果。
//! AST 是「递归数据结构」，所以节点里必须放 `Box<Expr>` —— `Box` 提供
//! 间接层打破无限大小，这是 Rust 里写树的标准做法。

use std::fmt;

/// 一元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-a`
    Neg,
    /// `+a`
    Pos,
}

impl UnaryOp {
    /// 运算符的源码符号。
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// 二元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `^`
    Pow,
}

impl BinaryOp {
    /// 运算符的源码符号。
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Pow => "^",
        }
    }

    /// 优先级（binding power）：`Pratt 解析` 用它决定「谁先结合」。
    ///
    /// 返回 `(左绑定力, 右绑定力)`：
    /// - 左结合（如 `a - b - c`）：左 < 右
    /// - 右结合（如 `a ^ b ^ c`）：左 > 右
    #[must_use]
    pub const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Add | Self::Sub => (1, 2),
            Self::Mul | Self::Div | Self::Rem => (3, 4),
            // 幂运算右结合：2 ^ 3 ^ 2 == 2 ^ (3 ^ 2)
            Self::Pow => (7, 6),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// 表达式节点。
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// 数字字面量。
    Number(f64),
    /// 变量 / 常量引用。
    Var(String),
    /// 一元运算，如 `-x`。
    Unary {
        /// 运算符。
        op: UnaryOp,
        /// 操作数。
        expr: Box<Expr>,
    },
    /// 二元运算，如 `a + b`。
    Binary {
        /// 运算符。
        op: BinaryOp,
        /// 左操作数。
        lhs: Box<Expr>,
        /// 右操作数。
        rhs: Box<Expr>,
    },
    /// 函数调用，如 `sqrt(2)`。
    Call {
        /// 函数名。
        name: String,
        /// 实参列表。
        args: Vec<Expr>,
    },
}

impl fmt::Display for Expr {
    /// 回显 AST：一律加括号，能直观看出优先级与结合性，调试时非常有用。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Var(name) => write!(f, "{name}"),
            Self::Unary { op, expr } => write!(f, "({op}{expr})"),
            Self::Binary { op, lhs, rhs } => write!(f, "({lhs} {op} {rhs})"),
            Self::Call { name, args } => {
                // 迭代器 + join，比手写 for 循环拼字符串更地道
                let args: Vec<String> = args.iter().map(ToString::to_string).collect();
                write!(f, "{name}({})", args.join(", "))
            }
        }
    }
}

/// 语句：目前只有两种 —— 表达式求值，或变量赋值。
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// 纯表达式，如 `1 + 2`。
    Expr(Expr),
    /// 赋值语句，如 `x = 1 + 2`。
    Assign {
        /// 变量名。
        name: String,
        /// 右侧表达式。
        expr: Expr,
    },
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expr(expr) => write!(f, "{expr}"),
            Self::Assign { name, expr } => write!(f, "{name} = {expr}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_binary_adds_parentheses() {
        let expr = Expr::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::Number(1.0)),
            rhs: Box::new(Expr::Number(2.0)),
        };
        assert_eq!(expr.to_string(), "(1 * 2)");
    }

    #[test]
    fn display_call_joins_args() {
        let expr = Expr::Call {
            name: "pow".to_string(),
            args: vec![Expr::Number(2.0), Expr::Number(10.0)],
        };
        assert_eq!(expr.to_string(), "pow(2, 10)");
    }

    #[test]
    fn binding_power_defines_associativity() {
        assert_eq!(BinaryOp::Sub.binding_power(), (1, 2)); // 左结合
        assert!(BinaryOp::Pow.binding_power().0 > BinaryOp::Pow.binding_power().1); // 右结合
        assert!(BinaryOp::Add.binding_power().0 < BinaryOp::Mul.binding_power().0);
        // 乘法更紧
    }
}
