// PlayerActor - 玩家实例
// 持有单个玩家的完整状态：位置、方向、地图、背包等
// 移动由客户端驱动，服务端验证并广播

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use tracing::{debug, info, warn};

use crate::gate::actor::{GateActor, SendToClient};
use crate::maps::loader::MapData;
use crate::util::wire::build_packet_bytes;

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
    /// 是否隐藏
    pub hidden: bool,
    /// 所属 session
    pub session_id: u64,
    /// 等级
    pub level: u16,
    /// 当前经验
    pub experience: i64,
    /// 升级所需经验
    pub max_experience: i64,
    /// 当前 HP
    pub hp: i32,
    /// 最大 HP
    pub max_hp: i32,
    /// 当前 MP
    pub mp: i32,
    /// 最大 MP
    pub max_mp: i32,
    /// 最小攻击力
    pub min_attack: i32,
    /// 最大攻击力
    pub max_attack: i32,
    /// 防御力
    pub defence: i32,
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
                hidden: false,
                session_id,
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
    type Reply = ();

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
            self.state.max_hp += 10;
            self.state.hp = self.state.max_hp;
            self.state.max_mp += 5;
            self.state.mp = self.state.max_mp;
            self.state.max_experience = (self.state.max_experience as f64 * 1.5) as i64;

            info!("Player {} leveled up to {}!", self.state.name, self.state.level);

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

/// 攻击结果（返回给 WorldActor 用于广播）
pub struct AttackResult {
    pub object_id: u32,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    pub spell: u8,
}
