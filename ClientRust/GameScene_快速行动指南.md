# 🚀 GameScene 功能扩展 - 快速行动指南

**目标**: 快速完善 GameScene 的核心功能  
**时间**: 今天开始  
**优先级**: 🔴 高优先级

---

## 📋 快速行动计划

### 第一步: Phase 1 - 玩家实体管理 (当前 - 2 小时)

**任务**: 增强 Player 组件和实现属性系统

#### Step 1.1: 扩展 Player 组件 (30 分钟)

编辑 `src/bevy/scenes/game_scene/components.rs`，添加新字段：

```rust
// 在 components.rs 中添加新的数据结构

/// 角色属性
#[derive(Debug, Clone, Copy)]
pub struct CharacterStats {
    pub attack: u16,
    pub defense: u16,
    pub magic_attack: u16,
    pub magic_defense: u16,
    pub speed: u16,
}

/// 增益效果
#[derive(Debug, Clone)]
pub struct BuffEffect {
    pub buff_id: u32,
    pub name: String,
    pub duration: f32,
    pub effect_type: u8,  // 0=治疗, 1=伤害, 2=速度, 3=防御
}

// 扩展 Player 组件
#[derive(Component, Debug, Clone)]
pub struct Player {
    pub character_id: i32,
    pub name: String,
    pub class: u8,
    pub gender: u8,
    pub level: u16,
    pub hair: u8,              // 新增: 发型
    pub face: u8,              // 新增: 脸型
    pub stats: CharacterStats, // 新增: 属性
    pub buffs: Vec<BuffEffect>, // 新增: 增益
}
```

#### Step 1.2: 实现属性更新系统 (45 分钟)

在 `src/bevy/scenes/game_scene/mod.rs` 中添加系统：

```rust
/// 玩家属性更新系统
pub fn update_player_stats_system(
    mut player_query: Query<(&Player, &mut Transform), Changed<Player>>,
    mut game_state: ResMut<GameSceneState>,
) {
    for (player, _transform) in player_query.iter_mut() {
        game_state.player_level = player.level;
        info!("📊 玩家属性已更新: Lv.{}", player.level);
    }
}

/// 处理增益效果
pub fn process_buffs_system(
    mut player_query: Query<&mut Player>,
    time: Res<Time>,
) {
    for mut player in player_query.iter_mut() {
        // 更新增益持续时间
        for buff in player.buffs.iter_mut() {
            buff.duration -= time.delta_secs();
        }
        
        // 移除过期增益
        player.buffs.retain(|buff| buff.duration > 0.0);
        
        if !player.buffs.is_empty() {
            info!("✨ 激活增益: {} 个", player.buffs.len());
        }
    }
}
```

#### Step 1.3: 注册新系统 (15 分钟)

在 `src/bin/main_bevy.rs` 中的 GameState::Game 系统注册中添加：

```rust
// 在 Game 状态的 Update 系统中添加
app.add_systems(Update, (
    // 新增系统
    update_player_stats_system,
    process_buffs_system,
    
    // ... 现有系统
).run_if(in_state(GameState::Game)));
```

#### Step 1.4: 测试验证 (30 分钟)

```bash
cargo build
cargo run --bin mir2_bevy

# 检查日志输出
# 应该看到:
# ✨ 激活增益: 0 个
# 📊 玩家属性已更新: Lv.1
```

---

### 第二步: Phase 2 - 地图加载基础 (明天 - 3 小时)

**任务**: 实现基础地图加载系统

#### Step 2.1: 定义地图数据结构 (1 小时)
#### Step 2.2: 实现地图加载系统 (1.5 小时)
#### Step 2.3: 测试地图渲染 (0.5 小时)

---

### 第三步: Phase 3 & 4 - NPC 和聊天 (后续 - 2 小时)

**任务**: 实现 NPC 交互和聊天系统

---

## 🎯 今天必完成

✅ **Phase 1 - 玩家实体管理**
- [ ] 扩展 Player 组件
- [ ] 实现属性更新系统
- [ ] 实现增益处理系统
- [ ] 注册新系统
- [ ] 编译测试通过

---

## 📝 代码模板

### 完整的 Phase 1 实现

**文件**: `src/bevy/scenes/game_scene/components.rs` (需要添加)

```rust
// 在现有代码后添加...

/// 角色属性
#[derive(Debug, Clone, Copy)]
pub struct CharacterStats {
    pub attack: u16,
    pub defense: u16,
    pub magic_attack: u16,
    pub magic_defense: u16,
    pub speed: u16,
}

impl Default for CharacterStats {
    fn default() -> Self {
        Self {
            attack: 10,
            defense: 5,
            magic_attack: 8,
            magic_defense: 4,
            speed: 100,
        }
    }
}

/// 增益效果
#[derive(Debug, Clone)]
pub struct BuffEffect {
    pub buff_id: u32,
    pub name: String,
    pub duration: f32,
    pub effect_type: u8,
}

// 扩展 Player (修改现有的)
#[derive(Component, Debug, Clone)]
pub struct Player {
    pub character_id: i32,
    pub name: String,
    pub class: u8,
    pub gender: u8,
    pub level: u16,
    pub hair: u8,
    pub face: u8,
    pub stats: CharacterStats,
    pub buffs: Vec<BuffEffect>,
}
```

**文件**: `src/bevy/scenes/game_scene/mod.rs` (需要添加系统)

```rust
// 在模块底部添加新系统

/// 玩家属性更新系统
pub fn update_player_stats_system(
    mut player_query: Query<(&Player, &mut Transform), Changed<Player>>,
    mut game_state: ResMut<GameSceneState>,
) {
    for (player, _transform) in player_query.iter_mut() {
        game_state.player_level = player.level;
        info!("📊 玩家属性已更新: Lv.{} | 攻击力:{}", 
            player.level, player.stats.attack);
    }
}

/// 处理增益效果
pub fn process_buffs_system(
    mut player_query: Query<&mut Player>,
    time: Res<Time>,
) {
    for mut player in player_query.iter_mut() {
        // 更新增益持续时间
        for buff in player.buffs.iter_mut() {
            buff.duration -= time.delta_secs();
        }
        
        // 移除过期增益
        let original_count = player.buffs.len();
        player.buffs.retain(|buff| buff.duration > 0.0);
        
        if player.buffs.len() < original_count {
            info!("💫 增益已消退: {} → {}", original_count, player.buffs.len());
        }
    }
}

// 导出新系统供 main_bevy.rs 使用
pub use process_buffs_system;
pub use update_player_stats_system;
```

**文件**: `src/bin/main_bevy.rs` (导入和注册)

在导入部分添加:
```rust
use bevy_modules::scenes::{
    // ... 现有导入 ...
    update_player_stats_system,
    process_buffs_system,
};
```

在 Game 状态系统注册中添加:
```rust
// GameScene 系统 - 玩家管理
app.add_systems(Update, (
    update_player_stats_system,
    process_buffs_system,
    // ... 现有系统 ...
).run_if(in_state(GameState::Game)));
```

---

## ✅ 验证清单

完成后检查：

- [ ] 代码编译无错误 (`cargo check` → ✅ 0 errors)
- [ ] 能正常运行 (`cargo run` → ✅ 正常启动)
- [ ] 看到日志输出 (`📊 玩家属性已更新`)
- [ ] 没有新的警告

---

## 🚀 快速命令

```bash
# 检查编译
cargo check

# 完整编译
cargo build

# 运行测试
cargo run --bin mir2_bevy

# 实时监控编译
cargo watch -x check
```

---

## 📊 进度跟踪

| 阶段 | 任务 | 状态 | ETA |
|------|------|------|-----|
| Phase 1 | 玩家属性管理 | 🔄 进行中 | 今天 |
| Phase 2 | 地图加载 | ⏳ 待做 | 明天 |
| Phase 3 | NPC 交互 | ⏳ 待做 | 明天 |
| Phase 4 | 聊天系统 | ⏳ 待做 | 明天 |
| Phase 5 | 网络同步 | ⏳ 待做 | 本周 |
| Phase 6 | 事件循环 | ⏳ 待做 | 本周末 |

---

## 💡 提示

1. **先从简单开始** - Phase 1 最简单，适合热身
2. **增量编译** - 使用 `cargo watch` 实时检查
3. **经常测试** - 每完成一个小任务就编译一次
4. **保存文档** - 完成后更新 GameScene_最终完成报告.md

---

**现在就可以开始！** 👉 按照上面的代码模板，逐步添加代码到对应文件中。

