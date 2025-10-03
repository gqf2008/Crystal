# 新架构使用指南 - PacketHandler 模式

## 📋 概述

新的 `protocol.rs` 实现了基于 **PacketHandler trait** 的数据包处理架构，取代了之前的 `ServerMessage` 枚举方式。

## 🎯 核心概念

### 旧架构 (已废弃)
```rust
// ❌ 旧方式: 创建中间枚举包装所有数据包
enum ServerMessage {
    Connected,
    UserLocation { x: i32, y: i32 },
    // ... 273 个变体
}

// 需要手动解析并映射到枚举
fn parse_server_message(opcode: i16, data: &[u8]) -> ServerMessage {
    match opcode {
        1 => ServerMessage::Connected,
        // ... 273 个 match 分支
    }
}
```

### 新架构 (推荐)
```rust
// ✅ 新方式: 直接使用 SharedRust 的数据包类型
use mir2_shared::packets;

impl PacketHandler for MyGame {
    fn on_connected(&mut self, packet: packets::Connected) {
        // packet 是强类型的 SharedRust 数据包
        println!("服务器版本: {}", packet.version);
    }
    
    fn on_user_location(&mut self, packet: packets::UserLocation) {
        // 直接访问字段，无需解包
        self.player_x = packet.location.x;
        self.player_y = packet.location.y;
    }
}
```

## 🔧 使用方法

### 步骤 1: 实现 PacketHandler trait

```rust
use crate::network::protocol::{PacketHandler, packets};

struct GameClient {
    connected: bool,
    player_position: (i32, i32),
    // ... 其他游戏状态
}

impl PacketHandler for GameClient {
    // 只实现你需要处理的数据包
    
    fn on_connected(&mut self, packet: packets::Connected) {
        tracing::info!("已连接到服务器");
        self.connected = true;
    }
    
    fn on_disconnect(&mut self, packet: packets::Disconnect) {
        tracing::warn!("服务器断开连接: {}", packet.reason);
        self.connected = false;
    }
    
    fn on_user_location(&mut self, packet: packets::UserLocation) {
        self.player_position = (packet.location.x, packet.location.y);
        tracing::debug!("玩家位置更新: {:?}", self.player_position);
    }
    
    fn on_map_information(&mut self, packet: packets::MapInformation) {
        tracing::info!("进入地图: {}", packet.file_name);
        // 处理地图切换逻辑
    }
    
    // 对于不需要处理的数据包，使用默认实现 (什么都不做)
    // 或者实现 on_unknown_packet 来记录未处理的数据包
    
    fn on_unknown_packet(&mut self, opcode: i16, data: &[u8]) {
        tracing::warn!(
            "收到未处理的数据包: opcode={}, size={}",
            opcode,
            data.len()
        );
    }
}
```

### 步骤 2: 在网络层使用分发器

```rust
use crate::network::protocol::{parse_packet_header, dispatch_packet};

// 在接收到网络数据后
fn handle_network_data(data: Vec<u8>, handler: &mut impl PacketHandler) -> Result<()> {
    // 1. 解析头部
    let header = parse_packet_header(&data)?;
    
    // 2. 分发数据包到 handler
    dispatch_packet(header, &data, handler)?;
    
    Ok(())
}

// 在游戏主循环中
async fn game_loop() {
    let mut game = GameClient::new();
    let mut network = NetworkStack::new();
    
    while let Some(event) = network.next_event().await {
        match event {
            NetworkEvent::Packet { header, payload } => {
                // 使用新的分发器
                if let Err(e) = dispatch_packet(header, &payload, &mut game) {
                    tracing::error!("数据包处理失败: {}", e);
                }
            }
            _ => { /* 处理其他事件 */ }
        }
    }
}
```

### 步骤 3: 发送客户端数据包

```rust
use crate::network::protocol::{serialize_client_packet, packets};

async fn send_login(network: &mut NetworkStack) -> Result<()> {
    // 创建客户端数据包
    let packet = packets::client::Login {
        account_id: "player123".to_string(),
        password: "secret".to_string(),
    };
    
    // 序列化
    let bytes = serialize_client_packet(&packet)?;
    
    // 发送
    network.send_bytes(bytes).await?;
    
    Ok(())
}
```

## 🎁 优势

### 1. 类型安全
```rust
// ✅ 编译时类型检查
fn on_user_location(&mut self, packet: packets::UserLocation) {
    let x = packet.location.x; // IDE 自动补全
    let y = packet.location.y; // 类型明确
}

// ❌ 旧方式需要运行时检查
match msg {
    ServerMessage::UserLocation { x, y } => {
        // 需要手动提取字段
    }
}
```

### 2. 易于扩展
```rust
// 添加新数据包处理只需实现对应方法
impl PacketHandler for GameClient {
    fn on_new_packet_type(&mut self, packet: packets::NewPacketType) {
        // 新增处理逻辑
    }
}

// 不需要修改 ServerMessage 枚举
// 不需要修改 parse_server_message 函数
```

### 3. 代码更清晰
```rust
// 每个处理函数职责单一
fn on_connected(&mut self, packet: packets::Connected) {
    // 只处理连接逻辑
}

fn on_user_location(&mut self, packet: packets::UserLocation) {
    // 只处理位置更新
}

// 不再有巨大的 match 语句
```

### 4. 利用 SharedRust 的完整实现
```rust
// SharedRust 已经实现了所有 273 个服务器数据包
// 每个都有完整的序列化/反序列化支持
// 直接使用，无需重复实现
```

## 📦 可用的数据包类型

### 连接相关
- `packets::Connected` - 服务器连接确认
- `packets::Disconnect` - 断开连接
- `packets::KeepAlive` - 心跳包

### 用户/角色
- `packets::UserInformation` - 用户信息
- `packets::UserLocation` - 用户位置
- `packets::UserSlotsRefresh` - 背包刷新

### 地图
- `packets::MapInformation` - 地图信息
- `packets::NewMapInfo` - 新地图详情
- `packets::WorldMapSetupInfo` - 世界地图设置

### 对象
- `packets::ObjectPlayer` - 玩家对象
- `packets::ObjectHero` - 英雄对象
- `packets::ObjectMonster` - 怪物对象
- `packets::ObjectNpc` - NPC 对象
- `packets::ObjectItem` - 物品对象

### 战斗
- `packets::ObjectAttack` - 攻击动作
- `packets::ObjectStruck` - 受击
- `packets::ObjectDied` - 死亡

### 物品/装备
- `packets::DeleteItem` - 删除物品
- `packets::DuraChanged` - 耐久度变化
- `packets::ObjectGold` - 金币对象

### 状态更新
- `packets::HealthChanged` - 生命值变化
- `packets::LevelChanged` - 等级变化
- `packets::GainExperience` - 获得经验
- `packets::ColourChanged` - 颜色变化

**完整列表**: SharedRust 有 273 个服务器数据包，都可以通过 `packets::` 访问

## 🚀 迁移指南

### 从旧代码迁移

#### 旧代码
```rust
match parse_server_message(header, payload) {
    ServerMessage::Connected => {
        self.connected = true;
    }
    ServerMessage::UserLocation { x, y } => {
        self.player_pos = (x, y);
    }
    // ... 更多分支
}
```

#### 新代码
```rust
// 1. 创建 handler
struct MyHandler {
    connected: bool,
    player_pos: (i32, i32),
}

impl PacketHandler for MyHandler {
    fn on_connected(&mut self, _packet: packets::Connected) {
        self.connected = true;
    }
    
    fn on_user_location(&mut self, packet: packets::UserLocation) {
        self.player_pos = (packet.location.x, packet.location.y);
    }
}

// 2. 使用分发器
let mut handler = MyHandler { ... };
dispatch_packet(header, &data, &mut handler)?;
```

## 📝 最佳实践

### 1. 只实现需要的方法
```rust
impl PacketHandler for GameClient {
    // 只实现游戏逻辑需要的数据包
    fn on_connected(&mut self, packet: packets::Connected) { }
    fn on_user_location(&mut self, packet: packets::UserLocation) { }
    // 其他使用默认实现 (空操作)
}
```

### 2. 记录未处理的数据包
```rust
impl PacketHandler for GameClient {
    fn on_unknown_packet(&mut self, opcode: i16, data: &[u8]) {
        tracing::debug!("跳过数据包: opcode={}", opcode);
        // 可以用于调试和发现缺失的处理逻辑
    }
}
```

### 3. 错误处理
```rust
impl PacketHandler for GameClient {
    fn on_map_information(&mut self, packet: packets::MapInformation) {
        if let Err(e) = self.load_map(&packet.file_name) {
            tracing::error!("地图加载失败: {}", e);
            // 处理错误...
        }
    }
}
```

### 4. 状态机模式
```rust
enum GameState {
    Login,
    SelectCharacter,
    InGame,
}

impl PacketHandler for GameClient {
    fn on_connected(&mut self, _packet: packets::Connected) {
        self.state = GameState::Login;
    }
    
    fn on_user_information(&mut self, packet: packets::UserInformation) {
        match self.state {
            GameState::Login => {
                // 登录成功，进入选择角色
                self.state = GameState::SelectCharacter;
            }
            _ => {}
        }
    }
}
```

## 🔍 调试技巧

### 1. 记录所有收到的数据包
```rust
impl PacketHandler for DebugHandler {
    fn on_unknown_packet(&mut self, opcode: i16, data: &[u8]) {
        tracing::trace!(
            "数据包: opcode={:04X}, size={}",
            opcode,
            data.len()
        );
    }
}
```

### 2. 包装 handler 添加日志
```rust
struct LoggingHandler<H: PacketHandler> {
    inner: H,
}

impl<H: PacketHandler> PacketHandler for LoggingHandler<H> {
    fn on_connected(&mut self, packet: packets::Connected) {
        tracing::info!("→ Connected");
        self.inner.on_connected(packet);
    }
    
    // ... 为每个方法添加日志
}
```

## 📚 下一步

- [ ] 在 `dispatch_packet` 中添加更多数据包类型的处理
- [ ] 为 `controls/mod.rs` 创建新的 handler 实现
- [ ] 扩展 `PacketHandler` trait 添加更多数据包方法
- [ ] 创建专门的 handler 用于不同的游戏状态（登录、游戏中等）

---

**总结**: 新架构更简单、更安全、更易维护，充分利用了 SharedRust 的完整实现。
