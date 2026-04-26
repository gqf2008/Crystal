/// 定时机器人，对应 C# MirEnvir/Robot.cs
/// 定期触发 NPC 脚本页面实现自动化事件

use chrono::{Datelike, Timelike};

#[derive(Debug, Clone)]
pub struct RobotTask {
    /// 触发月份（None = 每月）
    pub month: Option<u32>,
    /// 触发日期（None = 每天）
    pub day: Option<u32>,
    /// 触发小时（None = 每小时）
    pub hour: Option<u32>,
    /// 触发分钟（None = 每分钟）
    pub minute: Option<u32>,
    /// 触发星期（0=Sun..6=Sat, None = 不限）
    pub day_of_week: Option<u32>,
    /// NPC 脚本页面名称
    pub page: String,
    /// 上次触发的时间（避免同分钟重复触发）
    pub last_fired: Option<(u32, u32, u32, u32)>, // (month, day, hour, minute)
}

impl RobotTask {
    pub fn new(page: String) -> Self {
        Self { month: None, day: None, hour: None, minute: None, day_of_week: None, page, last_fired: None }
    }

    /// 检查当前时间是否匹配
    pub fn should_fire(&self, now: &chrono::NaiveDateTime) -> bool {
        if let Some(ref last) = self.last_fired {
            if *last == (now.month(), now.day(), now.hour(), now.minute()) {
                return false; // already fired this minute
            }
        }
        if let Some(m) = self.month { if m != now.month() { return false; } }
        if let Some(d) = self.day { if d != now.day() { return false; } }
        if let Some(h) = self.hour {
            if h == 24 && now.hour() != 0 { return false; }
            if h < 24 && h != now.hour() { return false; }
        }
        if let Some(min) = self.minute { if min != now.minute() { return false; } }
        if let Some(dow) = self.day_of_week {
            if dow != now.weekday().num_days_from_sunday() { return false; }
        }
        true
    }

    pub fn mark_fired(&mut self, now: &chrono::NaiveDateTime) {
        self.last_fired = Some((now.month(), now.day(), now.hour(), now.minute()));
    }
}

/// 从 NPC 脚本页面名称解析 cron 参数
/// 格式: Robot_MM_DD_HH_MM_DOW_page
pub fn parse_robot(page_name: &str) -> Option<RobotTask> {
    if !page_name.starts_with("Robot_") {
        return None;
    }
    let rest = &page_name[6..]; // strip "Robot_"
    let parts: Vec<&str> = rest.splitn(6, '_').collect();
    if parts.len() < 6 {
        return None;
    }
    let mut task = RobotTask::new(parts[5].to_string());
    if let Ok(v) = parts[0].parse() { task.month = Some(v); }
    if let Ok(v) = parts[1].parse() { task.day = Some(v); }
    if let Ok(v) = parts[2].parse() { task.hour = Some(v); }
    if let Ok(v) = parts[3].parse() { task.minute = Some(v); }
    if let Ok(v) = parts[4].parse() { task.day_of_week = Some(v); }
    Some(task)
}
