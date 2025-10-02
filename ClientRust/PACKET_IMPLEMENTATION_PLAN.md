# 数据包完整实现计划

## 总览
ServerPacketIds 共有 **285 个**数据包类型。

## 当前进度
- ✅ 已实现：**99 个** (35%)
- ⏳ 待实现：**186 个** (65%)

## 分类实现计划

### 第 1 批：NPC 交互系统 (18个)
**优先级：高** - 游戏核心功能

- [ ] NPCSell
- [ ] NPCRepair
- [ ] NPCSRepair  
- [ ] NPCRefine
- [ ] NPCCheckRefine
- [ ] NPCCollectRefine
- [ ] NPCReplaceWedRing
- [ ] NPCStorage
- [ ] NPCRequestInput
- [ ] NPCConsign
- [ ] NPCMarket
- [ ] NPCMarketPage
- [ ] NPCAwakening
- [ ] NPCDisassemble
- [ ] NPCDowngrade
- [ ] NPCReset
- [ ] NPCPearlGoods
- [ ] DefaultNPC

### 第 2 批：物品系统补充 (20个)
**优先级：高** - 背包/交易/精炼

- [ ] SplitItem
- [ ] SplitItem1
- [ ] DepositRefineItem
- [ ] RetrieveRefineItem
- [ ] RefineCancel
- [ ] RefineItem
- [ ] DepositTradeItem
- [ ] RetrieveTradeItem
- [ ] TakeBackHeroItem
- [ ] TransferHeroItem
- [ ] SellItem
- [ ] CraftItem
- [ ] RepairItem
- [ ] ItemRepaired
- [ ] ItemSlotSizeChanged
- [ ] ItemSealChanged
- [ ] RefreshItem
- [ ] CombineItem
- [ ] ItemUpgraded
- [ ] AwakeningLockedItem

### 第 3 批：魔法系统补充 (4个)
**优先级：高** - 战斗系统

- [ ] NewMagic
- [ ] RemoveMagic
- [ ] MagicLeveled
- [ ] SpellToggle

### 第 4 批：组队系统补充 (3个)
**优先级：中** - 社交功能

- [ ] SwitchGroup
- [ ] GroupMembersMap
- [ ] SendMemberLocation

### 第 5 批：公会系统补充 (8个)
**优先级：中** - 社交功能

- [ ] GuildNoticeChange
- [ ] GuildMemberChange
- [ ] GuildExpGain
- [ ] GuildNameRequest
- [ ] GuildStorageList
- [ ] GuildRequestWar
- [ ] GuildBuffList
- [ ] GuildTerritoryPage

### 第 6 批：英雄系统补充 (8个)
**优先级：高** - 玩法核心

- [ ] HeroCreateRequest
- [ ] UpdateHeroSpawnState
- [ ] UnlockHeroAutoPot
- [ ] SetAutoPotValue
- [ ] SetAutoPotItem
- [ ] SetHeroBehaviour
- [ ] ManageHeroes
- [ ] ChangeHero

### 第 7 批：玩家状态与信息 (15个)
**优先级：中**

- [ ] PlayerUpdate
- [ ] PlayerInspect
- [ ] LogOutSuccess
- [ ] LogOutFailed
- [ ] ReturnToLogin
- [ ] TimeOfDay
- [ ] ChangeAMode
- [ ] ChangePMode
- [ ] ObjectName
- [ ] UserStorage
- [ ] ObjectHealth
- [ ] ObjectMana
- [ ] BaseStatsInfo
- [ ] HeroBaseStatsInfo
- [ ] UserName

### 第 8 批：对象行为扩展 (18个)
**优先级：中** - 战斗动作

- [ ] UserBackStep
- [ ] ObjectBackStep
- [ ] UserDashAttack
- [ ] ObjectDashAttack
- [ ] UserAttackMove
- [ ] ObjectSitDown
- [ ] ObjectHidden
- [ ] ObjectSpell
- [ ] ObjectDeco
- [ ] ObjectSneaking
- [ ] ObjectLevelEffects

### 第 9 批：特效与战斗辅助 (8个)
**优先级：中**

- [ ] MapEffect
- [ ] AllowObserve
- [ ] InTrapRock
- [ ] SetConcentration
- [ ] SetElemental
- [ ] RemoveDelayedExplosion
- [ ] SetBindingShot
- [ ] Awakening

### 第 10 批：任务系统补充 (2个)
**优先级：中**

- [ ] ChangeQuest
- [ ] NewQuestInfo

### 第 11 批：婚姻导师系统 (3个)
**优先级：低**

- [ ] MarriageRequest
- [ ] DivorceRequest
- [ ] MentorRequest

### 第 12 批：坐骑与变身 (3个)
**优先级：中**

- [ ] MountUpdate
- [ ] TransformUpdate
- [ ] EquipSlotItem

### 第 13 批：钓鱼系统 (1个)
**优先级：低**

- [ ] FishingUpdate

### 第 14 批：轮回系统 (2个)
**优先级：低**

- [ ] CancelReincarnation
- [ ] RequestReincarnation

### 第 15 批：市场拍卖 (3个)
**优先级：中**

- [ ] ConsignItem
- [ ] MarketFail
- [ ] MarketSuccess

### 第 16 批：觉醒系统 (2个)
**优先级：中**

- [ ] AwakeningNeedMaterials
- [ ] Awakening

### 第 17 批：邮件系统 (7个)
**优先级：中** - 社交功能

- [ ] ReceiveMail
- [ ] MailLockedItem
- [ ] MailSendRequest
- [ ] MailSent
- [ ] ParcelCollected
- [ ] MailCost
- [ ] SendOutputMessage

### 第 18 批：背包仓库扩展 (2个)
**优先级：中**

- [ ] ResizeInventory
- [ ] ResizeStorage

### 第 19 批：灵兽系统 (4个)
**优先级：低**

- [ ] NewIntelligentCreature
- [ ] UpdateIntelligentCreatureList
- [ ] IntelligentCreatureEnableRename
- [ ] IntelligentCreaturePickup

### 第 20 批：社交关系 (3个)
**优先级：中**

- [ ] FriendUpdate
- [ ] LoverUpdate
- [ ] MentorUpdate

### 第 21 批：商城系统 (2个)
**优先级：中**

- [ ] GameShopInfo
- [ ] GameShopStock

### 第 22 批：排行榜 (1个)
**优先级：低**

- [ ] Rankings

### 第 23 批：门与开门 (1个)
**优先级：低**

- [ ] Opendoor

### 第 24 批：租赁系统 (12个)
**优先级：低**

- [ ] GetRentedItems
- [ ] ItemRentalRequest
- [ ] ItemRentalFee
- [ ] ItemRentalPeriod
- [ ] DepositRentalItem
- [ ] RetrieveRentalItem
- [ ] UpdateRentalItem
- [ ] CancelItemRental
- [ ] ItemRentalLock
- [ ] ItemRentalPartnerLock
- [ ] CanConfirmItemRental
- [ ] ConfirmItemRental

### 第 25 批：配方与制造 (1个)
**优先级：中**

- [ ] NewRecipeInfo

### 第 26 批：UI 辅助功能 (7个)
**优先级：低**

- [ ] OpenBrowser
- [ ] PlaySound
- [ ] SetTimer
- [ ] ExpireTimer
- [ ] UpdateNotice
- [ ] Roll
- [ ] SetCompass

### 第 27 批：账号相关 (5个)
**优先级：高** - 登录必需

- [ ] NewAccount
- [ ] ChangePassword
- [ ] ChangePasswordBanned
- [ ] NewCharacter
- [ ] NewCharacterSuccess
- [ ] DeleteCharacter
- [ ] DeleteCharacterSuccess

### 第 28 批：信息列表 (3个)
**优先级：高** - 初始化必需

- [ ] NewItemInfo
- [ ] NewHeroInfo
- [ ] NewChatItem

### 第 29 批：聊天物品统计 (1个)
**优先级：低**

- [ ] ChatItemStats

### 第 30 批：NPCUpdate (2个)
**优先级：中**

- [ ] NPCUpdate
- [ ] NPCImageUpdate

## 实施策略

### 阶段 A：核心功能优先 (第 1-3 周)
**目标：让游戏可以基本运行**

1. **第 1-2 周**：实现第 1-8 批（高优先级）
   - NPC 交互系统
   - 物品系统补充
   - 魔法系统补充
   - 英雄系统补充
   - 玩家状态
   - 对象行为
   
   **预期完成：~120 个数据包**

2. **第 3 周**：实现第 9-15 批（中优先级基础）
   - 特效系统
   - 任务系统
   - 坐骑变身
   - 市场觉醒
   
   **预期完成：~150 个数据包**

### 阶段 B：完善功能 (第 4-5 周)
**目标：补全所有游戏功能**

3. **第 4 周**：实现第 16-26 批（中优先级扩展）
   - 邮件系统
   - 灵兽系统
   - 社交系统
   - 商城排行
   - UI 辅助
   
   **预期完成：~220 个数据包**

4. **第 5 周**：实现第 27-30 批（补遗与优化）
   - 账号管理
   - 信息列表
   - 租赁系统
   - NPCUpdate
   
   **预期完成：全部 285 个数据包**

### 阶段 C：测试与优化 (第 6 周)
**目标：确保所有数据包正确工作**

- 单元测试
- 集成测试
- 与 C# 客户端对比测试
- 性能优化

## 当前行动计划

### 今天（第 1 天）
**目标：完成第 1-3 批，共 42 个数据包**

#### 上午任务
1. ✅ 创建本计划文档
2. 🔄 实现 NPC 交互系统（18 个）
   - 定义结构体
   - 实现解析函数
   - 添加枚举变体
   - 添加路由
   - 添加 UI 处理

#### 下午任务
3. 实现物品系统补充（20 个）
4. 实现魔法系统补充（4 个）

#### 晚上任务
5. 测试与验证
6. 文档更新

### 本周目标
- 完成前 8 批（约 120 个数据包）
- 游戏基本可运行

### 本月目标
- 完成所有 285 个数据包
- 开始实现完整的游戏逻辑（阶段 B）

## 注意事项

1. **严格参考 C# 实现**
   - 每个数据包都查看 GameScene.cs 中的处理逻辑
   - 保持字段名称和类型一致
   - 复制业务逻辑而不是简化

2. **测试驱动**
   - 每批完成后立即编译测试
   - 确保无编译错误
   - 运行时测试数据包解析

3. **文档同步**
   - 每批完成后更新本文档
   - 标记完成状态
   - 记录特殊问题

4. **性能考虑**
   - 避免不必要的内存分配
   - 使用 Rust 最佳实践
   - 保持与 C# 相同的逻辑复杂度

## 成功标准

- [ ] 所有 285 个数据包定义完成
- [ ] 所有解析函数正确实现
- [ ] 所有路由正确连接
- [ ] 所有 UI 处理器实现
- [ ] 编译无警告
- [ ] 能连接服务器接收所有数据包
- [ ] 与 C# 客户端行为一致
