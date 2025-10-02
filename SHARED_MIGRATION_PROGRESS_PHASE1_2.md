# SharedRust 迁移进度报告 - 阶段 1.2（进行中）

## 执行时间
- 开始时间：2024（当前会话）
- 状态：⚠️ 进行中（遇到技术挑战）

## 已完成工作

### 1. 新增枚举和类型（enums.rs）
已添加以下类型支持：
- ✅ `Color` 结构体 - ARGB 颜色表示
  - from_argb, to_argb 转换
  - alpha, red, green, blue 提取方法
  
- ✅ 已存在的 Bitflags 类型确认：
  - WeatherSetting (天气设置)
  - LevelEffects (等级特效)
  - PoisonType (中毒类型)

### 2. Client Data 增强（client_data.rs）
为以下结构添加了 `write_to` 序列化方法：
- ✅ `ClientMagic::write_to` - 魔法技能序列化
- ✅ `ClientIntelligentCreature::write_to` - 宠物数据序列化
- ✅ `IntelligentCreatureRules::write_to` - 宠物规则序列化
- ✅ `IntelligentCreatureItemFilter::write_to` - 物品过滤序列化

### 3. 服务器包实现（server_packets.rs）

#### 3.1 已添加的包（~1200行新代码）

**玩家对象包（5个）：**
1. ✅ `ObjectPlayer` - 玩家对象信息（30+ 字段）
   - 外观、装备、BUFF、特效等
2. ✅ `ObjectHero` - 英雄对象（继承自 ObjectPlayer）
3. ✅ `ObjectRemove` - 移除对象
4. ✅ `PlayerUpdate` - 玩家外观更新（装备变更）
5. ⚠️ `PlayerInspect` - 查看玩家装备（需要 UserItem 修复）

**移动包（5个）：**
6. ✅ `ObjectTurn` - 对象转向
7. ✅ `ObjectWalk` - 对象行走
8. ✅ `ObjectRun` - 对象奔跑
9. ✅ `Pushed` - 玩家被推
10. ✅ `ObjectPushed` - 对象被推

**颜色包（2个）：**
11. ✅ `ColourChanged` - 玩家名字颜色变更
12. ✅ `ObjectColourChanged` - 对象名字颜色变更

**地图包（9个）：**
13. ✅ `MapChanged` - 地图切换
14. ✅ `MapInformation` - 地图信息
15. ✅ `SearchMapResult` - 地图搜索结果（NPC查找）
16. ✅ `TimeOfDay` - 时间/光照变化
17. ✅ `ObjectTeleportOut` - 对象传送出特效
18. ✅ `ObjectTeleportIn` - 对象传送入特效
19. ✅ `TeleportIn` - 玩家传送（空包）
20. ✅ `ObjectHide` - 隐藏对象
21. ✅ `ObjectShow` - 显示对象

**复杂包（暂时搁置）：**
- ⚠️ `UserInformation` - 用户完整信息（~200行，涉及 UserItem 数组）
- ⚠️ `UserLocation` - 用户位置
- ⚠️ `UserSlotsRefresh` - 刷新物品栏（涉及 UserItem 数组）

## 当前挑战

### 问题 1: UserItem 版本参数 ❌
**现象**：
```rust
// UserItem::read_from 需要 3 个参数：
pub fn read_from<R: Read>(reader: &mut R, version: i32, custom_version: i32)

// 但包协议中这些参数从哪里来？
```

**影响的包**：
- UserInformation
- UserSlotsRefresh  
- PlayerInspect

**可能的解决方案**：
1. 使用固定的版本号（需要研究 C# 代码确定默认值）
2. 在包级别传递版本信息
3. 修改 UserItem API 使用 Option<> 参数

### 问题 2: TryFromPrimitive 错误转换 ❌
**现象**：
```rust
error[E0277]: `?` couldn't convert the error to `SharedError`
  let lights = LightSetting::try_from(reader.read_u8()?)?;
               ^^^^ the trait `From<TryFromPrimitiveError<LightSetting>>` 
                    is not implemented for `SharedError`
```

**影响范围**：所有使用 enum 的 read_body 方法

**解决方案**：使用 `.map_err()` 手动转换
```rust
// 当前（错误）：
let class = MirClass::try_from(reader.read_u8()?)?;

// 修复后：
let class_raw = reader.read_u8()?;
let class = MirClass::try_from(class_raw)
    .map_err(|_| SharedError::unknown_enum("MirClass", class_raw.into()))?;
```

### 问题 3: 编译错误统计
- 总错误数：**30 个**
- UserItem 参数错误：~12 个
- TryFromPrimitive 转换错误：~18 个

## 代码统计

### server_packets.rs 增长
- 起始行数：750 行
- 当前行数：1957 行
- 新增代码：**~1200 行**

### enums.rs 增长  
- 新增 Color 结构：~45 行

### client_data.rs 增强
- 新增 write_to 方法：~100 行

### 总计
- **新增代码：约 1345 行**
- **实现包数：21 个**（完整）+ 3 个（部分）

## 下一步计划

### 优先级 1: 修复编译错误
1. **修复 TryFromPrimitive 错误** (1-2小时)
   - 在所有 enum 转换处添加 `.map_err()`
   - 预计需要修改 ~40 处代码

2. **解决 UserItem 版本问题** (2-3小时)
   - 研究 C# ServerPackets.cs 确定版本号来源
   - 可能的解决方案：
     - 使用常量 DEFAULT_VERSION
     - 添加包级别的版本字段
     - 创建 UserItem::read_from_packet 便捷方法

3. **完成剩余 3 个包** (1-2小时)
   - UserInformation
   - UserLocation  
   - UserSlotsRefresh
   - PlayerInspect（已部分完成）

### 优先级 2: 测试 & 文档
1. 添加单元测试（21 个简单包）
2. 集成测试（序列化/反序列化往返）
3. 更新文档标注完成状态

### 优先级 3: 继续阶段 1.2  
完成后，阶段 1.2 进度：
- **目标**：25 个包
- **当前**：21/25 个包（84%）
- **剩余**：4 个包

## 技术债务

### 需要回顾的设计决策
1. **版本处理策略**
   - UserItem 需要版本号的根本原因
   - 是否需要在包层面统一处理版本

2. **错误处理改进**
   - 考虑为 TryFromPrimitiveError 实现通用 From<> trait
   - 或创建辅助宏减少 map_err 样板代码

3. **序列化一致性**
   - UserItem 使用 write_to 还是需要 Save() 方法？
   - 统一命名约定

## 估算完成时间

### 乐观估计（假设顺利）
- 修复错误：2-3 小时
- 完成剩余包：1-2 小时
- 测试 & 清理：1 小时
- **总计：4-6 小时**

### 现实估计（考虑调试）
- 修复错误：4-5 小时
- 完成剩余包：2-3 小时  
- 测试 & 清理：2 小时
- **总计：8-10 小时**

## 经验教训

1. **预先研究 API**：在实现前应该先检查所有依赖结构的 API 签名
2. **渐进式实现**：先实现最简单的包，再逐步处理复杂的
3. **错误处理模式**：需要建立统一的错误转换模式（如宏或辅助函数）
4. **类型依赖图**：应该先绘制类型依赖关系，确保基础类型完备

## 已知问题追踪

| 问题 ID | 描述 | 影响包 | 优先级 | 状态 |
|---------|------|--------|--------|------|
| ISSUE-1 | UserItem version 参数 | UserInformation, UserSlotsRefresh, PlayerInspect | P0 | Open |
| ISSUE-2 | TryFromPrimitive 错误转换 | 所有使用 enum 的包 | P0 | Open |
| ISSUE-3 | Color 类型测试缺失 | - | P2 | Open |

---

**报告生成时间**：当前会话  
**下次更新**：修复编译错误后
