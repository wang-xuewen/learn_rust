# calc —— 命令行表达式计算器（Rust 练手项目 #1）

> 许可证：非商业使用免费，商业使用（含商业培训）须事先书面授权 —— 见仓库根目录 [`LICENSE`](../../LICENSE)。

一个「麻雀虽小五脏俱全」的解释器：把一行数学表达式字符串，变成 AST，再算出结果。
它是 `how_to.md` 中 **Level 1** 的第一个选题，目标是用最小体量覆盖最多的 Rust 核心概念。

```text
&str ──词法分析──▶ Vec<Token> ──语法分析──▶ AST ──求值──▶ f64
```

## 快速开始

```bash
# 一次性求值
cargo run -p calc -- '1 + 2 * 3'          # 7
cargo run -p calc -- '(1 + 2) ^ 3'        # 27
cargo run -p calc -- 'sqrt(3^2 + 4^2)'    # 5

# 交互式 REPL
cargo run -p calc

# 管道批量计算（每行一个表达式）
printf 'x = 10\nx ^ 2\n' | cargo run -p calc

# 编译成正式二进制
cargo build --release -p calc
./target/release/calc 'max(1, 9, 3)'
```

## 支持的语法

| 类别 | 语法 | 说明 |
| --- | --- | --- |
| 数字 | `42` `3.14` `.5` `1e6` `2.5e-3` | 支持小数与科学计数法 |
| 二元运算 | `+` `-` `*` `/` `%` `^` | `^` 是幂运算，**右结合** |
| 一元运算 | `-x` `+x` | 优先级介于乘法与幂之间（`-2^2 == -4`） |
| 括号 | `( )` | 改变优先级 |
| 变量 | `x = 1 + 2` | 赋值后可在后续行使用 |
| 常量 | `pi` `e` `tau` | 内置 |
| 上次结果 | `ans` | 每个表达式求值后自动更新 |
| 函数 | 见下 | 支持可变参数 |
| 注释 | `# 注释` | 整行忽略 |

内置函数（REPL 里输入 `:funcs` 可查看）：
`sqrt` `cbrt` `abs` `sin` `cos` `tan` `asin` `acos` `atan` `atan2`
`ln` `log2` `log10` `exp` `floor` `ceil` `round` `pow` `min` `max` `sum`

REPL 命令：`:help` `:vars` `:funcs` `:quit`（也可 Ctrl-D）。

## 退出码

| 码 | 含义 |
| --- | --- |
| `0` | 成功 |
| `1` | 求值失败（语法错误、除零、未知变量等） |
| `2` | 命令行用法错误（未知选项） |

## 项目结构

```text
projects/calc
├── Cargo.toml
├── README.md
├── src
│   ├── lib.rs       # crate 根：模块声明、公开 API、文档
│   ├── main.rs      # CLI 入口：参数解析、REPL、批量模式
│   ├── token.rs     # Token / TokenKind 定义
│   ├── lexer.rs     # 词法分析：&str -> Vec<Token>
│   ├── ast.rs       # AST 节点 + 运算符优先级表
│   ├── parser.rs    # 语法分析：Token -> AST（Pratt 解析）
│   ├── eval.rs      # 求值：AST -> f64（变量表、内置函数表）
│   ├── session.rs   # 会话层：一行输入 -> Output（带状态）
│   ├── fmt.rs       # 结果格式化
│   └── error.rs     # 统一错误类型 CalcError
└── tests
    ├── api.rs       # 集成测试：只用公开 API
    └── cli.rs       # 端到端测试：直接跑二进制，验证退出码与输出
```

## 这个项目练到了哪些 Rust 知识点

| 知识点 | 在本项目中的体现 |
| --- | --- |
| **所有权 / 借用 / 生命周期** | `Lexer<'a>` 只借用 `&'a str`，全程不复制输入；`peek()` 返回 `&Token` 而非 `Token` |
| **枚举 + 模式匹配** | `TokenKind`、`Expr`、`Stmt`、`Output` 全是带数据的枚举；`match` / `if let` / `matches!` 到处都是 |
| **错误处理** | 统一 `CalcError`（`thiserror`），全程 `?` 传播，库代码零 `unwrap`（用 clippy 强制） |
| **trait** | `Evaluate` 抽象「可求值」；`Display` 让 AST 能回显、让错误能直接打印 |
| **迭代器** | `Peekable`、`map` + `collect::<Result<Vec<_>,_>>()`、`iter().find()`、`sort_by` |
| **集合** | `HashMap` 存变量表；`Vec` 存 AST 子节点与参数 |
| **智能指针 / 递归类型** | AST 用 `Box<Expr>` 打破无限大小 |
| **函数指针 + 静态表** | 内置函数表 `static FUNCTIONS`，加函数只需加一行 |
| **String / &str** | 词法扫描、错误文案拼接、数字格式化 |
| **测试** | 54 个单元测试 + 24 个集成/端到端测试 + 4 个文档测试 |
| **工程化** | workspace、共享 lint、`cargo fmt`、`cargo clippy -D warnings`、GitHub Actions CI |

## 测试

```bash
cargo test -p calc                 # 全部测试（单元 + 集成 + 文档）
cargo test -p calc --lib           # 只看单元测试
cargo test -p calc --test cli      # 只看 CLI 端到端测试
cargo test -p calc --doc           # 只看文档示例
```

## 本地质量门禁（提交前必跑）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 生成文档

```bash
cargo doc -p calc --open
```

## 还能继续加什么（练手进阶）

按难度递增，每加一个都会逼你用新的 Rust 特性：

1. **比较与逻辑运算**：`>` `<` `==` `&&` `||`，需要引入「值类型」枚举（`Value::Num` / `Value::Bool`）—— 练枚举与类型建模。
2. **更多语法**：`,` 分隔的多表达式、`if` 三元表达式 —— 练 AST 扩展。
3. **常量折叠优化器**：遍历 AST 把 `2 * 3` 折叠成 `6` —— 练 visitor 模式与 `Cow` / 可变遍历。
4. **自定义函数**：`f(x) = x * 2` —— 练闭包与环境捕获。
5. **历史记录**：用 `rustyline` 替换 `read_line`，支持上下键 —— 练第三方 crate 集成。
6. **零依赖重写**：去掉 `thiserror`，手写 `Display` + `std::error::Error`，体会宏帮你做了什么。
7. **性能**：把 AST 编译成字节码 + 栈式虚拟机 —— 练 `Vec<u8>` 指令与 `match` 分发。
