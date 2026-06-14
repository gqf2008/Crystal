//! Phase 2.3: 协议 conformance test — TCP 级别验证完整 codec framing。
//!
//! 这些测试启动真实的 GateActor + TCP listener,用 TCP 客户端发送
//! 经过 codec 双层 framing 的 packet,验证服务端正确解码并响应。
//!
//! 测试链路:
//!   Client TCP → codec::decode → GateActor → codec::encode → Client TCP
//!
//! 覆盖:
//!   1. Connected 自动下发(opcode=0)
//!   2. ClientVersion 握手(opcode=0→1 accepted)
//!   3. KeepAlive 往返(opcode=2→3)

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use kameo::actor::Spawn;

use crystal_server::gate::actor::{GateActor, run_gate_listener, SetAccountRef, SetWorldRef, SetMaxConnections};
use crystal_server::gate::codec;

const XOR_KEY: u8 = 0xAA;

/// XOR 加密/解密(与服务端 codec.rs 对称)。
fn xor(data: &[u8]) -> Vec<u8> {
    data.iter().map(|b| b ^ XOR_KEY).collect()
}

/// 构造客户端 packet: `[inner_len(2)][opcode(2)][body]` → codec encode → `[outer_len(2)][XOR(inner)]`
fn make_packet(opcode: i16, body: &[u8]) -> Vec<u8> {
    let inner_len = (4 + body.len()) as u16;
    let mut inner = Vec::with_capacity(4 + body.len());
    inner.extend_from_slice(&inner_len.to_le_bytes());
    inner.extend_from_slice(&opcode.to_le_bytes());
    inner.extend_from_slice(body);

    let outer_len = inner.len() as u16;
    let mut out = Vec::with_capacity(2 + inner.len());
    out.extend_from_slice(&outer_len.to_le_bytes());
    out.extend(xor(&inner));
    out
}

/// 从 TCP 流读取并解码一个 frame,返回 `(opcode, body)`。
async fn recv_packet(stream: &mut TcpStream) -> (i16, Vec<u8>) {
    let mut len_buf = [0u8; 2];
    stream.read_exact(&mut len_buf).await.expect("read len");
    let outer_len = u16::from_le_bytes(len_buf) as usize;

    let mut enc_payload = vec![0u8; outer_len];
    stream.read_exact(&mut enc_payload).await.expect("read payload");

    let inner = xor(&enc_payload);
    let inner_len = u16::from_le_bytes([inner[0], inner[1]]) as usize;
    let opcode = i16::from_le_bytes([inner[2], inner[3]]);
    let body = if inner_len > 4 {
        inner[4..inner_len].to_vec()
    } else {
        Vec::new()
    };
    (opcode, body)
}

/// 启动服务器并返回端口。
async fn start_server() -> u16 {
    let gate_ref = GateActor::spawn(());
    let _ = gate_ref.ask(SetMaxConnections(1024)).await;

    // 启动 TCP listener 在随机端口
    let gate_ref_clone = gate_ref.clone();
    let (port_tx, port_rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let addr = format!("127.0.0.1:{}", port);
            port_tx.send(port).unwrap();
            drop(listener); // 释放端口让 run_gate_listener 重新绑定

            // 用 run_gate_listener 重新绑定(它内部自己 bind)
            let _ = run_gate_listener(addr, gate_ref_clone).await;
        });
    });

    // 等待端口就绪
    let port = port_rx.recv_timeout(Duration::from_secs(5)).expect("server start timeout");
    tokio::time::sleep(Duration::from_millis(100)).await; // 等 listener 就绪
    port
}

#[tokio::test]
async fn test_connected_autosent() {
    let port = start_server().await;

    // 连接 — 服务端应自动发送 Connected (opcode=0)
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("connect");

    let (opcode, body) = recv_packet(&mut stream).await;

    assert_eq!(opcode, 0, "Expected ServerPacketIds::Connected (opcode=0)");
    assert!(body.is_empty(), "Connected has no body");
}

#[tokio::test]
async fn test_client_version_handshake() {
    let port = start_server().await;
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("connect");

    // 1. 收到 Connected
    let (opcode, _) = recv_packet(&mut stream).await;
    assert_eq!(opcode, 0, "Expected Connected");

    // 2. 发送 ClientVersion (opcode=0 in ClientPacketIds)
    //    body: [i32 hash_len][hash bytes]
    let hash = b"test_hash_12345!";
    let mut body = Vec::new();
    body.extend_from_slice(&(hash.len() as i32).to_le_bytes());
    body.extend_from_slice(hash);

    let packet = make_packet(0, &body); // ClientPacketIds::ClientVersion = 0
    stream.write_all(&packet).await.expect("send ClientVersion");

    // 3. 收到 ClientVersion 响应 (opcode=1, body=[0x01]=accepted)
    let (resp_opcode, resp_body) = recv_packet(&mut stream).await;

    assert_eq!(resp_opcode, 1, "Expected ServerPacketIds::ClientVersion (opcode=1)");
    assert!(!resp_body.is_empty(), "ClientVersion response should have body");
    assert_eq!(resp_body[0], 1, "Expected accepted=1");
}

#[tokio::test]
async fn test_keepalive_roundtrip() {
    let port = start_server().await;
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("connect");

    // 1. 消费 Connected
    let _ = recv_packet(&mut stream).await;

    // 2. 发送 KeepAlive (opcode=2)
    let body = (42i64).to_le_bytes();
    let packet = make_packet(2, &body); // ClientPacketIds::KeepAlive = 2
    stream.write_all(&packet).await.expect("send KeepAlive");

    // 3. 收到 KeepAlive 响应 (opcode=3 = ServerPacketIds::KeepAlive)
    let (resp_opcode, resp_body) = recv_packet(&mut stream).await;

    assert_eq!(resp_opcode, 3, "Expected ServerPacketIds::KeepAlive (opcode=3)");
    // 服务端 KeepAlive 响应 body 为空 (build_packet_bytes(KeepAlive, &[]))
    assert!(resp_body.is_empty(), "Server KeepAlive response has empty body");
}
