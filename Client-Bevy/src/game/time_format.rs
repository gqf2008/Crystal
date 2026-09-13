//! C# `Functions` 时间格式移植（`Shared/Functions/Functions.cs`）
//!
//! 两处用到的都是「按总时长分档 + 各档不同精度」，且**档内格式并不统一**
//! （有的补零有的不补），因此逐字照抄，不合并成通用实现：
//! - `PrintTimeSpanFromSeconds`（:86-108）：宠物到期/黑石条悬停（批16）
//! - `PrintTimeSpanFromMilliSeconds`（:110-132）：技能栏 `SkillMpCooldownKey`（批17）

/// C# `Functions.PrintTimeSpanFromSeconds`（`accurate = true`）：
/// <1m → `{s}s`；<1h → `{m}m {s:02}s`；<1d → `{h}h {m:02}m {s:02}s`；
/// 否则 `{d}d {h:02}h {m:02}m {s:02}s`。
pub(crate) fn format_time_span(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    let s = total % 60;
    let m = (total / 60) % 60;
    let h = (total / 3600) % 24;
    let d = total / 86400;
    if total < 60 {
        format!("{s}s")
    } else if total < 3600 {
        format!("{m}m {s:02}s")
    } else if total < 86400 {
        format!("{h}h {m:02}m {s:02}s")
    } else {
        format!("{d}d {h:02}h {m:02}m {s:02}s")
    }
}

/// C# `Functions.PrintTimeSpanFromMilliSeconds`：
/// <1m → `{秒}.{毫秒/100 取整}s`（如 1500ms → `1.5s`、800ms → `0.8s`）；
/// <1h → `{TotalMinutes 小数}m {秒:02}s`（如 90000ms → `1.5m 30s`）；
/// <1d → `{TotalHours 取整}h {分:02}m {秒:02}s`；否则 `{天}d {时}h {分:02}m {秒:02}s`。
pub(crate) fn format_time_span_ms(ms: i64) -> String {
    let total = ms.max(0);
    let secs = total / 1000;
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let d = secs / 86400;
    if total < 60_000 {
        format!("{}.{}s", s, (total % 1000) / 100)
    } else if total < 3_600_000 {
        // C# `t.TotalMinutes` 是 double → 整数分钟时打印成 "1"、否则 "1.5"
        let total_minutes = secs as f64 / 60.0;
        if (total_minutes.fract()).abs() < f64::EPSILON {
            format!("{}m {s:02}s", total_minutes as i64)
        } else {
            format!("{total_minutes}m {s:02}s")
        }
    } else if total < 86_400_000 {
        format!("{h}h {m:02}m {s:02}s")
    } else {
        format!("{d}d {h}h {m:02}m {s:02}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 秒档四段（宠物到期 / 黑石条悬停）
    #[test]
    fn seconds_matches_csharp() {
        assert_eq!(format_time_span(0.0), "0s");
        assert_eq!(format_time_span(59.0), "59s");
        assert_eq!(format_time_span(61.0), "1m 01s");
        assert_eq!(format_time_span(3661.0), "1h 01m 01s");
        assert_eq!(format_time_span(90061.0), "1d 01h 01m 01s");
        assert_eq!(format_time_span(-5.0), "0s");
    }

    /// 毫秒档四段（技能栏冷却）：首档带十分之一秒、第二档保留 TotalMinutes 小数
    #[test]
    fn milliseconds_matches_csharp() {
        assert_eq!(format_time_span_ms(0), "0.0s");
        assert_eq!(format_time_span_ms(800), "0.8s");
        assert_eq!(format_time_span_ms(1500), "1.5s");
        assert_eq!(format_time_span_ms(59_999), "59.9s");
        assert_eq!(format_time_span_ms(60_000), "1m 00s");
        assert_eq!(format_time_span_ms(90_000), "1.5m 30s");
        assert_eq!(format_time_span_ms(3_600_000), "1h 00m 00s");
        assert_eq!(format_time_span_ms(3_661_000), "1h 01m 01s");
        assert_eq!(format_time_span_ms(90_061_000), "1d 1h 01m 01s");
    }
}
