// Timer Dialog - 计时器对话框
// 显示活动计时器 (如任务倒计时、活动时间等)

use std::collections::HashMap;

/// 计时器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    /// 默认 (不显示蛋形计时器图标)
    Default = 0,
    /// 蛋形计时器 (Index 960)
    EggTimer = 1,
    /// 特殊计时器 (Index 440)
    Special = 2,
}

/// 客户端计时器
#[derive(Debug, Clone)]
pub struct ClientTimer {
    /// 计时器唯一键
    pub key: String,
    /// 计时器类型
    pub timer_type: TimerType,
    /// 总秒数
    pub seconds: i32,
    /// 相对时间 (秒时间戳)
    pub relative_time: i64,
    /// 是否需要刷新
    pub refresh: bool,
}

impl ClientTimer {
    /// 创建新计时器
    pub fn new(key: String, seconds: i32, timer_type: TimerType) -> Self {
        let mut timer = Self {
            key,
            timer_type,
            seconds,
            relative_time: 0,
            refresh: false,
        };
        timer.update(seconds, timer_type);
        timer
    }

    /// 更新计时器
    pub fn update(&mut self, seconds: i32, timer_type: TimerType) {
        self.seconds = seconds;
        self.timer_type = timer_type;
        // 注意: relative_time 需要在调用处设置 (当前时间 / 1000 + seconds)
        self.refresh = true;
    }

    /// 获取剩余时间 (秒)
    pub fn get_remaining_seconds(&self, current_time_ms: i64) -> i32 {
        let current_seconds = current_time_ms / 1000;
        let remaining = self.relative_time - current_seconds;
        remaining as i32
    }

    /// 检查计时器是否过期
    pub fn is_expired(&self, current_time_ms: i64) -> bool {
        self.get_remaining_seconds(current_time_ms) <= 0
    }
}

/// 计时器对话框
/// 
/// 功能:
/// - 显示倒计时 (HH:MM 或 MM:SS 格式)
/// - 多个计时器管理 (显示最近的一个)
/// - 动画蛋形计时器图标
/// - 自动过期移除
#[derive(Debug, Clone)]
pub struct TimerDialog {
    /// 活动计时器列表
    pub active_timers: HashMap<String, ClientTimer>,
    /// 当前显示的计时器key
    pub current_timer_key: Option<String>,
    /// 计时器是否已启动
    pub timer_started: bool,
    /// 当前倒计时值 (秒)
    pub timer_counter: i32,
    /// 下次更新时间 (毫秒时间戳)
    pub timer_time: i64,
    /// 是否可见
    pub visible: bool,
    /// 位置 (x, y)
    pub position: (i32, i32),
    /// 是否显示蛋形计时器图标
    pub show_egg_timer: bool,
    /// 蛋形计时器动画循环
    pub egg_timer_loop: bool,
}

impl Default for TimerDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerDialog {
    /// 创建新的计时器对话框
    pub fn new() -> Self {
        // 位置: 屏幕右侧, 距离底部230像素
        // C#: Settings.ScreenWidth - 120, Settings.ScreenHeight - 230
        Self {
            active_timers: HashMap::new(),
            current_timer_key: None,
            timer_started: false,
            timer_counter: 0,
            timer_time: 0,
            visible: false,
            position: (1160, 570), // 假设1280x800屏幕
            show_egg_timer: false,
            egg_timer_loop: false,
        }
    }

    /// 添加或更新计时器
    pub fn add_timer(&mut self, key: String, seconds: i32, timer_type: TimerType, current_time_ms: i64) {
        if let Some(timer) = self.active_timers.get_mut(&key) {
            // 更新已有计时器
            timer.update(seconds, timer_type);
            timer.relative_time = current_time_ms / 1000 + seconds as i64;
            timer.refresh = true;
        } else {
            // 创建新计时器
            let mut timer = ClientTimer::new(key.clone(), seconds, timer_type);
            timer.relative_time = current_time_ms / 1000 + seconds as i64;
            self.active_timers.insert(key, timer);
        }
    }

    /// 使计时器过期 (立即结束)
    pub fn expire_timer(&mut self, key: &str) {
        if let Some(timer) = self.active_timers.get_mut(key) {
            timer.relative_time = 0;
            if self.current_timer_key.as_deref() == Some(key) {
                self.timer_counter = 0;
            }
        }
    }

    /// 移除计时器
    pub fn remove_timer(&mut self, key: &str) {
        self.active_timers.remove(key);
        if self.current_timer_key.as_deref() == Some(key) {
            self.current_timer_key = None;
            self.timer_started = false;
        }
    }

    /// 获取最佳计时器 (剩余时间最少的)
    pub fn get_best_timer(&self, current_time_ms: i64) -> Option<&ClientTimer> {
        self.active_timers.values()
            .filter(|t| !t.is_expired(current_time_ms))
            .min_by_key(|t| t.relative_time)
    }

    /// 获取指定计时器
    pub fn get_timer(&self, key: &str) -> Option<&ClientTimer> {
        self.active_timers.get(key)
    }

    /// 处理计时器逻辑 (每帧调用)
    pub fn process(&mut self, current_time_ms: i64) {
        let timer = self.get_best_timer(current_time_ms);

        if let Some(timer) = timer {
            let key = timer.key.clone();
            
            if self.current_timer_key.as_deref() != Some(&key) || timer.refresh {
                // 切换到新计时器或刷新现有计时器
                self.current_timer_key = Some(key);
                
                if let Some(current_timer) = self.active_timers.get_mut(self.current_timer_key.as_ref().unwrap()) {
                    current_timer.refresh = false;
                    self.timer_started = true;
                    self.timer_time = current_time_ms + 1000;
                    self.timer_counter = current_timer.get_remaining_seconds(current_time_ms);
                    self.update_time_graphic(current_timer.timer_type);
                }
            }
        } else {
            // 没有活动计时器
            self.current_timer_key = None;
            self.timer_started = false;
            self.visible = false;
            return;
        }

        if !self.timer_started || current_time_ms < self.timer_time {
            return;
        }

        // 每秒更新一次
        self.timer_counter -= 1;
        self.timer_time = current_time_ms + 1000;

        if self.timer_counter < 0 {
            // 计时器到期
            self.visible = false;
            self.egg_timer_loop = false;
            self.timer_started = false;

            if let Some(key) = &self.current_timer_key.clone() {
                self.active_timers.remove(key);
            }
            self.current_timer_key = None;
            return;
        }

        // 更新显示
        if let Some(key) = &self.current_timer_key {
            if let Some(timer) = self.active_timers.get(key) {
                self.update_time_graphic(timer.timer_type);
            }
        }
    }

    /// 更新时间显示
    fn update_time_graphic(&mut self, timer_type: TimerType) {
        let hours = self.timer_counter / 3600;
        let minutes = (self.timer_counter % 3600) / 60;
        let seconds = self.timer_counter % 60;

        // C#中会更新4个数字图像索引:
        // _1000, _100, _colon, _10, _1
        // 如果小时>0: HH:MM 格式
        // 否则: MM:SS 格式

        self.visible = true;
        self.show_egg_timer = timer_type != TimerType::Default;
        self.egg_timer_loop = true;
    }

    /// 获取时间显示字符串 (用于测试)
    pub fn get_time_string(&self) -> String {
        let hours = self.timer_counter / 3600;
        let minutes = (self.timer_counter % 3600) / 60;
        let seconds = self.timer_counter % 60;

        if hours > 0 {
            format!("{:02}:{:02}", hours, minutes)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// 清空所有计时器
    pub fn clear(&mut self) {
        self.active_timers.clear();
        self.current_timer_key = None;
        self.timer_started = false;
        self.visible = false;
    }

    /// 获取活动计时器数量
    pub fn timer_count(&self) -> usize {
        self.active_timers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_timer_new() {
        let timer = ClientTimer::new("test".to_string(), 60, TimerType::EggTimer);
        assert_eq!(timer.key, "test");
        assert_eq!(timer.seconds, 60);
        assert_eq!(timer.timer_type, TimerType::EggTimer);
        assert!(timer.refresh);
    }

    #[test]
    fn test_client_timer_remaining() {
        let mut timer = ClientTimer::new("test".to_string(), 60, TimerType::Default);
        timer.relative_time = 100; // 100秒时间戳
        
        assert_eq!(timer.get_remaining_seconds(50000), 50); // 50秒已过
        assert_eq!(timer.get_remaining_seconds(90000), 10); // 90秒已过
        assert!(!timer.is_expired(90000));
        assert!(timer.is_expired(100000));
    }

    #[test]
    fn test_timer_dialog_new() {
        let dialog = TimerDialog::new();
        assert!(!dialog.visible);
        assert!(!dialog.timer_started);
        assert_eq!(dialog.timer_count(), 0);
    }

    #[test]
    fn test_add_timer() {
        let mut dialog = TimerDialog::new();
        let current_time = 10000; // 10秒

        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, current_time);
        assert_eq!(dialog.timer_count(), 1);

        let timer = dialog.get_timer("quest1").unwrap();
        assert_eq!(timer.seconds, 60);
        assert_eq!(timer.relative_time, 70); // 10 + 60
    }

    #[test]
    fn test_update_timer() {
        let mut dialog = TimerDialog::new();
        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, 10000);
        
        // 更新计时器
        dialog.add_timer("quest1".to_string(), 120, TimerType::Special, 20000);
        assert_eq!(dialog.timer_count(), 1); // 还是1个

        let timer = dialog.get_timer("quest1").unwrap();
        assert_eq!(timer.seconds, 120);
        assert_eq!(timer.relative_time, 140); // 20 + 120
        assert_eq!(timer.timer_type, TimerType::Special);
    }

    #[test]
    fn test_expire_timer() {
        let mut dialog = TimerDialog::new();
        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, 10000);
        
        dialog.expire_timer("quest1");
        let timer = dialog.get_timer("quest1").unwrap();
        assert_eq!(timer.relative_time, 0);
        assert!(timer.is_expired(1000));
    }

    #[test]
    fn test_remove_timer() {
        let mut dialog = TimerDialog::new();
        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, 10000);
        assert_eq!(dialog.timer_count(), 1);

        dialog.remove_timer("quest1");
        assert_eq!(dialog.timer_count(), 0);
        assert!(dialog.get_timer("quest1").is_none());
    }

    #[test]
    fn test_get_best_timer() {
        let mut dialog = TimerDialog::new();
        let current_time = 10000;

        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, current_time);
        dialog.add_timer("quest2".to_string(), 30, TimerType::EggTimer, current_time);
        dialog.add_timer("quest3".to_string(), 90, TimerType::EggTimer, current_time);

        let best = dialog.get_best_timer(current_time).unwrap();
        assert_eq!(best.key, "quest2"); // 最短时间
        assert_eq!(best.relative_time, 40); // 10 + 30
    }

    #[test]
    fn test_process_starts_timer() {
        let mut dialog = TimerDialog::new();
        let current_time = 10000;

        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, current_time);
        assert!(!dialog.timer_started);

        dialog.process(current_time);
        assert!(dialog.timer_started);
        assert!(dialog.visible);
        assert_eq!(dialog.timer_counter, 60);
    }

    #[test]
    fn test_process_countdown() {
        let mut dialog = TimerDialog::new();
        let mut current_time = 10000;

        dialog.add_timer("quest1".to_string(), 5, TimerType::EggTimer, current_time);
        dialog.process(current_time);
        assert_eq!(dialog.timer_counter, 5);

        // 每秒递减
        for i in (0..5).rev() {
            current_time += 1000;
            dialog.process(current_time);
            if i > 0 {
                assert_eq!(dialog.timer_counter, i);
                assert!(dialog.visible);
            }
        }

        // 计时器到期
        current_time += 1000;
        dialog.process(current_time);
        assert!(!dialog.visible);
        assert!(!dialog.timer_started);
        assert_eq!(dialog.timer_count(), 0);
    }

    #[test]
    fn test_get_time_string() {
        let mut dialog = TimerDialog::new();
        
        dialog.timer_counter = 65; // 1分5秒
        assert_eq!(dialog.get_time_string(), "01:05");

        dialog.timer_counter = 3665; // 1小时1分5秒
        assert_eq!(dialog.get_time_string(), "01:01");

        dialog.timer_counter = 5; // 5秒
        assert_eq!(dialog.get_time_string(), "00:05");
    }

    #[test]
    fn test_clear() {
        let mut dialog = TimerDialog::new();
        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, 10000);
        dialog.add_timer("quest2".to_string(), 30, TimerType::EggTimer, 10000);
        assert_eq!(dialog.timer_count(), 2);

        dialog.clear();
        assert_eq!(dialog.timer_count(), 0);
        assert!(!dialog.visible);
    }

    #[test]
    fn test_multiple_timers_switch() {
        let mut dialog = TimerDialog::new();
        let current_time = 10000;

        dialog.add_timer("quest1".to_string(), 60, TimerType::EggTimer, current_time);
        dialog.add_timer("quest2".to_string(), 30, TimerType::Special, current_time);

        // 应该显示最短的计时器
        dialog.process(current_time);
        assert_eq!(dialog.current_timer_key.as_deref(), Some("quest2"));

        // quest2 过期后应该切换到 quest1
        dialog.expire_timer("quest2");
        dialog.remove_timer("quest2");
        dialog.process(current_time + 1000);
        assert_eq!(dialog.current_timer_key.as_deref(), Some("quest1"));
    }
}
