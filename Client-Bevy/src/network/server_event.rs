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
    /// GuildStorageList：行会仓库物品
    GuildStorage { items: Vec<Option<StorageItem>> },
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
    /// NPCMarketPage：市场列表
    MarketListings { listings: Vec<MarketItem> },
    /// ConsignItem：寄售结果
    MarketConsign { uid: u64, success: bool },
    /// MarketSuccess：市场成功消息
    MarketSuccess { message: String },
    /// MarketFail：市场失败
    MarketFail { reason: u8 },
    /// GameShopInfo：商城目录
    ShopCatalog { items: Vec<ShopItem>, gold: u32 },
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
    /// Revived：本地玩家复活
    PlayerRevived,
    /// NewMagic：学会技能
    MagicLearned { magic: ClientMagic },
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
    },
}

use crate::game::dialogs::npc_goods::GoodsEntry;

use crate::game::dialogs::big_map::NpcRow;
use mir2_shared::data::client_data::ClientMagic;

use crate::game::dialogs::mail::{MailDetail, MailEntry};

use crate::game::dialogs::game_shop::ShopItem;
use crate::game::dialogs::guild_territory::TerritoryRow;

use crate::game::dialogs::market::MarketItem;

use crate::game::dialogs::buff::BuffEntry;
use crate::game::dialogs::creature::CreatureEntry;
use crate::game::dialogs::inspect::InspectItem;
use crate::game::dialogs::quest_log::QuestEntry;

use crate::game::dialogs::friend::FriendEntry;
use crate::game::dialogs::ranking::RankEntry;

use crate::game::dialogs::group::GroupMember;

use crate::game::dialogs::guild::{GuildMember, StorageItem};
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
