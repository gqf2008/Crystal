# NetworkEvent 移植状态文档

> 生成时间: 2026-04-21
> 分支: marcoquad-native-ui
> 目标: 将所有 PARTIAL/stub NetworkEvent handler 升级为通过 ECS 管线传递真实数据

## 管线架构

```
PacketHandler → NetworkEvent → dialog_system → UiCommand → ui_system → Dialog
                            ↘ network_apply_system → ECS Component 更新
```

---

## 一、已完成移植 (WIRED)

以下 NetworkEvent 变体已携带真实包数据，下游消费者（dialog_system / network_apply_system / ui_system）已同步更新。

### 连接 (Connection)
| 变体 | 字段 |
|------|------|
| `Disconnected` | `reason: String` |
| `KeepAliveReceived` | `time: i64` |
| `ClientVersionResponse` | `result: u8` |

### 认证 (Auth)
| 变体 | 字段 |
|------|------|
| `LoginSuccess` | `characters: Vec<SelectInfo>` |
| `LoginFailed` | `reason: String` |
| `NewAccountFailed` | `reason: String` |
| `ChangePasswordFailed` | `reason: String` |

### 角色 (Character)
| 变体 | 字段 |
|------|------|
| `LogOutSuccess` | `characters: Vec<SelectInfo>` |
| `StartGame` | `packet: StartGame` |
| `StartGameDelay` | `packet: StartGameDelay` |
| `StartGameBanned` | `packet: StartGameBanned` |
| `UserInformation` | `packet: UserInformation` |
| `PlayerUpdated` | `object_id, light, weapon, weapon_effect, armor, wings_effect` |

### 地图/移动 (Map/Movement)
| 变体 | 字段 |
|------|------|
| `MapInformation` | `packet: MapInformation` |
| `MapChanged` | `packet: MapChanged` |
| `PlayerLocationChanged` | `x, y` |
| `ObjectMoved` | `object_id, x, y, direction` |
| `ObjectTeleportingIn` | `object_id, teleport_type` |
| `ObjectTeleportingOut` | `object_id, teleport_type` |
| `ObjectBackStepped` | `object_id, location_x, location_y, direction, distance` |
| `ObjectDashing` | `object_id, location_x, location_y, direction` |
| `ObjectDashFailed` | `object_id, location_x, location_y, direction` |
| `ObjectSatDown` | `object_id, direction, location` |
| `PlayerDashFailed` | `location_x, location_y, direction` |

### 战斗 (Combat)
| 变体 | 字段 |
|------|------|
| `PlayerDied` | `x, y, direction` |
| `ObjectDied` | `object_id, location_x, location_y, direction, death_type` |
| `UserDashAttacked` | `x, y, direction` |
| `ObjectDashAttacked` | `object_id, location_x, location_y, direction, distance` |
| `ObjectDashFailed` | `object_id, location_x, location_y, direction` |
| `ObjectRevivedEvent` | `object_id, effect` |
| `ObjectLeveled` | `object_id, level` |
| `ObjectSneakingReceived` | `object_id, sneaking` |
| `ObjectDecoReceived` | `object_id, deco, remove` |
| `ObjectLevelEffectsReceived` | `object_id, level_effects` |
| `ObjectSpellReceived` | `object_id, location_x, location_y, spell` |
| `MagicListReceived` | `spell, target_id, target_x, target_y, cast, level` |
| `DamageIndicator` | `object_id, damage, damage_type` |
| `ObjectHealthPercent` | `object_id, percent, expire` |
| `DelayedExplosionRemoved` | `object_id: u32` |
| `RangeAttacked` | `target_id, target_x, target_y, spell, spell_level` |
| `ObjectRangeAttacked` | `object_id, location_x, location_y, direction, target_id, target_x, target_y, spell, spell_level` |
| `PushedEvent` | `object_id, x, y, direction` |
| `ObjectPushedEvent` | `object_id, x, y, direction` |
| `ObjectStruck` | `object_id, attacker_id, damage, location_x, location_y, direction` |
| `ObjectMagicCast` | `object_id, location_x, location_y, direction, spell, target_id, target_x, target_y, cast, level` |
| `MagicLearned` | `magic: ClientMagic, hero` |
| `MagicRemoved` | `spell, hero` |
| `MagicLeveledUp` | `spell, level, hero` |
| `SpellToggled` | `spell, can_use, hero` |
| `ObjectAttack` | `object_id, location_x, location_y, direction, spell, level, attack_type` |

### Buff/状态 (Buff/State)
| 变体 | 字段 |
|------|------|
| `ConcentrationSet` | `object_id, enabled, interrupted` |
| `ElementalSet` | `object_id, enabled, value, element, expire_time` |
| `BuffAdded` | `object_id, buff_id, visible, expire_time, infinite, paused` |

### 视觉/效果 (Visual/Effect)
| 变体 | 字段 |
|------|------|
| `ObjectEffectReceived` | `object_id, effect, effect_type, delay_time, time` |
| `MapEffectReceived` | `effect, location_x, location_y, value` |
| `OutputMessageReceived` | `message, message_type` |
| `DoorOpened` | `door_id, close` |

### 物体生命周期 (Object Lifecycle)
| 变体 | 字段 |
|------|------|
| `ObjectRemove` | `object_id` |

### 物品 (Item)
| 变体 | 字段 |
|------|------|
| `ItemMoved` | `grid, from, to, success` |
| `ItemEquipped` | `grid, unique_id, slot, success` |
| `ItemMerged` | `grid_from, grid_to, id_from, id_to, success` |
| `ItemRemoved` | `grid, unique_id, to, success` |
| `ItemSlotRemoved` | `grid, grid_to, slot, unique_id, success` |
| `ItemSlotEquipped` | `grid, grid_to, slot, unique_id, success` |
| `ItemSplit` | `grid, unique_id, count` |
| `ItemCombined` | `grid, id_from, id_to, success, destroy` |
| `SellItemReceived` | `unique_id, count, success` |
| `CraftItemReceived` | `unique_id, count, success` |
| `RepairItemReceived` | `unique_id` |
| `ItemRepairedEvent` | `unique_id, max_dura, current_dura` |
| `RefineItemDeposited` | `from, to, success` |
| `RefineItemRetrieved` | `from, to, success` |
| `RefineCancelled` | `unlock` |
| `RefineItemCompleted` | `unique_id` |
| `HeroItemTakenBack` | `from, to, success` |
| `HeroItemTransferred` | `from, to, success` |
| `NewItemInfoReceived` | `item_index, item_name` |
| `NewChatItemReceived` | `item_id` |
| `ItemRemoved` | `unique_id, to, success` |
| `ItemSlotRemoved` | `slot, unique_id, success` |
| `ItemDropped` | `unique_id, count, success` |
| `ItemLost` | `unique_id, count` |
| `ItemSealed` | `unique_id, expiry_date` |
| `ObjectHarvested` | `object_id, location_x, location_y, direction` |

### 交易 (Trade)
| 变体 | 字段 |
|------|------|
| `TradeItemAdded` | `items: Vec<Option<UserItem>>` |
| `TradeCancelledEvent` | `unlock` |
| `ConsignItemEvent` | `unique_id, success` |
| `MarketFailedEvent2` | `reason: String` |
| `MarketSuccessEvent2` | `message: String` |

### NPC
| 变体 | 字段 |
|------|------|
| `NPCUpdated` | `npc_id` |
| `NPCImageUpdated` | `npc_id, image` |
| `NPCRepairReceived` | `rate: f32` |
| `NPCSRepairReceived` | `rate: f32` |
| `NPCRefineReceived` | `rate: f32, refining: bool` |
| `NPCCollectRefineReceived` | `success: bool` |
| `NPCReplaceWedRingReceived` | `rate: f32` |
| `AwakeningLockedItemReceived` | `unique_id, locked` |
| `AwakeningReceived` | `result, remove_id` |

### 组队 (Group)
| 变体 | 字段 |
|------|------|
| `GroupMemberLocationUpdated` | `name, x, y` |

### 公会 (Guild)
| 变体 | 字段 |
|------|------|
| `GuildJoined` | `guild_name, rank_name, level, experience, max_experience, gold, spare_points, member_count, max_members, voting, item_count, buff_count, my_options, my_rank_id` |
| `GuildLeft` | — (由 GuildStatus 空名触发) |
| `GuildWarRequested` | `guild_name` |
| `GuildTerritoryPageReceived` | `territories: Vec<TerritoryInfo>` |
| `GuildTerritoryPurchased` | `success` |
| `GuildNameReceived` | `name: String` |

### 英雄 (Hero)
| 变体 | 字段 |
|------|------|
| `HeroCreateRequested` | `can_create_class: Vec<bool>` |
| `NewHeroCreated` | `hero_info: String` |
| `HeroAutoPotUnlocked` | `unlocked` |
| `HeroChanged` | `success` |
| `HeroBehaviourSet` | `behaviour, pet_mode` |

### 社交 (Social)
| 变体 | 字段 |
|------|------|
| `DivorceRequested2` | `lover_name` |
| `MentorRequested2` | `mentor_name` |

### 邮件 (Mail)
| 变体 | 字段 |
|------|------|
| `MailLockedItemReceived` | `unique_id, locked` |
| `MailSendRequestReceived` | `mail_id` |
| `MailSentEvent` | `result: i8` |
| `ParcelCollectedEvent` | `success` |

### 租赁 (Rental)
| 变体 | 字段 |
|------|------|
| `RentalItemUpdated` | `fee, period` |
| `ItemRentalCancelled` | `success` |
| `ItemRentalLocked` | `locked` |
| `ItemRentalPartnerLocked` | `locked` |
| `ItemRentalConfirmable` | `can_confirm` |
| `ItemRentalConfirmed` | `success` |

### 任务 (Quest)
| 变体 | 字段 |
|------|------|
| `QuestItemGained` | `item_id: i32` |

### 市场 (Market) — *2 后缀变体
| 变体 | 字段 |
|------|------|
| `NPCMarketEvent2` | `pages: Vec<String>` |
| `NPCMarketPageEvent2` | `listings: Vec<MarketListing>` |

---

## 二、空包 — 无需移植

服务器包本身没有字段，unit 变体是正确的。

| 变体 | 服务器包 |
|------|----------|
| `Connected` | Connected (空) |
| `GroupDisbanded` | DeleteGroup (空) |
| `TradeCompleted` | TradeConfirm (空) |
| `NPCSellReceived` | NPCSell (空) |
| `NPCCheckRefineReceived` | NPCCheckRefine (空) |
| `NPCStorageReceived` | NPCStorage (空) |
| `NPCConsignReceived` | NPCConsign (空) |
| `NPCConsignEvent` | NPCConsign (空) |
| `NPCAwakeningReceived` | NPCAwakening (空) |
| `NPCDisassembleReceived` | NPCDisassemble (空) |
| `NPCDowngradeReceived` | NPCDowngrade (空) |
| `NPCResetReceived` | NPCReset (空) |
| `PlayerTeleportedIn` | TeleportIn (空) |
| `PlayerRevived` | Revived (空) |
| `ReincarnationRequested` | RequestReincarnation (空) |
| `ReincarnationCancelled` | CancelReincarnation (空) |
| `LogOutFailed` | LogOutFailed (空) |
| `ReturnToLogin` | ReturnToLogin (空) |
| `ItemRentalRequested` | ItemRentalRequest (空) |
| `GuildWarReturn` | GuildNameRequest (空) |
| `NewAccountSuccess` | NewAccount (result=8) |
| `ChangePasswordSuccess` | ChangePassword (result=6) |
| `QuestListUpdated` | 通知事件 (无额外字段) |
| `GuildLeft` | GuildStatus (空名) |

---

## 三、已删除 — 死代码变体

以下裸变体已被带数据的 `*2` 或 `*Event` 后缀变体替代，已从 NetworkEvent 枚举中移除。

| 已删除变体 | 替代变体 |
|------------|----------|
| `NPCMarketEvent` | `NPCMarketEvent2 { pages }` |
| `NPCMarketPageEvent` | `NPCMarketPageEvent2 { listings }` |
| `ConsignItemReceived` | `ConsignItemEvent { unique_id, success }` |
| `MarketSuccessEvent` | `MarketSuccessEvent2 { message }` |
| `MarketFailedEvent` | `MarketFailedEvent2 { reason }` |
| `TradeCancelled` | `TradeCancelledEvent { unlock }` |

---

## 四、客户端→服务器 (无需移植)

纯客户端请求事件，无需从包解析数据：

`DisconnectRequest`, `KeepAliveSend`, `LoginRequest`, `NewAccountRequest`, `ChangePasswordRequest`, `NewCharacterRequest`, `DeleteCharacterRequest`, `StartGameRequest`, `MoveRequest`, `WalkRequest`, `RunRequest`, `TurnRequest`, `AttackRequest`, `MagicRequest`, `ChatRequest`, `InspectRequest`, `PickupItemRequest`, `MoveItemRequest`, `DropItemRequest`, `UseItemRequest`, `EquipItemRequest`, `RemoveItemRequest`, `RemoveSlotItemRequest`, `SplitItemRequest`, `MergeItemRequest`, `StoreItemRequest`, `TakeBackItemRequest`, `DropGoldRequest`, `EquipSlotItemRequest`, `CombineItemRequest`, `DropItemStackRequest`, `GroupInviteRequest`, `GroupAcceptRequest`, `GroupDeclineRequest`, `GroupLeaveRequest`, `GroupKickRequest`, `GuildInviteRequest`, `GuildAcceptRequest`, `GuildDeclineRequest`, `GuildLeaveRequest`, `TradeRequest`, `TradeReplyRequest`, `TradeGoldRequest`, `TradeConfirmRequest`, `TradeCancelRequest`, `AcceptQuestRequest`, `FinishQuestRequest`, `AbandonQuestRequest`, `ShareQuestRequest`, `NPCCallRequest`, `BuyItemRequest`, `SellItemRequest`, `RepairItemRequest`, `LogOutRequest`, `HarvestRequest`, `BuyItemBackRequest`, `SRepairItemRequest`, `CheckRefineRequest`, `ReplaceWedRingRequest`, `NPCConfirmInput`, `CreateHeroRequest`, `SetHeroAutoPotValue`, `SetHeroAutoPotItem`, `SetHeroBehaviourRequest`, `ChangeHeroRequest`, `SendMailRequest`, `ReadMailRequest`, `CollectParcelRequest`, `DeleteMailRequest`, `LockMailRequest`, `ConsignItemRequest`, `MarketSearchRequest`, `MarketRefreshRequest`, `MarketPageRequest`, `MarketBuyRequest`, `MarketGetBackRequest`, `MarketSellNowRequest`, `MountRideRequest`, `MountDismountRequest`, `TownReviveRequest`, `DepositRefineItemRequest`, `RetrieveRefineItemRequest`, `RefineCancelRequest`, `RefineItemRequest`, `DepositTradeItemRequest`, `RetrieveTradeItemRequest`, `TakeBackHeroItemRequest`, `TransferHeroItemRequest`, `SwitchGroupRequest`, `SpellToggleRequest`, `AwakeningNeedMaterialsRequest`, `AwakeningLockedItemRequest`, `AwakeningRequest`, `DisassembleItemRequest`, `DowngradeAwakeningRequest`, `ResetAddedItemRequest`, `MailLockedItemRequest`, `MailCostRequest`, `ItemRentalRequestEvent`, `ItemRentalFeeRequest`, `ItemRentalPeriodRequest`, `ItemRentalLockFeeEvent`, `ItemRentalLockItemEvent`, `FishingCastRequest`, `FishingAutocastToggle`, `AcceptReincarnationRequest`, `CancelReincarnationRequest`, `GameShopBuyRequest`, `ReportIssueRequest`, `GetRankingRequest`, `OpenDoorRequest`, `RequestMapInfoRequest`, `TeleportToNPCRequest`, `SearchMapRequest`, `ObserveRequest`, `MarriageRequestSend`, `MarriageReply`, `ChangeMarriageRequest`, `DivorceRequestSend`, `DivorceReply`, `AddMentorRequest`, `MentorReply`, `AllowMentorRequest`, `CancelMentorRequest`, `RequestUserNameQuery`, `RequestChatItemQuery`, `ChangeAModeRequest`, `ChangePModeRequest`, `ChangeTradeToggle`, `MagickeySet`, `AddFriendRequest`, `RemoveFriendRequest`, `RefreshFriendsRequest`, `AddMemoRequest`, `EditGuildMember`, `EditGuildNotice`, `GuildNameReturn`, `RequestGuildInfo`, `GuildStorageGoldChange`, `GuildStorageItemChangeRequest`, `GuildWarReturn`, `GuildBuffUpdate`, `GuildTerritoryPageRequest`, `PurchaseGuildTerritoryRequest`, `GetRentedItemsRequest`, `RentalItemDepositRequest`, `RentalItemRetrieveRequest`, `ItemRentalConfirm`, `ItemRentalCancel`, `CraftItemRequest`, `UpdateIntelligentCreatureRequest`, `IntelligentCreaturePickupRequest`, `RequestIntelligentCreatureUpdates`

---

## 五、移植历史

### 第一批 — 基础字段升级
- [x] KeepAliveReceived: 0 → packet.time
- [x] SellItemReceived: → unique_id, count, success
- [x] CraftItemReceived: → unique_id, count, success
- [x] RepairItemReceived: → unique_id
- [x] ItemRepairedEvent: → unique_id, max_dura, current_dura
- [x] TradeItemAdded: → items: Vec<Option<UserItem>>
- [x] TradeCancelledEvent: → unlock
- [x] PlayerDied: → x, y, direction
- [x] UserDashAttacked: → x, y, direction
- [x] PlayerUpdated: → object_id, light, weapon, weapon_effect, armor, wings_effect
- [x] DivorceRequested2: → lover_name
- [x] MentorRequested2: → mentor_name
- [x] GuildJoined: → 10字段
- [x] GuildWarRequested: → guild_name
- [x] GuildTerritoryPageReceived: → territories
- [x] NPCUpdated: → npc_id
- [x] NPCImageUpdated: → npc_id, image
- [x] HeroCreateRequested: → can_create_class
- [x] NewHeroCreated: → hero_info
- [x] HeroAutoPotUnlocked: → unlocked
- [x] HeroChanged: → success

### 第二批 — 精炼/英雄物品
- [x] RefineItemDeposited: → from, to, success
- [x] RefineItemRetrieved: → from, to, success
- [x] RefineCancelled: → unlock
- [x] HeroItemTakenBack: → from, to, success
- [x] HeroItemTransferred: → from, to, success

### 第三批 — 租赁系统
- [x] RentalItemUpdated: → fee, period
- [x] ItemRentalCancelled: → success
- [x] ItemRentalLocked: → locked
- [x] ItemRentalPartnerLocked: → locked
- [x] ItemRentalConfirmable: → can_confirm
- [x] ItemRentalConfirmed: → success

### 第四批 — 登出/心跳/杂项
- [x] LogOutSuccess: → characters: Vec<SelectInfo>
- [x] DelayedExplosionRemoved: → object_id
- [x] QuestItemGained: → item_id

### 第五批 — 邮件/物品/公会/魔法
- [x] MailLockedItemReceived: → unique_id, locked
- [x] MailSendRequestReceived: → mail_id
- [x] MailSentEvent: → result
- [x] ParcelCollectedEvent: → success
- [x] NewItemInfoReceived: → item_index, item_name
- [x] NewChatItemReceived: → item_id (新变体)
- [x] RefineItemCompleted: → unique_id
- [x] GuildTerritoryPurchased: → success
- [x] MagicListReceived: → spell, target_id, cast, level

### 第六批 — 移动/动画数据
- [x] ObjectTeleportingIn: → object_id, teleport_type
- [x] ObjectBackStepped: → object_id, location_x, location_y, direction, distance
- [x] ObjectDashing: → object_id, location_x, location_y, direction
- [x] PlayerDashFailed: → location_x, location_y, direction

### 第七批 — NPC 操作费率/结果
- [x] NPCRepairReceived: → rate: f32
- [x] NPCSRepairReceived: → rate: f32
- [x] NPCRefineReceived: → rate: f32, refining: bool
- [x] NPCCollectRefineReceived: → success: bool
- [x] NPCReplaceWedRingReceived: → rate: f32

### 第八批 — 死代码清理 + GuildLeft
- [x] 删除 6 个死代码裸变体 (NPCMarketEvent, NPCMarketPageEvent, ConsignItemReceived, MarketSuccessEvent, MarketFailedEvent, TradeCancelled)
- [x] GuildLeft: 由 GuildStatus 空名触发 (guild.rs handler 分支)

### 第九批 — 战斗/动画关键数据
- [x] ObjectDied: → object_id, location_x, location_y, direction, death_type
- [x] ObjectDashAttacked: → object_id, location_x, location_y, direction, distance
- [x] ObjectDashFailed: → object_id, location_x, location_y, direction
- [x] ObjectSatDown: → object_id, direction, location

### 第十批 — 对象状态字段
- [x] ObjectTeleportingOut: → object_id, teleport_type
- [x] ObjectRevivedEvent: → object_id, effect
- [x] ObjectLeveled: → object_id, level
- [x] ObjectSneakingReceived: → object_id, sneaking
- [x] ObjectDecoReceived: → object_id, deco, remove
- [x] ObjectLevelEffectsReceived: → object_id, level_effects
- [x] ObjectSpellReceived: → object_id, location_x, location_y, spell

### 第十一批 — 物品操作字段
- [x] ItemRemoved: → unique_id, to, success
- [x] ItemSlotRemoved: → slot, unique_id, success
- [x] ItemDropped: → unique_id, count, success
- [x] ItemLost: → unique_id, count
- [x] ItemSealed: → unique_id, expiry_date
- [x] ObjectHarvested: → object_id, location_x, location_y, direction
- [x] AwakeningLockedItemReceived: → unique_id, locked
- [x] AwakeningReceived: → result, remove_id

### 第十二批 — Chat 类型修复
- [x] Chat handler: 使用真实 chat_type 替代硬编码 ChatType::System

### 第十三批 — 远程攻击/推挤数据
- [x] RangeAttacked: → target_id, target_x, target_y, spell, spell_level
- [x] ObjectRangeAttacked: → object_id, location_x, location_y, direction, target_id, target_x, target_y, spell, spell_level
- [x] PushedEvent: → object_id, x, y, direction
- [x] ObjectPushedEvent: → object_id, x, y, direction

### 第十四批 — 状态/Buff/Hero 数据
- [x] ConcentrationSet: → object_id, enabled, interrupted
- [x] ElementalSet: → object_id, enabled, value, element, expire_time
- [x] BuffAdded: → object_id, buff_id, visible, expire_time, infinite, paused
- [x] HeroBehaviourSet: → behaviour, pet_mode

### 第十五批 — 视觉关键数据
- [x] ObjectStruck: → object_id, attacker_id, damage, location_x, location_y, direction
- [x] ObjectMagicCast: → object_id, location_x, location_y, direction, spell, target_id, target_x, target_y, cast, level
- [x] ObjectEffectReceived: → object_id, effect, effect_type, delay_time, time
- [x] MapEffectReceived: → effect, location_x, location_y, value
- [x] OutputMessageReceived: → message, message_type
- [x] DoorOpened: → door_id, close

### 第十六批 — 魔法 hero 标志和目标坐标
- [x] MagicLearned: → spell, level, hero
- [x] MagicRemoved: → spell, hero
- [x] MagicLeveledUp: → spell, level, hero
- [x] SpellToggled: → spell, can_use, hero
- [x] MagicListReceived: → spell, target_id, target_x, target_y, cast, level

### 第十七批 — 物品操作 grid 字段
- [x] ItemMoved: → grid, from, to, success
- [x] ItemEquipped: → grid, unique_id, slot, success
- [x] ItemMerged: → grid_from, grid_to, id_from, id_to, success
- [x] ItemRemoved: → grid, unique_id, to, success
- [x] ItemSlotRemoved: → grid, grid_to, slot, unique_id, success
- [x] ItemSlotEquipped: → grid, grid_to, slot, unique_id, success
- [x] ItemSplit: → grid, unique_id, count
- [x] ItemCombined: → grid, id_from, id_to, success, destroy

### 第十八批 — ClientMagic 完整数据
- [x] MagicLearned: → magic: ClientMagic, hero (替代 spell+level，携带 name/icon/key/experience/delay/range 等)
- [x] UiCommand::MagicLearned: → spell, name, level, icon (替代 spell+level)
- [x] learn_skill: 接受 icon 参数，更新 SkillInfo.icon_index 和 name

### 第十九批 — GuildJoined 补充字段
- [x] GuildJoined: → voting, item_count, buff_count, my_options (从 GuildStatus 包补充)

### 第二十批 — 分解 ObjectAttack / ObjectRemove 数据包结构体
- [x] ObjectAttack: → object_id, location_x, location_y, direction, spell, level, attack_type (替代 packet 整体传递)
- [x] ObjectRemove: → object_id (替代 packet 整体传递)
- [x] apply_object_attack: 改为接收独立参数，避免依赖 server packet 类型

### 第二十一批 — BuffAdded 字段接入 UI
- [x] BuffAdded dialog_system: 使用 visible 过滤不可见 buff，使用 paused 设置 is_paused
- [x] BuffAdded dialog_system: 使用 expire_time/infinite 计算 remaining_secs（支持 .NET ticks 和 Unix ms 两种格式）

### 第二十二批 — Magic 事件 hero 标志
- [x] UiCommand magic 变体: 增加 hero 字段 (MagicLearned/MagicLeveledUp/MagicRemoved/SpellToggled)
- [x] ui_system: 根据 hero 标志分支，玩家技能更新角色对话框，英雄技能留 TODO

### 第二十三批 — /simplify 审查修复
- [x] 关键 Bug: 删除 network_apply_system.rs 中重复的 `spell_casts.push()`
- [x] 性能: dialog_system.rs BuffAdded 循环外缓存 `SystemTime::now()`
- [x] 代码质量: ObjectAttack 6 字段元组替换为 `ObjectAttackData` 命名结构体
- [x] 代码质量: .NET ticks 转换魔法数字替换为命名常量 (`DOTNET_TICKS_AT_UNIX_EPOCH`, `TICKS_PER_MILLISECOND`)

### 第二十四批 — 网络事件 ECS 落地（network_apply_system）
- [x] ObjectBackStepped / PlayerBackStepped: 位置落地到 `apply_object_move`
- [x] ObjectDashing / PlayerDashing: 位置落地到 `apply_object_move`
- [x] PushedEvent / ObjectPushedEvent: 位置落地到 `apply_object_move`
- [x] UserDashAttacked / ObjectDashAttacked: 位置落地到 `apply_object_move`
- [x] UserAttackMoved: 位置落地到 `apply_object_move`
- [x] ObjectTeleportingOut: 映射到 `hidden_objects`（隐藏对象）
- [x] ObjectTeleportingIn: 映射到 `shown_objects`（显示对象）
- [x] ObjectSatDown: 怪物/NPC 更新 `MonsterAnimState.action = SitDown`
- [x] ObjectDashFailed: 怪物/NPC 更新 `MonsterAnimState.action = DashFail`
- [x] ObjectHarvested: 位置落地 + 怪物/NPC 更新 `MonsterAnimState.action = Harvest`
- [x] ObjectRangeAttacked: 位置落地 + 怪物/NPC 朝向更新
- [x] ObjectLeveled: 更新 `OtherPlayer.level`

### 第二十五批 — trace-only 事件 ECS 落地补充
- [x] ObjectManaPercent: 根据 percent 更新目标实体 `Mana.current`
- [x] ObjectSneakingReceived: 更新/插入目标实体 `Visibility.hidden`
- [x] ObjectLevelEffectsReceived: 更新/插入目标实体 `LevelEffectsFlags`
- [x] ObjectSpellReceived: 通过 `set_monster_anim` 设置 `MonsterAnimState.action = Spell`

### 第二十六批 — UiState 全局状态落地
- [x] TimeOfDayChanged: 直接写入 `UiState.time_of_day`
- [x] TransformUpdated: 直接写入 `UiState.transform_form`
- [x] MapEffectReceived: 直接写入 `UiState.pending_map_effect`
- [x] ObserveAllowed: 直接写入 `UiState.observe_allowed`
- [x] BindingShotSet: 直接写入 `UiState.binding_shot_enabled`
- [x] ConcentrationSet: 仅当 `object_id == local_player_object_id` 时写入 `UiState.concentration_enabled`
- [x] DoorOpened: `close=true` 时从 `UiState.open_doors` remove，`close=false` 时 insert

### 第二十七批 — ObjectMonster / ObjectPlayer 包字段补充落地
- [x] ObjectMonster.dead: 设置 `Health.current = 0`，插入 `DeathState`，`Visibility.dead = true`
- [x] ObjectMonster.hidden: 更新/插入 `Visibility.hidden`
- [x] ObjectMonster.poison: 映射到 `BuffList`（GREEN/RED → Poison，BLEEDING → Bleeding）
- [x] ObjectPlayer.dead: 设置 `Health.current = 0`，插入 `DeathState`，`Visibility.dead = true`
- [x] ObjectPlayer.hidden: 更新/插入 `Visibility.hidden`
- [x] ObjectPlayer.name_colour: 更新/插入 `NameColor`
- [x] ObjectPlayer.level_effects: 更新/插入 `LevelEffectsFlags`
- [x] ObjectPlayer.poison: 映射到 `BuffList`（GREEN/RED → Poison，BLEEDING → Bleeding）

### 第二十八批 — 补充字段与渲染联动
- [x] ObjectMonster.name_colour: 更新/插入 `NameColor`
- [x] ObjectMonster/Player.buffs: 通过 `map_server_buff` 映射可识别的 buff 到 `BuffList`
- [x] SpriteRenderSystem::draw_character 读取 `Visibility.hidden` 跳过隐藏实体（使 ObjectSneakingReceived / ObjectMonster.hidden / ObjectPlayer.hidden 真正生效）

### 第二十九批 — Buff ECS 修复与完善
- [x] Buff 组件新增 `paused: bool` 字段，`update` 暂停时不扣减 `remaining_duration`
- [x] BuffList 新增 `set_buff_paused` 方法
- [x] BuffAdded: 根据 `buff_id` 映射到正确的 `BuffType`（替代之前硬编码的 `Poison`）
- [x] BuffRemoved: 使用 `remove_buff` 按类型精确移除（替代之前的 `pop()`）
- [x] BuffPaused: 通过 `set_buff_paused` 落地到 `BuffList`

### 第三十批 — 死亡/复活 ECS 完善
- [x] ObjectDied: 更新/插入 `Visibility.dead = true`
- [x] Revived (PlayerRevived + ObjectRevivedEvent): 恢复 `Visibility.dead = false`、`Health.current`、`MonsterAnimState.action = Standing`

### 第三十一批 — 丢失事件补充与技能状态
- [x] PlayerStruck: 映射到 `object_struck`（复用受击音效/动画/伤害数字处理）
- [x] LearnedMagic 新增 `can_use: bool` 字段
- [x] SpellToggled: 更新本地玩家 `MagicList` 中对应法术的 `can_use`

### 第三十二批 — 中毒/流血处理修复
- [x] ObjectPoisonedEvent: 根据 `poison_type` (PoisonType bits) 映射到正确的 `BuffType`（替代之前硬编码的 `Poison`）

### 第三十三批 — 耐久度变化落地
- [x] DuraChanged: 更新本地玩家 `Inventory` 和 `Equipment` 中对应 `unique_id` 物品的 `current_dura`

### 第三十四批 — 陷阱岩状态落地
- [x] 新增 `InTrapRock` 组件（`components/player.rs`）
- [x] TrapRockEntered: 通过 deferred `Option<bool>` 在循环后设置本地玩家 `InTrapRock.trapped`
- [x] PlayerControlSystem: 查询前读取 `InTrapRock`，在输入处理结束后清除 `move_to`/`movement_mode`/path
- [x] PlayerControlSystem: `sync_move_to_server` 网络同步跳过陷阱状态（`!in_trap_rock`）

### 第三十五批 — 基础属性 ECS 落地
- [x] 扩展 `CombatStats` 组件：新增 ac_min/max, mac_min/max, dc_min/max, mc_min/max, sc_min/max 字段
- [x] BaseStatsReceived: 通过 deferred `Option<Vec<i32>>` 在循环后更新本地玩家 `CombatStats`
- [x] 同步更新 `defense`/`magic_defense`（取 AC/MAC 上限作为代表值）

### 第三十六批 — 元素状态 ECS 落地
- [x] 新增 `ElementalState` 组件（`components/combat.rs`）
- [x] ElementalSet: 通过 deferred Vec 在循环后更新目标实体 `ElementalState`

### 第三十七批 — 攻击/宠物模式组件 + Hack 修复
- [x] 新增 `AttackMode` / `PetMode` 组件（`components/player.rs`）
- [x] 修复 `AttackModeChanged` 将模式存入 `CombatStats.level` 的临时 hack，改为使用 `AttackMode` 组件
- [x] `PetModeChanged` 同样使用 `PetMode` 组件（之前仅 trace，无 ECS 落地）

### 第三十八批 — 对象装饰 ECS 落地
- [x] 新增 `ObjectDeco` 组件（`components/render.rs`）
- [x] ObjectDecoReceived: 通过 deferred Vec 在循环后更新/移除目标实体 `ObjectDeco`
- [x] 支持 `remove=true` 时使用 `hecs::World::remove_one` 精确移除组件

---

## 六、关键代码模式备忘

1. **Capture-count-before-move**: 先 `let count = packet.field.len()` 再 move packet 到 NetworkEvent
2. **Deferred Vec**: network_apply_system 中收集到 Vec，循环后再 apply（避免 borrow 冲突）
3. **find_entity_by_object_id**: 查询 ECS 实体时需要 ctx 的可变借用，不能在 events 迭代器内调用
4. **Copy type 无需解引用**: UiCommand match arms 中 Copy 类型按值绑定，不需要 `*`
5. **GuildStatus 空名 = 退会**: GuildStatus handler 检查 `packet.guild_name.is_empty()` 来区分加入/退会
6. ***2 后缀变体**: 当裸变体被升级版本替代时，用后缀 `2` 或 `Event` 命名，确认后再删除裸变体
