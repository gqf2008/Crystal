// ============================================================================
// 粒子引擎 — ParticleEngine (对应 C# ParticleEngine.cs + Particles/)
// ============================================================================
//
// 粒子系统核心，支持多种粒子类型：
// 雾、雪、雨、沙尘、暴风雪、鸟群、花瓣等天气/环境效果。

use std::time::{Duration, Instant};

/// 混合模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Additive,
}

/// 粒子类型 (对应 C# ParticleType 枚举)
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

/// RGBA 颜色
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const YELLOW: Self = Self { r: 255, g: 255, b: 0, a: 255 };
    pub const DARK_RED: Self = Self { r: 139, g: 0, b: 0, a: 255 };
    pub const DEEP_SKY_BLUE: Self = Self { r: 0, g: 191, b: 255, a: 255 };
    pub const FIREBRICK: Self = Self { r: 178, g: 34, b: 34, a: 255 };
    pub const GOLDENROD: Self = Self { r: 218, g: 165, b: 32, a: 255 };
    pub const PURPLE: Self = Self { r: 128, g: 0, b: 128, a: 255 };
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };

    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// 粒子图像信息 (对应 C# ParticleImageInfo)
#[derive(Debug, Clone)]
pub struct ParticleImageInfo {
    /// 图像库索引
    pub library_index: usize,
    /// 基础帧索引
    pub base_index: i32,
    /// 帧数
    pub count: i32,
    /// 当前帧
    pub current_frame: i32,
    /// 尺寸
    pub size: (i32, i32),
    /// 每帧绘制间隔
    pub draw_frame_ms: u64,
    /// 下一帧时间
    pub next_frame: Instant,
    /// 动画持续时间
    pub duration_ms: u64,
    /// 开始时间
    pub start: Instant,
    /// 延迟
    pub delay_ms: u64,
}

impl ParticleImageInfo {
    pub fn new(library_index: usize, base_index: i32, count: i32, draw_ms: u64) -> Self {
        let now = Instant::now();
        let duration_ms = draw_ms * count as u64;
        Self {
            library_index,
            base_index,
            count,
            current_frame: 0,
            size: (512, 512), // 默认尺寸，实际应从库中获取
            draw_frame_ms: draw_ms,
            next_frame: now + Duration::from_millis(duration_ms / count.max(1) as u64),
            duration_ms,
            start: now,
            delay_ms: 0,
        }
    }

    /// 处理帧动画
    pub fn process_image(&mut self) {
        let now = Instant::now();
        if now < self.next_frame {
            return;
        }

        self.current_frame += 1;
        if self.current_frame >= self.count {
            self.current_frame = 0;
            self.start = now + Duration::from_millis(self.delay_ms);
        }
        let frame_duration = self.duration_ms / self.count.max(1) as u64;
        self.next_frame =
            self.start + Duration::from_millis(frame_duration * (self.current_frame + 1) as u64);
    }
}

/// 单个粒子 (对应 C# Particle)
#[derive(Debug, Clone)]
pub struct Particle {
    /// 图像信息
    pub image_info: ParticleImageInfo,
    /// 位置
    pub position: (f32, f32),
    /// 旧位置
    pub old_position: (f32, f32),
    /// 速度
    pub velocity: (f32, f32),
    /// 旧速度
    pub old_velocity: (f32, f32),
    /// 颜色
    pub color: Color,
    /// 大小缩放
    pub size: f32,
    /// 混合模式
    pub blend_mode: BlendMode,
    /// 是否使用混合
    pub blend: bool,
    /// 混合比率
    pub blend_rate: f32,
    /// 存活截止时间 (None = 永久)
    pub alive_until: Option<Instant>,
    /// 更新间隔
    pub update_delay: Duration,
    /// 下次更新时间
    pub next_update: Instant,
    /// 粒子子类型 (用于区分不同行为)
    pub sub_type: ParticleSubType,
    /// 屏幕尺寸 (用于边界检测)
    screen_size: (i32, i32),
}

/// 粒子子类型 (对应 C# 的 FogParticle, SandParticle, SnowParticle, FlowerParticle)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSubType {
    Default,
    Fog,
    Sand,
    Snow,
    Flower,
}

impl Particle {
    /// 创建新粒子
    pub fn new(image_info: ParticleImageInfo, screen_size: (i32, i32)) -> Self {
        Self {
            image_info,
            position: (0.0, 0.0),
            old_position: (0.0, 0.0),
            velocity: (0.0, 0.0),
            old_velocity: (0.0, 0.0),
            color: Color::WHITE,
            size: 1.0,
            blend_mode: BlendMode::Normal,
            blend: false,
            blend_rate: 0.4,
            alive_until: None,
            update_delay: Duration::from_millis(50),
            next_update: Instant::now(),
            sub_type: ParticleSubType::Default,
            screen_size,
        }
    }

    /// 更新粒子位置
    pub fn update(&mut self) {
        let now = Instant::now();
        if now < self.next_update {
            return;
        }
        self.next_update = now + self.update_delay;
        self.old_position = self.position;
        self.position.0 += self.velocity.0;
        self.position.1 += self.velocity.1;
        self.wrap_position();
    }

    /// 边界环绕 (对应 C# OnPositionChanged)
    fn wrap_position(&mut self) {
        let (w, h) = self.image_info.size;
        if w == 0 || h == 0 {
            return;
        }
        let (sw, sh) = self.screen_size;

        let x_wrap = self.x_wrap_distance(w, sw);
        let y_wrap = self.y_wrap_distance(h, sh);

        if self.position.1 < -(h as f32) * 2.0 {
            self.position.1 += y_wrap;
        } else if self.position.1 > sh as f32 + h as f32 {
            self.position.1 -= y_wrap;
        }
        if self.position.0 < -(w as f32) * 2.0 {
            self.position.0 += x_wrap;
        } else if self.position.0 > sw as f32 + w as f32 {
            self.position.0 -= x_wrap;
        }
    }

    fn x_wrap_distance(&self, tile_w: i32, screen_w: i32) -> f32 {
        let cols = (screen_w as f32 / tile_w as f32).ceil() as i32 + 2;
        (tile_w * cols) as f32
    }

    fn y_wrap_distance(&self, tile_h: i32, screen_h: i32) -> f32 {
        let rows = (screen_h as f32 / tile_h as f32).ceil() as i32 + 2;
        (tile_h * rows) as f32
    }

    /// 是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(deadline) = self.alive_until {
            Instant::now() > deadline
        } else {
            false
        }
    }

    /// 处理帧动画
    pub fn process_image(&mut self) {
        self.image_info.process_image();
    }

    /// 应用偏移 (地图滚动时)
    pub fn offset(&mut self, dx: f32, dy: f32) {
        // 雾粒子不受地图滚动影响
        if self.sub_type == ParticleSubType::Fog {
            return;
        }
        self.position.0 += dx;
        self.position.1 += dy;
    }
}

/// 粒子引擎 (对应 C# ParticleEngine)
pub struct ParticleEngine {
    /// 发射器位置
    pub emitter_location: (f32, f32),
    /// 粒子列表
    pub particles: Vec<Particle>,
    /// 可用的图像信息模板
    pub textures: Vec<ParticleImageInfo>,
    /// 强制速度 (风力等)
    pub force_velocity: (f32, f32),
    /// 是否生成新粒子
    pub generate_particles: bool,
    /// 下次生成时间
    pub next_particle_time: Instant,
    /// 下次速度更新时间
    pub next_velocity_time: Instant,
    /// 速度更新间隔
    pub next_velocity_update: Duration,
    /// 生成间隔
    pub update_delay: Duration,
    /// 粒子类型
    pub particle_type: ParticleType,
    /// 屏幕尺寸
    screen_size: (i32, i32),
}

impl ParticleEngine {
    /// 创建粒子引擎
    pub fn new(
        textures: Vec<ParticleImageInfo>,
        location: (f32, f32),
        particle_type: ParticleType,
        screen_size: (i32, i32),
    ) -> Self {
        Self {
            emitter_location: location,
            particles: Vec::new(),
            textures,
            force_velocity: (0.0, 0.0),
            generate_particles: true,
            next_particle_time: Instant::now(),
            next_velocity_time: Instant::now(),
            next_velocity_update: Duration::from_millis(500),
            update_delay: Duration::from_millis(50),
            particle_type,
            screen_size,
        }
    }

    /// 生成新粒子 (对应 C# GenerateNewParticle)
    pub fn generate_new_particle(&mut self) {
        if self.textures.is_empty() {
            return;
        }

        let tex_idx = rand_index(self.textures.len());
        let image_info = self.textures[tex_idx].clone();
        let mut particle = Particle::new(image_info, self.screen_size);

        match self.particle_type {
            ParticleType::Fog | ParticleType::FogCloud => {
                particle.color = Color::WHITE;
                particle.blend_rate = 0.4;
                particle.blend = false;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::Sand => {
                particle.color = Color::YELLOW;
                particle.blend_rate = 0.2;
                particle.blend = false;
                particle.sub_type = ParticleSubType::Sand;
            }
            ParticleType::Snow => {
                particle.color = Color::WHITE;
                particle.blend_rate = 1.0;
                particle.blend = true;
                particle.sub_type = ParticleSubType::Snow;
            }
            ParticleType::Rain => {
                particle.color = Color::from_argb(133, 255, 255, 255);
                particle.blend_rate = 1.0;
                particle.blend = true;
            }
            ParticleType::RedFog => {
                particle.color = Color::DARK_RED;
                particle.blend_rate = 0.2;
                particle.blend = false;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::BlueFog => {
                particle.color = Color::DEEP_SKY_BLUE;
                particle.blend_rate = 0.2;
                particle.blend = false;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::YellowFog => {
                particle.color = Color::YELLOW;
                particle.blend_rate = 0.25;
                particle.blend = false;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::Leaves => {
                particle.color = Color::GOLDENROD;
                particle.blend_rate = 0.1;
                particle.blend = true;
                particle.blend_mode = BlendMode::Normal;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::FireyLeaves => {
                particle.color = Color::FIREBRICK;
                particle.blend_rate = 1.0;
                particle.blend = true;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::PurpleLeaves => {
                particle.color = Color::PURPLE;
                particle.blend_rate = 0.1;
                particle.blend = true;
                particle.blend_mode = BlendMode::Normal;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::FlowersRain => {
                particle.color = Color::WHITE;
                particle.blend_rate = 0.5;
                particle.blend = false;
                particle.sub_type = ParticleSubType::Flower;
            }
            ParticleType::Blizzard => {
                particle.color = Color::from_argb(255, 172, 229, 238);
                particle.blend_rate = 0.2;
                particle.sub_type = ParticleSubType::Fog;
            }
            ParticleType::BlizzardFrost => {
                particle.color = Color::WHITE;
                particle.size = rand_f32();
                particle.alive_until =
                    Some(Instant::now() + Duration::from_secs(1 + rand_index(2) as u64));
                particle.blend = false;
                particle.blend_rate = 0.35;
                let (sw, sh) = self.screen_size;
                particle.position = (
                    rand_index(sw as usize) as f32,
                    rand_range(sh / 2, sh) as f32,
                );
                particle.velocity = (0.0, 3.0 * rand_index(3) as f32);
            }
            ParticleType::RedFogEmber => {
                particle.color = Color::DARK_RED;
                particle.size = rand_f32();
                particle.alive_until =
                    Some(Instant::now() + Duration::from_secs(1 + rand_index(2) as u64));
                particle.blend = true;
                particle.blend_rate = 0.35;
                let (sw, sh) = self.screen_size;
                particle.position = (
                    rand_index(sw as usize) as f32,
                    rand_range(sh / 2, sh) as f32,
                );
                particle.velocity = (0.0, -2.0 * rand_index(3) as f32);
            }
            ParticleType::WhiteEmber => {
                particle.color = Color::WHITE;
                particle.size = rand_f32();
                particle.alive_until =
                    Some(Instant::now() + Duration::from_secs(1 + rand_index(2) as u64));
                particle.blend = true;
                particle.blend_rate = 0.35;
                let (sw, sh) = self.screen_size;
                particle.position = (
                    rand_index(sw as usize) as f32,
                    rand_range(sh / 2, sh) as f32,
                );
                particle.velocity = (0.0, -2.0 * rand_index(3) as f32);
            }
            ParticleType::YellowEmber => {
                particle.color = Color::YELLOW;
                particle.size = rand_f32();
                particle.alive_until =
                    Some(Instant::now() + Duration::from_secs(1 + rand_index(2) as u64));
                particle.blend = true;
                particle.blend_rate = 0.35;
                let (sw, sh) = self.screen_size;
                particle.position = (
                    rand_index(sw as usize) as f32,
                    rand_range(sh / 2, sh) as f32,
                );
                particle.velocity = (0.0, -2.0 * rand_index(3) as f32);
            }
            ParticleType::FloatingFlower => {
                particle.color = Color::WHITE;
                particle.size = rand_f32();
                particle.alive_until =
                    Some(Instant::now() + Duration::from_secs(5 + rand_index(4) as u64));
                particle.blend = true;
                particle.blend_rate = 1.0;
                let (sw, sh) = self.screen_size;
                particle.position = (
                    rand_index(sw as usize) as f32,
                    rand_range(sh / 4, sh * 2) as f32,
                );
                particle.velocity = (-2.0 * rand_index(4) as f32, -2.0 * rand_index(3) as f32);
            }
            ParticleType::Bird => {
                particle.color = Color::WHITE;
                particle.size = rand_f32();
                particle.alive_until =
                    Some(Instant::now() + Duration::from_secs(1 + rand_index(2) as u64));
                particle.blend = true;
                particle.blend_rate = 0.35;
                let (sw, sh) = self.screen_size;
                particle.position = (
                    rand_index(sw as usize) as f32,
                    rand_range(sh / 4, sh * 2) as f32,
                );
                particle.velocity = (-2.0, -2.0 * rand_index(3) as f32);
            }
            ParticleType::None | ParticleType::Test => {}
        }

        self.particles.push(particle);
    }

    /// 处理粒子系统 (对应 C# Process)
    pub fn process(&mut self) {
        // 处理帧动画
        for p in &mut self.particles {
            p.process_image();
        }

        // 生成新粒子
        let now = Instant::now();
        if self.generate_particles && now >= self.next_particle_time {
            self.next_particle_time = now + self.update_delay;
            self.generate_new_particle();
        }

        // 更新并移除过期粒子
        let mut i = 0;
        while i < self.particles.len() {
            self.particles[i].update();
            if self.particles[i].is_expired() {
                self.particles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// 应用地图偏移 (对应 C# ParticlesOffSet)
    pub fn offset(&mut self, dx: f32, dy: f32) {
        for p in &mut self.particles {
            p.offset(dx, dy);
        }
    }

    /// 粒子数量
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// 清除所有粒子
    pub fn clear(&mut self) {
        self.particles.clear();
    }
}

// 简单的伪随机数生成器 (xorshift32, 不引入 rand 依赖)
use std::sync::atomic::{AtomicU32, Ordering};

static RNG_STATE: AtomicU32 = AtomicU32::new(0);

fn next_rng() -> u32 {
    let mut s = RNG_STATE.load(Ordering::Relaxed);
    if s == 0 {
        // 用时间戳初始化种子
        s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
            | 1; // 确保非零
    }
    // xorshift32
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    RNG_STATE.store(s, Ordering::Relaxed);
    s
}

fn rand_index(max: usize) -> usize {
    if max == 0 {
        return 0;
    }
    next_rng() as usize % max
}

fn rand_f32() -> f32 {
    (next_rng() % 1000) as f32 / 1000.0
}

fn rand_range(min: i32, max: i32) -> i32 {
    if max <= min {
        return min;
    }
    min + rand_index((max - min) as usize) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_type_creation() {
        let textures = vec![ParticleImageInfo::new(0, 0, 1, 50)];
        let mut engine =
            ParticleEngine::new(textures, (0.0, 0.0), ParticleType::Fog, (1024, 768));
        engine.generate_new_particle();
        assert_eq!(engine.particle_count(), 1);
        assert_eq!(engine.particles[0].sub_type, ParticleSubType::Fog);
    }

    #[test]
    fn test_particle_expiry() {
        let info = ParticleImageInfo::new(0, 0, 1, 50);
        let mut p = Particle::new(info, (1024, 768));
        p.alive_until = Some(Instant::now() - Duration::from_secs(1));
        assert!(p.is_expired());
    }

    #[test]
    fn test_particle_no_expiry() {
        let info = ParticleImageInfo::new(0, 0, 1, 50);
        let p = Particle::new(info, (1024, 768));
        assert!(!p.is_expired()); // alive_until = None means lives forever
    }

    #[test]
    fn test_engine_process_removes_expired() {
        let textures = vec![ParticleImageInfo::new(0, 0, 1, 50)];
        let mut engine =
            ParticleEngine::new(textures, (0.0, 0.0), ParticleType::None, (1024, 768));
        engine.generate_particles = false;

        let info = ParticleImageInfo::new(0, 0, 1, 50);
        let mut p = Particle::new(info, (1024, 768));
        p.alive_until = Some(Instant::now() - Duration::from_secs(1));
        engine.particles.push(p);

        assert_eq!(engine.particle_count(), 1);
        engine.process();
        assert_eq!(engine.particle_count(), 0);
    }

    #[test]
    fn test_color_from_argb() {
        let c = Color::from_argb(128, 255, 0, 0);
        assert_eq!(c.a, 128);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }
}
