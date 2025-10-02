use std::{convert::TryFrom, io::Cursor};

use byteorder::{LittleEndian, ReadBytesExt};
use mir2_shared::{
    binary::{read_bool, read_dotnet_string},
    // Data types from SharedRust data module
    ClientHeroInformation, ClientIntelligentCreature, ClientMagic, ClientQuestInfo,
    ClientQuestProgress, GuildRank, ItemInfo, ClientMapInfo, UserItem, WorldMapSetup, Stats,
    // Enums
    enums::{
        AttackMode, BuffType, ChatType, DamageType, HeroBehaviour, HeroSpawnState,
        IntelligentCreatureType, ItemGrade, LevelEffects, MirClass, MirDirection, MirGender,
        MirGridType, PetMode, PoisonType, Spell, SpellEffect, WeatherSetting,
    },
    // Packet infrastructure
    packets::{PacketHeader, ServerPacketId},
    // Basic types
    Point,
};

// Import packet parsing functions from SharedRust (namespaced as 'packets')
// This provides access to parsing functions like packets::map::parse_map_information()
use mir2_shared::packets::server as packets;

// Import packet types from SharedRust
// All server packet types are now properly defined in mir2_shared::packets::server
use mir2_shared::packets::server::{
    // NPC System
    NPCSell, NPCRepair, NPCSRepair, NPCRefine, NPCCheckRefine, NPCCollectRefine,
    NPCReplaceWedRing, NPCStorage, NPCRequestInput,
    // Item System
    SellItem, RepairItem, ItemRepaired, SplitItem, SplitItem1, RefreshItem,
    ItemSlotSizeChanged, ItemSealChanged, CraftItem, NewItemInfo,
    // Magic System
    NewMagic, MagicLeveled, RemoveMagic, SpellToggle,
    // Player Status
    PlayerUpdate, PlayerInspect, LogOutSuccess, TimeOfDay, ChangeAMode, ChangePMode,
    ObjectName, UserStorage, SetAutoPotValue,
    // Object Status
    ObjectHealth, ObjectMana, ObjectHidden, MapEffect,
    // Group/Party System
    SwitchGroup, GroupMembersMap, SendMemberLocation,
    // Guild System
    GuildStorageList, GuildNoticeChange, GuildMemberChange,
    // Hero System
    UpdateHeroSpawnState, SetHeroBehaviour, ManageHeroes, HeroCreateRequest,
    // Quest System
    ChangeQuest, NewQuestInfo,
    // Account/Character Management
    NewCharacter, NewCharacterSuccess, DeleteCharacter, DeleteCharacterSuccess,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ServerMessage {
    Connected,
    ClientVersion {
        result: ClientVersionResult,
    },
    Disconnect {
        reason: u8,
    },
    KeepAlive {
        time: i64,
    },
    Login {
        result: LoginResult,
    },
    LoginBanned {
        reason: String,
        expiry_ticks: i64,
    },
    LoginSuccess {
        characters: Vec<CharacterSummary>,
    },
    Unknown {
        opcode: i16,
        payload: Vec<u8>,
    },
    Unimplemented {
        opcode: i16,
    },
    ParseError {
        opcode: i16,
        message: String,
    },
    StartGame {
        result: StartGameResult,
        resolution: i32,
    },
    StartGameBanned {
        reason: String,
        expiry_ticks: i64,
    },
    StartGameDelay {
        milliseconds: i64,
    },
    MapInformation(MapInformation),
    NewMapInfo(NewMapInfo),
    WorldMapSetup(WorldMapSetupInfo),
    SearchMapResult(SearchMapResult),
    UserInformation(UserInformation),
    UserLocation(UserLocation),
    UserSlotsRefresh(UserSlotsRefresh),
    ObjectPlayer(PlayerObject),
    ObjectHero(HeroObject),
    ObjectMonster(ObjectMonster),
    ObjectRemove(ObjectRemove),
    ObjectTurn(ObjectMotion),
    ObjectWalk(ObjectMotion),
    ObjectRun(ObjectMotion),
    ObjectAttack(ObjectAttack),
    Struck(Struck),
    ObjectStruck(ObjectStruck),
    DamageIndicator(DamageIndicator),
    DuraChanged(DuraChanged),
    DeleteItem(DeleteItem),
    DeleteQuestItem(DeleteQuestItem),
    ObjectItem(ObjectItem),
    ObjectGold(ObjectGold),
    GainedItem(GainedItem),
    GainedGold(GainedGold),
    LoseGold(LoseGold),
    GainedCredit(GainedCredit),
    LoseCredit(LoseCredit),
    GainedQuestItem(GainedQuestItem),
    Death(Death),
    ObjectDied(ObjectDied),
    ColourChanged(ColourChanged),
    ObjectColourChanged(ObjectColourChanged),
    ObjectGuildNameChanged(ObjectGuildNameChanged),
    GainExperience(GainExperience),
    GainHeroExperience(GainHeroExperience),
    LevelChanged(LevelChanged),
    HeroLevelChanged(HeroLevelChanged),
    ObjectLeveled(ObjectLeveled),
    HealthChanged(HealthChanged),
    HeroHealthChanged(HeroHealthChanged),
    ObjectHarvest(ObjectHarvest),
    ObjectHarvested(ObjectHarvested),
    ObjectNpc(ObjectNpc),
    NpcResponse(NpcResponse),
    Chat(Chat),
    ObjectChat(ObjectChat),
    Magic(Magic),
    MagicDelay(MagicDelay),
    MagicCast(MagicCast),
    ObjectMagic(ObjectMagic),
    ObjectEffect(ObjectEffect),
    ObjectProjectile(ObjectProjectile),
    RangeAttack(RangeAttack),
    MoveItem(MoveItem),
    EquipItem(EquipItem),
    MergeItem(MergeItem),
    RemoveItem(RemoveItem),
    RemoveSlotItem(RemoveSlotItem),
    TakeBackItem(TakeBackItem),
    StoreItem(StoreItem),
    UseItem(UseItem),
    DropItem(DropItem),
    Pushed(Pushed),
    ObjectPushed(ObjectPushed),
    AddBuff(AddBuff),
    RemoveBuff(RemoveBuff),
    PauseBuff(PauseBuff),
    TradeRequest(TradeRequest),
    TradeAccept(TradeAccept),
    TradeGold(TradeGold),
    TradeItem(TradeItem),
    TradeConfirm(TradeConfirm),
    TradeCancel(TradeCancel),
    GroupInvite(GroupInvite),
    AddMember(AddMember),
    DeleteGroup(DeleteGroup),
    DeleteMember(DeleteMember),
    ShareQuest(ShareQuest),
    CompleteQuest(CompleteQuest),
    NPCGoods(NPCGoods),
    ObjectHide(ObjectHide),
    ObjectShow(ObjectShow),
    Poisoned(Poisoned),
    ObjectPoisoned(ObjectPoisoned),
    Revived(Revived),
    ObjectRevived(ObjectRevived),
    GuildInvite(GuildInvite),
    GuildStatus(GuildStatus),
    GuildStorageGoldChange(GuildStorageGoldChange),
    GuildStorageItemChange(GuildStorageItemChange),
    NewHero(NewHero),
    HeroInformation(HeroInformation),
    MapChanged(MapChanged),
    ObjectTeleportOut(ObjectTeleportOut),
    ObjectTeleportIn(ObjectTeleportIn),
    TeleportIn(TeleportIn),
    ObjectRangeAttack(ObjectRangeAttack),
    UserDash(UserDash),
    ObjectDash(ObjectDash),
    UserDashFail(UserDashFail),
    ObjectDashFail(ObjectDashFail),
    // NPC System
    NPCSell(NPCSell),
    NPCRepair(NPCRepair),
    NPCSRepair(NPCSRepair),
    NPCRefine(NPCRefine),
    NPCCheckRefine(NPCCheckRefine),
    NPCCollectRefine(NPCCollectRefine),
    NPCReplaceWedRing(NPCReplaceWedRing),
    NPCStorage(NPCStorage),
    NPCRequestInput(NPCRequestInput),
    // Item System
    SellItem(SellItem),
    RepairItem(RepairItem),
    ItemRepaired(ItemRepaired),
    SplitItem(SplitItem),
    SplitItem1(SplitItem1),
    RefreshItem(RefreshItem),
    ItemSlotSizeChanged(ItemSlotSizeChanged),
    ItemSealChanged(ItemSealChanged),
    CraftItem(CraftItem),
    NewItemInfo(NewItemInfo),
    // Magic System
    NewMagic(NewMagic),
    MagicLeveled(MagicLeveled),
    RemoveMagic(RemoveMagic),
    SpellToggle(SpellToggle),
    // Player Status
    PlayerUpdate(PlayerUpdate),
    PlayerInspect(PlayerInspect),
    LogOutSuccess(LogOutSuccess),
    TimeOfDay(TimeOfDay),
    ChangeAMode(ChangeAMode),
    ChangePMode(ChangePMode),
    ObjectName(ObjectName),
    UserStorage(UserStorage),
    // Object Status
    ObjectHealth(ObjectHealth),
    ObjectMana(ObjectMana),
    ObjectHidden(ObjectHidden),
    MapEffect(MapEffect),
    // Group System
    SwitchGroup(SwitchGroup),
    GroupMembersMap(GroupMembersMap),
    SendMemberLocation(SendMemberLocation),
    // Guild System Extended
    GuildStorageList(GuildStorageList),
    GuildNoticeChange(GuildNoticeChange),
    GuildMemberChange(GuildMemberChange),
    // Hero System Extended
    UpdateHeroSpawnState(UpdateHeroSpawnState),
    SetAutoPotValue(SetAutoPotValue),
    SetHeroBehaviour(SetHeroBehaviour),
    ManageHeroes(ManageHeroes),
    HeroCreateRequest(HeroCreateRequest),
    // Quest System
    ChangeQuest(ChangeQuest),
    NewQuestInfo(NewQuestInfo),
    // Account/Character Management
    NewCharacter(NewCharacter),
    NewCharacterSuccess(NewCharacterSuccess),
    DeleteCharacter(DeleteCharacter),
    DeleteCharacterSuccess(DeleteCharacterSuccess),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientVersionResult {
    WrongVersion,
    CorrectVersion,
    Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginResult {
    Disabled,
    BadAccountId,
    BadPassword,
    AccountNotExist,
    WrongPassword,
    PasswordMustChange,
    Unknown(u8),
}

impl From<u8> for LoginResult {
    fn from(value: u8) -> Self {
        match value {
            0 => LoginResult::Disabled,
            1 => LoginResult::BadAccountId,
            2 => LoginResult::BadPassword,
            3 => LoginResult::AccountNotExist,
            4 => LoginResult::WrongPassword,
            5 => LoginResult::PasswordMustChange,
            other => LoginResult::Unknown(other),
        }
    }
}

impl From<u8> for ClientVersionResult {
    fn from(value: u8) -> Self {
        match value {
            0 => ClientVersionResult::WrongVersion,
            1 => ClientVersionResult::CorrectVersion,
            other => ClientVersionResult::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartGameResult {
    Disabled,
    NotLoggedIn,
    CharacterNotFound,
    NoStartPoint,
    Success,
    Unknown(u8),
}

impl From<u8> for StartGameResult {
    fn from(value: u8) -> Self {
        match value {
            0 => StartGameResult::Disabled,
            1 => StartGameResult::NotLoggedIn,
            2 => StartGameResult::CharacterNotFound,
            3 => StartGameResult::NoStartPoint,
            4 => StartGameResult::Success,
            other => StartGameResult::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSummary {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub last_access_ticks: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapInformation {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub lights: u8,
    pub lightning: bool,
    pub fire: bool,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather: WeatherSetting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMapInfo {
    pub map_index: i32,
    pub info: ClientMapInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMapSetupInfo {
    pub setup: WorldMapSetup,
    pub teleport_to_npc_cost: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMapResult {
    pub map_index: i32,
    pub npc_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerObject {
    pub object_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank_name: String,
    pub name_colour_argb: i32,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub poison: PoisonType,
    pub dead: bool,
    pub hidden: bool,
    pub effect: SpellEffect,
    pub wing_effect: u8,
    pub extra: bool,
    pub mount_type: i16,
    pub riding_mount: bool,
    pub fishing: bool,
    pub transform_type: i16,
    pub element_orb_effect: u32,
    pub element_orb_level: u32,
    pub element_orb_max: u32,
    pub buffs: Vec<BuffType>,
    pub level_effects: LevelEffects,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroObject {
    pub player: PlayerObject,
    pub owner_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRemove {
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMotion {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectAttack {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub spell: Spell,
    pub level: u8,
    pub attack_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Struck {
    pub attacker_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectStruck {
    pub object_id: u32,
    pub attacker_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageIndicator {
    pub damage: i32,
    pub damage_type: DamageType,
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuraChanged {
    pub unique_id: u64,
    pub current_dura: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectItem {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub location: Point,
    pub image: u16,
    pub grade: ItemGrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectGold {
    pub object_id: u32,
    pub gold: u32,
    pub location: Point,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainedItem {
    pub item: UserItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainedQuestItem {
    pub item: UserItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainedGold {
    pub gold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoseGold {
    pub gold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainedCredit {
    pub credit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoseCredit {
    pub credit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteItem {
    pub unique_id: u64,
    pub count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteQuestItem {
    pub unique_id: u64,
    pub count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourChanged {
    pub name_colour_argb: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectColourChanged {
    pub object_id: u32,
    pub name_colour_argb: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectGuildNameChanged {
    pub object_id: u32,
    pub guild_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainExperience {
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainHeroExperience {
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectLeveled {
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHarvest {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHarvested {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMonster {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub location: Point,
    pub image: u16,
    pub direction: MirDirection,
    pub effect: u8,
    pub ai: u8,
    pub light: u8,
    pub dead: bool,
    pub skeleton: bool,
    pub poison: PoisonType,
    pub hidden: bool,
    pub shock_time: i64,
    pub binding_shot_center: bool,
    pub extra: bool,
    pub extra_byte: u8,
    pub buffs: Vec<BuffType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectNpc {
    pub object_id: u32,
    pub name: String,
    pub name_colour_argb: i32,
    pub image: u16,
    pub colour_argb: i32,
    pub location: Point,
    pub direction: MirDirection,
    pub quest_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcResponse {
    pub page: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    pub message: String,
    pub chat_type: ChatType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectChat {
    pub object_id: u32,
    pub text: String,
    pub chat_type: ChatType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Magic {
    pub spell: Spell,
    pub target_id: u32,
    pub target: Point,
    pub cast: bool,
    pub level: u8,
    pub secondary_target_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicDelay {
    pub object_id: u32,
    pub spell: Spell,
    pub delay: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicCast {
    pub spell: Spell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMagic {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub spell: Spell,
    pub target_id: u32,
    pub target: Point,
    pub cast: bool,
    pub level: u8,
    pub self_broadcast: bool,
    pub secondary_target_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectEffect {
    pub object_id: u32,
    pub effect: SpellEffect,
    pub effect_type: u32,
    pub delay_time: u32,
    pub time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectProjectile {
    pub spell: Spell,
    pub source: u32,
    pub destination: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeAttack {
    pub target_id: u32,
    pub target: Point,
    pub spell: Spell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveItem {
    pub grid: MirGridType,
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub to: i32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeItem {
    pub grid_from: MirGridType,
    pub grid_to: MirGridType,
    pub id_from: u64,
    pub id_to: u64,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub to: i32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveSlotItem {
    pub grid: MirGridType,
    pub grid_to: MirGridType,
    pub unique_id: u64,
    pub to: i32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TakeBackItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseItem {
    pub unique_id: u64,
    pub success: bool,
    pub grid: MirGridType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropItem {
    pub unique_id: u64,
    pub count: u16,
    pub hero_item: bool,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pushed {
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectPushed {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddBuff {
    pub buff: ClientBuff,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientBuff {
    pub buff_type: BuffType,
    pub visible: bool,
    pub object_id: u32,
    pub expire_time: i64,
    pub infinite: bool,
    pub paused: bool,
    pub stats: Stats,
    pub values: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveBuff {
    pub buff_type: BuffType,
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseBuff {
    pub buff_type: BuffType,
    pub object_id: u32,
    pub paused: bool,
}

// Trading System
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeAccept {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeGold {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TradeItem {
    pub trade_items: Vec<Option<UserItem>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeConfirm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeCancel {
    pub unlock: bool,
}

// Party/Group System
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupInvite {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddMember {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteGroup;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteMember {
    pub name: String,
}

// Quest System (simplified for now)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareQuest {
    pub quest_index: i32,
    pub sharer_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteQuest {
    pub completed_quests: Vec<i32>,
}

// NPC System (using existing NpcResponse instead of NPCResponse)
#[derive(Debug, Clone, PartialEq)]
pub struct NPCGoods {
    pub list: Vec<UserItem>,
    pub rate: f32,
    pub panel_type: u8,
    pub hide_added_stats: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHide {
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectShow {
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Poisoned {
    pub poison: PoisonType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectPoisoned {
    pub object_id: u32,
    pub poison: PoisonType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revived;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRevived {
    pub object_id: u32,
    pub effect: bool,
}

// Guild System
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInvite {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildStatus {
    pub guild_name: String,
    pub guild_rank_name: String,
    pub level: u8,
    pub experience: i64,
    pub max_experience: i64,
    pub gold: u32,
    pub spare_points: u8,
    pub member_count: i32,
    pub max_members: i32,
    pub voting: bool,
    pub item_count: u8,
    pub buff_count: u8,
    pub my_options: u8,
    pub my_rank_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildStorageGoldChange {
    pub amount: u32,
    pub change_type: u8,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuildStorageItemChange {
    pub change_type: u8,
    pub to: i32,
    pub from: i32,
    pub user: i32,
    pub user_id: Option<i64>,
    pub item: Option<UserItem>,
}

// Hero System
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewHero {
    pub result: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeroInformation {
    pub object_id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
    pub magics: Vec<ClientMagic>,
    pub auto_pot: bool,
    pub auto_hp_percent: u8,
    pub auto_mp_percent: u8,
    pub hp_item_index: i32,
    pub mp_item_index: i32,
}

// Map/Teleport System
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapChanged {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub lights: u8,
    pub location: Point,
    pub direction: MirDirection,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather: WeatherSetting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTeleportOut {
    pub object_id: u32,
    pub teleport_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTeleportIn {
    pub object_id: u32,
    pub teleport_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportIn;

// Combat System (Additional)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRangeAttack {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub target_id: u32,
    pub target: Point,
    pub attack_type: u8,
    pub spell: Spell,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserDash {
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDash {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserDashFail {
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDashFail {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

// ============================================================================

// ============================================================================
// NOTE: NPC, Item, Magic, Player, Object, Group, Guild, Hero, Quest, Account
// packet structs have been moved to src/protocol_packets/packets/*.rs
// They are re-exported via: pub use crate::network::protocol_packets::packets::*;
// ============================================================================

pub struct Death {
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDied {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub death_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthChanged {
    pub hp: i32,
    pub mp: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroHealthChanged {
    pub hp: i32,
    pub mp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInformation {
    pub object_id: u32,
    pub real_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    pub name_colour_argb: i32,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    pub level_effects: LevelEffects,
    pub has_hero: bool,
    pub hero_behaviour: HeroBehaviour,
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
    pub quest_inventory: Option<Vec<Option<UserItem>>>,
    pub gold: u32,
    pub credit: u32,
    pub has_expanded_storage: bool,
    pub expanded_storage_expiry_binary: i64,
    pub magics: Vec<ClientMagic>,
    pub intelligent_creatures: Vec<ClientIntelligentCreature>,
    pub summoned_creature_type: IntelligentCreatureType,
    pub creature_summoned: bool,
    pub allow_observe: bool,
    pub observer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLocation {
    pub location: Point,
    pub direction: MirDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSlotsRefresh {
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
}

pub fn parse_server_message(header: PacketHeader, payload: Vec<u8>) -> ServerMessage {
    match ServerPacketId::try_from(header.opcode) {
        Ok(ServerPacketId::Connected) => ServerMessage::Connected,
        Ok(ServerPacketId::ClientVersion) => match payload.first().copied() {
            Some(byte) => ServerMessage::ClientVersion {
                result: ClientVersionResult::from(byte),
            },
            None => ServerMessage::ParseError {
                opcode: header.opcode,
                message: "client version packet missing result byte".to_string(),
            },
        },
        Ok(ServerPacketId::Disconnect) => match payload.first().copied() {
            Some(reason) => ServerMessage::Disconnect { reason },
            None => ServerMessage::ParseError {
                opcode: header.opcode,
                message: "disconnect packet missing reason byte".to_string(),
            },
        },
        Ok(ServerPacketId::KeepAlive) => {
            if payload.len() < 8 {
                return ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: "keepalive packet too short".to_string(),
                };
            }
            let time = i64::from_le_bytes(payload[0..8].try_into().expect("slice length checked"));
            ServerMessage::KeepAlive { time }
        }
        Ok(ServerPacketId::Login) => match payload.first().copied() {
            Some(result) => ServerMessage::Login {
                result: LoginResult::from(result),
            },
            None => ServerMessage::ParseError {
                opcode: header.opcode,
                message: "login packet missing result byte".to_string(),
            },
        },
        Ok(ServerPacketId::LoginBanned) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match read_dotnet_string(&mut cursor) {
                Ok(reason) => match cursor.read_i64::<LittleEndian>() {
                    Ok(expiry_ticks) => ServerMessage::LoginBanned {
                        reason,
                        expiry_ticks,
                    },
                    Err(err) => ServerMessage::ParseError {
                        opcode: header.opcode,
                        message: format!("login banned packet missing expiry: {err}"),
                    },
                },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("login banned packet invalid reason: {err}"),
                },
            }
        }
        Ok(ServerPacketId::LoginSuccess) => match packets::account::parse_login_success(&payload) {
            Ok(characters) => ServerMessage::LoginSuccess { characters },
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::StartGame) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match cursor.read_u8() {
                Ok(result_byte) => match cursor.read_i32::<LittleEndian>() {
                    Ok(resolution) => ServerMessage::StartGame {
                        result: StartGameResult::from(result_byte),
                        resolution,
                    },
                    Err(err) => ServerMessage::ParseError {
                        opcode: header.opcode,
                        message: format!("start game packet missing resolution: {err}"),
                    },
                },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("start game packet missing result: {err}"),
                },
            }
        }
        Ok(ServerPacketId::StartGameBanned) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match read_dotnet_string(&mut cursor) {
                Ok(reason) => match cursor.read_i64::<LittleEndian>() {
                    Ok(expiry_ticks) => ServerMessage::StartGameBanned {
                        reason,
                        expiry_ticks,
                    },
                    Err(err) => ServerMessage::ParseError {
                        opcode: header.opcode,
                        message: format!("start game banned packet missing expiry: {err}"),
                    },
                },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("start game banned packet invalid reason: {err}"),
                },
            }
        }
        Ok(ServerPacketId::StartGameDelay) => {
            let mut cursor = Cursor::new(payload.as_slice());
            match cursor.read_i64::<LittleEndian>() {
                Ok(milliseconds) => ServerMessage::StartGameDelay { milliseconds },
                Err(err) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message: format!("start game delay packet invalid payload: {err}"),
                },
            }
        }
        Ok(ServerPacketId::NewMapInfo) => match packets::map::parse_new_map_info(&payload) {
            Ok(info) => ServerMessage::NewMapInfo(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::MapInformation) => match packets::map::parse_map_information(&payload) {
            Ok(info) => ServerMessage::MapInformation(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::WorldMapSetup) => match packets::map::parse_world_map_setup(&payload) {
            Ok(info) => ServerMessage::WorldMapSetup(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::SearchMapResult) => {
            match packets::map::parse_search_map_result(&payload) {
                Ok(result) => ServerMessage::SearchMapResult(result),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::UserInformation) => {
            match packets::player::parse_user_information(&payload) {
                Ok(info) => ServerMessage::UserInformation(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::UserLocation) => match packets::player::parse_user_location(&payload) {
            Ok(info) => ServerMessage::UserLocation(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::UserSlotsRefresh) => {
            match packets::player::parse_user_slots_refresh(&payload) {
                Ok(info) => ServerMessage::UserSlotsRefresh(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectPlayer) => match packets::object::parse_object_player(&payload) {
            Ok(object) => ServerMessage::ObjectPlayer(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectHero) => match packets::hero::parse_object_hero(&payload) {
            Ok(object) => ServerMessage::ObjectHero(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectMonster) => {
            match packets::object::parse_object_monster(&payload) {
                Ok(object) => ServerMessage::ObjectMonster(object),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectRemove) => match packets::object::parse_object_remove(&payload) {
            Ok(object) => ServerMessage::ObjectRemove(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectTurn) => match packets::object::parse_object_motion(&payload) {
            Ok(object) => ServerMessage::ObjectTurn(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectWalk) => match packets::object::parse_object_motion(&payload) {
            Ok(object) => ServerMessage::ObjectWalk(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectRun) => match packets::object::parse_object_motion(&payload) {
            Ok(object) => ServerMessage::ObjectRun(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectAttack) => match packets::combat::parse_object_attack(&payload) {
            Ok(object) => ServerMessage::ObjectAttack(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Struck) => match packets::combat::parse_struck(&payload) {
            Ok(info) => ServerMessage::Struck(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectStruck) => match packets::combat::parse_object_struck(&payload) {
            Ok(object) => ServerMessage::ObjectStruck(object),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DamageIndicator) => {
            match packets::combat::parse_damage_indicator(&payload) {
                Ok(info) => ServerMessage::DamageIndicator(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::DuraChanged) => match packets::item::parse_dura_changed(&payload) {
            Ok(info) => ServerMessage::DuraChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DeleteItem) => match packets::item::parse_delete_item(&payload) {
            Ok(info) => ServerMessage::DeleteItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DeleteQuestItem) => {
            match packets::item::parse_delete_quest_item(&payload) {
                Ok(info) => ServerMessage::DeleteQuestItem(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectItem) => match packets::item::parse_object_item(&payload) {
            Ok(info) => ServerMessage::ObjectItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectGold) => match packets::item::parse_object_gold(&payload) {
            Ok(info) => ServerMessage::ObjectGold(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainedItem) => match packets::item::parse_gained_item(&payload) {
            Ok(info) => ServerMessage::GainedItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainedGold) => match packets::item::parse_gained_gold(&payload) {
            Ok(info) => ServerMessage::GainedGold(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::LoseGold) => match packets::item::parse_lose_gold(&payload) {
            Ok(info) => ServerMessage::LoseGold(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GainedCredit) => match packets::item::parse_gained_credit(&payload) {
            Ok(info) => ServerMessage::GainedCredit(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::LoseCredit) => match packets::item::parse_lose_credit(&payload) {
            Ok(info) => ServerMessage::LoseCredit(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Death) => match packets::combat::parse_death(&payload) {
            Ok(info) => ServerMessage::Death(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectDied) => match packets::combat::parse_object_died(&payload) {
            Ok(info) => ServerMessage::ObjectDied(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ColourChanged) => match packets::buff::parse_colour_changed(&payload) {
            Ok(info) => ServerMessage::ColourChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectColourChanged) => {
            match packets::buff::parse_object_colour_changed(&payload) {
                Ok(info) => ServerMessage::ObjectColourChanged(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectGuildNameChanged) => {
            match parse_object_guild_name_changed(&payload) {
                Ok(info) => ServerMessage::ObjectGuildNameChanged(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::GainExperience) => {
            match packets::player::parse_gain_experience(&payload) {
                Ok(info) => ServerMessage::GainExperience(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::GainHeroExperience) => {
            match packets::player::parse_gain_hero_experience(&payload) {
                Ok(info) => ServerMessage::GainHeroExperience(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::LevelChanged) => match packets::player::parse_level_changed(&payload) {
            Ok(info) => ServerMessage::LevelChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::HeroLevelChanged) => {
            match packets::hero::parse_hero_level_changed(&payload) {
                Ok(info) => ServerMessage::HeroLevelChanged(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectLeveled) => {
            match packets::object::parse_object_leveled(&payload) {
                Ok(info) => ServerMessage::ObjectLeveled(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::HealthChanged) => {
            match packets::combat::parse_health_changed(&payload) {
                Ok(info) => ServerMessage::HealthChanged(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::HeroHealthChanged) => {
            match packets::combat::parse_hero_health_changed(&payload) {
                Ok(info) => ServerMessage::HeroHealthChanged(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectHarvest) => {
            match packets::object::parse_object_harvest(&payload) {
                Ok(info) => ServerMessage::ObjectHarvest(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectHarvested) => {
            match packets::object::parse_object_harvested(&payload) {
                Ok(info) => ServerMessage::ObjectHarvested(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectNpc) => match packets::npc::parse_object_npc(&payload) {
            Ok(info) => ServerMessage::ObjectNpc(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::NPCResponse) => match packets::npc::parse_npc_response(&payload) {
            Ok(info) => ServerMessage::NpcResponse(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Chat) => match packets::chat::parse_chat(&payload) {
            Ok(chat) => ServerMessage::Chat(chat),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectChat) => match packets::chat::parse_object_chat(&payload) {
            Ok(chat) => ServerMessage::ObjectChat(chat),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Magic) => match packets::magic::parse_magic(&payload) {
            Ok(magic) => ServerMessage::Magic(magic),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::MagicDelay) => match packets::magic::parse_magic_delay(&payload) {
            Ok(delay) => ServerMessage::MagicDelay(delay),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::MagicCast) => match packets::magic::parse_magic_cast(&payload) {
            Ok(cast) => ServerMessage::MagicCast(cast),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectMagic) => match packets::magic::parse_object_magic(&payload) {
            Ok(magic) => ServerMessage::ObjectMagic(magic),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectEffect) => match packets::magic::parse_object_effect(&payload) {
            Ok(effect) => ServerMessage::ObjectEffect(effect),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectProjectile) => {
            match packets::magic::parse_object_projectile(&payload) {
                Ok(projectile) => ServerMessage::ObjectProjectile(projectile),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::RangeAttack) => match packets::combat::parse_range_attack(&payload) {
            Ok(attack) => ServerMessage::RangeAttack(attack),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::MoveItem) => match packets::item::parse_move_item(&payload) {
            Ok(info) => ServerMessage::MoveItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::EquipItem) => match packets::item::parse_equip_item(&payload) {
            Ok(info) => ServerMessage::EquipItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::MergeItem) => match packets::item::parse_merge_item(&payload) {
            Ok(info) => ServerMessage::MergeItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::RemoveItem) => match packets::item::parse_remove_item(&payload) {
            Ok(info) => ServerMessage::RemoveItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::RemoveSlotItem) => match packets::item::parse_remove_slot_item(&payload)
        {
            Ok(info) => ServerMessage::RemoveSlotItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::TakeBackItem) => match packets::item::parse_take_back_item(&payload) {
            Ok(info) => ServerMessage::TakeBackItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::StoreItem) => match packets::item::parse_store_item(&payload) {
            Ok(info) => ServerMessage::StoreItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::UseItem) => match packets::item::parse_use_item(&payload) {
            Ok(info) => ServerMessage::UseItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DropItem) => match packets::item::parse_drop_item(&payload) {
            Ok(info) => ServerMessage::DropItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Pushed) => match packets::combat::parse_pushed(&payload) {
            Ok(info) => ServerMessage::Pushed(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectPushed) => match packets::combat::parse_object_pushed(&payload) {
            Ok(info) => ServerMessage::ObjectPushed(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::AddBuff) => match packets::buff::parse_add_buff(&payload) {
            Ok(info) => ServerMessage::AddBuff(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::RemoveBuff) => match packets::buff::parse_remove_buff(&payload) {
            Ok(info) => ServerMessage::RemoveBuff(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::PauseBuff) => match packets::buff::parse_pause_buff(&payload) {
            Ok(info) => ServerMessage::PauseBuff(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::TradeRequest) => match packets::trade::parse_trade_request(&payload) {
            Ok(info) => ServerMessage::TradeRequest(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::TradeAccept) => match packets::trade::parse_trade_accept(&payload) {
            Ok(info) => ServerMessage::TradeAccept(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::TradeGold) => match packets::trade::parse_trade_gold(&payload) {
            Ok(info) => ServerMessage::TradeGold(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::TradeItem) => match packets::trade::parse_trade_item(&payload) {
            Ok(info) => ServerMessage::TradeItem(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::TradeConfirm) => match packets::trade::parse_trade_confirm(&payload) {
            Ok(info) => ServerMessage::TradeConfirm(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::TradeCancel) => match packets::trade::parse_trade_cancel(&payload) {
            Ok(info) => ServerMessage::TradeCancel(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GroupInvite) => match packets::group::parse_group_invite(&payload) {
            Ok(info) => ServerMessage::GroupInvite(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::AddMember) => match packets::group::parse_add_member(&payload) {
            Ok(info) => ServerMessage::AddMember(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DeleteGroup) => match packets::group::parse_delete_group(&payload) {
            Ok(info) => ServerMessage::DeleteGroup(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::DeleteMember) => match packets::group::parse_delete_member(&payload) {
            Ok(info) => ServerMessage::DeleteMember(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ShareQuest) => match packets::quest::parse_share_quest(&payload) {
            Ok(info) => ServerMessage::ShareQuest(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::CompleteQuest) => match packets::quest::parse_complete_quest(&payload) {
            Ok(info) => ServerMessage::CompleteQuest(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::NPCGoods) => match packets::npc::parse_npc_goods(&payload) {
            Ok(info) => ServerMessage::NPCGoods(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectHide) => match packets::map::parse_object_hide(&payload) {
            Ok(info) => ServerMessage::ObjectHide(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectShow) => match packets::map::parse_object_show(&payload) {
            Ok(info) => ServerMessage::ObjectShow(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::Poisoned) => match packets::buff::parse_poisoned(&payload) {
            Ok(info) => ServerMessage::Poisoned(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectPoisoned) => {
            match packets::buff::parse_object_poisoned(&payload) {
                Ok(info) => ServerMessage::ObjectPoisoned(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::Revived) => match packets::combat::parse_revived(&payload) {
            Ok(info) => ServerMessage::Revived(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectRevived) => {
            match packets::combat::parse_object_revived(&payload) {
                Ok(info) => ServerMessage::ObjectRevived(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::GuildInvite) => match packets::guild::parse_guild_invite(&payload) {
            Ok(info) => ServerMessage::GuildInvite(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GuildStatus) => match packets::guild::parse_guild_status(&payload) {
            Ok(info) => ServerMessage::GuildStatus(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::GuildStorageGoldChange) => {
            match parse_guild_storage_gold_change(&payload) {
                Ok(info) => ServerMessage::GuildStorageGoldChange(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::GuildStorageItemChange) => {
            match parse_guild_storage_item_change(&payload) {
                Ok(info) => ServerMessage::GuildStorageItemChange(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::NewHero) => match packets::hero::parse_new_hero(&payload) {
            Ok(info) => ServerMessage::NewHero(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::HeroInformation) => {
            match packets::hero::parse_hero_information(&payload) {
                Ok(info) => ServerMessage::HeroInformation(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::MapChanged) => match packets::map::parse_map_changed(&payload) {
            Ok(info) => ServerMessage::MapChanged(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectTeleportOut) => {
            match packets::map::parse_object_teleport_out(&payload) {
                Ok(info) => ServerMessage::ObjectTeleportOut(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::ObjectTeleportIn) => {
            match packets::map::parse_object_teleport_in(&payload) {
                Ok(info) => ServerMessage::ObjectTeleportIn(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::TeleportIn) => match packets::map::parse_teleport_in(&payload) {
            Ok(info) => ServerMessage::TeleportIn(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectRangeAttack) => {
            match packets::combat::parse_object_range_attack(&payload) {
                Ok(info) => ServerMessage::ObjectRangeAttack(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::UserDash) => match packets::combat::parse_user_dash(&payload) {
            Ok(info) => ServerMessage::UserDash(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectDash) => match packets::combat::parse_object_dash(&payload) {
            Ok(info) => ServerMessage::ObjectDash(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::UserDashFail) => match packets::combat::parse_user_dash_fail(&payload) {
            Ok(info) => ServerMessage::UserDashFail(info),
            Err(message) => ServerMessage::ParseError {
                opcode: header.opcode,
                message,
            },
        },
        Ok(ServerPacketId::ObjectDashFail) => {
            match packets::combat::parse_object_dash_fail(&payload) {
                Ok(info) => ServerMessage::ObjectDashFail(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        Ok(ServerPacketId::GainedQuestItem) => {
            match packets::quest::parse_gained_quest_item(&payload) {
                Ok(info) => ServerMessage::GainedQuestItem(info),
                Err(message) => ServerMessage::ParseError {
                    opcode: header.opcode,
                    message,
                },
            }
        }
        // NPC System Routes
        Ok(ServerPacketId::NPCSell) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCRepair) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCSRepair) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCRefine) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCCheckRefine) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCCollectRefine) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCReplaceWedRing) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCStorage) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NPCRequestInput) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Item System Routes
        Ok(ServerPacketId::SellItem) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::RepairItem) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ItemRepaired) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::SplitItem) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::SplitItem1) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::RefreshItem) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ItemSlotSizeChanged) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ItemSealChanged) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::CraftItem) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NewItemInfo) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Magic System Routes
        Ok(ServerPacketId::NewMagic) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::MagicLeveled) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::RemoveMagic) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::SpellToggle) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Player Status Routes
        Ok(ServerPacketId::PlayerUpdate) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::PlayerInspect) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::LogOutSuccess) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::TimeOfDay) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ChangeAMode) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ChangePMode) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ObjectName) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::UserStorage) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Object Status Routes
        Ok(ServerPacketId::ObjectHealth) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ObjectMana) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ObjectHidden) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::MapEffect) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Group System Routes
        Ok(ServerPacketId::SwitchGroup) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::GroupMembersMap) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::SendMemberLocation) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Guild System Routes
        Ok(ServerPacketId::GuildStorageList) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::GuildNoticeChange) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::GuildMemberChange) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Hero System Routes
        Ok(ServerPacketId::UpdateHeroSpawnState) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::SetAutoPotValue) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::SetHeroBehaviour) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::ManageHeroes) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::HeroCreateRequest) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Quest System Routes
        Ok(ServerPacketId::ChangeQuest) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NewQuestInfo) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        // Account/Character Routes
        Ok(ServerPacketId::NewCharacter) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::NewCharacterSuccess) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::DeleteCharacter) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(ServerPacketId::DeleteCharacterSuccess) => {
            // TODO: Implement packet parsing using mir2_shared::packets
            ServerMessage::Unimplemented {
                opcode: header.opcode,
            }
        }
        Ok(_) => ServerMessage::Unknown {
            opcode: header.opcode,
            payload,
        },
        Err(_) => ServerMessage::Unknown {
            opcode: header.opcode,
            payload,
        },
    }
}

// ============================================================================
// All parse_* functions have been moved to modular files:
// src/protocol_packets/packets/*.rs
//
// Module Structure:
//   - account.rs:  parse_login_success
//   - buff.rs:     parse_add_buff, parse_remove_buff, parse_pause_buff, etc.
//   - chat.rs:     parse_chat, parse_object_chat
//   - combat.rs:   parse_object_attack, parse_struck, parse_damage_indicator, etc.
//   - group.rs:    parse_group_invite, parse_add_member, parse_delete_group, etc.
//   - guild.rs:    parse_guild_invite, parse_guild_status, parse_guild_member_change
//   - hero.rs:     parse_object_hero, parse_new_hero, parse_hero_information, etc.
//   - item.rs:     parse_gained_item, parse_dura_changed, parse_delete_item, etc.
//   - magic.rs:    parse_magic, parse_magic_delay, parse_object_magic, etc.
//   - map.rs:      parse_map_information, parse_map_changed, parse_teleport_in, etc.
//   - npc.rs:      parse_npc_response, parse_npc_goods, parse_object_npc, etc.
//   - object.rs:   parse_object_player, parse_object_remove, parse_object_motion, etc.
//   - player.rs:   parse_user_information, parse_user_location, parse_gain_experience, etc.
//   - quest.rs:    parse_share_quest, parse_complete_quest, parse_gained_quest_item
//   - trade.rs:    parse_trade_request, parse_trade_accept, parse_trade_gold, etc.
//
// Access via: packets::<module>::parse_<function_name>(&payload)
// Example: packets::combat::parse_object_attack(&payload)
// ============================================================================
