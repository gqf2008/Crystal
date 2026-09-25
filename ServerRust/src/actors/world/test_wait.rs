//! e2e 等待原语的**唯一实现**（此前 `wait_opcode_body`/`recv_until` 在 e2e.rs、
//! mail.rs、map_sync.rs、session.rs、item.rs 各抄了一份，共 5 份）。
//!
//! ## 为什么不是「固定死线」
//!
//! 这些用例原本写死 `3s`/`5s`（少数 1–2s）的**绝对死线**。死线只在「慢或没来」时才起作用
//! （包一到就返回，happy path 不受影响），但**当机器被并行测试 / 其他 agent 的编译压满时，
//! 一次 actor 往返会超过 3–5s** ⇒ 整套跑出**假红**。实测（2026-09-25，本机 8 路满载）：
//! 同一份全绿的代码在满载下 3 次里红 2 次，而且每次红的用例都不同
//! （`e2e_awakening_success_path` / `e2e_script_move_teleport_resyncs_map_objects` /
//! `e2e_send_mail_stamp_removal_failure_aborts` / `e2e_sell_item_gold_cap_rejected_and_item_kept` /
//! `e2e_dup_kick_cancels_rental_and_notifies_old_client` / …），
//! 失败点都是 `recv_until(.., 3)` / `wait_opcode_body(.., 5)` 的 `expect("... missing")`。
//! 这类假红会污染「本地门禁全绿」这条交付判据，所以必须在**一处**修掉。
//!
//! ## 现在的语义（静默窗口 + 进度续期 + 硬上限）
//!
//! * **静默窗口** = `secs × 标度`：连续这么久没收到**任何**包才放弃；
//! * **进度续期**：期间只要还有包进来（说明管道在推进），就继续等；
//! * **硬上限** = 静默窗口 × 3：包一直流但目标包始终不来时，仍会失败（不无限等）。
//!
//! 对「期待某包**不出现**」的负控断言没有副作用：没有包 ⇒ 静默窗口一到就返回（与旧行为同）。
//! 标度默认 ×2（配合 `.cargo/config.toml` 的 `RUST_TEST_THREADS=2`：并行度是主防线，窗口是兜底；
//! ×4 会让「期待不出现」的负控断言多花一倍时间——本仓的负控有 1–2s 窗口，串行跑时这点直接进总时长）。
//! 可用环境变量 `MIR2_TEST_WAIT_SCALE` 覆盖（CI 想快速失败设 1，极端负载设 4–8）。

use std::time::Duration;

use tokio::sync::mpsc;

pub type RxChannel = mpsc::Receiver<Vec<u8>>;

/// 静默窗口标度（见模块文档）。非法值/0 回退默认 2。
pub fn wait_scale() -> u64 {
    std::env::var("MIR2_TEST_WAIT_SCALE")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(2)
}

/// 等到目标 opcode 为止，返回其 body（其余包被丢弃）。超时/通道关闭返回 None。
pub async fn wait_opcode_body(rx: &mut RxChannel, opcode: i16, secs: u64) -> Option<Vec<u8>> {
    let idle = Duration::from_secs(secs.max(1) * wait_scale());
    let hard = idle * 3;
    let start = tokio::time::Instant::now();
    while start.elapsed() < hard {
        match tokio::time::timeout(idle, rx.recv()).await {
            Ok(Some(data)) if data.len() >= 4 => {
                if i16::from_le_bytes([data[2], data[3]]) == opcode {
                    return Some(data[4..].to_vec());
                }
                // 不是目标包：算「有进度」，静默窗口重新计时（循环自然续期）
            }
            Ok(Some(_)) => continue,
            Ok(None) => return None, // 通道关闭
            Err(_) => return None,   // 静默窗口内没有任何包：放弃
        }
    }
    None
}

/// 等到目标 opcode 为止，返回（目标 body, 期间收到的全部（opcode, body））。语义同
/// [`wait_opcode_body`]（静默窗口 + 进度续期 + 硬上限）。
pub async fn recv_until(
    rx: &mut RxChannel,
    opcode: i16,
    secs: u64,
) -> Option<(Vec<u8>, Vec<(i16, Vec<u8>)>)> {
    let idle = Duration::from_secs(secs.max(1) * wait_scale());
    let hard = idle * 3;
    let start = tokio::time::Instant::now();
    let mut seen = Vec::new();
    while start.elapsed() < hard {
        match tokio::time::timeout(idle, rx.recv()).await {
            Ok(Some(data)) if data.len() >= 4 => {
                let op = i16::from_le_bytes([data[2], data[3]]);
                let body = data[4..].to_vec();
                seen.push((op, body.clone()));
                if op == opcode {
                    return Some((body, seen));
                }
            }
            Ok(Some(_)) => continue,
            Ok(None) => return None,
            Err(_) => return None,
        }
    }
    None
}
