`#[derive(Debug, Error, Clone, PartialEq)]` 这行（21 行）给 `CalcError` 枚举同时派生了 4 个 trait，各自有不同用途：

**`Error`**
来自 `thiserror` 宏，它自动为 `CalcError` 实现 `std::error::Error` trait，同时根据每个变体上的 `#[error("...")]` 生成 `Display`。没有它，这个类型就不能作为标准错误类型在 `?` 和 `Result` 体系里流通。

**`Debug`**
所有错误类型都应能 `println!("{:?}")` 或 `{:#?}` 打印出来用于调试。Rust 的 `std::error::Error` 要求 `Self: Debug`，所以这个是 `Error` 的硬依赖，缺了它编译都过不了。

**`Clone`**
让错误值可以被复制。上层（CLI/REPL）有时需要既 `?` 返回错误、又保留一份用于日志或自定义提示，或者把同一个错误在多处展示。没有 `Clone` 就只能 `move` 一次。因为所有字段（`char`、`usize`、`String`）本身都可 `Clone`，派生是无代价的。

**`PartialEq`**
让两个 `CalcError` 可以比较相等。主要用途是**单元测试**：你可以直接 `assert_eq!(parse("1e"), Err(CalcError::InvalidNumber {...}))` 来精确断言返回的是哪个错误、携带什么上下文。注意这里是 `PartialEq` 而不是 `Eq`，因为理论上 `f64` 字段之类的类型不满足 `Eq`，虽然本枚举字段都是 `char/usize/String`（其实满足 `Eq`），但 `thiserror` 派生习惯用 `PartialEq` 更通用稳妥。

简单说：`Error`+`Debug` 是参与标准错误体系的必要条件，`Clone`/`PartialEq` 是为上层使用便利和测试断言服务。四个都是零成本派生（编译器生成实现），不是随便写的模板。


f64::NAN 是 Rust 里 f64 类型的 NaN（Not a Number，非数）常量。NAN != NAN 意思是：把 NaN 和它自己用 == 比较，结果是 false。这违反直觉，但这是 IEEE 754 浮点标准规定的。