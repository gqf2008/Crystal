// 测试 CharacterDialog（角色对话框）

use client_macroquad::scenes::dialogs::game::CharacterDialog;
use client_macroquad::ui::text_renderer::*;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - 角色对话框测试".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - 角色对话框测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 初始化中文字体
    init_chinese_font().await;
    
    // 配置 egui 中文字体
    egui_macroquad::cfg(|ctx| {
        setup_egui_chinese_font(ctx);
    });
    
    // 创建角色对话框
    let mut character = CharacterDialog::new();
    
    println!("\n⚠️  装备渲染测试：");
    println!("   如果装备系统工作正常,你应该能在角色脚底看到:");
    println!("   1. 一个半透明的红色矩形 (30x35像素)");
    println!("   2. 矩形上面叠加了装备外观图标");
    println!("   位置大约在角色对话框中裸体人物的脚底附近\n");
    
    // 默认显示角色对话框
    let mut character_open = true;
    
    println!("✅ 角色对话框已创建并显示");
    println!("\n💡 测试功能清单:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 基础功能:");
    println!("   ✓ 拖拽标题栏可移动窗口");
    println!("   ✓ 点击关闭按钮或按ESC关闭对话框");
    println!("   ✓ 标签页切换（角色/技能/状态）");
    println!("\n⚔️ 角色装备页:");
    println!("   ✓ 14个装备栏位显示");
    println!("   ✓ 装备图标显示（武器/衣服/头盔等）");
    println!("   ✓ 装备耐久度显示（彩色进度条）");
    println!("   ✓ 角色属性面板（等级/经验/HP/MP等）");
    println!("   ✓ 点击装备栏位交互");
    println!("\n🔮 技能页:");
    println!("   ✓ 技能图标网格显示（3x4布局）");
    println!("   ✓ 技能等级显示");
    println!("   ✓ 技能经验进度显示");
    println!("   ✓ 鼠标悬停显示技能详情");
    println!("   ✓ 点击技能交互");
    println!("\n📊 状态页:");
    println!("   ✓ 详细属性信息显示");
    println!("   ✓ 经验值和等级进度");
    println!("   ✓ 属性点分配信息");
    println!("\n📊 测试数据:");
    println!("   • 角色等级：60（满级）");
    println!("   • 经验值：980,000 / 1,000,000");
    println!("   • 生命值：850 / 850");
    println!("   • 魔法值：450 / 450");
    println!("   • 装备数量：14件（满配）");
    println!("   • 技能数量：7个（全满级）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // FPS 统计
    let mut frame_times: Vec<f32> = Vec::with_capacity(60);
    let mut last_time = get_time();

    loop {
        // 绘制背景
        clear_background(Color::from_rgba(40, 50, 60, 255));

        // 绘制测试说明
        let hints = [
            "👤 角色对话框测试中...",
            "点击标签页切换 | 查看装备/技能/状态",
            "拖拽窗口移动 | 按ESC关闭",
        ];
        
        for (i, hint) in hints.iter().enumerate() {
            draw_text_centered(
                hint,
                screen_width() / 2.0,
                screen_height() / 2.0 - 60.0 + (i as f32 * 30.0),
                24.0,
                Color::from_rgba(200, 200, 200, 255),
            );
        }

        // egui UI - 合并到一次调用中
        let egui_start = get_time();
        egui_macroquad::ui(|ctx| {
            // 先绘制角色对话框
            use client_macroquad::scenes::dialogs::Dialog;
            character.show(ctx, &mut character_open);
            
            // 再绘制测试控制面板
            egui_macroquad::egui::Window::new("🧪 测试控制面板")
                .default_pos(egui_macroquad::egui::pos2(10.0, 50.0))
                .default_size(egui_macroquad::egui::vec2(300.0, 450.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("角色状态");
                    ui.separator();
                    
                    ui.label(format!("可见状态: {}", if character_open { "✅ 显示中" } else { "❌ 已隐藏" }));
                    ui.label(format!("当前标签页: {:?}", character.active_tab));
                    ui.label(format!("角色等级: {}", character.character_stats.level));
                    ui.label(format!("经验值: {} / {}", 
                        character.character_stats.experience, 
                        character.character_stats.next_exp
                    ));
                    ui.label(format!("生命值: {} / {}", 
                        character.character_stats.health.0, 
                        character.character_stats.health.1
                    ));
                    ui.label(format!("魔法值: {} / {}", 
                        character.character_stats.mana.0, 
                        character.character_stats.mana.1
                    ));
                    ui.label(format!("攻击力: {} - {}", 
                        character.character_stats.dc.0, 
                        character.character_stats.dc.1
                    ));
                    ui.label(format!("防御力: {} - {}", 
                        character.character_stats.ac.0, 
                        character.character_stats.ac.1
                    ));
                    
                    ui.separator();
                    ui.heading("快捷操作");
                    
                    if ui.button("👤 切换对话框显示").clicked() {
                        character_open = !character_open;
                    }
                    
                    if ui.button("⚔️ 切换到角色页").clicked() {
                        character.show_character_page();
                        println!("⚔️ 切换到角色装备页");
                    }
                    
                    if ui.button("📊 切换到状态页I").clicked() {
                        character.show_status_page();
                        println!("📊 切换到状态页I");
                    }
                    
                    if ui.button("📊 切换到状态页II").clicked() {
                        character.show_state_page();
                        println!("📊 切换到状态页II");
                    }
                    
                    if ui.button("🔮 切换到技能页").clicked() {
                        character.show_skill_page();
                        println!("🔮 切换到技能页");
                    }
                    
                    if ui.button("📍 重置位置").clicked() {
                        character.position = egui_macroquad::egui::pos2(300.0, 150.0);
                        println!("📍 角色对话框位置已重置");
                    }
                    
                    ui.separator();
                    ui.heading("装备状态");
                    
                    let equipped_count = character.equipment.iter().filter(|e| e.is_some()).count();
                    ui.label(format!("已装备: {} / 14", equipped_count));
                    
                    ui.separator();
                    ui.heading("技能状态");
                    
                    ui.label(format!("已学习技能: {}", character.skills.len()));
                    for skill in &character.skills {
                        ui.label(format!("  • {} Lv.{}/{}", 
                            skill.name, 
                            skill.level, 
                            skill.max_level
                        ));
                    }
                });
        });
        
        let egui_elapsed = get_time() - egui_start;

        // 绘制 egui
        egui_macroquad::draw();

        // FPS 计数
        let current_time = get_time();
        let frame_time = (current_time - last_time) as f32;
        last_time = current_time;

        frame_times.push(frame_time);
        if frame_times.len() > 60 {
            frame_times.remove(0);
        }

        let avg_frame_time = frame_times.iter().sum::<f32>() / frame_times.len() as f32;
        let fps = if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        };

        // 绘制性能信息
        let perf_text = format!(
            "FPS: {:.0} | Frame: {:.2}ms | egui: {:.2}ms",
            fps,
            avg_frame_time * 1000.0,
            egui_elapsed * 1000.0
        );
        
        draw_text_right_aligned(
            &perf_text,
            screen_width() - 10.0,
            screen_height() - 10.0,
            18.0,
            Color::from_rgba(100, 200, 100, 255),
        );

        // ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            if character_open {
                character_open = false;
                println!("⏹️ 角色对话框已关闭");
            } else {
                break;
            }
        }

        next_frame().await;
    }

    println!("\n👋 测试结束");
}
