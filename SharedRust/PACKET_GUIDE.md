# 📘 数据包使用指南

> SharedRust 网络数据包完整使用手册

## 目录

- [1. 数据包基础](#1-数据包基础)
- [2. 客户端数据包](#2-客户端数据包)
- [3. 服务器数据包](#3-服务器数据包)
- [4. 序列化与反序列化](#4-序列化与反序列化)
- [5. 常见使用场景](#5-常见使用场景)
- [6. 最佳实践](#6-最佳实践)

---

## 1. 数据包基础

### 1.1 Packet Trait

所有数据包都实现了 `Packet` trait：

```rust
pub trait Packet: Sized {
    /// 数据包操作码，用于识别数据包类型
    const OPCODE: i16;
    
    /// 从字节流读取数据包内容
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self>;
    
    /// 将数据包内容写入字节流
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()>;
    
    /// 数据包是否需要压缩（默认为false）
    fn is_compressed() -> bool {
        false
    }
}
```

### 1.2 数据包头部结构

```rust
pub struct PacketHeader {
    pub length: u16,    // 包括头部的总长度
    pub opcode: i16,    // 数据包操作码
}

// 头部大小：4字节
// length(2) + opcode(2)
```

### 1.3 数据包格式

```
+--------+--------+------------------+
| Length | Opcode |   Packet Body    |
| (u16)  | (i16)  |   (variable)     |
+--------+--------+------------------+
  2 bytes 2 bytes    n bytes
```

---

## 2. 客户端数据包

客户端数据包从客户端发送到服务器。

### 2.1 连接管理 (connection.rs)

#### ClientVersion - 版本验证

```rust
use mir2_shared::packets::client::ClientVersion;

let packet = ClientVersion {
    version_hash: vec![0x01, 0x02, 0x03, 0x04],
};

// OPCODE: 0
```

#### KeepAlive - 心跳包

```rust
use mir2_shared::packets::client::KeepAlive;

let packet = KeepAlive {
    time: std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64,
};

// OPCODE: 2
```

### 2.2 账户管理 (account.rs)

#### NewAccount - 注册账户

```rust
use mir2_shared::packets::client::NewAccount;

let packet = NewAccount {
    account_id: "username".to_string(),
    password: "hashed_password".to_string(),
    birth_date: chrono::NaiveDate::from_ymd(1990, 1, 1),
    user_name: "Display Name".to_string(),
    secret_question: "Question?".to_string(),
    secret_answer: "Answer".to_string(),
    email_address: "user@example.com".to_string(),
};

// OPCODE: 3
```

#### Login - 登录

```rust
use mir2_shared::packets::client::Login;

let packet = Login {
    account_id: "username".to_string(),
    password: "hashed_password".to_string(),
};

// OPCODE: 5
```

#### StartGame - 进入游戏

```rust
use mir2_shared::packets::client::StartGame;

let packet = StartGame {
    character_index: 0, // 选择第一个角色
};

// OPCODE: 8
```

### 2.3 角色管理 (character.rs)

#### NewCharacter - 创建角色

```rust
use mir2_shared::packets::client::NewCharacter;
use mir2_shared::enums::{MirClass, MirGender};

let packet = NewCharacter {
    name: "MyWarrior".to_string(),
    class: MirClass::Warrior,
    gender: MirGender::Male,
};

// OPCODE: 6
```

#### DeleteCharacter - 删除角色

```rust
use mir2_shared::packets::client::DeleteCharacter;

let packet = DeleteCharacter {
    character_index: 1,
};

// OPCODE: 7
```

### 2.4 移动操作 (movement.rs)

#### Turn - 转向

```rust
use mir2_shared::packets::client::Turn;
use mir2_shared::enums::MirDirection;

let packet = Turn {
    direction: MirDirection::Up,
};

// OPCODE: 10
```

#### Walk - 行走

```rust
use mir2_shared::packets::client::Walk;
use mir2_shared::enums::MirDirection;

let packet = Walk {
    direction: MirDirection::Right,
};

// OPCODE: 11
```

#### Run - 奔跑

```rust
use mir2_shared::packets::client::Run;
use mir2_shared::enums::MirDirection;

let packet = Run {
    direction: MirDirection::Down,
};

// OPCODE: 12
```

### 2.5 战斗操作 (combat.rs)

#### Attack - 攻击

```rust
use mir2_shared::packets::client::Attack;
use mir2_shared::enums::MirDirection;

let packet = Attack {
    direction: MirDirection::Up,
    spell: Spell::None,
};

// OPCODE: 44
```

#### Magic - 施法

```rust
use mir2_shared::packets::client::Magic;
use mir2_shared::enums::Spell;
use mir2_shared::map::Point;

let packet = Magic {
    spell: Spell::FireBall,
    direction: MirDirection::Up,
    target_id: Some(12345), // 目标ID（可选）
    location: Point::new(100, 100),
};

// OPCODE: 55
```

### 2.6 物品操作 (item.rs)

#### MoveItem - 移动物品

```rust
use mir2_shared::packets::client::MoveItem;
use mir2_shared::enums::MirGridType;

let packet = MoveItem {
    grid: MirGridType::Inventory,
    from: 0,  // 从背包第一格
    to: 10,   // 移动到第11格
};

// OPCODE: 14
```

#### UseItem - 使用物品

```rust
use mir2_shared::packets::client::UseItem;

let packet = UseItem {
    unique_id: 123456789, // 物品唯一ID
};

// OPCODE: 22
```

#### DropItem - 丢弃物品

```rust
use mir2_shared::packets::client::DropItem;

let packet = DropItem {
    unique_id: 123456789,
    count: 1,
};

// OPCODE: 23
```

### 2.7 NPC交互 (npc.rs)

#### CallNPC - 对话NPC

```rust
use mir2_shared::packets::client::CallNPC;

let packet = CallNPC {
    object_id: 5001, // NPC的对象ID
    key: "[Buy]".to_string(), // 对话选项
};

// OPCODE: 47
```

#### BuyItem - 购买物品

```rust
use mir2_shared::packets::client::BuyItem;

let packet = BuyItem {
    item_index: 10,
    count: 5,
};

// OPCODE: 48
```

#### SellItem - 出售物品

```rust
use mir2_shared::packets::client::SellItem;

let packet = SellItem {
    unique_id: 123456789,
    count: 1,
};

// OPCODE: 49
```

### 2.8 社交系统

#### 好友系统 (friend.rs)

```rust
use mir2_shared::packets::client::{AddFriend, RemoveFriend};

// 添加好友
let add = AddFriend {
    name: "PlayerName".to_string(),
};

// 删除好友
let remove = RemoveFriend {
    name: "PlayerName".to_string(),
};

// OPCODE: 124, 125
```

#### 组队系统 (group.rs)

```rust
use mir2_shared::packets::client::{AddMember, GroupInvite};

// 添加队员
let add = AddMember {
    name: "PlayerName".to_string(),
};

// 邀请入队
let invite = GroupInvite {
    accepted: true,
};

// OPCODE: 57, 59
```

### 2.9 邮件系统 (mail.rs)

#### SendMail - 发送邮件

```rust
use mir2_shared::packets::client::SendMail;

let packet = SendMail {
    recipient: "PlayerName".to_string(),
    subject: "Hello".to_string(),
    message: "This is a test mail.".to_string(),
    gold: 1000,
    items: vec![], // 附件物品
};

// OPCODE: 114
```

---

## 3. 服务器数据包

服务器数据包从服务器发送到客户端。

### 3.1 登录流程 (login.rs)

#### LoginSuccess - 登录成功

```rust
use mir2_shared::packets::server::LoginSuccess;

// 服务器端发送
let packet = LoginSuccess {
    characters: vec![
        SelectInfo {
            index: 0,
            name: "Warrior1".to_string(),
            level: 50,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            // ... 其他字段
        },
    ],
};

// OPCODE: 6
```

### 3.2 对象管理 (objects.rs)

#### ObjectPlayer - 玩家对象

```rust
use mir2_shared::packets::server::ObjectPlayer;

let packet = ObjectPlayer {
    object_id: 10001,
    name: "Player1".to_string(),
    guild_name: Some("MyGuild".to_string()),
    guild_rank: Some("Leader".to_string()),
    location: Point::new(100, 100),
    direction: MirDirection::Up,
    // ... 其他字段
};

// OPCODE: 26
```

#### ObjectMonster - 怪物对象

```rust
use mir2_shared::packets::server::ObjectMonster;

let packet = ObjectMonster {
    object_id: 20001,
    name: "Deer".to_string(),
    location: Point::new(105, 100),
    direction: MirDirection::Down,
    // ... 其他字段
};

// OPCODE: 27
```

### 3.3 经验系统 (experience.rs)

#### GainExperience - 获得经验

```rust
use mir2_shared::packets::server::GainExperience;

let packet = GainExperience {
    amount: 1000,
};

// OPCODE: 12
```

#### LevelChanged - 等级提升

```rust
use mir2_shared::packets::server::LevelChanged;

let packet = LevelChanged {
    level: 51,
    experience: 0,
    max_experience: 5000000,
};

// OPCODE: 14
```

### 3.4 物品系统

#### ObjectItem - 地面物品

```rust
use mir2_shared::packets::server::ObjectItem;

let packet = ObjectItem {
    object_id: 30001,
    name: "Gold".to_string(),
    location: Point::new(100, 105),
    item_id: 1,
};

// OPCODE: 18
```

#### GainedItem - 获得物品

```rust
use mir2_shared::packets::server::GainedItem;

let packet = GainedItem {
    item: UserItem {
        unique_id: 123456789,
        item_id: 100,
        current_dura: 1000,
        max_dura: 1000,
        // ... 其他字段
    },
};

// OPCODE: 20
```

---

## 4. 序列化与反序列化

### 4.1 序列化数据包

```rust
use mir2_shared::packets::base::serialize_packet;
use mir2_shared::packets::client::Login;

fn send_login(username: &str, password: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let packet = Login {
        account_id: username.to_string(),
        password: password.to_string(),
    };
    
    let mut buffer = Vec::new();
    serialize_packet(&mut buffer, &packet)?;
    
    Ok(buffer)
}
```

### 4.2 反序列化数据包

```rust
use mir2_shared::packets::base::deserialize_packet;
use mir2_shared::packets::server::LoginSuccess;
use std::io::Cursor;

fn handle_login_response(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut cursor = Cursor::new(data);
    let packet: LoginSuccess = deserialize_packet(&mut cursor)?;
    
    println!("登录成功！角色数量: {}", packet.characters.len());
    for character in &packet.characters {
        println!("- {} (Lv.{})", character.name, character.level);
    }
    
    Ok(())
}
```

### 4.3 从缓冲区提取数据包

```rust
use mir2_shared::packets::base::extract_packet;
use mir2_shared::packets::server::UserInformation;

fn process_buffer(buffer: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match extract_packet::<UserInformation>(buffer)? {
            Some((packet, remaining)) => {
                // 处理数据包
                println!("收到用户信息: {}", packet.name);
                
                // 更新缓冲区
                *buffer = remaining;
            }
            None => {
                // 没有完整数据包，等待更多数据
                break;
            }
        }
    }
    
    Ok(())
}
```

---

## 5. 常见使用场景

### 5.1 客户端登录流程

```rust
use mir2_shared::packets::client::{ClientVersion, Login, StartGame};
use mir2_shared::packets::server::{Connected, LoginSuccess, StartGameSuccess};

// 1. 发送版本信息
let version = ClientVersion {
    version_hash: vec![/* ... */],
};
send_packet(&version)?;

// 2. 接收连接确认
let connected: Connected = receive_packet()?;

// 3. 发送登录请求
let login = Login {
    account_id: "username".to_string(),
    password: "hashed_password".to_string(),
};
send_packet(&login)?;

// 4. 接收登录响应
let login_success: LoginSuccess = receive_packet()?;

// 5. 选择角色进入游戏
let start_game = StartGame {
    character_index: 0,
};
send_packet(&start_game)?;

// 6. 接收游戏启动确认
let start_success: StartGameSuccess = receive_packet()?;
```

### 5.2 移动和战斗

```rust
use mir2_shared::packets::client::{Walk, Attack};
use mir2_shared::packets::server::{UserLocation, ObjectAttack};

// 移动角色
let walk = Walk {
    direction: MirDirection::Up,
};
send_packet(&walk)?;

// 接收位置更新
let location: UserLocation = receive_packet()?;

// 攻击目标
let attack = Attack {
    direction: MirDirection::Up,
    spell: Spell::None,
};
send_packet(&attack)?;

// 接收攻击确认
let attack_result: ObjectAttack = receive_packet()?;
```

### 5.3 物品管理

```rust
use mir2_shared::packets::client::{UseItem, DropItem, MoveItem};
use mir2_shared::packets::server::{DeleteItem, GainedItem};

// 使用药水
let use_potion = UseItem {
    unique_id: potion_id,
};
send_packet(&use_potion)?;

// 移动物品
let move_item = MoveItem {
    grid: MirGridType::Inventory,
    from: 0,
    to: 10,
};
send_packet(&move_item)?;

// 丢弃物品
let drop = DropItem {
    unique_id: item_id,
    count: 1,
};
send_packet(&drop)?;
```

---

## 6. 最佳实践

### 6.1 错误处理

```rust
use mir2_shared::data::stats::{SharedError, SharedResult};

fn handle_packet(data: &[u8]) -> SharedResult<()> {
    match deserialize_packet::<Login>(data) {
        Ok(packet) => {
            // 处理数据包
            Ok(())
        }
        Err(SharedError::OpcodeMismatch { expected, actual }) => {
            eprintln!("数据包类型不匹配: 期望 {}, 实际 {}", expected, actual);
            Err(SharedError::OpcodeMismatch { expected, actual })
        }
        Err(e) => {
            eprintln!("反序列化失败: {:?}", e);
            Err(e)
        }
    }
}
```

### 6.2 批量处理数据包

```rust
fn process_network_buffer(buffer: &mut Vec<u8>) -> Result<usize, Box<dyn std::error::Error>> {
    let mut processed = 0;
    
    loop {
        // 检查是否有完整的头部
        if buffer.len() < 4 {
            break;
        }
        
        // 读取数据包长度
        let length = u16::from_le_bytes([buffer[0], buffer[1]]) as usize;
        
        // 检查是否有完整的数据包
        if buffer.len() < length {
            break;
        }
        
        // 提取数据包数据
        let packet_data = buffer[..length].to_vec();
        
        // 移除已处理的数据
        buffer.drain(..length);
        
        // 处理数据包
        handle_packet(&packet_data)?;
        
        processed += 1;
    }
    
    Ok(processed)
}
```

### 6.3 数据包缓存

```rust
use std::collections::HashMap;

struct PacketCache {
    cache: HashMap<i16, Vec<u8>>,
}

impl PacketCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
    
    fn cache_packet<P: Packet>(&mut self, packet: &P) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = Vec::new();
        serialize_packet(&mut buffer, packet)?;
        self.cache.insert(P::OPCODE, buffer);
        Ok(())
    }
    
    fn get_cached(&self, opcode: i16) -> Option<&[u8]> {
        self.cache.get(&opcode).map(|v| v.as_slice())
    }
}
```

### 6.4 异步网络处理

```rust
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn send_packet_async<P: Packet>(
    stream: &mut TcpStream,
    packet: &P,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    serialize_packet(&mut buffer, packet)?;
    
    stream.write_all(&buffer).await?;
    Ok(())
}

async fn receive_packet_async<P: Packet>(
    stream: &mut TcpStream,
) -> Result<P, Box<dyn std::error::Error>> {
    // 读取头部
    let mut header_buf = [0u8; 4];
    stream.read_exact(&mut header_buf).await?;
    
    let length = u16::from_le_bytes([header_buf[0], header_buf[1]]) as usize;
    
    // 读取完整数据包
    let mut packet_buf = vec![0u8; length];
    packet_buf[..4].copy_from_slice(&header_buf);
    stream.read_exact(&mut packet_buf[4..]).await?;
    
    // 反序列化
    let mut cursor = std::io::Cursor::new(packet_buf);
    Ok(deserialize_packet(&mut cursor)?)
}
```

---

## 附录：完整数据包列表

### 客户端数据包 (146个)

详见 `ClientPacketIds` 枚举定义 (ID: 0-145)

### 服务器数据包 (232+个)

详见 `ServerPacketIds` 枚举定义 (ID: 0-275)

---

**更新日期**: 2025年10月3日  
**版本**: 1.0.0
