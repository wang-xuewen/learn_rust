//! `calc` 的命令行入口。
//!
//! 三种使用方式：
//!
//! 1. `calc '1 + 2 * 3'` —— 一次求值后退出；
//! 2. `calc` —— 交互式 REPL（终端里）；
//! 3. `printf '1+1\n2*3\n' | calc` —— 非终端时自动按行批量计算。
//!
//! 退出码：`0` 成功，`1` 求值失败，`2` 命令行用法错误。

use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use calc::{format_number, functions, Output, Session};

/// 版本号来自 `Cargo.toml`，由编译器在编译期展开。
const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
calc —— 命令行表达式计算器

用法:
  calc <表达式>            计算表达式并打印结果
  calc                     交互式 REPL（终端中）
  calc -i, --interactive   强制进入交互式 REPL
  command | calc           从标准输入逐行批量计算（管道场景自动生效）

选项:
  -h, --help               显示帮助
  -V, --version            显示版本

例子:
  calc '1 + 2 * 3'                 -> 7
  calc 'sqrt(pow(3, 2) + 4 ^ 2)'   -> 5
  echo 'x = 10\nx ^ 2' | calc      -> 10 / 100
";

const REPL_HELP: &str = "\
命令:
  :help  :h  ?        显示本帮助
  :vars               列出当前变量
  :funcs              列出内置函数
  :quit  :q  :exit    退出（也可按 Ctrl-D）

语法:
  数字       42   3.14   1e6   .5
  运算符     +  -  *  /  %  ^        （^ 是幂运算，右结合）
  一元       -x   +x
  括号       ( )
  变量       x = 1 + 2              赋值后可在后续行使用
  常量       pi   e   tau
  上次结果   ans
  函数       sqrt(2)  pow(2, 10)  max(1, 9, 3)
  注释       以 # 开头的行会被忽略
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

/// 程序主体：把「退出码」显式建模成 `Result<(), u8>`，避免到处 `process::exit`。
fn run() -> Result<(), u8> {
    let mut interactive = false;
    let mut expr_parts: Vec<String> = Vec::new();

    // 直接消费 args 迭代器，避免无谓的 clone
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("calc {VERSION}");
                return Ok(());
            }
            "-i" | "--interactive" => interactive = true,
            other if other.starts_with('-') && other != "-" => {
                eprintln!("未知选项: {other}\n");
                eprint!("{USAGE}");
                return Err(2);
            }
            _ => expr_parts.push(arg),
        }
    }

    let mut session = Session::new();

    // 1) 命令行直接给了表达式：算完就走
    if !expr_parts.is_empty() {
        let line = expr_parts.join(" ");
        return match session.eval_line(&line) {
            Ok(output) => {
                print_output(&output);
                Ok(())
            }
            Err(err) => {
                eprintln!("错误: {err}");
                Err(1)
            }
        };
    }

    // 2) 终端（或显式 -i）：交互式 REPL
    if interactive || io::stdin().is_terminal() {
        return repl(&mut session).map_err(|err| {
            eprintln!("读取输入失败: {err}");
            1
        });
    }

    // 3) 管道：逐行批量计算
    batch(&mut session)
}

/// 交互式循环。
fn repl(session: &mut Session) -> io::Result<()> {
    println!("calc {VERSION} —— 输入表达式，:help 查看帮助，:quit 退出");

    let mut input = String::new();
    loop {
        print!("> ");
        // REPL 里必须手动 flush，否则提示符可能不显示
        io::stdout().flush()?;

        input.clear();
        let read = io::stdin().read_line(&mut input)?;
        if read == 0 {
            // EOF（Ctrl-D），友好地退出
            println!();
            break;
        }

        let line = input.trim();
        match line {
            "" => continue,
            ":q" | ":quit" | ":exit" => break,
            ":h" | ":help" | "?" => print!("{REPL_HELP}"),
            ":vars" => print_vars(session),
            ":funcs" => print_funcs(),
            _ => match session.eval_line(line) {
                Ok(output) => print_output(&output),
                Err(err) => eprintln!("错误: {err}"),
            },
        }
    }
    Ok(())
}

/// 批量模式：逐行读取 stdin 并计算，遇到错误继续（最后返回非零退出码）。
fn batch(session: &mut Session) -> Result<(), u8> {
    let mut failed = false;
    for line in io::stdin().lock().lines() {
        match line {
            Ok(line) => match session.eval_line(&line) {
                Ok(output) => print_output(&output),
                Err(err) => {
                    eprintln!("错误: {err}");
                    failed = true;
                }
            },
            Err(err) => {
                eprintln!("读取输入失败: {err}");
                return Err(1);
            }
        }
    }
    if failed {
        Err(1)
    } else {
        Ok(())
    }
}

/// 统一的结果打印逻辑：CLI 与 REPL 共用。
fn print_output(output: &Output) {
    match output {
        Output::Value(value) => println!("{}", format_number(*value)),
        Output::Assignment { name, value } => println!("{name} = {}", format_number(*value)),
        Output::Nothing => {}
    }
}

/// `:vars`：按名字排序打印变量（HashMap 本身无序，展示时需要排序）。
fn print_vars(session: &Session) {
    let mut vars: Vec<_> = session
        .context()
        .vars()
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect();
    vars.sort_by(|a, b| a.0.cmp(b.0));

    if vars.is_empty() {
        println!("（暂无变量）");
        return;
    }
    for (name, value) in vars {
        println!("{name} = {}", format_number(value));
    }
}

/// `:funcs`：列出内置函数及其说明。
fn print_funcs() {
    for func in functions() {
        println!("{:<7} {}", func.name, func.help);
    }
}
