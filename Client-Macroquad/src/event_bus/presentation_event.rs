// ============================================================================
// PresentationEvent - 表现层事件定义
// ============================================================================
//
// 职责：
// - 定义所有视觉/听觉表现相关的事件
// - 由 逻辑系统 产生，被 渲染系统/音效系统/粒子系统 消费
//
// 设计原则：
// - 一次性触发：这些事件通常是瞬时的（播放动画、音效）
// - 不影响游戏逻辑：纯表现层，关闭也不影响游戏运行
// - 独立于渲染实现：不依赖具体的渲染后端

use hecs::Entity;
use macroquad::prelude::Color;

// ============================================================================
// 表现层事件枚举
// ============================================================================

#[derive(Debug, Clone)]
pub enum PresentationEvent {
    // ========================================================================
    // 动画事件
    // ========================================================================
    
    /// 播放实体动画
    PlayAnimation {
        entity: Entity,
        animation: AnimationType,
        loop_mode: LoopMode,
    },
    
    /// 停止实体动画
    StopAnimation {
        entity: Entity,
        animation: AnimationType,
    },
    
    /// 播放动画特效（不绑定实体）
    PlayEffectAnimation {
        effect_type: EffectType,
        position: (f32, f32),
        duration: f32,
    },
    
    // ========================================================================
    // 粒子效果事件
    // ========================================================================
    
    /// 生成粒子效果
    SpawnParticle {
        particle_type: ParticleType,
        position: (f32, f32),
        velocity: Option<(f32, f32)>,
        duration: f32,
    },
    
    /// 生成跟随实体的粒子
    SpawnFollowParticle {
        entity: Entity,
        particle_type: ParticleType,
        offset: (f32, f32),
        duration: f32,
    },
    
    /// 停止粒子发射器
    StopParticleEmitter {
        emitter_id: u32,
    },
    
    // ========================================================================
    // 音效事件
    // ========================================================================
    
    /// 播放音效
    PlaySound {
        sound_id: String,
        position: Option<(f32, f32)>, // None = 全局音效
        volume: f32,
        pitch: f32,
    },
    
    /// 播放背景音乐
    PlayBackgroundMusic {
        music_id: String,
        fade_in_duration: f32,
    },
    
    /// 停止背景音乐
    StopBackgroundMusic {
        fade_out_duration: f32,
    },
    
    /// 播放环境音
    PlayAmbientSound {
        sound_id: String,
        volume: f32,
    },
    
    // ========================================================================
    // 相机特效事件
    // ========================================================================
    
    /// 相机震动
    CameraShake {
        intensity: f32,
        duration: f32,
        frequency: f32,
    },
    
    /// 相机缩放动画
    CameraZoom {
        target_zoom: f32,
        duration: f32,
        easing: EasingType,
    },
    
    /// 相机闪光
    CameraFlash {
        color: Color,
        duration: f32,
    },
    
    /// 相机跟随目标
    CameraFollowTarget {
        target: Entity,
        smooth: bool,
    },
    
    // ========================================================================
    // 屏幕特效事件
    // ========================================================================
    
    /// 屏幕闪烁
    ScreenFlash {
        color: Color,
        duration: f32,
    },
    
    /// 屏幕淡入
    ScreenFadeIn {
        duration: f32,
        from_color: Color,
    },
    
    /// 屏幕淡出
    ScreenFadeOut {
        duration: f32,
        to_color: Color,
    },
    
    /// 屏幕震动（独立于相机）
    ScreenShake {
        intensity: f32,
        duration: f32,
    },
    
    /// 后处理效果切换
    PostProcessToggle {
        effect_type: PostProcessType,
        enabled: bool,
    },
    
    // ========================================================================
    // 文字/UI特效事件
    // ========================================================================
    
    /// 飘字伤害
    FloatingText {
        text: String,
        position: (f32, f32),
        color: Color,
        font_size: f32,
        duration: f32,
    },
    
    /// 飘字治疗
    FloatingHeal {
        amount: i32,
        position: (f32, f32),
    },
    
    /// 飘字经验
    FloatingExperience {
        amount: i64,
        position: (f32, f32),
    },
    
    /// UI提示
    ShowToast {
        message: String,
        toast_type: ToastType,
        duration: f32,
    },
    
    // ========================================================================
    // 天气/环境特效事件
    // ========================================================================
    
    /// 改变天气
    ChangeWeather {
        weather_type: WeatherType,
        transition_duration: f32,
    },
    
    /// 改变光照
    ChangeLighting {
        light_level: f32,
        color_tint: Color,
        transition_duration: f32,
    },
    
    // ========================================================================
    // 技能/战斗特效事件
    // ========================================================================
    
    /// 施法特效
    SpellCastEffect {
        caster: Entity,
        spell_type: u8,
        target_position: Option<(f32, f32)>,
    },
    
    /// 命中特效
    HitEffect {
        target: Entity,
        hit_type: HitEffectType,
        position: (f32, f32),
    },
    
    /// 弹道特效
    ProjectileEffect {
        projectile_type: ProjectileType,
        from: (f32, f32),
        to: (f32, f32),
        speed: f32,
    },
}

// ============================================================================
// 辅助枚举
// ============================================================================

/// 动画类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationType {
    Idle,
    Walk,
    Run,
    Attack,
    Spell,
    Hit,
    Die,
    Revive,
}

/// 循环模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Once,           // 播放一次
    Loop,           // 循环播放
    PingPong,       // 往返播放
}

/// 特效类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    Explosion,
    Teleport,
    LevelUp,
    Heal,
    Buff,
}

/// 粒子类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    Fire,
    Smoke,
    Blood,
    Magic,
    Heal,
    Poison,
}

/// 缓动类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingType {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// 后处理效果类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostProcessType {
    Bloom,
    MotionBlur,
    ColorGrading,
    Vignette,
}

/// 提示类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Info,
    Success,
    Warning,
    Error,
}

/// 天气类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherType {
    Clear,
    Rain,
    Snow,
    Fog,
    Storm,
}

/// 命中特效类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitEffectType {
    Normal,
    Critical,
    Miss,
    Block,
}

/// 弹道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileType {
    Arrow,
    Fireball,
    Lightning,
    IceBolt,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_animation_types() {
        let idle = AnimationType::Idle;
        let walk = AnimationType::Walk;
        
        assert_ne!(idle, walk);
        assert_eq!(idle, AnimationType::Idle);
    }
    
    #[test]
    fn test_loop_modes() {
        let once = LoopMode::Once;
        let looping = LoopMode::Loop;
        
        assert_ne!(once, looping);
    }
}
