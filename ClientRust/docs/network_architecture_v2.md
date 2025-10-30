# 网络架构设计文档 (重构版)

## 🎯 设计目标

1. **职责清晰**: 每个组件只负责一件事
2. **线程隔离**: 网络 I/O 在 Tokio 线程,游戏逻辑在主线程
3. **协议复用**: 使用已有的 `protocol.rs` 模块,不重复造轮子
4. **事件驱动**: 通过 NetEventListener trait 解耦

## 📐 架构层次

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: 网络层 (Tokio 异步线程)                             │
│                                                              │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ NetworkManager                                        │   │
│ │ 职责:                                                 │   │
│ │ 1. TCP 连接管理 (connect/disconnect)                 │   │
│ │ 2. 接收原始字节流                                    │   │
│ │ 3. 发送原始字节流                                    │   │
│ │ 4. ❌ 不解析协议,不处理游戏逻辑                      │   │
│ └──────────────────────────────────────────────────────┘   │
│         ↓ mpsc::UnboundedSender<Vec<u8>>                    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ Layer 2: ECS 系统层 (主线程)                                 │
│                                                              │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ NetworkSyncSystem (Priority 150)                     │   │
│ │ 职责:                                                 │   │
│ │ 1. 从 NetworkManager 接收原始字节                    │   │
│ │ 2. 使用 protocol::dispatch_packet() 解析             │   │
│ │ 3. GameClient (PacketHandler) 生成 GameEvent        │   │
│ │ 4. 通过 event_tx 发送 GameEvent                      │   │
│ └──────────────────────────────────────────────────────┘   │
│         ↓ mpsc::UnboundedSender<GameEvent>                  │
│                                                              │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ GameState (实现 NetEventListener)                    │   │
│ │ 职责:                                                 │   │
│ │ 1. 接收 GameEvent                                    │   │
│ │ 2. 根据当前场景分发事件                              │   │
│ │ 3. 写入 GlobalEvents.game_events                     │   │
│ └──────────────────────────────────────────────────────┘   │
│         ↓                                                    │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ GlobalEvents.game_events: Vec<GameEvent>             │   │
│ └──────────────────────────────────────────────────────┘   │
│         ↓                                                    │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ GameEventSystem (Priority 510)                       │   │
│ │ 职责:                                                 │   │
│ │ 1. 从 GlobalEvents 读取 game_events                  │   │
│ │ 2. 创建/更新 ECS 实体和组件                          │   │
│ │ 3. 游戏状态只存在于 ECS World                        │   │
│ └──────────────────────────────────────────────────────┘   │
│         ↓                                                    │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ ECS World (Components)                               │   │
│ │ - Position, Health, PlayerData, MonsterData...       │   │
│ └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ Layer 3: 协议层 (无状态函数库)                               │
│                                                              │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ protocol.rs                                           │   │
│ │ - PacketHandler trait (276 个 on_* 方法)            │   │
│ │ - dispatch_packet(header, payload, handler)          │   │
│ │ - serialize_client_packet<P>(packet) -> Vec<u8>      │   │
│ └──────────────────────────────────────────────────────┘   │
│         ↑ 使用                                               │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ GameClient (实现 PacketHandler)                      │   │
│ │ 职责:                                                 │   │
│ │ 1. 实现 276 个 on_* 方法                             │   │
│ │ 2. 将服务器数据包转换为 GameEvent                    │   │
│ │ 3. ❌ 不维护游戏状态 (objects, player, hero)         │   │
│ │ 4. ✅ 只负责协议转换                                 │   │
│ └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 关键组件职责

### NetworkManager (Tokio 线程)

```rust
// src/network/network_manager.rs

职责:
✅ TCP 连接生命周期管理
✅ 接收/发送原始字节流
✅ 发送字节到 mpsc channel
❌ 不解析协议
❌ 不调用 GameClient
❌ 不处理游戏逻辑

关键修改:
- 删除 `game_client: Arc<RwLock<GameClient>>`
- 删除 `dispatch_server_packet()` 方法
- 添加 `packet_tx: mpsc::UnboundedSender<Vec<u8>>`
```

### NetworkSyncSystem (ECS 系统, Priority 150)

```rust
// src/ecs/systems/update/input/network_sync_system_v2.rs

职责:
✅ 从 NetworkManager 接收原始字节
✅ 使用 protocol::dispatch_packet() 解析
✅ GameClient 生成 GameEvent
✅ 发送 GameEvent 到 event_tx

数据流:
packet_receiver (Vec<u8>) 
  → process_packet() 
  → protocol::dispatch_packet(header, payload, &mut game_client)
  → GameClient::on_object_player() 等
  → send_event(GameEvent::ObjectSpawned)
  → event_tx channel
```

### NetEventListener trait (接口)

```rust
// src/network/net_event_listener.rs

pub trait NetEventListener {
    fn on_net_event(&mut self, event: GameEvent);
    fn on_net_events(&mut self, events: Vec<GameEvent>);
    fn on_connection_changed(&mut self, connected: bool);
    fn on_network_error(&mut self, error: String);
}

实现者:
- GameState: 根据场景分发事件
- 可扩展: 其他组件也可以实现此 trait
```

### GameClient (协议转换器)

```rust
// src/network/game_client.rs

当前状态 (需要重构):
❌ pub objects: HashMap<u32, GameObject>  // 删除
❌ pub player: Option<PlayerState>        // 删除
❌ pub hero: Option<HeroState>            // 删除
❌ pub map_info: Option<MapInfo>          // 删除
✅ pub event_tx: Option<UnboundedSender<GameEvent>>  // 保留

重构后:
impl PacketHandler for GameClient {
    fn on_object_player(&mut self, packet: ObjectPlayer) {
        // ✅ 只做协议转换
        let obj = GameObject::Player { ... };
        self.send_event(GameEvent::ObjectSpawned { object: obj });
        
        // ❌ 不维护状态
        // self.objects.insert(obj.id, obj);  // 删除这种代码
    }
}
```

### GameEventSystem (ECS 系统, Priority 510)

```rust
// src/ecs/systems/update/state_update/game_event_system.rs

职责:
✅ 从 GlobalEvents.game_events 读取事件
✅ 根据事件类型创建/更新 ECS 实体
✅ 游戏状态只存在于 ECS World

示例:
GameEvent::ObjectSpawned { object: GameObject::Player {...} }
  → world.spawn((
      NetworkSync::new(id, NetworkObjectType::Player),
      Position { x, y },
      PlayerData { name, level, ... },
      Sprite, Health, AnimationState
  ))
```

## 📊 数据流对比

### ❌ 旧架构 (问题)

```
NetworkManager (Tokio)
    ↓ 直接调用 GameClient (跨线程!)
GameClient 维护游戏状态 (❌ 不应该)
    ↓ 发送 GameEvent
❓ 谁处理 GameEvent? (没有明确的系统)
```

### ✅ 新架构 (正确)

```
NetworkManager (Tokio)
    ↓ mpsc::channel<Vec<u8>>
NetworkSyncSystem (ECS, 主线程)
    ↓ protocol::dispatch_packet()
GameClient (PacketHandler, 无状态)
    ↓ mpsc::channel<GameEvent>
GameState (NetEventListener)
    ↓ GlobalEvents.game_events
GameEventSystem (ECS)
    ↓ ECS World (唯一真实来源)
```

## 🎮 为什么不需要 NetworkPacketParserSystem?

**原因**:
1. `protocol.rs` 已经提供完整的解析功能
2. `dispatch_packet()` 基于 opcode 自动分发
3. `GameClient` 实现 `PacketHandler` trait
4. 在 `NetworkSyncSystem` 中直接调用更高效

**对比**:
```
❌ 低效方案:
NetworkSyncSystem (bytes → GlobalEvents.network_incoming)
  → NetworkPacketParserSystem (bytes → GameEvent)
  → GameEventSystem (GameEvent → ECS)

✅ 高效方案:
NetworkSyncSystem (bytes → GameEvent)
  → GameEventSystem (GameEvent → ECS)
```

## 🔥 重构 Checklist

### Phase 1: NetworkManager 简化 ✅

- [ ] 删除 `game_client: Arc<RwLock<GameClient>>`
- [ ] 删除 `dispatch_server_packet()` 方法
- [ ] 添加 `packet_tx: mpsc::UnboundedSender<Vec<u8>>`
- [ ] 修改 `process()` 方法直接发送原始字节

### Phase 2: NetworkSyncSystem 重构 ✅

- [x] 创建 `network_sync_system_v2.rs`
- [x] 添加 `game_client: GameClient`
- [x] 实现 `process_packet()` 方法
- [x] 使用 `protocol::dispatch_packet()`
- [x] 发送 GameEvent 到 event_tx

### Phase 3: GameClient 状态清理 ⏸️

- [ ] 删除 `objects: HashMap<u32, GameObject>`
- [ ] 删除 `player: Option<PlayerState>`
- [ ] 删除 `hero: Option<HeroState>`
- [ ] 删除 `map_info: Option<MapInfo>`
- [ ] 保留 `event_tx` 用于发送 GameEvent

### Phase 4: GameState 实现 NetEventListener ✅

- [x] 创建 `net_event_listener.rs`
- [x] GameState 实现 trait
- [x] 处理连接/断开事件
- [x] 分发事件到当前场景

### Phase 5: 替换旧 NetworkSyncSystem ⏸️

- [ ] 用 `network_sync_system_v2.rs` 替换旧版本
- [ ] 更新 mod.rs 导出
- [ ] 更新系统注册代码
- [ ] 测试编译

### Phase 6: 端到端测试 ⏸️

- [ ] 测试登录流程
- [ ] 测试角色选择
- [ ] 测试游戏场景
- [ ] 测试 NPC 交互
- [ ] 测试对象生成/移除

## 📝 代码示例

### 1. NetworkManager 发送原始字节

```rust
// src/network/network_manager.rs

pub struct NetworkManager {
    network: NetworkStack,
    packet_tx: mpsc::UnboundedSender<Vec<u8>>,  // 🆕
    // ❌ 删除: game_client: Arc<RwLock<GameClient>>
}

impl NetworkManager {
    pub async fn process(&mut self) -> Result<()> {
        while let Some(event) = self.network.poll_event() {
            match event {
                NetworkEvent::ServerPacket { header, payload } => {
                    // ✅ 直接发送原始字节
                    let _ = self.packet_tx.send(payload);
                    
                    // ❌ 删除: self.dispatch_server_packet(header, &payload);
                }
                _ => {}
            }
        }
        Ok(())
    }
}
```

### 2. NetworkSyncSystem 解析并生成事件

```rust
// src/ecs/systems/update/input/network_sync_system_v2.rs

impl NetworkSyncSystem {
    fn process_packet(&mut self, payload: &[u8]) -> Result<(), String> {
        let header = PacketHeader::parse(payload)?;
        
        // 使用 protocol 模块解析
        protocol::dispatch_packet(header, payload, &mut self.game_client)?;
        
        // GameClient 在 on_* 方法中已经发送了 GameEvent
        Ok(())
    }
}
```

### 3. GameState 接收并分发事件

```rust
// src/ecs/game_app.rs

impl NetEventListener for GameState {
    fn on_net_event(&mut self, event: GameEvent) {
        match self.scene_type {
            SceneType::Game => {
                // 写入 GlobalEvents,由 GameEventSystem 处理
                if let Some(game_scene) = self.current_scene.as_mut()... {
                    game_scene.handle_network_event(&mut self.world, &event);
                }
            }
            _ => { /* 其他场景处理 */ }
        }
    }
}
```

### 4. GameEventSystem 创建 ECS 实体

```rust
// src/ecs/systems/update/state_update/game_event_system.rs

impl System for GameEventSystem {
    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        let events = { /* drain GlobalEvents.game_events */ };
        
        for event in events {
            match event {
                GameEvent::ObjectSpawned { object } => {
                    self.handle_object_spawned(world, object);
                }
                _ => {}
            }
        }
        Ok(())
    }
}
```

## ✨ 架构优势

1. **职责清晰**: 每个组件只做一件事
2. **线程安全**: 网络 I/O 和游戏逻辑完全隔离
3. **易于测试**: 每个组件可以独立测试
4. **易于扩展**: 新场景只需实现 NetEventListener
5. **性能优化**: 减少跨线程同步,减少数据复制

## 🚀 下一步

1. 完成 NetworkManager 简化
2. 替换旧 NetworkSyncSystem
3. 清理 GameClient 状态
4. 端到端测试
5. 性能优化
