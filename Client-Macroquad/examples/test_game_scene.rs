// ============================================================================
// 测试游戏场景 - 集成所有混合对话框
// ============================================================================
//
// 运行: cargo run --example test_game_scene
// 
// 功能测试：
// - I = 背包对话框
// - C = 角色对话框  
// - B = 快捷栏对话框
// - S = 商城对话框
// - 1-4 = 切换角色标签页
// - ESC = 关闭对话框 / 退出
// ============================================================================

use macroquad::prelude::*;

use client_macroquad::resources::libraries::{set_data_path, load_core_libraries};
use client_macroquad::scenes::dialogs::game::{
    InventoryDialogHybrid,
    CharacterDialogHybrid,
    BeltDialogHybrid,
    GameShopDialogHybrid,
    CharacterTabHybrid,
};
use client_macroquad::ui::text_renderer::{init_chinese_font, draw_text_cn};

fn window_conf() -> Conf {
    Conf {
        window_title: "游戏场景 - 混合对话框集成测试".to_string(),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("============================================");
    println!("       游戏场景 - 混合对话框测试");
    println!("============================================");
    println!("快捷键:");
    println!("  I        打开/关闭背包");
    println!("  C        打开/关闭角色");
    println!("  B        打开/关闭快捷栏");
    println!("  S        打开/关闭商城");
    println!("  1-4      切换角色标签页");
    println!("  ESC      关闭对话框 / 退出");
    println!("============================================\n");
    
    // 初始化
    set_data_path("Data");
    let _ = load_core_libraries();
    init_chinese_font().await;
    
    // 创建对话框
    let mut inventory_dialog = InventoryDialogHybrid::new();
    let mut character_dialog = CharacterDialogHybrid::new();
    let mut belt_dialog = BeltDialogHybrid::new();
    let mut shop_dialog = GameShopDialogHybrid::new();
    
    // 加载纹理
    println!("🎮 加载对话框纹理...");
    inventory_dialog.load_textures().await;
    character_dialog.load_textures().await;
    belt_dialog.load_textures().await;
    shop_dialog.load_textures().await;
    println!("✅ 纹理加载完成");
    
    // 设置初始位置
    let sw = screen_width();
    let sh = screen_height();
    inventory_dialog.set_position(vec2(sw - 280.0, 100.0));
    character_dialog.set_position(vec2(50.0, 100.0));
    belt_dialog.set_position(vec2((sw - 230.0) / 2.0, sh - 60.0));
    shop_dialog.set_position(vec2((sw - 720.0) / 2.0, 100.0));
    belt_dialog.open(); // 快捷栏默认打开
    
    loop {
        // 背景
        clear_background(Color::from_rgba(30, 45, 30, 255));
        
        // 绘制网格
        let grid_color = Color::from_rgba(50, 65, 50, 255);
        for i in 0..=((screen_width() / 48.0) as i32 + 1) {
            let x = i as f32 * 48.0;
            draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
        }
        for i in 0..=((screen_height() / 32.0) as i32 + 1) {
            let y = i as f32 * 32.0;
            draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
        }
        
        // 快捷键处理
        if is_key_pressed(KeyCode::I) {
            inventory_dialog.toggle();
            println!("📦 背包: {}", if inventory_dialog.is_visible() { "打开" } else { "关闭" });
        }
        if is_key_pressed(KeyCode::C) {
            character_dialog.toggle();
            println!("👤 角色: {}", if character_dialog.is_visible() { "打开" } else { "关闭" });
        }
        if is_key_pressed(KeyCode::B) {
            belt_dialog.toggle();
            println!("🎒 快捷栏: {}", if belt_dialog.is_visible() { "打开" } else { "关闭" });
        }
        if is_key_pressed(KeyCode::S) {
            shop_dialog.toggle();
            println!("🛒 商城: {}", if shop_dialog.is_visible() { "打开" } else { "关闭" });
        }
        
        // ESC 关闭对话框或退出
        if is_key_pressed(KeyCode::Escape) {
            let any_visible = inventory_dialog.is_visible() 
                || character_dialog.is_visible()
                || shop_dialog.is_visible();
            if any_visible {
                inventory_dialog.close();
                character_dialog.close();
                shop_dialog.close();
                println!("❌ 关闭所有对话框");
            } else {
                println!("👋 退出");
                break;
            }
        }
        
        // 1-4 切换角色标签页
        if character_dialog.is_visible() {
            if is_key_pressed(KeyCode::Key1) {
                character_dialog.switch_tab(CharacterTabHybrid::Character);
            }
            if is_key_pressed(KeyCode::Key2) {
                character_dialog.switch_tab(CharacterTabHybrid::Status);
            }
            if is_key_pressed(KeyCode::Key3) {
                character_dialog.switch_tab(CharacterTabHybrid::State);
            }
            if is_key_pressed(KeyCode::Key4) {
                character_dialog.switch_tab(CharacterTabHybrid::Skills);
            }
        }
        
        // 绘制对话框（商城在最上层）
        inventory_dialog.update_and_draw();
        character_dialog.update_and_draw();
        belt_dialog.update_and_draw();
        shop_dialog.update_and_draw();
        
        // 帮助文字
        draw_text_cn(
            "快捷键: I=背包 C=角色 B=快捷栏 S=商城 | 1-4=角色标签 | ESC=关闭/退出",
            10.0, screen_height() - 25.0, 14.0, Color::from_rgba(200, 200, 200, 180)
        );
        
        // 状态指示器
        let mut status_parts = vec![];
        if inventory_dialog.is_visible() { status_parts.push("背包"); }
        if character_dialog.is_visible() { status_parts.push("角色"); }
        if belt_dialog.is_visible() { status_parts.push("快捷栏"); }
        if shop_dialog.is_visible() { status_parts.push("商城"); }
        
        let status = if status_parts.is_empty() {
            "当前打开: 无".to_string()
        } else {
            format!("当前打开: {}", status_parts.join(", "))
        };
        draw_text_cn(&status, 10.0, 20.0, 16.0, WHITE);
        
        next_frame().await;
    }
}
