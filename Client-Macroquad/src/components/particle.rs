// ============================================================================
// Particle Components - 粒子系统组件
// 参考 C# Client/MirGraphics/Particles/Particle.cs
// ============================================================================

/// 粒子组件
#[derive(Debug, Clone)]
pub struct Particle {
    /// 位置
    pub position: crate::components::core::Position,
    /// 速度 (像素/秒)
    pub velocity: crate::components::core::Position,
    /// 颜色
    pub color: ParticleColor,
    /// 大小
    pub size: f32,
    /// 存活至 (Unix时间戳)
    pub alive_until: f32,
    /// 混合模式
    pub blend_mode: BlendMode,
    /// 混合率
    pub blend_rate: f32,
    /// 图像信息
    pub image_info: Option<ParticleImageInfo>,
}

impl Particle {
    /// 创建新粒子
    pub fn new(x: f32, y: f32, vx: f32, vy: f32, lifetime: f32) -> Self {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();
        
        Self {
            position: crate::components::core::Position::new(x, y),
            velocity: crate::components::core::Position::new(vx, vy),
            color: ParticleColor { r: 255, g: 255, b: 255, a: 255 },
            size: 1.0,
            alive_until: current_time + lifetime,
            blend_mode: BlendMode::Normal,
            blend_rate: 1.0,
            image_info: None,
        }
    }
}

/// 粒子颜色 (RGBA)
#[derive(Debug, Clone, Copy)]
pub struct ParticleColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// 粒子图像信息
#[derive(Debug, Clone)]
pub struct ParticleImageInfo {
    /// 基础索引
    pub base_index: i32,
    /// 帧数
    pub frame_count: i32,
    /// 当前帧
    pub current_frame: i32,
    /// 帧间隔 (秒)
    pub frame_interval: f32,
    /// 下一帧时间
    pub next_frame_time: f32,
}

/// 混合模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Additive,
    Multiply,
    Screen,
}

/// 粒子发射器组件
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// 发射器位置
    pub emitter_location: crate::components::core::Position,
    /// 是否生成粒子
    pub generate_particles: bool,
    /// 下次生成时间
    pub next_particle_time: f32,
    /// 生成间隔 (秒)
    pub spawn_interval: f32,
    /// 外力速度 (如风力)
    pub force_velocity: crate::components::core::Position,
    /// 粒子类型
    pub particle_type: ParticleType,
}

/// 粒子类型 (参考 C# ParticleType 枚举)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    None,
    Fog,
    RedFog,
    RedFogEmber,
    BlueFog,
    YellowFog,
    WhiteEmber,
    YellowEmber,
    Test,
    Blizzard,
    BlizzardFrost,
    Bird,
    FogCloud,
    FloatingFlower,
    Sand,
    Snow,
    FlowersRain,
    Rain,
    Leaves,
    FireyLeaves,
    PurpleLeaves,
}
