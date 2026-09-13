//! #2606：kameo actor 之外的裸后台任务统一登记 + 优雅关闭 + 指标。
//!
//! ServerRust 的后台任务分两类：
//! 1. kameo actor（`Actor::spawn`）——生命周期由 kameo 托管，不在本模块范围；
//! 2. kameo 之外的裸任务（监听循环、tick 循环、每连接读循环、fire-and-forget 发包/写库）——
//!    此前直接 `tokio::spawn`：没有 owner、没有登记、没有指标，`ShutdownAll` 之后也无法确认
//!    它们是否结束（C# 服务端所有后台线程都由 `ServerTimer`/主循环显式 join，同语义）。
//!
//! 本模块提供进程级注册表 [`TaskRegistry`]（全局单例 [`registry`]）：
//! - [`TaskRegistry::spawn`]：登记任务并记账（`spawned_total` / `running` / `finished_total` /
//!   `panics_total`；panic 被 join 句柄捕获，不掀翻进程，但会计数 + error 日志）；
//! - [`TaskRegistry::shutdown`]：广播关闭信号后逐个 `await`，确认登记任务全部收敛；
//! - [`TaskRegistry::shutdown_signal`]：长驻循环（accept / tick / 会话读循环）`select!` 它退出；
//! - [`TaskMetrics::to_prometheus`]：指标文本，admin 端点 `GET /metrics` 暴露。
//!
//! 纪律：**kameo 之外一律不再写裸 `tokio::spawn`**，统一走本模块
//! （`tests::no_bare_tokio_spawn_outside_registry` 扫描源码兜底）。

use std::future::Future;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};

use tokio::sync::watch;
use tokio::task::JoinHandle;

/// 后台任务指标快照。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskMetrics {
    /// 累计登记（spawn）的任务数
    pub spawned_total: u64,
    /// 累计结束的任务数（正常返回 / panic / 被中止）
    pub finished_total: u64,
    /// 当前存活任务数
    pub running: i64,
    /// 累计 panic 的任务数
    pub panics_total: u64,
}

impl TaskMetrics {
    /// Prometheus 文本格式（admin `GET /metrics`）。
    pub fn to_prometheus(&self) -> String {
        format!(
            "# TYPE crystal_tasks_spawned_total counter\n\
             crystal_tasks_spawned_total {}\n\
             # TYPE crystal_tasks_finished_total counter\n\
             crystal_tasks_finished_total {}\n\
             # TYPE crystal_tasks_running gauge\n\
             crystal_tasks_running {}\n\
             # TYPE crystal_tasks_panics_total counter\n\
             crystal_tasks_panics_total {}\n",
            self.spawned_total, self.finished_total, self.running, self.panics_total
        )
    }

    /// 单行摘要（日志用）。
    pub fn summary(&self) -> String {
        format!(
            "spawned={} finished={} running={} panics={}",
            self.spawned_total, self.finished_total, self.running, self.panics_total
        )
    }
}

/// 关闭信号句柄：长驻循环 `select!` 它，收到信号即退出。
#[derive(Clone)]
pub struct ShutdownSignal {
    rx: watch::Receiver<bool>,
}

impl ShutdownSignal {
    /// 等待关闭信号（信号已发出则立即返回；可重复 await）。
    ///
    /// 注册表存活期间 sender 不会 drop，`changed()` 正常不会 Err；仍按 Err 收敛以避免
    /// 将来误用（例如持有信号却先 drop 注册表）导致永久挂起。
    pub async fn cancelled(&self) {
        let mut rx = self.rx.clone();
        if *rx.borrow_and_update() {
            return;
        }
        while rx.changed().await.is_ok() {
            if *rx.borrow_and_update() {
                return;
            }
        }
    }

    /// 信号是否已发出（非阻塞）。
    pub fn is_shutting_down(&self) -> bool {
        *self.rx.borrow()
    }
}

/// 进程级后台任务注册表（clone 廉价，共享同一份账本）。
#[derive(Clone)]
pub struct TaskRegistry {
    inner: Arc<Inner>,
}

struct Inner {
    /// 存活任务的 join 句柄（每次 spawn / shutdown 时回收已结束的句柄）
    live: StdMutex<Vec<LiveTask>>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    spawned_total: AtomicU64,
    finished_total: AtomicU64,
    panics_total: AtomicU64,
    running: AtomicI64,
}

struct LiveTask {
    name: &'static str,
    handle: JoinHandle<()>,
}

/// 任务账本守卫：任务结束（含 panic 展开、被 abort）时结算。
struct TaskGuard {
    inner: Arc<Inner>,
    name: &'static str,
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.inner.running.fetch_sub(1, Ordering::Relaxed);
        self.inner.finished_total.fetch_add(1, Ordering::Relaxed);
        if std::thread::panicking() {
            self.inner.panics_total.fetch_add(1, Ordering::Relaxed);
            tracing::error!("background task `{}` panicked", self.name);
        }
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskRegistry {
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            inner: Arc::new(Inner {
                live: StdMutex::new(Vec::new()),
                shutdown_tx,
                shutdown_rx,
                spawned_total: AtomicU64::new(0),
                finished_total: AtomicU64::new(0),
                panics_total: AtomicU64::new(0),
                running: AtomicI64::new(0),
            }),
        }
    }

    /// 登记并启动一个后台任务。`name` 用于 panic / 关闭日志定位。
    pub fn spawn<F>(&self, name: &'static str, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let inner = Arc::clone(&self.inner);
        inner.spawned_total.fetch_add(1, Ordering::Relaxed);
        inner.running.fetch_add(1, Ordering::Relaxed);
        let guard = TaskGuard {
            inner: Arc::clone(&inner),
            name,
        };
        let handle = tokio::spawn(async move {
            let _guard = guard;
            fut.await;
        });
        let mut live = self.lock_live();
        // 顺手回收已结束任务的句柄，避免长跑进程里句柄无限增长
        live.retain(|t| !t.handle.is_finished());
        live.push(LiveTask { name, handle });
    }

    /// 广播关闭信号，然后等待所有**已登记**任务结束。
    ///
    /// 返回关闭后的指标快照（`running` 归零即"无残留任务"）。调用方应自行加超时，
    /// 以免个别不响应信号的任务拖死进程。
    pub async fn shutdown(&self) -> TaskMetrics {
        self.inner.shutdown_tx.send_replace(true);
        let pending: Vec<LiveTask> = {
            let mut live = self.lock_live();
            std::mem::take(&mut *live)
        };
        let mut joined = 0u64;
        let mut not_joined = 0u64;
        for task in pending {
            match task.handle.await {
                Ok(()) => joined += 1,
                Err(e) if e.is_panic() => {
                    // panic 已在 TaskGuard::drop 计数，这里只补一条带任务名的日志
                    tracing::error!("background task `{}` ended with panic: {}", task.name, e);
                    joined += 1;
                }
                Err(e) => {
                    tracing::warn!("background task `{}` not joined: {}", task.name, e);
                    not_joined += 1;
                }
            }
        }
        let metrics = self.metrics();
        tracing::info!(
            "task registry shutdown: joined={} not_joined={} ({})",
            joined,
            not_joined,
            metrics.summary()
        );
        metrics
    }

    /// 指标快照。
    pub fn metrics(&self) -> TaskMetrics {
        TaskMetrics {
            spawned_total: self.inner.spawned_total.load(Ordering::Relaxed),
            finished_total: self.inner.finished_total.load(Ordering::Relaxed),
            running: self.inner.running.load(Ordering::Relaxed),
            panics_total: self.inner.panics_total.load(Ordering::Relaxed),
        }
    }

    /// 取关闭信号（长驻循环用）。
    pub fn shutdown_signal(&self) -> ShutdownSignal {
        ShutdownSignal {
            rx: self.inner.shutdown_rx.clone(),
        }
    }

    /// 是否已进入关闭流程。
    pub fn is_shutting_down(&self) -> bool {
        *self.inner.shutdown_rx.borrow()
    }

    fn lock_live(&self) -> std::sync::MutexGuard<'_, Vec<LiveTask>> {
        // 账本被 panic 毒化时继续用（换的是数量统计，不是用户数据）
        self.inner
            .live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

static GLOBAL: OnceLock<TaskRegistry> = OnceLock::new();

/// 进程级注册表（生产代码统一用它；单测用 [`TaskRegistry::new`] 造独立账本）。
pub fn registry() -> &'static TaskRegistry {
    GLOBAL.get_or_init(TaskRegistry::new)
}

/// 登记一个裸后台任务（`name` 见 [`TaskRegistry::spawn`]）。
pub fn spawn<F>(name: &'static str, fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    registry().spawn(name, fut);
}

/// 取进程级关闭信号。
pub fn shutdown_signal() -> ShutdownSignal {
    registry().shutdown_signal()
}

/// 关闭并等待全部登记任务（调用方自行加超时）。
pub async fn shutdown() -> TaskMetrics {
    registry().shutdown().await
}

/// 指标快照。
pub fn metrics() -> TaskMetrics {
    registry().metrics()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[tokio::test]
    async fn shutdown_waits_for_all_registered_tasks() {
        let reg = TaskRegistry::new();
        let started = Arc::new(AtomicUsize::new(0));
        for _ in 0..3 {
            let signal = reg.shutdown_signal();
            let started = Arc::clone(&started);
            reg.spawn("test.waiter", async move {
                started.fetch_add(1, Ordering::SeqCst);
                signal.cancelled().await;
            });
        }
        // 等 3 个任务都进入等待（否则 shutdown 可能先于 spawn 的 poll）
        for _ in 0..200 {
            if started.load(Ordering::SeqCst) == 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(started.load(Ordering::SeqCst), 3, "3 个任务都应被 poll 到");

        let metrics = tokio::time::timeout(Duration::from_secs(5), reg.shutdown())
            .await
            .expect("shutdown 应在超时内完成（信号能唤醒等待中的任务）");
        assert_eq!(metrics.spawned_total, 3);
        assert_eq!(metrics.finished_total, 3);
        assert_eq!(metrics.running, 0, "关闭后不应有残留任务");
        assert_eq!(metrics.panics_total, 0);
        assert_eq!(reg.metrics().running, 0);
        assert!(reg.is_shutting_down());
    }

    #[tokio::test]
    async fn shutdown_waits_for_short_lived_task_without_signal() {
        let reg = TaskRegistry::new();
        reg.spawn("test.short", async {
            tokio::time::sleep(Duration::from_millis(30)).await;
        });
        let metrics = tokio::time::timeout(Duration::from_secs(5), reg.shutdown())
            .await
            .expect("无信号依赖的短任务也应被 await 到结束");
        assert_eq!(metrics.spawned_total, 1);
        assert_eq!(metrics.finished_total, 1);
        assert_eq!(metrics.running, 0);
    }

    #[tokio::test]
    async fn panicking_task_is_counted_and_does_not_kill_process() {
        let reg = TaskRegistry::new();
        reg.spawn("test.panic", async {
            panic!("boom");
        });
        reg.spawn("test.ok", async {});
        let metrics = tokio::time::timeout(Duration::from_secs(5), reg.shutdown())
            .await
            .expect("panic 任务不应阻塞 shutdown");
        assert_eq!(metrics.spawned_total, 2);
        assert_eq!(metrics.finished_total, 2);
        assert_eq!(metrics.panics_total, 1, "panic 任务应计入 panics_total");
        assert_eq!(metrics.running, 0);
    }

    #[test]
    fn prometheus_text_exports_all_series() {
        let metrics = TaskMetrics {
            spawned_total: 7,
            finished_total: 5,
            running: 2,
            panics_total: 1,
        };
        let text = metrics.to_prometheus();
        for expect in [
            "crystal_tasks_spawned_total 7",
            "crystal_tasks_finished_total 5",
            "crystal_tasks_running 2",
            "crystal_tasks_panics_total 1",
        ] {
            assert!(text.contains(expect), "缺少指标行 {expect}：\n{text}");
        }
        assert_eq!(metrics.summary(), "spawned=7 finished=5 running=2 panics=1");
    }

    /// #2606 兜底：kameo 之外的裸 `tokio::spawn` 必须走本模块（源码扫描）。
    #[test]
    fn no_bare_tokio_spawn_outside_registry() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        scan_dir(&src, &mut offenders);
        assert!(
            offenders.is_empty(),
            "发现未登记的裸 tokio::spawn（请改用 crate::util::tasks::spawn）：{offenders:#?}"
        );
    }

    fn scan_dir(dir: &std::path::Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("读取源码目录") {
            let path = entry.expect("目录项").path();
            if path.is_dir() {
                scan_dir(&path, offenders);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) == Some("tasks.rs") {
                continue;
            }
            let content = std::fs::read_to_string(&path).expect("读取源码文件");
            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                if line.contains("tokio::spawn(") || line.contains("tokio::task::spawn(") {
                    offenders.push(format!("{}:{}: {}", path.display(), idx + 1, trimmed));
                }
            }
        }
    }
}
