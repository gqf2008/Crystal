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

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

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
        format!(
            r#"{{"status":"ok","uptime_secs":{},"online_players":{},"packets_in":{},"packets_out":{}}}"#,
            uptime, players, pkts_in, pkts_out
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
            tracing::error!("Admin server bind {} failed: {} (health check disabled)", addr, e);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((mut stream, peer)) => {
                let json = stats.health_json();
                let response = format!(
                    "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}\r\n",
                    json.len() + 2,
                    json
                );
                let _ = stream.write_all(response.as_bytes()).await;
                tracing::debug!("Admin request from {}", peer);
            }
            Err(e) => {
                tracing::warn!("Admin accept error: {}", e);
            }
        }
    }
}

use tokio::io::AsyncWriteExt;
