// AccountActor - 账号认证服务
// 对应 C# LoginSrv/AccountManager.cs

use std::collections::HashMap;

use kameo::actor::{Actor, ActorRef};
use kameo::prelude::Context;
use kameo::message::Message;
use argon2::password_hash::PasswordHash;
use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use pbkdf2::pbkdf2_hmac;
use rand_core::OsRng;
use sha1::Sha1;
use tracing::{info, warn};

use crate::db::{self, DbPool};
use crate::gate::actor::LoginResult;

/// Hash password using Argon2
fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

/// Verify password against a hash
/// Supports both Argon2 (native) and pbkdf2_sha1$ (migrated from C#)
/// Returns (success, needs_migration) — if needs_migration is true, the caller should re-hash with Argon2
fn verify_password(password: &str, hash: &str) -> (bool, bool) {
    // Check if it's a migrated PBKDF2 hash
    if let Some(rest) = hash.strip_prefix("pbkdf2_sha1$") {
        // Format: pbkdf2_sha1$<base64_salt>$<base64_hash>
        let parts: Vec<&str> = rest.splitn(3, '$').collect();
        if parts.len() == 3 {
            if let (Ok(salt), Ok(expected_hash)) = (data_encoding::BASE64.decode(parts[1].as_bytes()), data_encoding::BASE64.decode(parts[2].as_bytes())) {
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
}

/// AccountActor 状态
pub struct AccountActor {
    accounts: HashMap<String, AccountInfo>,
    /// GateActor 引用，用于回传登录结果
    gate_ref: ActorRef<crate::gate::actor::GateActor>,
    /// SQLite 数据库连接池
    db_pool: DbPool,
}

impl AccountActor {
    pub fn new(gate_ref: ActorRef<crate::gate::actor::GateActor>, db_pool: DbPool) -> Self {
        Self {
            accounts: HashMap::new(),
            gate_ref,
            db_pool,
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
            },
        );

        true
    }

    /// 登录验证
    /// 返回 (success, needs_db_save) — needs_db_save 表示密码已从 PBKDF2 迁移到 Argon2
    pub fn login(&mut self, username: &str, password: &str) -> (bool, bool) {
        if let Some(account) = self.accounts.get_mut(username) {
            let (ok, needs_migration) = verify_password(password, &account.password_hash);
            if !ok {
                warn!("Wrong password for account: {}", username);
                return (false, false);
            }
            // If migrated from C#, re-hash with Argon2 on first login
            if needs_migration {
                account.password_hash = hash_password(password);
                info!("Password hash migrated to Argon2 for account: {}", username);
            }
            if account.is_online {
                warn!("Account already online: {}", username);
                return (false, false);
            }
            account.is_online = true;
            info!("Account logged in: {}", username);
            (true, needs_migration)
        } else {
            // 自动注册不存在的账号
            info!("Auto-registering account: {}", username);
            self.register(username, password);
            (true, false)
        }
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

    async fn on_start((gate_ref, db_pool): (ActorRef<crate::gate::actor::GateActor>, DbPool), _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let mut actor = Self::new(gate_ref, db_pool);

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


// ============================================================
// 消息定义
// ============================================================

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
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (success, _needs_db_save) = self.login(&msg.username, &msg.password);

        // 同步到数据库
        if success {
            if let Some(account) = self.accounts.get(&msg.username) {
                if let Err(e) = db::save_account(&self.db_pool, account).await {
                    warn!("Failed to save account '{}' on login: {}", msg.username, e);
                }
            }
        }

        info!("Login result for '{}': {}", msg.username, success);

        // 将结果发回 GateActor，由 GateActor 发送协议包给客户端
        let _ = self.gate_ref.ask(LoginResult {
            session_id: msg.session_id,
            success,
            username: msg.username.clone(),
        }).await;
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

/// 修改密码请求
pub struct AccountChangePassword {
    pub session_id: u64,
    pub username: String,
    pub old_password: String,
    pub new_password: String,
}

impl Message<AccountChangePassword> for AccountActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AccountChangePassword,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(account) = self.accounts.get_mut(&msg.username) {
            // Verify old password before changing
            let (ok, _needs_migration) = verify_password(&msg.old_password, &account.password_hash);
            if !ok {
                warn!("Old password mismatch for account: {}", msg.username);
                return;
            }
            account.password_hash = hash_password(&msg.new_password);
            if let Err(e) = db::change_password(&self.db_pool, &msg.username, &account.password_hash).await {
                warn!("Failed to change password for '{}': {}", msg.username, e);
            } else {
                info!("Password changed for account: {}", msg.username);
            }
        } else {
            warn!("Account '{}' not found for password change", msg.username);
        }
    }
}
