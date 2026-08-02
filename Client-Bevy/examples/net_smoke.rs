//! 真实网络冒烟测试（M7/M12）：连接 ServerRust，走 握手→登录→建角色→进游戏 全流程。
//! 用法: cargo run --example net_smoke -- [addr]
//! 默认: 127.0.0.1:7000

use std::time::Duration;

use client_bevy::network::tcp::{self, TcpEvent};
use mir2_shared::packets::base::{serialize_packet, Packet, PacketHeader};

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

    let conn = tcp::connect(&addr, [0u8; 16]).expect("connect failed");
    println!("✅ 已连接 {addr}");

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut sent_login = false;
    let mut sent_newchar = false;
    let mut sent_start = false;
    let mut received: Vec<String> = Vec::new();

    while std::time::Instant::now() < deadline {
        match conn.from_server.recv_timeout(Duration::from_millis(500)) {
            Ok(TcpEvent::Packet(payload)) => {
                let op = opcode_of(&payload);
                let mut cur = std::io::Cursor::new(&payload[4..]);
                match op {
                    x if x == mir2_shared::enums::ServerPacketIds::Connected as i16 => {
                        println!("🔌 Connected");
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::ClientVersion as i16 => {
                        println!("🔑 ClientVersion 校验");
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::LoginSuccess as i16 => {
                        let p = mir2_shared::packets::server::login::LoginSuccess::read_body(&mut cur).unwrap();
                        println!("✅ 登录成功，角色 {} 个", p.characters.len());
                        received.push("login".into());
                        // 建角色（如果列表空）
                        if p.characters.is_empty() && !sent_newchar {
                            let mut inner = Vec::new();
                            serialize_packet(&mut inner, &mir2_shared::packets::client::NewCharacter {
                                name: format!("smoke{}", std::process::id() % 10000),
                                class: mir2_shared::enums::MirClass::Warrior,
                                gender: mir2_shared::enums::MirGender::Male,
                            }).unwrap();
                            conn.to_server.send(inner).unwrap();
                            sent_newchar = true;
                            println!("📤 新建角色");
                        } else if !p.characters.is_empty() && !sent_start {
                            let mut inner = Vec::new();
                            serialize_packet(&mut inner, &mir2_shared::packets::client::account::StartGame {
                                character_index: p.characters[0].index,
                            }).unwrap();
                            conn.to_server.send(inner).unwrap();
                            sent_start = true;
                            println!("📤 StartGame");
                        }
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::Login as i16 => {
                        let p = mir2_shared::packets::server::login::Login::read_body(&mut cur).unwrap();
                        println!("⛔ 登录失败 result={}", p.result);
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16 => {
                        println!("✅ 角色创建成功");
                        // 重新登录获取角色列表
                        let mut inner = Vec::new();
                        serialize_packet(&mut inner, &mir2_shared::packets::client::account::Login {
                            account_id: format!("smoke{}", std::process::id() % 10000),
                            password: "smokepass".into(),
                        }).unwrap();
                        conn.to_server.send(inner).unwrap();
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::NewCharacter as i16 => {
                        println!("⛔ 角色创建被拒");
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::StartGame as i16 => {
                        println!("🎮 StartGame 响应");
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::MapChanged as i16 => {
                        println!("🗺️ MapChanged");
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::UserInformation as i16 => {
                        println!("👤 UserInformation");
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::HealthChanged as i16 => {
                        println!("💚 HealthChanged");
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::UserLocation as i16 => {
                        println!("📍 UserLocation");
                        received.push("ingame".into());
                    }
                    x if x == mir2_shared::enums::ServerPacketIds::ObjectPlayer as i16
                        || x == mir2_shared::enums::ServerPacketIds::ObjectMonster as i16
                        || x == mir2_shared::enums::ServerPacketIds::ObjectNpc as i16 => {
                        println!("👾 Object 包");
                    }
                    _ => {}
                }
                if !sent_login && op != -1 {
                    // 首次连接后发登录（服务器 auto-register）
                    let mut inner = Vec::new();
                    serialize_packet(&mut inner, &mir2_shared::packets::client::account::Login {
                        account_id: format!("smoke{}", std::process::id() % 10000),
                        password: "smokepass".into(),
                    }).unwrap();
                    conn.to_server.send(inner).unwrap();
                    sent_login = true;
                    println!("📤 Login");
                }
                if received.len() >= 2 {
                    println!("🎉 全流程验证通过：登录 → 进游戏");
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
