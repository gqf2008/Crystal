//! 真实网络冒烟测试（M7）：连接 ServerRust，走 握手→登录 流程，打印收到的服务器包。
//! 用法: cargo run --example net_smoke -- [addr] [account] [password]
//! 默认: 127.0.0.1:7000 smoketest x（故意用错密码，验证 Login 失败包能收到）

use std::time::Duration;

use client_bevy::network::tcp::{self, TcpEvent};
use mir2_shared::packets::base::{serialize_packet, PacketHeader};

fn opcode_of(payload: &[u8]) -> i16 {
    let mut cur = std::io::Cursor::new(payload);
    PacketHeader::read_from(&mut cur).map(|h| h.opcode).unwrap_or(-1)
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let addr = args.get(1).cloned().unwrap_or_else(|| "127.0.0.1:7000".to_string());
    let account = args.get(2).cloned().unwrap_or_else(|| "smoketest".to_string());
    let password = args.get(3).cloned().unwrap_or_else(|| "wrongpass".to_string());

    let conn = tcp::connect(&addr, [0u8; 16]).expect("connect failed");
    println!("✅ 已连接 {addr}，等待 Connected…");

    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    let mut sent_login = false;
    while std::time::Instant::now() < deadline {
        match conn.from_server.recv_timeout(Duration::from_millis(500)) {
            Ok(TcpEvent::Packet(payload)) => {
                let op = opcode_of(&payload);
                println!("📥 服务器包 opcode={op} ({payload:02x?})", payload = payload.iter().take(16).collect::<Vec<_>>());
                if op == mir2_shared::enums::ServerPacketIds::Connected as i16 {
                    println!("🔌 Connected 到达（ClientVersion 已自动发送）");
                }
                if !sent_login {
                    // Connected 或 ClientVersion 响应后发 Login
                    let mut inner = Vec::new();
                    serialize_packet(
                        &mut inner,
                        &mir2_shared::packets::client::account::Login {
                            account_id: account.clone(),
                            password: password.clone(),
                        },
                    )
                    .unwrap();
                    conn.to_server.send(inner).unwrap();
                    println!("📤 发送 Login({account})");
                    sent_login = true;
                }
                if op == mir2_shared::enums::ServerPacketIds::Login as i16
                    || op == mir2_shared::enums::ServerPacketIds::LoginSuccess as i16
                {
                    println!("✅ 流程完成，收到登录结果包");
                    break;
                }
            }
            Ok(TcpEvent::Disconnected { reason }) => {
                println!("🔌 断开: {reason}");
                break;
            }
            Err(_) => {}
        }
    }
    println!("冒烟测试结束");
}
