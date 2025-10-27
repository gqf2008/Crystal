use anyhow::{Context, Result};
use tokio::runtime::{Builder, Runtime};

use crate::graphics;
use crate::network as net;
use crate::settings::ClientSettings;
use crate::version; // Use network module as 'net'

pub struct ClientRuntime {
    pub settings: ClientSettings,
    // pub keybinds: KeyBindSettings,  // TODO: 需要实现
    pub tokio: Runtime,
}

impl ClientRuntime {
    /// 初始化日志系统
    pub fn init_logging(log_level: &str) {
        use std::fs::File;
        use tracing_subscriber::fmt::writer::MakeWriterExt;

        let file = File::create("game.log").expect("creating log file");
        let file_writer = file.with_max_level(tracing::Level::INFO);

        tracing_subscriber::fmt()
            .with_env_filter(log_level)
            .with_target(false)
            .with_writer(file_writer.and(std::io::stdout)) // 同时输出到文件和控制台
            .init();
    }

    /// 加载客户端配置
    pub fn load_config(use_test_config: bool) -> Result<ClientSettings> {
        let settings =
            ClientSettings::load(use_test_config, None).context("loading client settings")?;
        Ok(settings)
    }

    /// 创建 Tokio runtime
    pub fn create_tokio_runtime() -> Result<Runtime> {
        Builder::new_multi_thread()
            .enable_all()
            .thread_name("mir2-client")
            .build()
            .context("building tokio runtime")
    }

    /// 初始化图像库系统（包括 MapLibs）
    pub fn init_graphics_libraries(data_path: &str) -> Result<()> {
        println!(
            "🚀🚀🚀 [RUNTIME] 开始初始化图像库系统, data_path = {}",
            data_path
        );
        tracing::info!("=== 初始化图像库系统 ===");
        tracing::info!("📂 数据路径: {}", data_path);

        match graphics::initialize_all_libraries(data_path) {
            Ok(_) => {
                println!("✅✅✅ [RUNTIME] 图像库初始化成功!");
                tracing::info!("✅ 图像库初始化完成");
                Ok(())
            }
            Err(e) => {
                println!("❌❌❌ [RUNTIME] 图像库初始化失败: {}", e);
                tracing::error!("❌ 图像库初始化失败: {}", e);
                Err(anyhow::anyhow!("initializing graphics libraries: {}", e))
            }
        }
    }

    /// 加载核心图形库（Data.lib, Prguse.lib等）
    pub fn load_core_libraries() -> Result<()> {
        tracing::info!("📦 正在加载核心图形库...");
        graphics::libraries::load_core_libraries().context("loading core libraries")?;
        tracing::info!("✅ 核心图形库加载成功");
        Ok(())
    }

    /// 加载中文字体到 ggez Context
    ///
    /// # 参数
    /// - `ctx`: ggez Context
    /// - `font_path`: 字体文件路径（相对于项目根目录）
    /// - `font_name`: 字体名称（用于后续引用）
    ///
    /// # 回退策略
    /// 1. 尝试加载指定字体
    /// 2. 如果失败，尝试加载 assets/fonts/ 目录下的备用字体
    /// 3. 如果仍失败，尝试加载 Windows 系统字体
    pub fn load_font_to_context(
        ctx: &mut ggez::Context,
        font_path: &str,
        font_name: &str,
    ) -> Result<()> {
        use ggez::graphics::FontData;
        use std::path::Path;
        ctx.gfx.window().set_ime_allowed(true);
        // 1. 尝试加载指定字体
        if Path::new(font_path).exists() {
            match std::fs::read(font_path) {
                Ok(font_bytes) => {
                    ctx.gfx.add_font(font_name, FontData::from_vec(font_bytes)?);
                    tracing::info!("✓ 字体加载成功: {} ({})", font_name, font_path);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("⚠ 字体加载失败: {} - {}", font_path, e);
                }
            }
        } else {
            tracing::warn!("⚠ 字体文件不存在: {}", font_path);
        }

        // 2. 尝试备用字体
        let fallback_fonts = [
            (
                "assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf",
                "AlibabaPuHuiTi",
            ),
            ("assets/fonts/SourceHanSansCN-Regular.otf", "SourceHanSans"),
        ];

        for (path, name) in &fallback_fonts {
            if Path::new(path).exists() {
                if let Ok(bytes) = std::fs::read(path) {
                    if let Ok(font_data) = FontData::from_vec(bytes) {
                        ctx.gfx.add_font(font_name, font_data);
                        tracing::info!("✓ 使用备用字体: {} ({})", name, path);
                        return Ok(());
                    }
                }
            }
        }

        // 3. 尝试 Windows 系统字体
        #[cfg(target_os = "windows")]
        {
            let system_fonts = [
                ("C:/Windows/Fonts/msyh.ttc", "微软雅黑"),
                ("C:/Windows/Fonts/simsun.ttc", "宋体"),
            ];

            for (path, name) in &system_fonts {
                if Path::new(path).exists() {
                    if let Ok(bytes) = std::fs::read(path) {
                        if let Ok(font_data) = FontData::from_vec(bytes) {
                            ctx.gfx.add_font(font_name, font_data);
                            tracing::info!("✓ 使用系统字体: {}", name);
                            return Ok(());
                        }
                    }
                }
            }
        }

        anyhow::bail!("无法加载任何可用的中文字体")
    }

    // /// 完整的 bootstrap 流程（原有方法，用于传统启动）
    // pub fn bootstrap(use_test_config: bool) -> Result<()> {
    //     Self::init_logging("info");

    //     let settings = Self::load_config(use_test_config)?;
    //     let tokio = Self::create_tokio_runtime()?;
    //     Self::load_core_libraries()?;

    //     let runtime = Self { settings, tokio };

    //    runtime
    // }

    // /// 创建 ClientRuntime 实例（供 ggez 等新架构使用）
    // pub fn new(use_test_config: bool) -> Result<Self> {
    //     let settings = Self::load_config(use_test_config)?;
    //     let tokio = Self::create_tokio_runtime()?;

    //     Ok(Self { settings, tokio })
    // }

    // fn run(self) -> Result<()> {
    //     let Self { settings, tokio } = self;

    //     tokio.block_on(async move {
    //         // TODO: Initialize audio engine (not yet implemented)
    //         // let audio = audio::AudioEngine::new(&settings.sound).context("initializing audio")?;

    //         let mut net = net::NetworkStack::new(&settings.network);
    //         net.connect(&settings.network)
    //             .await
    //             .context("initializing network")?;

    //         let _version_hash = match version::client_binary_hash() {
    //             Ok(hash) => {
    //                 tracing::info!(
    //                     hash = %version::hash_to_hex(&hash),
    //                     "computed client version hash"
    //                 );
    //                 hash
    //             }
    //             Err(err) => {
    //                 tracing::warn!(
    //                     error = %err,
    //                     "failed to compute client version hash, falling back to empty hash"
    //                 );
    //                 Vec::new()
    //             }
    //         };

    //         // TODO: Launch UI (Forms-based windows)
    //         // let launch_result = crate::ui::launch(&settings)
    //         //     .await
    //         //     .context("running ui")?;

    //         // Save settings
    //         settings.save().context("saving settings")?;

    //         tracing::info!("Client completed");

    //         Ok(())
    //     })
    // }
}
