// GateActor - TCP 接入层
// 对应 C# LoginGate/AppServer.cs + SelGate + GameGate
// 职责：接受 TCP 连接，解析帧，转发到 AccountActor/WorldActor

use std::collections::HashMap;

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

use mir2_shared::enums::{ClientPacketIds, ServerPacketIds};
use mir2_shared::packets::Packet;

use super::codec::{encode, decode};
use crate::util::wire::build_packet_bytes;

/// 会话 ID
pub type SessionId = u64;

/// 发送到客户端的数据通道
type SendChannel = mpsc::UnboundedSender<Vec<u8>>;

/// GateActor 状态
pub struct GateActor {
    /// 活跃会话的发送通道
    sessions: HashMap<SessionId, SendChannel>,
    /// 会话关联的用户名（登录成功后设置）
    session_usernames: HashMap<SessionId, String>,
    /// 会话关联的客户端 IP（C# MirConnection.IPAddress）
    session_ips: HashMap<SessionId, String>,
    /// 被封禁 IP -> 解封时间（unix 秒；C# Envir.IPBlocks）
    ip_blocks: HashMap<String, i64>,
    /// 每 IP 创建角色时间戳（unix 秒；C# ConnectionLogs[IP].CharactersMade）
    ip_character_creations: HashMap<String, Vec<i64>>,
    /// AccountActor 引用
    account_ref: Option<ActorRef<crate::actors::account::AccountActor>>,
    /// WorldActor 引用
    world_ref: Option<ActorRef<crate::actors::world::WorldActor>>,
    /// SocialActor 引用
    social_ref: Option<ActorRef<crate::actors::social::SocialActor>>,
    /// 最大并发连接数(Phase 1.1:防止资源耗尽;从 cfg.network.max_connections 设置)
    max_connections: usize,
}

impl GateActor {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_usernames: HashMap::new(),
            session_ips: HashMap::new(),
            ip_blocks: HashMap::new(),
            ip_character_creations: HashMap::new(),
            account_ref: None,
            world_ref: None,
            social_ref: None,
            max_connections: 1024,
        }
    }

    pub fn set_account_ref(&mut self, account_ref: ActorRef<crate::actors::account::AccountActor>) {
        self.account_ref = Some(account_ref);
    }

    pub fn set_world_ref(&mut self, world_ref: ActorRef<crate::actors::world::WorldActor>) {
        self.world_ref = Some(world_ref);
    }

    pub fn set_social_ref(&mut self, social_ref: ActorRef<crate::actors::social::SocialActor>) {
        self.social_ref = Some(social_ref);
    }
}

impl Actor for GateActor {
    type Args = ();
    type Error = anyhow::Error;

    async fn on_start(_args: (), _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        info!("GateActor started");
        Ok(Self::new())
    }
}

impl Default for GateActor {
    fn default() -> Self {
        Self::new()
    }
}

/// 启动 TCP 监听并处理连接
pub async fn run_gate_listener(addr: String, actor_ref: ActorRef<GateActor>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!("Gate listening on {}", addr);

    let mut session_id: SessionId = 1;

    loop {
        let (mut stream, peer_addr) = listener.accept().await?;
        debug!("New connection from {}", peer_addr);

        let sid = session_id;
        session_id += 1;

        // 为每个会话创建发送通道
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // 注册会话到 GateActor
        let _ = actor_ref.ask(SessionCreated {
            session_id: sid,
            sender: tx,
            ip: peer_addr.ip().to_string(),
        }).await;

        let gate_ref = actor_ref.clone();

        tokio::spawn(async move {
            let mut buf = Vec::with_capacity(4096);
            let mut temp = [0u8; 4096];

            loop {
                tokio::select! {
                    // 从网络读取数据
                    read_result = stream.read(&mut temp) => {
                        match read_result {
                            Ok(0) => {
                                debug!("Session {} disconnected", sid);
                                let _ = gate_ref.ask(ClientDisconnected { session_id: sid }).await;
                                return;
                            }
                            Ok(n) => {
                                buf.extend_from_slice(&temp[..n]);

                                // 尝试解码所有完整帧
                                while let Some((payload, consumed)) = decode(&buf) {
                                    let _ = gate_ref.ask(ClientData {
                                        session_id: sid,
                                        data: payload,
                                    }).await;
                                    buf.drain(..consumed);
                                }
                            }
                            Err(e) => {
                                error!("Session {} read error: {}", sid, e);
                                let _ = gate_ref.ask(ClientDisconnected { session_id: sid }).await;
                                return;
                            }
                        }
                    }
                    // 发送数据到客户端
                    Some(data) = rx.recv() => {
                        let mut encoded = Vec::new();
                        encode(&data, &mut encoded);
                        if let Err(e) = stream.write_all(&encoded).await {
                            error!("Session {} write error: {}", sid, e);
                            let _ = gate_ref.ask(ClientDisconnected { session_id: sid }).await;
                            return;
                        }
                    }
                }
            }
        });
    }
}

// ============================================================
// 消息定义
// ============================================================

/// 会话创建（内部）
pub struct SessionCreated {
    pub session_id: SessionId,
    pub sender: SendChannel,
    /// 客户端 IP（C# MirConnection.IPAddress，用于 IPBlocks 防刷）
    pub ip: String,
}

/// Phase 2.2: 优雅关机 — 断开所有 session,触发自动保存。
pub struct ShutdownAll;

/// Phase 1.1: 设置最大并发连接数(由 main.rs 从 cfg 传入)
pub struct SetMaxConnections(pub usize);

/// 收到客户端数据
pub struct ClientData {
    pub session_id: SessionId,
    pub data: Vec<u8>,
}

/// 向客户端发送数据
pub struct SendToClient {
    pub session_id: SessionId,
    pub data: Vec<u8>,
}

/// 客户端断开连接
pub struct ClientDisconnected {
    pub session_id: SessionId,
}

/// 登录结果（从 AccountActor 返回）
pub struct LoginResult {
    pub session_id: SessionId,
    pub success: bool,
    pub username: String,
    /// 角色摘要列表（登录成功时携带，用于选角界面）
    pub characters: Vec<crate::db::CharacterSummary>,
    /// 封禁到期时间（unix 秒；Some 时发 S.LoginBanned，C# WrongPasswordCount>=5 封 2 分钟）
    pub banned_until: Option<i64>,
}

/// 设置 AccountActor 引用
pub struct SetAccountRef {
    pub account_ref: ActorRef<crate::actors::account::AccountActor>,
}

/// 设置 WorldActor 引用
pub struct SetWorldRef {
    pub world_ref: ActorRef<crate::actors::world::WorldActor>,
}

/// 设置 SocialActor 引用（组队/交易/好友等社交转发）
pub struct SetSocialRef {
    pub social_ref: ActorRef<crate::actors::social::SocialActor>,
}

// ============================================================
// Handler 实现
// ============================================================

/// 当前 unix 秒（同步；IP 防刷用）
fn gate_unix_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Message<SessionCreated> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionCreated,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // C# Envir.IPBlocks：被封禁 IP 不接收连接（不注册会话，客户端超时断开）
        let now = gate_unix_now_secs();
        if self.ip_blocks.get(&msg.ip).map(|&u| u > now).unwrap_or(false) {
            warn!("Connection rejected from blocked IP {} (session {})", msg.ip, msg.session_id);
            return;
        }
        // Phase 1.1: 连接数限制 — 超过 max_connections 拒绝新连接
        if self.sessions.len() >= self.max_connections {
            warn!(
                "Connection rejected: session {} would exceed max_connections {} (current={})",
                msg.session_id, self.max_connections, self.sessions.len()
            );
            // 不 insert session,不发 Connected — 客户端会因为收不到响应而超时断开
            return;
        }
        self.sessions.insert(msg.session_id, msg.sender);
        self.session_ips.insert(msg.session_id, msg.ip.clone());
        debug!("Session {} created (active={})", msg.session_id, self.sessions.len());

        // 发送 Connected 包给客户端（客户端收到后会自动发送 ClientVersion）
        let connected_data = build_packet_bytes(ServerPacketIds::Connected as i16, &[]);
        let gate_ref = _ctx.actor_ref().clone();
        let _ = gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: connected_data,
            }).await;
    }
}

/// Phase 1.1: 设置最大并发连接数
impl Message<SetMaxConnections> for GateActor {
    type Reply = ();

    async fn handle(&mut self, msg: SetMaxConnections, _ctx: &mut Context<Self, Self::Reply>) {
        self.max_connections = msg.0;
        info!("GateActor max_connections set to {}", self.max_connections);
    }
}

/// Phase 2.2: 优雅关机 — 断开所有活跃 session,触发 PlayerDisconnected 保存。
impl Message<ShutdownAll> for GateActor {
    type Reply = usize;

    async fn handle(&mut self, _msg: ShutdownAll, ctx: &mut Context<Self, Self::Reply>) -> usize {
        let count = self.sessions.len();
        info!("ShutdownAll: disconnecting {} active sessions", count);
        let session_ids: Vec<u64> = self.sessions.keys().cloned().collect();
        for sid in &session_ids {
            let disconnect_data = crate::util::wire::build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::Disconnect as i16,
                // C# S.Disconnect.Reason：0=Server Closing（1 字节）
                &[0u8],
            );
            let _ = ctx.actor_ref().tell(SendToClient {
                session_id: *sid,
                data: disconnect_data,
            }).await;
        }
        count
    }
}

impl Message<ClientData> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ClientData,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        debug!(
            "Session {} received {} bytes (decoded)",
            msg.session_id,
            msg.data.len()
        );

        // 解析内层 PacketHeader (4 bytes: length u16 + opcode i16)
        const HEADER_SIZE: usize = 4;
        if msg.data.len() < HEADER_SIZE {
            warn!("Session {} received data too short for packet header", msg.session_id);
            return;
        }

        let length = u16::from_le_bytes([msg.data[0], msg.data[1]]) as usize;
        let opcode = i16::from_le_bytes([msg.data[2], msg.data[3]]);

        debug!("Session {} packet: length={}, opcode={}", msg.session_id, length, opcode);

        // 验证长度一致性
        if length > msg.data.len() || length < HEADER_SIZE {
            warn!("Session {} packet length mismatch: declared={}, available={}",
                  msg.session_id, length, msg.data.len());
            return;
        }

        let payload = &msg.data[HEADER_SIZE..length];
        let gate_ref = ctx.actor_ref().clone();

        match opcode {
            x if x == ClientPacketIds::ClientVersion as i16 => {
                // ClientVersion - 验证 payload 后回复 accepted
                handle_client_version(&gate_ref, msg.session_id, payload).await;
            }
            x if x == ClientPacketIds::NewAccount as i16 => {
                // NewAccount - Phase 1: 自动成功
                handle_new_account(&gate_ref, msg.session_id).await;
            }
            x if x == ClientPacketIds::Login as i16 => {
                // Login - 转发到 AccountActor (Phase 1.3: 输入验证)
                if let Some(account_ref) = &self.account_ref {
                    if let Some((username, password)) = parse_login_payload(payload) {
                        if !crate::util::validation::validate_username(&username) {
                            warn!("Login rejected: invalid username '{}' from session {}", username, msg.session_id);
                        } else if !crate::util::validation::validate_password(&password) {
                            warn!("Login rejected: invalid password length from session {} user={}", msg.session_id, username);
                        } else {
                            debug!("Login request: username={}", username);
                            let _ = account_ref.ask(crate::actors::account::LoginRequest {
                                session_id: msg.session_id,
                                username,
                                password,
                            }).await;
                        }
                    }
                } else {
                    warn!("AccountActor not linked");
                }
            }
            x if x == ClientPacketIds::StartGame as i16 => {
                // StartGame - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if payload.len() >= 4 {
                        let character_index = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                        debug!("StartGame request: character_index={}", character_index);
                        // 必须已登录才能进入游戏
                        if let Some(username) = self.session_usernames.get(&msg.session_id) {
                            let _ = world_ref.ask(crate::actors::world::StartGameRequest {
                                session_id: msg.session_id,
                                character_index,
                                account_username: username.clone(),
                            }).await;
                        } else {
                            warn!("StartGame rejected: session {} not logged in", msg.session_id);
                        }
                    }
                } else {
                    warn!("WorldActor not linked");
                }
            }
            x if x == ClientPacketIds::Turn as i16 => {
                // Turn - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref.ask(crate::actors::world::WorldTurnRequest {
                            session_id: msg.session_id,
                            direction,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::Walk as i16 => {
                // Walk - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref.ask(crate::actors::world::WorldMoveRequest {
                            session_id: msg.session_id,
                            direction,
                            is_run: false,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::Run as i16 => {
                // Run - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref.ask(crate::actors::world::WorldMoveRequest {
                            session_id: msg.session_id,
                            direction,
                            is_run: true,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::Attack as i16 => {
                // Attack - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if payload.len() >= 2 {
                        let direction = payload[0];
                        let spell = payload[1];
                        debug!("Attack: session={} dir={} spell={}", msg.session_id, direction, spell);
                        let _ = world_ref.ask(crate::actors::world::WorldAttackRequest {
                            session_id: msg.session_id,
                            direction,
                            spell,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::KeepAlive as i16 => {
                // KeepAlive - 回复心跳
                handle_keep_alive(&gate_ref, msg.session_id).await;
            }
            x if x == ClientPacketIds::LogOut as i16 => {
                // LogOut - 通知 WorldActor 清理并断开
                if let Some(world_ref) = &self.world_ref {
                    let _ = world_ref.ask(crate::actors::world::PlayerLogOut {
                        session_id: msg.session_id,
                    }).await;
                }
            }
            x if x == ClientPacketIds::Disconnect as i16 => {
                debug!("Client disconnect request from session {}", msg.session_id);
                // Forward to WorldActor for immediate player cleanup
                if let Some(world_ref) = &self.world_ref {
                    let _ = world_ref.ask(crate::actors::world::PlayerDisconnected {
                        session_id: msg.session_id,
                    }).await;
                }
                self.sessions.remove(&msg.session_id);
                let logged_out_username = self.session_usernames.get(&msg.session_id).cloned();
                self.session_usernames.remove(&msg.session_id);
                if let Some(username) = logged_out_username {
                    if let Some(account_ref) = &self.account_ref {
                        let _ = account_ref.ask(crate::actors::account::LogoutRequest {
                            username,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::Chat as i16 => {
                // Chat - 解析并广播 (Phase 1.3: 输入验证)
                if let Some(world_ref) = &self.world_ref {
                    if let Some(message) = parse_chat_payload(payload) {
                        if !crate::util::validation::validate_chat(&message) {
                            warn!("Session {} chat rejected: len={}", msg.session_id, message.len());
                        } else {
                            let _ = world_ref.ask(crate::actors::world::ChatRequest {
                                session_id: msg.session_id,
                                message,
                            }).await;
                        }
                    }
                }
            }
            x if x == ClientPacketIds::CallNPC as i16 => {
                // CallNPC - 与 NPC 对话
                debug!("CallNPC packet len={}", payload.len());
                if let Some(world_ref) = &self.world_ref {
                    if let Some((npc_object_id, key)) = parse_call_npc_payload(payload) {
                        debug!("CallNPC npc={} key={}", npc_object_id, key);
                        let _ = world_ref.ask(crate::actors::world::NPCCallRequest {
                            session_id: msg.session_id,
                            npc_object_id,
                            key,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::PickUp as i16 => {
                if let Some(world_ref) = &self.world_ref {
                    let _ = world_ref.ask(crate::actors::world::PickUpRequest {
                        session_id: msg.session_id,
                    }).await;
                }
            }
            x if x == ClientPacketIds::MoveItem as i16 => {
                forward_move_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::UseItem as i16 => {
                forward_use_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EquipItem as i16 => {
                forward_equip_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveItem as i16 => {
                forward_remove_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DeleteItem as i16 => {
                forward_delete_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DropItem as i16 => {
                forward_drop_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MergeItem as i16 => {
                forward_merge_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RangeAttack as i16 => {
                forward_range_attack(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Magic as i16 => {
                forward_magic(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Harvest as i16 => {
                forward_harvest(&self.world_ref, msg.session_id, payload);
            }
            // NPC 商店
            x if x == ClientPacketIds::BuyItem as i16 => {
                forward_buy_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SellItem as i16 => {
                forward_sell_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RepairItem as i16 => {
                forward_repair_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SRepairItem as i16 => {
                forward_repair_item(&self.world_ref, msg.session_id, payload); // 特殊修理走相同逻辑
            }
            x if x == ClientPacketIds::CraftItem as i16 => {
                forward_craft_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::BuyItemBack as i16 => {
                forward_buy_item_back(&self.world_ref, msg.session_id, payload);
            }
            // 仓库操作
            x if x == ClientPacketIds::StoreItem as i16 => {
                handle_store_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TakeBackItem as i16 => {
                handle_take_back_item(&self.world_ref, msg.session_id, payload);
            }
            // 金币
            x if x == ClientPacketIds::DropGold as i16 => {
                handle_drop_gold(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Inspect as i16 => {
                handle_inspect(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangeAMode as i16 => {
                forward_change_amode(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangePMode as i16 => {
                forward_change_pmode(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MagicKey as i16 => {
                forward_magic_key(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveSlotItem as i16 => {
                forward_remove_slot_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SplitItem as i16 => {
                forward_split_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TeleportToNPC as i16 => {
                forward_teleport_to_npc(&self.world_ref, msg.session_id, payload);
            }
            // 死亡恢复
            x if x == ClientPacketIds::TownRevive as i16 => {
                forward_town_revive(&self.world_ref, msg.session_id);
            }
            x if x == ClientPacketIds::SpellToggle as i16 => {
                forward_spell_toggle(&self.world_ref, msg.session_id, payload);
            }
            // 账号管理
            x if x == ClientPacketIds::NewCharacter as i16 => {
                // C# Envir.NewCharacter IP 防刷：封禁 IP / 每小时 >4 次 → 封 24h
                let now = gate_unix_now_secs();
                let ip = self.session_ips.get(&msg.session_id).cloned().unwrap_or_default();
                let mut blocked = false;
                if !ip.is_empty() {
                    if self.ip_blocks.get(&ip).map(|&u| u > now).unwrap_or(false) {
                        blocked = true;
                    } else {
                        let creations = self.ip_character_creations.entry(ip.clone()).or_default();
                        if creations.len() > 4 {
                            self.ip_blocks.insert(ip.clone(), now + 24 * 3600);
                            creations.clear();
                            blocked = true;
                        } else {
                            creations.push(now);
                            // C#：剔除超过 1 小时的记录
                            creations.retain(|&t| t + 3600 >= now);
                        }
                    }
                }
                if blocked {
                    let mut body = Vec::new();
                    body.push(0u8); // S.NewCharacter { Result = 0 }
                    let data = build_packet_bytes(ServerPacketIds::NewCharacter as i16, &body);
                    let gate_ref = ctx.actor_ref().clone();
                    let _ = gate_ref.tell(SendToClient {
                        session_id: msg.session_id,
                        data,
                    }).await;
                    warn!("NewCharacter rejected: IP {} rate-limited (session {})", ip, msg.session_id);
                    return;
                }
                forward_new_character(&self.world_ref, &self.session_usernames, msg.session_id, payload).await;
            }
            x if x == ClientPacketIds::ChangePassword as i16 => {
                forward_change_password(&self.account_ref, &self.session_usernames, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DeleteCharacter as i16 => {
                forward_delete_character(&self.world_ref, &self.session_usernames, msg.session_id, payload);
            }

            // ===== PR #1169: Warehouse password (client -> server) =====
            x if x == ClientPacketIds::UnlockStorage as i16 => {
                forward_unlock_storage(
                    &self.account_ref,
                    &self.world_ref,
                    &self.session_usernames,
                    msg.session_id,
                    payload,
                )
                .await;
            }
            x if x == ClientPacketIds::SetStoragePassword as i16 => {
                forward_set_storage_password(&self.account_ref, &self.session_usernames, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveStoragePassword as i16 => {
                forward_remove_storage_password(&self.account_ref, &self.session_usernames, msg.session_id, payload);
            }
            // 社交/组队
            x if x == ClientPacketIds::SwitchGroup as i16 => {
                forward_switch_group(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AddMember as i16 => {
                forward_add_member(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DellMember as i16 => {
                forward_dell_member(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GroupInvite as i16 => {
                forward_group_invite(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::NewHero as i16 => {
                forward_new_hero(&self.world_ref, msg.session_id, payload);
            }
            // 交易
            x if x == ClientPacketIds::ChangeTrade as i16 => {
                forward_change_trade(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeRequest as i16 => {
                forward_trade_request(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeReply as i16 => {
                forward_trade_reply(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeConfirm as i16 => {
                forward_trade_confirm(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeCancel as i16 => {
                forward_trade_cancel(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeGold as i16 => {
                forward_trade_gold(&self.social_ref, msg.session_id, payload);
            }
            // 好友
            x if x == ClientPacketIds::AddFriend as i16 => {
                forward_add_friend(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveFriend as i16 => {
                forward_remove_friend(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefreshFriends as i16 => {
                forward_refresh_friends(&self.social_ref, msg.session_id);
            }
            x if x == ClientPacketIds::AddMemo as i16 => {
                forward_add_memo(&self.social_ref, msg.session_id, payload);
            }
            // 邮件
            x if x == ClientPacketIds::SendMail as i16 => {
                handle_send_mail(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ReadMail as i16 => {
                handle_read_mail(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CollectParcel as i16 => {
                handle_collect_parcel(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DeleteMail as i16 => {
                handle_delete_mail(&self.world_ref, msg.session_id, payload);
            }
            // 行会
            x if x == ClientPacketIds::GuildInvite as i16 => {
                handle_guild_invite(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestGuildInfo as i16 => {
                handle_request_guild_info(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EditGuildMember as i16 => {
                handle_edit_guild_member(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EditGuildNotice as i16 => {
                handle_edit_guild_notice(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildNameReturn as i16 => {
                handle_guild_name_return(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildStorageGoldChange as i16 => {
                handle_guild_storage_gold(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildStorageItemChange as i16 => {
                handle_guild_storage_item(&self.social_ref, msg.session_id, payload);
            }
            // 婚姻
            x if x == ClientPacketIds::MarriageRequest as i16 => {
                handle_marriage_request(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarriageReply as i16 => {
                handle_marriage_reply(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangeMarriage as i16 => {
                handle_change_marriage(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DivorceRequest as i16 => {
                handle_divorce_request(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DivorceReply as i16 => {
                handle_divorce_reply(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AddMentor as i16 => {
                handle_add_mentor(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MentorReply as i16 => {
                handle_mentor_reply(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AllowMentor as i16 => {
                handle_allow_mentor(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CancelMentor as i16 => {
                handle_cancel_mentor(&self.social_ref, msg.session_id, payload);
            }
            // 任务
            x if x == ClientPacketIds::AcceptQuest as i16 => {
                handle_accept_quest(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::FinishQuest as i16 => {
                handle_finish_quest(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AbandonQuest as i16 => {
                handle_abandon_quest(&self.world_ref, msg.session_id, payload);
            }
            //  精炼
            x if x == ClientPacketIds::DepositRefineItem as i16 => {
                handle_deposit_refine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RetrieveRefineItem as i16 => {
                handle_retrieve_refine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefineCancel as i16 => {
                handle_refine_cancel(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefineItem as i16 => {
                handle_refine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CheckRefine as i16 => {
                handle_check_refine(&self.world_ref, msg.session_id, payload);
            }
            // 传送/地图
            x if x == ClientPacketIds::RequestMapInfo as i16 => {
                forward_request_map_info(&self.world_ref, msg.session_id, payload);
            }

            // ===== PR #1126: KR NPC/Quest Linking — info requests =====
            x if x == ClientPacketIds::RequestMonsterInfo as i16 => {
                forward_request_monster_info(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestNPCInfo as i16 => {
                forward_request_npc_info(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestItemInfo as i16 => {
                forward_request_item_info(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SearchMap as i16 => {
                forward_search_map(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Observe as i16 => {
                forward_observe(&self.world_ref, msg.session_id, payload);
            }
            // 其他
            x if x == ClientPacketIds::RequestUserName as i16 => {
                handle_request_user_name(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestChatItem as i16 => {
                handle_request_chat_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SetAutoPotValue as i16 => {
                forward_set_autopot_value(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SetAutoPotItem as i16 => {
                forward_set_autopot_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SetHeroBehaviour as i16 => {
                forward_set_hero_behaviour(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangeHero as i16 => {
                handle_change_hero(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TakeBackHeroItem as i16 => {
                handle_take_back_hero_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TransferHeroItem as i16 => {
                handle_transfer_hero_item(&self.world_ref, msg.session_id, payload);
            }
            // 宠物
            x if x == ClientPacketIds::UpdateIntelligentCreature as i16 => {
                handle_update_intelligent_creature(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::IntelligentCreaturePickup as i16 => {
                handle_intelligent_creature_pickup(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestIntelligentCreatureUpdates as i16 => {
                handle_request_intelligent_creature_updates(&self.world_ref, msg.session_id, payload);
            }
            // 装备槽
            x if x == ClientPacketIds::EquipSlotItem as i16 => {
                handle_equip_slot_item(&self.world_ref, msg.session_id, payload);
            }
            // 市场/寄售
            x if x == ClientPacketIds::ConsignItem as i16 => {
                forward_consign_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketSearch as i16 => {
                forward_market_search(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketRefresh as i16 => {
                forward_market_refresh(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketPage as i16 => {
                forward_market_page(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketBuy as i16 => {
                forward_market_buy(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketGetBack as i16 => {
                forward_market_get_back(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketSellNow as i16 => {
                forward_market_sell_now(&self.world_ref, msg.session_id, payload);
            }
            // 钓鱼
            x if x == ClientPacketIds::FishingCast as i16 => {
                forward_fishing_cast(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::FishingChangeAutocast as i16 => {
                forward_fishing_change_autocast(&self.world_ref, msg.session_id, payload);
            }
            // 觉醒/分解
            x if x == ClientPacketIds::CombineItem as i16 => {
                forward_combine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AwakeningNeedMaterials as i16 => {
                forward_awakening_need_materials(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AwakeningLockedItem as i16 => {
                forward_awakening_locked_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Awakening as i16 => {
                forward_awakening(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DisassembleItem as i16 => {
                forward_disassemble_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DowngradeAwakening as i16 => {
                forward_downgrade_awakening(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ResetAddedItem as i16 => {
                forward_reset_added_item(&self.world_ref, msg.session_id, payload);
            }
            // 交易子操作
            x if x == ClientPacketIds::DepositTradeItem as i16 => {
                forward_deposit_trade_item(&self.social_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RetrieveTradeItem as i16 => {
                forward_retrieve_trade_item(&self.social_ref, msg.session_id, payload);
            }
            // 行会扩展
            x if x == ClientPacketIds::GuildWarReturn as i16 => {
                forward_guild_war_return(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildBuffUpdate as i16 => {
                forward_guild_buff_update(&self.world_ref, msg.session_id, payload);
            }
            // 婚姻/师徒扩展
            x if x == ClientPacketIds::ReplaceWedRing as i16 => {
                handle_replace_wed_ring(&self.world_ref, msg.session_id, payload);
            }
            // 邮件扩展
            x if x == ClientPacketIds::LockMail as i16 => {
                forward_lock_mail(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MailLockedItem as i16 => {
                forward_mail_locked_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MailCost as i16 => {
                handle_mail_cost(&gate_ref, msg.session_id, payload).await;
            }
            // 轮回
            x if x == ClientPacketIds::ShareQuest as i16 => {
                forward_share_quest(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AcceptReincarnation as i16 => {
                forward_accept_reincarnation(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CancelReincarnation as i16 => {
                forward_cancel_reincarnation(&self.world_ref, msg.session_id, payload);
            }
            // 租赁系统
            x if x == ClientPacketIds::GetRentedItems as i16 => {
                forward_get_rented_items(&self.world_ref, msg.session_id);
            }
            x if x == ClientPacketIds::ItemRentalRequest as i16 => {
                forward_item_rental_request(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalFee as i16 => {
                forward_item_rental_fee(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalPeriod as i16 => {
                forward_item_rental_period(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DepositRentalItem as i16 => {
                forward_deposit_rental_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RetrieveRentalItem as i16 => {
                forward_retrieve_rental_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CancelItemRental as i16 => {
                forward_cancel_item_rental(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalLockFee as i16 => {
                forward_item_rental_lock_fee(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalLockItem as i16 => {
                forward_item_rental_lock_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ConfirmItemRental as i16 => {
                forward_confirm_item_rental(&self.world_ref, msg.session_id, payload);
            }
            // 其他
            x if x == ClientPacketIds::NPCConfirmInput as i16 => {
                forward_npc_confirm_input(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GameshopBuy as i16 => {
                forward_gameshop_buy(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ReportIssue as i16 => {
                forward_report_issue(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GetRanking as i16 => {
                forward_get_ranking(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Opendoor as i16 => {
                forward_opendoor(&self.world_ref, msg.session_id, payload);
            }
            // 行会领地 (auto-value enums)
            x if x == ClientPacketIds::GuildTerritoryPage as i16 => {
                forward_guild_territory_page(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::PurchaseGuildTerritory as i16 => {
                forward_purchase_guild_territory(&self.world_ref, msg.session_id, payload);
            }
            _ => {
                debug!("Unknown opcode {} from session {}", opcode, msg.session_id);
            }
        }
    }
}

impl Message<SendToClient> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SendToClient,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(tx) = self.sessions.get(&msg.session_id) {
            debug!("SendToClient: session={} bytes={}", msg.session_id, msg.data.len());
            let _ = tx.send(msg.data);
        } else {
            warn!(
                "Attempted to send to non-existent session {}",
                msg.session_id
            );
        }
    }
}

impl Message<ClientDisconnected> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ClientDisconnected,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // If session already removed (graceful Disconnect handled it), skip
        if !self.sessions.contains_key(&msg.session_id) {
            return;
        }
        self.sessions.remove(&msg.session_id);
        let logged_out_username = self.session_usernames.get(&msg.session_id).cloned();
        self.session_usernames.remove(&msg.session_id);
        debug!("Session {} disconnected (TCP close)", msg.session_id);
        if let Some(username) = logged_out_username {
            if let Some(account_ref) = &self.account_ref {
                let _ = account_ref.ask(crate::actors::account::LogoutRequest {
                    username,
                }).await;
            }
        }

        // 通知 WorldActor 清理玩家状态
        if let Some(world_ref) = &self.world_ref {
            let _ = world_ref.ask(crate::actors::world::PlayerDisconnected {
                session_id: msg.session_id,
            }).await;
        }
    }
}

impl Message<LoginResult> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: LoginResult,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if msg.success {
            // 记录 session 关联的用户名（用于 ChangePassword 等）
            self.session_usernames.insert(msg.session_id, msg.username.clone());

            // LoginSuccess: 角色列表（用 SharedRust 序列化，保证与客户端解析一致）
            let characters: Vec<mir2_shared::data::client_data::SelectInfo> = msg
                .characters
                .iter()
                .enumerate()
                .map(|(i, ch)| mir2_shared::data::client_data::SelectInfo {
                    index: i as i32,
                    name: ch.name.clone(),
                    level: ch.level,
                    class: mir2_shared::enums::MirClass::try_from(ch.class)
                        .unwrap_or(mir2_shared::enums::MirClass::Warrior),
                    gender: mir2_shared::enums::MirGender::try_from(ch.gender)
                        .unwrap_or(mir2_shared::enums::MirGender::Male),
                    last_access: chrono::DateTime::from_timestamp(ch.last_access, 0)
                        .unwrap_or_else(|| chrono::Utc::now()),
                })
                .collect();
            let mut body = Vec::new();
            if (mir2_shared::packets::server::login::LoginSuccess { characters })
                .write_body(&mut body)
                .is_err()
            {
                // 序列化失败：发空列表兜底
                body = Vec::new();
                body.extend_from_slice(&0i32.to_le_bytes());
            }
            let response_data = build_packet_bytes(ServerPacketIds::LoginSuccess as i16, &body);

            let gate_ref = ctx.actor_ref().clone();
            let _ = gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: response_data,
                }).await;
        } else if let Some(until) = msg.banned_until {
            // C#：封禁期登录 → S.LoginBanned（Reason + ExpiryDate，.NET DateTime ticks）
            let expiry_ticks = (until + 62135596800) * 10_000_000;
            let packet = mir2_shared::packets::server::login::LoginBanned {
                reason: "密码错误次数过多，账号已临时封禁".to_string(),
                expiry_date: expiry_ticks,
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let data = build_packet_bytes(ServerPacketIds::LoginBanned as i16, &body);
                let gate_ref = ctx.actor_ref().clone();
                let _ = gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data,
                    }).await;
            }
        } else {
            // Login failure
            let response_data = build_packet_bytes(ServerPacketIds::Login as i16, &[4u8]);
            let gate_ref = ctx.actor_ref().clone();
            let _ = gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: response_data,
                }).await;
        }
    }
}

impl Message<SetAccountRef> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetAccountRef,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.account_ref = Some(msg.account_ref);
        info!("GateActor linked to AccountActor");
    }
}

impl Message<SetWorldRef> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetWorldRef,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.world_ref = Some(msg.world_ref);
        info!("GateActor linked to WorldActor");
    }
}

impl Message<SetSocialRef> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetSocialRef,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.set_social_ref(msg.social_ref);
        info!("GateActor linked to SocialActor");
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 处理客户端版本：验证 payload 后回复 accepted
async fn handle_client_version(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    use std::io::Cursor;
    use byteorder::{LittleEndian, ReadBytesExt};

    // ClientVersion payload: [version_hash_length: i32 LE][version_hash: bytes]
    if payload.len() < 4 {
        warn!("Session {} ClientVersion payload too short", session_id);
        return;
    }

    let mut cursor = Cursor::new(payload);
    if let Ok(hash_len) = ReadBytesExt::read_i32::<LittleEndian>(&mut cursor) {
        if !(0..=256).contains(&hash_len) || payload.len() < 4 + hash_len as usize {
            warn!("Session {} ClientVersion invalid hash length: {}", session_id, hash_len);
            return;
        }
    }

    debug!("ClientVersion from session {}", session_id);
    let response = build_packet_bytes(ServerPacketIds::ClientVersion as i16, &[1u8]); // accepted
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: response,
        }).await;
}

/// 处理新账号注册
async fn handle_new_account(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    debug!("NewAccount request from session {}", session_id);
    // Phase 1: auto-register, respond success (result=8)
    let response = build_packet_bytes(ServerPacketIds::NewAccount as i16, &[8u8]);
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: response,
        }).await;
}

/// 解析登录包：account_id (DotNetString) + password (DotNetString)
fn parse_login_payload(payload: &[u8]) -> Option<(String, String)> {
    use std::io::Cursor;
    use mir2_shared::binary::read_dotnet_string;

    let mut cursor = Cursor::new(payload);
    match (read_dotnet_string(&mut cursor), read_dotnet_string(&mut cursor)) {
        (Ok(username), Ok(password)) => Some((username, password)),
        _ => None,
    }
}

/// 处理心跳：回复 KeepAlive
async fn handle_keep_alive(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    let response = build_packet_bytes(ServerPacketIds::KeepAlive as i16, &[]);
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: response,
        }).await;
}

/// 解析 DotNetString: [length: i32 LE][bytes...]
fn parse_dotnet_string(data: &[u8]) -> String {
    use std::io::Cursor;
    use mir2_shared::binary::read_dotnet_string;
    let mut cursor = Cursor::new(data);
    match read_dotnet_string(&mut cursor) {
        Ok(s) => s,
        Err(e) => {
            warn!("parse_dotnet_string: malformed input ({e:?}), data len={}", data.len());
            String::new()
        }
    }
}

/// 解析聊天包：DotNetString message + i32 linked_items_count
fn parse_chat_payload(payload: &[u8]) -> Option<String> {
    use std::io::Cursor;
    use mir2_shared::binary::read_dotnet_string;

    let mut cursor = Cursor::new(payload);
    read_dotnet_string(&mut cursor).ok()
}

/// 解析 CallNPC 包：[object_id: u32 LE][key: DotNetString]
fn parse_call_npc_payload(payload: &[u8]) -> Option<(u32, String)> {
    use std::io::Cursor;
    use byteorder::{LittleEndian, ReadBytesExt};
    use mir2_shared::binary::read_dotnet_string;

    if payload.len() < 4 {
        return None;
    }
    let mut cursor = Cursor::new(payload);
    let object_id = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).ok()?;
    let key = read_dotnet_string(&mut cursor).ok()?;
    Some((object_id, key))
}

// ============================================================================
// 物品操作 forward helpers（转发到 WorldActor）
// ============================================================================

fn forward_move_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let from = i32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[5..9].try_into().unwrap_or([0; 4]));
    let _ = world_ref.tell(crate::actors::world::MoveItemRequest {
        session_id, grid, from, to,
    }).try_send();
}

fn forward_use_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let _ = world_ref.tell(crate::actors::world::UseItemRequest {
        session_id, unique_id: uid,
    }).try_send();
}

fn forward_equip_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let slot = payload[9] as i32;
    let _ = world_ref.tell(crate::actors::world::EquipItemRequest {
        session_id, grid, unique_id: uid, slot,
    }).try_send();
}

fn forward_delete_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 11 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let hero = payload[10] != 0;
    let _ = world_ref.tell(crate::actors::world::DeleteItemRequest {
        session_id, unique_id: uid, count, hero,
    }).try_send();
}

fn forward_remove_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let _ = world_ref.tell(crate::actors::world::RemoveItemRequest {
        session_id, grid, unique_id: uid,
    }).try_send();
}

fn forward_drop_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 11 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let _hero_inv = payload[10] != 0;
    let _ = world_ref.tell(crate::actors::world::DropItemRequest {
        session_id, unique_id: uid, count,
    }).try_send();
}

fn forward_merge_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 18 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid_from = payload[0];
    let grid_to = payload[1];
    let from_uid = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let to_uid = u64::from_le_bytes(payload[10..18].try_into().unwrap_or([0; 8]));
    let _ = world_ref.tell(crate::actors::world::MergeItemRequest {
        session_id, grid_from, grid_to, from_uid, to_uid,
    }).try_send();
}

fn forward_split_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 13 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let _ = world_ref.tell(crate::actors::world::SplitItemRequest {
        session_id, grid, unique_id: uid, count,
    }).try_send();
}

/// BuyItem: [item_index: u64][count: u16][panel_type: u8]（C# 协议，无 npc_id）
fn forward_buy_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 11 { warn!("BuyItem payload too short: {}", payload.len()); return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let item_index = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let _panel_type = payload[10];
    debug!("BuyItem session={} item_index={} count={}", session_id, item_index, count);
    let _ = world_ref.tell(crate::actors::world::BuyItemRequest {
        session_id, item_index, count: count as u32,
    }).try_send();
}

/// SellItem: [uid: u64][count: u16]（C# 协议）
fn forward_sell_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let _ = world_ref.tell(crate::actors::world::SellItemRequest {
        session_id, unique_id: uid, count: count as u32,
    }).try_send();
}

fn forward_repair_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let _ = world_ref.tell(crate::actors::world::RepairItemRequest { session_id, unique_id: uid }).try_send();
}

/// RangeAttack: [dir: u8][x: i32][y: i32][target_id: u32][tx: i32][ty: i32]
fn forward_range_attack(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 21 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let dir = payload[0];
    let target_id = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let target_x = i32::from_le_bytes(payload[13..17].try_into().unwrap_or([0; 4]));
    let target_y = i32::from_le_bytes(payload[17..21].try_into().unwrap_or([0; 4]));
    debug!("RangeAttack: session={} dir={} target={} pos=({}, {})", session_id, dir, target_id, target_x, target_y);
    let _ = world_ref.tell(crate::actors::world::RangeAttackRequest { session_id, direction: dir, target_id, target_x, target_y }).try_send();
}

/// Magic: [spell: u8][dir: u8][target_id: u32][x: i32][y: i32]
fn forward_magic(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 12 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let spell = payload[0];
    let dir = payload[1];
    let target_id = u32::from_le_bytes(payload[2..6].try_into().unwrap_or([0; 4]));
    let target_x = i32::from_le_bytes(payload[6..10].try_into().unwrap_or([0; 4]));
    let target_y = i32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    debug!("Magic: session={} spell={} dir={} target={} pos=({}, {})", session_id, spell, dir, target_id, target_x, target_y);
    let _ = world_ref.tell(crate::actors::world::MagicRequest { session_id, direction: dir, spell, target_id, target_x, target_y }).try_send();
}

/// Harvest: [dir: u8] — 采集/挖矿请求
fn forward_harvest(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let dir = payload[0];
    debug!("Harvest: session={} dir={}", session_id, dir);
    let _ = world_ref.tell(crate::actors::world::HarvestRequest { session_id, direction: dir }).try_send();
}

// ============================================================================
// NPC 商店 / 合成 handlers
// ============================================================================

/// CraftItem: [recipe_id: u32][materials_count: u32]
fn forward_craft_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let recipe_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("CraftItem: session={} recipe={}", session_id, recipe_id);
    let _ = world_ref.tell(crate::actors::world::CraftItemRequest { session_id, recipe_id }).try_send();
}

/// BuyItemBack (回购): [unique_id: u64][count: u16]（C# ClientPackets.BuyItemBack wire）
fn forward_buy_item_back(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    debug!("BuyItemBack: session={} uid={} count={}", session_id, unique_id, count);
    let _ = world_ref.tell(crate::actors::world::BuyItemBackRequest {
        session_id,
        unique_id,
        count: count as u32,
    }).try_send();
}

// ============================================================================
// 仓库 handlers
// ============================================================================

/// StoreItem (存入仓库): [from: i32][to: i32]（C# 协议，from=背包格 to=仓库格）
fn handle_store_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 {
        debug!("StoreItem: session={} payload too short", session_id);
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("StoreItem: session={} from={} to={}", session_id, from, to);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::StoreItemRequest {
        session_id,
        from,
        to,
    }).try_send();
}

/// TakeBackItem (从仓库取出): [from: i32][to: i32]（C# 协议，from=仓库格 to=背包格）
fn handle_take_back_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 {
        debug!("TakeBackItem: session={} payload too short", session_id);
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("TakeBackItem: session={} from={} to={}", session_id, from, to);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::TakeBackItemRequest {
        session_id,
        from,
        to,
    }).try_send();
}

// ============================================================================
// 其他常用 handlers
// ============================================================================

/// DropGold (丢弃/设置金币): [amount: u32]
fn handle_drop_gold(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 {
        debug!("DropGold: session={} payload too short", session_id);
        return;
    }
    let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("DropGold: session={} amount={}", session_id, amount);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::DropGoldRequest {
        session_id,
        amount,
    }).try_send();
}

/// Inspect (查看玩家): [target_id: u32]
fn handle_inspect(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 {
        debug!("Inspect: session={} payload too short", session_id);
        return;
    }
    let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("Inspect: session={} target={}", session_id, target_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::InspectPlayerRequest { session_id, target_id }).try_send();
}

/// ChangeAMode (切换攻击模式): [mode: u8]
fn forward_change_amode(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let mode = payload[0];
    debug!("ChangeAMode: session={} mode={}", session_id, mode);
    let mode = mir2_shared::enums::AttackMode::try_from(mode).unwrap_or(mir2_shared::enums::AttackMode::Peace);
    let _ = world_ref.tell(crate::actors::world::ChangeAModeRequest { session_id, mode }).try_send();
}

/// ChangePMode (切换宠物模式): [mode: u8]
fn forward_change_pmode(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let mode = payload[0];
    debug!("ChangePMode: session={} mode={}", session_id, mode);
    let mode = mir2_shared::enums::PetMode::try_from(mode).unwrap_or(mir2_shared::enums::PetMode::Both);
    let _ = world_ref.tell(crate::actors::world::ChangePModeRequest { session_id, mode }).try_send();
}

/// MagicKey (设置快捷键): [spell: u8][key: u8][old_key: u8]
fn forward_magic_key(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 3 { return; }
    let spell = payload[0] as i32;
    let key = payload[1];
    let old_key = payload[2];
    debug!("MagicKey: session={} spell={} key={} old_key={}", session_id, spell, key, old_key);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::SetSpellKeyRequest { session_id, spell, key, old_key }).try_send();
}

/// RemoveSlotItem (移除插槽物品): [Grid:u8][GridTo:u8][UniqueID:u64][To:i32][FromUniqueID:u64]
fn forward_remove_slot_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 22 { return; }
    let grid = payload[0];
    let grid_to = payload[1];
    let unique_id = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let to = i32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    let from_unique_id = u64::from_le_bytes(payload[14..22].try_into().unwrap_or([0; 8]));
    debug!("RemoveSlotItem: session={} grid={} grid_to={} uid={} to={} from_uid={}",
           session_id, grid, grid_to, unique_id, to, from_unique_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RemoveSlotItemRequest {
        session_id, grid, grid_to, unique_id, to, from_unique_id,
    }).try_send();
}

/// TeleportToNPC: [npc_id: u32]
fn forward_teleport_to_npc(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("TeleportToNPC: session={} npc={}", session_id, npc_id);
    let _ = world_ref.tell(crate::actors::world::TeleportToNPCRequest { session_id, npc_id }).try_send();
}

fn forward_town_revive(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
) {
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::TownReviveRequest { session_id }).try_send();
}

// ============================================================================
// 死亡恢复 / 技能切换
// ============================================================================

/// SpellToggle: [spell: u8][can_use: i8] (can_use: -1=hero, 0=off, 1=on)
fn forward_spell_toggle(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let spell = payload[0] as i32;
    let can_use = payload[1] as i8;
    debug!("SpellToggle: session={} spell={} can_use={}", session_id, spell, can_use);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::SpellToggleRequest { session_id, spell, can_use }).try_send();
}

// ============================================================================
// 账号管理
// ============================================================================

/// ChangePassword: [old_password: DotNetString][new_password: DotNetString]
fn forward_change_password(
    account_ref: &Option<ActorRef<crate::actors::account::AccountActor>>,
    session_usernames: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let old_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + old_len + 2 { return; }
    let new_len = u16::from_le_bytes(payload[2 + old_len..4 + old_len].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 4 + old_len + new_len { return; }
    let old_password = String::from_utf8_lossy(&payload[2..2 + old_len]).to_string();
    let new_password = String::from_utf8_lossy(&payload[4 + old_len..4 + old_len + new_len]).to_string();

    if let Some(username) = session_usernames.get(&session_id) {
        if let Some(account_ref) = account_ref {
            let _ = account_ref.tell(crate::actors::account::AccountChangePassword {
                session_id,
                username: username.clone(),
                old_password,
                new_password,
            }).try_send();
        } else {
            warn!("ChangePassword: account_ref not available for session={}", session_id);
        }
    } else {
        warn!("ChangePassword: no username mapping for session={}", session_id);
    }
}

// ============================================================================
// PR #1169: Warehouse password forwards
// ============================================================================

/// UnlockStorage: [password: DotNetString]
async fn forward_unlock_storage(
    account_ref: &Option<ActorRef<crate::actors::account::AccountActor>>,
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_usernames: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    let password = parse_dotnet_string(payload);
    if let Some(username) = session_usernames.get(&session_id) {
        if let Some(account_ref) = account_ref {
            debug!(
                "UnlockStorage: session={} user={} pwd_len={}",
                session_id,
                username,
                password.len()
            );
            // #200：校验成功 → 通知 WorldActor 下发仓库内容（C# Player.SendStorage）
            let ok = account_ref
                .ask(crate::actors::account::ValidateStoragePasswordRequest {
                    session_id,
                    username: username.clone(),
                    raw_password: password,
                })
                .await
                .unwrap_or(false);
            if ok {
                if let Some(world_ref) = world_ref {
                    let _ = world_ref
                        .tell(crate::actors::world::StorageUnlockedRequest { session_id })
                        .try_send();
                }
            }
        } else {
            warn!(
                "UnlockStorage: account_ref not available for session={}",
                session_id
            );
        }
    } else {
        warn!(
            "UnlockStorage: no username mapping for session={}",
            session_id
        );
    }
}

/// SetStoragePassword: [current: DotNetString][new: DotNetString]
fn forward_set_storage_password(
    account_ref: &Option<ActorRef<crate::actors::account::AccountActor>>,
    session_usernames: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    // Parse first DotNetString (current), then second (new)
    let (current, rest) = {
        use std::io::Cursor;
        use mir2_shared::binary::read_dotnet_string;
        let mut c = Cursor::new(payload);
        let s = read_dotnet_string(&mut c).unwrap_or_default();
        let pos = c.position() as usize;
        (s, &payload[pos..])
    };
    let new = parse_dotnet_string(rest);
    if let Some(username) = session_usernames.get(&session_id) {
        if let Some(account_ref) = account_ref {
            debug!("SetStoragePassword: session={} user={} new_len={}", session_id, username, new.len());
            let _ = account_ref.tell(crate::actors::account::SetStoragePasswordRequest {
                session_id,
                username: username.clone(),
                current_raw: current,
                new_raw: new,
            }).try_send();
        } else {
            warn!("SetStoragePassword: account_ref not available for session={}", session_id);
        }
    } else {
        warn!("SetStoragePassword: no username mapping for session={}", session_id);
    }
}

/// RemoveStoragePassword: [current: DotNetString]
fn forward_remove_storage_password(
    account_ref: &Option<ActorRef<crate::actors::account::AccountActor>>,
    session_usernames: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    let current = parse_dotnet_string(payload);
    if let Some(username) = session_usernames.get(&session_id) {
        if let Some(account_ref) = account_ref {
            debug!("RemoveStoragePassword: session={} user={}", session_id, username);
            let _ = account_ref.tell(crate::actors::account::ClearStoragePasswordRequest {
                session_id,
                username: username.clone(),
                current_raw: current,
            }).try_send();
        } else {
            warn!("RemoveStoragePassword: account_ref not available for session={}", session_id);
        }
    } else {
        warn!("RemoveStoragePassword: no username mapping for session={}", session_id);
    }
}

/// NewCharacter: [name: DotNetString(7bit)][gender: u8][class: u8]（对齐 C# ClientPackets.NewCharacter）
async fn forward_new_character(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_usernames: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    // 用 SharedRust 的 DotNetString 解析（7-bit 长度前缀），与客户端一致
    let mut cur = std::io::Cursor::new(payload);
    let name = match mir2_shared::binary::read_dotnet_string(&mut cur) {
        Ok(n) => n,
        Err(e) => {
            warn!("NewCharacter name parse failed: {}", e);
            return;
        }
    };
    let gender = match cur.get_ref().get(cur.position() as usize).copied() {
        Some(g) => g,
        None => return,
    };
    let class = match cur.get_ref().get(cur.position() as usize + 1).copied() {
        Some(c) => c,
        None => return,
    };
    // hair 由服务端随机生成（C# HumanObject.NewCharacter: Hair = Random.Next(0, 9)）
    let hair = 0;

    // Phase 1.3: 角色名输入验证
    if !crate::util::validation::validate_character_name(&name) {
        warn!("NewCharacter rejected: invalid name '{}' from session {}", name, session_id);
        return;
    }
    debug!("NewCharacter: session={} name={} class={} gender={} hair={}", session_id, name, class, gender, hair);
    let account_username = session_usernames.get(&session_id).cloned().unwrap_or_else(|| name.clone());
    let req = crate::actors::world::NewCharacterRequest { session_id, name, class, gender, hair, account_username };
    match world_ref.ask(req).await {
        Ok(()) => info!("NewCharacter ask completed: session={}", session_id),
        Err(e) => warn!("NewCharacter ask failed: session={} err={}", session_id, e),
    }
}

/// DeleteCharacter: [character_index: i32]
fn forward_delete_character(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_usernames: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("DeleteCharacter: session={} index={}", session_id, character_index);
    let account_username = session_usernames.get(&session_id).cloned().unwrap_or_default();
    let _ = world_ref.tell(crate::actors::world::DeleteCharacterRequest { session_id, character_index, account_username }).try_send();
}

// ============================================================================
// 社交/组队
// ============================================================================

/// SwitchGroup: [allow_group: bool] (1 byte)
fn forward_switch_group(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let allow_group = payload[0] != 0;
    debug!("SwitchGroup: session={} allow={}", session_id, allow_group);
    let _ = social_ref.tell(crate::actors::social::SwitchGroupRequest {
        session_id,
        allow_group,
    }).try_send();
}

/// AddMember: [name: string] (DotNet string format)
fn forward_add_member(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let social_ref = match social_ref { Some(s) => s, None => return };
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    debug!("AddMember: session={} name={}", session_id, name);
    let _ = social_ref.tell(crate::actors::social::GroupInviteRequest {
        session_id,
        target_name: name,
    }).try_send();
}

/// GroupInvite: [accept_invite: bool] (1 byte) - 邀请回复
fn forward_group_invite(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let accept = payload[0] != 0;
    debug!("GroupInvite reply: session={} accept={}", session_id, accept);
    let _ = social_ref.tell(crate::actors::social::GroupInviteReply {
        session_id,
        inviter_id: 0,
        accept,
    }).try_send();
}

/// DellMember: [name: string] (DotNet string format)
fn forward_dell_member(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let social_ref = match social_ref { Some(s) => s, None => return };
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    debug!("DellMember: session={} name={}", session_id, name);
    let _ = social_ref.tell(crate::actors::social::DellMemberRequest {
        session_id,
        member_name: name,
    }).try_send();
}

// ============================================================================
// Hero/宠物
// ============================================================================

/// NewHero: C# C.NewHero = Name(string) + Gender(u8) + Class(u8)
fn forward_new_hero(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let Ok(packet) = mir2_shared::packets::client::hero::NewHero::read_body(&mut std::io::Cursor::new(payload)) else {
        warn!("NewHero: 解析失败 session={} len={}", session_id, payload.len());
        return;
    };
    debug!("NewHero: session={} name={} gender={:?} class={:?}", session_id, packet.name, packet.gender, packet.class);
    let _ = world_ref.tell(crate::actors::world::NewHeroRequest {
        session_id,
        name: packet.name,
        gender: packet.gender,
        class: packet.class,
    }).try_send();
}

/// SetHeroBehaviour: [behaviour: u8]
fn forward_set_hero_behaviour(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let behaviour = payload[0];
    debug!("SetHeroBehaviour: session={} behaviour={}", session_id, behaviour);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::SetHeroBehaviourRequest { session_id, behaviour }).try_send();
}

/// SetAutoPotValue: [stat: u8][value: u32]
fn forward_set_autopot_value(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 5 { return; }
    let stat = payload[0];
    let value = u32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    debug!("SetAutoPotValue: session={} stat={} value={}", session_id, stat, value);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::SetAutoPotValueRequest { session_id, stat, value }).try_send();
}

/// SetAutoPotItem: [grid: u8][item_index: i32]
fn forward_set_autopot_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 5 { return; }
    let grid = payload[0];
    let item_index = i32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    debug!("SetAutoPotItem: session={} grid={} item_index={}", session_id, grid, item_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::SetAutoPotItemRequest { session_id, grid, item_index }).try_send();
}

/// ChangeHero: [hero_index: u8]
fn handle_change_hero(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let hero_index = payload[0];
    debug!("ChangeHero: session={} index={}", session_id, hero_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ChangeHeroRequest { session_id, hero_index }).try_send();
}

/// TakeBackHeroItem: C# [from i32][to i32]（英雄格 → 主背包格，#203）
fn handle_take_back_hero_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("TakeBackHeroItem: session={} from={} to={}", session_id, from, to);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::TakeBackHeroItemRequest { session_id, from, to }).try_send();
}

/// TransferHeroItem: C# [from i32][to i32]（主背包格 → 英雄格，#203）
fn handle_transfer_hero_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("TransferHeroItem: session={} from={} to={}", session_id, from, to);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::TransferHeroItemRequest { session_id, from, to }).try_send();
}

// ============================================================================
// 交易系统
// ============================================================================

// ============================================================================
// 交易系统
// ============================================================================

/// ChangeTrade: 添加/移除交易物品（客户端触发）
fn forward_change_trade(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let is_add = payload[0] != 0;
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let grid = if payload.len() >= 10 { payload[9] } else { 0 };
    let count = if payload.len() >= 12 { u16::from_le_bytes(payload[10..12].try_into().unwrap_or([0; 2])) } else { 1 };

    if is_add {
        let _ = social_ref.tell(crate::actors::social::TradeAddItem {
            session_id, unique_id: uid, grid, count,
        }).try_send();
    } else {
        let _ = social_ref.tell(crate::actors::social::TradeRemoveItem {
            session_id, unique_id: uid,
        }).try_send();
    }
}

/// TradeRequest: 发起交易
fn forward_trade_request(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    if social_ref.is_none() { return; }
    // 注意：必须用 tell（ask 的 future 被丢弃时消息不会发出）
    let _ = social_ref.as_ref().unwrap().tell(crate::actors::social::TradeStartRequest {
        session_id,
    }).try_send();
}

/// TradeReply: [accept: bool]
fn forward_trade_reply(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let accept = payload[0] != 0;
    let _ = social_ref.tell(crate::actors::social::TradeStartReply {
        session_id, accept,
    }).try_send();
}

/// TradeConfirm: [locked: bool]
fn forward_trade_confirm(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let locked = payload[0] != 0;
    let _ = social_ref.tell(crate::actors::social::TradeConfirmLock {
        session_id, locked,
    }).try_send();
}

/// TradeCancel: 取消交易
fn forward_trade_cancel(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::TradeCancel {
        session_id,
    }).try_send();
}

/// TradeGold: [amount: u32]
fn forward_trade_gold(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let _ = social_ref.tell(crate::actors::social::TradeAddGold {
        session_id, amount,
    }).try_send();
}

// ============================================================================
// 好友系统
// ============================================================================

// ============================================================================
// 好友系统
// ============================================================================

/// AddFriend: [name: DotNetString][blocked: bool]
fn forward_add_friend(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let social_ref = match social_ref { Some(s) => s, None => return };
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串，随后 1 字节 blocked
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    let mut blocked_buf = [0u8; 1];
    let blocked = std::io::Read::read_exact(&mut cur, &mut blocked_buf).is_ok() && blocked_buf[0] != 0;
    debug!("AddFriend: session={} name={} blocked={}", session_id, name, blocked);
    let _ = social_ref.tell(crate::actors::social::AddFriendRequest {
        session_id, friend_name: name, blocked,
    }).try_send();
}

/// RemoveFriend: [character_index: i32]
fn forward_remove_friend(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RemoveFriend: session={} char_idx={}", session_id, character_index);
    let _ = social_ref.tell(crate::actors::social::RemoveFriendRequest {
        session_id, friend_object_id: character_index as u32,
    }).try_send();
}

/// RefreshFriends: no payload
fn forward_refresh_friends(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
) {
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::RefreshFriendsRequest {
        session_id,
    }).try_send();
}

/// AddMemo: [character_index: i32][memo: DotNetString]
fn forward_add_memo(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let social_ref = match social_ref { Some(s) => s, None => return };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let memo = if payload.len() > 4 {
        String::from_utf8_lossy(&payload[4..]).to_string()
    } else {
        String::new()
    };
    debug!("AddMemo: session={} char_idx={}", session_id, character_index);
    let _ = social_ref.tell(crate::actors::social::AddMemoRequest {
        session_id, friend_object_id: character_index as u32, memo,
    }).try_send();
}

// ============================================================================
// 邮件系统
// ============================================================================

/// SendMail: [name: DotNetString][message: DotNetString][gold: u32][items: 5*u64][stamped: bool]（C#/SharedRust wire）
fn handle_send_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let mut cur = std::io::Cursor::new(payload);

    // 收件人 + 正文（DotNet 7-bit 字符串；subject 由正文首行派生，C# 语义）
    let Ok(receiver_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    let Ok(message) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    let mut gold_buf = [0u8; 4];
    if std::io::Read::read_exact(&mut cur, &mut gold_buf).is_err() { return; }
    let gold = u32::from_le_bytes(gold_buf);
    let mut item_uids = Vec::new();
    for _ in 0..5 {
        let mut uid_buf = [0u8; 8];
        if std::io::Read::read_exact(&mut cur, &mut uid_buf).is_err() { return; }
        let uid = u64::from_le_bytes(uid_buf);
        if uid != 0 {
            item_uids.push(uid);
        }
    }
    let mut stamped_buf = [0u8; 1];
    let _stamped = if std::io::Read::read_exact(&mut cur, &mut stamped_buf).is_ok() { stamped_buf[0] } else { 0 };

    let subject = message.lines().next().unwrap_or("").to_string();
    debug!("SendMail: session={} to={} subject={} gold={} items={}", session_id, receiver_name, subject, gold, item_uids.len());
    let _ = world_ref.tell(crate::actors::world::SendMailRequest {
        session_id,
        receiver_name,
        subject,
        body: message,
        gold,
        item_uids,
    }).try_send();
}

/// ReadMail: [mail_id: u64]
fn handle_read_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0;8]));
    debug!("ReadMail: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ReadMailRequest { session_id, mail_id }).try_send();
}

/// CollectParcel: [mail_id: u64]
fn handle_collect_parcel(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0;8]));
    debug!("CollectParcel: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::CollectParcelRequest { session_id, mail_id }).try_send();
}

/// DeleteMail: [mail_id: u64]
fn handle_delete_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0;8]));
    debug!("DeleteMail: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::DeleteMailRequest { session_id, mail_id }).try_send();
}

// ============================================================================
// 行会系统
// ============================================================================

/// GuildInvite: [accept: bool] - 行会邀请回复
fn handle_guild_invite(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("GuildInvite: session={} accept={}", session_id, accept);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::GuildInviteReply { session_id, accept }).try_send();
}

/// RequestGuildInfo: [info_type: u8]
fn handle_request_guild_info(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    let info_type = payload.first().copied().unwrap_or(0);
    debug!("RequestGuildInfo: session={} type={}", session_id, info_type);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::RequestGuildInfo { session_id, info_type }).try_send();
}

/// EditGuildMember: [change_type: u8][rank_index: u8][name: DotNetString][rank_name: DotNetString]
fn handle_edit_guild_member(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 2 { return; }
    let change_type = payload[0];
    // C#/SharedRust：[change_type u8][rank_index u8][name DotNet][rank_name DotNet]
    let mut cur = std::io::Cursor::new(&payload[2..]);
    let Ok(member_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    debug!("EditGuildMember: session={} type={} name={}", session_id, change_type, member_name);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::EditGuildMemberRequest { session_id, change_type, member_name }).try_send();
}

/// EditGuildNotice: [count: i32][line1: DotNetString][line2: DotNetString]...
fn handle_edit_guild_notice(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    // C#/SharedRust：[count i32][lines DotNet...]
    let count = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4])) as usize;
    let mut notice_lines = Vec::new();
    let mut cur = std::io::Cursor::new(&payload[4..]);
    for _ in 0..count {
        match mir2_shared::binary::read_dotnet_string(&mut cur) {
            Ok(line) => notice_lines.push(line),
            Err(_) => break,
        }
    }
    debug!("EditGuildNotice: session={} lines={}", session_id, notice_lines.len());
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::EditGuildNoticeRequest { session_id, notice: notice_lines }).try_send();
}

/// GuildNameReturn: [name: DotNetString]
fn handle_guild_name_return(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    debug!("GuildNameReturn: session={} name={}", session_id, name);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::CreateGuildRequest { session_id, guild_name: name }).try_send();
}

/// GuildStorageGoldChange: [change_type: u8][amount: u32]
fn handle_guild_storage_gold(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 5 { return; }
    let change_type = payload[0];
    let amount = u32::from_le_bytes(payload[1..5].try_into().unwrap_or([0;4]));
    debug!("GuildStorageGoldChange: session={} type={} amount={}", session_id, change_type, amount);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::GuildStorageGoldChangeRequest { session_id, change_type, amount }).try_send();
}

/// GuildStorageItemChange: [change_type: u8][grid: u8][unique_id: u64][count: u32]
fn handle_guild_storage_item(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 12 { return; }
    let change_type = payload[0];
    let grid = payload[1];
    let uid = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    debug!("GuildStorageItemChange: session={} type={} grid={} uid={} count={}", session_id, change_type, grid, uid, count);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::GuildStorageItemChangeRequest { session_id, change_type, grid, unique_id: uid, count }).try_send();
}

// ============================================================================
// 婚姻系统
// ============================================================================

/// MarriageRequest: [target_name: DotNet 7-bit string]（C# BinaryWriter.Write(string)）
fn handle_marriage_request(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(target_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    debug!("MarriageRequest: session={} to={}", session_id, target_name);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::MarriageRequest { session_id, target_name }).try_send();
}

/// MarriageReply: [accept: bool]
fn handle_marriage_reply(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("MarriageReply: session={} accept={}", session_id, accept);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::MarriageReply { session_id, accept }).try_send();
}

/// ChangeMarriage: no payload or minimal
fn handle_change_marriage(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ChangeMarriage: session={}", session_id);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::SocialChangeMarriage { session_id }).try_send();
}

/// DivorceRequest: [partner_name: DotNet 7-bit string]
fn handle_divorce_request(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(partner_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    debug!("DivorceRequest: session={} partner={}", session_id, partner_name);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::SocialDivorceRequest { session_id, partner_name }).try_send();
}

/// DivorceReply: [accept: bool]
fn handle_divorce_reply(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("DivorceReply: session={} accept={}", session_id, accept);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::SocialDivorceReply { session_id, accept }).try_send();
}

/// AddMentor: [mentor_name: DotNet 7-bit string]（C# BinaryWriter.Write(string)）
fn handle_add_mentor(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(mentor_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else { return };
    debug!("AddMentor: session={} mentor={}", session_id, mentor_name);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::SocialAddMentor { session_id, mentor_name }).try_send();
}

/// MentorReply: [accept: bool]
fn handle_mentor_reply(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("MentorReply: session={} accept={}", session_id, accept);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::SocialMentorReply { session_id, accept }).try_send();
}

/// AllowMentor: [allow: bool]
fn handle_allow_mentor(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let allow = payload[0] != 0;
    debug!("AllowMentor: session={} allow={}", session_id, allow);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::SocialAllowMentor { session_id, allow }).try_send();
}

/// CancelMentor
fn handle_cancel_mentor(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("CancelMentor: session={}", session_id);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::SocialCancelMentor { session_id }).try_send();
}

// ============================================================================
// 宠物系统
// ============================================================================

/// UpdateIntelligentCreature: [creature_type: u8][pickup_mode: u8][custom_name: DotNetString?]
fn handle_update_intelligent_creature(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 2 { return; }
    let creature_type = payload[0];
    let pickup_mode = payload[1];
    debug!("UpdateIntelligentCreature: session={} type={} mode={}", session_id, creature_type, pickup_mode);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::UpdateIntelligentCreature { session_id, creature_type, pickup_mode }).try_send();
}

/// IntelligentCreaturePickup: [x: i32][y: i32]
fn handle_intelligent_creature_pickup(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let x = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4]));
    let y = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0;4]));
    debug!("IntelligentCreaturePickup: session={} x={} y={}", session_id, x, y);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::IntelligentCreaturePickup { session_id, x, y }).try_send();
}

/// RequestIntelligentCreatureUpdates: [request_updates: bool]
fn handle_request_intelligent_creature_updates(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let request_updates = payload[0] != 0;
    debug!("RequestIntelligentCreatureUpdates: session={} updates={}", session_id, request_updates);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RequestIntelligentCreatureUpdates { session_id, request_updates }).try_send();
}

// ============================================================================
// 任务系统
// ============================================================================

/// AcceptQuest: [npc_index: i32][quest_index: i32]
fn handle_accept_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let npc_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4]));
    let quest_index = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0;4]));
    debug!("AcceptQuest: session={} npc={} quest={}", session_id, npc_index, quest_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::AcceptQuestRequest { session_id, npc_index, quest_index }).try_send();
}

/// FinishQuest: [quest_index: i32][selected_item_index: i32]
fn handle_finish_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let quest_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4]));
    let selected_item_index = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0;4]));
    debug!("FinishQuest: session={} quest={}", session_id, quest_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::FinishQuestRequest { session_id, quest_index, selected_item_index }).try_send();
}

/// AbandonQuest: [quest_index: i32]
fn handle_abandon_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let quest_index = i32::from_le_bytes(payload[..4].try_into().unwrap_or([0;4]));
    debug!("AbandonQuest: session={} quest={}", session_id, quest_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::AbandonQuestRequest { session_id, quest_index }).try_send();
}

// ============================================================================
// 精炼系统
// ============================================================================

/// DepositRefineItem: [unique_id: u64]
fn handle_deposit_refine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("DepositRefineItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::DepositRefineItemRequest {
        session_id, unique_id: uid,
    }).try_send();
}

/// RetrieveRefineItem: [unique_id: u64]
fn handle_retrieve_refine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("RetrieveRefineItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RetrieveRefineItemRequest {
        session_id, unique_id: uid,
    }).try_send();
}

/// RefineCancel: []
fn handle_refine_cancel(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("RefineCancel: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RefineCancelRequest { session_id }).try_send();
}

/// RefineItem: [item_id: u32][materials: u32]
fn handle_refine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let item_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let materials = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("RefineItem: session={} item={} materials={}", session_id, item_id, materials);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RefineItemRequest {
        session_id, item_id, materials,
    }).try_send();
}

/// CheckRefine: [unique_id: u64]
fn handle_check_refine(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("CheckRefine: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::CheckRefineRequest {
        session_id, unique_id: uid,
    }).try_send();
}

// ============================================================================
// 传送/地图
// ============================================================================

/// RequestMapInfo: [map_index: i32]
fn forward_request_map_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let map_id = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestMapInfo: session={} map={}", session_id, map_id);
    let _ = world_ref.tell(crate::actors::world::RequestMapInfoRequest { session_id, map_id }).try_send();
}

/// PR #1126: Client requests detailed monster info (for tooltip).
/// Wire format: [monster_index: i32 LE]
fn forward_request_monster_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let monster_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestMonsterInfo: session={} idx={}", session_id, monster_index);
    let _ = world_ref.tell(crate::actors::world::RequestMonsterInfoRequest {
        session_id, monster_index,
    }).try_send();
}

/// PR #1126: Client requests detailed NPC info (for tooltip).
/// Wire format: [npc_index: i32 LE]
fn forward_request_npc_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let npc_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestNPCInfo: session={} idx={}", session_id, npc_index);
    let _ = world_ref.tell(crate::actors::world::RequestNPCInfoRequest {
        session_id, npc_index,
    }).try_send();
}

/// PR #1126: Client requests detailed item info (for tooltip).
/// Wire format: [item_index: i32 LE]
/// (Returns nothing for now — ItemInfo stream will be wired in a later PR.)
fn forward_request_item_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let item_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestItemInfo: session={} idx={}", session_id, item_index);
    let _ = world_ref.tell(crate::actors::world::RequestItemInfoRequest {
        session_id, item_index,
    }).try_send();
}

/// SearchMap: [keyword: DotNetString]
fn forward_search_map(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let name_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + name_len { return; }
    let keyword = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    debug!("SearchMap: session={} keyword={}", session_id, keyword);
    let _ = world_ref.tell(crate::actors::world::SearchMapRequest { session_id, keyword }).try_send();
}

/// Observe: [target_id: u32]
fn forward_observe(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("Observe: session={} target={}", session_id, target_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ObservePlayerRequest {
        session_id, target_id,
    }).try_send();
}

// ============================================================================
// 其他
// ============================================================================

/// ReplaceWedRing: [unique_id: u64] — 更换结婚戒指
fn handle_replace_wed_ring(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("ReplaceWedRing: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ReplaceWedRingRequest { session_id, unique_id: uid }).try_send();
}

/// RequestUserName: [target_id: u32]
fn handle_request_user_name(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestUserName: session={} target={}", session_id, target_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RequestUserNameMsg {
        session_id, object_id: target_id,
    }).try_send();
}

/// RequestChatItem: [unique_id: u64]
fn handle_request_chat_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("RequestChatItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RequestChatItemMsg {
        session_id, unique_id: uid,
    }).try_send();
}

// ============================================================================
// 剩余 opcode stub handlers（Phase 15：覆盖所有未处理的 opcode）
// ============================================================================

/// EquipSlotItem: [grid:u8][unique_id:u64][to_slot:i32][grid_to:u8] — 快捷装备栏装备
fn handle_equip_slot_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 14 { return; }
    let grid = payload[0];
    let unique_id = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let to_slot = i32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let grid_to = payload[13];
    debug!("EquipSlotItem: session={} grid={} uid={} to_slot={} grid_to={}", session_id, grid, unique_id, to_slot, grid_to);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::EquipSlotItemRequest { session_id, grid, unique_id, to_slot, grid_to }).try_send();
}

/// ConsignItem: [item_index: u32][price: u32][duration: u32] — 寄售
fn forward_consign_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 12 { return; }
    let unique_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let price = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("ConsignItem: session={} uid={} price={}", session_id, unique_id, price);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ConsignItemRequest { session_id, unique_id: unique_id as u64, price: price as u64 }).try_send();
}

/// MarketSearch: [keyword: DotNetString]
fn forward_market_search(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let item_index = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("MarketSearch: session={} item={}", session_id, item_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::MarketSearchRequest { session_id, item_index }).try_send();
}

/// MarketRefresh: []
fn forward_market_refresh(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("MarketRefresh: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::MarketRefreshRequest { session_id }).try_send();
}

/// MarketPage: [page: u32]
fn forward_market_page(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let page = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("MarketPage: session={} page={}", session_id, page);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::MarketPageRequest { session_id, page }).try_send();
}

/// MarketBuy: [listing_id: u32]
fn forward_market_buy(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let listing_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("MarketBuy: session={} listing={}", session_id, listing_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::MarketBuyRequest { session_id, listing_id: listing_id as u64, count: 1 }).try_send();
}

/// MarketGetBack: [listing_id: u32]
fn forward_market_get_back(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let listing_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("MarketGetBack: session={} listing={}", session_id, listing_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::MarketGetBackRequest { session_id, listing_id: listing_id as u64 }).try_send();
}

/// MarketSellNow: [item_index: u32][price: u32]
fn forward_market_sell_now(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let unique_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let price = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("MarketSellNow: session={} uid={} price={}", session_id, unique_id, price);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::MarketSellNowRequest { session_id, unique_id: unique_id as u64, price: price as u64 }).try_send();
}

/// FishingCast: [type: u8]
fn forward_fishing_cast(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let fishing_type = payload.first().copied().unwrap_or(0);
    debug!("FishingCast: session={} type={}", session_id, fishing_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::FishingCastRequest { session_id, fishing_type }).try_send();
}

/// FishingChangeAutocast: [enabled: bool]
fn forward_fishing_change_autocast(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let enabled = payload.first().copied().unwrap_or(0) != 0;
    debug!("FishingChangeAutocast: session={} enabled={}", session_id, enabled);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::FishingChangeAutocastRequest { session_id, enabled }).try_send();
}

/// CombineItem: [from: u32][to: u32]
fn forward_combine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let from_grid = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to_grid = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("CombineItem: session={} from={} to={}", session_id, from_grid, to_grid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::CombineItemRequest { session_id, from_grid, to_grid }).try_send();
}

/// AwakeningNeedMaterials: [unique_id: u64][awake_type: u8]
fn forward_awakening_need_materials(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 9 { return; }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let awake_type = payload[8];
    debug!("AwakeningNeedMaterials: session={} uid={}", session_id, unique_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::AwakeningNeedMaterialsRequest { session_id, unique_id, awake_type }).try_send();
}

/// AwakeningLockedItem: [unique_id: u64][locked: u8]
fn forward_awakening_locked_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 9 { return; }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let locked = payload[8] != 0;
    debug!("AwakeningLockedItem: session={} uid={} locked={}", session_id, unique_id, locked);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::AwakeningLockedItemRequest { session_id, unique_id, locked }).try_send();
}

/// Awakening: [unique_id: u64][awake_type: u8][position_idx: u32]
fn forward_awakening(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 9 { return; }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let awake_type = payload[8];
    debug!("Awakening: session={} uid={} type={}", session_id, unique_id, awake_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::AwakeningRequest { session_id, unique_id, awake_type }).try_send();
}

/// DisassembleItem: [unique_id: u64]
fn forward_disassemble_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DisassembleItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::DisassembleItemRequest { session_id, unique_id: uid }).try_send();
}

/// DowngradeAwakening: [unique_id: u64]
fn forward_downgrade_awakening(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DowngradeAwakening: session={} uid={}", session_id, unique_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::DowngradeAwakeningRequest { session_id, unique_id }).try_send();
}

/// ResetAddedItem: [unique_id: u64]
fn forward_reset_added_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("ResetAddedItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ResetAddedItemRequest { session_id, unique_id: uid }).try_send();
}

/// DepositTradeItem: [from: i32][to: i32]
fn forward_deposit_trade_item(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("DepositTradeItem: session={} from={} to={}", session_id, from, to);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::DepositTradeItemBySlot {
        session_id, from_slot: from, to_slot: to,
    }).try_send();
}

/// RetrieveTradeItem: [from: i32][to: i32]
fn forward_retrieve_trade_item(social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("RetrieveTradeItem: session={} from={} to={}", session_id, from, to);
    let social_ref = match social_ref { Some(s) => s, None => return };
    let _ = social_ref.tell(crate::actors::social::RetrieveTradeItemBySlot {
        session_id, from_slot: from, to_slot: to,
    }).try_send();
}

/// GuildWarReturn: [guild_name: DotNetString]
fn forward_guild_war_return(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let guild_name = parse_dotnet_string(payload);
    debug!("GuildWarReturn: session={} guild={}", session_id, guild_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::GuildWarReturnRequest { session_id, guild_name }).try_send();
}

/// GuildBuffUpdate: [buff_id: u32]
fn forward_guild_buff_update(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let buff_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("GuildBuffUpdate: session={} buff_id={}", session_id, buff_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::GuildBuffUpdateRequest { session_id, buff_id }).try_send();
}

/// LockMail: [mail_id: u64][lock: bool]
fn forward_lock_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 9 { return; }
    let mail_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let lock = payload[8] != 0;
    debug!("LockMail: session={} mail_id={} lock={}", session_id, mail_id, lock);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::LockMailRequest { session_id, mail_id, lock }).try_send();
}

/// MailLockedItem: [mail_id: u64][item_index: u32]
fn forward_mail_locked_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 12 { return; }
    let mail_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let item_index = u32::from_le_bytes(payload[8..12].try_into().unwrap_or([0; 4]));
    debug!("MailLockedItem: session={} mail_id={} item_index={}", session_id, mail_id, item_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::MailLockedItemRequest { session_id, mail_id, item_index }).try_send();
}

/// MailCost: [items_count: u32][gold: u32]
async fn handle_mail_cost(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("MailCost: session={}", session_id);
    // 返回计算结果（免费）
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_le_bytes());
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: build_packet_bytes(ServerPacketIds::MailCost as i16, &body),
        }).await;
}

/// ShareQuest: [quest_id: u32]
fn forward_share_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let quest_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("ShareQuest: session={} quest_id={}", session_id, quest_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ShareQuestRequest { session_id, quest_id }).try_send();
}

/// AcceptReincarnation: []
fn forward_accept_reincarnation(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("AcceptReincarnation: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::AcceptReincarnationRequest { session_id }).try_send();
}

/// CancelReincarnation: []
fn forward_cancel_reincarnation(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("CancelReincarnation: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::CancelReincarnationRequest { session_id }).try_send();
}

/// GetRentedItems: forward to WorldActor
fn forward_get_rented_items(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId) {
    debug!("GetRentedItems: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::GetRentedItemsRequest { session_id }).try_send();
}

/// ItemRentalRequest: [target_name: DotNetString]
fn forward_item_rental_request(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let target_name = parse_dotnet_string(payload);
    debug!("ItemRentalRequest: session={} target={}", session_id, target_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ItemRentalRequestMsg { session_id, target_name }).try_send();
}

/// ItemRentalFee: [amount: u32]
fn forward_item_rental_fee(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let amount = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("ItemRentalFee: session={} amount={}", session_id, amount);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ItemRentalFeeMsg { session_id, amount }).try_send();
}

/// ItemRentalPeriod: [duration: u32]
fn forward_item_rental_period(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let duration = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("ItemRentalPeriod: session={} duration={}", session_id, duration);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ItemRentalPeriodMsg { session_id, duration }).try_send();
}

/// DepositRentalItem: [unique_id: u64]
fn forward_deposit_rental_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DepositRentalItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::DepositRentalItemRequest { session_id, unique_id: uid }).try_send();
}

/// RetrieveRentalItem: [unique_id: u64]
fn forward_retrieve_rental_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("RetrieveRentalItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::RetrieveRentalItemRequest { session_id, unique_id: uid }).try_send();
}

/// CancelItemRental: []
fn forward_cancel_item_rental(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("CancelItemRental: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::CancelItemRentalRequest { session_id }).try_send();
}

/// ItemRentalLockFee: []
fn forward_item_rental_lock_fee(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ItemRentalLockFee: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ItemRentalLockFeeMsg { session_id }).try_send();
}

/// ItemRentalLockItem: []
fn forward_item_rental_lock_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ItemRentalLockItem: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ItemRentalLockItemMsg { session_id }).try_send();
}

/// ConfirmItemRental: []
fn forward_confirm_item_rental(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ConfirmItemRental: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ConfirmItemRentalMsg { session_id }).try_send();
}

/// NPCConfirmInput: [npc_id: u32][input: DotNetString]
fn forward_npc_confirm_input(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let input_text = parse_dotnet_string(&payload[4..]);
    debug!("NPCConfirmInput: session={} npc_id={} input={}", session_id, npc_id, input_text);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::NPCConfirmInputRequest { session_id, npc_id, input_text }).try_send();
}

/// GameshopBuy: [item_id: u32][quantity: u32]
fn forward_gameshop_buy(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let item_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let count = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("GameshopBuy: session={} item={} count={}", session_id, item_id, count);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::GameshopBuyRequest { session_id, item_id, count }).try_send();
}

/// ReportIssue: [type: u32][description: DotNetString]
fn forward_report_issue(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let issue_type = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) as u8 } else { 0 };
    let description = if payload.len() >= 4 { parse_dotnet_string(&payload[4..]) } else { String::new() };
    debug!("ReportIssue: session={} type={}", session_id, issue_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::ReportIssueRequest { session_id, issue_type, description }).try_send();
}

/// GetRanking: [type: u32][page: u32]
fn forward_get_ranking(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let rank_type = if !payload.is_empty() { payload[0] } else { 0 };
    debug!("GetRanking: session={} type={}", session_id, rank_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::GetRankingRequest { session_id, rank_type }).try_send();
}

/// Opendoor: [door_index: u8]
fn forward_opendoor(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let door_index = payload[0];
    debug!("Opendoor: session={} door_index={}", session_id, door_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::OpendoorRequest { session_id, door_index }).try_send();
}

/// GuildTerritoryPage: [page: u32]
fn forward_guild_territory_page(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let page = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("GuildTerritoryPage: session={} page={}", session_id, page);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::GuildTerritoryPageRequest { session_id, page }).try_send();
}

/// PurchaseGuildTerritory: [territory_id: u32]
fn forward_purchase_guild_territory(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let territory_id = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("PurchaseGuildTerritory: session={} territory={}", session_id, territory_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.tell(crate::actors::world::PurchaseGuildTerritoryRequest { session_id, territory_id }).try_send();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 7-bit encoded length + UTF-8 bytes
    fn make_dotnet_string(s: &str) -> Vec<u8> {
        let bytes = s.as_bytes();
        let mut len = bytes.len();
        let mut out = Vec::new();
        loop {
            let mut b = (len & 0x7F) as u8;
            len >>= 7;
            if len != 0 { b |= 0x80; }
            out.push(b);
            if len == 0 { break; }
        }
        out.extend_from_slice(bytes);
        out
    }

    #[test]
    fn test_parse_dotnet_string_empty() {
        let data = make_dotnet_string("");
        assert_eq!(parse_dotnet_string(&data), "");
    }

    #[test]
    fn test_parse_dotnet_string_hello() {
        let data = make_dotnet_string("hello");
        assert_eq!(parse_dotnet_string(&data), "hello");
    }

    #[test]
    fn test_parse_dotnet_string_chinese() {
        let data = make_dotnet_string("物品租赁");
        assert_eq!(parse_dotnet_string(&data), "物品租赁");
    }

    #[test]
    fn test_parse_dotnet_string_malformed_empty() {
        // Empty slice → read_u8 fails → returns empty string
        let result = parse_dotnet_string(&[]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_dotnet_string_malformed_truncated_length() {
        // Incomplete 7-bit length (continuation byte but no following byte)
        let data = [0x80]; // says "more bytes coming" but none follow
        let result = parse_dotnet_string(&data);
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_dotnet_string_malformed_truncated_body() {
        // Length says 10 bytes but only 3 provided
        let data = [10, 0x61, 0x62, 0x63];
        let result = parse_dotnet_string(&data);
        assert_eq!(result, "");
    }
}


