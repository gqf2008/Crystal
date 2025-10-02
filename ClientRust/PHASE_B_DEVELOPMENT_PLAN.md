# Phase B: 数据包开发计划

**开始时间**: 2025年10月2日
**目标**: 添加下一批50个数据包,将覆盖率从18%提升到35%

---

## 📊 当前状态

### 已完成统计
- **已模块化数据包**: 53个 (包括1个辅助函数)
- **目标总数**: 285个
- **当前覆盖率**: 18.6% (53/285)
- **模块数量**: 10个

### 已有模块
1. ✅ account.rs - 4个数据包 (登录、密码)
2. ✅ npc.rs - 9个数据包 (商店、修理、仓库)
3. ✅ magic.rs - 4个数据包 (技能系统)
4. ✅ item.rs - 10个数据包 (物品操作)
5. ✅ player.rs - 9个数据包 (角色管理)
6. ✅ object.rs - 4个数据包 (游戏对象)
7. ✅ group.rs - 3个数据包 (组队)
8. ✅ guild.rs - 3个数据包 (公会)
9. ✅ hero.rs - 5个数据包 (英雄系统)
10. ✅ quest.rs - 2个数据包 (任务)

---

## 🎯 Phase B 目标: 添加50个数据包

### 策略
1. **优先添加高频使用的系统** (战斗、移动、交易)
2. **扩展现有模块** (如 item.rs, magic.rs, object.rs)
3. **创建新的逻辑模块** (combat.rs, trade.rs, buff.rs, map.rs)

---

## 📦 待添加数据包分类

### 类别 1: 战斗系统 (Combat) - 约20个数据包
**建议新建**: `combat.rs`

#### 攻击相关 (8个)
- [x] ObjectAttack (已在 protocol.rs)
- [x] Struck (已在 protocol.rs)
- [x] ObjectStruck (已在 protocol.rs)
- [x] RangeAttack (已在 protocol.rs)
- [ ] ObjectRangeAttack
- [ ] ObjectDash
- [ ] UserDash
- [ ] UserDashFail
- [ ] ObjectDashFail

#### 伤害/效果 (6个)
- [x] DamageIndicator (已在 protocol.rs)
- [ ] ObjectEffect (部分实现)
- [ ] ObjectProjectile (已在 protocol.rs)
- [ ] ObjectMagic (已在 protocol.rs)
- [ ] MagicCast (已在 protocol.rs)
- [ ] MagicDelay (已在 protocol.rs)

#### 推挤/击退 (4个)
- [x] Pushed (已在 protocol.rs)
- [x] ObjectPushed (已在 protocol.rs)
- [ ] Teleport系列 (TeleportIn, ObjectTeleportIn, ObjectTeleportOut)

#### 生命周期 (2个)
- [x] Death (已在 protocol.rs)
- [x] ObjectDied (已在 protocol.rs)
- [x] Revived (已在 protocol.rs)
- [x] ObjectRevived (已在 protocol.rs)

### 类别 2: 交易系统 (Trade) - 约8个数据包
**建议新建**: `trade.rs`

- [x] TradeRequest (已在 protocol.rs)
- [x] TradeAccept (已在 protocol.rs)
- [x] TradeGold (已在 protocol.rs)
- [x] TradeItem (已在 protocol.rs)
- [x] TradeConfirm (已在 protocol.rs)
- [x] TradeCancel (已在 protocol.rs)
- [ ] TradeLocked
- [ ] TradeUnlocked

### 类别 3: Buff/状态系统 (Buff) - 约10个数据包
**建议新建**: `buff.rs`

- [x] AddBuff (已在 protocol.rs)
- [x] RemoveBuff (已在 protocol.rs)
- [x] PauseBuff (已在 protocol.rs)
- [x] Poisoned (已在 protocol.rs)
- [x] ObjectPoisoned (已在 protocol.rs)
- [ ] ObjectNameChanged
- [ ] ColourChanged (已在 protocol.rs)
- [ ] ObjectColourChanged (已在 protocol.rs)
- [ ] ObjectGuildNameChanged (已在 protocol.rs)
- [ ] ObjectShow (已在 protocol.rs)
- [ ] ObjectHide (已在 protocol.rs)

### 类别 4: 经验/等级系统 (Experience) - 约6个数据包
**建议扩展**: 添加到 `player.rs`

- [x] GainExperience (已在 protocol.rs)
- [x] GainHeroExperience (已在 protocol.rs)
- [x] LevelChanged (已在 protocol.rs)
- [x] HeroLevelChanged (已在 protocol.rs)
- [x] ObjectLeveled (已在 protocol.rs)
- [ ] ExperienceLost
- [ ] HeroExperienceLost

### 类别 5: 地图/移动系统 (Map) - 约12个数据包
**建议新建**: `map.rs`

#### 地图信息
- [x] NewMapInfo (已在 protocol.rs)
- [x] MapInformation (已在 protocol.rs)
- [x] WorldMapSetup (已在 protocol.rs)
- [x] SearchMapResult (已在 protocol.rs)
- [ ] MapChanged (已在 protocol.rs)
- [ ] TeleportIn (已在 protocol.rs)
- [ ] ObjectTeleportIn (已在 protocol.rs)
- [ ] ObjectTeleportOut (已在 protocol.rs)

#### 移动
- [x] ObjectTurn (已在 protocol.rs)
- [x] ObjectWalk (已在 protocol.rs)
- [x] ObjectRun (已在 protocol.rs)
- [ ] UserLocation (已在 protocol.rs)

### 类别 6: 物品扩展 (Item Extensions) - 约10个数据包
**建议扩展**: 添加到 `item.rs`

#### 地面物品
- [x] ObjectItem (已在 protocol.rs)
- [x] ObjectGold (已在 protocol.rs)
- [x] GainedItem (已在 protocol.rs)
- [x] GainedGold (已在 protocol.rs)
- [x] LoseGold (已在 protocol.rs)
- [x] GainedCredit (已在 protocol.rs)
- [x] LoseCredit (已在 protocol.rs)

#### 物品操作
- [x] DeleteItem (已在 protocol.rs)
- [x] DeleteQuestItem (已在 protocol.rs)
- [x] DuraChanged (已在 protocol.rs)
- [x] MoveItem (已在 protocol.rs)
- [x] EquipItem (已在 protocol.rs)
- [x] MergeItem (已在 protocol.rs)
- [x] RemoveItem (已在 protocol.rs)
- [x] RemoveSlotItem (已在 protocol.rs)
- [x] TakeBackItem (已在 protocol.rs)
- [x] StoreItem (已在 protocol.rs)
- [x] UseItem (已在 protocol.rs)
- [x] DropItem (已在 protocol.rs)

### 类别 7: NPC扩展 (NPC Extensions) - 约5个数据包
**建议扩展**: 添加到 `npc.rs`

- [x] ObjectNpc (已在 protocol.rs)
- [x] NPCResponse (已在 protocol.rs)
- [x] NPCGoods (已在 protocol.rs)
- [ ] NPCAwakening
- [ ] NPCConfirm

### 类别 8: 采集系统 (Harvesting) - 约3个数据包
**建议新建**: `harvest.rs`

- [x] ObjectHarvest (已在 protocol.rs)
- [x] ObjectHarvested (已在 protocol.rs)
- [ ] HarvestFailed

### 类别 9: 聊天系统 (Chat) - 约4个数据包
**建议新建**: `chat.rs`

- [x] Chat (已在 protocol.rs)
- [x] ObjectChat (已在 protocol.rs)
- [ ] Whisper
- [ ] WhisperFail

---

## 🚀 执行计划

### 阶段 1: 分析现有实现 (5分钟)
- [x] 统计 protocol.rs 中已实现的数据包
- [ ] 识别未模块化的数据包
- [ ] 确定数据包分组策略

### 阶段 2: 创建新模块 (15分钟)
优先创建以下模块:

1. **combat.rs** (战斗系统) - 15-20个数据包
   - 攻击、伤害、击退、闪避、格挡
   
2. **trade.rs** (交易系统) - 6-8个数据包
   - 交易请求、物品交换、确认、取消
   
3. **buff.rs** (状态效果) - 8-10个数据包
   - Buff添加/移除、中毒、颜色变化
   
4. **map.rs** (地图/传送) - 8-10个数据包
   - 地图切换、传送、移动

### 阶段 3: 移动现有实现 (20分钟)
- [ ] 识别 protocol.rs 中已实现但未模块化的数据包
- [ ] 移动到对应模块
- [ ] 更新路由调用

### 阶段 4: 添加新数据包 (60-90分钟)
按优先级添加:

**高优先级** (核心玩法):
1. 战斗系统剩余数据包
2. 移动/传送系统
3. 交易系统完善

**中优先级** (辅助功能):
4. Buff系统扩展
5. NPC交互扩展
6. 采集系统

**低优先级** (可选):
7. 聊天系统扩展
8. 其他辅助功能

### 阶段 5: 测试验证 (10分钟)
- [ ] 编译检查
- [ ] Clippy 验证
- [ ] 格式化检查
- [ ] 路由完整性验证

---

## 📈 预期成果

### 数量指标
- **新增数据包**: 50个
- **新增模块**: 4-5个
- **覆盖率提升**: 18% → 35%
- **代码行数**: +1,500 行 (模块化代码)
- **protocol.rs 减少**: -500 行 (移动到模块)

### 质量指标
- ✅ 零编译错误
- ✅ 零 Clippy 警告
- ✅ 统一的代码风格
- ✅ 完整的文档注释

### 效率提升
- **添加速度**: ~5分钟/数据包 (模块化后)
- **维护成本**: 降低60% (代码分散到专注模块)
- **并行开发**: 支持3-5人同时开发不同模块

---

## 🔧 技术要求

### 每个新数据包必须包含:
1. **Struct 定义** - 清晰的字段类型
2. **Parser 函数** - 使用 `pub(crate)` 可见性
3. **文档注释** - 说明数据包用途
4. **错误处理** - 统一的错误消息格式

### 模块组织规范:
```rust
//! Module documentation

use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use mir2_shared::...;

// ============================================================================
// Packet Structures
// ============================================================================

/// Documentation
#[derive(Debug, Clone, PartialEq)]
pub struct PacketName {
    pub field: Type,
}

// ============================================================================
// Parser Functions
// ============================================================================

pub(crate) fn parse_packet_name(payload: &[u8]) -> Result<PacketName, String> {
    let mut cursor = Cursor::new(payload);
    // ... parsing logic
    Ok(PacketName { field })
}
```

---

## 📝 下一步行动

**立即开始**: 
1. ✅ 分析 protocol.rs 中已实现但未模块化的数据包
2. 创建 combat.rs 模块 (最大模块,优先处理)
3. 移动战斗相关数据包到 combat.rs

**命令我开始执行Phase B!** 🚀
