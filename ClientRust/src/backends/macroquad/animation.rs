// ============================================================================
// Macroquad 动画系统
// ============================================================================
//
// 职责：
// - 帧序列播放
// - 循环/单次播放
// - 速度控制
// - 反向播放
// - 动画状态管理
//
// ============================================================================

use std::time::Duration;

/// 动画播放模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationMode {
    /// 循环播放
    Loop,

    /// 单次播放
    Once,

    /// 来回播放（乒乓模式）
    PingPong,
}

/// 帧动画
pub struct FrameAnimation {
    /// 帧索引列表
    frames: Vec<usize>,

    /// 当前帧索引（在 frames 数组中的位置）
    current_frame_index: usize,

    /// 每帧持续时间（秒）
    frame_duration: f32,

    /// 当前帧已经过的时间（秒）
    frame_elapsed: f32,

    /// 播放模式
    mode: AnimationMode,

    /// 是否正在播放
    playing: bool,

    /// 是否完成（仅对 Once 模式有效）
    finished: bool,

    /// 播放方向（true = 正向，false = 反向）
    forward: bool,
}

impl FrameAnimation {
    /// 创建新的帧动画
    ///
    /// # 参数
    /// - `frames`: 帧索引数组
    /// - `fps`: 每秒帧数
    /// - `mode`: 播放模式
    pub fn new(frames: Vec<usize>, fps: f32, mode: AnimationMode) -> Self {
        Self {
            frames,
            current_frame_index: 0,
            frame_duration: if fps > 0.0 { 1.0 / fps } else { 0.1 },
            frame_elapsed: 0.0,
            mode,
            playing: false,
            finished: false,
            forward: true,
        }
    }

    /// 创建循环动画
    pub fn looping(frames: Vec<usize>, fps: f32) -> Self {
        Self::new(frames, fps, AnimationMode::Loop)
    }

    /// 创建单次播放动画
    pub fn once(frames: Vec<usize>, fps: f32) -> Self {
        Self::new(frames, fps, AnimationMode::Once)
    }

    /// 创建乒乓动画
    pub fn ping_pong(frames: Vec<usize>, fps: f32) -> Self {
        Self::new(frames, fps, AnimationMode::PingPong)
    }

    /// 开始播放
    pub fn play(&mut self) {
        self.playing = true;
        self.finished = false;
    }

    /// 暂停播放
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// 停止播放并重置
    pub fn stop(&mut self) {
        self.playing = false;
        self.current_frame_index = 0;
        self.frame_elapsed = 0.0;
        self.finished = false;
        self.forward = true;
    }

    /// 重置到第一帧
    pub fn reset(&mut self) {
        self.current_frame_index = 0;
        self.frame_elapsed = 0.0;
        self.finished = false;
        self.forward = true;
    }

    /// 更新动画
    ///
    /// # 参数
    /// - `delta_time`: 距离上次更新的时间（秒）
    pub fn update(&mut self, delta_time: f32) {
        if !self.playing || self.finished || self.frames.is_empty() {
            return;
        }

        self.frame_elapsed += delta_time;

        // 检查是否需要切换帧
        while self.frame_elapsed >= self.frame_duration {
            self.frame_elapsed -= self.frame_duration;
            self.advance_frame();
        }
    }

    /// 前进到下一帧
    fn advance_frame(&mut self) {
        match self.mode {
            AnimationMode::Loop => {
                if self.forward {
                    self.current_frame_index = (self.current_frame_index + 1) % self.frames.len();
                } else {
                    if self.current_frame_index == 0 {
                        self.current_frame_index = self.frames.len() - 1;
                    } else {
                        self.current_frame_index -= 1;
                    }
                }
            }

            AnimationMode::Once => {
                if self.current_frame_index + 1 < self.frames.len() {
                    self.current_frame_index += 1;
                } else {
                    self.finished = true;
                    self.playing = false;
                }
            }

            AnimationMode::PingPong => {
                if self.forward {
                    if self.current_frame_index + 1 < self.frames.len() {
                        self.current_frame_index += 1;
                    } else {
                        self.forward = false;
                        if self.current_frame_index > 0 {
                            self.current_frame_index -= 1;
                        }
                    }
                } else {
                    if self.current_frame_index > 0 {
                        self.current_frame_index -= 1;
                    } else {
                        self.forward = true;
                        self.current_frame_index = 1.min(self.frames.len() - 1);
                    }
                }
            }
        }
    }

    /// 获取当前帧的图像索引
    pub fn current_frame(&self) -> usize {
        if self.frames.is_empty() {
            0
        } else {
            self.frames[self.current_frame_index]
        }
    }

    /// 设置播放速度（FPS）
    pub fn set_fps(&mut self, fps: f32) {
        self.frame_duration = if fps > 0.0 { 1.0 / fps } else { 0.1 };
    }

    /// 获取播放速度（FPS）
    pub fn fps(&self) -> f32 {
        if self.frame_duration > 0.0 {
            1.0 / self.frame_duration
        } else {
            10.0
        }
    }

    /// 设置播放模式
    pub fn set_mode(&mut self, mode: AnimationMode) {
        if self.mode != mode {
            self.mode = mode;
            self.reset();
        }
    }

    /// 获取播放模式
    pub fn mode(&self) -> AnimationMode {
        self.mode
    }

    /// 是否正在播放
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// 是否已完成（仅对 Once 模式有效）
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// 获取总帧数
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// 获取当前帧索引（在帧数组中的位置）
    pub fn current_frame_index(&self) -> usize {
        self.current_frame_index
    }

    /// 设置当前帧索引
    pub fn set_frame_index(&mut self, index: usize) {
        if index < self.frames.len() {
            self.current_frame_index = index;
            self.frame_elapsed = 0.0;
            self.finished = false;
        }
    }

    /// 反转播放方向
    pub fn reverse(&mut self) {
        self.forward = !self.forward;
    }

    /// 获取播放进度（0.0 ~ 1.0）
    pub fn progress(&self) -> f32 {
        if self.frames.is_empty() {
            return 0.0;
        }
        self.current_frame_index as f32 / (self.frames.len() - 1).max(1) as f32
    }
}

/// 动画状态机
///
/// 管理多个动画状态的切换
pub struct AnimationStateMachine {
    /// 动画状态集合：状态名 -> 动画
    states: std::collections::HashMap<String, FrameAnimation>,

    /// 当前状态名
    current_state: Option<String>,
}

impl AnimationStateMachine {
    pub fn new() -> Self {
        Self {
            states: std::collections::HashMap::new(),
            current_state: None,
        }
    }

    /// 添加动画状态
    pub fn add_state(&mut self, name: &str, animation: FrameAnimation) {
        self.states.insert(name.to_string(), animation);
    }

    /// 切换到指定状态
    pub fn set_state(&mut self, name: &str) -> bool {
        if self.states.contains_key(name) {
            // 停止当前动画
            if let Some(current_name) = &self.current_state {
                if let Some(anim) = self.states.get_mut(current_name) {
                    anim.stop();
                }
            }

            // 启动新动画
            if let Some(anim) = self.states.get_mut(name) {
                anim.play();
            }

            self.current_state = Some(name.to_string());
            true
        } else {
            false
        }
    }

    /// 更新当前动画
    pub fn update(&mut self, delta_time: f32) {
        if let Some(current_name) = &self.current_state {
            if let Some(anim) = self.states.get_mut(current_name) {
                anim.update(delta_time);
            }
        }
    }

    /// 获取当前帧
    pub fn current_frame(&self) -> Option<usize> {
        if let Some(current_name) = &self.current_state {
            self.states
                .get(current_name)
                .map(|anim| anim.current_frame())
        } else {
            None
        }
    }

    /// 获取当前状态名
    pub fn current_state_name(&self) -> Option<&str> {
        self.current_state.as_deref()
    }

    /// 获取指定状态的动画（可变）
    pub fn get_state_mut(&mut self, name: &str) -> Option<&mut FrameAnimation> {
        self.states.get_mut(name)
    }

    /// 获取指定状态的动画（不可变）
    pub fn get_state(&self, name: &str) -> Option<&FrameAnimation> {
        self.states.get(name)
    }
}

impl Default for AnimationStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_animation() {
        let mut anim = FrameAnimation::looping(vec![0, 1, 2, 3], 10.0);
        anim.play();

        assert_eq!(anim.current_frame(), 0);

        // 前进一帧
        anim.update(0.1);
        assert_eq!(anim.current_frame(), 1);

        // 前进到末尾并循环
        anim.update(0.3);
        assert_eq!(anim.current_frame(), 0);
    }

    #[test]
    fn test_once_animation() {
        let mut anim = FrameAnimation::once(vec![0, 1, 2], 10.0);
        anim.play();

        assert!(!anim.is_finished());

        // 播放完所有帧
        anim.update(0.3);

        assert!(anim.is_finished());
        assert!(!anim.is_playing());
    }

    #[test]
    fn test_ping_pong_animation() {
        let mut anim = FrameAnimation::ping_pong(vec![0, 1, 2], 10.0);
        anim.play();

        // 正向
        anim.update(0.1);
        assert_eq!(anim.current_frame(), 1);

        anim.update(0.1);
        assert_eq!(anim.current_frame(), 2);

        // 反向
        anim.update(0.1);
        assert_eq!(anim.current_frame(), 1);

        anim.update(0.1);
        assert_eq!(anim.current_frame(), 0);
    }
}
