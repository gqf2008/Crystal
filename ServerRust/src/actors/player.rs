// PlayerActor - 玩家实例
// 持有单个玩家的完整状态：位置、方向、地图、背包等
// 移动由客户端驱动，服务端验证并广播

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use tracing::{debug, info, warn};

use crate::actors::inventory::PlayerInventory;
use crate::actors::friend::FriendList;
use crate::actors::mail::Mailbox;
use crate::actors::guild::GuildRank;
use crate::actors::quest::QuestLog;
use crate::actors::creature::CreatureLog;
use crate::actors::refine::RefineLog;
use crate::gate::actor::{GateActor, SendToClient};
use crate::maps::loader::MapData;
use crate::util::wire::{build_packet_bytes, write_dotnet_string};

/// 方向增量 (MirDirection: Up=0, UpRight=1, Right=2, DownRight=3, Down=4, DownLeft=5, Left=6, UpLeft=7)
const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

/// 玩家状态
#[derive(Debug, Clone)]
pub struct PlayerState {
    /// 唯一对象 ID
    pub object_id: u32,
    /// 玩家名称
    pub name: String,
    /// 当前地图
    pub map_index: u16,
    /// 网格坐标 X
    pub x: i32,
    /// 网格坐标 Y
    pub y: i32,
    /// 朝向 (0..7)
    pub direction: u8,
    /// 攻击模式 (Peace/Group/Guild/EnemyGuild/RedBrown/All)
    pub attack_mode: mir2_shared::enums::AttackMode,
    /// 宠物模式 (Both/MoveOnly/AttackOnly/None/FocusMasterTarget)
    pub pet_mode: mir2_shared::enums::PetMode,
    /// 是否隐藏
    pub hidden: bool,
    /// 所属 session
    pub session_id: u64,
    /// 职业
    pub class: mir2_shared::enums::MirClass,
    /// 性别
    pub gender: mir2_shared::enums::MirGender,
    /// 发型
    pub hair: u8,
    /// 等级
    pub level: u16,
    /// 当前经验
    pub experience: i64,
    /// 升级所需经验
    pub max_experience: i64,
    /// 当前 HP
    pub hp: i32,
    /// 最大 HP（基础+装备加成后的总值）
    pub max_hp: i32,
    /// 当前 MP
    pub mp: i32,
    /// 最大 MP（基础+装备加成后的总值）
    pub max_mp: i32,
    /// 最小攻击力（基础+装备加成后的总值）
    pub min_attack: i32,
    /// 最大攻击力（基础+装备加成后的总值）
    pub max_attack: i32,
    /// 防御力（基础+装备加成后的总值）
    pub defence: i32,
    /// 装备加成：最小攻击力
    pub bonus_min_attack: i32,
    /// 装备加成：最大攻击力
    pub bonus_max_attack: i32,
    /// 装备加成：防御力
    pub bonus_defence: i32,
    /// 装备加成：最大 HP
    pub bonus_max_hp: i32,
    /// 装备加成：最大 MP
    pub bonus_max_mp: i32,
    /// 背包 + 装备 + 金币
    pub inventory: PlayerInventory,
    /// 所属组队 ID（None = 无组队）
    pub group_id: Option<u64>,
    /// 好友列表
    pub friend_list: FriendList,
    /// 收件箱
    pub mailbox: Mailbox,
    /// 所属行会名称
    pub guild_name: Option<String>,
    /// 行会 rank
    pub guild_rank: GuildRank,
    /// 任务日志
    pub quest_log: QuestLog,
    /// 配偶名称
    pub spouse_name: Option<String>,
    /// 是否允许拜师
    pub allow_mentor: bool,
    /// 导师名称
    pub mentor_name: Option<String>,
    /// 宠物信息
    pub creature_log: CreatureLog,
    /// 英雄索引（0 = 无英雄）
    pub hero_index: u8,
    /// 英雄背包
    pub hero_inventory: crate::actors::inventory::PlayerInventory,
    /// 精炼日志
    pub refine_log: RefineLog,
    /// 是否在钓鱼
    pub is_fishing: bool,
    /// 是否骑乘坐骑
    pub is_mounted: bool,
    /// 是否死亡（对应 C# Dead）
    pub is_dead: bool,
    /// PK 值（>0 = 红名，每杀1人+100，在线 tick 衰减）
    pub pk_points: i32,
    /// 累计击杀玩家数
    pub pk_kill_count: u32,
    /// 钓鱼自动释放
    pub fishing_autocast: bool,
    /// 轮回宿主（发起轮回的玩家 session_id）
    pub reincarnation_host: Option<u64>,
    /// 轮回是否已就绪
    pub reincarnation_ready: bool,
    /// 轮回过期时间（WorldActor tick count，过期则自动取消）
    pub reincarnation_expire_time: u64,
    /// 是否允许组队召回（对应 C# EnableGroupRecall）
    pub enable_group_recall: bool,
    /// 上次使用组队召回的时间戳（毫秒，对应 C# LastRecallTime）
    pub last_recall_time: u64,
    /// 是否允许配偶召回（对应 C# AllowLoverRecall）
    pub allow_lover_recall: bool,
    /// 是否为 GM（对应 C# IsGM / AccountInfo.AdminAccount）
    pub is_gm: bool,
    /// 当前 Buff/Debuff 列表
    pub buffs: Vec<crate::combat::buff::BuffInstance>,
}

impl PlayerState {
    /// 计算包含装备+Buff加成的最小攻击力
    pub fn effective_min_attack(&self) -> i32 {
        let base = self.min_attack + self.bonus_min_attack;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::AttackBoost { bonus: 0 },
        );
        (base + buff_bonus).max(0)
    }

    /// 计算包含装备+Buff加成的最大攻击力
    pub fn effective_max_attack(&self) -> i32 {
        let base = self.max_attack + self.bonus_max_attack;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::AttackBoost { bonus: 0 },
        );
        (base + buff_bonus).max(self.effective_min_attack())
    }

    /// 计算包含装备+Buff加成的防御力
    pub fn effective_defence(&self) -> i32 {
        let base = self.defence + self.bonus_defence;
        let buff_bonus = crate::combat::buff::get_stat_bonus(
            &self.buffs,
            &crate::combat::buff::BuffType::DefenseBoost { bonus: 0 },
        );
        (base + buff_bonus).max(0)
    }
}

/// PlayerActor 状态
pub struct PlayerActor {
    pub state: PlayerState,
    /// GateActor 引用，用于发数据给客户端
    gate_ref: ActorRef<GateActor>,
    /// 当前地图数据（用于边界+障碍物校验）
    map_data: Option<MapData>,
}

impl PlayerActor {
    pub fn new(
        object_id: u32,
        name: String,
        session_id: u64,
        map_index: u16,
        gate_ref: ActorRef<GateActor>,
    ) -> Self {
        Self {
            state: PlayerState {
                object_id,
                name,
                map_index,
                x: 330,
                y: 330,
                direction: 4, // Down
                attack_mode: mir2_shared::enums::AttackMode::Peace,
                pet_mode: mir2_shared::enums::PetMode::Both,
                hidden: false,
                session_id,
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
                hero_inventory: PlayerInventory::new(),
                refine_log: RefineLog::new(),
                is_fishing: false,
                is_mounted: false,
                is_dead: false,
                pk_points: 0,
                pk_kill_count: 0,
                fishing_autocast: false,
                reincarnation_host: None,
                reincarnation_ready: false,
                reincarnation_expire_time: 0,
                enable_group_recall: false,
                last_recall_time: 0,
                allow_lover_recall: false,
                is_gm: false,
                buffs: Vec::new(),
            },
            gate_ref,
            map_data: None,
        }
    }

    /// 设置地图数据
    pub fn set_map_data(&mut self, map: MapData) {
        self.map_data = Some(map);
    }

    /// 尝试移动（Walk=1格, Run=2格）
    pub fn try_move(&mut self, direction: u8, steps: i32) -> bool {
        if direction >= 8 {
            warn!("Invalid direction {}", direction);
            return false;
        }

        let dx = DIR_DX[direction as usize];
        let dy = DIR_DY[direction as usize];

        let new_x = self.state.x + dx * steps;
        let new_y = self.state.y + dy * steps;

        // 检查最终落点是否可行走
        if let Some(ref map) = self.map_data {
            if !map.is_walkable(new_x, new_y) {
                debug!("Player {} blocked at ({}, {})", self.state.name, new_x, new_y);
                return false;
            }
        }

        // 更新朝向
        self.state.direction = direction;
        self.state.x = new_x;
        self.state.y = new_y;
        true
    }

    /// 转向（不移动）
    pub fn turn(&mut self, direction: u8) {
        if direction < 8 {
            self.state.direction = direction;
        }
    }

    /// 发送 UserLocation 给玩家
    fn send_user_location(&self) {
        let mut body = Vec::new();
        body.extend_from_slice(&self.state.x.to_le_bytes());
        body.extend_from_slice(&self.state.y.to_le_bytes());
        body.push(self.state.direction);
        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
        });
    }
}

impl Actor for PlayerActor {
    type Args = (u32, String, u64, u16, ActorRef<GateActor>);
    type Error = anyhow::Error;

    async fn on_start(
        args: Self::Args,
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let (object_id, name, session_id, map_index, gate_ref) = args;
        debug!("PlayerActor spawned: {} (object_id={}, session={})", name, object_id, session_id);
        Ok(Self::new(object_id, name, session_id, map_index, gate_ref))
    }
}

// ============================================================
// 消息定义
// ============================================================

/// 移动类型
#[derive(Debug, Clone, Copy)]
pub enum MoveType {
    Walk,
    Run,
    Turn,
}

/// 移动请求（从 WorldActor 转发）
pub struct MoveRequest {
    pub session_id: u64,
    pub direction: u8,
    pub is_run: bool, // true = Run (2格), false = Walk (1格)
}

/// 转向请求
pub struct TurnRequest {
    pub session_id: u64,
    pub direction: u8,
}

/// 广播移动给其他玩家（其他 PlayerActor 收到此消息后发给自己的客户端）
pub struct BroadcastMovement {
    pub object_id: u32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub move_type: MoveType,
    pub exclude_session: u64,
}

/// 获取玩家状态（用于广播/序列化）
pub struct GetPlayerState;

/// 设置地图数据
pub struct SetMapData {
    pub map: MapData,
}

/// 设置玩家状态（用于从数据库加载后初始化）
pub struct SetPlayerState {
    pub state: PlayerState,
}

impl Message<SetPlayerState> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetPlayerState,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state = msg.state;
    }
}

/// 复活玩家：重置 HP/MP 到最大值，设置位置
pub struct RevivePlayer {
    pub x: i32,
    pub y: i32,
    pub map_index: u16,
}

impl Message<RevivePlayer> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RevivePlayer, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.is_dead = false;
        self.state.x = msg.x;
        self.state.y = msg.y;
        self.state.hp = self.state.max_hp;
        self.state.mp = self.state.max_mp;
        // 发送位置更新
        self.send_user_location();
        true
    }
}

// ============================================================
// Handler 实现
// ============================================================

impl Message<MoveRequest> for PlayerActor {
    type Reply = bool; // success

    async fn handle(
        &mut self,
        msg: MoveRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let steps = if msg.is_run { 2 } else { 1 };
        let success = self.try_move(msg.direction, steps);

        if success {
            debug!(
                "Player {} moved {} to ({}, {}) dir={}",
                self.state.name,
                if msg.is_run { "run" } else { "walk" },
                self.state.x,
                self.state.y,
                msg.direction
            );
            self.send_user_location();
        }

        success
    }
}

impl Message<TurnRequest> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TurnRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.turn(msg.direction);
        debug!("Player {} turned to dir={}", self.state.name, msg.direction);
        self.send_user_location();
    }
}

impl Message<BroadcastMovement> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: BroadcastMovement,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // 不给自己发
        if self.state.session_id == msg.exclude_session {
            return;
        }

        let opcode = match msg.move_type {
            MoveType::Walk => mir2_shared::enums::ServerPacketIds::ObjectWalk,
            MoveType::Run => mir2_shared::enums::ServerPacketIds::ObjectRun,
            MoveType::Turn => mir2_shared::enums::ServerPacketIds::ObjectTurn,
        };

        let mut body = Vec::new();
        body.extend_from_slice(&msg.object_id.to_le_bytes());
        body.extend_from_slice(&msg.x.to_le_bytes());
        body.extend_from_slice(&msg.y.to_le_bytes());
        body.push(msg.direction);

        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(opcode as i16, &body),
        });
    }
}

impl Message<GetPlayerState> for PlayerActor {
    type Reply = Option<PlayerState>;

    async fn handle(
        &mut self,
        _msg: GetPlayerState,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Some(self.state.clone())
    }
}

impl Message<SetMapData> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetMapData,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.set_map_data(msg.map);
    }
}

/// 攻击请求（从 WorldActor 转发）
pub struct AttackRequest {
    pub session_id: u64,
    pub direction: u8,
    pub spell: u8,
}

/// 受到伤害（从 WorldActor 转发，其他玩家攻击到自己）
pub struct TakeDamage {
    pub attacker_id: u32,
    pub attacker_session: u64,
    pub damage: i32,
}

impl Message<AttackRequest> for PlayerActor {
    type Reply = Option<AttackResult>;

    async fn handle(
        &mut self,
        msg: AttackRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if msg.direction < 8 {
            self.state.direction = msg.direction;
        }

        // Spell::None = 基本近战攻击
        // Phase 1：只处理近战，无目标验证，纯视觉效果
        debug!(
            "Player {} attacks: dir={} spell={}",
            self.state.name, msg.direction, msg.spell
        );

        // 广播 ObjectAttack 给其他玩家
        Some(AttackResult {
            object_id: self.state.object_id,
            x: self.state.x,
            y: self.state.y,
            direction: self.state.direction,
            spell: msg.spell,
        })
    }
}

impl Message<TakeDamage> for PlayerActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: TakeDamage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let damage = msg.damage.max(0);
        self.state.hp = (self.state.hp - damage).max(0);

        debug!(
            "Player {} took {} damage from object_id={} (hp: {}/{})",
            self.state.name, damage, msg.attacker_id, self.state.hp, self.state.max_hp
        );

        // 发送 Struck（自己被攻击的动画）
        let mut struck_body = Vec::new();
        struck_body.extend_from_slice(&msg.attacker_id.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::Struck as i16, &struck_body),
        });

        // 死亡处理
        if self.state.hp <= 0 && !self.state.is_dead {
            self.state.is_dead = true;
            debug!("Player {} died (attacker={})", self.state.name, msg.attacker_id);
            return true;
        }

        // 发送 HealthChanged
        if self.state.hp > 0 {
            let mut health_body = Vec::new();
            health_body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
            health_body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
            let _ = self.gate_ref.ask(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &health_body),
            });
        }
        false
    }
}

/// 获得经验（从 WorldActor 转发）
pub struct AddExperience {
    pub amount: i32,
}

impl Message<AddExperience> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddExperience,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let amount = msg.amount.max(0) as i64;
        self.state.experience += amount;

        debug!(
            "Player {} gained {} exp (total={}/{})",
            self.state.name, amount, self.state.experience, self.state.max_experience
        );

        // 发送 GainExperience 给客户端
        let mut body = Vec::new();
        body.extend_from_slice(&(amount as u32).to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::GainExperience as i16, &body),
        });

        // 检查升级
        const MAX_LEVEL: u16 = 200;
        while self.state.experience >= self.state.max_experience && self.state.level < MAX_LEVEL {
            self.state.experience -= self.state.max_experience;
            self.state.level += 1;

            // 属性成长（按职业）
            let (hp_gain, mp_gain, min_atk_gain, max_atk_gain, def_gain) = match self.state.class {
                mir2_shared::enums::MirClass::Warrior => (12, 4, 1, 2, 1),
                mir2_shared::enums::MirClass::Wizard => (6, 10, 1, 2, 0),
                mir2_shared::enums::MirClass::Taoist => (8, 8, 1, 2, 1),
                mir2_shared::enums::MirClass::Assassin => (8, 5, 1, 2, 1),
                mir2_shared::enums::MirClass::Archer => (7, 6, 1, 2, 1),
            };
            self.state.max_hp += hp_gain;
            self.state.hp = self.state.max_hp;
            self.state.max_mp += mp_gain;
            self.state.mp = self.state.max_mp;
            self.state.min_attack += min_atk_gain;
            self.state.max_attack += max_atk_gain;
            self.state.defence += def_gain;
            self.state.max_experience = (self.state.max_experience as f64 * 1.5) as i64;

            info!("Player {} leveled up to {}! (atk={}-{} def={})",
                  self.state.name, self.state.level, self.state.min_attack, self.state.max_attack, self.state.defence);

            // 发送 LevelChanged
            let mut lv_body = Vec::new();
            lv_body.extend_from_slice(&self.state.level.to_le_bytes());
            lv_body.extend_from_slice(&self.state.experience.to_le_bytes());
            lv_body.extend_from_slice(&self.state.max_experience.to_le_bytes());
            let _ = self.gate_ref.ask(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::LevelChanged as i16, &lv_body),
            });
        }
    }
}

/// 治疗请求（来自 Healing/MassHealing 等魔法）
pub struct Heal {
    pub amount: i32,
}

impl Message<Heal> for PlayerActor {
    type Reply = i32; // 实际回复量

    async fn handle(
        &mut self,
        msg: Heal,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.state.is_dead || msg.amount <= 0 {
            return 0;
        }
        let before = self.state.hp;
        self.state.hp = (self.state.hp + msg.amount).min(self.state.max_hp);
        let healed = self.state.hp - before;

        // 发送 HealthChanged 给客户端
        let mut body = Vec::new();
        body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
        body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
        });

        debug!("Player {} healed for {} HP ({} -> {})", self.state.name, healed, before, self.state.hp);
        healed
    }
}

/// 复活请求（WorldActor 在死亡倒计时后调用）
pub struct Revive;

impl Message<Revive> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Revive,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if !self.state.is_dead {
            return;
        }
        self.state.is_dead = false;
        self.state.hp = self.state.max_hp;
        self.state.mp = self.state.max_mp;

        // 发送 HealthChanged
        let mut body = Vec::new();
        body.extend_from_slice(&self.state.hp.to_le_bytes());
        body.extend_from_slice(&self.state.mp.to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
        });

        debug!("Player {} revived (hp={} mp={})", self.state.name, self.state.hp, self.state.mp);
    }
}

/// Buff 应用请求
pub struct ApplyBuff {
    pub buff: crate::combat::buff::BuffInstance,
}

impl Message<ApplyBuff> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ApplyBuff,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        crate::combat::buff::apply_buff(&mut self.state.buffs,
            msg.buff,
        );
    }
}

/// 移除指定类型的 Buff
pub struct RemoveBuff {
    pub buff_type: crate::combat::buff::BuffType,
}

impl Message<RemoveBuff> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RemoveBuff,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        crate::combat::buff::remove_buff_by_type(&mut self.state.buffs, &msg.buff_type);
    }
}

/// Buff tick（由 WorldActor 主循环每 tick 调用）
pub struct TickBuff;

impl Message<TickBuff> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: TickBuff,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.state.buffs.is_empty() {
            return;
        }
        let results = crate::combat::buff::tick_buffs(&mut self.state.buffs, 1);
        let mut total_hp = 0i32;
        let mut total_mp = 0i32;
        for r in &results {
            total_hp += r.hp_change;
            total_mp += r.mp_change;
        }
        if total_hp != 0 {
            self.state.hp = (self.state.hp + total_hp).clamp(0, self.state.max_hp);
        }
        if total_mp != 0 {
            self.state.mp = (self.state.mp + total_mp).clamp(0, self.state.max_mp);
        }
        // 移除过期 buff
        crate::combat::buff::expire_buffs(&mut self.state.buffs,
        );

        // 如有变化，同步客户端
        if total_hp != 0 || total_mp != 0 {
            let mut body = Vec::new();
            body.extend_from_slice(&self.state.hp.to_le_bytes());
            body.extend_from_slice(&self.state.mp.to_le_bytes());
            let _ = self.gate_ref.ask(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
            });
        }
    }
}

/// 设置装备属性加成（WorldActor 计算后下发）
pub struct SetStatBonuses {
    pub bonus_min_attack: i32,
    pub bonus_max_attack: i32,
    pub bonus_defence: i32,
    pub bonus_max_hp: i32,
    pub bonus_max_mp: i32,
}

impl Message<SetStatBonuses> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetStatBonuses,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let d_min = msg.bonus_min_attack - self.state.bonus_min_attack;
        let d_max = msg.bonus_max_attack - self.state.bonus_max_attack;
        let d_def = msg.bonus_defence - self.state.bonus_defence;
        let d_hp = msg.bonus_max_hp - self.state.bonus_max_hp;
        let d_mp = msg.bonus_max_mp - self.state.bonus_max_mp;

        if d_min != 0 || d_max != 0 || d_def != 0 || d_hp != 0 || d_mp != 0 {
            self.state.min_attack += d_min;
            self.state.max_attack += d_max;
            self.state.defence += d_def;
            self.state.max_hp += d_hp;
            self.state.max_mp += d_mp;

            //  Clamp HP/MP within new max
            self.state.hp = self.state.hp.min(self.state.max_hp);
            self.state.mp = self.state.mp.min(self.state.max_mp);

            self.state.bonus_min_attack = msg.bonus_min_attack;
            self.state.bonus_max_attack = msg.bonus_max_attack;
            self.state.bonus_defence = msg.bonus_defence;
            self.state.bonus_max_hp = msg.bonus_max_hp;
            self.state.bonus_max_mp = msg.bonus_max_mp;

            self.send_user_information_refresh();
        }
    }
}

/// 攻击结果（返回给 WorldActor 用于广播）
pub struct AttackResult {
    pub object_id: u32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub spell: u8,
}

/// 损耗装备耐久
pub struct DamageEquipment {
    pub slot: crate::actors::inventory::EquipmentSlot,
    pub amount: u16,
}

impl Message<DamageEquipment> for PlayerActor {
    type Reply = bool; // true = item broke

    async fn handle(
        &mut self,
        msg: DamageEquipment,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let broke = if let Some(ref mut item) = self.state.inventory.equipment[msg.slot as usize] {
            if item.current_dura > msg.amount {
                item.current_dura -= msg.amount;
                item.dura_changed = true;
                false
            } else {
                item.current_dura = 0;
                item.dura_changed = true;
                true
            }
        } else {
            false
        };

        if broke {
            self.state.inventory.equipment[msg.slot as usize] = None;
            self.send_equipment_changed();
            self.send_inventory_changed();
        }

        broke
    }
}

/// 修理所有装备（恢复耐久到最大值）
pub struct RepairAllEquipment;

impl Message<RepairAllEquipment> for PlayerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: RepairAllEquipment,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut any_repaired = false;
        for slot_idx in 0..crate::actors::inventory::EquipmentSlot::COUNT {
            if let Some(ref mut item) = self.state.inventory.equipment[slot_idx] {
                if item.current_dura < item.max_dura {
                    item.current_dura = item.max_dura;
                    item.dura_changed = true;
                    any_repaired = true;
                }
            }
        }
        if any_repaired {
            self.send_equipment_changed();
        }
    }
}

// ============================================================
// 背包操作消息 Handler
// ============================================================

/// 添加物品到背包
pub struct AddItemToInventory {
    pub item: mir2_shared::data::item::UserItem,
}

impl Message<AddItemToInventory> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AddItemToInventory, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        match self.state.inventory.add_item(msg.item) {
            Some((_grid, _uid)) => {
                // 发送 ItemChanged 通知客户端更新背包
                self.send_inventory_changed();
                true
            }
            None => false,
        }
    }
}

/// 背包内移动
pub struct InventoryMoveItem {
    pub from_grid: u8,
    pub to_grid: u8,
}

impl Message<InventoryMoveItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventoryMoveItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.move_item(msg.from_grid, msg.to_grid);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 获取物品信息
pub struct GetItemInfo {
    pub unique_id: u64,
}

impl Message<GetItemInfo> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: GetItemInfo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.get_item(msg.unique_id).cloned()
    }
}

/// 获取指定格子的物品信息
pub struct GetItemInfoByGrid {
    pub grid: u8,
}

impl Message<GetItemInfoByGrid> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: GetItemInfoByGrid, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.get_item_by_grid(msg.grid).cloned()
    }
}

/// 消耗物品
pub struct ConsumeItem {
    pub unique_id: u64,
}

impl Message<ConsumeItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: ConsumeItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let removed = self.state.inventory.remove_item_by_uid(msg.unique_id);
        if removed.is_some() {
            self.send_inventory_changed();
            true
        } else {
            false
        }
    }
}

/// 装备物品
pub struct InventoryEquipItem {
    pub grid: u8,
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<InventoryEquipItem> for PlayerActor {
    type Reply = Option<(Option<mir2_shared::data::item::UserItem>, u64)>;

    async fn handle(&mut self, msg: InventoryEquipItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = self.state.inventory.equip_item(msg.grid, msg.slot);
        if result.is_some() {
            self.send_inventory_changed();
            self.send_equipment_changed();
        }
        result
    }
}

/// 获取装备信息
pub struct GetEquipmentInfo {
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<GetEquipmentInfo> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: GetEquipmentInfo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.get_equipment(msg.slot).cloned()
    }
}

/// 卸下装备
pub struct InventoryUnequipItem {
    pub slot: crate::actors::inventory::EquipmentSlot,
}

impl Message<InventoryUnequipItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventoryUnequipItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let result = self.state.inventory.unequip_item(msg.slot);
        if result.is_some() {
            self.send_inventory_changed();
            self.send_equipment_changed();
            true
        } else {
            false
        }
    }
}

/// 从背包移除物品
pub struct RemoveItemFromInventory {
    pub unique_id: u64,
}

impl Message<RemoveItemFromInventory> for PlayerActor {
    type Reply = Option<mir2_shared::data::item::UserItem>;

    async fn handle(&mut self, msg: RemoveItemFromInventory, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let item = self.state.inventory.remove_item_by_uid(msg.unique_id);
        if item.is_some() {
            self.send_inventory_changed();
        }
        item
    }
}

/// 合并物品
pub struct InventoryMergeItem {
    pub from_grid: u8,
    pub to_grid: u8,
}

impl Message<InventoryMergeItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventoryMergeItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.merge_item(msg.from_grid, msg.to_grid);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 拆分物品
pub struct InventorySplitItem {
    pub grid: u8,
    pub count: u16,
}

impl Message<InventorySplitItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: InventorySplitItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.split_item(msg.grid, msg.count);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 修理物品
pub struct RepairItem {
    pub unique_id: u64,
}

impl Message<RepairItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RepairItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let success = self.state.inventory.repair_item(msg.unique_id);
        if success {
            self.send_inventory_changed();
        }
        success
    }
}

/// 丢弃金币
pub struct DropGold {
    pub amount: u64,
}

impl Message<DropGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DropGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.inventory.gold >= msg.amount {
            self.state.inventory.gold -= msg.amount;
            self.send_gold_changed();
            true
        } else {
            false
        }
    }
}

/// 添加金币
pub struct AddGold {
    pub amount: u64,
}

impl Message<AddGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AddGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.gold += msg.amount;
        self.send_gold_changed();
        true
    }
}

/// 扣减金币
pub struct DeductGold {
    pub amount: u64,
}

impl Message<DeductGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DeductGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.inventory.gold >= msg.amount {
            self.state.inventory.gold -= msg.amount;
            self.send_gold_changed();
            true
        } else {
            false
        }
    }
}

/// 扣减 MP
pub struct DeductMP {
    pub amount: i32,
}

impl Message<DeductMP> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DeductMP, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.mp >= msg.amount {
            self.state.mp -= msg.amount;
            // 同步客户端
            let mut body = Vec::new();
            body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
            body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
            let _ = self.gate_ref.ask(SendToClient {
                session_id: self.state.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
            });
            true
        } else {
            false
        }
    }
}

/// 恢复 MP
pub struct AddMP {
    pub amount: i32,
}

impl Message<AddMP> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddMP, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if msg.amount <= 0 { return; }
        self.state.mp = (self.state.mp + msg.amount).min(self.state.max_mp);
        // 同步客户端
        let mut body = Vec::new();
        body.extend_from_slice(&(self.state.hp as u32).to_le_bytes());
        body.extend_from_slice(&(self.state.mp as u32).to_le_bytes());
        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::HealthChanged as i16, &body),
        });
    }
}

/// 检查背包中是否有指定数量的物品（按 item_index）
pub struct HasItem {
    pub item_index: i32,
    pub count: u16,
}

impl Message<HasItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: HasItem, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.count_item_by_index(msg.item_index) >= msg.count
    }
}

/// 按 item_index 从背包中移除指定数量的物品
pub struct RemoveItemByIndex {
    pub item_index: i32,
    pub count: u16,
}

impl Message<RemoveItemByIndex> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveItemByIndex, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.remove_item_by_index(msg.item_index, msg.count)
    }
}

/// 增加 PK 值（击杀玩家时调用）
pub struct AddPkPoints {
    pub points: i32,
}

impl Message<AddPkPoints> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddPkPoints, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if msg.points > 0 {
            self.state.pk_points += msg.points;
            self.state.pk_kill_count += 1;
            debug!("Player {} PK points +{} (total={}, kills={})",
                   self.state.name, msg.points, self.state.pk_points, self.state.pk_kill_count);
        }
    }
}

/// PK 值衰减（每 tick 调用）
pub struct DecayPkPoints;

impl Message<DecayPkPoints> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: DecayPkPoints, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.pk_points > 0 {
            self.state.pk_points = (self.state.pk_points - 1).max(0);
        }
    }
}

/// 死亡时随机掉落背包物品（返回被掉落的物品列表）
pub struct DropRandomItemsOnDeath;

impl Message<DropRandomItemsOnDeath> for PlayerActor {
    type Reply = Vec<mir2_shared::data::item::UserItem>;

    async fn handle(
        &mut self,
        _msg: DropRandomItemsOnDeath,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut dropped = Vec::new();
        // 红名玩家掉落更多（基础 0-2，红名 +1-3）
        let base_max = if self.state.pk_points > 0 { 5u8 } else { 2u8 };
        let max_drop = fastrand::u8(0..=base_max);
        for _ in 0..max_drop {
            if let Some(item) = self.state.inventory.random_drop_one() {
                dropped.push(item);
            }
        }
        dropped
    }
}

/// 设置组队 ID
pub struct SetGroupId {
    pub group_id: Option<u64>,
}

impl Message<SetGroupId> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetGroupId, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.group_id = msg.group_id;
    }
}

/// 添加好友到列表
pub struct AddFriendToSelf {
    pub friend_oid: u32,
    pub friend_name: String,
}

impl Message<AddFriendToSelf> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddFriendToSelf, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.friend_list.add_friend(msg.friend_oid, msg.friend_name);
    }
}

/// 从列表移除好友
pub struct RemoveFriendFromSelf {
    pub friend_oid: u32,
}

impl Message<RemoveFriendFromSelf> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: RemoveFriendFromSelf, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.friend_list.remove_friend(msg.friend_oid)
    }
}

/// 设置好友备注
pub struct SetFriendMemo {
    pub friend_oid: u32,
    pub memo: String,
}

impl Message<SetFriendMemo> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: SetFriendMemo, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.friend_list.set_memo(msg.friend_oid, msg.memo)
    }
}

// ============================================================
// 邮件系统 Handler
// ============================================================

/// 添加邮件到收件箱
pub struct AddMail {
    pub mail: crate::actors::mail::MailMessage,
}

impl Message<AddMail> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddMail, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.mailbox.add_mail(msg.mail);
    }
}

/// 获取邮件内容
pub struct GetMail {
    pub mail_id: u64,
}

impl Message<GetMail> for PlayerActor {
    type Reply = Option<crate::actors::mail::MailMessage>;

    async fn handle(&mut self, msg: GetMail, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.get_mail(msg.mail_id).cloned()
    }
}

/// 标记邮件已读
pub struct MarkMailRead {
    pub mail_id: u64,
}

impl Message<MarkMailRead> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: MarkMailRead, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.mark_read(msg.mail_id)
    }
}

/// 收取邮件附件（返回金币和物品）
pub struct CollectMailAttachment {
    pub mail_id: u64,
}

impl Message<CollectMailAttachment> for PlayerActor {
    type Reply = Option<(u64, Vec<mir2_shared::data::item::UserItem>)>;

    async fn handle(&mut self, msg: CollectMailAttachment, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.collect_attachment(msg.mail_id)
    }
}

/// 删除邮件
pub struct DeleteMail {
    pub mail_id: u64,
}

impl Message<DeleteMail> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: DeleteMail, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.mailbox.delete_mail(msg.mail_id)
    }
}

// ============================================================
// 行会系统 Handler
// ============================================================

/// 设置玩家行会信息
pub struct SetGuildInfo {
    pub guild_name: Option<String>,
    pub rank: GuildRank,
}

impl Message<SetGuildInfo> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetGuildInfo, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.guild_name = msg.guild_name;
        self.state.guild_rank = msg.rank;
    }
}

// ============================================================
// 任务系统 Handler
// ============================================================

/// 更新任务日志
pub struct UpdateQuestLog {
    pub quest_log: crate::actors::quest::QuestLog,
}

impl Message<UpdateQuestLog> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: UpdateQuestLog, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.quest_log = msg.quest_log;
    }
}

/// 接受任务（在 PlayerActor 上执行）
pub struct AcceptQuest {
    pub quest: crate::actors::quest::QuestInstance,
}

impl Message<AcceptQuest> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AcceptQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.accept_quest(msg.quest)
    }
}

/// 完成任务（在 PlayerActor 上执行，返回完成的奖励信息）
pub struct CompleteQuest {
    pub quest_index: i32,
}

impl Message<CompleteQuest> for PlayerActor {
    type Reply = Option<crate::actors::quest::QuestInstance>;

    async fn handle(&mut self, msg: CompleteQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.complete_quest(msg.quest_index)
    }
}

/// 放弃任务
pub struct AbandonQuest {
    pub quest_index: i32,
}

impl Message<AbandonQuest> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: AbandonQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.abandon_quest(msg.quest_index)
    }
}

/// 获取任务
pub struct GetQuest {
    pub quest_index: i32,
}

impl Message<GetQuest> for PlayerActor {
    type Reply = Option<crate::actors::quest::QuestInstance>;

    async fn handle(&mut self, msg: GetQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.get_quest(msg.quest_index).cloned()
    }
}

/// 检查是否已完成过该任务
pub struct HasCompletedQuest {
    pub quest_index: i32,
}

impl Message<HasCompletedQuest> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: HasCompletedQuest, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.completed_indices.contains(&msg.quest_index)
    }
}

/// 查询任务状态
/// 返回: 0=未接受/不存在, 1=已接受(进行中), 2=已完成
pub struct CheckQuestState {
    pub quest_index: i32,
}

impl Message<CheckQuestState> for PlayerActor {
    type Reply = u8;

    async fn handle(&mut self, msg: CheckQuestState, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.state.quest_log.completed_indices.contains(&msg.quest_index) {
            return 2;
        }
        if self.state.quest_log.get_quest(msg.quest_index).is_some() {
            return 1;
        }
        0
    }
}

/// 处理怪物击杀进度
pub struct ProcessMonsterKill {
    pub monster_index: i32,
}

impl Message<ProcessMonsterKill> for PlayerActor {
    type Reply = Vec<(i32, i32, bool)>;

    async fn handle(&mut self, msg: ProcessMonsterKill, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.quest_log.process_kill(msg.monster_index)
    }
}

/// 检查任务物品进度（在背包变化后调用）
pub struct CheckQuestItemProgress;

impl Message<CheckQuestItemProgress> for PlayerActor {
    type Reply = Vec<(i32, i32, bool)>;

    async fn handle(&mut self, _msg: CheckQuestItemProgress, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let mut updated = Vec::new();
        for quest in &mut self.state.quest_log.quests {
            let mut any_changed = false;
            for p in &mut quest.progress {
                let count = self.state.inventory.count_item_by_index(p.progress_id);
                let count_i32 = count as i32;
                if count_i32 > p.current && count_i32 <= p.target {
                    p.current = count_i32;
                    any_changed = true;
                } else if count_i32 >= p.target && p.current < p.target {
                    p.current = p.target;
                    any_changed = true;
                }
            }
            if any_changed {
                let complete = quest.is_progress_complete();
                // 找到变化了的进度项（取第一个变化的作为代表）
                if let Some(p) = quest.progress.first() {
                    updated.push((quest.quest_index, p.progress_id, complete));
                }
            }
        }
        updated
    }
}

// ============================================================
// 婚姻/师徒系统 Handler
// ============================================================

/// 设置配偶名称
pub struct SetSpouse {
    pub spouse_name: Option<String>,
}

impl Message<SetSpouse> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetSpouse, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.spouse_name = msg.spouse_name;
    }
}

/// 设置是否允许拜师
pub struct SetAllowMentor {
    pub allow: bool,
}

impl Message<SetAllowMentor> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAllowMentor, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.allow_mentor = msg.allow;
    }
}

/// 设置导师名称
pub struct SetMentor {
    pub mentor_name: Option<String>,
}

impl Message<SetMentor> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetMentor, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.mentor_name = msg.mentor_name;
    }
}

/// 设置宠物信息
pub struct SetCreature {
    pub creature_log: CreatureLog,
}

impl Message<SetCreature> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetCreature, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.creature_log = msg.creature_log;
    }
}

/// 宠物饥饿计时
pub struct TickCreatureHunger {
    pub dt_seconds: u32,
}

impl Message<TickCreatureHunger> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: TickCreatureHunger, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.creature_log.tick(msg.dt_seconds);
    }
}

/// 设置攻击模式
pub struct SetAttackMode {
    pub mode: mir2_shared::enums::AttackMode,
}

impl Message<SetAttackMode> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAttackMode, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.attack_mode = msg.mode;
        debug!("Player {} attack mode -> {:?}", self.state.name, msg.mode);
    }
}

/// 设置宠物模式
pub struct SetPetMode {
    pub mode: mir2_shared::enums::PetMode,
}

impl Message<SetPetMode> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetPetMode, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.pet_mode = msg.mode;
        debug!("Player {} pet mode -> {:?}", self.state.name, msg.mode);
    }
}

/// 设置英雄索引
pub struct SetHeroIndex {
    pub hero_index: u8,
}

impl Message<SetHeroIndex> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetHeroIndex, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.hero_index = msg.hero_index;
    }
}

/// 设置玩家位置
pub struct SetPlayerPosition {
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub map_index: Option<u16>,
    pub is_mounted: Option<bool>,
}

impl Message<SetPlayerPosition> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetPlayerPosition, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.x = msg.x;
        self.state.y = msg.y;
        self.state.direction = msg.direction;
        if let Some(mi) = msg.map_index {
            self.state.map_index = mi;
        }
        if let Some(mounted) = msg.is_mounted {
            self.state.is_mounted = mounted;
        }
    }
}

/// 设置组队召回冷却时间
pub struct SetLastRecallTime {
    pub last_recall_time: u64,
}

impl Message<SetLastRecallTime> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetLastRecallTime, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.last_recall_time = msg.last_recall_time;
    }
}

/// 设置是否允许组队召回
pub struct SetEnableGroupRecall {
    pub enable: bool,
}

impl Message<SetEnableGroupRecall> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetEnableGroupRecall, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.enable_group_recall = msg.enable;
    }
}

/// 设置是否允许配偶召回（对应 C# AllowLoverRecall）
pub struct SetAllowLoverRecall {
    pub allow: bool,
}

impl Message<SetAllowLoverRecall> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetAllowLoverRecall, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.allow_lover_recall = msg.allow;
    }
}

/// 检查能否获得物品（背包是否有空间）
pub struct CanGainItems;

impl Message<CanGainItems> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, _msg: CanGainItems, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.state.inventory.can_gain_items()
    }
}

/// 检查能否获得金币（是否超过上限）
pub struct CanGainGold {
    pub amount: u32,
}

impl Message<CanGainGold> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: CanGainGold, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        (msg.amount as u64) + self.state.inventory.gold <= u32::MAX as u64
    }
}

/// 设置钓鱼状态
pub struct SetFishing {
    pub is_fishing: bool,
    pub autocast: bool,
}

impl Message<SetFishing> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetFishing, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.is_fishing = msg.is_fishing;
        self.state.fishing_autocast = msg.autocast;
    }
}

/// 从英雄背包取回物品到主背包
pub struct TakeBackHeroItem {
    pub grid: u8,
}

impl Message<TakeBackHeroItem> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: TakeBackHeroItem, _ctx: &mut Context<Self, Self::Reply>) {
        // 从英雄背包移除指定格子的物品并添加到主背包
        if let Some(slot) = self.state.hero_inventory.backpack[msg.grid as usize].take() {
            let _ = self.state.inventory.add_item(slot.item);
        }
    }
}

/// 从主背包转移物品到英雄背包
pub struct TransferHeroItem {
    pub grid: u8,
}

impl Message<TransferHeroItem> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: TransferHeroItem, _ctx: &mut Context<Self, Self::Reply>) {
        // 从主背包移除指定格子的物品并添加到英雄背包
        if let Some(slot) = self.state.inventory.backpack[msg.grid as usize].take() {
            let _ = self.state.hero_inventory.add_item(slot.item);
        }
    }
}

/// 设置精炼日志
pub struct SetRefineLog {
    pub refine_log: crate::actors::refine::RefineLog,
}

impl Message<SetRefineLog> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetRefineLog, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.refine_log = msg.refine_log;
    }
}

/// 存入仓库
pub struct StoreItem {
    pub grid: u8,
}

impl Message<StoreItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: StoreItem, _ctx: &mut Context<Self, Self::Reply>) -> bool {
        match self.state.inventory.store_item(msg.grid) {
            Some((_item, _storage_grid)) => true,
            None => false,
        }
    }
}

/// 从仓库取出
pub struct TakeBackItem {
    pub grid: u8,
}

impl Message<TakeBackItem> for PlayerActor {
    type Reply = bool;

    async fn handle(&mut self, msg: TakeBackItem, _ctx: &mut Context<Self, Self::Reply>) -> bool {
        match self.state.inventory.take_back_item(msg.grid) {
            Some((_item, _backpack_grid)) => true,
            None => false,
        }
    }
}

// ============================================================
// 轮回系统消息
// ============================================================

/// 清除当前玩家的轮回状态（被施法者/死亡玩家使用）
pub struct ClearReincarnation;

impl Message<ClearReincarnation> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ClearReincarnation, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.reincarnation_host = None;
        self.state.reincarnation_ready = false;
        self.state.reincarnation_expire_time = 0;
    }
}

/// 清除宿主的轮回状态（施法者使用）
pub struct ClearReincarnationHost;

impl Message<ClearReincarnationHost> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ClearReincarnationHost, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.reincarnation_ready = false;
        self.state.reincarnation_expire_time = 0;
    }
}

/// 以一半 HP 复活
pub struct ReviveAtHalfHp;

impl Message<ReviveAtHalfHp> for PlayerActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ReviveAtHalfHp, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.hp = (self.state.max_hp / 2).max(1);
        debug!("ReviveAtHalfHp: {} hp={}/{} (visual effect omitted)", self.state.name, self.state.hp, self.state.max_hp);
    }
}

// ============================================================
// 背包通知辅助函数
// ============================================================

impl PlayerActor {
    fn send_inventory_changed(&self) {
        // 发送 UserInformation 刷新（不含背包数据，客户端需主动查询）
        self.send_user_information_refresh();
    }

    fn send_equipment_changed(&self) {
        // 发送 UserInformation 刷新装备状态
        self.send_user_information_refresh();
    }

    fn send_gold_changed(&self) {
        // 发送 UserInformation 刷新金币
        self.send_user_information_refresh();
    }

    /// 发送 UserInformation 刷新（不含完整背包数据）
    fn send_user_information_refresh(&self) {
        use mir2_shared::enums::ServerPacketIds;
        let mut body = Vec::new();

        body.extend_from_slice(&self.state.object_id.to_le_bytes());   // object_id
        body.extend_from_slice(&1u32.to_le_bytes());                    // real_id
        write_dotnet_string(&mut body, &self.state.name);               // name
        write_dotnet_string(&mut body, "");                             // guild_name
        write_dotnet_string(&mut body, "");                             // guild_rank
        body.extend_from_slice(&0i32.to_le_bytes());                    // name_colour
        body.push(0u8);                                                 // class=Warrior
        body.push(0u8);                                                 // gender=Male
        body.extend_from_slice(&self.state.level.to_le_bytes());        // level
        body.extend_from_slice(&self.state.x.to_le_bytes());            // location_x
        body.extend_from_slice(&self.state.y.to_le_bytes());            // location_y
        body.push(self.state.direction);                                // direction
        body.push(0u8);                                                 // hair
        body.extend_from_slice(&self.state.hp.to_le_bytes());  // hp
        body.extend_from_slice(&self.state.mp.to_le_bytes());  // mp
        body.extend_from_slice(&self.state.experience.to_le_bytes()); // experience
        body.extend_from_slice(&self.state.max_experience.to_le_bytes()); // max_experience
        body.extend_from_slice(&0u16.to_le_bytes());                    // level_effects
        body.push(0u8);                                                 // has_hero=false
        body.push(0u8);                                                 // hero_behaviour=None

        // 背包/装备数据（简化版：不发送完整物品，客户端通过 ItemChanged 等增量包更新）
        body.push(0u8);                                                 // has_inventory=false
        body.push(0u8);                                                 // has_equipment=false
        body.push(0u8);                                                 // has_quest_inventory=false
        body.extend_from_slice(&(self.state.inventory.gold as u32).to_le_bytes()); // gold
        body.extend_from_slice(&0u32.to_le_bytes());                    // credit=0
        body.push(0u8);                                                 // has_expanded_storage=false
        body.extend_from_slice(&0i64.to_le_bytes());                    // expanded_storage_expiry_time
        body.extend_from_slice(&0i32.to_le_bytes());                    // magic_count=0
        body.extend_from_slice(&0i32.to_le_bytes());                    // creature_count=0
        body.push(0u8);                                                 // summoned_creature_type
        body.push(0u8);                                                 // creature_summoned=false
        body.push(0u8);                                                 // allow_observe=false

        let _ = self.gate_ref.ask(SendToClient {
            session_id: self.state.session_id,
            data: build_packet_bytes(ServerPacketIds::UserInformation as i16, &body),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> PlayerState {
        PlayerState {
            object_id: 1000,
            name: "TestPlayer".to_string(),
            map_index: 0,
            x: 330,
            y: 330,
            direction: 4,
            attack_mode: mir2_shared::enums::AttackMode::Peace,
            pet_mode: mir2_shared::enums::PetMode::Both,
            hidden: false,
            session_id: 1,
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
            hero_inventory: PlayerInventory::new(),
            refine_log: RefineLog::new(),
            is_fishing: false,
            is_mounted: false,
            fishing_autocast: false,
            reincarnation_host: None,
            reincarnation_ready: false,
            reincarnation_expire_time: 0,
            enable_group_recall: false,
            last_recall_time: 0,
            allow_lover_recall: false,
            is_gm: false,
            is_dead: false,
            pk_points: 0,
            pk_kill_count: 0,
            buffs: Vec::new(),
        }
    }

    #[test]
    fn test_spouse_initial() {
        assert!(make_state().spouse_name.is_none());
    }

    #[test]
    fn test_set_spouse() {
        let mut s = make_state();
        s.spouse_name = Some("Partner".to_string());
        assert_eq!(s.spouse_name, Some("Partner".to_string()));
        s.spouse_name = None;
        assert!(s.spouse_name.is_none());
    }

    #[test]
    fn test_allow_mentor_toggle() {
        let mut s = make_state();
        assert!(!s.allow_mentor);
        s.allow_mentor = true;
        assert!(s.allow_mentor);
        s.allow_mentor = false;
        assert!(!s.allow_mentor);
    }

    #[test]
    fn test_set_mentor() {
        let mut s = make_state();
        assert!(s.mentor_name.is_none());
        s.mentor_name = Some("Master".to_string());
        assert_eq!(s.mentor_name, Some("Master".to_string()));
        s.mentor_name = None;
        assert!(s.mentor_name.is_none());
    }

    #[test]
    fn test_married_can_have_mentor() {
        // A married player can still have a mentor
        let mut s = make_state();
        s.spouse_name = Some("Spouse".to_string());
        s.mentor_name = Some("Master".to_string());
        assert!(s.spouse_name.is_some());
        assert!(s.mentor_name.is_some());
    }
}
