//! Gate 加固回归测试（上线审查 阻断6 / 严重10 / 严重11 / 严重23）。
//!
//! 覆盖：
//!   1. 阻断6：RangeAttack 19/20/21 字节载荷不 panic 且行为正确（协议全长 21 字节）
//!   2. 严重10：未登录会话 NewCharacter 被拒并断开；已登录正常受理
//!   3. 严重11：未登录会话 ChangePassword 静默拒绝（AccountActor 不受理）；
//!      RequirePasswordChange 强制改密流程保留、且不得越权改他人账号
//!   4. 严重23：慢读客户端发送缓冲积满即踢线
//!
//! 注：这些测试走 GateActor 的 pub 消息接口 + 内存 SQLite，不依赖 Daneo1989 地图数据。

use kameo::actor::Spawn;
use std::time::Duration;
use tokio::sync::mpsc;

use crystal_server::actors::account::AccountActor;
use crystal_server::db;
use crystal_server::gate::actor::{
    parse_range_attack_payload, ClientData, GateActor, LoginResult, SendToClient, SessionCreated,
    SetAccountRef, TestProbeSession,
};
use crystal_server::util::wire::build_packet_bytes;
use mir2_shared::enums::{ClientPacketIds, ServerPacketIds};

/// 阻断6：RangeAttack 协议全长 21 字节（1+4+4+4+4+4）。
/// 旧实现检查 <19 却读 payload[17..21]，19/20 字节载荷越界 panic。
#[test]
fn range_attack_short_payload_no_panic() {
    assert_eq!(parse_range_attack_payload(&[]), None);
    assert_eq!(parse_range_attack_payload(&[0u8; 19]), None);
    assert_eq!(parse_range_attack_payload(&[0u8; 20]), None);
}

#[test]
fn range_attack_parses_21_bytes() {
    let mut payload = Vec::new();
    payload.push(3u8); // dir
    payload.extend_from_slice(&100i32.to_le_bytes()); // x
    payload.extend_from_slice(&200i32.to_le_bytes()); // y
    payload.extend_from_slice(&0xDEADu32.to_le_bytes()); // target_id
    payload.extend_from_slice(&(-5i32).to_le_bytes()); // tx
    payload.extend_from_slice(&7i32.to_le_bytes()); // ty
    assert_eq!(payload.len(), 21);
    assert_eq!(
        parse_range_attack_payload(&payload),
        Some((3, 0xDEAD, -5, 7))
    );
    // 多余字节忽略
    let mut extra = payload.clone();
    extra.extend_from_slice(&[0xFF, 0xFF]);
    assert_eq!(
        parse_range_attack_payload(&extra),
        Some((3, 0xDEAD, -5, 7))
    );
}

/// 严重10：未登录会话发 NewCharacter → 拒绝并断开（会话被移除）。
/// 旧代码会以角色名冒充账号名建角（unwrap_or_else(|| name.clone())）。
#[tokio::test]
async fn new_character_rejected_when_not_logged_in() {
    let gate_ref = GateActor::spawn(());
    let (tx, _rx) = mpsc::channel(8);
    gate_ref
        .ask(SessionCreated {
            session_id: 1,
            sender: tx,
            ip: "127.0.0.1".to_string(),
        })
        .await
        .unwrap();
    let (has_session, _) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(has_session);

    // NewCharacter payload: [name DotNetString][gender u8][class u8]
    let mut body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut body, "Hero1").unwrap();
    body.push(0);
    body.push(0);
    let data = build_packet_bytes(ClientPacketIds::NewCharacter as i16, &body);
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();

    let (has_session, has_username) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(!has_session, "未登录建角必须断开会话");
    assert!(!has_username);
}

/// 严重10：已登录会话发 NewCharacter 正常受理（会话保留，不被误踢）。
#[tokio::test]
async fn new_character_allowed_when_logged_in() {
    let gate_ref = GateActor::spawn(());
    let (tx, _rx) = mpsc::channel(8);
    gate_ref
        .ask(SessionCreated {
            session_id: 1,
            sender: tx,
            ip: "127.0.0.1".to_string(),
        })
        .await
        .unwrap();
    gate_ref
        .ask(LoginResult {
            session_id: 1,
            success: true,
            username: "acc1".to_string(),
            characters: vec![],
            banned_until: None,
            require_password_change: false,
        })
        .await
        .unwrap();

    let mut body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut body, "Hero1").unwrap();
    body.push(0);
    body.push(0);
    let data = build_packet_bytes(ClientPacketIds::NewCharacter as i16, &body);
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();

    let (has_session, has_username) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(has_session, "已登录建角不得踢线");
    assert!(has_username);
}

async fn spawn_gate_with_account() -> (
    kameo::actor::ActorRef<GateActor>,
    mpsc::Sender<Vec<u8>>,
    mpsc::Receiver<Vec<u8>>,
) {
    let gate_ref = GateActor::spawn(());
    let db_pool = db::init_db_pool("sqlite::memory:")
        .await
        .expect("in-memory db");
    let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool));
    gate_ref
        .ask(SetAccountRef { account_ref })
        .await
        .unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    gate_ref
        .ask(SessionCreated {
            session_id: 1,
            sender: tx.clone(),
            ip: "127.0.0.1".to_string(),
        })
        .await
        .unwrap();
    // SessionCreated 会自动下发 S.Connected，先吃掉，避免干扰后续断言
    let connected = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("Connected packet")
        .expect("channel open");
    let opcode = i16::from_le_bytes([connected[2], connected[3]]);
    assert_eq!(opcode, ServerPacketIds::Connected as i16);
    (gate_ref, tx, rx)
}

fn change_password_packet(account: &str, old: &str, new: &str) -> Vec<u8> {
    // ChangePassword: [account_id][current_password][new_password] 均为 DotNetString
    let mut body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut body, account).unwrap();
    mir2_shared::binary::write_dotnet_string(&mut body, old).unwrap();
    mir2_shared::binary::write_dotnet_string(&mut body, new).unwrap();
    build_packet_bytes(ClientPacketIds::ChangePassword as i16, &body)
}

/// 严重11：未登录会话发 ChangePassword → 静默拒绝，AccountActor 不受理
/// （会话发送通道收不到任何 ChangePassword 结果包；旧实现会转发并回 Result=4/5，
/// 构成未登录口令预言机）。
#[tokio::test]
async fn change_password_rejected_when_not_logged_in() {
    let (gate_ref, _tx, mut rx) = spawn_gate_with_account().await;

    let data = change_password_packet("someone", "oldpass", "newpass1");
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();

    let got = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(got.is_err(), "未登录改密不得产生回包: {:?}", got.ok());

    // 会话保留（仅拒绝，不踢线）
    let (has_session, _) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(has_session);
}

/// 严重11：RequirePasswordChange 强制改密流程不被登录态门禁堵死——
/// 登录返回 Result=5 的会话可改密（仅登记的账号），AccountActor 正常回包。
#[tokio::test]
async fn change_password_allowed_for_pending_password_change() {
    let (gate_ref, _tx, mut rx) = spawn_gate_with_account().await;

    // 登录返回 RequirePasswordChange（S.Login Result=5），登记改密待办
    gate_ref
        .ask(LoginResult {
            session_id: 1,
            success: false,
            username: "mustchange".to_string(),
            characters: vec![],
            banned_until: None,
            require_password_change: true,
        })
        .await
        .unwrap();
    // 吃掉 S.Login{Result=5} 回包
    let _ = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;

    let data = change_password_packet("mustchange", "oldpass", "newpass1");
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();

    // AccountActor 受理：账号不存在 → S.ChangePassword{Result=4}
    let got = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("强制改密流程必须受理并回包")
        .expect("channel open");
    // 内层包: [len u16][opcode i16][result u8]
    let opcode = i16::from_le_bytes([got[2], got[3]]);
    assert_eq!(opcode, ServerPacketIds::ChangePassword as i16);
    assert_eq!(got[4], 4u8);
}

/// 严重11：强制改密待办会话不得改「别人的」账号（越权拒绝，无回包）。
#[tokio::test]
async fn change_password_pending_cannot_target_other_account() {
    let (gate_ref, _tx, mut rx) = spawn_gate_with_account().await;

    gate_ref
        .ask(LoginResult {
            session_id: 1,
            success: false,
            username: "mustchange".to_string(),
            characters: vec![],
            banned_until: None,
            require_password_change: true,
        })
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;

    let data = change_password_packet("victim", "oldpass", "newpass1");
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();

    let got = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(got.is_err(), "待办会话改他人账号必须静默拒绝");
}

/// 严重23：慢读客户端发送缓冲积满即被踢线（有界通道 + try_send）。
#[tokio::test]
async fn slow_reader_kicked_when_send_buffer_full() {
    let gate_ref = GateActor::spawn(());
    // 接收端永远不读 → 通道积满
    let (tx, _rx) = mpsc::channel(8);
    gate_ref
        .ask(SessionCreated {
            session_id: 1,
            sender: tx,
            ip: "127.0.0.1".to_string(),
        })
        .await
        .unwrap();
    // 容量 8 的通道，连续发到积满
    for _ in 0..16 {
        gate_ref
            .ask(SendToClient {
                session_id: 1,
                data: vec![0u8; 4],
            })
            .await
            .unwrap();
    }
    let (has_session, _) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(!has_session, "发送缓冲积满必须踢线");
}

/// 踢线实效化：被踢（会话已注销）的连接再发 ClientData 必须被拒——
/// 旧实现入口无会话注册检查，被踢连接重发 ClientVersion/Login 仍被处理。
/// 红检：删掉 ClientData 入口的 sessions.contains_key 前置拒绝 → 能收到
/// ClientVersion accepted 回包 → 断言 FAILED。
#[tokio::test]
async fn client_data_rejected_after_session_kick() {
    let gate_ref = GateActor::spawn(());
    // 接收端不读 → 通道积满即踢线（严重23 路径）
    let (tx, _rx) = mpsc::channel(8);
    gate_ref
        .ask(SessionCreated {
            session_id: 1,
            sender: tx,
            ip: "127.0.0.1".to_string(),
        })
        .await
        .unwrap();
    for _ in 0..16 {
        gate_ref
            .ask(SendToClient {
                session_id: 1,
                data: vec![0u8; 4],
            })
            .await
            .unwrap();
    }
    let (has_session, _) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(!has_session, "前提：会话已被踢");

    // 被踢连接再发 ClientVersion——必须静默拒绝（不得回 accepted）
    let mut cv_body = Vec::new();
    let hash = b"kicked";
    cv_body.extend_from_slice(&(hash.len() as i32).to_le_bytes());
    cv_body.extend_from_slice(hash);
    let data = build_packet_bytes(ClientPacketIds::ClientVersion as i16, &cv_body);
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();

    // 会话仍不存在、且无任何状态变化（入口前置拒绝）
    let (has_session, has_username) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(!has_session && !has_username, "被踢连接的 ClientData 必须被拒");
}

/// 踢线后门禁：被踢连接重新 Login 成功（AccountActor 视角凭据合法）时，
/// LoginResult 不得回插 session_usernames——旧实现无条件回插，门禁全放行。
/// 红检：删掉 LoginResult 的 sessions.contains_key 确认 → has_username=true → FAILED。
#[tokio::test]
async fn login_result_not_reinserted_after_kick() {
    let gate_ref = GateActor::spawn(());
    let (tx, _rx) = mpsc::channel(8);
    gate_ref
        .ask(SessionCreated {
            session_id: 1,
            sender: tx,
            ip: "127.0.0.1".to_string(),
        })
        .await
        .unwrap();
    // 踢线
    for _ in 0..16 {
        gate_ref
            .ask(SendToClient {
                session_id: 1,
                data: vec![0u8; 4],
            })
            .await
            .unwrap();
    }
    let (has_session, _) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(!has_session, "前提：会话已被踢");

    // 被踢连接的 Login 结果返回（AccountActor 无法感知踢线）——不得回插映射
    gate_ref
        .ask(LoginResult {
            session_id: 1,
            success: true,
            username: "attacker".to_string(),
            characters: vec![],
            banned_until: None,
            require_password_change: false,
        })
        .await
        .unwrap();

    let (has_session, has_username) = gate_ref
        .ask(TestProbeSession { session_id: 1 })
        .await
        .unwrap();
    assert!(!has_session);
    assert!(!has_username, "被踢会话不得回插登录映射（门禁不得放行）");
}

/// 跨账号喷洒：已登录会话只能改【本会话登录的账号】的密码——
/// 旧实现登录态放行任意 account_id，构成对他人账号旧密码的在线爆破面。
/// 红检：授权判断改回 session_usernames.contains_key → AccountActor 回 Result=4 → FAILED。
#[tokio::test]
async fn change_password_logged_in_cannot_target_other_account() {
    let (gate_ref, _tx, mut rx) = spawn_gate_with_account().await;

    gate_ref
        .ask(LoginResult {
            session_id: 1,
            success: true,
            username: "acc1".to_string(),
            characters: vec![],
            banned_until: None,
            require_password_change: false,
        })
        .await
        .unwrap();
    // 吃掉 LoginSuccess 回包
    let _ = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;

    // 已登录 acc1，却请求改 victim 的密码——必须静默拒绝
    let data = change_password_packet("victim", "oldpass", "newpass1");
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();
    let got = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(got.is_err(), "登录态改他人账号必须静默拒绝: {:?}", got.ok());

    // 改【本会话账号】仍受理（AccountActor 回包；账号未注册 → Result=4）
    let data = change_password_packet("acc1", "oldpass", "newpass1");
    gate_ref.ask(ClientData { session_id: 1, data }).await.unwrap();
    let got = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("改本会话账号必须受理并回包")
        .expect("channel open");
    let opcode = i16::from_le_bytes([got[2], got[3]]);
    assert_eq!(opcode, ServerPacketIds::ChangePassword as i16);
    assert_eq!(got[4], 4u8);
}
