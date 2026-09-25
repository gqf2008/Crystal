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
//! 用法：`MIR2_LEAK_PROBE=1` + 带该特性的二进制 → **每轮全员登出、整图清理之后的空闲点**
//! 打印 `MEM_PROBE_IDLE …`（与 `leak_plateau` 的 RSS 采样同相位，才可比），随后打印
//! `MEM_PROBE_DELTA …`：**这一轮变化最大的几个精确尺寸**（见 `actors/world/session.rs`）。
//! 默认关闭：`ENABLED=false` 时分配路径只多两次原子读，且整个模块在默认构建里**不编译**。
use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

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

// ---- 精确尺寸直方图（2026-09-25 加）----
// 三档分桶只能把嫌疑缩到「哪一档」，但同一个 257B–16KB 档里，384B 的会话状态与 4KB 的整页缓冲
// 修法完全不同。所以再记**精确尺寸**：每个 size 的活跃字节与活跃对象数，并在每次 dump 时输出
// 「与上一次 dump 相比变化最大的前 N 个尺寸」——一个泄漏会直接读成「每轮 +N 个 size=X 的对象」，
// 而 X 往往就能认出是哪个结构/缓冲区。
//
// 为什么用静态数组而不是 HashMap/Vec：dump 路径**不许分配**——这里的分配会经过同一个计数分配器，
// 把读数本身污染掉（一个 Vec 快照 512KB 就能盖过每轮 0.5MB 的信号）。
// 这些数组在 .bss 里，不进堆，也就不会出现在 live_bytes 里。
const EXACT_MAX: usize = 65536;
static EXACT_LIVE: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static EXACT_COUNT: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static PREV_LIVE: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static PREV_COUNT: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
// 基线快照（只在第一次 dump 时写一次）：**累计**增长比「每轮变化」鲁棒得多——一个每轮 +8KB 的
// 泄漏会被每轮的分配抖动完全淹没（实测：单轮 top-8 里全是 ±几 KB 的噪声，看不出指纹），
// 但 8 轮之后它在累计榜上就是 +64KB，从噪声里浮出来。
static BASE_LIVE: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static BASE_COUNT: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static BASE_SET: AtomicBool = AtomicBool::new(false);

// ---- 按调用点归因（2026-09-25 加）----
// 尺寸直方图能说「留下了多少、多大」，但说不出「谁留下的」。于是给每次分配加一个**标签**：
// 分配时把当前标签写进分配块前面 16 字节的头部，释放时读回来，按标签减。
// 标签由调用点在**可疑区域**用 `let _g = TagGuard::enter(TAG_X);` 圈定（RAII 退出即还原）。
//
// 为什么用头部而不是别的：释放时必须知道"这块是谁分配的"，头里存是最省事、最不依赖上下文的做法。
// 代价与边界（都刻意限制在探针构建里）：
//   * `align > 16` 的类型（SIMD 等）走**不带头部**的老路径：只计总量，不计标签（罕见，且在标签和里显账）；
//   * `realloc` 用"新分配 + 拷贝 + 释放旧的"实现（头部才容易维护），比原生 realloc 多一次拷贝；
//   * 标签和与总活跃字节的差额会在 dump 里打出来（`gap`），不是 0 就说明有未加标签的字节。
pub const TAG_UNKNOWN: usize = 0;
pub const TAG_MATERIALIZE: usize = 1;
pub const TAG_SPAWN_SEND: usize = 2;
pub const TAG_LOGIN: usize = 3;
pub const TAG_LOGOUT: usize = 4;
pub const TAG_CLEANUP: usize = 5;
pub const TAG_TICK: usize = 6;
pub const TAG_DB: usize = 7;
/// `spawn_npcs_and_monsters` 本体（**整段无 await**，所以这个标签没有"让出线程"造成的误归因）。
pub const TAG_SPAWN_FN: usize = 8;
pub const TAG_N: usize = 9;
pub const TAG_NAMES: [&str; TAG_N] = [
    "unknown",
    "materialize",
    "spawn_send",
    "login",
    "logout",
    "cleanup",
    "tick",
    "db",
    "spawn_fn",
];
const TAG_HEADER: usize = 16;
static LIVE_BY_TAG: [AtomicUsize; TAG_N] = [const { AtomicUsize::new(0) }; TAG_N];
static PREV_BY_TAG: [AtomicUsize; TAG_N] = [const { AtomicUsize::new(0) }; TAG_N];
/// 当前标签（线程局部）：把 tag 限制在设置它的那条线程上，避免多线程 runtime 下跨线程污染。
/// （已知边界：同一线程上交错执行的任务会共享它；所以标签只用于粗粒度定位，不能当精确归属。）
/// 这里单开一个模块放 `thread_local!`，是为了把 clippy 的 missing_const_for_thread_local
/// 允许范围压到最小——该 lint 对已经是 `const { … }` 的写法在 clippy 1.97 上仍报（误报），
/// 而属性挂在 `thread_local!` 调用上不会传进宏展开。
#[allow(clippy::missing_const_for_thread_local)]
mod cur_tag {
    use super::TAG_UNKNOWN;
    use std::cell::Cell;

    thread_local! {
        pub(super) static CUR_TAG: Cell<usize> = const { Cell::new(TAG_UNKNOWN) };
    }
}

/// 当前标签（TLS 取不到时按 `unknown`——TLS 析构期间分配器仍可能被调用）。
pub fn current_tag() -> usize {
    cur_tag::CUR_TAG
        .try_with(|c| c.get())
        .unwrap_or(TAG_UNKNOWN)
}

fn set_current_tag(tag: usize) {
    let _ = cur_tag::CUR_TAG.try_with(|c| c.set(tag));
}

/// RAII：进入可疑区域时打标签，离开自动还原（含 panic 路径）。
pub struct TagGuard(usize);

impl TagGuard {
    pub fn enter(tag: usize) -> Self {
        let prev = current_tag();
        set_current_tag(tag);
        TagGuard(prev)
    }
}

impl Drop for TagGuard {
    fn drop(&mut self) {
        set_current_tag(self.0);
    }
}

/// 输出各标签的活跃字节与本轮增量，并刷新上一轮快照；返回 `(标签和, 总活跃字节)` 供调用方打差额。
pub fn report_tags(mut emit: impl FnMut(&'static str, usize, i64)) -> (usize, usize) {
    let mut sum = 0usize;
    for tag in 0..TAG_N {
        let live = LIVE_BY_TAG[tag].load(Ordering::Relaxed);
        let prev = PREV_BY_TAG[tag].load(Ordering::Relaxed);
        sum += live;
        if live != 0 || prev != 0 {
            emit(TAG_NAMES[tag], live, live as i64 - prev as i64);
        }
    }
    for tag in 0..TAG_N {
        PREV_BY_TAG[tag].store(LIVE_BY_TAG[tag].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    (sum, LIVE_BYTES.load(Ordering::Relaxed))
}

#[inline]
fn track_tag(tag: usize, size: usize, add: bool) {
    if tag >= TAG_N {
        return;
    }
    if add {
        LIVE_BY_TAG[tag].fetch_add(size, Ordering::Relaxed);
    } else {
        LIVE_BY_TAG[tag].fetch_sub(size, Ordering::Relaxed);
    }
}

// ---- 按标签的精确尺寸直方图：只跟**一个**标签走（默认 `materialize`）----
// 全局直方图能说"这一轮各尺寸涨了多少"，但里面混着所有调用点；问"**materialize 这段**每轮留下的
// 是哪些尺寸"必须单独记。这里只留一个标签的两份快照（活字节 + 计数器），避免 9 个标签 × 65537 的数组。
// 想换标签就改 `TAG_FOCUS_SIZE` 常量重新构建（探针构建，不对外）。
pub const TAG_FOCUS_SIZE: usize = TAG_MATERIALIZE;
static FOCUS_LIVE: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static FOCUS_PREV: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static FOCUS_COUNT: [AtomicUsize; EXACT_MAX + 1] = [const { AtomicUsize::new(0) }; EXACT_MAX + 1];
static FOCUS_PREV_COUNT: [AtomicUsize; EXACT_MAX + 1] =
    [const { AtomicUsize::new(0) }; EXACT_MAX + 1];

#[inline]
fn track_focus(tag: usize, size: usize, add: bool) {
    if tag != TAG_FOCUS_SIZE || size == 0 || size > EXACT_MAX {
        return;
    }
    if add {
        FOCUS_LIVE[size].fetch_add(size, Ordering::Relaxed);
        FOCUS_COUNT[size].fetch_add(1, Ordering::Relaxed);
    } else {
        FOCUS_LIVE[size].fetch_sub(size, Ordering::Relaxed);
        FOCUS_COUNT[size].fetch_sub(1, Ordering::Relaxed);
    }
}

/// 输出 `TAG_FOCUS_SIZE` 这个标签上「本轮变化最大的前 `top` 个尺寸」并刷新快照。
/// 全程不分配（emit 直接打印），返回 `(本轮该标签的正增量之和, 负增量之和)`。
pub fn report_focus_sizes(top: usize, mut emit: impl FnMut(usize, usize, i64, i64)) -> (i64, i64) {
    let mut picked = [usize::MAX; 16];
    let top = top.min(picked.len());
    let (mut pos_sum, mut neg_sum) = (0i64, 0i64);
    for size in 1..=EXACT_MAX {
        let d = FOCUS_LIVE[size].load(Ordering::Relaxed) as i64
            - FOCUS_PREV[size].load(Ordering::Relaxed) as i64;
        if d > 0 {
            pos_sum += d;
        } else {
            neg_sum += d;
        }
    }
    for _ in 0..top {
        let mut best: Option<(usize, u64, i64, i64, usize)> = None;
        for size in 1..=EXACT_MAX {
            if picked.contains(&size) {
                continue;
            }
            let live = FOCUS_LIVE[size].load(Ordering::Relaxed);
            let prev = FOCUS_PREV[size].load(Ordering::Relaxed);
            if live == 0 && prev == 0 {
                continue;
            }
            let dbytes = live as i64 - prev as i64;
            let score = dbytes.unsigned_abs();
            if best.is_none_or(|(_, s, _, _, _)| score > s) {
                let count = FOCUS_COUNT[size].load(Ordering::Relaxed);
                let dcount = count as i64 - FOCUS_PREV_COUNT[size].load(Ordering::Relaxed) as i64;
                best = Some((size, score, dbytes, dcount, count));
            }
        }
        match best {
            Some((size, _, dbytes, dcount, count)) => {
                if let Some(slot) = picked.iter_mut().find(|c| **c == usize::MAX) {
                    *slot = size;
                }
                emit(size, count, dbytes, dcount);
            }
            None => break,
        }
    }
    for size in 1..=EXACT_MAX {
        FOCUS_PREV[size].store(FOCUS_LIVE[size].load(Ordering::Relaxed), Ordering::Relaxed);
        FOCUS_PREV_COUNT[size].store(FOCUS_COUNT[size].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    (pos_sum, neg_sum)
}

/// `TAG_FOCUS_SIZE` 当前挂着的活跃字节合计（用于确认"这个标签确实在涨"）。
pub fn focus_live_total() -> usize {
    FOCUS_LIVE
        .iter()
        .skip(1)
        .map(|slot| slot.load(Ordering::Relaxed))
        .sum()
}

// ---- 活块登记表：把"此刻真的活着"的块按**轮次**记下来，idle 时做普查 ----
// 为什么需要它：标签/直方图都是"按分配路径记账"，会被 churn 洗掉（实测：区间残差忽大忽小，
// 分不出泄漏）；Windows 的 HeapWalk 又看不到 Rust 分配器的块（见 LESSON）。
// 这里换成本分配器自己登记：只登记 `>= REG_MIN` 的块（数量少、成本可控），每条记录带
// **登记时的轮次**，于是 idle 时可以直接列出"本轮新分配且现在仍存活"的块，连内容一起打出来。
// 边界：登记表是定容的开放寻址表（满了就丢计数，不覆盖别人的记录）；内容读取与释放存在竞态
// （探针专用，只在 idle 点读一次）。
const REG_SLOTS: usize = 1 << 20;
const REG_MIN: usize = 256;
const REG_PROBE: usize = 8;
static REG_PTR: [AtomicUsize; REG_SLOTS] = [const { AtomicUsize::new(0) }; REG_SLOTS];
// meta = size(低 32 位) | tag(bit 32..40) | gen(bit 40..64)
static REG_META: [AtomicU64; REG_SLOTS] = [const { AtomicU64::new(0) }; REG_SLOTS];
static REG_GEN: AtomicU64 = AtomicU64::new(0);
static REG_DROPPED: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn reg_hash(ptr: usize) -> usize {
    let mut h = ptr as u64;
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    (h as usize) & (REG_SLOTS - 1)
}

#[inline]
fn reg_insert(ptr: usize, size: usize, tag: usize) {
    if size < REG_MIN || size > u32::MAX as usize || ptr == 0 {
        return;
    }
    let meta =
        (size as u64) | ((tag as u64 & 0xff) << 32) | (REG_GEN.load(Ordering::Relaxed) << 40);
    let base = reg_hash(ptr);
    for i in 0..REG_PROBE {
        let slot = (base + i) & (REG_SLOTS - 1);
        if REG_PTR[slot]
            .compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            REG_META[slot].store(meta, Ordering::Release);
            return;
        }
    }
    REG_DROPPED.fetch_add(1, Ordering::Relaxed);
}

#[inline]
fn reg_remove(ptr: usize) {
    let base = reg_hash(ptr);
    for i in 0..REG_PROBE {
        let slot = (base + i) & (REG_SLOTS - 1);
        let cur = REG_PTR[slot].load(Ordering::Acquire);
        if cur == ptr {
            REG_PTR[slot].store(0, Ordering::Release);
            return;
        }
        if cur == 0 {
            return; // 空槽 ⇒ 这条记录不存在（或早已被删）
        }
    }
}

/// 枚举「登记于 `min_age` 轮之前、且现在仍在表里」的块：把 `(size, tag, ptr, 前 48 字节)` 交给 `emit`，
/// 然后轮次 +1。返回 `(登记的活块总数, 活块字节合计, 因表满丢弃的登记次数)`。
///
/// `min_age = 1` ⇒ "上上轮之前登记的还在"，`min_age = 2` ⇒ "熬过至少两轮还在"。
/// **`min_age >= 2` 才是泄漏判据**：它对"分配后很快释放"的 churn 免疫（实测新登记集合里混着大量
/// 下一轮就消失的块），只剩真正在攒的那些。
/// ⚠️ 内容的读取与其它线程的释放存在竞态（读已释放块的旧字节）——探针专用，且只读前 48 字节。
pub fn registry_report(
    min_age: u64,
    limit: usize,
    mut emit: impl FnMut(usize, usize, usize, &[u8]),
) -> (usize, usize, usize) {
    let target_gen = REG_GEN.load(Ordering::Relaxed);
    let mut live_entries = 0usize;
    let mut live_bytes = 0usize;
    let mut emitted = 0usize;
    let mut buf = [0u8; 48];
    for slot in 0..REG_SLOTS {
        let ptr = REG_PTR[slot].load(Ordering::Acquire);
        if ptr == 0 {
            continue;
        }
        let meta = REG_META[slot].load(Ordering::Acquire);
        let size = (meta & 0xffff_ffff) as usize;
        let tag = ((meta >> 32) & 0xff) as usize;
        live_entries += 1;
        live_bytes += size;
        let gen = meta >> 40;
        if target_gen.saturating_sub(gen) < min_age || emitted >= limit || size == 0 {
            continue;
        }
        let n = size.min(buf.len());
        // 竞态下的"尽力读取"：读到的是这块内存当时的字节
        unsafe {
            ptr::copy_nonoverlapping(ptr as *const u8, buf.as_mut_ptr(), n);
        }
        emitted += 1;
        emit(size, tag, ptr, &buf[..n]);
    }
    REG_GEN.fetch_add(1, Ordering::Relaxed);
    (
        live_entries,
        live_bytes,
        REG_DROPPED.load(Ordering::Relaxed),
    )
}

/// 某个标签当前挂着多少活跃字节（线程局部标签本身在分配时就记进了对应槽位）。
/// 用途：在**同步窗口**（无 await）前后各读一次，差值就是"这段时间里本线程按该标签分配的净字节"——
/// 它不受其它线程/任务的分配干扰，比全局 `live_bytes` 的窗口差干净得多。
pub fn tag_live(tag: usize) -> usize {
    if tag >= TAG_N {
        return 0;
    }
    LIVE_BY_TAG[tag].load(Ordering::Relaxed)
}

/// 记一次精确尺寸的分配/释放。超过 `EXACT_MAX` 的尺寸不记（仍然计入总量与三档分桶）。
///
/// 边界（已知且刻意保留）：`fetch_sub` 在记账不平衡时会回绕（例如内存是在 `set_enabled(true)`
/// 之前分配的、却在之后释放）。这与既有的 `LIVE_BYTES`/分桶口径一致——本探针只判**趋势**，
/// 不做精确记账；真出现回绕，打印出来的绝对值会明显荒谬，不会被误读成小增量。
#[inline]
fn track(size: usize, add: bool) {
    if size == 0 || size > EXACT_MAX {
        return;
    }
    if add {
        EXACT_LIVE[size].fetch_add(size, Ordering::Relaxed);
        EXACT_COUNT[size].fetch_add(1, Ordering::Relaxed);
    } else {
        EXACT_LIVE[size].fetch_sub(size, Ordering::Relaxed);
        EXACT_COUNT[size].fetch_sub(1, Ordering::Relaxed);
    }
}

/// 输出「与上一 dump 相比变化最大的前 `top` 个尺寸」并就地刷新上一轮快照。
/// 全程不分配（`emit` 由调用方直接打印，返回值不带堆对象）。
///
/// 参数含义（按顺序）：`size`、当前活跃字节、当前活跃对象数、本轮字节增量、本轮对象数增量。
///
/// 返回 `(本轮全部尺寸的正增量之和, 负增量之和)`：两者一起看就知道增长是**集中**还是**弥散**
/// （集中在少数尺寸 = 某个结构在攒；弥散在很多尺寸 = 更像通用池/多种对象一起攒）。
/// 第一次调用会顺便落下**基线快照**（供 `report_cumulative` 用）。
pub fn report_changed_sizes(
    top: usize,
    mut emit: impl FnMut(usize, usize, usize, i64, i64),
) -> (i64, i64) {
    let first = !BASE_SET.swap(true, Ordering::Relaxed);
    let mut picked = [usize::MAX; 16];
    let top = top.min(picked.len());
    let (mut pos_sum, mut neg_sum) = (0i64, 0i64);
    for size in 1..=EXACT_MAX {
        let d = EXACT_LIVE[size].load(Ordering::Relaxed) as i64
            - PREV_LIVE[size].load(Ordering::Relaxed) as i64;
        if d > 0 {
            pos_sum += d;
        } else {
            neg_sum += d;
        }
    }
    for _ in 0..top {
        let mut best: Option<(usize, u64, i64, i64, usize, usize)> = None;
        for size in 1..=EXACT_MAX {
            if picked.contains(&size) {
                continue;
            }
            let live = EXACT_LIVE[size].load(Ordering::Relaxed);
            let prev_live = PREV_LIVE[size].load(Ordering::Relaxed);
            if live == 0 && prev_live == 0 {
                continue;
            }
            let dbytes = live as i64 - prev_live as i64;
            let score = dbytes.unsigned_abs();
            if best.is_none_or(|(_, s, _, _, _, _)| score > s) {
                let count = EXACT_COUNT[size].load(Ordering::Relaxed);
                let dcount = count as i64 - PREV_COUNT[size].load(Ordering::Relaxed) as i64;
                best = Some((size, score, dbytes, dcount, live, count));
            }
        }
        match best {
            Some((size, _, dbytes, dcount, live, count)) => {
                if let Some(slot) = picked.iter_mut().find(|c| **c == usize::MAX) {
                    *slot = size;
                }
                emit(size, live, count, dbytes, dcount);
            }
            None => break,
        }
    }
    // 原地快照（不分配）：下一次 dump 的「变化」就是相对此刻的差。
    for size in 1..=EXACT_MAX {
        let live = EXACT_LIVE[size].load(Ordering::Relaxed);
        let count = EXACT_COUNT[size].load(Ordering::Relaxed);
        PREV_LIVE[size].store(live, Ordering::Relaxed);
        PREV_COUNT[size].store(count, Ordering::Relaxed);
        if first {
            BASE_LIVE[size].store(live, Ordering::Relaxed);
            BASE_COUNT[size].store(count, Ordering::Relaxed);
        }
    }
    (pos_sum, neg_sum)
}

/// 累计增长榜：相对**第一次 dump 的基线快照**，只列**仍在增长**的尺寸，按累计增量排序。
/// 这就是找指纹的那一栏——泄漏的特征是「同一个 size 每轮都涨一点，累计几十~几百 KB」。
/// 返回 `(全部尺寸的累计正增量之和, 负增量之和)`：与 `MEM_PROBE_NET` 配合，判断增长是
/// **集中在少数尺寸**（top-10 就能盖住总量）还是**弥散在很多尺寸**（top-10 只占零头）。
pub fn report_cumulative(
    top: usize,
    mut emit: impl FnMut(usize, usize, usize, i64, i64),
) -> (i64, i64) {
    if !BASE_SET.load(Ordering::Relaxed) {
        return (0, 0);
    }
    let (mut pos_sum, mut neg_sum) = (0i64, 0i64);
    for size in 1..=EXACT_MAX {
        let grow = EXACT_LIVE[size].load(Ordering::Relaxed) as i64
            - BASE_LIVE[size].load(Ordering::Relaxed) as i64;
        if grow > 0 {
            pos_sum += grow;
        } else {
            neg_sum += grow;
        }
    }
    let mut picked = [usize::MAX; 32];
    let top = top.min(picked.len());
    for _ in 0..top {
        let mut best: Option<(usize, i64, usize, usize)> = None;
        for size in 1..=EXACT_MAX {
            if picked.contains(&size) {
                continue;
            }
            let live = EXACT_LIVE[size].load(Ordering::Relaxed);
            let base = BASE_LIVE[size].load(Ordering::Relaxed);
            if live == 0 && base == 0 {
                continue;
            }
            let grow = live as i64 - base as i64;
            if grow <= 0 {
                continue; // 累计榜只看净增长
            }
            if best.is_none_or(|(_, g, _, _)| grow > g) {
                best = Some((size, grow, live, EXACT_COUNT[size].load(Ordering::Relaxed)));
            }
        }
        match best {
            Some((size, grow, live, count)) => {
                if let Some(slot) = picked.iter_mut().find(|c| **c == usize::MAX) {
                    *slot = size;
                }
                let dcount = count as i64 - BASE_COUNT[size].load(Ordering::Relaxed) as i64;
                emit(size, live, count, grow, dcount);
            }
            None => break,
        }
    }
    (pos_sum, neg_sum)
}

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
        // 带头部路径：把当前标签写进块首，释放时据此按标签减（见 TAG_* 注释）。
        //
        // ⚠️ 布局决策**只看 layout.align()，绝不看 enabled()**：一旦它跟着运行时开关变，
        // 「开关打开之前分配、打开之后释放」的块就会走错分支——dealloc 会把 ptr-16 当成标签头读
        // （其实是无关数据），再按带头的布局去 free，直接堆损坏。实测过一次：服务端起得来、
        // 20 个客户端**全部**登录失败（ok=0 failed=20），日志里没有 panic，很难查。
        // 代价：mem-probe 构建里 16 字节对齐以内的分配总是多 16 字节头部（哪怕没开探针）；
        // 默认构建不含本模块，所以生产零影响。
        if layout.align() <= TAG_HEADER {
            let Ok(tagged) =
                Layout::from_size_align(layout.size().saturating_add(TAG_HEADER), TAG_HEADER)
            else {
                return System.alloc(layout);
            };
            let base = System.alloc(tagged);
            if base.is_null() {
                return base;
            }
            let tag = current_tag();
            ptr::write(base as *mut usize, tag);
            if enabled() {
                LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                bucket(layout.size()).fetch_add(layout.size(), Ordering::Relaxed);
                track(layout.size(), true);
                track_tag(tag, layout.size(), true);
                track_focus(tag, layout.size(), true);
                reg_insert(base.add(TAG_HEADER) as usize, layout.size(), tag);
            }
            return base.add(TAG_HEADER);
        }
        let ptr = System.alloc(layout);
        if !ptr.is_null() && enabled() {
            LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            bucket(layout.size()).fetch_add(layout.size(), Ordering::Relaxed);
            track(layout.size(), true);
            // 无头部的路径（align > 16）没有标签，用 255 当"未打标签"哨兵，别让它从普查里漏掉
            reg_insert(ptr as usize, layout.size(), 255);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.align() <= TAG_HEADER {
            let base = ptr.sub(TAG_HEADER);
            let tag = ptr::read(base as *const usize);
            if enabled() {
                LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
                DEALLOCS.fetch_add(1, Ordering::Relaxed);
                bucket(layout.size()).fetch_sub(layout.size(), Ordering::Relaxed);
                track(layout.size(), false);
                track_tag(tag, layout.size(), false);
                track_focus(tag, layout.size(), false);
                reg_remove(ptr as usize);
            }
            // 同样的参数在 alloc 里成功过，这里理论不可达；真到这儿也只能不释放（不能按老布局释放，
            // 那会把 base 而不是 ptr 交给系统分配器）。
            if let Ok(tagged) =
                Layout::from_size_align(layout.size().saturating_add(TAG_HEADER), TAG_HEADER)
            {
                System.dealloc(base, tagged);
            }
            return;
        }
        if enabled() {
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
            DEALLOCS.fetch_add(1, Ordering::Relaxed);
            bucket(layout.size()).fetch_sub(layout.size(), Ordering::Relaxed);
            track(layout.size(), false);
            reg_remove(ptr as usize);
        }
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // 带头部路径：用"新分配 + 拷贝 + 释放旧的"实现（头部需要跟着搬家）。
        // 新块用的是**当前**标签（旧标签在旧块头部，这里刻意不读——realloc 的语义是"改大小"，
        // 调用点若在别的区域里，把新块记到当前标签更贴近"这次改动是谁引起的"）。
        // 布局决策同样只看 align（见 alloc 的说明）。
        if layout.align() <= TAG_HEADER {
            let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
                return ptr::null_mut();
            };
            let np = self.alloc(new_layout);
            if np.is_null() {
                return np;
            }
            ptr::copy_nonoverlapping(ptr, np, layout.size().min(new_size));
            self.dealloc(ptr, layout);
            return np;
        }
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() && enabled() {
            // 近似：把 realloc 记成「先加新尺寸、再减旧尺寸」，稳态下 live_bytes 的**趋势**仍然正确
            // （本项目只用它判「有没有持续增长」，不用它做精确记账）。
            LIVE_BYTES.fetch_add(new_size, Ordering::Relaxed);
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
            bucket(new_size).fetch_add(new_size, Ordering::Relaxed);
            bucket(layout.size()).fetch_sub(layout.size(), Ordering::Relaxed);
            track(new_size, true);
            track(layout.size(), false);
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
