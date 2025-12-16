// ============================================================================
// Sound Components - 音效组件
// ============================================================================
//
// 用于Layer 3到Layer 4的音效传递
//
// ============================================================================

/// 音效触发组件（Layer 3 写入，Layer 4 读取）
/// 
/// # 用途
/// - Layer 3的SoundTriggerSystem根据游戏事件决定应该播放什么音效
/// - Layer 4的SoundPlaybackSystem读取此组件并实际播放音效
/// - 播放后立即移除（一次性触发）
#[derive(Debug, Clone)]
pub struct SoundTrigger {
    /// 音效文件名（如 "attack.wav"）
    pub sound_file: String,
    
    /// 音效类型（用于分类管理）
    pub sound_type: SoundType,
    
    /// 音量（0.0-1.0）
    pub volume: f32,
    
    /// 是否循环播放
    pub looping: bool,
}

/// 临时音效发射器标记：用于一次性播放完即可销毁的实体。
///
/// 典型来源：服务器 PlaySound 包、UI 点击音效等。
#[derive(Debug, Clone, Copy, Default)]
pub struct OneShotSoundEmitter;

/// 攻击音效已触发标记（用于按帧触发音效的去重）
///
/// 说明：AttackState 会在攻击结束时被移除；此标记也会同步移除。
#[derive(Debug, Clone, Copy)]
pub struct AttackSoundPlayed {
    pub attack_start_time: std::time::Instant,
}

/// 挥砍/挥动音效已触发标记（用于怪物 SwingSound: base+4 的按帧触发去重）
#[derive(Debug, Clone, Copy)]
pub struct SwingSoundPlayed {
    pub attack_start_time: std::time::Instant,
}

impl SoundTrigger {
    /// 创建一次性音效触发
    pub fn once(sound_file: impl Into<String>, sound_type: SoundType) -> Self {
        Self {
            sound_file: sound_file.into(),
            sound_type,
            volume: 1.0,
            looping: false,
        }
    }
    
    /// 创建循环音效触发
    pub fn looping(sound_file: impl Into<String>, sound_type: SoundType) -> Self {
        Self {
            sound_file: sound_file.into(),
            sound_type,
            volume: 1.0,
            looping: true,
        }
    }
    
    /// 设置音量
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }
}

/// 音效类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundType {
    /// 背景音乐
    BackgroundMusic,
    
    /// 角色动作音效（攻击、受击、死亡）
    CharacterAction,
    
    /// 技能音效
    Spell,
    
    /// 物品音效（拾取、使用、装备）
    Item,
    
    /// UI音效（点击、打开对话框）
    UI,
    
    /// 环境音效（脚步声、环境音）
    Ambient,
    
    /// 系统音效（升级、任务完成）
    System,
}

impl Default for SoundTrigger {
    fn default() -> Self {
        Self {
            sound_file: String::new(),
            sound_type: SoundType::System,
            volume: 1.0,
            looping: false,
        }
    }
}

/// 持续音效组件（用于循环播放的环境音等）
/// 
/// # 与SoundTriggerComponent的区别
/// - SoundTrigger: 一次性触发，播放后移除
/// - PersistentSound: 持续存在，需要手动停止
#[derive(Debug, Clone)]
pub struct PersistentSound {
    /// 音效文件名
    pub sound_file: String,
    
    /// 音效类型
    pub sound_type: SoundType,
    
    /// 音量
    pub volume: f32,
    
    /// 是否正在播放
    pub is_playing: bool,
    
    /// 是否循环
    pub looping: bool,
}

impl PersistentSound {
    /// 创建持续音效
    pub fn new(sound_file: impl Into<String>, sound_type: SoundType, looping: bool) -> Self {
        Self {
            sound_file: sound_file.into(),
            sound_type,
            volume: 1.0,
            is_playing: true,
            looping,
        }
    }
    
    /// 停止播放
    pub fn stop(&mut self) {
        self.is_playing = false;
    }
    
    /// 开始播放
    pub fn play(&mut self) {
        self.is_playing = true;
    }
    
    /// 设置音量
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
}
