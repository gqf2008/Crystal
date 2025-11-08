// ============================================================================
// Macroquad 精灵系统演示
// ============================================================================
//
// 展示使用 SpriteManager 加载和渲染精灵
//
// 用法：
//   cargo run --bin demo_sprite_macroquad --no-default-features --features backend-macroquad
//
// ============================================================================

use macroquad::prelude::*;

// 使用 mir2_client 的资源和渲染后端
use mir2_client::backends::macroquad::SpriteManager;

/// macroquad 窗口配置
fn window_conf() -> Conf {
    Conf {
        window_title: "Macroquad 精灵系统演示 - Crystal".to_owned(),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🚀 Macroquad 精灵系统演示启动");

    // 创建精灵管理器
    let mut sprite_mgr = SpriteManager::new();
    sprite_mgr.set_max_cache_size(500);

    // 加载库文件
    let lib_path = "Data/ChrSel.lib";
    match sprite_mgr.load_library("ChrSel", lib_path) {
        Ok(_) => println!("✅ 成功加载库: {}", lib_path),
        Err(e) => {
            println!("❌ 无法加载库: {}", e);
            println!("   请确保 Data/ChrSel.lib 文件存在");
        }
    }

    let mut current_index = 0usize;
    let mut scale = 1.0f32;
    let mut rotation_angle = 0.0f32;
    let mut use_offset = true;
    let mut auto_rotate = false;

    println!("✅ 初始化完成，进入主循环");
    println!("\n📖 操作说明:");
    println!("   ← →     : 切换图像");
    println!("   ↑ ↓     : 缩放");
    println!("   R       : 开关自动旋转");
    println!("   O       : 开关偏移量");
    println!("   空格     : 清除缓存");
    println!("   ESC     : 退出\n");

    loop {
        let dt = get_frame_time();

        // ========== 输入处理 ==========
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        if is_key_pressed(KeyCode::Right) {
            current_index += 1;
            println!("➡️  图像索引: {}", current_index);
        }

        if is_key_pressed(KeyCode::Left) && current_index > 0 {
            current_index -= 1;
            println!("⬅️  图像索引: {}", current_index);
        }

        if is_key_pressed(KeyCode::Up) {
            scale = (scale + 0.1).min(5.0);
            println!("🔼 缩放: {:.1}x", scale);
        }

        if is_key_pressed(KeyCode::Down) {
            scale = (scale - 0.1).max(0.1);
            println!("🔽 缩放: {:.1}x", scale);
        }

        if is_key_pressed(KeyCode::R) {
            auto_rotate = !auto_rotate;
            println!("🔄 自动旋转: {}", if auto_rotate { "开" } else { "关" });
        }

        if is_key_pressed(KeyCode::O) {
            use_offset = !use_offset;
            println!("📍 使用偏移: {}", if use_offset { "是" } else { "否" });
        }

        if is_key_pressed(KeyCode::Space) {
            sprite_mgr.clear_all_cache();
            println!("🗑️  缓存已清除");
        }

        if auto_rotate {
            rotation_angle += dt;
        }

        // ========== 渲染 ==========
        clear_background(Color::from_rgba(40, 44, 52, 255));

        let screen_w = screen_width();
        let screen_h = screen_height();

        // 背景网格
        for x in (0..screen_w as i32).step_by(32) {
            draw_line(
                x as f32,
                0.0,
                x as f32,
                screen_h,
                1.0,
                Color::from_rgba(50, 54, 62, 255),
            );
        }
        for y in (0..screen_h as i32).step_by(32) {
            draw_line(
                0.0,
                y as f32,
                screen_w,
                y as f32,
                1.0,
                Color::from_rgba(50, 54, 62, 255),
            );
        }

        // 绘制精灵
        sprite_mgr.draw_sprite_ex(
            "ChrSel",
            current_index,
            screen_w * 0.5,
            screen_h * 0.5,
            scale,
            rotation_angle,
            WHITE,
            use_offset,
        );

        // UI 信息
        let stats = sprite_mgr.cache_stats();
        let info_text = format!(
            "图像: #{} | 缩放: {:.1}x | 旋转: {:.0}° | 偏移: {} | 缓存: {}/{} | FPS: {}",
            current_index,
            scale,
            rotation_angle.to_degrees(),
            if use_offset { "开" } else { "关" },
            stats.sprite_count,
            stats.max_cache_size,
            get_fps()
        );

        draw_rectangle(0.0, 0.0, screen_w, 40.0, Color::from_rgba(0, 0, 0, 180));
        draw_text(&info_text, 10.0, 25.0, 20.0, WHITE);

        // 帮助信息
        let help_text =
            "← → : 切换 | ↑ ↓ : 缩放 | R : 旋转 | O : 偏移 | 空格 : 清缓存 | ESC : 退出";
        draw_rectangle(
            0.0,
            screen_h - 30.0,
            screen_w,
            30.0,
            Color::from_rgba(0, 0, 0, 180),
        );
        draw_text(
            help_text,
            10.0,
            screen_h - 10.0,
            16.0,
            Color::from_rgba(200, 200, 200, 255),
        );

        next_frame().await;
    }

    println!("👋 演示结束");
    println!("📊 最终缓存统计:");
    let stats = sprite_mgr.cache_stats();
    println!("   - 精灵缓存: {}", stats.sprite_count);
    println!("   - 库数量: {}", stats.library_count);
}
