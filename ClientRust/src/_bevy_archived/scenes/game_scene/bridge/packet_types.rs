// 包类型定义 - 统一的服务器和客户端包枚举
// 
// 功能说明:
// 将 SharedRust 的所有包类型统一到枚举中,方便 Bevy Event 系统使用
// 
// 复用策略:
// 完全复用 SharedRust (mir2_shared) 的包定义,只是封装到枚举中
// SharedRust 项目应该都可以被复用

use bevy::prelude::*;

// ==================== 完全复用 mir2_shared 的包定义 ====================
// 重新导出常用的包类型供外部使用
pub use mir2_shared::packets::server::{
    // 对象管理
    ObjectPlayer, ObjectMonster, ObjectNpc, ObjectRemove, ObjectHero,
    
    // 对象行为
    ObjectTurn, ObjectWalk, ObjectRun, ObjectHarvest, ObjectHarvested,
    
    // 对象状态
    ObjectHealth, ObjectMana, ObjectHidden,
    
    // 玩家
    PlayerUpdate, PlayerInspect, LogOutSuccess, TimeOfDay,
    ChangeAMode, ChangePMode, ObjectName, UserStorage,
    
    // 地图
    MapEffect,
    
    // 聊天
    Chat,
    
    // 连接
    Connected, ClientVersion, Disconnect, KeepAlive,
    
    // 登录
    LoginSuccess, NewCharacter, NewCharacterSuccess,
    DeleteCharacter, DeleteCharacterSuccess, StartGame, StartGameBanned,
    StartGameDelay, NewAccount, ChangePassword, ChangePasswordBanned,
    
    // NPC
    NPCResponse, NPCGoods, NPCSell, NPCRepair, NPCSRepair, NPCRefine,
    NPCCheckRefine, NPCCollectRefine, NPCReplaceWedRing, NPCStorage,
    NPCUpdate, NPCImageUpdate, DefaultNPC,
    
    // 战斗
    DamageIndicator, GainedItem, GainExperience, LevelChanged,
    
    // Buff
    AddBuff, RemoveBuff,
    
    // 移动
    UserBackStep, ObjectBackStep, UserDashAttack,
    
    // 其他
    ObjectShow, ObjectHide,
};

pub use mir2_shared::packets::client::{
    // 连接
    ClientVersion as ClientVersionPacket, 
    Disconnect as ClientDisconnect,
    KeepAlive as ClientKeepAlive,
    
    // 登录
    NewAccount as ClientNewAccount, 
    ChangePassword as ClientChangePassword,
    Login, 
    NewCharacter as ClientNewCharacter, 
    DeleteCharacter as ClientDeleteCharacter,
    StartGame as ClientStartGame, 
    LogOut,
    
    // 移动
    Turn, Walk, Run,
    
    // 聊天
    Chat as ClientChat,
    
    // 战斗
    Attack, RangeAttack, Harvest,
    
    // NPC 交互
    CallNPC, BuyItem, SellItem, RepairItem, BuyItemBack, SRepairItem,
    
    // 物品操作
    MoveItem, StoreItem, TakeBackItem, MergeItem, EquipItem,
    RemoveItem, RemoveSlotItem, SplitItem, UseItem, DropItem, PickUp,
    DepositRefineItem, RetrieveRefineItem, RefineCancel, RefineItem,
    CheckRefine, ReplaceWedRing, DepositTradeItem, RetrieveTradeItem,
    
    // 技能
    MagicKey, Magic, SpellToggle,
    
    // 其他
    Inspect, 
    ChangeAMode as ClientChangeAMode, 
    ChangePMode as ClientChangePMode, 
    ChangeTrade,
    SwitchGroup, AddMember, DellMember, GroupInvite,
    TownRevive, ConsignItem, MarketSearch, MarketRefresh,
    MarketPage, MarketBuy, MarketGetBack, RequestUserName, RequestChatItem,
    EditGuildMember, EditGuildNotice, GuildInvite, RequestGuildInfo,
    GuildNameReturn, GuildStorageGoldChange, GuildStorageItemChange,
    GuildWarReturn, MarriageRequest as ClientMarriageRequest, 
    MarriageReply, ChangeMarriage,
    DivorceRequest as ClientDivorceRequest, DivorceReply, 
    AddMentor, MentorReply, AllowMentor,
    CancelMentor, TradeRequest as ClientTradeRequest, 
    TradeReply, TradeGold as ClientTradeGold, 
    TradeConfirm as ClientTradeConfirm,
    TradeCancel as ClientTradeCancel, EquipSlotItem, 
    FishingCast, FishingChangeAutocast,
    AcceptQuest, FinishQuest, AbandonQuest, ShareQuest,
};

/// 统一的服务器包枚举
/// 
/// 复用 mir2_shared 的所有包定义
/// 用于 Bevy Event 系统
#[derive(Event, Clone, Debug)]
pub enum ServerPacket {
    // ==================== 对象管理 ====================
    ObjectPlayer(ObjectPlayer),
    ObjectMonster(ObjectMonster),
    ObjectNpc(ObjectNpc),
    ObjectHero(ObjectHero),
    ObjectRemove(ObjectRemove),
    
    // ==================== 对象行为 ====================
    ObjectTurn(ObjectTurn),
    ObjectWalk(ObjectWalk),
    ObjectRun(ObjectRun),
    ObjectHarvest(ObjectHarvest),
    ObjectHarvested(ObjectHarvested),
    
    // ==================== 对象状态 ====================
    ObjectHealth(ObjectHealth),
    ObjectMana(ObjectMana),
    ObjectHidden(ObjectHidden),
    
    // ==================== 玩家状态 ====================
    PlayerUpdate(PlayerUpdate),
    PlayerInspect(PlayerInspect),
    LogOutSuccess(LogOutSuccess),
    TimeOfDay(TimeOfDay),
    ChangeAMode(ChangeAMode),
    ChangePMode(ChangePMode),
    ObjectName(ObjectName),
    UserStorage(UserStorage),
    LevelChanged(LevelChanged),
    GainExperience(GainExperience),
    
    // ==================== 地图 ====================
    MapEffect(MapEffect),
    
    // ==================== 聊天 ====================
    Chat(Chat),
    
    // ==================== 连接 ====================
    Connected(Connected),
    ClientVersion(ClientVersion),
    Disconnect(Disconnect),
    KeepAlive(KeepAlive),
    
    // ==================== 登录 ====================
    LoginSuccess(LoginSuccess),
    NewCharacter(NewCharacter),
    NewCharacterSuccess(NewCharacterSuccess),
    DeleteCharacter(DeleteCharacter),
    DeleteCharacterSuccess(DeleteCharacterSuccess),
    StartGame(StartGame),
    StartGameBanned(StartGameBanned),
    StartGameDelay(StartGameDelay),
    NewAccount(NewAccount),
    ChangePassword(ChangePassword),
    ChangePasswordBanned(ChangePasswordBanned),
    
    // ==================== NPC ====================
    NPCResponse(NPCResponse),
    NPCGoods(NPCGoods),
    NPCSell(NPCSell),
    NPCRepair(NPCRepair),
    NPCSRepair(NPCSRepair),
    NPCRefine(NPCRefine),
    NPCCheckRefine(NPCCheckRefine),
    NPCCollectRefine(NPCCollectRefine),
    NPCReplaceWedRing(NPCReplaceWedRing),
    NPCStorage(NPCStorage),
    NPCUpdate(NPCUpdate),
    NPCImageUpdate(NPCImageUpdate),
    DefaultNPC(DefaultNPC),
    
    // ==================== 战斗 ====================
    DamageIndicator(DamageIndicator),
    GainedItem(GainedItem),
    
    // ==================== Buff ====================
    AddBuff(AddBuff),
    RemoveBuff(RemoveBuff),
    
    // ==================== 移动 ====================
    UserBackStep(UserBackStep),
    ObjectBackStep(ObjectBackStep),
    UserDashAttack(UserDashAttack),
    
    // ==================== 其他 ====================
    ObjectShow(ObjectShow),
    ObjectHide(ObjectHide),
    
    // TODO: 根据需要添加更多包类型 (从 mir2_shared::packets::server)
    Unknown,
}

/// 统一的客户端包枚举
/// 
/// 复用 mir2_shared 的所有包定义
/// 用于 Bevy Event 系统
#[derive(Event, Clone, Debug)]
pub enum ClientPacket {
    // ==================== 连接 ====================
    ClientVersion(ClientVersionPacket),
    Disconnect(ClientDisconnect),
    KeepAlive(ClientKeepAlive),
    
    // ==================== 登录 ====================
    NewAccount(ClientNewAccount),
    ChangePassword(ClientChangePassword),
    Login(Login),
    NewCharacter(ClientNewCharacter),
    DeleteCharacter(ClientDeleteCharacter),
    StartGame(ClientStartGame),
    LogOut(LogOut),
    
    // ==================== 移动 ====================
    Turn(Turn),
    Walk(Walk),
    Run(Run),
    
    // ==================== 聊天 ====================
    Chat(ClientChat),
    
    // ==================== 战斗 ====================
    Attack(Attack),
    RangeAttack(RangeAttack),
    Harvest(Harvest),
    
    // ==================== NPC 交互 ====================
    CallNPC(CallNPC),
    BuyItem(BuyItem),
    SellItem(SellItem),
    RepairItem(RepairItem),
    BuyItemBack(BuyItemBack),
    SRepairItem(SRepairItem),
    
    // ==================== 物品操作 ====================
    MoveItem(MoveItem),
    StoreItem(StoreItem),
    TakeBackItem(TakeBackItem),
    MergeItem(MergeItem),
    EquipItem(EquipItem),
    RemoveItem(RemoveItem),
    RemoveSlotItem(RemoveSlotItem),
    SplitItem(SplitItem),
    UseItem(UseItem),
    DropItem(DropItem),
    PickUp(PickUp),
    DepositRefineItem(DepositRefineItem),
    RetrieveRefineItem(RetrieveRefineItem),
    RefineCancel(RefineCancel),
    RefineItem(RefineItem),
    CheckRefine(CheckRefine),
    ReplaceWedRing(ReplaceWedRing),
    DepositTradeItem(DepositTradeItem),
    RetrieveTradeItem(RetrieveTradeItem),
    
    // ==================== 技能 ====================
    MagicKey(MagicKey),
    Magic(Magic),
    SpellToggle(SpellToggle),
    
    // ==================== 其他 ====================
    Inspect(Inspect),
    ChangeAMode(ClientChangeAMode),
    ChangePMode(ClientChangePMode),
    ChangeTrade(ChangeTrade),
    SwitchGroup(SwitchGroup),
    AddMember(AddMember),
    DellMember(DellMember),
    GroupInvite(GroupInvite),
    TownRevive(TownRevive),
    ConsignItem(ConsignItem),
    MarketSearch(MarketSearch),
    MarketRefresh(MarketRefresh),
    MarketPage(MarketPage),
    MarketBuy(MarketBuy),
    MarketGetBack(MarketGetBack),
    RequestUserName(RequestUserName),
    RequestChatItem(RequestChatItem),
    EditGuildMember(EditGuildMember),
    EditGuildNotice(EditGuildNotice),
    GuildInvite(GuildInvite),
    RequestGuildInfo(RequestGuildInfo),
    GuildNameReturn(GuildNameReturn),
    GuildStorageGoldChange(GuildStorageGoldChange),
    GuildStorageItemChange(GuildStorageItemChange),
    GuildWarReturn(GuildWarReturn),
    MarriageRequest(ClientMarriageRequest),
    MarriageReply(MarriageReply),
    ChangeMarriage(ChangeMarriage),
    DivorceRequest(ClientDivorceRequest),
    DivorceReply(DivorceReply),
    AddMentor(AddMentor),
    MentorReply(MentorReply),
    AllowMentor(AllowMentor),
    CancelMentor(CancelMentor),
    TradeRequest(ClientTradeRequest),
    TradeReply(TradeReply),
    TradeGold(ClientTradeGold),
    TradeConfirm(ClientTradeConfirm),
    TradeCancel(ClientTradeCancel),
    EquipSlotItem(EquipSlotItem),
    FishingCast(FishingCast),
    FishingChangeAutocast(FishingChangeAutocast),
    AcceptQuest(AcceptQuest),
    FinishQuest(FinishQuest),
    AbandonQuest(AbandonQuest),
    ShareQuest(ShareQuest),
    
    // TODO: 根据需要添加更多包类型 (从 mir2_shared::packets::client)
    Unknown,
}

/// Bevy Event: 服务器包到达
#[derive(Event, Clone, Debug)]
pub struct ServerPacketEvent {
    pub packet: ServerPacket,
}

/// Bevy Event: 发送客户端包
#[derive(Event, Clone, Debug)]
pub struct ClientPacketEvent {
    pub packet: ClientPacket,
}

// ==================== 辅助函数 ====================

/// 从 mir2_shared 包创建 ServerPacketEvent
/// 
/// 示例用法:
/// ```
/// let player_packet = ObjectPlayer { ... };
/// commands.trigger(ServerPacket::ObjectPlayer(player_packet).into_event());
/// ```
impl ServerPacket {
    pub fn into_event(self) -> ServerPacketEvent {
        ServerPacketEvent { packet: self }
    }
}

impl ClientPacket {
    pub fn into_event(self) -> ClientPacketEvent {
        ClientPacketEvent { packet: self }
    }
}
