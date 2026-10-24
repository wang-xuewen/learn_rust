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
    // [must_use] 是 Rust 的一个属性（attribute），
    //   用作编译器的警告提示：如果被标注的函数的返回值被丢弃（没有使用），编译器会给出警告
    //   若你确实想忽略返回值且不想看到警告，可以显式写 let _ = op.symbol();，
    //   这告诉编译器"我有意丢弃"，#[must_use] 就不会报警。
    // const fn — 常量函数。它能在编译期被求值，而不只是运行期
    // 'static 表示它活整个程序运行期
    //   用 &'static str 而不是 String：避免堆分配，
    //   且能在 const fn 编译期返回（返回 String 需要堆分配，无法在编译期完成）。
    //   'static 的意思是：这个引用在程序运行的整个期间都有效（到程序退出才失效）。
    //   它只保证"活得够久"，并不承诺数据是在编译期被写进二进制的。
    //   你这个 symbol 返回的是字面量 "-" / "+"。编译器会把它们的字节直接塞进生成的
    //   可执行文件（通常是二进制的只读数据段 .rodata），程序一启动这些字节就在内存里了。
    //   所以对于"字面量产生的 &'static str"，你说"编译时就把值保存在程序中"——对你的场景是成立的正确理解。
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
        }
    }
}

impl fmt::Display for UnaryOp {
    // <'_> 里的 _ 是生命周期占位符：Formatter 本身带一个生命周期参数（<'a>，表示它借用了一段输出缓冲），
    // 这里让编译器自己推断，不用你手写。等价地写 Formatter<'_> 是惯例写法。
    // fmt::Result — 返回值类型，它是个别名
    //   pub type Result = Result<(), fmt::Error>;
    // Formatter<'a> 的 'a 表示"它借去写字的那个输出缓冲"要活多久；
    // 因为 Formatter 内部持有 &'a mut dyn Write 的借用，所以必须带这个生命周期参数。
    // 你平时写的 <'_> 只是懒得起名、让编译器自己推断这个时长。
    // 这个生命周期别处根本用不到（既没有出现在 self、也没有出现在返回值里），
    // 纯粹是 Formatter 自带的。既然不引用它，按惯例就用 _ 表示"懒得起名、你编译器自己看着办"，代码更干净。
    // buffer 和 Formatter 都是 println!/format! 这些宏在调用你的 fmt 之前就建好并借好的；
    // 它们通过参数 f 把"已接好 buffer 的 Formatter"递给你，你函数体一运行，
    // f 就已经能写了——你从没、也不该自己去指定那个 buffer。
    // 不是"编译好、运行时因为宏展开，Rust 才去决定生命周期"；
    // 而是"编译期，在宏展开 + 类型推断 + 借用检查阶段，编译器就已经把 _ 具体化成某个生命周期并验证通过了"。
    // _（或显式的 'a）只是告诉编译器："这里有个生命周期，我懒得起名/不用引用它，你帮我推断"。
    // 编译器在编译期根据上下文把它定成一个具体生命周期。
    // 这里用 _ 占个位，编译器在编译期（结合宏展开出来的代码）自行把生命周期推断成具体值并做借用检查；
    // 运行期这些生命周期早已消失，既不参与决策也无任何开销。
    // 看来主要还是因为很多代码都是宏写的，所以这里我只要写个'_就可以了，
    // 编译器会根据展开的宏来自行判断是否正确，而且formatter用到的引用是宏来处理的，
    // 所以我的代码几乎就不用管。是这样吧
    // _ 为什么能写 —— 不是因为宏，而是因为 Formatter 那个生命周期参数在你签名里别处都用不到
    //（没出现在 self、也没出现在返回值），所以按惯例用 _ 表示"懒得起名"，等价于显式 <'a>。
    // 就算完全没有宏，你实现 Display 也照样写 _。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// 二元运算符。
// Eq 是 PartialEq 的"子 trait"
// Eq 建立在 PartialEq 之上，Eq 存在的前提就是已经实现了 PartialEq。
// PartialEq 只提供 == / != 运算符，但不保证"自反性"（即不保证 a == a 永远成立）。
// Eq 是个空标记 trait，它的唯一含义是："我向你保证我的 == 是完全的等价关系（自反、对称、传递），
// 特别是 a == a 一定成立"。
// PartialEq 给 ==/!=，Eq 在此基础上承诺"完全等价"（自反）并解锁需要 Eq 的泛型 API；
// 而因为 Eq 依赖 PartialEq，所以两者必须成对写——Eq 离不开 PartialEq，PartialEq 单独写又不满足 Eq 约束
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
    // must_use:用来警告调用者必须使用某个返回值。如果调用者忽略了这个返回值，编译器会发出警告。
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
        // Box 在这里不是为了提高性能，而是必须——它用固定大小的指针取代了「内含自身」的无限嵌套
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
        // Vec<T> 在栈上只占固定大小（一个指针 + len + capacity，通常 24 字节），它真正的元素数据存在堆上
        args: Vec<Expr>,
    },
}

impl fmt::Display for Expr {
    /// 回显 AST：一律加括号，能直观看出优先级与结合性，调试时非常有用。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // write! 是 Rust 标准库（std）自带的宏，不是任何第三方 crate。
            // 它定义在 core::fmt 模块，并由 std::fmt 重导出，完整路径是 std::fmt::write!

            // write! 宏本身不需要手动导入——它是 macro_rules! 宏，
            // Rust 会自动把 std 里的宏放进作用域（属于「macro prelude」）。
            // 你文件里写的 use std::fmt; 是为了用 fmt::Formatter、fmt::Result、fmt::Display 
            // 这些类型/trait，而不是为了 write!。
            Self::Number(n) => write!(f, "{n}"),
            Self::Var(name) => write!(f, "{name}"),
            Self::Unary { op, expr } => write!(f, "({op}{expr})"),
            Self::Binary { op, lhs, rhs } => write!(f, "({lhs} {op} {rhs})"),
            Self::Call { name, args } => {
                // 迭代器 + join，比手写 for 循环拼字符串更地道
                // .map(ToString::to_string)：对每个 &Expr 调用 to_string() 转成 String。
                // ToString::to_string 是函数引用写法，等价于 |e| e.to_string()。

                // 变量遮蔽（shadowing）：右边的 args 是模式匹配绑定出来的 &Vec<Expr>，
                // 左边 let args 又声明了一个新的 Vec<String>，同名变量遮蔽了旧的。
                // 这样后面直接用 args.join(", ") 时拿到的就是字符串列表，不用另外起名

                // 一个类型只要实现了 Display，就自动、免费获得 ToString 实现，从而拥有 to_string() 方法。
                // 顺带一句,这里 to_string 把 Expr 转成 String，会调用你写的那个 fmt 方法。
                // 而 fmt 里遇到 Call 又会调用 args 里每个 Expr 的 to_string —— 所以这是一个递归的字符串生成过程，
                // 这也是为什么它能正确处理 pow(2, 10) 这种嵌套调用。

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

// #[cfg(test)] 是 Rust 的条件编译属性（conditional compilation attribute）。
// 它的意思是：只有当前正在运行测试（cargo test）时，才编译这个模块；
// 正常构建（cargo build / cargo run）时，整块代码被完全忽略，不参与编译。
// cfg = configuration，条件编译。
// test 是编译器内置的一个 cfg 标志：用 cargo test 编译时，test 标志为真；用 cargo build 编译时，它为假。
#[cfg(test)]
mod tests {
    // use super::*; 之后，测试可以直接调用同文件里未加 pub 的函数、结构体——这是"单元测试放同文件"的关键便利。
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
