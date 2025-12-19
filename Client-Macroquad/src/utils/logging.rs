use std::sync::Once;

static INIT: Once = Once::new();

/// 初始化 tracing 日志（幂等）。
///
/// - 默认日志级别：info
/// - 可通过环境变量 `RUST_LOG` 覆盖（例如 `RUST_LOG=client_macroquad=debug`）
pub fn init_tracing() {
    INIT.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            // macroquad 下通常不需要 target/module path；想要时可改成 true
            .with_target(false)
            .compact()
            .init();
    });
}
