# network - 网络通信模块

**对应C#代码**: `Client/MirNetwork/`  
**文件数**: 6  
**代码行数**: 6,022  
**状态**: 🚧 核心完成，服务器通信待完善

---

## 📚 目录

1. [模块概述](#-模块概述)
2. [架构设计](#-架构设计)
3. [核心组件](#-核心组件)
4. [协议系统](#-协议系统)
5. [使用指南](#-使用指南)
6. [开发状态](#-开发状态)

---

## 📖 模块概述

`network` 模块负责客户端与服务器之间的网络通信，包括：

- **TCP连接管理**: 建立、维护、断开连接
- **协议解析**: 数据包的序列化和反序列化
- **异步通信**: 使用 tokio 异步运行时
- **事件分发**: 将网络事件分发给游戏逻辑
- **命令模式**: 封装网络操作为命令

### 技术栈

- **tokio**: 异步运行时
- **tokio::net::TcpStream**: TCP连接
- **tokio::sync::mpsc**: 消息传递
- **serde**: 数据序列化
- **mir2_shared**: 共享协议定义

---

## 🏗 架构设计

### 模块结构

```
network/
├── mod.rs                  # 模块入口
├── network.rs              # 网络栈 (TCP连接管理)
├── protocol.rs             # 协议解析 (数据包序列化)
├── game_client.rs          # 游戏客户端 (高层API)
├── network_manager.rs      # 网络管理器 (异步任务)
└── network_command.rs      # 网络命令 (命令模式)
```

### 架构层次

```
游戏逻辑层 (ECS Systems)
        ↓
   GameClient (高层API)
        ↓
   NetworkCommand (命令模式)
        ↓
   NetworkManager (异步管理)
        ↓
   NetworkStack (TCP连接)
        ↓
   Protocol (协议解析)
        ↓
   TcpStream (底层TCP)
```

### 数据流向

#### 发送数据

```
Game Logic
    ↓ (调用 GameClient 方法)
GameClient::send_xxx()
    ↓ (创建 NetworkCommand)
NetworkCommand::SendPacket
    ↓ (通过 mpsc channel)
NetworkManager
    ↓ (序列化数据包)
Protocol::serialize()
    ↓ (写入 TCP)
TcpStream::write()
    ↓
服务器
```

#### 接收数据

```
服务器
    ↓
TcpStream::read()
    ↓ (反序列化数据包)
Protocol::deserialize()
    ↓ (分发到 NetworkManager)
NetworkManager
    ↓ (通过 mpsc channel)
NetworkEvent
    ↓ (ECS System 处理)
Game Logic
```

---

## 🔧 核心组件

### 1. NetworkStack (network.rs)

**职责**: TCP连接的底层管理

#### 核心结构

```rust
pub struct NetworkStack {
    stream: Option<TcpStream>,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    connected: bool,
}

pub enum NetworkEvent {
    Connected,
    Disconnected,
    PacketReceived(ClientPacket),
    Error(String),
}
```

#### 主要方法

```rust
impl NetworkStack {
    /// 创建新的网络栈
    pub fn new() -> Self;
    
    /// 连接到服务器
    pub async fn connect(&mut self, addr: &str) -> Result<()>;
    
    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()>;
    
    /// 发送数据包
    pub async fn send_packet(&mut self, packet: &ServerPacket) -> Result<()>;
    
    /// 接收数据包
    pub async fn receive_packet(&mut self) -> Result<Option<ClientPacket>>;
    
    /// 检查连接状态
    pub fn is_connected(&self) -> bool;
}
```

#### 特性

- ✅ 异步TCP连接
- ✅ 自动重连
- ✅ 读写缓冲区
- ✅ 连接状态管理
- ✅ 错误处理
- 🚧 心跳机制
- 🚧 超时处理

### 2. Protocol (protocol.rs)

**职责**: 数据包的序列化和反序列化

#### 数据包格式

```
+--------+--------+----------+
| Length | Type   | Payload  |
| 4 bytes| 2 bytes| N bytes  |
+--------+--------+----------+
```

#### 核心方法

```rust
/// 序列化数据包
pub fn serialize_packet(packet: &ServerPacket) -> Result<Vec<u8>>;

/// 反序列化数据包
pub fn deserialize_packet(data: &[u8]) -> Result<ClientPacket>;

/// 数据包分发
pub fn dispatch_packet(
    packet: ClientPacket,
    game_client: &Arc<Mutex<GameClient>>
) -> Result<()>;
```

#### 支持的数据包类型

**客户端 → 服务器 (ServerPacket)**:

```rust
pub enum ServerPacket {
    // 连接相关
    ClientVersion { version: String },
    Disconnect,
    KeepAlive { time: u64 },
    
    // 登录相关
    NewAccount { account_id: String, password: String },
    ChangePassword { account_id: String, current: String, new: String },
    Login { account_id: String, password: String },
    NewCharacter { name: String, gender: MirGender, class: MirClass },
    DeleteCharacter { character_index: i32 },
    StartGame { character_index: i32 },
    
    // 游戏内相关
    LogOut,
    Turn { direction: MirDirection },
    Walk { direction: MirDirection },
    Run { direction: MirDirection },
    Chat { message: String },
    MoveItem { grid: MirGridType, from: i32, to: i32 },
    StoreItem { from: i32, to: i32 },
    TakeBackItem { from: i32, to: i32 },
    MergeItem { from_grid: MirGridType, to_grid: MirGridType, from_slot: u64, to_slot: u64 },
    EquipItem { grid: MirGridType, unique_id: u64, to: i32 },
    RemoveItem { grid: MirGridType, unique_id: u64, to: i32 },
    RemoveSlotItem { grid: MirGridType, unique_id: u64, to: i32, from_grid: MirGridType, from_slot: u64 },
    SplitItem { grid: MirGridType, unique_id: u64, count: u32 },
    UseItem { unique_id: u64 },
    DropItem { unique_id: u64, count: u32 },
    DepositRefineItem { from: i32, to: i32 },
    RetrieveRefineItem { from: i32, to: i32 },
    RefineCancel,
    RefineItem { unique_id: u64 },
    CheckRefine { unique_id: u64 },
    ReplaceWedRing { unique_id: u64 },
    DepositTradeItem { from: i32, to: i32 },
    RetrieveTradeItem { from: i32, to: i32 },
    PickUp,
    Inspect { object_id: u32 },
    ChangeAMode { mode: AttackMode },
    ChangePMode { mode: PetMode },
    ChangeTrade { allow_trade: bool },
    Attack { direction: MirDirection, spell: Spell },
    RangeAttack { direction: MirDirection, location: Point, target_id: u32 },
    Harvest { direction: MirDirection },
    CallNPC { object_id: u32, key: String },
    TalkMonsterNPC { key: String },
    BuyItem { item_index: u32, count: u32, panel_type: PanelType },
    SellItem { unique_id: u64, count: u32 },
    RepairItem { unique_id: u64 },
    BuyItemBack { unique_id: u64, count: u32 },
    SRepairItem { unique_id: u64 },
    MagicKey { spell: Spell, key: u8 },
    Magic { spell: Spell, direction: MirDirection, location: Point, target_id: u32 },
    SwitchGroup { allow_group: bool },
    AddMember { name: String },
    DelMember { name: String },
    GroupInvite { accept_invite: bool },
    TownRevive,
    SpellToggle { spell: Spell, can_use: bool },
    ConsignItem { unique_id: u64, price: u64, item_type: ItemType },
    MarketSearch { match_name: String },
    MarketRefresh,
    MarketPage { page: i32 },
    MarketBuy { auction_id: u64 },
    MarketGetBack { auction_id: u64 },
    RequestUserName { user_id: u32 },
    RequestChatItem { chat_item_id: u64 },
    EditGuildMember { change_type: String, name: String, rank_name: String, rank_index: u8 },
    EditGuildNotice { notice: Vec<String> },
    GuildInvite { accept_invite: bool },
    GuildNameReturn { name: String },
    RequestGuildInfo { guild_index: i32 },
    GuildStorageGoldChange { type_: u8, amount: u32 },
    GuildStorageItemChange { type_: u8, from: i32, to: i32 },
    GuildWarReturn { guild_name: String },
    MarriageRequest,
    MarriageReply { accept_invite: bool },
    ChangeMarriage,
    DivorceRequest,
    DivorceReply { accept_invite: bool },
    AddMentor { name: String },
    MentorReply { accept_invite: bool },
    AllowMentor,
    CancelMentor { name: String },
    TradeRequest,
    TradeReply { accept_invite: bool },
    TradeGold { amount: u32 },
    TradeConfirm,
    TradeCancel,
    EquipSlotItem { grid: MirGridType, unique_id: u64, to: i32, from_grid: MirGridType, from_slot: u64 },
    FishingCast { cast_out: bool },
    FishingChangeAutocast { auto_cast: bool },
    AcceptQuest { npc_index: u32, quest_index: i32 },
    FinishQuest { quest_index: i32, selected_item_index: i32 },
    AbandonQuest { quest_index: i32 },
    ShareQuest { quest_index: i32 },
    AcceptReincarnation,
    CancelReincarnation,
    CombineItem { grid: MirGridType, id_list: Vec<u64> },
    SetConcentration { object_id: u32 },
    AwakeningNeedMaterials { unique_id: u64, type_: AwakeType },
    AwakeningLockedItem { unique_id: u64, index: u32 },
    Awakening { unique_id: u64, type_: AwakeType },
    DisassembleItem { unique_id: u64 },
    DowngradeAwakening { unique_id: u64 },
    ResetAddedItem { unique_id: u64 },
    SendMail { name: String, message: String, gold: u32, items: Vec<u64> },
    ReadMail { mail_id: u64 },
    CollectParcel { mail_id: u64 },
    DeleteMail { mail_id: u64 },
    LockHeroSpawn { lock_: bool },
    SetAutoPotValue { stat: Stat, percent: u16, item_index: u32 },
    SetAutoPotItem { stat: Stat, item_index: u32, enabled: bool },
    SetHeroBehaviour { behaviour: HeroBehaviour },
    ChangeHero,
    TameHero { npc_id: u32 },
    RequestIntelligentCreatureUpdates,
}
```

**服务器 → 客户端 (ClientPacket)**:

```rust
pub enum ClientPacket {
    // 连接相关
    Connected,
    ClientVersion { result: u8, version: String },
    Disconnect { reason: u8 },
    KeepAlive { time: u64 },
    
    // 登录相关
    NewAccount { result: u8 },
    ChangePassword { result: u8 },
    ChangePasswordBanned { reason: String, expires_at: String },
    Login { result: u8, banned_reason: String, banned_expires: String },
    LoginBanned { reason: String, expires_at: String },
    LoginSuccess { characters: Vec<SelectInfo> },
    NewCharacter { result: u8 },
    NewCharacterSuccess { char_info: SelectInfo },
    DeleteCharacter { result: u8 },
    DeleteCharacterSuccess { character_index: i32 },
    StartGame { result: u8, resolution: i32 },
    StartGameSuccess { character: CharacterInfo },
    StartGameBanned { reason: String, expires_at: String },
    StartGameDelay { milliseconds: u64 },
    
    // 地图相关
    MapInformation { file_name: String, title: String, mini_map: u16, big_map: u16, lights: LightSetting, map_dark_light: u8, music: u16 },
    NewMapInfo { map_index: i32, file_name: String, title: String, mini_map: u16, big_map: u16, lights: LightSetting, light: u8, fire: u8, lightning: u8, map_dark_light: u8, music: u16 },
    WorldMapSetup { setup: Vec<WorldMapSetup> },
    SearchMapResult { map_index: i32, coordinate: Point },
    
    // 对象相关
    ObjectPlayer { object_id: u32, name: String, name_colour: Color, class: MirClass, gender: MirGender, level: u16, location: Point, direction: MirDirection, hair: u8, light: u8, weapon: i32, armor: i32, poison: PoisonType, dead: bool, hidden: bool, effect: SpellEffect, wing_effect: u8, extra: bool, mount_type: i16, fishing: bool, transform: i32, element_orc: i32, can_attack: bool },
    ObjectRemove { object_id: u32 },
    ObjectTurn { object_id: u32, location: Point, direction: MirDirection },
    ObjectWalk { object_id: u32, location: Point, direction: MirDirection },
    ObjectRun { object_id: u32, location: Point, direction: MirDirection },
    Chat { message: String, chat_type: ChatType },
    ObjectChat { object_id: u32, text: String, chat_type: ChatType },
    NewItemInfo { item_info: ItemInfo },
    MoveItem { grid: MirGridType, from: i32, to: i32, success: bool },
    EquipItem { grid: MirGridType, unique_id: u64, to: i32, success: bool },
    MergeItem { from_grid: MirGridType, to_grid: MirGridType, from_slot: u64, to_slot: u64, success: bool },
    RemoveItem { grid: MirGridType, unique_id: u64, to: i32, success: bool },
    RemoveSlotItem { grid: MirGridType, unique_id: u64, to: i32, from_grid: MirGridType, from_slot: u64, success: bool },
    TakeBackItem { from: i32, to: i32, success: bool },
    StoreItem { from: i32, to: i32, success: bool },
    SplitItem { grid: MirGridType, unique_id: u64, count: u32, item: UserItem },
    SplitItem1 { grid: MirGridType, unique_id: u64, count: u32, grid_to: MirGridType, item: UserItem, success: bool },
    UseItem { unique_id: u64 },
    DropItem { unique_id: u64, count: u32, success: bool },
    PlayerUpdate { object_id: u32, light: u8, weapon: i32, armor: i32, wing_effect: u8 },
    PlayerInspect { name: String, equipment: Vec<Option<UserItem>>, class: MirClass, gender: MirGender, hair: u8, level: u16 },
    LogOutSuccess { characters: Vec<SelectInfo> },
    LogOutFailed,
    TimeOfDay { lights: LightSetting },
    ChangeAMode { mode: AttackMode },
    ChangePMode { mode: PetMode },
    ObjectAttack { object_id: u32, location: Point, direction: MirDirection, spell: Spell, level: u8, type_: u8 },
    Struck { attacker_id: u32 },
    ObjectStruck { object_id: u32, attacker_id: u32, location: Point, direction: MirDirection },
    DamageIndicator { damage: i32, damage_type: DamageType, object_id: u32 },
    DuraChanged { unique_id: u64, current_dura: u16 },
    HealthChanged { hp: u32, mp: u32 },
    DeleteItem { unique_id: u64, count: u32 },
    Death { location: Point, direction: MirDirection },
    ObjectDied { object_id: u32, location: Point, direction: MirDirection, type_: u8 },
    ColourChanged { feature_colour: Color, cap_colour: Color },
    ObjectColourChanged { object_id: u32, feature_colour: Color, cap_colour: Color },
    ObjectGuildNameChanged { object_id: u32, guild_name: String },
    GainExperience { amount: u32 },
    LevelChanged { level: u16, experience: u64, max_experience: u64 },
    ObjectLeveled { object_id: u32 },
    ObjectHarvest { object_id: u32, location: Point, direction: MirDirection },
    ObjectHarvested { object_id: u32, location: Point, direction: MirDirection },
    ObjectNPC { object_id: u32, name: String, name_colour: Color, image: u16, color: u8, location: Point, direction: MirDirection, quest_ids: Vec<i32> },
    NPCUpdate { npc_id: u32, quest_ids: Vec<i32> },
    NPCResponse { page: Vec<String> },
    ObjectHide { object_id: u32, hide: bool },
    ObjectShow { object_id: u32 },
    Poisoned { poison: PoisonType },
    ObjectPoisoned { object_id: u32, poison: PoisonType },
    MapChanged { file_name: String, title: String, mini_map: u16, big_map: u16, lights: LightSetting, location: Point, direction: MirDirection, map_dark_light: u8, music: u16 },
    ObjectTeleportOut { object_id: u32, type_: u8 },
    ObjectTeleportIn { object_id: u32, type_: u8 },
    TeleportIn,
    NPCGoods { goods_list: Vec<UserItem>, type_: PanelType },
    NPCSell,
    NPCRepair { rate: f32 },
    NPCSRepair { rate: f32 },
    NPCStorage,
    SellItem { unique_id: u64, success: bool },
    CraftItem { unique_id: u64, success: bool },
    RepairItem { unique_id: u64, success: bool },
    ItemRepaired { unique_id: u64, max_dura: u16, current_dura: u16 },
    NewMagic { magic: ClientMagic },
    RemoveMagic { magic_index: i32 },
    MagicLeveled { spell: Spell, level: u8, experience: u16, max_experience: u16 },
    Magic { spell: Spell, target_id: u32, target_location: Point, cast: bool, level: u8 },
    MagicDelay { spell: Spell, delay: u64, cooldown: u64 },
    MagicCast { spell: Spell },
    ObjectMagic { object_id: u32, location: Point, direction: MirDirection, spell: Spell, target_id: u32, target_location: Point, cast: bool, level: u8 },
    ObjectEffect { object_id: u32, effect: SpellEffect },
    RangeAttack { target_id: u32, target_location: Point, spell: Spell, level: u8 },
    Pushed { location: Point, direction: MirDirection },
    ObjectPushed { object_id: u32, location: Point, direction: MirDirection },
    ObjectName { object_id: u32, name: String },
    UserStorage { storage: Vec<UserItem> },
    SwitchGroup { allow_group: bool },
    DeleteGroup,
    DeleteMember { name: String },
    GroupInvite { name: String },
    AddMember { name: String },
    Revived,
    ObjectRevived { object_id: u32, effect: bool },
    SpellToggle { spell: Spell, can_use: bool },
    ObjectHealth { object_id: u32, percent: u8, expire: u8 },
    MapEffect { location: Point, effect: SpellEffect, value: u16 },
    ObjectRangeAttack { object_id: u32, location: Point, direction: MirDirection, target_id: u32, target_location: Point, spell: Spell, level: u8 },
    AddBuff { buff: Buff },
    RemoveBuff { buff_type: BuffType },
    ObjectHidden { object_id: u32, hidden: bool },
    RefreshItem { item: UserItem },
    ObjectSpell { object_id: u32, location: Point, spell: Spell, direction: MirDirection, target_id: u32, target_location: Point },
    UserDash { location: Point, direction: MirDirection },
    ObjectDash { object_id: u32, location: Point, direction: MirDirection },
    UserDashFail { location: Point },
    ObjectDashFail { object_id: u32, location: Point, direction: MirDirection },
    NPCConsign { npc_rate: f32, user_rate: f32 },
    NPCMarket { listings: Vec<ClientAuction>, pages: i32, user_mode: bool },
    NPCMarketPage { listings: Vec<ClientAuction>, page: i32, page_count: i32 },
    ConsignItem { unique_id: u64, success: bool },
    MarketFail { reason: u8 },
    MarketSuccess { message: String },
    ObjectSitDown { object_id: u32, location: Point, direction: MirDirection },
    InTrapRock { trapped: bool },
    BaseStatsInfo { stats: Vec<BaseStats> },
    UserName { id: u32, name: String },
    ChatItemStats { chat_item_id: u64, item_stats: String },
    GuildNameRequest,
    GuildNoticeChange { notice: Vec<String>, listing: Vec<GuildRank> },
    GuildMemberChange { name: String, rank_name: String, online: bool },
    GuildStatus { guild_name: String, guild_rank_name: String, member_count: i32, max_members: i32, storage_gold: u64, storage_items: Vec<UserItem>, buff: u32 },
    GuildInvite { name: String },
    GuildExpGain { amount: u32 },
    GuildNameReturn { name: String },
    GuildStorageGoldChange { type_: u8, amount: u32 },
    GuildStorageItemChange { type_: u8, user: i32, from: i32, to: i32 },
    GuildStorageList { items: Vec<UserItem> },
    GuildRequestWar { guild_name: String },
    DefaultNPC { object_id: u32 },
    NPCUpdate { npc_id: u32, quest_ids: Vec<i32> },
    NPCImageUpdate { npc_id: u32, image: u16 },
    MarriageRequest { name: String },
    DivorceRequest { name: String },
    MentorRequest { name: String },
    TradeRequest { name: String },
    TradeAccept { name: String },
    TradeGold { amount: u32 },
    TradeItem { item: UserItem },
    TradeConfirm,
    TradeCancel { reason: u8 },
    MountUpdate { object_id: u32, mount_type: i16 },
    TransformUpdate { object_id: u32, transform_type: i32 },
    EquipSlotItem { grid: MirGridType, unique_id: u64, to: i32, from_grid: MirGridType, from_slot: u64, success: bool },
    FishingUpdate { object_id: u32, fishing: bool },
    ChangeQuest { quest: ClientQuestProgress },
    CompleteQuest { quest_index: i32 },
    ShareQuest { quest_index: i32, share_id: i32 },
    NewQuestInfo { quest_info: QuestInfo },
    GainedQuestItem { item: UserItem },
    DeleteQuestItem { unique_id: u64, count: u32 },
    CancelReincarnation,
    RequestReincarnation,
    UserBackStep { location: Point, direction: MirDirection },
    ObjectBackStep { object_id: u32, location: Point, direction: MirDirection },
    UserDashAttack { location: Point, direction: MirDirection },
    ObjectDashAttack { object_id: u32, location: Point, direction: MirDirection, target_id: u32, target_location: Point },
    UserAttackMove { location: Point, direction: MirDirection },
    CombineItem { grid: MirGridType, success: bool },
    ItemUpgraded { item: UserItem },
    SetConcentration { object_id: u32, interrupted: bool, concentration: i32 },
    SetElemental { value: u64, state: bool, time: i64 },
    SetDelayedExplosion { object_id: u32, spell_id: Spell },
    ObjectDeco { object_id: u32, deco: i32 },
    ObjectSneaking { object_id: u32, sneaking: bool },
    ObjectLevelStream { object_id: u32, level: i32, exp: i64 },
    DataObjectPlayer { data: DataObjectPlayer },
    DataObjectHero { data: DataObjectHero },
    DataObjectMonster { data: DataObjectMonster },
    DataObjectItem { data: DataObjectItem },
    UpdateListeningPlayerList { list: Vec<i32> },
    NPCAwakening,
    NPCDisassemble { npc_rate: f32, user_rate: f32 },
    NPCDowngrade { npc_rate: f32, user_rate: f32 },
    NPCReset,
    AwakeningNeedMaterials { materials: Vec<UserItem> },
    AwakeningLockedItem { unique_id: u64, locked: bool },
    Awakening { unique_id: u64, awake: Awake, remove_need_unlock: bool, success: bool },
    ReceiveMail { mail: ClientMail, count: i32 },
    MailLockedItem { unique_id: u64, locked: bool },
    MailSendRequest { name: String },
    MailSent { mail: ClientMail },
    ParcelCollected { mail_id: u64, success: bool },
    MailDeleted { mail_id: u64, success: bool },
    NPCMail,
    ChangeHero { success: bool },
    HeroUpdate { hero: ClientHeroInformation },
    NewIntelligentCreature { creature: ClientIntelligentCreature },
    UpdateIntelligentCreature { creature: ClientIntelligentCreature },
    IntelligentCreatureEnableRename { creature_id: u64 },
    IntelligentCreaturePickup { object_id: u32, target_id: u32, location: Point },
    NPCPearlGoods { goods_list: Vec<ItemInfo> },
}
```

#### 特性

- ✅ 完整的协议定义
- ✅ 序列化/反序列化
- ✅ 数据包校验
- ✅ 错误处理
- 🚧 数据包加密
- 🚧 数据包压缩

### 3. GameClient (game_client.rs)

**职责**: 游戏客户端高层API，封装网络操作

#### 核心结构

```rust
pub struct GameClient {
    network_stack: Arc<Mutex<NetworkStack>>,
    event_sender: mpsc::Sender<GameEvent>,
    object_cache: HashMap<u32, GameObject>,
    player_id: Option<u32>,
    player_name: Option<String>,
}

pub type SharedGameClient = Arc<Mutex<GameClient>>;

pub enum GameEvent {
    Connected,
    Disconnected,
    LoginSuccess { characters: Vec<SelectInfo> },
    EnterGame { character: CharacterInfo },
    ObjectSpawned { object: GameObject },
    ObjectRemoved { object_id: u32 },
    ObjectMoved { object_id: u32, location: Point },
    ChatMessage { message: String, chat_type: ChatType },
    // ... 更多事件
}

pub struct GameObject {
    pub id: u32,
    pub name: String,
    pub object_type: ObjectType,
    pub location: Point,
    pub direction: MirDirection,
    // ... 更多属性
}
```

#### 主要方法

```rust
impl GameClient {
    /// 创建新的客户端
    pub fn new(event_sender: mpsc::Sender<GameEvent>) -> Self;
    
    /// 连接到服务器
    pub async fn connect(&mut self, addr: &str) -> Result<()>;
    
    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()>;
    
    // --- 登录相关 ---
    
    /// 登录
    pub async fn login(
        &mut self,
        account: &str,
        password: &str
    ) -> Result<()>;
    
    /// 创建新账号
    pub async fn create_account(
        &mut self,
        account: &str,
        password: &str
    ) -> Result<()>;
    
    /// 创建角色
    pub async fn create_character(
        &mut self,
        name: &str,
        gender: MirGender,
        class: MirClass
    ) -> Result<()>;
    
    /// 删除角色
    pub async fn delete_character(&mut self, index: i32) -> Result<()>;
    
    /// 开始游戏
    pub async fn start_game(&mut self, character_index: i32) -> Result<()>;
    
    // --- 移动相关 ---
    
    /// 行走
    pub async fn walk(&mut self, direction: MirDirection) -> Result<()>;
    
    /// 奔跑
    pub async fn run(&mut self, direction: MirDirection) -> Result<()>;
    
    /// 转向
    pub async fn turn(&mut self, direction: MirDirection) -> Result<()>;
    
    // --- 战斗相关 ---
    
    /// 攻击
    pub async fn attack(
        &mut self,
        direction: MirDirection,
        spell: Spell
    ) -> Result<()>;
    
    /// 远程攻击
    pub async fn range_attack(
        &mut self,
        direction: MirDirection,
        location: Point,
        target_id: u32
    ) -> Result<()>;
    
    /// 使用魔法
    pub async fn cast_magic(
        &mut self,
        spell: Spell,
        direction: MirDirection,
        location: Point,
        target_id: u32
    ) -> Result<()>;
    
    // --- 物品相关 ---
    
    /// 拾取物品
    pub async fn pickup(&mut self) -> Result<()>;
    
    /// 移动物品
    pub async fn move_item(
        &mut self,
        grid: MirGridType,
        from: i32,
        to: i32
    ) -> Result<()>;
    
    /// 使用物品
    pub async fn use_item(&mut self, unique_id: u64) -> Result<()>;
    
    /// 丢弃物品
    pub async fn drop_item(
        &mut self,
        unique_id: u64,
        count: u32
    ) -> Result<()>;
    
    /// 装备物品
    pub async fn equip_item(
        &mut self,
        grid: MirGridType,
        unique_id: u64,
        to: i32
    ) -> Result<()>;
    
    /// 卸下物品
    pub async fn remove_item(
        &mut self,
        grid: MirGridType,
        unique_id: u64,
        to: i32
    ) -> Result<()>;
    
    // --- 社交相关 ---
    
    /// 发送聊天消息
    pub async fn send_chat(&mut self, message: &str) -> Result<()>;
    
    /// 检查玩家
    pub async fn inspect(&mut self, object_id: u32) -> Result<()>;
    
    // --- NPC相关 ---
    
    /// 调用NPC
    pub async fn call_npc(
        &mut self,
        object_id: u32,
        key: &str
    ) -> Result<()>;
    
    /// 购买物品
    pub async fn buy_item(
        &mut self,
        item_index: u32,
        count: u32,
        panel_type: PanelType
    ) -> Result<()>;
    
    /// 出售物品
    pub async fn sell_item(
        &mut self,
        unique_id: u64,
        count: u32
    ) -> Result<()>;
    
    // --- 查询方法 ---
    
    /// 获取对象
    pub fn get_object(&self, object_id: u32) -> Option<&GameObject>;
    
    /// 获取所有对象
    pub fn get_all_objects(&self) -> Vec<&GameObject>;
    
    /// 获取玩家ID
    pub fn player_id(&self) -> Option<u32>;
}
```

#### 辅助函数

```rust
/// 创建共享客户端
pub fn new_shared_client(
    event_sender: mpsc::Sender<GameEvent>
) -> SharedGameClient {
    Arc::new(Mutex::new(GameClient::new(event_sender)))
}
```

### 4. NetworkManager (network_manager.rs)

**职责**: 管理异步网络任务

#### 核心结构

```rust
pub struct NetworkManager {
    command_receiver: mpsc::Receiver<NetworkCommand>,
    game_client: SharedGameClient,
}
```

#### 主要方法

```rust
impl NetworkManager {
    /// 创建网络管理器
    pub fn new(
        command_receiver: mpsc::Receiver<NetworkCommand>,
        game_client: SharedGameClient,
    ) -> Self;
    
    /// 运行网络任务（异步）
    pub async fn run(&mut self);
}

/// 网络任务入口点
pub async fn network_task(
    mut command_receiver: mpsc::Receiver<NetworkCommand>,
    game_client: SharedGameClient,
) {
    let mut manager = NetworkManager::new(command_receiver, game_client);
    manager.run().await;
}
```

#### 工作流程

```rust
// 1. 接收命令
while let Some(command) = command_receiver.recv().await {
    match command {
        NetworkCommand::Connect { addr } => {
            // 连接到服务器
        }
        NetworkCommand::Disconnect => {
            // 断开连接
        }
        NetworkCommand::SendPacket { packet } => {
            // 发送数据包
        }
    }
}

// 2. 接收数据包
while let Ok(Some(packet)) = network_stack.receive_packet().await {
    // 分发数据包
    dispatch_packet(packet, &game_client)?;
}
```

### 5. NetworkCommand (network_command.rs)

**职责**: 网络命令定义（命令模式）

#### 命令定义

```rust
pub enum NetworkCommand {
    /// 连接到服务器
    Connect {
        addr: String,
    },
    
    /// 断开连接
    Disconnect,
    
    /// 发送数据包
    SendPacket {
        packet: ServerPacket,
    },
    
    /// 心跳
    KeepAlive {
        time: u64,
    },
}
```

#### 特性

- ✅ 类型安全
- ✅ 易于扩展
- ✅ 解耦游戏逻辑和网络层
- ✅ 支持异步执行

---

## 🎮 协议系统

### 协议版本

```rust
pub const PROTOCOL_VERSION: &str = "1.0.0";
```

### 数据包结构

#### 通用格式

```
+--------+--------+----------+
| Length | Type   | Payload  |
| 4 bytes| 2 bytes| N bytes  |
+--------+--------+----------+

Length: 数据包总长度（包括Length字段本身）
Type: 数据包类型ID
Payload: 序列化的数据
```

#### 类型ID分配

| 范围 | 用途 |
|------|------|
| 0-999 | 连接和认证 |
| 1000-1999 | 角色管理 |
| 2000-2999 | 移动和位置 |
| 3000-3999 | 战斗和技能 |
| 4000-4999 | 物品和装备 |
| 5000-5999 | 社交和聊天 |
| 6000-6999 | NPC和任务 |
| 7000-7999 | 公会和组队 |
| 8000-8999 | 交易和市场 |
| 9000-9999 | 其他功能 |

### 数据序列化

使用 `serde` 和自定义二进制格式：

```rust
// 序列化
let packet = ServerPacket::Walk { direction: MirDirection::Up };
let bytes = serialize_packet(&packet)?;

// 反序列化
let packet = deserialize_packet(&bytes)?;
```

---

## 📖 使用指南

### 初始化网络系统

```rust
use crate::network::*;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // 1. 创建事件通道
    let (event_tx, mut event_rx) = mpsc::channel::<GameEvent>(100);
    
    // 2. 创建命令通道
    let (cmd_tx, cmd_rx) = mpsc::channel::<NetworkCommand>(100);
    
    // 3. 创建游戏客户端
    let game_client = new_shared_client(event_tx);
    
    // 4. 启动网络任务
    tokio::spawn(network_task(cmd_rx, game_client.clone()));
    
    // 5. 连接到服务器
    let client = game_client.lock().await;
    client.connect("127.0.0.1:7000").await.unwrap();
    drop(client);
    
    // 6. 处理游戏事件
    while let Some(event) = event_rx.recv().await {
        match event {
            GameEvent::Connected => {
                println!("已连接到服务器");
            }
            GameEvent::LoginSuccess { characters } => {
                println!("登录成功，角色列表: {:?}", characters);
            }
            // ... 处理其他事件
        }
    }
}
```

### 登录流程

```rust
// 1. 连接
client.connect("127.0.0.1:7000").await?;

// 2. 登录
client.login("username", "password").await?;

// 3. 等待登录成功事件
// GameEvent::LoginSuccess { characters }

// 4. 选择角色开始游戏
client.start_game(0).await?;

// 5. 等待进入游戏事件
// GameEvent::EnterGame { character }
```

### 移动玩家

```rust
// 行走
client.walk(MirDirection::Up).await?;

// 奔跑
client.run(MirDirection::Down).await?;

// 转向
client.turn(MirDirection::Left).await?;
```

### 战斗

```rust
// 普通攻击
client.attack(MirDirection::Right, Spell::None).await?;

// 远程攻击
client.range_attack(
    MirDirection::Up,
    Point::new(100, 100),
    target_id
).await?;

// 施放魔法
client.cast_magic(
    Spell::FireBall,
    MirDirection::Up,
    Point::new(100, 100),
    target_id
).await?;
```

### 物品操作

```rust
// 拾取
client.pickup().await?;

// 移动物品
client.move_item(
    MirGridType::Inventory,
    0,
    5
).await?;

// 使用物品
client.use_item(item_unique_id).await?;

// 装备物品
client.equip_item(
    MirGridType::Inventory,
    item_unique_id,
    0  // 装备槽位
).await?;

// 丢弃物品
client.drop_item(item_unique_id, 1).await?;
```

### 聊天

```rust
// 发送聊天消息
client.send_chat("你好!").await?;

// 接收聊天消息
// GameEvent::ChatMessage { message, chat_type }
```

### 对象管理

```rust
// 获取所有对象
let objects = client.get_all_objects();
for obj in objects {
    println!("对象: {} at ({}, {})", obj.name, obj.location.x, obj.location.y);
}

// 获取特定对象
if let Some(obj) = client.get_object(object_id) {
    println!("找到对象: {}", obj.name);
}
```

---

## 📊 开发状态

### 完成度统计

| 功能模块 | 完成度 | 说明 |
|---------|--------|------|
| **NetworkStack** | 85% | TCP连接完成，心跳和超时待完善 |
| **Protocol** | 95% | 协议定义完成，加密压缩待实现 |
| **GameClient** | 80% | 主要API完成，部分功能待实现 |
| **NetworkManager** | 90% | 异步管理完成，错误恢复待优化 |
| **NetworkCommand** | 100% | 命令定义完成 |

### 已实现功能清单

#### ✅ 连接管理

- [x] TCP连接
- [x] 异步IO
- [x] 连接状态管理
- [x] 断线检测
- [x] 自动重连（基础）

#### ✅ 协议支持

- [x] 登录/注册
- [x] 角色创建/删除/选择
- [x] 移动（行走/奔跑/转向）
- [x] 攻击（普通/远程/魔法）
- [x] 物品（拾取/移动/使用/装备）
- [x] 聊天
- [x] NPC交互
- [x] 对象同步

#### ✅ 客户端功能

- [x] 游戏事件系统
- [x] 对象缓存
- [x] 命令队列
- [x] 异步API

### 未实现功能清单

#### ⏳ 连接功能

- [ ] **心跳机制**: 保持连接活跃
- [ ] **超时处理**: 连接超时检测
- [ ] **智能重连**: 指数退避重连
- [ ] **连接池**: 多连接管理

#### ⏳ 协议功能

- [ ] **数据包加密**: AES/RSA加密
- [ ] **数据包压缩**: zlib/lz4压缩
- [ ] **数据包签名**: 防篡改
- [ ] **版本协商**: 协议版本兼容

#### ⏳ 高级功能

- [ ] **流量控制**: 带宽限制
- [ ] **优先级队列**: 重要数据包优先
- [ ] **批量发送**: 减少系统调用
- [ ] **统计信息**: 网络性能监控

#### ⏳ 完整协议支持

- [ ] **组队系统**: 组队协议
- [ ] **公会系统**: 公会协议
- [ ] **交易系统**: 交易协议
- [ ] **邮件系统**: 邮件协议
- [ ] **好友系统**: 好友协议

---

## 🚀 未来规划

### 短期目标 (1-2周)

1. **心跳和超时** 🔴 高优先级
   - 实现心跳包定时发送
   - 添加超时检测
   - 自动断线重连

2. **错误恢复** 🔴 高优先级
   - 网络错误处理
   - 数据包错误恢复
   - 连接状态恢复

3. **性能优化** 🟡 中优先级
   - 减少内存分配
   - 批量发送数据包
   - 优化序列化

### 中期目标 (3-4周)

4. **加密和压缩** 🟡 中优先级
   - 实现AES加密
   - 实现zlib压缩
   - 协议版本协商

5. **完整协议** 🟡 中优先级
   - 组队协议
   - 公会协议
   - 交易协议
   - 邮件协议

6. **网络监控** 🟢 低优先级
   - 统计信息收集
   - 性能监控
   - 日志记录

### 长期目标 (1-2月)

7. **高级功能**
   - 流量控制
   - 优先级队列
   - 连接池
   - WebSocket支持

8. **安全性**
   - 反作弊
   - 数据包签名
   - DDoS防护

---

## 🐛 已知问题

### 高优先级

- [ ] 快速断开重连时偶尔卡住
- [ ] 大量数据包时延迟增加
- [ ] 连接断开时事件处理不完整

### 中优先级

- [ ] 数据包序列化性能有优化空间
- [ ] 错误消息不够详细
- [ ] 网络统计信息不完整

### 低优先级

- [ ] 日志输出格式不统一
- [ ] 部分错误类型没有细分
- [ ] 文档注释不够完整

---

## 📝 安全考虑

### 数据验证

```rust
// 总是验证来自服务器的数据
fn validate_packet(packet: &ClientPacket) -> Result<()> {
    match packet {
        ClientPacket::ObjectPlayer { level, .. } => {
            if *level > MAX_LEVEL {
                return Err(anyhow!("Invalid level"));
            }
        }
        // ... 其他验证
    }
    Ok(())
}
```

### 敏感信息

```rust
// 不要在日志中输出密码
tracing::info!("登录: account={}, password=***", account);

// 不要在内存中长时间保存密码
let password = get_password();
send_login(account, &password).await?;
drop(password);  // 立即释放
```

---

## 🔗 相关文档

### 内部文档

- **ECS系统**: `../ecs/systems/README.md` - 网络事件的处理
- **对象系统**: `../objects/README.md` - 对象数据同步
- **共享代码**: `mir2_shared` - 协议定义

### 外部资源

- **tokio文档**: https://tokio.rs/
- **serde文档**: https://serde.rs/

---

**文档版本**: v1.0  
**最后更新**: 2025-10-28  
**维护者**: Crystal Mir2 Team
