// particles/fog_particle.rs
// 
// 对应 Client/MirGraphics/Particles/FogParticle.cs
// 
// 【设计原则】严格照搬 C# 原版

use super::particle::{Particle, ParticleTrait};
use crate::graphics::particle_engine::ParticleImageInfo;

/// 雾粒子
/// 
/// C# 原版:
/// ```csharp
/// public class FogParticle : Particle
/// {
///     private static int xwidth = (int)(512 * (Math.Ceiling(Settings.ScreenWidth / 512M) + 2));
///     private static int ywidth = (int)(512 * (Math.Ceiling(Settings.ScreenHeight / 512M) + 2));
///     private Vector2 xreset = new Vector2(xwidth, 0);
///     private Vector2 yreset = new Vector2(0, ywidth);
///     
///     public FogParticle(ParticleEngine engine, ParticleImageInfo image) {
///         Engine = engine;
///         ImageInfo = image;
///     }
///     
///     public override void Update() {
///         if (CMain.Now < NextUpdateTime) return;
///         NextUpdateTime = CMain.Now.AddMilliseconds(50);
///         Position += Velocity;
///     }
/// }
/// ```
pub struct FogParticle {
    base: Particle,
}

impl FogParticle {
    /// 创建新的雾粒子
    /// 
    /// C# 中在 ParticleEngine.GenerateNewParticle() 里创建:
    /// ```csharp
    /// case ParticleType.Fog:
    ///     particle = new FogParticle(this, Textures[CMain.Random.Next(Textures.Count)]) {
    ///         Color = Color.White,
    ///         Size = 1F,
    ///         BlendRate = 0.4F,
    ///         AliveTime = DateTime.MaxValue,
    ///         Blend = false,
    ///     };
    ///     particles.Add(particle);
    /// ```
    pub fn new(image_info: ParticleImageInfo, screen_size: (i32, i32)) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        
        // 随机位置
        let position = (
            rng.random_range(0..screen_size.0) as f32,
            rng.random_range(0..screen_size.1) as f32,
        );
        
        let mut base = Particle::new(image_info, position, screen_size);
        
        // C# 中的初始化设置
        base.velocity = (
            0.2 * rng.random_range(0..=2) as f32,
            0.2 * rng.random_range(0..=2) as f32,
        );
        base.color = [1.0, 1.0, 1.0, 1.0]; // Color.White
        base.size = 1.0;
        base.blend_rate = 0.4;
        base.alive_time = i64::MAX; // DateTime.MaxValue
        base.blend = false;
        
        Self { base }
    }
    
    /// 自定义颜色的雾粒子
    pub fn with_color(image_info: ParticleImageInfo, screen_size: (i32, i32), color: [f32; 4]) -> Self {
        let mut fog = Self::new(image_info, screen_size);
        fog.base.color = color;
        fog
    }
    
    /// 获取基础粒子的可变引用
    pub fn base_mut(&mut self) -> &mut Particle {
        &mut self.base
    }
    
    /// 获取基础粒子的引用
    pub fn base(&self) -> &Particle {
        &self.base
    }
}

/// 实现 ParticleTrait
impl ParticleTrait for FogParticle {
    fn update(&mut self) {
        // C# 原版就是调用基类的 Update()
        // FogParticle 没有重写 Update()，包裹逻辑在 OnPositionChanged
        self.base.update();
    }
    
    fn draw(&self) {
        self.base.draw();
    }
    
    fn process_image(&mut self) {
        self.base.process_image();
    }
    
    fn on_particle_end(&mut self) {
        self.base.on_particle_end();
    }
    
    fn get_alive_time(&self) -> i64 {
        self.base.alive_time
    }
    
    fn get_position(&self) -> (f32, f32) {
        self.base.position
    }
    
    fn set_position(&mut self, pos: (f32, f32)) {
        self.base.set_position(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fog_particle_creation() {
        let image_info = ParticleImageInfo::new("Effects", 100, 1, 50);
        let particle = FogParticle::new(image_info, (800, 600));
        
        let pos = particle.base.position;
        assert!(pos.0 >= 0.0 && pos.0 <= 800.0);
        assert!(pos.1 >= 0.0 && pos.1 <= 600.0);
    }
}
