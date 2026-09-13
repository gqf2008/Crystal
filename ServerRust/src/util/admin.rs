//! Phase 3.1: 轻量运维端点 — health check + 基础 metrics。
//!
//! 用纯 tokio TcpListener 实现,不依赖 axum(减镜像体积)。
//! 每次连接返回一行 JSON 状态然后关闭连接。
//!
//! 用法(curl):
//!   curl http://localhost:7001/        → health check
//!   curl http://localhost:7001/metrics → prometheus-style metrics
//!
//! 或用 TCP 纯文本:
//!   nc localhost 7001 → JSON 状态行

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 运维统计(线程安全,由各 Actor 通过原子操作更新)。
#[derive(Debug)]
pub struct AdminStats {
    pub start_time: Instant,
    pub online_players: AtomicU64,
    pub total_packets_in: AtomicU64,
    pub total_packets_out: AtomicU64,
}

impl Default for AdminStats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            online_players: AtomicU64::new(0),
            total_packets_in: AtomicU64::new(0),
            total_packets_out: AtomicU64::new(0),
        }
    }
}

impl AdminStats {
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 返回 JSON 状态字符串。
    pub fn health_json(&self) -> String {
        let players = self.online_players.load(Ordering::Relaxed);
        let uptime = self.uptime_secs();
        let pkts_in = self.total_packets_in.load(Ordering::Relaxed);
        let pkts_out = self.total_packets_out.load(Ordering::Relaxed);
        // #2606：后台任务指标同源暴露（registries 的唯一账本 → 不会和 /metrics 打架）
        let tasks = crate::util::tasks::metrics();
        format!(
            r#"{{"status":"ok","uptime_secs":{},"online_players":{},"packets_in":{},"packets_out":{},"tasks":{{"spawned_total":{},"finished_total":{},"running":{},"panics_total":{}}}}}"#,
            uptime,
            players,
            pkts_in,
            pkts_out,
            tasks.spawned_total,
            tasks.finished_total,
            tasks.running,
            tasks.panics_total
        )
    }
}

/// 启动 admin TCP 服务器。
///
/// 每次连接返回 JSON 状态然后关闭。不解析 HTTP 头(简化),
/// 但响应包含 HTTP/1.0 头,方便 curl 直接访问。
pub async fn run_admin_server(stats: Arc<AdminStats>, addr: String) {
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => {
            tracing::info!("Admin health check listening on {}", addr);
            l
        }
        Err(e) => {
            tracing::error!(
                "Admin server bind {} failed: {} (health check disabled)",
                addr,
                e
            );
            return;
        }
    };

    // #2606：随进程优雅关闭退出（否则 ShutdownAll 之后它仍是一个活着的登记任务）
    serve_admin(listener, stats, crate::util::tasks::shutdown_signal()).await;
}

/// admin 服务循环（独立出来便于单测用随机端口驱动）。
pub async fn serve_admin(
    listener: tokio::net::TcpListener,
    stats: Arc<AdminStats>,
    shutdown: crate::util::tasks::ShutdownSignal,
) {
    loop {
        let (mut stream, peer) = tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("Admin server shutting down");
                return;
            }
            accepted = listener.accept() => match accepted {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Admin accept error: {}", e);
                    continue;
                }
            },
        };

        // 读请求行判路径（200ms 超时：`nc` 裸连时可能一个字节都不发）
        let mut buf = [0u8; 256];
        let mut want_metrics = false;
        if let Ok(Ok(n)) =
            tokio::time::timeout(Duration::from_millis(200), stream.read(&mut buf)).await
        {
            if n > 0 {
                let request = String::from_utf8_lossy(&buf[..n]);
                want_metrics = request
                    .split_whitespace()
                    .nth(1)
                    .map(|path| path.starts_with("/metrics"))
                    .unwrap_or(false);
            }
        }

        let (content_type, body) = if want_metrics {
            (
                "text/plain; version=0.0.4",
                crate::util::tasks::metrics().to_prometheus(),
            )
        } else {
            ("application/json", format!("{}\n", stats.health_json()))
        };
        let response = format!(
            "HTTP/1.0 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
            content_type,
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes()).await;
        tracing::debug!("Admin request from {}", peer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, TcpStream};

    /// #2606：指标可见（/metrics prometheus 文本 + health JSON tasks 块）+ 关闭信号能收掉服务任务。
    #[tokio::test]
    async fn metrics_and_health_expose_tasks_and_shutdown_stops_server() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("绑定随机端口");
        let addr = listener.local_addr().expect("取端口");
        let stats = Arc::new(AdminStats::default());
        let reg = crate::util::tasks::TaskRegistry::new();
        reg.spawn(
            "admin.server.test",
            serve_admin(listener, stats, reg.shutdown_signal()),
        );

        let metrics_body = admin_get(addr, "/metrics").await;
        assert!(metrics_body.contains("200 OK"), "{metrics_body}");
        for series in [
            "crystal_tasks_spawned_total",
            "crystal_tasks_finished_total",
            "crystal_tasks_running",
            "crystal_tasks_panics_total",
        ] {
            assert!(
                metrics_body.contains(series),
                "指标端点缺少 {series}：{metrics_body}"
            );
        }

        let health_body = admin_get(addr, "/").await;
        assert!(
            health_body.contains(r#""tasks":{"spawned_total":"#),
            "health JSON 应含 tasks 块：{health_body}"
        );

        let metrics = tokio::time::timeout(Duration::from_secs(5), reg.shutdown())
            .await
            .expect("关闭信号应让 admin 服务任务退出");
        assert_eq!(metrics.running, 0, "关闭后不应有残留任务");
        assert_eq!(metrics.finished_total, 1);
    }

    async fn admin_get(addr: std::net::SocketAddr, path: &str) -> String {
        let mut stream = TcpStream::connect(addr).await.expect("连接 admin 端口");
        stream
            .write_all(format!("GET {path} HTTP/1.0\r\n\r\n").as_bytes())
            .await
            .expect("写请求");
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.expect("读响应");
        String::from_utf8_lossy(&buf).into_owned()
    }
}
