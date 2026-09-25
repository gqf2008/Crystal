//! 内存探针（仅 `--features mem-probe` 时编译）：一个**计数分配器**。
//!
//! 为什么需要它（2026-09-25）：`tools/ops/leak_plateau.ps1`（20 会话连登连退）在 release 构建上实测
//! 「空闲 RSS 每轮约 +0.6MB，12 轮不收敛」，而夹具自己的 release 基线是 0.1 MB/轮。同一轮里已用
//! 只读探针排除了「按 session 键的容器没清」（33 个容器断线后全为 0）。剩下的可能截然不同：
//!   - **真泄漏**：进程持有的活跃分配字节持续增长；
//!   - **分配器高水位/arena**：活跃字节平稳，但 malloc arena/空闲链表把 RSS 留在高位。
//! 只看 RSS 区分不了这两者，而修法完全不同（前者要改代码、后者并不影响可用性）。
//! 计数分配器给出 `live_bytes = Σ分配 - Σ释放`：**它持续涨才是真泄漏**。
//!
//! 用法：`MIR2_LEAK_PROBE=1` + 带该特性的二进制 → `PlayerDisconnected` 清理后打印
//! `MEM_PROBE sid=… live_bytes=… allocs=… deallocs=…`（见 `actors/world/session.rs`）。
//! 默认关闭：`ENABLED=false` 时分配路径只多两次原子读，且整个模块在默认构建里**不编译**。
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCS: AtomicUsize = AtomicUsize::new(0);
// 按尺寸分档的活跃字节（2026-09-25 加）：总 live 在涨时，看**哪一档**在涨能大幅缩小嫌疑范围——
// 小对象档（≤256B）多半是每会话的小状态/索引；中档（≤16KB）像玩家对象/背包向量；
// 大档（>16KB）更像整图/整表克隆或大缓冲区。
static LIVE_SMALL: AtomicUsize = AtomicUsize::new(0); // <= 256 B
static LIVE_MID: AtomicUsize = AtomicUsize::new(0); // <= 16 KiB
static LIVE_BIG: AtomicUsize = AtomicUsize::new(0); // > 16 KiB

/// 打开/关闭计数（由 `main.rs` 按 `MIR2_LEAK_PROBE` 决定；关闭时热路径只多一次原子读）。
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// 返回 `(live_bytes, allocs, deallocs)`。
pub fn stats() -> (usize, usize, usize) {
    (
        LIVE_BYTES.load(Ordering::Relaxed),
        ALLOCS.load(Ordering::Relaxed),
        DEALLOCS.load(Ordering::Relaxed),
    )
}

/// 返回按尺寸分档的活跃字节 `(<=256B, <=16KiB, >16KiB)`。
pub fn stats_by_size() -> (usize, usize, usize) {
    (
        LIVE_SMALL.load(Ordering::Relaxed),
        LIVE_MID.load(Ordering::Relaxed),
        LIVE_BIG.load(Ordering::Relaxed),
    )
}

#[inline]
fn bucket(size: usize) -> &'static AtomicUsize {
    if size <= 256 {
        &LIVE_SMALL
    } else if size <= 16 * 1024 {
        &LIVE_MID
    } else {
        &LIVE_BIG
    }
}

pub struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() && enabled() {
            LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            bucket(layout.size()).fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if enabled() {
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
            DEALLOCS.fetch_add(1, Ordering::Relaxed);
            bucket(layout.size()).fetch_sub(layout.size(), Ordering::Relaxed);
        }
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() && enabled() {
            // 近似：把 realloc 记成「先加新尺寸、再减旧尺寸」，稳态下 live_bytes 的**趋势**仍然正确
            // （本项目只用它判「有没有持续增长」，不用它做精确记账）。
            LIVE_BYTES.fetch_add(new_size, Ordering::Relaxed);
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
            bucket(new_size).fetch_add(new_size, Ordering::Relaxed);
            bucket(layout.size()).fetch_sub(layout.size(), Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL_COUNTING: CountingAllocator = CountingAllocator;

#[cfg(test)]
mod tests {
    use super::*;

    /// 只断言「计数器确实在动」——测试是并行跑的，其他测试的分配/释放会同时改动全局计数，
    /// 所以这里不比对具体数值（那种断言在并行下必然 flaky；真实信号看 drill 的 live_bytes 曲线）。
    #[test]
    fn counting_allocator_is_wired() {
        set_enabled(true);
        let (live0, allocs0, deallocs0) = stats();
        let buf: Vec<u8> = vec![0u8; 4 << 20];
        let (live1, allocs1, _) = stats();
        assert!(allocs1 > allocs0, "分配计数必须前进（说明分配器被接上了）");
        assert_ne!(live1, live0, "分配后活跃字节必须发生变化");
        drop(buf);
        let (_, _, deallocs2) = stats();
        assert!(deallocs2 > deallocs0, "释放计数必须前进");
        set_enabled(false);
    }
    // 注意：**不要**再写「关闭时不应改变计数」这类断言——测试是并行跑的，另一个测试可能正把
    // 计数打开，那条断言必然 flaky。默认关闭这件事由「`ENABLED` 初值为 false + 主程序只在
    // `MIR2_LEAK_PROBE=1` 时调用 `set_enabled(true)` + 默认构建根本不编译本模块」三件事保证。
}
