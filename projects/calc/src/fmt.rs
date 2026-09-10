//! 输出格式化：把 `f64` 渲染成人眼友好的字符串。
//!
//! 直接 `println!("{}", 0.1 + 0.2)` 会打印 `0.30000000000000004`，
//! 这里做一次「紧凑化」：整数不带小数点、小数最多 10 位、极大极小值走科学计数法。

/// 把浮点数格式化为紧凑、可读的字符串。
///
/// # 例子
///
/// ```
/// use calc::format_number;
///
/// assert_eq!(format_number(3.0), "3");
/// assert_eq!(format_number(0.5), "0.5");
/// assert_eq!(format_number(-0.0), "0");
/// ```
#[must_use]
pub fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    // -0.0 与 0.0 都显示成 0
    if value == 0.0 {
        return "0".to_string();
    }

    let abs = value.abs();
    if !(1e-9..1e15).contains(&abs) {
        return format!("{value:.6e}");
    }
    if value.fract() == 0.0 {
        return format!("{value:.0}");
    }

    let text = format!("{value:.10}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        text
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_have_no_decimal_point() {
        assert_eq!(format_number(3.0), "3");
        assert_eq!(format_number(-42.0), "-42");
    }

    #[test]
    fn negative_zero_is_normalized() {
        assert_eq!(format_number(-0.0), "0");
    }

    #[test]
    fn decimals_are_trimmed() {
        assert_eq!(format_number(0.5), "0.5");
        assert_eq!(format_number(1.25), "1.25");
        assert_eq!(format_number(1.0 / 3.0), "0.3333333333");
    }

    #[test]
    fn floating_point_noise_is_hidden() {
        assert_eq!(format_number(0.1 + 0.2), "0.3");
    }

    #[test]
    fn very_large_and_small_use_scientific_notation() {
        assert_eq!(format_number(1e20), "1.000000e20");
        assert!(format_number(1e-12).contains("e-12"));
    }

    #[test]
    fn special_values() {
        assert_eq!(format_number(f64::NAN), "NaN");
        assert_eq!(format_number(f64::INFINITY), "inf");
        assert_eq!(format_number(f64::NEG_INFINITY), "-inf");
    }
}
