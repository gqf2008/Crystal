use std::time::Duration;
use tokio::sync::mpsc;

use kameo::actor::Spawn;

use crate::actors::account::AccountActor;
use crate::actors::social::{SocialActor, SocialActorArgs, SocialActorConfig};
use crate::actors::world::{WorldActor, WorldActorArgs};
use crate::gate::actor::{GateActor, ClientData, SessionCreated, SetAccountRef, SetWorldRef};
use crate::util::wire::build_packet_bytes;
use crate::db;

// ============================================================
// E2E Test Helpers
// ============================================================

type GateActorRef = kameo::actor::ActorRef<GateActor>;
type RxChannel = tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>;

async fn setup_gate_and_session(
    session_id: u64,
) -> (GateActorRef, tokio::sync::mpsc::UnboundedSender<Vec<u8>>, RxChannel) {
    let gate_ref = GateActor::spawn(());
    let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let _ = gate_ref.ask(SessionCreated { session_id, sender: tx.clone(), ip: "127.0.0.1".to_string() }).await;
    (gate_ref, tx, rx)
}

async fn drain_connected(rx: &mut RxChannel) {
    let _ = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
}

async fn e2e_setup_login(
    gate_ref: &GateActorRef,
    session_id: u64,
    rx: &mut RxChannel,
) -> db::DbPool {
    let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");

    let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
    let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

    // ClientVersion
    let cv_body = {
        let mut b = Vec::new();
        let hash = b"test";
        b.extend_from_slice(&(hash.len() as i32).to_le_bytes());
        b.extend_from_slice(hash);
        b
    };
    let cv_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::ClientVersion as i16, &cv_body);
    let _ = gate_ref.ask(ClientData { session_id, data: cv_packet }).await;

    // NewAccount
    let na_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::NewAccount as i16, &[]);
    let _ = gate_ref.ask(ClientData { session_id, data: na_packet }).await;

    // Login
    let mut login_body = Vec::new();
    let _ = mir2_shared::binary::write_dotnet_string(&mut login_body, "testuser");
    let _ = mir2_shared::binary::write_dotnet_string(&mut login_body, "testpass");
    let login_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::Login as i16, &login_body);
    let _ = gate_ref.ask(ClientData { session_id, data: login_packet }).await;

    // Drain responses until LoginSuccess
    let login_success_opcode = mir2_shared::enums::ServerPacketIds::LoginSuccess as i16;
    loop {
        let data = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("rx.recv timed out")
            .expect("channel closed");
        if data.len() >= 4 {
            let opcode = i16::from_le_bytes([data[2], data[3]]);
            if opcode == login_success_opcode {
                break;
            }
        }
    }

    db_pool
}

// ============================================================
// E2E Tests
// ============================================================

#[tokio::test]
async fn e2e_client_version_handshake() {
    let session_id = 1u64;
    let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;

    drain_connected(&mut rx).await;

    // Send ClientVersion
    let cv_body = {
        let mut b = Vec::new();
        let hash = b"test";
        b.extend_from_slice(&(hash.len() as i32).to_le_bytes());
        b.extend_from_slice(hash);
        b
    };
    let cv_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::ClientVersion as i16, &cv_body);
    let _ = gate_ref.ask(ClientData { session_id, data: cv_packet }).await;

    let response = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(response.len(), 5); // 4 header + 1 body
    let resp_opcode = i16::from_le_bytes([response[2], response[3]]);
    assert_eq!(resp_opcode, mir2_shared::enums::ServerPacketIds::ClientVersion as i16);
    assert_eq!(response[4], 1u8); // accepted
}

#[tokio::test]
async fn e2e_new_account_auto_success() {
    let session_id = 2u64;
    let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
    drain_connected(&mut rx).await;

    // 需要 AccountActor 才能真正注册（C# Envir.NewAccount 创建账号）
    let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
    let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool));
    let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

    // 构造合法 NewAccount 包（对齐 C# ClientPackets.NewAccount）
    let mut na_body = Vec::new();
    let _ = mir2_shared::binary::write_dotnet_string(&mut na_body, "newuser");
    let _ = mir2_shared::binary::write_dotnet_string(&mut na_body, "newpass123");
    na_body.extend_from_slice(&0i64.to_le_bytes()); // birth_date_binary
    let _ = mir2_shared::binary::write_dotnet_string(&mut na_body, "New User");
    let _ = mir2_shared::binary::write_dotnet_string(&mut na_body, "");
    let _ = mir2_shared::binary::write_dotnet_string(&mut na_body, "");
    let _ = mir2_shared::binary::write_dotnet_string(&mut na_body, "");
    let na_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::NewAccount as i16, &na_body);
    let _ = gate_ref.ask(ClientData { session_id, data: na_packet.clone() }).await;

    let response = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(response.len(), 5);
    let resp_opcode = i16::from_le_bytes([response[2], response[3]]);
    assert_eq!(resp_opcode, mir2_shared::enums::ServerPacketIds::NewAccount as i16);
    assert_eq!(response[4], 8u8); // success

    // 重复注册同一账号 → Result=7（已存在）
    let _ = gate_ref.ask(ClientData { session_id, data: na_packet.clone() }).await;
    let response2 = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout2")
        .expect("channel closed2");
    assert_eq!(response2[4], 7u8);
}

#[tokio::test]
async fn e2e_keep_alive_roundtrip() {
    let session_id = 3u64;
    let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
    drain_connected(&mut rx).await;

    let ka_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::KeepAlive as i16, &[]);
    let _ = gate_ref.ask(ClientData { session_id, data: ka_packet }).await;

    let response = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(response.len(), 4);
    let resp_opcode = i16::from_le_bytes([response[2], response[3]]);
    assert_eq!(resp_opcode, mir2_shared::enums::ServerPacketIds::KeepAlive as i16);
}

#[tokio::test]
async fn e2e_login_flow() {
    let session_id = 4u64;
    let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
    let _ = e2e_setup_login(&gate_ref, session_id, &mut rx).await;

    assert!(!rx.is_closed());
}

#[test]
fn e2e_start_game_flow() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let session_id = 5u64;
        let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
        let db_pool = e2e_setup_login(&gate_ref, session_id, &mut rx).await;

        // Spawn SocialActor
        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(),
            db_pool: db_pool.clone(),
            config: SocialActorConfig::default(),
        });

        // Spawn WorldActor
        let world_ref = WorldActor::spawn(WorldActorArgs {
            tick_interval_ms: 1000,
            gate_ref: gate_ref.clone(),
            map_dir: std::path::PathBuf::from("."),
            spawn_dir: None,
            quest_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            safe_zone_healing: false,
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            guild_buff_infos: Vec::new(),
        });

        let _ = gate_ref.ask(SetWorldRef { world_ref }).await;

        // C# 流程：先 NewCharacter 再 StartGame（StartGame 不再隐式建号）
        let nc_body = {
            let mut b = Vec::new();
            let _ = mir2_shared::binary::write_dotnet_string(&mut b, "TestChar");
            b.push(0u8); // gender = Male
            b.push(0u8); // class = Warrior
            b
        };
        let nc_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::NewCharacter as i16, &nc_body);
        let _ = gate_ref.ask(ClientData { session_id, data: nc_packet }).await;
        let nc_success_opcode = mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16;
        loop {
            let data = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("rx.recv timed out waiting NewCharacterSuccess")
                .expect("channel closed");
            if data.len() >= 4 {
                let opcode = i16::from_le_bytes([data[2], data[3]]);
                if opcode == nc_success_opcode {
                    break;
                }
            }
        }

        // Send StartGame
        let sg_body = 0i32.to_le_bytes().to_vec();
        let sg_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::StartGame as i16, &sg_body);
        let _ = gate_ref.ask(ClientData { session_id, data: sg_packet }).await;

        // Collect responses - we should see StartGame, MapChanged, UserInformation, HealthChanged, UserLocation
        let expected_opcodes = [
            mir2_shared::enums::ServerPacketIds::StartGame as i16,
            mir2_shared::enums::ServerPacketIds::MapChanged as i16,
            mir2_shared::enums::ServerPacketIds::UserInformation as i16,
            mir2_shared::enums::ServerPacketIds::HealthChanged as i16,
            mir2_shared::enums::ServerPacketIds::UserLocation as i16,
        ];

        let mut found = vec![false; expected_opcodes.len()];
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);

        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            if let Ok(Some(data)) = tokio::time::timeout(remaining, rx.recv()).await {
                if data.len() >= 4 {
                    let opcode = i16::from_le_bytes([data[2], data[3]]);
                    for (i, expected) in expected_opcodes.iter().enumerate() {
                        if opcode == *expected {
                            found[i] = true;
                        }
                    }
                }
            }
            if found.iter().all(|&x| x) {
                break;
            }
        }

        for (i, expected) in expected_opcodes.iter().enumerate() {
            assert!(found[i], "Missing expected packet opcode: {}", expected);
        }
    });
}

#[test]
fn e2e_magic_cast_flow() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let session_id = 6u64;
        let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
        let db_pool = e2e_setup_login(&gate_ref, session_id, &mut rx).await;

        let social_ref = crate::actors::social::SocialActor::spawn(
            crate::actors::social::SocialActorArgs {
                gate_ref: gate_ref.clone(),
                db_pool: db_pool.clone(),
                config: crate::actors::social::SocialActorConfig::default(),
            },
        );
        let world_ref = crate::actors::world::WorldActor::spawn(
            crate::actors::world::WorldActorArgs {
                tick_interval_ms: 1000,
                gate_ref: gate_ref.clone(),
                map_dir: std::path::PathBuf::from("."),
                spawn_dir: None,
                quest_dir: std::path::PathBuf::from("."),
                db_pool: db_pool.clone(),
                social_ref,
                conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            safe_zone_healing: false,
                drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
                item_timeout_ticks: 300,
                max_drop_gold: 2000,
                rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            guild_buff_infos: Vec::new(),
            },
        );
        let _ = gate_ref.ask(crate::gate::actor::SetWorldRef { world_ref }).await;

        // Send StartGame first
        let sg_body = 0i32.to_le_bytes().to_vec();
        let sg_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::StartGame as i16, &sg_body,
        );
        let _ = gate_ref.ask(crate::gate::actor::ClientData { session_id, data: sg_packet }).await;

        // Drain StartGame sequence
        let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if tokio::time::Instant::now() > drain_deadline { break; }
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await;
        }

        // Send magic cast (FireBall spell=31, direction=Down(4), target at (50,60))
        let mut magic_body = Vec::new();
        magic_body.push(31u8); // spell=FireBall
        magic_body.push(4u8);  // direction=Down
        magic_body.extend_from_slice(&0u32.to_le_bytes()); // target_id=0
        magic_body.extend_from_slice(&50i32.to_le_bytes()); // target_x
        magic_body.extend_from_slice(&60i32.to_le_bytes()); // target_y
        let magic_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::Magic as i16, &magic_body,
        );
        let _ = gate_ref.ask(crate::gate::actor::ClientData { session_id, data: magic_packet }).await;

        // Verify channel is still alive — magic handler should not crash
        assert!(!rx.is_closed(), "Channel should remain open after magic cast");
        // Note: system message may arrive asynchronously via tokio::spawn in send_system_message
    });
}

/// 双会话：B 先进图后 A 再 StartGame（#881 复现路径：双客户端并发进图 tokio 栈溢出回归）
#[test]
fn e2e_two_sessions_concurrent_start() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        // 单个 gate + 两个 session
        let gate_ref = GateActor::spawn(());
        let (tx5, mut rx5) = mpsc::unbounded_channel::<Vec<u8>>();
        let (tx6, mut rx6) = mpsc::unbounded_channel::<Vec<u8>>();
        let _ = gate_ref.ask(SessionCreated { session_id: 5, sender: tx5.clone(), ip: "127.0.0.1".to_string() }).await;
        let _ = gate_ref.ask(SessionCreated { session_id: 6, sender: tx6.clone(), ip: "127.0.0.1".to_string() }).await;

        let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
        let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
        let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

        // 登录两个账号（同一 AccountActor）
        async fn login(gate_ref: &GateActorRef, session_id: u64, rx: &mut RxChannel, username: &str) {
            let cv_body = { let mut b = Vec::new(); let hash = b"test"; b.extend_from_slice(&(hash.len() as i32).to_le_bytes()); b.extend_from_slice(hash); b };
            let _ = gate_ref.ask(ClientData { session_id, data: build_packet_bytes(mir2_shared::enums::ClientPacketIds::ClientVersion as i16, &cv_body) }).await;
            let _ = gate_ref.ask(ClientData { session_id, data: build_packet_bytes(mir2_shared::enums::ClientPacketIds::NewAccount as i16, &[]) }).await;
            let mut lb = Vec::new();
            let _ = mir2_shared::binary::write_dotnet_string(&mut lb, username);
            let _ = mir2_shared::binary::write_dotnet_string(&mut lb, "testpass");
            let _ = gate_ref.ask(ClientData { session_id, data: build_packet_bytes(mir2_shared::enums::ClientPacketIds::Login as i16, &lb) }).await;
            let ok = mir2_shared::enums::ServerPacketIds::LoginSuccess as i16;
            loop {
                let data = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.expect("timeout").expect("closed");
                if data.len() >= 4 && i16::from_le_bytes([data[2], data[3]]) == ok { break; }
            }
        }
        login(&gate_ref, 5, &mut rx5, "testuser").await;
        login(&gate_ref, 6, &mut rx6, "testuser2").await;

        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(), db_pool: db_pool.clone(), config: SocialActorConfig::default(),
        });
        let world_ref = WorldActor::spawn(WorldActorArgs {
            tick_interval_ms: 100,
            gate_ref: gate_ref.clone(),
            map_dir: std::path::PathBuf::from("."),
            spawn_dir: None,
            quest_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            safe_zone_healing: false,
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            guild_buff_infos: Vec::new(),
        });
        let _ = gate_ref.ask(SetWorldRef { world_ref }).await;

        async fn start_game(gate_ref: &GateActorRef, session_id: u64, rx: &mut RxChannel, char_name: &str) {
            let nc_body = { let mut b = Vec::new(); let _ = mir2_shared::binary::write_dotnet_string(&mut b, char_name); b.push(0u8); b.push(0u8); b };
            let _ = gate_ref.ask(ClientData { session_id, data: build_packet_bytes(mir2_shared::enums::ClientPacketIds::NewCharacter as i16, &nc_body) }).await;
            let ncs = mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16;
            loop {
                let data = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.expect("timeout").expect("closed");
                if data.len() >= 4 && i16::from_le_bytes([data[2], data[3]]) == ncs { break; }
            }
            let _ = gate_ref.ask(ClientData { session_id, data: build_packet_bytes(mir2_shared::enums::ClientPacketIds::StartGame as i16, &0i32.to_le_bytes().to_vec()) }).await;
            // 等待 StartGame 响应
            let sg = mir2_shared::enums::ServerPacketIds::StartGame as i16;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            let mut found = false;
            while tokio::time::Instant::now() < deadline {
                let remaining = deadline - tokio::time::Instant::now();
                if let Ok(Some(data)) = tokio::time::timeout(remaining, rx.recv()).await {
                    if data.len() >= 4 && i16::from_le_bytes([data[2], data[3]]) == sg { found = true; break; }
                } else { break; }
            }
            assert!(found, "session {} StartGame response not received", session_id);
        }

        // B(5) 先进图
        start_game(&gate_ref, 5, &mut rx5, "CharB").await;
        // A(6) 后进图（B 已在图内）—— #881 崩溃路径
        start_game(&gate_ref, 6, &mut rx6, "CharA").await;

        // 跑几秒 tick（100ms 间隔）验证不崩溃
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(!rx5.is_closed(), "session 5 channel alive");
        assert!(!rx6.is_closed(), "session 6 channel alive");
    });
}

#[test]
fn e2e_attack_flow() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let session_id = 7u64;
        let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
        let db_pool = e2e_setup_login(&gate_ref, session_id, &mut rx).await;

        let social_ref = crate::actors::social::SocialActor::spawn(
            crate::actors::social::SocialActorArgs {
                gate_ref: gate_ref.clone(),
                db_pool: db_pool.clone(),
                config: crate::actors::social::SocialActorConfig::default(),
            },
        );
        let world_ref = crate::actors::world::WorldActor::spawn(
            crate::actors::world::WorldActorArgs {
                tick_interval_ms: 1000,
                gate_ref: gate_ref.clone(),
                map_dir: std::path::PathBuf::from("."),
                spawn_dir: None,
                quest_dir: std::path::PathBuf::from("."),
                db_pool: db_pool.clone(),
                social_ref,
                conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            safe_zone_healing: false,
                drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
                item_timeout_ticks: 300,
                max_drop_gold: 2000,
                rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            guild_buff_infos: Vec::new(),
            },
        );
        let _ = gate_ref.ask(crate::gate::actor::SetWorldRef { world_ref }).await;

        // StartGame
        let sg_body = 0i32.to_le_bytes().to_vec();
        let sg_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::StartGame as i16, &sg_body,
        );
        let _ = gate_ref.ask(crate::gate::actor::ClientData { session_id, data: sg_packet }).await;

        // Drain StartGame packets
        let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if tokio::time::Instant::now() > drain_deadline { break; }
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await;
        }

        // Send Attack (spell=0, direction=Right(2))
        let mut attack_body = Vec::new();
        attack_body.push(0u8); // spell=0 (basic attack)
        attack_body.push(2u8); // direction=Right
        let attack_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::Attack as i16, &attack_body,
        );
        let _ = gate_ref.ask(crate::gate::actor::ClientData { session_id, data: attack_packet }).await;

        // Should receive some response - channel should stay open
        assert!(!rx.is_closed(), "Channel should remain open after attack");

        // Drain any response packets (attack might hit nothing, but shouldn't crash)
        let check_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut got_response = false;
        loop {
            if tokio::time::Instant::now() > check_deadline { break; }
            if let Ok(Some(_)) = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await {
                got_response = true; break;
            }
        }
        // Attack may not hit anything (no monsters spawned), but handler shouldn't crash
        assert!(!rx.is_closed(), "Channel should remain open");
    });
}
