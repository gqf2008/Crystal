// Crystal Server - Legend of Mir 2 game server
// 启动入口：初始化 actors → 启动 TCP 监听 → 进入事件循环

use std::env;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use kameo::actor::Spawn;
use tracing::{info, error, warn};

use crystal_server::actors::account::AccountActor;
use crystal_server::actors::world::{WorldActor, WorldActorArgs};
use crystal_server::actors::social::{SocialActor, SocialActorArgs, SocialActorConfig};
use crystal_server::gate::actor::{GateActor, SetAccountRef, SetWorldRef, SetMaxConnections, ShutdownAll};
use crystal_server::util::config;
use crystal_server::db;

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()?;
    rt.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("crystal_server=info".parse()?)
                .add_directive("tokio=warn".parse()?)
                .add_directive("kameo=warn".parse()?),
        )
        .init();

    info!("Crystal Server starting...");

    // 加载配置
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "config/server.toml".to_string());

    let cfg = match config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            info!("Config not found ({}), using defaults", e);
            config::ServerConfig::default()
        }
    };

    info!("Config loaded: listen={}", cfg.network.listen_addr);

    // 启动 Actors
    info!("Spawning actors...");

    // GateActor 先启动
    let gate_ref = GateActor::spawn(());
    info!("GateActor spawned");

    // Phase 1.1: 把 cfg.network.max_connections 传给 GateActor(防止资源耗尽)
    let _ = gate_ref.ask(SetMaxConnections(cfg.network.max_connections)).await;
    info!("Configured max_connections={}", cfg.network.max_connections);

    let map_dir = PathBuf::from(&cfg.server.map_data_dir);
    let spawn_dir = PathBuf::from("Data/spawn");

    // 初始化 SQLite 数据库
    let db_path = PathBuf::from("data/crystal.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let db_pool = match db::init_db(&db_path).await {
        Ok(pool) => {
            info!("SQLite database initialized at {}", db_path.display());
            pool
        }
        Err(e) => {
            warn!("Failed to initialize SQLite DB, using in-memory only: {}", e);
            // Fallback: still start server but without persistence
            db::init_db(&PathBuf::from(":memory:")).await.expect("in-memory DB should always work")
        }
    };

    // WorldActor 启动，携带 GateActor 引用 + 地图目录 + 刷怪目录 + 数据库
    // SocialActor 先启动，WorldActor 依赖它。
    // 共享 cfg:social 字段(guild_creation_cost_gold)透传到 SocialActorConfig
    // 的 GUILD_CREATION_COST_GOLD 常量(之前是 hardcoded 1_000_000,现从 cfg 读)。
    let social_config = SocialActorConfig {
        map_infos: Arc::new(RwLock::new(HashMap::<i32, db::MapInfo>::new())),
        item_infos: Arc::new(RwLock::new(HashMap::<i32, db::ItemInfo>::new())),
        guild_creation_cost_gold: cfg.social.guild_creation_cost_gold,
    };
    info!("Social config: guild_creation_cost_gold = {}", social_config.guild_creation_cost_gold);
    let social_ref = SocialActor::spawn(SocialActorArgs {
        gate_ref: gate_ref.clone(),
        db_pool: db_pool.clone(),
        config: social_config,
    });
    info!("SocialActor spawned");

    let quest_dir = PathBuf::from("Daneo1989/Envir/Quests");
    let world_ref = WorldActor::spawn(WorldActorArgs {
        tick_interval_ms: cfg.server.tick_ms,
        gate_ref: gate_ref.clone(),
        map_dir,
        spawn_dir: Some(spawn_dir),
        quest_dir,
        db_pool: db_pool.clone(),
        social_ref: social_ref.clone(),
    });
    info!("WorldActor spawned (tick={}ms, map_dir={})", cfg.server.tick_ms, cfg.server.map_data_dir);

    // AccountActor 需要 GateActor 的引用和数据库
    let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
    info!("AccountActor spawned");

    // 双向链接：GateActor 需要 AccountActor 和 WorldActor 的引用
    let _ = gate_ref.ask(SetAccountRef {
        account_ref: account_ref.clone(),
    }).await;
    let _ = gate_ref.ask(SetWorldRef {
        world_ref: world_ref.clone(),
    }).await;

    // 启动 TCP 监听
    info!("Starting gate listener on {}...", cfg.network.listen_addr);

    let gate_addr = cfg.network.listen_addr.clone();
    let gate_ref_for_listener = gate_ref.clone();
    tokio::spawn(async move {
        if let Err(e) = crystal_server::gate::actor::run_gate_listener(gate_addr, gate_ref_for_listener).await {
            error!("Gate listener error: {}", e);
        }
    });

    info!("Server is ready! Press Ctrl+C to stop.");

    // 保持运行
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, initiating graceful shutdown...");

    // Phase 2.2: 优雅关机 — 断开所有 session 触发自动保存
    if let Ok(count) = gate_ref.ask(ShutdownAll).await {
        info!("Disconnect packets sent to {} sessions, waiting 5s for saves...", count);
    }
    // 给 actor 5 秒处理断连 + 保存
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    info!("Graceful shutdown complete. Goodbye.");

    Ok(())
}
