// AccountActor - 账号认证服务
// 对应 C# LoginSrv/AccountManager.cs

use std::collections::HashMap;

use kameo::actor::{Actor, ActorRef};
use kameo::prelude::Context;
use kameo::message::Message;
use tracing::{info, warn};

use crate::gate::actor::LoginResult;

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
}

impl AccountActor {
    pub fn new(gate_ref: ActorRef<crate::gate::actor::GateActor>) -> Self {
        Self {
            accounts: HashMap::new(),
            gate_ref,
        }
    }

    /// 注册账号（Phase 1：内存存储）
    pub fn register(&mut self, username: &str, password: &str) -> bool {
        if self.accounts.contains_key(username) {
            warn!("Account already exists: {}", username);
            return false;
        }

        self.accounts.insert(
            username.to_string(),
            AccountInfo {
                username: username.to_string(),
                password_hash: password.to_string(), // Phase 1 不哈希，后续替换
                is_online: false,
            },
        );

        info!("Account registered: {}", username);
        true
    }

    /// 登录验证
    pub fn login(&mut self, username: &str, password: &str) -> bool {
        if let Some(account) = self.accounts.get_mut(username) {
            if account.password_hash != password {
                warn!("Wrong password for account: {}", username);
                return false;
            }
            if account.is_online {
                warn!("Account already online: {}", username);
                return false;
            }
            account.is_online = true;
            info!("Account logged in: {}", username);
            true
        } else {
            // Phase 1：自动注册不存在的账号
            info!("Auto-registering account: {}", username);
            self.register(username, password);
            self.login(username, password)
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
    type Args = ActorRef<crate::gate::actor::GateActor>;
    type Error = anyhow::Error;

    async fn on_start(gate_ref: ActorRef<crate::gate::actor::GateActor>, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        info!("AccountActor started");
        Ok(Self::new(gate_ref))
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
        let success = self.login(&msg.username, &msg.password);
        info!("Login result for '{}': {}", msg.username, success);

        // 将结果发回 GateActor，由 GateActor 发送协议包给客户端
        let _ = self.gate_ref.ask(LoginResult {
            session_id: msg.session_id,
            success,
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
    }
}
