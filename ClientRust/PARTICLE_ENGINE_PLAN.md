# ParticleEngine 实现计划

**日期**: 2025年10月5日  
**预估工作量**: 8-12 小时  
**优先级**: 🟢 中等（游戏特效系统）

---

## 📋 概述

ParticleEngine 是 MirGraphics 的粒子特效系统，用于渲染：
- 天气效果（雾、雨、雪、风沙）
- 环境特效（叶子飘落、花瓣飘落）
- 战斗特效（火焰、爆炸等）

### C# 原版结构

```
Client/MirGraphics/
├── ParticleEngine.cs        (440 行) - 粒子引擎主类
└── Particles/
    ├── Particle.cs          (162 行) - 基础粒子类
    ├── FogParticle.cs       (xxx 行) - 雾粒子
    ├── SnowParticle.cs      (xxx 行) - 雪粒子
    └── ... 其他粒子类型
```

---

## 🎯 核心功能分析

### 1. ParticleImageInfo
```csharp
public class ParticleImageInfo {
    public MLibrary Library;      // 图像库
    public int BaseIndex;          // 起始索引
    public int Count;              // 帧数
    public int CurrentFrame;       // 当前帧
    public TimeSpan DrawFrameMS;   // 帧间隔
    public Size Size;              // 粒子尺寸
}
```

**功能**: 粒子动画信息

### 2. Particle (基类)
```csharp
public class Particle {
    public Vector2 Position;       // 位置
    public Vector2 Velocity;       // 速度
    public Color Color;            // 颜色
    public float Size;             // 大小
    public float BlendRate;        // 混合率
    public bool Blend;             // 是否混合
    public DateTime AliveTime;     // 存活时间
    
    public virtual void Update();  // 更新位置
    public void Draw();            // 绘制
}
```

**功能**: 
- 粒子物理模拟（位置 += 速度）
- 屏幕边界循环（从上飘出，从下进入）
- 动画播放
- 渲染

### 3. ParticleEngine
```csharp
public class ParticleEngine {
    public Vector2 EmitterLocation;       // 发射器位置
    protected List<Particle> particles;   // 粒子列表
    public Vector2 ForceVelocity;        // 全局力场
    public bool GenerateParticles;        // 是否生成新粒子
    
    public virtual Particle GenerateNewParticle(ParticleType type);
    public virtual void Update();
    public virtual void Draw();
}
```

**功能**:
- 管理粒子生命周期
- 生成新粒子
- 批量更新和渲染

### 4. 粒子类型

| 类型 | 说明 | 实现类 |
|-----|------|--------|
| Fog | 白色雾气 | FogParticle |
| RedFog | 红色雾气 | FogParticle |
| BlueFog | 蓝色雾气 | FogParticle |
| YellowFog | 黄色雾气 | FogParticle |
| Snow | 雪花 | SnowParticle |
| Rain | 雨滴 | Particle |
| Sand | 沙尘 | SandParticle |
| Leaves | 叶子 | FogParticle |
| FireyLeaves | 火焰叶子 | FogParticle |
| PurpleLeaves | 紫色叶子 | FogParticle |
| FlowersRain | 花瓣雨 | FlowerParticle |
| Bird | 鸟 | - |
| Blizzard | 暴风雪 | - |

---

## 📐 架构设计

### Rust 实现结构

```
ClientRust/src/graphics/
├── particle_engine.rs       (主引擎)
└── particles/
    ├── mod.rs               (模块导出)
    ├── particle.rs          (基础粒子)
    ├── fog_particle.rs      (雾粒子)
    ├── snow_particle.rs     (雪粒子)
    ├── sand_particle.rs     (沙尘粒子)
    └── flower_particle.rs   (花瓣粒子)
```

### 核心数据结构

```rust
// 粒子图像信息
pub struct ParticleImageInfo {
    pub library: String,           // 图像库名称
    pub base_index: usize,         // 起始索引
    pub count: usize,              // 帧数
    pub current_frame: usize,      // 当前帧
    pub draw_frame_ms: u64,        // 帧间隔（毫秒）
    pub width: u32,                // 宽度
    pub height: u32,               // 高度
    pub start_time: Instant,       // 开始时间
}

// 粒子基类
pub struct Particle {
    pub image_info: ParticleImageInfo,
    pub position: (f32, f32),      // 位置 (x, y)
    pub velocity: (f32, f32),      // 速度 (vx, vy)
    pub color: [f32; 4],           // RGBA
    pub size: f32,                 // 大小缩放
    pub blend_rate: f32,           // 混合率
    pub blend_mode: BlendMode,     // 混合模式
    pub alive_until: Option<Instant>, // 存活时间
    pub update_delay: Duration,    // 更新间隔
    last_update: Instant,          // 上次更新时间
}

// 粒子类型
pub enum ParticleType {
    None,
    Fog,
    RedFog,
    BlueFog,
    YellowFog,
    Snow,
    Rain,
    Sand,
    Leaves,
    FireyLeaves,
    PurpleLeaves,
    FlowersRain,
    // ... 其他类型
}

// 粒子引擎
pub struct ParticleEngine {
    emitter_location: (f32, f32),
    particles: Vec<Box<dyn ParticleBehavior>>,
    textures: Vec<ParticleImageInfo>,
    force_velocity: (f32, f32),
    generate_particles: bool,
    next_particle_time: Instant,
    particle_type: ParticleType,
    screen_width: u32,
    screen_height: u32,
}
```

### Trait 设计

```rust
// 粒子行为 trait
pub trait ParticleBehavior {
    fn update(&mut self, dt: f32, force: (f32, f32), screen_size: (u32, u32));
    fn draw(&self, dx_manager: &DXManager);
    fn is_alive(&self) -> bool;
    fn position(&self) -> (f32, f32);
}

// 为不同粒子类型实现 trait
impl ParticleBehavior for Particle { ... }
impl ParticleBehavior for FogParticle { ... }
impl ParticleBehavior for SnowParticle { ... }
```

---

## 🚀 实现步骤

### 阶段 1: 基础框架 (2-3 小时)

**目标**: 创建粒子系统的基础结构

#### 任务
1. ✅ 创建 `particle_engine.rs`
   - ParticleType 枚举
   - ParticleImageInfo 结构
   - ParticleEngine 结构
   - 基础方法框架

2. ✅ 创建 `particles/mod.rs`
   - 模块导出

3. ✅ 创建 `particles/particle.rs`
   - Particle 基类
   - ParticleBehavior trait
   - 基础物理模拟

4. ✅ 集成到 `graphics/mod.rs`

**验收标准**:
- 编译通过
- 可以创建 ParticleEngine
- 可以添加基础粒子

---

### 阶段 2: 基础粒子实现 (2-3 小时)

**目标**: 实现最简单的粒子类型

#### 任务
1. ✅ 实现基础 Particle
   - 位置更新
   - 速度应用
   - 边界循环
   - 简单绘制

2. ✅ 实现 FogParticle
   - 继承 Particle
   - 飘动效果
   - 慢速运动

3. ✅ 测试示例
   - 创建雾气效果测试
   - 验证渲染正确

**验收标准**:
- 雾气粒子正确渲染
- 粒子会移动
- 边界循环工作

---

### 阶段 3: 高级粒子类型 (2-3 小时)

**目标**: 实现其他常用粒子

#### 任务
1. ✅ SnowParticle (雪花)
   - 下落效果
   - 左右摇摆

2. ✅ RainParticle (雨滴)
   - 快速下落
   - 垂直运动

3. ✅ SandParticle (沙尘)
   - 横向飘移
   - 随机运动

4. ✅ FlowerParticle (花瓣)
   - 飘落旋转
   - 优雅动画

**验收标准**:
- 每种粒子有独特行为
- 渲染效果正确
- 性能可接受

---

### 阶段 4: 引擎完善 (2-3 小时)

**目标**: 完善粒子引擎功能

#### 任务
1. ✅ 粒子生成器
   - 自动生成新粒子
   - 控制生成速率
   - 粒子数量上限

2. ✅ 力场系统
   - 全局力场（风）
   - 影响所有粒子

3. ✅ 动画系统
   - 多帧动画播放
   - 帧率控制

4. ✅ 性能优化
   - 批量渲染
   - 视锥剔除
   - 粒子池复用

**验收标准**:
- 可以生成大量粒子（1000+）
- 保持 60 FPS
- 内存使用稳定

---

## 🎨 使用示例

### 创建雾气效果
```rust
// 加载粒子纹理
let textures = vec![
    ParticleImageInfo::new("Effects", 100, 1, 50),
    ParticleImageInfo::new("Effects", 101, 1, 50),
    ParticleImageInfo::new("Effects", 102, 1, 50),
];

// 创建粒子引擎
let mut particle_engine = ParticleEngine::new(
    textures,
    (400.0, 300.0),  // 发射器位置
    ParticleType::Fog,
    800,  // 屏幕宽度
    600,  // 屏幕高度
);

// 游戏循环
loop {
    particle_engine.update(delta_time);
    
    dx_manager.begin_frame([0.0, 0.0, 0.0, 1.0]);
    particle_engine.draw(&dx_manager);
    dx_manager.end_frame();
}
```

### 创建雪天效果
```rust
let snow_engine = ParticleEngine::new(
    snow_textures,
    (400.0, 0.0),  // 从顶部生成
    ParticleType::Snow,
    800, 600,
);

// 添加风力
snow_engine.set_force_velocity((2.0, 0.0));  // 向右的风
```

---

## 📊 性能考虑

### 优化策略

1. **批量渲染**
   - 收集所有粒子的绘制调用
   - 按纹理分组
   - 一次性提交

2. **对象池**
   - 复用粒子对象
   - 避免频繁分配/释放

3. **空间分区**
   - 屏幕外粒子不渲染
   - 简单的 AABB 检查

4. **LOD (Level of Detail)**
   - 远处粒子简化
   - 减少粒子数量

### 性能目标

| 场景 | 粒子数量 | 目标 FPS |
|-----|---------|---------|
| 轻度（雾） | 100-500 | 60 |
| 中度（雪） | 500-1000 | 60 |
| 重度（暴风雪） | 1000-2000 | 30-60 |

---

## 🔍 测试计划

### 单元测试
- [ ] 粒子位置更新
- [ ] 边界循环
- [ ] 动画帧切换
- [ ] 粒子生成

### 集成测试
- [ ] 与 DXManager 集成
- [ ] 批量渲染
- [ ] 性能测试

### 视觉测试
- [ ] 雾气效果
- [ ] 雪花效果
- [ ] 雨滴效果
- [ ] 花瓣效果

---

## 📝 注意事项

### 1. 坐标系统
- C# 使用 SlimDX.Vector2
- Rust 使用 (f32, f32) 元组
- 注意 Y 轴方向

### 2. 时间管理
- C# 使用 DateTime
- Rust 使用 std::time::Instant
- 需要转换逻辑

### 3. 混合模式
- 已实现基础 Alpha 混合
- 可能需要扩展

### 4. 纹理管理
- 需要加载粒子纹理库
- 缓存纹理避免重复加载

---

## 🎯 里程碑

- [ ] **里程碑 1**: 基础框架 (Day 1)
- [ ] **里程碑 2**: 基础粒子 (Day 2)
- [ ] **里程碑 3**: 高级粒子 (Day 3)
- [ ] **里程碑 4**: 引擎完善 (Day 4)

---

## 📚 参考资料

### C# 原版文件
- `Client/MirGraphics/ParticleEngine.cs`
- `Client/MirGraphics/Particles/Particle.cs`
- `Client/MirGraphics/Particles/FogParticle.cs`

### 相关技术
- 粒子系统设计模式
- 2D 粒子物理
- 批量渲染优化

---

**结论**: ParticleEngine 是一个中等复杂度的模块，需要系统性的实现。建议分 4 个阶段，每个阶段 2-3 小时，总计 8-12 小时完成。

**当前状态**: 📋 计划阶段

**下一步**: 开始实现阶段 1 - 基础框架
