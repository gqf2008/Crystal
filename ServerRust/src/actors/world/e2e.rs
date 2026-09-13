use std::time::Duration;
use tokio::sync::mpsc;

use kameo::actor::Spawn;

use crate::actors::account::AccountActor;
use crate::actors::social::{SocialActor, SocialActorArgs, SocialActorConfig};
use crate::actors::world::{WorldActor, WorldActorArgs};
use crate::db;
use crate::gate::actor::{ClientData, GateActor, SessionCreated, SetAccountRef, SetWorldRef};
use crate::util::wire::build_packet_bytes;

// ============================================================
// E2E Test Helpers
// ============================================================

type GateActorRef = kameo::actor::ActorRef<GateActor>;
type RxChannel = tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>;

async fn setup_gate_and_session(
    session_id: u64,
) -> (
    GateActorRef,
    tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    RxChannel,
) {
    let gate_ref = GateActor::spawn(());
    let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let _ = gate_ref
        .ask(SessionCreated {
            session_id,
            sender: tx.clone(),
            ip: "127.0.0.1".to_string(),
        })
        .await;
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
    let cv_packet = build_packet_bytes(
        mir2_shared::enums::ClientPacketIds::ClientVersion as i16,
        &cv_body,
    );
    let _ = gate_ref
        .ask(ClientData {
            session_id,
            data: cv_packet,
        })
        .await;

    // NewAccount
    let na_packet = build_packet_bytes(mir2_shared::enums::ClientPacketIds::NewAccount as i16, &[]);
    let _ = gate_ref
        .ask(ClientData {
            session_id,
            data: na_packet,
        })
        .await;

    // Login
    let mut login_body = Vec::new();
    let _ = mir2_shared::binary::write_dotnet_string(&mut login_body, "testuser");
    let _ = mir2_shared::binary::write_dotnet_string(&mut login_body, "testpass");
    let login_packet = build_packet_bytes(
        mir2_shared::enums::ClientPacketIds::Login as i16,
        &login_body,
    );
    let _ = gate_ref
        .ask(ClientData {
            session_id,
            data: login_packet,
        })
        .await;

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

// ============================================================
// #2827：精炼确认包（C# S.DepositRefineItem / S.RetrieveRefineItem）
// ============================================================

/// 等待指定 opcode 的包并返回其 body（去掉 4 字节头）；超时返回 None
async fn wait_opcode_body(rx: &mut RxChannel, opcode: i16, secs: u64) -> Option<Vec<u8>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(data)) if data.len() >= 4 => {
                if i16::from_le_bytes([data[2], data[3]]) == opcode {
                    return Some(data[4..].to_vec());
                }
            }
            Ok(Some(_)) => continue,
            _ => return None,
        }
    }
    None
}

/// #2827：存入/取回精炼物品**失败**也必须回确认包（C# `Enqueue(p)`，PlayerObject.cs:12511-12601）——
/// 客户端只有收到 `S.DepositRefineItem` 才会发 `C.RefineItem`（Bevy refine.rs:227-231），
/// 缺包会让 UI 精炼链路永久卡在「已请求存入武器…」。
///
/// 红检：去掉 `DepositRefineItemRequest`/`RetrieveRefineItemRequest` handler 里的
/// `send_refine_slot_ack` 调用 → 本用例两个断言都 FAILED。
#[test]
fn e2e_refine_deposit_and_retrieve_ack_packets() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let session_id = 21u64;
        let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
        let db_pool = e2e_setup_login(&gate_ref, session_id, &mut rx).await;

        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(),
            db_pool: db_pool.clone(),
            config: SocialActorConfig::default(),
        });
        let world_ref = WorldActor::spawn(WorldActorArgs {
            tick_interval_ms: 1000,
            gate_ref: gate_ref.clone(),
            map_dir: std::path::PathBuf::from("."),
            spawn_dir: None,
            quest_dir: std::path::PathBuf::from("."),
            npc_script_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            goods_on: true,
            goods_max_stored: 15,
            goods_buy_back_time_minutes: 60,
            goods_buy_back_max_stored: 20,
            safe_zone_healing: false,
            archive_inactive_after_months: 12,
            monster_recall_enabled: true,
            monster_recall_range: 12,
            monster_recall_cooldown_ms: 5000,
            exp_mob_level_difference: true,
            refine_cfg: crate::util::config::RefineConfig::default(),
            replace_wedring_cost: 125,
            lover_exp_bonus: 5,
            mentor_exp_boost: 10,
            mentor_damage_boost: 10,
            mentor_skill_boost: true,
            mentee_exp_bank: 1,
            orbs_exp_list: Vec::new(),
            orbs_dmg_list: Vec::new(),
            orbs_def_list: Vec::new(),
            awakening_cfg: Default::default(),
            gem_cfg: Default::default(),
            hero_exp_list: Vec::new(),
            setup_cfg: Default::default(),
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            drop_gold: true,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            random_item_stats: Vec::new(),
            guild_buff_infos: Vec::new(),
        });
        let _ = gate_ref.ask(SetWorldRef { world_ref }).await;

        // 建角 + 进图（角色无精炼中物品，故两处均为「失败」路径——正是本轮补的缺包处）
        let mut nc_body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut nc_body, "RefineChar");
        nc_body.push(0u8);
        nc_body.push(0u8);
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::NewCharacter as i16,
                    &nc_body,
                ),
            })
            .await;
        assert!(
            wait_opcode_body(
                &mut rx,
                mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16,
                3
            )
            .await
            .is_some(),
            "NewCharacterSuccess"
        );
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::StartGame as i16,
                    &0i32.to_le_bytes().to_vec(),
                ),
            })
            .await;
        assert!(
            wait_opcode_body(
                &mut rx,
                mir2_shared::enums::ServerPacketIds::StartGame as i16,
                5
            )
            .await
            .is_some(),
            "StartGame"
        );

        // 存入：from=999 越界（背包无此格）→ 必须回 success=false 的确认包
        let mut body = Vec::new();
        body.extend_from_slice(&999i32.to_le_bytes());
        body.extend_from_slice(&0i32.to_le_bytes());
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::DepositRefineItem as i16,
                    &body,
                ),
            })
            .await;
        let ack = wait_opcode_body(
            &mut rx,
            mir2_shared::enums::ServerPacketIds::DepositRefineItem as i16,
            3,
        )
        .await
        .expect("deposit ack packet missing (S.DepositRefineItem)");
        assert_eq!(
            ack.len(),
            9,
            "deposit ack body = [from i32][to i32][success u8]"
        );
        assert_eq!(i32::from_le_bytes(ack[0..4].try_into().unwrap()), 999);
        assert_eq!(i32::from_le_bytes(ack[4..8].try_into().unwrap()), 0);
        assert_eq!(ack[8], 0, "越界存入必须 success=false");

        // 取回：精炼栏为空（from=0）→ 必须回 success=false 的确认包
        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes());
        body.extend_from_slice(&0i32.to_le_bytes());
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::RetrieveRefineItem as i16,
                    &body,
                ),
            })
            .await;
        let ack = wait_opcode_body(
            &mut rx,
            mir2_shared::enums::ServerPacketIds::RetrieveRefineItem as i16,
            3,
        )
        .await
        .expect("retrieve ack packet missing (S.RetrieveRefineItem)");
        assert_eq!(
            ack.len(),
            9,
            "retrieve ack body = [from i32][to i32][success u8]"
        );
        assert_eq!(ack[8], 0, "空精炼栏取回必须 success=false");
    });
}

/// #2843：不在 `[@REFINE]` 页时存入武器必须被拒（C# `PlayerObject.cs:12509`）——
/// 背包里有可存入武器（DB 直插到格 0）时，无页请求只能回 `success=false`。
///
/// 红检：删除 `deposit_refine_item_inner` 里的 `npc_page_allows` 门槛 →
/// 存入会成功（`success=true`）→ 本用例断言 FAILED。
#[test]
fn e2e_refine_deposit_requires_refine_npc_page() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let session_id = 31u64;
        let (gate_ref, _tx, mut rx) = setup_gate_and_session(session_id).await;
        let db_pool = e2e_setup_login(&gate_ref, session_id, &mut rx).await;

        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(),
            db_pool: db_pool.clone(),
            config: SocialActorConfig::default(),
        });
        let world_ref = WorldActor::spawn(WorldActorArgs {
            tick_interval_ms: 1000,
            gate_ref: gate_ref.clone(),
            map_dir: std::path::PathBuf::from("."),
            spawn_dir: None,
            quest_dir: std::path::PathBuf::from("."),
            npc_script_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            goods_on: true,
            goods_max_stored: 15,
            goods_buy_back_time_minutes: 60,
            goods_buy_back_max_stored: 20,
            safe_zone_healing: false,
            archive_inactive_after_months: 12,
            monster_recall_enabled: true,
            monster_recall_range: 12,
            monster_recall_cooldown_ms: 5000,
            exp_mob_level_difference: true,
            refine_cfg: crate::util::config::RefineConfig::default(),
            replace_wedring_cost: 125,
            lover_exp_bonus: 5,
            mentor_exp_boost: 10,
            mentor_damage_boost: 10,
            mentor_skill_boost: true,
            mentee_exp_bank: 1,
            orbs_exp_list: Vec::new(),
            orbs_dmg_list: Vec::new(),
            orbs_def_list: Vec::new(),
            awakening_cfg: Default::default(),
            gem_cfg: Default::default(),
            hero_exp_list: Vec::new(),
            setup_cfg: Default::default(),
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            drop_gold: true,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            random_item_stats: Vec::new(),
            guild_buff_infos: Vec::new(),
        });
        let _ = gate_ref.ask(SetWorldRef { world_ref }).await;

        // 建角（此时背包为空）
        let mut nc_body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut nc_body, "RefinePageChar");
        nc_body.push(0u8);
        nc_body.push(0u8);
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::NewCharacter as i16,
                    &nc_body,
                ),
            })
            .await;
        assert!(
            wait_opcode_body(
                &mut rx,
                mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16,
                3
            )
            .await
            .is_some(),
            "NewCharacterSuccess"
        );

        // 直插一把武器到背包格 0（StartGame 时载入）
        let mut weapon = mir2_shared::data::item::UserItem::default();
        weapon.item_index = 1;
        weapon.unique_id = 9001;
        weapon.count = 1;
        let item_json = serde_json::to_string(&weapon).expect("serialize item");
        sqlx::query(
            "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES (?, 0, ?)",
        )
        .bind("RefinePageChar")
        .bind(item_json)
        .execute(&db_pool)
        .await
        .expect("insert backpack item");

        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::StartGame as i16,
                    &0i32.to_le_bytes().to_vec(),
                ),
            })
            .await;
        assert!(
            wait_opcode_body(
                &mut rx,
                mir2_shared::enums::ServerPacketIds::StartGame as i16,
                5
            )
            .await
            .is_some(),
            "StartGame"
        );

        // 未开 [@REFINE] 页 → 存入必须被拒（success=false）
        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes()); // from = 背包格 0（有武器）
        body.extend_from_slice(&0i32.to_le_bytes()); // to = 武器槽
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::DepositRefineItem as i16,
                    &body,
                ),
            })
            .await;
        let ack = wait_opcode_body(
            &mut rx,
            mir2_shared::enums::ServerPacketIds::DepositRefineItem as i16,
            3,
        )
        .await
        .expect("deposit ack packet missing");
        assert_eq!(ack.len(), 9, "ack body = [from i32][to i32][success u8]");
        assert_eq!(
            ack[8], 0,
            "未开 [@REFINE] 页时存入必须被拒（C# PlayerObject.cs:12509）"
        );
    });
}

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
    let cv_packet = build_packet_bytes(
        mir2_shared::enums::ClientPacketIds::ClientVersion as i16,
        &cv_body,
    );
    let _ = gate_ref
        .ask(ClientData {
            session_id,
            data: cv_packet,
        })
        .await;

    let response = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(response.len(), 5); // 4 header + 1 body
    let resp_opcode = i16::from_le_bytes([response[2], response[3]]);
    assert_eq!(
        resp_opcode,
        mir2_shared::enums::ServerPacketIds::ClientVersion as i16
    );
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
    let na_packet = build_packet_bytes(
        mir2_shared::enums::ClientPacketIds::NewAccount as i16,
        &na_body,
    );
    let _ = gate_ref
        .ask(ClientData {
            session_id,
            data: na_packet.clone(),
        })
        .await;

    let response = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(response.len(), 5);
    let resp_opcode = i16::from_le_bytes([response[2], response[3]]);
    assert_eq!(
        resp_opcode,
        mir2_shared::enums::ServerPacketIds::NewAccount as i16
    );
    assert_eq!(response[4], 8u8); // success

    // 重复注册同一账号 → Result=7（已存在）
    let _ = gate_ref
        .ask(ClientData {
            session_id,
            data: na_packet.clone(),
        })
        .await;
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
    let _ = gate_ref
        .ask(ClientData {
            session_id,
            data: ka_packet,
        })
        .await;

    let response = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(response.len(), 4);
    let resp_opcode = i16::from_le_bytes([response[2], response[3]]);
    assert_eq!(
        resp_opcode,
        mir2_shared::enums::ServerPacketIds::KeepAlive as i16
    );
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
            npc_script_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            goods_on: true,
            goods_max_stored: 15,
            goods_buy_back_time_minutes: 60,
            goods_buy_back_max_stored: 20,
            safe_zone_healing: false,
            archive_inactive_after_months: 12,
            monster_recall_enabled: true,
            monster_recall_range: 12,
            monster_recall_cooldown_ms: 5000,
            exp_mob_level_difference: true,
            refine_cfg: crate::util::config::RefineConfig::default(),
            replace_wedring_cost: 125,
            lover_exp_bonus: 5,
            mentor_exp_boost: 10,
            mentor_damage_boost: 10,
            mentor_skill_boost: true,
            mentee_exp_bank: 1,
            orbs_exp_list: Vec::new(),
            orbs_dmg_list: Vec::new(),
            orbs_def_list: Vec::new(),
            awakening_cfg: Default::default(),
            gem_cfg: Default::default(),
            hero_exp_list: Vec::new(),
            setup_cfg: Default::default(),
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            drop_gold: true,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            random_item_stats: Vec::new(),
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
        let nc_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::NewCharacter as i16,
            &nc_body,
        );
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: nc_packet,
            })
            .await;
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
        let sg_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::StartGame as i16,
            &sg_body,
        );
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: sg_packet,
            })
            .await;

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

        let social_ref =
            crate::actors::social::SocialActor::spawn(crate::actors::social::SocialActorArgs {
                gate_ref: gate_ref.clone(),
                db_pool: db_pool.clone(),
                config: crate::actors::social::SocialActorConfig::default(),
            });
        let world_ref =
            crate::actors::world::WorldActor::spawn(crate::actors::world::WorldActorArgs {
                tick_interval_ms: 1000,
                gate_ref: gate_ref.clone(),
                map_dir: std::path::PathBuf::from("."),
                spawn_dir: None,
                quest_dir: std::path::PathBuf::from("."),
                npc_script_dir: std::path::PathBuf::from("."),
                db_pool: db_pool.clone(),
                social_ref,
                conquest_cfg: crate::util::config::ConquestConfig::default(),
                rested_cfg: crate::util::config::RestedConfig::default(),
                pvp_cfg: crate::util::config::PvpConfig::default(),
                health_regen_weight: 10,
                mana_regen_weight: 10,
                goods_hide_added_stats: true,
                goods_on: true,
                goods_max_stored: 15,
                goods_buy_back_time_minutes: 60,
                goods_buy_back_max_stored: 20,
                safe_zone_healing: false,
                archive_inactive_after_months: 12,
                monster_recall_enabled: true,
                monster_recall_range: 12,
                monster_recall_cooldown_ms: 5000,
                exp_mob_level_difference: true,
                refine_cfg: crate::util::config::RefineConfig::default(),
                replace_wedring_cost: 125,
                lover_exp_bonus: 5,
                mentor_exp_boost: 10,
                mentor_damage_boost: 10,
                mentor_skill_boost: true,
                mentee_exp_bank: 1,
                orbs_exp_list: Vec::new(),
                orbs_dmg_list: Vec::new(),
                orbs_def_list: Vec::new(),
                awakening_cfg: Default::default(),
                gem_cfg: Default::default(),
                hero_exp_list: Vec::new(),
                setup_cfg: Default::default(),
                drop_rate: 1.0,
                exp_rate: 1.0,
                experience_list: Vec::new(),
                item_timeout_ticks: 300,
                max_drop_gold: 2000,
                drop_gold: true,
                rarity_cfg: crate::util::config::RarityConfig::default(),
                notice_path: "Notice.txt".to_string(),
                death_exp_penalty_percent: 0,
                movement_pacing_ms: 0,
                fishing_cfg: crate::util::ini::FishingConfig::default(),
                random_item_stats: Vec::new(),
                guild_buff_infos: Vec::new(),
            });
        let _ = gate_ref
            .ask(crate::gate::actor::SetWorldRef { world_ref })
            .await;

        // Send StartGame first
        let sg_body = 0i32.to_le_bytes().to_vec();
        let sg_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::StartGame as i16,
            &sg_body,
        );
        let _ = gate_ref
            .ask(crate::gate::actor::ClientData {
                session_id,
                data: sg_packet,
            })
            .await;

        // Drain StartGame sequence
        let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if tokio::time::Instant::now() > drain_deadline {
                break;
            }
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await;
        }

        // Send magic cast (FireBall spell=31, direction=Down(4), target at (50,60))
        let mut magic_body = Vec::new();
        magic_body.push(31u8); // spell=FireBall
        magic_body.push(4u8); // direction=Down
        magic_body.extend_from_slice(&0u32.to_le_bytes()); // target_id=0
        magic_body.extend_from_slice(&50i32.to_le_bytes()); // target_x
        magic_body.extend_from_slice(&60i32.to_le_bytes()); // target_y
        let magic_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::Magic as i16,
            &magic_body,
        );
        let _ = gate_ref
            .ask(crate::gate::actor::ClientData {
                session_id,
                data: magic_packet,
            })
            .await;

        // Verify channel is still alive — magic handler should not crash
        assert!(
            !rx.is_closed(),
            "Channel should remain open after magic cast"
        );
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
        let _ = gate_ref
            .ask(SessionCreated {
                session_id: 5,
                sender: tx5.clone(),
                ip: "127.0.0.1".to_string(),
            })
            .await;
        let _ = gate_ref
            .ask(SessionCreated {
                session_id: 6,
                sender: tx6.clone(),
                ip: "127.0.0.1".to_string(),
            })
            .await;

        let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
        let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
        let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

        // 登录两个账号（同一 AccountActor）
        async fn login(
            gate_ref: &GateActorRef,
            session_id: u64,
            rx: &mut RxChannel,
            username: &str,
        ) {
            let cv_body = {
                let mut b = Vec::new();
                let hash = b"test";
                b.extend_from_slice(&(hash.len() as i32).to_le_bytes());
                b.extend_from_slice(hash);
                b
            };
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::ClientVersion as i16,
                        &cv_body,
                    ),
                })
                .await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::NewAccount as i16,
                        &[],
                    ),
                })
                .await;
            let mut lb = Vec::new();
            let _ = mir2_shared::binary::write_dotnet_string(&mut lb, username);
            let _ = mir2_shared::binary::write_dotnet_string(&mut lb, "testpass");
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::Login as i16,
                        &lb,
                    ),
                })
                .await;
            let ok = mir2_shared::enums::ServerPacketIds::LoginSuccess as i16;
            loop {
                let data = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                    .await
                    .expect("timeout")
                    .expect("closed");
                if data.len() >= 4 && i16::from_le_bytes([data[2], data[3]]) == ok {
                    break;
                }
            }
        }
        login(&gate_ref, 5, &mut rx5, "testuser").await;
        login(&gate_ref, 6, &mut rx6, "testuser2").await;

        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(),
            db_pool: db_pool.clone(),
            config: SocialActorConfig::default(),
        });
        let world_ref = WorldActor::spawn(WorldActorArgs {
            tick_interval_ms: 100,
            gate_ref: gate_ref.clone(),
            map_dir: std::path::PathBuf::from("."),
            spawn_dir: None,
            quest_dir: std::path::PathBuf::from("."),
            npc_script_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            goods_on: true,
            goods_max_stored: 15,
            goods_buy_back_time_minutes: 60,
            goods_buy_back_max_stored: 20,
            safe_zone_healing: false,
            archive_inactive_after_months: 12,
            monster_recall_enabled: true,
            monster_recall_range: 12,
            monster_recall_cooldown_ms: 5000,
            exp_mob_level_difference: true,
            refine_cfg: crate::util::config::RefineConfig::default(),
            replace_wedring_cost: 125,
            lover_exp_bonus: 5,
            mentor_exp_boost: 10,
            mentor_damage_boost: 10,
            mentor_skill_boost: true,
            mentee_exp_bank: 1,
            orbs_exp_list: Vec::new(),
            orbs_dmg_list: Vec::new(),
            orbs_def_list: Vec::new(),
            awakening_cfg: Default::default(),
            gem_cfg: Default::default(),
            hero_exp_list: Vec::new(),
            setup_cfg: Default::default(),
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            drop_gold: true,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            random_item_stats: Vec::new(),
            guild_buff_infos: Vec::new(),
        });
        let _ = gate_ref.ask(SetWorldRef { world_ref }).await;

        async fn start_game(
            gate_ref: &GateActorRef,
            session_id: u64,
            rx: &mut RxChannel,
            char_name: &str,
        ) {
            let nc_body = {
                let mut b = Vec::new();
                let _ = mir2_shared::binary::write_dotnet_string(&mut b, char_name);
                b.push(0u8);
                b.push(0u8);
                b
            };
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::NewCharacter as i16,
                        &nc_body,
                    ),
                })
                .await;
            let ncs = mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16;
            loop {
                let data = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                    .await
                    .expect("timeout")
                    .expect("closed");
                if data.len() >= 4 && i16::from_le_bytes([data[2], data[3]]) == ncs {
                    break;
                }
            }
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::StartGame as i16,
                        &0i32.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            // 等待 StartGame 响应
            let sg = mir2_shared::enums::ServerPacketIds::StartGame as i16;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            let mut found = false;
            while tokio::time::Instant::now() < deadline {
                let remaining = deadline - tokio::time::Instant::now();
                if let Ok(Some(data)) = tokio::time::timeout(remaining, rx.recv()).await {
                    if data.len() >= 4 && i16::from_le_bytes([data[2], data[3]]) == sg {
                        found = true;
                        break;
                    }
                } else {
                    break;
                }
            }
            assert!(
                found,
                "session {} StartGame response not received",
                session_id
            );
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

        let social_ref =
            crate::actors::social::SocialActor::spawn(crate::actors::social::SocialActorArgs {
                gate_ref: gate_ref.clone(),
                db_pool: db_pool.clone(),
                config: crate::actors::social::SocialActorConfig::default(),
            });
        let world_ref =
            crate::actors::world::WorldActor::spawn(crate::actors::world::WorldActorArgs {
                tick_interval_ms: 1000,
                gate_ref: gate_ref.clone(),
                map_dir: std::path::PathBuf::from("."),
                spawn_dir: None,
                quest_dir: std::path::PathBuf::from("."),
                npc_script_dir: std::path::PathBuf::from("."),
                db_pool: db_pool.clone(),
                social_ref,
                conquest_cfg: crate::util::config::ConquestConfig::default(),
                rested_cfg: crate::util::config::RestedConfig::default(),
                pvp_cfg: crate::util::config::PvpConfig::default(),
                health_regen_weight: 10,
                mana_regen_weight: 10,
                goods_hide_added_stats: true,
                goods_on: true,
                goods_max_stored: 15,
                goods_buy_back_time_minutes: 60,
                goods_buy_back_max_stored: 20,
                safe_zone_healing: false,
                archive_inactive_after_months: 12,
                monster_recall_enabled: true,
                monster_recall_range: 12,
                monster_recall_cooldown_ms: 5000,
                exp_mob_level_difference: true,
                refine_cfg: crate::util::config::RefineConfig::default(),
                replace_wedring_cost: 125,
                lover_exp_bonus: 5,
                mentor_exp_boost: 10,
                mentor_damage_boost: 10,
                mentor_skill_boost: true,
                mentee_exp_bank: 1,
                orbs_exp_list: Vec::new(),
                orbs_dmg_list: Vec::new(),
                orbs_def_list: Vec::new(),
                awakening_cfg: Default::default(),
                gem_cfg: Default::default(),
                hero_exp_list: Vec::new(),
                setup_cfg: Default::default(),
                drop_rate: 1.0,
                exp_rate: 1.0,
                experience_list: Vec::new(),
                item_timeout_ticks: 300,
                max_drop_gold: 2000,
                drop_gold: true,
                rarity_cfg: crate::util::config::RarityConfig::default(),
                notice_path: "Notice.txt".to_string(),
                death_exp_penalty_percent: 0,
                movement_pacing_ms: 0,
                fishing_cfg: crate::util::ini::FishingConfig::default(),
                random_item_stats: Vec::new(),
                guild_buff_infos: Vec::new(),
            });
        let _ = gate_ref
            .ask(crate::gate::actor::SetWorldRef { world_ref })
            .await;

        // StartGame
        let sg_body = 0i32.to_le_bytes().to_vec();
        let sg_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::StartGame as i16,
            &sg_body,
        );
        let _ = gate_ref
            .ask(crate::gate::actor::ClientData {
                session_id,
                data: sg_packet,
            })
            .await;

        // Drain StartGame packets
        let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if tokio::time::Instant::now() > drain_deadline {
                break;
            }
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await;
        }

        // Send Attack (spell=0, direction=Right(2))
        let mut attack_body = Vec::new();
        attack_body.push(0u8); // spell=0 (basic attack)
        attack_body.push(2u8); // direction=Right
        let attack_packet = build_packet_bytes(
            mir2_shared::enums::ClientPacketIds::Attack as i16,
            &attack_body,
        );
        let _ = gate_ref
            .ask(crate::gate::actor::ClientData {
                session_id,
                data: attack_packet,
            })
            .await;

        // Should receive some response - channel should stay open
        assert!(!rx.is_closed(), "Channel should remain open after attack");

        // Drain any response packets (attack might hit nothing, but shouldn't crash)
        let check_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut got_response = false;
        loop {
            if tokio::time::Instant::now() > check_deadline {
                break;
            }
            if let Ok(Some(_)) =
                tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
            {
                got_response = true;
                break;
            }
        }
        // Attack may not hit anything (no monsters spawned), but handler shouldn't crash
        assert!(!rx.is_closed(), "Channel should remain open");
    });
}

// ============================================================
// #2824：观战镜像回归（C# PlayerObject.BroadcastObservePackets 7 类）
// ============================================================

/// 等待指定 opcode 的包到达；超时返回 false。用于观察者通道断言。
async fn wait_opcode(rx: &mut RxChannel, opcode: i16, secs: u64) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(data)) if data.len() >= 4 => {
                if i16::from_le_bytes([data[2], data[3]]) == opcode {
                    return true;
                }
            }
            Ok(Some(_)) => continue,
            // 超时或通道关闭
            _ => return false,
        }
    }
    false
}

/// 目标玩家转身后，其观察者应收到 ObjectTurn 镜像（#2573 链路 + #2824 回归防护）。
///
/// 红检：删除 `WorldTurnRequest` 里的 `mirror_to_observers` 调用 →
/// `observer should receive mirrored ObjectTurn` 断言 FAILED。
#[test]
fn e2e_observe_mirrors_target_turn() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        // 目标(11) / 观察者(12) 共用一个 gate
        let gate_ref = GateActor::spawn(());
        let (tx11, mut rx11) = mpsc::unbounded_channel::<Vec<u8>>();
        let (tx12, mut rx12) = mpsc::unbounded_channel::<Vec<u8>>();
        for (sid, tx) in [(11u64, tx11), (12u64, tx12)] {
            let _ = gate_ref
                .ask(SessionCreated {
                    session_id: sid,
                    sender: tx,
                    ip: "127.0.0.1".to_string(),
                })
                .await;
        }

        let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
        let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
        let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

        async fn login(
            gate_ref: &GateActorRef,
            session_id: u64,
            rx: &mut RxChannel,
            username: &str,
        ) {
            let cv_body = {
                let mut b = Vec::new();
                let hash = b"test";
                b.extend_from_slice(&(hash.len() as i32).to_le_bytes());
                b.extend_from_slice(hash);
                b
            };
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::ClientVersion as i16,
                        &cv_body,
                    ),
                })
                .await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::NewAccount as i16,
                        &[],
                    ),
                })
                .await;
            let mut lb = Vec::new();
            let _ = mir2_shared::binary::write_dotnet_string(&mut lb, username);
            let _ = mir2_shared::binary::write_dotnet_string(&mut lb, "testpass");
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::Login as i16,
                        &lb,
                    ),
                })
                .await;
            let ok = mir2_shared::enums::ServerPacketIds::LoginSuccess as i16;
            assert!(wait_opcode(rx, ok, 3).await, "session {session_id} login");
        }

        async fn start_game(
            gate_ref: &GateActorRef,
            session_id: u64,
            rx: &mut RxChannel,
            char_name: &str,
        ) {
            let mut nc_body = Vec::new();
            let _ = mir2_shared::binary::write_dotnet_string(&mut nc_body, char_name);
            nc_body.push(0u8);
            nc_body.push(0u8);
            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::NewCharacter as i16,
                        &nc_body,
                    ),
                })
                .await;
            let ncs = mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16;
            assert!(
                wait_opcode(rx, ncs, 3).await,
                "session {session_id} NewCharacterSuccess"
            );

            let _ = gate_ref
                .ask(ClientData {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::StartGame as i16,
                        &0i32.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            let sg = mir2_shared::enums::ServerPacketIds::StartGame as i16;
            assert!(
                wait_opcode(rx, sg, 5).await,
                "session {session_id} StartGame"
            );
        }

        login(&gate_ref, 11, &mut rx11, "obstarget").await;
        // 观察者只登录、不进图：这样它不在 `players` 里，目标的转身**同图广播**不会发给它，
        // 观察者收到的 ObjectTurn 只可能来自观战镜像链路（C# 跨图观察者的等价场景）。
        login(&gate_ref, 12, &mut rx12, "obsviewer").await;

        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(),
            db_pool: db_pool.clone(),
            config: SocialActorConfig::default(),
        });
        let world_ref = WorldActor::spawn(WorldActorArgs {
            tick_interval_ms: 100,
            gate_ref: gate_ref.clone(),
            map_dir: std::path::PathBuf::from("."),
            spawn_dir: None,
            quest_dir: std::path::PathBuf::from("."),
            npc_script_dir: std::path::PathBuf::from("."),
            db_pool: db_pool.clone(),
            social_ref,
            conquest_cfg: crate::util::config::ConquestConfig::default(),
            rested_cfg: crate::util::config::RestedConfig::default(),
            pvp_cfg: crate::util::config::PvpConfig::default(),
            health_regen_weight: 10,
            mana_regen_weight: 10,
            goods_hide_added_stats: true,
            goods_on: true,
            goods_max_stored: 15,
            goods_buy_back_time_minutes: 60,
            goods_buy_back_max_stored: 20,
            safe_zone_healing: false,
            archive_inactive_after_months: 12,
            monster_recall_enabled: true,
            monster_recall_range: 12,
            monster_recall_cooldown_ms: 5000,
            exp_mob_level_difference: true,
            refine_cfg: crate::util::config::RefineConfig::default(),
            replace_wedring_cost: 125,
            lover_exp_bonus: 5,
            mentor_exp_boost: 10,
            mentor_damage_boost: 10,
            mentor_skill_boost: true,
            mentee_exp_bank: 1,
            orbs_exp_list: Vec::new(),
            orbs_dmg_list: Vec::new(),
            orbs_def_list: Vec::new(),
            awakening_cfg: Default::default(),
            gem_cfg: Default::default(),
            hero_exp_list: Vec::new(),
            setup_cfg: Default::default(),
            drop_rate: 1.0,
            exp_rate: 1.0,
            experience_list: Vec::new(),
            item_timeout_ticks: 300,
            max_drop_gold: 2000,
            drop_gold: true,
            rarity_cfg: crate::util::config::RarityConfig::default(),
            notice_path: "Notice.txt".to_string(),
            death_exp_penalty_percent: 0,
            movement_pacing_ms: 0,
            fishing_cfg: crate::util::ini::FishingConfig::default(),
            random_item_stats: Vec::new(),
            guild_buff_infos: Vec::new(),
        });
        let _ = gate_ref.ask(SetWorldRef { world_ref }).await;

        start_game(&gate_ref, 11, &mut rx11, "ObsTarget").await;

        // 目标开启 AllowObserve（C# 客户端命令 @ALLOWOBSERVE）
        let mut cmd_body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut cmd_body, "@ALLOWOBSERVE");
        cmd_body.extend_from_slice(&0i32.to_le_bytes());
        let _ = gate_ref
            .ask(ClientData {
                session_id: 11,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::Chat as i16,
                    &cmd_body,
                ),
            })
            .await;
        let allow = mir2_shared::enums::ServerPacketIds::AllowObserve as i16;
        assert!(
            wait_opcode(&mut rx11, allow, 3).await,
            "target AllowObserve ack"
        );

        // 观察者发起 Observe（目标名）
        let mut observe_body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut observe_body, "ObsTarget");
        let _ = gate_ref
            .ask(ClientData {
                session_id: 12,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::Observe as i16,
                    &observe_body,
                ),
            })
            .await;
        assert!(
            wait_opcode(&mut rx12, allow, 3).await,
            "observer should get AllowObserve(true) after observing"
        );

        // 目标转身 → 观察者应收到 ObjectTurn 镜像
        let _ = gate_ref
            .ask(ClientData {
                session_id: 11,
                data: build_packet_bytes(mir2_shared::enums::ClientPacketIds::Turn as i16, &[2u8]),
            })
            .await;
        let turn = mir2_shared::enums::ServerPacketIds::ObjectTurn as i16;
        assert!(
            wait_opcode(&mut rx12, turn, 3).await,
            "observer should receive mirrored ObjectTurn"
        );
    });
}
