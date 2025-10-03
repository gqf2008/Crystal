# ClientRust 集成 SharedRust 快速指南

## 🚀 5分钟快速开始

### 第1步: 添加依赖

编辑 `ClientRust/Cargo.toml`:

```toml
[dependencies]
shared_rust = { path = "../SharedRust" }
byteorder = "1.5"
```

### 第2步: 导入模块

在你的Rust文件中:

```rust
// 导入所有常用类型
use shared_rust::prelude::*;

// 或者选择性导入
use shared_rust::enums::{MirDirection, MirClass, Spell};
use shared_rust::packets::client::movement::{Walk, Run};
use shared_rust::packets::server::connection::Connected;
use shared_rust::data::{UserItem, ClientQuestInfo};
```

### 第3步: 发送数据包到服务器

```rust
use shared_rust::packets::client::movement::Walk;
use shared_rust::enums::MirDirection;
use std::io::Cursor;

// 创建数据包
let packet = Walk {
    direction: MirDirection::Up,
};

// 序列化为字节
let mut buffer = Vec::new();
packet.write_to(&mut buffer).unwrap();

// 发送到服务器
send_to_server(&buffer);
```

### 第4步: 接收服务器数据包

```rust
use shared_rust::packets::server::connection::Connected;

// 从网络接收数据
let data: Vec<u8> = receive_from_server()?;

// 反序列化
let mut cursor = Cursor::new(data);
let packet = Connected::read_from(&mut cursor)?;

// 使用数据
println!("Connected! Session: {}", packet.session_id);
```

---

## 📦 常用功能示例

### 1. 角色登录流程

```rust
use shared_rust::packets::client::account::{Login, StartGame};
use shared_rust::packets::server::account::{LoginSuccess, StartGameSuccess};

// 1. 登录
let login = Login {
    account_id: "player123".to_string(),
    password: "password123".to_string(),
};
send_packet(&login)?;

// 2. 接收登录响应
let response = receive_packet::<LoginSuccess>()?;
println!("Login success! Characters: {:?}", response.characters);

// 3. 选择角色开始游戏
let start = StartGame {
    character_index: 0,
};
send_packet(&start)?;
```

### 2. 角色移动

```rust
use shared_rust::packets::client::movement::{Walk, Run};
use shared_rust::enums::MirDirection;

// 行走
let walk = Walk {
    direction: MirDirection::Right,
};
send_packet(&walk)?;

// 奔跑
let run = Run {
    direction: MirDirection::Up,
};
send_packet(&run)?;
```

### 3. 物品操作

```rust
use shared_rust::packets::client::item::{MoveItem, UseItem, DropItem};
use shared_rust::enums::MirGridType;

// 移动物品
let move_item = MoveItem {
    grid: MirGridType::Inventory,
    from: 0,
    to: 5,
};
send_packet(&move_item)?;

// 使用物品
let use_item = UseItem {
    unique_id: 12345,
    grid: MirGridType::Inventory,
};
send_packet(&use_item)?;

// 丢弃物品
let drop = DropItem {
    unique_id: 12345,
    count: 1,
    hero_inventory: false,
};
send_packet(&drop)?;
```

### 4. 战斗系统

```rust
use shared_rust::packets::client::combat::{Attack, Magic};
use shared_rust::enums::{MirDirection, Spell};

// 普通攻击
let attack = Attack {
    direction: MirDirection::Right,
    spell: Spell::None,
};
send_packet(&attack)?;

// 释放技能
let magic = Magic {
    object_id: 0,
    spell: Spell::Fireball,
    direction: MirDirection::Up,
    target_id: 123,
    location: (100, 200),
    spell_target_lock: false,
};
send_packet(&magic)?;
```

### 5. NPC交互

```rust
use shared_rust::packets::client::npc::{CallNPC, BuyItem};
use shared_rust::enums::PanelType;

// 呼叫NPC
let call = CallNPC {
    object_id: 456,
    key: "[BUY]".to_string(),
};
send_packet(&call)?;

// 购买物品
let buy = BuyItem {
    item_index: 100,
    count: 1,
    panel_type: PanelType::Buy,
};
send_packet(&buy)?;
```

### 6. 聊天系统

```rust
use shared_rust::packets::client::chat::Chat;
use shared_rust::data::ClientChatItem;

// 发送聊天消息
let chat = Chat {
    message: "Hello World!".to_string(),
    linked_items: vec![],
};
send_packet(&chat)?;

// 带物品链接的聊天
let chat_with_item = Chat {
    message: "Look at my sword: {0}".to_string(),
    linked_items: vec![ClientChatItem {
        item_unique_id: 12345,
        slot: 0,
        grid: MirGridType::Inventory,
    }],
};
send_packet(&chat_with_item)?;
```

### 7. 处理UserItem

```rust
use shared_rust::data::UserItem;

// 创建物品
let item = UserItem {
    unique_id: 12345,
    item_index: 100,
    current_dura: 1000,
    max_dura: 1000,
    count: 1,
    ac: 10,
    mac: 5,
    dc: 15,
    mc: 8,
    ..Default::default()
};

// 序列化
let mut buffer = Vec::new();
item.write_to(&mut buffer)?;

// 反序列化
let mut cursor = Cursor::new(buffer);
let loaded_item = UserItem::read_from(&mut cursor)?;
```

### 8. 任务系统

```rust
use shared_rust::packets::client::quest::{AcceptQuest, FinishQuest};

// 接受任务
let accept = AcceptQuest {
    npc_index: 123,
    quest_index: 5,
};
send_packet(&accept)?;

// 完成任务
let finish = FinishQuest {
    quest_index: 5,
    selected_item_index: 0,
};
send_packet(&finish)?;
```

---

## 🛠️ 辅助函数

### 通用数据包发送函数

```rust
use shared_rust::packets::base::Packet;
use std::io::Write;

fn send_packet<P: Packet>(packet: &P) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    
    // 写入数据包头
    buffer.write_i16::<LittleEndian>(packet.opcode())?;
    
    // 写入数据包体
    packet.write_body(&mut buffer)?;
    
    // 发送到网络
    send_to_server(&buffer)?;
    
    Ok(())
}
```

### 通用数据包接收函数

```rust
use shared_rust::packets::base::Packet;
use std::io::Read;

fn receive_packet<P: Packet>() -> Result<P, Box<dyn std::error::Error>> {
    // 从网络接收数据
    let data = receive_from_server()?;
    
    // 跳过前2字节(opcode)
    let mut cursor = Cursor::new(&data[2..]);
    
    // 反序列化数据包
    let packet = P::read_body(&mut cursor)?;
    
    Ok(packet)
}
```

---

## 🔍 错误处理

### 推荐模式

```rust
use shared_rust::data::stats::{SharedResult, SharedError};

fn process_packet(data: &[u8]) -> SharedResult<()> {
    let packet = SomePacket::read_from(&mut Cursor::new(data))?;
    
    // 处理数据包...
    
    Ok(())
}

// 调用
match process_packet(&data) {
    Ok(()) => println!("Success!"),
    Err(SharedError::IoError(e)) => eprintln!("IO Error: {}", e),
    Err(SharedError::InvalidUtf8) => eprintln!("Invalid UTF-8"),
    Err(SharedError::NegativeLength { field, length }) => {
        eprintln!("Negative length {} for field {}", length, field);
    },
    Err(e) => eprintln!("Error: {:?}", e),
}
```

---

## 📋 常用枚举值

### 方向 (MirDirection)

```rust
MirDirection::Up
MirDirection::UpRight
MirDirection::Right
MirDirection::DownRight
MirDirection::Down
MirDirection::DownLeft
MirDirection::Left
MirDirection::UpLeft
```

### 职业 (MirClass)

```rust
MirClass::Warrior
MirClass::Wizard
MirClass::Taoist
MirClass::Assassin
MirClass::Archer
MirClass::None
```

### 网格类型 (MirGridType)

```rust
MirGridType::Inventory       // 背包
MirGridType::Equipment       // 装备栏
MirGridType::Storage         // 仓库
MirGridType::BuyBack         // 回购
MirGridType::Trading         // 交易
MirGridType::Refine          // 精炼
MirGridType::GuildStorage    // 公会仓库
// ... 更多
```

### 技能 (Spell)

```rust
Spell::None
Spell::Fireball
Spell::Healing
Spell::FireBurst
Spell::Lightning
// ... 146个技能
```

---

## 💡 最佳实践

### 1. 使用类型别名简化代码

```rust
type Result<T> = shared_rust::data::stats::SharedResult<T>;
```

### 2. 创建数据包管理器

```rust
pub struct PacketManager {
    // 网络连接
}

impl PacketManager {
    pub fn send<P: Packet>(&mut self, packet: &P) -> Result<()> {
        // 统一发送逻辑
    }
    
    pub fn receive<P: Packet>(&mut self) -> Result<P> {
        // 统一接收逻辑
    }
}
```

### 3. 使用异步处理

```rust
use tokio;

async fn handle_packet(packet: SomePacket) {
    // 异步处理数据包
}
```

### 4. 批量处理数据包

```rust
fn process_packets(packets: &[Vec<u8>]) -> Result<()> {
    for data in packets {
        // 处理每个数据包
    }
    Ok(())
}
```

---

## 📚 完整文档链接

- **完整移植文档**: `../SharedRust/PORTING_DOCUMENTATION.md`
- **中文README**: `../SharedRust/README_CN.md`
- **英文README**: `../SharedRust/README.md`
- **检查清单**: `../SharedRust/MIGRATION_CHECKLIST.md`
- **完成报告**: `../SharedRust/COMPLETION_REPORT.md`

---

## 🆘 常见问题

### Q: 编译错误 "cannot find type `XXX` in module"

**A**: 确认导入路径正确:
```rust
use shared_rust::packets::client::movement::Walk;
```

### Q: 运行时错误 "InvalidUtf8"

**A**: 检查字符串编码,确保使用UTF-8

### Q: 数据包解析失败

**A**: 检查:
1. 数据包ID是否正确
2. 字节序是否为LittleEndian
3. 数据完整性

### Q: 性能优化建议

**A**: 
1. 使用`cargo build --release`编译
2. 重用buffer避免频繁分配
3. 使用对象池管理高频对象

---

## ✅ 集成检查清单

- [ ] Cargo.toml添加shared_rust依赖
- [ ] 成功编译ClientRust项目
- [ ] 测试基本数据包发送
- [ ] 测试基本数据包接收
- [ ] 实现错误处理
- [ ] 添加日志记录
- [ ] 性能测试通过

---

**🎉 开始使用SharedRust,享受Rust的高性能和类型安全!**

*如有问题,请查阅完整文档或提交Issue*
