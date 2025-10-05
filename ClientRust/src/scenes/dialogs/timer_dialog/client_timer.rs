// Client Timer - 客户端计时器
// 对应C#的ClientTimer类

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