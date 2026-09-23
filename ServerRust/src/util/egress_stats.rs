//! 出站包量统计：按 opcode 累计「包个数 × 尺寸」，用于给「入场路径载荷」定量。
//!
//! 背景：CAPACITY.md §4「~3.3MB/会话」与「每轮登出不归还 ~13MB」的拆解走到入场路径后，
//! 需要先回答「入场到底发了多少、最大的几个包是谁」，再决定要不要做「同内容包复用缓冲」。
//! 没有测量就不动结构，因此本模块**只记账、零行为改变**：
//! 未开开关时 `GateActor` 的每包开销只是一次 bool 判断。
//!
//! 开关：环境变量 `MIR2_EGRESS_STATS=1`（进程内只读一次）。
//! 输出：会话收尾时一行 `EGRESS_STATS session=<id> packets=.. bytes=.. top=..`。

use std::collections::HashMap;

/// 线帧头长度：u16 总长 + i16 opcode（见 [`crate::util::wire::build_packet_bytes`]）
pub const FRAME_HEADER_SIZE: usize = 4;

/// 从线帧里取 opcode；帧短于帧头返回 `None`（不 panic，统计不应成为新的崩溃点）
pub fn frame_opcode(frame: &[u8]) -> Option<i16> {
    if frame.len() < FRAME_HEADER_SIZE {
        return None;
    }
    Some(i16::from_le_bytes([frame[2], frame[3]]))
}

/// 是否启用出站统计（`MIR2_EGRESS_STATS=1`/`true`）
pub fn egress_stats_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        matches!(
            std::env::var("MIR2_EGRESS_STATS").as_deref(),
            Ok("1") | Ok("true")
        )
    })
}

/// 单个 opcode 的累计量
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OpcodeStat {
    /// 包个数
    pub count: u64,
    /// 累计字节（含帧头）
    pub bytes: u64,
    /// 单个最大字节数
    pub max: u64,
}

/// 单会话（或任意一组帧）的出站累计
#[derive(Debug, Default, Clone)]
pub struct EgressStats {
    /// 总包数
    pub packets: u64,
    /// 总字节数（含帧头）
    pub bytes: u64,
    per_opcode: HashMap<i16, OpcodeStat>,
}

impl EgressStats {
    /// 记一帧
    pub fn record(&mut self, frame: &[u8]) {
        let len = frame.len() as u64;
        self.packets += 1;
        self.bytes += len;
        if let Some(op) = frame_opcode(frame) {
            let entry = self.per_opcode.entry(op).or_default();
            entry.count += 1;
            entry.bytes += len;
            entry.max = entry.max.max(len);
        }
    }

    /// 某 opcode 的累计量（无记录返回 `None`）
    pub fn opcode(&self, op: i16) -> Option<OpcodeStat> {
        self.per_opcode.get(&op).copied()
    }

    /// 出现过的 opcode 个数
    pub fn opcode_count(&self) -> usize {
        self.per_opcode.len()
    }

    /// 按累计字节降序取前 `limit` 名（同字节数时按 opcode 升序，保证输出确定性）
    pub fn top(&self, limit: usize) -> Vec<(i16, OpcodeStat)> {
        let mut all: Vec<(i16, OpcodeStat)> =
            self.per_opcode.iter().map(|(k, v)| (*k, *v)).collect();
        all.sort_by(|a, b| b.1.bytes.cmp(&a.1.bytes).then(a.0.cmp(&b.0)));
        all.truncate(limit);
        all
    }

    /// 单行摘要：`packets=.. bytes=.. opcodes=.. top=<op>:<count>/<bytes>/<max>,...`
    pub fn summary(&self, limit: usize) -> String {
        let top = self
            .top(limit)
            .iter()
            .map(|(op, s)| format!("{}:{}/{}/{}", op, s.count, s.bytes, s.max))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "packets={} bytes={} opcodes={} top={}",
            self.packets,
            self.bytes,
            self.opcode_count(),
            top
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::wire::build_packet_bytes;

    #[test]
    fn frame_opcode_reads_wire_header_and_rejects_short_frames() {
        let frame = build_packet_bytes(0x1234, &[1, 2, 3]);
        assert_eq!(frame.len(), FRAME_HEADER_SIZE + 3);
        assert_eq!(frame_opcode(&frame), Some(0x1234));
        // 短帧/空帧不能 panic（统计不得成为新的崩溃点）
        assert_eq!(frame_opcode(&[0u8, 1, 2]), None);
        assert_eq!(frame_opcode(&[]), None);
    }

    #[test]
    fn egress_stats_aggregates_count_bytes_and_max_per_opcode() {
        let mut stats = EgressStats::default();
        stats.record(&build_packet_bytes(10, &[0u8; 6])); // 10 字节
        stats.record(&build_packet_bytes(10, &[0u8; 96])); // 100 字节
        stats.record(&build_packet_bytes(20, &[0u8; 16])); // 20 字节
        assert_eq!(stats.packets, 3);
        assert_eq!(stats.bytes, 10 + 100 + 20);
        assert_eq!(stats.opcode_count(), 2);
        let op10 = stats.opcode(10).expect("opcode 10 recorded");
        assert_eq!(op10.count, 2);
        assert_eq!(op10.bytes, 110);
        assert_eq!(op10.max, 100);
        let op20 = stats.opcode(20).expect("opcode 20 recorded");
        assert_eq!(op20.count, 1);
        assert_eq!(op20.bytes, 20);
        assert_eq!(op20.max, 20);
    }

    #[test]
    fn egress_top_is_ordered_by_bytes_desc_then_opcode() {
        let mut stats = EgressStats::default();
        stats.record(&build_packet_bytes(7, &[0u8; 16])); // 20 字节
        stats.record(&build_packet_bytes(5, &[0u8; 996])); // 1000 字节
        stats.record(&build_packet_bytes(9, &[0u8; 996])); // 1000 字节（并列）
        let top = stats.top(3);
        assert_eq!(top.len(), 3);
        // 并列 1000 字节时按 opcode 升序 → 5 在前
        assert_eq!(top[0].0, 5);
        assert_eq!(top[1].0, 9);
        assert_eq!(top[2].0, 7);
        let summary = stats.summary(2);
        assert!(summary.contains("packets=3"), "summary={summary}");
        assert!(summary.contains("bytes=2020"), "summary={summary}");
        assert!(summary.contains("5:1/1000/1000"), "summary={summary}");
    }
}
