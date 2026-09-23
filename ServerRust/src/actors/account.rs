// AccountActor - 账号认证服务
// 对应 C# LoginSrv/AccountManager.cs

use std::collections::HashMap;

use argon2::password_hash::PasswordHash;
use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use mir2_shared::packets::Packet;
use pbkdf2::pbkdf2_hmac;
use rand_core::OsRng;
use sha1::Sha1;
use tracing::{error, info, warn};

use crate::db::{self, DbPool};
use crate::gate::actor::LoginResult;

/// Hash password using Argon2.
///
/// Phase 1.1: 不再 unwrap()。Argon2 hash 只在 password 含 null byte 等
/// 极端情况失败,此时返回 fallback 占位 hash(空字符串),调用方会
/// 因为 verify 永远失败而拒绝登录。log error 便于运维发现。
fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    match argon2::Argon2::default().hash_password(password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => {
            error!("Failed to hash password (Argon2 error): {}", e);
            String::new()
        }
    }
}

/// Verify password against a hash
/// Supports both Argon2 (native) and pbkdf2_sha1$ (migrated from C#)
/// Returns (success, needs_migration) — if needs_migration is true, the caller should re-hash with Argon2
fn verify_password(password: &str, hash: &str) -> (bool, bool) {
    // Check if it's a migrated PBKDF2 hash
    if let Some(rest) = hash.strip_prefix("pbkdf2_sha1$") {
        // Format: pbkdf2_sha1$<base64_salt>$<base64_hash>
        let parts: Vec<&str> = rest.splitn(2, '$').collect();
        if parts.len() == 2 {
            if let (Ok(salt), Ok(expected_hash)) = (
                data_encoding::BASE64.decode(parts[0].as_bytes()),
                data_encoding::BASE64.decode(parts[1].as_bytes()),
            ) {
                let mut computed = vec![0u8; 24]; // Crypto.HashSize = 24
                pbkdf2_hmac::<Sha1>(password.as_bytes(), &salt, 50, &mut computed); // Crypto.Iterations = 50
                if computed == expected_hash {
                    return (true, true); // Verified, but needs Argon2 migration
                }
            }
        }
        return (false, false);
    }

    // Argon2 hash — try to verify
    if let Ok(ph) = PasswordHash::new(hash) {
        let ok = argon2::Argon2::default()
            .verify_password(password.as_bytes(), &ph)
            .is_ok();
        return (ok, false);
    }

    (false, false)
}

/// 账号信息
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub username: String,
    pub password_hash: String,
    pub is_online: bool,
    // PR #1169: Warehouse password fields
    /// Argon2 hash of the warehouse password. `None` means no password set.
    pub storage_password_hash: Option<String>,
    /// Unix timestamp (seconds) of when the password was last changed.
    pub storage_password_last_set: i64,
    /// 账户积分（NPC 脚本 GIVECREDIT/TAKECREDIT，对齐 C# Account.Credit）
    pub credit: u64,
    /// 连续密码错误次数（C# Account.WrongPasswordCount，>=5 封禁 2 分钟）
    pub wrong_password_count: u32,
    /// 封禁到期时间（unix 秒；0 = 未封禁，C# Account.ExpiryDate）
    pub banned_until: i64,
    /// 是否强制改密（C# AccountInfo.RequirePasswordChange；登录返回 Result=5）
    pub require_password_change: bool,
    /// 是否已购买仓库扩容（C# AccountInfo.HasExpandedStorage）
    pub has_expanded_storage: bool,
    /// 仓库扩容到期时间（unix 秒；0 = 无，C# AccountInfo.ExpandedStorageExpiryDate）
    pub expanded_storage_expiry_date: i64,
}

impl AccountInfo {
    pub fn has_storage_password(&self) -> bool {
        self.storage_password_hash.is_some()
    }
}

/// AccountActor 状态
pub struct AccountActor {
    accounts: HashMap<String, AccountInfo>,
    /// GateActor 引用，用于回传登录结果
    gate_ref: ActorRef<crate::gate::actor::GateActor>,
    /// SQLite 数据库连接池
    db_pool: DbPool,
    /// 是否允许注册新账号（C# Settings.AllowNewAccount，server.toml [social] allow_new_account）；
    /// 关闭时登录不存在账号不再自动注册（对齐 C# Envir.Login：无自助注册）
    allow_new_account: bool,
}

impl AccountActor {
    pub fn new(gate_ref: ActorRef<crate::gate::actor::GateActor>, db_pool: DbPool) -> Self {
        Self {
            accounts: HashMap::new(),
            gate_ref,
            db_pool,
            allow_new_account: true,
        }
    }

    /// 注册账号
    pub fn register(&mut self, username: &str, password: &str) -> bool {
        if self.accounts.contains_key(username) {
            warn!("Account already exists: {}", username);
            return false;
        }

        self.accounts.insert(
            username.to_string(),
            AccountInfo {
                username: username.to_string(),
                password_hash: hash_password(password),
                is_online: false,
                storage_password_hash: None,
                storage_password_last_set: 0,
                credit: 0,
                wrong_password_count: 0,
                banned_until: 0,
                require_password_change: false,
                has_expanded_storage: false,
                expanded_storage_expiry_date: 0,
            },
        );

        info!("Account registered: {}", username);
        true
    }

    /// 注册账号（使用已有的密码哈希，用于从数据库加载）
    pub fn register_with_hash(&mut self, username: &str, password_hash: &str) -> bool {
        if self.accounts.contains_key(username) {
            return false;
        }

        self.accounts.insert(
            username.to_string(),
            AccountInfo {
                username: username.to_string(),
                password_hash: password_hash.to_string(),
                is_online: false,
                storage_password_hash: None,
                storage_password_last_set: 0,
                credit: 0,
                wrong_password_count: 0,
                banned_until: 0,
                require_password_change: false,
                has_expanded_storage: false,
                expanded_storage_expiry_date: 0,
            },
        );

        true
    }

    /// 登录验证
    /// 返回 (success, needs_db_save) — needs_db_save 表示密码已从 PBKDF2 迁移到 Argon2
    pub fn login(&mut self, username: &str, password: &str) -> (bool, bool) {
        let now = Self::unix_now_secs();
        if let Some(account) = self.accounts.get_mut(username) {
            // C#：封禁期内直接拒绝
            if account.banned_until > now {
                warn!(
                    "Account banned until {}: {}",
                    account.banned_until, username
                );
                return (false, false);
            }
            let (ok, needs_migration) = verify_password(password, &account.password_hash);
            if !ok {
                // C#：WrongPasswordCount++，>=5 → 封禁 2 分钟
                account.wrong_password_count = account.wrong_password_count.saturating_add(1);
                if account.wrong_password_count >= 5 {
                    account.banned_until = now + 120;
                    warn!(
                        "Account '{}' banned for 2 minutes (too many wrong passwords)",
                        username
                    );
                } else {
                    warn!(
                        "Wrong password for account: {} (attempt {})",
                        username, account.wrong_password_count
                    );
                }
                return (false, false);
            }
            // 登录成功重置（C# WrongPasswordCount = 0 / Banned = false）
            account.wrong_password_count = 0;
            account.banned_until = 0;
            // If migrated from C#, re-hash with Argon2 on first login
            if needs_migration {
                account.password_hash = hash_password(password);
                info!("Password hash migrated to Argon2 for account: {}", username);
            }
            // C#：RequirePasswordChange=true → 登录返回 Result=5，不置在线
            if account.require_password_change {
                warn!("Account '{}' requires password change", username);
                return (false, false);
            }
            if account.is_online {
                warn!("Account already online: {}", username);
                return (false, false);
            }
            account.is_online = true;
            info!("Account logged in: {}", username);
            (true, needs_migration)
        } else {
            // 登录不存在账号的自动注册：受 AllowNewAccount 门控（C# 本无自助注册，
            // 本服兼容开关；关闭时拒绝，避免绕过 gate 的注册开关与每 IP 防刷）
            if !self.allow_new_account {
                warn!(
                    "Login rejected: account '{}' not found and AllowNewAccount=false",
                    username
                );
                return (false, false);
            }
            info!("Auto-registering account: {}", username);
            if self.register(username, password) {
                // 与正常 login 分支对齐：注册成功即本次登录成功，必须置在线——
                // 否则「在线账号拒登」对新账号首登不生效，第三方可随即同账号登入，
                // 直到一次 logout/login 周期才恢复
                if let Some(account) = self.accounts.get_mut(username) {
                    account.is_online = true;
                }
                info!("Account logged in: {}", username);
            }
            (true, false)
        }
    }

    /// 当前是否处于封禁期，返回封禁到期 unix 秒
    pub fn banned_until(&self, username: &str) -> Option<i64> {
        let now = Self::unix_now_secs();
        self.accounts
            .get(username)
            .map(|a| a.banned_until)
            .filter(|&until| until > now)
    }

    fn unix_now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// 登出
    pub fn logout(&mut self, username: &str) {
        if let Some(account) = self.accounts.get_mut(username) {
            account.is_online = false;
            info!("Account logged out: {}", username);
        }
    }
}

impl Actor for AccountActor {
    type Args = (ActorRef<crate::gate::actor::GateActor>, DbPool);
    type Error = anyhow::Error;

    async fn on_start(
        (gate_ref, db_pool): (ActorRef<crate::gate::actor::GateActor>, DbPool),
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let mut actor = Self::new(gate_ref, db_pool);

        // 读取 AllowNewAccount 开关（C# Settings.AllowNewAccount；与 main.rs 相同的
        // 配置路径解析：argv[1] 或 config/server.toml；读取失败回退 C# 默认 true）
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "config/server.toml".to_string());
        actor.allow_new_account = crate::util::config::load_config(&config_path)
            .map(|c| c.social.allow_new_account)
            .unwrap_or(true);

        // 从数据库加载已有账号到内存
        match db::load_all_accounts(&actor.db_pool).await {
            Ok(accounts) => {
                for acc in accounts {
                    // 强制设为离线（上次可能是异常退出）
                    let mut acc = acc;
                    acc.is_online = false;
                    info!("Loaded account from DB: {}", acc.username);
                    actor.accounts.insert(acc.username.clone(), acc);
                }
                info!("Loaded {} accounts from database", actor.accounts.len());
            }
            Err(e) => {
                warn!("Failed to load accounts from database: {}", e);
            }
        }

        Ok(actor)
    }
}

// =============================================================================
// PR #1169: Warehouse password methods (AccountActor core)
// =============================================================================

/// 验证仓库密码。返回 `(result_code, has_password)`。
/// `result_code` 见 master `Shared/ServerPackets.cs::StorageUnlockResult` 注释:
/// 0=Success 1=BadPassword 2=WrongPassword 3=NotAvailable 4=NoPasswordSet
/// `has_password` 反映 account 当前是否设了密码(用于客户端判断走哪条 UI 分支)。
pub fn validate_storage_password(
    actor: &AccountActor,
    username: &str,
    raw_password: &str,
) -> (u8, bool) {
    let account = match actor.accounts.get(username) {
        Some(a) => a,
        None => return (3, false), // NotAvailable
    };
    let stored_hash = match &account.storage_password_hash {
        Some(h) => h,
        None => return (4, false), // NoPasswordSet — directly unlock
    };
    // Use the same Argon2 verify used for account password.
    // Returns (verified, needs_argon2_migration) — we ignore migration here.
    let (verified, _) = verify_password(raw_password, stored_hash);
    if verified {
        (0, true)
    } else {
        (2, true) // WrongPassword
    }
}

/// 设置或修改仓库密码。返回 result code (0-4, 见 master Shared/ServerPackets.cs)。
pub fn set_storage_password(
    actor: &mut AccountActor,
    username: &str,
    current_raw: &str,
    new_raw: &str,
) -> u8 {
    let account = match actor.accounts.get_mut(username) {
        Some(a) => a,
        None => return 0, // NotAvailable
    };
    // If old password is set, verify current_raw matches
    if let Some(stored_hash) = &account.storage_password_hash {
        let (verified, _) = verify_password(current_raw, stored_hash);
        if !verified {
            return 2; // WrongCurrentPassword
        }
    }
    // (C# also validates new password format with regex; we skip that for now.)
    account.storage_password_hash = Some(hash_password(new_raw));
    account.storage_password_last_set = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    4 // Success
}

/// 删除仓库密码。需要当前密码确认。
pub fn clear_storage_password(actor: &mut AccountActor, username: &str, current_raw: &str) -> u8 {
    let account = match actor.accounts.get_mut(username) {
        Some(a) => a,
        None => return 5, // NoPasswordSet (also used for invalid account)
    };
    let stored_hash = match &account.storage_password_hash {
        Some(h) => h.clone(),
        None => return 5, // NoPasswordSet
    };
    let (verified, _) = verify_password(current_raw, &stored_hash);
    if !verified {
        return 2; // WrongCurrentPassword
    }
    account.storage_password_hash = None;
    account.storage_password_last_set = 0;
    4 // Success
}

// ============================================================
// 消息定义
// ============================================================

/// 注册请求（NewAccount 真正创建账号；返回是否成功，false=账号已存在）
pub struct RegisterAccountRequest {
    pub username: String,
    pub password: String,
}

impl Message<RegisterAccountRequest> for AccountActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: RegisterAccountRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.register(&msg.username, &msg.password)
    }
}

/// 登录请求
pub struct LoginRequest {
    pub session_id: u64,
    pub username: String,
    pub password: String,
}

/// 登出请求
#[derive(Debug)]
pub struct LogoutRequest {
    pub username: String,
}

// ============================================================
// Handler 实现
// ============================================================

impl Message<LoginRequest> for AccountActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: LoginRequest,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // 2026-09-23 容量标定（tools/ops/CAPACITY.md）：登录 p95 随并发线性增长
        // （100→1.36s、200→2.96s，约 14ms/次），根因是**argon2id 校验在 AccountActor 里串行**——
        // 账号校验是 CPU 密集（m=19456,t=2），却把整个 actor 邮箱堵住。
        // 修法：**只把 CPU 校验挪出去**（spawn_blocking，吃满 blocker 线程池），
        // 校验结果用 `PasswordVerified` 回投，状态改动与回包逻辑仍全部留在 actor 内（语义不变）。
        if let Some(acc) = self.accounts.get(&msg.username) {
            let now = Self::unix_now_secs();
            if acc.banned_until > now {
                // 封禁期内直接拒绝（与原逻辑一致，无需校验 CPU）
                return self.finish_login(msg.session_id, msg.username, false).await;
            }
            let hash = acc.password_hash.clone();
            let me = ctx.actor_ref().clone();
            let (sid, user, pass) = (msg.session_id, msg.username.clone(), msg.password.clone());
            crate::util::tasks::spawn("account.password_verify", async move {
                let pw = pass.clone();
                let (ok, needs_migration) =
                    tokio::task::spawn_blocking(move || verify_password(&pw, &hash))
                        .await
                        .unwrap_or((false, false));
                let _ = me
                    .tell(PasswordVerified {
                        session_id: sid,
                        username: user,
                        password: pass,
                        ok,
                        needs_migration,
                    })
                    .await;
            });
            return;
        }
        // 账号不存在：保留原同步路径（受 AllowNewAccount 门控的自动注册；无 argon2 以外的重活）
        let (success, _needs_db_save) = self.login(&msg.username, &msg.password);
        self.finish_login(msg.session_id, msg.username, success)
            .await;
    }
}

/// argon2 校验完成回投（见 `LoginRequest` 的分流注释）。
pub struct PasswordVerified {
    pub session_id: u64,
    pub username: String,
    pub password: String,
    pub ok: bool,
    pub needs_migration: bool,
}

impl Message<PasswordVerified> for AccountActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: PasswordVerified,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let now = Self::unix_now_secs();
        let success = if !msg.ok {
            // 与原 `login()` 一致：WrongPasswordCount++，>=5 封 2 分钟
            if let Some(account) = self.accounts.get_mut(&msg.username) {
                account.wrong_password_count = account.wrong_password_count.saturating_add(1);
                if account.wrong_password_count >= 5 {
                    account.banned_until = now + 120;
                    warn!(
                        "Account '{}' banned for 2 minutes (too many wrong passwords)",
                        msg.username
                    );
                } else {
                    warn!(
                        "Wrong password for account: {} (attempt {})",
                        msg.username, account.wrong_password_count
                    );
                }
            }
            false
        } else if let Some(account) = self.accounts.get_mut(&msg.username) {
            account.wrong_password_count = 0;
            account.banned_until = 0;
            if msg.needs_migration {
                account.password_hash = hash_password(&msg.password);
                info!(
                    "Password hash migrated to Argon2 for account: {}",
                    msg.username
                );
            }
            if account.require_password_change {
                warn!("Account '{}' requires password change", msg.username);
                false
            } else if account.is_online {
                warn!("Account already online: {}", msg.username);
                false
            } else {
                account.is_online = true;
                info!("Account logged in: {}", msg.username);
                true
            }
        } else {
            false
        };
        self.finish_login(msg.session_id, msg.username, success)
            .await;
    }
}

impl AccountActor {
    /// 登录收尾（原 `LoginRequest` 处理的后半段）：算出封禁/强制改密标记 → 落库 → 查角色列表 → 回包。
    /// 抽出来是为了让「同步路径（账号不存在/封禁）」与「异步校验路径（PasswordVerified）」共用同一套语义。
    async fn finish_login(&mut self, session_id: u64, username: String, success: bool) {
        let banned_until = if success {
            None
        } else {
            self.banned_until(&username)
        };
        // C# RequirePasswordChange：密码正确但需强制改密（login 返回 false 且未封禁）
        let require_password_change = !success
            && banned_until.is_none()
            && self
                .accounts
                .get(&username)
                .map(|a| a.require_password_change)
                .unwrap_or(false);

        // 同步到数据库
        if success {
            if let Some(account) = self.accounts.get(&username) {
                if let Err(e) = db::save_account(&self.db_pool, account).await {
                    warn!("Failed to save account '{}' on login: {}", username, e);
                }
            }
        }

        info!("Login result for '{}': {}", username, success);

        // 角色列表（登录成功时查询）
        let characters = if success {
            match db::list_character_summaries(&self.db_pool, &username).await {
                Ok(chars) => chars,
                Err(e) => {
                    warn!("Failed to list characters for '{}': {}", username, e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // 将结果发回 GateActor，由 GateActor 发送协议包给客户端
        let _ = self
            .gate_ref
            .tell(LoginResult {
                session_id,
                success,
                username,
                characters,
                banned_until,
                require_password_change,
            })
            .await;
    }
}

impl Message<LogoutRequest> for AccountActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: LogoutRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.logout(&msg.username);

        // 同步到数据库（标记离线）
        if let Err(e) = db::set_account_offline(&self.db_pool, &msg.username).await {
            warn!("Failed to set account '{}' offline: {}", msg.username, e);
        }
    }
}

// =============================================================================
// PR #1169: Warehouse password request messages
// =============================================================================

/// 验证仓库密码 (从 GateActor 转发)
pub struct ValidateStoragePasswordRequest {
    pub session_id: u64,
    pub username: String,
    pub raw_password: String,
}

impl Message<ValidateStoragePasswordRequest> for AccountActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: ValidateStoragePasswordRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (result, has_password) =
            validate_storage_password(self, &msg.username, &msg.raw_password);
        // Send StorageUnlockResult back to client
        let packet = mir2_shared::packets::server::StorageUnlockResult {
            result,
            has_password,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            // #23 下行 try_send：gate forward_unlock_storage 对本消息内联 ask，
            // 此处若阻塞等 gate 邮箱空位即构成 gate→account→gate ask-reply 活锁环
            // （单连接 DoS）。校验结果已由 ask reply 返回 gate，发包只是下行通知，
            // 邮箱满丢包 warn 容忍，由 gate 会话通道积满踢线路径兜底。
            if let Err(e) = self
                .gate_ref
                .tell(crate::gate::actor::SendToClient {
                    session_id: msg.session_id,
                    data: crate::util::wire::build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::StorageUnlockResult as i16,
                        &body,
                    ),
                })
                .try_send()
            {
                warn!(
                    "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                    msg.session_id,
                    crate::actors::world::dropped_send_opcode(&e),
                    e
                );
            }
        }
        // #200：校验成功（0=成功 / 4=无密码直接解锁）→ GateActor 通知 WorldActor 下发仓库
        result == 0 || result == 4
    }
}

/// 设置/修改仓库密码 (从 GateActor 转发)
pub struct SetStoragePasswordRequest {
    pub session_id: u64,
    pub username: String,
    pub current_raw: String,
    pub new_raw: String,
}

impl Message<SetStoragePasswordRequest> for AccountActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetStoragePasswordRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let result = set_storage_password(self, &msg.username, &msg.current_raw, &msg.new_raw);
        // Persist
        if result == 4 {
            if let Some(acc) = self.accounts.get(&msg.username) {
                if let Err(e) = db::save_account(&self.db_pool, acc).await {
                    warn!("Failed to save storage password: {}", e);
                }
            }
        }
        // Compute LastSetTime (use the freshly-set value, or 0 if removing)
        let last_set = self
            .accounts
            .get(&msg.username)
            .map(|a| a.storage_password_last_set)
            .unwrap_or(0);
        let has_password = self
            .accounts
            .get(&msg.username)
            .map(|a| a.has_storage_password())
            .unwrap_or(false);
        let packet = mir2_shared::packets::server::StoragePasswordResult {
            result,
            // 这个 handler 处理 SetStoragePasswordRequest;ClearStoragePasswordRequest
            // 由独立 handler 单独发包。removing 字段总是 false 表示本次是 set 而非 remove。
            removing: false,
            has_password,
            last_set_time: last_set,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            // #23 下行 try_send（见 ValidateStoragePasswordRequest 注释）
            if let Err(e) = self
                .gate_ref
                .tell(crate::gate::actor::SendToClient {
                    session_id: msg.session_id,
                    data: crate::util::wire::build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::StoragePasswordResult as i16,
                        &body,
                    ),
                })
                .try_send()
            {
                warn!(
                    "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                    msg.session_id,
                    crate::actors::world::dropped_send_opcode(&e),
                    e
                );
            }
        }
    }
}

/// 删除仓库密码 (从 GateActor 转发)
pub struct ClearStoragePasswordRequest {
    pub session_id: u64,
    pub username: String,
    pub current_raw: String,
}

impl Message<ClearStoragePasswordRequest> for AccountActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ClearStoragePasswordRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let result = clear_storage_password(self, &msg.username, &msg.current_raw);
        if result == 4 {
            if let Some(acc) = self.accounts.get(&msg.username) {
                if let Err(e) = db::save_account(&self.db_pool, acc).await {
                    warn!("Failed to save storage password clear: {}", e);
                }
            }
        }
        let packet = mir2_shared::packets::server::StoragePasswordResult {
            result,
            removing: true,
            has_password: false,
            last_set_time: 0,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            // #23 下行 try_send（见 ValidateStoragePasswordRequest 注释）
            if let Err(e) = self
                .gate_ref
                .tell(crate::gate::actor::SendToClient {
                    session_id: msg.session_id,
                    data: crate::util::wire::build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::StoragePasswordResult as i16,
                        &body,
                    ),
                })
                .try_send()
            {
                warn!(
                    "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                    msg.session_id,
                    crate::actors::world::dropped_send_opcode(&e),
                    e
                );
            }
        }
    }
}

/// 修改密码请求
pub struct AccountChangePassword {
    pub session_id: u64,
    pub username: String,
    pub old_password: String,
    pub new_password: String,
    /// 本会话登录的账号（gate 侧 session_usernames 值；None = 强制改密待办例外路径）。
    /// 纵深防御：登录态会话只能改本会话登录的账号，防跨账号喷洒（gate 已拦，此处再校验）
    pub session_account: Option<String>,
}

/// 查询账号封禁到期时间（封禁检查/测试观测缝）
pub struct GetAccountBannedUntil {
    pub username: String,
}

impl Message<GetAccountBannedUntil> for AccountActor {
    type Reply = Option<i64>;

    async fn handle(
        &mut self,
        msg: GetAccountBannedUntil,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.banned_until(&msg.username)
    }
}

impl Message<AccountChangePassword> for AccountActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AccountChangePassword,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let gate_ref = self.gate_ref.clone();
        // 封禁提示用独立克隆（send_result 闭包 move 了 gate_ref）
        let ban_gate_ref = gate_ref.clone();
        let send_result = |result: u8| async move {
            let packet = mir2_shared::packets::server::login::ChangePassword { result };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                // #23 下行 try_send（见 ValidateStoragePasswordRequest 注释）
                if let Err(e) = gate_ref
                    .tell(crate::gate::actor::SendToClient {
                        session_id: msg.session_id,
                        data: crate::util::wire::build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ChangePassword as i16,
                            &body,
                        ),
                    })
                    .try_send()
                {
                    warn!(
                        "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                        msg.session_id,
                        crate::actors::world::dropped_send_opcode(&e),
                        e
                    );
                }
            }
        };

        // 跨账号喷洒校验（纵深防御；gate 已拦）：登录态会话只能改本会话登录账号；
        // None = RequirePasswordChange 强制改密例外（账号名 gate 已比对登记值）。
        // 按 Result=4（账号不存在）回复，与未知账号同响应，不泄露账号是否存在
        if let Some(expected) = &msg.session_account {
            if expected != &msg.username {
                warn!(
                    "ChangePassword rejected: session account '{}' cannot target '{}'",
                    expected, msg.username
                );
                send_result(4).await;
                return;
            }
        }
        let Some(account) = self.accounts.get_mut(&msg.username) else {
            // C#：账号不存在 → Result=4
            warn!("Account '{}' not found for password change", msg.username);
            send_result(4).await;
            return;
        };
        // #2340：C# Envir.cs:3816-3824——封禁中 → ChangePasswordBanned（原因 + expiry ticks）；到期自动解封
        let now_secs = Self::unix_now_secs();
        if account.banned_until > now_secs {
            let packet = mir2_shared::packets::server::login::ChangePasswordBanned {
                reason: "登录尝试失败次数过多，账号暂时封禁".to_string(),
                expiry_date: crate::actors::world::unix_secs_to_dotnet_ticks(account.banned_until),
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                // #23 下行 try_send（见 ValidateStoragePasswordRequest 注释）
                if let Err(e) = ban_gate_ref
                    .tell(crate::gate::actor::SendToClient {
                        session_id: msg.session_id,
                        data: crate::util::wire::build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ChangePasswordBanned as i16,
                            &body,
                        ),
                    })
                    .try_send()
                {
                    warn!(
                        "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                        msg.session_id,
                        crate::actors::world::dropped_send_opcode(&e),
                        e
                    );
                }
            }
            warn!(
                "ChangePassword rejected: account '{}' banned until {}",
                msg.username, account.banned_until
            );
            return;
        }
        if account.banned_until > 0 {
            // C# 到期自动解封（Envir.cs:3823）
            account.banned_until = 0;
        }
        // Verify old password before changing（C#：不匹配 → Result=5）
        let (ok, _needs_migration) = verify_password(&msg.old_password, &account.password_hash);
        if !ok {
            // 安全加固（超越 C#）：旧密码错误计数 + >=5 封禁 2 分钟，镜像 login() 的
            // WrongPasswordCount 机制——否则 ChangePassword 是在线爆破任意账号口令的后门。
            // 计数与登录【不分离】：C# 每账号只有单一 WrongPasswordCount 字段
            // （登录成功/改密成功清零，错误累加），分开计数会偏离 C# 语义且给爆破者
            // 两条独立的 5 次额度；共享计数下任一路径错误都消耗同一额度。
            account.wrong_password_count = account.wrong_password_count.saturating_add(1);
            if account.wrong_password_count >= 5 {
                account.banned_until = now_secs + 120;
                warn!(
                    "Account '{}' banned for 2 minutes (too many wrong old-password attempts)",
                    msg.username
                );
            } else {
                warn!(
                    "Old password mismatch for account: {} (attempt {})",
                    msg.username, account.wrong_password_count
                );
            }
            send_result(5).await;
            return;
        }
        // 旧密码验证通过：重置错误计数（与登录成功同语义）
        account.wrong_password_count = 0;
        account.banned_until = 0;
        account.password_hash = hash_password(&msg.new_password);
        // C# ChangePassword：成功后 RequirePasswordChange = false
        account.require_password_change = false;
        if let Err(e) =
            db::change_password(&self.db_pool, &msg.username, &account.password_hash).await
        {
            warn!("Failed to change password for '{}': {}", msg.username, e);
        } else {
            info!("Password changed for account: {}", msg.username);
        }
        if let Err(e) = db::save_account(&self.db_pool, account).await {
            warn!("Failed to persist require_password_change reset: {}", e);
        }
        // C#：成功 → Result=6
        send_result(6).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kameo::actor::Spawn;

    /// 红绿回归：登录不存在账号的自动注册必须受 AllowNewAccount 门控（严重9）。
    /// 红检：删掉 login() 里的 allow_new_account 判断 → 第一组断言 FAILED（账号被自动注册）。
    #[tokio::test]
    async fn login_unknown_account_respects_allow_new_account() {
        let gate_ref = crate::gate::actor::GateActor::spawn(());
        let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");

        // 关闭开关：不自动注册、不创建账号
        let mut actor = AccountActor::new(gate_ref.clone(), db_pool.clone());
        actor.allow_new_account = false;
        let (ok, _) = actor.login("ghost", "pw12345");
        assert!(!ok, "AllowNewAccount=false 时未知账号登录必须失败");
        assert!(
            !actor.accounts.contains_key("ghost"),
            "AllowNewAccount=false 时不得自动注册账号"
        );

        // 开启开关：保持既有兼容行为（自动注册并登录成功）
        let mut actor = AccountActor::new(gate_ref, db_pool);
        actor.allow_new_account = true;
        let (ok, _) = actor.login("newbie", "pw12345");
        assert!(ok, "AllowNewAccount=true 时未知账号登录自动注册");
        assert!(actor.accounts.contains_key("newbie"));
    }

    /// 红绿回归：自动注册分支登录成功后必须置 is_online（严重）。
    /// 红检：回退修复（删掉注册分支里的 is_online = true）→ 首登后第三方
    /// 同账号登录不被拒，最后一条断言 FAILED。
    #[tokio::test]
    async fn auto_register_first_login_marks_online_and_rejects_relogin() {
        let gate_ref = crate::gate::actor::GateActor::spawn(());
        let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
        let mut actor = AccountActor::new(gate_ref, db_pool);
        actor.allow_new_account = true;

        // 新账号首登：自动注册 + 登录成功，且必须立即置在线
        let (ok, _) = actor.login("fresh", "pw12345");
        assert!(ok, "未知账号首登自动注册并成功");
        assert!(
            actor.accounts.get("fresh").map(|a| a.is_online) == Some(true),
            "自动注册分支登录成功后必须置 is_online（与正常 login 分支对齐）"
        );

        // 第三方随即持同口令同账号登入：必须被「在线账号拒登」拦下
        let (ok2, _) = actor.login("fresh", "pw12345");
        assert!(!ok2, "账号在线期间同账号再次登录必须被拒");
    }

    /// 红绿回归：ChangePassword 旧密码错误必须计数并在 >=5 次后封禁 2 分钟（严重11 后半）。
    /// 红检：把 handle 里 wrong_password_count 累加分支删掉 → 第五次后 banned_until 仍为 0，断言失败。
    #[tokio::test]
    async fn change_password_wrong_old_password_counts_and_bans() {
        let gate_ref = crate::gate::actor::GateActor::spawn(());
        let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
        let actor_ref = AccountActor::spawn((gate_ref, db_pool));

        // 注册账号（走注册消息，避免依赖自动注册行为）
        let registered = actor_ref
            .ask(RegisterAccountRequest {
                username: "victim".to_string(),
                password: "correct_pw".to_string(),
            })
            .await
            .expect("register");
        assert!(registered);

        // 连续 5 次错误旧密码
        for _ in 0..5 {
            actor_ref
                .ask(AccountChangePassword {
                    session_id: 1,
                    username: "victim".to_string(),
                    old_password: "wrong_pw".to_string(),
                    new_password: "new_pw".to_string(),
                    session_account: Some("victim".to_string()),
                })
                .await
                .expect("change attempt");
        }

        let banned = actor_ref
            .ask(GetAccountBannedUntil {
                username: "victim".to_string(),
            })
            .await
            .expect("query ban")
            .expect("5 次错误旧密码后必须处于封禁期");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(
            banned > now,
            "5 次错误旧密码后账号必须处于封禁期（banned_until={banned}, now={now}）"
        );
    }
}
