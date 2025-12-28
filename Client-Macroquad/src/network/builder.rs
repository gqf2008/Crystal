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
use super::handlers::NetworkEvent;



/// 网络上下文 - 游戏层唯一接口
#[derive(Clone)]
pub struct NetContext {
    outbound: Sender<NetworkEvent>,
    inbound: Receiver<NetworkEvent>,
}

impl NetContext {
    /// 创建空的 NetContext (用于测试/占位)
    /// 
    /// 注意：这个上下文无法实际发送或接收网络数据
    /// 正常使用请通过 NetworkBuilder::build() 创建
    pub fn new() -> Self {
        use crossbeam_channel::unbounded;
        let (tx, rx) = unbounded();
        Self {
            outbound: tx,
            inbound: rx,
        }
    }
    
    /// 发送事件到网络
    #[inline]
    pub fn send(&self, event: NetworkEvent) -> Result<()> {
        self.outbound
            .send(event)
            .map_err(|_| anyhow::anyhow!("Network disconnected"))
    }

    // pub fn outbound_sender(&self) -> Sender<NetworkEvent> {
    //     self.outbound.clone()
    // }

    // pub fn inbound_receiver(&self) -> Receiver<NetworkEvent> {
    //     self.inbound.clone()
    // }

    /// 接收所有待处理事件（非阻塞）
    #[inline]
    pub fn recv_all(&self) -> Vec<NetworkEvent> {
        self.inbound.try_iter().collect()
    }

    /// 尝试接收单个事件（非阻塞）
    #[inline]
    pub fn try_recv(&self) -> Option<NetworkEvent> {
        self.inbound.try_recv().ok()
    }

    // ========== 事件分类便利方法 ==========

    /// 接收所有事件并按类型分类
    pub fn recv_categorized(&self) -> CategorizedEvents {
        let mut result = CategorizedEvents::default();

        for event in self.inbound.try_iter() {
            match &event {
                // 连接相关
                NetworkEvent::Connected | NetworkEvent::Disconnected { .. } => {
                    result.connection.push(event);
                }

                // 认证相关
                NetworkEvent::LoginSuccess { .. }
                | NetworkEvent::LoginFailed { .. }
                | NetworkEvent::NewAccountSuccess
                | NetworkEvent::NewAccountFailed { .. } => {
                    result.auth.push(event);
                }

                // 角色相关
                NetworkEvent::CharacterCreated { .. }
                | NetworkEvent::CharacterDeleted { .. }
                | NetworkEvent::StartGame { .. }
                | NetworkEvent::StartGameDelay { .. }
                | NetworkEvent::StartGameBanned { .. }
                | NetworkEvent::UserInformation { .. } => {
                    result.character.push(event);
                }

                // 玩家状态
                NetworkEvent::PlayerLocationChanged { .. }
                | NetworkEvent::HealthChanged { .. }
                | NetworkEvent::ManaChanged { .. }
                | NetworkEvent::ExperienceGained { .. }
                | NetworkEvent::LevelUp { .. }
                | NetworkEvent::GoldChanged { .. } => {
                    result.player_state.push(event);
                }

                // 战斗相关
                NetworkEvent::PlayerStruck { .. }
                | NetworkEvent::PlayerDied
                | NetworkEvent::ObjectStruck { .. }
                | NetworkEvent::ObjectDied { .. } => {
                    result.combat.push(event);
                }

                // 聊天消息
                NetworkEvent::ChatMessage { .. } | NetworkEvent::SystemMessage { .. } => {
                    result.chat.push(event);
                }

                // 世界对象
                NetworkEvent::ObjectSpawned { .. }
                | NetworkEvent::ObjectRemoved { .. }
                | NetworkEvent::ObjectMoved { .. } => {
                    result.world_objects.push(event);
                }

                // 地图相关
                NetworkEvent::MapInformation { .. } | NetworkEvent::MapChanged { .. } => {
                    result.map.push(event);
                }

                // 物品相关
                NetworkEvent::ItemGained { .. }
                | NetworkEvent::ItemLost { .. }
                | NetworkEvent::ItemMoved { .. } => {
                    result.items.push(event);
                }

                // NPC相关
                NetworkEvent::NpcDialog { .. } | NetworkEvent::NPCGoods { .. } => {
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
    pub fn recv_connection_events(&self) -> Vec<NetworkEvent> {
        self.recv_all()
            .into_iter()
            .filter(|e| matches!(e, NetworkEvent::Connected | NetworkEvent::Disconnected { .. }))
            .collect()
    }

    /// 只接收聊天消息
    pub fn recv_chat_messages(&self) -> Vec<NetworkEvent> {
        self.recv_all()
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    NetworkEvent::ChatMessage { .. } | NetworkEvent::SystemMessage { .. }
                )
            })
            .collect()
    }

    /// 只接收战斗事件
    pub fn recv_combat_events(&self) -> Vec<NetworkEvent> {
        self.recv_all()
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    NetworkEvent::PlayerStruck { .. }
                        | NetworkEvent::PlayerDied
                        | NetworkEvent::ObjectStruck { .. }
                        | NetworkEvent::ObjectDied { .. }
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
            .any(|e| matches!(e, NetworkEvent::Connected | NetworkEvent::Disconnected { .. }))
    }

    /// 检查是否收到登录成功
    /// 
    /// ⚠️ 注意：此方法会消费 channel 中的所有事件来进行检查
    /// 如果需要保留事件，请使用 `recv_all()` 并手动过滤
    pub fn check_login_success(&self) -> bool {
        self.inbound
            .try_iter()
            .any(|e| matches!(e, NetworkEvent::LoginSuccess { .. }))
    }
}

/// 分类的网络事件
#[derive(Default,Clone)]
pub struct CategorizedEvents {
    pub connection: Vec<NetworkEvent>,
    pub auth: Vec<NetworkEvent>,
    pub character: Vec<NetworkEvent>,
    pub player_state: Vec<NetworkEvent>,
    pub combat: Vec<NetworkEvent>,
    pub chat: Vec<NetworkEvent>,
    pub world_objects: Vec<NetworkEvent>,
    pub map: Vec<NetworkEvent>,
    pub items: Vec<NetworkEvent>,
    pub npc: Vec<NetworkEvent>,
    pub other: Vec<NetworkEvent>,
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

    pub fn iter(&self) -> impl Iterator<Item = &NetworkEvent> {
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
    server_addr: String,
    use_mock: bool,
    client_version_hash: [u8; 16],
}

impl NetworkBuilder {
    pub fn new(server_addr: String) -> Self {
        Self {
            server_addr,
            use_mock: false,
            client_version_hash: [0u8; 16],
        }
    }

    pub fn with_mock(mut self, use_mock: bool) -> Self {
        self.use_mock = use_mock;
        self
    }

    pub fn with_client_version_hash(mut self, hash: [u8; 16]) -> Self {
        self.client_version_hash = hash;
        self
    }

   

    /// 构建网络模块
    ///
    /// 根据配置返回真实网络或模拟网络：
    /// - mock(false): 连接真实服务器 (TcpStream + Network)
    /// - mock(true): 使用模拟网络 (MockNetwork)
    ///
    /// 两种模式返回相同的 NetContext 接口
    pub fn build(self) -> Result<NetContext> {
        if self.use_mock {
            // 模拟网络模式
            tracing::info!("🎭 使用模拟网络模式");
            let (tx, rx) = super::mock::MockNetwork::new();
            Ok(NetContext {
                outbound: tx,
                inbound: rx,
            })
        } else {
            // 真实网络模式
            // 1. 连接服务器
            let addr = &self.server_addr;
            tracing::info!("Connecting to {}...", addr);
            let w = TcpStream::connect(&addr)?;
            // NOTE: Crystal 服务器端的洪泛保护 `MaxPacket` 实际统计的是 5 秒窗口内的 socket receive 回调次数。
            // 如果开启 TCP_NODELAY，小包会更容易被拆成多个 TCP 段，导致服务端计数飙升并误判为 "Large amount of Packets"。
            // 这里保持 Nagle 开启（默认），让小包尽可能合并。
            w.set_nodelay(false)?;
            let r = w.try_clone()?;
            tracing::info!("Connected to {}", addr);

            // 2. 创建 Network（自动启动读写线程）
            let (tx, rx) = Network::new((w, r), self.client_version_hash);

            // 3. 返回 NetContext
            Ok(NetContext {
                outbound: tx,
                inbound: rx,
            })
        }
    }
}
