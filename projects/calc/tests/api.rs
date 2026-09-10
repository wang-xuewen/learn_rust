// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 zongge —— 非商业使用免费；商业使用（含商业培训）须事先书面授权，见仓库根目录 LICENSE。

//! 集成测试：只通过 crate 的公开 API 使用 `calc`，模拟真实调用方。
//!
//! 集成测试放在 `tests/` 下，编译成独立的可执行文件，
//! 因此只能访问 `pub` 接口 —— 这能顺带检查「公开 API 设计得够不够用」。

// 测试里为了方便允许 unwrap / expect / panic（库代码本身仍然禁止）
#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use calc::{eval, format_number, parse, tokenize, CalcError, Context, Evaluate, Output, Session};
use calc::{Expr, Stmt, TokenKind};

/// 浮点比较：避免直接用 `==` 比较二进制浮点结果。
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "期望 {expected}，实际 {actual}"
    );
}

#[test]
fn end_to_end_simple_arithmetic() {
    assert_eq!(eval("1 + 2").unwrap(), 3.0);
    assert_eq!(eval("10 / 4").unwrap(), 2.5);
    assert_close(eval("1 / 3").unwrap(), 0.333_333_333_3);
}

#[test]
fn end_to_end_precedence_and_parens() {
    assert_eq!(eval("2 + 3 * 4").unwrap(), 14.0);
    assert_eq!(eval("(2 + 3) * 4").unwrap(), 20.0);
    assert_eq!(eval("2 ^ 3 ^ 2").unwrap(), 512.0);
    assert_eq!(eval("-3 ^ 2").unwrap(), -9.0);
    assert_eq!(eval("7 % 4").unwrap(), 3.0);
}

#[test]
fn end_to_end_functions_and_constants() {
    assert_eq!(eval("sqrt(16)").unwrap(), 4.0);
    assert_eq!(eval("pow(2, 10)").unwrap(), 1024.0);
    assert_close(eval("sin(pi / 2)").unwrap(), 1.0);
    assert_close(eval("ln(e)").unwrap(), 1.0);
    assert_eq!(eval("max(3, 7, 5)").unwrap(), 7.0);
    assert_eq!(eval("sum(1, 2, 3, 4)").unwrap(), 10.0);
    assert_eq!(eval("abs(0 - 2.5)").unwrap(), 2.5);
}

#[test]
fn deep_nesting_does_not_overflow() {
    // 故意来 200 层括号，验证递归解析/求值的稳健性
    let expr = format!("{}{}{}", "(".repeat(200), "1 + 1", ")".repeat(200));
    assert_eq!(eval(&expr).unwrap(), 2.0);
}

#[test]
fn tokenize_is_public() {
    let tokens = tokenize("1 + x").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number(1.0),
            TokenKind::Plus,
            TokenKind::Ident("x".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn parse_produces_inspectable_ast() {
    let stmt = parse("1 + 2 * 3").unwrap();
    match stmt {
        Stmt::Expr(Expr::Binary { op, lhs, rhs }) => {
            assert_eq!(op.symbol(), "+");
            assert_eq!(lhs.to_string(), "1");
            assert_eq!(rhs.to_string(), "(2 * 3)");
        }
        other => panic!("期望二元表达式，实际是 {other:?}"),
    }
}

#[test]
fn evaluate_with_custom_context() {
    let mut ctx = Context::new();
    ctx.set("radius", 3.0);
    let expr = match parse("pi * radius ^ 2").unwrap() {
        Stmt::Expr(expr) => expr,
        other => panic!("期望表达式，实际是 {other:?}"),
    };
    assert_close(expr.evaluate(&ctx).unwrap(), 28.274_333_882_3);
}

#[test]
fn session_keeps_state_and_ans() {
    let mut session = Session::new();
    assert_eq!(session.eval_line("2 * 3").unwrap(), Output::Value(6.0));
    assert_eq!(session.eval_line("ans * ans").unwrap(), Output::Value(36.0));
    assert_eq!(
        session.eval_line("size = 4").unwrap(),
        Output::Assignment {
            name: "size".to_string(),
            value: 4.0,
        }
    );
    assert_eq!(session.eval_line("size ^ 2").unwrap(), Output::Value(16.0));
    assert_eq!(session.context().get("size"), Some(4.0));
}

#[test]
fn error_variants_are_informative() {
    assert_eq!(eval("1 / 0"), Err(CalcError::DivisionByZero));
    assert!(matches!(
        eval("nope"),
        Err(CalcError::UnknownVariable { .. })
    ));
    assert!(matches!(
        eval("f(1)"),
        Err(CalcError::UnknownFunction { .. })
    ));
    assert!(matches!(
        eval("sqrt(1, 2)"),
        Err(CalcError::WrongArity { .. })
    ));
    assert!(matches!(
        eval("sqrt(-4)"),
        Err(CalcError::DomainError { .. })
    ));
    assert!(matches!(eval("1 +"), Err(CalcError::UnexpectedEof { .. })));
    assert!(matches!(
        eval("1 @ 2"),
        Err(CalcError::UnexpectedChar { .. })
    ));
    assert_eq!(eval("   "), Err(CalcError::EmptyExpression));
}

#[test]
fn error_messages_are_user_friendly() {
    let err = eval("1 / 0").unwrap_err();
    assert_eq!(err.to_string(), "除数为 0");

    let err = eval("1 @ 2").unwrap_err();
    assert!(err.to_string().contains("@"));
}

#[test]
fn formatting_is_stable() {
    assert_eq!(format_number(1.0), "1");
    assert_eq!(format_number(1.5), "1.5");
    assert_eq!(format_number(0.1 + 0.2), "0.3");
    assert_eq!(format_number(f64::NAN), "NaN");
}

#[test]
fn regression_golden_cases() {
    // 一组「黄金用例」：以后重构解析器时，这里能第一时间发现行为变化
    let cases = [
        ("1+2*3", 7.0),
        ("(1+2)*3", 9.0),
        ("10-2-3", 5.0),
        ("100/10/2", 5.0),
        ("2*-3", -6.0),
        ("-2^2", -4.0),
        ("2^0.5", std::f64::consts::SQRT_2),
        ("sqrt(9)+abs(-4)", 7.0),
        ("round(3.6)", 4.0),
        ("floor(-1.2)", -2.0),
        ("min(5,2,9)", 2.0),
        ("1e3/2", 500.0),
    ];
    for (input, expected) in cases {
        assert_close(
            eval(input).unwrap_or_else(|e| panic!("{input} 求值失败: {e}")),
            expected,
        );
    }
}
