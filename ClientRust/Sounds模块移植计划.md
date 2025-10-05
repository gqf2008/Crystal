# MirSounds 模块移植计划

## 📋 模块概述

MirSounds 负责游戏中的音效和音乐播放管理。

## 🏗️ C# 架构分析

### 核心文件
1. **SoundManager.cs** - 音频管理器
   - 音效播放 (OneShot)
   - 循环音效 (Loop)
   - 背景音乐 (Music)
   - 音量控制
   - 延迟播放
   - 缓存管理

2. **SoundList.cs** - 音效索引列表
   - 从 SoundList.lst 加载音效映射
   - 定义所有音效常量 (ButtonA, WalkGroundL, etc.)

3. **Libraries/** - 音频库抽象
   - **ISoundLibrary.cs** - 音频库接口
   - **CachedSound.cs** - 缓存的音频数据
   - **OneShotProvider.cs** - 单次播放提供者
   - **LoopProvider.cs** - 循环播放提供者

### 依赖
- **NAudio** (C#): 音频播放库
- **rodio** (Rust): 音频播放库 (已部分使用)

## 📊 功能清单

### Phase 1: 核心基础 ✅ (已有部分)
- [x] AudioEngine 基本结构
- [ ] 音频文件加载 (wav/mp3)
- [ ] 音量控制
- [ ] 简单的音效播放

### Phase 2: 音效管理
- [ ] SoundList - 音效索引加载
- [ ] SoundManager - 核心管理器
  - [ ] PlaySound() - 播放音效
  - [ ] StopSound() - 停止音效
  - [ ] PlayMusic() - 播放音乐
  - [ ] StopMusic() - 停止音乐
  - [ ] 音量调节
  - [ ] 延迟播放队列

### Phase 3: 高级特性
- [ ] 音频缓存系统
- [ ] 循环播放支持
- [ ] 多通道混音
- [ ] 过期清理 (ExpireTime)
- [ ] 立体声/单声道转换

## 🎯 实现策略

### 选项 A: 完整移植 (推荐)
**优点:**
- 功能完整,与 C# 版本一致
- 支持所有游戏音效需求
- 良好的性能(缓存机制)

**缺点:**
- 工作量较大 (~3-5 天)
- 需要处理音频格式兼容性

**实现步骤:**
1. Week 1: 
   - SoundList 加载器
   - 基础音效播放
   - 音量控制
2. Week 2:
   - 循环播放
   - 音乐系统
   - 缓存管理
3. Week 3:
   - 延迟播放
   - 过期清理
   - 测试和优化

### 选项 B: 简化版本
**优点:**
- 快速实现 (~1-2 天)
- 满足基本需求

**缺点:**
- 功能受限
- 后期可能需要重构

**实现内容:**
- 基础音效播放
- 简单音量控制
- 无缓存/无循环

### 选项 C: 暂时禁用 (当前状态)
**当前状态:**
```rust
pub fn new(sound: &SoundSettings) -> Result<Self> {
    Err(anyhow::anyhow!("Audio system temporarily disabled"))
}
```

## 📝 技术决策

### Rust 音频库选择
- **rodio** (已使用): 
  - ✅ 纯 Rust
  - ✅ 跨平台
  - ✅ 支持 wav/mp3/flac
  - ❌ API 相对简单

- **kira**:
  - ✅ 游戏音频专用
  - ✅ 支持循环/音量/空间音效
  - ❌ 较新,文档较少

- **cpal**:
  - ✅ 低级音频 API
  - ❌ 需要手动处理更多细节

**推荐:** 继续使用 **rodio**,功能足够且稳定

### 架构设计

```rust
// 模块结构
src/sounds/
├── mod.rs              // 模块入口,AudioEngine
├── sound_manager.rs    // SoundManager (对应 C# SoundManager)
├── sound_list.rs       // SoundList 常量和加载器
├── cached_sound.rs     // 音频缓存
├── sound_player.rs     // 音效播放器抽象
└── tests/              // 单元测试
```

### 关键类型

```rust
// Sound ID type
pub type SoundId = i32;

// Sound metadata
pub struct SoundInfo {
    pub id: SoundId,
    pub filename: String,
    pub loop_sound: bool,
}

// Cached audio data
pub struct CachedSound {
    pub id: SoundId,
    pub data: Vec<u8>,
    pub expire_time: u64,
}

// Sound manager
pub struct SoundManager {
    volume: i32,
    music_volume: i32,
    cached_sounds: HashMap<SoundId, CachedSound>,
    looping_sounds: HashMap<SoundId, Sink>,
    music_sink: Option<Sink>,
    delayed_sounds: Vec<(u64, SoundId)>,
}
```

## ✅ Phase 1 接受标准

- [ ] 可以加载 SoundList.lst 文件
- [ ] 可以播放单个音效 (wav/mp3)
- [ ] 可以调节音量 (0-100)
- [ ] 可以播放背景音乐
- [ ] 可以停止音效/音乐
- [ ] 基本的单元测试

## 🚀 推荐执行计划

**立即开始 Phase 1:**
1. 创建 sound_list.rs - 加载音效索引
2. 修复 AudioEngine - 启用 rodio
3. 实现基础播放功能
4. 添加音量控制
5. 编写单元测试

**预计时间:** 1-2 天

## 📌 注意事项

1. **音频文件路径**
   - C#: `Settings.SoundPath`
   - Rust: 需要从配置中读取

2. **音频格式支持**
   - rodio 支持: wav, mp3, flac, ogg, vorbis
   - 确保游戏资源使用支持的格式

3. **性能考虑**
   - 音频解码可能较慢,需要缓存
   - 考虑使用异步加载

4. **错误处理**
   - 音频文件不存在
   - 格式不支持
   - 设备初始化失败

## 🎯 成功标准

完成后应该能够:
```rust
use crate::sounds::{SoundManager, SoundList};

// 初始化
let mut manager = SoundManager::new()?;

// 播放音效
manager.play_sound(SoundList::ButtonA, false, 0)?;

// 播放背景音乐
manager.play_music(SoundList::IntroMusic, true)?;

// 调节音量
manager.set_volume(80);
manager.set_music_volume(60);

// 停止
manager.stop_music();
```

---

**建议:** 选择 **选项 A (完整移植)** + **Phase 1 先行**

现在可以开始实现吗? 
- [x] 是 - 开始 Phase 1
- [ ] 否 - 需要更多讨论
