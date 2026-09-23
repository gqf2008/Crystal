//! 落库失败的**补偿队列**（2026-09-23，`crystal-persist-loud` 的后续项）。
//!
//! **2026-09-24 owner 拍板**：落库失败一律**直接反馈到客户端**；服务器侧**暂不实现**保护/补偿，
//! 也**不要求落盘 spool**。本模块因此**不是上线门槛**，按「既有低成本兜底」保留：DB 短暂不可写时
//! 少丢一点数据；真出问题以「客户端可见报错（`db::persist_failure_notice` +
//! `world::notify_persist_failure`）+ `PERSIST_LOST` 日志」为准。若将来 owner 要求 spool 再做。
//!
//! 背景：写锁长于 `busy_timeout` 时，下线/落库会失败。PR #3045 已把它从「静默 warn 即放弃」
//! 改成「`ERROR PERSIST_LOST` 可告警」，但**数据仍然丢**。本模块补上另一半：失败的写入
//! 留在队列里，等存储恢复后在世界 tick 上有界补写（replay）。
//!
//! 为什么是**队列 + 每 tick 有界 flush**（而不是原地重试/后台任务）：
//! - 原地重试会把连接占用从 5s 拉到 ~10s，把登录读路径挤到超时（实测见 `tools/ops/README.md` §5c-1）；
//! - 队列把重试挪到世界自己的 tick 上，每 tick 只试 1–2 条，代价被摊平且**可观测**
//!   （`PERSIST_REPLAY` / `PERSIST_DROPPED` 日志）；
//! - 同 `(玩家, 类型)` 只保留**最新**一份（旧快照被新快照覆盖即正确，见 `PendingWrite::key`）。
//!
//! 上限与放弃语义：队列最多 [`MAX_ITEMS`] 条、单条最多 [`MAX_ATTEMPTS`] 次；超限/超次数都会
//! `error!` 记 `PERSIST_DROPPED`（**响亮**地丢，不静默）。「长锁下允许丢到什么程度才算可上线」
//! 是产品门槛问题，已单独开线程交人工拍板，不在这里自行定义。

use super::*;
use std::future::Future;
use std::pin::Pin;

/// 一条待补写的落库
pub(crate) enum PendingWrite {
    /// 角色全量保存（背包/装备/仓库/任务/邮件…，最值钱的一条）
    Character {
        account: String,
        state: Box<crate::actors::player::PlayerState>,
    },
    /// 英雄列表
    Heroes {
        name: String,
        heroes: Vec<db::DbHero>,
    },
    /// 驯服宠物（随角色一起进出世界，丢了就永久少一只）
    Pets {
        name: String,
        pets: Vec<TamedPetInfo>,
    },
    /// 最后下线时间（簿记）
    LastAccess { name: String, ts: i64 },
}

impl PendingWrite {
    /// 类型标签（日志用）
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            PendingWrite::Character { .. } => "player_character",
            PendingWrite::Heroes { .. } => "heroes",
            PendingWrite::Pets { .. } => "player_pets",
            PendingWrite::LastAccess { .. } => "last_access",
        }
    }

    /// 去重键：同一玩家同一类型只保留最新（后写胜过先写是正确语义）
    fn key(&self) -> (String, &'static str) {
        match self {
            PendingWrite::Character { state, .. } => (state.name.clone(), "player_character"),
            PendingWrite::Heroes { name, .. } => (name.clone(), "heroes"),
            PendingWrite::Pets { name, .. } => (name.clone(), "player_pets"),
            PendingWrite::LastAccess { name, .. } => (name.clone(), "last_access"),
        }
    }

    /// 真正落库（与直接路径用的是同一批 db 函数，保证语义一致）
    async fn write(&self, pool: &DbPool) -> anyhow::Result<()> {
        match self {
            PendingWrite::Character { account, state } => {
                db::save_character(pool, state, account).await
            }
            PendingWrite::Heroes { name, heroes } => db::save_heroes(pool, name, heroes).await,
            PendingWrite::Pets { name, pets } => db::save_player_pets(pool, name, pets).await,
            PendingWrite::LastAccess { name, ts } => db::update_last_access(pool, name, *ts).await,
        }
    }
}

/// 补写出口：生产用 [`DbSink`]；单测注入假实现来制造「失败 N 次后成功 / 一直失败」序列。
pub(crate) trait PersistSink: Sync {
    fn save<'a>(
        &'a self,
        write: &'a PendingWrite,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;
}

/// 真实补写出口（打到 DB）
pub(crate) struct DbSink<'a>(pub(crate) &'a DbPool);

impl PersistSink for DbSink<'_> {
    fn save<'a>(
        &'a self,
        write: &'a PendingWrite,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(write.write(self.0))
    }
}

/// 队列里的一条 + 已尝试次数
struct PendingAttempt {
    write: PendingWrite,
    attempts: u32,
}

/// 队列容量上限（只在存储故障时才会用满；角色快照一条几十 KB，128 条约数 MB 量级）
pub(crate) const MAX_ITEMS: usize = 128;
/// 单条最多尝试次数，超过则响亮丢弃
pub(crate) const MAX_ATTEMPTS: u32 = 20;
/// 每 tick 最多补写几条（100ms/tick 时约 20 条/秒）
pub(crate) const FLUSH_PER_TICK: usize = 2;

/// 补写统计（供日志/测试断言）
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct FlushStats {
    /// 本次补写成功条数
    pub ok: usize,
    /// 本次仍失败（留在队列）条数
    pub retried: usize,
    /// 因超过尝试上限被丢弃条数
    pub dropped: usize,
}

/// 落库失败补偿队列
#[derive(Default)]
pub(crate) struct PendingPersists {
    items: Vec<PendingAttempt>,
    pub(crate) enqueued_total: u64,
    pub(crate) replayed_total: u64,
    pub(crate) dropped_total: u64,
}

impl PendingPersists {
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// 入队（同 key 覆盖旧快照；超过容量丢**最旧**的一条并响亮记录）
    pub(crate) fn enqueue(&mut self, write: PendingWrite) {
        let key = write.key();
        if let Some(slot) = self.items.iter_mut().find(|it| it.write.key() == key) {
            slot.write = write;
            slot.attempts = 0;
            self.enqueued_total += 1;
            return;
        }
        if self.items.len() >= MAX_ITEMS {
            let old = self.items.remove(0);
            self.dropped_total += 1;
            error!(
                "PERSIST_DROPPED {} reason=queue_full player={} attempts={}",
                old.write.kind(),
                old.write.key().0,
                old.attempts
            );
        }
        self.items.push(PendingAttempt { write, attempts: 0 });
        self.enqueued_total += 1;
    }

    /// 每 tick 调用：最多补写 [`FLUSH_PER_TICK`] 条
    pub(crate) async fn flush(&mut self, pool: &DbPool) -> FlushStats {
        self.flush_with(&DbSink(pool), FLUSH_PER_TICK, MAX_ATTEMPTS)
            .await
    }

    /// 带预算/上限的补写
    pub(crate) async fn flush_with(
        &mut self,
        sink: &dyn PersistSink,
        budget: usize,
        max_attempts: u32,
    ) -> FlushStats {
        let mut stats = FlushStats::default();
        if self.items.is_empty() || budget == 0 {
            return stats;
        }
        let mut keep: Vec<PendingAttempt> = Vec::with_capacity(self.items.len());
        let mut deferred: Vec<PendingAttempt> = Vec::new();
        let mut attempted = 0usize;
        for mut item in std::mem::take(&mut self.items) {
            if attempted >= budget {
                keep.push(item);
                continue;
            }
            attempted += 1;
            let player = item.write.key().0;
            let kind = item.write.kind();
            match sink.save(&item.write).await {
                Ok(()) => {
                    stats.ok += 1;
                    self.replayed_total += 1;
                    info!(
                        "PERSIST_REPLAY ok {kind} player={player} attempts={}",
                        item.attempts + 1
                    );
                }
                Err(e) => {
                    item.attempts += 1;
                    if item.attempts >= max_attempts {
                        stats.dropped += 1;
                        self.dropped_total += 1;
                        error!(
                            "PERSIST_DROPPED {kind} reason=max_attempts player={player} attempts={} err={e}",
                            item.attempts
                        );
                    } else {
                        // 轮转：本轮失败的挪到队尾，避免队首两条把别人饿死
                        stats.retried += 1;
                        deferred.push(item);
                    }
                }
            }
        }
        keep.extend(deferred);
        self.items = keep;
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// 造一条不碰库的待写（LastAccess 最轻）
    fn item(name: &str) -> PendingWrite {
        PendingWrite::LastAccess {
            name: name.to_string(),
            ts: 1,
        }
    }

    /// 假补写出口：按 `fail_remaining` 决定失败几次，并记录调用顺序
    struct FakeSink {
        fail_remaining: Mutex<usize>,
        calls: Mutex<Vec<String>>,
    }

    impl FakeSink {
        fn new(fail_times: usize) -> Self {
            Self {
                fail_remaining: Mutex::new(fail_times),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl PersistSink for FakeSink {
        fn save<'a>(
            &'a self,
            write: &'a PendingWrite,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
            let name = write.key().0;
            self.calls.lock().unwrap().push(name.clone());
            let fail = {
                let mut left = self.fail_remaining.lock().unwrap();
                if *left > 0 {
                    *left -= 1;
                    true
                } else {
                    false
                }
            };
            Box::pin(async move {
                if fail {
                    Err(anyhow::anyhow!("database is locked"))
                } else {
                    Ok(())
                }
            })
        }
    }

    /// 门禁（补偿队列核心语义）：**失败必须留队**、成功后出队；失败的条目挪到队尾，
    /// 不会把排在后面的玩家饿死。
    ///
    /// 阳性对照（落地时实做）：把失败分支里的 `deferred.push(item)` 删掉（即失败就丢）
    /// → 第一条断言（失败后仍在队列）立即红。
    #[tokio::test]
    async fn failed_write_stays_queued_and_rotates() {
        let mut q = PendingPersists::default();
        q.enqueue(item("alice"));
        q.enqueue(item("bob"));

        // 预算 1：第一轮只试队首 alice，且 alice 这次失败 → 留在队列、挪到队尾
        let sink = FakeSink::new(1);
        let s1 = q.flush_with(&sink, 1, 20).await;
        assert_eq!(s1.retried, 1, "失败的一条必须留在队列里");
        assert_eq!(q.len(), 2, "失败后不得丢条目");
        assert_eq!(sink.calls.lock().unwrap().as_slice(), ["alice".to_string()]);

        // 第二轮预算 1：应轮到 bob（说明失败条目被挪到队尾，没有饿死别人）
        let s2 = q.flush_with(&sink, 1, 20).await;
        assert_eq!(s2.ok, 1, "第二轮应补写成功 bob");
        assert_eq!(q.len(), 1, "成功条目出队");
        assert_eq!(
            sink.calls.lock().unwrap().last().map(String::as_str),
            Some("bob"),
            "第二轮必须轮到 bob"
        );

        // 第三轮：alice 恢复 → 补写成功、队列清空
        let s3 = q.flush_with(&sink, 1, 20).await;
        assert_eq!(s3.ok, 1);
        assert!(q.is_empty(), "恢复后队列必须清空");
        assert_eq!(q.replayed_total, 2);
        assert_eq!(q.dropped_total, 0, "本轮不该有任何丢弃");
    }

    /// 门禁：超过尝试上限必须**响亮丢弃**（既不死循环、也不无限涨内存）
    #[tokio::test]
    async fn gives_up_loudly_after_max_attempts() {
        let mut q = PendingPersists::default();
        q.enqueue(item("carol"));
        let sink = FakeSink::new(usize::MAX);
        let mut stats = FlushStats::default();
        for _ in 0..3 {
            stats = q.flush_with(&sink, 1, 3).await;
        }
        assert_eq!(stats.dropped, 1, "第 3 次尝试应触发丢弃");
        assert!(q.is_empty(), "丢弃后不得留在队列");
        assert_eq!(q.dropped_total, 1);
    }

    /// 门禁：同一 `(玩家, 类型)` 只保留最新快照（不重复占位）
    #[tokio::test]
    async fn dedups_same_player_and_kind() {
        let mut q = PendingPersists::default();
        q.enqueue(PendingWrite::LastAccess {
            name: "dave".into(),
            ts: 1,
        });
        q.enqueue(PendingWrite::LastAccess {
            name: "dave".into(),
            ts: 99,
        });
        assert_eq!(q.len(), 1, "同一玩家同一类型只留一条");
        q.enqueue(PendingWrite::Pets {
            name: "dave".into(),
            pets: Vec::new(),
        });
        assert_eq!(q.len(), 2, "不同类型各自保留");
    }

    /// 门禁：队列满时丢**最旧**的一条并计数（不会无限涨内存）
    #[tokio::test]
    async fn caps_queue_and_drops_oldest() {
        let mut q = PendingPersists::default();
        for i in 0..(MAX_ITEMS + 5) {
            q.enqueue(item(&format!("p{i}")));
        }
        assert_eq!(q.len(), MAX_ITEMS);
        assert_eq!(q.dropped_total, 5, "超容量的部分应被丢弃并计数");
    }
}
