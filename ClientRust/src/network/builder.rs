// 网络模块 Builder 模式 - 简化版
// 
// 职责：
// 1. 连接服务器 (TcpStream)
// 2. 创建 Network（内部自动启动读写线程）
// 3. 返回 NetContext 给游戏层

use std::sync::Arc;
use std::net::TcpStream;
use parking_lot::RwLock;
use anyhow::Result;
use crossbeam_channel::{Sender, Receiver};

use super::handlers::GameEvent;
use super::client::Network;
use crate::settings::ClientSettings;

/// 网络上下文 - 游戏层唯一接口
pub struct NetContext {
    outgoing: Sender<GameEvent>,
    incoming: Receiver<GameEvent>,
}

impl NetContext {
    // ========== 基础方法 ==========
    
    /// 发送事件到网络
    #[inline]
    pub fn send(&self, event: GameEvent) -> Result<()> {
        self.outgoing.send(event)
            .map_err(|_| anyhow::anyhow!("Network disconnected"))
    }
    
    /// 接收所有待处理事件（非阻塞）
    #[inline]
    pub fn recv_all(&self) -> Vec<GameEvent> {
        self.incoming.try_iter().collect()
    }
    
    /// 尝试接收单个事件（非阻塞）
    #[inline]
    pub fn try_recv(&self) -> Option<GameEvent> {
        self.incoming.try_recv().ok()
    }
    
    // ========== 事件分类便利方法 ==========
    
    /// 接收所有事件并按类型分类
    pub fn recv_categorized(&self) -> CategorizedEvents {
        let mut result = CategorizedEvents::default();
        
        for event in self.incoming.try_iter() {
            match &event {
                // 连接相关
                GameEvent::Connected | GameEvent::Disconnected { .. } => {
                    result.connection.push(event);
                }
                
                // 认证相关
                GameEvent::LoginSuccess | GameEvent::LoginFailed { .. } 
                | GameEvent::NewAccountSuccess | GameEvent::NewAccountFailed { .. } => {
                    result.auth.push(event);
                }
                
                // 角色相关
                GameEvent::CharacterCreated { .. } | GameEvent::CharacterDeleted { .. }
                | GameEvent::StartGame { .. } => {
                    result.character.push(event);
                }
                
                // 玩家状态
                GameEvent::PlayerLocationChanged { .. } | GameEvent::HealthChanged { .. }
                | GameEvent::ManaChanged { .. } | GameEvent::ExperienceGained { .. }
                | GameEvent::LevelUp { .. } | GameEvent::GoldChanged { .. } => {
                    result.player_state.push(event);
                }
                
                // 战斗相关
                GameEvent::PlayerStruck { .. } | GameEvent::PlayerDied
                | GameEvent::ObjectStruck { .. } | GameEvent::ObjectDied { .. } => {
                    result.combat.push(event);
                }
                
                // 聊天消息
                GameEvent::ChatMessage { .. } | GameEvent::SystemMessage { .. } => {
                    result.chat.push(event);
                }
                
                // 世界对象
                GameEvent::ObjectSpawned { .. } | GameEvent::ObjectRemoved { .. }
                | GameEvent::ObjectMoved { .. } => {
                    result.world_objects.push(event);
                }
                
                // 地图相关
                GameEvent::MapChanged { .. } => {
                    result.map.push(event);
                }
                
                // 物品相关
                GameEvent::ItemGained { .. } | GameEvent::ItemLost { .. }
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
        self.recv_all().into_iter()
            .filter(|e| matches!(e, 
                GameEvent::Connected | GameEvent::Disconnected { .. }
            ))
            .collect()
    }
    
    /// 只接收聊天消息
    pub fn recv_chat_messages(&self) -> Vec<GameEvent> {
        self.recv_all().into_iter()
            .filter(|e| matches!(e, 
                GameEvent::ChatMessage { .. } | GameEvent::SystemMessage { .. }
            ))
            .collect()
    }
    
    /// 只接收战斗事件
    pub fn recv_combat_events(&self) -> Vec<GameEvent> {
        self.recv_all().into_iter()
            .filter(|e| matches!(e,
                GameEvent::PlayerStruck { .. } | GameEvent::PlayerDied 
                | GameEvent::ObjectStruck { .. } | GameEvent::ObjectDied { .. }
            ))
            .collect()
    }
    
    /// 检查是否有连接事件
    pub fn has_connection_events(&self) -> bool {
        self.incoming.try_iter().any(|e| matches!(e,
            GameEvent::Connected | GameEvent::Disconnected { .. }
        ))
    }
    
    /// 检查是否收到登录成功
    pub fn check_login_success(&self) -> bool {
        self.incoming.try_iter().any(|e| matches!(e, GameEvent::LoginSuccess))
    }
}

/// 分类的网络事件
#[derive(Default)]
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

/// 网络构建器
pub struct NetworkBuilder {
    settings: Arc<RwLock<ClientSettings>>,
}

impl NetworkBuilder {
    pub fn new(settings: Arc<RwLock<ClientSettings>>) -> Self {
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
        let network_settings = self.settings.read().network.clone();
        let addr = format!("{}:{}", network_settings.ip_address, network_settings.port);
        
        tracing::info!("Connecting to {}...", addr);
        let stream = TcpStream::connect(&addr)?;
        stream.set_nodelay(true)?;
        tracing::info!("Connected to {}", addr);
        
        // 2. 创建 Network（自动启动读写线程）
        let (tx, rx) = Network::new(stream);
        
        // 3. 返回 NetContext
        Ok(NetContext {
            outgoing: tx,
            incoming: rx,
        })
    }
}
