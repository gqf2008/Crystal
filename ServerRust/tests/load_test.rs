//! Phase 3.2: 负载测试 — 模拟并发 TCP 客户端,测量连接 + 认证吞吐量。
//!
//! 默认 #[ignore](避免 CI 超时)。手动运行:
//!   cargo test --test load_test -- --ignored --nocapture
//!
//! 输出:并发数 / 成功率 / 平均延迟 / 最大延迟 / 总耗时

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use kameo::actor::Spawn;

use crystal_server::gate::actor::{GateActor, run_gate_listener, SetAccountRef, SetMaxConnections};
use crystal_server::actors::account::AccountActor;
use crystal_server::db;

const XOR_KEY: u8 = 0xAA;

fn xor(data: &[u8]) -> Vec<u8> {
    data.iter().map(|b| b ^ XOR_KEY).collect()
}

fn make_packet(opcode: i16, body: &[u8]) -> Vec<u8> {
    let inner_len = (4 + body.len()) as u16;
    let mut inner = Vec::with_capacity(4 + body.len());
    inner.extend_from_slice(&inner_len.to_le_bytes());
    inner.extend_from_slice(&opcode.to_le_bytes());
    inner.extend_from_slice(body);
    let mut out = Vec::with_capacity(2 + inner.len());
    out.extend_from_slice(&(inner.len() as u16).to_le_bytes());
    out.extend(xor(&inner));
    out
}

async fn recv_packet(stream: &mut TcpStream) -> (i16, Vec<u8>) {
    let mut len_buf = [0u8; 2];
    stream.read_exact(&mut len_buf).await.unwrap();
    let outer_len = u16::from_le_bytes(len_buf) as usize;
    let mut enc = vec![0u8; outer_len];
    stream.read_exact(&mut enc).await.unwrap();
    let inner = xor(&enc);
    let inner_len = u16::from_le_bytes([inner[0], inner[1]]) as usize;
    let opcode = i16::from_le_bytes([inner[2], inner[3]]);
    let body = if inner_len > 4 { inner[4..inner_len].to_vec() } else { Vec::new() };
    (opcode, body)
}

fn find_free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let p = l.local_addr().unwrap().port();
    drop(l);
    p
}

/// 单个客户端:connect → ClientVersion → NewAccount → Login。
/// 返回耗时(ms)。
async fn simulate_client(port: u16, id: usize) -> Result<u64, String> {
    let start = Instant::now();

    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("connect: {}", e))?;

    // 1. Connected
    let (op, _) = recv_packet(&mut stream).await;
    if op != 0 { return Err(format!("expected Connected, got {}", op)); }

    // 2. ClientVersion
    let mut body = Vec::new();
    body.extend_from_slice(&4i32.to_le_bytes());
    body.extend_from_slice(b"test");
    stream.write_all(&make_packet(0, &body)).await.unwrap();
    let _ = recv_packet(&mut stream).await; // response

    // 3. NewAccount
    stream.write_all(&make_packet(3, &[])).await.unwrap();
    let _ = recv_packet(&mut stream).await;

    // 4. Login
    let mut login_body = Vec::new();
    mir2_shared::binary::write_dotnet_string(&mut login_body, &format!("user{}", id)).unwrap();
    mir2_shared::binary::write_dotnet_string(&mut login_body, "pass").unwrap();
    stream.write_all(&make_packet(5, &login_body)).await.unwrap();

    // 5. Wait for LoginSuccess (opcode=9)
    //    AccountActor 处理 Argon2 密码哈希是串行的,并发登录会被串行化。
    //    给 30 秒超时容忍 Argon2 + actor 调度延迟。
    let (op, resp_body) = tokio::time::timeout(
        Duration::from_secs(30),
        recv_packet(&mut stream),
    ).await.map_err(|_| "login timeout (>30s)".to_string())?;

    if op == 9 {
        // Verify character count = 0
        if resp_body.len() >= 4 {
            let count = i32::from_le_bytes(resp_body[0..4].try_into().unwrap_or([0; 4]));
            if count != 0 { return Err(format!("unexpected char count {}", count)); }
        }
        Ok(start.elapsed().as_millis() as u64)
    } else {
        Err(format!("login failed opcode={}", op))
    }
}

async fn run_load_test(concurrent: usize) {
    let port = find_free_port();
    let gate_ref = GateActor::spawn_with_mailbox((), kameo::mailbox::unbounded());
    let _ = gate_ref.ask(SetMaxConnections(concurrent + 100)).await;
    let db_pool = db::init_db_pool("sqlite::memory:").await.unwrap();
    let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool));
    let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

    let g2 = gate_ref.clone();
    tokio::spawn(async move {
        let _ = run_gate_listener(format!("127.0.0.1:{}", port), g2).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("\n=== Load Test: {} concurrent clients ===", concurrent);
    let test_start = Instant::now();

    let success = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicUsize::new(0));
    let latencies = Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));

    let mut handles = Vec::new();
    for i in 0..concurrent {
        let s = success.clone();
        let f = fail.clone();
        let lats = latencies.clone();
        handles.push(tokio::spawn(async move {
            match simulate_client(port, i).await {
                Ok(ms) => {
                    s.fetch_add(1, Ordering::Relaxed);
                    lats.lock().unwrap().push(ms);
                }
                Err(e) => {
                    f.fetch_add(1, Ordering::Relaxed);
                    eprintln!("  Client {} failed: {}", i, e);
                }
            }
        }));
    }
    for h in handles { let _ = h.await; }

    let total_ms = test_start.elapsed().as_millis();
    let ok = success.load(Ordering::Relaxed);
    let failed = fail.load(Ordering::Relaxed);
    let mut lats = latencies.lock().unwrap().clone();
    lats.sort_unstable();

    let avg = if lats.is_empty() { 0.0 } else { lats.iter().sum::<u64>() as f64 / lats.len() as f64 };
    let p50 = if lats.is_empty() { 0 } else { lats[lats.len() / 2] };
    let p99 = if lats.is_empty() { 0 } else { lats[(lats.len() as f64 * 0.99) as usize] };
    let max = lats.last().copied().unwrap_or(0);

    println!("  Success:   {}/{} ({:.1}%)", ok, concurrent, ok as f64 / concurrent as f64 * 100.0);
    println!("  Failed:    {}", failed);
    println!("  Total:     {}ms", total_ms);
    println!("  Throughput: {:.1} clients/sec", concurrent as f64 / (total_ms as f64 / 1000.0));
    println!("  Latency avg: {:.0}ms  p50: {}ms  p99: {}ms  max: {}ms", avg, p50, p99, max);
    println!();

    assert!(ok > 0, "At least some clients should succeed");
    let min_success_rate = if concurrent <= 100 { 0.95 } else { 0.80 };
    assert!(
        ok as f64 / concurrent as f64 >= min_success_rate,
        "Success rate {:.1}% < {:.0}% threshold",
        ok as f64 / concurrent as f64 * 100.0,
        min_success_rate * 100.0
    );
}

#[tokio::test]
#[ignore]
async fn load_test_20() {
    run_load_test(20).await;
}

#[tokio::test]
#[ignore]
async fn load_test_100() {
    run_load_test(100).await;
}

#[tokio::test]
#[ignore]
async fn load_test_300() {
    run_load_test(300).await;
}
