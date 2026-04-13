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

use super::codec::{encode, decode};
use crate::util::wire::{build_packet_bytes, write_dotnet_string};

/// 会话 ID
pub type SessionId = u64;

/// 发送到客户端的数据通道
type SendChannel = mpsc::UnboundedSender<Vec<u8>>;

/// GateActor 状态
pub struct GateActor {
    /// 活跃会话的发送通道
    sessions: HashMap<SessionId, SendChannel>,
    /// AccountActor 引用
    account_ref: Option<ActorRef<crate::actors::account::AccountActor>>,
    /// WorldActor 引用
    world_ref: Option<ActorRef<crate::actors::world::WorldActor>>,
}

impl GateActor {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            account_ref: None,
            world_ref: None,
        }
    }

    pub fn set_account_ref(&mut self, account_ref: ActorRef<crate::actors::account::AccountActor>) {
        self.account_ref = Some(account_ref);
    }

    pub fn set_world_ref(&mut self, world_ref: ActorRef<crate::actors::world::WorldActor>) {
        self.world_ref = Some(world_ref);
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
}

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
}

/// 设置 AccountActor 引用
pub struct SetAccountRef {
    pub account_ref: ActorRef<crate::actors::account::AccountActor>,
}

/// 设置 WorldActor 引用
pub struct SetWorldRef {
    pub world_ref: ActorRef<crate::actors::world::WorldActor>,
}

// ============================================================
// Handler 实现
// ============================================================

impl Message<SessionCreated> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionCreated,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.sessions.insert(msg.session_id, msg.sender);
        debug!("Session {} created", msg.session_id);

        // 发送 Connected 包给客户端（客户端收到后会自动发送 ClientVersion）
        let connected_data = build_packet_bytes(ServerPacketIds::Connected as i16, &[]);
        let gate_ref = _ctx.actor_ref().clone();
        let _ = gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: connected_data,
        });
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
                handle_client_version(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::NewAccount as i16 => {
                // NewAccount - Phase 1: 自动成功
                handle_new_account(&gate_ref, msg.session_id);
            }
            x if x == ClientPacketIds::Login as i16 => {
                // Login - 转发到 AccountActor
                if let Some(account_ref) = &self.account_ref {
                    if let Some((username, password)) = parse_login_payload(payload) {
                        debug!("Login request: username={}", username);
                        let _ = account_ref.ask(crate::actors::account::LoginRequest {
                            session_id: msg.session_id,
                            username,
                            password,
                        }).await;
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
                        let _ = world_ref.ask(crate::actors::world::StartGameRequest {
                            session_id: msg.session_id,
                            character_index,
                        }).await;
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
                handle_keep_alive(&gate_ref, msg.session_id);
            }
            x if x == ClientPacketIds::LogOut as i16 => {
                // LogOut - 通知 WorldActor 清理并断开
                if let Some(world_ref) = &self.world_ref {
                    let _ = world_ref.ask(crate::actors::world::PlayerLogOut {
                        session_id: msg.session_id,
                    }).await;
                }
            }
            x if x == ClientPacketIds::Chat as i16 => {
                // Chat - 解析并广播
                if let Some(world_ref) = &self.world_ref {
                    if let Some(message) = parse_chat_payload(payload) {
                        let _ = world_ref.ask(crate::actors::world::ChatRequest {
                            session_id: msg.session_id,
                            message,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::CallNPC as i16 => {
                // CallNPC - 与 NPC 对话
                if let Some(world_ref) = &self.world_ref {
                    if let Some((npc_object_id, key)) = parse_call_npc_payload(payload) {
                        let _ = world_ref.ask(crate::actors::world::NPCCallRequest {
                            session_id: msg.session_id,
                            npc_object_id,
                            key,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::PickUp as i16 => {
                // PickUp - 拾取物品（Phase 1：回复无物品）
                handle_pickup(&gate_ref, msg.session_id);
            }
            x if x == ClientPacketIds::MoveItem as i16 => {
                handle_move_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::UseItem as i16 => {
                handle_use_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EquipItem as i16 => {
                handle_equip_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveItem as i16 => {
                handle_remove_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DropItem as i16 => {
                handle_drop_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MergeItem as i16 => {
                handle_merge_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RangeAttack as i16 => {
                handle_range_attack(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Magic as i16 => {
                handle_magic(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Harvest as i16 => {
                handle_harvest(&gate_ref, msg.session_id, payload);
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
        self.sessions.remove(&msg.session_id);
        debug!("Session {} disconnected", msg.session_id);

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
        let response_data = if msg.success {
            // LoginSuccess: 空角色列表
            let mut body = Vec::new();
            body.extend_from_slice(&0i32.to_le_bytes()); // count = 0
            build_packet_bytes(ServerPacketIds::LoginSuccess as i16, &body)
        } else {
            // Login failure (result=4: Wrong Password)
            build_packet_bytes(ServerPacketIds::Login as i16, &[4u8])
        };

        let gate_ref = ctx.actor_ref().clone();
        let _ = gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: response_data,
        }).await;
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

// ============================================================
// 辅助函数
// ============================================================

/// 处理客户端版本：验证 payload 后回复 accepted
fn handle_client_version(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
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
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: response,
    });
}

/// 处理新账号注册
fn handle_new_account(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    debug!("NewAccount request from session {}", session_id);
    // Phase 1: auto-register, respond success (result=8)
    let response = build_packet_bytes(ServerPacketIds::NewAccount as i16, &[8u8]);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: response,
    });
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

/// 处理 PickUp：回复 "附近没有物品"（Phase 1：无地面物品系统）
fn handle_pickup(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    // 发送 Chat 消息（ChatType::System = 2），客户端 chat handler 会解析为 SystemMessage
    let mut body = Vec::new();
    write_dotnet_string(&mut body, "附近没有可以拾取的物品。");
    body.push(2u8); // ChatType::System
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::Chat as i16, &body),
    });
}

/// 处理心跳：回复 KeepAlive
fn handle_keep_alive(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    let response = build_packet_bytes(ServerPacketIds::KeepAlive as i16, &[]);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: response,
    });
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
// 物品操作 stub handler（Phase 1：回复操作失败/不支持）
// ============================================================================

/// 回复"操作失败"系统消息
fn reply_item_op_failed(gate_ref: &ActorRef<GateActor>, session_id: SessionId, msg: &str) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, msg);
    body.push(2u8); // ChatType::System
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::Chat as i16, &body),
    });
}

/// MoveItem: [grid: u8][from: i32][to: i32]
fn handle_move_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 9 {
        let grid = payload[0];
        let from = i32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
        let to = i32::from_le_bytes([payload[5], payload[6], payload[7], payload[8]]);
        debug!("MoveItem: session={} grid={} from={} to={}", session_id, grid, from, to);
    }
    reply_item_op_failed(gate_ref, session_id, "背包整理功能暂未开放。");
}

/// UseItem: [unique_id: u64]
fn handle_use_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("UseItem: session={} uid={}", session_id, uid);
    }
    reply_item_op_failed(gate_ref, session_id, "物品使用功能暂未开放。");
}

/// EquipItem: [grid: u8][unique_id: u64][to: u8]
fn handle_equip_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 10 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        let to = payload[9];
        debug!("EquipItem: session={} grid={} uid={} slot={}", session_id, grid, uid, to);
    }
    reply_item_op_failed(gate_ref, session_id, "装备功能暂未开放。");
}

/// RemoveItem: [grid: u8][unique_id: u64][to: u8]
fn handle_remove_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 10 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        debug!("RemoveItem: session={} grid={} uid={}", session_id, grid, uid);
    }
    reply_item_op_failed(gate_ref, session_id, "卸下装备功能暂未开放。");
}

/// DropItem: [unique_id: u64][count: u16][hero_inventory: bool]
fn handle_drop_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 10 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("DropItem: session={} uid={}", session_id, uid);
    }
    reply_item_op_failed(gate_ref, session_id, "丢弃物品功能暂未开放。");
}

/// MergeItem: [grid_from: u8][grid_to: u8][id_from: u64][id_to: u64]
fn handle_merge_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 18 {
        let from = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
        let to = u64::from_le_bytes(payload[10..18].try_into().unwrap_or([0; 8]));
        debug!("MergeItem: session={} from={} to={}", session_id, from, to);
    }
    reply_item_op_failed(gate_ref, session_id, "物品堆叠功能暂未开放。");
}

/// RangeAttack: [dir: u8][x: i32][y: i32][target_id: u32][tx: i32][ty: i32]
fn handle_range_attack(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 21 {
        let dir = payload[0];
        let target_id = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
        debug!("RangeAttack: session={} dir={} target={}", session_id, dir, target_id);
    }
    // 远程攻击暂不支持，复用 Attack 的逻辑提示
    reply_item_op_failed(gate_ref, session_id, "远程攻击功能暂未开放。");
}

/// Magic: [spell: u8][dir: u8][target_id: u32][x: i32][y: i32]
fn handle_magic(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 12 {
        let spell = payload[0];
        let dir = payload[1];
        debug!("Magic: session={} spell={} dir={}", session_id, spell, dir);
    }
    reply_item_op_failed(gate_ref, session_id, "魔法技能功能暂未开放。");
}

/// Harvest: [dir: u8]
fn handle_harvest(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let dir = payload[0];
        debug!("Harvest: session={} dir={}", session_id, dir);
    }
    reply_item_op_failed(gate_ref, session_id, "采集功能暂未开放。");
}
