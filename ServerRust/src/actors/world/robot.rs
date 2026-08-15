/// 定时机器人，对应 C# MirEnvir/Robot.cs（#2572 接线激活）。
///
/// C# 语义（NPCScript.cs:280-285 + Robot.cs:79-112）：
/// - Robot 类型脚本（`<NPCPath>/00Robot.txt`）加载时，`[@_` 开头且含 "TIME" 的行
///   整行大写后交给 `Robot.AddRobot`；
/// - `AddRobot` 用正则 `\(([0-9#]{1,2}),([0-9#]{1,2}),([0-9#]{1,2}),([0-9#]{1,2}),([^\s]+)\)`
///   从行内提取五元组 (MM,DD,hh,mm,DOW)：前四组为 1-2 位数字或 `#`（通配），
///   第五组为星期名（SUNDAY..SATURDAY）或数字（不可解析 = 通配）；页 Key = 整行大写；
/// - `Robot.Process` 由 Envir 每分钟驱动，再按全部任务的最高时间粒度节流
///   （任一任务指定分钟 → 每分钟检查；否则任一指定小时 → 每小时；否则每天），
///   匹配当前 (月,日,时,分,星期) 的任务通过 `script.Call(页)` 执行其脚本段
///   （无玩家上下文的系统级动作，如 GLOBALMESSAGE/MONGEN/MONCLEAR）。
use chrono::{Datelike, Timelike};
use std::path::Path;

use super::npc_script::ParsedScript;

/// C# Settings.RobotNPCFilename = "00Robot"（Envir/NPCs/00Robot.txt）
const ROBOT_NPC_FILENAME: &str = "00Robot";

#[derive(Debug, Clone)]
pub struct RobotTask {
    /// 触发月份（None = 每月）
    pub month: Option<u32>,
    /// 触发日期（None = 每天）
    pub day: Option<u32>,
    /// 触发小时（None = 每小时；C# 语义下 24 永不匹配）
    pub hour: Option<u32>,
    /// 触发分钟（None = 每分钟）
    pub minute: Option<u32>,
    /// 触发星期（0=Sun..6=Sat, None = 不限）
    pub day_of_week: Option<u32>,
    /// NPC 脚本页面名（C# Robot.Page：整行大写，作为 `script.Call` 的段 Key）
    pub page: String,
    /// 上次触发的时间（同分钟去重；C# 由检查粒度天然保证，此处防御 tick 重复到达）
    pub last_fired: Option<(u32, u32, u32, u32)>, // (month, day, hour, minute)
}

impl RobotTask {
    pub fn new(page: String) -> Self {
        Self {
            month: None,
            day: None,
            hour: None,
            minute: None,
            day_of_week: None,
            page,
            last_fired: None,
        }
    }

    /// 检查当前时间是否匹配（C# Robot.IsMatch：非空字段全匹配才触发）
    pub fn should_fire(&self, now: &chrono::NaiveDateTime) -> bool {
        if let Some(ref last) = self.last_fired {
            if *last == (now.month(), now.day(), now.hour(), now.minute()) {
                return false; // already fired this minute
            }
        }
        if let Some(m) = self.month {
            if m != now.month() {
                return false;
            }
        }
        if let Some(d) = self.day {
            if d != now.day() {
                return false;
            }
        }
        if let Some(h) = self.hour {
            if h != now.hour() {
                return false;
            }
        }
        if let Some(min) = self.minute {
            if min != now.minute() {
                return false;
            }
        }
        if let Some(dow) = self.day_of_week {
            if dow != now.weekday().num_days_from_sunday() {
                return false;
            }
        }
        true
    }

    pub fn mark_fired(&mut self, now: &chrono::NaiveDateTime) {
        self.last_fired = Some((now.month(), now.day(), now.hour(), now.minute()));
    }
}

/// 解析触发窗字段：1-2 位且全为 `[0-9#]` 才合法（C# 正则 `[0-9#]{1,2}`）。
/// 全数字 → 具体值；含 `#` 或不可解析 → `None`（通配，C# int.TryParse 失败保持 null）。
/// 返回 `None` 表示该字段非法，整个五元组不匹配。
fn parse_time_field(s: &str) -> Option<Option<u32>> {
    if s.is_empty() || s.len() > 2 || !s.bytes().all(|b| b.is_ascii_digit() || b == b'#') {
        return None;
    }
    if s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse::<u32>().ok().map(Some)
    } else {
        Some(None) // 含 # → 通配
    }
}

/// 星期解析（C# `Enum.TryParse<DayOfWeek>(value, true)`）：
/// 星期名大小写不敏感；数字也接受（枚举 TryParse 行为），越界值永不匹配；
/// 其他值返回 `None`（通配）。
fn parse_day_of_week(s: &str) -> Option<u32> {
    if let Ok(v) = s.parse::<u32>() {
        return Some(v);
    }
    Some(match s {
        "SUNDAY" => 0,
        "MONDAY" => 1,
        "TUESDAY" => 2,
        "WEDNESDAY" => 3,
        "THURSDAY" => 4,
        "FRIDAY" => 5,
        "SATURDAY" => 6,
        _ => return None,
    })
}

/// 从脚本行解析机器人任务（C# Robot.AddRobot）。
///
/// 输入为原始脚本行（如 `[@_TIME(8,#,#,#,#)]`），在行内查找第一个形如
/// `(MM,DD,hh,mm,DOW)` 的五元组；`page` 为整行大写（C# 传入 AddRobot 前已 ToUpper）。
/// 未找到合法五元组返回 `None`（该行不注册任务）。
pub fn parse_robot(line: &str) -> Option<RobotTask> {
    let up = line.to_uppercase();
    let bytes = up.as_bytes();
    let mut pos = 0;
    while let Some(rel) = bytes[pos..].iter().position(|&b| b == b'(') {
        let start = pos + rel + 1; // '(' 之后
        let mut fields: Vec<&str> = Vec::with_capacity(5);
        let mut cur = start;
        let mut ok = true;
        // 前四组：1-2 位 [0-9#]，每组后紧跟 ','
        for _ in 0..4 {
            let field_start = cur;
            while cur < bytes.len()
                && cur - field_start < 2
                && (bytes[cur].is_ascii_digit() || bytes[cur] == b'#')
            {
                cur += 1;
            }
            let field = &up[field_start..cur];
            if cur >= bytes.len() || bytes[cur] != b',' || parse_time_field(field).is_none() {
                ok = false;
                break;
            }
            fields.push(field);
            cur += 1; // 跳过 ','
        }
        // 第五组：非空、无空白、以 ')' 结尾的串（C# `[^\s]+\)`）
        if ok {
            let dow_start = cur;
            let mut dow_end = cur;
            while dow_end < bytes.len() && !bytes[dow_end].is_ascii_whitespace() {
                if bytes[dow_end] == b')' {
                    break;
                }
                dow_end += 1;
            }
            if dow_end > dow_start && dow_end < bytes.len() && bytes[dow_end] == b')' {
                fields.push(&up[dow_start..dow_end]);
                let mut task = RobotTask::new(up.clone());
                if let Some(Some(v)) = parse_time_field(fields[0]) {
                    task.month = Some(v);
                }
                if let Some(Some(v)) = parse_time_field(fields[1]) {
                    task.day = Some(v);
                }
                if let Some(Some(v)) = parse_time_field(fields[2]) {
                    task.hour = Some(v);
                }
                if let Some(Some(v)) = parse_time_field(fields[3]) {
                    task.minute = Some(v);
                }
                task.day_of_week = parse_day_of_week(fields[4]);
                return Some(task);
            }
        }
        pos = start; // 从本次 '(' 之后继续找下一个 '('
    }
    None
}

/// 解析 Robot 脚本文本（C# NPCScript.ParseDefault 的 Robot 分支）。
///
/// `[@_` 开头且含 "TIME" 的行注册为触发窗（一行一窗，多行多窗）；
/// 同时整体解析为 `ParsedScript` 供触发时按页 Key 执行对应段。
pub fn parse_robot_script(content: &str) -> (Vec<RobotTask>, ParsedScript) {
    let mut tasks = Vec::new();
    for line in content.lines() {
        let up = line.to_uppercase();
        if !up.starts_with("[@_") || !up.contains("TIME") {
            continue;
        }
        if let Some(task) = parse_robot(line) {
            tasks.push(task);
        }
    }
    (tasks, ParsedScript::parse(content))
}

/// 从脚本目录读取 `00Robot.txt` 并解析（C# Envir 启动加载 RobotNPC）。
/// 文件缺失返回 `None`（等价 C# 未配置机器人脚本，不注册任何任务）。
pub fn load_robot_script(script_dir: &Path) -> Option<(Vec<RobotTask>, ParsedScript)> {
    let path = script_dir.join(format!("{}.txt", ROBOT_NPC_FILENAME));
    let content = std::fs::read_to_string(path).ok()?;
    Some(parse_robot_script(&content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    #[test]
    fn parse_time_line_with_wildcards() {
        // 官方 00Robot.txt 数据样例：仅指定月，其余通配
        let t = parse_robot("[@_TIME(8,#,#,#,#)]").expect("should parse");
        assert_eq!(t.month, Some(8));
        assert_eq!(t.day, None);
        assert_eq!(t.hour, None);
        assert_eq!(t.minute, None);
        assert_eq!(t.day_of_week, None);
        assert_eq!(t.page, "[@_TIME(8,#,#,#,#)]");
    }

    #[test]
    fn parse_time_line_dow_name() {
        // 小写行：页 Key 归一为大写（C# AddRobot 收到 ToUpper 后的行）
        let t = parse_robot("[@_time(#,13,#,#,Thursday)]").expect("should parse");
        assert_eq!(t.day, Some(13));
        assert_eq!(t.day_of_week, Some(4)); // C# DayOfWeek.Thursday = 4
        assert_eq!(t.page, "[@_TIME(#,13,#,#,THURSDAY)]");
    }

    #[test]
    fn parse_full_tuple() {
        let t = parse_robot("[@_TIME(12,25,0,17,Monday)]").expect("should parse");
        assert_eq!(
            (t.month, t.day, t.hour, t.minute, t.day_of_week),
            (Some(12), Some(25), Some(0), Some(17), Some(1))
        );
    }

    #[test]
    fn parse_numeric_dow() {
        // C# Enum.TryParse<DayOfWeek>("4") 接受数字 → Thursday
        let t = parse_robot("[@_TIME(#,#,#,#,4)]").expect("should parse");
        assert_eq!(t.day_of_week, Some(4));
    }

    #[test]
    fn parse_unknown_dow_is_wildcard() {
        // C# Enum.TryParse 失败 → DayOfWeek 保持 null（通配），任务仍注册
        let t = parse_robot("[@_TIME(#,#,#,#,NOSUCHDAY)]").expect("should parse");
        assert_eq!(t.day_of_week, None);
    }

    #[test]
    fn parse_rejects_invalid_lines() {
        // 无五元组
        assert!(parse_robot("[@_MAIN]").is_none());
        // 前四组只允许 1-2 位数字或 #（3 位月份不匹配，对齐 C# 正则 [0-9#]{1,2}）
        assert!(parse_robot("[@_TIME(100,#,#,#,#)]").is_none());
        // 只有四元组
        assert!(parse_robot("[@_TIME(#,#,#,#)]").is_none());
        // 前四组不接受其他通配符（如 *）
        assert!(parse_robot("[@_TIME(*,#,#,#,#)]").is_none());
        // 空白分隔的组不匹配（C# 正则要求 ',' 紧跟字段）
        assert!(parse_robot("[@_TIME(8, #, #, #, #)]").is_none());
    }

    #[test]
    fn parse_robot_script_official_data() {
        // 官方 Daneo1989/Envir/NPCs/00Robot.txt 全文（5 个触发窗全部登记）
        let content = "\
[@_TIME(8,#,#,#,#)]
#ACT
GLOBALMESSAGE \"I get called in august\" System

[@_TIME(#,13,#,#,#)]
#ACT
GLOBALMESSAGE \"I get called at day 13\" System

[@_TIME(#,#,12,#,#)]
#ACT
GLOBALMESSAGE \"I get called at 12PM\" System

[@_TIME(#,#,#,17,#)]
#ACT
GLOBALMESSAGE \"I get called at 17 minute\" System

[@_TIME(#,#,#,#,Thursday)]
#ACT
GLOBALMESSAGE \"I get called on Thursday\" System
";
        let (tasks, script) = parse_robot_script(content);
        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[0].month, Some(8));
        assert_eq!(tasks[1].day, Some(13));
        assert_eq!(tasks[2].hour, Some(12));
        assert_eq!(tasks[3].minute, Some(17));
        assert_eq!(tasks[4].day_of_week, Some(4));
        // 每个触发窗都能在解析后的脚本里找到对应执行段
        for t in &tasks {
            let key = t.page.trim_start_matches('[').trim_end_matches(']');
            assert!(script.find(key).is_some(), "section not found: {}", t.page);
        }
    }

    #[test]
    fn parse_robot_script_gates_on_prefix_and_time() {
        // C# ParseDefault 只处理 [@_ 前缀的行，且需含 "TIME"
        let (tasks, _) = parse_robot_script("[@TIME(1,1,1,1,MONDAY)]\n[@_OTHER]\n");
        assert!(tasks.is_empty());
    }

    #[test]
    fn should_fire_exact_and_wildcard() {
        let mut t = RobotTask::new(String::new());
        t.month = Some(8);
        t.day = Some(13);
        t.hour = Some(12);
        t.minute = Some(17);
        assert!(t.should_fire(&dt(2026, 8, 13, 12, 17)));
        assert!(!t.should_fire(&dt(2026, 8, 13, 12, 18))); // 分钟不符
        assert!(!t.should_fire(&dt(2026, 8, 14, 12, 17))); // 日不符
        assert!(!t.should_fire(&dt(2026, 9, 13, 12, 17))); // 月不符

        // 全通配任务任意时刻匹配
        let wildcard = RobotTask::new(String::new());
        assert!(wildcard.should_fire(&dt(2026, 1, 1, 0, 0)));
    }

    #[test]
    fn should_fire_window_boundaries() {
        // 进窗：16:59 不触发 → 17:00 触发；出窗：18:00 起不再触发
        let mut t = RobotTask::new(String::new());
        t.hour = Some(17);
        assert!(!t.should_fire(&dt(2026, 8, 13, 16, 59)));
        assert!(t.should_fire(&dt(2026, 8, 13, 17, 0)));
        assert!(t.should_fire(&dt(2026, 8, 13, 17, 1))); // 窗内后续分钟仍匹配（C# 混合粒度行为）
        assert!(!t.should_fire(&dt(2026, 8, 13, 18, 0)));
    }

    #[test]
    fn should_fire_minute_boundary() {
        let mut t = RobotTask::new(String::new());
        t.minute = Some(17);
        assert!(!t.should_fire(&dt(2026, 8, 13, 12, 16)));
        assert!(t.should_fire(&dt(2026, 8, 13, 12, 17)));
        assert!(!t.should_fire(&dt(2026, 8, 13, 12, 18)));
    }

    #[test]
    fn should_fire_dow() {
        let mut t = RobotTask::new(String::new());
        t.day_of_week = Some(4); // Thursday
        assert!(t.should_fire(&dt(2026, 8, 13, 0, 0))); // 2026-08-13 是周四
        assert!(!t.should_fire(&dt(2026, 8, 14, 0, 0))); // 周五
    }

    #[test]
    fn should_fire_marks_fired_once_per_minute() {
        let mut t = RobotTask::new(String::new());
        let now = dt(2026, 8, 13, 12, 17);
        assert!(t.should_fire(&now));
        t.mark_fired(&now);
        assert!(!t.should_fire(&now)); // 同一分钟不重复触发
        assert!(t.should_fire(&dt(2026, 8, 13, 12, 18))); // 下一分钟恢复
    }

    #[test]
    fn should_fire_hour_24_never_matches() {
        // C# 语义：Hour=24 时 date.Hour(0-23) 永不等于 24，任务永不触发
        let mut t = RobotTask::new(String::new());
        t.hour = Some(24);
        assert!(!t.should_fire(&dt(2026, 8, 13, 0, 0)));
        assert!(!t.should_fire(&dt(2026, 8, 13, 23, 59)));
    }
}
