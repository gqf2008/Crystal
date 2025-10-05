// particle_engine.rs
// 
// 对应 Client/MirGraphics/ParticleEngine.cs
// 
// 【设计原则】严格照搬 C# 原版，禁止过度抽象
// - 不使用 trait 对象（C# 用简单继承）
// - 使用 long 时间戳而非 Instant（对应 CMain.Time）
// - 直接翻译逻辑，不 Rust 化

use crate::graphics::LibraryName;

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
    /// 库名称（使用全局库管理器）
    pub library: LibraryName,
    
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
    pub fn new(library: LibraryName, index: i32, count: i32, draw_ms: i32) -> Self {
        let now = get_time();
        let duration = draw_ms * count;
        
        Self {
            library,
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
    
    // 直接使用 Vec<Particle>，与 C# List<Particle> 完全对应
    particles: Vec<crate::graphics::particles::Particle>,
    
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
        
        // 直接创建 Particle 并根据类型设置参数
        // 与 C# 的 GenerateNewParticle switch 逻辑完全对应
        let mut particle = crate::graphics::particles::Particle::new(
            texture, 
            (rng.random_range(0..self.screen_width) as f32, 
             rng.random_range(0..self.screen_height) as f32),
            screen_size
        );
        
        match self.particle_type {
            ParticleType::Fog => {
                // C#: Color = Color.White, Size = 1F, BlendRate = 0.4F, AliveTime = DateTime.MaxValue, Blend = false
                particle.color = [1.0, 1.0, 1.0, 1.0];
                particle.size = 1.0;
                particle.blend_rate = 0.4;
                particle.alive_time = i64::MAX;
                particle.blend = false;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::RedFog => {
                particle.color = [0.545, 0.0, 0.0, 1.0]; // Color.DarkRed
                particle.size = 1.0;
                particle.alive_time = i64::MAX;
                particle.blend_rate = 0.2;
                particle.blend = false;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::BlueFog => {
                particle.color = [0.0, 0.749, 1.0, 1.0]; // Color.DeepSkyBlue
                particle.size = 1.0;
                particle.alive_time = i64::MAX;
                particle.blend_rate = 0.2;
                particle.blend = false;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::YellowFog => {
                particle.color = [1.0, 1.0, 0.0, 1.0]; // Color.Yellow
                particle.size = 1.0;
                particle.blend_rate = 0.25;
                particle.alive_time = i64::MAX;
                particle.blend = false;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::FogCloud => {
                particle.color = [0.0, 0.0, 0.0, 0.0]; // Color.Transparent
                particle.size = 1.0;
                particle.blend_rate = 0.2;
                particle.alive_time = i64::MAX;
                particle.blend = false;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::Snow => {
                // C#: Color = Color.White, Size = 1F, BlendRate = 1F, AliveTime = DateTime.MaxValue, Blend = true
                particle.color = [1.0, 1.0, 1.0, 1.0];
                particle.size = 1.0;
                particle.blend_rate = 1.0;
                particle.alive_time = i64::MAX;
                particle.blend = true;
                particle.velocity = (
                    0.5 * rng.random_range(-2..=2) as f32,
                    1.0 + rng.random_range(0..3) as f32,
                );
            }
            
            ParticleType::Sand => {
                // C#: Color = Color.Yellow, Size = 1F, BlendRate = 0.2F, AliveTime = DateTime.MaxValue, Blend = false
                particle.color = [1.0, 1.0, 0.0, 1.0];
                particle.size = 1.0;
                particle.blend_rate = 0.2;
                particle.alive_time = i64::MAX;
                particle.blend = false;
                particle.velocity = (
                    1.0 + rng.random_range(0..3) as f32,
                    0.5 * rng.random_range(-1..=1) as f32,
                );
            }
            
            ParticleType::FlowersRain => {
                // C#: Color = Color.White, Size = 1F, BlendRate = 0.5F, AliveTime = DateTime.MaxValue, Blend = false
                particle.color = [1.0, 1.0, 1.0, 1.0];
                particle.size = 1.0;
                particle.blend_rate = 0.5;
                particle.alive_time = i64::MAX;
                particle.blend = false;
                particle.velocity = (
                    rng.random_range(-2..=2) as f32,
                    2.0 + rng.random_range(0..3) as f32,
                );
            }
            
            ParticleType::Leaves => {
                particle.color = [0.855, 0.647, 0.125, 1.0]; // Color.Goldenrod
                particle.blend_rate = 0.1;
                particle.alive_time = i64::MAX;
                particle.blend = true;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::FireyLeaves => {
                particle.color = [0.698, 0.133, 0.133, 1.0]; // Color.Firebrick
                particle.blend_rate = 1.0;
                particle.alive_time = i64::MAX;
                particle.blend = true;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::PurpleLeaves => {
                particle.color = [0.502, 0.0, 0.502, 1.0]; // Color.Purple
                particle.blend_rate = 0.1;
                particle.alive_time = i64::MAX;
                particle.blend = true;
                particle.velocity = (
                    0.2 * rng.random_range(0..=2) as f32,
                    0.2 * rng.random_range(0..=2) as f32,
                );
            }
            
            ParticleType::Rain => {
                particle.color = [1.0, 1.0, 1.0, 0.522]; // #ffffff85
                particle.size = 1.0;
                particle.blend_rate = 1.0;
                particle.alive_time = i64::MAX;
                particle.blend = true;
                particle.velocity = (0.0, 5.0 + rng.random_range(0..5) as f32);
            }
            
            // 其他类型暂时不实现
            _ => {
                return; // 不添加未实现的类型
            }
        }
        
        self.particles.push(particle);
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
            
            if now > particle.alive_time {
                particle.on_particle_end();
                return false;
            }
            true
        });
    }
    
    /// C# Draw() 方法
    /// 
    /// 使用全局库管理器，不再需要传入 library 参数
    /// 
    /// 优化版本:使用批处理渲染,大幅提升性能
    pub fn draw(
        &self,
        dx_manager: &mut crate::graphics::dx_manager::DXManager,
        screen_width: i32,
        screen_height: i32,
    ) -> std::io::Result<()> {
        use crate::graphics::get_library;
        
        // 使用GPU实例化模式收集所有粒子的实例数据
        for particle in &self.particles {
            // 从全局管理器获取对应的库
            let library_name = particle.image_info.library;
            if let Some(lib_arc) = get_library(library_name) {
                let mut library = lib_arc.lock().unwrap();
                particle.draw_instanced(&mut library, dx_manager, screen_width, screen_height)?;
            }
        }
        
        // 一次性渲染所有收集的粒子 (GPU实例化)
        dx_manager.flush_instanced_batch()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        Ok(())
    }
    
    /// C# ParticlesOffSet 方法
    pub fn particles_offset(&mut self, offset: (i32, i32)) {
        for particle in &mut self.particles {
            let (x, y) = particle.position;
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
        let info = ParticleImageInfo::new(LibraryName::Effect, 100, 5, 50);
        assert_eq!(info.base_index, 100);
        assert_eq!(info.count, 5);
        assert_eq!(info.current_frame, 0);
        assert_eq!(info.duration, 250);
    }
    
    #[test]
    fn test_particle_engine_creation() {
        let textures = vec![ParticleImageInfo::new(LibraryName::Effect, 100, 3, 50)];
        let engine = ParticleEngine::new(textures, (400.0, 300.0), ParticleType::Fog, 800, 600);
        
        assert_eq!(engine.particle_count(), 0);
        assert_eq!(engine.emitter_location, (400.0, 300.0));
    }
    
    #[test]
    fn test_generate_fog_particle() {
        let textures = vec![ParticleImageInfo::new(LibraryName::Effect, 100, 3, 50)];
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
            ParticleImageInfo::new(LibraryName::Effect, 100, 3, 50),
            ParticleImageInfo::new(LibraryName::Effect, 200, 5, 50),
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
        let textures = vec![ParticleImageInfo::new(LibraryName::Effect, 100, 3, 50)];
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
