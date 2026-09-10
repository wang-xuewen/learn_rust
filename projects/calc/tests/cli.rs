// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 zongge —— 非商业使用免费；商业使用（含商业培训）须事先书面授权，见仓库根目录 LICENSE。

//! 端到端测试：直接跑编译好的二进制，验证 CLI 行为（参数、退出码、stdout/stderr、管道）。
//!
//! `env!("CARGO_BIN_EXE_calc")` 由 cargo 在编译测试时注入可执行文件路径。

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::process::{Command, Stdio};

/// 拿到被测二进制的 `Command`。
fn calc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_calc"))
}

/// 运行并把 stdout / stderr 转成 `String`。
#[derive(Debug)]
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str], stdin: Option<&str>) -> Run {
    let mut command = calc();
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("无法启动 calc");
    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin 未捕获")
            .write_all(input.as_bytes())
            .expect("写入 stdin 失败");
    }
    let output = child.wait_with_output().expect("等待进程失败");

    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

#[test]
fn evaluates_expression_from_args() {
    let result = run(&["1 + 2 * 3"], None);
    assert_eq!(result.code, Some(0));
    assert_eq!(result.stdout, "7\n");
}

#[test]
fn multiple_args_are_joined() {
    let result = run(&["1", "+", "2"], None);
    assert_eq!(result.stdout, "3\n");
}

#[test]
fn assignment_prints_name_and_value() {
    let result = run(&["x = 1 + 2"], None);
    assert_eq!(result.code, Some(0));
    assert_eq!(result.stdout, "x = 3\n");
}

#[test]
fn function_and_constant_work_from_cli() {
    let result = run(&["sqrt(pow(3, 2) + 4 ^ 2)"], None);
    assert_eq!(result.stdout, "5\n");
}

#[test]
fn evaluation_error_exits_with_1_and_writes_stderr() {
    let result = run(&["1 / 0"], None);
    assert_eq!(result.code, Some(1));
    assert_eq!(result.stdout, "");
    assert!(result.stderr.contains("除数为 0"));
}

#[test]
fn syntax_error_exits_with_1() {
    let result = run(&["1 +"], None);
    assert_eq!(result.code, Some(1));
    assert!(result.stderr.contains("语法错误"));
}

#[test]
fn unknown_option_exits_with_2() {
    let result = run(&["--nope"], None);
    assert_eq!(result.code, Some(2));
    assert!(result.stderr.contains("未知选项"));
}

#[test]
fn help_exits_with_0_and_prints_usage() {
    for flag in ["-h", "--help"] {
        let result = run(&[flag], None);
        assert_eq!(result.code, Some(0));
        assert!(result.stdout.contains("用法"));
    }
}

#[test]
fn version_flag_prints_version() {
    let result = run(&["--version"], None);
    assert_eq!(result.code, Some(0));
    assert!(result.stdout.trim().starts_with("calc "));
}

#[test]
fn batch_mode_evaluates_each_line_from_stdin() {
    let result = run(&[], Some("1 + 1\n2 * 3\n"));
    assert_eq!(result.code, Some(0));
    assert_eq!(result.stdout, "2\n6\n");
}

#[test]
fn batch_mode_supports_variables_and_comments() {
    let result = run(&[], Some("# 注释会被忽略\nx = 10\nx ^ 2\n"));
    assert_eq!(result.code, Some(0));
    assert_eq!(result.stdout, "x = 10\n100\n");
}

#[test]
fn batch_mode_reports_error_but_keeps_going() {
    let result = run(&[], Some("1 + 1\n1 / 0\n3 * 3\n"));
    assert_eq!(result.code, Some(1));
    assert_eq!(result.stdout, "2\n9\n");
    assert!(result.stderr.contains("除数为 0"));
}
