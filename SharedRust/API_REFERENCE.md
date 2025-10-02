# 📚 API参考文档

> SharedRust 核心API完整参考

## 目录

- [1. 数据包API](#1-数据包api)
- [2. 枚举类型](#2-枚举类型)
- [3. 数据结构](#3-数据结构)
- [4. 工具函数](#4-工具函数)
- [5. 错误处理](#5-错误处理)

---

## 1. 数据包API

### 1.1 核心Trait

#### Packet

```rust
pub trait Packet: Sized {
    const OPCODE: i16;
    
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self>;
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()>;
    
    fn is_compressed() -> bool {
        false
    }
}
```

**说明**: 所有数据包必须实现此trait。

**类型参数**:
- `R: Read` - 实现了 `std::io::Read` 的类型
- `W: Write` - 实现了 `std::io::Write` 的类型

**返回值**:
- `SharedResult<T>` - 成功返回 `Ok(T)`，失败返回 `Err(SharedError)`

### 1.2 序列化函数

#### serialize_packet

```rust
pub fn serialize_packet<W: Write, P: Packet>(
    writer: &mut W,
    packet: &P,
) -> SharedResult<()>
```

**功能**: 将数据包序列化为字节流。

**参数**:
- `writer` - 写入目标
- `packet` - 要序列化的数据包

**示例**:
```rust
let login = Login { /* ... */ };
let mut buffer = Vec::new();
serialize_packet(&mut buffer, &login)?;
```

#### deserialize_packet

```rust
pub fn deserialize_packet<R: Read, P: Packet>(
    reader: &mut R
) -> SharedResult<P>
```

**功能**: 从字节流反序列化数据包。

**参数**:
- `reader` - 读取源

**返回**: 反序列化后的数据包

**示例**:
```rust
let mut cursor = Cursor::new(data);
let packet: LoginSuccess = deserialize_packet(&mut cursor)?;
```

#### extract_packet

```rust
pub fn extract_packet<P: Packet>(
    buffer: &[u8]
) -> SharedResult<Option<(P, Vec<u8>)>>
```

**功能**: 从缓冲区提取完整的数据包。

**参数**:
- `buffer` - 数据缓冲区

**返回**:
- `Some((packet, remaining))` - 成功提取，返回数据包和剩余数据
- `None` - 缓冲区中没有完整数据包

**示例**:
```rust
match extract_packet::<UserInfo>(buffer)? {
    Some((packet, remaining)) => {
        // 处理packet
        *buffer = remaining;
    }
    None => {
        // 等待更多数据
    }
}
```

### 1.3 数据包头部

#### PacketHeader

```rust
pub struct PacketHeader {
    pub length: u16,
    pub opcode: i16,
}

impl PacketHeader {
    pub const HEADER_SIZE: usize = 4;
    
    pub fn new(length: u16, opcode: i16) -> Self;
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self>;
    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()>;
}
```

**字段说明**:
- `length` - 包含头部的总长度（字节）
- `opcode` - 数据包操作码

---

## 2. 枚举类型

### 2.1 数据包ID

#### ClientPacketIds

```rust
pub enum ClientPacketIds {
    ClientVersion = 0,
    Disconnect = 1,
    KeepAlive = 2,
    NewAccount = 3,
    // ... 共146个
}
```

#### ServerPacketIds

```rust
pub enum ServerPacketIds {
    Connected = 0,
    ClientVersion = 1,
    Disconnect = 2,
    KeepAlive = 3,
    // ... 共232+个
}
```

### 2.2 游戏枚举

#### MirClass - 职业

```rust
pub enum MirClass {
    Warrior = 0,    // 战士
    Wizard = 1,     // 法师
    Taoist = 2,     // 道士
    Assassin = 3,   // 刺客
    Archer = 4,     // 弓箭手
}
```

**用法**:
```rust
let class = MirClass::Warrior;
let value: u8 = class.into(); // 转换为u8
let class = MirClass::try_from(0u8)?; // 从u8转换
```

#### MirDirection - 方向

```rust
pub enum MirDirection {
    Up = 0,
    UpRight = 1,
    Right = 2,
    DownRight = 3,
    Down = 4,
    DownLeft = 5,
    Left = 6,
    UpLeft = 7,
}
```

**方法**:
```rust
impl MirDirection {
    pub fn reverse(&self) -> Self;
    pub fn to_angle(&self) -> f32;
}
```

#### MirGender - 性别

```rust
pub enum MirGender {
    Male = 0,
    Female = 1,
}
```

#### Spell - 技能

```rust
pub enum Spell {
    None = 0,
    Fencing = 1,
    FatalSword = 2,
    // ... 共100+个技能
}
```

#### ItemType - 物品类型

```rust
pub enum ItemType {
    Nothing = 0,
    Weapon = 1,
    Armour = 2,
    Helmet = 3,
    Necklace = 4,
    Bracelet = 5,
    Ring = 6,
    Amulet = 7,
    Belt = 8,
    Boots = 9,
    Stone = 10,
    Torch = 11,
    Potion = 12,
    Ore = 13,
    Meat = 14,
    // ...
}
```

#### MirGridType - 网格类型

```rust
pub enum MirGridType {
    None = 0,
    Inventory = 1,      // 背包
    Equipment = 2,      // 装备
    Trade = 3,          // 交易
    Storage = 4,        // 仓库
    BuyBack = 5,        // 回购
    DropPanel = 6,      // 丢弃面板
    Inspect = 7,        // 检查
    TrustMerchant = 8,  // 寄售商人
    GuildStorage = 9,   // 公会仓库
    GuestTrade = 10,    // 访客交易
    Mount = 11,         // 坐骑
    Fishing = 12,       // 钓鱼
    QuestInventory = 13,// 任务背包
    // ...
}
```

#### AttackMode - 攻击模式

```rust
pub enum AttackMode {
    Peace = 0,      // 和平
    Group = 1,      // 组队
    Guild = 2,      // 公会
    EnemyGuild = 3, // 敌对公会
    RedBrown = 4,   // 红名
    All = 5,        // 全体
}
```

#### PetMode - 宠物模式

```rust
pub enum PetMode {
    Both = 0,           // 攻击和移动
    MoveOnly = 1,       // 只移动
    AttackOnly = 2,     // 只攻击
    None = 3,           // 休息
    FocusMasterTarget = 4, // 专注主人目标
}
```

---

## 3. 数据结构

### 3.1 地图坐标

#### Point

```rust
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self;
    pub fn zero() -> Self;
    pub fn distance_to(&self, other: &Point) -> f32;
    pub fn manhattan_distance(&self, other: &Point) -> i32;
}
```

**示例**:
```rust
let p1 = Point::new(100, 100);
let p2 = Point::new(105, 100);
let distance = p1.distance_to(&p2); // 5.0
let manhattan = p1.manhattan_distance(&p2); // 5
```

### 3.2 物品系统

#### UserItem

```rust
pub struct UserItem {
    pub unique_id: u64,         // 唯一ID
    pub item_id: i32,            // 物品ID
    pub current_dura: u16,       // 当前耐久
    pub max_dura: u16,           // 最大耐久
    pub count: u32,              // 数量
    pub ac: u8,                  // 防御
    pub mac: u8,                 // 魔防
    pub dc: u8,                  // 攻击
    pub mc: u8,                  // 魔法
    pub sc: u8,                  // 道术
    pub accuracy: u8,            // 准确
    pub agility: u8,             // 敏捷
    pub hp: u8,                  // 生命
    pub mp: u8,                  // 魔法值
    pub attack_speed: i8,        // 攻速
    pub luck: i8,                // 幸运
    pub slots_count: u8,         // 孔数
    pub refine: u8,              // 精炼等级
    // ...
}

impl UserItem {
    pub fn read_from<R: Read>(
        reader: &mut R,
        version: i32,
        file_version: i32,
    ) -> SharedResult<Self>;
    
    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()>;
}
```

### 3.3 角色信息

#### SelectInfo

```rust
pub struct SelectInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub last_access: u64,
}
```

### 3.4 属性系统

#### Stats

```rust
pub struct Stats {
    pub min_ac: u8,
    pub max_ac: u8,
    pub min_mac: u8,
    pub max_mac: u8,
    pub min_dc: u8,
    pub max_dc: u8,
    pub min_mc: u8,
    pub max_mc: u8,
    pub min_sc: u8,
    pub max_sc: u8,
    pub accuracy: u8,
    pub agility: u8,
    pub hp: u16,
    pub mp: u16,
    // ...
}
```

### 3.5 对象信息

#### ObjectInfo

```rust
pub struct ObjectInfo {
    pub object_id: u32,
    pub name: String,
    pub name_colour: Color,
    pub location: Point,
    pub direction: MirDirection,
    pub dead: bool,
    pub skeleton: bool,
    pub poison: PoisonType,
    pub hidden: bool,
    pub effect: SpellEffect,
    pub weapon: i16,
    pub armour: i16,
    pub light: u8,
}
```

---

## 4. 工具函数

### 4.1 方向计算

#### direction_from_point

```rust
pub fn direction_from_point(source: Point, destination: Point) -> MirDirection
```

**功能**: 计算从源点到目标点的方向。

**示例**:
```rust
let dir = direction_from_point(
    Point::new(100, 100),
    Point::new(105, 100)
); // MirDirection::Right
```

#### point_move

```rust
pub fn point_move(point: Point, direction: MirDirection, distance: i32) -> Point
```

**功能**: 沿指定方向移动点。

**示例**:
```rust
let new_pos = point_move(
    Point::new(100, 100),
    MirDirection::Up,
    5
); // Point(100, 95)
```

### 4.2 距离计算

#### functions_in_range

```rust
pub fn functions_in_range(
    location: Point,
    destination: Point,
    range: i32
) -> bool
```

**功能**: 检查两点是否在指定范围内。

**示例**:
```rust
let in_range = functions_in_range(
    Point::new(100, 100),
    Point::new(105, 100),
    10
); // true
```

---

## 5. 错误处理

### 5.1 错误类型

#### SharedError

```rust
pub enum SharedError {
    IoError(std::io::Error),
    InvalidPacketLength(u16),
    OpcodeMismatch {
        expected: i16,
        actual: i16,
    },
    PacketTooLarge(usize),
    InvalidEnumValue(u8),
    StringConversionError,
    CompressionError(String),
    DecompressionError(String),
}
```

### 5.2 Result类型

```rust
pub type SharedResult<T> = Result<T, SharedError>;
```

**用法**:
```rust
fn process_packet() -> SharedResult<()> {
    let packet = deserialize_packet(reader)?;
    // 处理packet
    Ok(())
}
```

### 5.3 错误转换

```rust
impl From<std::io::Error> for SharedError {
    fn from(err: std::io::Error) -> Self {
        SharedError::IoError(err)
    }
}

impl From<std::string::FromUtf8Error> for SharedError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        SharedError::StringConversionError
    }
}
```

---

## 6. 常量定义

### 6.1 游戏常量

```rust
pub const MAX_LEVEL: u16 = 400;
pub const MAX_HP: u16 = 65535;
pub const MAX_MP: u16 = 65535;

pub const MAX_INVENTORY: usize = 46;
pub const MAX_EQUIPMENT: usize = 14;
pub const MAX_STORAGE: usize = 80;

pub const MAX_GROUP_MEMBERS: usize = 15;
pub const MAX_GUILD_MEMBERS: usize = 500;

pub const MOVEMENT_SPEED: u32 = 100;
pub const ATTACK_SPEED_BASE: u32 = 1000;
```

### 6.2 网络常量

```rust
pub const PACKET_HEADER_SIZE: usize = 4;
pub const MAX_PACKET_SIZE: usize = 65535;
pub const COMPRESSION_THRESHOLD: usize = 1024;
```

---

## 7. Traits和接口

### 7.1 序列化Trait

```rust
pub trait BinarySerialize {
    fn serialize<W: Write>(&self, writer: &mut W) -> SharedResult<()>;
    fn deserialize<R: Read>(reader: &mut R) -> SharedResult<Self>
    where
        Self: Sized;
}
```

**实现示例**:
```rust
impl BinarySerialize for Point {
    fn serialize<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.x)?;
        writer.write_i32::<LittleEndian>(self.y)?;
        Ok(())
    }
    
    fn deserialize<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        })
    }
}
```

---

## 8. 宏

### 8.1 数据包定义宏

```rust
#[macro_export]
macro_rules! define_packet {
    ($name:ident, $opcode:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field: $ty,)*
        }
        
        impl Packet for $name {
            const OPCODE: i16 = $opcode;
            
            fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
                Ok(Self {
                    $($field: <$ty>::deserialize(reader)?,)*
                })
            }
            
            fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
                $(self.$field.serialize(writer)?;)*
                Ok(())
            }
        }
    };
}
```

**使用示例**:
```rust
define_packet!(MyPacket, 999, {
    value: i32,
    name: String,
});
```

---

## 9. 性能优化

### 9.1 零拷贝读取

```rust
pub fn read_bytes_zero_copy(reader: &mut &[u8], len: usize) -> SharedResult<&[u8]> {
    if reader.len() < len {
        return Err(SharedError::InvalidPacketLength(len as u16));
    }
    let result = &reader[..len];
    *reader = &reader[len..];
    Ok(result)
}
```

### 9.2 批量序列化

```rust
pub fn serialize_packets<W: Write, P: Packet>(
    writer: &mut W,
    packets: &[P],
) -> SharedResult<()> {
    for packet in packets {
        serialize_packet(writer, packet)?;
    }
    Ok(())
}
```

---

## 10. 调试工具

### 10.1 数据包dump

```rust
pub fn dump_packet_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
```

### 10.2 数据包验证

```rust
pub fn validate_packet<P: Packet>(data: &[u8]) -> SharedResult<bool> {
    if data.len() < PACKET_HEADER_SIZE {
        return Ok(false);
    }
    
    let length = u16::from_le_bytes([data[0], data[1]]) as usize;
    let opcode = i16::from_le_bytes([data[2], data[3]]);
    
    Ok(data.len() >= length && opcode == P::OPCODE)
}
```

---

**文档版本**: 1.0.0  
**最后更新**: 2025年10月3日  
**Rust版本要求**: 1.70+
