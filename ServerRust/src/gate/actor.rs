// GateActor - TCP 接入层
// 对应 C# LoginGate/AppServer.cs + SelGate + GameGate
// 职责：接受 TCP 连接，解析帧，转发到 AccountActor/WorldActor

use std::collections::HashMap;

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use mir2_shared::enums::{ClientPacketIds, ServerPacketIds};
use mir2_shared::packets::Packet;

use super::codec::{decode, encode};
use crate::util::wire::build_packet_bytes;

/// 会话 ID
pub type SessionId = u64;

/// 发送到客户端的数据通道（有界：慢读客户端广播堆积即踢线，防内存 DoS，#23）
type SendChannel = mpsc::Sender<Vec<u8>>;

/// 每会话待发队列容量；积满说明客户端慢读/不读，直接踢线
///
/// 容量下限锚定实机进图洪峰（2026-09-17 冒烟实测）：比奇一类大图进图时
/// 服务端一次性下发 ~2000 包（43 NPC + ~1900 怪物 + 地物/门/互见），
/// 1024 在 localhost 都有 ~50% 概率积满误踢正常客户端。16384 留 8 倍余量；
/// 慢读踢线语义不变（积满 16384 条≈1.6MB 仍未 drain 才踢）。
const SESSION_SEND_CAPACITY: usize = 16384;

/// GateActor kameo mailbox 容量（main.rs 建 actor 时使用）。
/// 容量下限锚定实机进图洪峰（2026-09-17 冒烟实测）：大图进图单次洪峰
/// ~2000 包/会话，1024 会把正常客户端的对象包静默丢弃（隐形怪物/NPC）；
/// 65536 覆盖 ~30 会话同时进图洪峰且内存上限 ~6MB。
pub const GATE_MAILBOX_CAPACITY: usize = 65536;

/// ShutdownAll 逐会话清理的整体超时（通知已 fire-and-forget，正常即时完成；
/// 超时仅兜底防回归，超时后后台清理任务继续跑）
const SHUTDOWN_ALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// GateActor 状态
pub struct GateActor {
    /// 活跃会话的发送通道
    sessions: HashMap<SessionId, SendChannel>,
    /// 会话关联的用户名（登录成功后设置）
    session_usernames: HashMap<SessionId, String>,
    /// 登录返回 RequirePasswordChange（S.Login Result=5）的会话 → 账号名：
    /// 仅这些会话允许未登录态改密，且只能改该账号（#11 爆破面收敛 + 强制改密流程保留）
    pending_password_change: HashMap<SessionId, String>,
    /// 会话关联的客户端 IP（C# MirConnection.IPAddress）
    session_ips: HashMap<SessionId, String>,
    /// 每会话踢线/清理信号：terminate_session 触发后读循环排空已排队数据并关闭 TCP
    /// （踢线实效化——旧实现只删映射，TCP 读循环存活，被踢连接可继续发包）
    session_cancels: HashMap<SessionId, tokio::sync::watch::Sender<bool>>,
    /// 登出中（LogOut 已受理、LogOutCleanup 未落地）的会话：
    /// 此窗口内除 KeepAlive 外拒收一切包——world 侧玩家记录已删而 gate 会话/
    /// 登录映射尚在，客户端 rapid-fire 的 StartGame 会以旧角色状态重进
    logging_out: std::collections::HashSet<SessionId>,
    /// 被封禁 IP -> 解封时间（unix 秒；C# Envir.IPBlocks）
    ip_blocks: HashMap<String, i64>,
    /// 每 IP 创建角色时间戳（unix 秒；C# ConnectionLogs[IP].CharactersMade）
    ip_character_creations: HashMap<String, Vec<i64>>,
    /// 每 IP 注册账号时间戳（unix 秒；C# ConnectionLogs[IP].AccountsMade，>2/小时封 24h）
    ip_accounts_made: HashMap<String, Vec<i64>>,
    /// AccountActor 引用
    account_ref: Option<ActorRef<crate::actors::account::AccountActor>>,
    /// WorldActor 引用
    world_ref: Option<ActorRef<crate::actors::world::WorldActor>>,
    /// SocialActor 引用
    social_ref: Option<ActorRef<crate::actors::social::SocialActor>>,
    /// 最大并发连接数(Phase 1.1:防止资源耗尽;从 cfg.network.max_connections 设置)
    max_connections: usize,
    /// 出站统计开关（`MIR2_EGRESS_STATS=1`）：仅记账、零行为改变
    egress_enabled: bool,
    /// 每会话出站累计（包个数 × 尺寸），会话收尾时打一行 `EGRESS_STATS`
    egress_stats: HashMap<SessionId, crate::util::egress_stats::EgressStats>,
}

impl GateActor {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_usernames: HashMap::new(),
            pending_password_change: HashMap::new(),
            session_ips: HashMap::new(),
            session_cancels: HashMap::new(),
            logging_out: std::collections::HashSet::new(),
            ip_blocks: HashMap::new(),
            ip_character_creations: HashMap::new(),
            ip_accounts_made: HashMap::new(),
            account_ref: None,
            world_ref: None,
            social_ref: None,
            max_connections: 1024,
            egress_enabled: crate::util::egress_stats::egress_stats_enabled(),
            egress_stats: HashMap::new(),
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

    /// 会话终止清理：移除发送通道/用户名/IP 映射，通知 AccountActor 登出
    /// （is_online 置 false，否则旧账号永远「已在线」无法重登），
    /// 并按需通知 WorldActor 玩家下线（触发落库）。
    /// TCP 断连 / 优雅 Disconnect / LogOut / 慢读踢线 / ShutdownAll 共用。
    async fn terminate_session(&mut self, session_id: SessionId, notify_world: bool) {
        self.sessions.remove(&session_id);
        self.session_ips.remove(&session_id);
        self.pending_password_change.remove(&session_id);
        // 登出标记随会话清理一并清位（LogOutCleanup 路径的正常出口）
        self.logging_out.remove(&session_id);
        // 出站统计（`MIR2_EGRESS_STATS=1`）：会话收尾时把该会话的载荷构成打成一行，
        // 供入场路径定量（CAPACITY.md §4 的「包个数 × 尺寸」）。仅读账，不改行为。
        if self.egress_enabled {
            if let Some(stats) = self.egress_stats.remove(&session_id) {
                info!("EGRESS_STATS session={} {}", session_id, stats.summary(8));
            }
        }
        // 踢线实效化：通知读循环退出并关闭 TCP——旧实现只删映射，读循环存活，
        // 被踢连接可继续发 ClientData（靠入口拦截兜底）、关了 TCP 也因
        // ClientDisconnected 守卫早退导致 is_online 永卡
        if let Some(cancel) = self.session_cancels.remove(&session_id) {
            let _ = cancel.send(true);
        }
        let logged_out_username = self.session_usernames.remove(&session_id);
        // 顶号安全（2026-09-23，CAPACITY.md §3.6 规格①）：**只有该账号最后一个绑定会话**
        // 才有权把账号置离线。原实现是无条件置离线——同账号顶号时，旧会话的断开清理会把
        // 新会话正在用的账号标记离线（可被再次登录顶号、且新会话登出变成空操作）。
        // 之前的保护是「gate 在 StartGame 处理里内联摘除旧绑定」（靠邮箱 FIFO 保证顺序），
        // 那可行但把 gate 卡在长 await 上；改成这条规则后，**顺序不再影响正确性**。
        let offline_username =
            should_mark_account_offline(logged_out_username.as_deref(), &self.session_usernames);
        if logged_out_username.is_some() && offline_username.is_none() {
            debug!(
                "Session {} removed binding for '{}' but account still bound by another session — not marking offline",
                session_id,
                logged_out_username.as_deref().unwrap_or("")
            );
        }
        // 有界邮箱死锁加固（#23）：gate 处理器内联 ask world/account 时，world 清理
        // 又 tell(SendToClient) 回 gate（邮箱满即阻塞）构成 gate→world→gate 循环等待
        // ——对世界/账号的通知一律 fire-and-forget，gate 不内联 await
        if let Some(username) = offline_username {
            if let Some(account_ref) = self.account_ref.clone() {
                crate::util::tasks::spawn("gate.account_logout", async move {
                    let _ = account_ref
                        .ask(crate::actors::account::LogoutRequest { username })
                        .await;
                });
            }
        }
        if notify_world {
            if let Some(world_ref) = self.world_ref.clone() {
                crate::util::tasks::spawn("gate.player_disconnected", async move {
                    let _ = world_ref
                        .ask(crate::actors::world::PlayerDisconnected { session_id })
                        .await;
                });
            }
        }
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
/// 会话清理时：**谁有权把账号置离线**（纯函数，便于门禁）。
///
/// 规则：只有「该账号的最后一个绑定会话」被清理时才置离线。
///
/// 背景（CAPACITY.md §3.6 规格①）：同账号顶号时，新会话接管、旧会话被踢；旧会话的断开清理
/// 若**无条件**置离线，就会出现「新会话还在线、账号已被标记离线」——既可能被第三方再次登录顶号，
/// 也让新会话的登出变成空操作。原实现靠「gate 在 StartGame 处理里内联摘除旧绑定 + 邮箱 FIFO
/// 保证顺序」来规避，代价是 gate 被长 await（建号+载图+发进场序列）堵住整个邮箱。
/// 改成这条规则后，解绑与断开的先后不再影响正确性，长 await 才能挪出去。
pub(crate) fn should_mark_account_offline(
    removed_username: Option<&str>,
    remaining: &HashMap<SessionId, String>,
) -> Option<String> {
    let username = removed_username?;
    if remaining.values().any(|u| u == username) {
        return None;
    }
    Some(username.to_string())
}

pub async fn run_gate_listener(addr: String, actor_ref: ActorRef<GateActor>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!("Gate listening on {}", addr);

    let mut session_id: SessionId = 1;

    // #2606：进程关闭时退出 accept 循环（否则它活过 ShutdownAll，成为残留任务）
    let shutdown = crate::util::tasks::shutdown_signal();
    loop {
        let (mut stream, peer_addr) = tokio::select! {
            _ = shutdown.cancelled() => {
                info!("Gate listener shutting down after {} sessions", session_id - 1);
                return Ok(());
            }
            accepted = listener.accept() => accepted?,
        };
        debug!("New connection from {}", peer_addr);

        let sid = session_id;
        session_id += 1;

        // 为每个会话创建发送通道（有界，容量见 SESSION_SEND_CAPACITY）
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(SESSION_SEND_CAPACITY);

        let gate_ref = actor_ref.clone();
        // #2606：每连接读循环也登记（生命周期 = 连接；关闭信号兜底唤醒）
        let session_shutdown = shutdown.clone();
        // 2026-09-23 压测（tools/ops/load_baseline.ps1）发现：连接建立后的「注册 + 取取消信号」
        // 是两次 GateActor 邮箱往返，原先**在 accept 循环里 await**——意味着一个个连接排队建会话。
        // 实测 30 并发登录时单次注册 200–330ms，登录 p95 7.8s。这里整体丢进任务，
        // accept 循环立刻回到 accept()。会话号仍在循环里顺序分配（保持既有语义）。
        crate::util::tasks::spawn("gate.session_setup", async move {
            // 注册会话到 GateActor
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id: sid,
                    sender: tx,
                    ip: peer_addr.ip().to_string(),
                })
                .await;

            // 踢线/会话清理信号：terminate_session 触发后本读循环排空已排队数据并关 TCP
            let kick_rx = gate_ref
                .ask(TakeSessionCancel { session_id: sid })
                .await
                .ok()
                .flatten();

            let mut buf = Vec::with_capacity(4096);
            let mut temp = [0u8; 4096];

            loop {
                tokio::select! {
                    // 进程关闭：不再读，交由 ShutdownAll 的断连/存档流程收尾
                    _ = session_shutdown.cancelled() => {
                        debug!("Session {} stopped by server shutdown", sid);
                        return;
                    }
                    // 踢线/会话清理：先把已排队数据（如 S.Disconnect）尽量写完，再关 TCP
                    _ = async {
                        if let Some(rx) = &kick_rx {
                            let mut rx = rx.clone();
                            // 订阅前已被踢（注册后立即 terminate 的竞态）：
                            // 当前值已是 true 时立即返回，不傻等下一次变化
                            if *rx.borrow_and_update() {
                                return;
                            }
                            let _ = rx.changed().await;
                        }
                    }, if kick_rx.is_some() => {
                        // 排空写加整体超时：慢读僵尸连接的内核缓冲塞满后 write_all
                        // 会永久挂起，session_reader 任务随之泄漏（长占连接资源）。
                        // 收尾包（如 S.Disconnect）尽力而为即可——2s 写不完放弃排空，
                        // 直接关 TCP 释放任务
                        let drain = async {
                            while let Ok(data) = rx.try_recv() {
                                let mut encoded = Vec::new();
                                encode(&data, &mut encoded);
                                if stream.write_all(&encoded).await.is_err() {
                                    break;
                                }
                            }
                        };
                        if tokio::time::timeout(std::time::Duration::from_secs(2), drain)
                            .await
                            .is_err()
                        {
                            warn!(
                                "Session {} kick drain write timed out, forcing close",
                                sid
                            );
                        }
                        let _ = stream.shutdown().await;
                        debug!("Session {} closed by gate (session terminated)", sid);
                        return;
                    }
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

/// 登出收尾清理（WorldActor 在 S.LogOutSuccess 落链之后入队；gate 邮箱 FIFO 保证
/// 先处理 SendToClient 再处理本消息）。只有收到它 gate 才删会话 + 置账号离线——
/// 否则 LogOutSuccess 会因会话先删被静默丢弃（严重19）。
pub struct LogOutCleanup {
    pub session_id: SessionId,
}

// 顶号解绑（原 UnbindSessionLogin）已移除：world dup-kick 改经
// StartGameReply.kicked_session_id 把被踢旧会话 id 带回，gate 在下方
// StartGame 臂内联摘除 session_usernames——同 handler 内完成，先于邮箱中
// 任何后到的 ClientDisconnected/LogOutCleanup 落地（原 spawn 异步 tell 与
// 已排队断开消息无 happens-before；且旧客户端被踢后重新 Login 会重建绑定，
// 迟到的异步解绑会误删新绑定，保留它不是兜底而是新竞态源，故移除）。

/// 登录结果（从 AccountActor 返回）
pub struct LoginResult {
    pub session_id: SessionId,
    pub success: bool,
    pub username: String,
    /// 角色摘要列表（登录成功时携带，用于选角界面）
    pub characters: Vec<crate::db::CharacterSummary>,
    /// 封禁到期时间（unix 秒；Some 时发 S.LoginBanned，C# WrongPasswordCount>=5 封 2 分钟）
    pub banned_until: Option<i64>,
    /// 需要强制改密（C# AccountInfo.RequirePasswordChange → S.Login{Result=5}）
    pub require_password_change: bool,
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

/// 会话状态探针：查询会话注册/登录态（诊断与集成回归测试用，只读）
pub struct TestProbeSession {
    pub session_id: SessionId,
}

impl Message<TestProbeSession> for GateActor {
    /// (会话已注册, 已登录)
    type Reply = (bool, bool);

    async fn handle(
        &mut self,
        msg: TestProbeSession,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        (
            self.sessions.contains_key(&msg.session_id),
            self.session_usernames.contains_key(&msg.session_id),
        )
    }
}

/// 测试辅助：直接写入登录绑定（session_usernames）——顶号路径回归需要第二会话
/// 持同账号绑定经 gate StartGame 臂（同账号在线时 Login 被拒，无法走正常登录），
/// 与 TestProbeSession/TestSetLoggingOut 同为红绿回归专用，不得用于生产路径
pub struct TestBindSessionLogin {
    pub session_id: SessionId,
    pub username: String,
}

impl Message<TestBindSessionLogin> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TestBindSessionLogin,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.session_usernames.insert(msg.session_id, msg.username);
    }
}

/// 测试探针：会话是否处于登出窗口（logging_out），红绿回归断言用，只读
pub struct TestProbeLoggingOut {
    pub session_id: SessionId,
}

impl Message<TestProbeLoggingOut> for GateActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: TestProbeLoggingOut,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.logging_out.contains(&msg.session_id)
    }
}

/// 测试探针：直接置/清登出标记——生产路径只能经 LogOut Success 置位、
/// terminate_session 清位；此消息供红绿回归确定性撑开竞态窗口（置位后
/// 验证门禁拒收语义），不得用于生产路径
pub struct TestSetLoggingOut {
    pub session_id: SessionId,
    pub on: bool,
}

impl Message<TestSetLoggingOut> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TestSetLoggingOut,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if msg.on {
            self.logging_out.insert(msg.session_id);
        } else {
            self.logging_out.remove(&msg.session_id);
        }
    }
}

/// 订阅会话踢线信号（gate listener 的读循环在注册后取走；terminate_session 触发）
pub struct TakeSessionCancel {
    pub session_id: SessionId,
}

impl Message<TakeSessionCancel> for GateActor {
    type Reply = Option<tokio::sync::watch::Receiver<bool>>;

    async fn handle(
        &mut self,
        msg: TakeSessionCancel,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.session_cancels
            .get(&msg.session_id)
            .map(|tx| tx.subscribe())
    }
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
        if self
            .ip_blocks
            .get(&msg.ip)
            .map(|&u| u > now)
            .unwrap_or(false)
        {
            warn!(
                "Connection rejected from blocked IP {} (session {})",
                msg.ip, msg.session_id
            );
            return;
        }
        // Phase 1.1: 连接数限制 — 超过 max_connections 拒绝新连接
        if self.sessions.len() >= self.max_connections {
            warn!(
                "Connection rejected: session {} would exceed max_connections {} (current={})",
                msg.session_id,
                self.max_connections,
                self.sessions.len()
            );
            // 不 insert session,不发 Connected — 客户端会因为收不到响应而超时断开
            return;
        }
        self.sessions.insert(msg.session_id, msg.sender);
        self.session_ips.insert(msg.session_id, msg.ip.clone());
        // 踢线信号通道（读循环经 TakeSessionCancel 订阅；terminate_session 触发）
        let (cancel_tx, _) = tokio::sync::watch::channel(false);
        self.session_cancels.insert(msg.session_id, cancel_tx);
        debug!(
            "Session {} created (active={})",
            msg.session_id,
            self.sessions.len()
        );

        // 发送 Connected 包给客户端（客户端收到后会自动发送 ClientVersion）
        let connected_data = build_packet_bytes(ServerPacketIds::Connected as i16, &[]);
        let gate_ref = _ctx.actor_ref().clone();
        let _ = gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: connected_data,
            })
            .try_send();
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

/// C# @CLEARIPBLOCKS（PlayerObject.cs:3065-3069）：清空全部 IP 封禁（GM）
pub struct ClearIpBlocks;

impl Message<ClearIpBlocks> for GateActor {
    type Reply = ();

    async fn handle(&mut self, _msg: ClearIpBlocks, _ctx: &mut Context<Self, Self::Reply>) {
        let count = self.ip_blocks.len();
        self.ip_blocks.clear();
        info!("ClearIpBlocks: cleared {} IP blocks", count);
    }
}

/// Phase 2.2: 优雅关机 — 断开所有活跃 session,触发 PlayerDisconnected 保存。
impl Message<ShutdownAll> for GateActor {
    type Reply = usize;

    async fn handle(&mut self, _msg: ShutdownAll, _ctx: &mut Context<Self, Self::Reply>) -> usize {
        let count = self.sessions.len();
        info!("ShutdownAll: disconnecting {} active sessions", count);
        let session_ids: Vec<u64> = self.sessions.keys().cloned().collect();
        let disconnect_data = crate::util::wire::build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::Disconnect as i16,
            // C# S.Disconnect.Reason：0=Server Closing（1 字节）
            &[0u8],
        );
        for sid in &session_ids {
            // 直接塞进会话发送通道（不能走 SendToClient 自转发：下方随即移除会话，
            // mailbox 里的 SendToClient 处理时会找不到通道）
            if let Some(tx) = self.sessions.get(sid) {
                let _ = tx.try_send(disconnect_data.clone());
            }
        }
        // #22：主动逐会话触发 PlayerDisconnected 落库 + 账号下线，
        // 不等客户端 Disconnect 回包（客户端 5 秒内不回即丢档）。
        // 有界邮箱死锁加固：terminate_session 对世界/账号的通知已改 fire-and-forget，
        // 本循环不再内联 await world/account；仍加整体超时兜底 + 日志，
        // 优雅关机不得被单点拖死（C# 主循环显式 join 全部后台线程，同语义）。
        let terminated = session_ids.len();
        match tokio::time::timeout(SHUTDOWN_ALL_TIMEOUT, async {
            for sid in session_ids {
                self.terminate_session(sid, true).await;
            }
        })
        .await
        {
            Ok(()) => info!("ShutdownAll: {} sessions terminated", terminated),
            Err(_) => {
                // 超时兜底：terminate_session 处理过的会话已自行移出 sessions，
                // 剩余的就是未处理的——它们尚未收到 world/account 通知（丢档 + 卡在
                // 在线）。spawn 一个无超时后台任务继续逐个补发通知（通知本身
                // spawn fire-and-forget，循环不阻塞）；gate 本地映射随进程退出收尾
                let remaining: Vec<(SessionId, Option<String>)> = self
                    .sessions
                    .keys()
                    .map(|sid| (*sid, self.session_usernames.get(sid).cloned()))
                    .collect();
                error!(
                    "ShutdownAll: timed out after {:?} ({} of {} sessions cleaned inline; dispatching world/account notifications for the remaining {} in a detached background task)",
                    SHUTDOWN_ALL_TIMEOUT,
                    terminated - remaining.len(),
                    terminated,
                    remaining.len()
                );
                let account_ref = self.account_ref.clone();
                let world_ref = self.world_ref.clone();
                crate::util::tasks::spawn("gate.shutdown_all_notify_bg", async move {
                    for (sid, username) in remaining {
                        if let Some(username) = username {
                            if let Some(account_ref) = account_ref.clone() {
                                crate::util::tasks::spawn("gate.account_logout", async move {
                                    let _ = account_ref
                                        .ask(crate::actors::account::LogoutRequest { username })
                                        .await;
                                });
                            }
                        }
                        if let Some(world_ref) = world_ref.clone() {
                            crate::util::tasks::spawn("gate.player_disconnected", async move {
                                let _ = world_ref
                                    .ask(crate::actors::world::PlayerDisconnected {
                                        session_id: sid,
                                    })
                                    .await;
                            });
                        }
                    }
                    info!(
                        "ShutdownAll: background notifications for remaining sessions dispatched"
                    );
                });
            }
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
            warn!(
                "Session {} received data too short for packet header",
                msg.session_id
            );
            return;
        }

        let length = u16::from_le_bytes([msg.data[0], msg.data[1]]) as usize;
        let opcode = i16::from_le_bytes([msg.data[2], msg.data[3]]);

        debug!(
            "Session {} packet: length={}, opcode={}",
            msg.session_id, length, opcode
        );

        // 验证长度一致性
        if length > msg.data.len() || length < HEADER_SIZE {
            warn!(
                "Session {} packet length mismatch: declared={}, available={}",
                msg.session_id,
                length,
                msg.data.len()
            );
            return;
        }

        let payload = &msg.data[HEADER_SIZE..length];
        let gate_ref = ctx.actor_ref().clone();

        // 踢线实效化：会话已注销（被踢/已清理）的连接除协议心跳外一律拒收——
        // 旧实现入口无注册检查，被踢连接（其读循环可能尚未退出）重发 Login 直达
        // AccountActor，LoginResult 再无条件回插映射，门禁全放行
        if opcode != ClientPacketIds::KeepAlive as i16
            && !self.sessions.contains_key(&msg.session_id)
        {
            debug!(
                "ClientData rejected: session {} not registered (opcode={})",
                msg.session_id, opcode
            );
            return;
        }

        // 登出窗口门禁：LogOut 已受理（S.LogOutSuccess → LogOutCleanup 在途）到
        // 会话清理落地之间，除 KeepAlive 外一律拒收——否则客户端 rapid-fire 的
        // StartGame 会趁 gate 登录映射尚在、world 玩家记录已删的窗口重进游戏
        if opcode != ClientPacketIds::KeepAlive as i16 && self.logging_out.contains(&msg.session_id)
        {
            debug!(
                "ClientData rejected: session {} is logging out (opcode={})",
                msg.session_id, opcode
            );
            return;
        }

        match opcode {
            x if x == ClientPacketIds::ClientVersion as i16 => {
                // ClientVersion - 验证 payload 后回复 accepted
                handle_client_version(&gate_ref, msg.session_id, payload).await;
            }
            x if x == ClientPacketIds::NewAccount as i16 => {
                self.handle_new_account(ctx.actor_ref(), msg.session_id, payload)
                    .await;
            }
            x if x == ClientPacketIds::Login as i16 => {
                // C# Settings.AllowLogin：关闭时 → S.Login{Result=0}
                let allow_login = if let Some(s) = &self.social_ref {
                    s.ask(crate::actors::social::NpcGetAllowLogin)
                        .await
                        .unwrap_or(true)
                } else {
                    true
                };
                if !allow_login {
                    let data = build_packet_bytes(ServerPacketIds::Login as i16, &[0u8]);
                    let gate_ref = ctx.actor_ref().clone();
                    let _ = gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data,
                        })
                        .try_send();
                    warn!(
                        "Login rejected: AllowLogin=false session={}",
                        msg.session_id
                    );
                    return;
                }
                // Login - 转发到 AccountActor (Phase 1.3: 输入验证)
                if let Some(account_ref) = &self.account_ref {
                    if let Some((username, password)) = parse_login_payload(payload) {
                        if !crate::util::validation::validate_username(&username) {
                            warn!(
                                "Login rejected: invalid username '{}' from session {}",
                                username, msg.session_id
                            );
                        } else if !crate::util::validation::validate_password(&password) {
                            warn!(
                                "Login rejected: invalid password length from session {} user={}",
                                msg.session_id, username
                            );
                        } else {
                            debug!("Login request: username={}", username);
                            // 登录链路分段观测（crystal-login-latency）：这一段是 gate 等
                            // AccountActor 的处理时间（含它内部的账号写 / 角色列表查询）。
                            let t_ask = std::time::Instant::now();
                            let _ = account_ref
                                .ask(crate::actors::account::LoginRequest {
                                    session_id: msg.session_id,
                                    username: username.clone(),
                                    password,
                                })
                                .await;
                            let ask_ms = t_ask.elapsed().as_millis() as u64;
                            if ask_ms >= 500 {
                                warn!(
                                    "LOGIN_TIMING gate_ask session={} user={} ask_ms={}",
                                    msg.session_id, username, ask_ms
                                );
                            } else {
                                debug!(
                                    "LOGIN_TIMING gate_ask session={} user={} ask_ms={}",
                                    msg.session_id, username, ask_ms
                                );
                            }
                        }
                    }
                } else {
                    warn!("AccountActor not linked");
                }
            }
            x if x == ClientPacketIds::StartGame as i16 => {
                // StartGame - 转发到 WorldActor
                if let Some(world_ref) = self.world_ref.clone() {
                    if payload.len() >= 4 {
                        let character_index =
                            i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                        debug!("StartGame request: character_index={}", character_index);
                        // 必须已登录才能进入游戏
                        let username = self.session_usernames.get(&msg.session_id).cloned();
                        if let Some(username) = username {
                            // 2026-09-23（CAPACITY.md §3.6）：**不再内联 await**。
                            //
                            // 原写法在这里 `await world_ref.ask(StartGameRequest)`——建号 + 载图 +
                            // 发整段进场序列都在这个 await 里完成，期间 gate 邮箱只进不出；
                            // 实测这正是单目标 `SendToClient` 丢包的根因（20 会话 3377 条，
                            // 丢的还就是进图对象包）。
                            //
                            // 之所以当初必须内联：靠邮箱 FIFO 保证「顶号解绑」先于旧会话的
                            // ClientDisconnected/LogOutCleanup 落地，否则 terminate_session 会
                            // 把新会话在用的账号置离线。现在这条已在 `should_mark_account_offline`
                            // 里改成「该账号最后一个绑定会话才有权置离线」，**顺序不再影响正确性**，
                            // 因此可以 spawn 出去、用 `StartGameFinished` 回投做解绑。
                            let gate_ref = ctx.actor_ref().clone();
                            let session_id = msg.session_id;
                            crate::util::tasks::spawn("gate.startgame", async move {
                                let kicked = world_ref
                                    .ask(crate::actors::world::StartGameRequest {
                                        session_id,
                                        character_index,
                                        account_username: username,
                                    })
                                    .await
                                    .ok()
                                    .and_then(|r| r.kicked_session_id);
                                let _ = gate_ref
                                    .tell(StartGameFinished {
                                        session_id,
                                        kicked_session_id: kicked,
                                    })
                                    .await;
                            });
                        } else {
                            warn!(
                                "StartGame rejected: session {} not logged in",
                                msg.session_id
                            );
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
                        let _ = world_ref
                            .ask(crate::actors::world::WorldTurnRequest {
                                session_id: msg.session_id,
                                direction,
                            })
                            .await;
                    }
                }
            }
            x if x == ClientPacketIds::Walk as i16 => {
                // Walk - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref
                            .ask(crate::actors::world::WorldMoveRequest {
                                session_id: msg.session_id,
                                direction,
                                is_run: false,
                            })
                            .await;
                    }
                }
            }
            x if x == ClientPacketIds::Run as i16 => {
                // Run - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref
                            .ask(crate::actors::world::WorldMoveRequest {
                                session_id: msg.session_id,
                                direction,
                                is_run: true,
                            })
                            .await;
                    }
                }
            }
            x if x == ClientPacketIds::Attack as i16 => {
                // Attack - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if payload.len() >= 2 {
                        let direction = payload[0];
                        let spell = payload[1];
                        debug!(
                            "Attack: session={} dir={} spell={}",
                            msg.session_id, direction, spell
                        );
                        let _ = world_ref
                            .ask(crate::actors::world::WorldAttackRequest {
                                session_id: msg.session_id,
                                direction,
                                spell,
                            })
                            .await;
                    }
                }
            }
            x if x == ClientPacketIds::KeepAlive as i16 => {
                // KeepAlive - 回复心跳
                handle_keep_alive(&gate_ref, msg.session_id).await;
            }
            x if x == ClientPacketIds::LogOut as i16 => {
                // LogOut - 通知 WorldActor 清理并落库（C# MirConnection.LogOut → StopGame）
                // 严重19 回归修复：gate 不得在此无条件 terminate_session——
                // Success 路径 S.LogOutSuccess 是 world 在 ask 期间 tell 进 gate 邮箱的，
                // 先删会话会让它静默丢弃（客户端卡死游戏场景、is_online 永卡）；
                // Blocked（战斗 10s 内）路径误清会留下幽灵 PlayerActor + 账号离线。
                // 正确顺序由 world 保证：S.LogOutSuccess 落链后再 tell LogOutCleanup
                // （gate 邮箱 FIFO 保序），gate 收到 LogOutCleanup 才 terminate_session；
                // Blocked 时 gate 不动任何状态（会话未删，S.LogOutFailed 天然可达）。
                // 已知权衡（头阻塞，暂不改行为）：本臂内联 await world ask（PlayerLogOut
                // 含落库 DB 写），world 处理期间 gate 收包主循环停摆、全服客户端包排队。
                // 当前接受该代价——换来严格 FIFO 保序（LogOutSuccess → LogOutCleanup
                // 不得乱序，见上）且落库通常在毫秒~百毫秒级；后续方向：world 收单即
                // 早应答、清理结果异步回推（复用 LogOutCleanup 通道），或按会话有序
                // 任务队列把落库移出 gate 收包关键路径。
                if let Some(world_ref) = &self.world_ref {
                    match world_ref
                        .ask(crate::actors::world::PlayerLogOut {
                            session_id: msg.session_id,
                        })
                        .await
                    {
                        Ok(crate::actors::world::PlayerLogOutReply::Success) => {
                            // world 已按序入队 S.LogOutSuccess → LogOutCleanup，等清理消息；
                            // 清理落地前打登出标记，拒收 rapid-fire 的 StartGame 等重进包
                            self.logging_out.insert(msg.session_id);
                        }
                        Ok(crate::actors::world::PlayerLogOutReply::Blocked) => {
                            debug!(
                                "LogOut blocked by world (combat cooldown), session {} untouched",
                                msg.session_id
                            );
                        }
                        Err(e) => {
                            warn!(
                                "PlayerLogOut ask failed for session {}: {}",
                                msg.session_id, e
                            );
                        }
                    }
                }
            }
            x if x == ClientPacketIds::Disconnect as i16 => {
                debug!("Client disconnect request from session {}", msg.session_id);
                // Forward to WorldActor for immediate player cleanup + 账号/会话清理
                self.terminate_session(msg.session_id, true).await;
            }
            x if x == ClientPacketIds::Chat as i16 => {
                // Chat - 解析并广播 (Phase 1.3: 输入验证)
                if let Some(world_ref) = &self.world_ref {
                    if let Some((message, linked_items)) = parse_chat_packet(payload) {
                        if !crate::util::validation::validate_chat(&message) {
                            warn!(
                                "Session {} chat rejected: len={}",
                                msg.session_id,
                                message.len()
                            );
                        } else {
                            let _ = world_ref
                                .ask(crate::actors::world::ChatRequest {
                                    session_id: msg.session_id,
                                    message,
                                    linked_items,
                                })
                                .await;
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
                        let _ = world_ref
                            .ask(crate::actors::world::NPCCallRequest {
                                session_id: msg.session_id,
                                npc_object_id,
                                key,
                            })
                            .await;
                    }
                }
            }
            x if x == ClientPacketIds::PickUp as i16 => {
                if let Some(world_ref) = &self.world_ref {
                    let _ = world_ref
                        .ask(crate::actors::world::PickUpRequest {
                            session_id: msg.session_id,
                        })
                        .await;
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
                forward_repair_item(&self.world_ref, msg.session_id, payload, false);
            }
            x if x == ClientPacketIds::SRepairItem as i16 => {
                forward_repair_item(&self.world_ref, msg.session_id, payload, true);
                // 特殊修理 ×3
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
                // #10：未登录（无 session_usernames 映射）直接拒绝并断开——
                // 否则旧代码会以角色名冒充账号名建角（unwrap_or_else(|| name.clone())）
                if !self.session_usernames.contains_key(&msg.session_id) {
                    warn!(
                        "NewCharacter rejected: session {} not logged in, disconnecting",
                        msg.session_id
                    );
                    let data = build_packet_bytes(ServerPacketIds::Disconnect as i16, &[0u8]);
                    if let Some(tx) = self.sessions.get(&msg.session_id) {
                        let _ = tx.try_send(data);
                    }
                    self.terminate_session(msg.session_id, false).await;
                    return;
                }
                // C# Envir.NewCharacter IP 防刷：封禁 IP / 每小时 >4 次 → 封 24h
                let now = gate_unix_now_secs();
                let ip = self
                    .session_ips
                    .get(&msg.session_id)
                    .cloned()
                    .unwrap_or_default();
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
                    let body = vec![0u8]; // S.NewCharacter { Result = 0 }
                    let data = build_packet_bytes(ServerPacketIds::NewCharacter as i16, &body);
                    let gate_ref = ctx.actor_ref().clone();
                    let _ = gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data,
                        })
                        .try_send();
                    warn!(
                        "NewCharacter rejected: IP {} rate-limited (session {})",
                        ip, msg.session_id
                    );
                    return;
                }
                forward_new_character(
                    &self.world_ref,
                    &self.session_usernames,
                    msg.session_id,
                    payload,
                )
                .await;
            }
            x if x == ClientPacketIds::ChangePassword as i16 => {
                forward_change_password(
                    ctx.actor_ref(),
                    &self.social_ref,
                    &self.account_ref,
                    &self.session_usernames,
                    &self.pending_password_change,
                    msg.session_id,
                    payload,
                )
                .await;
            }
            x if x == ClientPacketIds::DeleteCharacter as i16 => {
                forward_delete_character(
                    &self.world_ref,
                    &self.session_usernames,
                    msg.session_id,
                    payload,
                );
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
                forward_set_storage_password(
                    &self.account_ref,
                    &self.session_usernames,
                    msg.session_id,
                    payload,
                );
            }
            x if x == ClientPacketIds::RemoveStoragePassword as i16 => {
                forward_remove_storage_password(
                    &self.account_ref,
                    &self.session_usernames,
                    msg.session_id,
                    payload,
                );
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
            x if x == ClientPacketIds::ReviveHero as i16 => {
                handle_revive_hero(&self.world_ref, msg.session_id, payload);
            }
            // 宠物
            x if x == ClientPacketIds::UpdateIntelligentCreature as i16 => {
                handle_update_intelligent_creature(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::IntelligentCreaturePickup as i16 => {
                handle_intelligent_creature_pickup(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestIntelligentCreatureUpdates as i16 => {
                handle_request_intelligent_creature_updates(
                    &self.world_ref,
                    msg.session_id,
                    payload,
                );
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
                forward_mail_cost(&self.world_ref, msg.session_id, payload);
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
        let Some(tx) = self.sessions.get(&msg.session_id) else {
            warn!(
                "Attempted to send to non-existent session {}",
                msg.session_id
            );
            return;
        };
        // 出站统计（`MIR2_EGRESS_STATS=1`）：关掉开关时只多一次 bool 判断
        if self.egress_enabled {
            self.egress_stats
                .entry(msg.session_id)
                .or_default()
                .record(&msg.data);
        }
        debug!(
            "SendToClient: session={} bytes={}",
            msg.session_id,
            msg.data.len()
        );
        let bytes = msg.data.len();
        // #23：有界通道 + try_send——慢读/不读客户端积压超限即踢线，
        // 不能用 send().await 阻塞 GateActor 邮箱（一人慢读拖死全服广播）
        match tx.try_send(msg.data) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(
                    "Session {} send buffer full ({} bytes dropped): kicking slow reader",
                    msg.session_id, bytes
                );
                self.terminate_session(msg.session_id, true).await;
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // 写端任务已退出（连接实际已断），按断连清理
                debug!(
                    "Session {} send channel closed, cleaning up",
                    msg.session_id
                );
                self.terminate_session(msg.session_id, true).await;
            }
        }
    }
}

/// 批量下发：**同一份数据发给多个会话**（世界→客户端扇出的批量路径）。
///
/// 为什么需要它：广播若逐会话 `tell(SendToClient)`，就是 N 条邮箱消息 + N 份 payload 拷贝，
/// 20 人同图即 20 倍扇出——实测 20 个世界会话就把 GateActor 邮箱打满
/// （`gate mailbox full` 上万条，玩家侧表现为丢包/卡顿）。批量后一次广播只投递一条消息，
/// payload 只构造一次（`Arc` 共享）。
///
/// 语义与逐条 `SendToClient` **保持一致**：某个会话的发送通道满/已关闭时，仍按"慢读者"
/// 策略告警并踢线——批量不等于放宽背压保护。
pub struct SendToClients {
    pub sessions: Vec<SessionId>,
    pub data: std::sync::Arc<Vec<u8>>,
}

impl Message<SendToClients> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SendToClients,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let bytes = msg.data.len();
        for sid in &msg.sessions {
            let Some(tx) = self.sessions.get(sid) else {
                continue;
            };
            match tx.try_send((*msg.data).clone()) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!(
                        "Session {} send buffer full ({} bytes dropped): kicking slow reader",
                        sid, bytes
                    );
                    self.terminate_session(*sid, true).await;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    debug!("Session {} send channel closed, cleaning up", sid);
                    self.terminate_session(*sid, true).await;
                }
            }
        }
    }
}

/// StartGame 的异步回投（见 ClientData 里 StartGame 分支的注释）。
///
/// gate 不再内联 await world_ref.ask(StartGameRequest)（那会把整个邮箱堵在
/// "建号+载图+发进场序列"上，实测正是单目标丢包的根因），改为 spawn 后由本消息回来做收尾。
/// 收尾只剩"顶号解绑"——而它现在已经不依赖顺序（见 should_mark_account_offline）。
pub struct StartGameFinished {
    pub session_id: SessionId,
    pub kicked_session_id: Option<SessionId>,
}

impl Message<StartGameFinished> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StartGameFinished,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(old_sid) = msg.kicked_session_id {
            if self.session_usernames.remove(&old_sid).is_some() {
                info!(
                    "Session {} login binding removed (duplicate-login kick), new session={}",
                    old_sid, msg.session_id
                );
            }
        }
    }
}

impl Message<LogOutCleanup> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: LogOutCleanup,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // WorldActor 侧已由 PlayerLogOut 落库，不再重复 PlayerDisconnected；
        // 账号置离线（LogoutRequest）随之延后到此处，保证 LogOutSuccess 先落链
        self.terminate_session(msg.session_id, false).await;
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
        debug!("Session {} disconnected (TCP close)", msg.session_id);
        // 账号登出 + WorldActor 玩家清理（落库）
        self.terminate_session(msg.session_id, true).await;
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
            // 踢线后门禁：会话已注销（被踢后读循环存活期间重 Login）不得回插映射——
            // 旧代码无条件回插 session_usernames，被踢连接重 Login 后门禁全放行
            if !self.sessions.contains_key(&msg.session_id) {
                warn!(
                    "LoginResult ignored: session {} no longer registered (user={})",
                    msg.session_id, msg.username
                );
                // AccountActor 已置 is_online=true：回退置离线，否则该账号永卡在线
                if let Some(account_ref) = self.account_ref.clone() {
                    let username = msg.username.clone();
                    crate::util::tasks::spawn("gate.login_online_rollback", async move {
                        let _ = account_ref
                            .ask(crate::actors::account::LogoutRequest { username })
                            .await;
                    });
                }
                return;
            }
            // 记录 session 关联的用户名（用于 ChangePassword 等）
            self.session_usernames
                .insert(msg.session_id, msg.username.clone());
            // 登录成功，强制改密待办随之失效
            self.pending_password_change.remove(&msg.session_id);

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
                        .unwrap_or_else(chrono::Utc::now),
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
                })
                .try_send();
        } else if msg.require_password_change {
            // C#：RequirePasswordChange=true → S.Login { Result = 5 }
            // #11：登记该会话的强制改密待办——允许此会话未登录态改密（仅此账号），
            // 否则 #11 的登录态门禁会把强制改密流程一并堵死
            self.pending_password_change
                .insert(msg.session_id, msg.username.clone());
            let response_data = build_packet_bytes(ServerPacketIds::Login as i16, &[5u8]);
            let gate_ref = ctx.actor_ref().clone();
            let _ = gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: response_data,
                })
                .try_send();
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
                    })
                    .try_send();
            }
        } else {
            // Login failure
            let response_data = build_packet_bytes(ServerPacketIds::Login as i16, &[4u8]);
            let gate_ref = ctx.actor_ref().clone();
            let _ = gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: response_data,
                })
                .try_send();
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
async fn handle_client_version(
    gate_ref: &ActorRef<GateActor>,
    session_id: SessionId,
    payload: &[u8],
) {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;

    // ClientVersion payload: [version_hash_length: i32 LE][version_hash: bytes]
    if payload.len() < 4 {
        warn!("Session {} ClientVersion payload too short", session_id);
        return;
    }

    let mut cursor = Cursor::new(payload);
    if let Ok(hash_len) = ReadBytesExt::read_i32::<LittleEndian>(&mut cursor) {
        if !(0..=256).contains(&hash_len) || payload.len() < 4 + hash_len as usize {
            warn!(
                "Session {} ClientVersion invalid hash length: {}",
                session_id, hash_len
            );
            return;
        }
    }

    debug!("ClientVersion from session {}", session_id);
    let response = build_packet_bytes(ServerPacketIds::ClientVersion as i16, &[1u8]); // accepted
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: response,
        })
        .try_send();
}

impl GateActor {
    /// 处理新账号注册（对齐 C# Envir.NewAccount：Result 0-8）
    async fn handle_new_account(
        &mut self,
        gate_ref: &ActorRef<GateActor>,
        session_id: SessionId,
        payload: &[u8],
    ) {
        debug!("NewAccount request from session {}", session_id);

        let send_result = |result: u8| async move {
            let response = build_packet_bytes(ServerPacketIds::NewAccount as i16, &[result]);
            let _ = gate_ref
                .tell(SendToClient {
                    session_id,
                    data: response,
                })
                .try_send();
        };

        // C# Settings.AllowNewAccount → Result=0
        let allow = if let Some(s) = &self.social_ref {
            s.ask(crate::actors::social::NpcGetAllowNewAccount)
                .await
                .unwrap_or(true)
        } else {
            true
        };
        if !allow {
            send_result(0).await;
            return;
        }

        // C# IP 限流：每小时 >2 个账号 → 封 IP 24h → Result=0
        let now = gate_unix_now_secs();
        let ip = self
            .session_ips
            .get(&session_id)
            .cloned()
            .unwrap_or_default();
        if !ip.is_empty() {
            if self.ip_blocks.get(&ip).map(|&u| u > now).unwrap_or(false) {
                send_result(0).await;
                return;
            }
            let made = self.ip_accounts_made.entry(ip.clone()).or_default();
            if made.len() > 2 {
                self.ip_blocks.insert(ip.clone(), now + 24 * 3600);
                made.clear();
                send_result(0).await;
                return;
            }
            made.push(now);
            made.retain(|&t| t + 3600 >= now);
        }

        // 解析包
        let Ok(packet) = mir2_shared::packets::client::account::NewAccount::read_body(
            &mut std::io::Cursor::new(payload),
        ) else {
            warn!("NewAccount: parse failed session={}", session_id);
            return;
        };

        // C# AccountIDReg / PasswordReg 格式校验
        if !crate::util::validation::validate_username(&packet.account_id) {
            send_result(1).await;
            return;
        }
        if !crate::util::validation::validate_password(&packet.password) {
            send_result(2).await;
            return;
        }
        // 邮箱：非空时需合法且 <=50（C# EMailReg）
        let email_ok = packet.email_address.trim().is_empty()
            || (packet.email_address.len() <= 50
                && packet.email_address.contains('@')
                && packet.email_address.contains('.'));
        if !email_ok {
            send_result(3).await;
            return;
        }
        if packet.user_name.len() > 20 {
            send_result(4).await;
            return;
        }
        if packet.secret_question.len() > 30 {
            send_result(5).await;
            return;
        }
        if packet.secret_answer.len() > 30 {
            send_result(6).await;
            return;
        }

        // 真正注册（C#：已存在 → Result=7；成功 → Result=8）
        let created = if let Some(account_ref) = &self.account_ref {
            account_ref
                .ask(crate::actors::account::RegisterAccountRequest {
                    username: packet.account_id,
                    password: packet.password,
                })
                .await
                .unwrap_or(false)
        } else {
            false
        };
        if created {
            send_result(8).await;
        } else {
            send_result(7).await;
        }
    }
}

/// 解析登录包：account_id (DotNetString) + password (DotNetString)
fn parse_login_payload(payload: &[u8]) -> Option<(String, String)> {
    use mir2_shared::binary::read_dotnet_string;
    use std::io::Cursor;

    let mut cursor = Cursor::new(payload);
    match (
        read_dotnet_string(&mut cursor),
        read_dotnet_string(&mut cursor),
    ) {
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
        })
        .try_send();
}

/// 解析 DotNetString: [length: i32 LE][bytes...]
fn parse_dotnet_string(data: &[u8]) -> String {
    use mir2_shared::binary::read_dotnet_string;
    use std::io::Cursor;
    let mut cursor = Cursor::new(data);
    match read_dotnet_string(&mut cursor) {
        Ok(s) => s,
        Err(e) => {
            warn!(
                "parse_dotnet_string: malformed input ({e:?}), data len={}",
                data.len()
            );
            String::new()
        }
    }
}

/// 解析聊天包：DotNetString message + i32 linked_items_count
/// 解析聊天包：DotNetString message + i32 linked_items_count + ChatItem...（C# C.Chat）
fn parse_chat_packet(payload: &[u8]) -> Option<(String, Vec<mir2_shared::data::item::ChatItem>)> {
    let mut cursor = std::io::Cursor::new(payload);
    let packet = mir2_shared::packets::client::Chat::read_body(&mut cursor).ok()?;
    Some((packet.message, packet.linked_items))
}

/// 解析 CallNPC 包：[object_id: u32 LE][key: DotNetString]
fn parse_call_npc_payload(payload: &[u8]) -> Option<(u32, String)> {
    use byteorder::{LittleEndian, ReadBytesExt};
    use mir2_shared::binary::read_dotnet_string;
    use std::io::Cursor;

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
    if payload.len() < 9 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let grid = payload[0];
    let from = i32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[5..9].try_into().unwrap_or([0; 4]));
    let _ = world_ref
        .tell(crate::actors::world::MoveItemRequest {
            session_id,
            grid,
            from,
            to,
        })
        .try_send();
}

fn forward_use_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let _ = world_ref
        .tell(crate::actors::world::UseItemRequest {
            session_id,
            unique_id: uid,
        })
        .try_send();
}

fn forward_equip_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 13 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    // C# C.EquipItem.To = int（4 字节）；客户端双击装备恒发 0，服务端按物品类型自动判定
    let slot = i32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let _ = world_ref
        .tell(crate::actors::world::EquipItemRequest {
            session_id,
            grid,
            unique_id: uid,
            slot,
        })
        .try_send();
}

fn forward_delete_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 11 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let hero = payload[10] != 0;
    let _ = world_ref
        .tell(crate::actors::world::DeleteItemRequest {
            session_id,
            unique_id: uid,
            count,
            hero,
        })
        .try_send();
}

fn forward_remove_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let _ = world_ref
        .tell(crate::actors::world::RemoveItemRequest {
            session_id,
            grid,
            unique_id: uid,
        })
        .try_send();
}

fn forward_drop_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    // C# C.DropItem：uid u64 + count u16 + hero bool = 11 字节；SharedRust/客户端：count u32 + hero u8 = 13 字节
    // 兼容两者：count 取低 2 字节（实际堆叠 < 65536），hero 标志取 count 高字节或第 12 字节
    if payload.len() < 13 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let _hero_inv = payload[10] != 0 || payload[12] != 0;
    let _ = world_ref
        .tell(crate::actors::world::DropItemRequest {
            session_id,
            unique_id: uid,
            count,
        })
        .try_send();
}

fn forward_merge_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 18 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let grid_from = payload[0];
    let grid_to = payload[1];
    let from_uid = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let to_uid = u64::from_le_bytes(payload[10..18].try_into().unwrap_or([0; 8]));
    let _ = world_ref
        .tell(crate::actors::world::MergeItemRequest {
            session_id,
            grid_from,
            grid_to,
            from_uid,
            to_uid,
        })
        .try_send();
}

fn forward_split_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 13 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let _ = world_ref
        .tell(crate::actors::world::SplitItemRequest {
            session_id,
            grid,
            unique_id: uid,
            count,
        })
        .try_send();
}

/// BuyItem: [item_index: u64][count: u16][panel_type: u8]（C# 协议，无 npc_id）
fn forward_buy_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 11 {
        warn!("BuyItem payload too short: {}", payload.len());
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let item_index = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let _panel_type = payload[10];
    debug!(
        "BuyItem session={} item_index={} count={}",
        session_id, item_index, count
    );
    let _ = world_ref
        .tell(crate::actors::world::BuyItemRequest {
            session_id,
            item_index,
            count: count as u32,
        })
        .try_send();
}

/// SellItem: [uid: u64][count: u16]（C# 协议）
fn forward_sell_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let _ = world_ref
        .tell(crate::actors::world::SellItemRequest {
            session_id,
            unique_id: uid,
            count: count as u32,
        })
        .try_send();
}

fn forward_repair_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
    special: bool,
) {
    if payload.len() < 8 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let _ = world_ref
        .tell(crate::actors::world::RepairItemRequest {
            session_id,
            unique_id: uid,
            special,
        })
        .try_send();
}

/// RangeAttack: [dir: u8][x: i32][y: i32][target_id: u32][tx: i32][ty: i32] = 21 字节
/// 解析成功返回 (dir, target_id, target_x, target_y)；长度不足返回 None（不 panic）
pub fn parse_range_attack_payload(payload: &[u8]) -> Option<(u8, u32, i32, i32)> {
    // 阻断6：协议全长 21 字节（1+4+4+4+4+4），旧检查 <19 导致 19/20 字节载荷
    // 在 payload[17..21] 处越界 panic
    if payload.len() < 21 {
        return None;
    }
    let dir = payload[0];
    let target_id = u32::from_le_bytes(payload[9..13].try_into().ok()?);
    let target_x = i32::from_le_bytes(payload[13..17].try_into().ok()?);
    let target_y = i32::from_le_bytes(payload[17..21].try_into().ok()?);
    Some((dir, target_id, target_x, target_y))
}

fn forward_range_attack(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let Some((dir, target_id, target_x, target_y)) = parse_range_attack_payload(payload) else {
        return;
    };
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    debug!(
        "RangeAttack: session={} dir={} target={} pos=({}, {})",
        session_id, dir, target_id, target_x, target_y
    );
    let _ = world_ref
        .tell(crate::actors::world::RangeAttackRequest {
            session_id,
            direction: dir,
            target_id,
            target_x,
            target_y,
        })
        .try_send();
}

/// Magic: [spell: u8][dir: u8][target_id: u32][x: i32][y: i32]
fn forward_magic(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    // #2573：C# C.Magic wire（ClientPackets.cs:1122-1130）
    // [ObjectID u32][Spell u8][Direction u8][TargetID u32][X i32][Y i32][SpellTargetLock u8] = 19B
    if payload.len() < 19 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let object_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let spell = payload[4];
    let dir = payload[5];
    let target_id = u32::from_le_bytes(payload[6..10].try_into().unwrap_or([0; 4]));
    let target_x = i32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    let target_y = i32::from_le_bytes(payload[14..18].try_into().unwrap_or([0; 4]));
    let spell_target_lock = payload[18] != 0;
    debug!(
        "Magic: session={} object={} spell={} dir={} target={} pos=({}, {}) lock={}",
        session_id, object_id, spell, dir, target_id, target_x, target_y, spell_target_lock
    );
    let _ = world_ref
        .tell(crate::actors::world::MagicRequest {
            session_id,
            direction: dir,
            spell,
            target_id,
            target_x,
            target_y,
            object_id,
            spell_target_lock,
        })
        .try_send();
}

/// Harvest: [dir: u8] — 采集/挖矿请求
fn forward_harvest(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let dir = payload[0];
    debug!("Harvest: session={} dir={}", session_id, dir);
    let _ = world_ref
        .tell(crate::actors::world::HarvestRequest {
            session_id,
            direction: dir,
        })
        .try_send();
}

// ============================================================================
// NPC 商店 / 合成 handlers
// ============================================================================

/// CraftItem: [unique_id: u64][count: u16][slots_len: i32][slots: i32×N]
/// （C# ClientPackets.CraftItem wire；#2573 修——此前只取前 4 字节丢 Count+Slots）
fn forward_craft_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 14 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let slots_len =
        i32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4])).max(0) as usize;
    // 防御：slots 超长包截断（正常 ≤ 背包 46 格）
    let slots: Vec<i32> = payload[14..]
        .chunks_exact(4)
        .take(slots_len.min(64))
        .map(|c| i32::from_le_bytes(c.try_into().unwrap_or([0; 4])))
        .collect();
    debug!(
        "CraftItem: session={} unique={} count={} slots={}",
        session_id,
        unique_id,
        count,
        slots.len()
    );
    let _ = world_ref
        .tell(crate::actors::world::CraftItemRequest {
            session_id,
            unique_id,
            count,
            slots,
        })
        .try_send();
}

/// BuyItemBack (回购): [unique_id: u64][count: u16]（C# ClientPackets.BuyItemBack wire）
fn forward_buy_item_back(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    debug!(
        "BuyItemBack: session={} uid={} count={}",
        session_id, unique_id, count
    );
    let _ = world_ref
        .tell(crate::actors::world::BuyItemBackRequest {
            session_id,
            unique_id,
            count: count as u32,
        })
        .try_send();
}

// ============================================================================
// 仓库 handlers
// ============================================================================

/// StoreItem (存入仓库): [from: i32][to: i32]（C# 协议，from=背包格 to=仓库格）
fn handle_store_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        debug!("StoreItem: session={} payload too short", session_id);
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("StoreItem: session={} from={} to={}", session_id, from, to);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::StoreItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
}

/// TakeBackItem (从仓库取出): [from: i32][to: i32]（C# 协议，from=仓库格 to=背包格）
fn handle_take_back_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        debug!("TakeBackItem: session={} payload too short", session_id);
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "TakeBackItem: session={} from={} to={}",
        session_id, from, to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::TakeBackItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
}

// ============================================================================
// 其他常用 handlers
// ============================================================================

/// DropGold (丢弃/设置金币): [amount: u32]
fn handle_drop_gold(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        debug!("DropGold: session={} payload too short", session_id);
        return;
    }
    let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("DropGold: session={} amount={}", session_id, amount);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::DropGoldRequest { session_id, amount })
        .try_send();
}

/// Inspect (查看玩家): [target_id: u32]
/// Inspect (查看玩家): [target_id: u32][ranking: u8][name dotnet]
fn handle_inspect(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 5 {
        debug!("Inspect: session={} payload too short", session_id);
        return;
    }
    let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let ranking = payload[4] != 0;
    let mut name = String::new();
    if ranking && payload.len() > 5 {
        use mir2_shared::binary::read_dotnet_string;
        let mut cursor = std::io::Cursor::new(&payload[5..]);
        name = read_dotnet_string(&mut cursor).unwrap_or_default();
    }
    debug!(
        "Inspect: session={} target={} ranking={} name={}",
        session_id, target_id, ranking, name
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::InspectPlayerRequest {
            session_id,
            target_id,
            ranking,
            name,
        })
        .try_send();
}

/// ChangeAMode (切换攻击模式): [mode: u8]
fn forward_change_amode(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let mode = payload[0];
    debug!("ChangeAMode: session={} mode={}", session_id, mode);
    let mode = mir2_shared::enums::AttackMode::try_from(mode)
        .unwrap_or(mir2_shared::enums::AttackMode::Peace);
    let _ = world_ref
        .tell(crate::actors::world::ChangeAModeRequest { session_id, mode })
        .try_send();
}

/// ChangePMode (切换宠物模式): [mode: u8]
fn forward_change_pmode(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let mode = payload[0];
    debug!("ChangePMode: session={} mode={}", session_id, mode);
    let mode =
        mir2_shared::enums::PetMode::try_from(mode).unwrap_or(mir2_shared::enums::PetMode::Both);
    let _ = world_ref
        .tell(crate::actors::world::ChangePModeRequest { session_id, mode })
        .try_send();
}

/// MagicKey (设置快捷键): [spell: u8][key: u8][old_key: u8]
fn forward_magic_key(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 3 {
        return;
    }
    let spell = payload[0] as i32;
    let key = payload[1];
    let old_key = payload[2];
    debug!(
        "MagicKey: session={} spell={} key={} old_key={}",
        session_id, spell, key, old_key
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::SetSpellKeyRequest {
            session_id,
            spell,
            key,
            old_key,
        })
        .try_send();
}

/// RemoveSlotItem (移除插槽物品): [Grid:u8][GridTo:u8][UniqueID:u64][To:i32][FromUniqueID:u64]
fn forward_remove_slot_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 22 {
        return;
    }
    let grid = payload[0];
    let grid_to = payload[1];
    let unique_id = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let to = i32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    let from_unique_id = u64::from_le_bytes(payload[14..22].try_into().unwrap_or([0; 8]));
    debug!(
        "RemoveSlotItem: session={} grid={} grid_to={} uid={} to={} from_uid={}",
        session_id, grid, grid_to, unique_id, to, from_unique_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RemoveSlotItemRequest {
            session_id,
            grid,
            grid_to,
            unique_id,
            to,
            from_unique_id,
        })
        .try_send();
}

/// TeleportToNPC: [npc_id: u32]
fn forward_teleport_to_npc(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("TeleportToNPC: session={} npc={}", session_id, npc_id);
    let _ = world_ref
        .tell(crate::actors::world::TeleportToNPCRequest { session_id, npc_id })
        .try_send();
}

fn forward_town_revive(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
) {
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::TownReviveRequest { session_id })
        .try_send();
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
    if payload.len() < 2 {
        return;
    }
    let spell = payload[0] as i32;
    let can_use = payload[1] as i8;
    debug!(
        "SpellToggle: session={} spell={} can_use={}",
        session_id, spell, can_use
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::SpellToggleRequest {
            session_id,
            spell,
            can_use,
        })
        .try_send();
}

// ============================================================================
// 账号管理
// ============================================================================

/// ChangePassword: [account_id DotNetString][current_password DotNetString][new_password DotNetString]
/// （对齐 C# Envir.ChangePassword：Result 0=开关关闭 1=账号格式 2=当前密码格式 3=新密码格式；
/// 4=账号不存在 5=当前密码错误 6=成功 由 AccountActor 返回）
async fn forward_change_password(
    gate_ref: &ActorRef<GateActor>,
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    account_ref: &Option<ActorRef<crate::actors::account::AccountActor>>,
    session_usernames: &HashMap<SessionId, String>,
    pending_password_change: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    let mut cur = std::io::Cursor::new(payload);
    let (account_id, old_password, new_password) = match (
        mir2_shared::binary::read_dotnet_string(&mut cur),
        mir2_shared::binary::read_dotnet_string(&mut cur),
        mir2_shared::binary::read_dotnet_string(&mut cur),
    ) {
        (Ok(a), Ok(o), Ok(n)) => (a, o, n),
        _ => {
            warn!("ChangePassword: parse failed session={}", session_id);
            return;
        }
    };

    // #11：登录态才受理（对齐 C# MirConnection.ChangePassword 的 Stage 检查），
    // 防止任意连接爆破他人账号旧密码（Result=5 构成口令预言机）。
    // 唯一例外：登录返回 Result=5（RequirePasswordChange）的会话，
    // 允许未登录态改密、且只能改登记的账号。
    // 跨账号喷洒加固：登录态会话只能改【本会话登录的账号】——旧代码登录后放行
    // 任意 account_id，构成对已登录他人账号旧密码的在线爆破/锁定喷洒面
    let session_account = session_usernames.get(&session_id).cloned();
    let authorized = match &session_account {
        Some(logged_in) => logged_in == &account_id,
        None => pending_password_change
            .get(&session_id)
            .map(|u| u == &account_id)
            .unwrap_or(false),
    };
    if !authorized {
        warn!(
            "ChangePassword rejected: session {} not authorized for account={}",
            session_id, account_id
        );
        return;
    }

    let send_result = |result: u8| async move {
        let packet = mir2_shared::packets::server::login::ChangePassword { result };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let data = build_packet_bytes(ServerPacketIds::ChangePassword as i16, &body);
            let _ = gate_ref.tell(SendToClient { session_id, data }).try_send();
        }
    };

    // C# Settings.AllowChangePassword → Result=0
    let allow = if let Some(s) = social_ref {
        s.ask(crate::actors::social::NpcGetAllowChangePassword)
            .await
            .unwrap_or(true)
    } else {
        true
    };
    if !allow {
        send_result(0).await;
        return;
    }
    // C# AccountIDReg / PasswordReg 格式校验
    if !crate::util::validation::validate_username(&account_id) {
        send_result(1).await;
        return;
    }
    if !crate::util::validation::validate_password(&old_password) {
        send_result(2).await;
        return;
    }
    if !crate::util::validation::validate_password(&new_password) {
        send_result(3).await;
        return;
    }

    if let Some(account_ref) = account_ref {
        let _ = account_ref
            .tell(crate::actors::account::AccountChangePassword {
                session_id,
                username: account_id,
                old_password,
                new_password,
                // 纵深防御：AccountActor 侧再校验一次「登录态只能改本会话账号」；
                // None = 强制改密待办例外路径（账号名已在上方比对登记值）
                session_account,
            })
            .try_send();
    } else {
        warn!(
            "ChangePassword: account_ref not available for session={}",
            session_id
        );
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
    if payload.is_empty() {
        return;
    }
    // Parse first DotNetString (current), then second (new)
    let (current, rest) = {
        use mir2_shared::binary::read_dotnet_string;
        use std::io::Cursor;
        let mut c = Cursor::new(payload);
        let s = read_dotnet_string(&mut c).unwrap_or_default();
        let pos = c.position() as usize;
        (s, &payload[pos..])
    };
    let new = parse_dotnet_string(rest);
    if let Some(username) = session_usernames.get(&session_id) {
        if let Some(account_ref) = account_ref {
            debug!(
                "SetStoragePassword: session={} user={} new_len={}",
                session_id,
                username,
                new.len()
            );
            let _ = account_ref
                .tell(crate::actors::account::SetStoragePasswordRequest {
                    session_id,
                    username: username.clone(),
                    current_raw: current,
                    new_raw: new,
                })
                .try_send();
        } else {
            warn!(
                "SetStoragePassword: account_ref not available for session={}",
                session_id
            );
        }
    } else {
        warn!(
            "SetStoragePassword: no username mapping for session={}",
            session_id
        );
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
            debug!(
                "RemoveStoragePassword: session={} user={}",
                session_id, username
            );
            let _ = account_ref
                .tell(crate::actors::account::ClearStoragePasswordRequest {
                    session_id,
                    username: username.clone(),
                    current_raw: current,
                })
                .try_send();
        } else {
            warn!(
                "RemoveStoragePassword: account_ref not available for session={}",
                session_id
            );
        }
    } else {
        warn!(
            "RemoveStoragePassword: no username mapping for session={}",
            session_id
        );
    }
}

/// NewCharacter: [name: DotNetString(7bit)][gender: u8][class: u8]（对齐 C# ClientPackets.NewCharacter）
async fn forward_new_character(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_usernames: &HashMap<SessionId, String>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
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
        warn!(
            "NewCharacter rejected: invalid name '{}' from session {}",
            name, session_id
        );
        return;
    }
    debug!(
        "NewCharacter: session={} name={} class={} gender={} hair={}",
        session_id, name, class, gender, hair
    );
    // #10：未登录会话不得建角（match 臂已拦截并断连，此处纵深防御），
    // 绝不允许用角色名冒充账号名
    let Some(account_username) = session_usernames.get(&session_id).cloned() else {
        warn!(
            "NewCharacter rejected: no account mapping for session={}",
            session_id
        );
        return;
    };
    let req = crate::actors::world::NewCharacterRequest {
        session_id,
        name,
        class,
        gender,
        hair,
        account_username,
    };
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
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!(
        "DeleteCharacter: session={} index={}",
        session_id, character_index
    );
    let account_username = session_usernames
        .get(&session_id)
        .cloned()
        .unwrap_or_default();
    let _ = world_ref
        .tell(crate::actors::world::DeleteCharacterRequest {
            session_id,
            character_index,
            account_username,
        })
        .try_send();
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
    if payload.is_empty() {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let allow_group = payload[0] != 0;
    debug!("SwitchGroup: session={} allow={}", session_id, allow_group);
    let _ = social_ref
        .tell(crate::actors::social::SwitchGroupRequest {
            session_id,
            allow_group,
        })
        .try_send();
}

/// AddMember: [name: string] (DotNet string format)
fn forward_add_member(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    debug!("AddMember: session={} name={}", session_id, name);
    let _ = social_ref
        .tell(crate::actors::social::GroupInviteRequest {
            session_id,
            target_name: name,
        })
        .try_send();
}

/// GroupInvite: [accept_invite: bool] (1 byte) - 邀请回复
fn forward_group_invite(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let accept = payload[0] != 0;
    debug!(
        "GroupInvite reply: session={} accept={}",
        session_id, accept
    );
    let _ = social_ref
        .tell(crate::actors::social::GroupInviteReply {
            session_id,
            inviter_id: 0,
            accept,
        })
        .try_send();
}

/// DellMember: [name: string] (DotNet string format)
fn forward_dell_member(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    debug!("DellMember: session={} name={}", session_id, name);
    let _ = social_ref
        .tell(crate::actors::social::DellMemberRequest {
            session_id,
            member_name: name,
        })
        .try_send();
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
    if payload.is_empty() {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let Ok(packet) =
        mir2_shared::packets::client::hero::NewHero::read_body(&mut std::io::Cursor::new(payload))
    else {
        warn!(
            "NewHero: 解析失败 session={} len={}",
            session_id,
            payload.len()
        );
        return;
    };
    debug!(
        "NewHero: session={} name={} gender={:?} class={:?}",
        session_id, packet.name, packet.gender, packet.class
    );
    let _ = world_ref
        .tell(crate::actors::world::NewHeroRequest {
            session_id,
            name: packet.name,
            gender: packet.gender,
            class: packet.class,
        })
        .try_send();
}

/// SetHeroBehaviour: [behaviour: u8]
fn forward_set_hero_behaviour(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let behaviour = payload[0];
    debug!(
        "SetHeroBehaviour: session={} behaviour={}",
        session_id, behaviour
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::SetHeroBehaviourRequest {
            session_id,
            behaviour,
        })
        .try_send();
}

/// SetAutoPotValue: [stat: u8][value: u32]
fn forward_set_autopot_value(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 5 {
        return;
    }
    let stat = payload[0];
    let value = u32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    debug!(
        "SetAutoPotValue: session={} stat={} value={}",
        session_id, stat, value
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::SetAutoPotValueRequest {
            session_id,
            stat,
            value,
        })
        .try_send();
}

/// SetAutoPotItem: [grid: u8][item_index: i32]
fn forward_set_autopot_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 5 {
        return;
    }
    let grid = payload[0];
    let item_index = i32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    debug!(
        "SetAutoPotItem: session={} grid={} item_index={}",
        session_id, grid, item_index
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::SetAutoPotItemRequest {
            session_id,
            grid,
            item_index,
        })
        .try_send();
}

/// ChangeHero: [hero_index: u8]
fn handle_change_hero(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let hero_index = payload[0];
    debug!("ChangeHero: session={} index={}", session_id, hero_index);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ChangeHeroRequest {
            session_id,
            hero_index,
        })
        .try_send();
}

/// ReviveHero：英雄一键复活（#1216，空 body）
fn handle_revive_hero(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let _ = payload;
    debug!("ReviveHero: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ReviveHeroRequest { session_id })
        .try_send();
}

/// TakeBackHeroItem: C# [from i32][to i32]（英雄格 → 主背包格，#203）
fn handle_take_back_hero_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "TakeBackHeroItem: session={} from={} to={}",
        session_id, from, to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::TakeBackHeroItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
}

/// TransferHeroItem: C# [from i32][to i32]（主背包格 → 英雄格，#203）
fn handle_transfer_hero_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "TransferHeroItem: session={} from={} to={}",
        session_id, from, to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::TransferHeroItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
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
    if payload.len() < 9 {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let is_add = payload[0] != 0;
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let grid = if payload.len() >= 10 { payload[9] } else { 0 };
    let count = if payload.len() >= 12 {
        u16::from_le_bytes(payload[10..12].try_into().unwrap_or([0; 2]))
    } else {
        1
    };

    if is_add {
        let _ = social_ref
            .tell(crate::actors::social::TradeAddItem {
                session_id,
                unique_id: uid,
                grid,
                count,
            })
            .try_send();
    } else {
        let _ = social_ref
            .tell(crate::actors::social::TradeRemoveItem {
                session_id,
                unique_id: uid,
            })
            .try_send();
    }
}

/// TradeRequest: 发起交易
fn forward_trade_request(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    if social_ref.is_none() {
        return;
    }
    // 注意：必须用 tell（ask 的 future 被丢弃时消息不会发出）
    let _ = social_ref
        .as_ref()
        .unwrap()
        .tell(crate::actors::social::TradeStartRequest { session_id })
        .try_send();
}

/// TradeReply: [accept: bool]
fn forward_trade_reply(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let accept = payload[0] != 0;
    let _ = social_ref
        .tell(crate::actors::social::TradeStartReply { session_id, accept })
        .try_send();
}

/// TradeConfirm: [locked: bool]
fn forward_trade_confirm(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let locked = payload[0] != 0;
    let _ = social_ref
        .tell(crate::actors::social::TradeConfirmLock { session_id, locked })
        .try_send();
}

/// TradeCancel: 取消交易
fn forward_trade_cancel(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::TradeCancel { session_id })
        .try_send();
}

/// TradeGold: [amount: u32]
fn forward_trade_gold(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let _ = social_ref
        .tell(crate::actors::social::TradeAddGold { session_id, amount })
        .try_send();
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
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串，随后 1 字节 blocked
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    let mut blocked_buf = [0u8; 1];
    let blocked =
        std::io::Read::read_exact(&mut cur, &mut blocked_buf).is_ok() && blocked_buf[0] != 0;
    debug!(
        "AddFriend: session={} name={} blocked={}",
        session_id, name, blocked
    );
    let _ = social_ref
        .tell(crate::actors::social::AddFriendRequest {
            session_id,
            friend_name: name,
            blocked,
        })
        .try_send();
}

/// RemoveFriend: [character_index: i32]
fn forward_remove_friend(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!(
        "RemoveFriend: session={} char_idx={}",
        session_id, character_index
    );
    let _ = social_ref
        .tell(crate::actors::social::RemoveFriendRequest {
            session_id,
            friend_object_id: character_index as u32,
        })
        .try_send();
}

/// RefreshFriends: no payload
fn forward_refresh_friends(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
) {
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::RefreshFriendsRequest { session_id })
        .try_send();
}

/// AddMemo: [character_index: i32][memo: DotNetString]
fn forward_add_memo(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let memo = if payload.len() > 4 {
        String::from_utf8_lossy(&payload[4..]).to_string()
    } else {
        String::new()
    };
    debug!(
        "AddMemo: session={} char_idx={}",
        session_id, character_index
    );
    let _ = social_ref
        .tell(crate::actors::social::AddMemoRequest {
            session_id,
            friend_object_id: character_index as u32,
            memo,
        })
        .try_send();
}

// ============================================================================
// 邮件系统
// ============================================================================

/// SendMail: [name: DotNetString][message: DotNetString][gold: u32][items: 5*u64][stamped: bool]（C#/SharedRust wire）
fn handle_send_mail(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let mut cur = std::io::Cursor::new(payload);

    // 收件人 + 正文（DotNet 7-bit 字符串；subject 由正文首行派生，C# 语义）
    let Ok(receiver_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    let Ok(message) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    let mut gold_buf = [0u8; 4];
    if std::io::Read::read_exact(&mut cur, &mut gold_buf).is_err() {
        return;
    }
    let gold = u32::from_le_bytes(gold_buf);
    let mut item_uids = Vec::new();
    for _ in 0..5 {
        let mut uid_buf = [0u8; 8];
        if std::io::Read::read_exact(&mut cur, &mut uid_buf).is_err() {
            return;
        }
        let uid = u64::from_le_bytes(uid_buf);
        if uid != 0 {
            item_uids.push(uid);
        }
    }
    let mut stamped_buf = [0u8; 1];
    let stamped = if std::io::Read::read_exact(&mut cur, &mut stamped_buf).is_ok() {
        stamped_buf[0] != 0
    } else {
        false
    };

    let subject = message.lines().next().unwrap_or("").to_string();
    debug!(
        "SendMail: session={} to={} subject={} gold={} items={} stamped={}",
        session_id,
        receiver_name,
        subject,
        gold,
        item_uids.len(),
        stamped
    );
    let _ = world_ref
        .tell(crate::actors::world::SendMailRequest {
            session_id,
            receiver_name,
            subject,
            body: message,
            gold,
            item_uids,
            stamped,
        })
        .try_send();
}

/// ReadMail: [mail_id: u64]
fn handle_read_mail(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("ReadMail: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ReadMailRequest {
            session_id,
            mail_id,
        })
        .try_send();
}

/// CollectParcel: [mail_id: u64]
fn handle_collect_parcel(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("CollectParcel: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::CollectParcelRequest {
            session_id,
            mail_id,
        })
        .try_send();
}

/// DeleteMail: [mail_id: u64]
fn handle_delete_mail(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DeleteMail: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::DeleteMailRequest {
            session_id,
            mail_id,
        })
        .try_send();
}

// ============================================================================
// 行会系统
// ============================================================================

/// GuildInvite: [accept: bool] - 行会邀请回复
fn handle_guild_invite(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let accept = payload[0] != 0;
    debug!("GuildInvite: session={} accept={}", session_id, accept);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::GuildInviteReply { session_id, accept })
        .try_send();
}

/// RequestGuildInfo: [info_type: u8]
fn handle_request_guild_info(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let info_type = payload.first().copied().unwrap_or(0);
    debug!(
        "RequestGuildInfo: session={} type={}",
        session_id, info_type
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::RequestGuildInfo {
            session_id,
            info_type,
        })
        .try_send();
}

/// EditGuildMember: [change_type: u8][rank_index: u8][name: DotNetString][rank_name: DotNetString]
fn handle_edit_guild_member(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 {
        return;
    }
    let change_type = payload[0];
    // C#/SharedRust：[change_type u8][rank_index u8][name DotNet][rank_name DotNet]
    let rank_index = payload[1];
    let mut cur = std::io::Cursor::new(&payload[2..]);
    let Ok(member_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    let rank_name = mir2_shared::binary::read_dotnet_string(&mut cur).unwrap_or_default();
    debug!(
        "EditGuildMember: session={} type={} name={} rank_index={} rank_name={}",
        session_id, change_type, member_name, rank_index, rank_name
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::EditGuildMemberRequest {
            session_id,
            change_type,
            member_name,
            rank_index,
            rank_name,
        })
        .try_send();
}

/// EditGuildNotice: [count: i32][line1: DotNetString][line2: DotNetString]...
fn handle_edit_guild_notice(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    // C#/SharedRust：[count i32][lines DotNet...]
    let count = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) as usize;
    let mut notice_lines = Vec::new();
    let mut cur = std::io::Cursor::new(&payload[4..]);
    for _ in 0..count {
        match mir2_shared::binary::read_dotnet_string(&mut cur) {
            Ok(line) => notice_lines.push(line),
            Err(_) => break,
        }
    }
    debug!(
        "EditGuildNotice: session={} lines={}",
        session_id,
        notice_lines.len()
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::EditGuildNoticeRequest {
            session_id,
            notice: notice_lines,
        })
        .try_send();
}

/// GuildNameReturn: [name: DotNetString]
fn handle_guild_name_return(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    // C#/SharedRust：name 是 DotNet 7-bit 编码字符串
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    debug!("GuildNameReturn: session={} name={}", session_id, name);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::CreateGuildRequest {
            session_id,
            guild_name: name,
        })
        .try_send();
}

/// GuildStorageGoldChange: [change_type: u8][amount: u32]
fn handle_guild_storage_gold(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 5 {
        return;
    }
    let change_type = payload[0];
    let amount = u32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    debug!(
        "GuildStorageGoldChange: session={} type={} amount={}",
        session_id, change_type, amount
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::GuildStorageGoldChangeRequest {
            session_id,
            change_type,
            amount,
        })
        .try_send();
}

/// GuildStorageItemChange: [change_type: u8][grid: u8][unique_id: u64][count: u32]
fn handle_guild_storage_item(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 14 {
        return;
    }
    let change_type = payload[0];
    let grid = payload[1];
    let uid = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    debug!(
        "GuildStorageItemChange: session={} type={} grid={} uid={} count={}",
        session_id, change_type, grid, uid, count
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::GuildStorageItemChangeRequest {
            session_id,
            change_type,
            grid,
            unique_id: uid,
            count,
        })
        .try_send();
}

// ============================================================================
// 婚姻系统
// ============================================================================

/// MarriageRequest: [target_name: DotNet 7-bit string]（C# BinaryWriter.Write(string)）
fn handle_marriage_request(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(target_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    debug!("MarriageRequest: session={} to={}", session_id, target_name);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::MarriageRequest {
            session_id,
            target_name,
        })
        .try_send();
}

/// MarriageReply: [accept: bool]
fn handle_marriage_reply(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let accept = payload[0] != 0;
    debug!("MarriageReply: session={} accept={}", session_id, accept);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::MarriageReply { session_id, accept })
        .try_send();
}

/// ChangeMarriage: no payload or minimal
fn handle_change_marriage(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("ChangeMarriage: session={}", session_id);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::SocialChangeMarriage { session_id })
        .try_send();
}

/// DivorceRequest: [partner_name: DotNet 7-bit string]
fn handle_divorce_request(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(partner_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    debug!(
        "DivorceRequest: session={} partner={}",
        session_id, partner_name
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::SocialDivorceRequest {
            session_id,
            partner_name,
        })
        .try_send();
}

/// DivorceReply: [accept: bool]
fn handle_divorce_reply(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let accept = payload[0] != 0;
    debug!("DivorceReply: session={} accept={}", session_id, accept);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::SocialDivorceReply { session_id, accept })
        .try_send();
}

/// AddMentor: [mentor_name: DotNet 7-bit string]（C# BinaryWriter.Write(string)）
fn handle_add_mentor(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(mentor_name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    debug!("AddMentor: session={} mentor={}", session_id, mentor_name);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::SocialAddMentor {
            session_id,
            mentor_name,
        })
        .try_send();
}

/// MentorReply: [accept: bool]
fn handle_mentor_reply(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let accept = payload[0] != 0;
    debug!("MentorReply: session={} accept={}", session_id, accept);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::SocialMentorReply { session_id, accept })
        .try_send();
}

/// AllowMentor: [allow: bool]
fn handle_allow_mentor(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let allow = payload[0] != 0;
    debug!("AllowMentor: session={} allow={}", session_id, allow);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::SocialAllowMentor { session_id, allow })
        .try_send();
}

/// CancelMentor
fn handle_cancel_mentor(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("CancelMentor: session={}", session_id);
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::SocialCancelMentor {
            session_id,
            force: true,
        })
        .try_send();
}

// ============================================================================
// 宠物系统
// ============================================================================

/// UpdateIntelligentCreature: [creature_type: u8][pickup_mode: u8][custom_name: DotNetString?]
/// UpdateIntelligentCreature: [type u8][pet_mode u8][custom_name dotnet][summon u8][unsummon u8][release u8]
/// UpdateIntelligentCreature: [type u8][pet_mode u8][custom_name dotnet][summon u8][unsummon u8][release u8]
/// UpdateIntelligentCreature: [type u8][pet_mode u8][name dotnet][summon u8][unsummon u8][release u8][filter 9][grade u8][options_save u8]
fn handle_update_intelligent_creature(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        debug!(
            "UpdateIntelligentCreature: session={} payload too short",
            session_id
        );
        return;
    }
    let mut cur = std::io::Cursor::new(payload);
    let next_u8 = |cur: &mut std::io::Cursor<&[u8]>| {
        let mut b = [0u8; 1];
        if std::io::Read::read_exact(cur, &mut b).is_ok() {
            b[0]
        } else {
            0
        }
    };
    let creature_type = next_u8(&mut cur);
    let pet_mode = next_u8(&mut cur);
    use mir2_shared::binary::read_dotnet_string;
    let custom_name = read_dotnet_string(&mut cur).unwrap_or_default();
    let summon_me = next_u8(&mut cur) != 0;
    let unsummon_me = next_u8(&mut cur) != 0;
    let release_me = next_u8(&mut cur) != 0;
    let mut filter = [0u8; 9];
    for b in filter.iter_mut() {
        *b = next_u8(&mut cur);
    }
    let grade = next_u8(&mut cur);
    let options_save = next_u8(&mut cur) != 0;
    debug!(
        "UpdateIntelligentCreature: session={} type={} mode={} name={} summon={} unsummon={} release={} save={}",
        session_id, creature_type, pet_mode, custom_name, summon_me, unsummon_me, release_me, options_save
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::UpdateIntelligentCreature {
            session_id,
            creature_type,
            pet_mode,
            custom_name,
            summon_me,
            unsummon_me,
            release_me,
            filter,
            grade,
            options_save,
        })
        .try_send();
}

/// 解析 C.IntelligentCreaturePickup（SharedRust packets/client/misc.rs）：
/// `[mouse_mode: u8][x: i32][y: i32]`（9 字节）。
/// C# GameScene.cs:804/811：MouseMode=false 半自动、true 鼠标拾取。
fn parse_pet_pickup(payload: &[u8]) -> Option<(bool, i32, i32)> {
    if payload.len() < 9 {
        return None;
    }
    let mouse_mode = payload[0] != 0;
    let x = i32::from_le_bytes(payload[1..5].try_into().ok()?);
    let y = i32::from_le_bytes(payload[5..9].try_into().ok()?);
    Some((mouse_mode, x, y))
}

/// IntelligentCreaturePickup: [mouse_mode: u8][x: i32][y: i32]
fn handle_intelligent_creature_pickup(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let Some((mouse_mode, x, y)) = parse_pet_pickup(payload) else {
        return;
    };
    debug!(
        "IntelligentCreaturePickup: session={} mouse_mode={} x={} y={}",
        session_id, mouse_mode, x, y
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::IntelligentCreaturePickup {
            session_id,
            mouse_mode,
            x,
            y,
        })
        .try_send();
}

/// RequestIntelligentCreatureUpdates: [request_updates: bool]
fn handle_request_intelligent_creature_updates(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let request_updates = payload[0] != 0;
    debug!(
        "RequestIntelligentCreatureUpdates: session={} updates={}",
        session_id, request_updates
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RequestIntelligentCreatureUpdates {
            session_id,
            request_updates,
        })
        .try_send();
}

// ============================================================================
// 任务系统
// ============================================================================

/// AcceptQuest: [npc_index: i32][quest_index: i32]
fn handle_accept_quest(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let npc_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let quest_index = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "AcceptQuest: session={} npc={} quest={}",
        session_id, npc_index, quest_index
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::AcceptQuestRequest {
            session_id,
            npc_index,
            quest_index,
        })
        .try_send();
}

/// FinishQuest: [quest_index: i32][selected_item_index: i32]
fn handle_finish_quest(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let quest_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let selected_item_index = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("FinishQuest: session={} quest={}", session_id, quest_index);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::FinishQuestRequest {
            session_id,
            quest_index,
            selected_item_index,
        })
        .try_send();
}

/// AbandonQuest: [quest_index: i32]
fn handle_abandon_quest(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let quest_index = i32::from_le_bytes(payload[..4].try_into().unwrap_or([0; 4]));
    debug!("AbandonQuest: session={} quest={}", session_id, quest_index);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::AbandonQuestRequest {
            session_id,
            quest_index,
        })
        .try_send();
}

// ============================================================================
// 精炼系统
// ============================================================================

/// DepositRefineItem: [unique_id: u64]
/// DepositRefineItem: [from: i32][to: i32]（C# C.DepositRefineItem 槽位线格式）
fn handle_deposit_refine_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "DepositRefineItem: session={} from={} to={}",
        session_id, from, to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::DepositRefineItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
}

/// RetrieveRefineItem: [from: i32][to: i32]（C# C.RetrieveRefineItem 槽位线格式）
fn handle_retrieve_refine_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "RetrieveRefineItem: session={} from={} to={}",
        session_id, from, to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RetrieveRefineItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
}

/// RefineCancel: []
fn handle_refine_cancel(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("RefineCancel: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RefineCancelRequest { session_id })
        .try_send();
}

/// RefineItem: [unique_id: u64]（C# C.RefineItem 精炼栏物品 uid）
fn handle_refine_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let unique_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("RefineItem: session={} uid={}", session_id, unique_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RefineItemRequest {
            session_id,
            unique_id,
        })
        .try_send();
}

/// CheckRefine: [unique_id: u64]
fn handle_check_refine(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("CheckRefine: session={} uid={}", session_id, uid);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::CheckRefineRequest {
            session_id,
            unique_id: uid,
        })
        .try_send();
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
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let map_id = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestMapInfo: session={} map={}", session_id, map_id);
    let _ = world_ref
        .tell(crate::actors::world::RequestMapInfoRequest { session_id, map_id })
        .try_send();
}

/// PR #1126: Client requests detailed monster info (for tooltip).
/// Wire format: [monster_index: i32 LE]
fn forward_request_monster_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let monster_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!(
        "RequestMonsterInfo: session={} idx={}",
        session_id, monster_index
    );
    let _ = world_ref
        .tell(crate::actors::world::RequestMonsterInfoRequest {
            session_id,
            monster_index,
        })
        .try_send();
}

/// PR #1126: Client requests detailed NPC info (for tooltip).
/// Wire format: [npc_index: i32 LE]
fn forward_request_npc_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let npc_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestNPCInfo: session={} idx={}", session_id, npc_index);
    let _ = world_ref
        .tell(crate::actors::world::RequestNPCInfoRequest {
            session_id,
            npc_index,
        })
        .try_send();
}

/// PR #1126: Client requests detailed item info (for tooltip).
/// Wire format: [item_index: i32 LE]
/// (Returns nothing for now — ItemInfo stream will be wired in a later PR.)
fn forward_request_item_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let item_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestItemInfo: session={} idx={}", session_id, item_index);
    let _ = world_ref
        .tell(crate::actors::world::RequestItemInfoRequest {
            session_id,
            item_index,
        })
        .try_send();
}

/// SearchMap: [keyword: DotNetString]
fn forward_search_map(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 {
        return;
    }
    let world_ref = match world_ref {
        Some(w) => w,
        None => {
            return;
        }
    };
    let name_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + name_len {
        return;
    }
    let keyword = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    debug!("SearchMap: session={} keyword={}", session_id, keyword);
    let _ = world_ref
        .tell(crate::actors::world::SearchMapRequest {
            session_id,
            keyword,
        })
        .try_send();
}

/// Observe: [name: DotNetString]（C# C.Observe 目标玩家名）
fn forward_observe(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(name) = mir2_shared::binary::read_dotnet_string(&mut cur) else {
        return;
    };
    debug!("Observe: session={} target={}", session_id, name);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ObservePlayerRequest { session_id, name })
        .try_send();
}

// ============================================================================
// 其他
// ============================================================================

/// ReplaceWedRing: [unique_id: u64] — 更换结婚戒指
fn handle_replace_wed_ring(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("ReplaceWedRing: session={} uid={}", session_id, uid);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ReplaceWedRingRequest {
            session_id,
            unique_id: uid,
        })
        .try_send();
}

/// RequestUserName: [target_id: u32]
fn handle_request_user_name(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!(
        "RequestUserName: session={} target={}",
        session_id, target_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RequestUserNameMsg {
            session_id,
            object_id: target_id,
        })
        .try_send();
}

/// RequestChatItem: [unique_id: u64]
fn handle_request_chat_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("RequestChatItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RequestChatItemMsg {
            session_id,
            unique_id: uid,
        })
        .try_send();
}

// ============================================================================
// 剩余 opcode stub handlers（Phase 15：覆盖所有未处理的 opcode）
// ============================================================================

/// EquipSlotItem: [grid:u8][unique_id:u64][to_slot:i32][grid_to:u8] — 快捷装备栏装备
fn handle_equip_slot_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 14 {
        return;
    }
    let grid = payload[0];
    let unique_id = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let to_slot = i32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let grid_to = payload[13];
    debug!(
        "EquipSlotItem: session={} grid={} uid={} to_slot={} grid_to={}",
        session_id, grid, unique_id, to_slot, grid_to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::EquipSlotItemRequest {
            session_id,
            grid,
            unique_id,
            to_slot,
            grid_to,
        })
        .try_send();
}

/// ConsignItem: [unique_id u64][price u32][panel_type u8]（对齐 SharedRust ConsignItem/C#）
fn forward_consign_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 13 {
        return;
    }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let price = u32::from_le_bytes(payload[8..12].try_into().unwrap_or([0; 4]));
    let panel_type = payload[12];
    let market_type = if panel_type == mir2_shared::enums::MarketPanelType::Auction as u8 {
        1
    } else {
        0
    };
    debug!(
        "ConsignItem: session={} uid={} price={} type={}",
        session_id, unique_id, price, market_type
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ConsignItemRequest {
            session_id,
            unique_id,
            price: price as u64,
            market_type,
        })
        .try_send();
}

/// MarketSearch: [match: DotNetString][type: u8][usermode: bool][min_shape: i16][max_shape: i16][market_type: u8]（C# C.MarketSearch）
fn forward_market_search(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let mut cur = std::io::Cursor::new(payload);
    let Ok(packet) = mir2_shared::packets::client::market::MarketSearch::read_body(&mut cur) else {
        return;
    };
    // SharedRust 枚举 = C# + 3：转回 C# 原始值（0=不过滤）
    let item_type = (packet.item_type as u8).saturating_sub(3);
    let market_type = (packet.market_type as u8).saturating_sub(3);
    debug!(
        "MarketSearch: session={} kw={} type={} usermode={} shapes=[{},{}] mkt={}",
        session_id,
        packet.match_text,
        item_type,
        packet.user_mode,
        packet.min_shape,
        packet.max_shape,
        market_type
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::MarketSearchRequest {
            session_id,
            keyword: packet.match_text,
            item_type,
            user_mode: packet.user_mode,
            min_shape: packet.min_shape,
            max_shape: packet.max_shape,
            market_type,
        })
        .try_send();
}

/// MarketRefresh: []
fn forward_market_refresh(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("MarketRefresh: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::MarketRefreshRequest { session_id })
        .try_send();
}

/// MarketPage: [page: u32]
fn forward_market_page(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let page = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("MarketPage: session={} page={}", session_id, page);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::MarketPageRequest { session_id, page })
        .try_send();
}

/// MarketBuy: [auction_id u64][bid_price u32]（对齐 SharedRust MarketBuy/C#）
fn forward_market_buy(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 12 {
        return;
    }
    let listing_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let bid_price = u32::from_le_bytes(payload[8..12].try_into().unwrap_or([0; 4]));
    debug!(
        "MarketBuy: session={} listing={} bid={}",
        session_id, listing_id, bid_price
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::MarketBuyRequest {
            session_id,
            listing_id,
            count: 1,
            bid_price,
        })
        .try_send();
}

/// MarketGetBack: [mode: u8][auction_id: u64]（C# C.MarketGetBack 线格式）
fn forward_market_get_back(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 {
        return;
    }
    let mode = payload[0];
    let auction_id = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    debug!(
        "MarketGetBack: session={} mode={} auction={}",
        session_id, mode, auction_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::MarketGetBackRequest {
            session_id,
            mode,
            auction_id,
        })
        .try_send();
}

/// MarketSellNow: [auction_id: u64]（C# C.MarketSellNow 线格式，仅拍卖ID）
fn forward_market_sell_now(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let auction_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!(
        "MarketSellNow: session={} auction={}",
        session_id, auction_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::MarketSellNowRequest {
            session_id,
            auction_id,
        })
        .try_send();
}

/// FishingCast: [type: u8]
fn forward_fishing_cast(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let fishing_type = payload.first().copied().unwrap_or(0);
    debug!("FishingCast: session={} type={}", session_id, fishing_type);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::FishingCastRequest {
            session_id,
            fishing_type,
        })
        .try_send();
}

/// FishingChangeAutocast: [enabled: bool]
fn forward_fishing_change_autocast(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let enabled = payload.first().copied().unwrap_or(0) != 0;
    debug!(
        "FishingChangeAutocast: session={} enabled={}",
        session_id, enabled
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::FishingChangeAutocastRequest {
            session_id,
            enabled,
        })
        .try_send();
}

/// CombineItem: [grid: u8][id_from: u64][id_to: u64]（C# C.CombineItem 线格式）
fn forward_combine_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 17 {
        return;
    }
    let grid = payload[0];
    let id_from = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let id_to = u64::from_le_bytes(payload[9..17].try_into().unwrap_or([0; 8]));
    debug!(
        "CombineItem: session={} grid={} from={} to={}",
        session_id, grid, id_from, id_to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::CombineItemRequest {
            session_id,
            grid,
            id_from,
            id_to,
        })
        .try_send();
}

/// AwakeningNeedMaterials: [unique_id: u64][awake_type: u8]
fn forward_awakening_need_materials(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 {
        return;
    }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let awake_type = payload[8];
    debug!(
        "AwakeningNeedMaterials: session={} uid={}",
        session_id, unique_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::AwakeningNeedMaterialsRequest {
            session_id,
            unique_id,
            awake_type,
        })
        .try_send();
}

/// AwakeningLockedItem: [unique_id: u64][locked: u8]
fn forward_awakening_locked_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 {
        return;
    }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let locked = payload[8] != 0;
    debug!(
        "AwakeningLockedItem: session={} uid={} locked={}",
        session_id, unique_id, locked
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::AwakeningLockedItemRequest {
            session_id,
            unique_id,
            locked,
        })
        .try_send();
}

/// Awakening: [unique_id: u64][awake_type: u8][position_idx: u32]
fn forward_awakening(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 {
        return;
    }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    let awake_type = payload[8];
    debug!(
        "Awakening: session={} uid={} type={}",
        session_id, unique_id, awake_type
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::AwakeningRequest {
            session_id,
            unique_id,
            awake_type,
        })
        .try_send();
}

/// DisassembleItem: [unique_id: u64]
fn forward_disassemble_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DisassembleItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::DisassembleItemRequest {
            session_id,
            unique_id: uid,
        })
        .try_send();
}

/// DowngradeAwakening: [unique_id: u64]
fn forward_downgrade_awakening(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let unique_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!(
        "DowngradeAwakening: session={} uid={}",
        session_id, unique_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::DowngradeAwakeningRequest {
            session_id,
            unique_id,
        })
        .try_send();
}

/// ResetAddedItem: [unique_id: u64]
fn forward_reset_added_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("ResetAddedItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ResetAddedItemRequest {
            session_id,
            unique_id: uid,
        })
        .try_send();
}

/// DepositTradeItem: [from: i32][to: i32]
fn forward_deposit_trade_item(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "DepositTradeItem: session={} from={} to={}",
        session_id, from, to
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::DepositTradeItemBySlot {
            session_id,
            from_slot: from,
            to_slot: to,
        })
        .try_send();
}

/// RetrieveTradeItem: [from: i32][to: i32]
fn forward_retrieve_trade_item(
    social_ref: &Option<ActorRef<crate::actors::social::SocialActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "RetrieveTradeItem: session={} from={} to={}",
        session_id, from, to
    );
    let social_ref = match social_ref {
        Some(s) => s,
        None => return,
    };
    let _ = social_ref
        .tell(crate::actors::social::RetrieveTradeItemBySlot {
            session_id,
            from_slot: from,
            to_slot: to,
        })
        .try_send();
}

/// GuildWarReturn: [guild_name: DotNetString]
fn forward_guild_war_return(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let guild_name = parse_dotnet_string(payload);
    debug!(
        "GuildWarReturn: session={} guild={}",
        session_id, guild_name
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::GuildWarReturnRequest {
            session_id,
            guild_name,
        })
        .try_send();
}

/// GuildBuffUpdate: [buff_id: u32]
/// GuildBuffUpdate: [action: u8][buff_id: i32]（C# C.GuildBuffUpdate：0=请求列表 1=启用 2=激活）
fn forward_guild_buff_update(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 5 {
        return;
    }
    let action = payload[0];
    let buff_id = i32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4])) as u32;
    debug!(
        "GuildBuffUpdate: session={} action={} buff_id={}",
        session_id, action, buff_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::GuildBuffUpdateRequest {
            session_id,
            action,
            buff_id,
        })
        .try_send();
}

/// LockMail: [mail_id: u64][lock: bool]
fn forward_lock_mail(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 {
        return;
    }
    let mail_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let lock = payload[8] != 0;
    debug!(
        "LockMail: session={} mail_id={} lock={}",
        session_id, mail_id, lock
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::LockMailRequest {
            session_id,
            mail_id,
            lock,
        })
        .try_send();
}

/// MailLockedItem: [mail_id: u64][item_index: u32]
/// MailLockedItem: [unique_id: u64][locked: bool]（C# C.MailLockedItem；服务端回显）
fn forward_mail_locked_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 {
        return;
    }
    let unique_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let locked = payload[8] != 0;
    debug!(
        "MailLockedItem: session={} uid={} locked={}",
        session_id, unique_id, locked
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::MailLockedItemRequest {
            session_id,
            unique_id,
            locked,
        })
        .try_send();
}

/// MailCost: [gold: u32][items: 5*u64][stamped: bool]（C#/SharedRust wire；#2538）
fn forward_mail_cost(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    // 派发前已剥 4 字节帧头（与 handle_send_mail 同一入口）
    let mut cur = std::io::Cursor::new(payload);
    let Ok(p) = mir2_shared::packets::client::mail::MailCost::read_body(&mut cur) else {
        debug!("MailCost: session={} 解析失败", session_id);
        return;
    };
    let item_uids: Vec<u64> = p
        .items_idx
        .iter()
        .copied()
        .filter(|&uid| uid != 0)
        .collect();
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    debug!(
        "MailCost: session={} gold={} items={} stamped={}",
        session_id,
        p.gold,
        item_uids.len(),
        p.stamped
    );
    let _ = world_ref
        .tell(crate::actors::world::MailCostRequest {
            session_id,
            gold: p.gold,
            item_uids,
            stamped: p.stamped,
        })
        .try_send();
}

/// ShareQuest: [quest_id: u32]
fn forward_share_quest(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let quest_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("ShareQuest: session={} quest_id={}", session_id, quest_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ShareQuestRequest {
            session_id,
            quest_id,
        })
        .try_send();
}

/// AcceptReincarnation: []
fn forward_accept_reincarnation(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("AcceptReincarnation: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::AcceptReincarnationRequest { session_id })
        .try_send();
}

/// CancelReincarnation: []
fn forward_cancel_reincarnation(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("CancelReincarnation: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::CancelReincarnationRequest { session_id })
        .try_send();
}

/// GetRentedItems: forward to WorldActor
fn forward_get_rented_items(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
) {
    debug!("GetRentedItems: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::GetRentedItemsRequest { session_id })
        .try_send();
}

/// ItemRentalRequest: [target_name: DotNetString]
fn forward_item_rental_request(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let target_name = parse_dotnet_string(payload);
    debug!(
        "ItemRentalRequest: session={} target={}",
        session_id, target_name
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ItemRentalRequestMsg {
            session_id,
            target_name,
        })
        .try_send();
}

/// ItemRentalFee: [amount: u32]
fn forward_item_rental_fee(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let amount = if payload.len() >= 4 {
        u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]))
    } else {
        0
    };
    debug!("ItemRentalFee: session={} amount={}", session_id, amount);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ItemRentalFeeMsg { session_id, amount })
        .try_send();
}

/// ItemRentalPeriod: [duration: u32]
fn forward_item_rental_period(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let duration = if payload.len() >= 4 {
        u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]))
    } else {
        0
    };
    debug!(
        "ItemRentalPeriod: session={} duration={}",
        session_id, duration
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ItemRentalPeriodMsg {
            session_id,
            duration,
        })
        .try_send();
}

/// DepositRentalItem: [from: i32][to: i32]（C# C.DepositRentalItem 槽位线格式）
fn forward_deposit_rental_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "DepositRentalItem: session={} from={} to={}",
        session_id, from, to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::DepositRentalItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
}

/// RetrieveRentalItem: [from: i32][to: i32]（C# C.RetrieveRentalItem 槽位线格式）
fn forward_retrieve_rental_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 {
        return;
    }
    let from = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!(
        "RetrieveRentalItem: session={} from={} to={}",
        session_id, from, to
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::RetrieveRentalItemRequest {
            session_id,
            from,
            to,
        })
        .try_send();
}

/// CancelItemRental: []
fn forward_cancel_item_rental(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("CancelItemRental: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::CancelItemRentalRequest { session_id })
        .try_send();
}

/// ItemRentalLockFee: []
fn forward_item_rental_lock_fee(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("ItemRentalLockFee: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ItemRentalLockFeeMsg { session_id })
        .try_send();
}

/// ItemRentalLockItem: []
fn forward_item_rental_lock_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("ItemRentalLockItem: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ItemRentalLockItemMsg { session_id })
        .try_send();
}

/// ConfirmItemRental: []
fn forward_confirm_item_rental(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    debug!("ConfirmItemRental: session={}", session_id);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ConfirmItemRentalMsg { session_id })
        .try_send();
}

/// NPCConfirmInput: [npc_id: u32][page_name: DotNetString][value: DotNetString]（C# C.NPCConfirmInput）
fn forward_npc_confirm_input(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 {
        return;
    }
    let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let mut cursor = std::io::Cursor::new(&payload[4..]);
    use mir2_shared::binary::read_dotnet_string;
    let (page_name, input_text) = match (
        read_dotnet_string(&mut cursor),
        read_dotnet_string(&mut cursor),
    ) {
        (Ok(p), Ok(v)) => (p, v),
        _ => (String::new(), String::new()),
    };
    debug!(
        "NPCConfirmInput: session={} npc_id={} page={} input={}",
        session_id, npc_id, page_name, input_text
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::NPCConfirmInputRequest {
            session_id,
            npc_id,
            page_name,
            input_text,
        })
        .try_send();
}

/// GameshopBuy: [g_index: i32][quantity: u8][p_type: i32]（C# C.GameshopBuy 线格式；payload 已去包头）
/// #2566：解析 PType（0=Credit 信用点 / 1=Gold 金币，i32 LE）透传 world 侧分支扣费
fn parse_gameshop_buy_payload(payload: &[u8]) -> Option<(u32, u32, i32)> {
    if payload.len() < 9 {
        return None;
    }
    let item_id = i32::from_le_bytes(payload[0..4].try_into().ok()?) as u32;
    let count = payload[4] as u32;
    let p_type = i32::from_le_bytes(payload[5..9].try_into().ok()?);
    Some((item_id, count, p_type))
}

fn forward_gameshop_buy(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let Some((item_id, count, p_type)) = parse_gameshop_buy_payload(payload) else {
        return;
    };
    debug!(
        "GameshopBuy: session={} item={} count={} p_type={}",
        session_id, item_id, count, p_type
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::GameshopBuyRequest {
            session_id,
            item_id,
            count,
            p_type,
        })
        .try_send();
}

/// ReportIssue: [type: u32][description: DotNetString]
fn forward_report_issue(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let issue_type = if payload.len() >= 4 {
        u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) as u8
    } else {
        0
    };
    let description = if payload.len() >= 4 {
        parse_dotnet_string(&payload[4..])
    } else {
        String::new()
    };
    debug!("ReportIssue: session={} type={}", session_id, issue_type);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::ReportIssueRequest {
            session_id,
            issue_type,
            description,
        })
        .try_send();
}

/// GetRanking: [type: u8][online_only: u8]
fn forward_get_ranking(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let rank_type = if !payload.is_empty() { payload[0] } else { 0 };
    let online_only = if payload.len() > 1 {
        payload[1] != 0
    } else {
        false
    };
    debug!(
        "GetRanking: session={} type={} online_only={}",
        session_id, rank_type, online_only
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::GetRankingRequest {
            session_id,
            rank_type,
            online_only,
        })
        .try_send();
}

/// Opendoor: [door_index: u8]
fn forward_opendoor(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let door_index = payload[0];
    debug!("Opendoor: session={} door_index={}", session_id, door_index);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::OpendoorRequest {
            session_id,
            door_index,
        })
        .try_send();
}

/// GuildTerritoryPage: [page: u32]
fn forward_guild_territory_page(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let page = if payload.len() >= 4 {
        u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]))
    } else {
        0
    };
    debug!("GuildTerritoryPage: session={} page={}", session_id, page);
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::GuildTerritoryPageRequest { session_id, page })
        .try_send();
}

/// PurchaseGuildTerritory: [territory_id: u32]
fn forward_purchase_guild_territory(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    let territory_id = if payload.len() >= 4 {
        u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]))
    } else {
        0
    };
    debug!(
        "PurchaseGuildTerritory: session={} territory={}",
        session_id, territory_id
    );
    let world_ref = match world_ref {
        Some(w) => w,
        None => return,
    };
    let _ = world_ref
        .tell(crate::actors::world::PurchaseGuildTerritoryRequest {
            session_id,
            territory_id,
        })
        .try_send();
}

#[cfg(test)]
mod tests {
    /// 红绿回归（进图洪峰踢线）：SESSION_SEND_CAPACITY 必须 ≥ 8192。
    /// 2026-09-17 实机冒烟：比奇大图进图单次洪峰 ~2000 包/会话（43 NPC +
    /// 顶号安全门禁（2026-09-23，CAPACITY.md §3.6 规格①）：
    /// 同账号两个会话时，**旧会话先断开不得把账号置离线**；只有最后一个绑定会话断开才置离线。
    ///
    /// 为什么这条重要：它把「gate 必须内联 await StartGame 才能保证顺序」这个前提拆掉了——
    /// 也正是靠它，StartGame 才敢 spawn 出去（否则旧会话的 ClientDisconnected 会把
    /// 新会话正在用的账号标记离线：可被再次顶号、且新会话登出变成空操作）。
    ///
    /// 阳性对照（实做）：把 should_mark_account_offline 改成无条件 Some(username) →
    /// 第二个断言立即红。
    #[test]
    fn only_last_bound_session_marks_account_offline() {
        let mut bindings: HashMap<SessionId, String> = HashMap::new();
        bindings.insert(1, "alice".to_string());
        bindings.insert(2, "alice".to_string()); // 顶号：新会话接管同账号

        // 旧会话(1)被顶号后断开：账号仍有会话 2 在用 → 不得置离线
        let removed = bindings.remove(&1);
        assert_eq!(
            should_mark_account_offline(removed.as_deref(), &bindings),
            None,
            "旧会话断开不得把新会话正在用的账号置离线"
        );

        // 新会话(2)断开：这是最后一个绑定 → 才置离线
        let removed = bindings.remove(&2);
        assert_eq!(
            should_mark_account_offline(removed.as_deref(), &bindings).as_deref(),
            Some("alice"),
            "最后一个绑定会话断开必须置离线（否则账号永远在线）"
        );

        // 从未绑定过的会话：啥也不做
        assert_eq!(should_mark_account_offline(None, &bindings), None);
    }

    /// ~1900 怪物 + 地物/门/互见），1024 容量在 localhost 都有 ~50% 概率
    /// 积满触发 "kicking slow reader" 误踢正常客户端。8192 是下限锚，
    /// 实际取 16384 留 8 倍余量。
    #[test]
    fn session_send_capacity_absorbs_map_entry_burst() {
        assert!(
            SESSION_SEND_CAPACITY >= 8192,
            "SESSION_SEND_CAPACITY={} 小于进图洪峰下限 8192：大图进图会误踢正常客户端",
            SESSION_SEND_CAPACITY
        );
    }

    /// 红绿回归（进图洪峰丢包）：GATE_MAILBOX_CAPACITY 必须 ≥ 32768。
    /// world→gate 的 tell+try_send 洪峰（~2000 包/会话）溢出即静默丢对象包
    ///（隐形怪物/NPC）；32768 覆盖 ~15 会话同时进图，实际取 65536。
    #[test]
    fn gate_mailbox_capacity_absorbs_concurrent_entry_bursts() {
        assert!(
            GATE_MAILBOX_CAPACITY >= 32768,
            "GATE_MAILBOX_CAPACITY={} 小于并发进图洪峰下限 32768：对象包会被静默丢弃",
            GATE_MAILBOX_CAPACITY
        );
    }

    use super::*;

    /// 7-bit encoded length + UTF-8 bytes
    fn make_dotnet_string(s: &str) -> Vec<u8> {
        let bytes = s.as_bytes();
        let mut len = bytes.len();
        let mut out = Vec::new();
        loop {
            let mut b = (len & 0x7F) as u8;
            len >>= 7;
            if len != 0 {
                b |= 0x80;
            }
            out.push(b);
            if len == 0 {
                break;
            }
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

    /// #2566：C# C.GameshopBuy 线格式 [g_index: i32][quantity: u8][p_type: i32]（LE，payload 已去包头）
    #[test]
    fn test_parse_gameshop_buy_payload_ptype() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&7i32.to_le_bytes()); // GIndex
        payload.push(3); // Quantity
        payload.extend_from_slice(&0i32.to_le_bytes()); // PType=0 (Credit)
        assert_eq!(parse_gameshop_buy_payload(&payload), Some((7, 3, 0)));

        let mut gold = Vec::new();
        gold.extend_from_slice(&(-1i32).to_le_bytes());
        gold.push(99);
        gold.extend_from_slice(&1i32.to_le_bytes()); // PType=1 (Gold)
        assert_eq!(parse_gameshop_buy_payload(&gold), Some((4294967295, 99, 1)));

        // 不足 9 字节 → None（不 panic）
        assert_eq!(parse_gameshop_buy_payload(&[]), None);
        assert_eq!(parse_gameshop_buy_payload(&[1, 2, 3, 4, 5, 6, 7, 8]), None);
        // 多余字节 → 只读前 9 字节
        let mut extra = payload.clone();
        extra.extend_from_slice(&[0xFF]);
        assert_eq!(parse_gameshop_buy_payload(&extra), Some((7, 3, 0)));
    }

    #[test]
    fn test_parse_dotnet_string_malformed_truncated_body() {
        // Length says 10 bytes but only 3 provided
        let data = [10, 0x61, 0x62, 0x63];
        let result = parse_dotnet_string(&data);
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_pet_pickup_wire_format() {
        // #1558：C.IntelligentCreaturePickup = [mouse_mode u8][x i32][y i32]（9 字节）
        // 旧实现按 8 字节 [x][y] 解析导致偏移 1 字节
        let mut payload = Vec::new();
        payload.push(1u8); // mouse_mode = true（鼠标拾取）
        payload.extend_from_slice(&100i32.to_le_bytes());
        payload.extend_from_slice(&200i32.to_le_bytes());
        let (mouse_mode, x, y) = parse_pet_pickup(&payload).expect("9 字节包应解析成功");
        assert!(mouse_mode);
        assert_eq!(x, 100);
        assert_eq!(y, 200);

        // mouse_mode = false（半自动）
        payload[0] = 0;
        let (mouse_mode, x, y) = parse_pet_pickup(&payload).expect("半自动包应解析成功");
        assert!(!mouse_mode);
        assert_eq!(x, 100);
        assert_eq!(y, 200);
    }

    #[test]
    fn test_parse_pet_pickup_rejects_short_payload() {
        // 8 字节旧格式（缺 mouse_mode）必须拒绝，避免 x/y 偏移
        let mut payload = Vec::new();
        payload.extend_from_slice(&100i32.to_le_bytes());
        payload.extend_from_slice(&200i32.to_le_bytes());
        assert_eq!(parse_pet_pickup(&payload), None);
        assert_eq!(parse_pet_pickup(&[]), None);
    }

    fn big_stack_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap()
    }

    fn client_version_packet() -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&4i32.to_le_bytes());
        body.extend_from_slice(b"test");
        build_packet_bytes(ClientPacketIds::ClientVersion as i16, &body)
    }

    /// 红绿回归（LogOut+StartGame 竞态）：登出窗口内（LogOut 已受理、
    /// LogOutCleanup 未落地）除 KeepAlive 外一切包必须拒收——否则 rapid-fire
    /// 的 StartGame 会趁 gate 登录映射尚在、world 玩家记录已删的窗口重进游戏。
    /// 红检：删掉 ClientData 里的 logging_out 守卫 → 窗口内 ClientVersion 会
    /// 收到 S.ClientVersion 响应 → 断言 FAILED。
    #[test]
    fn logging_out_session_rejects_all_but_keepalive() {
        use kameo::actor::Spawn;
        use std::time::Duration;
        let rt = big_stack_runtime();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(16);
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id: 1,
                    sender: tx,
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            // 排空注册即发的 S.Connected（否则干扰后续响应断言）
            let _ = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("SessionCreated 必须发 S.Connected");

            // 置登出标记（生产路径只能经 LogOut Success 置位；
            // 测试直接置位以确定性撑开竞态窗口）
            let _ = gate_ref
                .ask(TestSetLoggingOut {
                    session_id: 1,
                    on: true,
                })
                .await;
            assert!(
                gate_ref
                    .ask(TestProbeLoggingOut { session_id: 1 })
                    .await
                    .unwrap(),
                "登出标记必须置位"
            );

            // 窗口内 ClientVersion 必须被拒（无任何响应）
            let _ = gate_ref
                .ask(ClientData {
                    session_id: 1,
                    data: client_version_packet(),
                })
                .await;
            assert!(
                tokio::time::timeout(Duration::from_millis(300), rx.recv())
                    .await
                    .is_err(),
                "登出窗口内 ClientVersion 必须被拒收（不得有响应）"
            );

            // KeepAlive 仍放行（连接保活不受门禁影响）
            let _ = gate_ref
                .ask(ClientData {
                    session_id: 1,
                    data: build_packet_bytes(ClientPacketIds::KeepAlive as i16, &[]),
                })
                .await;
            let resp = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("KeepAlive 必须放行")
                .expect("channel open");
            assert_eq!(
                i16::from_le_bytes([resp[2], resp[3]]),
                ServerPacketIds::KeepAlive as i16,
                "登出窗口内 KeepAlive 必须照常应答"
            );

            // 清标记后恢复正常放行
            let _ = gate_ref
                .ask(TestSetLoggingOut {
                    session_id: 1,
                    on: false,
                })
                .await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id: 1,
                    data: client_version_packet(),
                })
                .await;
            let resp = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("清标记后 ClientVersion 必须放行")
                .expect("channel open");
            assert_eq!(
                i16::from_le_bytes([resp[2], resp[3]]),
                ServerPacketIds::ClientVersion as i16
            );
        });
    }

    /// 红绿回归：登出标记必须随会话清理（LogOutCleanup → terminate_session）
    /// 一并清位，不得泄漏到后续同名会话状态判断。
    /// 红检：删掉 terminate_session 里的 logging_out.remove → 清理后探针仍 true
    /// → 断言 FAILED。
    #[test]
    fn logging_out_flag_cleared_on_cleanup() {
        use kameo::actor::Spawn;
        let rt = big_stack_runtime();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let (tx, _rx) = mpsc::channel::<Vec<u8>>(16);
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id: 2,
                    sender: tx,
                    ip: "127.0.0.1".to_string(),
                })
                .await;
            let _ = gate_ref
                .ask(TestSetLoggingOut {
                    session_id: 2,
                    on: true,
                })
                .await;

            let _ = gate_ref.tell(LogOutCleanup { session_id: 2 }).await;

            assert!(
                !gate_ref
                    .ask(TestProbeLoggingOut { session_id: 2 })
                    .await
                    .unwrap(),
                "会话清理后登出标记必须清位"
            );
            let (has_session, _) = gate_ref
                .ask(TestProbeSession { session_id: 2 })
                .await
                .unwrap();
            assert!(!has_session, "LogOutCleanup 必须删除会话");
        });
    }
}
