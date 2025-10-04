// particle_engine.rs
// 
// 对应 Client/MirGraphics/ParticleEngine.cs
// 
// 【设计原则】严格照搬 C# 原版，禁止过度抽象
// - 不使用 trait 对象（C# 用简单继承）
// - 使用 long 时间戳而非 Instant（对应 CMain.Time）
// - 直接翻译逻辑，不 Rust 化

/// 粒子类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    None = 0,
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

/// 粒子图像信息
/// 
/// C# 原版:
/// ```csharp
/// public class ParticleImageInfo {
///     public MLibrary Library;
///     public Size Size = Size.Empty;
///     public int Count = 0;
///     public TimeSpan DrawFrameMS = TimeSpan.FromMilliseconds(50);
///     public int BaseIndex, Duration;
///     public long Start;
///     public int CurrentFrame;
///     public long NextFrame;
/// }
/// ```
#[derive(Clone)]
pub struct ParticleImageInfo {
    // 暂时用库名，后续集成 MLibrary 时替换
    pub library_name: String,
    
    pub base_index: i32,
    pub duration: i32,
    pub start: i64,
    pub current_frame: i32,
    pub next_frame: i64,
    pub count: i32,
    pub draw_frame_ms: i32,
    
    // Size
    pub width: i32,
    pub height: i32,
}

impl ParticleImageInfo {
    /// C# 构造函数:
    /// ```csharp
    /// public ParticleImageInfo(MLibrary file, int index, int count = 1, int drawMS = 50) {
    ///     BaseIndex = index;
    ///     Library = file;
    ///     Size = Library.GetSize(index);
    ///     Count = count;
    ///     DrawFrameMS = TimeSpan.FromMilliseconds(50);
    ///     Start = CMain.Time;
    ///     NextFrame = Start + (Duration / Count) * (CurrentFrame + 1);
    ///     Duration = drawMS * count;
    /// }
    /// ```
    pub fn new(library_name: impl Into<String>, index: i32, count: i32, draw_ms: i32) -> Self {
        let now = get_time();
        let duration = draw_ms * count;
        
        Self {
            library_name: library_name.into(),
            base_index: index,
            duration,
            start: now,
            current_frame: 0,
            next_frame: now + (duration / count) as i64,
            count,
            draw_frame_ms: draw_ms,
            width: 0,
            height: 0,
        }
    }
}

/// 粒子引擎
/// 
/// C# 原版:
/// ```csharp
/// public class ParticleEngine {
///     public Vector2 EmitterLocation { get; set; }
///     protected List<Particle> particles;
///     protected List<ParticleImageInfo> Textures;
///     public Vector2 ForceVelocity = Vector2.Zero;
///     public bool GenerateParticles;
///     public DateTime NextParticleTime;
///     public DateTime NextVelocityTime;
///     public TimeSpan NextVelocityUpdate = TimeSpan.FromMilliseconds(500);
///     public TimeSpan UpdateDelay = TimeSpan.FromMilliseconds(50);
///     ParticleType type;
/// }
/// ```
pub struct ParticleEngine {
    pub emitter_location: (f32, f32),
    pub textures: Vec<ParticleImageInfo>,
    pub force_velocity: (f32, f32),
    pub generate_particles: bool,
    pub next_particle_time: i64,
    pub next_velocity_time: i64,
    pub next_velocity_update: i64,
    pub update_delay: i64,
    
    particle_type: ParticleType,
    
    // 注意：C# 用 List<Particle>，这里先用 trait object
    // 阶段 2 可能改为 enum Particle { Fog(..), Snow(..), ... }
    particles: Vec<Box<dyn crate::graphics::particles::ParticleTrait>>,
    
    screen_width: i32,
    screen_height: i32,
}

impl ParticleEngine {
    pub fn new(
        textures: Vec<ParticleImageInfo>,
        location: (f32, f32),
        particle_type: ParticleType,
        screen_width: i32,
        screen_height: i32,
    ) -> Self {
        let now = get_time();
        
        Self {
            emitter_location: location,
            textures,
            force_velocity: (0.0, 0.0),
            generate_particles: true,
            next_particle_time: now,
            next_velocity_time: now + 500,
            next_velocity_update: 500,
            update_delay: 50,
            particle_type,
            particles: Vec::new(),
            screen_width,
            screen_height,
        }
    }
    
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
    
    /// 生成新粒子
    /// 
    /// C# 原版 GenerateNewParticle():
    /// ```csharp
    /// public virtual Particle GenerateNewParticle(ParticleType type) {
    ///     Particle particle = null;
    ///     switch (type) {
    ///         case ParticleType.Fog:
    ///             particle = new FogParticle(this, Textures[CMain.Random.Next(Textures.Count)]) {
    ///                 Color = Color.White,
    ///                 Size = 1F,
    ///                 BlendRate = 0.4F,
    ///                 AliveTime = DateTime.MaxValue,
    ///                 Blend = false,
    ///             };
    ///             particles.Add(particle);
    ///             break;
    ///         // ... 其他类型
    ///     }
    ///     return particle;
    /// }
    /// ```
    pub fn generate_new_particle(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();
        
        if self.textures.is_empty() {
            return;
        }
        
        // 随机选择纹理
        let texture_index = rng.random_range(0..self.textures.len());
        let texture = self.textures[texture_index].clone();
        let screen_size = (self.screen_width, self.screen_height);
        
        match self.particle_type {
            ParticleType::Fog => {
                let particle = crate::graphics::particles::FogParticle::new(texture, screen_size);
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::RedFog => {
                let particle = crate::graphics::particles::FogParticle::with_color(
                    texture, 
                    screen_size, 
                    [0.545, 0.0, 0.0, 1.0] // Color.DarkRed
                );
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::BlueFog => {
                let particle = crate::graphics::particles::FogParticle::with_color(
                    texture, 
                    screen_size, 
                    [0.0, 0.749, 1.0, 1.0] // Color.DeepSkyBlue
                );
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::YellowFog => {
                let mut particle = crate::graphics::particles::FogParticle::with_color(
                    texture, 
                    screen_size, 
                    [1.0, 1.0, 0.0, 1.0] // Color.Yellow
                );
                particle.base_mut().blend_rate = 0.25;
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::FogCloud => {
                let mut particle = crate::graphics::particles::FogParticle::with_color(
                    texture, 
                    screen_size, 
                    [0.0, 0.0, 0.0, 0.0] // Color.Transparent
                );
                particle.base_mut().blend_rate = 0.2;
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::Snow => {
                let particle = crate::graphics::particles::SnowParticle::new(texture, screen_size);
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::Sand => {
                let particle = crate::graphics::particles::SandParticle::new(texture, screen_size);
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::FlowersRain => {
                let particle = crate::graphics::particles::FlowerParticle::new(texture, screen_size);
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::Leaves => {
                let particle = crate::graphics::particles::FogParticle::with_color(
                    texture, 
                    screen_size, 
                    [0.855, 0.647, 0.125, 1.0] // Color.Goldenrod
                );
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::FireyLeaves => {
                let particle = crate::graphics::particles::FogParticle::with_color(
                    texture, 
                    screen_size, 
                    [0.698, 0.133, 0.133, 1.0] // Color.Firebrick
                );
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::PurpleLeaves => {
                let particle = crate::graphics::particles::FogParticle::with_color(
                    texture, 
                    screen_size, 
                    [0.502, 0.0, 0.502, 1.0] // Color.Purple
                );
                self.particles.push(Box::new(particle));
            }
            
            ParticleType::Rain => {
                // Rain 使用基础 Particle 类
                let mut particle = crate::graphics::particles::Particle::new(texture, self.emitter_location, screen_size);
                particle.color = [1.0, 1.0, 1.0, 0.522]; // #ffffff85
                particle.size = 1.0;
                particle.blend_rate = 1.0;
                particle.alive_time = i64::MAX;
                particle.blend = true;
                self.particles.push(Box::new(particle));
            }
            
            // 其他类型暂时不实现
            _ => {}
        }
    }
    
    /// 提供公共访问 base 的方法
    pub fn get_screen_size(&self) -> (i32, i32) {
        (self.screen_width, self.screen_height)
    }
    
    /// C# Process() 方法:
    /// ```csharp
    /// public void Process() {
    ///     foreach (var particle in particles)
    ///         particle.ProcessImage();
    ///     
    ///     if (GenerateParticles && CMain.Now > NextParticleTime) {
    ///         NextParticleTime = CMain.Now + UpdateDelay;
    ///         GenerateNewParticle(type);
    ///     }
    ///     
    ///     for (int particle = 0; particle < particles.Count; particle++) {
    ///         particles[particle].Update();
    ///         if (CMain.Now > particles[particle].AliveTime) {
    ///             particles[particle].OnParticleEnd();
    ///             particles.RemoveAt(particle);
    ///             particle--;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn process(&mut self) {
        let now = get_time();
        
        // Step 1: ProcessImage
        for particle in &mut self.particles {
            particle.process_image();
        }
        
        // Step 2: Generate particles
        if self.generate_particles && now > self.next_particle_time {
            self.next_particle_time = now + self.update_delay;
            self.generate_new_particle();
        }
        
        // Step 3: Update and remove dead particles
        self.particles.retain_mut(|particle| {
            particle.update();
            
            if now > particle.get_alive_time() {
                particle.on_particle_end();
                return false;
            }
            true
        });
    }
    
    /// C# Draw() 方法
    pub fn draw(&self) {
        for particle in &self.particles {
            particle.draw();
        }
    }
    
    /// C# ParticlesOffSet 方法
    pub fn particles_offset(&mut self, offset: (i32, i32)) {
        for particle in &mut self.particles {
            let (x, y) = particle.get_position();
            particle.set_position((x + offset.0 as f32, y + offset.1 as f32));
        }
    }
    
    pub fn clear(&mut self) {
        self.particles.clear();
        self.textures.clear();
    }
}

/// 获取当前时间 (毫秒)
/// 对应 C# 的 CMain.Time (long 类型)
pub fn get_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_particle_image_info() {
        let info = ParticleImageInfo::new("Effects", 100, 5, 50);
        assert_eq!(info.base_index, 100);
        assert_eq!(info.count, 5);
        assert_eq!(info.current_frame, 0);
        assert_eq!(info.duration, 250);
    }
    
    #[test]
    fn test_particle_engine_creation() {
        let textures = vec![ParticleImageInfo::new("Effects", 100, 3, 50)];
        let engine = ParticleEngine::new(textures, (400.0, 300.0), ParticleType::Fog, 800, 600);
        
        assert_eq!(engine.particle_count(), 0);
        assert_eq!(engine.emitter_location, (400.0, 300.0));
    }
    
    #[test]
    fn test_generate_fog_particle() {
        let textures = vec![ParticleImageInfo::new("Effects", 100, 3, 50)];
        let mut engine = ParticleEngine::new(textures, (400.0, 300.0), ParticleType::Fog, 800, 600);
        
        // 生成粒子
        engine.generate_new_particle();
        assert_eq!(engine.particle_count(), 1);
        
        // 再生成几个
        engine.generate_new_particle();
        engine.generate_new_particle();
        assert_eq!(engine.particle_count(), 3);
    }
    
    #[test]
    fn test_generate_different_particle_types() {
        let textures = vec![
            ParticleImageInfo::new("Effects", 100, 3, 50),
            ParticleImageInfo::new("Effects", 200, 5, 50),
        ];
        
        // 测试不同类型
        let types = vec![
            ParticleType::Fog,
            ParticleType::Snow,
            ParticleType::Sand,
            ParticleType::FlowersRain,
            ParticleType::RedFog,
            ParticleType::BlueFog,
        ];
        
        for particle_type in types {
            let mut engine = ParticleEngine::new(
                textures.clone(), 
                (400.0, 300.0), 
                particle_type, 
                800, 
                600
            );
            
            engine.generate_new_particle();
            assert_eq!(engine.particle_count(), 1, "Failed for type {:?}", particle_type);
        }
    }
    
    #[test]
    fn test_particle_engine_process() {
        let textures = vec![ParticleImageInfo::new("Effects", 100, 3, 50)];
        let mut engine = ParticleEngine::new(textures, (400.0, 300.0), ParticleType::Snow, 800, 600);
        
        // 初始没有粒子
        assert_eq!(engine.particle_count(), 0);
        
        // 强制设置下次生成时间为过去，确保立即生成
        engine.next_particle_time = 0;
        
        // 处理一次（应该生成新粒子）
        engine.process();
        assert_eq!(engine.particle_count(), 1);
        
        // 再强制生成
        engine.next_particle_time = 0;
        engine.process();
        assert_eq!(engine.particle_count(), 2);
    }
}
