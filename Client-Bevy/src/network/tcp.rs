// ============================================================================
// 真实 TCP 网络客户端（迁移计划 M7）
//
// 与 Client-Macroquad/src/network/client.rs 的读写双线程设计保持一致，
// 但只做帧传输，不做业务解析：
//   - 写线程：接收游戏层序列化好的内层包 [inner_len][opcode][body]，
//     用 codec 加外层帧后写出；5 秒无包自动发 KeepAlive 心跳。
//   - 读线程：codec 解码后把完整内层包交给游戏线程；
//     收到服务器 Connected 后自动发送 ClientVersion 完成握手。
//   - 断线（EOF/IO 错误）→ 发 TcpEvent::Disconnected 通知游戏层。
// ============================================================================

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use mir2_shared::packets::base::{serialize_packet, PacketHeader};

use crate::network::codec;

/// 心跳间隔：5 秒无包自动发送 KeepAlive（服务端超时通常是 10 秒）
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// 读线程从 TCP 收到的事件
pub enum TcpEvent {
    /// 完整内层包 [inner_len(2)][opcode(2)][body]
    Packet(Vec<u8>),
    /// 连接断开（EOF / IO 错误 / 帧解码错误）
    Disconnected { reason: String },
}

/// 真实 TCP 连接（游戏线程侧句柄）
pub struct TcpConnection {
    /// 发往服务器的内层包
    pub to_server: Sender<Vec<u8>>,
    /// 服务器数据：完整内层包或断线通知
    pub from_server: Receiver<TcpEvent>,
}

/// 建立到服务器的 TCP 连接并启动读写线程。
///
/// `client_version_hash`：ClientVersion 的 16 字节版本哈希
/// （服务端开启 CheckVersion 时必须匹配 Settings.VersionHashes）。
pub fn connect(addr: &str, client_version_hash: [u8; 16]) -> std::io::Result<TcpConnection> {
    let stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true).ok();
    let read_stream = stream.try_clone()?;

    let (to_server, write_rx) = bounded::<Vec<u8>>(1024);
    let (from_tx, from_server) = bounded::<TcpEvent>(1024);
    let shutdown = Arc::new(AtomicBool::new(false));

    // 读线程
    {
        let tx = from_tx.clone();
        let to_write = to_server.clone();
        let shutdown = shutdown.clone();
        std::thread::Builder::new()
            .name("bevy-net-read".into())
            .spawn(move || {
                read_loop(read_stream, tx, to_write, client_version_hash, shutdown);
            })
            .expect("failed to spawn net read thread");
    }

    // 写线程
    {
        let tx = from_tx;
        let shutdown = shutdown.clone();
        std::thread::Builder::new()
            .name("bevy-net-write".into())
            .spawn(move || {
                write_loop(stream, write_rx, tx, shutdown);
            })
            .expect("failed to spawn net write thread");
    }

    Ok(TcpConnection {
        to_server,
        from_server,
    })
}

/// 读线程：读 TCP → codec 解码 → 完整内层包 → 游戏线程
fn read_loop(
    mut stream: TcpStream,
    tx: Sender<TcpEvent>,
    to_write: Sender<Vec<u8>>,
    client_version_hash: [u8; 16],
    shutdown: Arc<AtomicBool>,
) {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];

    loop {
        let n = match stream.read(&mut chunk) {
            Ok(0) => {
                shutdown.store(true, Ordering::Relaxed);
                tracing::warn!("🔌 服务器关闭连接（EOF）");
                let _ = tx.send(TcpEvent::Disconnected {
                    reason: "服务器关闭连接（EOF）".to_string(),
                });
                break;
            }
            Ok(n) => n,
            Err(e) => {
                shutdown.store(true, Ordering::Relaxed);
                tracing::error!("🔌 读网络错误: {}", e);
                let _ = tx.send(TcpEvent::Disconnected {
                    reason: format!("读网络错误: {}", e),
                });
                break;
            }
        };
        buf.extend_from_slice(&chunk[..n]);

        // 解码所有完整帧
        loop {
            match codec::decode(&buf) {
                Some(Ok((payload, consumed))) => {
                    buf.drain(..consumed);

                    // 握手：服务器 Connected 到达后自动发 ClientVersion
                    if payload.len() >= PacketHeader::HEADER_SIZE {
                        let mut cur = std::io::Cursor::new(&payload[..]);
                        if let Ok(header) = PacketHeader::read_from(&mut cur) {
                            if header.opcode == mir2_shared::enums::ServerPacketIds::Connected as i16 {
                                let mut inner = Vec::new();
                                if serialize_packet(
                                    &mut inner,
                                    &mir2_shared::packets::client::connection::ClientVersion {
                                        version_hash: client_version_hash.to_vec(),
                                    },
                                )
                                .is_ok()
                                {
                                    tracing::info!("📤 自动发送 ClientVersion（握手）");
                                    let _ = to_write.send(inner);
                                }
                            }
                        }
                    }

                    if tx.send(TcpEvent::Packet(payload)).is_err() {
                        tracing::warn!("游戏线程已退出，读线程停止");
                        shutdown.store(true, Ordering::Relaxed);
                        return;
                    }
                }
                Some(Err(e)) => {
                    shutdown.store(true, Ordering::Relaxed);
                    tracing::error!("🔌 帧解码错误: {}", e);
                    let _ = tx.send(TcpEvent::Disconnected {
                        reason: format!("帧解码错误: {}", e),
                    });
                    break;
                }
                None => break, // 数据不足
            }
        }
    }
}

/// 写线程：接收内层包 → codec 外帧 → 写出；超时自动心跳
fn write_loop(
    mut stream: TcpStream,
    rx: Receiver<Vec<u8>>,
    tx: Sender<TcpEvent>,
    shutdown: Arc<AtomicBool>,
) {
    loop {
        if shutdown.load(Ordering::Relaxed) {
            tracing::info!("写线程停止（收到关闭信号）");
            return;
        }

        match rx.recv_timeout(HEARTBEAT_INTERVAL) {
            Ok(inner) => {
                if write_inner(&mut stream, &inner).is_err() {
                    shutdown.store(true, Ordering::Relaxed);
                    let _ = tx.send(TcpEvent::Disconnected {
                        reason: "写网络错误".to_string(),
                    });
                    return;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if shutdown.load(Ordering::Relaxed) {
                    return;
                }
                // 游戏层 5 秒没发包 → 心跳
                let mut inner = Vec::new();
                if serialize_packet(
                    &mut inner,
                    &mir2_shared::packets::client::connection::KeepAlive {
                        time: chrono::Utc::now().timestamp_millis(),
                    },
                )
                .is_ok()
                {
                    if write_inner(&mut stream, &inner).is_err() {
                        shutdown.store(true, Ordering::Relaxed);
                        let _ = tx.send(TcpEvent::Disconnected {
                            reason: "发送心跳失败".to_string(),
                        });
                        return;
                    }
                    tracing::debug!("💓 自动发送 KeepAlive");
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                tracing::info!("游戏线程关闭，写线程停止");
                return;
            }
        }
    }
}

/// 把内层包编码成外帧并写出（写线程共用）
fn write_inner(stream: &mut TcpStream, inner: &[u8]) -> std::io::Result<()> {
    let mut framed = Vec::with_capacity(inner.len() + 2);
    codec::encode(inner, &mut framed);
    stream.write_all(&framed)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::net::TcpListener;
    use std::thread;

    /// 从 TCP 流读取并解码一个外帧，返回内层包字节
    fn read_frame(stream: &mut TcpStream) -> Vec<u8> {
        let mut len_buf = [0u8; 2];
        stream.read_exact(&mut len_buf).unwrap();
        let outer_len = u16::from_le_bytes(len_buf) as usize;
        let mut enc = vec![0u8; outer_len];
        stream.read_exact(&mut enc).unwrap();
        let payload: Vec<u8> = enc.iter().map(|b| b ^ 0xAA).collect();
        payload
    }

    fn header_opcode(payload: &[u8]) -> i16 {
        let mut cur = Cursor::new(payload);
        PacketHeader::read_from(&mut cur).unwrap().opcode
    }

    #[test]
    fn test_connect_handshake_and_outbound() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();

        let server = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            // 服务器先发 Connected（ServerPacketIds::Connected = 0，空 body）
            let mut inner = Vec::new();
            inner.extend_from_slice(&4u16.to_le_bytes()); // inner_len = header only
            inner.extend_from_slice(&0i16.to_le_bytes()); // opcode = Connected
            let mut framed = Vec::new();
            codec::encode(&inner, &mut framed);
            s.write_all(&framed).unwrap();

            // 期望客户端自动发 ClientVersion（ClientPacketIds::ClientVersion = 0）
            let f1 = read_frame(&mut s);
            assert_eq!(
                header_opcode(&f1),
                mir2_shared::enums::ClientPacketIds::ClientVersion as i16,
                "握手后应自动发送 ClientVersion"
            );

            // 游戏侧随后发 Login（ClientPacketIds::Login = 5）
            let f2 = read_frame(&mut s);
            assert_eq!(
                header_opcode(&f2),
                mir2_shared::enums::ClientPacketIds::Login as i16,
                "应收到 Login 内层包"
            );
            (header_opcode(&f1), header_opcode(&f2))
        });

        let conn = connect(&addr, [0u8; 16]).unwrap();

        // 游戏侧应收到 Connected 事件
        match conn.from_server.recv_timeout(Duration::from_secs(5)).unwrap() {
            TcpEvent::Packet(p) => assert_eq!(
                header_opcode(&p),
                mir2_shared::enums::ServerPacketIds::Connected as i16
            ),
            other => panic!("预期 Connected 包，实际 {:?}", std::mem::discriminant(&other)),
        }

        // 游戏侧发送 Login
        let mut inner = Vec::new();
        serialize_packet(
            &mut inner,
            &mir2_shared::packets::client::account::Login {
                account_id: "test".to_string(),
                password: "12345".to_string(),
            },
        )
        .unwrap();
        conn.to_server.send(inner).unwrap();

        let (h1, h2) = server.join().unwrap();
        assert_eq!(h1, mir2_shared::enums::ClientPacketIds::ClientVersion as i16);
        assert_eq!(h2, mir2_shared::enums::ClientPacketIds::Login as i16);
    }

    #[test]
    fn test_disconnect_notification() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();

        let server = thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            // 立即关闭连接（不发任何包）
            let _ = s.shutdown(std::net::Shutdown::Both);
        });

        let conn = connect(&addr, [0u8; 16]).unwrap();
        server.join().unwrap();

        match conn.from_server.recv_timeout(Duration::from_secs(5)).unwrap() {
            TcpEvent::Disconnected { .. } => {}
            other => panic!("预期 Disconnected，实际 {:?}", std::mem::discriminant(&other)),
        }
    }
}
