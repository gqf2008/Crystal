// particles/particle.rs
// 
// 对应 Client/MirGraphics/Particles/Particle.cs
// 
// 【设计原则】严格照搬 C# 原版，不过度抽象

use crate::graphics::particle_engine::{ParticleImageInfo, get_time};

/// 混合模式 (C# BlendMode enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Additive,
    InvLight,
}

/// 粒子基类
/// 
/// C# 原版:
/// ```csharp
/// public class Particle {
///     public ParticleImageInfo ImageInfo { get; set; }
///     public ParticleEngine Engine { get; set; }
///     public BlendMode BlendMode = BlendMode.NORMAL;
///     public Vector2 OldPosition = Vector2.Zero;
///     public Vector2 Position { get; set; }
///     public Vector2 OldVelocity = Vector2.Zero;
///     public Vector2 Velocity { get; set; }
///     public Color Color { get; set; }
///     public float Size { get; set; }
///     public DateTime AliveTime { get; set; }
///     public bool Blend { get; set; }
///     public float BlendRate { get; set; }
///     public TimeSpan UpdateDelay = TimeSpan.FromMilliseconds(50);
///     protected DateTime NextUpdateTime { get; set; }
///     protected DateTime NextDraw { get; set; }
/// }
/// ```
pub struct Particle {
    pub image_info: ParticleImageInfo,
    pub blend_mode: BlendMode,
    pub old_position: (f32, f32),
    pub position: (f32, f32),
    pub old_velocity: (f32, f32),
    pub velocity: (f32, f32),
    pub color: [f32; 4],  // RGBA
    pub size: f32,
    pub alive_time: i64,  // C# DateTime 用毫秒时间戳
    pub blend: bool,
    pub blend_rate: f32,
    pub update_delay: i64,
    pub next_update_time: i64,
    pub next_draw: i64,
    
    // 屏幕尺寸（用于包裹计算）
    screen_width: i32,
    screen_height: i32,
}

impl Particle {
    pub fn new(image_info: ParticleImageInfo, position: (f32, f32), screen_size: (i32, i32)) -> Self {
        let now = get_time();
        
        Self {
            image_info,
            blend_mode: BlendMode::Normal,
            old_position: (0.0, 0.0),
            position,
            old_velocity: (0.0, 0.0),
            velocity: (0.0, 0.0),
            color: [1.0, 1.0, 1.0, 1.0],
            size: 1.0,
            alive_time: now + 3600 * 1000, // 默认 1 小时
            blend: false,
            blend_rate: 1.0,
            update_delay: 50,
            next_update_time: now + 50,
            next_draw: now,
            screen_width: screen_size.0,
            screen_height: screen_size.1,
        }
    }
    
    /// C# Update() 方法:
    /// ```csharp
    /// public virtual void Update() {
    ///     if (CMain.Now < NextUpdateTime) return;
    ///     NextUpdateTime = CMain.Now + UpdateDelay;
    ///     Position += Velocity;
    /// }
    /// ```
    pub fn update(&mut self) {
        let now = get_time();
        if now < self.next_update_time {
            return;
        }
        
        self.next_update_time = now + self.update_delay;
        
        // 更新位置时触发 OnPositionChanged
        let new_pos = (
            self.position.0 + self.velocity.0,
            self.position.1 + self.velocity.1,
        );
        self.set_position(new_pos);
    }
    
    /// 设置位置并触发 OnPositionChanged
    pub fn set_position(&mut self, new_position: (f32, f32)) {
        if self.position == new_position {
            return;
        }
        
        self.old_position = self.position;
        self.position = new_position;
        self.on_position_changed();
    }
    
    /// C# OnPositionChanged() 方法:
    /// ```csharp
    /// protected virtual void OnPositionChanged() {
    ///     if (ImageInfo.Size.Height == 0 || ImageInfo.Size.Width == 0)
    ///         return;
    ///     
    ///     int xwidth = (int)(ImageInfo.Size.Width * (Math.Ceiling(Settings.ScreenWidth / (decimal)ImageInfo.Size.Width) + 2));
    ///     int ywidth = (int)(ImageInfo.Size.Height * (Math.Ceiling(Settings.ScreenHeight / (decimal)ImageInfo.Size.Height) + 2));
    ///     Vector2 xreset = new Vector2(xwidth, 0);
    ///     Vector2 yreset = new Vector2(0, ywidth);
    ///     
    ///     if (Position.Y < -ImageInfo.Size.Height * 2)
    ///         Position += yreset;
    ///     else if (Position.Y > Settings.ScreenHeight + ImageInfo.Size.Height)
    ///         Position -= yreset;
    ///     else if (Position.X < -ImageInfo.Size.Width * 2)
    ///         Position += xreset;
    ///     else if (Position.X > Settings.ScreenWidth + ImageInfo.Size.Width)
    ///         Position -= xreset;
    /// }
    /// ```
    pub fn on_position_changed(&mut self) {
        if self.image_info.height == 0 || self.image_info.width == 0 {
            return;
        }
        
        let w = self.image_info.width;
        let h = self.image_info.height;
        
        let xwidth = (w as f32 * ((self.screen_width as f32 / w as f32).ceil() + 2.0)) as i32;
        let ywidth = (h as f32 * ((self.screen_height as f32 / h as f32).ceil() + 2.0)) as i32;
        
        let xreset = (xwidth as f32, 0.0);
        let yreset = (0.0, ywidth as f32);
        
        if self.position.1 < -(h * 2) as f32 {
            self.position.0 += yreset.0;
            self.position.1 += yreset.1;
        } else if self.position.1 > (self.screen_height + h) as f32 {
            self.position.0 -= yreset.0;
            self.position.1 -= yreset.1;
        } else if self.position.0 < -(w * 2) as f32 {
            self.position.0 += xreset.0;
            self.position.1 += xreset.1;
        } else if self.position.0 > (self.screen_width + w) as f32 {
            self.position.0 -= xreset.0;
            self.position.1 -= xreset.1;
        }
    }
    
    /// C# Draw() 方法:
    /// ```csharp
    /// public void Draw() {
    ///     if (ImageInfo == null) return;
    ///     
    ///     int drawx = (int)Position.X;
    ///     int drawy = (int)Position.Y;
    ///     
    ///     if (Blend)
    ///         ImageInfo.Library.DrawBlend(ImageInfo.BaseIndex + ImageInfo.CurrentFrame, new Point(drawx, drawy), Color, true, BlendRate);
    ///     else
    ///         ImageInfo.Library.Draw(ImageInfo.BaseIndex + ImageInfo.CurrentFrame, new Point(drawx, drawy), Color, true, BlendRate);
    /// }
    /// ```
    pub fn draw(
        &self,
        library: &mut crate::graphics::mlibrary::MLibrary,
        dx_manager: &mut crate::graphics::dx_manager::DXManager,
    ) -> std::io::Result<()> {
        let index = self.image_info.base_index + self.image_info.current_frame;
        let pos = (self.position.0 as i32, self.position.1 as i32);
        
        if self.blend {
            library.draw_blend(dx_manager, index, pos, self.color, true, self.blend_rate)?;
        } else {
            library.draw(dx_manager, index, pos, self.color, true, self.blend_rate)?;
        }
        
        Ok(())
    }
    
    /// C# ProcessImage() 方法:
    /// ```csharp
    /// public void ProcessImage() {
    ///     if (CMain.Time <= ImageInfo.NextFrame) return;
    ///     
    ///     if (++ImageInfo.CurrentFrame >= ImageInfo.Count) {
    ///         ImageInfo.CurrentFrame = 0;
    ///         ImageInfo.Start = CMain.Time + ImageInfo.Delay;
    ///         ImageInfo.NextFrame = ImageInfo.Start + (ImageInfo.Duration / ImageInfo.Count) * (ImageInfo.CurrentFrame + 1);
    ///     } else {
    ///         ImageInfo.NextFrame = ImageInfo.Start + (ImageInfo.Duration / ImageInfo.Count) * (ImageInfo.CurrentFrame + 1);
    ///     }
    /// }
    /// ```
    pub fn process_image(&mut self) {
        let now = get_time();
        if now <= self.image_info.next_frame {
            return;
        }
        
        self.image_info.current_frame += 1;
        if self.image_info.current_frame >= self.image_info.count {
            self.image_info.current_frame = 0;
            self.image_info.start = now;
        }
        
        self.image_info.next_frame = self.image_info.start + 
            (self.image_info.duration / self.image_info.count) as i64 * 
            (self.image_info.current_frame + 1) as i64;
    }
    
    pub fn on_particle_end(&mut self) {
        // 子类可以重写
    }
}

/// 粒子 trait - 最小接口
/// 
/// 注意：这个 trait 只是为了让不同粒子类型能存储在同一个 Vec
/// C# 用继承，Rust 用 trait，但保持最小化
pub trait ParticleTrait {
    fn update(&mut self);
    fn draw(
        &self,
        library: &mut crate::graphics::mlibrary::MLibrary,
        dx_manager: &mut crate::graphics::dx_manager::DXManager,
    ) -> std::io::Result<()>;
    fn process_image(&mut self);
    fn on_particle_end(&mut self);
    fn get_alive_time(&self) -> i64;
    fn get_position(&self) -> (f32, f32);
    fn set_position(&mut self, pos: (f32, f32));
}

/// 为基础 Particle 实现 trait
impl ParticleTrait for Particle {
    fn update(&mut self) {
        Particle::update(self);
    }
    
    fn draw(
        &self,
        library: &mut crate::graphics::mlibrary::MLibrary,
        dx_manager: &mut crate::graphics::dx_manager::DXManager,
    ) -> std::io::Result<()> {
        Particle::draw(self, library, dx_manager)
    }
    
    fn process_image(&mut self) {
        Particle::process_image(self);
    }
    
    fn on_particle_end(&mut self) {
        Particle::on_particle_end(self);
    }
    
    fn get_alive_time(&self) -> i64 {
        self.alive_time
    }
    
    fn get_position(&self) -> (f32, f32) {
        self.position
    }
    
    fn set_position(&mut self, pos: (f32, f32)) {
        Particle::set_position(self, pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::particle_engine::ParticleImageInfo;
    
    #[test]
    fn test_particle_creation() {
        let image_info = ParticleImageInfo::new("Effects", 100, 1, 50);
        let particle = Particle::new(image_info, (100.0, 200.0), (800, 600));
        
        assert_eq!(particle.position, (100.0, 200.0));
        assert_eq!(particle.size, 1.0);
    }
    
    #[test]
    fn test_particle_update() {
        let image_info = ParticleImageInfo::new("Effects", 100, 1, 50);
        let mut particle = Particle::new(image_info, (0.0, 0.0), (800, 600));
        
        particle.velocity = (10.0, 20.0);
        particle.next_update_time = 0; // 强制立即更新
        particle.update();
        
        assert_eq!(particle.position, (10.0, 20.0));
    }
}
