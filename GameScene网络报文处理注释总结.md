# GameScene 网络报文处理注释总结

## 文档说明
本文档总结了 `GameScene.ProcessPacket()` 方法及相关报文处理函数的注释工作。

## 已完成的报文处理方法

### 1. 连接和地图相关 (6个方法)

| 方法名 | 功能说明 | 关键数据 |
|--------|---------|---------|
| `KeepAlive()` | 心跳包处理，计算Ping值 | CMain.PingTime = 当前时间 - 发送时间 |
| `MapInformation()` | 加载新地图 | 地图文件、天气、光照、音乐 |
| `WorldMapSetup()` | 初始化大地图传送系统 | 传送费用、地图布局 |
| `NewMapInfo()` | 添加新地图到列表 | 地图索引、传送点、NPC |
| `CreateBigMapButtons()` | 创建地图按钮 | 传送点按钮、NPC按钮 |
| `SearchMapResult()` | 显示地图搜索结果 | 目标地图、目标NPC |

### 2. 玩家信息相关 (3个方法)

| 方法名 | 功能说明 | 关键数据 |
|--------|---------|---------|
| `UserInformation()` | 初始化玩家角色 | 装备、背包、技能栏、金币 |
| `UserSlotsRefresh()` | 刷新装备槽位 | 动态调整装备栏大小 |
| `UserLocation()` | 强制设置玩家位置 | 传送、防外挂位置纠正 |

### 3. 对象管理相关 (6个方法)

| 方法名 | 功能说明 | 关键数据 |
|--------|---------|---------|
| `ObjectPlayer()` | 创建其他玩家对象 | 外观、装备、位置 |
| `ObjectHero()` | 创建英雄对象 | 英雄数据、HeroObject引用 |
| `ObjectRemove()` | 移除对象 | 离开视野/传送/死亡 |
| `ObjectTurn()` | 对象转向 | 方向、位置 |
| `ObjectWalk()` | 对象行走 | 行走动作队列 |
| `ObjectRun()` | 对象奔跑 | 奔跑动作队列 |

### 4. 聊天相关 (2个方法)

| 方法名 | 功能说明 | 关键数据 |
|--------|---------|---------|
| `ReceiveChat()` | 接收聊天消息 | 消息文本、消息类型 |
| `ObjectChat()` | 对象说话 | 聊天窗口 + 头顶气泡 |

### 5. 物品操作相关 (4个方法)

| 方法名 | 功能说明 | 关键逻辑 |
|--------|---------|---------|
| `MoveItem()` | 同容器内移动/交换物品 | 背包、仓库、交易栏、精炼栏、英雄背包 |
| `EquipItem()` | 装备穿戴/卸下 | 玩家装备、英雄装备、耐久度更新 |
| `EquipSlotItem()` | 特殊槽位装备 | 宝石镶嵌槽、坐骑槽、钓鱼槽 |
| `CombineItem()` | 物品合成 | 消耗材料、可能销毁目标(强化失败) |

## ProcessPacket() 方法结构

```csharp
public override void ProcessPacket(Packet p)
{
    switch (p.Index)
    {
        // ==================== 连接和地图 ====================
        case ServerPacketIds.KeepAlive:              // 心跳包
        case ServerPacketIds.MapInformation:         // 地图信息
        case ServerPacketIds.NewMapInfo:             // 新地图
        case ServerPacketIds.WorldMapSetup:          // 世界地图设置
        case ServerPacketIds.SearchMapResult:        // 搜索结果

        // ==================== 玩家信息 ====================
        case ServerPacketIds.UserInformation:        // 用户信息
        case ServerPacketIds.UserSlotsRefresh:       // 槽位刷新
        case ServerPacketIds.UserLocation:           // 位置强制设置

        // ==================== 对象管理 ====================
        case ServerPacketIds.ObjectPlayer:           // 玩家对象
        case ServerPacketIds.ObjectHero:             // 英雄对象
        case ServerPacketIds.ObjectRemove:           // 移除对象
        case ServerPacketIds.ObjectTurn:             // 转向
        case ServerPacketIds.ObjectWalk:             // 行走
        case ServerPacketIds.ObjectRun:              // 奔跑

        // ==================== 聊天 ====================
        case ServerPacketIds.Chat:                   // 聊天消息
        case ServerPacketIds.ObjectChat:             // 对象说话

        // ==================== 物品操作 ====================
        case ServerPacketIds.MoveItem:               // 移动物品
        case ServerPacketIds.EquipItem:              // 装备物品
        case ServerPacketIds.EquipSlotItem:          // 槽位物品
        case ServerPacketIds.CombineItem:            // 合成物品
        case ServerPacketIds.MergeItem:              // 合并物品
        case ServerPacketIds.SplitItem:              // 拆分物品
        case ServerPacketIds.UseItem:                // 使用物品
        case ServerPacketIds.DropItem:               // 丢弃物品
        // ... 还有约200个其他报文类型
    }
}
```

## 核心设计模式

### 1. 客户端预测 + 服务器验证
```csharp
// 客户端先执行动作(预测)
if (p.ObjectID == User.ObjectID && !Observing) return; // 不处理自己的回包

// 服务器验证结果
if (!p.Success) return; // 失败则不执行
```

### 2. 锁定机制防止重复操作
```csharp
toCell.Locked = false;   // 操作完成后解锁
fromCell.Locked = false;
```

### 3. 动作队列系统
```csharp
ob.ActionFeed.Add(new QueuedAction { 
    Action = MirAction.Walking, 
    Direction = p.Direction, 
    Location = p.Location 
});
```

## 容器类型 (MirGridType)

| 类型 | 说明 | 大小 |
|------|------|------|
| Inventory | 玩家背包 | 46格(前6格是腰带) |
| Equipment | 玩家装备栏 | 14格 |
| Storage | 个人仓库 | 80格 |
| Trade | 交易栏 | 10格 |
| Refine | 精炼栏 | 16格 |
| HeroInventory | 英雄背包 | 40格 |
| HeroEquipment | 英雄装备栏 | 14格 |
| Socket | 宝石镶嵌槽 | 可变 |
| Mount | 坐骑装备槽 | 可变 |
| Fishing | 钓鱼装备槽 | 可变 |
| GuildStorage | 公会仓库 | 112格 |

## 物品操作流程

### 移动物品 (MoveItem)
```
1. 根据 Grid 类型找到 fromCell 和 toCell
2. 解锁两个格子
3. 验证服务器返回 (p.Success)
4. 交换物品
5. 刷新属性和耐久度
```

### 装备物品 (EquipItem)
```
1. 根据 Grid 确定装备栏 (玩家/英雄)
2. 通过 UniqueID 查找源格子
3. 解锁格子
4. 验证服务器返回
5. 交换物品 (穿上新装备，卸下旧装备)
6. 更新耐久度显示
7. 刷新属性 (User/Hero)
```

### 合成物品 (CombineItem)
```
1. 找到材料格子 (fromCell) 和目标格子 (toCell)
2. 解锁格子
3. 如果失败且需要销毁，清空目标格子
4. 验证服务器返回
5. 消耗材料 (减少数量或删除)
6. 刷新属性
```

## 关键时间控制

| 延迟类型 | 时长 | 用途 |
|---------|------|------|
| InputDelay | 400ms | UserLocation后的输入延迟 |
| MoveTime | 100ms | 移动间隔 |
| CMain.NextPing | 60s | 心跳包发送间隔 |

## Rust 实现建议

### 1. 报文处理架构
```rust
pub trait PacketHandler {
    fn handle_packet(&mut self, packet: ServerPacket) -> Result<()>;
}

pub enum ServerPacket {
    KeepAlive { time: u64 },
    MapInformation { 
        map_index: u32, 
        file_name: String,
        title: String,
        lights: LightSetting,
        // ...
    },
    ObjectPlayer { /* ... */ },
    // ... 其他200+报文类型
}
```

### 2. 使用模式匹配
```rust
impl GameScene {
    pub fn process_packet(&mut self, packet: ServerPacket) -> Result<()> {
        match packet {
            ServerPacket::KeepAlive { time } => {
                self.keep_alive(time);
            }
            ServerPacket::MapInformation { .. } => {
                self.map_information(/* ... */);
            }
            ServerPacket::ObjectWalk { object_id, direction, location } => {
                if object_id == self.user.object_id && !self.observing {
                    return Ok(()); // 不处理自己的回包
                }
                // 添加到动作队列
                if let Some(obj) = self.map_control.objects.get_mut(&object_id) {
                    obj.action_feed.push(QueuedAction {
                        action: MirAction::Walking,
                        direction,
                        location,
                    });
                }
            }
            // ... 其他报文
        }
        Ok(())
    }
}
```

### 3. 异步处理建议
```rust
// 使用 tokio 异步处理网络报文
use tokio::sync::mpsc;

pub struct PacketReceiver {
    rx: mpsc::Receiver<ServerPacket>,
}

impl PacketReceiver {
    pub async fn process_packets(&mut self, game_scene: &mut GameScene) {
        while let Some(packet) = self.rx.recv().await {
            if let Err(e) = game_scene.process_packet(packet) {
                eprintln!("Error processing packet: {}", e);
            }
        }
    }
}
```

### 4. 物品容器泛型设计
```rust
pub trait ItemContainer {
    fn get_cell(&self, index: usize) -> Option<&ItemCell>;
    fn get_cell_mut(&mut self, index: usize) -> Option<&mut ItemCell>;
    fn find_by_unique_id(&self, unique_id: u64) -> Option<&ItemCell>;
}

pub struct Inventory {
    cells: Vec<ItemCell>,
    belt_idx: usize,
}

impl ItemContainer for Inventory {
    // 实现统一的物品操作接口
}
```

## 待处理的报文类型 (约200个)

### 战斗相关
- ObjectAttack (对象攻击)
- Struck (受击)
- ObjectStruck (对象受击)
- Death (死亡)
- ObjectDied (对象死亡)
- Magic (魔法)
- ObjectMagic (对象魔法)

### NPC相关
- NPCResponse (NPC响应)
- NPCGoods (NPC商品)
- NPCStorage (NPC仓库)
- NPCRequestInput (NPC输入请求)

### 社交相关
- GroupInvite (组队邀请)
- GuildInvite (公会邀请)
- TradeRequest (交易请求)
- FriendUpdate (好友更新)

### 任务相关
- ChangeQuest (任务变更)
- CompleteQuest (完成任务)
- ShareQuest (分享任务)

### 邮件相关
- ReceiveMail (接收邮件)
- MailSent (邮件发送)
- ParcelCollected (包裹领取)

## 下一步工作建议

1. **继续添加注释** - 按功能模块逐步完成剩余报文处理方法
2. **创建报文分类文档** - 将200+报文按功能分组便于理解
3. **绘制交互流程图** - 关键流程如装备、交易、战斗的时序图
4. **Rust原型实现** - 先实现核心报文的Rust处理逻辑
5. **性能优化分析** - 标注高频报文和优化点

## 参考资料

- `GameScene.cs` - 主场景类 (约13000行)
- `ServerPackets.cs` - 服务器报文定义
- `ClientPackets.cs` - 客户端报文定义
- `Enums.cs` - 枚举定义 (MirGridType, MirAction等)

---
**创建时间**: 2025-10-09  
**状态**: 持续更新中 - 已完成 21/200+ 报文方法注释
