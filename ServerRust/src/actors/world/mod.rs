// WorldActor - 游戏世界主循环
// 对应 C# GameSrv/WorldServer.cs + M2Server 核心逻辑

// 子模块（已集成）
mod awakening;
mod combat;
pub mod ai;
#[allow(dead_code)]
mod conquest;
#[allow(dead_code)]
mod dragon;
mod guild;
mod hero;
mod item;
mod mail;
mod market;
mod npc;
mod npc_script;
mod quest;
mod report;
#[allow(dead_code)]
mod robot;
mod session;
pub mod spell;
mod tick;

// Re-export submodule structs for external access
pub use tick::Tick;
pub use tick::ProcessDelayedActions;
pub use session::*;
pub use item::*;
pub use combat::*;
pub use awakening::*;
pub use market::*;
pub use mail::*;
pub use quest::*;
pub use hero::*;
pub use guild::*;
pub use npc::*;

// Re-exports for submodules (use super::*)
pub use std::collections::{HashMap, HashSet};
pub use std::path::{Path, PathBuf};
pub use kameo::actor::{Actor, ActorRef, Spawn};
pub use kameo::prelude::Context;
pub use kameo::message::Message;
pub use tokio::time::{interval, Duration};
pub use tracing::{info, debug, warn};
pub use chrono::Timelike;
pub use crate::actors::player::*;
pub use crate::actors::inventory::{EquipmentSlot, GroundItem, PlayerInventory, generate_item_uid};
pub use crate::actors::refine::{RefineStatus, RefineLog};
pub use crate::actors::friend::FriendList;
pub use crate::actors::mail::{MailMessage, Mailbox, generate_mail_id};
pub use crate::actors::guild::GuildRank;
pub use crate::actors::quest::{QuestInstance, QuestProgress, QuestStatus, QuestLog};
pub use crate::actors::creature::{IntelligentCreature, CreatureType, PickupMode, CreatureLog};
pub use crate::combat::attack::{self as combat_attack};
pub use crate::combat::buff;
pub use crate::db::{self, DbPool};
pub use crate::gate::actor::{SendToClient, GateActor};
pub use crate::actors::social::{SocialActor, SocialChatCommand};
pub use mir2_shared::packets::Packet;
pub use mir2_shared;
pub use crate::maps::loader::{self, MapData};
pub use crate::util::wire::{build_packet_bytes, write_dotnet_string};

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
    /// 攻城/GT 配置（对齐 C# Settings.BuyGTGold/ExtendGT）
    pub conquest_cfg: crate::util::config::ConquestConfig,
    /// 休息经验加成配置（C# Settings.Rested*）
    pub rested_cfg: crate::util::config::RestedConfig,
    /// 全局掉落倍率（C# Settings.DropRate）
    pub drop_rate: f64,
    /// 地面物品超时 ticks（= item_timeout_secs * 10，100ms/tick）
    pub item_timeout_ticks: u64,
    /// 金币掉落每堆上限（C# Settings.MaxDropGold = 2000）
    pub max_drop_gold: u32,
    /// 精英怪配置（C# Settings.MonsterRarity*）
    pub rarity_cfg: crate::util::config::RarityConfig,
    /// 服务器公告文件路径（C# Settings.NoticePath）
    pub notice_path: String,
    /// 死亡经验惩罚百分比（默认 0=关闭，对齐 C#）
    pub death_exp_penalty_percent: u32,
    /// 回血权重（C# Settings.HealthRegenWeight）
    pub health_regen_weight: u32,
    /// 回蓝权重（C# Settings.ManaRegenWeight）
    pub mana_regen_weight: u32,
    /// 商店隐藏附加属性（C# Settings.GoodsHideAddedStats）
    pub goods_hide_added_stats: bool,
}

/// 世界中的玩家记录
#[derive(Clone)]
pub(crate) struct PlayerRecord {
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
    /// 是否已下发世界地图配置（C# WorldMapSetupSent，每连接一次）
    world_map_setup_sent: bool,
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

/// NPC 延迟执行动作（TIMERECALL/DELAYGOTO，对齐 C# DelayedAction DelayedType.NPC）
#[derive(Debug, Clone)]
pub struct DelayedNpcAction {
    /// 到期 tick（100ms/tick）
    pub expire_tick: u64,
    /// 目标 NPC object_id（脚本来源；execute_section 需要 npc 上下文）
    pub npc_object_id: u32,
    /// 目标 section 名（缺省 main）
    pub section: String,
    /// CALL 目标：直接指定脚本 db_index（覆盖 npc_object_id 查到的 db_index）
    pub target_db_index: Option<i32>,
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
                        db_index: n.db_index,
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
    rarity: crate::util::config::RarityConfig,
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
    /// NPC 数据库索引（对应 npc_infos，商店商品/脚本用；TOML 刷怪配置可选）
    #[serde(default)]
    db_index: i32,
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
pub enum MonsterAiType {
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
pub struct MonsterAiProfile {
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
pub enum MonsterAiState {
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
pub struct MonsterState {
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
    // ===== 战斗公式扩展字段（对齐 C# MonsterObject 的 Stats）=====
    pub min_ac: i32,
    pub max_ac: i32,
    pub min_mac: i32,
    pub max_mac: i32,
    pub agility: i32,
    pub accuracy: i32,
    /// 护甲倍率（C# ArmourRate，默认 1.0）
    pub armour_rate: f32,
    /// 伤害倍率（C# DamageRate，默认 1.0）
    pub damage_rate: f32,
    pub magic_resist: i32,
    pub critical_rate: i32,
    pub critical_damage: i32,
    pub luck: i32,
    pub reflect: i32,
    pub damage_reduction_percent: i32,
    /// 运行时中毒/负面状态列表
    pub poison_list: Vec<crate::combat::poison::Poison>,
    /// 是否为亡灵类型（ThunderBolt +50%、TurnUndead 秒杀用，C# MonsterInfo.Undead）
    pub undead: bool,
    /// 主人 session（None=普通怪，Some=召唤物/奴仆）
    pub master_session: Option<u64>,
    /// 召唤物到期 tick（0=永不过期；>0 时到点自动消失，对齐 C# 召唤时限）
    pub recall_at_tick: u64,
    /// AI 行为（Boss=专属 impl，普通怪=DefaultBehavior）
    pub behavior: Box<dyn crate::actors::world::ai::MonsterBehavior + Send + Sync>,
}

fn dist_to_spawn(monster: &MonsterState) -> i32 {
    (monster.x - monster.spawn_x).abs() + (monster.y - monster.spawn_y).abs()
}

/// 运行时 NPC 状态
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct NpcState {
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

// 魔法 spell ID 常量。值必须与 SharedRust `Spell` 枚举一致（DB 存的是枚举值）。
// 早期版本这里写的是 C# 原数值（系统性偏小），与 SharedRust 枚举/DB 不一致，
// 导致持久法术判定、buff/heal 分支、怪物施法等全部错位。现统一取自 Spell 枚举。
const SPELL_HEALING: u8 = mir2_shared::enums::Spell::Healing as u8;          // 64
const SPELL_MASS_HEALING: u8 = mir2_shared::enums::Spell::MassHealing as u8; // 78
const SPELL_HEALING_CIRCLE: u8 = mir2_shared::enums::Spell::HealingCircle as u8; // 89
const SPELL_MAGIC_SHIELD: u8 = mir2_shared::enums::Spell::MagicShield as u8; // 46
const SPELL_SOUL_SHIELD: u8 = mir2_shared::enums::Spell::SoulShield as u8;   // 72
const SPELL_BLESSED_ARMOUR: u8 = mir2_shared::enums::Spell::BlessedArmour as u8; // 74
const SPELL_TELEPORT: u8 = mir2_shared::enums::Spell::Teleport as u8;        // 40
const SPELL_BLINK: u8 = mir2_shared::enums::Spell::Blink as u8;              // 57
const SPELL_FIREBALL: u8 = mir2_shared::enums::Spell::FireBall as u8;        // 34，法师怪物默认法术
const SPELL_FIREWALL: u8 = mir2_shared::enums::Spell::FireWall as u8;        // 42
const SPELL_BLIZZARD: u8 = mir2_shared::enums::Spell::Blizzard as u8;        // 53
const SPELL_METEOR_STRIKE: u8 = mir2_shared::enums::Spell::MeteorStrike as u8; // 55
const SPELL_POISON_CLOUD: u8 = mir2_shared::enums::Spell::PoisonCloud as u8; // 86
const SPELL_EXPLOSIVE_TRAP: u8 = mir2_shared::enums::Spell::ExplosiveTrap as u8; // 127
// 弹道类法术（任务3）
const SPELL_GREAT_FIREBALL: u8 = mir2_shared::enums::Spell::GreatFireBall as u8; // 37
const SPELL_THUNDERBOLT: u8 = mir2_shared::enums::Spell::ThunderBolt as u8;    // 39
const SPELL_FROST_CRUNCH: u8 = mir2_shared::enums::Spell::FrostCrunch as u8;   // 44
const SPELL_VAMPIRISM: u8 = mir2_shared::enums::Spell::Vampirism as u8;
const SPELL_SOUL_FIREBALL: u8 = mir2_shared::enums::Spell::SoulFireBall as u8; // 64 道士·灵魂火球（MAC 弹道）        // 48
const SPELL_METEOR_SHOWER: u8 = mir2_shared::enums::Spell::MeteorShower as u8; // 158 法师·流星火雨（弹道+副目标半伤）
const SPELL_FIRE_BOUNCE: u8 = mir2_shared::enums::Spell::FireBounce as u8;     // 157 法师·火焰弹跳（链式弹射）
// 即时 AoE 类法术（任务4）
const SPELL_FIREBANG: u8 = mir2_shared::enums::Spell::FireBang as u8;          // 41
const SPELL_ICE_STORM: u8 = mir2_shared::enums::Spell::IceStorm as u8;         // 49
const SPELL_LIGHTNING: u8 = mir2_shared::enums::Spell::Lightning as u8;        // 43
const SPELL_THUNDERSTORM: u8 = mir2_shared::enums::Spell::ThunderStorm as u8;  // 45
const SPELL_FLAME_FIELD: u8 = mir2_shared::enums::Spell::FlameField as u8;     // 52
// 道士法术
const SPELL_POISONING: u8 = mir2_shared::enums::Spell::Poisoning as u8;        // 66
const SPELL_HIDING: u8 = mir2_shared::enums::Spell::Hiding as u8;              // 70
const SPELL_MASS_HIDING: u8 = mir2_shared::enums::Spell::MassHiding as u8;     // 71
const SPELL_TRAP_HEXAGON: u8 = mir2_shared::enums::Spell::TrapHexagon as u8;   // 76
const SPELL_PURIFICATION: u8 = mir2_shared::enums::Spell::Purification as u8;  // 77
// 战士近战技能（被动触发于攻击时）
const SPELL_SLAYING: u8 = mir2_shared::enums::Spell::Slaying as u8;            // 5 攻杀
const SPELL_THRUSTING: u8 = mir2_shared::enums::Spell::Thrusting as u8;        // 6 刺杀（直线穿透）
const SPELL_HALFMOON: u8 = mir2_shared::enums::Spell::HalfMoon as u8;          // 7 半月（范围）
const SPELL_SHOULDER_DASH: u8 = mir2_shared::enums::Spell::ShoulderDash as u8; // 8 野蛮冲撞
const SPELL_CROSS_HALFMOON: u8 = mir2_shared::enums::Spell::CrossHalfMoon as u8; // 13 十字半月
const SPELL_FIRE_BURST: u8 = mir2_shared::enums::Spell::FireBurst as u8;             // 97 刺客·火焰爆发（同 Repulsion）
const SPELL_TRAP: u8 = mir2_shared::enums::Spell::Trap as u8;                       // 98 刺客·陷阱（目标 60s 麻痹）
const SPELL_FLAMING_SWORD: u8 = mir2_shared::enums::Spell::FlamingSword as u8;     // 8 战士·烈焰剑（下一次近战附加火焰加成）
const SPELL_TWIN_DRAKE_BLADE: u8 = mir2_shared::enums::Spell::TwinDrakeBlade as u8;   // 6 战士·双龙斩（下一次近战双段伤害）
const SPELL_SLASHING_BURST: u8 = mir2_shared::enums::Spell::SlashingBurst as u8;     // 15 战士·横扫千军（冲锋+伤害）
const SPELL_BLADE_AVALANCHE: u8 = mir2_shared::enums::Spell::BladeAvalanche as u8; // 14 冰刀斩（3列×3行前向 AoE）
// 弓箭手法术（Archer，弹道物理系 + 自身 buff）
const SPELL_STRAIGHT_SHOT: u8 = mir2_shared::enums::Spell::StraightShot as u8;   // 125 直线弹道
const SPELL_DOUBLE_SHOT: u8 = mir2_shared::enums::Spell::DoubleShot as u8;       // 126 双发弹道
const SPELL_CONCENTRATION: u8 = mir2_shared::enums::Spell::Concentration as u8;  // 132 魔力恢复 buff
const SPELL_ELEMENTAL_BARRIER: u8 = mir2_shared::enums::Spell::ElementalBarrier as u8; // 134 元素护盾（减伤）
const SPELL_BINDING_SHOT: u8 = mir2_shared::enums::Spell::BindingShot as u8;     // 143 定身射击（弹道+Paralysis）
const SPELL_NAPALM_SHOT: u8 = mir2_shared::enums::Spell::NapalmShot as u8;       // 141 范围爆炸（弹道+AOE）
const SPELL_MIRRORING: u8 = mir2_shared::enums::Spell::Mirroring as u8;          // 51 分身/反伤 buff
// 刺客法术（Assassin，buff 系 + 位移系 + 物理攻击系）
const SPELL_HASTE: u8 = mir2_shared::enums::Spell::Haste as u8;                  // 96 攻击速度+
const SPELL_FLASH_DASH: u8 = mir2_shared::enums::Spell::FlashDash as u8;         // 97 突进
const SPELL_LIGHT_BODY: u8 = mir2_shared::enums::Spell::LightBody as u8;         // 98 敏捷+
const SPELL_HEAVENLY_SWORD: u8 = mir2_shared::enums::Spell::HeavenlySword as u8; // 99 直线AoE
const SPELL_DOUBLE_SLASH: u8 = mir2_shared::enums::Spell::DoubleSlash as u8;         // 92 刺客·双斩（下一次近战双段伤害）
const SPELL_MPEATER: u8 = mir2_shared::enums::Spell::MPEater as u8;                    // 101 刺客·MP吞噬（近战被动吸蓝）
const SPELL_HEMORRHAGE: u8 = mir2_shared::enums::Spell::Hemorrhage as u8;              // 104 刺客·放血（近战被动流血）
const SPELL_MOON_MIST: u8 = mir2_shared::enums::Spell::MoonMist as u8;                 // 106 刺客·月雾（隐身+范围伤害）
const SPELL_CAT_TONGUE: u8 = mir2_shared::enums::Spell::CatTongue as u8;               // 107 刺客·猫舌（DC 弹道）
const SPELL_ELEMENTAL_SHOT: u8 = mir2_shared::enums::Spell::ElementalShot as u8;       // 128 弓手·元素箭（DC 弹道）
const SPELL_ONE_WITH_NATURE: u8 = mir2_shared::enums::Spell::OneWithNature as u8;     // 139 弓手·与自然合一（5x5 AoE）
const SPELL_MENTAL_STATE: u8 = mir2_shared::enums::Spell::MentalState as u8;           // 141 弓手·精神状态（模式切换）
const SPELL_VAMPIRE_SHOT: u8 = mir2_shared::enums::Spell::VampireShot as u8;           // 133 弓手·吸血箭（弹道+吸血）
const SPELL_POISON_SHOT: u8 = mir2_shared::enums::Spell::PoisonShot as u8;             // 135 弓手·毒箭（弹道+绿毒）
const SPELL_CRIPPLE_SHOT: u8 = mir2_shared::enums::Spell::CrippleShot as u8;           // 136 弓手·减速箭（弹道+减速）
const SPELL_MOON_LIGHT: u8 = mir2_shared::enums::Spell::MoonLight as u8;         // 103 隐身
const SPELL_FATAL_SWORD: u8 = mir2_shared::enums::Spell::FatalSword as u8;           // 91 刺客·致命一击（被动：10% 触发，下一击 +5*(Lv+1)）
const SPELL_SWIFT_FEET: u8 = mir2_shared::enums::Spell::SwiftFeet as u8;         // 105 移动速度+
const SPELL_DARK_BODY: u8 = mir2_shared::enums::Spell::DarkBody as u8;           // 106 隐身+攻击
const SPELL_CRESCENT_SLASH: u8 = mir2_shared::enums::Spell::CrescentSlash as u8; // 108 扇形AoE
const SPELL_FURY: u8 = mir2_shared::enums::Spell::Fury as u8;                    // 19 攻击力+
const SPELL_RAGE: u8 = mir2_shared::enums::Spell::Rage as u8;                    // 16 暴击+
const SPELL_BACK_STEP: u8 = mir2_shared::enums::Spell::BackStep as u8;           // 130 后跳
const SPELL_DELAYED_EXPLOSION: u8 = mir2_shared::enums::Spell::DelayedExplosion as u8; // 125 弓手·定时爆炸（3s 引爆）

// 召唤系法术（在施法者附近 spawn 一只 MonsterState 作为战斗召唤物）
const SPELL_SUMMON_SKELETON: u8 = mir2_shared::enums::Spell::SummonSkeleton as u8; // 68 道士·召唤骷髅
const SPELL_SUMMON_SHINSU: u8 = mir2_shared::enums::Spell::SummonShinsu as u8;    // 81 道士·召唤神兽
const SPELL_SUMMON_HOLY_DEVA: u8 = mir2_shared::enums::Spell::SummonHolyDeva as u8; // 83 法师·召唤圣兽
const SPELL_SUMMON_VAMPIRE: u8 = mir2_shared::enums::Spell::SummonVampire as u8;  // 135 弓箭手·召唤血蝠
const SPELL_SUMMON_TOAD: u8 = mir2_shared::enums::Spell::SummonToad as u8;        // 137 弓箭手·召唤蟾蜍
const SPELL_SUMMON_SNAKES: u8 = mir2_shared::enums::Spell::SummonSnakes as u8;    // 140 弓箭手·召唤蛇
const SPELL_STONETRAP: u8 = mir2_shared::enums::Spell::Stonetrap as u8;           // 133 弓手·石阵（召唤 StoneTrap 宠物，持续 (Lv*5+10)s）

// ===== 特殊/辅助类法术（任务：补齐剩余主动法术）=====
// 战士系
const SPELL_LION_ROAR: u8 = mir2_shared::enums::Spell::LionRoar as u8;
const SPELL_BATTLE_CRY: u8 = mir2_shared::enums::Spell::BattleCry as u8; // 153 战士·战吼（同 LionRoar 嘲讽）            // 9 战士·嘲讽（范围内怪物仇恨转移）
const SPELL_PROTECTION_FIELD: u8 = mir2_shared::enums::Spell::ProtectionField as u8; // 12 战士·群体减伤
const SPELL_COUNTER_ATTACK: u8 = mir2_shared::enums::Spell::CounterAttack as u8;  // 14 战士/刺客·反击 buff
const SPELL_IMMORTAL_SKIN: u8 = mir2_shared::enums::Spell::ImmortalSkin as u8;         // 17 战士·不死之肤（AC 提升 buff）
const SPELL_ENTRAPMENT: u8 = mir2_shared::enums::Spell::Entrapment as u8;         // 7 战士·拉拽+麻痹
// 法师系
const SPELL_TURN_UNDEAD: u8 = mir2_shared::enums::Spell::TurnUndead as u8;        // 44 法师·秒杀低级亡灵
const SPELL_REPULSION: u8 = mir2_shared::enums::Spell::Repulsion as u8;           // 32 法师·推开周围怪物
const SPELL_ELECTRIC_SHOCK: u8 = mir2_shared::enums::Spell::ElectricShock as u8;  // 33 法师·驯服怪物
const SPELL_HELLFIRE: u8 = mir2_shared::enums::Spell::HellFire as u8;             // 35 法师·地狱火（三向直线 AoE）
const SPELL_ENERGY_SHIELD: u8 = mir2_shared::enums::Spell::EnergyShield as u8;    // 84 道士·能量盾（减伤 buff）
const SPELL_MEDITATION: u8 = mir2_shared::enums::Spell::Meditation as u8;         // 126 弓手·冥想（施法返还 MP 被动）
const SPELL_ICETHRUST: u8 = mir2_shared::enums::Spell::IceThrust as u8;           // 53 法师·冰刺（幸运暴击+溅射）
const SPELL_MAGIC_BOOSTER: u8 = mir2_shared::enums::Spell::MagicBooster as u8;    // 51 法师·MP 上限提升 buff
const SPELL_FLAME_DISRUPTOR: u8 = mir2_shared::enums::Spell::FlameDisruptor as u8;   // 47 法师·火焰干扰（非亡灵 ×1.5）
const SPELL_STORM_ESCAPE: u8 = mir2_shared::enums::Spell::StormEscape as u8;           // 55 法师·风遁（定点传送）
// 道士系
const SPELL_REVELATION: u8 = mir2_shared::enums::Spell::Revelation as u8;         // 70 道士·显血/反隐
const SPELL_REINCARNATION: u8 = mir2_shared::enums::Spell::Reincarnation as u8;   // 79 道士·复活死亡玩家
const SPELL_HALLUCINATION: u8 = mir2_shared::enums::Spell::Hallucination as u8;       // 76 道士·幻觉（怪物失目标不攻击）
const SPELL_ULTIMATE_ENHANCER: u8 = mir2_shared::enums::Spell::UltimateEnhancer as u8; // 77 道士·终极强化（DC/MC/SC 提升 buff）
const SPELL_PET_ENHANCER: u8 = mir2_shared::enums::Spell::PetEnhancer as u8;         // 85 道士·宠物强化（DC/AC 提升）
const SPELL_ENERGY_REPULSOR: u8 = mir2_shared::enums::Spell::EnergyRepulsor as u8; // 72 道士·气功波（同 Repulsion）
const SPELL_CURSE: u8 = mir2_shared::enums::Spell::Curse as u8;                   // 81 道士·诅咒（区域减攻+减速）
const SPELL_PLAGUE: u8 = mir2_shared::enums::Spell::Plague as u8;                 // 82 道士·瘟疫（3x3 随机毒+伤害）
// 刺客系
const SPELL_POISON_SWORD: u8 = mir2_shared::enums::Spell::PoisonSword as u8;      // 99 刺客·武器涂毒 buff

impl MonsterState {
    /// 受击：经 behavior.on_attacked 过滤后扣血。
    /// 返回实际扣除的血量（Boss 睡眠/免疫/无敌期返 0）。
    pub fn take_damage(&mut self, damage: i32) -> i32 {
        if damage <= 0 { return 0; }
        let mut behavior = std::mem::replace(&mut self.behavior,
            Box::new(crate::actors::world::ai::DefaultBehavior::new()));
        let actual = behavior.on_attacked(damage);
        self.behavior = behavior;
        self.hp = self.hp.saturating_sub(actual);
        actual
    }

    /// 从 MonsterInfo.stats 填充战斗属性（AC/MAC/Agility/Crit 等）
    /// 在构造 MonsterState 时调用，替代硬编码 0。
    pub fn fill_combat_stats(&mut self, info: &db::MonsterInfo) {
        use mir2_shared::enums::Stat;
        let get = |s: Stat| info.stats.get(&(s as u8)).copied().unwrap_or(0);
        self.min_ac = get(Stat::MinAC);
        self.max_ac = get(Stat::MaxAC);
        self.min_mac = get(Stat::MinMAC);
        self.max_mac = get(Stat::MaxMAC);
        self.agility = get(Stat::Agility);
        self.accuracy = get(Stat::Accuracy);
        self.magic_resist = get(Stat::MagicResist);
        self.critical_rate = get(Stat::CriticalRate);
        self.critical_damage = get(Stat::CriticalDamage);
        self.luck = get(Stat::Luck);
        self.reflect = get(Stat::Reflect);
        self.damage_reduction_percent = get(Stat::DamageReductionPercent);
    }

    /// 中毒：经 behavior.on_poison 过滤。
    pub fn try_apply_poison(&mut self, poison: crate::combat::poison::Poison) {
        let mut behavior = std::mem::replace(&mut self.behavior,
            Box::new(crate::actors::world::ai::DefaultBehavior::new()));
        if behavior.on_poison(poison) {
            crate::combat::poison::apply_poison(&mut self.poison_list, poison);
        }
        self.behavior = behavior;
    }

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

    /// 构建战斗公式用的属性快照
    pub fn to_combat_stats(&self) -> crate::combat::attack::CombatStats {
        use crate::combat::attack::CombatStats;
        // C# ProcessPoison：红毒降防（-0.5）/ 眩晕增伤（+0.5）
        let (mut armour_rate, mut damage_rate) = (self.armour_rate, self.damage_rate);
        for p in &self.poison_list {
            if p.p_type.intersects(mir2_shared::enums::PoisonType::RED) {
                armour_rate -= 0.5;
            }
            if p.p_type.intersects(mir2_shared::enums::PoisonType::STUN) {
                damage_rate += 0.5;
            }
        }
        CombatStats {
            min_atk: self.min_dmg,
            max_atk: self.max_dmg,
            min_ac: self.min_ac,
            max_ac: self.max_ac,
            min_mac: self.min_mac,
            max_mac: self.max_mac,
            agility: self.agility,
            accuracy: self.accuracy,
            luck: self.luck,
            critical_rate: self.critical_rate,
            critical_damage: self.critical_damage,
            magic_resist: self.magic_resist,
            reflect: self.reflect,
            damage_reduction_percent: self.damage_reduction_percent,
            armour_rate,
            damage_rate,
            ..Default::default()
        }
    }
}

/// 商店回购条目
#[derive(Debug, Clone)]
pub struct BuybackItem {
    pub item: mir2_shared::data::item::UserItem,
    pub sell_price: u64,
    /// 回购过期时间（Unix 毫秒；C# Settings.GoodsBuyBackTime = 60 分钟）
    pub expires_at: i64,
}

/// WorldActor 状态
pub struct WorldActor {
    /// Tick 计数器
    pub(crate) tick_count: u64,
    /// 在线玩家 Actor 引用（按 session_id 索引）
    pub(crate) players: HashMap<u64, PlayerRecord>,
    /// 商店回购列表（session_id -> 最近卖出的物品，最多保留 10 个）
    pub(crate) buyback_items: HashMap<u64, Vec<BuybackItem>>,
    /// 已加载的地图缓存
    pub(crate) maps: HashMap<u16, MapData>,
    /// GateActor 引用，用于发数据包给客户端
    pub(crate) gate_ref: ActorRef<GateActor>,
    /// 自身 ActorRef（#283：PlayerActor 升级通知回传用；on_start 设置）
    pub(crate) self_ref: Option<ActorRef<WorldActor>>,
    /// 聊天物品已推送记录（#285：session_id → 已发送 uid，避免重复 NewChatItem）
    pub(crate) chat_items_sent: HashMap<u64, std::collections::HashSet<u64>>,
    /// 地图目录
    pub(crate) map_dir: PathBuf,
    /// 刷怪配置目录
    pub(crate) spawn_dir: Option<PathBuf>,
    /// NPC 脚本/INI 根目录（C# NPCPath 等价，SAVEVALUE/LOADVALUE 用）
    pub(crate) script_dir: PathBuf,
    /// 下一个对象 ID
    pub(crate) next_object_id: u32,
    /// 活跃怪物（按 object_id 索引）
    pub(crate) monsters: HashMap<u32, MonsterState>,
    /// #306 诅咒状态（怪物 oid → (减伤百分比, 到期 tick)）
    pub(crate) cursed_monsters: HashMap<u32, (i32, u64)>,
    /// NPC 脚本计时器（SETTIMER）：session -> (timer_id, expire_tick)
    pub(crate) npc_timers: HashMap<u64, HashMap<i32, u64>>,
    /// ENTERMAP 暂存传送点（NeedMove 踩点暂存，C# NPCData["NPCMoveMap"]）：session -> (map, x, y)
    pub(crate) session_last_movement: HashMap<u64, (u16, i32, i32)>,
    /// NPC 脚本延迟执行（TIMERECALL/DELAYGOTO）：session -> 待执行动作列表
    pub(crate) npc_delayed_actions: HashMap<u64, Vec<DelayedNpcAction>>,
    /// #312 烈焰剑状态（session → (到期 tick, 技能等级)）
    pub(crate) flaming_sword: HashMap<u64, (u64, u8)>,
    /// #318 双段近战状态（session → (到期 tick, 等级, 类型: 0=双龙斩 1=双斩)）
    pub(crate) double_hit_melee: HashMap<u64, (u64, u8, u8)>,
    /// #345 MPEater 计数（session → 累计）
    pub(crate) mp_eater_count: HashMap<u64, i32>,
    /// #345 Hemorrhage 计数（session → 累计）
    pub(crate) hemorrhage_count: HashMap<u64, i32>,
    /// #395 幻觉状态（怪物 oid → 到期 tick，期内不攻击）
    pub(crate) hallucinated: HashMap<u32, u64>,
    /// #409 精神状态（session → 0 攻击/1 特技/2 组队模式）
    pub(crate) mental_state: HashMap<u64, u8>,
    /// #448 致命一击状态（session → (到期 tick, 等级)）
    /// FatalSword 被动已触发（C# bool FatalSword，下一击消耗）
    pub(crate) fatal_sword_armed: HashSet<u64>,
    /// #448 宠物强化（怪物 oid → (到期 tick, DC 加成, AC 加成)）
    pub(crate) pet_enhanced: HashMap<u32, (u64, i32, i32)>,
    /// 召唤物等级（怪物 oid → 等级；C# MonsterObject.PetLevel = magic.Level）
    pub(crate) pet_levels: HashMap<u32, i32>,
    /// Revelation 显血窗口（对象 oid → 到期 tick；C# RevTime）
    pub(crate) revealed_hp: HashMap<u32, u64>,
    /// Boss 延迟攻击队列（到期 tick → 攻击；C# DelayedAction DelayedType.Damage）
    pub(crate) boss_pending_attacks: Vec<(u64, ai::DelayedAttack)>,
    /// #471 宠物协战（宠物 oid → 主人攻击的怪物 oid）
    pub(crate) pet_targets: HashMap<u32, u32>,
    /// 活跃 NPC（按 object_id 索引）
    pub(crate) npcs: HashMap<u32, NpcState>,
    /// 等待重生的怪物 (object_id → 重生 tick)
    pub(crate) respawn_queue: HashMap<u32, (MonsterSpawn, u64)>,
    /// 世界Boss存活队列 (object_id → 自动消失 tick)
    pub(crate) world_boss_queue: HashMap<u32, u64>,
    /// 死亡玩家复活队列 (session_id → 死亡 tick)
    pub(crate) player_death_queue: HashMap<u64, u64>,
    /// 钓鱼进度计数器 (session_id → 已钓鱼 tick 数)
    pub(crate) fishing_tick_counters: HashMap<u64, u32>,
    /// 地面掉落物品
    pub(crate) ground_items: Vec<GroundItem>,
    /// 已打开的门 (map_index, door_index)
    pub(crate) open_doors: std::collections::HashSet<(u16, u8)>,
    /// SQLite 数据库连接池
    pub(crate) db_pool: DbPool,
    /// 游戏配置：地图信息（key = map index）
    pub(crate) map_infos: HashMap<i32, db::MapInfo>,
    /// 游戏配置：物品信息
    pub(crate) item_infos: HashMap<i32, db::ItemInfo>,
    /// 游戏配置：怪物信息
    pub(crate) monster_infos: HashMap<i32, db::MonsterInfo>,
    /// 怪物名称 → index 缓存（Boss 召唤按名查 MonsterInfo 用）
    pub(crate) monster_name_index: HashMap<String, i32>,
    /// 合成配方列表（NPC Craft 用）
    #[allow(dead_code)]
    pub(crate) recipe_infos: Vec<db::RecipeInfo>,
    /// 游戏配置：怪物掉落（monster_index -> drop list）
    pub(crate) monster_drops: HashMap<i32, Vec<db::MonsterDropInfo>>,
    /// 游戏配置：NPC 信息
    pub(crate) npc_infos: HashMap<i32, db::NPCInfo>,
    /// 游戏配置：NPC 商品（npc_index -> goods list）
    pub(crate) npc_goods: HashMap<i32, Vec<db::NpcGoodsInfo>>,
    /// 会话当前对话的 NPC（BuyItem 用，客户端协议不含 npc_id）
    pub(crate) session_npc: HashMap<u64, u32>,
    /// 游戏配置：NPC 脚本 ((npc_index, page_name) -> lines)
    pub(crate) npc_scripts: HashMap<(i32, String), Vec<String>>,
    /// 游戏配置：任务信息
    pub(crate) quest_infos: HashMap<i32, db::QuestInfo>,
    /// 游戏配置：魔法信息（key = spell ID）
    pub(crate) magic_infos: HashMap<u32, db::MagicInfo>,
    /// 游戏配置：龙信息
    pub(crate) dragon_info: Option<db::DragonInfo>,
    /// 游戏商店物品（从 DB 加载）
    pub(crate) game_shop_items: Vec<db::GameShopItem>,
    /// 地图传送点索引: (map_index, source_x, source_y) -> MapMovementInfo
    pub(crate) movement_index: HashMap<(i32, i32, i32), db::MapMovementInfo>,
    /// SocialActor 引用（用于转发社交命令）
    pub(crate) social_ref: ActorRef<SocialActor>,
    /// 攻城/GT 配置
    pub(crate) conquest_cfg: crate::util::config::ConquestConfig,
    /// 休息经验加成配置（C# Settings.Rested*）
    pub(crate) rested_cfg: crate::util::config::RestedConfig,
    /// 全局掉落倍率
    pub(crate) drop_rate: f64,
    /// 地面物品超时 ticks
    pub(crate) item_timeout_ticks: u64,
    /// 金币掉落每堆上限
    pub(crate) max_drop_gold: u32,
    /// 精英怪配置
    pub(crate) rarity_cfg: crate::util::config::RarityConfig,
    /// 服务器公告文件路径（C# Settings.NoticePath）
    pub(crate) notice_path: String,
    /// 死亡经验惩罚百分比（默认 0=关闭，对齐 C#）
    pub(crate) death_exp_penalty_percent: u32,
    /// 回血权重（C# Settings.HealthRegenWeight）
    pub(crate) health_regen_weight: u32,
    /// 回蓝权重（C# Settings.ManaRegenWeight）
    pub(crate) mana_regen_weight: u32,
    /// 商店隐藏附加属性（C# Settings.GoodsHideAddedStats）
    pub(crate) goods_hide_added_stats: bool,
    /// 全局经验倍率事件
    pub(crate) global_exp_multiplier: f64,
    /// 全局掉落倍率
    pub(crate) global_drop_multiplier: f64,
    /// 全局金币倍率
    pub(crate) global_gold_multiplier: f64,
    /// 全局事件过期时间（tick count）
    pub(crate) global_exp_event_end_tick: u64,
    /// 当前全局事件名称
    pub(crate) global_event_name: Option<String>,
    /// 隐身中的玩家 session 集合（用于视野管理）
    pub(crate) invisible_sessions: std::collections::HashSet<u64>,
    /// Phase 1.4: 反作弊 — 每个玩家上次移动时间戳(用于速度 hack 检测)
    pub(crate) last_move_time: std::collections::HashMap<u64, std::time::Instant>,
    /// 当前光照设置（0=Normal, 1=Dawn, 2=Day, 3=Evening, 4=Night）
    pub(crate) current_light: mir2_shared::enums::LightSetting,
    /// 寄售/拍卖列表
    pub(crate) auctions: Vec<AuctionListing>,
    /// 下一个拍卖ID
    pub(crate) next_auction_id: u64,
    /// 市场搜索缓存 (session_id -> search results indices)
    pub(crate) market_search_cache: HashMap<u64, MarketSearchCache>,
    /// 物品租赁会话 (initiator_session_id -> RentalSession)
    pub(crate) rental_sessions: HashMap<u64, RentalSession>,
    /// 已生效的租赁记录 (renter_name -> list of RentedItem)
    pub(crate) player_rentals: HashMap<String, Vec<RentedItem>>,
    /// 持久法术对象（火墙、暴风雪等），按 object_id 索引
    pub(crate) spell_objects: HashMap<u32, spell::SpellObject>,
    /// 弹道法术的延迟结算队列（对齐 C# DelayedAction）
    pub(crate) pending_spell_completions: Vec<PendingSpellCompletion>,
    /// tick_spell_completions 期间的吸血回血暂存（session_id, amount）
    pub(crate) vamp_heals: Vec<(u64, i32)>,
    /// 定时机器人任务
    pub(crate) robot_tasks: Vec<robot::RobotTask>,
    /// 机器人上次检查的分钟值
    pub(crate) robot_last_check_minute: u32,
    /// 龙系统状态
    pub(crate) dragon_state: Option<dragon::DragonState>,
    /// 征服区域列表
    pub(crate) conquest_instances: Vec<conquest::ConquestInstance>,
    /// 城门/城墙/攻城武器
    pub(crate) siege_structures: HashMap<u32, conquest::SiegeStructure>,
    /// 行会战争声明 (guild_name -> set of enemy guild names)
    pub(crate) guild_wars: HashMap<String, std::collections::HashSet<String>>,
    /// 英雄战斗 AI 运行时状态（按主人 session_id 索引）
    pub(crate) hero_ai_states: HashMap<u64, HeroCombatAI>,
    /// 玩家英雄列表（按主人 session_id 索引；内存态，#188，DB 持久化后续批次）
    pub(crate) player_heroes: HashMap<u64, Vec<HeroInfo>>,
}

/// 英雄信息（#188：C# ClientHeroInformation 语义，内存态）

/// 英雄创建结果码（C# S.NewHero.Result：1=BadName 4=MaxHeroes 10=Success）
pub(crate) fn hero_create_result(name: &str, has_hero: bool) -> u8 {
    if has_hero {
        4
    } else if name.trim().is_empty() {
        1
    } else {
        10
    }
}
#[derive(Debug, Clone)]
pub struct HeroInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: mir2_shared::enums::MirClass,
    pub gender: mir2_shared::enums::MirGender,
    /// 是否死亡（REVIVEHERO 复活，对齐 C# Hero.Dead）
    pub dead: bool,
    /// 是否封印（SEALHERO，对齐 C# Hero.Sealed）
    pub sealed: bool,
}


/// 租赁会话状态
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct RentalSession {
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
pub(crate) struct RentedItem {
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

    /// C# ItemObject.Drop(distance)：掉落物在 range 内散落（简化：随机偏移可行走格，回退原点）
pub(crate) fn scatter_drop_position(map: Option<&MapData>, x: i32, y: i32, range: i32) -> (i32, i32) {
    if let Some(m) = map {
        for _ in 0..12 {
            let nx = x + fastrand::i32(-range..=range);
            let ny = y + fastrand::i32(-range..=range);
            if m.is_walkable(nx, ny) {
                return (nx, ny);
            }
        }
    }
    (x, y)
}


impl WorldActor {
    pub fn new(gate_ref: ActorRef<GateActor>, map_dir: PathBuf, spawn_dir: Option<PathBuf>, db_pool: DbPool, social_ref: ActorRef<SocialActor>) -> Self {
        Self {
            tick_count: 0,
            npc_timers: HashMap::new(),
            session_last_movement: HashMap::new(),
            npc_delayed_actions: HashMap::new(),
            players: HashMap::new(),
            buyback_items: HashMap::new(),
            maps: HashMap::new(),
            gate_ref,
            self_ref: None,
            chat_items_sent: HashMap::new(),
            map_dir,
            spawn_dir,
            script_dir: PathBuf::from("."),
            next_object_id: 1000,
            monsters: HashMap::new(),
            cursed_monsters: HashMap::new(),
            flaming_sword: HashMap::new(),
            double_hit_melee: HashMap::new(),
            mp_eater_count: HashMap::new(),
            hemorrhage_count: HashMap::new(),
            hallucinated: HashMap::new(),
            mental_state: HashMap::new(),
            fatal_sword_armed: HashSet::new(),
            boss_pending_attacks: Vec::new(),
            pet_enhanced: HashMap::new(),
            pet_levels: HashMap::new(),
            revealed_hp: HashMap::new(),
            pet_targets: HashMap::new(),
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
            monster_name_index: HashMap::new(),
            recipe_infos: Vec::new(),
            monster_drops: HashMap::new(),
            npc_infos: HashMap::new(),
            npc_goods: HashMap::new(),
            session_npc: HashMap::new(),
            npc_scripts: HashMap::new(),
            quest_infos: HashMap::new(),
            magic_infos: HashMap::new(),
            dragon_info: None,
            game_shop_items: Vec::new(),
            movement_index: HashMap::new(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            drop_rate: 1.0,
            item_timeout_ticks: 600,
            max_drop_gold: 2000,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            global_exp_multiplier: 1.0,
            global_drop_multiplier: 1.0,
            global_gold_multiplier: 1.0,
            global_exp_event_end_tick: 0,
            global_event_name: None,
            invisible_sessions: HashSet::new(),
            last_move_time: std::collections::HashMap::new(),
            current_light: Self::light_for_hour(chrono::Local::now().hour()),
            auctions: Vec::new(),
            next_auction_id: 1,
            market_search_cache: HashMap::new(),
            rental_sessions: HashMap::new(),
            player_rentals: HashMap::new(),
            spell_objects: HashMap::new(),
            pending_spell_completions: Vec::new(),
            vamp_heals: Vec::new(),
            robot_tasks: Vec::new(),
            robot_last_check_minute: 0,
            dragon_state: None,
            conquest_instances: default_conquest_instances(),
            siege_structures: HashMap::new(),
            guild_wars: HashMap::new(),
            hero_ai_states: HashMap::new(),
            player_heroes: HashMap::new(),
        }
    }

    /// 计算全局经验倍率后的经验值
    pub(crate) fn apply_global_exp_multiplier(&self, base: i32) -> i32 {
        if self.tick_count < self.global_exp_event_end_tick {
            (base as f64 * self.global_exp_multiplier).round() as i32
        } else {
            base
        }
    }

    /// 根据小时计算光照设置（基于服务器本地时区）
    pub(crate) fn light_for_hour(hour: u32) -> mir2_shared::enums::LightSetting {
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
    pub(crate) fn send_time_of_day(&self, session_id: u64, light: mir2_shared::enums::LightSetting) {
        let packet = mir2_shared::packets::server::player::TimeOfDay { lights: light as u8 };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize TimeOfDay: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::TimeOfDay as i16, &body),
        }).try_send();
    }

    /// 发送 ObjectRemove 给同地图其他玩家，使该玩家从他人视野中消失
    pub(crate) async fn hide_player_from_others(&self, session_id: u64, state: &crate::actors::player::PlayerState) {
        let mut body = Vec::new();
        body.extend_from_slice(&state.object_id.to_le_bytes());
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &body);
        for (sid, record) in &self.players {
            if *sid == session_id { continue; }
            if let Ok(Some(other_state)) = record.actor_ref.ask(GetPlayerState).await {
                if other_state.map_index == state.map_index {
                    let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: packet.clone() }).await;
                }
            }
        }
    }

    /// 发送 ObjectPlayer 给同地图其他玩家，使该玩家重新出现在他人视野中
    pub(crate) async fn reveal_player_to_others(&self, session_id: u64, state: &crate::actors::player::PlayerState) {
        // C#：Hidden=false 广播（客户端取消隐身显示）
        self.broadcast_object_hidden(state.object_id, false, state.map_index).await;
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
                    let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: packet.clone() }).await;
                }
            }
        }
    }

    /// 加载或获取已缓存的地图
    /// 按地图索引加载地图数据（支持多地图并存）。
    /// `map_index` 来自 MapInfo.index，`file_name` 是地图文件路径。
    pub(crate) fn get_or_load_map(&mut self, file_name: &str, map_index: u16) -> Option<&MapData> {
        let need_load = !self.maps.contains_key(&map_index)
            || self.maps.get(&map_index).map(|m| m.file_name != file_name).unwrap_or(true);
        if need_load {
            match loader::load_map(file_name, &self.map_dir) {
                Ok(mut map) => {
                    info!("Loaded map: {} ({}x{}) → slot {}", map.file_name, map.width, map.height, map_index);
                    if let Some(mi) = self.map_infos.values().find(|m| m.file_name == file_name) {
                        if mi.no_fight {
                            map.safe_zone_rects.push((0, 0, map.width as i32 - 1, map.height as i32 - 1));
                        }
                    }
                    Self::apply_hardcoded_safe_zones(file_name, &mut map);
                    self.maps.insert(map_index, map);
                }
                Err(e) => {
                    warn!("Failed to load map '{}': {}", file_name, e);
                    return None;
                }
            }
        }
        self.maps.get(&map_index)
    }

    /// 为已知地图注入默认安全区（坐标为 Mir2 经典值）
    pub(crate) fn apply_hardcoded_safe_zones(file_name: &str, map: &mut MapData) {
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
    pub(crate) fn alloc_object_id(&mut self) -> u32 {
        let id = self.next_object_id;
        self.next_object_id += 1;
        id
    }

    /// 获取所有其他玩家的引用（排除指定 session）
    pub(crate) fn other_players(&self, exclude_session: u64) -> Vec<&PlayerRecord> {
        self.players.values()
            .filter(|r| r.session_id != exclude_session)
            .collect()
    }

    /// NPC 改发型/转职/变性后刷新外观（自身 UserInformation + 同图广播 ObjectPlayer）
    pub(crate) async fn refresh_player_appearance(&self, session_id: u64) {
        let Some(record) = self.players.get(&session_id) else { return };
        let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await else { return };
        let packet = build_user_information_packet(&state, &self.item_infos);
        let _ = self.gate_ref.tell(SendToClient { session_id, data: packet }).await;
        self.broadcast_player_appearance(session_id, &state).await;
    }

    /// C# Attacked()：向同图其他玩家广播 ObjectStruck + DamageIndicator（PvP 命中表现）
    pub(crate) async fn broadcast_pvp_hit(&self, target_oid: u32, attacker_oid: u32, x: i32, y: i32, dir: u8, damage: i32, map_index: u16) {
        let mut struck_body = Vec::new();
        struck_body.extend_from_slice(&target_oid.to_le_bytes());
        struck_body.extend_from_slice(&attacker_oid.to_le_bytes());
        struck_body.extend_from_slice(&(x as u32).to_le_bytes());
        struck_body.extend_from_slice(&(y as u32).to_le_bytes());
        struck_body.push(dir);
        let struck_packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectStruck as i16, &struck_body);
        let mut dmg_body = Vec::new();
        dmg_body.extend_from_slice(&damage.to_le_bytes());
        dmg_body.push(0u8); // damage_type = normal
        dmg_body.extend_from_slice(&target_oid.to_le_bytes());
        let dmg_packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::DamageIndicator as i16, &dmg_body);
        for (sid, r) in &self.players {
            if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                if os.map_index == map_index {
                    let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: struck_packet.clone() }).await;
                    let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: dmg_packet.clone() }).await;
                }
            }
        }
    }

    /// C# Hidden 属性：向同图其他玩家广播 S.ObjectHidden（隐身/现身）
    pub(crate) async fn broadcast_object_hidden(&self, object_id: u32, hidden: bool, map_index: u16) {
        let packet = mir2_shared::packets::server::object::ObjectHidden { object_id, hidden };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            for (sid, record) in &self.players {
                if let Ok(Some(os)) = record.actor_ref.ask(GetPlayerState).await {
                    if os.map_index == map_index {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid,
                            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectHidden as i16, &body.clone()),
                        }).await;
                    }
                }
            }
        }
    }

    /// C# Teleport/Knockback：向自身发 UserLocation + 同图其他玩家 BroadcastMovement
    pub(crate) async fn broadcast_position_change(&self, session_id: u64, x: i32, y: i32, direction: u8) {
        let mut loc = Vec::new();
        loc.extend_from_slice(&x.to_le_bytes());
        loc.extend_from_slice(&y.to_le_bytes());
        loc.push(direction);
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &loc),
        }).await;
        let object_id = self.players.get(&session_id).map(|r| r.object_id).unwrap_or(0);
        let others: Vec<_> = self.other_players(session_id).into_iter().map(|r| r.actor_ref.clone()).collect();
        for other in others {
            let _ = other.ask(crate::actors::player::BroadcastMovement {
                object_id,
                x,
                y,
                direction,
                move_type: crate::actors::player::MoveType::Walk,
                exclude_session: session_id,
            }).await;
        }
    }

    /// 推开玩家（C# HumanObject.Pushed：沿 dir 最多 distance 格，遇不可行走/出界停止；
    /// 方向取反，发 Pushed 给本人 + ObjectPushed 给其他人）
    pub(crate) async fn push_player(&mut self, session_id: u64, dir: u8, distance: i32) -> usize {
        if dir >= 8 || distance <= 0 {
            return 0;
        }
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return 0,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return 0,
        };
        if state.is_dead {
            return 0;
        }
        let map_index = state.map_index;
        let (dx, dy) = (MON_DIR_DX[dir as usize], MON_DIR_DY[dir as usize]);
        let mut nx = state.x;
        let mut ny = state.y;
        let mut steps = 0usize;
        for _ in 0..distance {
            let tx = nx + dx;
            let ty = ny + dy;
            let walkable = self.maps.get(&map_index)
                .map(|m| m.is_walkable(tx, ty))
                .unwrap_or(false);
            if !walkable {
                break;
            }
            nx = tx;
            ny = ty;
            steps += 1;
        }
        if steps == 0 {
            return 0;
        }
        // C#：被推开时朝向反方向
        let reverse_dir = ((dir as usize + 4) % 8) as u8;
        let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
            x: nx, y: ny, direction: reverse_dir,
            map_index: None, is_mounted: None,
        }).await;
        // 本人：Pushed（location + direction）
        let mut self_body = Vec::new();
        self_body.extend_from_slice(&(nx as u32).to_le_bytes());
        self_body.extend_from_slice(&(ny as u32).to_le_bytes());
        self_body.push(reverse_dir);
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Pushed as i16, &self_body),
        }).await;
        // 他人：ObjectPushed（object_id + location + direction）
        let mut obj_body = Vec::new();
        obj_body.extend_from_slice(&state.object_id.to_le_bytes());
        obj_body.extend_from_slice(&(nx as u32).to_le_bytes());
        obj_body.extend_from_slice(&(ny as u32).to_le_bytes());
        obj_body.push(reverse_dir);
        let obj_pkt = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectPushed as i16, &obj_body);
        for (sid, _) in &self.players {
            if *sid == session_id { continue; }
            let _ = self.gate_ref.tell(SendToClient {
                session_id: *sid,
                data: obj_pkt.clone(),
            }).await;
        }
        debug!("Push player {} {} tiles dir={} to ({},{})", session_id, steps, dir, nx, ny);
        steps
    }

    /// 推开怪物（C# MonsterObject.Pushed：受 Info.CanPush 限制，逐格校验 walkable + 怪物占用，
    /// 移动后广播 ObjectPushed；朝向取反）
    pub(crate) async fn push_monster(&mut self, oid: u32, dir: u8, distance: i32) -> usize {
        if dir >= 8 || distance <= 0 {
            return 0;
        }
        let (map_index, start_x, start_y, can_push) = match self.monsters.get(&oid) {
            Some(m) => {
                let can = self.monster_infos.get(&m.monster_index)
                    .map(|i| i.can_push)
                    .unwrap_or(true);
                (m.map_index, m.x, m.y, can)
            }
            None => return 0,
        };
        if !can_push {
            return 0;
        }
        let (dx, dy) = (MON_DIR_DX[dir as usize], MON_DIR_DY[dir as usize]);
        let mut occupied: std::collections::HashSet<(i32, i32)> = self.monsters.values()
            .filter(|m| m.hp > 0)
            .map(|m| (m.x, m.y))
            .collect();
        let mut nx = start_x;
        let mut ny = start_y;
        let mut steps = 0usize;
        for _ in 0..distance {
            let tx = nx + dx;
            let ty = ny + dy;
            let walkable = self.maps.get(&map_index)
                .map(|m| m.is_walkable(tx, ty))
                .unwrap_or(false);
            if !walkable || occupied.contains(&(tx, ty)) {
                break;
            }
            nx = tx;
            ny = ty;
            steps += 1;
        }
        if steps == 0 {
            return 0;
        }
        let reverse_dir = ((dir as usize + 4) % 8) as u8;
        if let Some(m) = self.monsters.get_mut(&oid) {
            m.x = nx;
            m.y = ny;
            m.direction = reverse_dir;
            m.provoked = true;
        }
        let mut obj_body = Vec::new();
        obj_body.extend_from_slice(&oid.to_le_bytes());
        obj_body.extend_from_slice(&(nx as u32).to_le_bytes());
        obj_body.extend_from_slice(&(ny as u32).to_le_bytes());
        obj_body.push(reverse_dir);
        let obj_pkt = build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectPushed as i16, &obj_body);
        for (sid, _) in &self.players {
            let _ = self.gate_ref.tell(SendToClient {
                session_id: *sid,
                data: obj_pkt.clone(),
            }).await;
        }
        debug!("Push monster {} {} tiles dir={} to ({},{})", oid, steps, dir, nx, ny);
        steps
    }

    /// 发送 NPC 商店商品列表（DB 商品）
    pub(crate) fn send_npc_goods(&self, session_id: u64, npc: &NpcState) {
        let goods = self.npc_goods.get(&npc.db_index).cloned().unwrap_or_default();

        let mut items = Vec::new();
        for good in &goods {
            let mut item = mir2_shared::data::item::UserItem {
                item_index: good.item_index,
                count: good.count as u16,
                ..Default::default()
            };
            // 填充物品信息（客户端商店列表需要名称/价格/图标/类型；枚举需 C#→SharedRust +3）
            enrich_item_info(&mut item, &self.item_infos);
            if let Some(info) = self.item_infos.get(&good.item_index) {
                item.max_dura = info.durability as u16;
                item.current_dura = info.durability as u16;
            }
            items.push(item);
        }

        self.send_npc_goods_items(session_id, npc, items);
    }

    /// 发送回购列表（C# 语义：原物品 + 原始 unique_id，客户端据此发 BuyItemBack）
    pub(crate) fn send_buyback_goods(&mut self, session_id: u64, npc: &NpcState) {
        // C#：过期回购物品清理（GoodsBuyBackTime = 60 分钟）
        let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        if let Some(list) = self.buyback_items.get_mut(&session_id) {
            list.retain(|b| b.expires_at > now_ms);
        }
        let items: Vec<mir2_shared::data::item::UserItem> = self
            .buyback_items
            .get(&session_id)
            .map(|l| {
                l.iter()
                    .map(|b| {
                        let mut item = b.item.clone();
                        enrich_item_info(&mut item, &self.item_infos);
                        item
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.send_npc_goods_items(session_id, npc, items);
    }

    /// 发送 NPCGoods 通用实现（rate 由 NPC 配置决定）
    fn send_npc_goods_items(
        &self,
        session_id: u64,
        npc: &NpcState,
        items: Vec<mir2_shared::data::item::UserItem>,
    ) {
        // Use DB rate if available, default 1.0
        let rate = if npc.db_index > 0 {
            self.npc_infos.get(&npc.db_index).map(|n| n.rate as f32 / 100.0).unwrap_or(1.0)
        } else {
            1.0
        };

        let npc_goods_packet = mir2_shared::packets::server::npc_interaction::NPCGoods {
            list: items.clone(),
            rate,
            panel_type: mir2_shared::enums::PanelType::Buy,
            hide_added_stats: self.goods_hide_added_stats,
        };

        let mut body = Vec::new();
        if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &npc_goods_packet) {
            warn!("Failed to serialize NPCGoods: {}", e);
            return;
        }

        // serialize_packet 已写入完整内层包头（length+opcode），不能再用 build_packet_bytes 二次包装
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: body,
        }).try_send();
        debug!("Sent {} goods from NPC '{}' (rate={}) to session {}", items.len(), npc.name, rate, session_id);
    }

    /// 发送 NPC 面板（出售/修理等，空商品列表）
    pub(crate) fn send_npc_panel(&self, session_id: u64, panel_type: mir2_shared::enums::PanelType) {
        let packet = mir2_shared::packets::server::npc_interaction::NPCGoods {
            list: Vec::new(),
            rate: 1.0,
            panel_type,
            hide_added_stats: self.goods_hide_added_stats,
        };
        let mut body = Vec::new();
        if let Err(e) = mir2_shared::packets::base::serialize_packet(
            &mut std::io::Cursor::new(&mut body), &packet) {
            warn!("Failed to serialize NPCGoods panel: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: body,
        }).try_send();
        debug!("Sent NPC panel {:?} to session {}", panel_type, session_id);
    }

    /// 发送仓库内容给客户端（打开仓库 UI）
    pub(crate) fn send_user_storage(&self, session_id: u64, storage: &[Option<crate::actors::inventory::InventorySlot>]) {
        let items: Vec<Option<mir2_shared::data::item::UserItem>> = storage.iter()
            .map(|slot| slot.as_ref().map(|s| s.item.clone()))
            .collect();

        let packet = mir2_shared::packets::server::player::UserStorage { storage: items };
        let mut body = Vec::new();
        if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet) {
            warn!("Failed to serialize UserStorage: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: body,
        }).try_send();
        debug!("Sent UserStorage to session {}", session_id);
    }

    /// 发送 CombineItem 响应给客户端
    pub(crate) fn send_combine_item_response(
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
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CombineItem as i16, &body),
        }).try_send();
    }

    /// 广播玩家外观更新给同地图的其他玩家
    pub(crate) async fn broadcast_player_appearance(&self,
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
            let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: packet.clone() }).await;
        }
    }

    /// 强制玩家下坐骑并广播外观更新
    pub(crate) async fn dismount_player(&mut self,
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
    pub(crate) async fn spawn_single_drop(&mut self, monster: &MonsterState, item_index: i32, count: u16) {
        let drop_oid = self.alloc_object_id();
        if item_index == 0 {
            // 对齐 C# Settings.MaxDropGold：每堆金币 ≤ max_drop_gold，超过拆成多堆
            let total = count as u64;
            let mut remaining = total;
            let mut piles = 0u32;
            while remaining > 0 {
                let pile = remaining.min(self.max_drop_gold as u64) as u32;
                remaining -= pile as u64;
                let oid = if piles == 0 { drop_oid } else { self.alloc_object_id() };
                let object_gold = mir2_shared::packets::server::ObjectGold {
                    object_id: oid,
                    gold: pile,
                    location_x: monster.x,
                    location_y: monster.y,
                };
                let mut buf = Vec::new();
                if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_gold) {
                    warn!("Failed to serialize ObjectGold: {}", e);
                    continue;
                }
                for session_id in self.players.keys() {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *session_id,
                        data: buf.clone(),
                    }).await;
                }
                self.ground_items.push(GroundItem {
                    object_id: oid,
                    item: mir2_shared::data::item::UserItem {
                        item_index: 0,
                        count: pile as u16,
                        ..Default::default()
                    },
                    x: monster.x,
                    y: monster.y,
                    map_index: monster.map_index,
                    dropper_session: None,
                    drop_tick: self.tick_count,
                });
                piles += 1;
            }
            debug!("Monster '{}' dropped {} gold ({} piles) at ({}, {})", monster.name, total, piles, monster.x, monster.y);
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
            // 补 ItemInfo（ObjectItem 携带 info 供客户端渲染名称/图标，与 M16 玩家丢弃路径一致）
            enrich_item_info(&mut item, &self.item_infos);
            // C#：掉落散落（Settings.DropRange）
            let (dx, dy) = scatter_drop_position(self.maps.get(&monster.map_index), monster.x, monster.y, 4);
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: item.clone(),
                location_x: dx,
                location_y: dy,
            };
            let mut buf = Vec::new();
            if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item) {
                warn!("Failed to serialize ObjectItem: {}", e);
                return;
            }
            for session_id in self.players.keys() {
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: *session_id,
                    data: buf.clone(),
                }).await;
            }
            self.ground_items.push(GroundItem {
                object_id: drop_oid,
                item,
                x: dx,
                y: dy,
                map_index: monster.map_index,
                dropper_session: None,
                drop_tick: self.tick_count,
            });
            debug!("Monster '{}' dropped item index={} count={} at ({}, {})", monster.name, item_index, count, dx, dy);
        }
    }

    pub(crate) async fn spawn_monster_drops(&mut self, monster: &MonsterState) {
        let drops = match self.monster_drops.get(&monster.monster_index) {
            Some(d) if !d.is_empty() => d.clone(),
            _ => return,
        };

        let count_mul = drop_count_multiplier(monster.is_boss, monster.is_elite);
        let global_drop_mul = if self.tick_count < self.global_exp_event_end_tick {
            self.global_drop_multiplier
        } else { 1.0 };
        // 玩家掉落 Buff（Potion shape 5 Drop，C# BuffType.Drop）：按击杀目标 target_session 查找
        let player_drop_mul: f64 = if let Some(sid) = monster.target_session {
            if let Some(record) = self.players.get(&sid) {
                if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                    if self.tick_count < state.drop_multiplier_end_tick {
                        state.drop_multiplier
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            } else {
                1.0
            }
        } else {
            1.0
        };

        for drop in &drops {
            let roll = fastrand::f64();
            // 全局掉落倍率（C# Settings.DropRate）+ 玩家掉落 Buff：chance * drop_rate * drop_buff，上限 1.0
            let effective_chance = (drop.chance * self.drop_rate * player_drop_mul).min(1.0);
            if roll > effective_chance {
                continue;
            }
            let count = if drop.max_count > drop.min_count {
                fastrand::u16(drop.min_count..=drop.max_count).saturating_mul(count_mul)
            } else {
                drop.min_count.saturating_mul(count_mul)
            };
            let adjusted = (count as f64 * global_drop_mul * player_drop_mul).round() as u16;
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
                let adjusted = (count as f64 * global_drop_mul * player_drop_mul).round() as u16;
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
                let adjusted = (count as f64 * global_drop_mul * player_drop_mul).round() as u16;
                self.spawn_single_drop(monster, drop.item_index, adjusted.max(1)).await;
            }
        }
    }

    /// 玩家死亡时随机掉落背包物品和金币（安全区不掉落）
    pub(crate) async fn handle_player_death_drop(
        &mut self,
        session_id: u64,
        x: i32,
        y: i32,
        map_index: u16,
    ) {
        // C# DeathDrop：NoDropPlayer 地图直接返回；安全区也不掉落（保留现有保护）
        if self.map_infos.get(&(map_index as i32)).map(|m| m.no_drop_player).unwrap_or(false) {
            return;
        }
        if self.maps.get(&map_index).map(|m| m.is_safe_zone(x, y)).unwrap_or(false) {
            return;
        }
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // C#：PKPoints > 200 → RedDeathDrop（概率更高）；否则 DeathDrop
        let red = state.pk_points > 200;
        // C#：掉落散落（Settings.DropRange=4）
        let (drop_x, drop_y) = scatter_drop_position(self.maps.get(&map_index), x, y, 4);

        let mut dropped_items: Vec<mir2_shared::data::item::UserItem> = Vec::new();

        // ===== 装备槽位（C# DeathDrop 先遍历装备） =====
        for slot_idx in 0..crate::actors::inventory::EquipmentSlot::COUNT {
            let Some(slot) = crate::actors::inventory::EquipmentSlot::from_i32(slot_idx as i32) else {
                continue;
            };
            let Some(item) = state.inventory.equipment[slot_idx].as_ref() else { continue };
            let Some(info) = self.item_infos.get(&item.item_index) else { continue };
            let bind = info.bind_mode;
            // C# BindMode.DontDeathdrop = 0x0001：不掉落
            if (bind & 0x0001) != 0 {
                continue;
            }
            // 结婚戒指（C#：WeddingRing != -1 且为左戒指不掉）
            if item.wedding_ring != -1 {
                continue;
            }
            // 封印物品未到期不掉（Rust 简化：任何封印状态都不掉）
            if item.sealed_info.is_some() {
                continue;
            }
            // 租赁物品：C# 返还主人；Rust 简化不参与死亡掉落
            if item.rental_information.is_some() {
                continue;
            }

            // C# BindMode.BreakOnDeath = 0x0100：碎裂（移除但不落地）
            if (bind & 0x0100) != 0 {
                let _ = record.actor_ref.ask(crate::actors::player::TakeEquipmentOnDeath { slot }).await;
                continue;
            }

            if item.count > 1 {
                // 堆叠：按百分比掉（C# RandomomRange(10, rate)：10 次 p=1/rate 二项）
                let rate = if red { 4 } else { 8 };
                let mut percent = 0;
                for _ in 0..10 {
                    if fastrand::i32(..rate) == 0 {
                        percent += 1;
                    }
                }
                if percent == 0 && !red {
                    continue;
                }
                let drop_count = ((item.count as f32) / 10.0 * percent as f32).ceil() as u16;
                if drop_count == 0 {
                    continue;
                }
                if let Some(dropped) = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventoryCount {
                    unique_id: item.unique_id,
                    count: drop_count,
                }).await.unwrap_or(None) {
                    dropped_items.push(dropped);
                }
            } else {
                // 单件：1/30（非红）或 1/10（红）整件掉落
                let chance = if red { 10 } else { 30 };
                if fastrand::i32(..chance) == 0 {
                    if let Some(dropped) = record.actor_ref.ask(crate::actors::player::TakeEquipmentOnDeath { slot }).await.unwrap_or(None) {
                        dropped_items.push(dropped);
                    }
                }
            }
        }

        // ===== 背包（C# 遍历 Inventory） =====
        let backpack = state.inventory.backpack.clone();
        for s in backpack.iter().flatten() {
            let item = &s.item;
            let Some(info) = self.item_infos.get(&item.item_index) else { continue };
            let bind = info.bind_mode;
            if (bind & 0x0001) != 0 {
                continue;
            }
            if item.wedding_ring != -1 {
                continue;
            }
            if item.sealed_info.is_some() {
                continue;
            }
            if item.rental_information.is_some() {
                continue;
            }

            if item.count > 1 {
                let rate = if red { 4 } else { 8 };
                let mut percent = 0;
                for _ in 0..10 {
                    if fastrand::i32(..rate) == 0 {
                        percent += 1;
                    }
                }
                if percent == 0 && !red {
                    continue;
                }
                let drop_count = ((item.count as f32) / 10.0 * percent as f32).ceil() as u16;
                if drop_count == 0 {
                    continue;
                }
                if let Some(dropped) = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventoryCount {
                    unique_id: item.unique_id,
                    count: drop_count,
                }).await.unwrap_or(None) {
                    dropped_items.push(dropped);
                }
            } else {
                let chance = if red { 10 } else { 30 };
                if fastrand::i32(..chance) == 0 {
                    if let Some(dropped) = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventoryCount {
                        unique_id: item.unique_id,
                        count: 1,
                    }).await.unwrap_or(None) {
                        dropped_items.push(dropped);
                    }
                }
            }
        }

        // 落地物品（C# 死亡不掉金币；散落 + Meat 掉 2000 耐久）
        for mut item in dropped_items {
            // C# HumanObject.DropItem：Meat 落地 current_dura -= 2000
            if self.item_infos.get(&item.item_index).map(|i| i.item_type == 15 /* Meat */).unwrap_or(false) {
                item.current_dura = item.current_dura.saturating_sub(2000);
            }
            let drop_oid = self.alloc_object_id();
            let object_item = mir2_shared::packets::server::ObjectItem {
                object_id: drop_oid,
                item: item.clone(),
                location_x: drop_x,
                location_y: drop_y,
            };
            let mut buf = Vec::new();
            if let Err(e) = mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut buf), &object_item) {
                warn!("Failed to serialize ObjectItem: {}", e);
                continue;
            }
            for sid in self.players.keys() {
                let _ = self.gate_ref.tell(SendToClient { session_id: *sid, data: buf.clone() }).await;
            }
            self.ground_items.push(GroundItem { object_id: drop_oid, item, x: drop_x, y: drop_y, map_index, dropper_session: Some(session_id), drop_tick: self.tick_count });
        }
    }

/// 执行 NPC 脚本行，解析条件命令与动作命令
    /// 返回 (显示文本, GOTO 目标页面名)
    pub(crate) async fn eval_npc_script(
        &mut self,
        lines: &mut [String],
        session_id: u64,
        npc: &NpcState,
    ) -> (Vec<String>, Option<String>) {
        let mut output = Vec::new();
        let mut skip = false;
        let mut goto_target: Option<String> = None;
        // NPC 邮件暂存：COMPOSEMAIL 创建，ADDMAILGOLD/ADDMAILITEM 累积附件，SENDMAIL 发送
        // 对齐 C# NPCSegment.cs 的 ActionType.ComposeMail/AddMailGold/AddMailItem/SendMail
        let mut mail_info: Option<MailMessage> = None;

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
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id,
                                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
                                    }).await;
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
                                            let _ = self.gate_ref.tell(SendToClient {
                                                session_id,
                                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NewMagic as i16, &body),
                                            }).await;
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
                                        // #214：spell 为 C# 编号，客户端需 SharedRust +3
                                        let spell_enum =
                                            mir2_shared::enums::Spell::try_from(spell as u8 + 3)
                                                .unwrap_or(mir2_shared::enums::Spell::None);
                                        let leveled = mir2_shared::packets::server::magic::MagicLeveled { object_id: record.object_id, spell: spell_enum, level: state.magics.iter().find(|m| m.spell == spell).map(|m| m.level).unwrap_or(0), experience: 0 };
                                        let mut body = Vec::new();
                                        if leveled.write_body(&mut body).is_ok() {
                                            let _ = self.gate_ref.tell(SendToClient {
                                                session_id,
                                                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MagicLeveled as i16, &body),
                                            }).await;
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
                                    min_ac: 0,
                                    max_ac: 0,
                                    min_mac: 0,
                                    max_mac: 0,
                                    agility: 0,
                                    accuracy: 0,
                                    armour_rate: 1.0,
                                    damage_rate: 1.0,
                                    magic_resist: 0,
                                    critical_rate: 0,
                                    critical_damage: 0,
                                    luck: 0,
                                    reflect: 0,
                                    damage_reduction_percent: 0,
                                    poison_list: Vec::new(),
            undead: false,
                                    master_session: None,
                                    recall_at_tick: 0,
                                    behavior: ai::make_behavior(&monster_info.name),
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
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id: *session_id,
                                        data: packet.clone(),
                                    }).await;
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
                                    let _ = self.gate_ref.tell(SendToClient {
                                        session_id,
                                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
                                    }).await;
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
                                let _ = self.gate_ref.tell(SendToClient {
                                    session_id,
                                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
                                }).await;
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
                    // ===== 补齐的高频 NPC 指令 =====
                    // MONGEN：在 NPC 位置生成怪物（对齐 C# ActionType.Mongen）
                    "MONGEN" => {
                        let mob_name = parts.next().unwrap_or("").to_string();
                        let count = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        if mob_name.is_empty() { continue; }
                        for _ in 0..count {
                            if let Some(&idx) = self.monster_name_index.get(&mob_name.to_lowercase()) {
                                let info_opt = self.monster_infos.get(&idx).cloned();
                                if let Some(info) = info_opt {
                                    let new_oid = self.alloc_object_id();
                                    let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
                                    let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
                                    let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
                                    let map_index = self.npc_infos.get(&npc.db_index).map(|i| i.map_index as u16).unwrap_or(0);
                                    self.monsters.insert(new_oid, MonsterState {
                                        object_id: new_oid,
                                        name: info.name.clone(),
                                        image: info.image as u16,
                                        monster_index: idx,
                                        x: npc.x, y: npc.y, direction: 0,
                                        hp, max_hp: hp, min_dmg, max_dmg, xp: info.experience,
                                        spawn_x: npc.x, spawn_y: npc.y, map_index,
                                        next_attack_tick: 0, next_move_tick: 0, next_summon_tick: 0,
                                        ai_profile: MonsterAiProfile::from_info(&info),
                                        ai_state: MonsterAiState::Idle,
                                        target_session: None, provoked: false,
                                        is_elite: false, is_boss: false,
                                        min_ac: 0, max_ac: 0, min_mac: 0, max_mac: 0,
                                        agility: 0, accuracy: 0, armour_rate: 1.0, damage_rate: 1.0,
                                        magic_resist: 0, critical_rate: 0, critical_damage: 0,
                                        luck: 0, reflect: 0, damage_reduction_percent: 0,
                                        poison_list: Vec::new(), undead: info.undead,
                                        master_session: None, recall_at_tick: 0,
                                        behavior: ai::make_behavior(&info.name),
                                    });
                                }
                            }
                        }
                        debug!("NPC MONGEN: {} x{} at ({},{})", mob_name, count, npc.x, npc.y);
                    }
                    // CHANGECLASS：转职（对齐 C# ActionType.ChangeClass）
                    "CHANGECLASS" => {
                        let class_id = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        if let Ok(class) = mir2_shared::enums::MirClass::try_from(class_id) {
                            if let Some(record) = self.players.get(&session_id) {
                                let _ = record.actor_ref.ask(crate::actors::player::ChangeClass { class }).await;
                                send_system_message(&self.gate_ref, session_id, &format!("转职成功！"));
                            }
                        }
                    }
                    // CHANGEHAIR：改发型
                    "CHANGEHAIR" => {
                        let hair = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                        if let Some(record) = self.players.get(&session_id) {
                            let _ = record.actor_ref.ask(crate::actors::player::SetHair { hair }).await;
                        }
                    }
                    // TIMERECALL <秒> [section]：延迟执行当前 NPC 脚本段（对齐 C# ActionType.TimeRecall + DelayedAction DelayedType.NPC）
                    "TIMERECALL" => {
                        let secs = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        let section = parts.next()
                            .map(|s| s.trim_matches(|c| c == '[' || c == ']').to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "main".to_string());
                        let expire_tick = self.tick_count.saturating_add(secs * 10);
                        self.npc_delayed_actions.entry(session_id).or_default().push(DelayedNpcAction {
                            expire_tick,
                            npc_object_id: npc.object_id,
                            section: section.clone(),
                            target_db_index: None,
                        });
                        debug!("NPC TIMERECALL: session={} section='{}' in {}s (expire {})", session_id, section, secs, expire_tick);
                    }
                    // TIMERECALLGROUP <秒> [section]：给所有组员注册延迟执行（对齐 C# ActionType.TimeRecallGroup）
                    "TIMERECALLGROUP" => {
                        let secs = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        let section = parts.next()
                            .map(|s| s.trim_matches(|c| c == '[' || c == ']').to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "main".to_string());
                        let gid = if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                state.group_id
                            } else { None }
                        } else { None };
                        let mut targets = vec![session_id];
                        if let Some(gid) = gid {
                            for (sid, r) in &self.players {
                                if *sid == session_id { continue; }
                                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                                    if os.group_id == Some(gid) {
                                        targets.push(*sid);
                                    }
                                }
                            }
                        }
                        let expire_tick = self.tick_count.saturating_add(secs * 10);
                        for sid in &targets {
                            self.npc_delayed_actions.entry(*sid).or_default().push(DelayedNpcAction {
                                expire_tick,
                                npc_object_id: npc.object_id,
                                section: section.clone(),
                                target_db_index: None,
                            });
                        }
                        debug!("NPC TIMERECALLGROUP: session={} targets={} section='{}' in {}s",
                            session_id, targets.len(), section, secs);
                    }
                    // DELAYGOTO <秒> <section>：延迟跳转到脚本段（对齐 C# ActionType.DelayGoto）
                    "DELAYGOTO" => {
                        let secs = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        let section = parts.next()
                            .map(|s| s.trim_matches(|c| c == '[' || c == ']').to_string())
                            .filter(|s| !s.is_empty());
                        match section {
                            Some(section) => {
                                let expire_tick = self.tick_count.saturating_add(secs * 10);
                                self.npc_delayed_actions.entry(session_id).or_default().push(DelayedNpcAction {
                                    expire_tick,
                                    npc_object_id: npc.object_id,
                                    section: section.clone(),
                                    target_db_index: None,
                                });
                                debug!("NPC DELAYGOTO: session={} section='{}' in {}s", session_id, section, secs);
                            }
                            None => warn!("NPC DELAYGOTO: missing section (session={})", session_id),
                        }
                    }
                    // BREAKTIMERECALL：取消该玩家所有 NPC 延迟执行（对齐 C# ActionType.BreakTimeRecall）
                    "BREAKTIMERECALL" => {
                        self.npc_delayed_actions.remove(&session_id);
                        debug!("NPC BREAKTIMERECALL: session={}", session_id);
                    }
                    // GROUPTELEPORT：组队传送（简化：传送玩家 + 同图组员到目标点）
                    "GROUPTELEPORT" => {
                        let tx = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(npc.x);
                        let ty = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(npc.y);
                        if let Some(record) = self.players.get(&session_id) {
                            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                                let gid = state.group_id;
                                let map_idx = state.map_index;
                                // 传送自身
                                let _ = record.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                    x: tx, y: ty, direction: state.direction,
                                    map_index: None, is_mounted: None,
                                }).await;
                                // 传送组员
                                if let Some(gid) = gid {
                                    // 收集同地图的组员
                                    let mut group_sessions = Vec::new();
                                    for (sid, r) in &self.players {
                                        if *sid == session_id { continue; }
                                        if let Ok(Some(st)) = r.actor_ref.ask(GetPlayerState).await {
                                            if st.group_id == Some(gid) && st.map_index == map_idx {
                                                group_sessions.push(*sid);
                                            }
                                        }
                                    }
                                    for sid in group_sessions {
                                        if let Some(r) = self.players.get(&sid) {
                                            let _ = r.actor_ref.ask(crate::actors::player::SetPlayerPosition {
                                                x: tx, y: ty, direction: state.direction,
                                                map_index: None, is_mounted: None,
                                            }).await;
                                        }
                                    }
                                }
                            }
                        }
                        debug!("NPC GROUPTELEPORT to ({},{})", tx, ty);
                    }
                    // ===== NPC 邮件指令（对齐 C# NPCSegment.cs ComposeMail/AddMailGold/AddMailItem/SendMail）=====
                    // 流程：COMPOSEMAIL 创建邮件 → ADDMAILGOLD/ADDMAILITEM 累积附件 → SENDMAIL 发送给收件人
                    // COMPOSEMAIL "正文" 发件人名
                    "COMPOSEMAIL" => {
                        // 正文可能含空格，从 inner 中提取引号内容；发件人取最后一个 token
                        let msg = extract_quoted(inner).unwrap_or_default();
                        let sender = inner.split_whitespace().last().map(|s| s.to_string())
                            .unwrap_or_else(|| "系统".to_string());
                        mail_info = Some(MailMessage {
                            mail_id: generate_mail_id(),
                            sender_name: sender,
                            receiver_name: String::new(),
                            subject: "系统邮件".to_string(),
                            body: msg,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0),
                            read: false,
                            collected: false,
                            locked: false,
                            gold: 0,
                            items: Vec::new(),
                        });
                        debug!("NPC COMPOSEMAIL: staged mail_id from session={}", session_id);
                    }
                    // ADDMAILGOLD amount
                    "ADDMAILGOLD" => {
                        let amount = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        if let Some(m) = mail_info.as_mut() {
                            m.gold = m.gold.saturating_add(amount);
                        }
                    }
                    // ADDMAILITEM item_name count
                    "ADDMAILITEM" => {
                        let item_name = parts.next().unwrap_or("").to_string();
                        let count = parts.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
                        if item_name.is_empty() { continue; }
                        let m = match mail_info.as_mut() { Some(m) => m, None => continue };
                        if m.items.len() >= 5 { continue; } // 附件最多 5 个
                        // 按名查 ItemInfo（线性扫描，item_infos 通常数千条以内）
                        let info_opt = self.item_infos.values()
                            .find(|i| i.name.eq_ignore_ascii_case(&item_name))
                            .cloned();
                        if let Some(info) = info_opt {
                            // 对齐 C# Envir.CreateFreshItem：按 stack_size 拆分堆叠
                            let mut remaining = count;
                            let stack = info.stack_size.max(1) as u16;
                            while remaining > 0 && m.items.len() < 5 {
                                let take = remaining.min(stack);
                                remaining -= take;
                                m.items.push(mir2_shared::data::item::UserItem {
                                    unique_id: generate_item_uid(),
                                    item_index: info.index,
                                    count: take,
                                    current_dura: info.durability as u16,
                                    max_dura: info.durability as u16,
                                    identified: info.is_identified(),
                                    ..Default::default()
                                });
                            }
                        } else {
                            warn!("NPC ADDMAILITEM: item not found: {}", item_name);
                        }
                    }
                    // SENDMAIL recipient_name
                    "SENDMAIL" => {
                        let recipient = parts.next().unwrap_or("").to_string();
                        let mut mail = match mail_info.take() {
                            Some(m) => m, None => {
                                send_system_message(&self.gate_ref, session_id, "请先用 COMPOSEMAIL 撰写邮件");
                                continue;
                            }
                        };
                        if recipient.is_empty() {
                            send_system_message(&self.gate_ref, session_id, "SENDMAIL 缺少收件人");
                            continue;
                        }
                        mail.receiver_name = recipient.clone();
                        // 查找在线收件人（按名，忽略大小写）
                        let target_session = self.find_session_by_name_ignore_case(&recipient).await;
                        if let Some(target) = target_session {
                            // 在线：直接投递
                            if let Some(target_record) = self.players.get(&target) {
                                let _ = target_record.actor_ref.ask(crate::actors::player::AddMail { mail: mail.clone() }).await;
                                send_mail_received_packet(&self.gate_ref, target, &mail);
                                debug!("NPC SENDMAIL delivered online: -> {}", recipient);
                                send_system_message(&self.gate_ref, session_id, "邮件已发送");
                            }
                        } else {
                            // 离线：持久化到数据库（load_mail 在角色登录时读回）
                            if let Err(e) = db::insert_mail(&self.db_pool, &recipient, &mail).await {
                                warn!("NPC SENDMAIL: failed to save offline mail for {}: {}", recipient, e);
                                send_system_message(&self.gate_ref, session_id, "邮件发送失败，请稍后重试");
                            } else {
                                debug!("NPC SENDMAIL saved offline: -> {}", recipient);
                                send_system_message(&self.gate_ref, session_id, "邮件已发送（玩家离线，将在登录时收到）");
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
        let tick_ref = actor_ref.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(args.tick_interval_ms));
            loop {
                interval.tick().await;
                let _ = tick_ref.ask(Tick).await;
                let _ = tick_ref.ask(ProcessDelayedActions).await;
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
        // 建名称→index 缓存（Boss 召唤按名查用，对齐 C# Envir.GetMonsterInfo(name)）
        let monster_name_index: HashMap<String, i32> = monster_infos.iter()
            .map(|(idx, m)| (m.name.to_lowercase(), *idx))
            .collect();
        let item_name_index: HashMap<String, i32> = item_infos.iter()
            .map(|(idx, i)| (i.name.to_lowercase(), *idx))
            .collect();

        // 从 C# Drops/*.txt 导入掉落表（首次运行或 DB 为空时）
        let drop_dir = args.map_dir.join("Envir").join("Drops");
        if drop_dir.exists() {
            if let Err(e) = db::import_drops_from_dir(&drop_dir, &monster_infos, &item_name_index, &args.db_pool).await {
                warn!("Failed to import drops from {}: {}", drop_dir.display(), e);
            }
        }

        let monster_drops = match db::load_monster_drops(&args.db_pool).await {
            Ok(d) => { info!("Loaded drop configs for {} monsters from database", d.len()); d }
            Err(e) => { warn!("Failed to load monster_drops from DB: {}", e); HashMap::new() }
        };

        let npc_infos_list = match db::load_npc_infos(&args.db_pool).await {
            Ok(m) => { info!("Loaded {} NPC configs from database", m.len()); m }
            Err(e) => { warn!("Failed to load npc_infos from DB: {}", e); Vec::new() }
        };
        let npc_infos: HashMap<i32, db::NPCInfo> = npc_infos_list.into_iter().map(|n| (n.index, n)).collect();

        // 从 C# NPCs/*.txt 导入 NPC 脚本（首次运行或 DB 为空时）
        let npc_dir = args.map_dir.join("Envir").join("NPCs");
        if npc_dir.exists() {
            let npc_infos_vec: Vec<db::NPCInfo> = npc_infos.values().cloned().collect();
            if let Err(e) = db::import_npc_scripts_from_dir(&npc_dir, &npc_infos_vec, &args.db_pool).await {
                warn!("Failed to import NPC scripts from {}: {}", npc_dir.display(), e);
            }
        }

        let npc_scripts = match db::load_npc_scripts(&args.db_pool).await {
            Ok(s) => { info!("Loaded {} NPC script pages from database", s.len()); s }
            Err(e) => { warn!("Failed to load npc_scripts from DB: {}", e); HashMap::new() }
        };

        // 从 NPC 脚本的 [Trade] 段导入 NPC 商品（需要 npc_scripts 已加载）
        if let Err(e) = db::import_npc_goods_from_scripts(&args.db_pool, &npc_scripts, &item_name_index).await {
            warn!("Failed to import NPC goods from scripts: {}", e);
        }
        // 重新加载 npc_goods（如果刚导入了数据）
        let npc_goods = match db::load_npc_goods(&args.db_pool).await {
            Ok(g) => { info!("Loaded goods for {} NPCs from database", g.len()); g }
            Err(e) => { warn!("Failed to load npc_goods from DB: {}", e); HashMap::new() }
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

        // 从 C# Recipe/*.txt 导入配方
        let recipe_dir = args.map_dir.join("Envir").join("Recipe");
        if recipe_dir.exists() {
            if let Err(e) = db::import_recipes_from_dir(&recipe_dir, &item_name_index, &args.db_pool).await {
                warn!("Failed to import recipes from {}: {}", recipe_dir.display(), e);
            }
        }

        let recipe_infos = match db::load_recipe_infos(&args.db_pool).await {
            Ok(r) => { info!("Loaded {} craft recipes from database", r.len()); r }
            Err(e) => { warn!("Failed to load recipe_infos from DB: {}", e); Vec::new() }
        };

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
            npc_timers: HashMap::new(),
            session_last_movement: HashMap::new(),
            npc_delayed_actions: HashMap::new(),
            players: HashMap::new(),
            buyback_items: HashMap::new(),
            maps: HashMap::new(),
            gate_ref: args.gate_ref,
            self_ref: Some(actor_ref),
            chat_items_sent: HashMap::new(),
            map_dir: args.map_dir,
            spawn_dir: args.spawn_dir,
            script_dir: args.quest_dir.clone(),
            next_object_id: 1000,
            monsters: HashMap::new(),
            cursed_monsters: HashMap::new(),
            flaming_sword: HashMap::new(),
            double_hit_melee: HashMap::new(),
            mp_eater_count: HashMap::new(),
            hemorrhage_count: HashMap::new(),
            hallucinated: HashMap::new(),
            mental_state: HashMap::new(),
            fatal_sword_armed: HashSet::new(),
            boss_pending_attacks: Vec::new(),
            pet_enhanced: HashMap::new(),
            pet_levels: HashMap::new(),
            revealed_hp: HashMap::new(),
            pet_targets: HashMap::new(),
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
            monster_name_index,
            recipe_infos,
            monster_drops,
            npc_infos,
            npc_goods,
            session_npc: HashMap::new(),
            npc_scripts,
            quest_infos,
            magic_infos,
            dragon_info,
            game_shop_items,
            movement_index,
            social_ref: args.social_ref,
            conquest_cfg: args.conquest_cfg,
            rested_cfg: args.rested_cfg,
            drop_rate: args.drop_rate,
            item_timeout_ticks: args.item_timeout_ticks,
            max_drop_gold: args.max_drop_gold,
            rarity_cfg: args.rarity_cfg,
            notice_path: args.notice_path.clone(),
            death_exp_penalty_percent: args.death_exp_penalty_percent,
            health_regen_weight: args.health_regen_weight,
            mana_regen_weight: args.mana_regen_weight,
            goods_hide_added_stats: args.goods_hide_added_stats,
            global_exp_multiplier: 1.0,
            global_drop_multiplier: 1.0,
            global_gold_multiplier: 1.0,
            global_exp_event_end_tick: 0,
            global_event_name: None,
            invisible_sessions: HashSet::new(),
            last_move_time: std::collections::HashMap::new(),
            current_light: Self::light_for_hour(chrono::Local::now().hour()),
            auctions,
            next_auction_id,
            market_search_cache: HashMap::new(),
            rental_sessions: HashMap::new(),
            player_rentals: HashMap::new(),
            spell_objects: HashMap::new(),
            pending_spell_completions: Vec::new(),
            vamp_heals: Vec::new(),
            robot_tasks: Vec::new(),
            robot_last_check_minute: 0,
            dragon_state: None,
            conquest_instances: default_conquest_instances(),
            siege_structures: HashMap::new(),
            guild_wars: HashMap::new(),
            hero_ai_states: HashMap::new(),
            player_heroes: HashMap::new(),
        })
    }
}

/// 默认行会领地实例（conquest_infos 表为空时种子；对应 C# Envir 默认沙巴克等 8 个领地）
/// M36：领地列表/购买 E2E 用；购买仅内存态（服务端暂无持久化）
fn default_conquest_instances() -> Vec<conquest::ConquestInstance> {
    (1..=8)
        .map(|i| conquest::ConquestInstance::new(i, 0, 0, conquest::ConquestGame::ControlPoints))
        .collect()
}

/// 市场搜索缓存
#[derive(Debug, Clone)]
pub(crate) struct MarketSearchCache {
    results: Vec<usize>, // indices into self.auctions
}

impl WorldActor {
    fn send_awakening_result(&self, session_id: u64, result: i32, remove_id: i64) {
        let packet = mir2_shared::packets::server::awakening_system::Awakening { result, remove_id };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize Awakening result: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Awakening as i16, &body),
        }).try_send();
    }
}

impl WorldActor {
    fn send_rental_packet<T: mir2_shared::packets::Packet>(&self, session_id: u64, packet: T) {
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize rental packet: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id,
            data: build_packet_bytes(T::OPCODE, &body),
        }).try_send();
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

    /// 按名查找在线玩家 session（忽略大小写）。
    /// 用于 NPC SENDMAIL 等需要宽松匹配收件人的场景。
    /// 按怪物名在指定地图 (x,y) 刷 count 只怪物（NPC 脚本 MONGEN 等用，对齐 C# Envir.GetMonsterInfo + MonsterObject.Spawn）
    /// 返回成功刷出的数量；怪物名不存在或信息缺失返回 0
    pub(crate) async fn spawn_monster_named(
        &mut self,
        name: &str,
        x: i32,
        y: i32,
        count: u32,
        map_index: u16,
    ) -> usize {
        let Some(&idx) = self.monster_name_index.get(&name.to_lowercase()) else {
            warn!("spawn_monster_named: monster '{}' not found", name);
            return 0;
        };
        let Some(info) = self.monster_infos.get(&idx).cloned() else {
            warn!("spawn_monster_named: monster info '{}' missing", name);
            return 0;
        };
        let hp = info.stats.get(&(mir2_shared::enums::Stat::HP as u8)).copied().unwrap_or(50);
        let min_dmg = info.stats.get(&(mir2_shared::enums::Stat::MinDC as u8)).copied().unwrap_or(5);
        let max_dmg = info.stats.get(&(mir2_shared::enums::Stat::MaxDC as u8)).copied().unwrap_or(10);
        let mut spawned = 0usize;
        for _ in 0..count.min(50) {
            let oid = self.alloc_object_id();
            let packet = build_object_monster_packet(
                &MonsterSpawn {
                    name: info.name.clone(),
                    image: info.image as u16,
                    monster_index: idx,
                    x,
                    y,
                    direction: 0,
                    hp,
                    min_dmg,
                    max_dmg,
                    xp: info.experience,
                    map_index,
                },
                oid,
                &info.name,
            );
            // 广播给同地图在线玩家（避免跨图玩家看到不该出现的怪）
            let online: Vec<u64> = self.players.keys().copied().collect();
            for sid in online {
                if let Some(record) = self.players.get(&sid) {
                    if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                        if st.map_index == map_index {
                            let _ = self.gate_ref.tell(SendToClient { session_id: sid, data: packet.clone() }).await;
                        }
                    }
                }
            }
            self.monsters.insert(oid, MonsterState {
                object_id: oid,
                name: info.name.clone(),
                image: info.image as u16,
                monster_index: idx,
                x,
                y,
                direction: 0,
                hp,
                max_hp: hp,
                min_dmg,
                max_dmg,
                xp: info.experience,
                spawn_x: x,
                spawn_y: y,
                map_index,
                next_attack_tick: 0,
                next_move_tick: 0,
                next_summon_tick: 0,
                ai_profile: MonsterAiProfile::from_info(&info),
                ai_state: MonsterAiState::Idle,
                target_session: None,
                provoked: false,
                is_elite: false,
                is_boss: false,
                min_ac: 0,
                max_ac: 0,
                min_mac: 0,
                max_mac: 0,
                agility: 0,
                accuracy: 0,
                armour_rate: 1.0,
                damage_rate: 1.0,
                magic_resist: 0,
                critical_rate: 0,
                critical_damage: 0,
                luck: 0,
                reflect: 0,
                damage_reduction_percent: 0,
                poison_list: Vec::new(),
                undead: info.undead,
                master_session: None,
                recall_at_tick: 0,
                behavior: ai::make_behavior(&info.name),
            });
            spawned += 1;
        }
        debug!("spawn_monster_named: '{}' x{} at ({},{}) map {} spawned={}", name, count, x, y, map_index, spawned);
        spawned
    }

    /// NPC 脚本延迟执行到期处理（对齐 C# DelayedAction DelayedType.NPC：到点执行脚本段）
    pub(crate) async fn process_delayed_actions(&mut self) {
        let now = self.tick_count;
        // 收集到点动作，避免在遍历时借用 self
        let mut due: Vec<(u64, DelayedNpcAction)> = Vec::new();
        for (session_id, actions) in &self.npc_delayed_actions {
            for act in actions {
                if act.expire_tick <= now {
                    due.push((*session_id, act.clone()));
                }
            }
        }
        if due.is_empty() {
            return;
        }
        // 移除到点动作
        for (session_id, _act) in &due {
            if let Some(actions) = self.npc_delayed_actions.get_mut(session_id) {
                actions.retain(|a| a.expire_tick > now);
            }
        }
        // 逐个执行
        for (session_id, act) in due {
            let Some(npc) = self.npcs.get(&act.npc_object_id).cloned() else { continue };
            // CALL 用 target_db_index 覆盖（目标脚本是另一个 NPC），否则用 npc.db_index
            let db_index = act.target_db_index.unwrap_or(npc.db_index);
            let section_upper = act.section.to_uppercase();
            let script_key = (db_index, section_upper.clone());
            let Some(lines) = self.npc_scripts.get(&script_key).cloned() else { continue };
            let joined = lines.join("\n");
            if !npc_script::is_csharp_format(&joined) {
                // 旧 <CMD> 格式：延迟到期后用旧引擎执行该页（对齐 C# DelayedAction 到点执行脚本页）
                let mut lines = lines;
                let (name, level) = match self.players.get(&session_id) {
                    Some(r) => match r.actor_ref.ask(GetPlayerState).await {
                        Ok(Some(st)) => (st.name.clone(), st.level),
                        _ => (String::new(), 0),
                    },
                    None => (String::new(), 0),
                };
                for line in &mut lines {
                    *line = line.replace("$USERNAME", &name)
                                .replace("$NPCNAME", &npc.name)
                                .replace("$LEVEL", &level.to_string());
                }
                // eval_npc_script 的 Future 极大，直接内联会把 tick 任务栈打爆；Box::pin 放到堆上
                let (out, _goto) = Box::pin(async { self.eval_npc_script(&mut lines, session_id, &npc).await }).await;
                if !out.is_empty() {
                    let mut body = Vec::new();
                    body.extend_from_slice(&(out.len() as i32).to_le_bytes());
                    for line in &out {
                        write_dotnet_string(&mut body, line);
                    }
                    let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCResponse as i16, &body);
                    let _ = self.gate_ref.tell(SendToClient { session_id, data: packet }).await;
                    debug!("NPC delayed action (old format) fired: session={} npc={} section='{}' say_lines={}",
                           session_id, npc.name, act.section, out.len());
                }
                continue;
            }
            let parsed = npc_script::ParsedScript::parse(&joined);
            let target = parsed
                .find(&act.section)
                .or_else(|| parsed.find(&section_upper))
                .or_else(|| parsed.main_section());
            let Some(section) = target else { continue };
            let mut custom_vars: HashMap<String, String> = HashMap::new();
            let res = parsed.execute_section(section, self, session_id, &npc, &mut custom_vars).await;
            debug!("NPC delayed action fired: session={} npc={} section='{}' say_lines={} goto={:?}",
                   session_id, npc.name, act.section, res.say_lines.len(), res.goto);
        }
    }

    /// NPC 脚本计时器到期清理（对齐 C# Envir.Timers 到期移除；无自动执行，脚本用 CHECKTIMER 轮询）
    pub(crate) fn tick_npc_timers(&mut self) {
        let now = self.tick_count;
        for timers in self.npc_timers.values_mut() {
            timers.retain(|_, expire| *expire > now);
        }
    }

    /// 清除指定地图所有存活怪物（NPC 脚本 MONCLEAR，对齐 C# ActionType.MonClear：怪物 Die + 广播）
    /// 返回清除数量
    pub(crate) async fn clear_monsters_on_map(&mut self, map_index: u16) -> usize {
        let to_clear: Vec<u32> = self.monsters.iter()
            .filter(|(_, m)| m.map_index == map_index)
            .map(|(id, _)| *id)
            .collect();
        let mut cleared = 0usize;
        for oid in to_clear {
            if let Some(monster) = self.monsters.get(&oid) {
                let died = Self::build_object_died_packet(oid, monster.x, monster.y, monster.direction);
                let remove = Self::build_object_remove_packet(oid);
                let online: Vec<u64> = self.players.keys().copied().collect();
                for session_id in online {
                    let _ = self.gate_ref.tell(SendToClient { session_id, data: died.clone() }).await;
                    let _ = self.gate_ref.tell(SendToClient { session_id, data: remove.clone() }).await;
                }
            }
            self.monsters.remove(&oid);
            self.respawn_queue.remove(&oid);
            self.world_boss_queue.remove(&oid);
            self.cursed_monsters.remove(&oid);
            cleared += 1;
        }
        debug!("MONCLEAR: cleared {} monsters on map {}", cleared, map_index);
        cleared
    }

    async fn find_session_by_name_ignore_case(&self, name: &str) -> Option<u64> {
        for (sid, record) in &self.players {
            if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                if state.name.eq_ignore_ascii_case(name) {
                    return Some(*sid);
                }
            }
        }
        None
    }

    /// 重新计算装备属性加成并设置到 PlayerActor
    /// 返回最新的 PlayerState（如果成功）
    pub(crate) async fn recalculate_and_set_stat_bonuses(&self, session_id: u64) -> Option<PlayerState> {
        let record = self.players.get(&session_id)?;
        let state = record.actor_ref.ask(GetPlayerState).await.ok()??;
        let b = calculate_equipment_bonuses(&state.inventory.equipment, &self.item_infos);
        let _ = record.actor_ref.ask(crate::actors::player::SetStatBonuses {
            bonus_min_attack: b.min_atk,
            bonus_max_attack: b.max_atk,
            bonus_defence: b.max_ac, // 保留旧字段兼容（defence 用 AC）
            bonus_max_hp: b.hp,
            bonus_max_mp: b.mp,
            bonus_min_mc: b.min_mc,
            bonus_max_mc: b.max_mc,
            bonus_min_sc: b.min_sc,
            bonus_max_sc: b.max_sc,
            // 战斗公式扩展字段
            bonus_min_ac: b.min_ac,
            bonus_max_ac: b.max_ac,
            bonus_min_mac: b.min_mac,
            bonus_max_mac: b.max_mac,
            luck: b.luck,
            critical_rate: b.critical_rate,
            critical_damage: b.critical_damage,
            magic_resist: b.magic_resist,
            reflect: b.reflect,
            attack_bonus: b.attack_bonus,
            hp_drain_rate_percent: b.hp_drain_rate_percent,
            agility: b.agility,
            accuracy: b.accuracy,
            freezing: b.freezing,
            poison_attack: b.poison_attack,
            health_recovery: b.health_recovery,
            spell_recovery: b.spell_recovery,
            attack_speed: b.attack_speed,
            poison_resist: b.poison_resist,
        }).await;
        Some(state)
    }

    /// 广播装备视觉变化给同地图其他玩家
    pub(crate) async fn broadcast_equipment_visuals(&self, session_id: u64, state: &PlayerState) {
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
        for other in self.other_players(session_id) {
            send_player_update(
                &self.gate_ref, other.session_id, state.object_id,
                light, weapon_shape, weapon_effect, armor_shape, 0,
            ).await;
        }
    }

    /// #285：聊天物品链接 —— 解析 `%名字#uid%` 并在发送方背包/装备中查找，
    /// 向所有在线玩家（含自己）推送 S.NewChatItem（按会话去重，C# SentChatItem）
    pub(crate) async fn send_chat_item_links(&mut self, session_id: u64, message: &str) {
        let mut uids: Vec<u64> = Vec::new();
        let mut i = 0;
        while i < message.len() {
            let bytes = message.as_bytes();
            if bytes[i] == b'%' {
                if let Some(rel) = message[i + 1..].find('%') {
                    let inner = &message[i + 1..i + 1 + rel];
                    if let Some(hash) = inner.rfind('#') {
                        if let Ok(uid) = inner[hash + 1..].trim().parse::<u64>() {
                            uids.push(uid);
                        }
                    }
                    i += rel + 2;
                    continue;
                }
            }
            i += 1;
        }
        if uids.is_empty() {
            return;
        }
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // 在发送方背包/装备中按 unique_id 查找（C# PlayerObject 聊天链接查 Inventory）
        let mut items: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        let mut push_item = |item: &mir2_shared::data::item::UserItem| {
            if uids.contains(&item.unique_id)
                && !items.iter().any(|it| it.unique_id == item.unique_id)
            {
                let mut it = item.clone();
                enrich_item_info(&mut it, &self.item_infos);
                items.push(it);
            }
        };
        for slot in state.inventory.backpack.iter() {
            if let Some(s) = slot {
                push_item(&s.item);
            }
        }
        for eq in state.inventory.equipment.iter() {
            if let Some(it) = eq {
                push_item(it);
            }
        }
        if items.is_empty() {
            return;
        }
        for item in &items {
            let mut body = Vec::new();
            if mir2_shared::packets::base::serialize_packet(
                &mut std::io::Cursor::new(&mut body),
                &mir2_shared::packets::server::NewChatItem { item: item.clone() },
            )
            .is_err()
            {
                continue;
            }
            let data = body;
            for sid in self.players.keys() {
                let sent = self.chat_items_sent.entry(*sid).or_default();
                if sent.insert(item.unique_id) {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *sid,
                        data: data.clone(),
                    }).await;
                }
            }
        }
        info!("Chat item links sent for session {}: {:?}", session_id, uids);
    }

/// 通过 object_id 查找玩家
    #[allow(dead_code)]
    pub(crate) async fn find_player_by_object_id(&self, target_id: u32) -> Option<PlayerState> {
        for r in self.players.values() {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.object_id == target_id {
                    return Some(s);
                }
            }
        }
        None
    }

    /// 构建 ObjectDied 数据包
    pub(crate) fn build_object_died_packet(object_id: u32, x: i32, y: i32, direction: u8) -> Vec<u8> {
        let mut body = Vec::with_capacity(14);
        body.extend_from_slice(&object_id.to_le_bytes());
        body.extend_from_slice(&(x as u32).to_le_bytes());
        body.extend_from_slice(&(y as u32).to_le_bytes());
        body.push(direction);
        body.push(0u8);
        build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectDied as i16, &body)
    }

    /// 构建 ObjectRemove 数据包
    pub(crate) fn build_object_remove_packet(object_id: u32) -> Vec<u8> {
        let body = object_id.to_le_bytes().to_vec();
        build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectRemove as i16, &body)
    }
}
/// #283：玩家升级 → 向同图其他玩家广播 ObjectLeveled（C# 升级表现）
pub struct PlayerLeveled {
    pub session_id: u64,
    pub object_id: u32,
    pub level: u16,
}

impl Message<PlayerLeveled> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: PlayerLeveled, _ctx: &mut Context<Self, Self::Reply>) {
        for other in self.other_players(msg.session_id) {
            let mut body = Vec::new();
            body.extend_from_slice(&msg.object_id.to_le_bytes());
            body.extend_from_slice(&msg.level.to_le_bytes());
            let _ = self.gate_ref.tell(SendToClient {
                session_id: other.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ObjectLeveled as i16, &body),
            }).await;
        }
        info!("Player {} leveled to {} broadcast", msg.object_id, msg.level);
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

/// 从 NPC 指令 inner 文本中提取第一段引号内容。
/// 对齐 C# NPCSegment.cs 中 COMPOSEMAIL 用 regexQuote 匹配 "..." 的逻辑。
/// 支持 "..." 与 '...'；未找到返回 None。
fn extract_quoted(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut start = None;
    let quote = b'"';
    for (i, &b) in bytes.iter().enumerate() {
        if b == quote {
            if start.is_none() {
                start = Some(i + 1);
            } else {
                // 闭合引号
                let s_start = start.unwrap();
                return Some(s[s_start..i].to_string());
            }
        }
    }
    None
}

pub(crate) fn send_system_message(gate_ref: &ActorRef<GateActor>, session_id: u64, message: &str) {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    crate::util::wire::write_dotnet_string(&mut body, message);
    body.push(mir2_shared::enums::ChatType::System as u8); // ChatType::System=5（SharedRust 枚举与 C# 差 3）
    let packet = build_packet_bytes(ServerPacketIds::Chat as i16, &body);
    let gate_ref = gate_ref.clone();
    tokio::spawn(async move {
        let _ = gate_ref.tell(SendToClient { session_id, data: packet }).await;
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
    let gate_ref = gate_ref.clone();
    let session_ids: Vec<u64> = players.keys().copied().collect();
    tokio::spawn(async move {
        for session_id in session_ids {
            let _ = gate_ref.tell(SendToClient {
                session_id,
                data: packet.clone(),
            }).await;
        }
    });
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
async fn send_opendoor(gate_ref: &ActorRef<GateActor>, session_id: u64, door_index: u8, close: bool) {
    let mut body = Vec::new();
    body.push(door_index);
    body.push(if close { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Opendoor as i16, &body),
    }).await;
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
            let _ = gate_ref.tell(SendToClient {
                session_id: record.session_id,
                data: packet.clone(),
            }).await;
        }
    }
}

fn send_move_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, from: i32, to: i32, success: bool) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&from.to_le_bytes());
    body.extend_from_slice(&to.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MoveItem as i16, &body),
    }).try_send();
}

fn send_use_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, uid: u64) {
    let mut body = Vec::new();
    body.extend_from_slice(&uid.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UseItem as i16, &body),
    }).try_send();
}

/// 计算装备属性加成总和
/// 装备属性加成汇总（从 ItemInfo.stats 累加所有装备）
#[derive(Debug, Default, Clone, Copy)]
pub struct EquipmentBonuses {
    pub min_atk: i32, pub max_atk: i32,
    pub min_mc: i32, pub max_mc: i32,
    pub min_sc: i32, pub max_sc: i32,
    pub min_ac: i32, pub max_ac: i32,
    pub min_mac: i32, pub max_mac: i32,
    pub hp: i32, pub mp: i32,
    pub luck: i32,
    pub critical_rate: i32, pub critical_damage: i32,
    pub magic_resist: i32, pub reflect: i32,
    pub attack_bonus: i32, pub hp_drain_rate_percent: i32,
    pub agility: i32, pub accuracy: i32,
    pub freezing: i32, pub poison_attack: i32,
    pub health_recovery: i32, pub spell_recovery: i32,
    pub attack_speed: i32, pub poison_resist: i32,
}

fn calculate_equipment_bonuses(
    equipment: &[Option<mir2_shared::data::item::UserItem>],
    item_infos: &std::collections::HashMap<i32, crate::db::ItemInfo>,
) -> EquipmentBonuses {
    use mir2_shared::enums::Stat;
    let mut b = EquipmentBonuses::default();

    for eq in equipment.iter().flatten() {
        if let Some(info) = item_infos.get(&eq.item_index) {
            let get = |s: Stat| info.stats.get(&(s as u8)).copied().unwrap_or(0);
            b.min_atk += get(Stat::MinDC);
            b.max_atk += get(Stat::MaxDC);
            b.min_mc += get(Stat::MinMC);
            b.max_mc += get(Stat::MaxMC);
            b.min_sc += get(Stat::MinSC);
            b.max_sc += get(Stat::MaxSC);
            b.min_ac += get(Stat::MinAC);
            b.max_ac += get(Stat::MaxAC);
            b.min_mac += get(Stat::MinMAC);
            b.max_mac += get(Stat::MaxMAC);
            b.hp += get(Stat::HP);
            b.mp += get(Stat::MP);
            b.luck += get(Stat::Luck);
            b.critical_rate += get(Stat::CriticalRate);
            b.critical_damage += get(Stat::CriticalDamage);
            b.magic_resist += get(Stat::MagicResist);
            b.reflect += get(Stat::Reflect);
            b.attack_bonus += get(Stat::AttackBonus);
            b.hp_drain_rate_percent += get(Stat::HPDrainRatePercent);
            b.agility += get(Stat::Agility);
            b.accuracy += get(Stat::Accuracy);
            b.freezing += get(Stat::Freezing);
            b.poison_attack += get(Stat::PoisonAttack);
            b.health_recovery += get(Stat::HealthRecovery);
            b.spell_recovery += get(Stat::SpellRecovery);
            b.attack_speed += get(Stat::AttackSpeed);
            b.poison_resist += get(Stat::PoisonResist);
        }
    }
    b
}

fn send_equip_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, uid: u64, slot: i32, success: bool) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&slot.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::EquipItem as i16, &body),
    }).try_send();
}

fn send_remove_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, uid: u64, success: bool) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RemoveItem as i16, &body),
    }).try_send();
}

fn send_drop_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, uid: u64, count: u32, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&count.to_le_bytes()); // DropItem 包：count 是 u32
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::DropItem as i16, &body),
    }).try_send();
}

fn send_merge_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid_from: u8, grid_to: u8, from_uid: u64, to_uid: u64, success: bool) {
    let mut body = Vec::new();
    body.push(grid_from);
    body.push(grid_to);
    body.extend_from_slice(&from_uid.to_le_bytes());
    body.extend_from_slice(&to_uid.to_le_bytes());
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MergeItem as i16, &body),
    }).try_send();
}

fn send_split_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, grid: u8, uid: u64, count: u32) {
    let mut body = Vec::new();
    body.push(grid);
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&(count as u16).to_le_bytes()); // SellItem 包：count 是 u16（与 SharedRust 一致）
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SplitItem as i16, &body),
    }).try_send();
}

fn send_sell_item_response(gate_ref: &ActorRef<GateActor>, session_id: u64, uid: u64, count: u32, success: bool) {
    let mut body = Vec::new();
    body.extend_from_slice(&uid.to_le_bytes());
    body.extend_from_slice(&(count as u16).to_le_bytes()); // SellItem 包：count 是 u16（与 SharedRust 一致）
    body.push(if success { 1u8 } else { 0u8 });
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::SellItem as i16, &body),
    }).try_send();
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
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ReceiveMail as i16, &body),
    }).try_send();
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
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ReceiveMail as i16, &body),
    }).try_send();
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

    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::PlayerInspect as i16, &body),
    }).try_send();
}

// ============================================================
// 任务系统网络辅助函数
// ============================================================

fn send_quest_complete_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, quest_index: i32) {
    let mut body = Vec::new();
    body.extend_from_slice(&quest_index.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::CompleteQuest as i16, &body),
    }).try_send();
}

// ============================================================
// 英雄系统网络辅助函数
// ============================================================

fn send_hero_update_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, hero_index: u8) {
    let body = vec![hero_index];
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ChangeHero as i16, &body),
    }).try_send();
}

// ============================================================
// 仓库/金币网络辅助函数
// ============================================================


fn send_gold_changed_packet(gate_ref: &ActorRef<GateActor>, session_id: u64, amount: u64) {
    // C# S.LoseGold.Gold = 扣减金额，不是扣后总额
    let mut body = Vec::new();
    body.extend_from_slice(&(amount as u32).to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LoseGold as i16, &body),
    }).try_send();
}

/// 下发 S.ManageHeroes（C# ManageHeroes：max_count + current_hero + heroes，#188）
pub(crate) fn send_manage_heroes_packet(
    gate_ref: &ActorRef<GateActor>,
    session_id: u64,
    state: &PlayerState,
    heroes: &[HeroInfo],
) {
    let to_info = |h: &HeroInfo| mir2_shared::data::client_data::ClientHeroInformation {
        index: h.index,
        name: h.name.clone(),
        level: h.level,
        class: h.class,
        gender: h.gender,
    };
    let current_hero = heroes.iter().find(|h| h.index as u8 == state.hero_index).map(to_info);
    let list: Vec<mir2_shared::data::client_data::ClientHeroInformation> =
        heroes.iter().map(to_info).collect();
    let packet = mir2_shared::packets::server::hero::ManageHeroes {
        max_count: 1,
        current_hero,
        heroes: list,
    };
    let mut body = Vec::new();
    if packet.write_body(&mut body).is_err() {
        warn!("Failed to serialize ManageHeroes");
        return;
    }
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ManageHeroes as i16, &body),
    }).try_send();
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
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UpdateIntelligentCreatureList as i16, &body),
    }).try_send();
}

// ============================================================
// 游戏进入序列
// ============================================================

/// 发送完整的游戏进入序列到客户端
async fn send_game_entry_sequence(
    gate_ref: ActorRef<GateActor>,
    session_id: u64,
    state: &PlayerState,
    map_file: &str,
    map_title: &str,
    is_big_map: bool,
    item_infos: &std::collections::HashMap<i32, db::ItemInfo>,
) {
    use mir2_shared::enums::ServerPacketIds;

    let sid = session_id;

    // 1. StartGame (result=4=Success, resolution=0)
    let mut start_game_body = Vec::new();
    start_game_body.push(4u8);
    start_game_body.extend_from_slice(&0i32.to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::StartGame as i16, &start_game_body),
    }).await;

    // 2. MapChanged
    let map_changed = build_map_changed_packet(state.map_index, map_file, map_title, state.x, state.y, is_big_map);
    let _ = gate_ref.tell(SendToClient {
        session_id: sid,
        data: map_changed,
    }).await;

    // 3. UserInformation（含背包/装备 ItemInfo）
    let user_info = build_user_information_packet(state, item_infos);
    let _ = gate_ref.tell(SendToClient {
        session_id: sid,
        data: user_info,
    }).await;

    // 4. HealthChanged
    let mut health_body = Vec::new();
    health_body.extend_from_slice(&(state.hp as u32).to_le_bytes());
    health_body.extend_from_slice(&(state.mp as u32).to_le_bytes());
    let _ = gate_ref.tell(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::HealthChanged as i16, &health_body),
    }).await;

    // 4.5 任务日志推送（M43：C# S.ChangeQuest 语义，登录同步已接任务）
    for quest in &state.quest_log.quests {
        if quest.status == crate::actors::quest::QuestStatus::Accepted
            || quest.status == crate::actors::quest::QuestStatus::InProgress
        {
            crate::actors::social_packets::send_quest_change_packet(&gate_ref, session_id, quest);
        }
    }

    // 5. UserLocation
    let mut location_body = Vec::new();
    location_body.extend_from_slice(&state.x.to_le_bytes());
    location_body.extend_from_slice(&state.y.to_le_bytes());
    location_body.push(state.direction);
    let _ = gate_ref.tell(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &location_body),
    }).await;

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
/// 构建 NewMapInfo 包（M53：大地图地图信息，含 NPC 列表）
/// wire 对齐 SharedRust NewMapInfo：map_index/title/width/height/big_map/movements/npcs
fn build_new_map_info_packet(
    map_index: i32,
    title: &str,
    npcs: &[NpcState],
    npc_infos: &std::collections::HashMap<i32, db::NPCInfo>,
) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&map_index.to_le_bytes());
    write_dotnet_string(&mut body, title);
    // width/height/big_map：客户端本地有地图数据，服务端补 0
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    // movements：暂无传送点数据，发空
    body.extend_from_slice(&0i32.to_le_bytes());
    // npcs
    body.extend_from_slice(&(npcs.len() as i32).to_le_bytes());
    for n in npcs {
        body.extend_from_slice(&n.object_id.to_le_bytes());
        write_dotnet_string(&mut body, &n.name);
        body.extend_from_slice(&n.x.to_le_bytes());
        body.extend_from_slice(&n.y.to_le_bytes());
        let icon = npc_infos.get(&n.db_index).map(|i| i.big_map_icon).unwrap_or(0);
        body.extend_from_slice(&icon.to_le_bytes());
        let can_tp = npc_infos.get(&n.db_index).map(|i| i.can_teleport_to).unwrap_or(true);
        body.push(if can_tp { 1u8 } else { 0u8 });
    }

    build_packet_bytes(ServerPacketIds::NewMapInfo as i16, &body)
}

/// 传送 NPC 费用（C# Settings.TeleportToNPCCost）
pub(crate) const TELEPORT_TO_NPC_COST: i32 = 1000;

/// 构建 S.WorldMapSetupInfo（C# 线格式：enabled/count/icons/teleport_cost）
/// icons 取 DB 中 big_map=true 的地图（前 64 个，image_index 用 MapLinkIcon 帧序号）
pub(crate) fn build_world_map_setup_packet(
    map_infos: &std::collections::HashMap<i32, db::MapInfo>,
    teleport_cost: i32,
) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    let icons: Vec<&db::MapInfo> = map_infos.values().filter(|m| m.big_map).take(64).collect();
    body.push(1u8); // enabled
    body.extend_from_slice(&(icons.len() as i32).to_le_bytes());
    for (i, m) in icons.iter().enumerate() {
        body.extend_from_slice(&(i as i32).to_le_bytes());
        write_dotnet_string(&mut body, &m.title);
        body.extend_from_slice(&m.index.to_le_bytes());
    }
    body.extend_from_slice(&teleport_cost.to_le_bytes());
    build_packet_bytes(ServerPacketIds::WorldMapSetup as i16, &body)
}

/// 按 DB NPCInfo 构建 NewMapInfo（C.RequestMapInfo 用，C# CheckMapInfo 语义）
pub(crate) fn build_new_map_info_packet_from_db(
    map_index: i32,
    title: &str,
    npcs: &[db::NPCInfo],
) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();
    body.extend_from_slice(&map_index.to_le_bytes());
    write_dotnet_string(&mut body, title);
    // width/height/big_map：客户端本地有地图数据，服务端补 0
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    // movements：暂无传送点数据，发空
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&(npcs.len() as i32).to_le_bytes());
    for n in npcs {
        body.extend_from_slice(&(n.index as u32).to_le_bytes());
        write_dotnet_string(&mut body, &n.name);
        body.extend_from_slice(&n.x.to_le_bytes());
        body.extend_from_slice(&n.y.to_le_bytes());
        body.extend_from_slice(&n.big_map_icon.to_le_bytes());
        body.push(if n.can_teleport_to { 1u8 } else { 0u8 });
    }
    build_packet_bytes(ServerPacketIds::NewMapInfo as i16, &body)
}

/// 根据 PK 值计算名字颜色（0=白名, 1=红名, 2=橙名）
pub(crate) fn name_colour_for_pk(pk_points: i32) -> i32 {
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

/// C# ItemType → SharedRust ItemType（两者编号差 3：Nothing=0→3, Weapon=1→4 ...）
fn shared_item_type(cs_type: i32) -> mir2_shared::enums::ItemType {
    mir2_shared::enums::ItemType::try_from((cs_type + 3) as u8)
        .unwrap_or(mir2_shared::enums::ItemType::Nothing)
}

/// 给物品补 ItemInfo（DB 配置 → SharedRust，编号差 3；已有 info 则跳过）
pub(crate) fn enrich_item_info(
    item: &mut mir2_shared::data::item::UserItem,
    item_infos: &std::collections::HashMap<i32, db::ItemInfo>,
) {
    if item.info.is_some() {
        return;
    }
    item.info = item_infos.get(&item.item_index).map(|info| mir2_shared::data::item::ItemInfo {
        index: info.index,
        name: info.name.clone(),
        item_type: shared_item_type(info.item_type),
        // SharedRust 枚举从 3 开始（C# 从 0 开始），默认值 0 会让客户端 try_from 失败
        grade: mir2_shared::enums::ItemGrade::try_from((info.grade + 3) as u8)
            .unwrap_or(mir2_shared::enums::ItemGrade::None),
        required_type: mir2_shared::enums::RequiredType::try_from((info.required_type + 3) as u8)
            .unwrap_or(mir2_shared::enums::RequiredType::Level),
        required_class: mir2_shared::enums::RequiredClass::from_bits_truncate(
            info.required_class as u8,
        ),
        required_gender: mir2_shared::enums::RequiredGender::from_bits_truncate(
            info.required_gender as u8,
        ),
        set: mir2_shared::enums::ItemSet::try_from((info.set_type + 3) as u8)
            .unwrap_or(mir2_shared::enums::ItemSet::None),
        // C# SpecialItemMode 位值与 SharedRust 一致（如 Revival=0x10），无需 +3；共享字段名 unique
        unique: mir2_shared::enums::SpecialItemMode::from_bits_truncate(info.special_mode as u16),
        shape: info.shape as i16,
        weight: info.weight as u8,
        light: info.light as u8,
        required_amount: info.required_amount as u8,
        image: info.image as u16,
        durability: info.durability as u16,
        price: info.price,
        stack_size: info.stack_size as u16,
        start_item: info.start_item,
        effect: info.effect as u8,
        // C# ItemInfo bools 位：0x01 NeedIdentify / 0x02 ShowGroupPickup / 0x04 ClassBased / 0x08 LevelBased / 0x10 CanMine / 0x20 GlobalDropNotify
        need_identify: (info.bool_flags & 0x01) != 0,
        show_group_pickup: (info.bool_flags & 0x02) != 0,
        class_based: (info.bool_flags & 0x04) != 0,
        level_based: (info.bool_flags & 0x08) != 0,
        can_mine: (info.bool_flags & 0x10) != 0,
        global_drop_notify: (info.bool_flags & 0x20) != 0,
        can_fast_run: info.can_fast_run,
        can_awakening: info.can_awakening,
        // C# BindMode 位值与 SharedRust 一致（DontDeathdrop=0x1 等），无需 +3
        bind: mir2_shared::enums::BindMode::from_bits_truncate(info.bind_mode as u16),
        random_stats_id: info.random_stats_id as u8,
        slots: info.slots as u8,
        tool_tip: info.tool_tip.clone(),
        // DB stats 已在加载层 +3 转 SharedRust key；转成共享 Stats
        stats: {
            let mut s = mir2_shared::data::stats::Stats::new();
            for (k, v) in &info.stats {
                if let Ok(stat) = mir2_shared::enums::Stat::try_from(*k) {
                    s.set(stat, *v);
                }
            }
            s
        },
        ..Default::default()
    });
}

/// PlayerMagic + magic_infos → 客户端 ClientMagic（#212；DB 用 C# 编号，客户端用 SharedRust +3）
pub(crate) fn build_client_magic(
    info: &db::MagicInfo,
    magic: &crate::actors::player::PlayerMagic,
) -> mir2_shared::data::client_data::ClientMagic {
    mir2_shared::data::client_data::ClientMagic {
        name: info.name.clone(),
        spell: mir2_shared::enums::Spell::try_from(magic.spell as u8 + 3)
            .unwrap_or(mir2_shared::enums::Spell::None),
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
    }
}
fn build_user_information_packet(
    state: &PlayerState,
    item_infos: &std::collections::HashMap<i32, db::ItemInfo>,
) -> Vec<u8> {
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
    body.push(state.hero_behaviour);                // hero_behaviour (C# 0..3)

    // 背包（40 格，含 info 供客户端显示名称/图标/类型）
    body.push(1u8);                                           // has_inventory=true
    body.extend_from_slice(&(state.inventory.backpack.len() as i32).to_le_bytes());
    for slot in state.inventory.backpack.iter() {
        if let Some(slot) = slot {
            body.push(1u8);
            let mut item = slot.item.clone();
            enrich_item_info(&mut item, item_infos);
            if item.write_to_with_info(&mut body).is_err() {
                body.push(0u8); // 回退：空格子
            }
        } else {
            body.push(0u8);
        }
    }
    // 装备（12 槽，含 info）
    body.push(1u8);                                           // has_equipment=true
    body.extend_from_slice(&(state.inventory.equipment.len() as i32).to_le_bytes());
    for eq in state.inventory.equipment.iter() {
        if let Some(item) = eq {
            body.push(1u8);
            let mut item = item.clone();
            enrich_item_info(&mut item, item_infos);
            if item.write_to_with_info(&mut body).is_err() {
                body.push(0u8);
            }
        } else {
            body.push(0u8);
        }
    }
    body.push(0u8);                                           // has_quest_inventory=false
    body.extend_from_slice(&(state.inventory.gold as u32).to_le_bytes()); // gold
    body.extend_from_slice(&0u32.to_le_bytes());              // credit
    body.push(0u8);                                           // has_expanded_storage=false
    body.extend_from_slice(&0i64.to_le_bytes());              // expanded_storage_expiry_time
    body.extend_from_slice(&0i32.to_le_bytes());              // magic_count=0
    body.extend_from_slice(&0i32.to_le_bytes());              // intelligent creatures count=0
    body.push(0u8);                                           // summoned_creature_type
    body.push(0u8);                                           // creature_summoned=false
    body.push(0u8);                                           // allow_observe=false
    body.push(0u8);                                           // observer=false

    // #208：角色面板属性段（18 x i32；最终值 = 基础 + 装备加成）
    body.extend_from_slice(&(state.max_hp + state.bonus_max_hp).to_le_bytes());
    body.extend_from_slice(&(state.max_mp + state.bonus_max_mp).to_le_bytes());
    for v in [
        state.min_ac + state.bonus_min_ac,
        state.max_ac + state.bonus_max_ac,
    ] {
        body.extend_from_slice(&v.to_le_bytes());
    }
    for v in [
        state.min_mac + state.bonus_min_mac,
        state.max_mac + state.bonus_max_mac,
    ] {
        body.extend_from_slice(&v.to_le_bytes());
    }
    for v in [
        state.min_attack + state.bonus_min_attack,
        state.max_attack + state.bonus_max_attack,
    ] {
        body.extend_from_slice(&v.to_le_bytes());
    }
    for v in [
        state.min_mc + state.bonus_min_mc,
        state.max_mc + state.bonus_max_mc,
    ] {
        body.extend_from_slice(&v.to_le_bytes());
    }
    for v in [
        state.min_sc + state.bonus_min_sc,
        state.max_sc + state.bonus_max_sc,
    ] {
        body.extend_from_slice(&v.to_le_bytes());
    }
    body.extend_from_slice(&state.critical_rate.to_le_bytes());
    body.extend_from_slice(&state.critical_damage.to_le_bytes());
    body.extend_from_slice(&state.attack_speed.to_le_bytes()); // attack_speed（装备加成 Stat::AttackSpeed）
    body.extend_from_slice(&state.accuracy.to_le_bytes());
    body.extend_from_slice(&state.agility.to_le_bytes());
    body.extend_from_slice(&state.luck.to_le_bytes());

    // #210：State 页段（11 x i32；负重 = 物品 weight × count）
    let bag_weight: i32 = state
        .inventory
        .backpack
        .iter()
        .flatten()
        .map(|s| {
            item_infos
                .get(&s.item.item_index)
                .map(|i| i.weight)
                .unwrap_or(0)
                * i32::from(s.item.count)
        })
        .sum();
    let wear_weight: i32 = state
        .inventory
        .equipment
        .iter()
        .flatten()
        .map(|i| item_infos.get(&i.item_index).map(|i| i.weight).unwrap_or(0))
        .sum();
    // C# HandWeight：武器（含火把）重量
    let hand_weight: i32 = state.inventory.get_equipment(crate::actors::inventory::EquipmentSlot::Weapon)
        .and_then(|i| item_infos.get(&i.item_index))
        .map(|i| i.weight)
        .unwrap_or(0);
    for v in [
        bag_weight,
        wear_weight,
        hand_weight,
        state.magic_resist,
        state.poison_resist,
        state.health_recovery,
        state.spell_recovery,
        state.poison_recovery,
        state.holy,
        state.freezing,
        state.poison_attack,
    ] {
        body.extend_from_slice(&v.to_le_bytes());
    }

    build_packet_bytes(ServerPacketIds::UserInformation as i16, &body)
}

/// 构建 ObjectPlayer 数据包（其他玩家进入视野）
pub(crate) fn build_object_player_packet(
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
    // SharedRust SpellEffect::None=3（C# 从 0 开始），写 0 会让客户端 try_from 失败
    body.push(mir2_shared::enums::SpellEffect::None as u8); // effect=None
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
async fn send_player_update(
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
    let _ = gate_ref.tell(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::PlayerUpdate as i16, &body),
    }).await;
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
    body.push(0u8);                                     // effect（C# ObjectMonster.Effect，缺失会导致客户端解析错位）
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
async fn spawn_npcs_and_monsters(
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
        let _ = gate_ref.tell(SendToClient {
            session_id,
            data: packet,
        }).await;

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

        // 精英判定（C# Settings.MonsterRarity* 配置化）
        let is_elite = fastrand::u8(1..=100) <= ctx.rarity.elite_chance_percent;
        let (name, hp, max_hp, min_dmg, max_dmg, xp) = if is_elite {
            (
                format!("[精英] {}", monster.name),
                (monster.hp as f64 * ctx.rarity.elite_hp_multiplier).max(1.0) as i32,
                (monster.hp as f64 * ctx.rarity.elite_hp_multiplier).max(1.0) as i32,
                (monster.min_dmg as f64 * ctx.rarity.elite_dmg_multiplier) as i32,
                (monster.max_dmg as f64 * ctx.rarity.elite_dmg_multiplier) as i32,
                (monster.xp as f64 * ctx.rarity.elite_xp_multiplier).max(1.0) as i32,
            )
        } else {
            (monster.name.clone(), monster.hp, monster.hp, monster.min_dmg, monster.max_dmg, monster.xp)
        };

        let packet = build_object_monster_packet(monster, object_id, &name);
        let _ = gate_ref.tell(SendToClient {
            session_id,
            data: packet,
        }).await;

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
            min_ac: 0,
            max_ac: 0,
            min_mac: 0,
            max_mac: 0,
            agility: 0,
            accuracy: 0,
            armour_rate: 1.0,
            damage_rate: 1.0,
            magic_resist: 0,
            critical_rate: 0,
            critical_damage: 0,
            luck: 0,
            reflect: 0,
            damage_reduction_percent: 0,
            poison_list: Vec::new(),
            undead: ctx.monster_infos.get(&monster.monster_index).map(|i| i.undead).unwrap_or(false),
            master_session: None,
            recall_at_tick: 0,
            behavior: ai::make_behavior(&name),
        });
        // 从 MonsterInfo 填充战斗属性（AC/MAC/Agility/Crit 等）
        if let Some(m) = monsters.last_mut() {
            if let Some(info) = ctx.monster_infos.get(&monster.monster_index) {
                m.fill_combat_stats(info);
            }
        }
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
                    let _ = gate_ref.tell(SendToClient { session_id, data: packet }).await;
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
                        min_ac: 0,
                        max_ac: 0,
                        min_mac: 0,
                        max_mac: 0,
                        agility: 0,
                        accuracy: 0,
                        armour_rate: 1.0,
                        damage_rate: 1.0,
                        magic_resist: 0,
                        critical_rate: 0,
                        critical_damage: 0,
                        luck: 0,
                        reflect: 0,
                        damage_reduction_percent: 0,
                        poison_list: Vec::new(),
            undead: false,
                        master_session: None,
                        recall_at_tick: 0,
                        behavior: ai::make_behavior(&dragon.monster_name),
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
            min_ac: 0,
            max_ac: 0,
            min_mac: 0,
            max_mac: 0,
            agility: 0,
            accuracy: 0,
            armour_rate: 1.0,
            damage_rate: 1.0,
            magic_resist: 0,
            critical_rate: 0,
            critical_damage: 0,
            luck: 0,
            reflect: 0,
            damage_reduction_percent: 0,
            poison_list: Vec::new(),
            undead: false,
            master_session: None,
            recall_at_tick: 0,
            behavior: ai::make_behavior("TestBoss"),
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

#[cfg(test)]
mod e2e;



#[cfg(test)]
mod hero_tests {
    use super::*;

    #[test]
    fn hero_create_result_codes() {
        // C# S.NewHero.Result：1=BadName 4=MaxHeroes 10=Success
        assert_eq!(hero_create_result("", false), 1);
        assert_eq!(hero_create_result("   ", false), 1);
        assert_eq!(hero_create_result("Hero", true), 4);
        assert_eq!(hero_create_result("Hero", false), 10);
    }
}
