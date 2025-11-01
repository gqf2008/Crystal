// ============================================================================
// Mock Network - 模拟网络实现（用于开发工具和离线测试）
// ============================================================================
//
// 提供完全本地的网络模拟，无需真实服务器：
// - 模拟连接/断开
// - 模拟角色数据
// - 模拟地图数据
// - 模拟基本的游戏事件响应
//
// 使用方式：
//   let net_ctx = NetworkBuilder::new(settings)
//       .mock(true)
//       .build()?;
//
// ============================================================================

use super::handlers::GameEvent;
use crate::objects::map_code::MapReader;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// 模拟网络实现
pub struct MockNetwork {
    /// 线程是否运行
    running: Arc<AtomicBool>,
    /// 接收游戏层发送的事件
    game_tx: Sender<GameEvent>,
    /// 游戏层接收事件的通道
    game_rx: Receiver<GameEvent>,
    /// 模拟网络线程句柄
    _handle: Option<thread::JoinHandle<()>>,
}

impl MockNetwork {
    /// 创建新的模拟网络
    ///
    /// # 返回
    /// (发送通道, 接收通道) - 供 NetContext 使用
    pub fn new() -> (Sender<GameEvent>, Receiver<GameEvent>) {
        let (client_tx, mock_rx) = crossbeam_channel::unbounded();
        let (mock_tx, client_rx) = crossbeam_channel::unbounded();

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // 启动模拟网络线程
        let handle = thread::spawn(move || {
            tracing::info!("🌐 MockNetwork 启动");

            // 立即发送连接成功事件
            let _ = mock_tx.send(GameEvent::Connected);

            while running_clone.load(Ordering::Relaxed) {
                match mock_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => {
                        Self::handle_game_event(event, &mock_tx);
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        // 正常超时，继续循环
                        continue;
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        tracing::info!("🔌 客户端断开连接");
                        break;
                    }
                }
            }

            tracing::info!("🛑 MockNetwork 关闭");
        });

        // 将 MockNetwork 实例泄漏到静态生命周期，防止被Drop
        // 这样线程可以一直运行到程序结束
        let mock = MockNetwork {
            running,
            game_tx: client_tx.clone(),
            game_rx: client_rx.clone(),
            _handle: Some(handle),
        };
        
        // 使用 Box::leak 防止 Drop
        let _ = Box::leak(Box::new(mock));

        // 返回通道供 NetContext 使用
        (client_tx, client_rx)
    }

    /// 处理游戏层发送的事件
    fn handle_game_event(event: GameEvent, response_tx: &Sender<GameEvent>) {
        tracing::debug!("📥 MockNetwork 收到事件: {:?}", event);

        match event {
            // 断开请求
            GameEvent::DisconnectRequest => {
                tracing::info!("👋 模拟断开连接");
                let _ = response_tx.send(GameEvent::Disconnected {
                    reason: "User requested".to_string(),
                });
            }

            // 登录请求
            GameEvent::LoginRequest { username, .. } => {
                tracing::info!("🔐 模拟登录: {}", username);
                // 延迟一点模拟网络延迟
                thread::sleep(Duration::from_millis(100));
                // 返回空角色列表
                let _ = response_tx.send(GameEvent::LoginSuccess {
                    characters: vec![],
                });
            }

            // 新建账号请求
            GameEvent::NewAccountRequest { account_id, .. } => {
                tracing::info!("📝 模拟创建账号: {}", account_id);
                thread::sleep(Duration::from_millis(100));
                let _ = response_tx.send(GameEvent::NewAccountSuccess);
            }

            // 创建角色请求
            GameEvent::NewCharacterRequest { name, .. } => {
                tracing::info!("🧙 模拟创建角色: {}", name);
                thread::sleep(Duration::from_millis(100));
                let _ = response_tx.send(GameEvent::CharacterCreated {
                    name: name.clone(),
                });
            }

            // 删除角色请求
            GameEvent::DeleteCharacterRequest { index } => {
                tracing::info!("🗑️ 模拟删除角色: {}", index);
                thread::sleep(Duration::from_millis(100));
                let _ = response_tx.send(GameEvent::CharacterDeleted {
                    index: index as u32,
                });
            }

            // 开始游戏请求
            GameEvent::StartGameRequest { character_index } => {
                tracing::info!("🎮 模拟开始游戏: 角色索引 {}", character_index);
                thread::sleep(Duration::from_millis(200));

                // 发送开始游戏响应
                let _ = response_tx.send(GameEvent::StartGame { delay: 500 });

                // 加载地图并发送 MapChanged 事件
                Self::load_and_send_map(&response_tx, "Map/0.map", 0, "比奇城");

                // 模拟玩家信息
                let _ = response_tx.send(GameEvent::UserInformation {
                    location_x: 330,
                    location_y: 330,
                    hp: 100,
                    mp: 50,
                    gold: 1000,
                });
            }

            // 移动请求
            GameEvent::MoveRequest { .. }
            | GameEvent::WalkRequest { .. }
            | GameEvent::RunRequest { .. } => {
                tracing::debug!("🚶 模拟移动");
                // 暂不响应具体位置，由客户端自己计算
            }

            // 聊天请求
            GameEvent::ChatRequest { message, chat_type } => {
                tracing::info!("💬 模拟发送聊天: {}", message);
                // 回显消息
                let _ = response_tx.send(GameEvent::ChatMessage {
                    sender: "MockServer".to_string(),
                    message: format!("Echo: {}", message),
                    chat_type,
                });
            }

            // 其他事件暂不处理
            _ => {
                tracing::debug!("⚠️ MockNetwork 暂不处理事件: {:?}", event);
            }
        }
    }

    /// 加载地图并发送 MapChanged 事件
    fn load_and_send_map(response_tx: &Sender<GameEvent>, map_path: &str, map_index: i32, title: &str) {
        tracing::info!("📂 尝试加载地图: {}", map_path);
        
        match MapReader::new(map_path) {
            Ok(map_reader) => {
                tracing::info!(
                    "✅ 成功加载地图: {} ({}x{})",
                    map_path,
                    map_reader.width,
                    map_reader.height
                );
                
                // 发送 MapChanged 事件
                let _ = response_tx.send(GameEvent::MapChanged {
                    map_index,
                    file_name: map_path.to_string(),
                    title: title.to_string(),
                });
                
                // TODO: 这里需要将 MapReader 数据发送给客户端
                // 目前暂时只发送事件，MapReader 需要在游戏循环中处理
            }
            Err(e) => {
                tracing::error!("❌ 加载地图失败 {}: {:?}", map_path, e);
            }
        }
    }

    /// 停止模拟网络
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for MockNetwork {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        tracing::debug!("MockNetwork 实例销毁");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_network_connection() {
        let (tx, rx) = MockNetwork::new();

        // 等待自动发送的 Connected 事件
        thread::sleep(Duration::from_millis(200));

        // 应该收到连接成功事件
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events.iter().any(|e| matches!(e, GameEvent::Connected)));

        // 发送断开请求
        tx.send(GameEvent::DisconnectRequest).unwrap();
        thread::sleep(Duration::from_millis(200));

        // 应该收到断开事件
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::Disconnected { .. })));
    }

    #[test]
    fn test_mock_network_login() {
        let (tx, rx) = MockNetwork::new();

        // 发送登录请求
        tx.send(GameEvent::LoginRequest {
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
        })
        .unwrap();

        thread::sleep(Duration::from_millis(300));

        // 应该收到登录成功事件
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::LoginSuccess { .. })));
    }
}
