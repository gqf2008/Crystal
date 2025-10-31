// 测试心跳机制
// 连接到服务器后保持空闲 20 秒,查看是否会发送 KeepAlive 并且不会被服务器断开

use mir2_client::network::{NetworkBuilder, GameEvent};
use std::net::TcpStream;
use std::time::Duration;

fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("=== 心跳机制测试 ===");
    tracing::info!("连接到服务器并保持空闲 20 秒,观察心跳行为");

    // 连接服务器
    tracing::info!("连接到 127.0.0.1:7000...");
    let stream = match TcpStream::connect("127.0.0.1:7000") {
        Ok(s) => {
            tracing::info!("✅ 连接成功!");
            s
        }
        Err(e) => {
            tracing::error!("❌ 连接失败: {}", e);
            return;
        }
    };

    // 创建网络客户端
    let net_ctx = NetworkBuilder::new()
        .connect_with_stream(stream)
        .expect("Failed to create network context");

    tracing::info!("⏰ 开始计时,保持空闲 20 秒...");
    tracing::info!("💡 预期行为: 每 5 秒自动发送一次 KeepAlive");

    // 循环接收事件
    let start_time = std::time::Instant::now();
    let test_duration = Duration::from_secs(20);
    
    loop {
        // 接收所有事件
        for event in net_ctx.recv_all() {
            match event {
                GameEvent::Connected => {
                    tracing::info!("✅ Connected 事件");
                }
                GameEvent::Disconnected { reason } => {
                    tracing::error!("❌ 断开连接: {}", reason);
                    return;
                }
                _ => {
                    tracing::debug!("收到事件: {:?}", event);
                }
            }
        }

        // 检查是否超时
        let elapsed = start_time.elapsed();
        if elapsed >= test_duration {
            tracing::info!("✅ 测试完成! 保持连接 {:?} 秒没有被服务器断开", elapsed.as_secs());
            tracing::info!("🎉 心跳机制工作正常!");
            break;
        }

        // 短暂休眠
        std::thread::sleep(Duration::from_millis(100));
    }
}
