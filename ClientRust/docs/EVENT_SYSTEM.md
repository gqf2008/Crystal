# 全局事件系统使用指南

## 概述

全局事件系统是一个事件驱动的 ECS 架构核心组件，用于统一管理游戏中的所有事件。

## 设计原则

1. **统一事件管理**：所有事件（键盘、鼠标、IME、网络）统一存储在 `GlobalEvents` 组件中
2. **Vec 缓存**：使用 `Vec` 缓存事件，支持多系统并发读取
3. **Channel 机制**：网络立即发送事件使用 `Channel`，其他事件使用队列
4. **自动清理**：每帧结束自动清理，防止事件重放
5. **便捷过滤**：提供丰富的事件过滤方法

## 快速开始

### 1. 初始化全局事件组件

在游戏启动时创建 `GlobalEvents` 组件：

```rust
use mir2_client::ecs::{GlobalEvents, EventCollectorSystem};

// 在 World 初始化时
let mut world = World::new();
EventCollectorSystem::ensure_global_events(&mut world);
```

### 2. 收集事件

在 GGEZ 事件处理器中收集事件：

```rust
impl EventHandler for MyGame {
    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            // 获取全局事件组件
            if let Some(events) = EventCollectorSystem::get_global_events_mut(&mut self.world) {
                events.push_keyboard(keycode, true, false);
            }
        }
        Ok(())
    }
    
    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) -> GameResult {
        if let Some(events) = EventCollectorSystem::get_global_events_mut(&mut self.world) {
            events.push_mouse(MouseEvent::ButtonDown { button, x, y });
        }
        Ok(())
    }
}
```

### 3. 在系统中消费事件

各个 ECS 系统读取自己关注的事件：

```rust
// 移动系统示例
pub struct MovementSystem;

impl MovementSystem {
    pub fn update(world: &mut World) -> GameResult {
        // 获取全局事件（只读）
        let events = EventCollectorSystem::get_global_events(world);
        
        if let Some(events) = events {
            // 过滤 W 键按下事件
            for key_event in events.filter_key(KeyCode::KeyW) {
                if key_event.pressed {
                    println!("玩家按下 W 键，向上移动");
                    
                    // 修改实体组件
                    for (_, (mut pos, player)) in world.query::<(&mut Position, &Player)>().iter() {
                        pos.y -= 1.0;
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

### 4. 清理事件

在每帧结束时调用清理：

```rust
impl EventHandler for MyGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 执行所有系统...
        MovementSystem::update(&mut self.world)?;
        CombatSystem::update(&mut self.world)?;
        
        // 🔥 重要：在帧末尾清理事件
        EventCleanupSystem::cleanup(&mut self.world)?;
        
        Ok(())
    }
}
```

## 事件类型

### 键盘事件

```rust
// 添加键盘事件
events.push_keyboard(KeyCode::KeyW, true, false);

// 过滤键盘事件
for key in events.filter_key_pressed() {
    println!("按下: {:?}", key.keycode);
}

for key in events.filter_key(KeyCode::Escape) {
    println!("ESC 键");
}
```

### 鼠标事件

```rust
// 添加鼠标移动
events.push_mouse(MouseEvent::Move { x: 100.0, y: 200.0, dx: 1.0, dy: 2.0 });

// 添加鼠标按钮
events.push_mouse(MouseEvent::ButtonDown { 
    button: MouseButton::Left, 
    x: 100.0, 
    y: 200.0 
});

// 过滤鼠标事件
for mouse_move in events.filter_mouse_move() {
    if let MouseEvent::Move { x, y, .. } = mouse_move {
        println!("鼠标移动到: ({}, {})", x, y);
    }
}

for click in events.filter_mouse_button_down(MouseButton::Left) {
    println!("左键点击");
}
```

### IME 字符输入

```rust
// 添加字符输入
events.push_ime('中');

// 读取字符输入
for ime_event in &events.ime_events {
    println!("输入字符: {}", ime_event.character);
}
```

### 游戏事件

```rust
// 添加游戏事件
events.push_game_event(GameEvent::PlayerMoveRequest {
    target_x: 100.0,
    target_y: 200.0,
    run: true,
});

// 过滤游戏事件
for event in events.filter_game_events(|e| {
    matches!(e, GameEvent::PlayerMoveRequest { .. })
}) {
    println!("玩家移动请求: {:?}", event);
}
```

### 网络包（立即发送）

```rust
// 发送网络命令到网络线程（立即发送）
events.send_network_command(NetworkCommand::Walk {
    direction: MirDirection::Up,
});

// 在 NetworkSyncSystem 中添加接收到的包
events.push_incoming_packet(NetworkPacket {
    packet_type: "ObjectPlayer".to_string(),
    data: vec![1, 2, 3],
});

// 在 PacketProcessingSystem 中处理接收到的包
for packet in events.drain_incoming_packets() {
    println!("处理网络包: {}", packet.packet_type);
}
```

## 高级用法

### 自定义事件过滤

```rust
// 过滤特定条件的游戏事件
let attack_events: Vec<_> = events.filter_game_events(|e| {
    matches!(e, GameEvent::AttackRequest { .. })
}).collect();

// 组合过滤
let shift_w_pressed = events.keyboard_events.iter().any(|e| {
    e.keycode == KeyCode::ShiftLeft && e.pressed
}) && events.keyboard_events.iter().any(|e| {
    e.keycode == KeyCode::KeyW && e.pressed
});
```

### 事件统计

```rust
// 获取当前帧统计
let stats = events.get_frame_stats();
println!("键盘事件: {}", stats.keyboard_count);
println!("鼠标事件: {}", stats.mouse_count);
println!("总事件数: {}", stats.total_count);

// 打印统计（调试）
EventCleanupSystem::print_stats(&world);
```

### 启用事件日志

```rust
// 启用详细日志
if let Some(events) = EventCollectorSystem::get_global_events_mut(&mut world) {
    events.enable_logging = true;
}

// 每次事件添加时会打印：
// 🎹 键盘事件: KeyW 按下
// 🖱️  鼠标事件: Move { x: 100.0, y: 200.0, dx: 10.0, dy: 20.0 }
// ✏️  IME 输入: '中'
// 🎮 游戏事件: PlayerMoveRequest { ... }
```

## 系统执行顺序

推荐的系统执行顺序：

```
1. EventCollectorSystem (确保 GlobalEvents 存在)
2. InputSystem (收集输入到 GlobalEvents)
3. MovementSystem (读取移动事件)
4. CombatSystem (读取战斗事件)
5. UISystem (读取 UI 事件)
6. NetworkSystem (处理网络包)
...
N. EventCleanupSystem (清理当前帧事件) ← 最后执行！
```

## 注意事项

1. **单例模式**：`GlobalEvents` 应该只在 World 中创建一个实例
2. **只读访问**：大部分系统应该只读取事件，不修改事件队列
3. **清理时机**：必须在所有系统执行完毕后调用 `EventCleanupSystem::cleanup()`
4. **网络包特殊性**：网络包使用 Channel，不受帧清理影响
5. **并发读取**：多个系统可以同时读取同一批事件（使用 `&GlobalEvents`）

## 性能考虑

- **Vec 缓存**：事件使用 `Vec` 存储，避免频繁分配
- **过滤器惰性**：所有过滤器都是惰性的（`Iterator`），不会创建中间集合
- **Channel 零拷贝**：网络包使用 `mpsc::channel`，零拷贝传输
- **自动清理**：每帧自动清理，避免内存泄漏

## 完整示例

参见 `examples/event_system_demo.rs` 了解完整的集成示例。

## 测试

运行测试：

```bash
cargo test --lib ecs::components::events
cargo test --lib ecs::systems::update::state_update::event_cleanup_system
```
