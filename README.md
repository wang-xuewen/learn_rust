# learn_rust —— Rust 练手项目工作区

按 [`how_to.md`](./how_to.md) 的路线图，用**一个小项目练一个主题**的方式从「会语法」走到「有工程能力」。
这里用 Cargo **workspace** 组织：所有小项目放在 `projects/` 下，共享依赖版本、共享 lint 规则、一条命令统一编译/测试。

## 目录结构

```text
learn_rust/
├── Cargo.toml              # workspace 根：members = ["projects/*"]
├── README.md               # 本文件
├── how_to.md               # 学习路线图
├── .github/workflows/ci.yml# CI：fmt / clippy / test / release build
└── projects/
    └── calc/               # 项目 #1：命令行表达式计算器
```

## 已收录项目

| # | 项目 | 主题 | Level | 状态 |
| --- | --- | --- | --- | --- |
| 1 | [`calc`](./projects/calc) | 表达式解析、字符串、错误处理、match | L1 | ✅ 完成 |

后续候选（来自 `how_to.md` 的 Level 1 清单）：`wc` 复刻（文件 IO + 迭代器）、JSON 简易解析器（手写词法）、猜数字游戏（扩展版）。

## 常用命令

```bash
# 编译整个工作区
cargo build --workspace

# 跑全部测试（单元 + 集成 + 文档）
cargo test --workspace

# 只跑某个项目
cargo run -p calc -- '1 + 2 * 3'
cargo test -p calc

# 代码质量（提交/推送前必跑，CI 里也会跑）
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings

# 生成并打开文档
cargo doc --workspace --open
```

## 新增一个小项目

`members` 用了通配符，所以**不需要改根 `Cargo.toml`**（除非要加共享依赖）：

```bash
cd projects
cargo new 项目名            # 二进制项目
# 或 cargo new --lib 项目名 # 库项目

# 在新项目的 Cargo.toml 里继承工作区配置，保持风格统一
```

新项目的 `Cargo.toml` 建议这样写（直接复制 `projects/calc/Cargo.toml` 改名字即可）：

```toml
[package]
name = "项目名"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false

[dependencies]
# 优先复用工作区里已声明的版本：thiserror = { workspace = true }

[lints]
workspace = true
```

### 工作区统一的约定

- **共享元数据**：`version` / `edition` / `rust-version` / `license` 在根 `Cargo.toml` 的 `[workspace.package]` 里统一维护。
- **共享 lint**（在根 `Cargo.toml` 的 `[workspace.lints]`）：
  - `unsafe_code = "forbid"`：练手阶段禁止 unsafe；
  - `missing_docs` / `missing_debug_implementations`：逼自己写文档注释；
  - `clippy::unwrap_used` / `expect_used` / `panic`：库代码里不许 `unwrap`，必须把错误交给调用方。
    > 测试代码里可以局部 `#![allow(...)]` 放开，但要在注释里说明理由。
- **依赖只加必要的**：每引一个 crate，先问一句「标准库能不能做」。

## 许可证

见 [`LICENSE`](./LICENSE)：

- **免费**：个人学习、研究、实验、非商业项目，以及非营利/公立教育科研机构、政府机构的使用；可修改、可分发（需保留 LICENSE 与源码头部的 SPDX 标识）。
- **禁止**：未获书面授权的任何商业用途 —— 包括**商业培训（付费课程、企业内训、训练营）**、付费产品/SaaS、商业交付、出版与二次销售。
- **商业授权**：须事先取得书面授权（邮件确认即可），可协商费用；未经授权使用即构成侵权。

> 严格来说这不是 OSI 认证的开源许可证（开源定义不允许限制商业用途），而是「源码公开 + 免费非商业授权」，
> 基于 [PolyForm Noncommercial 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0) 并附加商业培训限制条款。

## 每个项目的完成标准

按「能部署到生产」的要求自查：

1. 模块拆分清晰（`mod` 分层，不把所有代码塞一个文件）；
2. 有自定义错误类型，不用 `unwrap` 掩盖问题；
3. 单元测试 + 集成测试 + 文档示例（`cargo test` 全绿）；
4. 公开 API 有 `///` 文档注释，`cargo doc` 能生成；
5. `cargo fmt --check` 与 `cargo clippy -D warnings` 均无告警；
6. 有 README：能跑起来、讲清设计、列出学到的知识点；
7. 提交前本地跑一遍「常用命令」里的质量门禁。
