# Hero Packet 处理说明

## 概述

英雄系统使用两个主要的数据包进行通信:

1. **ObjectHero** - 用于生成和完整初始化英雄对象
2. **HeroInformation** - 用于触发英雄相关事件(只包含 hero_id)

## ObjectHero 包

### 用途
当英雄首次出现在游戏世界中时使用,包含完整的英雄信息。

### 结构
```rust
pub struct ObjectHero {
    pub player: ObjectPlayer,  // 继承自 ObjectPlayer 的所有字段
    pub owner_name: String,    // 主人名称
}
```

### ObjectPlayer 字段 (继承)
- `object_id: u32` - 对象ID
- `name: String` - 英雄名称
- `guild_name: String` - 公会名称
- `guild_rank_name: String` - 公会等级名称
- `name_colour: i32` - 名称颜色
- `class: MirClass` - 职业
- `gender: MirGender` - 性别
- `level: u16` - 等级
- `location_x: i32` - X坐标
- `location_y: i32` - Y坐标
- `direction: MirDirection` - 方向
- `hair: u8` - 发型
- `light: u8` - 光照
- `weapon: i16` - 武器
- `weapon_effect: i16` - 武器特效
- `armour: i16` - 盔甲
- `poison: PoisonType` - 中毒状态
- `dead: bool` - 是否死亡
- `hidden: bool` - 是否隐藏
- `effect: SpellEffect` - 法术效果
- `wing_effect: u8` - 翅膀效果
- `extra: bool` - 额外标志
- `mount_type: i16` - 坐骑类型
- `riding_mount: bool` - 是否骑乘
- `fishing: bool` - 是否钓鱼
- `transform_type: i16` - 变身类型
- `element_orb_effect: u32` - 元素珠效果
- `element_orb_lvl: u32` - 元素珠等级
- `element_orb_max: u32` - 元素珠最大值
- `buffs: Vec<BuffType>` - Buff列表
- `level_effects: LevelEffects` - 等级效果

### 使用示例
```rust
fn on_object_hero(&mut self, packet: packets::ObjectHero) {
    let mut hero = HeroObject::new(packet.player.object_id);
    hero.load_from_object(&packet);
    
    // 英雄现在已经完全初始化
    self.hero = Some(hero);
}
```

## HeroInformation 包

### 用途
用于触发英雄相关的更新或事件,不包含完整的英雄数据。

### 结构
```rust
pub struct HeroInformation {
    pub hero_id: u32,  // 仅包含英雄ID
}
```

### 使用场景
- 请求更新特定英雄的信息
- 触发英雄状态检查
- 作为其他更详细数据包的前置包

### 使用示例
```rust
fn on_hero_information(&mut self, packet: packets::HeroInformation) {
    // 只包含 hero_id,用于标识要更新的英雄
    if let Some(hero) = &self.hero {
        if hero.map_object.object_id() == packet.hero_id {
            // 触发英雄信息更新
            hero.load_hero_info(&packet);
        }
    }
}
```

## HeroObject 实现

### 两个 load 方法

```rust
impl HeroObject {
    /// 从 ObjectHero 包加载完整数据 (首次生成)
    pub fn load_from_object(&mut self, info: &ObjectHero) {
        let player = &info.player;
        
        // 设置基本信息
        self.map_object.set_name(player.name.clone());
        self.map_object.set_name_colour_argb(player.name_colour);
        self.owner_name = info.owner_name.clone();
        
        // 设置位置和方向
        let location = Point::new(player.location_x, player.location_y);
        self.map_object.set_location(location);
        self.map_object.set_direction(player.direction);
        
        // 外观
        self.class = player.class;
        self.gender = player.gender;
        self.level = player.level;
        self.hair = player.hair;
        self.weapon = player.weapon as i32;
        self.weapon_effect = player.weapon_effect as i32;
        self.armour = player.armour as i32;
        
        // 状态
        self.map_object.set_light(player.light);
        self.map_object.set_poison(player.poison);
        self.map_object.set_dead(player.dead);
        self.map_object.set_hidden(player.hidden);
        self.map_object.set_buffs(player.buffs.clone());
        
        // 设置为已生成状态
        self.spawn_state = HeroState::Spawned;
    }
    
    /// 从 HeroInformation 包更新 (仅包含 hero_id)
    pub fn load_hero_info(&mut self, _info: &HeroInformation) {
        // HeroInformation 只包含 hero_id
        // 实际的英雄数据应该在 ObjectHero 或其他详细包中
        // 这个包主要用于触发英雄相关事件
    }
}
```

## 相关包

### ClientHeroInformation
用于英雄列表显示 (如选择英雄界面)

```rust
pub struct ClientHeroInformation {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
}
```

### NewHeroInformation
包含多个英雄的列表信息

```rust
pub struct NewHeroInformation {
    pub heroes: Vec<ClientHeroInformation>,
}
```

## 数据包流程

### 英雄生成流程
1. 服务器发送 `ObjectHero` - 包含完整英雄数据
2. 客户端创建 `HeroObject` 并调用 `load_from_object()`
3. 英雄出现在游戏世界中

### 英雄信息更新流程
1. 服务器发送 `HeroInformation` - 只包含 hero_id
2. 客户端识别是哪个英雄
3. 等待后续的详细数据包 (如 HeroState, HeroExperience 等)

## 注意事项

1. **ObjectHero vs HeroInformation**
   - `ObjectHero` = 完整数据,用于首次加载
   - `HeroInformation` = 仅 ID,用于触发更新

2. **继承关系**
   - `ObjectHero` 包含完整的 `ObjectPlayer`
   - 英雄继承了玩家的所有属性

3. **状态管理**
   - 使用 `HeroState` 枚举管理英雄的召唤状态
   - `ObjectHero` 设置初始状态为 `Spawned`

4. **主人关系**
   - `owner_name` 字段标识英雄的主人
   - `owner_id` 需要在其他包中获取

## 测试

```rust
#[test]
fn test_load_from_object() {
    use mir2_shared::packets::server::{ObjectHero, ObjectPlayer};
    
    let player = ObjectPlayer {
        object_id: 123,
        name: "TestHero".to_string(),
        level: 50,
        class: MirClass::Warrior,
        gender: MirGender::Male,
        // ... 其他字段
    };
    
    let packet = ObjectHero {
        player,
        owner_name: "TestPlayer".to_string(),
    };
    
    let mut hero = HeroObject::new(123);
    hero.load_from_object(&packet);
    
    assert_eq!(hero.map_object.name(), "TestHero");
    assert_eq!(hero.owner_name, "TestPlayer");
    assert_eq!(hero.spawn_state, HeroState::Spawned);
}
```

## 总结

✅ **ObjectHero** - 用于完整的英雄生成和初始化  
✅ **HeroInformation** - 用于触发英雄相关事件 (仅 ID)  
✅ **load_from_object()** - 处理 ObjectHero 包  
✅ **load_hero_info()** - 处理 HeroInformation 包  

这样的设计允许服务器灵活地发送完整数据或仅发送更新触发器,优化网络带宽。
