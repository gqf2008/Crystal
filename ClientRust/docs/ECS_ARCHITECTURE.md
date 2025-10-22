# GGEZ + hecs ECS 架构文档

## 📚 概述

这是热血传奇客户端的新架构,结合了:
- **GGEZ**: 简单强大的 2D 游戏框架 (渲染、音频、输入)
- **hecs**: 轻量级高性能 ECS 库 (实体管理、游戏逻辑)

## 🎯 设计目标

1. **简单易用**: 比 Bevy 学习曲线更平缓
2. **性能优异**: ECS 架构 + 缓存友好设计
3. **代码清晰**: 逻辑解耦,易于维护
4. **功能完整**: 保留 GGEZ 的所有渲染特性 (ADD 混合等)

## 🏗️ 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                     GameScene (GGEZ)                        │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              GameWorld (hecs::World)                  │  │
│  │                                                       │  │
│  │  Entities (实体):                                    │  │
│  │  ├─ Player Entity                                    │  │
│  │  │   ├─ Position                                     │  │
│  │  │   ├─ Velocity                                     │  │
│  │  │   ├─ Sprite                                       │  │
│  │  │   ├─ Animation                                    │  │
│  │  │   ├─ Health                                       │  │
│  │  │   └─ PlayerData                                   │  │
│  │  │                                                    │  │
│  │  ├─ Monster Entity                                   │  │
│  │  │   ├─ Position                                     │  │
│  │  │   ├─ Velocity                                     │  │
│  │  │   ├─ Sprite                                       │  │
│  │  │   ├─ AI                                           │  │
│  │  │   └─ MonsterData                                  │  │
│  │  │                                                    │  │
│  │  ├─ Spell Entity (ADD混合特效)                       │  │
│  │  └─ ...                                              │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  Systems (系统):                                            │
│  ├─ MovementSystem    (移动逻辑)                           │
│  ├─ AnimationSystem   (动画更新)                           │
│  ├─ CombatSystem      (战斗计算)                           │
│  ├─ AISystem          (怪物AI)                             │
│  ├─ RenderSystem      (渲染排序)                           │
│  └─ NetworkSystem     (网络同步)                           │
│                                                             │
│  GGEZ (渲染层):                                             │
│  ├─ Canvas.draw() - 绘制精灵                               │
│  ├─ BlendMode::Add - ADD 混合 ⭐                           │
│  ├─ Audio - 音效播放                                       │
│  └─ Input - 键盘/鼠标                                       │
└─────────────────────────────────────────────────────────────┘
```

## 📦 模块结构

```
src/ecs/
├── mod.rs              # 模块导出
├── components.rs       # 组件定义
├── systems.rs          # 系统实现
├── world.rs            # 游戏世界管理
└── game_scene_example.rs  # 使用示例
```

## 🧩 核心组件

### 基础组件

```rust
Position     // 位置 (所有实体)
Velocity     // 速度 (移动实体)
Direction    // 方向
Sprite       // 精灵渲染
Animation    // 动画状态
Health       // 生命值
```

### 特殊组件

```rust
Player       // 玩家数据
LocalPlayer  // 本地玩家标记
RemotePlayer // 远程玩家标记
Monster      // 怪物数据
AIState      // AI状态
NPC          // NPC数据
Spell        // 技能数据
Lifetime     // 生命周期
```

## ⚙️ 系统说明

### MovementSystem - 移动系统
- 更新所有实体的位置
- 处理平滑移动插值
- 格子坐标转换

### AnimationSystem - 动画系统
- 更新所有动画帧
- 处理动画循环/结束
- 同步精灵帧

### CombatSystem - 战斗系统
- 伤害计算
- 命中判定
- 血量更新

### AISystem - AI系统
- 怪物AI状态机
- 巡逻/追击/攻击逻辑
- 目标选择

### RenderSystem - 渲染系统
- 收集可见实体
- 渲染顺序排序
- 摄像机裁剪

### NetworkSyncSystem - 网络同步
- 远程玩家同步
- 怪物状态同步
- 实体创建/移除

## 🎨 ADD 混合特效示例

```rust
// 创建技能特效 (自动使用 ADD 混合)
world.spawn_spell_effect(
    1,                              // 技能ID
    player_id,                      // 施法者
    Point::new(100, 100),           // 起点
    Point::new(110, 100),           // 终点
    2000,                           // 持续2秒
);

// 渲染时自动应用 ADD 混合
let blend_mode = match sprite.blend_mode {
    BlendMode::Add => ggez::graphics::BlendMode::ADD, // ⭐
    _ => ggez::graphics::BlendMode::ALPHA,
};
```

## 🚀 使用示例

### 1. 创建游戏世界

```rust
let mut world = GameWorld::new();

// 创建玩家
let player = world.spawn_local_player(
    "TestPlayer".to_string(),
    MirClass::Warrior,
    MirGender::Male,
    Point::new(100, 100),
);

// 创建怪物
let monster = world.spawn_monster(
    1,
    "小怪".to_string(),
    0,
    Point::new(105, 100),
);
```

### 2. 更新游戏逻辑

```rust
fn update(&mut self, delta: Duration) {
    // 运行系统
    MovementSystem::update(&mut self.world.world, delta);
    AnimationSystem::update(&mut self.world.world, delta);
    AISystem::update(&mut self.world.world, delta);
    
    // 清理死亡实体
    self.world.cleanup_dead_entities();
}
```

### 3. 渲染游戏画面

```rust
fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
    // 收集可见实体
    let entities = RenderSystem::collect_visible_entities(
        &self.world.world,
        camera_x, camera_y,
        40, 30,
    );
    
    // 渲染
    for (entity, pos, sprite, order) in entities {
        self.draw_sprite(ctx, canvas, &pos, &sprite)?;
    }
    
    Ok(())
}
```

### 4. 处理网络消息

```rust
fn handle_network_packet(&mut self, packet: ServerPacket) {
    match packet {
        ServerPacket::ObjectPlayer { id, x, y, .. } => {
            NetworkSyncSystem::sync_remote_player(
                &mut self.world.world,
                id, x, y,
                MirDirection::Down,
                MirAction::Standing,
            );
        }
        _ => {}
    }
}
```

## 📊 性能对比

| 特性 | 纯 GGEZ | GGEZ + hecs | Bevy |
|------|---------|-------------|------|
| 代码复杂度 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ |
| 实体管理 | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| 渲染控制 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| ADD 混合 | ✅ 原生 | ✅ 原生 | ❌ 需自定义 |
| 学习曲线 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ |
| 性能 | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

## ✅ 优势总结

1. **最佳平衡**: 简单性 + 性能 + 灵活性
2. **学习成本低**: 比 Bevy 容易上手
3. **代码更清晰**: ECS 解耦 + GGEZ 简单渲染
4. **功能完整**: ADD 混合、音频、输入全支持
5. **易于调试**: 直接控制渲染流程
6. **性能优异**: hecs 的缓存友好设计

## 🎯 下一步

1. ✅ 完成 ECS 基础架构
2. ⏳ 迁移现有 GameScene 到 ECS
3. ⏳ 实现完整的战斗系统
4. ⏳ 网络同步系统
5. ⏳ UI 系统集成

## 📖 参考资料

- [hecs 文档](https://docs.rs/hecs)
- [GGEZ 文档](https://docs.rs/ggez)
- [ECS 设计模式](https://en.wikipedia.org/wiki/Entity_component_system)
