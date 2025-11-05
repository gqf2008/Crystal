use anyhow::{Context, Result};

use crate::graphics;
use crate::settings::ClientSettings;

pub struct ClientRuntime {
    pub settings: ClientSettings,
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

   
}
