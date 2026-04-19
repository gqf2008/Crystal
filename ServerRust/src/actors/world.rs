// WorldActor - 游戏世界主循环
// 对应 C# GameSrv/WorldServer.cs + M2Server 核心逻辑

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::prelude::Context;
use kameo::message::Message;
use tokio::time::{interval, Duration};
use tracing::{info, debug, warn};

use crate::actors::player::{PlayerActor, PlayerState, MoveType, MoveRequest, TurnRequest, BroadcastMovement, GetPlayerState, SetMapData, SetPlayerState, AttackRequest, TakeDamage, AddItemToInventory, InventoryMoveItem, GetItemInfo, ConsumeItem, InventoryEquipItem, GetEquipmentInfo, InventoryUnequipItem, RemoveItemFromInventory, InventoryMergeItem, InventorySplitItem, DropGold, AddGold, DeductGold, AddExperience, AcceptQuest, CompleteQuest, AbandonQuest, GetQuest, HasCompletedQuest, SetCreature, TickCreatureHunger, SetHeroIndex, StoreItem, TakeBackItem, SetRefineLog, SetAttackMode, SetPetMode, SetPlayerPosition, SetFishing, ClearReincarnation, ClearReincarnationHost, ReviveAtHalfHp};
use crate::actors::inventory::{EquipmentSlot, GroundItem, PlayerInventory, generate_item_uid};
use crate::actors::refine::{RefineStatus, RefineLog};
use crate::actors::friend::FriendList;
use crate::actors::mail::{MailMessage, Mailbox, generate_mail_id};
use crate::actors::guild::GuildRank;
use crate::actors::quest::{QuestInstance, QuestStatus, QuestLog};
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
}

/// 地图刷怪配置
#[derive(Debug, Clone, Default)]
pub struct SpawnConfig {
    pub npcs: Vec<NpcSpawn>,
    pub monsters: Vec<MonsterSpawn>,
}

/// 加载刷怪配置
fn load_spawn_config(map_name: &str, spawn_dir: &Path) -> SpawnConfig {
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
    /// AI 配置（创建时从 DB 加载）
    pub ai_profile: MonsterAiProfile,
    /// 当前 AI 状态
    pub ai_state: MonsterAiState,
    /// 当前目标玩家 session（None = 无目标）
    pub target_session: Option<u64>,
    /// 是否已被激怒（Passive 怪物被攻击后变为 Aggressive）
    pub provoked: bool,
}

fn dist_to_spawn(monster: &MonsterState) -> i32 {
    (monster.x - monster.spawn_x).abs() + (monster.y - monster.spawn_y).abs()
}

/// 运行时 NPC 状态
#[derive(Clone)]
struct NpcState {
    pub object_id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub db_index: i32,
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

/// WorldActor 状态
pub struct WorldActor {
    /// Tick 计数器
    tick_count: u64,
    /// 在线玩家 Actor 引用（按 session_id 索引）
    players: HashMap<u64, PlayerRecord>,
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
    /// 死亡玩家复活队列 (session_id → 死亡 tick)
    player_death_queue: HashMap<u64, u64>,
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
}

impl WorldActor {
    pub fn new(gate_ref: ActorRef<GateActor>, map_dir: PathBuf, spawn_dir: Option<PathBuf>, db_pool: DbPool, social_ref: ActorRef<SocialActor>) -> Self {
        Self {
            tick_count: 0,
            players: HashMap::new(),
            maps: HashMap::new(),
            gate_ref,
            map_dir,
            spawn_dir,
            next_object_id: 1000,
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            respawn_queue: HashMap::new(),
            player_death_queue: HashMap::new(),
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

    /// 怪物死亡时生成掉落并广播给所有在线玩家
    async fn spawn_monster_drops(&mut self, monster: &MonsterState) {
        // 查找该怪物的掉落配置
        let drops = match self.monster_drops.get(&monster.monster_index) {
            Some(d) if !d.is_empty() => d.clone(),
            _ => return,
        };

        for drop in drops {
            // 概率判定
            let roll = fastrand::f64();
            if roll > drop.chance {
                continue;
            }

            // 掉落数量
            let count = if drop.max_count > drop.min_count {
                fastrand::u16(drop.min_count..=drop.max_count)
            } else {
                drop.min_count
            };

            let drop_oid = self.alloc_object_id();

            if drop.item_index == 0 {
                // 金币掉落（用 ObjectGold）
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
                    continue;
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
                // 物品掉落
                let mut item = mir2_shared::data::item::UserItem {
                    item_index: drop.item_index,
                    unique_id: generate_item_uid(),
                    count,
                    ..Default::default()
                };
                // 填充耐久（如果有物品配置）
                if let Some(info) = self.item_infos.get(&drop.item_index) {
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
                    continue;
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
                debug!("Monster '{}' dropped item index={} count={} at ({}, {})", monster.name, drop.item_index, count, monster.x, monster.y);
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
        _npc: &NpcState,
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
                        }
                    }
                    "GOTO" => {
                        if let Some(target) = parts.next() {
                            goto_target = Some(target.to_string());
                            break;
                        }
                    }
                    "BREAK" => break,
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

        let quest_infos_list = match db::load_quest_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} quest configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load quest_infos from DB: {}", e); Vec::new() }
        };
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
            maps: HashMap::new(),
            gate_ref: args.gate_ref,
            map_dir: args.map_dir,
            spawn_dir: args.spawn_dir,
            next_object_id: 1000,
            monsters: HashMap::new(),
            npcs: HashMap::new(),
            respawn_queue: HashMap::new(),
            player_death_queue: HashMap::new(),
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
            let player_positions: Vec<(u64, i32, i32, u32)> = {
                let mut results = Vec::new();
                for (session_id, record) in &self.players {
                    if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                        if !state.is_dead {
                            let in_safe = self.maps.get(&state.map_index)
                                .map(|m| m.is_safe_zone(state.x, state.y))
                                .unwrap_or(false);
                            if !in_safe {
                                results.push((*session_id, state.x, state.y, state.object_id));
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
            // 预收集怪物当前位置（用于碰撞检测）
            let monster_positions: HashSet<(i32, i32)> = self.monsters.values().map(|m| (m.x, m.y)).collect();

            for (oid, monster) in &mut self.monsters {
                let profile = &monster.ai_profile;

                // 找最近玩家（在视野范围内）
                let mut nearest: Option<(u64, i32, i32, i32)> = None;
                for (session, px, py, _) in &player_positions {
                    let dist = (monster.x - px).abs() + (monster.y - py).abs();
                    if dist <= profile.aggro_range {
                        if nearest.is_none_or(|n| dist < n.3) {
                            nearest = Some((*session, *px, *py, dist));
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
                        // 伤害
                        if let Some(record) = self.players.get(&target_session) {
                            if record.actor_ref.ask(TakeDamage {
                                attacker_id: monster.object_id,
                                attacker_session: target_session,
                                damage,
                            }).await.unwrap_or(false) {
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
                                }
                            }
                        }
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
                                            amount: xp_per,
                                        }).await;
                                    }
                                }
                                debug!("GroupXP: {} members split {} xp ({} each) from '{}'", group_sessions.len(), monster.xp, xp_per, monster.name);
                            }
                        } else if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::AddExperience {
                                amount: monster.xp,
                            }).await;
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
                    };
                    self.respawn_queue.insert(*oid, (spawn, respawn_tick));
                }
            }

            // 处理玩家死亡掉落（在怪物循环外，避免借用冲突）
            for (sid, x, y, map_index) in death_drops {
                self.handle_player_death_drop(sid, x, y, map_index).await;
            }
        }

        // --- 玩家 Buff tick + 死亡复活（每 5 ticks 执行一次） ---
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

        // --- PK 值衰减 + 名字颜色广播（每 10 ticks） ---
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

        // --- 地面物品过期清理（每 50 ticks ≈ 5 秒清理一次） ---
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

        // --- 重生处理 ---
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
                map_index: 0,
                next_attack_tick: 0,
                next_move_tick: 0,
                ai_profile,
                ai_state: MonsterAiState::Idle,
                target_session: None,
                provoked: false,
            });
            debug!("Monster '{}' respawned as #{}", spawn.name, new_oid);
        }

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

        // --- 定期自动保存（每 300 ticks = 30 秒 @ 100ms） ---
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
                allow_lover_recall: false,
                is_gm: false,
                pk_points: 0,
                pk_kill_count: 0,
                buffs: Vec::new(),
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

        let _ = player_ref.ask(SetPlayerState { state: loaded_state.clone() });

        self.players.insert(msg.session_id, PlayerRecord {
            actor_ref: player_ref,
            session_id: msg.session_id,
            name: player_name.clone(),
            account_username: msg.account_username.clone(),
            last_pk_points: loaded_state.pk_points,
        });

        info!("Player {} entered world (object_id={}, session={})",
              player_name, object_id, msg.session_id);

        // 行会在线状态由 SocialActor 管理

        // 多玩家可见性：向新玩家发送已有玩家的 ObjectPlayer
        let existing_players: Vec<_> = self.players.values()
            .filter(|r| r.session_id != msg.session_id)
            .cloned()
            .collect();

        for existing in &existing_players {
            if let Ok(Some(ep_state)) = existing.actor_ref.ask(GetPlayerState).await {
                let packet = build_object_player_packet(
                    &ep_state.name, ep_state.object_id, ep_state.x, ep_state.y, ep_state.direction, 1,
                    name_colour_for_pk(ep_state.pk_points),
                );
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                });
            }
        }

        // 向已有玩家发送新玩家的 ObjectPlayer
        let new_player_packet = build_object_player_packet(
            &player_name, object_id, loaded_state.x, loaded_state.y, loaded_state.direction, 1,
            name_colour_for_pk(loaded_state.pk_points),
        );
        for existing in &existing_players {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: existing.session_id,
                data: new_player_packet.clone(),
            });
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
            msg.session_id,
            &mut self.next_object_id,
            &spawn_ctx,
        );
        for npc in new_npcs {
            self.npcs.insert(npc.object_id, npc);
        }
        for monster in new_monsters {
            self.monsters.insert(monster.object_id, monster);
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
                    // 命中怪物 - 使用战斗模块计算伤害（从 PlayerState 读取真实属性）
                    let attack_result = combat_attack::resolve_attack(
                        state.min_attack, state.max_attack, 0
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

                            // 使用战斗模块计算伤害（从 PlayerState 读取真实属性，包含目标防御）
                            let attack_result = combat_attack::resolve_attack(
                                state.min_attack, state.max_attack, other_state.defence
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

        // 延迟发送 ObjectHarvested（采集完成）
        let gate_ref = self.gate_ref.clone();
        let object_id = state.object_id;
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
            send_system_message(&gate_ref, msg.session_id, "采集成功");
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

        // 获取玩家名称
        let player_name = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            state.name
        } else {
            return;
        };

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
                let _ = &npc_db.class_required;
                if let Some(ref dow) = npc_db.day_of_week {
                    let today = chrono::Utc::now().format("%A").to_string();
                    let today_short = &today[..3];
                    if !dow.is_empty() && !dow.contains(&today) && !dow.contains(today_short) {
                        debug!("NPC {} not available on {}", npc.name, today);
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
                        lines
                    }
                    "[@Buy]" => {
                        self.send_npc_goods(msg.session_id, &npc);
                        return;
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
        let pickup_idx = self.ground_items.iter().position(|gi| {
            gi.map_index == state.map_index
                && (gi.x - player_pos.0).abs() <= 1
                && (gi.y - player_pos.1).abs() <= 1
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
        let item_info = record.actor_ref.ask(GetItemInfo { unique_id: msg.unique_id }).await.unwrap_or(None);
        match item_info {
            Some(_item) => {
                // 消耗品：扣减 count 或移除
                let consumed = record.actor_ref.ask(ConsumeItem { unique_id: msg.unique_id }).await.unwrap_or(false);
                if consumed {
                    debug!("Player session={} used item uid={}", msg.session_id, msg.unique_id);
                    // 发送 UseItem 响应
                    send_use_item_response(&self.gate_ref, msg.session_id, msg.unique_id);
                }
            }
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            }
        }
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

        let goods_list = self.npc_goods.get(&npc_db_index).cloned().unwrap_or_default();
        let good = match goods_list.iter().find(|g| g.item_index == msg.item_index as i32) {
            Some(g) => g,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该 NPC 不出售此物品");
                return;
            }
        };

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

        // Create item from DB template
        let item = mir2_shared::data::item::UserItem {
            item_index: msg.item_index as i32,
            count: msg.count as u16,
            max_dura: item_db.durability as u16,
            current_dura: item_db.durability as u16,
            ..Default::default()
        };

        let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
        send_system_message(&self.gate_ref, msg.session_id, &format!("购买成功 (花费 {} 金币)", total_price));
        let npc_name = self.npcs.get(&msg.npc_id).map(|n| n.name.as_str()).unwrap_or("?");
        debug!("BuyItem: {} bought item={} ({}) x{} for {} gold from NPC '{}'", state.name, item_db.name, msg.item_index, msg.count, total_price, npc_name);
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
    pub slot: u8,
}

impl Message<EquipSlotItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: EquipSlotItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // slot 对应 EquipmentSlot 索引
        let equip_slot = match crate::actors::inventory::EquipmentSlot::from_i32(msg.slot as i32) {
            Some(s) => s,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "无效装备槽");
                return;
            }
        };

        // 查找该装备槽对应的物品 unique_id（从快捷栏映射）
        // 简化：假设快捷栏 slot 对应背包中的某个格子，实际应从玩家快捷栏配置读取
        // 这里取背包中第一个匹配该装备类型的物品
        if state.inventory.get_equipment(equip_slot).is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "该装备槽已有物品");
            return;
        }

        // 从背包找第一个物品并装备（简化实现）
        let found = state.inventory.backpack.iter()
            .find_map(|s| s.as_ref().map(|slot| (slot.grid, slot.item.clone())));

        let Some((grid, _item)) = found else {
            send_system_message(&self.gate_ref, msg.session_id, "背包中没有可装备的物品");
            return;
        };

        // 执行装备
        let result = record.actor_ref.ask(crate::actors::player::InventoryEquipItem { grid, slot: equip_slot }).await.unwrap_or(None);
        if result.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "装备成功");
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
        let quest = make_quest_instance(&quest_db);
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
                amount: completed_quest.exp_reward as i32,
            }).await;
        }
        if completed_quest.gold_reward > 0 {
            let _ = record.actor_ref.ask(AddGold { amount: completed_quest.gold_reward }).await;
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

        for monster_id in hit_monster_ids {
            if let Some(monster) = self.monsters.get_mut(&monster_id) {
                let attack_result = combat_attack::resolve_attack(
                    state.min_attack, state.max_attack, 0
                );
                let damage = attack_result.damage;
                monster.hp = monster.hp.saturating_sub(damage);
                monster.provoked = true;
                monster.target_session = Some(msg.session_id);
                debug!("RangeAttack: {} -> monster {} for {} damage", state.name, monster_id, damage);
                if monster.hp <= 0 {
                    // 死亡由 Tick 循环处理（广播 ObjectDied + 重生）
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

        // Validate spell exists in DB
        let spell_db = self.magic_infos.get(&(msg.spell as u32));
        let spell_range = spell_db.map(|m| m.range as i32).unwrap_or(2);
        let power = spell_db.map(|m| m.power_base).unwrap_or(10);

        let object_id = state.object_id;
        let target_x = msg.target_x;
        let target_y = msg.target_y;

        // 广播 ObjectAttack（带 spell type）
        let others: Vec<_> = self.other_players(msg.session_id)
            .into_iter().cloned()
            .collect();
        for other in &others {
            let mut body = Vec::new();
            body.extend_from_slice(&object_id.to_le_bytes());
            body.push(msg.direction);
            body.push(msg.spell);
            let _ = self.gate_ref.ask(SendToClient {
                session_id: other.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &body),
            });
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

// ---------- 物品合成/回购 ----------

/// 合成物品请求
pub struct CraftItemRequest {
    pub session_id: u64,
    pub recipe_id: u32,
}

impl Message<CraftItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CraftItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // 简化实现：发送合成成功确认
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let mut body = Vec::new();
        body.extend_from_slice(&msg.recipe_id.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes()); // count
        body.push(1u8); // success = true
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CraftItem as i16, &body),
        });

        debug!("CraftItem: {} recipe={}", state.name, msg.recipe_id);
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
        // 简化：发送回购成功确认
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        send_system_message(&self.gate_ref, msg.session_id, "回购成功");
        debug!("BuyItemBack: {} item_index={}", state.name, msg.item_index);
    }
}

// ---------- 角色管理 ----------

/// 修改密码请求
pub struct ChangePasswordRequest {
    pub session_id: u64,
    pub new_password: String,
}

impl Message<ChangePasswordRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ChangePasswordRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // 密码存储在 AccountActor 中，此路径仅在 AccountActor 不可用时触发
        send_system_message(&self.gate_ref, msg.session_id, "密码修改服务暂时不可用，请稍后重试");
        warn!("ChangePassword via WorldActor fallback: session={}", msg.session_id);
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
            allow_lover_recall: false,
            is_gm: false,
            pk_points: 0,
            pk_kill_count: 0,
            buffs: Vec::new(),
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
        }
        debug!("LockMail: {} mail_id={} lock={}", state.name, msg.mail_id, msg.lock);
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
        }
        debug!("MailLockedItem: {} mail_id={} item_index={}", state.name, msg.mail_id, msg.item_index);
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

        debug!("CombineItem: {} from={} to={}", state.name, msg.from_grid, msg.to_grid);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CombineItem as i16, &[0u8]),
        });
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

        debug!("DisassembleItem: {} uid={}", state.name, msg.unique_id);
        send_system_message(&self.gate_ref, msg.session_id, "该物品无法分解");
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

        debug!("ResetAddedItem: {} uid={}", state.name, msg.unique_id);
        debug!("ResetAddedItem: {} uid={} (feature not implemented)", state.name, msg.unique_id);
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
            }
        }

        debug!("Mail sent from {} to {} (gold={}, items={})", sender_state.name, msg.receiver_name, total_gold, mail.items.len());
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
// 市场/寄售系统（返回空列表）
// ============================================================

pub struct MarketSearchRequest {
    pub session_id: u64,
    pub item_index: u32,
}

impl Message<MarketSearchRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketSearchRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketSearch: session={} item={}", msg.session_id, msg.item_index);
        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MarketSuccess as i16, &body),
        });
    }
}

pub struct MarketRefreshRequest {
    pub session_id: u64,
}

impl Message<MarketRefreshRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketRefreshRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketRefresh: session={}", msg.session_id);
        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarket as i16, &body),
        });
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
        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes());
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
        send_system_message(&self.gate_ref, msg.session_id, "该商品已下架");
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
        send_system_message(&self.gate_ref, msg.session_id, "无法取回寄售物品");
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
        debug!("MarketSellNow: session={} uid={} price={} (consignment not implemented)", msg.session_id, msg.unique_id, msg.price);
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
        debug!("ConsignItem: session={} uid={} price={} (consignment not implemented)", msg.session_id, msg.unique_id, msg.price);
    }
}

// ============================================================
// 物品租赁系统
// ============================================================

pub struct ItemRentalRequestMsg {
    pub session_id: u64,
    pub target_name: String,
}

impl Message<ItemRentalRequestMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalRequestMsg, _ctx: &mut Context<Self, Self::Reply>) {
        // ItemRentalRequestMsg carries target_name but we don't have name→session_id lookup yet.
        // The full rental system is not implemented; this handler exists for future extension.
        debug!("ItemRentalRequest: session={} target={} (rental not implemented)", msg.session_id, msg.target_name);
    }
}

pub struct DepositRentalItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<DepositRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: DepositRentalItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("DepositRentalItem: session={} uid={} (rental not implemented)", msg.session_id, msg.unique_id);
    }
}

pub struct RetrieveRentalItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<RetrieveRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: RetrieveRentalItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("RetrieveRentalItem: session={} uid={} (rental not implemented)", msg.session_id, msg.unique_id);
    }
}

pub struct CancelItemRentalRequest {
    pub session_id: u64,
}

impl Message<CancelItemRentalRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: CancelItemRentalRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("CancelItemRental: session={} (rental not implemented)", msg.session_id);
    }
}

pub struct ItemRentalFeeMsg {
    pub session_id: u64,
    pub amount: u32,
}

impl Message<ItemRentalFeeMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalFeeMsg, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("ItemRentalFee: session={} amount={} (rental not implemented)", msg.session_id, msg.amount);
    }
}

pub struct ItemRentalPeriodMsg {
    pub session_id: u64,
    pub duration: u32,
}

impl Message<ItemRentalPeriodMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalPeriodMsg, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("ItemRentalPeriod: session={} duration={} (rental not implemented)", msg.session_id, msg.duration);
    }
}

pub struct ItemRentalLockFeeMsg {
    pub session_id: u64,
}

impl Message<ItemRentalLockFeeMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalLockFeeMsg, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("ItemRentalLockFee: session={} (rental not implemented)", msg.session_id);
    }
}

pub struct ItemRentalLockItemMsg {
    pub session_id: u64,
}

impl Message<ItemRentalLockItemMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalLockItemMsg, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("ItemRentalLockItem: session={} (rental not implemented)", msg.session_id);
    }
}

pub struct ConfirmItemRentalMsg {
    pub session_id: u64,
}

impl Message<ConfirmItemRentalMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ConfirmItemRentalMsg, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("ConfirmItemRental: session={} (rental not implemented)", msg.session_id);
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

        // Guild war mechanics not yet implemented; respond with acknowledgment
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
                            let quest = make_quest_instance(quest_db);
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
                            let _ = record.actor_ref.ask(AddExperience { amount: quest_db.exp_reward }).await;
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
        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Rankings as i16, &body),
        });
    }
}

// ============================================================
// 辅助函数
// ============================================================

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

/// 从 DB 任务配置创建任务实例
fn make_quest_instance(qi: &db::QuestInfo) -> QuestInstance {
    QuestInstance {
        quest_index: qi.index,
        title: qi.name.clone(),
        status: QuestStatus::InProgress,
        progress: vec![],
        exp_reward: qi.exp_reward as i64,
        gold_reward: qi.gold_reward.max(0) as u64,
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
    write_dotnet_string(&mut body, ""); // guild_name (简化)
    body.extend_from_slice(&state.level.to_le_bytes());
    body.push(0u8); // class=Warrior (简化)
    body.push(0u8); // gender=Male (简化)
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
    body.push(0u8);                                           // class=Warrior
    body.push(0u8);                                           // gender=Male
    body.extend_from_slice(&state.level.to_le_bytes());       // level
    body.extend_from_slice(&state.x.to_le_bytes());           // location_x
    body.extend_from_slice(&state.y.to_le_bytes());           // location_y
    body.push(state.direction);                               // direction
    body.push(0u8);                                           // hair
    body.extend_from_slice(&state.hp.to_le_bytes());          // hp
    body.extend_from_slice(&state.mp.to_le_bytes());          // mp
    body.extend_from_slice(&state.experience.to_le_bytes());  // experience
    body.extend_from_slice(&state.max_experience.to_le_bytes()); // max_experience
    body.extend_from_slice(&0u16.to_le_bytes());              // level_effects
    body.push(0u8);                                           // has_hero=false
    body.push(0u8);                                           // hero_behaviour=None

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
) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&object_id.to_le_bytes());   // object_id
    write_dotnet_string(&mut body, name);               // name
    write_dotnet_string(&mut body, "");                 // guild_name
    write_dotnet_string(&mut body, "");                 // guild_rank_name
    body.extend_from_slice(&name_colour.to_le_bytes()); // name_colour
    body.push(0u8);                                     // class=Warrior
    body.push(0u8);                                     // gender=Male
    body.extend_from_slice(&level.to_le_bytes());       // level
    body.extend_from_slice(&x.to_le_bytes());           // location_x
    body.extend_from_slice(&y.to_le_bytes());           // location_y
    body.push(direction);                               // direction
    body.push(0u8);                                     // hair
    body.push(1u8);                                     // light
    body.extend_from_slice(&0i16.to_le_bytes());        // weapon
    body.extend_from_slice(&0i16.to_le_bytes());        // weapon_effect
    body.extend_from_slice(&0i16.to_le_bytes());        // armour
    body.extend_from_slice(&0u16.to_le_bytes());        // poison=None (client reads u16)
    body.push(0u8);                                     // dead=false
    body.push(0u8);                                     // hidden=false
    body.push(0u8);                                     // effect=None
    body.push(0u8);                                     // wing_effect
    body.push(0u8);                                     // extra=false
    body.extend_from_slice(&0i16.to_le_bytes());        // mount_type
    body.push(0u8);                                     // riding_mount=false
    body.push(0u8);                                     // fishing=false
    body.extend_from_slice(&0i16.to_le_bytes());        // transform_type
    body.extend_from_slice(&0u32.to_le_bytes());        // element_orb_effect
    body.extend_from_slice(&0u32.to_le_bytes());        // element_orb_lvl
    body.extend_from_slice(&0u32.to_le_bytes());        // element_orb_max
    body.extend_from_slice(&0i32.to_le_bytes());        // buffs count=0
    body.extend_from_slice(&0u16.to_le_bytes());        // level_effects=None (client reads u16)

    build_packet_bytes(ServerPacketIds::ObjectPlayer as i16, &body)
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
    session_id: u64,
    next_object_id: &mut u32,
    ctx: &SpawnContext<'_>,
) -> (Vec<NpcState>, Vec<MonsterState>) {
    // Try DB-loaded configs first, fall back to TOML
    let config = if let Some(mi) = ctx.map_info {
        spawn_config_from_db(mi, ctx.monster_infos, ctx.npc_infos)
    } else if let Some(d) = spawn_dir {
        load_spawn_config(map_file, d)
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
        });
    }

    // 发送怪物并创建运行时状态
    let mut monsters = Vec::new();
    for monster in &config.monsters {
        let object_id = *next_object_id;
        *next_object_id += 1;
        let packet = build_object_monster_packet(monster, object_id, &monster.name);
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
            name: monster.name.clone(),
            image: monster.image,
            monster_index: monster.monster_index,
            x: monster.x,
            y: monster.y,
            direction: monster.direction,
            hp: monster.hp,
            max_hp: monster.hp,
            min_dmg: monster.min_dmg,
            max_dmg: monster.max_dmg,
            xp: monster.xp,
            spawn_x: monster.x,
            spawn_y: monster.y,
            map_index: 0,
            next_attack_tick: 0,
            next_move_tick: 0,
            ai_profile,
            ai_state: MonsterAiState::Idle,
            target_session: None,
            provoked: false,
        });
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
                        &MonsterSpawn { name: dragon.monster_name.clone(), image: monster_db.image as u16, monster_index, x: dragon.location_x, y: dragon.location_y, direction: 0, hp, min_dmg, max_dmg, xp },
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
                        map_index: 0,
                        next_attack_tick: 0,
                        next_move_tick: 0,
                        ai_profile,
                        ai_state: MonsterAiState::Idle,
                        target_session: None,
                        provoked: false,
                    });
                    info!("Spawned dragon at ({}, {}) on map {}", dragon.location_x, dragon.location_y, map_file);
                }
            }
        }
    }

    (npcs, monsters)
}
