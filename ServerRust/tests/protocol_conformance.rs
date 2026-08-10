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
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use kameo::actor::Spawn;

use crystal_server::gate::actor::{
    GateActor, run_gate_listener,
    SetAccountRef, SetWorldRef, SetMaxConnections,
};
use crystal_server::actors::account::AccountActor;
use crystal_server::actors::social::{SocialActor, SocialActorArgs, SocialActorConfig};
use crystal_server::actors::world::{WorldActor, WorldActorArgs};
use crystal_server::db;
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
    let port = find_free_port();
    let gate_ref = GateActor::spawn_with_mailbox((), kameo::mailbox::unbounded());
    let _ = gate_ref.ask(SetMaxConnections(1024)).await;

    tokio::spawn(async move {
        let _ = run_gate_listener(format!("127.0.0.1:{}", port), gate_ref).await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    port
}

/// 启动带 AccountActor 的服务器(支持 Login 流程)。
async fn start_server_with_login() -> u16 {
    let port = find_free_port();
    let gate_ref = GateActor::spawn_with_mailbox((), kameo::mailbox::unbounded());
    let _ = gate_ref.ask(SetMaxConnections(1024)).await;

    // AccountActor + in-memory DB
    let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
    let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool));
    let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

    tokio::spawn(async move {
        let _ = run_gate_listener(format!("127.0.0.1:{}", port), gate_ref).await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    port
}

/// 启动带 WorldActor 的完整服务器(支持 StartGame 流程)。
/// 需要 Daneo1989/ 目录(地图 + 任务文件)。
async fn start_server_with_world() -> u16 {
    let port = find_free_port();
    let gate_ref = GateActor::spawn_with_mailbox((), kameo::mailbox::unbounded());
    let _ = gate_ref.ask(SetMaxConnections(1024)).await;

    let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");

    // AccountActor
    let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
    let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

    // SocialActor
    let social_config = SocialActorConfig::default();
    let social_ref = SocialActor::spawn(SocialActorArgs {
        gate_ref: gate_ref.clone(),
        db_pool: db_pool.clone(),
        config: social_config,
    });

    // WorldActor
    let world_ref = WorldActor::spawn(WorldActorArgs {
        tick_interval_ms: 100,
        gate_ref: gate_ref.clone(),
        map_dir: PathBuf::from("Daneo1989"),
        spawn_dir: Some(PathBuf::from("Data/spawn")),
        quest_dir: PathBuf::from("Daneo1989/Envir/Quests"),
        db_pool: db_pool.clone(),
        social_ref: social_ref.clone(),
        conquest_cfg: Default::default(),
        rested_cfg: Default::default(),
        pvp_cfg: Default::default(),
        drop_rate: 1.0,
        exp_rate: 1.0,
        item_timeout_ticks: 300,
        max_drop_gold: 2000,
        rarity_cfg: Default::default(),
        notice_path: String::new(),
        death_exp_penalty_percent: 0,
        movement_pacing_ms: 0,
        health_regen_weight: 1,
        mana_regen_weight: 1,
        goods_hide_added_stats: false,
        safe_zone_healing: false,
        experience_list: Vec::new(),
        fishing_cfg: Default::default(),
        random_item_stats: Vec::new(),
        guild_buff_infos: Vec::new(),
    });
    let _ = gate_ref.ask(SetWorldRef { world_ref }).await;

    tokio::spawn(async move {
        let _ = run_gate_listener(format!("127.0.0.1:{}", port), gate_ref).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    port
}

/// 找一个空闲端口(bind 一次然后释放)。
fn find_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
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

#[tokio::test]
async fn test_login_full_flow() {
    let port = start_server_with_login().await;
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("connect");

    // 1. 收到 Connected
    let (opcode, _) = recv_packet(&mut stream).await;
    assert_eq!(opcode, 0, "Expected Connected");

    // 2. 发送 ClientVersion
    let hash = b"test_hash_12345!";
    let mut cv_body = Vec::new();
    cv_body.extend_from_slice(&(hash.len() as i32).to_le_bytes());
    cv_body.extend_from_slice(hash);
    stream.write_all(&make_packet(0, &cv_body)).await.expect("send CV");
    // 消费 ClientVersion 响应
    let _ = recv_packet(&mut stream).await;

    // 3. 发送 NewAccount (opcode=3) — 服务端按 C# 协议解析 account_id/password 等字段
    let mut na_body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "testuser").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "testpass").unwrap();
    na_body.extend_from_slice(&0i64.to_le_bytes()); // birth_date_binary
    mir2_shared::binary::write_dotnet_string(&mut na_body, "Test User").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "question").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "answer").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "test@example.com").unwrap();
    stream.write_all(&make_packet(3, &na_body)).await.expect("send NewAccount");
    // 消费 NewAccount 响应
    let _ = recv_packet(&mut stream).await;

    // 4. 发送 Login (opcode=5): [username: DotNetString][password: DotNetString]
    let mut login_body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut login_body, "testuser").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut login_body, "testpass").unwrap();
    stream.write_all(&make_packet(5, &login_body)).await.expect("send Login");

    // 5. 接收 Login 响应
    //    成功 → ServerPacketIds::LoginSuccess (opcode=9), body=[count=0i32]
    //    失败 → ServerPacketIds::Login (opcode=7), body=[4u8]
    let (resp_opcode, resp_body) = tokio::time::timeout(
        Duration::from_secs(5),
        recv_packet(&mut stream),
    ).await.expect("Login response timeout");

    if resp_opcode == 9 {
        // LoginSuccess: body starts with character count (i32)
        assert!(resp_body.len() >= 4, "LoginSuccess body should have count field");
        let count = i32::from_le_bytes(resp_body[0..4].try_into().unwrap_or([0; 4]));
        assert_eq!(count, 0, "New account should have 0 characters");
        tracing::info!("✅ LoginSuccess: 0 characters");
    } else if resp_opcode == 7 {
        panic!("Login failed (opcode=7): {}", resp_body.first().unwrap_or(&0));
    } else {
        panic!("Unexpected login response opcode={}, expected 9 (LoginSuccess) or 7 (Login fail)", resp_opcode);
    }
}

#[test]
fn test_startgame_full_flow() {
    // WorldActor Tick 为巨型 async 状态机：必须用 8MB 栈 runtime（同 e2e 测试，避免栈溢出）
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
    let port = start_server_with_world().await;
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("connect");

    // 1. Connected
    let _ = recv_packet(&mut stream).await;

    // 2. ClientVersion
    let hash = b"test_hash_12345!";
    let mut cv_body = Vec::new();
    cv_body.extend_from_slice(&(hash.len() as i32).to_le_bytes());
    cv_body.extend_from_slice(hash);
    stream.write_all(&make_packet(0, &cv_body)).await.unwrap();
    let _ = recv_packet(&mut stream).await; // consume CV response

    // 3. NewAccount（对齐 C# 协议：account_id/password 等字段）
    let mut na_body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "e2euser").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "e2epass").unwrap();
    na_body.extend_from_slice(&0i64.to_le_bytes()); // birth_date_binary
    mir2_shared::binary::write_dotnet_string(&mut na_body, "E2E User").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "question").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "answer").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut na_body, "e2e@example.com").unwrap();
    stream.write_all(&make_packet(3, &na_body)).await.unwrap();
    let _ = recv_packet(&mut stream).await;

    // 4. Login
    let mut login_body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut login_body, "e2euser").unwrap();
    mir2_shared::binary::write_dotnet_string(&mut login_body, "e2epass").unwrap();
    stream.write_all(&make_packet(5, &login_body)).await.unwrap();

    // Wait for LoginSuccess (opcode=9)
    loop {
        let (op, _) = tokio::time::timeout(Duration::from_secs(10), recv_packet(&mut stream))
            .await
            .expect("login response timeout");
        if op == 9 { break; } // LoginSuccess
    }

    // 5. 先建角色（C# 对齐：StartGame 不再隐式建号）
    let nc_body = {
        let mut b = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut b, "E2EChar");
        b.push(0u8); // gender = Male
        b.push(0u8); // class = Warrior
        b
    };
    stream.write_all(&make_packet(6, &nc_body)).await.unwrap(); // ClientPacketIds::NewCharacter = 6
    let nc_success = mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16;
    loop {
        let (op, _) = tokio::time::timeout(Duration::from_secs(10), recv_packet(&mut stream))
            .await
            .expect("NewCharacterSuccess timeout");
        if op == nc_success {
            break;
        }
    }

    // 6. StartGame (ClientPacketIds::StartGame = 8, body = character_index: i32 = 0)
    let sg_body = 0i32.to_le_bytes();
    stream.write_all(&make_packet(8, &sg_body)).await.unwrap();

    // 7. 接收 StartGame 响应序列（用枚举 opcode，避免硬编码漂移）：
    //    StartGame / MapChanged / UserInformation / HealthChanged / UserLocation
    let startgame_op = mir2_shared::enums::ServerPacketIds::StartGame as i16;
    let mapchanged_op = mir2_shared::enums::ServerPacketIds::MapChanged as i16;
    let userinfo_op = mir2_shared::enums::ServerPacketIds::UserInformation as i16;
    let health_op = mir2_shared::enums::ServerPacketIds::HealthChanged as i16;
    let loc_op = mir2_shared::enums::ServerPacketIds::UserLocation as i16;
    let mut got_startgame = false;
    let mut got_map_changed = false;
    let mut got_userinfo = false;
    let mut got_health = false;
    let mut got_location = false;
    for _ in 0..20 {
        let result = tokio::time::timeout(Duration::from_secs(15), recv_packet(&mut stream)).await;
        match result {
            Ok((op, body)) => {
                if op == startgame_op { got_startgame = true; }
                if op == mapchanged_op { got_map_changed = true; }
                if op == userinfo_op { got_userinfo = true; }
                if op == health_op { got_health = true; }
                if op == loc_op { got_location = true; }
                if got_startgame && got_map_changed && got_userinfo && got_health && got_location {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    assert!(got_startgame, "Expected StartGame response from server (got map_changed={} userinfo={} health={} loc={})", got_map_changed, got_userinfo, got_health, got_location);
    });
}

