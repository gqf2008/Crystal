# sounds - 音效系统模块

**对应C#代码**: `Client/MirSounds/`  
**文件数**: 9  
**代码行数**: 1,398  
**状态**: ✅ 核心完成，高级功能待完善

---

## 📚 目录

1. [模块概述](#-模块概述)
2. [架构设计](#-架构设计)
3. [核心组件](#-核心组件)
4. [音效列表](#-音效列表)
5. [使用指南](#-使用指南)
6. [开发状态](#-开发状态)

---

## 📖 模块概述

`sounds` 模块负责游戏的音频播放，包括：

- **音效管理**: 加载和管理游戏音效
- **音乐播放**: 背景音乐播放和循环
- **音量控制**: 全局和单个音效音量控制
- **空间音效**: 基于位置的3D音效（基础）
- **音效库**: 组织和缓存音效资源

### 技术栈

- **rodio**: 音频播放库
- **wav**: WAV文件解析
- **标准音频格式**: WAV, MP3（通过rodio）

### 与C#版本的对应关系

| C# 文件 | Rust 文件 | 说明 |
|---------|----------|------|
| `SoundList.cs` | `sound_list.rs` | 音效ID列表 |
| `SoundManager.cs` | `sound_manager.rs` | 音效管理器 |
| `SoundLibrary.cs` | `libraries/mod.rs` | 音效库系统 |
| ❌ | `sound_loader.rs` | 音效加载器（遗留） |

---

## 🏗 架构设计

### 模块结构

```
sounds/
├── mod.rs                  # 模块入口
├── sound_list.rs           # 音效ID枚举 (~200行)
├── sound_manager.rs        # 音效管理器 (~600行)
├── sound_loader.rs         # 音效加载器（遗留）
└── libraries/              # 音效库系统
    ├── mod.rs              # 库入口
    └── oneshot_provider.rs # 单次播放提供者
```

### 架构层次

```
游戏逻辑层
        ↓
   SoundManager (管理)
        ↓
   SoundLibrary (缓存)
        ↓
   CachedSound (音效数据)
        ↓
   rodio (播放)
```

### 数据流向

#### 播放流程

```
游戏事件
    ↓
sound_manager.play(sound_id)
    ↓
查找音效文件
    ↓
SoundLibrary::get_or_load()
    ↓ (未缓存)
加载WAV文件
    ↓
创建 CachedSound
    ↓
rodio::Sink::append()
    ↓
音频输出
```

---

## 🔧 核心组件

### 1. SoundId (sound_list.rs)

**职责**: 定义所有游戏音效的ID

#### 音效分类

```rust
pub enum SoundId {
    // === UI音效 ===
    ButtonA,           // 按钮点击
    ButtonB,           // 按钮悬停
    ButtonC,           // 按钮按下
    ClickA,            // 点击音效A
    ClickB,            // 点击音效B
    Gold,              // 金币
    EatDrug,           // 吃药
    
    // === 战斗音效 ===
    // 攻击
    SwingShort,        // 近战挥砍（短）
    SwingWood,         // 木质武器
    SwingMetal,        // 金属武器
    
    // 受击
    StruckBodySword,   // 剑击中身体
    StruckBodyAxe,     // 斧击中身体
    StruckBodyLongStick, // 长棍击中
    StruckBodyFist,    // 拳击中
    
    // 死亡
    ManDie,            // 男性死亡
    WomanDie,          // 女性死亡
    
    // === 魔法音效 ===
    FireBall,          // 火球术
    Thunder,           // 雷电术
    Healing,           // 治愈术
    Teleport,          // 传送术
    Flame,             // 烈火剑法
    Lightning,         // 疾光电影
    Explosion,         // 爆裂火焰
    
    // === 怪物音效 ===
    MonsterDie1,       // 怪物死亡1
    MonsterDie2,       // 怪物死亡2
    MonsterAttack1,    // 怪物攻击1
    MonsterAttack2,    // 怪物攻击2
    
    // === 环境音效 ===
    WalkGround,        // 地面脚步
    WalkStone,         // 石头脚步
    WalkGrass,         // 草地脚步
    WalkWood,          // 木板脚步
    
    RunGround,         // 地面奔跑
    RunStone,          // 石头奔跑
    
    // === 物品音效 ===
    DropItem,          // 丢弃物品
    PickUp,            // 拾取物品
    Inventory,         // 背包打开
    
    // === 其他 ===
    Login,             // 登录
    SelectChar,        // 选择角色
    LevelUp,           // 升级
    // ... 更多音效
}
```

#### 音效文件命名

```rust
/// 生成音效文件名
pub fn generate_filename(sound_id: SoundId) -> String {
    match sound_id {
        SoundId::ButtonA => "ButtonA.wav",
        SoundId::FireBall => "M1-1.wav",  // 魔法音效
        SoundId::MonsterDie1 => "Mon-1.wav",
        // ...
    }
}
```

#### 音效列表加载

```rust
/// 从文件加载音效列表
pub fn load_sound_list<P: AsRef<Path>>(path: P) -> Result<Vec<SoundId>>;
```

### 2. SoundManager (sound_manager.rs)

**职责**: 全局音效管理器

#### 核心结构

```rust
pub struct SoundManager {
    /// 音频输出流
    _stream: OutputStream,
    
    /// 音频句柄
    stream_handle: OutputStreamHandle,
    
    /// 音效库（缓存）
    sound_library: SoundLibrary,
    
    /// 当前播放的音效
    active_sounds: Vec<Sink>,
    
    /// BGM播放器
    bgm_sink: Option<Sink>,
    
    /// 全局音量
    master_volume: f32,
    
    /// 音效音量
    sound_volume: f32,
    
    /// 音乐音量
    music_volume: f32,
    
    /// 是否静音
    muted: bool,
    
    /// 音效根目录
    sound_path: PathBuf,
}
```

#### 主要方法

```rust
impl SoundManager {
    /// 创建新的音效管理器
    pub fn new() -> Result<Self>;
    
    /// 设置音效路径
    pub fn set_sound_path<P: AsRef<Path>>(&mut self, path: P);
    
    // === 播放控制 ===
    
    /// 播放音效（单次）
    pub fn play(&mut self, sound_id: SoundId) -> Result<()>;
    
    /// 播放音效（带音量）
    pub fn play_with_volume(
        &mut self,
        sound_id: SoundId,
        volume: f32
    ) -> Result<()>;
    
    /// 播放音效（空间音效，基于位置）
    pub fn play_at_location(
        &mut self,
        sound_id: SoundId,
        location: Point,
        listener_location: Point
    ) -> Result<()>;
    
    /// 播放BGM（循环）
    pub fn play_music(&mut self, sound_id: SoundId) -> Result<()>;
    
    /// 停止BGM
    pub fn stop_music(&mut self);
    
    /// 停止所有音效
    pub fn stop_all(&mut self);
    
    // === 音量控制 ===
    
    /// 设置主音量 (0.0 - 1.0)
    pub fn set_master_volume(&mut self, volume: f32);
    
    /// 设置音效音量 (0.0 - 1.0)
    pub fn set_sound_volume(&mut self, volume: f32);
    
    /// 设置音乐音量 (0.0 - 1.0)
    pub fn set_music_volume(&mut self, volume: f32);
    
    /// 获取主音量
    pub fn master_volume(&self) -> f32;
    
    /// 静音/取消静音
    pub fn set_muted(&mut self, muted: bool);
    
    /// 是否静音
    pub fn is_muted(&self) -> bool;
    
    // === 更新 ===
    
    /// 更新（清理已播放完的音效）
    pub fn update(&mut self);
}
```

#### 特性

- ✅ 单次播放
- ✅ 循环播放（BGM）
- ✅ 音量控制
- ✅ 静音功能
- ✅ 音效缓存
- ✅ 空间音效（基础）
- 🚧 淡入淡出
- 🚧 3D音效（完整）

### 3. SoundLibrary (libraries/mod.rs)

**职责**: 音效缓存和管理

#### 核心结构

```rust
pub struct SoundLibrary {
    /// 缓存的音效
    sounds: HashMap<SoundId, CachedSound>,
    
    /// 音效文件路径
    sound_path: PathBuf,
}

pub struct CachedSound {
    /// 音效数据
    data: Arc<Vec<u8>>,
    
    /// 采样率
    sample_rate: u32,
    
    /// 声道数
    channels: u16,
    
    /// 时长（秒）
    duration: f32,
}
```

#### 主要方法

```rust
impl SoundLibrary {
    /// 创建新的音效库
    pub fn new<P: AsRef<Path>>(sound_path: P) -> Self;
    
    /// 获取或加载音效
    pub fn get_or_load(
        &mut self,
        sound_id: SoundId
    ) -> Result<&CachedSound>;
    
    /// 预加载音效
    pub fn preload(&mut self, sound_id: SoundId) -> Result<()>;
    
    /// 预加载多个音效
    pub fn preload_batch(&mut self, sound_ids: &[SoundId]) -> Result<()>;
    
    /// 清除缓存
    pub fn clear_cache(&mut self);
    
    /// 获取缓存大小
    pub fn cache_size(&self) -> usize;
}

impl CachedSound {
    /// 从WAV文件加载
    pub fn from_wav_file<P: AsRef<Path>>(path: P) -> Result<Self>;
    
    /// 创建音频源
    pub fn create_source(&self) -> impl Source<Item = i16> + Send;
}
```

### 4. OneShotProvider (libraries/oneshot_provider.rs)

**职责**: 提供单次播放的音频源

```rust
pub struct OneShotProvider {
    data: Arc<Vec<u8>>,
    sample_rate: u32,
    channels: u16,
    position: usize,
}

impl Source for OneShotProvider {
    type Item = i16;
    
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.data.len() / 2 - self.position)
    }
    
    fn channels(&self) -> u16 {
        self.channels
    }
    
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    fn total_duration(&self) -> Option<Duration> {
        // 计算总时长
    }
}

impl Iterator for OneShotProvider {
    type Item = i16;
    
    fn next(&mut self) -> Option<i16> {
        // 返回下一个采样
    }
}
```

---

## 🎵 音效列表

### UI音效（~20个）

| 音效ID | 文件 | 用途 |
|--------|------|------|
| ButtonA | ButtonA.wav | 按钮点击 |
| ButtonB | ButtonB.wav | 按钮悬停 |
| Gold | Gold.wav | 金币音效 |
| EatDrug | EatDrug.wav | 吃药 |
| Inventory | Inventory.wav | 背包打开 |
| ClickA | ClickA.wav | 点击音效 |

### 战斗音效（~50个）

#### 攻击音效

| 音效ID | 文件 | 用途 |
|--------|------|------|
| SwingShort | SwingShort.wav | 近战挥砍 |
| SwingMetal | SwingMetal.wav | 金属武器 |
| SwingWood | SwingWood.wav | 木质武器 |

#### 受击音效

| 音效ID | 文件 | 用途 |
|--------|------|------|
| StruckBodySword | StruckBodySword.wav | 剑击中 |
| StruckBodyAxe | StruckBodyAxe.wav | 斧击中 |
| StruckBodyFist | StruckBodyFist.wav | 拳击中 |

#### 死亡音效

| 音效ID | 文件 | 用途 |
|--------|------|------|
| ManDie | ManDie.wav | 男性死亡 |
| WomanDie | WomanDie.wav | 女性死亡 |

### 魔法音效（~30个）

| 音效ID | 文件 | 用途 |
|--------|------|------|
| FireBall | M1-1.wav | 火球术 |
| Thunder | M2-1.wav | 雷电术 |
| Healing | M3-1.wav | 治愈术 |
| Teleport | M4-1.wav | 传送术 |
| Flame | M5-1.wav | 烈火剑法 |
| Lightning | M6-1.wav | 疾光电影 |
| Explosion | M7-1.wav | 爆裂火焰 |

### 怪物音效（~40个）

| 音效ID | 文件 | 用途 |
|--------|------|------|
| MonsterDie1 | Mon-1.wav | 怪物死亡1 |
| MonsterDie2 | Mon-2.wav | 怪物死亡2 |
| MonsterAttack1 | MonAttack-1.wav | 怪物攻击1 |

### 环境音效（~20个）

| 音效ID | 文件 | 用途 |
|--------|------|------|
| WalkGround | WalkGround.wav | 地面脚步 |
| WalkStone | WalkStone.wav | 石头脚步 |
| RunGround | RunGround.wav | 地面奔跑 |

### BGM（~10个）

| 音效ID | 文件 | 用途 |
|--------|------|------|
| MusicLogin | Music-Login.wav | 登录音乐 |
| MusicSelect | Music-Select.wav | 选择角色 |
| MusicGame | Music-Game.wav | 游戏主题 |

---

## 📖 使用指南

### 初始化

```rust
use crate::sounds::*;

fn main() -> Result<()> {
    // 创建音效管理器
    let mut sound_manager = SoundManager::new()?;
    
    // 设置音效路径
    sound_manager.set_sound_path("./Sound");
    
    // 设置音量
    sound_manager.set_master_volume(0.8);
    sound_manager.set_sound_volume(1.0);
    sound_manager.set_music_volume(0.6);
    
    Ok(())
}
```

### 播放音效

```rust
// 播放UI音效
sound_manager.play(SoundId::ButtonA)?;

// 播放战斗音效
sound_manager.play(SoundId::SwingShort)?;

// 播放魔法音效
sound_manager.play(SoundId::FireBall)?;
```

### 播放BGM

```rust
// 播放登录音乐（循环）
sound_manager.play_music(SoundId::MusicLogin)?;

// 停止音乐
sound_manager.stop_music();
```

### 音量控制

```rust
// 调整主音量
sound_manager.set_master_volume(0.5);

// 调整音效音量
sound_manager.set_sound_volume(0.8);

// 调整音乐音量
sound_manager.set_music_volume(0.3);

// 静音
sound_manager.set_muted(true);

// 取消静音
sound_manager.set_muted(false);
```

### 空间音效

```rust
// 播放空间音效（音量随距离衰减）
sound_manager.play_at_location(
    SoundId::MonsterAttack1,
    Point::new(100, 100),  // 音效位置
    Point::new(90, 90),    // 听者位置
)?;
```

### 预加载

```rust
// 预加载常用音效
let common_sounds = vec![
    SoundId::ButtonA,
    SoundId::Gold,
    SoundId::EatDrug,
    SoundId::SwingShort,
    SoundId::FireBall,
];

for sound_id in common_sounds {
    sound_manager.sound_library.preload(sound_id)?;
}
```

### 更新（每帧调用）

```rust
// 游戏循环中
fn update(&mut self) {
    // 清理已播放完的音效
    self.sound_manager.update();
}
```

---

## 📊 开发状态

### 完成度统计

| 功能模块 | 完成度 | 说明 |
|---------|--------|------|
| **SoundManager** | 90% | 核心功能完成，高级功能待完善 |
| **SoundLibrary** | 95% | 缓存系统完成 |
| **音效播放** | 100% | 单次播放完成 |
| **BGM播放** | 100% | 循环播放完成 |
| **音量控制** | 100% | 完整的音量控制 |
| **空间音效** | 60% | 基础实现，3D音效待完善 |
| **淡入淡出** | 0% | 未实现 |

### 已实现功能清单

#### ✅ 核心功能

- [x] 音效管理器
- [x] 音效库缓存
- [x] WAV文件加载
- [x] 音效播放（单次）
- [x] BGM播放（循环）
- [x] 音量控制
- [x] 静音功能

#### ✅ 音效类型

- [x] UI音效
- [x] 战斗音效
- [x] 魔法音效
- [x] 怪物音效
- [x] 环境音效
- [x] BGM

#### ✅ 优化

- [x] 音效缓存
- [x] 延迟加载
- [x] 自动清理
- [x] 预加载

#### ✅ 辅助功能

- [x] 音效列表
- [x] 文件名生成
- [x] 空间音效（基础）

### 未实现功能清单

#### ⏳ 高级功能

- [ ] **淡入淡出**: 音效渐变
- [ ] **音效混音**: 多音效混合
- [ ] **完整3D音效**: 立体声、方向音效
- [ ] **音效优先级**: 重要音效优先播放
- [ ] **音效限制**: 同时播放数量限制

#### ⏳ 格式支持

- [ ] **MP3支持**: MP3格式音效
- [ ] **OGG支持**: OGG格式音效
- [ ] **流式播放**: 大文件流式播放

#### ⏳ 优化

- [ ] **智能预加载**: 预测需要的音效
- [ ] **内存优化**: 更好的缓存策略
- [ ] **音效压缩**: 减少内存占用

#### ⏳ 工具

- [ ] **音效编辑器**: 音效查看和编辑
- [ ] **音效测试**: 音效测试工具

---

## 🚀 未来规划

### 短期目标 (1-2周)

1. **淡入淡出** 🟡 中优先级
   - 音效淡入
   - 音效淡出
   - BGM交叉淡入淡出

2. **音效混音** 🟡 中优先级
   - 多音效混合
   - 音效组
   - 音效优先级

3. **完整3D音效** 🟢 低优先级
   - 立体声定位
   - 方向音效
   - 距离衰减优化

### 中期目标 (3-4周)

4. **格式扩展** 🟢 低优先级
   - MP3支持
   - OGG支持
   - 流式播放

5. **智能预加载** 🟡 中优先级
   - 场景相关音效预加载
   - 战斗音效预加载
   - 内存管理

6. **音效编辑器** 🟢 低优先级
   - 音效查看器
   - 音量调整工具
   - 音效测试

### 长期目标 (1-2月)

7. **高级功能**
   - 音效序列器
   - 音效触发器
   - 动态音效

8. **性能优化**
   - 音效压缩
   - 批量加载
   - 异步加载

---

## 🐛 已知问题

### 高优先级

- [ ] 大量音效同时播放时CPU占用高
- [ ] 音效缓存无限增长

### 中优先级

- [ ] 空间音效距离衰减不够平滑
- [ ] BGM切换无淡入淡出

### 低优先级

- [ ] 部分音效文件名不规范
- [ ] 音效加载错误处理不够优雅

---

## 🔧 性能考虑

### 内存使用

**估算**:

```
单个音效: ~100KB
缓存音效(50个): ~5MB
BGM: ~5MB
总计: ~10MB
```

**优化策略**:

1. **LRU缓存**: 最少使用的音效自动卸载
2. **预加载**: 只预加载常用音效
3. **流式播放**: 大文件不完全加载

### CPU使用

**当前性能**:

- 播放音效: ~0.1ms
- 混音: ~1ms (10个音效)
- 瓶颈: 同时播放数量

**优化策略**:

1. **音效限制**: 限制同时播放数量
2. **音效优先级**: 重要音效优先
3. **硬件加速**: 使用GPU混音（未来）

---

## 📝 最佳实践

### 音效播放

```rust
// ✅ 正确：检查错误
if let Err(e) = sound_manager.play(SoundId::ButtonA) {
    tracing::warn!("Failed to play sound: {}", e);
}

// ❌ 错误：忽略错误
sound_manager.play(SoundId::ButtonA).ok();
```

### 音量设置

```rust
// ✅ 正确：范围限制
let volume = volume.clamp(0.0, 1.0);
sound_manager.set_master_volume(volume);

// ❌ 错误：不检查范围
sound_manager.set_master_volume(1.5);  // 可能太大
```

### 预加载

```rust
// ✅ 正确：预加载常用音效
fn preload_common_sounds(manager: &mut SoundManager) {
    let sounds = vec![SoundId::ButtonA, SoundId::Gold];
    for sound in sounds {
        manager.sound_library.preload(sound).ok();
    }
}

// ❌ 错误：预加载所有音效
fn preload_all_sounds(manager: &mut SoundManager) {
    // 内存占用过高
}
```

---

## 🔗 相关文档

### 内部文档

- **ECS系统**: `../ecs/systems/README.md` - 音效触发系统
- **对象系统**: `../objects/README.md` - 对象音效

### 外部资源

- **rodio文档**: https://docs.rs/rodio/ - 音频播放库
- **WAV格式**: https://en.wikipedia.org/wiki/WAV - WAV文件格式

---

## 💡 技术细节

### WAV文件加载

```rust
// 读取WAV文件
let mut reader = hound::WavReader::open(path)?;
let spec = reader.spec();

// 读取采样
let samples: Vec<i16> = reader
    .samples::<i16>()
    .map(|s| s.unwrap())
    .collect();

// 创建音效
let cached_sound = CachedSound {
    data: Arc::new(samples),
    sample_rate: spec.sample_rate,
    channels: spec.channels,
    duration: samples.len() as f32 / spec.sample_rate as f32,
};
```

### 空间音效计算

```rust
// 计算距离
let dx = (sound_pos.x - listener_pos.x) as f32;
let dy = (sound_pos.y - listener_pos.y) as f32;
let distance = (dx * dx + dy * dy).sqrt();

// 距离衰减
let max_distance = 20.0;
let volume = if distance < max_distance {
    1.0 - (distance / max_distance)
} else {
    0.0
};

// 播放
sound_manager.play_with_volume(sound_id, volume)?;
```

### 音效循环

```rust
// 创建循环音源
let source = CachedSound::create_source(&cached_sound)
    .repeat_infinite();

// 创建播放器
let sink = Sink::try_new(&stream_handle)?;
sink.append(source);

// 保存引用（防止停止）
self.bgm_sink = Some(sink);
```

---

**文档版本**: v1.0  
**最后更新**: 2025-10-28  
**维护者**: Crystal Mir2 Team
