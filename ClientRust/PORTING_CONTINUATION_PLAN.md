# ClientRust 继续移植工作计划

## 📊 当前状态分析

### 编译错误统计
- **总错误数**: 154个
- **主要错误类型**:
  1. `E0432`: 未解析的导入 (占大多数)
  2. `E0382`: 值移动错误
  3. `E0046`: 未实现的trait方法
  4. `E0277`: trait未实现
  
### 核心问题
1. ❌ 模块导入路径错误 (`crate::audio`, `crate::net`, `crate::protocol`等不存在)
2. ❌ SharedRust模块路径变更 (`mir2_shared::client_packets` → `mir2_shared::packets::client`)
3. ❌ 协议处理模块位置不明确
4. ⚠️ 一些trait实现缺失

---

## 🎯 移植策略

### 阶段1: 修复模块结构 (优先级: 🔴 最高)

#### 1.1 更新SharedRust导入
**错误**: `use mir2_shared::client_packets::*`  
**正确**: `use mir2_shared::packets::client::*`

**错误**: `use mir2_shared::stats::*`  
**正确**: `use mir2_shared::data::stats::*`

**错误**: `use mir2_shared::client_data::*`  
**正确**: `use mir2_shared::data::client_data::*`

#### 1.2 统一网络模块结构
```
ClientRust/src/network/
├── mod.rs          # 网络模块入口
├── network.rs      # TCP连接和数据收发 (已完成)
├── protocol.rs     # 协议处理器 (需创建)
└── handlers/       # 数据包处理器
    ├── mod.rs
    ├── connection.rs   # 连接相关 (Connected, Disconnect等)
    ├── account.rs      # 账户相关 (Login, NewCharacter等)
    ├── player.rs       # 玩家相关 (ObjectPlayer, PlayerUpdate等)
    ├── object.rs       # 对象相关 (ObjectRemove, ObjectTurn等)
    ├── item.rs         # 物品相关 (UserItem, GainedItem等)
    ├── magic.rs        # 魔法相关 (NewMagic, MagicDelay等)
    ├── npc.rs          # NPC相关 (NPCResponse等)
    ├── guild.rs        # 公会相关
    ├── group.rs        # 组队相关
    ├── quest.rs        # 任务相关
    └── ...
```

### 阶段2: 实现协议处理器 (优先级: 🟠 高)

#### 2.1 创建Protocol Router
利用SharedRust的完整数据包定义:

```rust
// src/network/protocol.rs
use mir2_shared::packets::server::*;
use mir2_shared::packets::base::Packet;
use std::io::Cursor;

pub struct ProtocolHandler {
    // 游戏状态引用
}

impl ProtocolHandler {
    pub fn handle_packet(&mut self, opcode: i16, data: &[u8]) -> Result<()> {
        use mir2_shared::enums::ServerPacketIds;
        
        match opcode {
            x if x == ServerPacketIds::Connected as i16 => {
                let packet = connection::Connected::read_body(&mut Cursor::new(data))?;
                self.handle_connected(packet)
            }
            x if x == ServerPacketIds::LoginSuccess as i16 => {
                let packet = account::LoginSuccess::read_body(&mut Cursor::new(data))?;
                self.handle_login_success(packet)
            }
            // ... 273个服务器数据包
            _ => {
                warn!("Unknown packet opcode: {}", opcode);
                Ok(())
            }
        }
    }
}
```

#### 2.2 实现各类处理器
每个handler负责处理特定类别的数据包，更新游戏状态。

### 阶段3: 发送数据包 (优先级: 🟡 中)

#### 3.1 创建发包辅助函数
```rust
// src/network/network.rs 扩展

impl NetworkStack {
    /// 发送客户端数据包
    pub fn send_packet<P: Packet>(&mut self, packet: &P) -> Result<()> {
        let mut buffer = Vec::new();
        
        // 写入长度占位符
        buffer.extend_from_slice(&[0u8, 0u8]);
        
        // 写入opcode
        buffer.write_i16::<LittleEndian>(P::OPCODE)?;
        
        // 写入数据包体
        packet.write_body(&mut buffer)?;
        
        // 回填长度
        let length = buffer.len() as u16;
        LittleEndian::write_u16(&mut buffer[0..2], length);
        
        // 加入发送队列
        self.send_queue.push_back(buffer);
        
        Ok(())
    }
}
```

#### 3.2 使用示例
```rust
use mir2_shared::packets::client::connection::ClientVersion;

// 发送客户端版本
let version_packet = ClientVersion {
    version_hash: calculate_hash(),
};
network.send_packet(&version_packet)?;
```

---

## 📋 具体实施步骤

### Step 1: 修复所有导入错误 (预计: 2小时)
- [ ] 全局搜索替换 `mir2_shared::client_packets` → `mir2_shared::packets::client`
- [ ] 全局搜索替换 `mir2_shared::stats` → `mir2_shared::data::stats`
- [ ] 全局搜索替换 `mir2_shared::client_data` → `mir2_shared::data::client_data`
- [ ] 移除不存在的模块导入 (`crate::audio`, `crate::net`, `crate::protocol`等)
- [ ] 统一使用新的模块结构

### Step 2: 创建网络模块 (预计: 3小时)
- [ ] 创建 `src/network/protocol.rs` - 协议路由器
- [ ] 创建 `src/network/handlers/` 目录结构
- [ ] 实现 `handlers/connection.rs` (4个数据包)
- [ ] 实现 `handlers/account.rs` (登录相关)
- [ ] 扩展 `NetworkStack::send_packet()` 方法

### Step 3: 实现核心数据包处理 (预计: 5小时)
- [ ] Connection handlers (Connected, Disconnect, KeepAlive等)
- [ ] Account handlers (LoginSuccess, NewCharacter等)
- [ ] Player handlers (ObjectPlayer, PlayerUpdate等)
- [ ] Object handlers (ObjectRemove, ObjectTurn等)

### Step 4: 修复其他编译错误 (预计: 2小时)
- [ ] 修复 `UserItem` clone问题
- [ ] 实现缺失的trait方法
- [ ] 修复生命周期错误

### Step 5: 测试和验证 (预计: 2小时)
- [ ] 编译通过
- [ ] 单元测试
- [ ] 连接服务器测试

---

## 🔧 技术要点

### 1. 利用SharedRust的完整实现

SharedRust已经实现了:
- ✅ 所有273个服务器数据包的反序列化
- ✅ 所有146个客户端数据包的序列化
- ✅ 完整的枚举定义(61个,103%完成度)
- ✅ .NET兼容的二进制序列化

**我们只需要**:
- 📌 读取数据包 → 调用 `Packet::read_body()`
- 📌 发送数据包 → 调用 `Packet::write_body()`
- 📌 处理业务逻辑 → 更新游戏状态

### 2. 数据包处理模式

```rust
// 接收数据包
match opcode {
    x if x == ServerPacketIds::ObjectPlayer as i16 => {
        let packet = player::ObjectPlayer::read_body(&mut reader)?;
        
        // 业务逻辑
        game_state.add_player(packet.object_id, packet);
    }
}

// 发送数据包
let walk_packet = movement::Walk {
    direction: MirDirection::Up,
};
network.send_packet(&walk_packet)?;
```

### 3. 错误处理策略

```rust
use mir2_shared::data::stats::{SharedResult, SharedError};

fn handle_packet_data(data: &[u8]) -> SharedResult<()> {
    // 使用SharedRust的错误类型
    let packet = SomePacket::read_body(&mut Cursor::new(data))?;
    
    // 处理...
    
    Ok(())
}
```

---

## 📦 SharedRust提供的工具

### 可直接使用的模块

```rust
// 数据包
use mir2_shared::packets::{
    client::*,    // 146个客户端数据包
    server::*,    // 273个服务器数据包
    base::Packet, // Packet trait
};

// 枚举
use mir2_shared::enums::{
    MirDirection,      // 8方向
    MirClass,          // 6职业
    Spell,             // 146技能
    ItemType,          // 57物品类型
    ClientPacketIds,   // 146个客户端ID
    ServerPacketIds,   // 273个服务器ID
    // ... 其他56个枚举
};

// 数据结构
use mir2_shared::data::{
    UserItem,          // 物品数据(37字段)
    ClientQuestInfo,   // 任务信息(20字段)
    ClientMagic,       // 魔法数据
    ClientFriend,      // 好友数据
    // ... 更多数据结构
};

// 工具
use mir2_shared::binary::{
    write_dotnet_string,  // .NET字符串序列化
    read_dotnet_string,   // .NET字符串反序列化
};
```

---

## 🎯 预期成果

完成后ClientRust将能够:
1. ✅ 连接到C#服务器
2. ✅ 发送/接收所有数据包(419个)
3. ✅ 正确解析二进制协议
4. ✅ 处理游戏逻辑
5. ✅ 编译通过,零运行时错误

---

## 📝 下一步行动

**立即开始**: 修复导入错误  
**使用工具**: 全局搜索替换  
**验证方式**: `cargo check`  
**预计完成**: 今天内完成Step 1-2

**准备好开始了吗?** 🚀
