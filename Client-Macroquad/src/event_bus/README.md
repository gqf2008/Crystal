# EventBus 事件总线

## 概述

EventBus 是游戏的统一事件通信系统，实现了系统间的解耦通信。

### 核心特性

✅ **类型安全** - 所有事件在编译期检查  
✅ **零拷贝** - 通过迭代器访问，避免克隆  
✅ **职责清晰** - 每种事件有明确的生产者和消费者  
✅ **帧内有效** - 事件在帧结束后自动清空  
✅ **统计监控** - 记录事件数量，方便性能分析

---

## 事件分类

### 1. InputEvent (输入事件)

**来源**: 操作系统 → `InputSystem`  
**消费者**: `PlayerControlSystem`, `CameraSystem`, `UISystem`

```rust
use crate::event_bus::InputEvent;
use macroquad::prelude::KeyCode;
use std::time::Instant;

// 键盘事件
ctx.events_mut().send_input(InputEvent::KeyDown {
    keycode: KeyCode::W,
    repeat: false,
    modifiers: KeyModifiers::none(),
    timestamp: Instant::now(),
});

// 鼠标事件
ctx.events_mut().send_input(InputEvent::MouseDown {
    button: MouseButton::Left,
    x: 100.0,
    y: 200.0,
    timestamp: Instant::now(),
});

// 触摸事件
ctx.events_mut().send_input(InputEvent::TouchStart {
    id: 0,
    x: 100.0,
    y: 200.0,
    timestamp: Instant::now(),
});
```

### 2. NetworkEvent (网络事件)

**来源**: 服务器 → `NetworkSystem`  
**消费者**: 各种逻辑系统

```rust
use crate::event_bus::NetworkEvent;

// 从网络系统接收
let events = ctx.network().recv_all();
for event in events {
    ctx.events_mut().send_network(event);
}

// 逻辑系统消费
for event in ctx.events().network_events() {
    match event {
        NetworkEvent::LoginSuccess { characters } => { /* ... */ },
        NetworkEvent::MapChanged { map_index, .. } => { /* ... */ },
        _ => {}
    }
}
```

### 3. GameLogicEvent (游戏逻辑事件)

**来源**: 各种逻辑系统  
**消费者**: 其他逻辑系统、表现层系统

```rust
use crate::event_bus::{GameLogicEvent, DamageType};

// 发送伤害事件
ctx.events_mut().send_logic(GameLogicEvent::DamageDealt {
    attacker: attacker_entity,
    target: target_entity,
    damage: 100,
    damage_type: DamageType::Physical,
});

// 监听伤害事件（触发动画）
for event in ctx.events().logic_events() {
    if let GameLogicEvent::DamageDealt { target, damage, .. } = event {
        // 播放受击动画
    }
}
```

### 4. UIEvent (UI事件)

**来源**: UI系统  
**消费者**: 逻辑系统、网络系统

```rust
use crate::event_bus::UIEvent;

// UI按钮点击
ctx.events_mut().send_ui(UIEvent::ButtonClicked {
    button_id: "attack_button".to_string(),
});

// 聊天消息发送
ctx.events_mut().send_ui(UIEvent::ChatMessageSent {
    message: "Hello!".to_string(),
    chat_type: ChatType::World,
});

// 逻辑系统消费
for event in ctx.events().ui_events() {
    match event {
        UIEvent::ChatMessageSent { message, chat_type } => {
            // 发送到网络
            ctx.network().send(NetworkEvent::ChatRequest { 
                message: message.clone(), 
                chat_type: *chat_type 
            });
        }
        _ => {}
    }
}
```

### 5. PresentationEvent (表现层事件)

**来源**: 逻辑系统  
**消费者**: 渲染系统、音效系统、粒子系统

```rust
use crate::event_bus::{PresentationEvent, AnimationType, LoopMode};

// 触发动画
ctx.events_mut().send_presentation(PresentationEvent::PlayAnimation {
    entity: player_entity,
    animation: AnimationType::Attack,
    loop_mode: LoopMode::Once,
});

// 播放音效
ctx.events_mut().send_presentation(PresentationEvent::PlaySound {
    sound_id: "sword_hit".to_string(),
    position: Some((100.0, 200.0)),
    volume: 1.0,
    pitch: 1.0,
});

// 粒子系统消费
for event in ctx.events().presentation_events() {
    if let PresentationEvent::SpawnParticle { particle_type, position, .. } = event {
        // 生成粒子
    }
}
```

---

## 使用示例

### 完整的战斗流程

```rust
// ============================================================================
// 1. 玩家控制系统：输入 → 攻击指令
// ============================================================================
impl LogicSystem for PlayerControlSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 监听鼠标点击
        for event in ctx.events().input_events() {
            if let InputEvent::MouseDown { button: MouseButton::Left, x, y, .. } = event {
                // 转换为游戏逻辑事件
                let target = find_entity_at(*x, *y);
                if let Some(target_entity) = target {
                    ctx.events_mut().send_logic(GameLogicEvent::AttackCommand {
                        attacker: player_entity,
                        target: target_entity,
                    });
                }
            }
        }
        Ok(())
    }
}

// ============================================================================
// 2. 战斗系统：攻击指令 → 伤害计算 → 伤害事件
// ============================================================================
impl LogicSystem for CombatSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 监听攻击指令
        for event in ctx.events().logic_events() {
            if let GameLogicEvent::AttackCommand { attacker, target } = event {
                // 计算伤害
                let damage = calculate_damage(*attacker, *target);
                
                // 发送伤害事件
                ctx.events_mut().send_logic(GameLogicEvent::DamageDealt {
                    attacker: *attacker,
                    target: *target,
                    damage,
                    damage_type: DamageType::Physical,
                });
                
                // 发送网络事件（同步到服务器）
                ctx.network().send(NetworkEvent::AttackRequest {
                    direction: MirDirection::Down,
                    spell: 0,
                });
            }
        }
        Ok(())
    }
}

// ============================================================================
// 3. 动画系统：伤害事件 → 播放动画
// ============================================================================
impl LogicSystem for AnimationSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 监听伤害事件
        for event in ctx.events().logic_events() {
            if let GameLogicEvent::DamageDealt { target, .. } = event {
                // 触发受击动画
                ctx.events_mut().send_presentation(PresentationEvent::PlayAnimation {
                    entity: *target,
                    animation: AnimationType::Hit,
                    loop_mode: LoopMode::Once,
                });
                
                // 触发血液粒子
                ctx.events_mut().send_presentation(PresentationEvent::SpawnParticle {
                    particle_type: ParticleType::Blood,
                    position: get_entity_position(*target),
                    velocity: None,
                    duration: 1.0,
                });
            }
        }
        Ok(())
    }
}

// ============================================================================
// 4. 音效系统：伤害事件 → 播放音效
// ============================================================================
impl LogicSystem for SoundSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 监听表现层事件
        for event in ctx.events().presentation_events() {
            if let PresentationEvent::PlaySound { sound_id, position, volume, pitch } = event {
                // 播放音效
                play_sound(sound_id, *position, *volume, *pitch);
            }
        }
        Ok(())
    }
}
```

---

## 主循环集成

```rust
// 游戏主循环
loop {
    let dt = get_frame_time();
    
    // 1. 更新逻辑（系统产生/消费事件）
    scheduler.update(&mut ctx, dt)?;
    
    // 2. 渲染
    scheduler.draw(&mut gfx_ctx, &mut canvas, &ctx.world)?;
    
    // 3. 清空帧事件（重要！）
    ctx.events_mut().clear_frame();
    
    next_frame().await;
}
```

---

## API 参考

### 发送事件

```rust
ctx.events_mut().send_input(event);         // 发送输入事件
ctx.events_mut().send_network(event);       // 发送网络事件
ctx.events_mut().send_logic(event);         // 发送逻辑事件
ctx.events_mut().send_ui(event);            // 发送UI事件
ctx.events_mut().send_presentation(event);  // 发送表现层事件
```

### 读取事件（零拷贝）

```rust
ctx.events().input_events()         // 迭代器
ctx.events().network_events()       // 迭代器
ctx.events().logic_events()         // 迭代器
ctx.events().ui_events()            // 迭代器
ctx.events().presentation_events()  // 迭代器
```

### 查询事件

```rust
ctx.events().has_input_events()         // bool
ctx.events().has_network_events()       // bool
ctx.events().has_logic_events()         // bool
ctx.events().has_ui_events()            // bool
ctx.events().has_presentation_events()  // bool
ctx.events().total_event_count()        // usize
```

### 帧管理

```rust
ctx.events_mut().clear_frame();     // 清空所有事件
ctx.events().stats();               // 获取统计信息
```

---

## 设计原则

### ✅ 应该使用事件的场景

- 系统间松耦合通信
- 一对多广播（一个事件多个监听者）
- 跨层通信（输入层 → 逻辑层 → 表现层）
- 瞬时触发（音效、粒子、动画）

### ❌ 不应该使用事件的场景

- 持久状态（应该用 Component）
- 跨帧数据（应该用 Resource 或 Component）
- 高频更新（每帧数百次，应该直接修改 Component）
- 需要返回值（应该用直接函数调用）

---

## 性能优化建议

1. **预分配容量** - EventBus 已经为常见事件类型预分配了容量
2. **批量发送** - 使用 `send_*_batch()` 批量发送事件
3. **避免克隆** - 使用迭代器访问，避免 `collect()`
4. **及时清空** - 每帧结束调用 `clear_frame()`

---

## 故障排查

### 问题：事件发送了但没收到

**原因**: 可能在发送前就调用了 `clear_frame()`  
**解决**: 确保发送和消费在同一帧内

### 问题：事件累积导致内存泄漏

**原因**: 忘记调用 `clear_frame()`  
**解决**: 在主循环每帧结束调用 `clear_frame()`

### 问题：事件顺序不对

**原因**: 系统执行顺序问题  
**解决**: 检查系统优先级（priority）

---

## 扩展

### 添加新事件类型

1. 在对应的 `*_event.rs` 文件中添加枚举变体
2. 文档化生产者和消费者
3. 添加单元测试

### 添加新事件类别

如果现有5种事件类型不够用：

1. 创建新的 `src/event_bus/my_event.rs`
2. 在 `EventBus` 中添加新队列
3. 实现 `send_my_event()` 和 `my_events()` 方法

---

## 总结

EventBus 提供了一个清晰、类型安全、高性能的事件通信机制，帮助你构建松耦合的游戏架构。

**核心思想**: 让每个系统专注于自己的职责，通过事件进行通信，而不是相互依赖。
