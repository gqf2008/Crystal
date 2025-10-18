# GameScene Bevy 架构设计与模块复用

## 📋 目标

将 ggez 版本的 GameScene 移植到 Bevy,复用现有的模块:
- ✅ `graphics` - MLibrary 纹理系统和渲染管理
- ✅ `network` - 网络协议和包处理
- ✅ `objects` - 游戏对象系统 (MapObject, UserObject, MonsterObject 等)
- ✅ `resolution` - 分辨率管理
- ✅ `resources` - 嵌入式资源
- ✅ `SharedRust` - 共享数据结构和包定义

## 🏗️ 架构设计

### 核心原则

1. **数据驱动** - 使用 Bevy ECS 管理游戏状态
2. **模块复用** - 复用 ggez 版本的核心逻辑
3. **渲染分离** - 使用 Bevy 渲染,但保持 MLibrary 纹理加载
4. **网络独立** - 网络层独立于渲染层
5. **对象系统桥接** - 在 ECS 和对象系统之间建立桥接

### 模块依赖关系

```
GameScene (Bevy)
├── components.rs          [ECS 组件定义]
│   └── 复用: objects::MapObject traits
│
├── constants.rs           [常量]
│   └── 复用: resolution 配置
│
├── player_systems.rs      [玩家系统]
│   ├── 复用: objects::UserObject
│   └── 复用: SharedRust::Stats
│
├── map_systems.rs         [地图系统] ⚠️ 需要完善
│   ├── 复用: objects::map_code (MapReader)
│   ├── 复用: graphics::MLibrary
│   └── 复用: ggez::game_scene::MapRenderer 逻辑
│
├── interaction_systems.rs [交互系统]
│   └── 复用: objects::NpcObject
│
├── chat_systems.rs        [聊天系统]
│   └── 复用: network 消息
│
├── network_systems.rs     [网络系统] ⚠️ 需要完善
│   ├── 复用: network::NetworkManager
│   ├── 复用: SharedRust::packets
│   └── 复用: ggez::game_scene packet 处理逻辑
│
└── game_loop_systems.rs   [游戏循环]
    └── 复用: ggez::game_scene 事件处理

Bevy 特定模块:
├── rendering/             [渲染层] 🆕 需要创建
│   ├── mlibrary_assets.rs    - Bevy 资源加载 MLibrary
│   ├── sprite_renderer.rs    - Sprite 批处理渲染
│   └── map_renderer.rs       - 地图渲染 (移植自 ggez)
│
└── bridge/                [桥接层] 🆕 需要创建
    ├── object_sync.rs        - ECS ↔ MapObject 同步
    └── network_bridge.rs     - 网络线程 ↔ ECS 通信
```

## 🔧 需要完善的模块

### 1. map_systems.rs - 地图加载和渲染

**当前状态**: 只有占位符函数
**需要移植**:
- ggez `game_scene.rs` 的 MapRenderer 逻辑
- ggez `camera.rs` 的摄像机系统
- C# GameScene.cs 的 MapControl 逻辑

**复用模块**:
```rust
use crate::objects::map_code::{MapReader, CellInfo};  // 地图数据读取
use crate::graphics::MLibrary;                         // 纹理加载
use crate::graphics::Libraries;                        // 库管理
```

**关键功能**:
- `load_map_system()` - 加载地图文件 (.map)
- `create_map_layers_system()` - 创建图层 (Ground, LowWall, HighWall, etc.)
- `spawn_map_objects_system()` - 生成地图对象 (NPC, Monster, Item)
- `update_map_state_system()` - 更新地图状态
- `render_map_system()` - 渲染地图 (需要创建新的 rendering 模块)

### 2. network_systems.rs - 网络同步

**当前状态**: 只有模拟代码
**需要移植**:
- ggez `game_scene.rs` 的 `process_packet()` 逻辑
- C# GameScene.cs 的网络包处理

**复用模块**:
```rust
use crate::network::NetworkManager;                    // 网络管理
use mir2_shared::packets::server::*;                   // 服务器包
use mir2_shared::packets::client::*;                   // 客户端包
```

**关键功能**:
- `process_server_packets_system()` - 处理服务器包
- `send_player_action_system()` - 发送玩家动作
- `sync_objects_system()` - 同步游戏对象
- 参考 ggez 版本的 2000+ 行包处理代码

### 3. rendering/ - Bevy 渲染层 (新建)

**需要创建**:
- `mlibrary_assets.rs` - Bevy 资源系统集成
- `sprite_renderer.rs` - Sprite 批处理
- `map_renderer.rs` - 地图渲染

**参考**:
- ggez `map_renderer.rs` - 地图渲染逻辑
- ggez `camera.rs` - 摄像机系统
- Bevy 示例: 2D sprite batching

### 4. bridge/ - 对象系统桥接 (新建)

**需要创建**:
- `object_sync.rs` - MapObject ↔ Bevy Entity 同步
- `network_bridge.rs` - 网络线程 ↔ ECS 通信

**功能**:
```rust
// object_sync.rs
// 将 MapObject 的状态同步到 Bevy Entity
pub fn sync_map_object_to_entity(object: &MapObject, entity: Entity, commands: &mut Commands);

// 将 Bevy Entity 的状态同步到 MapObject
pub fn sync_entity_to_map_object(entity: Entity, object: &mut MapObject, query: &Query<...>);

// network_bridge.rs
// 从网络线程接收包,转换为 Bevy 事件
pub fn network_to_bevy_system(network: Res<NetworkManager>, events: EventWriter<ServerPacket>);

// 从 Bevy 系统发送包到网络线程
pub fn bevy_to_network_system(events: EventReader<ClientPacket>, network: Res<NetworkManager>);
```

## 📦 复用策略

### graphics::MLibrary (纹理加载)

**ggez 版本**:
```rust
// 使用 ggez::graphics::Image
let texture = ggez_manager.get_texture("Tiles_001");
canvas.draw(texture, DrawParam::default());
```

**Bevy 版本**:
```rust
// 创建 Bevy 资源系统适配器
pub struct MLibraryAssets {
    textures: HashMap<String, Handle<Image>>,
    mlibrary: MLibrary,
}

impl MLibraryAssets {
    pub fn load_texture(&mut self, 
        name: &str, 
        images: &mut Assets<Image>
    ) -> Option<Handle<Image>> {
        // 1. 从 MLibrary 读取原始数据
        let image_info = self.mlibrary.get_image(name)?;
        
        // 2. 转换为 Bevy Image
        let bevy_image = Image::new(
            Extent3d { width: image_info.width, height: image_info.height, .. },
            TextureDimension::D2,
            image_info.data,
            TextureFormat::Rgba8UnormSrgb,
        );
        
        // 3. 存储到 Assets
        let handle = images.add(bevy_image);
        self.textures.insert(name.to_string(), handle.clone());
        Some(handle)
    }
}
```

### objects::MapObject (游戏对象)

**ggez 版本**:
```rust
// MapObject 直接管理状态和渲染
impl MapObject {
    fn update(&mut self, delta: f32) { ... }
    fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) { ... }
}
```

**Bevy 版本**:
```rust
// 使用 ECS 组件 + MapObject 桥接
#[derive(Component)]
pub struct MapObjectRef {
    object: Arc<Mutex<Box<dyn MapObject>>>,
}

// 同步系统
fn sync_map_objects_system(
    mut query: Query<(&mut Transform, &MapObjectRef)>
) {
    for (mut transform, obj_ref) in query.iter_mut() {
        let obj = obj_ref.object.lock().unwrap();
        transform.translation = Vec3::new(obj.get_x(), obj.get_y(), 0.0);
    }
}
```

### network::NetworkManager (网络管理)

**ggez 版本**:
```rust
// 在主线程中轮询网络
impl Scene for GameScene {
    fn update(&mut self) {
        while let Some(packet) = self.network.poll_packet() {
            self.process_packet(packet);
        }
    }
}
```

**Bevy 版本**:
```rust
// 使用 Bevy 事件系统
#[derive(Event)]
pub struct ServerPacketEvent(pub ServerPacket);

// 网络接收系统 (独立线程)
fn network_receive_system(
    network: Res<NetworkManager>,
    mut events: EventWriter<ServerPacketEvent>
) {
    while let Some(packet) = network.poll_packet() {
        events.send(ServerPacketEvent(packet));
    }
}

// 包处理系统
fn process_packets_system(
    mut events: EventReader<ServerPacketEvent>,
    mut game_state: ResMut<GameSceneState>
) {
    for event in events.read() {
        match &event.0 {
            ServerPacket::ObjectPlayer(p) => { /* 处理玩家对象 */ }
            ServerPacket::ObjectMonster(p) => { /* 处理怪物对象 */ }
            // ... 参考 ggez 版本的 2000+ 行处理逻辑
        }
    }
}
```

## 🚀 实施计划

### Phase 1: 基础设施 (1-2天)

1. ✅ 创建模块结构 (已完成)
2. 🔄 创建 `rendering/` 模块
3. 🔄 创建 `bridge/` 模块
4. 🔄 集成 MLibrary 到 Bevy 资源系统

### Phase 2: 地图系统 (2-3天)

1. 🔄 移植 MapRenderer 逻辑
2. 🔄 实现地图加载 (`map_systems.rs`)
3. 🔄 实现地图渲染 (`rendering/map_renderer.rs`)
4. 🔄 实现摄像机跟随

### Phase 3: 对象系统 (2-3天)

1. 🔄 实现 MapObject 桥接 (`bridge/object_sync.rs`)
2. 🔄 移植 UserObject 逻辑
3. 🔄 移植 MonsterObject 逻辑
4. 🔄 移植 NpcObject 逻辑

### Phase 4: 网络系统 (3-4天)

1. 🔄 实现网络桥接 (`bridge/network_bridge.rs`)
2. 🔄 移植包处理逻辑 (参考 ggez `process_packet()`)
3. 🔄 实现客户端包发送
4. 🔄 实现服务器包处理

### Phase 5: 测试和优化 (2-3天)

1. 🔄 集成测试
2. 🔄 性能优化
3. 🔄 Bug 修复

## 📚 参考代码位置

### ggez 版本核心文件

```
ClientRust/src/scenes/game_scene.rs        (2344行) - 主场景逻辑
ClientRust/src/scenes/game_scene/
├── camera.rs                              - 摄像机系统
└── map_renderer.rs                        - 地图渲染器
```

### 关键复用模块

```
ClientRust/src/graphics/
├── mlibrary.rs                            - MLibrary 核心实现
└── ggez_manager_simple.rs                 - ggez 渲染管理

ClientRust/src/objects/
├── map_object.rs                          - MapObject trait
├── user_object.rs                         - UserObject (玩家)
├── monster_object.rs                      - MonsterObject (怪物)
├── npc_object.rs                          - NpcObject (NPC)
└── map_code.rs                            - MapReader (地图加载)

ClientRust/src/network/
├── network_manager.rs                     - 网络管理器
└── protocol.rs                            - 协议定义

SharedRust/src/packets/
├── server/                                - 服务器包定义
└── client/                                - 客户端包定义
```

## 🎯 下一步行动

1. **立即**: 创建 `rendering/` 和 `bridge/` 模块
2. **优先**: 完善 `map_systems.rs` 地图加载
3. **重要**: 完善 `network_systems.rs` 包处理
4. **后续**: 测试和优化

## 📝 注意事项

1. **保持一致性**: 尽量保持 ggez 版本的逻辑,减少移植风险
2. **性能考虑**: Bevy 的 ECS 可能比对象系统更高效,需要性能测试
3. **线程安全**: 注意网络线程和主线程的同步
4. **资源管理**: MLibrary 的纹理数据较大,注意内存管理
