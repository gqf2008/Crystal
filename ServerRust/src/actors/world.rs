// WorldActor - 游戏世界主循环
// 对应 C# GameSrv/WorldServer.cs + M2Server 核心逻辑

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::prelude::Context;
use kameo::message::Message;
use tokio::time::{interval, Duration};
use tracing::{info, debug, warn};

use crate::actors::player::{PlayerActor, MoveType, MoveRequest, TurnRequest, BroadcastMovement, GetPlayerState, SetMapData, AttackRequest, TakeDamage};
use crate::gate::actor::{SendToClient, GateActor};
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
}

/// 世界中的玩家记录
#[derive(Clone)]
struct PlayerRecord {
    /// PlayerActor 引用
    actor_ref: ActorRef<PlayerActor>,
    /// Session ID（用于路由到 GateActor）
    session_id: u64,
}

/// NPC 定义（从刷怪配置加载）
#[derive(Debug, Clone)]
pub struct NpcSpawn {
    pub name: String,
    pub image: u16, // Monster enum value
    pub x: i32,
    pub y: i32,
    pub direction: u8,
}

/// 怪物定义（从刷怪配置加载）
#[derive(Debug, Clone)]
pub struct MonsterSpawn {
    pub name: String,
    pub image: u16, // Monster enum value
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
                    }).collect(),
                    monsters: raw.monsters.into_iter().map(|m| MonsterSpawn {
                        name: m.name,
                        image: m.image,
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

/// 运行时怪物状态
struct MonsterState {
    pub object_id: u32,
    pub name: String,
    pub image: u16,
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
    /// 下次可攻击的 tick
    pub next_attack_tick: u64,
}

fn dist_to_spawn(monster: &MonsterState) -> i32 {
    (monster.x - monster.spawn_x).abs() + (monster.y - monster.spawn_y).abs()
}

/// 运行时 NPC 状态
#[allow(dead_code)]
struct NpcState {
    pub object_id: u32,
    pub name: String,
    pub image: u16,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
}

/// 方向增量 (8 方向 MirDirection)
const MON_DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const MON_DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

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
}

impl WorldActor {
    pub fn new(gate_ref: ActorRef<GateActor>, map_dir: PathBuf, spawn_dir: Option<PathBuf>) -> Self {
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
        }
    }

    /// 加载或获取已缓存的地图
    fn get_or_load_map(&mut self, file_name: &str) -> Option<&MapData> {
        if !self.maps.contains_key(&0) || self.maps.get(&0).map(|m| m.file_name != file_name).unwrap_or(true) {
            match loader::load_map(file_name, &self.map_dir) {
                Ok(map) => {
                    info!("Loaded map: {} ({}x{})", map.file_name, map.width, map.height);
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
        let mut body = Vec::new();
        // NPCGoods: [count: i32 LE][items...][rate: f32 LE][panel_type: u8][hide_added_stats: bool]
        body.extend_from_slice(&0i32.to_le_bytes()); // 空商品列表
        body.extend_from_slice(&1.0f32.to_le_bytes()); // rate = 1.0
        body.push(0u8); // PanelType::Buy
        body.push(0u8); // hide_added_stats = false
        let packet = build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCGoods as i16, &body);
        let _ = self.gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });
        debug!("Sent empty goods list from NPC '{}' to session {}", npc.name, session_id);
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

/// 聊天请求（从 GateActor 转发）
pub struct ChatRequest {
    pub session_id: u64,
    pub message: String,
}

/// NPC 对话请求（从 GateActor 转发）
pub struct NPCCallRequest {
    pub session_id: u64,
    pub npc_object_id: u32,
    pub key: String,
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
                        results.push((*session_id, state.x, state.y, state.object_id));
                    }
                }
                results
            };

            // 对每个怪物执行 AI
            let mut dead_monsters = Vec::new();
            let mut moved_monsters = Vec::new();

            for (oid, monster) in &mut self.monsters {
                // 找最近玩家
                let mut nearest: Option<(u64, i32, i32, i32)> = None; // (session, px, py, dist)
                for (session, px, py, _) in &player_positions {
                    let dist = (monster.x - px).abs() + (monster.y - py).abs();
                    if nearest.map_or(true, |n| dist < n.3) {
                        nearest = Some((*session, *px, *py, dist));
                    }
                }

                let (aggro_range, attack_range, attack_cooldown_ticks) = (10, 1, 5);

                if let Some((target_session, px, py, dist)) = nearest {
                    if dist <= attack_range && self.tick_count >= monster.next_attack_tick {
                        // 攻击范围内且冷却完毕 → 攻击最近玩家
                        let damage = ((self.tick_count.wrapping_add(*oid as u64).wrapping_mul(7)) as i32 % (monster.max_dmg - monster.min_dmg + 1))
                            + monster.min_dmg;
                        debug!("Monster '{}' (#{}) attacks Player {} for {} dmg", monster.name, *oid, target_session, damage);

                        // 记录攻击冷却
                        monster.next_attack_tick = self.tick_count + attack_cooldown_ticks;

                        // 发送 ObjectAttack 动画
                        let mut attack_body = Vec::new();
                        attack_body.extend_from_slice(&monster.object_id.to_le_bytes());
                        attack_body.extend_from_slice(&(monster.x as u32).to_le_bytes());
                        attack_body.extend_from_slice(&(monster.y as u32).to_le_bytes());
                        attack_body.push(monster.direction);
                        attack_body.push(0u8); // spell
                        attack_body.extend_from_slice(&0u16.to_le_bytes());
                        attack_body.push(0u8);
                        let attack_packet = build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ObjectAttack as i16, &attack_body);
                        let _ = self.gate_ref.ask(SendToClient {
                            session_id: target_session,
                            data: attack_packet,
                        });

                        // 让玩家扣血
                        if let Some(record) = self.players.get(&target_session) {
                            let _ = record.actor_ref.ask(TakeDamage {
                                attacker_id: monster.object_id,
                                attacker_session: target_session,
                                damage: damage as i32,
                            });
                        }
                    } else if dist <= aggro_range && dist > attack_range {
                        // 在仇恨范围内但不在攻击范围 → 走向玩家
                        let (nx, ny, dir) = monster.step_toward(px, py);
                        moved_monsters.push((*oid, nx, ny, dir));
                    }
                } else if dist_to_spawn(monster) > 2 {
                    // 没有玩家在仇恨范围 → 回出生点
                    let (nx, ny, dir) = monster.step_toward(monster.spawn_x, monster.spawn_y);
                    moved_monsters.push((*oid, nx, ny, dir));
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

                    // 发放经验给最近的玩家
                    let mut nearest_session: Option<u64> = None;
                    let mut nearest_dist = i32::MAX;
                    for (session_id, record) in &self.players {
                        if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
                            let dist = (state.x - monster.x).abs() + (state.y - monster.y).abs();
                            if dist < nearest_dist {
                                nearest_dist = dist;
                                nearest_session = Some(*session_id);
                            }
                        }
                    }
                    if let Some(session_id) = nearest_session {
                        if let Some(record) = self.players.get(&session_id) {
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
            self.monsters.insert(new_oid, MonsterState {
                object_id: new_oid,
                name: spawn.name.clone(),
                image: spawn.image,
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
                next_attack_tick: 0,
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
}

impl Message<StartGameRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StartGameRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        info!(
            "StartGame: session={}, character_index={}",
            msg.session_id, msg.character_index
        );

        let object_id = self.alloc_object_id();
        let player_name = format!("Player_{}", object_id);

        // 加载默认地图 "n0"
        let map_file = "n0";
        if self.get_or_load_map(map_file).is_some() {
            info!("Map '{}' loaded for player {}", map_file, player_name);
        }

        // 创建 PlayerActor
        let player_ref = PlayerActor::spawn((
            object_id,
            player_name.clone(),
            msg.session_id,
            0, // map_index
            self.gate_ref.clone(),
        ));

        // 将地图数据注入 PlayerActor
        if let Some(map_data) = self.maps.get(&0).cloned() {
            let _ = player_ref.ask(SetMapData { map: map_data });
        }

        self.players.insert(msg.session_id, PlayerRecord {
            actor_ref: player_ref,
            session_id: msg.session_id,
        });

        info!("Player {} entered world (object_id={}, session={})",
              player_name, object_id, msg.session_id);

        // 多玩家可见性：向新玩家发送已有玩家的 ObjectPlayer
        let existing_players: Vec<_> = self.players.values()
            .filter(|r| r.session_id != msg.session_id)
            .cloned()
            .collect();

        for existing in &existing_players {
            if let Ok(Some(state)) = existing.actor_ref.ask(GetPlayerState).await {
                let packet = build_object_player_packet(
                    &state.name, state.object_id, state.x, state.y, state.direction, 1,
                );
                let _ = self.gate_ref.ask(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                });
            }
        }

        // 向已有玩家发送新玩家的 ObjectPlayer
        let new_player_packet = build_object_player_packet(
            &player_name, object_id, 330, 330, 4, 1,
        );
        for existing in &existing_players {
            let _ = self.gate_ref.ask(SendToClient {
                session_id: existing.session_id,
                data: new_player_packet.clone(),
            });
        }

        // 发送游戏进入序列
        send_game_entry_sequence(self.gate_ref.clone(), msg.session_id, &player_name, object_id);

        // 发送地图上的 NPC 和怪物
        let spawn_dir = self.spawn_dir.clone();
        let (new_npcs, new_monsters) = spawn_npcs_and_monsters(self.gate_ref.clone(), &spawn_dir, map_file, msg.session_id, &mut || self.alloc_object_id());
        for npc in new_npcs {
            self.npcs.insert(npc.object_id, npc);
        }
        for monster in new_monsters {
            self.monsters.insert(monster.object_id, monster);
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
            Some(r) => r,
            None => {
                warn!("Attack request for unknown session {}", msg.session_id);
                return;
            }
        };

        // 发送攻击请求到 PlayerActor
        if let Ok(Some(result)) = record.actor_ref.ask(AttackRequest {
            session_id: msg.session_id,
            direction: msg.direction,
            spell: msg.spell,
        }).await {
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
                    // 命中怪物
                    let tick_hash = (self.tick_count.wrapping_mul(6364136223846793005).wrapping_add(result.object_id as u64)) as i32;
                    let damage = (tick_hash.abs() % 5) + 1;
                    monster.hp = monster.hp.saturating_sub(damage);
                    debug!("Player {} hit monster '{}' (#{}) for {} dmg (hp={}/{})",
                           result.object_id, monster.name, *oid, damage, monster.hp, monster.max_hp);

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
                            let tick_hash = (self.tick_count.wrapping_mul(6364136223846793005).wrapping_add(result.object_id as u64).wrapping_add(other_session)) as i32;
                            let damage = (tick_hash.abs() % 5) + 1; // 1-5 damage
                            let _ = other_actor.ask(TakeDamage {
                                attacker_id: result.object_id,
                                attacker_session: msg.session_id,
                                damage,
                            });
                            debug!("Hit! {} damaged {} for {} (dist={})",
                                   result.object_id, other_state.name, damage, dist);
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

        // 获取玩家位置
        let player_pos = if let Ok(Some(state)) = record.actor_ref.ask(GetPlayerState).await {
            (state.x, state.y)
        } else {
            return;
        };

        // 查找对应的 NPC
        let npc = match self.npcs.get(&msg.npc_object_id) {
            Some(n) => n,
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

        debug!("Player called NPC '{}' (#{}) with key='{}'", npc.name, msg.npc_object_id, msg.key);

        // 发送 NPCResponse 对话页面
        let dialog_lines = match msg.key.as_str() {
            "[@Main]" => vec![
                format!("欢迎来到{}", npc.name),
                "有什么我可以帮你的吗？".to_string(),
            ],
            "[@Buy]" => {
                // 触发 NPC 商店（Phase 2：发送空商品列表打开商店 UI）
                self.send_npc_goods(msg.session_id, npc);
                return;
            }
            _ => vec![
                format!("{} 说：", npc.name),
                format!("你说了：{}", msg.key),
            ],
        };

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
// 游戏进入序列
// ============================================================

/// 发送完整的游戏进入序列到客户端
fn send_game_entry_sequence(
    gate_ref: ActorRef<GateActor>,
    session_id: u64,
    player_name: &str,
    object_id: u32,
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
    let map_changed = build_map_changed_packet();
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: map_changed,
    });

    // 3. UserInformation
    let user_info = build_user_information_packet(player_name, object_id);
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: user_info,
    });

    // 4. HealthChanged
    let mut health_body = Vec::new();
    health_body.extend_from_slice(&120u32.to_le_bytes());
    health_body.extend_from_slice(&60u32.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::HealthChanged as i16, &health_body),
    });

    // 5. UserLocation
    let mut location_body = Vec::new();
    location_body.extend_from_slice(&330i32.to_le_bytes());
    location_body.extend_from_slice(&330i32.to_le_bytes());
    location_body.push(4u8);
    let _ = gate_ref.ask(SendToClient {
        session_id: sid,
        data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &location_body),
    });

    info!("Game entry sequence sent to session {}", sid);
}

// ============================================================
// 数据包构建辅助函数
// ============================================================

fn build_map_changed_packet() -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&0i32.to_le_bytes());
    write_dotnet_string(&mut body, "n0");
    write_dotnet_string(&mut body, "Beginner Training");
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body.push(1u8);
    body.extend_from_slice(&330i32.to_le_bytes());
    body.extend_from_slice(&330i32.to_le_bytes());
    body.push(4u8);
    body.push(1u8);
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());

    build_packet_bytes(ServerPacketIds::MapChanged as i16, &body)
}

fn build_user_information_packet(player_name: &str, object_id: u32) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    // --- 字段顺序必须与客户端 UserInformation::read_body 一致 ---
    body.extend_from_slice(&object_id.to_le_bytes());   // object_id
    body.extend_from_slice(&1u32.to_le_bytes());        // real_id
    write_dotnet_string(&mut body, player_name);        // name
    write_dotnet_string(&mut body, "");                 // guild_name
    write_dotnet_string(&mut body, "");                 // guild_rank
    body.extend_from_slice(&0i32.to_le_bytes());        // name_colour
    body.push(0u8);                                     // class=Warrior
    body.push(0u8);                                     // gender=Male
    body.extend_from_slice(&1u16.to_le_bytes());        // level
    body.extend_from_slice(&330i32.to_le_bytes());      // location_x
    body.extend_from_slice(&330i32.to_le_bytes());      // location_y
    body.push(4u8);                                     // direction=Down
    body.push(0u8);                                     // hair
    body.extend_from_slice(&120i32.to_le_bytes());      // hp
    body.extend_from_slice(&60i32.to_le_bytes());       // mp
    body.extend_from_slice(&0i64.to_le_bytes());        // experience
    body.extend_from_slice(&100i64.to_le_bytes());      // max_experience
    body.extend_from_slice(&0u16.to_le_bytes());        // level_effects
    body.push(0u8);                                     // has_hero=false
    body.push(0u8);                                     // hero_behaviour=None

    // 客户端期望的后续字段（read_body 继续读取的部分）
    body.push(0u8);                                     // has_inventory=false
    body.push(0u8);                                     // has_equipment=false
    body.push(0u8);                                     // has_quest_inventory=false
    body.extend_from_slice(&0u32.to_le_bytes());        // gold
    body.extend_from_slice(&0u32.to_le_bytes());        // credit
    body.push(0u8);                                     // has_expanded_storage=false
    body.extend_from_slice(&0i64.to_le_bytes());        // expanded_storage_expiry_time
    body.extend_from_slice(&0i32.to_le_bytes());        // magic_count=0
    body.extend_from_slice(&0i32.to_le_bytes());        // creature_count=0
    body.push(0u8);                                     // summoned_creature_type
    body.push(0u8);                                     // creature_summoned=false
    body.push(0u8);                                     // allow_observe=false
    body.push(0u8);                                     // observer=false

    build_packet_bytes(ServerPacketIds::UserInformation as i16, &body)
}

/// 构建 ObjectPlayer 数据包（其他玩家进入视野）
fn build_object_player_packet(
    name: &str, object_id: u32, x: i32, y: i32, direction: u8, level: u16,
) -> Vec<u8> {
    use mir2_shared::enums::ServerPacketIds;
    let mut body = Vec::new();

    body.extend_from_slice(&object_id.to_le_bytes());   // object_id
    write_dotnet_string(&mut body, name);               // name
    write_dotnet_string(&mut body, "");                 // guild_name
    write_dotnet_string(&mut body, "");                 // guild_rank_name
    body.extend_from_slice(&0i32.to_le_bytes());        // name_colour
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
    alloc_object_id: &mut dyn FnMut() -> u32,
) -> (Vec<NpcState>, Vec<MonsterState>) {
    let spawn_dir = match spawn_dir {
        Some(d) => d,
        None => return (Vec::new(), Vec::new()),
    };

    let config = load_spawn_config(map_file, spawn_dir);
    if config.npcs.is_empty() && config.monsters.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // 发送 NPC 并创建运行时状态
    let mut npcs = Vec::new();
    for npc in &config.npcs {
        let object_id = alloc_object_id();
        let packet = build_object_npc_packet(npc, object_id);
        let _ = gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });

        npcs.push(NpcState {
            object_id,
            name: npc.name.clone(),
            image: npc.image,
            x: npc.x,
            y: npc.y,
            direction: npc.direction,
        });
    }

    // 发送怪物并创建运行时状态
    let mut monsters = Vec::new();
    for monster in &config.monsters {
        let object_id = alloc_object_id();
        let packet = build_object_monster_packet(monster, object_id, &monster.name);
        let _ = gate_ref.ask(SendToClient {
            session_id,
            data: packet,
        });

        monsters.push(MonsterState {
            object_id,
            name: monster.name.clone(),
            image: monster.image,
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
            next_attack_tick: 0,
        });
    }

    info!("Spawned {} NPCs and {} monsters for session {}",
          config.npcs.len(), config.monsters.len(), session_id);
    (npcs, monsters)
}
