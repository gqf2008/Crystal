// ============================================================================
// 服务端事件（ServerEvent）——网络层与游戏/UI 解耦的事件总线
// ============================================================================
// 背景：network_system 曾直接 ResMut 各 UI State（God System，issue #65）。
// 目标：网络层只负责 解码 → 广播 ServerEvent；各游戏/UI 模块自己消费事件。
// 本模块先覆盖高频/核心包作为样板，其余包逐步迁移。
// 使用：`events.write(ServerEvent::...)`；消费方 `EventReader<ServerEvent>`。

use bevy::prelude::*;
use mir2_shared::packets::server::{chat, combat, drops, experience, item_operations, npc_interaction};

/// 服务端事件（按包类型组织；字段为消费方需要的最终值）
/// Bevy 0.19：Message（替代旧 EventReader/EventWriter）
#[derive(Message, Debug, Clone)]
pub enum ServerEvent {
    /// HealthChanged：HP/MP 当前值
    HealthChanged { hp: i32, mp: i32 },
    /// GainedGold：增量（击杀掉落等），消费方负责累加余额
    GoldGained { gold: u32 },
    /// LoseGold：扣减金额（C# S.LoseGold），消费方负责余额扣减
    GoldLost { amount: u32 },
    /// TimeOfDay：服务端昼夜（C# S.TimeOfDay）
    TimeOfDay { light: mir2_shared::enums::LightSetting },
    /// ObjectColourChanged：对象名字颜色（C# PK 红名）
    ObjectColourChanged { object_id: u32, name_colour_argb: i32 },
    /// StoragePasswordResult：仓库密码设置/移除结果（C# result：4=成功 2=当前密码错误 5=未设置）
    StoragePasswordResult { result: u8 },
    /// LogOutSuccess：登出成功，返回选角
    LogOutSuccess,
    /// ChangeAMode：攻击模式确认（C# S.ChangeAMode）
    AttackModeChanged { mode: mir2_shared::enums::AttackMode },
    /// ManageHeroes：英雄列表（C# S.ManageHeroes）
    HeroManageReceived {
        heroes: Vec<mir2_shared::data::client_data::ClientHeroInformation>,
        current: Option<mir2_shared::data::client_data::ClientHeroInformation>,
    },
    /// NewHero：创建英雄结果（C# S.NewHero.Result：1=BadName 4=MaxHeroes 10=Success）
    NewHeroResult { result: u8 },
    /// SetHeroBehaviour：英雄行为确认（C# S.SetHeroBehaviour，值 0=攻击 1=反击 2=跟随 3=自定义）
    HeroBehaviourSet { behaviour: u8 },
    /// SetAutoPotValue：英雄自动药阈值（C# S.SetAutoPotValue；stat 12=HP 13=MP）
    HeroAutoPotSet { stat: u8, value: u32 },
    /// SetAutoPotItem：英雄自动药物品（C# S.SetAutoPotItem）
    HeroAutoPotItemSet { grid: u8, item_index: i32 },
    /// GainExperience：经验增量
    ExperienceGained { amount: i64 },
    /// LevelChanged：新等级 + 经验（原实现一并更新 exp/max_exp）
    LevelChanged { level: u16, exp: i64, max_exp: i64 },
    /// Chat / ObjectChat：聊天消息（颜色映射由消费端 chat.rs 负责）
    Chat {
        text: String,
        chat_type: mir2_shared::enums::ChatType,
    },
    /// NPCResponse：NPC 对话页（行 + 可见）
    NpcDialog { lines: Vec<String>, visible: bool },
    /// MoveItem：背包内交换（仅 Inventory grid 成功响应）
    InventoryMoved { from: usize, to: usize },
    /// EquipItem：装备成功（背包→装备槽，旧装备放回背包）
    ItemEquipped { unique_id: u64, to: usize },
    /// RemoveItem：卸下装备（装备槽→背包）
    ItemRemoved { unique_id: u64 },
    /// UseItem：使用成功（背包计数减一/移除）
    ItemUsed { unique_id: u64 },
    /// #228 DuraChanged：物品持久度变化
    ItemDuraChanged { unique_id: u64, current_dura: u16 },
    /// #228 DeleteItem：物品删除/消耗
    ItemDeleted { unique_id: u64 },
    /// #228 GainedItem：获得物品入包
    ItemGained { item: InvItem },
    /// #230 MapEffect：地图范围特效
    MapEffect { x: i32, y: i32, effect: u8 },
    /// #230 PlaySound：服务端指定音效
    PlaySound { sound_id: u32 },
    /// #230 SetTimer：启动计时器（秒）
    TimerSet { timer_id: i32, seconds: i32 },
    /// #230 ExpireTimer：计时器到期/关闭
    TimerExpired { timer_id: i32 },
    /// #232 MountUpdate：坐骑上/下马
    MountUpdated {
        object_id: u32,
        mount_type: i16,
        is_mounted: bool,
    },
    /// #236 Poisoned/ObjectPoisoned：中毒状态
    ObjectPoisoned { object_id: u32, poisoned: bool },
    /// #240 ItemRepaired：修理结果（耐久/最大耐久更新）
    ItemRepaired {
        unique_id: u64,
        max_dura: u16,
        current_dura: u16,
    },
    /// #240 ItemSlotSizeChanged：镶嵌槽位数量变化
    ItemSlotSizeChanged { unique_id: u64, slot_size: i32 },
    /// #242 SpellToggle：开关技能状态同步
    SpellToggled {
        spell: mir2_shared::enums::Spell,
        can_use: bool,
    },
    /// #248 NPCImageUpdate：NPC 形象变化
    NpcImageUpdated { npc_id: u32, image: u16 },
    /// #248 GainedCredit：声望增加
    CreditGained { credit: u32 },
    /// #248 LoseCredit：声望减少
    CreditLost { amount: u32 },
    /// #250 SetCompass：罗盘目标方向
    CompassTarget { x: i32, y: i32 },
    /// #254 SendMemberLocation：小队成员位置
    MemberLocation { name: String, x: i32, y: i32 },
    /// Roll：骰子/尤茨结果（npc_id 由 roll 消费端从 NpcDialogState 读取）
    Roll {
        r#type: i32,
        page: String,
        result: i32,
        auto_roll: bool,
        visible: bool,
        started_at: f32,
        finished: bool,
    },
    /// AwakeningNeedMaterials：觉醒材料需求（item_id, count）
    AwakeningMaterials { materials: Vec<(i32, i32)> },
    /// Awakening：觉醒结果
    AwakeningResult { result: i32, result_text: String },
    /// UserStorage：仓库物品全量（服务端打开仓库时下发）
    StorageOpened { items: Vec<Option<InvItem>>, visible: bool },
    /// GuildStatus（1 字节格式）：是否在行会；false 时消费端清空行会数据
    GuildInGuild { in_guild: bool },
    /// GuildStatus（完整格式）：行会全量信息
    GuildData {
        name: String,
        leader: String,
        notice: Vec<String>,
        members: Vec<GuildMember>,
        gold: u32,
    },
    /// GuildStorageList：行会仓库物品（unique_id, item_index, count, info_name）
    GuildStorage { items: Vec<(u64, i32, u16, String)> },
    /// GroupMembersMap：组队成员全量
    GroupMembers { members: Vec<GroupMember> },
    /// GroupInvite：收到组队邀请
    GroupInvite { inviter_name: String, inviter_id: u64 },
    /// DeleteGroup：组队解散
    GroupDeleted,
    /// DeleteMember：成员离开
    GroupMemberLeft { name: String },
    /// MentorRequest：收到拜师邀请
    MentorInvite { name: String, level: u16 },
    /// MentorUpdate：师徒信息更新
    MentorUpdate {
        name: String,
        level: u32,
        online: bool,
        mentee_exp: i64,
    },
    /// FriendUpdate：好友列表增量（列表或单个）
    FriendUpdated { entries: Vec<FriendEntry> },
    /// Rankings：排行榜
    Rankings { entries: Vec<RankEntry> },
    /// GuildNoticeChange：行会公告更新
    GuildNotice { notice: Vec<String> },
    /// ChangeQuest：任务进度更新（C# 语义：仅更新，移除由 CompleteQuest 负责）
    QuestChanged { entry: QuestEntry },
    /// CompleteQuest：任务完成（从日志移除）
    QuestCompleted { id: i32 },
    /// AddBuff：获得/刷新状态
    BuffAdded { tag: u8, ticks: u32 },
    /// RemoveBuff：状态消失
    BuffRemoved { tag: u8 },
    /// PlayerInspect：查看玩家
    InspectPlayer {
        name: String,
        guild: String,
        level: u16,
        class: u8,
        gender: u8,
        items: Vec<InspectItem>,
    },
    /// UpdateIntelligentCreatureList：宠物列表
    CreatureList { creatures: Vec<CreatureEntry> },
    /// ChangeHero：切换英雄
    HeroChanged { index: u8 },
    /// MarriageRequest：求婚邀请
    MarriageInvite { name: String },
    /// LoverUpdate：婚姻状态
    MarriageStatus { married: bool },
    /// DivorceRequest：离婚请求
    DivorceRequest,
    /// ItemRentalRequest：收到租赁请求（物主）
    RentalRequestReceived,
    /// UpdateRentalItem：租赁物品更新
    RentalItemUpdate { has_item: bool, fee: u32, period: i32 },
    /// ItemRentalFee：费用更新
    RentalFee { fee: u32 },
    /// ItemRentalPeriod：期限更新
    RentalPeriod { period: i32 },
    /// DepositRentalItem：存入租赁物品
    RentalDeposit { uid: u64, success: bool },
    /// RetrieveRentalItem：取回租赁物品
    RentalRetrieve { uid: u64, success: bool },
    /// ItemRentalLock：本侧锁定
    RentalLocked,
    /// ItemRentalPartnerLock：对方锁定
    RentalPartnerLocked,
    /// CanConfirmItemRental：可确认状态
    RentalCanConfirm { can_confirm: bool },
    /// ConfirmItemRental：确认结果
    RentalConfirmed { success: bool },
    /// CancelItemRental：取消
    RentalCancelled,
    /// NPCMarket：市场页数
    MarketPages { pages: usize },
    /// NPCMarketPage：市场列表（auction_id, unique_id, item_index, count, info_name, seller, price）
    MarketListings { listings: Vec<(u64, u64, i32, u16, String, String, u32)> },
    /// ConsignItem：寄售结果
    MarketConsign { uid: u64, success: bool },
    /// MarketSuccess：市场成功消息
    MarketSuccess { message: String },
    /// MarketFail：市场失败
    MarketFail { reason: u8 },
    /// GameShopInfo：商城目录（item_index, gold_price, credit_price, category, stock）
    ShopCatalog { items: Vec<(i32, u32, u32, String, i32)>, gold: u32 },
    /// GameShopStock：商品库存更新
    ShopStock { item_id: i32, stock: i32 },
    /// GuildTerritoryPage：领地列表
    TerritoryList { rows: Vec<TerritoryRow> },
    /// GuildRequestWar：宣战确认
    TerritoryWar { guild_name: String },
    /// TradeGold：对方交易金币
    TradeGold { amount: u64 },
    /// TradeCancel：交易关闭/取消
    TradeCancelled,
    /// FishingUpdate：钓鱼进度
    FishingUpdate { progress: i32, success: bool },
    /// ReceiveMail：邮件（列表条目 + 可选详情）
    MailReceived { entry: MailEntry, detail: Option<MailDetail> },
    /// ParcelCollected：收取邮件附件结果（C# sbyte：-1=无 0=已全部收取 1=成功）
    ParcelCollected { result: i8 },
    /// TradeRequest：交易请求/打开（状态机由消费端根据自身状态应用）
    TradeRequested { name: String },
    /// TradeConfirm：锁定状态（a=发起者）
    TradeConfirm { a_locked: bool, b_locked: bool },
    /// TradeItem：对方物品更新
    TradeItemUpdate { uid: u64, grid: usize, count: u16, is_add: bool },
    /// DepositTradeItem：放入交易槽结果
    TradeDeposit { from: i32, to: i32, success: bool },
    /// GuildMemberChange：行会成员变化（加入/离开/更新）
    GuildMemberChanged {
        name: String,
        rank: u8,
        online: bool,
        joined: bool,
        removed: bool,
    },
    /// GuildInvite：收到行会邀请
    GuildInvited { name: String },
    /// Rankings 解析失败：清空排行
    RankingsCleared,
    /// MapChanged：天气更新
    WeatherChanged { code: u16 },
    /// NewMapInfo：地图信息（大地图）
    MapInfo {
        map_index: i32,
        title: String,
        npcs: Vec<NpcRow>,
    },
    /// Death：本地玩家死亡
    PlayerDied,
    /// RequestReincarnation：收到轮回术复活请求（#222）
    ReincarnationRequested,
    /// Revived：本地玩家复活
    PlayerRevived,
    /// #226 ObjectHide：对象隐藏（隐身等）
    ObjectHidden { object_id: u32 },
    /// #226 ObjectShow：对象显形
    ObjectShown { object_id: u32 },
    /// #226 ObjectSitDown：对象坐下
    ObjectSitDown { object_id: u32, direction: u8 },
    /// #226 ObjectPushed：对象被击退（位置 + 朝向）
    ObjectPushed {
        object_id: u32,
        x: i32,
        y: i32,
        direction: u8,
    },
    /// #226 ObjectTeleportOut：对象传送消失
    ObjectTeleportOut { object_id: u32 },
    /// #226 ObjectTeleportIn：对象传送出现
    ObjectTeleportIn { object_id: u32 },
    /// NewMagic：学会技能
    MagicLearned { magic: ClientMagic },
    /// MagicLeveled：技能升级（C# S.MagicLeveled）
    MagicLeveled {
        object_id: u32,
        spell: mir2_shared::enums::Spell,
        level: u8,
        experience: u16,
    },
    /// CraftItem：合成结果
    CraftResult { recipe_id: u32, count: u16, success: bool },
    /// NPCGoods：商品对话框（Buy/Craft 等）
    NpcGoods { goods: Vec<GoodsEntry>, rate: f32 },
    /// NPCGoods（Sell/Repair/SpecialRepair）：出售/修理面板
    NpcSellPanel { panel_type: mir2_shared::enums::PanelType },
    /// UserInformation：进图初始化同步（HUD/技能/背包/装备/物品名缓存）
    UserInformation {
        name: String,
        level: u16,
        hp: i32,
        mp: i32,
        exp: i64,
        max_exp: i64,
        gold: u32,
        class: u8,
        object_id: u32,
        magics: Vec<ClientMagic>,
        inventory: Vec<Option<InvItem>>,
        equipment: Vec<Option<InvItem>>,
        item_names: Vec<(i32, String)>,
        /// #208：角色面板属性（服务端最终值）
        max_hp: i32,
        max_mp: i32,
        ac: [i32; 2],
        mac: [i32; 2],
        dc: [i32; 2],
        mc: [i32; 2],
        sc: [i32; 2],
        critical_rate: i32,
        critical_damage: i32,
        attack_speed: i32,
        accuracy: i32,
        agility: i32,
        luck: i32,
        /// #210：State 页数据
        bag_weight: i32,
        wear_weight: i32,
        hand_weight: i32,
        magic_resist: i32,
        poison_resist: i32,
        health_recovery: i32,
        spell_recovery: i32,
        poison_recovery: i32,
        holy: i32,
        freezing: i32,
        poison_atk: i32,
    },
    /// HeroInformation：英雄完整信息（C# S.HeroInformation : UserInformation + autopot，#203）
    HeroInformation {
        object_id: u32,
        name: String,
        class: u8,
        gender: u8,
        level: u16,
        hp: i32,
        mp: i32,
        exp: i64,
        max_exp: i64,
        inventory: Vec<Option<InvItem>>,
        equipment: Vec<Option<InvItem>>,
        magics: Vec<ClientMagic>,
        auto_pot: bool,
        auto_hp_percent: u8,
        auto_mp_percent: u8,
        hp_item_index: i32,
        mp_item_index: i32,
    },
}

use crate::game::dialogs::npc_goods::GoodsEntry;

use crate::game::dialogs::big_map::NpcRow;
use mir2_shared::data::client_data::ClientMagic;

use crate::game::dialogs::mail::{MailDetail, MailEntry};

use crate::game::dialogs::guild_territory::TerritoryRow;


use crate::game::dialogs::buff::BuffEntry;
use crate::game::dialogs::creature::CreatureEntry;
use crate::game::dialogs::inspect::InspectItem;
use crate::game::dialogs::quest_log::QuestEntry;

use crate::game::dialogs::friend::FriendEntry;
use crate::game::dialogs::ranking::RankEntry;

use crate::game::dialogs::group::GroupMember;

use crate::game::dialogs::guild::GuildMember;
use crate::game::dialogs::inventory::InvItem;

/// 从已解码的服务端包构造 ServerEvent（便于各分支统一发送）
pub mod from_packet {
    use super::*;

    pub fn health_changed(p: &combat::HealthChanged) -> ServerEvent {
        ServerEvent::HealthChanged { hp: p.hp as i32, mp: p.mp as i32 }
    }
    pub fn gold_gained(p: &drops::GainedGold) -> ServerEvent {
        ServerEvent::GoldGained { gold: p.gold }
    }
    pub fn gold_lost(p: &drops::LoseGold) -> ServerEvent {
        ServerEvent::GoldLost { amount: p.gold }
    }
    pub fn experience_gained(p: &experience::GainExperience) -> ServerEvent {
        ServerEvent::ExperienceGained { amount: p.amount as i64 }
    }
    pub fn level_changed(p: &experience::LevelChanged) -> ServerEvent {
        ServerEvent::LevelChanged {
            level: p.level,
            exp: p.experience,
            max_exp: p.max_experience,
        }
    }
    pub fn chat(p: &chat::Chat) -> ServerEvent {
        ServerEvent::Chat { text: p.message.clone(), chat_type: p.chat_type }
    }
    pub fn object_chat(p: &chat::ObjectChat) -> ServerEvent {
        ServerEvent::Chat { text: p.text.clone(), chat_type: p.chat_type }
    }
    pub fn npc_dialog(p: &npc_interaction::NPCResponse) -> ServerEvent {
        ServerEvent::NpcDialog { lines: p.page.clone(), visible: true }
    }
    pub fn move_item(p: &item_operations::MoveItem) -> ServerEvent {
        ServerEvent::InventoryMoved { from: p.from as usize, to: p.to as usize }
    }
    pub fn equip_item(p: &item_operations::EquipItem) -> ServerEvent {
        ServerEvent::ItemEquipped { unique_id: p.unique_id as u64, to: p.to as usize }
    }
    pub fn remove_item(p: &item_operations::RemoveItem) -> ServerEvent {
        ServerEvent::ItemRemoved { unique_id: p.unique_id as u64 }
    }
    pub fn use_item(p: &item_operations::UseItem) -> ServerEvent {
        ServerEvent::ItemUsed { unique_id: p.unique_id as u64 }
    }
}
