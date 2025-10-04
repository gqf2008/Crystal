// particles/flower_particle.rs
// 
// 对应 Client/MirGraphics/Particles/FogParticle.cs 中的 FlowerParticle
// 
// 花瓣粒子 - 飘落的花瓣效果

use super::particle::{Particle, ParticleTrait};
use crate::graphics::particle_engine::ParticleImageInfo;

/// 花瓣粒子
/// 
/// C# 原版:
/// ```csharp
/// public class FlowerParticle : Particle
/// {
///     private static int xwidth = (int)(400 * (Math.Ceiling(Settings.ScreenWidth / 400M) + 2));
///     private static int ywidth = (int)(400 * (Math.Ceiling(Settings.ScreenHeight / 400M) + 2));
///     
///     public FlowerParticle(ParticleEngine engine, ParticleImageInfo image) {
///         Engine = engine;
///         ImageInfo = image;
///     }
///     
///     public override void Update() {
///         if (CMain.Now < NextUpdateTime) return;
///         NextUpdateTime = CMain.Now.AddMilliseconds(20);  // 注意：20ms，更快
///         Position += Velocity;
///     }
/// }
/// ```
pub struct FlowerParticle {
    base: Particle,
}

impl FlowerParticle {
    /// 创建新的花瓣粒子
    /// 
    /// C# 中在 ParticleEngine.GenerateNewParticle() 里创建:
    /// ```csharp
    /// case ParticleType.FlowersRain:
    ///     particle = new FlowerParticle(this, Textures[CMain.Random.Next(Textures.Count)]) {
    ///         Color = Color.White,
    ///         Size = 1F,
    ///         BlendRate = 0.5F,
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
        base.blend_rate = 0.5; // 50% 不透明度
        base.alive_time = i64::MAX;
        base.blend = false;
        base.update_delay = 20; // 注意：20ms，比其他粒子更快
        
        Self { base }
    }
}

impl ParticleTrait for FlowerParticle {
    fn update(&mut self) {
        self.base.update();
    }
    
    fn draw(
        &self,
        library: &mut crate::graphics::mlibrary::MLibrary,
        dx_manager: &mut crate::graphics::dx_manager::DXManager,
    ) -> std::io::Result<()> {
        self.base.draw(library, dx_manager)
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
