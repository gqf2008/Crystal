// 网络模块 Builder 模式 - 简化版
//
// 职责：
// 1. 连接服务器 (TcpStream)
// 2. 创建 Network（内部自动启动读写线程）
// 3. 返回 NetContext 给游戏层
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use std::net::TcpStream;
use super::client::Network;
use super::handlers::GameEvent;
use crate::settings::NetworkSettings;



/// 网络上下文 - 游戏层唯一接口
#[derive(Clone)]
pub struct NetContext {
    outbound: Sender<GameEvent>,
    inbound: Receiver<GameEvent>,
}

impl NetContext {
    /// 发送事件到网络
    #[inline]
    pub fn send(&self, event: GameEvent) -> Result<()> {
        self.outbound
            .send(event)
            .map_err(|_| anyhow::anyhow!("Network disconnected"))
    }

    // pub fn outbound_sender(&self) -> Sender<GameEvent> {
    //     self.outbound.clone()
    // }

    // pub fn inbound_receiver(&self) -> Receiver<GameEvent> {
    //     self.inbound.clone()
    // }

    /// 接收所有待处理事件（非阻塞）
    #[inline]
    pub fn recv_all(&self) -> Vec<GameEvent> {
        self.inbound.try_iter().collect()
    }

    /// 尝试接收单个事件（非阻塞）
    #[inline]
    pub fn try_recv(&self) -> Option<GameEvent> {
        self.inbound.try_recv().ok()
    }

    // ========== 事件分类便利方法 ==========

    /// 接收所有事件并按类型分类
    pub fn recv_categorized(&self) -> CategorizedEvents {
        let mut result = CategorizedEvents::default();

        for event in self.inbound.try_iter() {
            match &event {
                // 连接相关
                GameEvent::Connected | GameEvent::Disconnected { .. } => {
                    result.connection.push(event);
                }

                // 认证相关
                GameEvent::LoginSuccess { .. }
                | GameEvent::LoginFailed { .. }
                | GameEvent::NewAccountSuccess
                | GameEvent::NewAccountFailed { .. } => {
                    result.auth.push(event);
                }

                // 角色相关
                GameEvent::CharacterCreated { .. }
                | GameEvent::CharacterDeleted { .. }
                | GameEvent::StartGame { .. } => {
                    result.character.push(event);
                }

                // 玩家状态
                GameEvent::PlayerLocationChanged { .. }
                | GameEvent::HealthChanged { .. }
                | GameEvent::ManaChanged { .. }
                | GameEvent::ExperienceGained { .. }
                | GameEvent::LevelUp { .. }
                | GameEvent::GoldChanged { .. } => {
                    result.player_state.push(event);
                }

                // 战斗相关
                GameEvent::PlayerStruck { .. }
                | GameEvent::PlayerDied
                | GameEvent::ObjectStruck { .. }
                | GameEvent::ObjectDied { .. } => {
                    result.combat.push(event);
                }

                // 聊天消息
                GameEvent::ChatMessage { .. } | GameEvent::SystemMessage { .. } => {
                    result.chat.push(event);
                }

                // 世界对象
                GameEvent::ObjectSpawned { .. }
                | GameEvent::ObjectRemoved { .. }
                | GameEvent::ObjectMoved { .. } => {
                    result.world_objects.push(event);
                }

                // 地图相关
                GameEvent::MapChanged { .. } => {
                    result.map.push(event);
                }

                // 物品相关
                GameEvent::ItemGained { .. }
                | GameEvent::ItemLost { .. }
                | GameEvent::ItemMoved { .. } => {
                    result.items.push(event);
                }

                // NPC相关
                GameEvent::NpcDialog { .. } | GameEvent::NPCGoods { .. } => {
                    result.npc.push(event);
                }

                // 其他
                _ => {
                    result.other.push(event);
                }
            }
        }

        result
    }

    /// 只接收连接状态事件
    pub fn recv_connection_events(&self) -> Vec<GameEvent> {
        self.recv_all()
            .into_iter()
            .filter(|e| matches!(e, GameEvent::Connected | GameEvent::Disconnected { .. }))
            .collect()
    }

    /// 只接收聊天消息
    pub fn recv_chat_messages(&self) -> Vec<GameEvent> {
        self.recv_all()
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    GameEvent::ChatMessage { .. } | GameEvent::SystemMessage { .. }
                )
            })
            .collect()
    }

    /// 只接收战斗事件
    pub fn recv_combat_events(&self) -> Vec<GameEvent> {
        self.recv_all()
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    GameEvent::PlayerStruck { .. }
                        | GameEvent::PlayerDied
                        | GameEvent::ObjectStruck { .. }
                        | GameEvent::ObjectDied { .. }
                )
            })
            .collect()
    }

    /// 检查是否有连接事件
    /// 
    /// ⚠️ 注意：此方法会消费 channel 中的所有事件来进行检查
    /// 如果需要保留事件，请使用 `recv_connection_events()` 获取事件列表
    pub fn has_connection_events(&self) -> bool {
        self.inbound
            .try_iter()
            .any(|e| matches!(e, GameEvent::Connected | GameEvent::Disconnected { .. }))
    }

    /// 检查是否收到登录成功
    /// 
    /// ⚠️ 注意：此方法会消费 channel 中的所有事件来进行检查
    /// 如果需要保留事件，请使用 `recv_all()` 并手动过滤
    pub fn check_login_success(&self) -> bool {
        self.inbound
            .try_iter()
            .any(|e| matches!(e, GameEvent::LoginSuccess { .. }))
    }
}

/// 分类的网络事件
#[derive(Default,Clone)]
pub struct CategorizedEvents {
    pub connection: Vec<GameEvent>,
    pub auth: Vec<GameEvent>,
    pub character: Vec<GameEvent>,
    pub player_state: Vec<GameEvent>,
    pub combat: Vec<GameEvent>,
    pub chat: Vec<GameEvent>,
    pub world_objects: Vec<GameEvent>,
    pub map: Vec<GameEvent>,
    pub items: Vec<GameEvent>,
    pub npc: Vec<GameEvent>,
    pub other: Vec<GameEvent>,
}

impl CategorizedEvents {
    /// 获取所有事件的总数
    pub fn total_count(&self) -> usize {
        self.connection.len()
            + self.auth.len()
            + self.character.len()
            + self.player_state.len()
            + self.combat.len()
            + self.chat.len()
            + self.world_objects.len()
            + self.map.len()
            + self.items.len()
            + self.npc.len()
            + self.other.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }

    pub fn clear(&mut self) {
        self.connection.clear();
        self.auth.clear();
        self.character.clear();
        self.player_state.clear();
        self.combat.clear();
        self.chat.clear();
        self.world_objects.clear();
        self.map.clear();
        self.items.clear();
        self.npc.clear();
        self.other.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &GameEvent> {
        self.connection
            .iter()
            .chain(self.auth.iter())
            .chain(self.character.iter())
            .chain(self.player_state.iter())
            .chain(self.combat.iter())
            .chain(self.chat.iter())
            .chain(self.world_objects.iter())
            .chain(self.map.iter())
            .chain(self.items.iter())
            .chain(self.npc.iter())
            .chain(self.other.iter())
    }

    
}

/// 网络构建器
pub struct NetworkBuilder {
    settings: NetworkSettings,
}

impl NetworkBuilder {
    pub fn new(settings: NetworkSettings) -> Self {
        Self { settings }
    }

    /// 构建网络模块
    ///
    /// 步骤：
    /// 1. 连接服务器 (TcpStream)
    /// 2. 创建 Network（自动启动读写线程）
    /// 3. 返回 NetContext
    pub fn build(self) -> Result<NetContext> {
        // 1. 连接服务器
        let addr = format!("{}:{}", self.settings.ip_address, self.settings.port);
        tracing::info!("Connecting to {}...", addr);
        let w = TcpStream::connect(&addr)?;
        w.set_nodelay(true)?;
        let r = w.try_clone()?;
        tracing::info!("Connected to {}", addr);

        // 2. 创建 Network（自动启动读写线程）
        let (tx, rx) = Network::new((w, r));

        // 3. 返回 NetContext
        Ok(NetContext {
            outbound: tx,
            inbound: rx,
        })
    }
}
