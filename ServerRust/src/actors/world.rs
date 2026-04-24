// WorldActor - 游戏世界主循环
// 对应 C# GameSrv/WorldServer.cs + M2Server 核心逻辑

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::prelude::Context;
use kameo::message::Message;
use tokio::time::{interval, Duration};
use tracing::{info, debug, warn};
use chrono::Timelike;

use crate::actors::player::{
    PlayerActor, PlayerState, MoveType, MoveRequest, TurnRequest, BroadcastMovement,
    GetPlayerState, SetMapData, SetPlayerState, AttackRequest, TakeDamage,
    AddItemToInventory, InventoryMoveItem, GetItemInfo, ConsumeItem,
    InventoryEquipItem, GetEquipmentInfo, InventoryUnequipItem,
    RemoveItemFromInventory, InventoryMergeItem, InventorySplitItem,
    DropGold, AddGold, DeductGold, DeductMP, AddExperience,
    AcceptQuest, CompleteQuest, AbandonQuest, GetQuest, HasCompletedQuest,
    SetCreature, TickCreatureHunger, RestoreCreatureHunger,
    SetHeroIndex, SetHeroBehaviour, SetAutoPotValue, SetAutoPotItem,
    StoreItem, TakeBackItem, SetRefineLog, SetAttackMode, SetPetMode,
    SetSpellKey, ToggleSpell, RemoveSlotItemMsg, SetPlayerPosition, SetFishing,
    ClearReincarnation, ClearReincarnationHost, ReviveAtHalfHp, SetExpMultiplier,
};
use crate::actors::inventory::{EquipmentSlot, GroundItem, PlayerInventory, generate_item_uid};
use crate::actors::refine::{RefineStatus, RefineLog};
use crate::actors::friend::FriendList;
use crate::actors::mail::{MailMessage, Mailbox, generate_mail_id};
use crate::actors::guild::GuildRank;
use crate::actors::quest::{QuestInstance, QuestProgress, QuestStatus, QuestLog};
use crate::actors::creature::{IntelligentCreature, CreatureType, PickupMode, CreatureLog};
use crate::combat::attack::{self as combat_attack};
#[allow(unused_imports)]
use crate::combat::buff;
use crate::db::{self, DbPool};
use crate::gate::actor::{SendToClient, GateActor};
use crate::actors::social::{SocialActor, SocialChatCommand};
use mir2_shared::packets::Packet;
use crate::maps::loader::{self, MapData};
use crate::util::wire::{build_packet_bytes, write_dotnet_string};
/// WorldActor 启动参数
pub struct WorldActorArgs {
    pub tick_interval_ms: u64,
    pub gate_ref: ActorRef<GateActor>,
    /// 地图文件所在目录
    pub map_dir: PathBuf,
    /// 刷怪配置文件所在目录（可选）
    pub spawn_dir: Option<PathBuf>,
    /// 任务文件所在目录（{file_name}.txt）
    pub quest_dir: PathBuf,
    /// SQLite 数据库连接池
    pub db_pool: DbPool,
    /// SocialActor 引用（用于转发社交命令）
    pub social_ref: ActorRef<SocialActor>,
}

/// 世界中的玩家记录
#[derive(Clone)]
struct PlayerRecord {
    /// PlayerActor 引用
    actor_ref: ActorRef<PlayerActor>,
    /// Session ID（用于路由到 GateActor）
    session_id: u64,
    /// 玩家名称（缓存，避免 async 查找）
    name: String,
    /// 账号名（用于数据库存取）
    account_username: String,
    /// 上次广播的 PK 值（用于检测名字颜色变化）
    last_pk_points: i32,
    /// 玩家 object_id（缓存，避免 async 查找）
    object_id: u32,
}

/// NPC 定义（从刷怪配置加载）
#[derive(Debug, Clone)]
pub struct NpcSpawn {
    pub name: String,
    pub image: u16, // Monster enum value
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub db_index: i32,
}

/// 怪物定义（从 DB 配置或 TOML 加载）
#[derive(Debug, Clone)]
pub struct MonsterSpawn {
    pub name: String,
    pub image: u16,
    /// 对应 monster_infos.index（掉落查询用）
    pub monster_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub hp: i32,
    pub min_dmg: i32,
    pub max_dmg: i32,
    pub xp: i32,
    pub map_index: u16,
}

/// 地图刷怪配置
#[derive(Debug, Clone, Default)]
pub struct SpawnConfig {
    pub npcs: Vec<NpcSpawn>,
    pub monsters: Vec<MonsterSpawn>,
}

/// 加载刷怪配置
fn load_spawn_config(map_name: &str, map_index: u16, spawn_dir: &Path) -> SpawnConfig {
    let path = spawn_dir.join(format!("{}.toml", map_name));
    if !path.exists() {
        debug!("No spawn config for map '{}'", map_name);
        return SpawnConfig::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<RawSpawnConfig>(&content) {
            Ok(raw) => {
                info!("Loaded spawn config: {} ({} NPCs, {} monsters)",
                      path.display(), raw.npcs.len(), raw.monsters.len());
                SpawnConfig {
                    npcs: raw.npcs.into_iter().map(|n| NpcSpawn {
                        name: n.name,
                        image: n.image,
                        x: n.x,
                        y: n.y,
                        direction: n.direction,
                        db_index: 0,
                    }).collect(),
                    monsters: raw.monsters.into_iter().map(|m| MonsterSpawn {
                        name: m.name,
                        image: m.image,
                        monster_index: 0, // TOML spawn 无 DB 索引，后续通过 image/name 匹配
                        x: m.x,
                        y: m.y,
                        direction: m.direction,
                        hp: m.hp,
                        min_dmg: m.min_dmg,
                        max_dmg: m.max_dmg,
                        xp: m.xp,
                        map_index,
                    }).collect(),
                }
            }
            Err(e) => {
                warn!("Failed to parse spawn config '{}': {}", path.display(), e);
                SpawnConfig::default()
            }
        },
        Err(e) => {
            warn!("Failed to read spawn config '{}': {}", path.display(), e);
            SpawnConfig::default()
        }
    }
}

use mir2_shared::enums::Stat;

/// Build SpawnConfig from DB-loaded MapInfo + MonsterInfo
fn spawn_config_from_db(
    map_info: &db::MapInfo,
    monster_infos: &HashMap<i32, db::MonsterInfo>,
    npc_infos: &HashMap<i32, db::NPCInfo>,
) -> SpawnConfig {
    let npcs: Vec<NpcSpawn> = npc_infos.values()
        .filter(|n| n.map_index == map_info.index)
        .map(|n| NpcSpawn {
            name: n.name.clone(),
            image: n.image as u16,
            x: n.x,
            y: n.y,
            direction: 0, // NPCs spawn facing north by default
            db_index: n.index,
        })
        .collect();

    let monsters: Vec<MonsterSpawn> = map_info.respawns.iter().filter_map(|r| {
        let mi = monster_infos.get(&r.monster_index)?;
        let hp = mi.stats.get(&(Stat::HP as u8)).copied().unwrap_or(50);
        let min_ac = mi.stats.get(&(Stat::MinAC as u8)).copied().unwrap_or(0);
        let max_ac = mi.stats.get(&(Stat::MaxAC as u8)).copied().unwrap_or(5);
        Some(MonsterSpawn {
            name: mi.name.clone(),
            image: mi.image as u16,
            monster_index: mi.index,
            x: r.x,
            y: r.y,
            direction: r.direction as u8,
            hp,
            min_dmg: min_ac,
            max_dmg: max_ac,
            xp: mi.experience,
            map_index: map_info.index as u16,
        })
    }).collect();

    if !monsters.is_empty() || !npcs.is_empty() {
        debug!("DB spawn config for map '{}': {} NPCs, {} monsters", map_info.file_name, npcs.len(), monsters.len());
    }

    SpawnConfig { npcs, monsters }
}

/// DB spawn context — bundles references to avoid 9-arg function
struct SpawnContext<'a> {
    map_info: Option<&'a db::MapInfo>,
    monster_infos: &'a HashMap<i32, db::MonsterInfo>,
    npc_infos: &'a HashMap<i32, db::NPCInfo>,
    dragon_info: Option<&'a db::DragonInfo>,
}

#[derive(serde::Deserialize)]
struct RawSpawnConfig {
    #[serde(default)]
    npcs: Vec<RawNpc>,
    #[serde(default)]
    monsters: Vec<RawMonster>,
}

#[derive(serde::Deserialize)]
struct RawNpc {
    name: String,
    image: u16,
    x: i32,
    y: i32,
    #[serde(default = "default_direction")]
    direction: u8,
}

#[derive(serde::Deserialize)]
struct RawMonster {
    name: String,
    image: u16,
    x: i32,
    y: i32,
    #[serde(default = "default_direction")]
    direction: u8,
    #[serde(default = "default_hp")]
    hp: i32,
    #[serde(default = "default_min_dmg")]
    min_dmg: i32,
    #[serde(default = "default_max_dmg")]
    max_dmg: i32,
    #[serde(default = "default_xp")]
    xp: i32,
}

fn default_direction() -> u8 { 4 }
fn default_hp() -> i32 { 50 }
fn default_min_dmg() -> i32 { 1 }
fn default_max_dmg() -> i32 { 5 }
fn default_xp() -> i32 { 10 }

/// AI 行为类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonsterAiType {
    /// 被动：不会主动攻击，只有被攻击才反击
    Passive,
    /// 主动：发现玩家就追击并攻击（默认）
    Aggressive,
    /// 逃跑：低血量时会逃跑
    Coward,
    /// 守卫：只保护特定区域，超出范围回退
    Guard,
    /// Boss：强化版主动，更高攻击频率和范围
    Boss,
    /// 远程：保持距离进行物理远程攻击
    Ranged,
    /// 法师：保持距离进行魔法远程攻击
    Mage,
    /// 治疗者：治疗附近受伤的怪物
    Healer,
    /// 召唤者：低血量时召唤援军
    Summoner,
}

impl MonsterAiType {
    /// 从 DB 的 ai 字段解析
    fn from_db_ai(ai: i32) -> Self {
        match ai {
            0 | 1 => Self::Passive,
            64 | 81 | 82 | 252 => Self::Boss,
            4 | 5 => Self::Coward,
            6 => Self::Guard,
            10 | 11 | 12 => Self::Ranged,
            20 | 21 | 22 => Self::Mage,
            30 | 31 => Self::Healer,
            40 | 41 => Self::Summoner,
            _ => Self::Aggressive,
        }
    }
}

/// AI 运行时参数（从 MonsterInfo 构建）
#[derive(Debug, Clone)]
struct MonsterAiProfile {
    pub ai_type: MonsterAiType,
    /// 视野/仇恨范围
    pub aggro_range: i32,
    /// 攻击范围（1=近战，>1=远程）
    pub attack_range: i32,
    /// 攻击冷却（ticks）
    pub attack_cooldown: u64,
    /// 移动速度（每多少 tick 移动一次）
    pub move_interval: u64,
    /// 逃跑阈值（HP 百分比，Coward 用）
    pub flee_threshold: f32,
}

impl MonsterAiProfile {
    /// 从 MonsterInfo 构建默认 AI 配置
    fn from_info(info: &db::MonsterInfo) -> Self {
        let ai_type = MonsterAiType::from_db_ai(info.ai);
        let view_range = info.view_range.max(3);
        let (aggro_range, attack_range, attack_cooldown, move_interval) = match ai_type {
            MonsterAiType::Passive => (view_range, 1, 10, 2),
            MonsterAiType::Aggressive => (view_range, 1, 5, 2),
            MonsterAiType::Coward => (view_range / 2, 1, 8, 1),
            MonsterAiType::Guard => (view_range, 1, 5, 2),
            MonsterAiType::Boss => (view_range * 2, 2, 3, 1),
            MonsterAiType::Ranged => (view_range, 4, 6, 2),
            MonsterAiType::Mage => (view_range, 6, 8, 2),
            MonsterAiType::Healer => (view_range, 4, 8, 2),
            MonsterAiType::Summoner => (view_range, 1, 5, 2),
        };
        Self {
            ai_type,
            aggro_range,
            attack_range,
            attack_cooldown,
            move_interval,
            flee_threshold: if ai_type == MonsterAiType::Coward { 0.3 } else { 0.0 },
        }
    }
}

/// AI 运行时状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonsterAiState {
    ///  idle / 巡逻
    Idle,
    /// 追击目标
    Chase,
    /// 攻击中
    Attack,
    /// 逃跑
    Flee,
    /// 返回出生点
    Return,
}

/// 运行时怪物状态
#[derive(Clone)]
struct MonsterState {
    pub object_id: u32,
    pub name: String,
    pub image: u16,
    /// 对应 monster_infos 的 index（用于掉落查询）
    pub monster_index: i32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub min_dmg: i32,
    pub max_dmg: i32,
    pub xp: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    /// 所在地图索引（当前服务器单地图运行，默认 0）
    pub map_index: u16,
    /// 下次可攻击的 tick
    pub next_attack_tick: u64,
    /// 下次可移动的 tick
    pub next_move_tick: u64,
    /// 下次可召唤的 tick（Summoner 用）
    pub next_summon_tick: u64,
    /// AI 配置（创建时从 DB 加载）
    pub ai_profile: MonsterAiProfile,
    /// 当前 AI 状态
    pub ai_state: MonsterAiState,
    /// 当前目标玩家 session（None = 无目标）
    pub target_session: Option<u64>,
    /// 是否已被激怒（Passive 怪物被攻击后变为 Aggressive）
    pub provoked: bool,
    /// 是否为精英怪物
    pub is_elite: bool,
    /// 是否为世界Boss
    pub is_boss: bool,
}

fn dist_to_spawn(monster: &MonsterState) -> i32 {
    (monster.x - monster.spawn_x).abs() + (monster.y - monster.spawn_y).abs()
}

/// 运行时 NPC 状态
#[derive(Clone)]
#[allow(dead_code)]
struct NpcState {
    pub object_id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub db_index: i32,
    pub map_index: u16,
}

/// 方向增量 (8 方向 MirDirection)
const MON_DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const MON_DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

/// 默认复活点
const DEFAULT_SPAWN_X: i32 = 330;
const DEFAULT_SPAWN_Y: i32 = 330;

// 魔法 spell ID 常量（Mir2 原数值）
const SPELL_HEALING: u8 = 61;
const SPELL_MASS_HEALING: u8 = 75;
const SPELL_HEALING_CIRCLE: u8 = 86;
const SPELL_MAGIC_SHIELD: u8 = 43;
const SPELL_SOUL_SHIELD: u8 = 69;
const SPELL_BLESSED_ARMOUR: u8 = 71;
const SPELL_TELEPORT: u8 = 37;
const SPELL_FIREBALL: u8 = 31; // 法师怪物默认法术

impl MonsterState {
    /// 朝目标方向走一步，返回新位置和方向
    fn step_toward(&self, tx: i32, ty: i32) -> (i32, i32, u8) {
        let dx = tx - self.x;
        let dy = ty - self.y;
        let mut best_dir = 4u8;
        let mut best_dist = (dx * dx + dy * dy) as u64;
        for dir in 0..8u8 {
            let nx = self.x + MON_DIR_DX[dir as usize];
            let ny = self.y + MON_DIR_DY[dir as usize];
            let dist = ((nx - tx).pow(2) + (ny - ty).pow(2)) as u64;
            if dist < best_dist {
                best_dist = dist;
                best_dir = dir;
            }
        }
        let nx = self.x + MON_DIR_DX[best_dir as usize];
        let ny = self.y + MON_DIR_DY[best_dir as usize];
        (nx, ny, best_dir)
    }

    /// 远离目标方向走一步（逃跑用）
    fn step_away(&self, tx: i32, ty: i32) -> (i32, i32, u8) {
        let dx = tx - self.x;
        let dy = ty - self.y;
        // 远离 = 朝向相反方向
        let opposite_x = self.x - dx;
        let opposite_y = self.y - dy;
        self.step_toward(opposite_x, opposite_y)
    }
}

/// 商店回购条目
#[derive(Debug, Clone)]
pub struct BuybackItem {
    pub item: mir2_shared::data::item::UserItem,
    pub sell_price: u64,
}

/// WorldActor 状态
pub struct WorldActor {
    /// Tick 计数器
    tick_count: u64,
    /// 在线玩家 Actor 引用（按 session_id 索引）
    players: HashMap<u64, PlayerRecord>,
    /// 商店回购列表（session_id -> 最近卖出的物品，最多保留 10 个）
    buyback_items: HashMap<u64, Vec<BuybackItem>>,
    /// 已加载的地图缓存
    maps: HashMap<u16, MapData>,
    /// GateActor 引用，用于发数据包给客户端
    gate_ref: ActorRef<GateActor>,
    /// 地图目录
    map_dir: PathBuf,
    /// 刷怪配置目录
    spawn_dir: Option<PathBuf>,
    /// 下一个对象 ID
    next_object_id: u32,
    /// 活跃怪物（按 object_id 索引）
    monsters: HashMap<u32, MonsterState>,
    /// 活跃 NPC（按 object_id 索引）
    npcs: HashMap<u32, NpcState>,
    /// 等待重生的怪物 (object_id → 重生 tick)
    respawn_queue: HashMap<u32, (MonsterSpawn, u64)>,
    /// 世界Boss存活队列 (object_id → 自动消失 tick)
    world_boss_queue: HashMap<u32, u64>,
    /// 死亡玩家复活队列 (session_id → 死亡 tick)
    player_death_queue: HashMap<u64, u64>,
    /// 钓鱼进度计数器 (session_id → 已钓鱼 tick 数)
    fishing_tick_counters: HashMap<u64, u32>,
    /// 地面掉落物品
    ground_items: Vec<GroundItem>,
    /// 已打开的门 (map_index, door_index)
    open_doors: std::collections::HashSet<(u16, u8)>,
    /// SQLite 数据库连接池
    db_pool: DbPool,
    /// 游戏配置：地图信息（key = map index）
    map_infos: HashMap<i32, db::MapInfo>,
    /// 游戏配置：物品信息
    item_infos: HashMap<i32, db::ItemInfo>,
    /// 游戏配置：怪物信息
    monster_infos: HashMap<i32, db::MonsterInfo>,
    /// 游戏配置：怪物掉落（monster_index -> drop list）
    monster_drops: HashMap<i32, Vec<db::MonsterDropInfo>>,
    /// 游戏配置：NPC 信息
    npc_infos: HashMap<i32, db::NPCInfo>,
    /// 游戏配置：NPC 商品（npc_index -> goods list）
    npc_goods: HashMap<i32, Vec<db::NpcGoodsInfo>>,
    /// 游戏配置：NPC 脚本 ((npc_index, page_name) -> lines)
    npc_scripts: HashMap<(i32, String), Vec<String>>,
    /// 游戏配置：任务信息
    quest_infos: HashMap<i32, db::QuestInfo>,
    /// 游戏配置：魔法信息（key = spell ID）
    magic_infos: HashMap<u32, db::MagicInfo>,
    /// 游戏配置：龙信息
    dragon_info: Option<db::DragonInfo>,
    /// 游戏商店物品（从 DB 加载）
    game_shop_items: Vec<db::GameShopItem>,
    /// 地图传送点索引: (map_index, source_x, source_y) -> MapMovementInfo
    movement_index: HashMap<(i32, i32, i32), db::MapMovementInfo>,
    /// SocialActor 引用（用于转发社交命令）
    social_ref: ActorRef<SocialActor>,
    /// 全局经验倍率事件
    global_exp_multiplier: f64,
    /// 全局掉落倍率
    global_drop_multiplier: f64,
    /// 全局金币倍率
    global_gold_multiplier: f64,
    /// 全局事件过期时间（tick count）
    global_exp_event_end_tick: u64,
    /// 当前全局事件名称
    global_event_name: Option<String>,
    /// 隐身中的玩家 session 集合（用于视野管理）
    invisible_sessions: std::collections::HashSet<u64>,
    /// 当前光照设置（0=Normal, 1=Dawn, 2=Day, 3=Evening, 4=Night）
    current_light: mir2_shared::enums::LightSetting,
    /// 寄售/拍卖列表
    auctions: Vec<AuctionListing>,
    /// 下一个拍卖ID
    next_auction_id: u64,
    /// 市场搜索缓存 (session_id -> search results indices)
    market_search_cache: HashMap<u64, MarketSearchCache>,
    /// 物品租赁会话 (initiator_session_id -> RentalSession)
    rental_sessions: HashMap<u64, RentalSession>,
    /// 已生效的租赁记录 (renter_name -> list of RentedItem)
    player_rentals: HashMap<String, Vec<RentedItem>>,
    /// 行会战争声明 (guild_name -> set of enemy guild names)
    guild_wars: HashMap<String, std::collections::HashSet<String>>,
}

/// 租赁会话状态
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RentalSession {
    partner_session: u64,
    partner_name: String,
    fee: u32,
    period_hours: u32,
    owner_item: Option<mir2_shared::data::item::UserItem>,
    renter_locked: bool,
    owner_locked: bool,
}

/// 已生效的租赁记录
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RentedItem {
    item: mir2_shared::data::item::UserItem,
    owner_name: String,
    renter_name: String,
    rental_fee: u32,
    expiry_timestamp: i64,
}

/// 寄售列表项
#[derive(Debug, Clone)]
pub struct AuctionListing {
    pub auction_id: u64,
    pub seller_name: String,
    pub item: mir2_shared::data::item::UserItem,
    pub price: u32,
    pub consignment_date: i64,
    pub sold: bool,
    pub buyer_name: Option<String>,
    pub item_type: u8, // MarketItemType
}

impl WorldActor {
    pub fn new(gate_ref: ActorRef<GateActor>, map_dir: PathBuf, spawn_dir: Option<PathBuf>, db_pool: DbPool, social_ref: ActorRef<SocialActor>) -> Self {
        Self {
            tick_count: 0,
            players: HashMap::new(),
            buyback_items: HashMap::new(),
            maps: HashMap::new(),
            gate_ref,
            map_dir,
            spawn_dir,
            next_object_id: 1000,
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            respawn_queue: HashMap::new(),
            world_boss_queue: HashMap::new(),
            player_death_queue: HashMap::new(),
            fishing_tick_counters: HashMap::new(),
            ground_items: Vec::new(),
            open_doors: std::collections::HashSet::new(),
            db_pool,
            map_infos: HashMap::new(),
            item_infos: HashMap::new(),
            monster_infos: HashMap::new(),
            monster_drops: HashMap::new(),
            npc_infos: HashMap::new(),
            npc_goods: HashMap::new(),
            npc_scripts: HashMap::new(),
            quest_infos: HashMap::new(),
            magic_infos: HashMap::new(),
            dragon_info: None,
            game_shop_items: Vec::new(),
            movement_index: HashMap::new(),
            social_ref,
            global_exp_multiplier: 1.0,
            global_drop_multiplier: 1.0,
            global_gold_multiplier: 1.0,
            global_exp_event_end_tick: 0,
            global_event_name: None,
            invisible_sessions: HashSet::new(),
            current_light: Self::light_for_hour(chrono::Local::now().hour()),
            auctions: Vec::new(),
            next_auction_id: 1,
            market_search_cache: HashMap::new(),
            rental_sessions: HashMap::new(),
            player_rentals: HashMap::new(),
            guild_wars: HashMap::new(),
        }
    }

    /// 计算全局经验倍率后的经验值
    fn apply_global_exp_multiplier(&self, base: i32) -> i32 {
        if self.tick_count < self.global_exp_event_end_tick {
            (base as f64 * self.global_exp_multiplier).round() as i32
        } else {
            base
        }
    }

    /// 根据小时计算光照设置（基于服务器本地时区）
    fn light_for_hour(hour: u32) -> mir2_shared::enums::LightSetting {
        use mir2_shared::enums::LightSetting;
        match hour {
            0..=4 => LightSetting::Night,
            5..=6 => LightSetting::Dawn,
            7..=16 => LightSetting::Day,
            17..=18 => LightSetting::Evening,
            19..=23 => LightSetting::Night,
            _ => LightSetting::Day,
        }
    }

    /// 发送当前 TimeOfDay 给指定玩家
    fn send_time_of_day(&self, session_id: u64, light: mir2_shared::enums::LightSetting) {
        let packet = mir2_shared::packets::server::player::TimeOfDay { lights: light as u8 };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize TimeOfDay: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TimeOfDay as i16, &body),
        });
    }

    /// 发送 ObjectRemove 给同地图其他玩家，使该玩家从他人视野中消失
    async fn hide_player_from_others(&self, session_id: u64, state: &crate::actors::player::PlayerState) {
        let mut body = Vec::new();
        body.extend_from_slice(&state.object_id.to_le_bytes());
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &body);
        for (sid, record) in &self.players {
            if *sid == session_id { continue; }
            if let Ok(Some(other_state)) = record.actor_ref.ask(GetPlayerState).await {
                if other_state.map_index == state.map_index {
                    let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: packet.clone() });
                }
            }
        }
    }

    /// 发送 ObjectPlayer 给同地图其他玩家，使该玩家重新出现在他人视野中
    async fn reveal_player_to_others(&self, session_id: u64, state: &crate::actors::player::PlayerState) {
        let weapon = state.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(-1);
        let armor = state.inventory.get_equipment(EquipmentSlot::Armour)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(0);
        let weapon_effect = state.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.effect as i16).unwrap_or(0);
        let packet = build_object_player_packet(
            &state.name, state.object_id, state.x, state.y, state.direction, state.level,
            name_colour_for_pk(state.pk_points),
            state.class, state.gender, state.hair,
            weapon, weapon_effect, armor,
            state.mount_type, state.is_mounted,
        );
        for (sid, record) in &self.players {
            if *sid == session_id { continue; }
            if let Ok(Some(other_state)) = record.actor_ref.ask(GetPlayerState).await {
                if other_state.map_index == state.map_index {
                    let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: packet.clone() });
                }
            }
        }
    }

    /// 加载或获取已缓存的地图
    fn get_or_load_map(&mut self, file_name: &str) -> Option<&MapData> {
        if !self.maps.contains_key(&0) || self.maps.get(&0).map(|m| m.file_name != file_name).unwrap_or(true) {
            match loader::load_map(file_name, &self.map_dir) {
                Ok(mut map) => {
                    info!("Loaded map: {} ({}x{})", map.file_name, map.width, map.height);
                    // 应用 DB 中的 no_fight（整图安全区）
                    if let Some(mi) = self.map_infos.values().find(|m| m.file_name == file_name) {
                        if mi.no_fight {
                            map.safe_zone_rects.push((0, 0, map.width as i32 - 1, map.height as i32 - 1));
                        }
                    }
                    // 硬编码常见地图安全区（可后续迁移到配置）
                    Self::apply_hardcoded_safe_zones(file_name, &mut map);
                    self.maps.insert(0, map);
                }
                Err(e) => {
                    warn!("Failed to load map '{}': {}", file_name, e);
                    return None;
                }
            }
        }
        self.maps.get(&0)
    }

    /// 为已知地图注入默认安全区（坐标为 Mir2 经典值）
    fn apply_hardcoded_safe_zones(file_name: &str, map: &mut MapData) {
        let name = file_name.to_lowercase();
        if name.contains("0") || name.contains("bichon") {
            // 比奇省安全区（新手村附近）
            map.safe_zone_rects.push((260, 245, 295, 285));
        }
        if name.contains("3") || name.contains("mongchon") || name.contains("pranja") {
            // 盟重省安全区
            map.safe_zone_rects.push((325, 255, 360, 295));
        }
    }

    /// 分配下一个对象 ID
    fn alloc_object_id(&mut self) -> u32 {
        let id = self.next_object_id;
        self.next_object_id += 1;
        id
    }

    /// 获取所有其他玩家的引用（排除指定 session）
    fn other_players(&self, exclude_session: u64) -> Vec<&PlayerRecord> {
        self.players.values()
            .filter(|r| r.session_id != exclude_session)
            .collect()
    }

    /// 发送 NPC 商店商品列表（Phase 1：空列表，仅打开 UI）
    fn send_npc_goods(&self, session_id: u64, npc: &NpcState) {
        // Use DB rate if available, default 1.0
        let rate = if npc.db_index > 0 {
            self.npc_infos.get(&npc.db_index).map(|n| n.rate as f32 / 100.0).unwrap_or(1.0)
        } else {
            1.0
        };

        let goods = self.npc_goods.get(&npc.db_index).cloned().unwrap_or_default();

        let mut items = Vec::new();
        for good in &goods {
            let mut item = mir2_shared::data::item::UserItem {
                item_index: good.item_index,
                count: good.count as u16,
                ..Default::default()
            };
            // 填充耐久（如果有物品配置）
            if let Some(info) = self.item_infos.get(&good.item_index) {
                item.max_dura = info.durability as u16;
                item.current_dura = info.durability as u16;
            }
            items.push(item);
        }

        let npc_goods_packet = mir2_shared::packets::server::npc_interaction::NPCGoods {
            list: items,
            rate,
            panel_type: mir2_shared::enums::PanelType::Buy,
            hide_added_stats: false,
        };

        let mut body = Vec::new();
        if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &npc_goods_packet) {
            warn!("Failed to serialize NPCGoods: {}", e);
            return;
        }
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCGoods as i16, &body);
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });
        debug!("Sent {} goods from NPC '{}' (rate={}) to session {}", goods.len(), npc.name, rate, session_id);
    }

    /// 发送 NPC 面板（出售/修理等，空商品列表）
    fn send_npc_panel(&self, session_id: u64, panel_type: mir2_shared::enums::PanelType) {
        let packet = mir2_shared::packets::server::npc_interaction::NPCGoods {
            list: Vec::new(),
            rate: 1.0,
            panel_type,
            hide_added_stats: false,
        };
        let mut body = Vec::new();
        if let Err(e) = mir2_shared::packets::base::serialize_packet(
            &mut std::io::Cursor::new(&mut body), &packet) {
            warn!("Failed to serialize NPCGoods panel: {}", e);
            return;
        }
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCGoods as i16, &body);
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });
        debug!("Sent NPC panel {:?} to session {}", panel_type, session_id);
    }

    /// 发送仓库内容给客户端（打开仓库 UI）
    fn send_user_storage(&self, session_id: u64, storage: &[Option<crate::actors::inventory::InventorySlot>]) {
        let items: Vec<Option<mir2_shared::data::item::UserItem>> = storage.iter()
            .map(|slot| slot.as_ref().map(|s| s.item.clone()))
            .collect();

        let packet = mir2_shared::packets::server::player::UserStorage { storage: items };
        let mut body = Vec::new();
        if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet) {
            warn!("Failed to serialize UserStorage: {}", e);
            return;
        }
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserStorage as i16, &body);
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });
        debug!("Sent UserStorage to session {}", session_id);
    }

    /// 发送 CombineItem 响应给客户端
    fn send_combine_item_response(
        &self,
        session_id: u64,
        id_from: u64,
        id_to: u64,
        success: bool,
        destroy: bool,
    ) {
        let packet = mir2_shared::packets::server::item_operations::CombineItem {
            grid: mir2_shared::enums::MirGridType::Inventory,
            id_from,
            id_to,
            success,
            destroy,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize CombineItem: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CombineItem as i16, &body),
        });
    }

    /// 广播玩家外观更新给同地图的其他玩家
    async fn broadcast_player_appearance(&self,
        session_id: u64,
        state: &crate::actors::player::PlayerState,
    ) {
        // 隐身玩家不广播外观变化
        if self.invisible_sessions.contains(&session_id) { return; }
        let weapon = state.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(-1);
        let armor = state.inventory.get_equipment(EquipmentSlot::Armour)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.shape as i16).unwrap_or(0);
        let weapon_effect = state.inventory.get_equipment(EquipmentSlot::Weapon)
            .and_then(|item| self.item_infos.get(&item.item_index))
            .map(|info| info.effect as i16).unwrap_or(0);
        let packet = build_object_player_packet(
            &state.name, state.object_id, state.x, state.y, state.direction, state.level,
            name_colour_for_pk(state.pk_points),
            state.class, state.gender, state.hair,
            weapon, weapon_effect, armor,
            state.mount_type, state.is_mounted,
        );
        let player_map_index = state.map_index;
        for (sid, other_record) in &self.players {
            if *sid == session_id { continue; }
            if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                if other_state.map_index != player_map_index { continue; }
            }
            let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: packet.clone() });
        }
    }

    /// 强制玩家下坐骑并广播外观更新
    async fn dismount_player(&mut self,
        session_id: u64,
    ) {
        let Some(record) = self.players.get(&session_id) else { return };
        let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await else { return };
        if !state.is_mounted { return; }
        state.is_mounted = false;
        state.mount_type = 0;
        let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
        self.broadcast_player_appearance(session_id, &state).await;
    }

    /// 怪物死亡时生成掉落并广播给所有在线玩家
    async fn spawn_single_drop(&mut self, monster: &MonsterState, item_index: i32, count: u16) {
        let drop_oid = self.alloc_object_id();
        if item_index == 0 {
            let gold = count as u32;
            let object_gold = mir2_shared::packets::server::ObjectGold {
                object_id: drop_oid,
                gold,
                location_x: monster.x,
                location_y: monster.y,
            };
            let mut buf = Vec::new();
            if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_gold) {
                warn!("Failed to serialize ObjectGold: {}", e);
                return;
            }
            for session_id in self.players.keys() {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: *session_id,
                    data: buf.clone(),
                });
            }
            self.ground_items.push(GroundItem {
                object_id: drop_oid,
                item: mir2_shared::data::item::UserItem {
                    item_index: 0,
                    count,
                    ..Default::default()
                },
                x: monster.x,
                y: monster.y,
                map_index: monster.map_index,
                dropper_session: None,
                drop_tick: self.tick_count,
            });
            debug!("Monster '{}' dropped {} gold at ({}, {})", monster.name, gold, monster.x, monster.y);
        } else {
            let mut item = mir2_shared::data::item::UserItem {
                item_index,
                unique_id: generate_item_uid(),
                count,
                ..Default::default()
            };
            if let Some(info) = self.item_infos.get(&item_index) {
                item.max_dura = info.durability as u16;
                item.current_dura = info.durability as u16;
            }
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: item.clone(),
                location_x: monster.x,
                location_y: monster.y,
            };
            let mut buf = Vec::new();
            if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item) {
                warn!("Failed to serialize ObjectItem: {}", e);
                return;
            }
            for session_id in self.players.keys() {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: *session_id,
                    data: buf.clone(),
                });
            }
            self.ground_items.push(GroundItem {
                object_id: drop_oid,
                item,
                x: monster.x,
                y: monster.y,
                map_index: monster.map_index,
                dropper_session: None,
                drop_tick: self.tick_count,
            });
            debug!("Monster '{}' dropped item index={} count={} at ({}, {})", monster.name, item_index, count, monster.x, monster.y);
        }
    }

    async fn spawn_monster_drops(&mut self, monster: &MonsterState) {
        let drops = match self.monster_drops.get(&monster.monster_index) {
            Some(d) if !d.is_empty() => d.clone(),
            _ => return,
        };

        let count_mul = drop_count_multiplier(monster.is_boss, monster.is_elite);
        let global_drop_mul = if self.tick_count < self.global_exp_event_end_tick {
            self.global_drop_multiplier
        } else { 1.0 };

        for drop in &drops {
            let roll = fastrand::f64();
            if roll > drop.chance {
                continue;
            }
            let count = if drop.max_count > drop.min_count {
                fastrand::u16(drop.min_count..=drop.max_count).saturating_mul(count_mul)
            } else {
                drop.min_count.saturating_mul(count_mul)
            };
            let adjusted = (count as f64 * global_drop_mul).round() as u16;
            self.spawn_single_drop(monster, drop.item_index, adjusted.max(1)).await;
        }

        // 精英怪额外掉落判定（50% 原概率）
        if monster.is_elite {
            for drop in &drops {
                let bonus_chance = drop.chance * 0.5;
                let roll = fastrand::f64();
                if roll > bonus_chance {
                    continue;
                }
                let count = if drop.max_count > drop.min_count {
                    fastrand::u16(drop.min_count..=drop.max_count)
                } else {
                    drop.min_count
                };
                let adjusted = (count as f64 * global_drop_mul).round() as u16;
                self.spawn_single_drop(monster, drop.item_index, adjusted.max(1)).await;
            }
        }

        // 世界Boss额外掉落：大量金币 + 全掉落保底
        if monster.is_boss {
            let global_gold_mul = if self.tick_count < self.global_exp_event_end_tick {
                self.global_gold_multiplier
            } else { 1.0 };
            let gold_drop = (fastrand::u32(5000..=20000) as f64 * global_gold_mul).round() as u64;
            self.spawn_single_drop(monster, 0, gold_drop as u16).await;
            for drop in &drops {
                let count = if drop.max_count > drop.min_count {
                    fastrand::u16(drop.min_count..=drop.max_count).saturating_mul(2)
                } else {
                    drop.min_count.saturating_mul(2)
                };
                let adjusted = (count as f64 * global_drop_mul).round() as u16;
                self.spawn_single_drop(monster, drop.item_index, adjusted.max(1)).await;
            }
        }
    }

    /// 玩家死亡时随机掉落背包物品和金币（安全区不掉落）
    async fn handle_player_death_drop(
        &mut self,
        session_id: u64,
        x: i32,
        y: i32,
        map_index: u16,
    ) {
        // 安全区不掉落
        if self.maps.get(&map_index).map(|m| m.is_safe_zone(x, y)).unwrap_or(false) {
            return;
        }
        let actor_ref = match self.players.get(&session_id) {
            Some(r) => r.actor_ref.clone(),
            None => return,
        };

        // 掉落背包物品（0-2 个）
        let dropped = actor_ref.ask(crate::actors::player::DropRandomItemsOnDeath).await.unwrap_or_default();
        for item in dropped {
            let drop_oid = self.alloc_object_id();
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: item.clone(),
                location_x: x,
                location_y: y,
            };
            let mut buf = Vec::new();
            if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item) {
                warn!("Failed to serialize ObjectItem: {}", e);
                continue;
            }
            for sid in self.players.keys() {
                let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: buf.clone() });
            }
            self.ground_items.push(GroundItem { object_id: drop_oid, item, x, y, map_index, dropper_session: Some(session_id), drop_tick: self.tick_count });
        }

        // 掉落金币（1-5%）
        if let Ok(Some(state)) = actor_ref.ask(GetPlayerState).await {
            let pct = fastrand::u8(1..=5);
            let gold_drop = state.inventory.gold * pct as u64 / 100;
            if gold_drop > 0 {
                let _ = actor_ref.ask(crate::actors::player::DeductGold { amount: gold_drop }).await;
                let drop_oid = self.alloc_object_id();
                let object_gold = mir2_shared::packets::server::ObjectGold {
                    object_id: drop_oid,
                    gold: gold_drop as u32,
                    location_x: x,
                    location_y: y,
                };
                let mut buf = Vec::new();
                if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_gold) {
                    warn!("Failed to serialize ObjectGold: {}", e);
                } else {
                    for sid in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: buf.clone() });
                    }
                    self.ground_items.push(GroundItem {
                        object_id: drop_oid,
                        item: mir2_shared::data::item::UserItem {
                            item_index: 0,
                            count: gold_drop as u16,
                            ..Default::default()
                        },
                        x, y, map_index, dropper_session: Some(session_id),
                        drop_tick: self.tick_count,
                    });
                }
            }
        }
    }

    /// 执行 NPC 脚本行，解析条件命令与动作命令
    /// 返回 (显示文本, GOTO 目标页面名)
    async fn eval_npc_script(
        &mut self,
        lines: &mut [String],
        session_id: u64,
        npc: &NpcState,
    ) -> (Vec<String>, Option<String>) {
        let mut output = Vec::new();
        let mut skip = false;
        let mut goto_target: Option<String> = None;

        for line in lines.iter_mut() {
            let t = line.trim();
            if t.starts_with('<') && t.ends_with('>') {
                let inner = &t[1..t.len() - 1];
                let mut parts = inner.split_whitespace();
                let cmd = match parts.next() {
                    Some(c) => c.to_uppercase(),
                    None => continue,
                };

                if skip {
                    if cmd == "END" || cmd == "ENDIF" {
                        skip = false;
                    }
                    continue;
                }

                match cmd.as_str() {
                    "CHECKLEVEL" => {
                        let min = parts.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);
                        let max = parts.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(u16::MAX);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.level < min || state.level > max {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKCLASS" => {
                        let mask = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let class_bit = 1u8 << (state.class as u8);
                                if class_bit & mask == 0 {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKGENDER" => {
                        let required = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.gender as u8 != required {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKGOLD" => {
                        let amount = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.inventory.gold < amount {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKITEM" => {
                        let item_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let count = parts.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
                        if let Some(record) = self.players.get(&session_id) {
                            let has = record.actor_ref.ask(crate::actors::player::HasItem {
                                item_index, count,
                            }).await.unwrap_or(false);
                            if !has {
                                skip = true;
                            }
                        }
                    }
                    "CHECKQUEST" => {
                        let quest_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let required_state = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
                        if let Some(record) = self.players.get(&session_id) {
                            let actual_state = record.actor_ref.ask(crate::actors::player::CheckQuestState {
                                quest_index,
                            }).await.unwrap_or(0);
                            if actual_state != required_state {
                                skip = true;
                            }
                        }
                    }
                    "CHECKQUESTTIME" => {
                        let quest_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);
                                let expired = state.quest_log.quests.iter().any(|q| {
                                    q.quest_index == quest_index
                                        && q.time_limit_seconds > 0
                                        && matches!(q.status, QuestStatus::InProgress | QuestStatus::Accepted)
                                        && now.saturating_sub(q.start_time) >= q.time_limit_seconds as u64
                                });
                                if expired {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKPKPOINT" => {
                        let min = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let max = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(i32::MAX);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.pk_points < min || state.pk_points > max {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKGUILD" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.guild_name.is_none() {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKSPOUSE" => {
                        let required = parts.next().unwrap_or("").to_string();
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if required.is_empty() {
                                    // 空字符串 = 检查是否有任何配偶
                                    if state.spouse_name.is_none() {
                                        skip = true;
                                    }
                                } else if state.spouse_name.as_ref().map(|s| s.as_str()) != Some(&required) {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKMENTOR" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.mentor_name.is_none() {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKREINCARNATION" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.reincarnation_host.is_none() && !state.reincarnation_ready {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKMOUNTED" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if !state.is_mounted {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKFISHING" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if !state.is_fishing {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKPET" => {
                        let required_type = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let matches = match state.creature_log.active_creature {
                                    Some(ref c) if c.enabled => {
                                        if required_type == 0 {
                                            true // any pet
                                        } else {
                                            c.creature_type as u8 == required_type
                                        }
                                    }
                                    _ => false,
                                };
                                if !matches {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKPETFOOD" => {
                        let min_hunger = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(20);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let enough = match state.creature_log.active_creature {
                                    Some(ref c) if c.enabled => c.hunger >= min_hunger,
                                    _ => false,
                                };
                                if !enough {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKBUFF" => {
                        let buff_type_str = parts.next().unwrap_or("").to_uppercase();
                        let target_buff = match buff_type_str.as_str() {
                            "HPREGEN" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::HpRegen { amount_per_tick: 0 })),
                            "MPREGEN" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::MpRegen { amount_per_tick: 0 })),
                            "ATTACK" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::AttackBoost { bonus: 0 })),
                            "DEFENSE" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::DefenseBoost { bonus: 0 })),
                            "POISON" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::Poison { damage_per_tick: 0 })),
                            "SILENCE" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::Silence)),
                            "STUN" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::Stun)),
                            "INVISIBILITY" => Some(std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility)),
                            _ => None,
                        };
                        if let Some(target_tag) = target_buff {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let has_buff = state.buffs.iter().any(|b| std::mem::discriminant(&b.buff_type) == target_tag);
                                    if !has_buff {
                                        skip = true;
                                    }
                                }
                            }
                        }
                    }
                    "CHECKWEAPON" => {
                        let required_index = parts.next().and_then(|s| s.parse::<i32>().ok());
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let weapon = state.inventory.get_equipment(crate::actors::inventory::EquipmentSlot::Weapon);
                                let matches = match required_index {
                                    Some(idx) => weapon.map(|w| w.item_index == idx).unwrap_or(false),
                                    None => weapon.is_some(),
                                };
                                if !matches {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKMAP" => {
                        let map_name = parts.next().unwrap_or("").to_string();
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let current_map = self.map_infos.get(&(state.map_index as i32))
                                    .map(|m| m.file_name.as_str())
                                    .unwrap_or("");
                                if !current_map.eq_ignore_ascii_case(&map_name) {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKDIRECTION" => {
                        let required = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.direction != required {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKNEARBY" => {
                        let distance = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(10);
                        let min_count = parts.next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let mut nearby = 0usize;
                                for (_, other) in &self.players {
                                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                                        if os.session_id == state.session_id { continue; }
                                        if os.map_index != state.map_index { continue; }
                                        let dist = (state.x - os.x).abs() + (state.y - os.y).abs();
                                        if dist <= distance {
                                            nearby += 1;
                                        }
                                    }
                                }
                                if nearby < min_count {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKEXP" => {
                        let min = parts.next().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                        let max = parts.next().and_then(|s| s.parse::<i64>().ok()).unwrap_or(i64::MAX);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.experience < min || state.experience > max {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKHP" => {
                        let min = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let max = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(i32::MAX);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.hp < min || state.hp > max {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "CHECKTIME" => {
                        let min_hour = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        let max_hour = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(23);
                        let now = chrono::Local::now();
                        let hour = now.hour();
                        if hour < min_hour || hour > max_hour {
                            skip = true;
                        }
                    }
                    "CHECKDAY" => {
                        let day_name = parts.next().unwrap_or("").to_lowercase();
                        let today = chrono::Local::now().format("%A").to_string().to_lowercase();
                        if today != day_name {
                            skip = true;
                        }
                    }
                    "RAND" => {
                        let n = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        if n > 1 {
                            let roll = fastrand::u32(1..=n);
                            if roll != 1 {
                                skip = true;
                            }
                        }
                    }
                    "CHECKMP" => {
                        let min = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let max = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(i32::MAX);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.mp < min || state.mp > max {
                                    skip = true;
                                }
                            }
                        }
                    }
                    "TAKEGOLD" => {
                        let amount = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::DeductGold { amount }).await;
                        }
                    }
                    "GIVEGOLD" => {
                        let amount = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount }).await;
                        }
                    }
                    "TAKEITEM" => {
                        let item_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let count = parts.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
                        if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::RemoveItemByIndex {
                                item_index, count,
                            }).await;
                        }
                    }
                    "TAKEPETFOOD" => {
                        let item_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let count = parts.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
                        if let Some(record) = self.players.get(&session_id) {
                            let removed = record.actor_ref.ask(crate::actors::player::RemoveItemByIndex {
                                item_index, count,
                            }).await.unwrap_or(false);
                            if !removed {
                                send_system_message(&self.gate_ref, session_id, "你没有足够的宠物食物");
                            }
                        }
                    }
                    "GIVEITEM" => {
                        let item_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let count = parts.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
                        if let Some(record) = self.players.get(&session_id) {
                            let mut item = mir2_shared::data::item::UserItem {
                                item_index,
                                count,
                                ..Default::default()
                            };
                            if let Some(info) = self.item_infos.get(&item_index) {
                                item.max_dura = info.durability as u16;
                                item.current_dura = info.durability as u16;
                            }
                            let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory {
                                item,
                            }).await;
                            let updates = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await.unwrap_or_default();
                            if !updates.is_empty() {
                                send_system_message(&self.gate_ref, session_id, "任务进度更新：获得物品");
                            }
                        }
                    }
                    "REPAIR" => {
                        if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::RepairAllEquipment).await;
                        }
                    }
                    "RESURRECT" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.is_dead {
                                    let _ = record.actor_ref.ask(crate::actors::player::Revive).await;
                                    send_system_message(&self.gate_ref, session_id, "你已复活！");
                                    debug!("NPC resurrect: {}", state.name);
                                }
                            }
                        }
                    }
                    "HEAL" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.hp < state.max_hp || state.mp < state.max_mp {
                                    state.hp = state.max_hp;
                                    state.mp = state.max_mp;
                                    let (hp, mp) = (state.hp, state.mp);
                                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerState { state }).await;
                                    let mut body = Vec::new();
                                    body.extend_from_slice(&hp.to_le_bytes());
                                    body.extend_from_slice(&mp.to_le_bytes());
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id,
                                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
                                    });
                                    send_system_message(&self.gate_ref, session_id, "你的生命和魔法已恢复！");
                                }
                            }
                        }
                    }
                    "STORAGE" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                self.send_user_storage(session_id, &state.inventory.storage);
                            }
                        }
                    }
                    "GIVEEXP" => {
                        let amount = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if amount > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                let _ = record.actor_ref.ask(crate::actors::player::AddExperience { amount: self.apply_global_exp_multiplier(amount) }).await;
                            }
                        }
                    }
                    "GIVEBUFF" => {
                        let buff_type_str = parts.next().unwrap_or("").to_uppercase();
                        let duration = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(30);
                        let interval = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        let power = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let buff_type = match buff_type_str.as_str() {
                            "HPREGEN" => Some(crate::combat::buff::BuffType::HpRegen { amount_per_tick: power.max(1) }),
                            "MPREGEN" => Some(crate::combat::buff::BuffType::MpRegen { amount_per_tick: power.max(1) }),
                            "ATTACK" => Some(crate::combat::buff::BuffType::AttackBoost { bonus: power }),
                            "DEFENSE" => Some(crate::combat::buff::BuffType::DefenseBoost { bonus: power }),
                            "POISON" => Some(crate::combat::buff::BuffType::Poison { damage_per_tick: power.max(1) }),
                            "SILENCE" => Some(crate::combat::buff::BuffType::Silence),
                            "STUN" => Some(crate::combat::buff::BuffType::Stun),
                            "INVISIBILITY" => Some(crate::combat::buff::BuffType::Invisibility),
                            _ => None,
                        };
                        if let Some(bt) = buff_type {
                            let is_invis = matches!(bt, crate::combat::buff::BuffType::Invisibility);
                            let buff = crate::combat::buff::BuffInstance::new(bt, duration, interval);
                            if let Some(record) = self.players.get(&session_id) {
                                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                                if is_invis {
                                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                        self.invisible_sessions.insert(session_id);
                                        self.hide_player_from_others(session_id, &state).await;
                                        send_system_message(&self.gate_ref, session_id, "你进入了隐身状态");
                                    }
                                }
                            }
                        }
                    }
                    "GIVESKILL" => {
                        let spell = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if spell > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                let mut added = false;
                                if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                    if !state.magics.iter().any(|m| m.spell == spell) {
                                        state.magics.push(crate::actors::player::PlayerMagic::new(spell));
                                        let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
                                        added = true;
                                    }
                                }
                                if added {
                                    // 发送 NewMagic 包给客户端
                                    if let Some(info) = self.magic_infos.get(&(spell as u32)) {
                                        let magic = mir2_shared::data::client_data::ClientMagic {
                                            name: info.name.clone(),
                                            spell: mir2_shared::enums::Spell::try_from(spell as u8).unwrap_or(mir2_shared::enums::Spell::None),
                                            base_cost: info.base_cost as u8,
                                            level_cost: info.level_cost as u8,
                                            icon: info.icon as u8,
                                            level1: info.level1 as u8,
                                            level2: info.level2 as u8,
                                            level3: info.level3 as u8,
                                            need1: info.need1 as u16,
                                            need2: info.need2 as u16,
                                            need3: info.need3 as u16,
                                            level: 0,
                                            key: 0,
                                            experience: 0,
                                            delay: info.delay_base as i64,
                                            range: info.range as u8,
                                            cast_time: 0,
                                        };
                                        let new_magic = mir2_shared::packets::server::magic::NewMagic { magic, hero: false };
                                        let mut body = Vec::new();
                                        if new_magic.write_body(&mut body).is_ok() {
                                            let _ = self.gate_ref.ask(SendToClient {
                                                session_id,
                                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewMagic as i16, &body),
                                            });
                                        }
                                    }
                                    send_system_message(&self.gate_ref, session_id, "你学会了一项新技能！");
                                    debug!("GIVESKILL: session={} spell={}", session_id, spell);
                                } else {
                                    send_system_message(&self.gate_ref, session_id, "你已经学会了这个技能");
                                }
                            }
                        }
                    }
                    "CHECKSKILL" => {
                        let spell = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let min_level = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        let max_level = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(255);
                        if spell > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let has_skill = state.magics.iter().any(|m| m.spell == spell && m.level >= min_level && m.level <= max_level);
                                    if !has_skill {
                                        skip = true;
                                    }
                                }
                            }
                        }
                    }
                    "UPGRADESKILL" => {
                        let spell = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if spell > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                let mut upgraded = false;
                                if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                    if let Some(magic) = state.magics.iter_mut().find(|m| m.spell == spell) {
                                        if magic.level < 3 {
                                            magic.level += 1;
                                            upgraded = true;
                                        }
                                    }
                                    if upgraded {
                                        let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
                                        // 发送 MagicLeveled 包
                                        let spell_enum = mir2_shared::enums::Spell::try_from(spell as u8).unwrap_or(mir2_shared::enums::Spell::None);
                                        let leveled = mir2_shared::packets::server::magic::MagicLeveled { spell: spell_enum, level: state.magics.iter().find(|m| m.spell == spell).map(|m| m.level).unwrap_or(0), hero: false };
                                        let mut body = Vec::new();
                                        if leveled.write_body(&mut body).is_ok() {
                                            let _ = self.gate_ref.ask(SendToClient {
                                                session_id,
                                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MagicLeveled as i16, &body),
                                            });
                                        }
                                        send_system_message(&self.gate_ref, session_id, "技能升级成功！");
                                        debug!("UPGRADESKILL: session={} spell={} level={}", session_id, spell, state.magics.iter().find(|m| m.spell == spell).map(|m| m.level).unwrap_or(0));
                                    } else {
                                        send_system_message(&self.gate_ref, session_id, "技能已达到最高等级或未学习");
                                    }
                                }
                            }
                        }
                    }
                    "SETFLAG" => {
                        let key = parts.next().unwrap_or("").to_string();
                        let value = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if !key.is_empty() {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                    state.flags.insert(key, value);
                                    let _ = record.actor_ref.ask(SetPlayerState { state }).await;
                                }
                            }
                        }
                    }
                    "CHECKFLAG" => {
                        let key = parts.next().unwrap_or("").to_string();
                        let min = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let max = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(i32::MAX);
                        if !key.is_empty() {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let flag_val = state.flags.get(&key).copied().unwrap_or(0);
                                    if flag_val < min || flag_val > max {
                                        skip = true;
                                    }
                                }
                            }
                        }
                    }
                    "INCFLAG" => {
                        let key = parts.next().unwrap_or("").to_string();
                        let amount = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
                        if !key.is_empty() {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let new_val = state.flags.get(&key).copied().unwrap_or(0).saturating_add(amount);
                                    state.flags.insert(key, new_val);
                                    let _ = record.actor_ref.ask(SetPlayerState { state }).await;
                                }
                            }
                        }
                    }
                    "DELFLAG" => {
                        let key = parts.next().unwrap_or("").to_string();
                        if !key.is_empty() {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                    state.flags.remove(&key);
                                    let _ = record.actor_ref.ask(SetPlayerState { state }).await;
                                }
                            }
                        }
                    }
                    "ACCEPTQUEST" => {
                        let quest_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if quest_index > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                // Check quest exists
                                let Some(quest_db) = self.quest_infos.get(&quest_index) else {
                                    send_system_message(&self.gate_ref, session_id, "任务不存在");
                                    continue;
                                };
                                // Check level
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    if state.level < quest_db.required_min_level as u16 {
                                        send_system_message(&self.gate_ref, session_id, "等级不足");
                                        continue;
                                    }
                                    if quest_db.required_max_level > 0 && state.level > quest_db.required_max_level as u16 {
                                        send_system_message(&self.gate_ref, session_id, "等级过高");
                                        continue;
                                    }
                                }
                                // Check not already accepted or completed
                                if let Ok(Some(_)) = record.actor_ref.ask(GetQuest { quest_index }).await {
                                    send_system_message(&self.gate_ref, session_id, "该任务已接受");
                                    continue;
                                }
                                if let Ok(true) = record.actor_ref.ask(HasCompletedQuest { quest_index }).await {
                                    send_system_message(&self.gate_ref, session_id, "该任务已完成");
                                    continue;
                                }
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);
                                let quest = make_quest_instance(quest_db, now);
                                if let Ok(true) = record.actor_ref.ask(AcceptQuest { quest }).await {
                                    send_system_message(&self.gate_ref, session_id, "任务已接受");
                                }
                            }
                        }
                    }
                    "COMPLETEQUEST" => {
                        let quest_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if quest_index > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                let completed_quest = match record.actor_ref.ask(CompleteQuest { quest_index }).await {
                                    Ok(Some(q)) => q,
                                    _ => {
                                        send_system_message(&self.gate_ref, session_id, "无法完成任务");
                                        continue;
                                    }
                                };
                                if completed_quest.exp_reward > 0 {
                                    let _ = record.actor_ref.ask(AddExperience { amount: self.apply_global_exp_multiplier(completed_quest.exp_reward as i32) }).await;
                                }
                                if completed_quest.gold_reward > 0 {
                                    let _ = record.actor_ref.ask(AddGold { amount: completed_quest.gold_reward }).await;
                                }
                                if let Some(quest_db) = self.quest_infos.get(&quest_index) {
                                    for reward in &quest_db.fixed_rewards {
                                        let mut item = mir2_shared::data::item::UserItem {
                                            item_index: reward.item_index,
                                            count: reward.count,
                                            ..Default::default()
                                        };
                                        if let Some(info) = self.item_infos.get(&reward.item_index) {
                                            item.max_dura = info.durability as u16;
                                            item.current_dura = info.durability as u16;
                                        }
                                        let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
                                    }
                                    if !quest_db.fixed_rewards.is_empty() {
                                        let _ = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await;
                                    }
                                }
                                send_system_message(&self.gate_ref, session_id, &format!("任务完成！获得 {} 经验，{} 金币", completed_quest.exp_reward, completed_quest.gold_reward));
                                send_quest_complete_packet(&self.gate_ref, session_id, completed_quest.quest_index);
                            }
                        }
                    }
                    "ABANDONQUEST" => {
                        let quest_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        if quest_index > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(true) = record.actor_ref.ask(AbandonQuest { quest_index }).await {
                                    send_system_message(&self.gate_ref, session_id, "任务已放弃");
                                }
                            }
                        }
                    }
                    "MOUNT" => {
                        let mount_type = parts.next().and_then(|s| s.parse::<i16>().ok()).unwrap_or(1);
                        if mount_type > 0 {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                    if !state.is_mounted {
                                        // Check map mount restrictions
                                        if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                                            if mi.no_mount {
                                                send_system_message(&self.gate_ref, session_id, "该地图禁止骑乘坐骑");
                                                continue;
                                            }
                                            if mi.need_bridle {
                                                let has_bridle = state.inventory.backpack.iter().any(|slot| {
                                                    slot.as_ref().and_then(|s| {
                                                        self.item_infos.get(&s.item.item_index)
                                                            .map(|info| {
                                                                let name = info.name.to_lowercase();
                                                                name.contains("bridle") || name.contains("马鞭")
                                                            })
                                                    }).unwrap_or(false)
                                                });
                                                if !has_bridle {
                                                    send_system_message(&self.gate_ref, session_id, "你需要马鞭才能在此地图骑乘坐骑");
                                                    continue;
                                                }
                                            }
                                        }
                                        state.is_mounted = true;
                                        state.mount_type = mount_type;
                                        let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
                                        self.broadcast_player_appearance(session_id, &state).await;
                                        send_system_message(&self.gate_ref, session_id, "你骑上了坐骑");
                                    }
                                }
                            }
                        }
                    }
                    "DISMOUNT" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.is_mounted {
                                    state.is_mounted = false;
                                    state.mount_type = 0;
                                    let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
                                    self.broadcast_player_appearance(session_id, &state).await;
                                    send_system_message(&self.gate_ref, session_id, "你下了坐骑");
                                }
                            }
                        }
                    }
                    "SPAWNWORLDBOSS" => {
                        let monster_index = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        let map_name = parts.next().unwrap_or("");
                        let bx = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(330);
                        let by = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(330);
                        if monster_index > 0 {
                            if let Some(monster_info) = self.monster_infos.get(&monster_index).cloned() {
                                let target_map_index = self.map_infos.values()
                                    .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                                    .map(|m| m.index as u16)
                                    .unwrap_or(0);
                                if target_map_index == 0 {
                                    send_system_message(&self.gate_ref, session_id, "地图不存在");
                                    continue;
                                }
                                let walkable = self.maps.get(&target_map_index)
                                    .map(|m| m.is_walkable(bx, by))
                                    .unwrap_or(false);
                                if !walkable {
                                    send_system_message(&self.gate_ref, session_id, "该坐标不可行走");
                                    continue;
                                }
                                let boss_oid = self.alloc_object_id();
                                let boss_hp = monster_info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(100).saturating_mul(10);
                                let boss_min_dmg = monster_info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5).saturating_mul(3);
                                let boss_max_dmg = monster_info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10).saturating_mul(3);
                                let boss_xp = monster_info.experience.saturating_mul(5);
                                let boss = MonsterState {
                                    object_id: boss_oid,
                                    name: format!("[世界Boss] {}", monster_info.name),
                                    image: monster_info.image as u16,
                                    monster_index,
                                    x: bx,
                                    y: by,
                                    direction: 0,
                                    hp: boss_hp,
                                    max_hp: boss_hp,
                                    min_dmg: boss_min_dmg,
                                    max_dmg: boss_max_dmg,
                                    xp: boss_xp,
                                    spawn_x: bx,
                                    spawn_y: by,
                                    map_index: target_map_index,
                                    next_attack_tick: 0,
                                    next_move_tick: 0,
                                    next_summon_tick: 0,
                                    ai_profile: MonsterAiProfile::from_info(&monster_info),
                                    ai_state: MonsterAiState::Idle,
                                    target_session: None,
                                    provoked: true, // Boss is always aggressive
                                    is_elite: false,
                                    is_boss: true,
                                };
                                self.monsters.insert(boss_oid, boss);
                                // 10 minutes = 6000 ticks (100ms each)
                                self.world_boss_queue.insert(boss_oid, self.tick_count + 6000);
                                let packet = build_object_monster_packet(
                                    &MonsterSpawn {
                                        name: format!("[世界Boss] {}", monster_info.name),
                                        image: monster_info.image as u16,
                                        monster_index,
                                        x: bx,
                                        y: by,
                                        direction: 0,
                                        hp: boss_hp,
                                        min_dmg: boss_min_dmg,
                                        max_dmg: boss_max_dmg,
                                        xp: boss_xp,
                                        map_index: target_map_index,
                                    }, boss_oid, &format!("[世界Boss] {}", monster_info.name));
                                for session_id in self.players.keys() {
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id: *session_id,
                                        data: packet.clone(),
                                    });
                                }
                                let map_title = self.map_infos.get(&(target_map_index as i32))
                                    .map(|m| m.title.clone())
                                    .unwrap_or_else(|| map_name.to_string());
                                broadcast_system_message(&self.gate_ref, &self.players,
                                    &format!("⚠️ 世界Boss {} 降临 {}！勇士们，前往讨伐！", monster_info.name, map_title));
                                debug!("World boss '{}' spawned as #{} at ({},{})", monster_info.name, boss_oid, bx, by);
                            }
                        }
                    }
                    "LOCAL" => {
                        let message = parts.collect::<Vec<_>>().join(" ");
                        if !message.is_empty() {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let map_index = state.map_index;
                                    for (sid, other) in &self.players {
                                        if let Ok(Some(other_state)) = other.actor_ref.ask(GetPlayerState).await {
                                            if other_state.map_index == map_index {
                                                send_system_message(&self.gate_ref, *sid, &format!("[本地] {}", message));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "GLOBAL" => {
                        let message = parts.collect::<Vec<_>>().join(" ");
                        if !message.is_empty() {
                            for sid in self.players.keys() {
                                send_system_message(&self.gate_ref, *sid, &format!("[全局] {}", message));
                            }
                        }
                    }
                    "GIVEPETFOOD" => {
                        let amount = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(20);
                        if let Some(record) = self.players.get(&session_id) {
                            let restored = record.actor_ref.ask(RestoreCreatureHunger { amount }).await.unwrap_or(false);
                            if restored {
                                send_system_message(&self.gate_ref, session_id, &format!("宠物吃了食物，饥饿值恢复 {} 点", amount));
                            } else {
                                send_system_message(&self.gate_ref, session_id, "你没有召唤宠物");
                            }
                        }
                    }
                    "REMOVEBUFF" => {
                        let buff_type_str = parts.next().unwrap_or("").to_uppercase();
                        let buff_type = match buff_type_str.as_str() {
                            "HPREGEN" => Some(crate::combat::buff::BuffType::HpRegen { amount_per_tick: 0 }),
                            "MPREGEN" => Some(crate::combat::buff::BuffType::MpRegen { amount_per_tick: 0 }),
                            "ATTACK" => Some(crate::combat::buff::BuffType::AttackBoost { bonus: 0 }),
                            "DEFENSE" => Some(crate::combat::buff::BuffType::DefenseBoost { bonus: 0 }),
                            "POISON" => Some(crate::combat::buff::BuffType::Poison { damage_per_tick: 0 }),
                            "SILENCE" => Some(crate::combat::buff::BuffType::Silence),
                            "STUN" => Some(crate::combat::buff::BuffType::Stun),
                            "INVISIBILITY" => Some(crate::combat::buff::BuffType::Invisibility),
                            _ => None,
                        };
                        if let Some(bt) = buff_type {
                            if let Some(record) = self.players.get(&session_id) {
                                let _ = record.actor_ref.ask(crate::actors::player::RemoveBuff { buff_type: bt }).await;
                            }
                        }
                    }
                    "GIVEPET" => {
                        let creature_type_id = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        let creature_type = crate::actors::creature::CreatureType::from(creature_type_id);
                        if creature_type != crate::actors::creature::CreatureType::None {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let mut log = state.creature_log;
                                    let mut creature = crate::actors::creature::IntelligentCreature::new(creature_type);
                                    creature.enabled = true;
                                    log.set_creature(creature);
                                    let _ = record.actor_ref.ask(crate::actors::player::SetCreature { creature_log: log }).await;
                                    send_system_message(&self.gate_ref, session_id, "获得新宠物！");
                                    debug!("GIVEPET: {} type={:?}", state.name, creature_type);
                                }
                            }
                        }
                    }
                    "SAY" => {
                        let message = parts.collect::<Vec<_>>().join(" ");
                        if !message.is_empty() {
                            send_system_message(&self.gate_ref, session_id, &message);
                        }
                    }
                    "GOTO" => {
                        if let Some(target) = parts.next() {
                            goto_target = Some(target.to_string());
                            break;
                        }
                    }
                    "CLOSE" => {
                        output.clear();
                        break;
                    }
                    "BREAK" => break,
                    "TELEPORT" | "MOVE" => {
                        let map_name = parts.next().unwrap_or("");
                        let tx = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(330);
                        let ty = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(330);
                        // 查找 map_index（通过 file_name 匹配）
                        let target_map_index = self.map_infos.values()
                            .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                            .map(|m| m.index as u16);
                        if let Some(map_index) = target_map_index {
                            if let Some(record) = self.players.get(&session_id) {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                        x: tx,
                                        y: ty,
                                        direction: state.direction,
                                        map_index: Some(map_index),
                                        is_mounted: None,
                                    }).await;
                                    let mut body = Vec::new();
                                    body.extend_from_slice(&tx.to_le_bytes());
                                    body.extend_from_slice(&ty.to_le_bytes());
                                    body.push(state.direction);
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id,
                                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
                                    });
                                    debug!("NPC teleport: session={} to map={} ({},{})", session_id, map_name, tx, ty);
                                }
                            }
                        } else {
                            warn!("NPC teleport: map '{}' not found", map_name);
                        }
                    }
                    "RECALL" => {
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let map_index = self.npc_infos.get(&npc.db_index)
                                    .map(|i| i.map_index as u16)
                                    .unwrap_or(state.map_index);
                                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x: npc.x,
                                    y: npc.y,
                                    direction: state.direction,
                                    map_index: Some(map_index),
                                    is_mounted: None,
                                }).await;
                                let mut body = Vec::new();
                                body.extend_from_slice(&npc.x.to_le_bytes());
                                body.extend_from_slice(&npc.y.to_le_bytes());
                                body.push(state.direction);
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
                                });
                                debug!("NPC recall: session={} to npc {} ({},{})", session_id, npc.name, npc.x, npc.y);
                            }
                        }
                    }
                    "LOTTERY" => {
                        let cost = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(100);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                if state.inventory.gold < cost {
                                    send_system_message(&self.gate_ref, session_id, "金币不足，无法抽奖");
                                    continue;
                                }
                                let deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: cost }).await.unwrap_or(false);
                                if !deducted {
                                    send_system_message(&self.gate_ref, session_id, "金币扣除失败");
                                    continue;
                                }
                                // 抽奖掉落表
                                let roll = fastrand::u32(1..=100);
                                let (item_idx, count, prize_name) = match roll {
                                    1..=50 => (0, 0, ""), // 50% 空手
                                    51..=75 => (1, 1, "经验丹"),
                                    76..=90 => (2, 1, "回城卷"),
                                    91..=97 => (3, 1, "随机传送卷"),
                                    98..=99 => (4, 1, "双倍经验卷"),
                                    100 => (5, 10, "经验丹大礼包"),
                                    _ => (0, 0, ""),
                                };
                                if item_idx > 0 {
                                    let item = crate::actors::inventory::make_item(item_idx, count);
                                    let added = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await.unwrap_or(false);
                                    if added {
                                        send_system_message(
                                            &self.gate_ref, session_id,
                                            &format!("恭喜中奖！获得 {} x{}", prize_name, count));
                                    } else {
                                        send_system_message(
                                            &self.gate_ref, session_id,
                                            &format!("恭喜中奖！但背包已满，{} x{} 无法获得", prize_name, count));
                                    }
                                } else {
                                    send_system_message(
                                        &self.gate_ref, session_id, "很遗憾，这次没有中奖...");
                                }
                                debug!("LOTTERY: session={} roll={} prize={}x{}", session_id, roll, item_idx, count);
                            }
                        }
                    }
                    _ => {}
                }
            } else if !skip {
                output.push(line.clone());
            }
        }
        (output, goto_target)
    }
}

impl Actor for WorldActor {
    type Args = WorldActorArgs;
    type Error = anyhow::Error;

    async fn on_start(
        args: WorldActorArgs,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        info!("WorldActor started (tick interval: {}ms)", args.tick_interval_ms);

        // 启动主循环
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(args.tick_interval_ms));
            loop {
                interval.tick().await;
                let _ = actor_ref.ask(Tick).await;
            }
        });

        // Load guilds from DB (SocialActor handles guild data now)
        let _guilds = match db::load_guilds(&args.db_pool).await {
            Ok(g) => {
                info!("Loaded {} guilds from database", g.len());
                g
            }
            Err(e) => {
                warn!("Failed to load guilds from DB: {}", e);
                HashMap::new()
            }
        };

        // Load game configs from DB (migrated from Server.MirDB)
        let map_infos_list = match db::load_map_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} map configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load map_infos from DB: {}", e); Vec::new() }
        };
        let map_infos: HashMap<i32, db::MapInfo> = map_infos_list.into_iter().map(|m| (m.index, m)).collect();

        let item_infos_list = match db::load_item_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} item configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load item_infos from DB: {}", e); Vec::new() }
        };
        let item_infos: HashMap<i32, db::ItemInfo> = item_infos_list.into_iter().map(|i| (i.index, i)).collect();

        let monster_infos_list = match db::load_monster_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} monster configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load monster_infos from DB: {}", e); Vec::new() }
        };
        let monster_infos: HashMap<i32, db::MonsterInfo> = monster_infos_list.into_iter().map(|m| (m.index, m)).collect();

        let monster_drops = match db::load_monster_drops(&args.db_pool).await {
            Ok(d) => { info!("Loaded drop configs for {} monsters from database", d.len()); d }
            Err(e) => { warn!("Failed to load monster_drops from DB: {}", e); HashMap::new() }
        };

        let npc_infos_list = match db::load_npc_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} NPC configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load npc_infos from DB: {}", e); Vec::new() }
        };
        let npc_infos: HashMap<i32, db::NPCInfo> = npc_infos_list.into_iter().map(|n| (n.index, n)).collect();

        let npc_goods = match db::load_npc_goods(&args.db_pool).await {
            Ok(g) => { info!("Loaded goods for {} NPCs from database", g.len()); g }
            Err(e) => { warn!("Failed to load npc_goods from DB: {}", e); HashMap::new() }
        };

        let npc_scripts = match db::load_npc_scripts(&args.db_pool).await {
            Ok(s) => { info!("Loaded {} NPC script pages from database", s.len()); s }
            Err(e) => { warn!("Failed to load npc_scripts from DB: {}", e); HashMap::new() }
        };

        let mut quest_infos_list = match db::load_quest_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} quest configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load quest_infos from DB: {}", e); Vec::new() }
        };
        db::resolve_quest_tasks(&mut quest_infos_list, &args.quest_dir, &monster_infos, &item_infos);
        let mut resolved_kill = 0usize;
        let mut resolved_item = 0usize;
        let mut resolved_flag = 0usize;
        for q in &quest_infos_list {
            resolved_kill += q.kill_tasks.len();
            resolved_item += q.item_tasks.len();
            resolved_flag += q.flag_tasks.len();
        }
        info!("Resolved {} kill tasks, {} item tasks, {} flag tasks from quest files", resolved_kill, resolved_item, resolved_flag);
        let quest_infos: HashMap<i32, db::QuestInfo> = quest_infos_list.into_iter().map(|q| (q.index, q)).collect();

        let magic_infos_list = match db::load_magic_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} magic configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load magic_infos from DB: {}", e); Vec::new() }
        };
        let magic_infos: HashMap<u32, db::MagicInfo> = magic_infos_list.into_iter().map(|m| (m.spell as u32, m)).collect();

        let dragon_info = match db::load_dragon_info(&args.db_pool, &monster_infos).await {
            Ok(d) => { if d.is_some() { info!("Loaded dragon config from database"); } d }
            Err(e) => { warn!("Failed to load dragon_info from DB: {}", e); None }
        };

        let game_shop_items = match db::load_game_shop_items(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} game shop items from database", m.len()); m }
            Err(e) => { warn!("Failed to load game_shop_items from DB: {}", e); Vec::new() }
        };

        // Load auctions from DB
        let mut auctions = Vec::new();
        let mut next_auction_id = 1u64;
        match db::load_all_auctions(&args.db_pool).await {
            Ok(rows) => {
                for (id, seller, item_json, price, date, sold, item_type, buyer_name) in rows {
                    if let Ok(item) = serde_json::from_str::<mir2_shared::data::item::UserItem>(&item_json) {
                        let aid = id as u64;
                        if aid >= next_auction_id {
                            next_auction_id = aid + 1;
                        }
                        auctions.push(AuctionListing {
                            auction_id: aid,
                            seller_name: seller,
                            item,
                            price: price as u32,
                            consignment_date: date,
                            sold: sold != 0,
                            buyer_name,
                            item_type: item_type as u8,
                        });
                    }
                }
                info!("Loaded {} auctions from database", auctions.len());
            }
            Err(e) => warn!("Failed to load auctions: {}", e),
        }

        // Build movement trigger index for O(1) lookup: (map_index, source_x, source_y) -> MapMovementInfo
        let movement_index: HashMap<(i32, i32, i32), db::MapMovementInfo> = {
            let mut idx = HashMap::new();
            for mi in map_infos.values() {
                for mv in &mi.movements {
                    idx.insert((mi.index, mv.source_x, mv.source_y), mv.clone());
                }
            }
            info!("Indexed {} movement triggers", idx.len());
            idx
        };

        Ok(Self {
            tick_count: 0,
            players: HashMap::new(),
            buyback_items: HashMap::new(),
            maps: HashMap::new(),
            gate_ref: args.gate_ref,
            map_dir: args.map_dir,
            spawn_dir: args.spawn_dir,
            next_object_id: 1000,
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            respawn_queue: HashMap::new(),
            world_boss_queue: HashMap::new(),
            player_death_queue: HashMap::new(),
            fishing_tick_counters: HashMap::new(),
            ground_items: Vec::new(),
            open_doors: std::collections::HashSet::new(),
            db_pool: args.db_pool,
            map_infos,
            item_infos,
            monster_infos,
            monster_drops,
            npc_infos,
            npc_goods,
            npc_scripts,
            quest_infos,
            magic_infos,
            dragon_info,
            game_shop_items,
            movement_index,
            social_ref: args.social_ref,
            global_exp_multiplier: 1.0,
            global_drop_multiplier: 1.0,
            global_gold_multiplier: 1.0,
            global_exp_event_end_tick: 0,
            global_event_name: None,
            invisible_sessions: HashSet::new(),
            current_light: Self::light_for_hour(chrono::Local::now().hour()),
            auctions,
            next_auction_id,
            market_search_cache: HashMap::new(),
            rental_sessions: HashMap::new(),
            player_rentals: HashMap::new(),
            guild_wars: HashMap::new(),
        })
    }
}

// ============================================================
// 消息定义
// ============================================================

/// 游戏主循环 Tick
pub struct Tick;

/// 开始游戏请求（从 GateActor 转发）
pub struct StartGameRequest {
    pub session_id: u64,
    pub character_index: i32,
    pub account_username: String,
}

/// 移动请求（从 GateActor 转发）
pub struct WorldMoveRequest {
    pub session_id: u64,
    pub direction: u8,
    pub is_run: bool,
}

/// 转向请求（从 GateActor 转发）
pub struct WorldTurnRequest {
    pub session_id: u64,
    pub direction: u8,
}

/// 玩家断开连接
pub struct PlayerDisconnected {
    pub session_id: u64,
}

/// 攻击请求（从 GateActor 转发）
pub struct WorldAttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
}

/// 玩家主动登出（从 GateActor 转发）
pub struct PlayerLogOut {
    pub session_id: u64,
}

/// 采集请求（从 GateActor 转发）
pub struct HarvestRequest {
    pub session_id: u64,
    pub direction: u8,
}

/// 聊天请求（从 GateActor 转发）
pub struct ChatRequest {
    pub session_id: u64,
    pub message: String,
}

/// 切换攻击模式请求（从 GateActor 转发）
pub struct ChangeAModeRequest {
    pub session_id: u64,
    pub mode: mir2_shared::enums::AttackMode,
}

/// 切换宠物模式请求（从 GateActor 转发）
pub struct ChangePModeRequest {
    pub session_id: u64,
    pub mode: mir2_shared::enums::PetMode,
}

/// 设置技能快捷键请求（从 GateActor 转发）
pub struct SetSpellKeyRequest {
    pub session_id: u64,
    pub spell: i32,
    pub key: u8,
    pub old_key: u8,
}

/// 技能开关切换请求（从 GateActor 转发）
pub struct SpellToggleRequest {
    pub session_id: u64,
    pub spell: i32,
    pub can_use: i8,
}

/// 设置英雄行为模式请求（从 GateActor 转发）
pub struct SetHeroBehaviourRequest {
    pub session_id: u64,
    pub behaviour: u8,
}

/// 设置自动药水阈值请求（从 GateActor 转发）
pub struct SetAutoPotValueRequest {
    pub session_id: u64,
    pub stat: u8,
    pub value: u32,
}

/// 设置自动药水物品请求（从 GateActor 转发）
pub struct SetAutoPotItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub item_index: i32,
}

/// 从装备插槽移除物品请求（从 GateActor 转发）
pub struct RemoveSlotItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub grid_to: u8,
    pub unique_id: u64,
    pub to: i32,
    pub from_unique_id: u64,
}

/// NPC 对话请求（从 GateActor 转发）
pub struct NPCCallRequest {
    pub session_id: u64,
    pub npc_object_id: u32,
    pub key: String,
}

// ============================================================
// 物品系统消息
// ============================================================

/// 拾取地面物品
pub struct PickUpRequest {
    pub session_id: u64,
}

/// 背包内移动物品
pub struct MoveItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub from: i32,
    pub to: i32,
}

/// 使用物品
pub struct UseItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

/// 装备物品
pub struct EquipItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub slot: i32,
}

/// 卸下装备
pub struct RemoveItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
}

/// 丢弃物品
pub struct DropItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub count: u16,
}

/// 合并物品
pub struct MergeItemRequest {
    pub session_id: u64,
    pub grid_from: u8,
    pub grid_to: u8,
    pub from_uid: u64,
    pub to_uid: u64,
}

/// 拆分物品
pub struct SplitItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub count: u32,
}

/// 丢弃金币
pub struct DropGoldRequest {
    pub session_id: u64,
    pub amount: u32,
}

/// 购买物品（NPC 商店）
pub struct BuyItemRequest {
    pub session_id: u64,
    pub npc_id: u32,
    pub item_index: u32,
    pub count: u32,
}

/// 出售物品（NPC 商店）
pub struct SellItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub count: u32,
}

// ============================================================
// 邮件系统消息
// ============================================================

/// 发送邮件
pub struct SendMailRequest {
    pub session_id: u64,
    pub receiver_name: String,
    pub subject: String,
    pub body: String,
    pub gold: u32,
    pub item_uids: Vec<u64>,
}

/// 读取邮件
pub struct ReadMailRequest {
    pub session_id: u64,
    pub mail_id: u64,
}

/// 收取邮件附件
pub struct CollectParcelRequest {
    pub session_id: u64,
    pub mail_id: u64,
}

/// 删除邮件
pub struct DeleteMailRequest {
    pub session_id: u64,
    pub mail_id: u64,
}

// ============================================================
// 任务系统消息
// ============================================================

/// 接受任务
pub struct AcceptQuestRequest {
    pub session_id: u64,
    pub npc_index: i32,
    pub quest_index: i32,
}

/// 完成任务
pub struct FinishQuestRequest {
    pub session_id: u64,
    pub quest_index: i32,
    pub selected_item_index: i32,
}

/// 放弃任务
pub struct AbandonQuestRequest {
    pub session_id: u64,
    pub quest_index: i32,
}

// ============================================================
// Tick 子系统方法
// ============================================================

impl WorldActor {
    /// 玩家 Buff tick + 死亡复活（每 5 ticks）
    async fn tick_buffs_and_revive(&mut self) {
        if self.tick_count % 5 == 0 {
            let mut to_revive = Vec::new();
            let mut to_remove = Vec::new();
            for (session_id, record) in &self.players {
                let _ = record.actor_ref.ask(crate::actors::player::TickBuff).await;
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_dead {
                        match self.player_death_queue.get(session_id) {
                            None => {
                                self.player_death_queue.insert(*session_id, self.tick_count);
                            }
                            Some(death_tick) => {
                                if self.tick_count >= death_tick + 60 {
                                    to_revive.push(*session_id);
                                }
                            }
                        }
                    } else if self.player_death_queue.contains_key(session_id) {
                        to_remove.push(*session_id);
                    }
                }
            }
            for session_id in to_remove {
                self.player_death_queue.remove(&session_id);
            }
            for session_id in to_revive {
                self.player_death_queue.remove(&session_id);
                if let Some(record) = self.players.get(&session_id) {
                    let _ = record.actor_ref.ask(crate::actors::player::Revive).await;
                }
            }
        }
    }

    /// 地图环境伤害 + 禁止坐骑地图自动下坐骑（每 20 ticks）
    async fn tick_environment_damage(&mut self) {
        if self.tick_count % 20 == 0 {
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_dead { continue; }
                    if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                        if mi.fire || mi.lightning {
                            let in_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            if in_safe { continue; }
                            let damage = if mi.fire { mi.fire_damage } else { mi.lightning_damage };
                            if damage > 0 {
                                let died = record.actor_ref.ask(TakeDamage {
                                    attacker_id: 0, // environment
                                    attacker_session: 0,
                                    damage,
                                }).await.unwrap_or(false);
                                if died {
                                    self.player_death_queue.insert(*session_id, self.tick_count);
                                    broadcast_system_message(&self.gate_ref, &self.players,
                                        &format!("{} 在{}中倒下了", state.name,
                                            if mi.fire { "火海" } else { "雷暴" }));
                                } else {
                                    let msg = if mi.fire { "你受到了火焰伤害！" } else { "你受到了闪电伤害！" };
                                    send_system_message(&self.gate_ref, *session_id, msg);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 自动下坐骑：进入禁止坐骑地图时
        if self.tick_count % 20 == 0 {
            let mut to_dismount: Vec<u64> = Vec::new();
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.is_mounted {
                        if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                            if mi.no_mount {
                                to_dismount.push(*session_id);
                            }
                        }
                    }
                }
            }
            for session_id in to_dismount {
                self.dismount_player(session_id).await;
                send_system_message(&self.gate_ref, session_id, "该地图禁止骑乘坐骑，已自动下坐骑");
            }
        }
    }

    /// 经验倍率过期、全局事件过期、随机世界事件、隐身过期（每 100 ticks）
    async fn tick_exp_events_and_invisibility(&mut self) {
        if self.tick_count % 100 == 0 {
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.exp_multiplier > 1.0 && self.tick_count >= state.exp_multiplier_end_tick {
                        let _ = record.actor_ref.ask(SetExpMultiplier {
                            multiplier: 1.0,
                            end_tick: 0,
                        }).await;
                        send_system_message(&self.gate_ref, *session_id, "双倍经验效果已结束");
                        debug!("Exp multiplier expired for session {}", session_id);
                    }
                }
            }
            // 全局事件过期广播
            if self.tick_count >= self.global_exp_event_end_tick && self.global_exp_event_end_tick > 0 {
                let event_name = self.global_event_name.take().unwrap_or_else(|| "活动".to_string());
                self.global_exp_multiplier = 1.0;
                self.global_drop_multiplier = 1.0;
                self.global_gold_multiplier = 1.0;
                self.global_exp_event_end_tick = 0;
                for (session_id, _) in &self.players {
                    send_system_message(&self.gate_ref, *session_id, &format!("全服{}已结束", event_name));
                }
                info!("Global event ended: {}", event_name);
            }
            // 随机世界事件触发（每 36000 ticks = 1 小时，20% 概率）
            if self.tick_count > 0 && self.tick_count % 36000 == 0 && self.global_exp_event_end_tick == 0 {
                let roll = fastrand::u32(1..=100);
                if roll <= 20 {
                    let event_roll = fastrand::u32(1..=100);
                    let (name, exp_mul, drop_mul, gold_mul, duration_min) = match event_roll {
                        1..=40 => ("双倍经验", 2.0, 1.0, 1.0, 10),
                        41..=70 => ("掉落狂欢", 1.0, 2.0, 1.0, 10),
                        71..=90 => ("金币雨", 1.0, 1.0, 2.0, 10),
                        _ => ("三重盛宴", 2.0, 2.0, 2.0, 5),
                    };
                    let duration_ticks = duration_min * 600;
                    self.global_exp_multiplier = exp_mul;
                    self.global_drop_multiplier = drop_mul;
                    self.global_gold_multiplier = gold_mul;
                    self.global_exp_event_end_tick = self.tick_count + duration_ticks;
                    self.global_event_name = Some(name.to_string());
                    broadcast_system_message(&self.gate_ref, &self.players,
                        &format!("【世界事件】{} 活动已启动！经验 x{} 掉落 x{} 金币 x{}，持续 {} 分钟！",
                            name, exp_mul, drop_mul, gold_mul, duration_min));
                    info!("Random world event started: {} (exp={} drop={} gold={} for {} min)",
                        name, exp_mul, drop_mul, gold_mul, duration_min);
                }
            }
            // 隐身过期检查：从 invisible_sessions 中移除已过期玩家并广播现身
            let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
            let mut to_reveal: Vec<(u64, crate::actors::player::PlayerState)> = Vec::new();
            for session_id in &self.invisible_sessions {
                if let Some(record) = self.players.get(session_id) {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        let still_invisible = state.buffs.iter()
                            .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                        if !still_invisible {
                            to_reveal.push((*session_id, state));
                        }
                    }
                }
            }
            for (session_id, state) in to_reveal {
                self.invisible_sessions.remove(&session_id);
                self.reveal_player_to_others(session_id, &state).await;
                send_system_message(&self.gate_ref, session_id, "隐身效果已结束");
            }
        }
    }

    /// PK 值衰减 + 名字颜色广播（每 10 ticks）
    async fn tick_pk_decay(&mut self) {
        if self.tick_count % 10 == 0 {
            let mut colour_changes = Vec::new();
            for (session_id, record) in &self.players {
                let _ = record.actor_ref.ask(crate::actors::player::DecayPkPoints).await;
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    let new_colour = name_colour_for_pk(state.pk_points);
                    let old_colour = name_colour_for_pk(record.last_pk_points);
                    if new_colour != old_colour {
                        colour_changes.push((*session_id, state.object_id, new_colour, state.pk_points));
                    }
                }
            }
            for (session_id, object_id, new_colour, pk_points) in colour_changes {
                if let Some(record) = self.players.get_mut(&session_id) {
                    record.last_pk_points = pk_points;
                }
                let packet = build_object_colour_changed_packet(object_id, new_colour);
                for (sid, _) in &self.players {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *sid,
                        data: packet.clone(),
                    });
                }
            }
        }
    }

    /// 钓鱼收获判定（每 tick）
    async fn tick_fishing(&mut self) {
        let mut caught = Vec::new(); // session_id
        let mut stopped = Vec::new(); // session_id
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                if !state.is_fishing { continue; }

                let counter = self.fishing_tick_counters.entry(*session_id).or_insert(0);
                *counter += 1;

                // 钓鱼需要 30~70 ticks（约 3~7 秒）才有收获
                let required = 30 + ((*session_id as u32 + *counter).wrapping_mul(1103515245).wrapping_add(12345) % 41);
                if *counter >= required {
                    // 收获判定
                    let roll = ((*session_id + self.tick_count) % 100) as u8;
                    if roll < 60 {
                        // 金币 10~50
                        let gold = 10 + ((*session_id + self.tick_count) % 41) as u64;
                        let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: gold }).await;
                        send_system_message(&self.gate_ref, *session_id, &format!("钓到了宝箱！获得 {} 金币", gold));
                    } else if roll < 80 {
                        // 随机物品：从已加载的物品中挑一个低阶物品
                        let item_index = Self::random_fishing_item_index(&self.item_infos, *session_id, self.tick_count);
                        let item = crate::actors::inventory::make_item(item_index, 1);
                        let added = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await.unwrap_or(false);
                        if added {
                            send_system_message(&self.gate_ref, *session_id, "钓到了一件物品！");
                        } else {
                            send_system_message(&self.gate_ref, *session_id, "钓到了物品，但背包已满！");
                        }
                    } else if roll < 95 {
                        // 经验 10~30
                        let xp = 10 + ((*session_id + self.tick_count) % 21) as i32;
                        let _ = record.actor_ref.ask(crate::actors::player::AddExperience { amount: self.apply_global_exp_multiplier(xp) }).await;
                        send_system_message(&self.gate_ref, *session_id, &format!("钓到了经验珠！获得 {} 经验", xp));
                    } else {
                        send_system_message(&self.gate_ref, *session_id, "鱼跑了...");
                    }

                    if state.fishing_autocast {
                        caught.push(*session_id);
                    } else {
                        stopped.push(*session_id);
                    }
                }
            }
        }
        for session_id in caught {
            self.fishing_tick_counters.insert(session_id, 0);
            // Send bite state then auto-recast waiting state
            let bite_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 2, fishing_success: true };
            let mut body = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&bite_packet, &mut body) {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
                });
            }
            // Then immediately send waiting state for autocast
            let wait_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 1, fishing_success: false };
            let mut body2 = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&wait_packet, &mut body2) {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body2),
                });
            }
        }
        for session_id in stopped {
            self.fishing_tick_counters.remove(&session_id);
            if let Some(record) = self.players.get(&session_id) {
                let _ = record.actor_ref.ask(crate::actors::player::SetFishing { is_fishing: false, autocast: false }).await;
            }
            // Send idle state
            let idle_packet = mir2_shared::packets::server::miscellaneous::FishingUpdate { fishing_progress: 0, fishing_success: false };
            let mut body = Vec::new();
            if let Ok(()) = mir2_shared::packets::Packet::write_body(&idle_packet, &mut body) {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
                });
            }
        }
    }

    /// 地面物品过期清理（每 50 ticks）
    async fn tick_ground_cleanup(&mut self) {
        if self.tick_count % 50 == 0 {
            const GROUND_ITEM_LIFETIME_TICKS: u64 = 600; // ~60 秒
            let expired: Vec<_> = self.ground_items.iter()
                .filter(|gi| self.tick_count >= gi.drop_tick + GROUND_ITEM_LIFETIME_TICKS)
                .map(|gi| (gi.object_id, gi.map_index))
                .collect();
            if !expired.is_empty() {
                self.ground_items.retain(|gi| self.tick_count < gi.drop_tick + GROUND_ITEM_LIFETIME_TICKS);
                for (oid, map_idx) in &expired {
                    let mut remove_body = Vec::new();
                    remove_body.extend_from_slice(&oid.to_le_bytes());
                    let remove_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &remove_body);
                    for (sid, rec) in &self.players {
                        if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                            if s.map_index == *map_idx {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: remove_packet.clone(),
                                });
                            }
                        }
                    }
                }
                debug!("Cleaned up {} expired ground items", expired.len());
            }
        }
    }

    /// 怪物重生处理（每 tick）
    async fn tick_respawn(&mut self) {
        let mut to_respawn = Vec::new();
        for (oid, (spawn, tick)) in &self.respawn_queue {
            if self.tick_count >= *tick {
                to_respawn.push((*oid, spawn.clone()));
            }
        }
        for (oid, spawn) in to_respawn {
            self.respawn_queue.remove(&oid);
            let new_oid = self.alloc_object_id();
            let packet = build_object_monster_packet(&spawn, new_oid, &spawn.name);
            for session_id in self.players.keys() {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: *session_id,
                    data: packet.clone(),
                });
            }
            let ai_profile = self.monster_infos
                .get(&spawn.monster_index)
                .map(MonsterAiProfile::from_info)
                .unwrap_or_else(|| MonsterAiProfile {
                    ai_type: MonsterAiType::Aggressive,
                    aggro_range: 10,
                    attack_range: 1,
                    attack_cooldown: 5,
                    move_interval: 2,
                    flee_threshold: 0.0,
                });
            // 精英判定：3% 概率
            let is_elite = fastrand::u8(1..=100) <= 3;
            let (name, hp, max_hp, min_dmg, max_dmg, xp) = if is_elite {
                (
                    format!("[精英] {}", spawn.name),
                    spawn.hp.saturating_mul(2),
                    spawn.hp.saturating_mul(2),
                    (spawn.min_dmg as f32 * 1.5) as i32,
                    (spawn.max_dmg as f32 * 1.5) as i32,
                    spawn.xp.saturating_mul(2),
                )
            } else {
                (spawn.name.clone(), spawn.hp, spawn.hp, spawn.min_dmg, spawn.max_dmg, spawn.xp)
            };
            self.monsters.insert(new_oid, MonsterState {
                object_id: new_oid,
                name: name.clone(),
                image: spawn.image,
                monster_index: spawn.monster_index,
                x: spawn.x,
                y: spawn.y,
                direction: spawn.direction,
                hp,
                max_hp,
                min_dmg,
                max_dmg,
                xp,
                spawn_x: spawn.x,
                spawn_y: spawn.y,
                map_index: spawn.map_index,
                next_attack_tick: 0,
                next_move_tick: 0,
                next_summon_tick: 0,
                ai_profile,
                ai_state: MonsterAiState::Idle,
                target_session: None,
                provoked: false,
                is_elite,
                is_boss: false,
            });
            if is_elite {
                let map_name = self.map_infos.get(&(spawn.map_index as i32)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
                broadcast_system_message(&self.gate_ref, &self.players,
                    &format!("一只 [精英]{} 出现在 {}！勇士们，前往讨伐！", spawn.name, map_name));
                debug!("Elite monster '{}' spawned as #{} at ({},{})", name, new_oid, spawn.x, spawn.y);
            } else {
                debug!("Monster '{}' respawned as #{}", spawn.name, new_oid);
            }
        }
    }

    /// 世界Boss超时消失（每 tick）
    async fn tick_boss_timeout(&mut self) {
        let mut boss_despawns = Vec::new();
        for (oid, despawn_tick) in &self.world_boss_queue {
            if should_despawn_boss(self.tick_count, *despawn_tick) {
                boss_despawns.push(*oid);
            }
        }
        for oid in boss_despawns {
            self.world_boss_queue.remove(&oid);
            if let Some(monster) = self.monsters.remove(&oid) {
                let body = oid.to_le_bytes().to_vec();
                let packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &body);
                for session_id in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *session_id,
                        data: packet.clone(),
                    });
                }
                broadcast_system_message(&self.gate_ref, &self.players,
                    &format!("世界Boss {} 因无人挑战而消失了", monster.name));
                debug!("World boss '{}' (#{}) despawned (timeout)", monster.name, oid);
            }
        }
    }

    /// 任务超时检查（每 100 ticks）
    async fn tick_quest_timeout(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    for quest in &state.quest_log.quests {
                        if quest.time_limit_seconds > 0
                            && matches!(quest.status, QuestStatus::InProgress | QuestStatus::Accepted)
                            && now.saturating_sub(quest.start_time) >= quest.time_limit_seconds as u64
                        {
                            let failed = record.actor_ref.ask(crate::actors::player::FailQuest {
                                quest_index: quest.quest_index,
                            }).await.unwrap_or(false);
                            if failed {
                                send_system_message(
                                    &self.gate_ref, *session_id,
                                    &format!("任务 '{}' 已超时失败", quest.title)
                                );
                                debug!("Quest expired: {} for session {}", quest.title, session_id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 宠物自动拾取（每 tick）
    async fn tick_pet_pickup(&mut self) {
        let mut pet_pickups: Vec<(usize, u64)> = Vec::new(); // (ground_item_index, session_id)
        for (session_id, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                let creature = match state.creature_log.active_creature {
                    Some(ref c) if c.enabled && !c.is_starving() => c,
                    _ => continue,
                };
                let pickup_mode = creature.pickup_mode;
                if pickup_mode == crate::actors::creature::PickupMode::None {
                    continue;
                }
                // 找附近匹配的物品（最多拾取 1 个/ tick）
                for (gi_idx, gi) in self.ground_items.iter().enumerate() {
                    let dist = (state.x - gi.x).abs() + (state.y - gi.y).abs();
                    if dist > 3 { continue; }
                    if gi.map_index != state.map_index { continue; }
                    // 绑定物品（dropper_session 不为空）不能拾取
                    if gi.dropper_session.is_some() && gi.dropper_session != Some(*session_id) { continue; }

                    let is_gold = gi.item.item_index == 0;
                    let should_pickup = match pickup_mode {
                        crate::actors::creature::PickupMode::GoldOnly => is_gold,
                        crate::actors::creature::PickupMode::GoldAndItem => true,
                        crate::actors::creature::PickupMode::All => true,
                        _ => false,
                    };
                    if should_pickup {
                        pet_pickups.push((gi_idx, *session_id));
                        break; // 每个玩家每 tick 最多拾取 1 个
                    }
                }
            }
        }

        // 应用拾取（从后往前删除，避免索引偏移）
        pet_pickups.sort_by(|a, b| b.0.cmp(&a.0));
        pet_pickups.dedup_by(|a, b| a.0 == b.0); // 同一物品只拾取一次

        for (gi_idx, session_id) in pet_pickups {
            if gi_idx >= self.ground_items.len() { continue; }
            let gi = self.ground_items.remove(gi_idx);

            // 广播移除
            let remove_body = gi.object_id.to_le_bytes().to_vec();
            let remove_packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &remove_body);
            for sid in self.players.keys() {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: *sid,
                    data: remove_packet.clone(),
                });
            }

            if let Some(record) = self.players.get(&session_id) {
                if gi.item.item_index == 0 {
                    // 金币
                    let gold = gi.item.count as u64;
                    let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: gold }).await;
                    send_system_message(&self.gate_ref, session_id,
                        &format!("宠物帮你拾取了 {} 金币", gold));
                } else {
                    // 检查背包空间
                    let has_space = record.actor_ref.ask(crate::actors::player::HasItemSpace).await.unwrap_or(false);
                    if has_space {
                        let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory {
                            item: gi.item.clone(),
                        }).await;
                        send_system_message(
                            &self.gate_ref, session_id,
                            &format!("宠物帮你拾取了物品"));
                    } else {
                        // 背包已满，把物品掉回去
                        self.ground_items.push(gi);
                        send_system_message(&self.gate_ref, session_id,
                            "宠物发现物品但你的背包已满");
                    }
                }
            }
        }
    }

    /// NPC 商店自动补货（每小时）
    async fn tick_shop_restock(&mut self) {
        if self.tick_count.is_multiple_of(36000) {
            let mut restocked = 0usize;
            for goods_list in self.npc_goods.values_mut() {
                for good in goods_list.iter_mut() {
                    if !good.infinite_stock && good.stock < good.max_stock {
                        good.stock = good.max_stock;
                        restocked += 1;
                    }
                }
            }
            if restocked > 0 {
                info!("NPC shop restock: {} items restocked", restocked);
            }
        }
    }

    /// 精炼自动完成（每 100 ticks）
    async fn tick_refine_complete(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            for (session_id, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if let Some(ref item) = state.refine_log.active_refine {
                        if item.status == RefineStatus::Pending && current_time >= item.finish_time {
                            let mut log = state.refine_log.clone();
                            let success = log.finish();
                            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                            if success {
                                send_system_message(&self.gate_ref, *session_id, "精炼完成！物品已提升");
                            } else {
                                send_system_message(&self.gate_ref, *session_id, "精炼失败，物品已损毁");
                            }
                            debug!("AutoRefine: {} result={}", state.name, success);
                        }
                    }
                }
            }
        }
    }

    /// HP/MP 回复 + 宠物饥饿 tick（每 100 ticks）
    async fn tick_regen_and_hunger(&mut self) {
        if self.tick_count.is_multiple_of(100) {
            debug!(
                "World tick #{} (online: {}, monsters: {})",
                self.tick_count, self.players.len(), self.monsters.len()
            );

            // 每 10 秒（100 ticks @ 100ms）回复 HP/MP
            for record in self.players.values() {
                // 宠物饥饿值
                let _ = record.actor_ref.ask(TickCreatureHunger { dt_seconds: 10 });

                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    let hp_regen = 5;
                    let mp_regen = 3;
                    let new_hp = (state.hp + hp_regen).min(state.max_hp);
                    let new_mp = (state.mp + mp_regen).min(state.max_mp);

                    if new_hp != state.hp || new_mp != state.mp {
                        // 发送 HealthChanged
                        let mut health_body = Vec::new();
                        health_body.extend_from_slice(&(new_hp as u32).to_le_bytes());
                        health_body.extend_from_slice(&(new_mp as u32).to_le_bytes());
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: state.session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::HealthChanged as i16,
                                &health_body,
                            ),
                        });
                    }
                }
            }
        }
    }

    /// 昼夜循环（每 600 ticks）
    async fn tick_day_night(&mut self) {
        if self.tick_count.is_multiple_of(600) {
            let hour = chrono::Local::now().hour();
            let new_light = Self::light_for_hour(hour);
            if new_light != self.current_light {
                self.current_light = new_light;
                for session_id in self.players.keys() {
                    self.send_time_of_day(*session_id, new_light);
                }
                let light_name = match new_light {
                    mir2_shared::enums::LightSetting::Dawn => "黎明",
                    mir2_shared::enums::LightSetting::Day => "白天",
                    mir2_shared::enums::LightSetting::Evening => "黄昏",
                    mir2_shared::enums::LightSetting::Night => "夜晚",
                    _ => "正常",
                };
                info!("Time of day changed to {} (hour={})", light_name, hour);
            }
        }
    }

    /// 定期自动保存（每 300 ticks）
    async fn tick_auto_save(&mut self) {
        if self.tick_count % 300 == 0 && !self.players.is_empty() {
            let player_count = self.players.len();
            let mut saved = 0;
            for record in self.players.values() {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if let Err(e) = db::save_character(&self.db_pool, &state, &record.name).await {
                        warn!("Auto-save failed for player {}: {}", record.name, e);
                    } else {
                        saved += 1;
                    }
                }
            }
            info!("Auto-saved {} players to database ({} online)", saved, player_count);
        }
    }

    /// 拍卖过期清理（每 36000 ticks = 1小时）
    async fn tick_auction_expiry(&mut self) {
        if self.tick_count % 36000 == 0 {
            let now = chrono::Local::now().timestamp();
            let seven_days = 7 * 24 * 60 * 60;
            let mut expired = Vec::new();
            for (idx, auction) in self.auctions.iter().enumerate() {
                if !auction.sold && (now - auction.consignment_date) > seven_days {
                    expired.push(idx);
                }
            }
            for idx in expired.into_iter().rev() {
                let auction = self.auctions.remove(idx);
                let _ = db::delete_auction(&self.db_pool, auction.auction_id as i64).await;

                // Return item to seller
                let mut seller_online = false;
                for (_, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if state.name == auction.seller_name {
                            let added = record.actor_ref.ask(AddItemToInventory { item: auction.item.clone() }).await.unwrap_or(false);
                            if added {
                                send_system_message(&self.gate_ref, record.session_id, "寄售物品已过期，已退回背包");
                            } else {
                                send_system_message(&self.gate_ref, record.session_id, "寄售物品已过期，背包已满，物品已通过邮件退回");
                                send_item_via_mail(&self.db_pool, &auction.seller_name, auction.item.clone(), "寄售物品退回", "寄售物品已过期，背包已满");
                            }
                            seller_online = true;
                            break;
                        }
                    }
                }
                if !seller_online {
                    // Seller offline — send item via mail
                    send_item_via_mail(&self.db_pool, &auction.seller_name, auction.item.clone(), "寄售物品退回", "寄售物品已过期");
                }
                debug!("Auction {} expired and removed", auction.auction_id);
            }
        }
    }

    /// 租赁过期处理（每 3600 ticks = 6分钟检查一次）
    async fn tick_rental_expiry(&mut self) {
        if self.tick_count % 3600 == 0 {
            let now = chrono::Local::now().timestamp();
            let mut expired_renters: Vec<String> = Vec::new();

            for (renter_name, rentals) in &mut self.player_rentals {
                let mut still_valid: Vec<RentedItem> = Vec::new();
                for rental in rentals.drain(..) {
                    if rental.expiry_timestamp > now {
                        still_valid.push(rental);
                        continue;
                    }
                    // Rental expired - try to remove from renter and return to owner
                    let mut returned = false;
                    for (_, record) in &self.players {
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            if state.name == *renter_name {
                                // Try to remove item from renter
                                let removed = record.actor_ref.ask(RemoveItemFromInventory {
                                    unique_id: rental.item.unique_id,
                                }).await.ok().flatten();
                                if removed.is_some() {
                                    // Return to owner if online
                                    for (_, owner_record) in &self.players {
                                        if let Ok(Some(owner_state)) = owner_record.actor_ref.ask(GetPlayerState).await {
                                            if owner_state.name == rental.owner_name {
                                                let added = owner_record.actor_ref.ask(AddItemToInventory {
                                                    item: rental.item.clone(),
                                                }).await.unwrap_or(false);
                                                if added {
                                                    send_system_message(&self.gate_ref, owner_record.session_id,
                                                        &format!("租赁物品 {} 已到期收回", rental.item.item_index));
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    send_system_message(&self.gate_ref, record.session_id,
                                        &format!("租赁物品 {} 已到期，已归还给 {}", rental.item.item_index, rental.owner_name));
                                    returned = true;
                                } else {
                                    send_system_message(&self.gate_ref, record.session_id,
                                        &format!("租赁物品 {} 已到期，但物品不在背包中", rental.item.item_index));
                                }
                                break;
                            }
                        }
                    }
                    if !returned {
                        // Renter offline or item not in inventory — return to owner via online or mail
                        let mut owner_online = false;
                        for (_, owner_record) in &self.players {
                            if let Ok(Some(owner_state)) = owner_record.actor_ref.ask(GetPlayerState).await {
                                if owner_state.name == rental.owner_name {
                                    let added = owner_record.actor_ref.ask(AddItemToInventory {
                                        item: rental.item.clone(),
                                    }).await.unwrap_or(false);
                                    if added {
                                        send_system_message(&self.gate_ref, owner_record.session_id,
                                            &format!("租赁物品 {} 已到期收回", rental.item.item_index));
                                    } else {
                                        send_system_message(&self.gate_ref, owner_record.session_id,
                                            &format!("租赁物品 {} 已到期，背包已满，已通过邮件退回", rental.item.item_index));
                                        send_item_via_mail(&self.db_pool, &rental.owner_name, rental.item.clone(),
                                            "租赁物品退回", &format!("租赁物品 {} 已到期", rental.item.item_index));
                                    }
                                    owner_online = true;
                                    break;
                                }
                            }
                        }
                        if !owner_online {
                            send_item_via_mail(&self.db_pool, &rental.owner_name, rental.item.clone(),
                                "租赁物品退回", &format!("租赁物品 {} 已到期", rental.item.item_index));
                        }
                    }
                    debug!("Rental expired: {} -> {} item={}", rental.owner_name, renter_name, rental.item.item_index);
                }
                if still_valid.is_empty() {
                    expired_renters.push(renter_name.clone());
                } else {
                    *rentals = still_valid;
                }
            }
            for name in expired_renters {
                self.player_rentals.remove(&name);
            }
        }
    }
}

// ============================================================
// Handler 实现
// ============================================================

impl Message<Tick> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Tick,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tick_count += 1;

        // --- 怪物 AI ---
        if !self.monsters.is_empty() && !self.players.is_empty() {
            // 收集所有玩家位置（避免在循环中借用 self）
            // 预收集玩家位置 + PK 值（用于 Guard AI 红名优先）
            let player_positions: Vec<(u64, i32, i32, u32, i32)> = {
                let mut results = Vec::new();
                let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
                for (session_id, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if !state.is_dead {
                            // 隐身玩家不会被怪物检测到
                            let is_invisible = state.buffs.iter()
                                .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                            if is_invisible { continue; }
                            let in_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            if !in_safe {
                                results.push((*session_id, state.x, state.y, state.object_id, state.pk_points));
                            }
                        }
                    }
                }
                results
            };

            // 对每个怪物执行 AI
            let mut dead_monsters = Vec::new();
            let mut moved_monsters = Vec::new();
            let mut moved_targets: HashSet<(i32, i32)> = HashSet::new();
            let mut death_drops: Vec<(u64, i32, i32, u16)> = Vec::new();
            let mut broken_armor: Vec<(u64, EquipmentSlot)> = Vec::new();
            let mut dismount_sessions: Vec<u64> = Vec::new();
            // 预收集怪物当前位置（用于碰撞检测）
            let monster_positions: HashSet<(i32, i32)> = self.monsters.values().map(|m| (m.x, m.y)).collect();
            // 预收集怪物快照（用于 Healer AI 寻找受伤盟友）
            let monster_snapshot: Vec<(u32, i32, i32, i32, i32, u16, i32, String, u16, u8)> = self.monsters.values()
                .map(|m| (m.object_id, m.x, m.y, m.hp, m.max_hp, m.map_index, m.monster_index, m.name.clone(), m.image, m.direction))
                .collect();
            // Healer 治疗动作和 Summoner 召唤动作（在循环后应用）
            let mut heal_actions: Vec<(u32, i32)> = Vec::new();
            let mut summon_spawns: Vec<MonsterSpawn> = Vec::new();

            for (oid, monster) in &mut self.monsters {
                let profile = &monster.ai_profile;

                // 找最近玩家（在视野范围内）
                // Guard AI：优先攻击红名玩家（PK 值 > 0）
                let mut nearest: Option<(u64, i32, i32, i32)> = None;
                if profile.ai_type == MonsterAiType::Guard {
                    // 先找范围内的红名玩家
                    let mut red_nearest: Option<(u64, i32, i32, i32)> = None;
                    for (session, px, py, _, pk) in &player_positions {
                        let dist = (monster.x - px).abs() + (monster.y - py).abs();
                        if dist <= profile.aggro_range && *pk > 0 {
                            if red_nearest.is_none_or(|n| dist < n.3) {
                                red_nearest = Some((*session, *px, *py, dist));
                            }
                        }
                    }
                    if red_nearest.is_some() {
                        nearest = red_nearest;
                    } else {
                        for (session, px, py, _, _) in &player_positions {
                            let dist = (monster.x - px).abs() + (monster.y - py).abs();
                            if dist <= profile.aggro_range {
                                if nearest.is_none_or(|n| dist < n.3) {
                                    nearest = Some((*session, *px, *py, dist));
                                }
                            }
                        }
                    }
                } else {
                    for (session, px, py, _, _) in &player_positions {
                        let dist = (monster.x - px).abs() + (monster.y - py).abs();
                        if dist <= profile.aggro_range {
                            if nearest.is_none_or(|n| dist < n.3) {
                                nearest = Some((*session, *px, *py, dist));
                            }
                        }
                    }
                }

                // 更新目标
                if let Some((sess, _, _, _)) = nearest {
                    monster.target_session = Some(sess);
                } else {
                    monster.target_session = None;
                }

                // 低血量逃跑判定（Coward）
                let hp_pct = monster.hp as f32 / monster.max_hp as f32;
                let is_fleeing = profile.ai_type == MonsterAiType::Coward && hp_pct < profile.flee_threshold;

                // 是否在攻击冷却中
                let can_attack = self.tick_count >= monster.next_attack_tick;
                // 是否可以移动（移动间隔）
                let can_move = self.tick_count >= monster.next_move_tick;

                // Passive 怪物：未激怒时不主动攻击
                let should_chase = match profile.ai_type {
                    MonsterAiType::Passive => monster.provoked,
                    MonsterAiType::Guard => nearest.is_some_and(|(_, _, _, d)| d <= profile.aggro_range) && dist_to_spawn(monster) <= profile.aggro_range * 2,
                    _ => nearest.is_some(),
                };

                if let Some((target_session, px, py, dist)) = nearest {
                    if is_fleeing && can_move {
                        // 逃跑：远离目标
                        let (nx, ny, dir) = monster.step_away(px, py);
                        if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(true)
                            && !monster_positions.contains(&(nx, ny))
                            && moved_targets.insert((nx, ny))
                        {
                            moved_monsters.push((*oid, nx, ny, dir));
                        }
                        monster.next_move_tick = self.tick_count + profile.move_interval;
                        monster.ai_state = MonsterAiState::Flee;
                    } else if dist <= profile.attack_range && can_attack {
                        // Healer AI：优先治疗附近受伤的怪物
                        let mut did_heal = false;
                        if profile.ai_type == MonsterAiType::Healer {
                            let mut best_target: Option<(u32, i32)> = None; // (oid, deficit)
                            for (snap_oid, sx, sy, shp, smax, smap, _, _, _, _) in &monster_snapshot {
                                if *snap_oid == *oid { continue; }
                                if *smap != monster.map_index { continue; }
                                let dist_ally = (monster.x - sx).abs() + (monster.y - sy).abs();
                                if dist_ally <= profile.aggro_range && *shp < *smax {
                                    let deficit = *smax - *shp;
                                    if best_target.is_none_or(|(_, d)| deficit > d) {
                                        best_target = Some((*snap_oid, deficit));
                                    }
                                }
                            }
                            if let Some((target_oid, _)) = best_target {
                                let heal_amount = (monster.max_hp / 4).max(10);
                                heal_actions.push((target_oid, heal_amount));
                                monster.next_attack_tick = self.tick_count + profile.attack_cooldown;
                                monster.ai_state = MonsterAiState::Attack;
                                did_heal = true;
                                debug!("Monster '{}' (#{}) heals ally #{} for {} HP", monster.name, *oid, target_oid, heal_amount);
                                // 广播治疗法术效果
                                let mut heal_body = Vec::new();
                                heal_body.extend_from_slice(&monster.object_id.to_le_bytes());
                                heal_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                                heal_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                                heal_body.push(monster.direction);
                                heal_body.push(SPELL_HEALING);
                                heal_body.extend_from_slice(&0u16.to_le_bytes());
                                heal_body.push(0u8);
                                let heal_packet = build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &heal_body);
                                for sid in self.players.keys() {
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id: *sid,
                                        data: heal_packet.clone(),
                                    });
                                }
                            }
                        }
                        // Summoner AI：低血量时召唤援军
                        let mut did_summon = false;
                        if profile.ai_type == MonsterAiType::Summoner && !did_heal {
                            let hp_pct = monster.hp as f32 / monster.max_hp as f32;
                            if hp_pct < 0.5 && self.tick_count >= monster.next_summon_tick {
                                // 找附近可行走的位置
                                let offsets: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                                let mut spawn_count = 0;
                                for (dx, dy) in offsets {
                                    if spawn_count >= 2 { break; }
                                    let sx = monster.x + dx;
                                    let sy = monster.y + dy;
                                    if self.maps.get(&monster.map_index).map(|m| m.is_walkable(sx, sy)).unwrap_or(false)
                                        && !monster_positions.contains(&(sx, sy))
                                    {
                                        summon_spawns.push(MonsterSpawn {
                                            name: format!("{}的召唤物", monster.name),
                                            image: monster.image,
                                            monster_index: monster.monster_index,
                                            x: sx,
                                            y: sy,
                                            direction: monster.direction,
                                            hp: (monster.max_hp / 2).max(1),
                                            min_dmg: (monster.min_dmg / 2).max(1),
                                            max_dmg: (monster.max_dmg / 2).max(1),
                                            xp: (monster.xp / 2).max(1),
                                            map_index: monster.map_index,
                                        });
                                        spawn_count += 1;
                                    }
                                }
                                if spawn_count > 0 {
                                    monster.next_summon_tick = self.tick_count + 100; // 10秒冷却
                                    monster.next_attack_tick = self.tick_count + profile.attack_cooldown;
                                    monster.ai_state = MonsterAiState::Attack;
                                    did_summon = true;
                                    debug!("Monster '{}' (#{}) summons {} adds", monster.name, *oid, spawn_count);
                                }
                            }
                        }
                        if did_heal || did_summon {
                            // 已执行特殊动作，跳过普通攻击
                        } else {
                            // 攻击
                            let dmg_range = (monster.max_dmg - monster.min_dmg).max(1);
                            let damage = ((self.tick_count.wrapping_add(*oid as u64).wrapping_mul(7)) as i32 % dmg_range)
                                + monster.min_dmg;
                            debug!("Monster '{}' (#{}) attacks Player {} for {} dmg [AI={:?}]", monster.name, *oid, target_session, damage, profile.ai_type);
                            monster.next_attack_tick = self.tick_count + profile.attack_cooldown;
                            monster.ai_state = MonsterAiState::Attack;

                        let is_ranged = matches!(profile.ai_type, MonsterAiType::Ranged | MonsterAiType::Mage);
                        let spell_id = match profile.ai_type {
                            MonsterAiType::Mage => SPELL_FIREBALL,
                            MonsterAiType::Ranged => 1u8,
                            _ => 0u8,
                        };

                        // ObjectAttack 广播
                        let mut attack_body = Vec::new();
                        attack_body.extend_from_slice(&monster.object_id.to_le_bytes());
                        attack_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                        attack_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                        attack_body.push(monster.direction);
                        attack_body.push(spell_id);
                        attack_body.extend_from_slice(&0u16.to_le_bytes());
                        attack_body.push(0u8);
                        let attack_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);
                        if is_ranged {
                            // 远程/法术攻击广播给所有玩家（弹道动画）
                            for sid in self.players.keys() {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: attack_packet.clone(),
                                });
                            }
                        } else {
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: target_session,
                                data: attack_packet,
                            });
                        }
                        // 安全区保护：目标在安全区内则不受怪物伤害
                        let target_in_safe = self.maps.get(&monster.map_index)
                            .map(|m| m.is_safe_zone(px, py))
                            .unwrap_or(false);

                        if !target_in_safe {
                            // 伤害
                            if let Some(record) = self.players.get(&target_session) {
                                let died = record.actor_ref.ask(TakeDamage {
                                    attacker_id: monster.object_id,
                                    attacker_session: target_session,
                                    damage,
                                }).await.unwrap_or(false);

                                // 被攻击时自动下坐骑
                                if !died {
                                    dismount_sessions.push(target_session);
                                }

                                // 装备耐久损耗（存活时）
                                if !died {
                                    let armor_slots = [
                                        EquipmentSlot::Armour,
                                        EquipmentSlot::Helmet,
                                        EquipmentSlot::BraceletL,
                                        EquipmentSlot::BraceletR,
                                        EquipmentSlot::RingL,
                                        EquipmentSlot::RingR,
                                        EquipmentSlot::Shoes,
                                        EquipmentSlot::Necklace,
                                    ];
                                    let slot = armor_slots[fastrand::usize(0..armor_slots.len())];
                                    let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                                        slot,
                                        amount: 1,
                                    }).await.unwrap_or(false);
                                    if broke {
                                        debug!("Player session={} {:?} broke from monster damage!", target_session, slot);
                                        // 延迟到怪物循环结束后广播（避免借用冲突）
                                        broken_armor.push((target_session, slot));
                                    }
                                }

                                if died {
                                    if let Ok(Some(victim)) = record.actor_ref.ask(GetPlayerState).await {
                                        let mut died_body = Vec::new();
                                        died_body.extend_from_slice(&victim.object_id.to_le_bytes());
                                        died_body.extend_from_slice(&(victim.x as u32).to_le_bytes());
                                        died_body.extend_from_slice(&(victim.y as u32).to_le_bytes());
                                        died_body.push(victim.direction);
                                        died_body.push(0u8);
                                        let died_packet = build_packet_bytes(
                                            mir2_shared::enums::ServerPacketIds::ObjectDied as i16, &died_body);
                                        for (sid, _) in &self.players {
                                            let _ = self.gate_ref.ask(SendToClient {
                                                session_id: *sid,
                                                data: died_packet.clone(),
                                            });
                                        }
                                        death_drops.push((target_session, victim.x, victim.y, victim.map_index));

                                        // 死亡经验惩罚：损失 2% 当前等级所需经验
                                        let penalty = (victim.max_experience / 50).max(1) as i32;
                                        let deducted = record.actor_ref.ask(crate::actors::player::DeductExperience {
                                            amount: penalty,
                                        }).await.unwrap_or(0);
                                        if deducted > 0 {
                                            send_system_message(
                                                &self.gate_ref, target_session,
                                                &format!("你损失了 {} 经验值", deducted)
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            debug!("Monster '{}' attack on {} blocked: target in safe zone", monster.name, target_session);
                        }
                        } // close else (normal attack)
                    } else if should_chase && dist > profile.attack_range && can_move {
                        // 追击
                        let (nx, ny, dir) = monster.step_toward(px, py);
                        if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(true)
                            && !monster_positions.contains(&(nx, ny))
                            && moved_targets.insert((nx, ny))
                        {
                            moved_monsters.push((*oid, nx, ny, dir));
                        }
                        monster.next_move_tick = self.tick_count + profile.move_interval;
                        monster.ai_state = MonsterAiState::Chase;
                    }
                } else if can_move && dist_to_spawn(monster) > 2 {
                    // 无目标 → 回出生点
                    let (nx, ny, dir) = monster.step_toward(monster.spawn_x, monster.spawn_y);
                    if self.maps.get(&monster.map_index).map(|m| m.is_walkable(nx, ny)).unwrap_or(true)
                        && !monster_positions.contains(&(nx, ny))
                        && moved_targets.insert((nx, ny))
                    {
                        moved_monsters.push((*oid, nx, ny, dir));
                    }
                    monster.next_move_tick = self.tick_count + profile.move_interval;
                    monster.ai_state = MonsterAiState::Return;
                } else {
                    monster.ai_state = MonsterAiState::Idle;
                }

                // 检查死亡
                if monster.hp <= 0 {
                    dead_monsters.push(*oid);
                }
            }

            // 应用 Healer 治疗（在循环外，避免借用冲突）
            for (target_oid, heal_amount) in &heal_actions {
                if let Some(target) = self.monsters.get_mut(target_oid) {
                    target.hp = (target.hp + *heal_amount).min(target.max_hp);
                }
            }

            // 应用 Summoner 召唤（在循环外创建新怪物）
            for spawn in &summon_spawns {
                let new_oid = self.alloc_object_id();
                let packet = build_object_monster_packet(spawn, new_oid, &spawn.name);
                for session_id in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *session_id,
                        data: packet.clone(),
                    });
                }
                let ai_profile = self.monster_infos
                    .get(&spawn.monster_index)
                    .map(MonsterAiProfile::from_info)
                    .unwrap_or_else(|| MonsterAiProfile {
                        ai_type: MonsterAiType::Aggressive,
                        aggro_range: 10,
                        attack_range: 1,
                        attack_cooldown: 5,
                        move_interval: 2,
                        flee_threshold: 0.0,
                    });
                self.monsters.insert(new_oid, MonsterState {
                    object_id: new_oid,
                    name: spawn.name.clone(),
                    image: spawn.image,
                    monster_index: spawn.monster_index,
                    x: spawn.x,
                    y: spawn.y,
                    direction: spawn.direction,
                    hp: spawn.hp,
                    max_hp: spawn.hp,
                    min_dmg: spawn.min_dmg,
                    max_dmg: spawn.max_dmg,
                    xp: spawn.xp,
                    spawn_x: spawn.x,
                    spawn_y: spawn.y,
                    map_index: spawn.map_index,
                    next_attack_tick: 0,
                    next_move_tick: 0,
                    next_summon_tick: 0,
                    ai_profile,
                    ai_state: MonsterAiState::Idle,
                    target_session: None,
                    provoked: false,
                    is_elite: false,
                    is_boss: false,
                });
                debug!("Summoned monster '{}' as #{} at ({},{})", spawn.name, new_oid, spawn.x, spawn.y);
            }

            // 应用移动并广播
            for (oid, nx, ny, dir) in &moved_monsters {
                if let Some(m) = self.monsters.get_mut(oid) {
                    m.x = *nx;
                    m.y = *ny;
                    m.direction = *dir;

                    // 广播 ObjectWalk（object_id + x + y + direction，~12字节 vs ObjectMonster ~40字节）
                    let mut walk_body = Vec::new();
                    walk_body.extend_from_slice(&oid.to_le_bytes());
                    walk_body.extend_from_slice(&m.x.to_le_bytes());
                    walk_body.extend_from_slice(&m.y.to_le_bytes());
                    walk_body.push(m.direction);
                    let walk_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectWalk as i16, &walk_body);
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: walk_packet.clone(),
                        });
                    }
                }
            }

            // 处理破损装备广播（避免在怪物循环内借用 self）
            for (target_session, slot) in &broken_armor {
                if let Some(record) = self.players.get(target_session) {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        let (b_min, b_max, b_def, b_hp, b_mp) = calculate_equipment_bonuses(
                            &state.inventory.equipment, &self.item_infos,
                        );
                        let _ = record.actor_ref.ask(crate::actors::player::SetStatBonuses {
                            bonus_min_attack: b_min,
                            bonus_max_attack: b_max,
                            bonus_defence: b_def,
                            bonus_max_hp: b_hp,
                            bonus_max_mp: b_mp,
                        }).await;
                        if *slot == EquipmentSlot::Weapon || *slot == EquipmentSlot::Armour {
                            let weapon_shape = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.shape as i16).unwrap_or(-1);
                            let armor_shape = state.inventory.get_equipment(EquipmentSlot::Armour)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.shape as i16).unwrap_or(0);
                            let weapon_effect = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.effect as i16).unwrap_or(0);
                            for other in self.other_players(*target_session) {
                                send_player_update(
                                    &self.gate_ref, other.session_id, state.object_id,
                                    0, weapon_shape, weapon_effect, armor_shape, 0,
                                );
                            }
                        }
                    }
                }
            }

            // 处理死亡怪物
            for oid in &dead_monsters {
                if let Some(monster) = self.monsters.remove(oid) {
                    debug!("Monster '{}' (#{}) died", monster.name, oid);
                    // 发送 ObjectDied（死亡动画）
                    let mut died_body = Vec::new();
                    died_body.extend_from_slice(&oid.to_le_bytes());
                    died_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                    died_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                    died_body.push(monster.direction);
                    died_body.push(0u8); // death_type = normal
                    let died_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectDied as i16, &died_body);
                    // 发送 ObjectRemove（清理实体）
                    let remove_body = oid.to_le_bytes().to_vec();
                    let remove_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &remove_body);
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: died_packet.clone(),
                        });
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: remove_packet.clone(),
                        });
                    }

                    // 生成掉落物品
                    self.spawn_monster_drops(&monster).await;

                    // 世界Boss被击败广播
                    if monster.is_boss {
                        self.world_boss_queue.remove(oid);
                        broadcast_system_message(
                            &self.gate_ref, &self.players,
                            &format!("世界Boss {} 被英勇的勇士们击败了！", monster.name));
                        debug!("World boss '{}' defeated", monster.name);
                    }

                    // 发放经验（支持组队平分）
                    let mut nearest_session: Option<u64> = None;
                    let mut nearest_dist = i32::MAX;
                    let mut nearest_group_id: Option<u64> = None;
                    for (session_id, record) in &self.players {
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            let dist = (state.x - monster.x).abs() + (state.y - monster.y).abs();
                            if dist < nearest_dist {
                                nearest_dist = dist;
                                nearest_session = Some(*session_id);
                                nearest_group_id = state.group_id;
                            }
                        }
                    }
                    if let Some(session_id) = nearest_session {
                        if let Some(group_id) = nearest_group_id {
                            // 组队经验：组内所有在线成员平分
                            let mut group_sessions = Vec::new();
                            for (sid, record) in &self.players {
                                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                    if state.group_id == Some(group_id) {
                                        let dist = (state.x - monster.x).abs() + (state.y - monster.y).abs();
                                        if dist <= 12 && state.map_index == monster.map_index {
                                            group_sessions.push(*sid);
                                        }
                                    }
                                }
                            }
                            if !group_sessions.is_empty() {
                                let xp_per = (monster.xp / group_sessions.len() as i32).max(1);
                                for sid in &group_sessions {
                                    if let Some(record) = self.players.get(sid) {
                                        let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                            amount: self.apply_global_exp_multiplier(xp_per),
                                        }).await;
                                    }
                                }
                                debug!("GroupXP: {} members split {} xp ({} each) from '{}'", group_sessions.len(), monster.xp, xp_per, monster.name);
                            }
                            // 组队师徒/夫妻经验加成
                            for sid in &group_sessions {
                                if let Some(record) = self.players.get(sid) {
                                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                        // 师徒加成
                                        if let Some(ref mentor_name) = state.mentor_name {
                                            for (other_sid, other_record) in &self.players {
                                                if *other_sid == *sid { continue; }
                                                if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                                    if other_state.name.eq_ignore_ascii_case(mentor_name)
                                                        && other_state.map_index == state.map_index {
                                                        let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                        if dist <= 12 {
                                                            let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                            let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            send_system_message(&self.gate_ref, *sid, "师徒同心，额外获得经验！");
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        // 夫妻加成
                                        if let Some(ref spouse_name) = state.spouse_name {
                                            for (other_sid, other_record) in &self.players {
                                                if *other_sid == *sid { continue; }
                                                if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                                    if other_state.name.eq_ignore_ascii_case(spouse_name)
                                                        && other_state.map_index == state.map_index {
                                                        let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                        if dist <= 12 {
                                                            let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                            let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                                amount: self.apply_global_exp_multiplier(bonus),
                                                            }).await;
                                                            send_system_message(&self.gate_ref, *sid, "夫妻同心，额外获得经验！");
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // 组队任务击杀进度
                            for sid in &group_sessions {
                                if let Some(record) = self.players.get(sid) {
                                    let updates = record.actor_ref.ask(crate::actors::player::ProcessMonsterKill {
                                        monster_index: monster.monster_index,
                                    }).await.unwrap_or_default();
                                    if !updates.is_empty() {
                                        send_system_message(&self.gate_ref, *sid, &format!("任务进度更新：击杀了 {}", monster.name));
                                    }
                                    for (quest_index, _mid, complete) in updates {
                                        debug!("QuestKill: session={} quest={} monster={} complete={}", sid, quest_index, monster.monster_index, complete);
                                    }
                                }
                            }
                        } else if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                amount: self.apply_global_exp_multiplier(monster.xp),
                            }).await;
                            // 单人师徒/夫妻经验加成
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                // 师徒加成
                                if let Some(ref mentor_name) = state.mentor_name {
                                    for (other_sid, other_record) in &self.players {
                                        if *other_sid == session_id { continue; }
                                        if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                            if other_state.name.eq_ignore_ascii_case(mentor_name)
                                                && other_state.map_index == state.map_index {
                                                let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                if dist <= 12 {
                                                    let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                    let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    send_system_message(
                                                        &self.gate_ref, session_id, "师徒同心，额外获得经验！");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                // 夫妻加成
                                if let Some(ref spouse_name) = state.spouse_name {
                                    for (other_sid, other_record) in &self.players {
                                        if *other_sid == session_id { continue; }
                                        if let Ok(Some(other_state)) = other_record.actor_ref.ask(GetPlayerState).await {
                                            if other_state.name.eq_ignore_ascii_case(spouse_name)
                                                && other_state.map_index == state.map_index {
                                                let dist = (other_state.x - state.x).abs() + (other_state.y - state.y).abs();
                                                if dist <= 12 {
                                                    let bonus = (monster.xp as f64 * 0.10).round() as i32;
                                                    let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    let _ = other_record.actor_ref.ask(crate::actors::player::AddExperience {
                                                        amount: self.apply_global_exp_multiplier(bonus),
                                                    }).await;
                                                    send_system_message(
                                                        &self.gate_ref, session_id, "夫妻同心，额外获得经验！");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // 单人任务击杀进度
                            let updates = record.actor_ref.ask(crate::actors::player::ProcessMonsterKill {
                                monster_index: monster.monster_index,
                            }).await.unwrap_or_default();
                            if !updates.is_empty() {
                                send_system_message(&self.gate_ref, session_id, &format!("任务进度更新：击杀了 {}", monster.name));
                            }
                            for (quest_index, _mid, complete) in updates {
                                debug!("QuestKill: session={} quest={} monster={} complete={}", session_id, quest_index, monster.monster_index, complete);
                            }
                        }
                    }

                    // 加入重生队列（3 秒后重生 = 30 ticks @ 100ms）
                    let respawn_tick = self.tick_count + 30;
                    let spawn = MonsterSpawn {
                        name: monster.name.clone(),
                        image: monster.image,
                        monster_index: monster.monster_index,
                        x: monster.spawn_x,
                        y: monster.spawn_y,
                        direction: monster.direction,
                        hp: monster.max_hp,
                        min_dmg: monster.min_dmg,
                        max_dmg: monster.max_dmg,
                        xp: monster.xp,
                        map_index: monster.map_index,
                    };
                    self.respawn_queue.insert(*oid, (spawn, respawn_tick));
                }
            }

            // 处理玩家死亡掉落（在怪物循环外，避免借用冲突）
            for (sid, x, y, map_index) in death_drops {
                self.handle_player_death_drop(sid, x, y, map_index).await;
            }
            // 处理被怪物攻击后的自动下坐骑（在怪物循环外，避免借用冲突）
            for sid in dismount_sessions {
                self.dismount_player(sid).await;
            }
        }

        self.tick_buffs_and_revive().await;

        self.tick_environment_damage().await;

        self.tick_exp_events_and_invisibility().await;

        self.tick_pk_decay().await;

        self.tick_fishing().await;

        self.tick_ground_cleanup().await;

        self.tick_respawn().await;

        self.tick_boss_timeout().await;

        self.tick_quest_timeout().await;

        self.tick_pet_pickup().await;

        self.tick_shop_restock().await;

        self.tick_refine_complete().await;
        self.tick_regen_and_hunger().await;

        self.tick_day_night().await;

        self.tick_auto_save().await;

        self.tick_auction_expiry().await;

        self.tick_rental_expiry().await;
    }
}

impl Message<StartGameRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StartGameRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        info!(
            "StartGame: session={}, account={}, character_index={}",
            msg.session_id, msg.account_username, msg.character_index
        );

        // 尝试从数据库加载角色
        let mut state: Option<PlayerState> = None;
        match db::list_characters_by_account(&self.db_pool, &msg.account_username).await {
            Ok(chars) if !chars.is_empty() => {
                let idx = msg.character_index.max(0) as usize;
                if idx < chars.len() {
                    let (char_name, _map_idx, _x, _y) = &chars[idx];
                    info!("Loading character '{}' for account '{}'", char_name, msg.account_username);
                    if let Ok(Some(loaded)) = db::load_character(&self.db_pool, char_name).await {
                        state = Some(loaded);
                    } else {
                        warn!("Failed to load character '{}' from DB", char_name);
                    }
                }
            }
            Ok(_) => {
                info!("No characters found for account '{}'", msg.account_username);
            }
            Err(e) => {
                warn!("Failed to list characters for account '{}': {}", msg.account_username, e);
            }
        }

        // 如果加载失败，创建默认角色
        let state = state.unwrap_or_else(|| {
            info!("Creating default character for account '{}'", msg.account_username);
            PlayerState {
                object_id: 0,
                name: format!("Player_{}", self.alloc_object_id()),
                map_index: 0,
                x: 330,
                y: 330,
                direction: 4,
                attack_mode: mir2_shared::enums::AttackMode::Peace,
                pet_mode: mir2_shared::enums::PetMode::Both,
                hidden: false,
                session_id: msg.session_id,
                class: mir2_shared::enums::MirClass::Warrior,
                gender: mir2_shared::enums::MirGender::Male,
                hair: 0,
                level: 1,
                experience: 0,
                max_experience: 100,
                hp: 120,
                max_hp: 120,
                mp: 60,
                max_mp: 60,
                min_attack: 5,
                max_attack: 10,
                defence: 2,
                bonus_min_attack: 0,
                bonus_max_attack: 0,
                bonus_defence: 0,
                bonus_max_hp: 0,
                bonus_max_mp: 0,
                inventory: PlayerInventory::new(),
                group_id: None,
                friend_list: FriendList::new(),
                mailbox: Mailbox::new(),
                guild_name: None,
                guild_rank: GuildRank::Member,
                quest_log: QuestLog::new(),
                spouse_name: None,
                allow_mentor: false,
                mentor_name: None,
                creature_log: CreatureLog::new(),
                hero_index: 0,
                hero_behaviour: 0,
                auto_pot_hp: 0,
                auto_pot_mp: 0,
                auto_pot_hp_item: 0,
                auto_pot_mp_item: 0,
                hero_inventory: PlayerInventory::new(),
                refine_log: RefineLog::new(),
                is_fishing: false,
                fishing_autocast: false,
                reincarnation_host: None,
                reincarnation_ready: false,
                reincarnation_expire_time: 0,
                enable_group_recall: false,
                last_recall_time: 0,
                is_dead: false,
                is_mounted: false,
                mount_type: 0,
                allow_lover_recall: false,
                is_gm: false,
                pk_points: 0,
                pk_kill_count: 0,
                buffs: Vec::new(),
                magics: Vec::new(),
                flags: std::collections::HashMap::new(),
                exp_multiplier: 1.0,
                exp_multiplier_end_tick: 0,
            }
        });

        let object_id = self.alloc_object_id();
        let player_name = state.name.clone();
        let map_index = state.map_index;

        // 创建 PlayerActor
        let player_ref = PlayerActor::spawn((
            object_id,
            player_name.clone(),
            msg.session_id,
            map_index,
            self.gate_ref.clone(),
        ));

        // 加载地图 — 优先用 DB 中的 map_infos 获取文件名
        let (map_file, map_title, map_info_idx) = self.map_infos.get(&(map_index as i32))
            .map(|m| (m.file_name.clone(), m.title.clone(), m.index))
            .unwrap_or_else(|| ("n0".to_string(), "Unknown".to_string(), 0));

        if self.get_or_load_map(&map_file).is_some() {
            info!("Map '{}' loaded for player {}", map_file, player_name);
        }

        // 注入地图数据
        if let Some(map_data) = self.maps.get(&0).cloned() {
            let _ = player_ref.ask(SetMapData { map: map_data });
        }

        // 注入数据库加载的状态
        let mut loaded_state = state;
        loaded_state.object_id = object_id;
        loaded_state.session_id = msg.session_id;

        // If position is (0,0), place at map safe zone spawn point
        if loaded_state.x == 0 && loaded_state.y == 0 {
            if let Some(mi) = self.map_infos.get(&(map_index as i32)) {
                if let Some(sz) = mi.safe_zones.iter().find(|s| s.start_point) {
                    info!("Placing {} at safe zone spawn ({}, {})", player_name, sz.x, sz.y);
                    loaded_state.x = sz.x;
                    loaded_state.y = sz.y;
                }
            }
        }

        // 初始化装备属性加成（从已装备物品计算）
        let (b_min, b_max, b_def, b_hp, b_mp) = calculate_equipment_bonuses(
            &loaded_state.inventory.equipment, &self.item_infos,
        );
        loaded_state.bonus_min_attack = b_min;
        loaded_state.bonus_max_attack = b_max;
        loaded_state.bonus_defence = b_def;
        loaded_state.bonus_max_hp = b_hp;
        loaded_state.bonus_max_mp = b_mp;

        let _ = player_ref.ask(SetPlayerState { state: loaded_state.clone() });

        self.players.insert(msg.session_id, PlayerRecord {
            actor_ref: player_ref,
            session_id: msg.session_id,
            name: player_name.clone(),
            account_username: msg.account_username.clone(),
            last_pk_points: loaded_state.pk_points,
            object_id: loaded_state.object_id,
        });

        info!("Player {} entered world (object_id={}, session={})",
              player_name, object_id, msg.session_id);

        // 行会在线状态由 SocialActor 管理

        // 多玩家可见性：向新玩家发送已有玩家的 ObjectPlayer
        let existing_players: Vec<_> = self.players.values()
            .filter(|r| r.session_id != msg.session_id)
            .cloned()
            .collect();

        let invis_tag = std::mem::discriminant(&crate::combat::buff::BuffType::Invisibility);
        for existing in &existing_players {
            if let Ok(Some(ep_state)) = existing.actor_ref.ask(GetPlayerState).await {
                // 跳过隐身玩家
                let is_invisible = ep_state.buffs.iter()
                    .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
                if is_invisible { continue; }
                let ep_weapon = ep_state.inventory.get_equipment(EquipmentSlot::Weapon)
                    .and_then(|item| self.item_infos.get(&item.item_index))
                    .map(|info| info.shape as i16).unwrap_or(-1);
                let ep_armor = ep_state.inventory.get_equipment(EquipmentSlot::Armour)
                    .and_then(|item| self.item_infos.get(&item.item_index))
                    .map(|info| info.shape as i16).unwrap_or(0);
                let ep_weapon_effect = ep_state.inventory.get_equipment(EquipmentSlot::Weapon)
                    .and_then(|item| self.item_infos.get(&item.item_index))
                    .map(|info| info.effect as i16).unwrap_or(0);
                let packet = build_object_player_packet(
                    &ep_state.name, ep_state.object_id, ep_state.x, ep_state.y, ep_state.direction, ep_state.level,
                    name_colour_for_pk(ep_state.pk_points),
                    ep_state.class, ep_state.gender, ep_state.hair,
                    ep_weapon, ep_weapon_effect, ep_armor,
                    ep_state.mount_type, ep_state.is_mounted,
                );
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                });
            }
        }

        // 向已有玩家发送新玩家的 ObjectPlayer（隐身新玩家不发送）
        let new_is_invisible = loaded_state.buffs.iter()
            .any(|b| std::mem::discriminant(&b.buff_type) == invis_tag);
        if new_is_invisible {
            self.invisible_sessions.insert(msg.session_id);
        }
        if !new_is_invisible {
            let new_weapon = loaded_state.inventory.get_equipment(EquipmentSlot::Weapon)
                .and_then(|item| self.item_infos.get(&item.item_index))
                .map(|info| info.shape as i16).unwrap_or(-1);
            let new_armor = loaded_state.inventory.get_equipment(EquipmentSlot::Armour)
                .and_then(|item| self.item_infos.get(&item.item_index))
                .map(|info| info.shape as i16).unwrap_or(0);
            let new_weapon_effect = loaded_state.inventory.get_equipment(EquipmentSlot::Weapon)
                .and_then(|item| self.item_infos.get(&item.item_index))
                .map(|info| info.effect as i16).unwrap_or(0);
            let new_player_packet = build_object_player_packet(
                &player_name, object_id, loaded_state.x, loaded_state.y, loaded_state.direction, loaded_state.level,
                name_colour_for_pk(loaded_state.pk_points),
                loaded_state.class, loaded_state.gender, loaded_state.hair,
                new_weapon, new_weapon_effect, new_armor,
                loaded_state.mount_type, loaded_state.is_mounted,
            );
            for existing in &existing_players {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: existing.session_id,
                    data: new_player_packet.clone(),
                });
            }
        }

        // 发送游戏进入序列（使用真实状态数据）
        let is_big_map = self.map_infos.get(&map_info_idx).map(|m| m.big_map).unwrap_or(false);
        send_game_entry_sequence(self.gate_ref.clone(), msg.session_id, &loaded_state, &map_file, &map_title, is_big_map);

        // 发送地图上的 NPC 和怪物
        let spawn_dir = self.spawn_dir.clone();
        let spawn_ctx = SpawnContext {
            map_info: self.map_infos.get(&map_info_idx),
            monster_infos: &self.monster_infos,
            npc_infos: &self.npc_infos,
            dragon_info: self.dragon_info.as_ref(),
        };
        let (new_npcs, new_monsters) = spawn_npcs_and_monsters(
            self.gate_ref.clone(),
            &spawn_dir,
            &map_file,
            loaded_state.map_index,
            msg.session_id,
            &mut self.next_object_id,
            &spawn_ctx,
        );
        for npc in new_npcs {
            self.npcs.insert(npc.object_id, npc);
        }
        for monster in &new_monsters {
            self.monsters.insert(monster.object_id, monster.clone());
        }

        // 初始生成精英广播
        for monster in &new_monsters {
            if monster.is_elite {
                let map_name = self.map_infos.get(&(map_index as i32)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
                broadcast_system_message(&self.gate_ref, &self.players,
                    &format!("一只 [精英]{} 出现在 {}！勇士们，前往讨伐！", monster.name.strip_prefix("[精英] ").unwrap_or(&monster.name), map_name));
            }
        }

        // 同步当前地图上的地面物品给新玩家
        let map_index_val = loaded_state.map_index;
        let ground_sync: Vec<_> = self.ground_items.iter()
            .filter(|gi| gi.map_index == map_index_val)
            .map(|gi| (gi.object_id, gi.item.clone(), gi.x, gi.y))
            .collect();
        for (drop_oid, item, x, y) in ground_sync {
            if item.item_index == 0 {
                let object_gold = mir2_shared::packets::server::ObjectGold {
                    object_id: drop_oid,
                    gold: item.count as u32,
                    location_x: x,
                    location_y: y,
                };
                let mut buf = Vec::new();
                if mir2_shared::packets::base::serialize_packet(
                    &mut std::io::Cursor::new(&mut buf), &object_gold).is_ok() {
                    let _ = self.gate_ref.ask(SendToClient { session_id: msg.session_id, data: buf });
                }
            } else {
                let object_item = mir2_shared::packets::server::ObjectItem {
                    object_id: drop_oid,
                    item,
                    location_x: x,
                    location_y: y,
                };
                let mut buf = Vec::new();
                if mir2_shared::packets::base::serialize_packet(
                    &mut std::io::Cursor::new(&mut buf), &object_item).is_ok() {
                    let _ = self.gate_ref.ask(SendToClient { session_id: msg.session_id, data: buf });
                }
            }
        }

        // 同步当前地图上已打开的门给新玩家
        let open_doors_sync: Vec<_> = self.open_doors.iter()
            .filter(|(map_idx, _)| *map_idx == map_index_val)
            .map(|(_, door_idx)| *door_idx)
            .collect();
        for door_idx in open_doors_sync {
            send_opendoor(&self.gate_ref, msg.session_id, door_idx, false);
        }

        // 发送已学习的技能列表给客户端
        for magic in &loaded_state.magics {
            if let Some(info) = self.magic_infos.get(&(magic.spell as u32)) {
                let client_magic = mir2_shared::data::client_data::ClientMagic {
                    name: info.name.clone(),
                    spell: mir2_shared::enums::Spell::try_from(magic.spell as u8).unwrap_or(mir2_shared::enums::Spell::None),
                    base_cost: info.base_cost as u8,
                    level_cost: info.level_cost as u8,
                    icon: info.icon as u8,
                    level1: info.level1 as u8,
                    level2: info.level2 as u8,
                    level3: info.level3 as u8,
                    need1: info.need1 as u16,
                    need2: info.need2 as u16,
                    need3: info.need3 as u16,
                    level: magic.level,
                    key: magic.key,
                    experience: magic.experience,
                    delay: info.delay_base as i64,
                    range: info.range as u8,
                    cast_time: 0,
                };
                let new_magic = mir2_shared::packets::server::magic::NewMagic { magic: client_magic, hero: false };
                let mut body = Vec::new();
                if new_magic.write_body(&mut body).is_ok() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewMagic as i16, &body),
                    });
                }
                // Send SpellToggle for toggled-on spells
                if magic.toggled {
                    let mut toggle_body = Vec::new();
                    toggle_body.extend_from_slice(&loaded_state.object_id.to_le_bytes());
                    toggle_body.push(magic.spell as u8);
                    toggle_body.push(1u8); // canUse = true
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SpellToggle as i16, &toggle_body),
                    });
                }
            }
        }

        // 发送当前昼夜光照给新玩家
        self.send_time_of_day(msg.session_id, self.current_light);

        // 发送自动药水设置（恢复持久化数据）
        if loaded_state.auto_pot_hp > 0 {
            let mut body = Vec::new();
            body.push(12u8); // Stat = HP (C# Stat.HP = 12)
            body.extend_from_slice(&loaded_state.auto_pot_hp.to_le_bytes());
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotValue as i16, &body),
            });
        }
        if loaded_state.auto_pot_mp > 0 {
            let mut body = Vec::new();
            body.push(13u8); // Stat = MP (C# Stat.MP = 13)
            body.extend_from_slice(&loaded_state.auto_pot_mp.to_le_bytes());
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotValue as i16, &body),
            });
        }

        // 发送欢迎消息
        let online_count = self.players.len();
        let light_name = match self.current_light {
            mir2_shared::enums::LightSetting::Dawn => "黎明",
            mir2_shared::enums::LightSetting::Day => "白天",
            mir2_shared::enums::LightSetting::Evening => "黄昏",
            mir2_shared::enums::LightSetting::Night => "夜晚",
            _ => "正常",
        };
        send_system_message(&self.gate_ref, msg.session_id,
            &format!("欢迎来到水晶世界！当前在线玩家: {} 人，当前时间: {}", online_count, light_name));
    }
}

impl Message<WorldMoveRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WorldMoveRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("Move request for unknown session {}", msg.session_id);
                return;
            }
        };
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            if state.is_dead { return; }
        }

        let move_type = if msg.is_run { MoveType::Run } else { MoveType::Walk };

        // 发送移动请求到 PlayerActor
        if let Ok(success) = record.actor_ref.ask(MoveRequest {
            session_id: msg.session_id,
            direction: msg.direction,
            is_run: msg.is_run,
        }).await {
            if !success {
                return;
            }
        } else {
            return;
        }

        // 获取移动后的状态并广播给其他玩家
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            // 隐身玩家移动时不广播给其他人
            if !self.invisible_sessions.contains(&msg.session_id) {
                let others: Vec<_> = self.other_players(msg.session_id)
                    .into_iter()
                    .map(|r| r.actor_ref.clone())
                    .collect();

                for other in others {
                    let _ = other.ask(BroadcastMovement {
                        object_id: state.object_id,
                        x: state.x,
                        y: state.y,
                        direction: state.direction,
                        move_type,
                        exclude_session: msg.session_id,
                    });
                }
            }

            // 检查是否踩到地图传送点（Movement）— O(1) index lookup
            let mv = self.movement_index.get(&(state.map_index as i32, state.x, state.y)).cloned();

            if let Some(mv) = mv {
                let dest_map_index = mv.map_index;
                let dest_x = mv.dest_x;
                let dest_y = mv.dest_y;

                // Look up dest map file name from DB-loaded map_infos
                let dest_map_info = self.map_infos.get(&dest_map_index).cloned();

                if let Some(dest_mi) = dest_map_info {
                    if dest_mi.no_teleport {
                        debug!("Movement trigger blocked: map {} has no_teleport", dest_map_index);
                        return;
                    }
                    // Check no_escape on source map
                    if let Some(src_mi) = self.map_infos.get(&(state.map_index as i32)) {
                        if src_mi.no_escape {
                            debug!("Movement trigger blocked: source map {} has no_escape", state.map_index);
                            return;
                        }
                    }

                    let dest_file = dest_mi.file_name.clone();
                    let dest_title = dest_mi.title.clone();
                    let is_big_map = dest_mi.big_map;
                    let player_ref = record.actor_ref.clone();
                    let player_name = record.name.clone();

                    // Load dest map
                    if self.get_or_load_map(&dest_file).is_some() {
                        info!("Player {} teleported via movement: {} ({},{}) -> {} ({},{})",
                            player_name, state.map_index, state.x, state.y,
                            dest_map_index, dest_x, dest_y);

                        // Inject new map data into player for collision/pathfinding
                        if let Some(map_data) = self.maps.get(&0).cloned() {
                            let _ = player_ref.ask(SetMapData { map: map_data });
                        }

                        // Update player position
                        let _ = player_ref.ask(SetPlayerPosition {
                            x: dest_x,
                            y: dest_y,
                            direction: state.direction,
                            map_index: Some(dest_map_index as u16),
                            is_mounted: None,
                        });

                        // Send MapChanged packet
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: msg.session_id,
                            data: build_map_changed_packet(dest_map_index as u16, &dest_file, &dest_title, dest_x, dest_y, is_big_map),
                        });

                        // Send UserLocation to confirm new position
                        if let Ok(Some(new_state)) = player_ref.ask(GetPlayerState).await {
                            let mut loc_body = Vec::new();
                            loc_body.extend_from_slice(&(new_state.x as u32).to_le_bytes());
                            loc_body.extend_from_slice(&(new_state.y as u32).to_le_bytes());
                            loc_body.push(new_state.direction);
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: msg.session_id,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &loc_body),
                            });
                        }

                        // 清理旧地图视野：发送 ObjectRemove 给该玩家（移除旧地图上的怪物/玩家/地面物品）
                        let old_map = state.map_index;
                        for (oid, monster) in &self.monsters {
                            if monster.map_index == old_map {
                                let mut rb = Vec::new();
                                rb.extend_from_slice(&oid.to_le_bytes());
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: msg.session_id,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rb),
                                });
                            }
                        }
                        for (sid, rec) in &self.players {
                            if *sid != msg.session_id {
                                if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                                    if s.map_index == old_map {
                                        let mut rb = Vec::new();
                                        rb.extend_from_slice(&s.object_id.to_le_bytes());
                                        let _ = self.gate_ref.ask(SendToClient {
                                            session_id: msg.session_id,
                                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rb),
                                        });
                                    }
                                }
                            }
                        }
                        for gi in &self.ground_items {
                            if gi.map_index == old_map {
                                let mut rb = Vec::new();
                                rb.extend_from_slice(&gi.object_id.to_le_bytes());
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: msg.session_id,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &rb),
                                });
                            }
                        }

                        // 发送新地图上的 NPC 和怪物给该玩家
                        let spawn_ctx = SpawnContext {
                            map_info: self.map_infos.get(&dest_map_index),
                            monster_infos: &self.monster_infos,
                            npc_infos: &self.npc_infos,
                            dragon_info: self.dragon_info.as_ref(),
                        };
                        let dest_file_clone = dest_file.clone();
                        let (new_npcs, new_monsters) = spawn_npcs_and_monsters(
                            self.gate_ref.clone(),
                            &self.spawn_dir,
                            &dest_file_clone,
                            dest_map_index as u16,
                            msg.session_id,
                            &mut self.next_object_id,
                            &spawn_ctx,
                        );
                        for npc in new_npcs {
                            self.npcs.insert(npc.object_id, npc);
                        }
                        for monster in &new_monsters {
                            self.monsters.insert(monster.object_id, monster.clone());
                        }

                        // 初始生成精英广播
                        for monster in &new_monsters {
                            if monster.is_elite {
                                let map_name = self.map_infos.get(&(dest_map_index)).map(|m| m.title.clone()).unwrap_or_else(|| "未知地图".to_string());
                                broadcast_system_message(
                                    &self.gate_ref, &self.players,
                                    &format!("一只 [精英]{} 出现在 {}！勇士们，前往讨伐！", monster.name.strip_prefix("[精英] ").unwrap_or(&monster.name), map_name));
                            }
                        }

                        // 同步新地图上的地面物品
                        let dest_map_u16 = dest_map_index as u16;
                        for gi in &self.ground_items {
                            if gi.map_index != dest_map_u16 { continue; }
                            if gi.item.item_index == 0 {
                                let object_gold = mir2_shared::packets::server::ObjectGold {
                                    object_id: gi.object_id,
                                    gold: gi.item.count as u32,
                                    location_x: gi.x,
                                    location_y: gi.y,
                                };
                                let mut buf = Vec::new();
                                if mir2_shared::packets::base::serialize_packet(
                                    &mut std::io::Cursor::new(&mut buf), &object_gold).is_ok() {
                                    let _ = self.gate_ref.ask(SendToClient { session_id: msg.session_id, data: buf });
                                }
                            } else {
                                let object_item = mir2_shared::packets::server::ObjectItem {
                                    object_id: gi.object_id,
                                    item: gi.item.clone(),
                                    location_x: gi.x,
                                    location_y: gi.y,
                                };
                                let mut buf = Vec::new();
                                if mir2_shared::packets::base::serialize_packet(
                                    &mut std::io::Cursor::new(&mut buf), &object_item).is_ok() {
                                    let _ = self.gate_ref.ask(SendToClient { session_id: msg.session_id, data: buf });
                                }
                            }
                        }

                        // 同步新地图上已打开的门
                        for (map_idx, door_idx) in &self.open_doors {
                            if *map_idx == dest_map_u16 {
                                send_opendoor(&self.gate_ref, msg.session_id, *door_idx, false);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Message<WorldTurnRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WorldTurnRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("Turn request for unknown session {}", msg.session_id);
                return;
            }
        };

        let _ = record.actor_ref.ask(TurnRequest {
            session_id: msg.session_id,
            direction: msg.direction,
        }).await;

        // 广播转向
        if let Ok(Some(state)) = record.actor_ref.ask(crate::actors::player::GetPlayerState).await {
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| r.actor_ref.clone())
                .collect();

            for other in others {
                let _ = other.ask(BroadcastMovement {
                    object_id: state.object_id,
                    x: state.x,
                    y: state.y,
                    direction: state.direction,
                    move_type: MoveType::Turn,
                    exclude_session: msg.session_id,
                });
            }
        }
    }
}

impl Message<PlayerDisconnected> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: PlayerDisconnected,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.remove(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        self.invisible_sessions.remove(&msg.session_id);
        self.market_search_cache.remove(&msg.session_id);

        info!("Player removed from world (session={})", msg.session_id);

        // 保存玩家数据到数据库
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            if let Err(e) = db::save_character(&self.db_pool, &state, &record.account_username).await {
                warn!("Failed to save player {} on disconnect: {}", record.name, e);
            } else {
                info!("Player {} saved to database on disconnect", record.name);
            }

            // 行会离线状态由 SocialActor 管理
        }

        // 组队离线状态由 SocialActor 管理

        // 通知其他玩家该玩家已离开
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| (r.actor_ref.clone(), r.session_id))
                .collect();

            let opcode = mir2_shared::enums::ServerPacketIds::ObjectRemove as i16;
            let mut body = Vec::new();
            body.extend_from_slice(&state.object_id.to_le_bytes());
            let packet = build_packet_bytes(opcode, &body);

            for (_, other_session) in others {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: other_session,
                    data: packet.clone(),
                });
            }
        }
    }
}

impl Message<WorldAttackRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WorldAttackRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => {
                warn!("Attack request for unknown session {}", msg.session_id);
                return;
            }
        };

        // 发送攻击请求到 PlayerActor，同时获取玩家属性用于伤害计算
        let attacker_state = record.actor_ref.ask(GetPlayerState).await.ok().flatten();
        if let Some(ref state) = attacker_state {
            if state.is_dead { return; }
        }

        // 攻击时自动下坐骑
        self.dismount_player(msg.session_id).await;

        // 攻击时打破隐身
        if self.invisible_sessions.remove(&msg.session_id) {
            if let Some(ref state) = attacker_state {
                let _ = record.actor_ref.ask(crate::actors::player::RemoveBuff {
                    buff_type: crate::combat::buff::BuffType::Invisibility,
                }).await;
                self.reveal_player_to_others(msg.session_id, state).await;
            }
        }

        if let (Some(ref state), Ok(Some(result))) = (attacker_state, record.actor_ref.ask(AttackRequest {
            session_id: msg.session_id,
            direction: msg.direction,
            spell: msg.spell,
        }).await) {
            // 广播 ObjectAttack 给其他玩家
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| (r.actor_ref.clone(), r.session_id))
                .collect();

            let mut attack_body = Vec::new();
            attack_body.extend_from_slice(&result.object_id.to_le_bytes());
            attack_body.extend_from_slice(&(result.x as u32).to_le_bytes());
            attack_body.extend_from_slice(&(result.y as u32).to_le_bytes());
            attack_body.push(result.direction);
            attack_body.push(result.spell);
            attack_body.extend_from_slice(&0u16.to_le_bytes()); // spell_level
            attack_body.push(0u8); // attack_type
            let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);

            // --- 检测是否命中怪物 ---
            // 计算攻击方向的前方位置
            let atk_dir = result.direction as usize % 8;
            let target_x = result.x + MON_DIR_DX[atk_dir];
            let target_y = result.y + MON_DIR_DY[atk_dir];

            let mut hit_monster = false;
            for (oid, monster) in &mut self.monsters {
                let dist = (monster.x - target_x).abs() + (monster.y - target_y).abs();
                if dist <= 1 {
                    // 命中怪物 - 使用战斗模块计算伤害（包含 Buff 加成）
                    let attack_result = combat_attack::resolve_attack(
                        state.effective_min_attack(), state.effective_max_attack(), 0
                    );
                    let damage = attack_result.damage;
                    monster.hp = monster.hp.saturating_sub(damage);
                    monster.provoked = true;
                    monster.target_session = Some(msg.session_id);
                    debug!("Player {} hit monster '{}' (#{}) for {} dmg (crit={}) (hp={}/{})",
                           result.object_id, monster.name, *oid, damage, attack_result.is_critical, monster.hp, monster.max_hp);

                    // 发送 ObjectStruck（受击动画）
                    let mut struck_body = Vec::new();
                    struck_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&result.object_id.to_le_bytes());
                    struck_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                    struck_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                    struck_body.push(monster.direction);
                    let struck_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);

                    // 发送 DamageIndicator（伤害数字）
                    let mut dmg_body = Vec::new();
                    dmg_body.extend_from_slice(&damage.to_le_bytes());
                    dmg_body.push(0u8); // damage_type = normal
                    dmg_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    let dmg_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);

                    // 发送 ObjectHealth（百分比血条）
                    let percent = ((monster.hp.max(0) as f32 / monster.max_hp as f32) * 100.0) as u8;
                    let mut health_body = Vec::new();
                    health_body.extend_from_slice(&monster.object_id.to_le_bytes());
                    health_body.push(percent);
                    health_body.extend_from_slice(&0u16.to_le_bytes()); // expire
                    let health_packet = build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::ObjectHealth as i16, &health_body);

                    // 广播给所有玩家
                    for session_id in self.players.keys() {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: struck_packet.clone(),
                        });
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: dmg_packet.clone(),
                        });
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *session_id,
                            data: health_packet.clone(),
                        });
                    }

                    hit_monster = true;
                    break; // 一次只打一只
                }
            }

            // 武器耐久损耗（每次攻击一次）
            if hit_monster {
                if let Some(record) = self.players.get(&msg.session_id) {
                    let broke = record.actor_ref.ask(crate::actors::player::DamageEquipment {
                        slot: EquipmentSlot::Weapon,
                        amount: 1,
                    }).await.unwrap_or(false);
                    if broke {
                        debug!("Player {} weapon broke!", result.object_id);
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            let (b_min, b_max, b_def, b_hp, b_mp) = calculate_equipment_bonuses(
                                &state.inventory.equipment, &self.item_infos,
                            );
                            let _ = record.actor_ref.ask(crate::actors::player::SetStatBonuses {
                                bonus_min_attack: b_min,
                                bonus_max_attack: b_max,
                                bonus_defence: b_def,
                                bonus_max_hp: b_hp,
                                bonus_max_mp: b_mp,
                            }).await;
                            let weapon_shape = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.shape as i16).unwrap_or(-1);
                            let armor_shape = state.inventory.get_equipment(EquipmentSlot::Armour)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.shape as i16).unwrap_or(0);
                            let weapon_effect = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.effect as i16).unwrap_or(0);
                            let light: u8 = state.inventory.get_equipment(EquipmentSlot::Weapon)
                                .and_then(|item| self.item_infos.get(&item.item_index))
                                .map(|info| info.light as u8)
                                .unwrap_or(0)
                                .max(state.inventory.get_equipment(EquipmentSlot::Armour)
                                    .and_then(|item| self.item_infos.get(&item.item_index))
                                    .map(|info| info.light as u8)
                                    .unwrap_or(0));
                            for other in self.other_players(msg.session_id) {
                                send_player_update(
                                    &self.gate_ref, other.session_id, state.object_id,
                                    light, weapon_shape, weapon_effect, armor_shape, 0,
                                );
                            }
                        }
                    }
                }
            }

            // --- 玩家间伤害（仅在未命中怪物时） ---
            if !hit_monster {
                for (other_actor, other_session) in others {
                    // 获取其他玩家位置做距离检测
                    if let Ok(Some(other_state)) = other_actor.ask(GetPlayerState).await {
                        // 计算曼哈顿距离（Mir2 使用 8 方向近战范围约 1-2 格）
                        let dist = (other_state.x - result.x).abs() + (other_state.y - result.y).abs();
                        const MELEE_RANGE: i32 = 2; // 近战有效范围

                        // 发送 ObjectAttack 动画（无论距离）
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: other_session,
                            data: packet.clone(),
                        });

                        // 只有范围内的玩家才受到伤害
                        if dist <= MELEE_RANGE {
                            // 攻击模式检查
                            if !can_attack_player(state, &other_state) {
                                continue;
                            }

                            // 安全区保护：双方任一在安全区内则禁止伤害
                            let attacker_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            let target_safe = self.maps.get(&other_state.map_index)
                                .map(|m| m.is_safe_zone(other_state.x, other_state.y))
                                .unwrap_or(false);
                            if attacker_safe || target_safe {
                                continue;
                            }

                            // 使用战斗模块计算伤害（包含 Buff 加成）
                            let attack_result = combat_attack::resolve_attack(
                                state.effective_min_attack(), state.effective_max_attack(), other_state.effective_defence()
                            );
                            let damage = attack_result.damage;
                            if other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage,
                            }).await.unwrap_or(false) {
                                let mut died_body = Vec::new();
                                died_body.extend_from_slice(&other_state.object_id.to_le_bytes());
                                died_body.extend_from_slice(&(other_state.x as u32).to_le_bytes());
                                died_body.extend_from_slice(&(other_state.y as u32).to_le_bytes());
                                died_body.push(other_state.direction);
                                died_body.push(0u8);
                                let died_packet = build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::ObjectDied as i16, &died_body);
                                for (sid, _) in &self.players {
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id: *sid,
                                        data: died_packet.clone(),
                                    });
                                }
                                self.handle_player_death_drop(other_session, other_state.x, other_state.y, other_state.map_index).await;

                                // 击杀玩家：增加 PK 值并广播名字颜色变化
                                let _ = record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
                                if let Ok(Some(attacker_state)) = record.actor_ref.ask(GetPlayerState).await {
                                    let colour_packet = build_object_colour_changed_packet(
                                        attacker_state.object_id,
                                        name_colour_for_pk(attacker_state.pk_points),
                                    );
                                    for (sid, _) in &self.players {
                                        let _ = self.gate_ref.ask(SendToClient {
                                            session_id: *sid,
                                            data: colour_packet.clone(),
                                        });
                                    }
                                    if let Some(r) = self.players.get_mut(&msg.session_id) {
                                        r.last_pk_points = attacker_state.pk_points;
                                    }
                                }
                            }
                            debug!("Hit! {} damaged {} for {} (dist={}, crit={})",
                                   result.object_id, other_state.name, damage, dist, attack_result.is_critical);
                        }
                    }
                }
            } else {
                // 命中怪物时也要广播 ObjectAttack 给所有玩家
                for (_other_actor, other_session) in &self.players.iter().map(|(s, r)| (r.actor_ref.clone(), *s)).collect::<Vec<_>>() {
                    let _ = self.gate_ref.ask(SendToClient {
                        session_id: *other_session,
                        data: packet.clone(),
                    });
                }
            }
        }
    }
}

// ============================================================
// 采集系统（Harvest：挖矿/采集）
// ============================================================

/// 方向到坐标偏移（8 方向）
const HARVEST_DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const HARVEST_DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

impl Message<HarvestRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: HarvestRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let dir = msg.direction as usize % 8;
        let target_x = state.x + HARVEST_DIR_DX[dir];
        let target_y = state.y + HARVEST_DIR_DY[dir];

        debug!(
            "Harvest: {} session={} dir={} target=({}, {})",
            state.name, msg.session_id, dir, target_x, target_y
        );

        // 检查当前地图是否可采集
        let map_info = self.map_infos.get(&(state.map_index as i32));
        let mine_index = map_info.map(|m| m.mine_index).unwrap_or(0);
        if mine_index <= 0 {
            send_system_message(&self.gate_ref, msg.session_id, "这里没有什么可采集的");
            return;
        }

        // 检查是否持有镐类工具
        let has_pickaxe = state.inventory.backpack.iter().chain(state.inventory.storage.iter())
            .any(|slot| {
                if let Some(item) = slot {
                    self.item_infos.get(&item.item.item_index)
                        .map(|info| {
                            let n = info.name.to_lowercase();
                            n.contains('镐') || n.contains("pick") || n.contains("hoe") || n.contains("锄")
                        })
                        .unwrap_or(false)
                } else {
                    false
                }
            });
        if !has_pickaxe {
            send_system_message(&self.gate_ref, msg.session_id, "你需要一把镐才能采矿");
            return;
        }

        // 广播 ObjectHarvest 给附近其他玩家
        let harvest_body = {
            let mut b = Vec::new();
            b.extend_from_slice(&state.object_id.to_le_bytes());
            b.extend_from_slice(&(target_x as i32).to_le_bytes());
            b.extend_from_slice(&(target_y as i32).to_le_bytes());
            b.push(msg.direction);
            build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectHarvest as i16, &b)
        };
        for other in self.other_players(msg.session_id) {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: other.session_id,
                data: harvest_body.clone(),
            });
        }

        // 延迟处理采集结果
        let object_id = state.object_id;
        let gate_ref = self.gate_ref.clone();
        let actor_ref = record.actor_ref.clone();
        let item_infos = self.item_infos.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let mut b = Vec::new();
            b.extend_from_slice(&object_id.to_le_bytes());
            b.extend_from_slice(&(target_x as i32).to_le_bytes());
            b.extend_from_slice(&(target_y as i32).to_le_bytes());
            b.push(msg.direction);
            let packet = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectHarvested as i16, &b,
            );
            let _ = gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: packet,
            });

            // 掉落判定
            let roll = (msg.session_id.wrapping_add(tokio::time::Instant::now().elapsed().as_millis() as u64) % 100) as u8;
            let (drop_item_index, drop_count, drop_name) = match mine_index {
                1 if roll < 70 => (500, 1 + (roll % 2) as u16, "铁矿石"),
                2 if roll < 50 => (501, 1, "金矿石"),
                3 if roll < 30 => (502, 1, "宝石"),
                _ => (0, 0, ""),
            };
            if drop_item_index > 0 {
                let item_name = item_infos.get(&drop_item_index)
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| drop_name.to_string());
                let item = mir2_shared::data::item::UserItem {
                    item_index: drop_item_index,
                    count: drop_count,
                    ..Default::default()
                };
                let _ = actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
                send_system_message(&gate_ref, msg.session_id,
                    &format!("采集成功！获得了 {} x{}", item_name, drop_count));
            } else {
                send_system_message(&gate_ref, msg.session_id, "采集成功，但这次什么也没有挖到");
            }
        });
    }
}

impl Message<PlayerLogOut> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: PlayerLogOut,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.remove(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("Logout request for unknown session {}", msg.session_id);
                return;
            }
        };
        self.invisible_sessions.remove(&msg.session_id);
        self.market_search_cache.remove(&msg.session_id);

        // Clean up active rental sessions involving this player
        if let Some(session) = self.rental_sessions.remove(&msg.session_id) {
            // This player was the renter (initiator) - return item to owner
            if let Some(item) = session.owner_item {
                if let Some(owner_record) = self.players.get(&session.partner_session) {
                    let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
                    send_system_message(&self.gate_ref, session.partner_session, "租赁对方已下线，物品已退回");
                }
            }
        }
        // Check if this player is the owner in someone else's rental session
        let renter_session = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);
        if let Some(renter_sid) = renter_session {
            if let Some(session) = self.rental_sessions.remove(&renter_sid) {
                // Return item to this player (owner, who is logging out)
                if let Some(item) = session.owner_item {
                    let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
                }
                send_system_message(&self.gate_ref, renter_sid, "租赁对方已下线，租赁已取消");
            }
        }

        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            info!("Player {} logged out (session={})", state.name, msg.session_id);

            // 保存玩家数据到数据库
            if let Err(e) = db::save_character(&self.db_pool, &state, &record.account_username).await {
                warn!("Failed to save player {} on logout: {}", record.name, e);
            } else {
                info!("Player {} saved to database on logout", record.name);
            }

            // 发送 LogOutSuccess 给客户端
            let mut body = Vec::new();
            body.extend_from_slice(&0i32.to_le_bytes()); // character count = 0
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LogOutSuccess as i16, &body),
            });

            // 通知其他玩家该玩家已离开
            let others: Vec<_> = self.other_players(msg.session_id)
                .into_iter()
                .map(|r| (r.actor_ref.clone(), r.session_id))
                .collect();

            let opcode = mir2_shared::enums::ServerPacketIds::ObjectRemove as i16;
            let mut remove_body = Vec::new();
            remove_body.extend_from_slice(&state.object_id.to_le_bytes());
            let packet = build_packet_bytes(opcode, &remove_body);

            for (_, other_session) in others {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: other_session,
                    data: packet.clone(),
                });
            }
        }
        // 玩家已从 self.players 移除，无需再发 PlayerDisconnected
    }
}

impl Message<ChatRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ChatRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        use mir2_shared::globals::MAX_CHAT_LENGTH;

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("Chat from unknown session {}", msg.session_id);
                return;
            }
        };

        // 截断过长消息（避免 UTF-8 边界截断导致 panic）
        let message = if msg.message.len() > MAX_CHAT_LENGTH {
            msg.message.chars().take(MAX_CHAT_LENGTH).collect()
        } else {
            msg.message
        };

        if message.trim().is_empty() {
            return;
        }

        // Check for social chat commands and forward to SocialActor
        let parts: Vec<&str> = message.split_whitespace().collect();
        let cmd = parts.first().unwrap_or(&"").to_uppercase();
        match cmd.as_str() {
            "GROUPRECALL" | "RECALLMEMBER" | "RECALL" | "ENABLEGROUPRECALL" | "DISABLEGROUPRECALL" | "RIDE" => {
                let args: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();
                let _ = self.social_ref.ask(SocialChatCommand {
                    session_id: msg.session_id,
                    command: cmd,
                    args,
                }).await;
                return;
            }
            _ => {}
        }

        // 获取玩家名称、组队和公会信息
        let (player_name, group_id, guild_name) = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            (state.name, state.group_id, state.guild_name.clone())
        } else {
            return;
        };

        // 私聊 /w <name> <message>
        if let Some(whisper_cmd) = message.strip_prefix("/w ").or_else(|| message.strip_prefix("/W ")) {
            let mut whisper_parts = whisper_cmd.splitn(2, ' ');
            let target_name = whisper_parts.next().unwrap_or("").trim();
            let whisper_msg = whisper_parts.next().unwrap_or("").trim();
            if !target_name.is_empty() && !whisper_msg.is_empty() {
                let mut found = false;
                for (sid, other) in &self.players {
                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                        if os.name.eq_ignore_ascii_case(target_name) {
                            found = true;
                            // 发给目标: WhisperIn
                            let mut in_body = Vec::new();
                            write_dotnet_string(&mut in_body, &format!("{}: {}", player_name, whisper_msg));
                            in_body.push(mir2_shared::enums::ChatType::WhisperIn as u8);
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: *sid,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &in_body),
                            });
                            // 发给自己: WhisperOut
                            let mut out_body = Vec::new();
                            write_dotnet_string(&mut out_body, &format!("-> {}: {}", target_name, whisper_msg));
                            out_body.push(mir2_shared::enums::ChatType::WhisperOut as u8);
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: msg.session_id,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &out_body),
                            });
                            debug!("Whisper: {} -> {}: {}", player_name, target_name, whisper_msg);
                            break;
                        }
                    }
                }
                if !found {
                    send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                }
                return;
            }
        }

        // 组队聊天 /g <message> 或 ! <message>
        let group_msg = message.strip_prefix("/g ").or_else(|| message.strip_prefix("/G "))
            .or_else(|| message.strip_prefix("! "));
        if let Some(gmsg) = group_msg {
            let gmsg = gmsg.trim();
            if !gmsg.is_empty() {
                if let Some(gid) = group_id {
                    let mut sent = false;
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.group_id == Some(gid) {
                                let mut body = Vec::new();
                                write_dotnet_string(&mut body, &format!("[组队] {}: {}", player_name, gmsg));
                                body.push(mir2_shared::enums::ChatType::Group as u8);
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                                });
                                sent = true;
                            }
                        }
                    }
                    if sent {
                        debug!("Group chat: {} (group={}): {}", player_name, gid, gmsg);
                    }
                    return;
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "你不在队伍中");
                    return;
                }
            }
        }

        // 公会聊天 /guild <message> 或 /gu <message>
        let guild_msg = message.strip_prefix("/guild ").or_else(|| message.strip_prefix("/GUILD "))
            .or_else(|| message.strip_prefix("/gu ")).or_else(|| message.strip_prefix("/GU "));
        if let Some(gmsg) = guild_msg {
            let gmsg = gmsg.trim();
            if !gmsg.is_empty() {
                if let Some(ref gname) = guild_name {
                    let mut sent = false;
                    for (sid, other) in &self.players {
                        if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                            if os.guild_name.as_ref() == Some(gname) {
                                let mut body = Vec::new();
                                write_dotnet_string(&mut body, &format!("[公会] {}: {}", player_name, gmsg));
                                body.push(mir2_shared::enums::ChatType::Guild as u8);
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                                });
                                sent = true;
                            }
                        }
                    }
                    if sent {
                        debug!("Guild chat: {} (guild={}): {}", player_name, gname, gmsg);
                    }
                    return;
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "你不在公会中");
                    return;
                }
            }
        }

        // 喊话 /s <message> — 同地图广播
        if let Some(smsg) = message.strip_prefix("/s ").or_else(|| message.strip_prefix("/S ")) {
            let smsg = smsg.trim();
            if !smsg.is_empty() {
                let sender_map = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    state.map_index
                } else {
                    return;
                };
                let mut sent = 0usize;
                for (sid, other) in &self.players {
                    if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                        if os.map_index == sender_map {
                            let mut body = Vec::new();
                            write_dotnet_string(&mut body, &format!("[喊话] {}: {}", player_name, smsg));
                            body.push(mir2_shared::enums::ChatType::Shout as u8);
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: *sid,
                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body),
                            });
                            sent += 1;
                        }
                    }
                }
                debug!("Shout: {} on map {}: {} ({} recipients)", player_name, sender_map, smsg, sent);
                return;
            }
        }

        // GM 全服公告 /announce <message>
        if let Some(amsg) = message.strip_prefix("/announce ").or_else(|| message.strip_prefix("/ANNOUNCE ")) {
            let amsg = amsg.trim();
            if !amsg.is_empty() {
                if amsg.len() > MAX_CHAT_LENGTH {
                    send_system_message(&self.gate_ref, msg.session_id, "公告内容过长");
                    return;
                }
                let is_gm = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    state.is_gm
                } else {
                    false
                };
                if is_gm {
                    broadcast_system_message(&self.gate_ref, &self.players,
                        &format!("[公告] {}", amsg));
                    debug!("Announce: {}", amsg);
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                }
                return;
            }
        }

        // GM 经验活动 /expevent <multiplier> <duration_minutes>
        if let Some(eargs) = message.strip_prefix("/expevent ").or_else(|| message.strip_prefix("/EXPEVENT ")) {
            let is_gm = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                state.is_gm
            } else {
                false
            };
            if !is_gm {
                send_system_message(&self.gate_ref, msg.session_id, "你没有权限使用此命令");
                return;
            }
            let parts: Vec<&str> = eargs.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                if let (Ok(mul), Ok(dur)) = (parts[0].parse::<f64>(), parts[1].parse::<u64>()) {
                    if mul < 1.0 || mul > 10.0 {
                        send_system_message(&self.gate_ref, msg.session_id, "倍率范围: 1.0 ~ 10.0");
                        return;
                    }
                    let dur = dur.min(1440);
                    let duration_ticks = dur * 600; // minutes -> ticks (1 min = 600 ticks @ 100ms)
                    self.global_exp_multiplier = mul;
                    self.global_drop_multiplier = mul;
                    self.global_gold_multiplier = mul;
                    self.global_exp_event_end_tick = self.tick_count + duration_ticks;
                    self.global_event_name = Some("经验活动".to_string());
                    broadcast_system_message(&self.gate_ref, &self.players,
                        &format!("【服务器活动】经验倍率 x{} 已启动，持续 {} 分钟！", mul, dur));
                    debug!("GM {} started exp event: x{} for {} min", msg.session_id, mul, dur);
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "用法: /expevent <倍率> <分钟>");
                }
            } else {
                send_system_message(&self.gate_ref, msg.session_id, "用法: /expevent <倍率> <分钟>");
            }
            return;
        }

        // 在线人数 /online
        if message.trim().eq_ignore_ascii_case("/online") || message.trim().eq_ignore_ascii_case("/who") {
            let count = self.players.len();
            send_system_message(&self.gate_ref, msg.session_id,
                &format!("当前在线玩家: {} 人", count));
            return;
        }

        let formatted = format!("[{}]: {}", player_name, message);
        debug!("Chat from {}: {}", player_name, message);

        // 广播给所有在线玩家（ChatType::Normal = 0）
        // 客户端 read_body 期望: [message: DotNetString][chat_type: u8]
        let mut body = Vec::new();
        write_dotnet_string(&mut body, &formatted);
        body.push(0u8); // ChatType::Normal
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::Chat as i16, &body);

        for session_id in self.players.keys() {
            // 不给自己回发（本地已 add_message）
            if *session_id == msg.session_id {
                continue;
            }
            let _ = self.gate_ref.ask(SendToClient {
                session_id: *session_id,
                data: packet.clone(),
            });
        }
    }
}

impl Message<ChangeAModeRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ChangeAModeRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 更新玩家攻击模式
        let _ = record.actor_ref.ask(SetAttackMode { mode: msg.mode }).await;

        // 发送 ChangeAMode 确认包给客户端
        let body = vec![msg.mode as u8];
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChangeAMode as i16, &body);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: packet,
        });
        debug!("ChangeAMode: session={} mode={:?}", msg.session_id, msg.mode);
    }
}

impl Message<ChangePModeRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ChangePModeRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 更新玩家宠物模式
        let _ = record.actor_ref.ask(SetPetMode { mode: msg.mode }).await;

        // 发送 ChangePMode 确认包给客户端
        let body = vec![msg.mode as u8];
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChangePMode as i16, &body);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: packet,
        });
        debug!("ChangePMode: session={} mode={:?}", msg.session_id, msg.mode);
    }
}

impl Message<SetSpellKeyRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetSpellKeyRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.actor_ref.ask(SetSpellKey {
            spell: msg.spell,
            key: msg.key,
            old_key: msg.old_key,
        }).await;
    }
}

impl Message<SpellToggleRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SpellToggleRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // can_use: -1 = hero toggle (skip for now), 0 = off, 1 = on
        if msg.can_use < 0 { return; }
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let toggled = msg.can_use > 0;
        let object_id = record.object_id;
        let _ = record.actor_ref.ask(ToggleSpell {
            spell: msg.spell,
            toggled,
        }).await;
        // Send SpellToggle confirmation to client
        let mut body = Vec::new();
        body.extend_from_slice(&object_id.to_le_bytes());
        body.push(msg.spell as u8);
        body.push(if toggled { 1u8 } else { 0u8 });
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SpellToggle as i16, &body),
        });
    }
}

impl Message<SetHeroBehaviourRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetHeroBehaviourRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.actor_ref.ask(SetHeroBehaviour { behaviour: msg.behaviour }).await;
        // Send HeroBehaviour confirmation to client
        let body = vec![msg.behaviour];
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetHeroBehaviour as i16, &body),
        });
    }
}

impl Message<SetAutoPotValueRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetAutoPotValueRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.actor_ref.ask(SetAutoPotValue { stat: msg.stat, value: msg.value }).await;
        // Send SetAutoPotValue confirmation to client
        let mut body = Vec::new();
        body.push(msg.stat);
        body.extend_from_slice(&msg.value.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotValue as i16, &body),
        });
    }
}

impl Message<SetAutoPotItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetAutoPotItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.actor_ref.ask(SetAutoPotItem { grid: msg.grid, item_index: msg.item_index }).await;
        // Send SetAutoPotItem confirmation to client
        let mut body = Vec::new();
        body.push(msg.grid);
        body.extend_from_slice(&msg.item_index.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SetAutoPotItem as i16, &body),
        });
    }
}

impl Message<RemoveSlotItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RemoveSlotItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let success = record.actor_ref.ask(RemoveSlotItemMsg {
            grid: msg.grid,
            grid_to: msg.grid_to,
            unique_id: msg.unique_id,
            to: msg.to,
            from_unique_id: msg.from_unique_id,
        }).await.unwrap_or(false);
        // Send RemoveSlotItem response to client
        let mut body = Vec::new();
        body.push(msg.grid);
        body.push(msg.grid_to);
        body.extend_from_slice(&msg.unique_id.to_le_bytes());
        body.extend_from_slice(&msg.to.to_le_bytes());
        body.push(if success { 1u8 } else { 0u8 });
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveSlotItem as i16, &body),
        });
    }
}

impl Message<NPCCallRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: NPCCallRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => {
                warn!("NPC call from unknown session {}", msg.session_id);
                return;
            }
        };

        // 获取玩家状态
        let player_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if player_state.is_dead { return; }
        let player_pos = (player_state.x, player_state.y);

        // 查找对应的 NPC
        let npc = match self.npcs.get(&msg.npc_object_id) {
            Some(n) => n.clone(),
            None => {
                warn!("NPC call for unknown object_id {}", msg.npc_object_id);
                return;
            }
        };

        // 距离校验（NPC 交互范围 2 格）
        let dist = (npc.x - player_pos.0).abs() + (npc.y - player_pos.1).abs();
        if dist > 2 {
            debug!("Player too far from NPC {} (dist={})", npc.name, dist);
            return;
        }

        // DB NPC visibility: time/level/class restrictions
        if npc.db_index > 0 {
            if let Some(npc_db) = self.npc_infos.get(&npc.db_index) {
                if npc_db.min_lev > 0 && player_state.level < npc_db.min_lev as u16 {
                    debug!("NPC {} requires min level {}", npc.name, npc_db.min_lev);
                    return;
                }
                if npc_db.max_lev > 0 && player_state.level > npc_db.max_lev as u16 {
                    debug!("NPC {} requires max level {}", npc.name, npc_db.max_lev);
                    return;
                }
                if let Some(ref required) = npc_db.class_required {
                    let class_name = format!("{:?}", player_state.class);
                    if !required.is_empty() && required != &class_name {
                        debug!("NPC {} requires class {} (player is {})", npc.name, required, class_name);
                        return;
                    }
                }
                if let Some(ref dow) = npc_db.day_of_week {
                    let today = chrono::Utc::now().format("%A").to_string();
                    let today_short = &today[..3];
                    if !dow.is_empty() && !dow.contains(&today) && !dow.contains(today_short) {
                        debug!("NPC {} not available on {}", npc.name, today);
                        return;
                    }
                }
                // Time-based visibility: hour_start/minute_start to hour_end/minute_end
                if npc_db.time_visible > 0 {
                    let now = chrono::Local::now();
                    let current_minutes = now.hour() as i32 * 60 + now.minute() as i32;
                    let start_minutes = npc_db.hour_start * 60 + npc_db.minute_start;
                    let end_minutes = npc_db.hour_end * 60 + npc_db.minute_end;
                    let in_window = if start_minutes <= end_minutes {
                        current_minutes >= start_minutes && current_minutes <= end_minutes
                    } else {
                        // Crosses midnight (e.g. 22:00 to 06:00)
                        current_minutes >= start_minutes || current_minutes <= end_minutes
                    };
                    if !in_window {
                        debug!("NPC {} not available at {}:{} (window {}:{}-{}:{})",
                            npc.name, now.hour(), now.minute(),
                            npc_db.hour_start, npc_db.minute_start, npc_db.hour_end, npc_db.minute_end);
                        return;
                    }
                }
                // Flag requirement check
                if npc_db.flag_needed > 0 {
                    let flag_key = format!("NPC_VISIBLE_{}", npc_db.flag_needed);
                    let has_flag = player_state.flags.get(&flag_key).copied().unwrap_or(0) > 0;
                    if !has_flag {
                        debug!("NPC {} requires flag {}", npc.name, npc_db.flag_needed);
                        return;
                    }
                }
            }
        }

        debug!("Player called NPC '{}' (#{}) with key='{}'", npc.name, msg.npc_object_id, msg.key);

        // 优先使用 DB 脚本（支持 GOTO 跳转）
        let mut dialog_lines = Vec::new();
        let mut current_key = msg.key.clone();
        let mut goto_depth = 0;
        const MAX_GOTO_DEPTH: usize = 10;

        while goto_depth < MAX_GOTO_DEPTH {
            goto_depth += 1;
            let script_key = (npc.db_index, current_key.clone());
            if let Some(mut lines) = self.npc_scripts.get(&script_key).cloned() {
                for line in &mut lines {
                    *line = line.replace("$USERNAME", &player_state.name)
                                .replace("$NPCNAME", &npc.name)
                                .replace("$LEVEL", &player_state.level.to_string());
                }
                let (out, goto) = self.eval_npc_script(&mut lines, msg.session_id, &npc).await;
                if let Some(target) = goto {
                    current_key = format!("[@{}]", target);
                    continue;
                }
                dialog_lines = out;
                break;
            } else {
                dialog_lines = match current_key.as_str() {
                    "[@Main]" => {
                        let mut lines = vec![format!("欢迎来到{}", npc.name)];
                        if npc.db_index > 0 {
                            if let Some(npc_db) = self.npc_infos.get(&npc.db_index) {
                                let pending: Vec<&db::QuestInfo> = npc_db.collect_quest_indexes.iter()
                                    .filter_map(|qi| self.quest_infos.get(qi))
                                    .collect();
                                let finishable: Vec<&db::QuestInfo> = npc_db.finish_quest_indexes.iter()
                                    .filter_map(|qi| self.quest_infos.get(qi))
                                    .collect();
                                if !pending.is_empty() {
                                    lines.push("——可接受任务——".into());
                                    for q in &pending {
                                        lines.push(format!("[{}] {}", q.name, q.file_name));
                                    }
                                }
                                if !finishable.is_empty() {
                                    lines.push("——可完成任务——".into());
                                    for q in &finishable {
                                        lines.push(format!("[{}] {}", q.name, q.file_name));
                                    }
                                }
                            }
                        }
                        if lines.len() == 1 {
                            lines.push("有什么我可以帮你的吗？".into());
                        }
                        if npc.db_index > 0 && self.npc_goods.get(&npc.db_index).is_some_and(|g| !g.is_empty()) {
                            lines.push("<购买/@Buy>".into());
                        }
                        lines.push("<出售/@Sell>".into());
                        lines.push("<修理/@Repair>".into());
                        lines.push("<仓库/@Storage>".into());
                        lines
                    }
                    "[@Buy]" => {
                        self.send_npc_goods(msg.session_id, &npc);
                        return;
                    }
                    "[@Sell]" => {
                        dialog_lines = vec![
                            format!("{}: 请把要出售的物品放入窗口。", npc.name),
                        ];
                        self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Sell);
                        break;
                    }
                    "[@Repair]" => {
                        dialog_lines = vec![
                            format!("{}: 我会帮你修好装备的。", npc.name),
                        ];
                        self.send_npc_panel(msg.session_id, mir2_shared::enums::PanelType::Repair);
                        break;
                    }
                    "[@Storage]" => {
                        dialog_lines = vec![
                            format!("{}: 请妥善保管你的物品。", npc.name),
                        ];
                        self.send_user_storage(msg.session_id, &player_state.inventory.storage);
                        break;
                    }
                    _ => vec![
                        format!("{} 说：", npc.name),
                        format!("你说了：{}", msg.key),
                    ],
                };
                break;
            }
        }

        let mut body = Vec::new();
        body.extend_from_slice(&(dialog_lines.len() as i32).to_le_bytes());
        for line in &dialog_lines {
            write_dotnet_string(&mut body, line);
        }
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCResponse as i16, &body);

        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: packet,
        });
    }
}

// ============================================================
// 物品系统 Handler
// ============================================================

impl Message<PickUpRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: PickUpRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }
        let player_pos = (state.x, state.y);

        // 查找附近可拾取的物品（1 格内，同地图）
        const OWNERSHIP_TICKS: u64 = 300; // ~30 秒保护期
        let pickup_idx = self.ground_items.iter().position(|gi| {
            if gi.map_index != state.map_index { return false; }
            if (gi.x - player_pos.0).abs() > 1 { return false; }
            if (gi.y - player_pos.1).abs() > 1 { return false; }
            // 所有权保护：保护期内只有掉落者可拾取
            if let Some(dropper) = gi.dropper_session {
                if self.tick_count < gi.drop_tick + OWNERSHIP_TICKS && dropper != msg.session_id {
                    return false;
                }
            }
            true
        });

        if let Some(idx) = pickup_idx {
            let ground_item = self.ground_items.remove(idx);
            let picked_oid = ground_item.object_id;
            debug!(
                "Player session={} picked up item uid={} at ({}, {})",
                msg.session_id, ground_item.item.unique_id, ground_item.x, ground_item.y
            );

            // 通知 PlayerActor 添加到背包
            let mut picked_up = false;
            if let Ok(success) = record.actor_ref.ask(AddItemToInventory {
                item: ground_item.item.clone(),
            }).await {
                if success {
                    picked_up = true;
                } else {
                    // 背包已满，放回去
                    self.ground_items.push(ground_item);
                    send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                }
            } else {
                self.ground_items.push(ground_item);
            }

            // 拾取成功：广播 ObjectRemove 给同地图玩家
            if picked_up {
                let mut remove_body = Vec::new();
                remove_body.extend_from_slice(&picked_oid.to_le_bytes());
                let remove_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &remove_body);
                for (sid, rec) in &self.players {
                    if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                        if s.map_index == state.map_index {
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: *sid,
                                data: remove_packet.clone(),
                            });
                        }
                    }
                }
                // 检查任务物品进度
                let updates = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await.unwrap_or_default();
                if !updates.is_empty() {
                    send_system_message(&self.gate_ref, msg.session_id, "任务进度更新：获得物品");
                }
                for (quest_index, _item_index, complete) in updates {
                    debug!("QuestItem: session={} quest={} complete={}", msg.session_id, quest_index, complete);
                }
            }
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "附近没有可以拾取的物品。");
        }
    }
}

impl Message<MoveItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MoveItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let success = record.actor_ref.ask(InventoryMoveItem {
            from_grid: msg.grid,
            to_grid: msg.to as u8,
        }).await.unwrap_or(false);

        if success {
            // 发送 ItemChanged 通知（用 MoveItem 响应）
            send_move_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.from, msg.to, true);
        }
    }
}

impl Message<UseItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: UseItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // Check map no_drug flag
        let player_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if player_state.is_dead { return; }
        if let Some(mi) = self.map_infos.get(&(player_state.map_index as i32)) {
            if mi.no_drug {
                send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用物品");
                return;
            }
        }

        // 查询物品信息
        let user_item = record.actor_ref.ask(GetItemInfo { unique_id: msg.unique_id }).await.unwrap_or(None);
        let item_index = match user_item {
            Some(ref item) => item.item_index,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        let item_db = self.item_infos.get(&item_index).cloned();

        // 消耗品：扣减 count 或移除
        let consumed = record.actor_ref.ask(ConsumeItem { unique_id: msg.unique_id }).await.unwrap_or(false);
        if !consumed {
            send_system_message(&self.gate_ref, msg.session_id, "使用物品失败");
            return;
        }

        debug!("Player session={} used item uid={} index={}", msg.session_id, msg.unique_id, item_index);

        // 特殊物品：双倍经验卷（不依赖 item_type）
        if item_index == 4 {
            let duration_ticks = 6000; // 10分钟 = 6000 ticks @ 100ms
            let end_tick = self.tick_count + duration_ticks;
            let _ = record.actor_ref.ask(SetExpMultiplier {
                multiplier: 2.0,
                end_tick,
            }).await;
            send_system_message(&self.gate_ref, msg.session_id, "双倍经验效果已启动，持续10分钟！");
            debug!("DoubleExpScroll: {} activated 2x exp for 10 min", player_state.name);
        }

        // 根据物品类型执行效果
        if let Some(ref db) = item_db {
            match db.item_type {
                // Potion
                13 => {
                    use mir2_shared::enums::Stat;
                    let hp_recover = db.stats.get(&(Stat::HP as u8)).copied().unwrap_or(0);
                    let mp_recover = db.stats.get(&(Stat::MP as u8)).copied().unwrap_or(0);
                    if hp_recover > 0 {
                        let _ = record.actor_ref.ask(crate::actors::player::Heal {
                            amount: hp_recover,
                        }).await;
                    }
                    if mp_recover > 0 {
                        let _ = record.actor_ref.ask(crate::actors::player::AddMP {
                            amount: mp_recover,
                        }).await;
                    }
                    if hp_recover > 0 || mp_recover > 0 {
                        debug!("Potion: {} recovered hp={} mp={}", player_state.name, hp_recover, mp_recover);
                    }
                }
                // Scroll (回城卷 / 随机传送卷)
                17 => {
                    if let Some(mi) = self.map_infos.get(&(player_state.map_index as i32)) {
                        if mi.no_escape {
                            send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送卷");
                            return;
                        }
                        match item_index {
                            // 回城卷 -> 传送到当前地图安全区
                            2 => {
                                if mi.no_town_teleport {
                                    send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用回城卷");
                                    return;
                                }
                                let (tx, ty) = self.maps.get(&player_state.map_index)
                                    .and_then(|m| m.safe_zone_rects.first())
                                    .map(|(x1, y1, x2, y2)| ((x1 + x2) / 2, (y1 + y2) / 2))
                                    .unwrap_or((330, 330));
                                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x: tx,
                                    y: ty,
                                    direction: player_state.direction,
                                    map_index: None,
                                    is_mounted: None,
                                }).await;
                                send_system_message(&self.gate_ref, msg.session_id, "已返回安全区");
                                debug!("Scroll: {} teleported to safe zone ({}, {})", player_state.name, tx, ty);
                            }
                            // 随机传送卷 -> 传送到当前地图随机可行走位置
                            3 => {
                                if mi.no_random {
                                    send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用随机传送卷");
                                    return;
                                }
                                if let Some(map) = self.maps.get(&player_state.map_index) {
                                    let (max_x, max_y) = (map.width as i32, map.height as i32);
                                    let mut attempts = 0;
                                    let mut rx = player_state.x;
                                    let mut ry = player_state.y;
                                    while attempts < 20 {
                                        let cx = fastrand::i32(0..max_x);
                                        let cy = fastrand::i32(0..max_y);
                                        if map.is_walkable(cx, cy) {
                                            rx = cx;
                                            ry = cy;
                                            break;
                                        }
                                        attempts += 1;
                                    }
                                    let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                        x: rx,
                                        y: ry,
                                        direction: player_state.direction,
                                        map_index: None,
                                        is_mounted: None,
                                    }).await;
                                    send_system_message(&self.gate_ref, msg.session_id, "随机传送完成");
                                    debug!("RandomScroll: {} teleported to ({}, {})", player_state.name, rx, ry);
                                }
                            }
                            _ => {
                                // 未知卷轴 -> 默认回城行为
                                let (tx, ty) = self.maps.get(&player_state.map_index)
                                    .and_then(|m| m.safe_zone_rects.first())
                                    .map(|(x1, y1, x2, y2)| ((x1 + x2) / 2, (y1 + y2) / 2))
                                    .unwrap_or((330, 330));
                                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x: tx,
                                    y: ty,
                                    direction: player_state.direction,
                                    map_index: None,
                                    is_mounted: None,
                                }).await;
                                send_system_message(&self.gate_ref, msg.session_id, "已返回安全区");
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // 发送 UseItem 响应
        send_use_item_response(&self.gate_ref, msg.session_id, msg.unique_id);
    }
}

impl Message<EquipItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: EquipItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let slot = match EquipmentSlot::from_i32(msg.slot) {
            Some(s) => s,
            None => return,
        };

        let result = record.actor_ref.ask(InventoryEquipItem {
            grid: msg.grid,
            slot,
        }).await.unwrap_or(None);

        match result {
            Some((_old_equipment, _new_uid)) => {
                debug!("Player session={} equipped item uid={} to slot {}", msg.session_id, msg.unique_id, msg.slot);
                send_equip_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, msg.slot, true);

                // 重新计算装备加成
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    let (b_min, b_max, b_def, b_hp, b_mp) = calculate_equipment_bonuses(
                        &state.inventory.equipment, &self.item_infos,
                    );
                    let _ = record.actor_ref.ask(crate::actors::player::SetStatBonuses {
                        bonus_min_attack: b_min,
                        bonus_max_attack: b_max,
                        bonus_defence: b_def,
                        bonus_max_hp: b_hp,
                        bonus_max_mp: b_mp,
                    }).await;

                    // 广播装备视觉变化
                    let weapon_shape = state.inventory.get_equipment(EquipmentSlot::Weapon)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.shape as i16)
                        .unwrap_or(-1);
                    let armor_shape = state.inventory.get_equipment(EquipmentSlot::Armour)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.shape as i16)
                        .unwrap_or(0);
                    let weapon_effect = state.inventory.get_equipment(EquipmentSlot::Weapon)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.effect as i16)
                        .unwrap_or(0);
                    let light: u8 = state.inventory.get_equipment(EquipmentSlot::Weapon)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.light as u8)
                        .unwrap_or(0)
                        .max(state.inventory.get_equipment(EquipmentSlot::Armour)
                            .and_then(|item| self.item_infos.get(&item.item_index))
                            .map(|info| info.light as u8)
                            .unwrap_or(0));
                    for other in self.other_players(msg.session_id) {
                        send_player_update(
                            &self.gate_ref, other.session_id, state.object_id,
                            light, weapon_shape, weapon_effect, armor_shape, 0,
                        );
                    }
                }
            }
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "装备失败");
            }
        }
    }
}

impl Message<RemoveItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RemoveItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 找到该 uid 在哪个装备槽位
        let mut found_slot = None;
        for slot_idx in 0..EquipmentSlot::COUNT {
            let slot = EquipmentSlot::from_i32(slot_idx as i32).unwrap();
            let eq_info = record.actor_ref.ask(GetEquipmentInfo { slot }).await.unwrap_or(None);
            if let Some(eq) = eq_info {
                if eq.unique_id == msg.unique_id {
                    found_slot = Some(slot);
                    break;
                }
            }
        }

        let Some(slot) = found_slot else { return; };

        let result = record.actor_ref.ask(InventoryUnequipItem { slot }).await;
        match result {
            Ok(true) => {
                debug!("Player session={} unequipped item uid={} from slot {:?}", msg.session_id, msg.unique_id, slot);
                send_remove_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, true);

                // 重新计算装备加成
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    let (b_min, b_max, b_def, b_hp, b_mp) = calculate_equipment_bonuses(
                        &state.inventory.equipment, &self.item_infos,
                    );
                    let _ = record.actor_ref.ask(crate::actors::player::SetStatBonuses {
                        bonus_min_attack: b_min,
                        bonus_max_attack: b_max,
                        bonus_defence: b_def,
                        bonus_max_hp: b_hp,
                        bonus_max_mp: b_mp,
                    }).await;

                    // 广播装备视觉变化
                    let weapon_shape = state.inventory.get_equipment(EquipmentSlot::Weapon)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.shape as i16)
                        .unwrap_or(-1);
                    let armor_shape = state.inventory.get_equipment(EquipmentSlot::Armour)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.shape as i16)
                        .unwrap_or(0);
                    let weapon_effect = state.inventory.get_equipment(EquipmentSlot::Weapon)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.effect as i16)
                        .unwrap_or(0);
                    let light: u8 = state.inventory.get_equipment(EquipmentSlot::Weapon)
                        .and_then(|item| self.item_infos.get(&item.item_index))
                        .map(|info| info.light as u8)
                        .unwrap_or(0)
                        .max(state.inventory.get_equipment(EquipmentSlot::Armour)
                            .and_then(|item| self.item_infos.get(&item.item_index))
                            .map(|info| info.light as u8)
                            .unwrap_or(0));
                    for other in self.other_players(msg.session_id) {
                        send_player_update(
                            &self.gate_ref, other.session_id, state.object_id,
                            light, weapon_shape, weapon_effect, armor_shape, 0,
                        );
                    }
                }
            }
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "背包已满，无法卸下装备");
            }
        }
    }
}

impl Message<DropItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DropItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let item = record.actor_ref.ask(RemoveItemFromInventory { unique_id: msg.unique_id }).await.unwrap_or(None);
        if let Some(item) = item {
            let player_pos = (state.x, state.y);

            debug!("Player session={} dropped item uid={}", msg.session_id, msg.unique_id);

            // 广播 ObjectItem 给所有玩家
            let drop_oid = self.alloc_object_id();
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: item.clone(),
                location_x: player_pos.0,
                location_y: player_pos.1,
            };
            let mut buf = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item).is_ok() {
                for sid in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: buf.clone() });
                }
            }

            // 添加到地面物品
            self.ground_items.push(GroundItem {
                object_id: drop_oid,
                item: item.clone(),
                x: player_pos.0,
                y: player_pos.1,
                map_index: state.map_index,
                dropper_session: Some(msg.session_id),
                drop_tick: self.tick_count,
            });

            send_drop_item_response(&self.gate_ref, msg.session_id, msg.unique_id, msg.count as u32, true);
        }
    }
}

impl Message<MergeItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MergeItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let success = record.actor_ref.ask(InventoryMergeItem {
            from_grid: msg.grid_from,
            to_grid: msg.grid_to,
        }).await.unwrap_or(false);

        if success {
            send_merge_item_response(&self.gate_ref, msg.session_id, msg.grid_from, msg.grid_to, msg.from_uid, msg.to_uid, true);
        }
    }
}

impl Message<SplitItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SplitItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let success = record.actor_ref.ask(InventorySplitItem {
            grid: msg.grid,
            count: msg.count as u16,
        }).await.unwrap_or(false);

        if success {
            send_split_item_response(&self.gate_ref, msg.session_id, msg.grid, msg.unique_id, msg.count);
        }
    }
}

impl Message<DropGoldRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DropGoldRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        if msg.amount == 0 {
            return;
        }

        if state.inventory.gold < msg.amount as u64 {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        let amount = msg.amount as u64;
        let success = record.actor_ref.ask(DropGold { amount }).await.unwrap_or(false);
        if success {
            let player_pos = match record.actor_ref.ask(GetPlayerState).await {
                Ok(Some(s)) => (s.x, s.y),
                _ => return,
            };

            // 广播 ObjectGold 给所有玩家
            let drop_oid = self.alloc_object_id();
            let object_gold = mir2_shared::packets::server::ObjectGold {
                object_id: drop_oid,
                gold: amount as u32,
                location_x: player_pos.0,
                location_y: player_pos.1,
            };
            let mut buf = Vec::new();
            if mir2_shared::packets::base::serialize_packet(
                &mut std::io::Cursor::new(&mut buf), &object_gold).is_ok() {
                for sid in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: buf.clone() });
                }
            }

            // 地面金币（用特殊物品表示）
            let gold_item = mir2_shared::data::item::UserItem {
                item_index: 0, // 0 = gold marker
                count: amount as u16,
                ..Default::default()
            };
            self.ground_items.push(GroundItem {
                object_id: drop_oid,
                item: gold_item,
                x: player_pos.0,
                y: player_pos.1,
                map_index: state.map_index,
                dropper_session: Some(msg.session_id),
                drop_tick: self.tick_count,
            });

            // 通知客户端金币变化
            send_gold_changed_packet(&self.gate_ref, msg.session_id, state.inventory.gold - amount);
            debug!("DropGold: {} dropped {} gold", state.name, msg.amount);
        }
    }
}

impl Message<BuyItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: BuyItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 查找 NPC 并验证商品是否在销售列表中
        let npc_db_index = match self.npcs.get(&msg.npc_id) {
            Some(n) => n.db_index,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该 NPC");
                return;
            }
        };

        // 获取商品列表（可变引用以便扣减库存）
        let goods_list = match self.npc_goods.get_mut(&npc_db_index) {
            Some(list) => list,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该 NPC 不出售任何物品");
                return;
            }
        };
        let good_idx = match goods_list.iter().position(|g| g.item_index == msg.item_index as i32) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该 NPC 不出售此物品");
                return;
            }
        };

        // 检查库存
        let good = &goods_list[good_idx];
        if !good.infinite_stock && good.stock <= 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品已售罄");
            return;
        }
        if !good.infinite_stock && good.stock < msg.count as i32 {
            send_system_message(&self.gate_ref, msg.session_id, &format!("库存不足（仅剩 {} 个）", good.stock));
            return;
        }

        // Validate item against DB-loaded item_infos
        let item_db = match self.item_infos.get(&(msg.item_index as i32)) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        // 计算价格：优先使用 npc_goods 中的自定义价格，否则使用 item_db.price * NPC rate（整数运算避免浮点误差）
        let npc_rate = self.npc_infos.get(&npc_db_index).map(|n| n.rate).unwrap_or(100).max(1) as u64;
        let base_price = if good.price > 0 { good.price as u64 } else { item_db.price as u64 };
        let price_per_unit = ((base_price * npc_rate) / 100).max(1);
        let total_price = price_per_unit * msg.count as u64;

        // 检查金币
        if state.inventory.gold < total_price {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        // 扣除金币
        let _ = record.actor_ref.ask(DeductGold { amount: total_price }).await;

        // 扣减库存
        if !goods_list[good_idx].infinite_stock {
            goods_list[good_idx].stock -= msg.count as i32;
        }

        // Create item from DB template
        let item = mir2_shared::data::item::UserItem {
            item_index: msg.item_index as i32,
            count: msg.count as u16,
            max_dura: item_db.durability as u16,
            current_dura: item_db.durability as u16,
            ..Default::default()
        };

        let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
        let updates = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await.unwrap_or_default();
        if !updates.is_empty() {
            send_system_message(&self.gate_ref, msg.session_id, "任务进度更新：获得物品");
        }
        send_system_message(&self.gate_ref, msg.session_id, &format!("购买成功 (花费 {} 金币)", total_price));
        let npc_name = self.npcs.get(&msg.npc_id).map(|n| n.name.as_str()).unwrap_or("?");
        debug!("BuyItem: {} bought item={} ({}) x{} for {} gold from NPC '{}' (stock={})", state.name, item_db.name, msg.item_index, msg.count, total_price, npc_name,
            if goods_list[good_idx].infinite_stock { "∞".to_string() } else { goods_list[good_idx].stock.to_string() });
    }
}

impl Message<SellItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SellItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查物品是否在背包中
        let item_data = match state.inventory.get_item(msg.unique_id) {
            Some(i) => i.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        // 移除物品
        let removed = record.actor_ref.ask(RemoveItemFromInventory { unique_id: msg.unique_id }).await.unwrap_or(None);
        if removed.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "移除物品失败");
            return;
        }

        // 定价：基于 DB 中物品的 price（卖价通常为买价的一半）
        let item_db_price = self.item_infos.get(&item_data.item_index)
            .map(|i| i.price as u64)
            .unwrap_or(item_data.item_index as u64 * 5);
        let total_gold = (item_db_price / 2).max(1) * msg.count as u64;

        let success = record.actor_ref.ask(AddGold { amount: total_gold }).await.unwrap_or(false);
        if success {
            // 记录到回购列表（最多保留 10 个）
            let buyback = BuybackItem {
                item: item_data.clone(),
                sell_price: total_gold,
            };
            let list = self.buyback_items.entry(msg.session_id).or_default();
            list.insert(0, buyback);
            while list.len() > 10 {
                list.pop();
            }
            send_sell_item_response(&self.gate_ref, msg.session_id, msg.unique_id, msg.count, true);
            debug!("SellItem: {} sold item={} x{} for {} gold", state.name, item_data.item_index, msg.count, total_gold);
        }
    }
}

// ============================================================
// 修理系统 Handler
// ============================================================

/// 修理费用：每缺失 1 点耐久 = 1 金币
const REPAIR_COST_PER_DURA: u64 = 1;

/// 修理物品请求
pub struct RepairItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<RepairItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RepairItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 获取物品信息计算修理费
        let item_data = match state.inventory.get_item(msg.unique_id) {
            Some(i) => i.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };

        // 计算耐久缺失和修理费
        let dura_deficit = item_data.max_dura.saturating_sub(item_data.current_dura) as u64;
        if dura_deficit == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品不需要修理");
            return;
        }
        let repair_cost = dura_deficit * REPAIR_COST_PER_DURA;

        // 检查金币
        if state.inventory.gold < repair_cost {
            send_system_message(&self.gate_ref, msg.session_id, &format!("金币不足（需要 {} 金币）", repair_cost));
            return;
        }

        // 扣除金币
        let _ = record.actor_ref.ask(DeductGold { amount: repair_cost }).await;

        // 执行修理
        let success = record.actor_ref.ask(crate::actors::player::RepairItem { unique_id: msg.unique_id }).await.unwrap_or(false);
        if success {
            send_system_message(&self.gate_ref, msg.session_id, &format!("修理成功（花费 {} 金币）", repair_cost));
            debug!("RepairItem: {} repaired item={} cost={}", state.name, msg.unique_id, repair_cost);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "修理失败");
        }
    }
}

/// 快捷装备栏装备
pub struct EquipSlotItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
    pub to_slot: i32,
    pub grid_to: u8,
}

impl Message<EquipSlotItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: EquipSlotItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };

        let equip_slot = match crate::actors::inventory::EquipmentSlot::from_i32(msg.to_slot) {
            Some(s) => s,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "无效装备槽");
                return;
            }
        };

        // 从 source grid 中查找物品的 backpack index
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let grid_idx = state.inventory.backpack.iter()
            .position(|s| s.as_ref().map_or(false, |slot| slot.item.unique_id == msg.unique_id));

        let Some(grid) = grid_idx else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
            return;
        };

        let result = record.actor_ref.ask(crate::actors::player::InventoryEquipItem {
            grid: grid as u8,
            slot: equip_slot,
        }).await.unwrap_or(None);

        if result.is_some() {
            debug!("EquipSlotItem: {} equipped uid={} to slot {:?}", state.name, msg.unique_id, equip_slot);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "装备失败");
        }
    }
}

/// 更换结婚戒指
pub struct ReplaceWedRingRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<ReplaceWedRingRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ReplaceWedRingRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查物品是否在背包中
        if state.inventory.get_item(msg.unique_id).is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }

        // 找到该物品在背包中的格子
        let grid = state.inventory.backpack.iter()
            .find_map(|s| s.as_ref().filter(|slot| slot.item.unique_id == msg.unique_id).map(|slot| slot.grid));

        let Some(grid) = grid else {
            send_system_message(&self.gate_ref, msg.session_id, "物品不在背包中");
            return;
        };

        // 装备到戒指槽（优先左戒指槽，如果已有则右戒指槽）
        let target_slot = if state.inventory.get_equipment(crate::actors::inventory::EquipmentSlot::RingL).is_none() {
            crate::actors::inventory::EquipmentSlot::RingL
        } else {
            crate::actors::inventory::EquipmentSlot::RingR
        };

        let result = record.actor_ref.ask(crate::actors::player::InventoryEquipItem { grid, slot: target_slot }).await.unwrap_or(None);
        if result.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "戒指已更换");
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "戒指装备失败");
        }
    }
}

/// 查看玩家信息
pub struct InspectPlayerRequest {
    pub session_id: u64,
    pub target_id: u32,
}

impl Message<InspectPlayerRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: InspectPlayerRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let mut target_state: Option<crate::actors::player::PlayerState> = None;
        for r in self.players.values() {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.object_id == msg.target_id {
                    target_state = Some(s);
                    break;
                }
            }
        }

        let Some(target) = target_state else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到目标玩家");
            return;
        };

        // 发送 PlayerInspect 包
        send_inspect_packet(&self.gate_ref, msg.session_id, &target);
    }
}

/// 观察玩家
pub struct ObservePlayerRequest {
    pub session_id: u64,
    pub target_id: u32,
}

impl Message<ObservePlayerRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ObservePlayerRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let mut target_state: Option<crate::actors::player::PlayerState> = None;
        for r in self.players.values() {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.object_id == msg.target_id {
                    target_state = Some(s);
                    break;
                }
            }
        }

        let Some(target) = target_state else {
            return;
        };

        // Send AllowObserve(true)
        let mut allow_body = Vec::new();
        allow_body.push(1u8);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AllowObserve as i16, &allow_body),
        });

        // Send PlayerInspect with target info
        send_inspect_packet(&self.gate_ref, msg.session_id, &target);
    }
}

/// 城镇复活请求
pub struct TownReviveRequest {
    pub session_id: u64,
}

impl Message<TownReviveRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TownReviveRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if !state.is_dead { return; }

        // 复活：重置 HP/MP 到最大值，回到地图出生点
        let spawn_x = DEFAULT_SPAWN_X;
        let spawn_y = DEFAULT_SPAWN_Y;

        let _ = record.actor_ref.ask(crate::actors::player::RevivePlayer {
            x: spawn_x,
            y: spawn_y,
            map_index: state.map_index,
        }).await;

        // 发送 HealthChanged 通知
        let mut health_body = Vec::new();
        health_body.extend_from_slice(&(state.max_hp as u32).to_le_bytes());
        health_body.extend_from_slice(&(state.max_mp as u32).to_le_bytes());
        let _ = self.gate_ref.ask(crate::gate::actor::SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &health_body),
        });

        debug!("TownRevive: {} revived at ({}, {})", state.name, spawn_x, spawn_y);
    }
}

// ============================================================
// 任务系统 Handler
// ============================================================

impl Message<AcceptQuestRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AcceptQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        // Validate quest exists in DB
        let quest_db = self.quest_infos.get(&msg.quest_index).cloned();
        let Some(quest_db) = quest_db else {
            send_system_message(&self.gate_ref, msg.session_id, "任务不存在");
            return;
        };

        // Check level requirement
        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            if state.level < quest_db.required_min_level as u16 {
                send_system_message(&self.gate_ref, msg.session_id, "等级不足");
                return;
            }
            if quest_db.required_max_level > 0 && state.level > quest_db.required_max_level as u16 {
                send_system_message(&self.gate_ref, msg.session_id, "等级过高");
                return;
            }
        }

        // 检查是否已接受该任务
        if let Ok(Some(_quest)) = record.actor_ref.ask(GetQuest { quest_index: msg.quest_index }).await {
            send_system_message(&self.gate_ref, msg.session_id, "该任务已接受");
            return;
        }

        // 检查是否已完成过该任务
        if let Ok(true) = record.actor_ref.ask(HasCompletedQuest { quest_index: msg.quest_index }).await {
            send_system_message(&self.gate_ref, msg.session_id, "该任务已完成");
            return;
        }

        // Create quest instance from DB data
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let quest = make_quest_instance(&quest_db, now);
        let accepted = match record.actor_ref.ask(AcceptQuest { quest }).await {
            Ok(s) => s, _ => return,
        };

        if accepted {
            send_system_message(&self.gate_ref, msg.session_id, "任务已接受");
            debug!("Quest accepted: {} ({}) by session {}", quest_db.name, msg.quest_index, msg.session_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "任务接受失败");
        }
    }
}

impl Message<FinishQuestRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: FinishQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let completed_quest = match record.actor_ref.ask(CompleteQuest { quest_index: msg.quest_index }).await {
            Ok(Some(q)) => q,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "任务不存在");
                return;
            }
        };

        // 发放奖励
        if completed_quest.exp_reward > 0 {
            let _ = record.actor_ref.ask(AddExperience {
                amount: self.apply_global_exp_multiplier(completed_quest.exp_reward as i32),
            }).await;
        }
        if completed_quest.gold_reward > 0 {
            let _ = record.actor_ref.ask(AddGold { amount: completed_quest.gold_reward }).await;
        }

        // 发放固定物品奖励
        if let Some(quest_db) = self.quest_infos.get(&msg.quest_index) {
            for reward in &quest_db.fixed_rewards {
                let mut item = mir2_shared::data::item::UserItem {
                    item_index: reward.item_index,
                    count: reward.count,
                    ..Default::default()
                };
                if let Some(info) = self.item_infos.get(&reward.item_index) {
                    item.max_dura = info.durability as u16;
                    item.current_dura = info.durability as u16;
                }
                let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
            }
            if !quest_db.fixed_rewards.is_empty() {
                let _ = record.actor_ref.ask(crate::actors::player::CheckQuestItemProgress).await;
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("任务完成！获得 {} 经验，{} 金币", completed_quest.exp_reward, completed_quest.gold_reward));
        send_quest_complete_packet(&self.gate_ref, msg.session_id, completed_quest.quest_index);
        debug!("Quest completed: {} by session {}", msg.quest_index, msg.session_id);
    }
}

impl Message<AbandonQuestRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AbandonQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let abandoned = match record.actor_ref.ask(AbandonQuest { quest_index: msg.quest_index }).await {
            Ok(s) => s, _ => return,
        };

        if abandoned {
            send_system_message(&self.gate_ref, msg.session_id, "任务已放弃");
            debug!("Quest abandoned: {} by session {}", msg.quest_index, msg.session_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "任务不存在");
        }
    }
}

// ============================================================
// 英雄系统 Handler
// ============================================================

/// 切换英雄
pub struct ChangeHeroRequest {
    pub session_id: u64,
    pub hero_index: u8,
}

impl Message<ChangeHeroRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ChangeHeroRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if msg.hero_index == 0 && state.hero_index == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "你没有可用的英雄");
            return;
        }

        let _ = record.actor_ref.ask(SetHeroIndex { hero_index: msg.hero_index });
        send_hero_update_packet(&self.gate_ref, msg.session_id, msg.hero_index);
        debug!("Hero switched: {} -> index {}", state.name, msg.hero_index);
    }
}

/// 从英雄背包取回物品
pub struct TakeBackHeroItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
}

impl Message<TakeBackHeroItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TakeBackHeroItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 从英雄背包移除物品并添加到主背包
        let _ = record.actor_ref.ask(crate::actors::player::TakeBackHeroItem {
            grid: msg.grid,
        }).await;

        debug!("Hero item taken back: {} grid={} uid={}", state.name, msg.grid, msg.unique_id);
    }
}

/// 转移物品到英雄背包
pub struct TransferHeroItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub unique_id: u64,
}

impl Message<TransferHeroItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TransferHeroItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 从主背包移除物品并添加到英雄背包
        let _ = record.actor_ref.ask(crate::actors::player::TransferHeroItem {
            grid: msg.grid,
        }).await;

        debug!("Hero item transferred: {} grid={} uid={}", state.name, msg.grid, msg.unique_id);
    }
}

// ============================================================
// 宠物系统 Handler
// ============================================================

/// 更新/设置宠物
pub struct UpdateIntelligentCreature {
    pub session_id: u64,
    pub creature_type: u8,
    pub pickup_mode: u8,
}

impl Message<UpdateIntelligentCreature> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: UpdateIntelligentCreature, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let creature_type = CreatureType::from(msg.creature_type);
        let pickup = PickupMode::from(msg.pickup_mode);

        if creature_type == CreatureType::None {
            // 关闭宠物
            let mut log = state.creature_log;
            log.active_creature = None;
            let _ = record.actor_ref.ask(SetCreature { creature_log: log });
            send_system_message(&self.gate_ref, msg.session_id, "宠物已关闭");
            return;
        }

        // 设置或更新宠物
        let mut log = state.creature_log;
        if let Some(ref mut c) = log.active_creature {
            // 更新已有宠物
            c.pickup_mode = pickup;
        } else {
            // 创建新宠物
            let mut creature = IntelligentCreature::new(creature_type);
            creature.pickup_mode = pickup;
            creature.enabled = true;
            log.active_creature = Some(creature);
        }
        let creature_ref = log.active_creature.clone();
        let _ = record.actor_ref.ask(SetCreature { creature_log: log });

        send_creature_list_packet(&self.gate_ref, msg.session_id, creature_ref.as_ref());
        debug!("UpdateIntelligentCreature: {} type={:?} mode={:?}", state.name, creature_type, pickup);
    }
}

/// 宠物拾取地面物品
pub struct IntelligentCreaturePickup {
    pub session_id: u64,
    pub x: i32,
    pub y: i32,
}

impl Message<IntelligentCreaturePickup> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: IntelligentCreaturePickup, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否有激活的宠物
        let pickup_mode = match &state.creature_log.active_creature {
            Some(c) if c.enabled && !c.is_starving() => c.pickup_mode,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "没有可用的宠物");
                return;
            }
        };

        // 根据拾取模式过滤
        if pickup_mode == PickupMode::None {
            send_system_message(&self.gate_ref, msg.session_id, "宠物拾取模式未设置");
            return;
        }

        // 查找附近的地面物品（同地图）
        let distance = 3; // 宠物拾取范围
        let item_idx = self.ground_items.iter().position(|item| {
            item.map_index == state.map_index
                && (item.x - msg.x).abs() <= distance
                && (item.y - msg.y).abs() <= distance
        });

        if let Some(idx) = item_idx {
            let item = self.ground_items.remove(idx);
            let picked_oid = item.object_id;
            // 将物品添加到玩家背包
            let mut picked_up = false;
            if let Some(rec) = self.players.get(&msg.session_id) {
                if let Ok(true) = rec.actor_ref.ask(AddItemToInventory {
                    item: item.item.clone(),
                }).await {
                    picked_up = true;
                }
            }
            if picked_up {
                // 广播 ObjectRemove
                let mut remove_body = Vec::new();
                remove_body.extend_from_slice(&picked_oid.to_le_bytes());
                let remove_packet = build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &remove_body);
                for (sid, rec) in &self.players {
                    if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                        if s.map_index == state.map_index {
                            let _ = self.gate_ref.ask(SendToClient {
                                session_id: *sid,
                                data: remove_packet.clone(),
                            });
                        }
                    }
                }
                debug!("Creature pickup: {} picked up item at ({},{})",
                       state.name, msg.x, msg.y);
            } else {
                // 添加失败，放回去
                self.ground_items.push(item);
            }
        }
    }
}

/// 请求宠物更新列表
pub struct RequestIntelligentCreatureUpdates {
    pub session_id: u64,
    pub request_updates: bool,
}

impl Message<RequestIntelligentCreatureUpdates> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestIntelligentCreatureUpdates, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 发送当前宠物列表
        let creature_ref = state.creature_log.active_creature.clone();
        send_creature_list_packet(&self.gate_ref, msg.session_id, creature_ref.as_ref());
    }
}

// ============================================================
// 仓库/金币 Handler
// ============================================================

/// 存入仓库
pub struct StoreItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub uid: u64,
    pub count: u32,
}

impl Message<StoreItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: StoreItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查仓库是否有空位
        if !state.inventory.storage_has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "仓库已满");
            return;
        }

        // 检查物品是否在背包中
        if state.inventory.get_item(msg.uid).is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }

        // 执行存入
        let result = record.actor_ref.ask(StoreItem { grid: msg.grid }).await;
        match result {
            Ok(true) => {
                send_store_item_packet(&self.gate_ref, msg.session_id, msg.grid, true);
                debug!("StoreItem: {} grid={} uid={}", state.name, msg.grid, msg.uid);
            }
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "存入仓库失败");
            }
        }
    }
}

/// 从仓库取出
pub struct TakeBackItemRequest {
    pub session_id: u64,
    pub grid: u8,
    pub uid: u64,
    pub count: u32,
}

impl Message<TakeBackItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TakeBackItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查背包是否有空位
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            return;
        }

        // 执行取出
        let result = record.actor_ref.ask(TakeBackItem { grid: msg.grid }).await;
        match result {
            Ok(true) => {
                send_take_back_item_packet(&self.gate_ref, msg.session_id, msg.grid, true);
                debug!("TakeBackItem: {} grid={} uid={}", state.name, msg.grid, msg.uid);
            }
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "取出物品失败");
            }
        }
    }
}

// ============================================================
// 精炼系统 Handler
// ============================================================

/// 存入精炼物品
pub struct DepositRefineItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<DepositRefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DepositRefineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查物品是否在背包中
        if state.inventory.get_item(msg.unique_id).is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }

        // 更新精炼日志
        let mut log = state.refine_log;
        if !log.deposit_item(msg.unique_id) {
            send_system_message(&self.gate_ref, msg.session_id, "已有精炼进行中");
            return;
        }

        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log });
        send_system_message(&self.gate_ref, msg.session_id, "精炼物品已存入");
        debug!("DepositRefineItem: {} uid={}", state.name, msg.unique_id);
    }
}

/// 取回精炼物品
pub struct RetrieveRefineItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<RetrieveRefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RetrieveRefineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查精炼是否完成或可取回
        if state.refine_log.active_refine.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "没有精炼物品可取回");
            return;
        }

        // 检查背包是否有空位
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            return;
        }

        let mut log = state.refine_log;
        if let Some(_item) = log.retrieve() {
            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log });
            send_system_message(&self.gate_ref, msg.session_id, "精炼物品已取回");
            debug!("RetrieveRefineItem: {} uid={}", state.name, msg.unique_id);
        }
    }
}

/// 取消精炼
pub struct RefineCancelRequest {
    pub session_id: u64,
}

impl Message<RefineCancelRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RefineCancelRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if state.refine_log.active_refine.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "没有精炼可取消");
            return;
        }

        let mut log = state.refine_log;
        log.cancel();
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log });
        send_system_message(&self.gate_ref, msg.session_id, "精炼已取消");
        debug!("RefineCancel: {}", state.name);
    }
}

/// 开始精炼
pub struct RefineItemRequest {
    pub session_id: u64,
    pub item_id: u32,
    pub materials: u32,
}

impl Message<RefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RefineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否有待精炼物品
        if state.refine_log.active_refine.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "没有待精炼的物品");
            return;
        }

        // 开始精炼（60 秒完成，80% 成功率）
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut log = state.refine_log;
        let duration = 60u64; // 1 分钟
        let success_chance = 80u8; // 80%
        log.start_refine(msg.item_id, current_time, duration, success_chance);
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log });

        send_system_message(&self.gate_ref, msg.session_id, "精炼已开始，请稍后查看");
        debug!("RefineItem: {} item={} materials={}", state.name, msg.item_id, msg.materials);
    }
}

/// 检查精炼状态
pub struct CheckRefineRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<CheckRefineRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CheckRefineRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(ref item) = state.refine_log.active_refine {
            if item.status == RefineStatus::Pending && current_time >= item.finish_time {
                // 精炼完成，自动标记为完成
                let mut log = state.refine_log;
                let success = log.finish();
                let _ = record.actor_ref.ask(SetRefineLog { refine_log: log });
                if success {
                    send_system_message(&self.gate_ref, msg.session_id, "精炼成功！物品已提升");
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "精炼失败，物品已损毁");
                }
                debug!("CheckRefine: {} result={}", state.name, success);
            } else if item.status == RefineStatus::Ready {
                send_system_message(&self.gate_ref, msg.session_id, "精炼已完成，请取回物品");
            } else {
                let remaining = item.finish_time.saturating_sub(current_time);
                send_system_message(&self.gate_ref, msg.session_id, &format!("精炼进行中，剩余 {} 秒", remaining));
            }
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "没有精炼进行中");
        }
    }
}

// ============================================================
// 辅助函数
// ============================================================

// ---------- RangeAttack / Magic ----------

/// 远程攻击请求（同普通攻击，但带目标位置）
pub struct RangeAttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
}

impl Message<RangeAttackRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RangeAttackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let object_id = state.object_id;
        let target_x = msg.target_x;
        let target_y = msg.target_y;

        // 广播 ObjectAttack 给其他玩家
        let others: Vec<_> = self.other_players(msg.session_id)
            .into_iter().cloned()
            .collect();
        for other in &others {
            let mut body = Vec::new();
            body.extend_from_slice(&object_id.to_le_bytes());
            body.push(msg.direction);
            body.push(0u8); // spell = 0 (range attack)
            let _ = self.gate_ref.ask(SendToClient {
                session_id: other.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &body),
            });
        }

        // 检测范围内的怪物
        let hit_monster_ids: Vec<u32> = self.monsters.iter()
            .filter(|(_, m)| {
                let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                dist <= 1
            })
            .map(|(id, _)| *id)
            .collect();

        let mut hit_monster = false;
        for monster_id in hit_monster_ids {
            if let Some(monster) = self.monsters.get_mut(&monster_id) {
                let attack_result = combat_attack::resolve_attack(
                    state.effective_min_attack(), state.effective_max_attack(), 0
                );
                let damage = attack_result.damage;
                monster.hp = monster.hp.saturating_sub(damage);
                monster.provoked = true;
                monster.target_session = Some(msg.session_id);
                debug!("RangeAttack: {} -> monster {} for {} damage", state.name, monster_id, damage);
                hit_monster = true;
                if monster.hp <= 0 {
                    // 死亡由 Tick 循环处理（广播 ObjectDied + 重生）
                }
            }
        }

        // 未命中怪物时尝试命中玩家（PvP）
        if !hit_monster {
            for other in &others {
                if let Ok(Some(other_state)) = other.actor_ref.ask(GetPlayerState).await {
                    let dist = (other_state.x - target_x).abs() + (other_state.y - target_y).abs();
                    if dist <= 1 {
                        // 攻击模式检查
                        if !can_attack_player(&state, &other_state) {
                            continue;
                        }
                        // 安全区保护
                        let attacker_safe = self.maps.get(&state.map_index)
                            .map(|m| m.is_safe_zone(state.x, state.y))
                            .unwrap_or(false);
                        let target_safe = self.maps.get(&other_state.map_index)
                            .map(|m| m.is_safe_zone(other_state.x, other_state.y))
                            .unwrap_or(false);
                        if attacker_safe || target_safe {
                            continue;
                        }

                        let attack_result = combat_attack::resolve_attack(
                            state.effective_min_attack(), state.effective_max_attack(), other_state.effective_defence()
                        );
                        let damage = attack_result.damage;
                        if other.actor_ref.ask(TakeDamage {
                            attacker_id: object_id,
                            attacker_session: msg.session_id,
                            damage,
                        }).await.unwrap_or(false) {
                            // 目标死亡处理
                            let mut died_body = Vec::new();
                            died_body.extend_from_slice(&other_state.object_id.to_le_bytes());
                            died_body.extend_from_slice(&(other_state.x as u32).to_le_bytes());
                            died_body.extend_from_slice(&(other_state.y as u32).to_le_bytes());
                            died_body.push(other_state.direction);
                            died_body.push(0u8);
                            let died_packet = build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::ObjectDied as i16, &died_body);
                            for (sid, _) in &self.players {
                                let _ = self.gate_ref.ask(SendToClient {
                                    session_id: *sid,
                                    data: died_packet.clone(),
                                });
                            }
                            self.handle_player_death_drop(other.session_id, other_state.x, other_state.y, other_state.map_index).await;

                            // 增加 PK 值
                            let _ = record.actor_ref.ask(crate::actors::player::AddPkPoints { points: 100 }).await;
                            if let Ok(Some(attacker_state)) = record.actor_ref.ask(GetPlayerState).await {
                                let colour_packet = build_object_colour_changed_packet(
                                    attacker_state.object_id,
                                    name_colour_for_pk(attacker_state.pk_points),
                                );
                                for (sid, _) in &self.players {
                                    let _ = self.gate_ref.ask(SendToClient {
                                        session_id: *sid,
                                        data: colour_packet.clone(),
                                    });
                                }
                                if let Some(r) = self.players.get_mut(&msg.session_id) {
                                    r.last_pk_points = attacker_state.pk_points;
                                }
                            }
                        }
                        debug!("RangeAttack PvP: {} damaged {} for {}", state.name, other_state.name, damage);
                        break; // 远程攻击只命中一个目标
                    }
                }
            }
        }
    }
}

/// 技能释放请求
pub struct MagicRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
    pub target_id: u32,
    pub target_x: i32,
    pub target_y: i32,
}

impl Message<MagicRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: MagicRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => { return; }
        };
        if state.is_dead { return; }

        // 施法时自动下坐骑
        self.dismount_player(msg.session_id).await;

        // 施法时打破隐身
        if self.invisible_sessions.remove(&msg.session_id) {
            let _ = record.actor_ref.ask(crate::actors::player::RemoveBuff {
                buff_type: crate::combat::buff::BuffType::Invisibility,
            }).await;
            self.reveal_player_to_others(msg.session_id, &state).await;
        }

        // Validate spell exists in DB
        let spell_db = self.magic_infos.get(&(msg.spell as u32));

        // 检查玩家是否已学习该技能（基础攻击魔法不需要学习）
        let basic_spells = [0, 1]; // None, 基础攻击
        if !basic_spells.contains(&msg.spell) && !state.magics.iter().any(|m| m.spell == msg.spell as i32) {
            send_system_message(&self.gate_ref, msg.session_id, "你尚未学会这个技能");
            return;
        }
        let spell_range = spell_db.map(|m| m.range as i32).unwrap_or(2);
        let power = spell_db.map(|m| m.power_base).unwrap_or(10);
        let mp_cost = spell_db.map(|m| m.base_cost).unwrap_or(5);

        // 检查并扣除 MP
        if state.mp < mp_cost {
            send_system_message(&self.gate_ref, msg.session_id, "魔法值不足");
            return;
        }
        let mp_ok = record.actor_ref.ask(DeductMP { amount: mp_cost }).await.unwrap_or(false);
        if !mp_ok {
            send_system_message(&self.gate_ref, msg.session_id, "魔法值不足");
            return;
        }

        let object_id = state.object_id;
        let target_x = msg.target_x;
        let target_y = msg.target_y;

        // 发送 MagicCast 给施法者（确认施法）
        let spell_enum = mir2_shared::enums::Spell::try_from(msg.spell)
            .unwrap_or(mir2_shared::enums::Spell::None);
        let magic_cast = mir2_shared::packets::server::magic_combat::MagicCast { spell: spell_enum };
        let mut cast_body = Vec::new();
        if magic_cast.write_body(&mut cast_body).is_ok() {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MagicCast as i16, &cast_body),
            });
        }

        // 广播 ObjectMagic 给其他玩家
        let others: Vec<_> = self.other_players(msg.session_id)
            .into_iter().cloned()
            .collect();
        let object_magic = mir2_shared::packets::server::magic_combat::ObjectMagic {
            object_id,
            location_x: state.x,
            location_y: state.y,
            direction: mir2_shared::enums::MirDirection::try_from(msg.direction)
                .unwrap_or(mir2_shared::enums::MirDirection::Up),
            spell: spell_enum,
            target_id: msg.target_id,
            target_x,
            target_y,
            cast: true,
            level: 0,
            self_broadcast: false,
            secondary_target_ids: Vec::new(),
        };
        let mut om_body = Vec::new();
        if object_magic.write_body(&mut om_body).is_ok() {
            for other in &others {
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: other.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectMagic as i16, &om_body),
                });
            }
        }

        // 根据魔法类型执行不同效果
        match msg.spell {
            // --- 治愈类 ---
            SPELL_HEALING | SPELL_MASS_HEALING | SPELL_HEALING_CIRCLE => {
                let heal_amount = power.max(10);
                let _ = record.actor_ref.ask(crate::actors::player::Heal {
                    amount: heal_amount,
                }).await;
                debug!("Magic: {} casts Healing(spell={}) for {} HP", state.name, msg.spell, heal_amount);
            }
            // --- Buff 类 ---
            SPELL_MAGIC_SHIELD => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::DefenseBoost { bonus: (power / 2).max(5) },
                    300, // 30秒 @ 100ms tick
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts MagicShield (defense +{})", state.name, (power / 2).max(5));
            }
            SPELL_SOUL_SHIELD => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::DefenseBoost { bonus: (power / 3).max(3) },
                    600, // 60秒
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts SoulShield (defense +{})", state.name, (power / 3).max(3));
            }
            SPELL_BLESSED_ARMOUR => {
                let buff = crate::combat::buff::BuffInstance::new(
                    crate::combat::buff::BuffType::AttackBoost { bonus: (power / 2).max(5) },
                    600,
                    5,
                );
                let _ = record.actor_ref.ask(crate::actors::player::ApplyBuff { buff }).await;
                debug!("Magic: {} casts BlessedArmour (attack +{})", state.name, (power / 2).max(5));
            }
            // --- 传送类 ---
            SPELL_TELEPORT => {
                if let Some(mi) = self.map_infos.get(&(state.map_index as i32)) {
                    if mi.no_teleport {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送魔法");
                        return;
                    }
                    if mi.no_escape {
                        send_system_message(&self.gate_ref, msg.session_id, "该地图无法使用传送魔法");
                        return;
                    }
                }
                // 限制在地图边界内
                let (max_x, max_y) = self.maps.get(&state.map_index)
                    .map(|m| (m.width as i32, m.height as i32))
                    .unwrap_or((i32::MAX, i32::MAX));
                let tx = target_x.clamp(0, max_x - 1);
                let ty = target_y.clamp(0, max_y - 1);
                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                    x: tx,
                    y: ty,
                    direction: msg.direction,
                    map_index: None,
                    is_mounted: None,
                }).await;
                debug!("Magic: {} teleports to ({}, {})", state.name, tx, ty);
            }
            // --- 默认：伤害类 ---
            _ => {
                // 技能命中范围内的怪物
                let hit_monster_ids: Vec<u32> = self.monsters.iter()
                    .filter(|(_, m)| {
                        let dist = (m.x - target_x).abs() + (m.y - target_y).abs();
                        dist <= spell_range
                    })
                    .map(|(id, _)| *id)
                    .collect();

                // 魔法伤害 = spell power + 玩家魔法加成
                let power_min = spell_db.map(|m| m.power_base).unwrap_or(10).max(1);
                let power_max = spell_db.map(|m| (m.power_base + m.power_bonus).max(power_min)).unwrap_or(power_min + 5);
                let magic_bonus = state.min_attack / 4; // 简化：攻击力的一部分转化为魔法伤害
                for monster_id in hit_monster_ids {
                    if let Some(monster) = self.monsters.get_mut(&monster_id) {
                        let base_damage = fastrand::i32(power_min..=power_max);
                        let damage = (base_damage + magic_bonus).max(1);
                        monster.hp = monster.hp.saturating_sub(damage);
                        monster.provoked = true;
                        monster.target_session = Some(msg.session_id);
                        debug!("Magic: {} spell={} -> monster {} for {} damage (base={} bonus={})", state.name, msg.spell, monster_id, damage, base_damage, magic_bonus);
                    }
                }
            }
        }
    }
}

// ---------- 传送/地图 ----------

/// 传送到 NPC 请求
pub struct TeleportToNPCRequest {
    pub session_id: u64,
    pub npc_id: u32,
}

impl Message<TeleportToNPCRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: TeleportToNPCRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // 按 object_id 查找 NPC
        let npc = self.npcs.get(&msg.npc_id).cloned();
        let Some(npc) = npc else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该 NPC");
            return;
        };

        // 传送到 NPC 附近
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead { return; }

        let new_x = npc.x;
        let new_y = npc.y;

        // 更新玩家位置
        let _ = record.actor_ref.ask(SetPlayerPosition { x: new_x, y: new_y, direction: npc.direction, map_index: None, is_mounted: None });
        let mut body = Vec::new();
        body.extend_from_slice(&new_x.to_le_bytes());
        body.extend_from_slice(&new_y.to_le_bytes());
        body.push(npc.direction);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
        });

        debug!("TeleportToNPC: {} -> {} ({}, {})", state.name, npc.name, new_x, new_y);
    }
}

/// 请求地图信息（传送）
pub struct RequestMapInfoRequest {
    pub session_id: u64,
    pub map_id: u32,
}

impl Message<RequestMapInfoRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestMapInfoRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // Look up target map from DB
        let Some(dest_mi) = self.map_infos.get(&(msg.map_id as i32)) else {
            send_system_message(&self.gate_ref, msg.session_id, "地图不存在");
            return;
        };

        // Check no_recall flag
        if dest_mi.no_recall {
            send_system_message(&self.gate_ref, msg.session_id, "无法传送到该地图");
            return;
        }

        let dest_file = dest_mi.file_name.clone();
        let dest_title = dest_mi.title.clone();

        // Place at safe zone spawn point if available
        let (spawn_x, spawn_y) = dest_mi.safe_zones.iter()
            .find(|s| s.start_point)
            .map(|s| (s.x, s.y))
            .unwrap_or((DEFAULT_SPAWN_X, DEFAULT_SPAWN_Y));

        // Load dest map
        if self.get_or_load_map(&dest_file).is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "地图加载失败");
            return;
        }

        // Inject new map data into player for collision/pathfinding
        if let Some(map_data) = self.maps.get(&0).cloned() {
            let _ = record.actor_ref.ask(SetMapData { map: map_data });
        }

        // Update player position
        let _ = record.actor_ref.ask(SetPlayerPosition { x: spawn_x, y: spawn_y, direction: state.direction, map_index: Some(msg.map_id as u16), is_mounted: None });

        // Send MapChanged first, then UserLocation (client processes in order)
        let map_changed_body = build_map_changed_packet(msg.map_id as u16, &dest_file, &dest_title, spawn_x, spawn_y, false);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: map_changed_body,
        });

        let mut body = Vec::new();
        body.extend_from_slice(&spawn_x.to_le_bytes());
        body.extend_from_slice(&spawn_y.to_le_bytes());
        body.push(state.direction);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
        });

        debug!("RequestMapInfo: {} -> map {} ({}) ({}, {})", state.name, msg.map_id, dest_file, spawn_x, spawn_y);
    }
}

/// 搜索地图/NPC
pub struct SearchMapRequest {
    pub session_id: u64,
    pub keyword: String,
}

impl Message<SearchMapRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SearchMapRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let keyword_lower = msg.keyword.to_lowercase();

        // 搜索匹配的地图
        let matched_maps: Vec<_> = self.map_infos.values()
            .filter(|m| m.title.to_lowercase().contains(&keyword_lower) || m.file_name.to_lowercase().contains(&keyword_lower))
            .collect();

        // 搜索匹配的 NPC
        let matched_npcs: Vec<_> = self.npcs.values()
            .filter(|n| n.name.to_lowercase().contains(&keyword_lower))
            .collect();

        if matched_maps.is_empty() && matched_npcs.is_empty() {
            send_system_message(&self.gate_ref, msg.session_id, "未找到匹配结果");
            return;
        }

        let mut result = String::new();
        if !matched_maps.is_empty() {
            result.push_str(&format!("地图({}): ", matched_maps.len()));
            for (i, m) in matched_maps.iter().take(5).enumerate() {
                if i > 0 { result.push_str(", "); }
                result.push_str(&format!("{}(#{}))", m.title, m.index));
            }
        }
        if !matched_npcs.is_empty() {
            if !result.is_empty() { result.push_str(" | "); }
            result.push_str(&format!("NPC({}): ", matched_npcs.len()));
            for (i, n) in matched_npcs.iter().take(5).enumerate() {
                if i > 0 { result.push_str(", "); }
                result.push_str(&format!("{}({},{})", n.name, n.x, n.y));
            }
        }
        send_system_message(&self.gate_ref, msg.session_id, &result);
        debug!("SearchMap: {} maps, {} NPCs matching '{}'", matched_maps.len(), matched_npcs.len(), msg.keyword);
    }
}

// ---------- 合成配方 ----------

/// 合成材料
#[derive(Debug, Clone)]
pub struct CraftIngredient {
    pub item_index: i32,
    pub count: u16,
}

/// 合成配方
#[derive(Debug, Clone)]
pub struct CraftRecipe {
    pub recipe_id: u32,
    pub product_index: i32,
    pub product_count: u16,
    pub success_rate: u8,
    pub ingredients: Vec<CraftIngredient>,
}

/// 硬编码合成配方表（后续可从 DB 加载）
fn get_craft_recipes() -> Vec<CraftRecipe> {
    vec![
        // recipe_id 1: 铁剑 = 木材 x3 + 铁矿石 x2, 80%
        CraftRecipe {
            recipe_id: 1,
            product_index: 100,
            product_count: 1,
            success_rate: 80,
            ingredients: vec![
                CraftIngredient { item_index: 1, count: 3 },
                CraftIngredient { item_index: 2, count: 2 },
            ],
        },
        // recipe_id 2: 治疗药水 = 草药 x2 + 清水 x1, 95%
        CraftRecipe {
            recipe_id: 2,
            product_index: 101,
            product_count: 1,
            success_rate: 95,
            ingredients: vec![
                CraftIngredient { item_index: 3, count: 2 },
                CraftIngredient { item_index: 4, count: 1 },
            ],
        },
        // recipe_id 3: 强化石 = 铁矿石 x5, 60%
        CraftRecipe {
            recipe_id: 3,
            product_index: 102,
            product_count: 1,
            success_rate: 60,
            ingredients: vec![
                CraftIngredient { item_index: 2, count: 5 },
            ],
        },
    ]
}

// ---------- 物品合成/回购 ----------

/// 合成物品请求
pub struct CraftItemRequest {
    pub session_id: u64,
    pub recipe_id: u32,
}

impl Message<CraftItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CraftItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 查找配方
        let recipes = get_craft_recipes();
        let recipe = match recipes.iter().find(|r| r.recipe_id == msg.recipe_id) {
            Some(r) => r.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "未知配方");
                let mut body = Vec::new();
                body.extend_from_slice(&msg.recipe_id.to_le_bytes());
                body.extend_from_slice(&0u16.to_le_bytes());
                body.push(0u8); // success = false
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
                });
                return;
            }
        };

        // 检查背包空间
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            let mut body = Vec::new();
            body.extend_from_slice(&msg.recipe_id.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.push(0u8);
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
            });
            return;
        }

        // 检查材料
        for ing in &recipe.ingredients {
            let has = record.actor_ref.ask(crate::actors::player::HasItem {
                item_index: ing.item_index,
                count: ing.count,
            }).await.unwrap_or(false);
            if !has {
                send_system_message(&self.gate_ref, msg.session_id, "材料不足");
                let mut body = Vec::new();
                body.extend_from_slice(&msg.recipe_id.to_le_bytes());
                body.extend_from_slice(&0u16.to_le_bytes());
                body.push(0u8);
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
                });
                return;
            }
        }

        // 扣除材料
        for ing in &recipe.ingredients {
            let _ = record.actor_ref.ask(crate::actors::player::RemoveItemByIndex {
                item_index: ing.item_index,
                count: ing.count,
            }).await;
        }

        // 成功率判定
        let success = fastrand::u8(0..100) < recipe.success_rate;

        if success {
            let mut item = mir2_shared::data::item::UserItem {
                item_index: recipe.product_index,
                count: recipe.product_count,
                ..Default::default()
            };
            if let Some(info) = self.item_infos.get(&recipe.product_index) {
                item.max_dura = info.durability as u16;
                item.current_dura = info.durability as u16;
            }
            let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
            send_system_message(&self.gate_ref, msg.session_id, "合成成功！");
            debug!("CraftItem: {} recipe={} success", state.name, msg.recipe_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "合成失败，材料已消耗");
            debug!("CraftItem: {} recipe={} failed", state.name, msg.recipe_id);
        }

        // 发送 CraftItem 响应
        let mut body = Vec::new();
        body.extend_from_slice(&msg.recipe_id.to_le_bytes());
        body.extend_from_slice(&(if success { recipe.product_count } else { 0 }).to_le_bytes());
        body.push(if success { 1u8 } else { 0u8 });
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
        });
    }
}

/// 回购物品请求（从 NPC 回购最近卖出的物品）
pub struct BuyItemBackRequest {
    pub session_id: u64,
    pub item_index: u32,
}

impl Message<BuyItemBackRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: BuyItemBackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 查找回购列表中的对应物品
        let list = match self.buyback_items.get_mut(&msg.session_id) {
            Some(l) => l,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有可回购的物品");
                return;
            }
        };
        let idx = match list.iter().position(|b| b.item.item_index == msg.item_index as i32) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该物品已无法回购");
                return;
            }
        };

        let buyback = list.remove(idx);
        let cost = buyback.sell_price * 2;

        // 检查背包空间
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            list.insert(idx, buyback);
            return;
        }

        // 扣除金币
        let deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: cost }).await.unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            list.insert(idx, buyback);
            return;
        }

        // 添加物品到背包
        let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory {
            item: buyback.item.clone(),
        }).await;

        send_system_message(&self.gate_ref, msg.session_id, &format!("回购成功，花费 {} 金币", cost));
        debug!("BuyItemBack: {} item_index={} cost={}", state.name, msg.item_index, cost);
    }
}

// ---------- 角色管理 ----------

/// 创建角色请求
pub struct NewCharacterRequest {
    pub session_id: u64,
    pub name: String,
    pub class: u8,
    pub gender: u8,
    pub hair: u16,
    pub account_username: String,
}

impl Message<NewCharacterRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: NewCharacterRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // 验证角色名称
        if msg.name.is_empty() || msg.name.len() > 20 {
            send_system_message(&self.gate_ref, msg.session_id, "角色名称无效");
            return;
        }
        // 检查名称是否已被使用（在线玩家）
        for r in self.players.values() {
            if r.name.eq_ignore_ascii_case(&msg.name) {
                send_system_message(&self.gate_ref, msg.session_id, "角色名称已被使用");
                return;
            }
        }
        // 检查数据库中是否已有该角色
        match db::load_character(&self.db_pool, &msg.name).await {
            Ok(Some(_)) => {
                send_system_message(&self.gate_ref, msg.session_id, "角色名称已被使用");
                return;
            }
            Err(e) => {
                warn!("Failed to check character name '{}': {}", msg.name, e);
            }
            Ok(None) => {}
        }

        // 创建默认角色状态并保存到数据库
        let class = mir2_shared::enums::MirClass::try_from(msg.class)
            .unwrap_or(mir2_shared::enums::MirClass::Warrior);
        let gender = mir2_shared::enums::MirGender::try_from(msg.gender)
            .unwrap_or(mir2_shared::enums::MirGender::Male);
        let default_state = PlayerState {
            object_id: 0,
            name: msg.name.clone(),
            map_index: 0,
            x: 330,
            y: 330,
            direction: 4,
            attack_mode: mir2_shared::enums::AttackMode::Peace,
            pet_mode: mir2_shared::enums::PetMode::Both,
            hidden: false,
            session_id: 0,
            class,
            gender,
            hair: msg.hair as u8,
            level: 1,
            experience: 0,
            max_experience: 100,
            hp: 120,
            max_hp: 120,
            mp: 60,
            max_mp: 60,
            min_attack: 5,
            max_attack: 10,
            defence: 2,
            bonus_min_attack: 0,
            bonus_max_attack: 0,
            bonus_defence: 0,
            bonus_max_hp: 0,
            bonus_max_mp: 0,
            inventory: PlayerInventory::new(),
            group_id: None,
            friend_list: FriendList::new(),
            mailbox: Mailbox::new(),
            guild_name: None,
            guild_rank: GuildRank::Member,
            quest_log: QuestLog::new(),
            spouse_name: None,
            allow_mentor: false,
            mentor_name: None,
            creature_log: CreatureLog::new(),
            hero_index: 0,
                hero_behaviour: 0,
                auto_pot_hp: 0,
                auto_pot_mp: 0,
                auto_pot_hp_item: 0,
                auto_pot_mp_item: 0,
            hero_inventory: PlayerInventory::new(),
            refine_log: RefineLog::new(),
            is_fishing: false,
            fishing_autocast: false,
            reincarnation_host: None,
            reincarnation_ready: false,
            reincarnation_expire_time: 0,
            enable_group_recall: false,
            last_recall_time: 0,
            is_dead: false,
            is_mounted: false,
            mount_type: 0,
            allow_lover_recall: false,
            is_gm: false,
            pk_points: 0,
            pk_kill_count: 0,
            buffs: Vec::new(),
            magics: Vec::new(),
            flags: std::collections::HashMap::new(),
            exp_multiplier: 1.0,
            exp_multiplier_end_tick: 0,
        };
        if let Err(e) = db::save_character(&self.db_pool, &default_state, &msg.account_username).await {
            warn!("Failed to save new character '{}': {}", msg.name, e);
        }

        let mut body = Vec::new();
        write_dotnet_string(&mut body, &msg.name);
        body.push(msg.class);
        body.push(msg.gender);
        body.extend_from_slice(&msg.hair.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewCharacter as i16, &body),
        });

        debug!("NewCharacter: session={} name={} class={} gender={}", msg.session_id, msg.name, msg.class, msg.gender);
    }
}

/// 删除角色请求
pub struct DeleteCharacterRequest {
    pub session_id: u64,
    pub character_index: i32,
    pub account_username: String,
}

impl Message<DeleteCharacterRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DeleteCharacterRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // 从数据库删除角色及相关数据
        // 注意：当前仅返回成功，后续可加入"需要密码确认"等逻辑
        let success = true;

        let mut body = Vec::new();
        body.extend_from_slice(&msg.character_index.to_le_bytes());
        body.push(if success { 1u8 } else { 0u8 });
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DeleteCharacter as i16, &body),
        });

        debug!("DeleteCharacter: session={} index={}", msg.session_id, msg.character_index);
    }
}

/// 创建英雄请求
pub struct NewHeroRequest {
    pub session_id: u64,
    pub hero_type: u8,
}

impl Message<NewHeroRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: NewHeroRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 设置英雄索引
        let hero_index = msg.hero_type;
        let _ = record.actor_ref.ask(SetHeroIndex { hero_index });

        let body = vec![hero_index, 1u8];
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewHero as i16, &body),
        });

        debug!("NewHero: {} type={}", state.name, msg.hero_type);
    }
}

// ============================================================
// 钓鱼系统
// ============================================================

pub struct FishingCastRequest {
    pub session_id: u64,
    pub fishing_type: u8,
}

impl Message<FishingCastRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: FishingCastRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };
        if state.is_dead { return; }

        let _ = record.actor_ref.ask(SetFishing { is_fishing: true, autocast: false });

        // Send FishingUpdate: progress=1 (waiting), success=false
        use mir2_shared::packets::server::miscellaneous::FishingUpdate;
        let packet = FishingUpdate { fishing_progress: 1, fishing_success: false };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
            });
        }

        debug!("FishingCast: {} type={}", state.name, msg.fishing_type);
    }
}

pub struct FishingChangeAutocastRequest {
    pub session_id: u64,
    pub enabled: bool,
}

impl Message<FishingChangeAutocastRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: FishingChangeAutocastRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let _ = record.actor_ref.ask(SetFishing { is_fishing: state.is_fishing, autocast: msg.enabled });

        // Send FishingUpdate: progress=5 (autocast toggle), success=enabled
        use mir2_shared::packets::server::miscellaneous::FishingUpdate;
        let packet = FishingUpdate { fishing_progress: 5, fishing_success: msg.enabled };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::FishingUpdate as i16, &body),
            });
        }

        debug!("FishingChangeAutocast: {} enabled={}", state.name, msg.enabled);
    }
}

// ============================================================
// 邮件锁定
// ============================================================

pub struct LockMailRequest {
    pub session_id: u64,
    pub mail_id: u64,
    pub lock: bool,
}

impl Message<LockMailRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: LockMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let mut state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if let Some(mail) = state.mailbox.get_mail_mut(msg.mail_id) {
            mail.locked = msg.lock;
            let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
            debug!("LockMail: {} mail_id={} lock={}", state.name, msg.mail_id, msg.lock);
        }
    }
}

pub struct MailLockedItemRequest {
    pub session_id: u64,
    pub mail_id: u64,
    pub item_index: u32,
}

impl Message<MailLockedItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MailLockedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let mut state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if let Some(mail) = state.mailbox.get_mail_mut(msg.mail_id) {
            mail.locked = true;
            let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
            debug!("MailLockedItem: {} mail_id={} item_index={}", state.name, msg.mail_id, msg.item_index);
        }
    }
}

// ============================================================
// 任务分享
// ============================================================

pub struct ShareQuestRequest {
    pub session_id: u64,
    pub quest_id: u32,
}

impl Message<ShareQuestRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ShareQuestRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // Must be in a group to share
        let group_id = match state.group_id {
            Some(gid) => gid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你需要加入队伍才能分享任务");
                return;
            }
        };

        // Verify the player has the quest
        let has_quest = match record.actor_ref.ask(GetQuest { quest_index: msg.quest_id as i32 }).await {
            Ok(Some(_)) => true,
            _ => false,
        };
        if !has_quest {
            send_system_message(&self.gate_ref, msg.session_id, "你没有这个任务");
            return;
        }

        // Send ShareQuest packet to all group members (except self)
        use mir2_shared::packets::server::miscellaneous::ShareQuest as ShareQuestPacket;
        let packet = ShareQuestPacket { quest_id: msg.quest_id as i32 };
        let mut body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&packet, &mut body) {
            let data = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ShareQuest as i16, &body);
            for (sid, rec) in &self.players {
                if *sid == msg.session_id { continue; }
                if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                    if s.group_id == Some(group_id) {
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: *sid,
                            data: data.clone(),
                        });
                    }
                }
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("已分享任务 #{}", msg.quest_id));
        debug!("ShareQuest: {} quest_id={}", state.name, msg.quest_id);
    }
}

// ============================================================
// 物品操作（合成/分解/重置）
// ============================================================

pub struct CombineItemRequest {
    pub session_id: u64,
    pub from_grid: u32,
    pub to_grid: u32,
}

impl Message<CombineItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: CombineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let from_grid = msg.from_grid as u8;
        let to_grid = msg.to_grid as u8;

        // 获取源物品和目标物品
        let source = match record.actor_ref.ask(crate::actors::player::GetItemInfoByGrid { grid: from_grid }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到源物品");
                self.send_combine_item_response(msg.session_id, 0, 0, false, false);
                return;
            }
        };
        let target = match record.actor_ref.ask(crate::actors::player::GetItemInfoByGrid { grid: to_grid }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到目标物品");
                self.send_combine_item_response(msg.session_id, 0, 0, false, false);
                return;
            }
        };

        // 获取物品信息
        let source_info = match self.item_infos.get(&source.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "无法识别源物品");
                self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
                return;
            }
        };
        let target_info = match self.item_infos.get(&target.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "无法识别目标物品");
                self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
                return;
            }
        };

        // 源物品必须是宝石 (ItemType::Gem = 18)
        if source_info.item_type != 18 {
            send_system_message(&self.gate_ref, msg.session_id, "源物品不是宝石");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
            return;
        }

        // 目标物品必须是可镶嵌的装备
        let can_socket = matches!(target_info.item_type,
            1 | 2 | 4 | 5 | 6 | 7 | 9 | 10 | 19
        );
        if !can_socket {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法镶嵌宝石");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
            return;
        }

        // 检查目标物品是否有空槽位
        let slot_count = target_info.slots as usize;
        let filled_slots = target.slots.iter().filter(|s| s.is_some()).count();
        if slot_count == 0 || filled_slots >= slot_count {
            send_system_message(&self.gate_ref, msg.session_id, "目标物品没有空槽位");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
            return;
        }

        // 执行镶嵌
        let result = record.actor_ref.ask(crate::actors::player::SocketGem {
            from_grid,
            to_grid,
            target_slot_count: slot_count,
        }).await.ok().flatten();

        if let Some((source_uid, target_uid)) = result {
            send_system_message(&self.gate_ref, msg.session_id, "宝石镶嵌成功！");
            self.send_combine_item_response(msg.session_id, source_uid, target_uid, true, true);
            debug!("CombineItem: {} socketed gem {} into item {}", state.name, source_uid, target_uid);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "宝石镶嵌失败");
            self.send_combine_item_response(msg.session_id, source.unique_id, target.unique_id, false, false);
        }
    }
}

pub struct DisassembleItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<DisassembleItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: DisassembleItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // 查找物品
        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };

        // 获取物品信息
        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该物品无法分解");
                return;
            }
        };

        // 只有装备类物品可以分解（有耐久度的非消耗品）
        if item_info.durability <= 0 || item_info.item_type == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法分解");
            return;
        }

        // 分解产出 = 根据等级和类型决定
        let grade = item_info.grade.max(1);
        let item_name = item_info.name.clone();
        let (mat_index, mat_count, mat_name) = match item_info.item_type {
            // 武器 -> 铁矿石
            1 => (500, grade as u16, "铁矿石"),
            // 盔甲/饰品 -> 布料/皮革
            2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 => (501, grade as u16, "皮革"),
            _ => (502, (grade / 2).max(1) as u16, "宝石碎片"),
        };

        // 移除原物品
        let removed = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory {
            unique_id: msg.unique_id,
        }).await.ok().flatten();
        if removed.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "分解失败：无法移除物品");
            return;
        }

        // 给予材料
        let material = crate::actors::inventory::make_item(mat_index, mat_count);
        let added = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item: material }).await.unwrap_or(false);
        if added {
            send_system_message(&self.gate_ref, msg.session_id,
                &format!("分解成功！获得 {} x{}", mat_name, mat_count));
        } else {
            // 背包满了：把材料丢到地上
            let drop_oid = self.alloc_object_id();
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: mir2_shared::data::item::UserItem {
                    item_index: mat_index,
                    count: mat_count,
                    ..Default::default()
                },
                location_x: state.x,
                location_y: state.y,
            };
            let mut buf = Vec::new();
            if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item) {
                warn!("Failed to serialize disassemble drop: {}", e);
            } else {
                for sid in self.players.keys() {
                    let _ = self.gate_ref.ask(SendToClient { session_id: *sid, data: buf.clone() });
                }
                self.ground_items.push(GroundItem {
                    object_id: drop_oid,
                    item: mir2_shared::data::item::UserItem {
                        item_index: mat_index,
                        count: mat_count,
                        ..Default::default()
                    },
                    x: state.x,
                    y: state.y,
                    map_index: state.map_index,
                    dropper_session: Some(msg.session_id),
                    drop_tick: self.tick_count,
                });
            }
            send_system_message(&self.gate_ref, msg.session_id,
                &format!("分解成功！背包已满，{} x{} 已掉落在地", mat_name, mat_count));
        }
        debug!("DisassembleItem: {} disassembled {} into {} x{}", state.name, item_name, mat_name, mat_count);
    }
}

// ============================================================
// 觉醒系统
// ============================================================

pub struct AwakeningNeedMaterialsRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub awake_type: u8,
}

impl Message<AwakeningNeedMaterialsRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AwakeningNeedMaterialsRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };

        let _awake_type = match mir2_shared::enums::AwakeType::try_from(msg.awake_type) {
            Ok(t) => t,
            Err(_) => {
                send_system_message(&self.gate_ref, msg.session_id, "无效的觉醒类型");
                return;
            }
        };

        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };

        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品信息不存在");
                return;
            }
        };

        if !item_info.can_awakening {
            send_system_message(&self.gate_ref, msg.session_id, "该物品不支持觉醒");
            return;
        }

        // 计算所需材料：觉醒材料是 item_type=35 的物品
        // shape 编码：0=DC, 1=MC, 2=SC, 3=AC, 4=MAC, 5=HpMp, 100=通用
        let awake_level = item.awake.awake_level();
        let grade_index = match item_info.grade {
            1..=4 => item_info.grade - 1,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "该物品品级不支持觉醒");
                return;
            }
        };

        // 材料数量 = 基础值 * (1 + 已觉醒等级)
        let base_count: i32 = match grade_index {
            0 => 3,
            1 => 5,
            2 => 8,
            _ => 12,
        };
        let needed = base_count * (1 + awake_level as i32);

        // 查找匹配的觉醒材料物品
        let type_shape = msg.awake_type.saturating_sub(1) as i32;
        let mut materials = Vec::new();
        for (idx, info) in self.item_infos.iter() {
            if info.item_type != 35 { continue; } // ItemType::Awakening
            if info.shape == type_shape || info.shape == 100 {
                materials.push(mir2_shared::packets::server::awakening_system::MaterialInfo {
                    item_id: *idx,
                    count: needed,
                });
            }
        }

        let packet = mir2_shared::packets::server::awakening_system::AwakeningNeedMaterials {
            item_id: item.item_index,
            materials,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize AwakeningNeedMaterials: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AwakeningNeedMaterials as i16, &body),
        });
    }
}

pub struct AwakeningLockedItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub locked: bool,
}

impl Message<AwakeningLockedItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AwakeningLockedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let packet = mir2_shared::packets::server::awakening_system::AwakeningLockedItem {
            unique_id: msg.unique_id,
            locked: msg.locked,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize AwakeningLockedItem: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AwakeningLockedItem as i16, &body),
        });
    }
}

pub struct AwakeningRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub awake_type: u8,
}

impl Message<AwakeningRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AwakeningRequest, _ctx: &mut Context<Self, Self::Reply>) {
        use mir2_shared::packets::server::awakening_system::*;

        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let awake_type = match mir2_shared::enums::AwakeType::try_from(msg.awake_type) {
            Ok(t) => t,
            Err(_) => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };

        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };

        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };

        // 验证：物品可觉醒
        if !item_info.can_awakening {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 验证：未达最大等级
        if item.awake.is_max_level() {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_MAX_LEVEL, -1);
            return;
        }

        // 验证：觉醒类型匹配（已觉醒的物品不能换类型）
        if item.awake.awake_type != mir2_shared::enums::AwakeType::None
            && item.awake.awake_type != awake_type
        {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 验证物品类型与觉醒类型兼容性 (Weapon=1, Armour=2, Helmet=4)
        let compatible = match item_info.item_type {
            1 => matches!(awake_type, mir2_shared::enums::AwakeType::Dc | mir2_shared::enums::AwakeType::Mc | mir2_shared::enums::AwakeType::Sc),
            4 => matches!(awake_type, mir2_shared::enums::AwakeType::Ac | mir2_shared::enums::AwakeType::Mac),
            2 => awake_type == mir2_shared::enums::AwakeType::HpMp,
            _ => false,
        };
        if !compatible {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 品级 (Common=1, Rare=2, Legendary=3, Mythical=4)
        let grade = match item_info.grade {
            1..=4 => item_info.grade,
            _ => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };
        let awake_level = item.awake.awake_level();

        // 检查金币：费用 = 1500 * (1 + awakeLevel * 2) * grade
        let gold_cost = 1500u64 * (1 + awake_level as u64 * 2) * grade as u64;
        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: gold_cost }).await.unwrap_or(false);
        if !has_gold {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_GOLD, -1);
            return;
        }

        // 检查材料：计算所需数量
        let base_count: u16 = match grade {
            1 => 3,
            2 => 5,
            3 => 8,
            _ => 12,
        };
        let needed = base_count * (1 + awake_level as u16);

        // 查找匹配的觉醒材料
        let type_shape = msg.awake_type.saturating_sub(1) as i32;
        let mut material_index: Option<i32> = None;
        for (idx, info) in self.item_infos.iter() {
            if info.item_type == 35 // ItemType::Awakening
                && (info.shape == type_shape || info.shape == 100)
            {
                material_index = Some(*idx);
                break;
            }
        }
        let mat_idx = match material_index {
            Some(idx) => idx,
            None => {
                // 没有配置觉醒材料，跳过材料检查
                0
            }
        };

        // 检查材料数量
        if mat_idx > 0 {
            let available = record.actor_ref.ask(crate::actors::player::CountItemsByIndex {
                item_index: mat_idx,
            }).await.unwrap_or(0);
            if available < needed {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_MATERIALS, -1);
                return;
            }
        }

        // 扣除材料
        if mat_idx > 0 {
            let consumed = record.actor_ref.ask(crate::actors::player::ConsumeItemsByIndex {
                item_index: mat_idx,
                count: needed,
            }).await.unwrap_or(false);
            if !consumed {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_MATERIALS, -1);
                return;
            }
        }

        // 扣除金币
        let gold_deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: gold_cost }).await;
        if !gold_deducted.unwrap_or(false) {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_GOLD, -1);
            return;
        }

        // 执行觉醒：70% 成功率
        let roll = fastrand::u8(0..100);
        if roll < mir2_shared::data::item::Awake::SUCCESS_RATE {
            // 成功：计算觉醒值
            let chance_max = mir2_shared::data::item::Awake::CHANCE_MAX
                .get(grade.saturating_sub(1) as usize)
                .copied()
                .unwrap_or(1);
            let rate = match item_info.item_type {
                1 => mir2_shared::data::item::Awake::WEAPON_RATE,  // Weapon
                4 => mir2_shared::data::item::Awake::HELMET_RATE,  // Helmet
                2 => mir2_shared::data::item::Awake::ARMOUR_RATE,  // Armour
                _ => 1,
            };
            let value = (fastrand::u8(1..=chance_max) as i32 * rate as i32).max(1) as u8;

            let mut awake = item.awake.clone();
            awake.awake_type = awake_type;
            awake.levels.push(value);

            let set = record.actor_ref.ask(crate::actors::player::SetItemAwake {
                unique_id: msg.unique_id,
                awake,
            }).await.unwrap_or(false);

            if set {
                debug!("Awakening success: {} item={} type={:?} value={}", state.name, msg.unique_id, awake_type, value);
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_SUCCESS, -1);
                send_system_message(&self.gate_ref, msg.session_id,
                    &format!("觉醒成功！{} +{}", awake_type_name(awake_type), value));
            } else {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            }
        } else {
            // 失败：物品被摧毁
            let removed = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory {
                unique_id: msg.unique_id,
            }).await.ok().flatten();
            if removed.is_some() {
                debug!("Awakening destroy: {} item={} destroyed", state.name, msg.unique_id);
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_DESTROYED, msg.unique_id as i64);
                send_system_message(&self.gate_ref, msg.session_id, "觉醒失败，物品已损毁！");
            } else {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            }
        }
    }
}

pub struct DowngradeAwakeningRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<DowngradeAwakeningRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DowngradeAwakeningRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };

        if item.awake.awake_level() == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品没有觉醒等级");
            return;
        }

        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品信息不存在");
                return;
            }
        };

        let grade = match item_info.grade {
            1..=4 => item_info.grade,
            _ => 1,
        };

        // 降级费用 = 3000 * (1 + (awakeLevel+1) * 2) * grade
        let awake_level = item.awake.awake_level() as u64;
        let gold_cost = 3000u64 * (1 + (awake_level + 1) * 2) * grade as u64;

        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: gold_cost }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, msg.session_id, &format!("金币不足，降级需要 {} 金币", gold_cost));
            return;
        }

        let gold_deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: gold_cost }).await;
        if !gold_deducted.unwrap_or(false) {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // 移除最后一级觉醒
        let mut awake = item.awake.clone();
        awake.levels.pop();
        if awake.levels.is_empty() {
            awake.awake_type = mir2_shared::enums::AwakeType::None;
        }

        let set = record.actor_ref.ask(crate::actors::player::SetItemAwake {
            unique_id: msg.unique_id,
            awake,
        }).await.unwrap_or(false);

        if set {
            debug!("DowngradeAwakening: {} item={} new_level={}", state.name, msg.unique_id, item.awake.awake_level().saturating_sub(1));
            send_system_message(&self.gate_ref, msg.session_id, "觉醒降级成功");
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "降级失败");
        }
    }
}

impl WorldActor {
    fn send_awakening_result(&self, session_id: u64, result: i32, remove_id: i64) {
        let packet = mir2_shared::packets::server::awakening_system::Awakening { result, remove_id };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize Awakening result: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Awakening as i16, &body),
        });
    }
}

fn awake_type_name(t: mir2_shared::enums::AwakeType) -> &'static str {
    match t {
        mir2_shared::enums::AwakeType::Dc => "攻击",
        mir2_shared::enums::AwakeType::Mc => "魔法",
        mir2_shared::enums::AwakeType::Sc => "道术",
        mir2_shared::enums::AwakeType::Ac => "防御",
        mir2_shared::enums::AwakeType::Mac => "魔防",
        mir2_shared::enums::AwakeType::HpMp => "生命/魔法",
        _ => "未知",
    }
}

pub struct ResetAddedItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<ResetAddedItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ResetAddedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let success = record.actor_ref.ask(crate::actors::player::ResetItemAddedStats {
            unique_id: msg.unique_id,
        }).await.unwrap_or(false);
        if success {
            send_system_message(&self.gate_ref, msg.session_id, "物品附加属性已重置");
            debug!("ResetAddedItem: {} uid={} - success", state.name, msg.unique_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该物品或无法重置");
            debug!("ResetAddedItem: {} uid={} - failed", state.name, msg.unique_id);
        }
    }
}

// ============================================================
// 对象查询
// ============================================================

pub struct RequestUserNameMsg {
    pub session_id: u64,
    pub object_id: u32,
}

impl Message<RequestUserNameMsg> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestUserNameMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let name = if let Some(npc) = self.npcs.get(&msg.object_id) {
            Some(npc.name.clone())
        } else if let Some(mon) = self.monsters.get(&msg.object_id) {
            Some(mon.name.clone())
        } else {
            for (_, record) in &self.players {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if state.object_id == msg.object_id {
                        // Found — send UserName response
                        let mut body = Vec::new();
                        body.extend_from_slice(&msg.object_id.to_le_bytes());
                        crate::util::wire::write_dotnet_string(&mut body, &state.name);
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: msg.session_id,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserName as i16, &body),
                        });
                        return;
                    }
                }
            }
            None
        };

        if let Some(name) = name {
            let mut body = Vec::new();
            body.extend_from_slice(&msg.object_id.to_le_bytes());
            crate::util::wire::write_dotnet_string(&mut body, &name);
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserName as i16, &body),
            });
        }
    }
}

pub struct RequestChatItemMsg {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<RequestChatItemMsg> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestChatItemMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(), None => return,
        };
        let item_info = record.actor_ref.ask(crate::actors::player::GetItemInfo {
            unique_id: msg.unique_id,
        }).await.ok().flatten();

        if let Some(item) = item_info {
            let mut stats_parts = Vec::new();
            if let Some(ref info) = item.info {
                stats_parts.push(info.name.clone());
                for (stat, value) in info.stats.iter() {
                    if value != 0 {
                        stats_parts.push(format!("{:?}: {}", stat, value));
                    }
                }
                if item.current_dura > 0 || info.durability > 0 {
                    stats_parts.push(format!("Dur: {}/{}", item.current_dura, info.durability));
                }
            } else {
                stats_parts.push(format!("Item#{}", item.item_index));
            }
            let stats_str = stats_parts.join(", ");
            let mut body = Vec::new();
            body.extend_from_slice(&msg.unique_id.to_le_bytes());
            crate::util::wire::write_dotnet_string(&mut body, &stats_str);
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChatItemStats as i16, &body),
            });
        }
    }
}

// ============================================================
// 轮回系统
// ============================================================

pub struct AcceptReincarnationRequest {
    pub session_id: u64,
}

impl Message<AcceptReincarnationRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: AcceptReincarnationRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // AcceptReincarnation: dead player accepts reincarnation from host.
        // C#: if ReincarnationHost != null && ReincarnationHost.ReincarnationReady -> Revive(HP/2)
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // Check if this player has a valid reincarnation host
        if state.reincarnation_host.is_none() {
            debug!("AcceptReincarnation: {} has no host", state.name);
            return;
        }

        let host_session = state.reincarnation_host.unwrap();
        // Verify host is still online and ready
        if !self.players.contains_key(&host_session) {
            debug!("AcceptReincarnation: host disconnected for {}", state.name);
            let _ = record.actor_ref.ask(ClearReincarnation);
            return;
        }

        debug!("AcceptReincarnation: {} accepted from host session={}", state.name, host_session);

        // Revive the dead player at half HP
        let _ = record.actor_ref.ask(ReviveAtHalfHp);

        // Clear reincarnation state on both players
        let _ = record.actor_ref.ask(ClearReincarnation);
        if let Some(host_record) = self.players.get(&host_session) {
            let _ = host_record.actor_ref.ask(ClearReincarnationHost);
        }
    }
}

pub struct CancelReincarnationRequest {
    pub session_id: u64,
}

impl Message<CancelReincarnationRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: CancelReincarnationRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // CancelReincarnation: dead player cancels reincarnation.
        // C#: ReincarnationExpireTime = Envir.Time (immediate expiry triggers cleanup)
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("CancelReincarnation: {}", state.name);

        // Set expire time to now, triggering immediate cleanup
        let _ = record.actor_ref.ask(ClearReincarnation);

        // Also notify host to clear their state
        if let Some(host_session) = state.reincarnation_host {
            if let Some(host_record) = self.players.get(&host_session) {
                let _ = host_record.actor_ref.ask(ClearReincarnationHost);
            }
        }
    }
}

// ============================================================
// 开门
// ============================================================

pub struct OpendoorRequest {
    pub session_id: u64,
    pub door_index: u8,
}

impl Message<OpendoorRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: OpendoorRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("Opendoor: {} door_index={}", state.name, msg.door_index);

        // Track open door state per map
        let map_key = state.map_index;
        self.open_doors.insert((map_key, msg.door_index));

        // Send Opendoor response to the player
        send_opendoor(&self.gate_ref, msg.session_id, msg.door_index, false);

        // Broadcast to all other players on the same map
        broadcast_opendoor_async(&self.gate_ref, &self.players, map_key, msg.door_index, false, msg.session_id).await;
    }
}

// ============================================================
// 邮件系统 Handler
// ============================================================

impl Message<SendMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SendMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let sender_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        if msg.receiver_name == sender_state.name {
            send_system_message(&self.gate_ref, msg.session_id, "不能给自己发送邮件");
            return;
        }

        // 检查金币是否足够
        let total_gold = msg.gold as u64;
        if sender_state.inventory.gold < total_gold {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        // 从发送者扣除物品
        let mut items: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        for uid in &msg.item_uids {
            if let Some(item) = sender_state.inventory.get_item(*uid) {
                items.push(item.clone());
            }
        }

        // 从发送者扣除金币和物品
        if total_gold > 0 {
            let _ = record.actor_ref.ask(DeductGold { amount: total_gold }).await;
        }
        for uid in &msg.item_uids {
            let _ = record.actor_ref.ask(RemoveItemFromInventory { unique_id: *uid }).await;
        }

        // 创建邮件
        let mail = MailMessage {
            mail_id: generate_mail_id(),
            sender_name: sender_state.name.clone(),
            receiver_name: msg.receiver_name.clone(),
            subject: msg.subject.clone(),
            body: msg.body.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            read: false,
            collected: false,
            locked: false,
            gold: total_gold,
            items,
        };

        // 查找收件人
        let mut target_session: Option<u64> = None;
        for (sid, r) in &self.players {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.name == msg.receiver_name {
                    target_session = Some(*sid);
                    break;
                }
            }
        }

        if let Some(target) = target_session {
            if let Some(target_record) = self.players.get(&target) {
                let _ = target_record.actor_ref.ask(crate::actors::player::AddMail { mail: mail.clone() }).await;
                send_mail_received_packet(&self.gate_ref, target, &mail);
                debug!("Mail delivered online: {} -> {}", sender_state.name, msg.receiver_name);
            }
        } else {
            // 收件人不在线，保存到数据库
            if let Err(e) = db::insert_mail(&self.db_pool, &msg.receiver_name, &mail).await {
                warn!("Failed to save offline mail for {}: {}", msg.receiver_name, e);
                send_system_message(&self.gate_ref, msg.session_id, "邮件发送失败，请稍后重试");
                return;
            }
            debug!("Mail saved offline: {} -> {}", sender_state.name, msg.receiver_name);
        }

        send_system_message(&self.gate_ref, msg.session_id, "邮件已发送");
    }
}

impl Message<ReadMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ReadMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let mail = match record.actor_ref.ask(crate::actors::player::GetMail { mail_id: msg.mail_id }).await {
            Ok(Some(m)) => m, _ => return,
        };

        send_mail_content_packet(&self.gate_ref, msg.session_id, &mail);
        let _ = record.actor_ref.ask(crate::actors::player::MarkMailRead { mail_id: msg.mail_id }).await;
    }
}

impl Message<CollectParcelRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CollectParcelRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let result = match record.actor_ref.ask(crate::actors::player::CollectMailAttachment { mail_id: msg.mail_id }).await {
            Ok(Some(r)) => r, _ => {
                send_system_message(&self.gate_ref, msg.session_id, "收取失败");
                return;
            }
        };

        let (gold, items) = result;
        if gold > 0 {
            let _ = record.actor_ref.ask(AddGold { amount: gold }).await;
        }
        for item in items {
            let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
        }

        send_system_message(&self.gate_ref, msg.session_id, "附件已收取");
    }
}

impl Message<DeleteMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DeleteMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let deleted = match record.actor_ref.ask(crate::actors::player::DeleteMail { mail_id: msg.mail_id }).await {
            Ok(d) => d, _ => return,
        };

        if deleted {
            send_system_message(&self.gate_ref, msg.session_id, "邮件已删除");
        }
    }
}

// ============================================================
// 市场/寄售系统
// ============================================================

/// 市场搜索缓存
#[derive(Debug, Clone)]
struct MarketSearchCache {
    results: Vec<usize>, // indices into self.auctions
}

pub struct MarketSearchRequest {
    pub session_id: u64,
    pub item_index: u32,
}

impl Message<MarketSearchRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketSearchRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketSearch: session={} item={}", msg.session_id, msg.item_index);

        // Collect indices of unsold auctions matching criteria
        let mut results: Vec<usize> = Vec::new();
        for (idx, auction) in self.auctions.iter().enumerate() {
            if auction.sold {
                continue;
            }
            if msg.item_index > 0 && auction.item.item_index != msg.item_index as i32 {
                continue;
            }
            results.push(idx);
        }

        let total = results.len();
        let pages = (total / 10 + if total % 10 > 0 { 1 } else { 0 }).max(1);

        // Store search results for pagination
        self.market_search_cache.insert(msg.session_id, MarketSearchCache {
            results: results.clone(),
        });

        // Send page count (NPCMarket)
        let page_packet = mir2_shared::packets::server::market_system::NPCMarket {
            pages: vec!["市场".to_string(); pages],
        };
        let mut body = Vec::new();
        if let Err(e) = page_packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarket: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarket as i16, &body),
        });

        // Send first page
        let end = 10.min(results.len());
        if end > 0 {
            let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = results[..end]
                .iter()
                .filter_map(|&idx| self.auctions.get(idx))
                .map(|a| mir2_shared::packets::server::market_system::MarketListing {
                    auction_id: a.auction_id,
                    item: a.item.clone(),
                    seller_name: a.seller_name.clone(),
                    price: a.price,
                    consignment_date: a.consignment_date,
                })
                .collect();
            let page_packet = mir2_shared::packets::server::market_system::NPCMarketPage { listings };
            let mut body = Vec::new();
            if let Err(e) = page_packet.write_body(&mut body) {
                warn!("Failed to serialize NPCMarketPage: {}", e);
                return;
            }
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
            });
        }
    }
}

pub struct MarketRefreshRequest {
    pub session_id: u64,
}

impl Message<MarketRefreshRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketRefreshRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketRefresh: session={}", msg.session_id);

        // Collect all unsold auctions
        let mut results: Vec<usize> = Vec::new();
        for (idx, auction) in self.auctions.iter().enumerate() {
            if !auction.sold {
                results.push(idx);
            }
        }

        let total = results.len();
        let pages = (total / 10 + if total % 10 > 0 { 1 } else { 0 }).max(1);

        // Update search cache
        self.market_search_cache.insert(msg.session_id, MarketSearchCache {
            results: results.clone(),
        });

        // Send page count (NPCMarket)
        let page_packet = mir2_shared::packets::server::market_system::NPCMarket {
            pages: vec!["市场".to_string(); pages],
        };
        let mut body = Vec::new();
        if let Err(e) = page_packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarket: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarket as i16, &body),
        });

        // Send first page
        let end = 10.min(results.len());
        if end > 0 {
            let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = results[..end]
                .iter()
                .filter_map(|&idx| self.auctions.get(idx))
                .map(|a| mir2_shared::packets::server::market_system::MarketListing {
                    auction_id: a.auction_id,
                    item: a.item.clone(),
                    seller_name: a.seller_name.clone(),
                    price: a.price,
                    consignment_date: a.consignment_date,
                })
                .collect();
            let page_packet = mir2_shared::packets::server::market_system::NPCMarketPage { listings };
            let mut body = Vec::new();
            if let Err(e) = page_packet.write_body(&mut body) {
                warn!("Failed to serialize NPCMarketPage: {}", e);
                return;
            }
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
            });
        }
    }
}

pub struct MarketPageRequest {
    pub session_id: u64,
    pub page: u32,
}

impl Message<MarketPageRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketPageRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketPage: session={} page={}", msg.session_id, msg.page);

        let cache = match self.market_search_cache.get(&msg.session_id) {
            Some(c) => c.clone(),
            None => {
                let packet = mir2_shared::packets::server::market_system::NPCMarketPage {
                    listings: Vec::new(),
                };
                let mut body = Vec::new();
                if let Err(e) = packet.write_body(&mut body) {
                    warn!("Failed to serialize NPCMarketPage: {}", e);
                    return;
                }
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
                });
                return;
            }
        };

        let page = msg.page as usize;
        let start = page * 10;
        let end = (start + 10).min(cache.results.len());

        let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = cache.results[start..end]
            .iter()
            .filter_map(|&idx| self.auctions.get(idx))
            .map(|a| mir2_shared::packets::server::market_system::MarketListing {
                auction_id: a.auction_id,
                item: a.item.clone(),
                seller_name: a.seller_name.clone(),
                price: a.price,
                consignment_date: a.consignment_date,
            })
            .collect();

        let packet = mir2_shared::packets::server::market_system::NPCMarketPage { listings };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarketPage: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
        });
    }
}

pub struct MarketBuyRequest {
    pub session_id: u64,
    pub listing_id: u64,
    pub count: u32,
}

impl Message<MarketBuyRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketBuyRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketBuy: session={} listing={} count={}", msg.session_id, msg.listing_id, msg.count);

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let buyer_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if buyer_state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法购买");
            return;
        }

        let auction_idx = match self.auctions.iter().position(|a| a.auction_id == msg.listing_id && !a.sold) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该商品已下架");
                return;
            }
        };

        // Prevent buying own listing
        if self.auctions[auction_idx].seller_name == buyer_state.name {
            send_system_message(&self.gate_ref, msg.session_id, "不能购买自己的商品");
            return;
        }

        let auction = &self.auctions[auction_idx];
        let price = auction.price as u64;
        let seller_name = auction.seller_name.clone();
        let item = auction.item.clone();

        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: price }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        let deducted = record.actor_ref.ask(DeductGold { amount: price }).await.unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // Try to add item to inventory first — if full, refund gold
        let added = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
        if !added {
            let _ = record.actor_ref.ask(AddGold { amount: price }).await;
            send_system_message(&self.gate_ref, msg.session_id, "背包已满，购买失败，金币已退回");
            return;
        }

        // Item delivered successfully — now persist the sale
        if let Err(e) = db::mark_auction_sold(&self.db_pool, msg.listing_id as i64, &buyer_state.name).await {
            warn!("Failed to mark auction {} sold in DB: {}", msg.listing_id, e);
            // In-memory state is still updated; the sale is valid
        }

        if let Some(a) = self.auctions.get_mut(auction_idx) {
            a.sold = true;
            a.buyer_name = Some(buyer_state.name.clone());
        }

        // Give gold to seller (online) or via mail (offline)
        let mut seller_online = false;
        for (_, seller_record) in &self.players {
            if let Ok(Some(seller_state)) = seller_record.actor_ref.ask(GetPlayerState).await {
                if seller_state.name == seller_name {
                    let _ = seller_record.actor_ref.ask(AddGold { amount: price }).await;
                    send_system_message(&self.gate_ref, seller_record.session_id, &format!("{} 购买了你的商品，获得 {} 金币", buyer_state.name, price));
                    seller_online = true;
                    break;
                }
            }
        }
        if !seller_online {
            // Send gold to offline seller via mail
            let mail = MailMessage {
                mail_id: generate_mail_id(),
                sender_name: "市场交易".to_string(),
                receiver_name: seller_name.clone(),
                subject: "商品售出".to_string(),
                body: format!("你寄售的商品已售出，获得 {} 金币", price),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                read: false,
                collected: false,
                locked: false,
                gold: price,
                items: Vec::new(),
            };
            if let Err(e) = db::insert_mail(&self.db_pool, &seller_name, &mail).await {
                warn!("Failed to save market sale mail for {}: {}", seller_name, e);
            }
            debug!("Seller {} is offline, gold {} sent via mail", seller_name, price);
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("购买成功：获得物品"));

        let packet = mir2_shared::packets::server::market_system::MarketSuccess {
            message: "购买成功".to_string(),
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize MarketSuccess: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MarketSuccess as i16, &body),
        });
    }
}

pub struct MarketGetBackRequest {
    pub session_id: u64,
    pub listing_id: u64,
}

impl Message<MarketGetBackRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketGetBackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketGetBack: session={} listing={}", msg.session_id, msg.listing_id);

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let auction_idx = match self.auctions.iter().position(|a| {
            a.auction_id == msg.listing_id && a.seller_name == state.name && !a.sold
        }) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该寄售物品或已售出");
                return;
            }
        };

        let item = self.auctions[auction_idx].item.clone();

        // Try to add item to inventory first — if full, don't delete the auction
        let added = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
        if !added {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满，无法取回物品");
            return;
        }

        let _ = db::delete_auction(&self.db_pool, msg.listing_id as i64).await;
        self.auctions.remove(auction_idx);
        send_system_message(&self.gate_ref, msg.session_id, "取回寄售物品成功");
    }
}

pub struct MarketSellNowRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub price: u64,
}

impl Message<MarketSellNowRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketSellNowRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketSellNow: session={} uid={} price={}", msg.session_id, msg.unique_id, msg.price);

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法操作");
            return;
        }

        let auction_idx = match self.auctions.iter().position(|a| {
            a.auction_id == msg.unique_id && a.seller_name == state.name && !a.sold
        }) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该寄售物品");
                return;
            }
        };

        let auction = &self.auctions[auction_idx];
        let price = auction.price as u64;
        let commission = price / 10;
        let seller_gold = price - commission;

        let _ = db::delete_auction(&self.db_pool, msg.unique_id as i64).await;
        self.auctions.remove(auction_idx);

        let _ = record.actor_ref.ask(AddGold { amount: seller_gold }).await;
        send_system_message(&self.gate_ref, msg.session_id, &format!("立即售出成功，扣除手续费 {} 金币，获得 {} 金币", commission, seller_gold));
    }
}

pub struct ConsignItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub price: u64,
}

impl Message<ConsignItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ConsignItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法寄售");
            return;
        }

        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };

        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品信息不存在");
                return;
            }
        };

        // 检查绑定：不能出售绑定的物品
        if item_info.bind_mode & 0x0004 != 0 {
            send_system_message(&self.gate_ref, msg.session_id, "绑定的物品无法寄售");
            return;
        }

        let price = msg.price as u32;
        if price == 0 || price > 1_000_000_000 {
            send_system_message(&self.gate_ref, msg.session_id, "价格无效");
            return;
        }

        // 寄售费用 = 5000 金币
        const CONSIGN_FEE: u64 = 5000;
        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: CONSIGN_FEE }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, msg.session_id, &format!("寄售需要 {} 金币", CONSIGN_FEE));
            return;
        }

        // 扣除费用
        let fee_ok = record.actor_ref.ask(crate::actors::player::DeductGold { amount: CONSIGN_FEE }).await.unwrap_or(false);
        if !fee_ok {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // 从背包移除物品
        let removed = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory {
            unique_id: msg.unique_id,
        }).await.ok().flatten();
        if removed.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "移除物品失败");
            return;
        }

        let auction_id = self.next_auction_id;
        self.next_auction_id += 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let item_json = match serde_json::to_string(&item) {
            Ok(j) => j,
            Err(e) => {
                warn!("Failed to serialize item for auction: {}", e);
                send_system_message(&self.gate_ref, msg.session_id, "寄售失败：数据错误");
                return;
            }
        };

        // 保存到数据库
        if let Err(e) = db::save_auction(&self.db_pool, auction_id as i64, &state.name, &item_json, price as i64, now, 0,
        ).await {
            warn!("Failed to save auction: {}", e);
            // Rollback: return item and refund fee
            let _ = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await;
            let _ = record.actor_ref.ask(AddGold { amount: CONSIGN_FEE }).await;
            send_system_message(&self.gate_ref, msg.session_id, "寄售失败：数据库错误，物品和金币已退回");
            return;
        }

        // 添加到内存列表
        self.auctions.push(AuctionListing {
            auction_id,
            seller_name: state.name.clone(),
            item: item.clone(),
            price,
            consignment_date: now,
            sold: false,
            buyer_name: None,
            item_type: 0,
        });

        // 发送成功响应
        let packet = mir2_shared::packets::server::market_system::ConsignItem {
            unique_id: msg.unique_id,
            success: true,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize ConsignItem response: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ConsignItem as i16, &body),
        });

        send_system_message(&self.gate_ref, msg.session_id,
            &format!("寄售成功！{} 以 {} 金币上架", item_info.name, price));
        debug!("ConsignItem: {} listed {} for {} gold (aid={})", state.name, item.item_index, price, auction_id);
    }
}

// ============================================================
// 物品租赁系统
// ============================================================

impl WorldActor {
    fn send_rental_packet<T: mir2_shared::packets::Packet>(&self, session_id: u64, packet: T) {
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize rental packet: {}", e);
            return;
        }
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: build_packet_bytes(T::OPCODE, &body),
        });
    }

    async fn find_session_by_name(&self, name: &str) -> Option<u64> {
        for (sid, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                if state.name == name {
                    return Some(*sid);
                }
            }
        }
        None
    }
}

pub struct ItemRentalRequestMsg {
    pub session_id: u64,
    pub target_name: String,
}

impl Message<ItemRentalRequestMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalRequestMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法租赁");
            return;
        }

        // Find target player by name
        let target_session = match self.find_session_by_name(&msg.target_name).await {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        if target_session == msg.session_id {
            send_system_message(&self.gate_ref, msg.session_id, "不能向自己发起租赁");
            return;
        }

        // Create rental session (initiator = renter, partner = owner)
        self.rental_sessions.insert(msg.session_id, RentalSession {
            partner_session: target_session,
            partner_name: msg.target_name.clone(),
            fee: 0,
            period_hours: 0,
            owner_item: None,
            renter_locked: false,
            owner_locked: false,
        });

        // Send rental request to target (owner)
        self.send_rental_packet(target_session, mir2_shared::packets::server::rental_system::ItemRentalRequest {});
        send_system_message(&self.gate_ref, target_session, &format!("{} 想向你租赁物品", state.name));
        debug!("ItemRentalRequest: {} -> {} (session {})", state.name, msg.target_name, target_session);
    }
}

pub struct DepositRentalItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<DepositRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: DepositRentalItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };

        // Find the rental session where this player is the partner (owner)
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let item = match record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::DepositRentalItem {
                    unique_id: msg.unique_id,
                    success: false,
                });
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.owner_item = Some(item.clone());
        }

        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::DepositRentalItem {
            unique_id: msg.unique_id,
            success: true,
        });
        // Also update the renter's dialog
        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::UpdateRentalItem {
            item: item.clone(),
            rental_fee: self.rental_sessions.get(&initiator).map(|s| s.fee).unwrap_or(0),
            rental_period: self.rental_sessions.get(&initiator).map(|s| s.period_hours as i32).unwrap_or(0),
        });
        debug!("DepositRentalItem: session={} uid={}", msg.session_id, msg.unique_id);
    }
}

pub struct RetrieveRentalItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<RetrieveRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: RetrieveRentalItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };

        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let item = if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.owner_item.take()
        } else {
            None
        };

        if let Some(item) = item {
            let added = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                unique_id: msg.unique_id,
                success: added,
            });
            // Update renter's dialog (clear item)
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::UpdateRentalItem {
                item: mir2_shared::data::item::UserItem::default(),
                rental_fee: 0,
                rental_period: 0,
            });
            debug!("RetrieveRentalItem: session={} uid={}", msg.session_id, msg.unique_id);
        } else {
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                unique_id: msg.unique_id,
                success: false,
            });
        }
    }
}

pub struct CancelItemRentalRequest {
    pub session_id: u64,
}

impl Message<CancelItemRentalRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: CancelItemRentalRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // Cancel can be sent by either renter or owner
        let (initiator, is_renter) = if let Some(_) = self.rental_sessions.get(&msg.session_id) {
            (msg.session_id, true)
        } else {
            match self.rental_sessions.iter().find(|(_, s)| s.partner_session == msg.session_id).map(|(k, _)| *k) {
                Some(sid) => (sid, false),
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                    return;
                }
            }
        };

        let session = self.rental_sessions.remove(&initiator);
        if let Some(s) = session {
            // Return item to owner if deposited
            if let Some(item) = s.owner_item {
                if let Some(record) = self.players.get(&s.partner_session) {
                    let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
                }
            }
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::CancelItemRental {
                unique_id: 0,
                success: true,
            });
            let other = if is_renter { s.partner_session } else { initiator };
            self.send_rental_packet(other, mir2_shared::packets::server::rental_system::CancelItemRental {
                unique_id: 0,
                success: true,
            });
            debug!("CancelItemRental: session={} (initiator={})", msg.session_id, initiator);
        }
    }
}

pub struct ItemRentalFeeMsg {
    pub session_id: u64,
    pub amount: u32,
}

impl Message<ItemRentalFeeMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalFeeMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.fee = msg.amount;
        }

        // Broadcast fee to both players
        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ItemRentalFee { fee: msg.amount });
        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalFee { fee: msg.amount });
        debug!("ItemRentalFee: initiator={} fee={}", initiator, msg.amount);
    }
}

pub struct ItemRentalPeriodMsg {
    pub session_id: u64,
    pub duration: u32,
}

impl Message<ItemRentalPeriodMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalPeriodMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.period_hours = msg.duration;
        }

        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ItemRentalPeriod { period: msg.duration as i32 });
        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalPeriod { period: msg.duration as i32 });
        debug!("ItemRentalPeriod: initiator={} hours={}", initiator, msg.duration);
    }
}

pub struct ItemRentalLockFeeMsg {
    pub session_id: u64,
}

impl Message<ItemRentalLockFeeMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalLockFeeMsg, _ctx: &mut Context<Self, Self::Reply>) {
        // LockFee is sent by the renter (initiator)
        let (partner, both_locked) = {
            let session = match self.rental_sessions.get_mut(&msg.session_id) {
                Some(s) => s,
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                    return;
                }
            };
            session.renter_locked = true;
            (session.partner_session, session.owner_locked)
        };

        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalLock {
            unique_id: 0,
            locked: true,
        });
        self.send_rental_packet(partner, mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
            unique_id: 0,
            locked: true,
        });

        // Check if both locked and can confirm
        if both_locked {
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
            self.send_rental_packet(partner, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
        }
        debug!("ItemRentalLockFee: session={}", msg.session_id);
    }
}

pub struct ItemRentalLockItemMsg {
    pub session_id: u64,
}

impl Message<ItemRentalLockItemMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalLockItemMsg, _ctx: &mut Context<Self, Self::Reply>) {
        // LockItem is sent by the owner (partner)
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let (partner, item_uid, both_locked) = {
            let session = match self.rental_sessions.get_mut(&initiator) {
                Some(s) => s,
                None => return,
            };
            session.owner_locked = true;
            (
                session.partner_session,
                session.owner_item.as_ref().map(|i| i.unique_id).unwrap_or(0),
                session.renter_locked,
            )
        };

        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalLock {
            unique_id: item_uid,
            locked: true,
        });
        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
            unique_id: item_uid,
            locked: true,
        });
        if both_locked {
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
            self.send_rental_packet(partner, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
        }
        debug!("ItemRentalLockItem: session={}", msg.session_id);
    }
}

pub struct ConfirmItemRentalMsg {
    pub session_id: u64,
}

impl Message<ConfirmItemRentalMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ConfirmItemRentalMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let (initiator, _) = if let Some(_) = self.rental_sessions.get(&msg.session_id) {
            (msg.session_id, true)
        } else {
            match self.rental_sessions.iter().find(|(_, s)| s.partner_session == msg.session_id).map(|(k, _)| *k) {
                Some(sid) => (sid, false),
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                    return;
                }
            }
        };

        let session = match self.rental_sessions.remove(&initiator) {
            Some(s) => s,
            None => return,
        };

        if !session.renter_locked || !session.owner_locked {
            send_system_message(&self.gate_ref, msg.session_id, "双方尚未锁定");
            return;
        }

        let item = match session.owner_item {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有租赁物品");
                return;
            }
        };

        let fee = session.fee as u64;
        let renter_record = match self.players.get(&initiator) { Some(r) => r.clone(), None => return };
        let owner_record = match self.players.get(&session.partner_session) { Some(r) => r.clone(), None => return };

        // Check renter has enough gold
        let has_gold = renter_record.actor_ref.ask(crate::actors::player::HasGold { amount: fee }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, initiator, "金币不足，无法支付租金");
            // Return item to owner
            let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            return;
        }

        // Deduct gold from renter
        let deducted = renter_record.actor_ref.ask(DeductGold { amount: fee }).await.unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, initiator, "金币扣除失败，租赁取消");
            let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            return;
        }

        // Give gold to owner
        let _ = owner_record.actor_ref.ask(AddGold { amount: fee }).await;

        // Give item to renter
        let added = renter_record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
        if !added {
            // Give gold back and return item to owner
            let _ = renter_record.actor_ref.ask(AddGold { amount: fee }).await;
            let _ = owner_record.actor_ref.ask(DeductGold { amount: fee }).await;
            let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
            send_system_message(&self.gate_ref, initiator, "背包已满，租赁失败");
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            return;
        }

        send_system_message(&self.gate_ref, initiator, &format!("租赁成功！支付 {} 金币，获得物品 {}", fee, item.item_index));
        send_system_message(&self.gate_ref, session.partner_session, &format!("租赁成功！获得 {} 金币，物品 {} 已出租", fee, item.item_index));

        // Record the rental for expiry tracking
        let period_hours = session.period_hours.max(1);
        let expiry = chrono::Local::now().timestamp() + (period_hours as i64 * 3600);
        self.player_rentals.entry(renter_record.name.clone())
            .or_default()
            .push(RentedItem {
                item: item.clone(),
                owner_name: owner_record.name.clone(),
                renter_name: renter_record.name.clone(),
                rental_fee: session.fee,
                expiry_timestamp: expiry,
            });

        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: true });
        self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: true });
        debug!("ConfirmItemRental: {} -> {} item={} fee={}", initiator, session.partner_session, item.item_index, fee);
    }
}

pub struct GetRentedItemsRequest {
    pub session_id: u64,
}

impl Message<GetRentedItemsRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GetRentedItemsRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let items: Vec<mir2_shared::packets::server::rental_system::RentalItemInfo> =
            self.player_rentals.get(&state.name)
                .map(|rentals| rentals.iter().map(|r| {
                    mir2_shared::packets::server::rental_system::RentalItemInfo {
                        item: r.item.clone(),
                        rental_fee: r.rental_fee,
                        rental_period: 0,
                        expiry_date: r.expiry_timestamp,
                    }
                }).collect())
                .unwrap_or_default();

        let packet = mir2_shared::packets::server::rental_system::GetRentedItems { items };
        self.send_rental_packet(msg.session_id, packet);
        debug!("GetRentedItems: {} count={}", state.name, self.player_rentals.get(&state.name).map(|v| v.len()).unwrap_or(0));
    }
}

// ============================================================
// 行会战/领地
// ============================================================

pub struct GuildWarReturnRequest {
    pub session_id: u64,
    pub guild_name: String,
}

impl Message<GuildWarReturnRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildWarReturnRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // GuildWarReturn: query if a guild exists and return its war status
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("GuildWarReturn: {} querying guild={}", state.name, msg.guild_name);

        if state.guild_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        }

        let sender_guild = state.guild_name.as_ref().unwrap();
        if msg.guild_name == *sender_guild {
            send_system_message(&self.gate_ref, msg.session_id, "不能向自己的行会宣战");
            return;
        }

        // 行会信息由 SocialActor 管理，此处仅做简单校验
        if msg.guild_name.is_empty() {
            send_system_message(&self.gate_ref, msg.session_id, "行会名称无效");
            return;
        }

        // C# requires guild leader (rank 0) to declare war
        if state.guild_rank != GuildRank::Leader {
            send_system_message(&self.gate_ref, msg.session_id, "只有行会会长才能宣战");
            return;
        }

        // Record the war declaration
        self.guild_wars.entry(sender_guild.clone()).or_default().insert(msg.guild_name.clone());
        self.guild_wars.entry(msg.guild_name.clone()).or_default().insert(sender_guild.clone());

        // Notify all online members of the declaring guild
        let war_msg = format!("行会 {} 已向 {} 宣战！", sender_guild, msg.guild_name);
        for (sid, rec) in &self.players {
            if *sid == msg.session_id { continue; }
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                if s.guild_name.as_deref() == Some(sender_guild.as_str()) {
                    send_system_message(&self.gate_ref, *sid, &war_msg);
                }
            }
        }

        // Notify all online members of the target guild
        let target_msg = format!("行会 {} 已向你们宣战！", sender_guild);
        for (sid, rec) in &self.players {
            if let Ok(Some(s)) = rec.actor_ref.ask(GetPlayerState).await {
                if s.guild_name.as_deref() == Some(msg.guild_name.as_str()) {
                    send_system_message(&self.gate_ref, *sid, &target_msg);
                }
            }
        }

        // Send GuildRequestWar packet back to the declarer
        use mir2_shared::packets::server::miscellaneous::GuildRequestWar;
        let war_packet = GuildRequestWar { guild_name: msg.guild_name.clone() };
        let mut war_body = Vec::new();
        if let Ok(()) = mir2_shared::packets::Packet::write_body(&war_packet, &mut war_body) {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildRequestWar as i16, &war_body),
            });
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("已向 {} 行会宣战", msg.guild_name));
    }
}

pub struct GuildBuffUpdateRequest {
    pub session_id: u64,
    pub buff_id: u32,
}

impl Message<GuildBuffUpdateRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildBuffUpdateRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("GuildBuffUpdate: {} buff_id={}", state.name, msg.buff_id);

        if state.guild_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        }

        // buff_id=0 means "request buff list" - send empty list (no buffs defined yet)
        if msg.buff_id == 0 {
            let body = Vec::new();
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildBuffList as i16, &body),
            });
        } else {
            debug!("GuildBuffUpdate: {} buff_id={} (guild buff system not implemented)", state.name, msg.buff_id);
        }
    }
}

pub struct GuildTerritoryPageRequest {
    pub session_id: u64,
    pub page: u32,
}

impl Message<GuildTerritoryPageRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GuildTerritoryPageRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("GuildTerritoryPage: session={} page={}", msg.session_id, msg.page);
        // Send empty territory list (no territories defined)
        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GuildTerritoryPage as i16, &body),
        });
    }
}

pub struct PurchaseGuildTerritoryRequest {
    pub session_id: u64,
    pub territory_id: u32,
}

impl Message<PurchaseGuildTerritoryRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: PurchaseGuildTerritoryRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("PurchaseGuildTerritory: {} territory={}", state.name, msg.territory_id);

        if state.guild_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
            return;
        }

        if state.guild_rank != GuildRank::Leader {
            send_system_message(&self.gate_ref, msg.session_id, "只有行会会长才能购买领地");
            return;
        }

        send_system_message(&self.gate_ref, msg.session_id, "当前没有可购买的行会领地");
    }
}

// ============================================================
// NPC确认输入
// ============================================================

pub struct NPCConfirmInputRequest {
    pub session_id: u64,
    pub npc_id: u32,
    pub input_text: String,
}

impl Message<NPCConfirmInputRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: NPCConfirmInputRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        debug!("NPCConfirmInput: {} npc_id={} input={}", state.name, msg.npc_id, msg.input_text);

        // Try to match input as a quest file_name for quick acceptance
        let npc = match self.npcs.get(&msg.npc_id) {
            Some(n) => n,
            None => return,
        };
        if npc.db_index > 0 {
            if let Some(npc_db) = self.npc_infos.get(&npc.db_index) {
                // Check if input matches a collectable quest
                let quest_db = npc_db.collect_quest_indexes.iter()
                    .filter_map(|qi| self.quest_infos.get(qi))
                    .find(|q| q.file_name == msg.input_text || q.name == msg.input_text);
                if let Some(quest_db) = quest_db {
                    if state.level >= quest_db.required_min_level as u16
                        && (quest_db.required_max_level == 0 || state.level <= quest_db.required_max_level as u16)
                    {
                        // Check not already accepted
                        if let Ok(None) = record.actor_ref.ask(GetQuest { quest_index: quest_db.index }).await {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            let quest = make_quest_instance(quest_db, now);
                            if let Ok(true) = record.actor_ref.ask(AcceptQuest { quest }).await {
                                send_system_message(&self.gate_ref, msg.session_id,
                                    &format!("任务已接受: {}", quest_db.name));
                            }
                            return;
                        }
                    }
                }
                // Check if input matches a finishable quest
                let quest_db = npc_db.finish_quest_indexes.iter()
                    .filter_map(|qi| self.quest_infos.get(qi))
                    .find(|q| q.file_name == msg.input_text || q.name == msg.input_text);
                if let Some(quest_db) = quest_db {
                    // Complete the quest
                    if let Ok(Some(quest)) = record.actor_ref.ask(GetQuest { quest_index: quest_db.index }).await {
                        if quest.status == QuestStatus::InProgress {
                            let _ = record.actor_ref.ask(CompleteQuest { quest_index: quest_db.index }).await;
                            // Grant rewards
                            let _ = record.actor_ref.ask(AddExperience { amount: self.apply_global_exp_multiplier(quest_db.exp_reward) }).await;
                            let _ = record.actor_ref.ask(AddGold { amount: quest_db.gold_reward.max(0) as u64 }).await;
                            send_system_message(&self.gate_ref, msg.session_id,
                                &format!("任务完成: +{}经验, +{}金币", quest_db.exp_reward, quest_db.gold_reward.max(0)));
                            return;
                        }
                    }
                }
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, "无法识别该指令");
    }
}

// ============================================================
// 游戏商店/举报/排名
// ============================================================

/// 游戏商店商品定义
struct ShopItem {
    item_index: i32,
    gold_price: u32,
    credit_price: u32,
    count: i32,
    class: u8,
    category: &'static str,
    stock: i32,
}

/// 游戏商店硬编码目录（fallback，当 DB 无数据时使用）
fn game_shop_catalog_fallback() -> &'static [ShopItem] {
    &[
        // 经验丹 - 增加1000经验
        ShopItem { item_index: 1, gold_price: 5000, credit_price: 100, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 回城卷
        ShopItem { item_index: 2, gold_price: 1000, credit_price: 20, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 随机传送卷
        ShopItem { item_index: 3, gold_price: 2000, credit_price: 40, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 双倍经验卷
        ShopItem { item_index: 4, gold_price: 10000, credit_price: 200, count: 1, class: 255, category: "消耗品", stock: 999 },
        // 经验丹x10
        ShopItem { item_index: 5, gold_price: 40000, credit_price: 800, count: 10, class: 255, category: "消耗品", stock: 999 },
    ]
}

/// 发送游戏商店目录给玩家
fn send_game_shop_catalog(gate_ref: &ActorRef<GateActor>, session_id: u64, gold: u32, shop_items: &[db::GameShopItem]) {
    use mir2_shared::packets::server::special_systems::{GameShopInfo, GameShopItem as ProtoItem};

    let items: Vec<ProtoItem> = if shop_items.is_empty() {
        // Fallback to hardcoded
        game_shop_catalog_fallback().iter().map(|s| ProtoItem {
            item_index: s.item_index,
            gold_price: s.gold_price,
            credit_price: s.credit_price,
            count: s.count,
            class: s.class,
            category: s.category.to_string(),
            stock: s.stock,
            is_bought: false,
            deal: false,
        }).collect()
    } else {
        shop_items.iter().map(|s| ProtoItem {
            item_index: s.item_index,
            gold_price: s.gold_price,
            credit_price: s.credit_price,
            count: s.count as i32,
            class: 255, // DB class_name is string; use default
            category: s.category.clone(),
            stock: s.stock,
            is_bought: false,
            deal: s.deal,
        }).collect()
    };

    let packet = GameShopInfo {
        items,
        credit: 0,
        gold,
    };

    let mut body = Vec::new();
    let _ = packet.write_body(&mut body);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GameShopInfo as i16, &body),
    });
}

pub struct GameshopBuyRequest {
    pub session_id: u64,
    pub item_id: u32,
    pub count: u32,
}

impl Message<GameshopBuyRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GameshopBuyRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // item_id=0 请求商店目录
        if msg.item_id == 0 {
            debug!("GameShop: {} requesting catalog", state.name);
            send_game_shop_catalog(&self.gate_ref, msg.session_id, state.inventory.gold as u32, &self.game_shop_items);
            return;
        }

        // 查找商品（优先 DB，fallback 硬编码）
        let db_item = self.game_shop_items.iter().find(|i| i.item_index as u32 == msg.item_id);
        let fallback = game_shop_catalog_fallback().iter().find(|i| i.item_index as u32 == msg.item_id);
        let (item_price, item_count) = if let Some(di) = db_item {
            (di.gold_price as u64, di.count as u32)
        } else if let Some(fi) = fallback {
            (fi.gold_price as u64, fi.count as u32)
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "商品不存在");
            return;
        };

        let buy_count = msg.count.max(1).min(item_count);
        let total_gold = item_price.saturating_mul(buy_count as u64);

        debug!("GameshopBuy: {} item={} count={} gold={}", state.name, msg.item_id, buy_count, total_gold);

        // 检查金币
        if state.inventory.gold < total_gold as u64 {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        // 先构建邮件（在扣金币前，避免扣款后交付失败导致玩家损失）
        let shop_item = self.game_shop_items.iter().find(|i| i.item_index as u32 == msg.item_id);
        let item_index = if let Some(si) = shop_item {
            si.item_index
        } else {
            msg.item_id as i32
        };

        let mail_items: Vec<mir2_shared::data::item::UserItem> = if let Some(item_db) = self.item_infos.get(&item_index) {
            (0..buy_count).map(|_| {
                let uid = generate_item_uid();
                mir2_shared::data::item::UserItem {
                    unique_id: uid,
                    item_index: item_db.index,
                    count: 1,
                    current_dura: item_db.durability as u16,
                    max_dura: item_db.durability as u16,
                    // TODO: verify bool_flags bit 0 = identified; C# source uses enum flag
                    identified: item_db.start_item || item_db.bool_flags & (1 << 0) != 0,
                    ..Default::default()
                }
            }).collect()
        } else {
            (0..buy_count).map(|_| {
                mir2_shared::data::item::UserItem {
                    unique_id: generate_item_uid(),
                    item_index,
                    ..Default::default()
                }
            }).collect()
        };

        let mail = MailMessage {
            mail_id: generate_mail_id(),
            sender_name: "GameShop".to_string(),
            receiver_name: state.name.clone(),
            subject: "商城购买".to_string(),
            body: format!("您购买了 {} 件商品", buy_count),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            read: false,
            collected: false,
            locked: false,
            gold: 0,
            items: mail_items,
        };

        // 扣款
        let _ = record.actor_ref.ask(DeductGold { amount: total_gold as u64 }).await;

        // 发送邮件
        send_mail_received_packet(&self.gate_ref, msg.session_id, &mail);
        let _ = record.actor_ref.ask(crate::actors::player::AddMail { mail }).await;

        send_system_message(&self.gate_ref, msg.session_id,
            &format!("购买成功！已扣除金币 {}，物品已通过邮件发送", total_gold));

        // 发送库存更新
        let stock_remaining = item_count.saturating_sub(buy_count);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GameShopStock as i16, &{
                let mut body = Vec::new();
                body.extend_from_slice(&(msg.item_id as i32).to_le_bytes());
                body.extend_from_slice(&stock_remaining.to_le_bytes());
                body
            }),
        });
    }
}

pub struct ReportIssueRequest {
    pub session_id: u64,
    pub issue_type: u8,
    pub description: String,
}

impl Message<ReportIssueRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ReportIssueRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("ReportIssue: session={} type={}", msg.session_id, msg.issue_type);
        send_system_message(&self.gate_ref, msg.session_id, "举报信息已提交，感谢您的反馈");
    }
}

pub struct GetRankingRequest {
    pub session_id: u64,
    pub rank_type: u8,
}

impl Message<GetRankingRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GetRankingRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("GetRanking: session={} type={}", msg.session_id, msg.rank_type);

        // 收集在线玩家信息
        let mut entries: Vec<(String, u8, i32, i64)> = Vec::new();
        for (_, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                entries.push((
                    state.name.clone(),
                    state.class as u8,
                    state.level as i32,
                    state.experience,
                ));
            }
        }

        // 按等级降序、经验降序排序
        entries.sort_by(|a, b| {
            b.2.cmp(&a.2).then_with(|| b.3.cmp(&a.3))
        });

        // 取前 20 名
        let rankings: Vec<mir2_shared::packets::server::special_systems::RankInfo> = entries
            .into_iter()
            .take(20)
            .enumerate()
            .map(|(idx, (name, class, level, experience))| {
                mir2_shared::packets::server::special_systems::RankInfo {
                    rank: (idx + 1) as i32,
                    player_name: name,
                    class,
                    level,
                    experience,
                }
            })
            .collect();

        let packet = mir2_shared::packets::server::special_systems::Rankings { rankings };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Rankings as i16, &body),
            });
        }
    }
}

// ============================================================
// 辅助函数
// ============================================================

impl WorldActor {
    /// 从已加载的物品配置中随机选择一个适合钓鱼收获的物品索引
    fn random_fishing_item_index(
        item_infos: &HashMap<i32, db::ItemInfo>,
        session_id: u64,
        tick_count: u64,
    ) -> i32 {
        if item_infos.is_empty() {
            return 1;
        }
        let keys: Vec<i32> = item_infos.keys().copied().collect();
        let idx = ((session_id + tick_count) as usize) % keys.len();
        keys[idx]
    }
}

fn send_system_message(gate_ref: &ActorRef<GateActor>, session_id: u64, message: &str) {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    crate::util::wire::write_dotnet_string(&mut body, message);
    body.push(0u8); // ChatType::System
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::Chat as i16, &body),
    });
}

/// Send an item to a player via mail (for offline delivery or inventory-full fallback)
fn send_item_via_mail(
    db_pool: &crate::db::DbPool,
    receiver_name: &str,
    item: mir2_shared::data::item::UserItem,
    subject: &str,
    body: &str,
) {
    let mail = MailMessage {
        mail_id: generate_mail_id(),
        sender_name: "系统".to_string(),
        receiver_name: receiver_name.to_string(),
        subject: subject.to_string(),
        body: body.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        read: false,
        collected: false,
        locked: false,
        gold: 0,
        items: vec![item],
    };
    // Fire and forget — we're likely in a tick handler
    let pool = db_pool.clone();
    let receiver = receiver_name.to_string();
    tokio::spawn(async move {
        if let Err(e) = db::insert_mail(&pool, &receiver, &mail).await {
            warn!("Failed to send item via mail to {}: {}", receiver, e);
        }
    });
}

/// 向所有在线玩家广播系统消息
fn broadcast_system_message(gate_ref: &ActorRef<GateActor>, players: &HashMap<u64, PlayerRecord>, message: &str) {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    crate::util::wire::write_dotnet_string(&mut body, message);
    body.push(mir2_shared::enums::ChatType::System as u8);
    let packet = build_packet_bytes(ServerPacketIds::Chat as i16, &body);
    for session_id in players.keys() {
        let _ = gate_ref.ask(SendToClient {
            session_id: *session_id,
            data: packet.clone(),
        });
    }
}

/// 从 DB 任务配置创建任务实例
fn make_quest_instance(qi: &db::QuestInfo, start_time: u64) -> QuestInstance {
    let mut progress = Vec::new();
    for kill in &qi.kill_tasks {
        progress.push(QuestProgress {
            progress_id: kill.monster_index,
            current: 0,
            target: kill.count,
        });
    }
    for item in &qi.item_tasks {
        progress.push(QuestProgress {
            progress_id: item.item_index,
            current: 0,
            target: item.count,
        });
    }
    for flag in &qi.flag_tasks {
        progress.push(QuestProgress {
            progress_id: flag.number,
            current: 0,
            target: 1,
        });
    }
    QuestInstance {
        quest_index: qi.index,
        title: qi.name.clone(),
        status: QuestStatus::InProgress,
        progress,
        exp_reward: qi.exp_reward as i64,
        gold_reward: qi.gold_reward.max(0) as u64,
        start_time,
        time_limit_seconds: qi.time_limit_seconds,
    }
}

/// Send Opendoor response to a single player
fn send_opendoor(gate_ref: &ActorRef<GateActor>, session_id: u64, door_index: u8, close: bool) {
    let mut body = Vec::new();
    body.push(door_index);
    body.push(if close { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Opendoor as i16, &body),
    });
}

/// Broadcast Opendoor to all players on a map (excluding the initiator)
async fn broadcast_opendoor_async(gate_ref: &ActorRef<GateActor>, players: &HashMap<u64, PlayerRecord>, map_index: u16, door_index: u8, close: bool, exclude_session_id: u64) {
    let mut body = Vec::new();
    body.push(door_index);
    body.push(if close { 1u8 } else { 0u8 });
    let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::Opendoor as i16, &body);

    for record in players.values() {
        if record.session_id == exclude_session_id { continue; }
        let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await else { continue };
        if state.map_index == map_index {
            let _ = gate_ref.ask(SendToClient {
                session_id: record.session_id,
                data: packet.clone(),
            });
        }
    }
}

fn send_move_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, from: i32, to: i32, success: bool) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&from.to_le_bytes());
    body.extend_from_slice(&to.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MoveItem as i16, &body),
    });
}

fn send_use_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, uid: u64) {
    let mut body = Vec::new();
    body.extend_from_slice(&uid.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UseItem as i16, &body),
    });
}

/// 计算装备属性加成总和
fn calculate_equipment_bonuses(
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    item_infos: &std::collections::HashMap<i32, crate::db::ItemInfo>,
) -> (i32, i32, i32, i32, i32) {
    use mir2_shared::enums::Stat;
    let mut min_atk = 0i32;
    let mut max_atk = 0i32;
    let mut def = 0i32;
    let mut hp = 0i32;
    let mut mp = 0i32;

    for eq in equipment.iter().flatten() {
        if let Some(info) = item_infos.get(&eq.item_index) {
            min_atk += info.stats.get(&(Stat::MinDC as u8)).copied().unwrap_or(0);
            max_atk += info.stats.get(&(Stat::MaxDC as u8)).copied().unwrap_or(0);
            def += info.stats.get(&(Stat::MaxAC as u8)).copied().unwrap_or(0);
            hp += info.stats.get(&(Stat::HP as u8)).copied().unwrap_or(0);
            mp += info.stats.get(&(Stat::MP as u8)).copied().unwrap_or(0);
        }
    }

    (min_atk, max_atk, def, hp, mp)
}

fn send_equip_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, uid: u64, slot: i32, success: bool) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&slot.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::EquipItem as i16, &body),
    });
}

fn send_remove_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, uid: u64, success: bool) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveItem as i16, &body),
    });
}

fn send_drop_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, uid: u64, count: u32, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&count.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DropItem as i16, &body),
    });
}

fn send_merge_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid_from: u8, grid_to: u8, from_uid: u64, to_uid: u64, success: bool) {
    let mut body = Vec::new();
    body.push(grid_from);
    body.push(grid_to);
    body.extend_from_slice(&from_uid.to_le_bytes());
    body.extend_from_slice(&to_uid.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MergeItem as i16, &body),
    });
}

fn send_split_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, uid: u64, count: u32) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&count.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SplitItem as i16, &body),
    });
}

fn send_sell_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, uid: u64, count: u32, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&count.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SellItem as i16, &body),
    });
}

// ============================================================
// 邮件系统网络辅助函数
// ============================================================

fn send_mail_received_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, mail: &MailMessage) {
    let mut body = Vec::new();
    body.extend_from_slice(&mail.mail_id.to_le_bytes());
    write_dotnet_string(&mut body, &mail.sender_name);
    write_dotnet_string(&mut body, &mail.subject);
    body.extend_from_slice(&mail.timestamp.to_le_bytes());
    body.push(if mail.read { 1u8 } else { 0u8 });
    body.push(if mail.collected { 1u8 } else { 0u8 });
    body.extend_from_slice(&(mail.gold as u32).to_le_bytes());
    body.push(mail.items.len() as u8);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ReceiveMail as i16, &body),
    });
}

fn send_mail_content_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, mail: &MailMessage) {
    let mut body = Vec::new();
    body.extend_from_slice(&mail.mail_id.to_le_bytes());
    write_dotnet_string(&mut body, &mail.sender_name);
    write_dotnet_string(&mut body, &mail.subject);
    write_dotnet_string(&mut body, &mail.body);
    body.extend_from_slice(&mail.timestamp.to_le_bytes());
    body.push(if mail.read { 1u8 } else { 0u8 });
    body.push(if mail.collected { 1u8 } else { 0u8 });
    body.extend_from_slice(&(mail.gold as u32).to_le_bytes());
    body.push(mail.items.len() as u8);
    // 发送附件物品信息
    for item in &mail.items {
        body.extend_from_slice(&item.unique_id.to_le_bytes());
        body.extend_from_slice(&(item.item_index as u32).to_le_bytes());
        write_dotnet_string(&mut body, &item.info.as_ref().map(|i| i.name.clone()).unwrap_or_default());
        body.extend_from_slice(&item.count.to_le_bytes());
        body.extend_from_slice(&item.current_dura.to_le_bytes());
        body.extend_from_slice(&item.max_dura.to_le_bytes());
    }
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ReceiveMail as i16, &body),
    });
}

fn send_inspect_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, state: &crate::actors::player::PlayerState) {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&state.object_id.to_le_bytes());
    write_dotnet_string(&mut body, &state.name);
    write_dotnet_string(&mut body, state.guild_name.as_deref().unwrap_or(""));
    body.extend_from_slice(&state.level.to_le_bytes());
    body.push(state.class as u8);
    body.push(state.gender as u8);
    // 装备信息（只发送已装备的）
    body.push(state.inventory.equipment.iter().filter(|s| s.is_some()).count() as u8);
    for eq in state.inventory.equipment.iter().flatten() {
        body.extend_from_slice(&eq.unique_id.to_le_bytes());
        body.extend_from_slice(&eq.item_index.to_le_bytes());
        body.extend_from_slice(&(eq.current_dura as i32).to_le_bytes());
        body.extend_from_slice(&(eq.max_dura as i32).to_le_bytes());
    }

    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::PlayerInspect as i16, &body),
    });
}

// ============================================================
// 任务系统网络辅助函数
// ============================================================

fn send_quest_complete_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, quest_index: i32) {
    let mut body = Vec::new();
    body.extend_from_slice(&quest_index.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CompleteQuest as i16, &body),
    });
}

// ============================================================
// 英雄系统网络辅助函数
// ============================================================

fn send_hero_update_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, hero_index: u8) {
    let body = vec![hero_index];
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChangeHero as i16, &body),
    });
}

// ============================================================
// 仓库/金币网络辅助函数
// ============================================================

fn send_store_item_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, _grid: u8, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&0i32.to_le_bytes()); // from
    body.extend_from_slice(&0i32.to_le_bytes()); // to
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::StoreItem as i16, &body),
    });
}

fn send_take_back_item_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, _grid: u8, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&0i32.to_le_bytes()); // from
    body.extend_from_slice(&0i32.to_le_bytes()); // to
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TakeBackItem as i16, &body),
    });
}

fn send_gold_changed_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, new_gold: u64) {
    let mut body = Vec::new();
    body.extend_from_slice(&(new_gold as u32).to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LoseGold as i16, &body),
    });
}

// ============================================================
// 宠物系统网络辅助函数
// ============================================================

fn send_creature_list_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, creature: Option<&IntelligentCreature>) {
    let mut body = Vec::new();
    if let Some(c) = creature {
        body.extend_from_slice(&1i32.to_le_bytes());
        body.push(c.creature_type as u8);
        body.push(c.pickup_mode as u8);
        body.push(if c.enabled { 1u8 } else { 0u8 });
        body.push(c.hunger);
        write_dotnet_string(&mut body, c.custom_name.as_deref().unwrap_or(""));
    } else {
        body.extend_from_slice(&0i32.to_le_bytes());
    }
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UpdateIntelligentCreatureList as i16, &body),
    });
}

// ============================================================
// 游戏进入序列
// ============================================================

/// 发送完整的游戏进入序列到客户端
fn send_game_entry_sequence(
    gate_ref: ActorRef<GateActor>,
    session_id: u64,
    state: &PlayerState,
    map_file: &str,
    map_title: &str,
    is_big_map: bool,
) {
    use mir2_shared::enums::ServerPacketIds;

    let sid = session_id;

    // 1. StartGame (result=4=Success, resolution=0)
    let mut start_game_body = Vec::new();
    start_game_body.push(4u8);
    start_game_body.extend_from_slice(&0i32.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::StartGame as i16, &start_game_body),
    });

    // 2. MapChanged
    let map_changed = build_map_changed_packet(state.map_index, map_file, map_title, state.x, state.y, is_big_map);
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: map_changed,
    });

    // 3. UserInformation
    let user_info = build_user_information_packet(state);
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: user_info,
    });

    // 4. HealthChanged
    let mut health_body = Vec::new();
    health_body.extend_from_slice(&(state.hp as u32).to_le_bytes());
    health_body.extend_from_slice(&(state.mp as u32).to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::HealthChanged as i16, &health_body),
    });

    // 5. UserLocation
    let mut location_body = Vec::new();
    location_body.extend_from_slice(&state.x.to_le_bytes());
    location_body.extend_from_slice(&state.y.to_le_bytes());
    location_body.push(state.direction);
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &location_body),
    });

    info!("Game entry sequence sent to session {}", sid);
}

// ============================================================
// 数据包构建辅助函数
// ============================================================

fn build_map_changed_packet(
    map_index: u16,
    file_name: &str,
    title: &str,
    spawn_x: i32,
    spawn_y: i32,
    is_big_map: bool,
) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&(map_index as i32).to_le_bytes());
    write_dotnet_string(&mut body, file_name);
    write_dotnet_string(&mut body, title);
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body.push(if is_big_map { 1u8 } else { 0u8 });
    body.extend_from_slice(&spawn_x.to_le_bytes());
    body.extend_from_slice(&spawn_y.to_le_bytes());
    body.push(4u8);
    body.push(1u8);
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());

    build_packet_bytes(ServerPacketIds::MapChanged as i16, &body)
}

/// 根据 PK 值计算名字颜色（0=白名, 1=红名, 2=橙名）
fn name_colour_for_pk(pk_points: i32) -> i32 {
    if pk_points >= 200 {
        1 // Red
    } else if pk_points >= 100 {
        2 // Orange
    } else {
        0 // White
    }
}

/// 检查攻击者是否可以在当前攻击模式下攻击目标玩家
fn can_attack_player(attacker: &PlayerState, target: &PlayerState) -> bool {
    use mir2_shared::enums::AttackMode;
    match attacker.attack_mode {
        AttackMode::Peace => false,
        AttackMode::Group => {
            // 不能攻击同组成员
            attacker.group_id.is_none() || attacker.group_id != target.group_id
        }
        AttackMode::Guild => {
            // 不能攻击同行会成员
            attacker.guild_name.is_none() || attacker.guild_name != target.guild_name
        }
        AttackMode::EnemyGuild => {
            // 简化：只能攻击不同行会的玩家（且双方都有行会）
            attacker.guild_name.is_some()
                && target.guild_name.is_some()
                && attacker.guild_name != target.guild_name
        }
        AttackMode::RedBrown => {
            // 只能攻击红名/橙名玩家
            target.pk_points >= 100
        }
        AttackMode::All => true,
    }
}

fn build_user_information_packet(state: &PlayerState) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    // --- 字段顺序必须与客户端 UserInformation::read_body 一致 ---
    body.extend_from_slice(&state.object_id.to_le_bytes());   // object_id
    body.extend_from_slice(&1u32.to_le_bytes());              // real_id
    write_dotnet_string(&mut body, &state.name);              // name
    write_dotnet_string(&mut body, state.guild_name.as_deref().unwrap_or(""));  // guild_name
    write_dotnet_string(&mut body, "");                       // guild_rank
    body.extend_from_slice(&name_colour_for_pk(state.pk_points).to_le_bytes()); // name_colour
    body.push(state.class as u8);                             // class
    body.push(state.gender as u8);                            // gender
    body.extend_from_slice(&state.level.to_le_bytes());       // level
    body.extend_from_slice(&state.x.to_le_bytes());           // location_x
    body.extend_from_slice(&state.y.to_le_bytes());           // location_y
    body.push(state.direction);                               // direction
    body.push(state.hair);                                    // hair
    body.extend_from_slice(&state.hp.to_le_bytes());          // hp
    body.extend_from_slice(&state.mp.to_le_bytes());          // mp
    body.extend_from_slice(&state.experience.to_le_bytes());  // experience
    body.extend_from_slice(&state.max_experience.to_le_bytes()); // max_experience
    body.extend_from_slice(&0u16.to_le_bytes());              // level_effects
    body.push(0u8);                                           // has_hero=false
    body.push(state.hero_behaviour);                           // hero_behaviour

    // 客户端期望的后续字段（read_body 继续读取的部分）
    body.push(0u8);                                           // has_inventory=false
    body.push(0u8);                                           // has_equipment=false
    body.push(0u8);                                           // has_quest_inventory=false
    body.extend_from_slice(&(state.inventory.gold as u32).to_le_bytes()); // gold
    body.extend_from_slice(&0u32.to_le_bytes());              // credit
    body.push(0u8);                                           // has_expanded_storage=false
    body.extend_from_slice(&0i64.to_le_bytes());              // expanded_storage_expiry_time
    body.extend_from_slice(&0i32.to_le_bytes());              // magic_count=0
    body.extend_from_slice(&0i32.to_le_bytes());              // creature_count=0
    body.push(0u8);                                           // summoned_creature_type
    body.push(0u8);                                           // creature_summoned=false
    body.push(0u8);                                           // allow_observe=false
    body.push(0u8);                                           // observer=false

    build_packet_bytes(ServerPacketIds::UserInformation as i16, &body)
}

/// 构建 ObjectPlayer 数据包（其他玩家进入视野）
fn build_object_player_packet(
    name: &str, object_id: u32, x: i32, y: i32, direction: u8, level: u16,
    name_colour: i32,
    class: mir2_shared::enums::MirClass,
    gender: mir2_shared::enums::MirGender,
    hair: u8,
    weapon: i16, weapon_effect: i16, armor: i16,
    mount_type: i16, is_mounted: bool,
) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&object_id.to_le_bytes());   // object_id
    write_dotnet_string(&mut body, name);               // name
    write_dotnet_string(&mut body, "");                 // guild_name
    write_dotnet_string(&mut body, "");                 // guild_rank_name
    body.extend_from_slice(&name_colour.to_le_bytes()); // name_colour
    body.push(class as u8);                             // class
    body.push(gender as u8);                            // gender
    body.extend_from_slice(&level.to_le_bytes());       // level
    body.extend_from_slice(&x.to_le_bytes());           // location_x
    body.extend_from_slice(&y.to_le_bytes());           // location_y
    body.push(direction);                               // direction
    body.push(hair);                                    // hair
    body.push(1u8);                                     // light
    body.extend_from_slice(&weapon.to_le_bytes());        // weapon
    body.extend_from_slice(&weapon_effect.to_le_bytes()); // weapon_effect
    body.extend_from_slice(&armor.to_le_bytes());         // armour
    body.extend_from_slice(&0u16.to_le_bytes());        // poison=None (client reads u16)
    body.push(0u8);                                     // dead=false
    body.push(0u8);                                     // hidden=false
    body.push(0u8);                                     // effect=None
    body.push(0u8);                                     // wing_effect
    body.push(0u8);                                     // extra=false
    body.extend_from_slice(&mount_type.to_le_bytes());  // mount_type
    body.push(if is_mounted { 1u8 } else { 0u8 });      // riding_mount
    body.push(0u8);                                     // fishing=false
    body.extend_from_slice(&0i16.to_le_bytes());        // transform_type
    body.extend_from_slice(&0u32.to_le_bytes());        // element_orb_effect
    body.extend_from_slice(&0u32.to_le_bytes());        // element_orb_lvl
    body.extend_from_slice(&0u32.to_le_bytes());        // element_orb_max
    body.extend_from_slice(&0i32.to_le_bytes());        // buffs count=0
    body.extend_from_slice(&0u16.to_le_bytes());        // level_effects=None (client reads u16)

    build_packet_bytes(ServerPacketIds::ObjectPlayer as i16, &body)
}

/// 发送 PlayerUpdate 数据包（装备视觉变化）
fn send_player_update(
    gate_ref: &ActorRef<GateActor>,
    session_id: u64,
    object_id: u32,
    light: u8,
    weapon: i16,
    weapon_effect: i16,
    armor: i16,
    wings_effect: u8,
) {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    body.extend_from_slice(&object_id.to_le_bytes());
    body.push(light);
    body.extend_from_slice(&weapon.to_le_bytes());
    body.extend_from_slice(&weapon_effect.to_le_bytes());
    body.extend_from_slice(&armor.to_le_bytes());
    body.push(wings_effect);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::PlayerUpdate as i16, &body),
    });
}

/// 构建 ObjectColourChanged 数据包
fn build_object_colour_changed_packet(object_id: u32, name_colour: i32) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    body.extend_from_slice(&object_id.to_le_bytes());
    body.extend_from_slice(&name_colour.to_le_bytes());
    build_packet_bytes(ServerPacketIds::ObjectColourChanged as i16, &body)
}

/// 构建 ObjectNpc 数据包
fn build_object_npc_packet(npc: &NpcSpawn, object_id: u32) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&object_id.to_le_bytes());   // object_id
    write_dotnet_string(&mut body, &npc.name);          // name
    body.extend_from_slice(&0i32.to_le_bytes());        // name_colour
    body.extend_from_slice(&npc.image.to_le_bytes());   // image (NPC/Monster enum)
    body.extend_from_slice(&0i32.to_le_bytes());        // colour
    body.extend_from_slice(&npc.x.to_le_bytes());       // location_x
    body.extend_from_slice(&npc.y.to_le_bytes());       // location_y
    body.push(npc.direction);                           // direction
    body.extend_from_slice(&0i32.to_le_bytes());        // quest_ids count=0

    build_packet_bytes(ServerPacketIds::ObjectNpc as i16, &body)
}

/// 构建 ObjectMonster 数据包
fn build_object_monster_packet(monster: &MonsterSpawn, object_id: u32, name: &str) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&object_id.to_le_bytes());   // object_id
    write_dotnet_string(&mut body, name);               // name
    body.extend_from_slice(&0i32.to_le_bytes());        // name_colour
    body.extend_from_slice(&monster.x.to_le_bytes());   // location_x
    body.extend_from_slice(&monster.y.to_le_bytes());   // location_y
    body.extend_from_slice(&monster.image.to_le_bytes()); // image (Monster enum)
    body.push(monster.direction);                       // direction
    body.push(0u8);                                     // effect=None
    body.push(0u8);                                     // ai=None
    body.push(1u8);                                     // light
    body.push(0u8);                                     // dead=false
    body.push(0u8);                                     // skeleton=false
    body.extend_from_slice(&0u16.to_le_bytes());        // poison=None
    body.push(0u8);                                     // hidden=false
    body.extend_from_slice(&0i64.to_le_bytes());        // shock_time
    body.push(0u8);                                     // binding_shot_center=false
    body.push(0u8);                                     // extra=false
    body.push(0u8);                                     // extra_byte
    body.extend_from_slice(&0i32.to_le_bytes());        // buffs count=0

    build_packet_bytes(ServerPacketIds::ObjectMonster as i16, &body)
}

/// 发送地图上的 NPC 和怪物给新玩家，返回 NPC 和怪物列表
fn spawn_npcs_and_monsters(
    gate_ref: ActorRef<GateActor>,
    spawn_dir: &Option<PathBuf>,
    map_file: &str,
    map_index: u16,
    session_id: u64,
    next_object_id: &mut u32,
    ctx: &SpawnContext<'_>,
) -> (Vec<NpcState>, Vec<MonsterState>) {
    // Try DB-loaded configs first, fall back to TOML
    let config = if let Some(mi) = ctx.map_info {
        spawn_config_from_db(mi, ctx.monster_infos, ctx.npc_infos)
    } else if let Some(d) = spawn_dir {
        load_spawn_config(map_file, map_index, d)
    } else {
        return (Vec::new(), Vec::new());
    };
    if config.npcs.is_empty() && config.monsters.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // 发送 NPC 并创建运行时状态
    let mut npcs = Vec::new();
    for npc in &config.npcs {
        let object_id = *next_object_id;
        *next_object_id += 1;
        let packet = build_object_npc_packet(npc, object_id);
        let _ = gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });

        npcs.push(NpcState {
            object_id,
            name: npc.name.clone(),
            x: npc.x,
            y: npc.y,
            direction: npc.direction,
            db_index: npc.db_index,
            map_index,
        });
    }

    // 发送怪物并创建运行时状态
    let mut monsters = Vec::new();
    for monster in &config.monsters {
        let object_id = *next_object_id;
        *next_object_id += 1;

        // 精英判定：3% 概率
        let is_elite = fastrand::u8(1..=100) <= 3;
        let (name, hp, max_hp, min_dmg, max_dmg, xp) = if is_elite {
            (
                format!("[精英] {}", monster.name),
                monster.hp.saturating_mul(2),
                monster.hp.saturating_mul(2),
                (monster.min_dmg as f32 * 1.5) as i32,
                (monster.max_dmg as f32 * 1.5) as i32,
                monster.xp.saturating_mul(2),
            )
        } else {
            (monster.name.clone(), monster.hp, monster.hp, monster.min_dmg, monster.max_dmg, monster.xp)
        };

        let packet = build_object_monster_packet(monster, object_id, &name);
        let _ = gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });

        let ai_profile = ctx.monster_infos
            .get(&monster.monster_index)
            .map(MonsterAiProfile::from_info)
            .unwrap_or_else(|| MonsterAiProfile {
                ai_type: MonsterAiType::Aggressive,
                aggro_range: 10,
                attack_range: 1,
                attack_cooldown: 5,
                move_interval: 2,
                flee_threshold: 0.0,
            });
        monsters.push(MonsterState {
            object_id,
            name: name.clone(),
            image: monster.image,
            monster_index: monster.monster_index,
            x: monster.x,
            y: monster.y,
            direction: monster.direction,
            hp,
            max_hp,
            min_dmg,
            max_dmg,
            xp,
            spawn_x: monster.x,
            spawn_y: monster.y,
            map_index,
            next_attack_tick: 0,
            next_move_tick: 0,
            next_summon_tick: 0,
            ai_profile,
            ai_state: MonsterAiState::Idle,
            target_session: None,
            provoked: false,
            is_elite,
            is_boss: false,
        });
        if is_elite {
            debug!("Elite monster '{}' spawned as #{} at ({},{})", name, object_id, monster.x, monster.y);
        }
    }

    info!("Spawned {} NPCs and {} monsters for session {}",
          config.npcs.len(), config.monsters.len(), session_id);

    // Spawn dragon if enabled and on this map
    if let Some(dragon) = ctx.dragon_info {
        if dragon.enabled && dragon.map_file_name == map_file {
            if let Some(monster_index) = dragon.monster_index {
                if let Some(monster_db) = ctx.monster_infos.get(&monster_index) {
                    let object_id = *next_object_id;
                    *next_object_id += 1;
                    let hp = monster_db.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(10000);
                    let min_dmg = monster_db.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(50);
                    let max_dmg = monster_db.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(100);
                    let xp = monster_db.experience;
                    let packet = build_object_monster_packet(
                        &MonsterSpawn { name: dragon.monster_name.clone(), image: monster_db.image as u16, monster_index, x: dragon.location_x, y: dragon.location_y, direction: 0, hp, min_dmg, max_dmg, xp, map_index },
                        object_id,
                        &dragon.monster_name,
                    );
                    let _ = gate_ref.ask(SendToClient { session_id, data: packet });
                    let ai_profile = MonsterAiProfile::from_info(monster_db);
                    monsters.push(MonsterState {
                        object_id,
                        name: dragon.monster_name.clone(),
                        image: monster_db.image as u16,
                        monster_index,
                        x: dragon.location_x,
                        y: dragon.location_y,
                        direction: 0,
                        hp,
                        max_hp: hp,
                        min_dmg,
                        max_dmg,
                        xp,
                        spawn_x: dragon.location_x,
                        spawn_y: dragon.location_y,
                        map_index,
                        next_attack_tick: 0,
                        next_move_tick: 0,
                        next_summon_tick: 0,
                        ai_profile,
                        ai_state: MonsterAiState::Idle,
                        target_session: None,
                        provoked: false,
                        is_elite: false,
                        is_boss: false,
                    });
                    info!("Spawned dragon at ({}, {}) on map {}", dragon.location_x, dragon.location_y, map_file);
                }
            }
        }
    }

    (npcs, monsters)
}

fn drop_count_multiplier(is_boss: bool, is_elite: bool) -> u16 {
    if is_boss { 3 } else if is_elite { 2 } else { 1 }
}

fn should_despawn_boss(tick_count: u64, despawn_tick: u64) -> bool {
    tick_count >= despawn_tick
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_colour_for_pk_thresholds() {
        assert_eq!(name_colour_for_pk(0), 0);    // White
        assert_eq!(name_colour_for_pk(50), 0);   // White
        assert_eq!(name_colour_for_pk(99), 0);   // White
        assert_eq!(name_colour_for_pk(100), 2);  // Orange
        assert_eq!(name_colour_for_pk(150), 2);  // Orange
        assert_eq!(name_colour_for_pk(199), 2);  // Orange
        assert_eq!(name_colour_for_pk(200), 1);  // Red
        assert_eq!(name_colour_for_pk(500), 1);  // Red
    }

    #[test]
    fn test_elite_multiplier_saturation() {
        assert_eq!(5u16.saturating_mul(2), 10);
        assert_eq!(100u16.saturating_mul(2), 200);
        assert_eq!(u16::MAX.saturating_mul(2), u16::MAX);
    }

    #[test]
    fn test_drop_count_multiplier() {
        assert_eq!(drop_count_multiplier(false, false), 1); // normal
        assert_eq!(drop_count_multiplier(false, true), 2);  // elite
        assert_eq!(drop_count_multiplier(true, false), 3);  // boss
        assert_eq!(drop_count_multiplier(true, true), 3);   // boss trumps elite
    }

    #[test]
    fn test_should_despawn_boss() {
        assert!(!should_despawn_boss(0, 6000));
        assert!(!should_despawn_boss(5999, 6000));
        assert!(should_despawn_boss(6000, 6000)); // 10 min timeout
        assert!(should_despawn_boss(6001, 6000));
        assert!(should_despawn_boss(99999, 6000));
    }

    #[test]
    fn test_boss_monster_state() {
        let boss = MonsterState {
            object_id: 1,
            name: "TestBoss".to_string(),
            image: 100,
            monster_index: 50,
            x: 100,
            y: 100,
            direction: 0,
            hp: 10000,
            max_hp: 10000,
            min_dmg: 50,
            max_dmg: 100,
            xp: 5000,
            spawn_x: 100,
            spawn_y: 100,
            map_index: 0,
            next_attack_tick: 0,
            next_move_tick: 0,
            next_summon_tick: 0,
            ai_profile: MonsterAiProfile {
                ai_type: MonsterAiType::Boss,
                aggro_range: 10,
                attack_range: 2,
                attack_cooldown: 3,
                move_interval: 1,
                flee_threshold: 0.0,
            },
            ai_state: MonsterAiState::Idle,
            target_session: None,
            provoked: false,
            is_elite: false,
            is_boss: true,
        };
        assert!(boss.is_boss);
        assert!(!boss.is_elite);
        assert_eq!(boss.ai_profile.ai_type, MonsterAiType::Boss);
    }

    #[test]
    fn test_light_for_hour() {
        use mir2_shared::enums::LightSetting;
        assert_eq!(WorldActor::light_for_hour(0), LightSetting::Night);
        assert_eq!(WorldActor::light_for_hour(4), LightSetting::Night);
        assert_eq!(WorldActor::light_for_hour(5), LightSetting::Dawn);
        assert_eq!(WorldActor::light_for_hour(6), LightSetting::Dawn);
        assert_eq!(WorldActor::light_for_hour(7), LightSetting::Day);
        assert_eq!(WorldActor::light_for_hour(12), LightSetting::Day);
        assert_eq!(WorldActor::light_for_hour(16), LightSetting::Day);
        assert_eq!(WorldActor::light_for_hour(17), LightSetting::Evening);
        assert_eq!(WorldActor::light_for_hour(18), LightSetting::Evening);
        assert_eq!(WorldActor::light_for_hour(19), LightSetting::Night);
        assert_eq!(WorldActor::light_for_hour(23), LightSetting::Night);
    }

    #[test]
    fn test_awake_type_name() {
        use mir2_shared::enums::AwakeType;
        assert_eq!(awake_type_name(AwakeType::Dc), "攻击");
        assert_eq!(awake_type_name(AwakeType::Mc), "魔法");
        assert_eq!(awake_type_name(AwakeType::Sc), "道术");
        assert_eq!(awake_type_name(AwakeType::Ac), "防御");
        assert_eq!(awake_type_name(AwakeType::Mac), "魔防");
        assert_eq!(awake_type_name(AwakeType::HpMp), "生命/魔法");
        assert_eq!(awake_type_name(AwakeType::None), "未知");
    }

    #[test]
    fn test_awake_success_rate_constant() {
        assert_eq!(mir2_shared::data::item::Awake::SUCCESS_RATE, 70);
        assert_eq!(mir2_shared::data::item::Awake::MAX_AWAKE_LEVEL, 5);
    }

    #[test]
    fn test_awake_level_and_value() {
        use mir2_shared::data::item::Awake;
        use mir2_shared::enums::AwakeType;

        let mut awake = Awake::default();
        assert_eq!(awake.awake_level(), 0);
        assert!(!awake.is_max_level());
        assert_eq!(awake.awake_value(), 0);

        awake.awake_type = AwakeType::Dc;
        awake.levels = vec![2, 3, 1];
        assert_eq!(awake.awake_level(), 3);
        assert_eq!(awake.awake_value(), 6);
        assert_eq!(awake.get_dc(), 6);
        assert_eq!(awake.get_mc(), 0);
        assert!(!awake.is_max_level());

        awake.levels = vec![1, 1, 1, 1, 1];
        assert!(awake.is_max_level());
    }
}
