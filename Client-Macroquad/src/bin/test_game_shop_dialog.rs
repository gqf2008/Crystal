// 测试 GameShopDialog（游戏商城）

use client_macroquad::scenes::dialogs::game::GameShopDialog;
use client_macroquad::ui::text_renderer::*;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - 游戏商城测试".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - 游戏商城测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 初始化中文字体
    init_chinese_font().await;
    
    // 配置 egui 中文字体
    egui_macroquad::cfg(|ctx| {
        setup_egui_chinese_font(ctx);
    });
    
    // 创建商城对话框
    let mut shop = GameShopDialog::new();
    
    // 默认显示商城
    let mut shop_open = true;
    
    println!("✅ 商城对话框已创建并显示");
    println!("\n💡 测试功能清单:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 基础功能:");
    println!("   ✓ 拖拽标题栏可移动窗口");
    println!("   ✓ 点击关闭按钮或按ESC关闭商城");
    println!("   ✓ 商品分类标签页切换（全部/热销/特价/新品）");
    println!("   ✓ 职业筛选标签页（全部/战士/刺客/道士/法师/弓箭手）");
    println!("\n🛍️ 商品浏览:");
    println!("   ✓ 4x2网格布局（每页8个商品）");
    println!("   ✓ 商品图标和名称显示");
    println!("   ✓ 价格信息（金币/元宝）");
    println!("   ✓ 库存状态（缺货商品标记）");
    println!("   ✓ 热销/新品标签");
    println!("\n🔍 商品预览:");
    println!("   ✓ 点击商品显示详细预览窗口");
    println!("   ✓ 预览窗口可拖拽移动");
    println!("   ✓ 显示物品名称、描述、价格");
    println!("   ✓ 8方向预览切换（◀/▶按钮）");
    println!("   ✓ 点击外部区域或按ESC关闭预览");
    println!("   ✓ 再次点击商品关闭预览");
    println!("\n📄 分页浏览:");
    println!("   ✓ 上一页/下一页按钮");
    println!("   ✓ 页码显示（当前页/总页数）");
    println!("   ✓ 切换分类/职业时自动重置到第1页");
    println!("\n💰 货币系统:");
    println!("   ✓ 实时显示玩家金币数量");
    println!("   ✓ 实时显示玩家元宝数量");
    println!("   ✓ 购买前检查余额是否足够");
    println!("\n📊 测试数据:");
    println!("   • 初始金币：999,999");
    println!("   • 初始元宝：10,000");
    println!("   • 商品数量：5件（含不同分类）");
    println!("   • 龙纹剑（武器，热销）");
    println!("   • 天师道袍（防具，新品）");
    println!("   • 强效金疮药（药品）");
    println!("   • 传送戒指（特殊，热销+新品，缺货）");
    println!("   • 华丽时装（时装，新品）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // FPS 统计
    let mut frame_times: Vec<f32> = Vec::with_capacity(60);
    let mut last_time = get_time();

    loop {
        // 绘制背景
        clear_background(Color::from_rgba(40, 50, 60, 255));

        // 绘制测试说明
        let hints = [
            "🛒 游戏商城测试中...",
            "点击商品查看详情 | 切换分类/职业浏览",
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

        // egui UI
        let egui_start = get_time();
        egui_macroquad::ui(|ctx| {
            // 绘制商城
            use client_macroquad::scenes::dialogs::Dialog;
            shop.show(ctx, &mut shop_open);
            
            // 绘制测试控制面板
            egui_macroquad::egui::Window::new("🧪 测试控制面板")
                .default_pos(egui_macroquad::egui::pos2(10.0, 50.0))
                .default_size(egui_macroquad::egui::vec2(300.0, 450.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("商城状态");
                    ui.separator();
                    
                    ui.label(format!("可见状态: {}", if shop_open { "✅ 显示中" } else { "❌ 已隐藏" }));
                    ui.label(format!("玩家金币: {} 💰", shop.player_gold));
                    ui.label(format!("玩家元宝: {} 💎", shop.player_ingot));
                    ui.label(format!("当前分类: {}", shop.selected_section.display_name()));
                    ui.label(format!("当前职业: {}", shop.selected_class.display_name()));
                    ui.label(format!("商品总数: {}", shop.shop_items.len()));
                    ui.label(format!("过滤后: {}", shop.filtered_items.len()));
                    ui.label(format!("当前页: {}/{}", 
                        shop.current_page + 1, 
                        if shop.filtered_items.is_empty() { 1 } else { (shop.filtered_items.len() + shop.items_per_page - 1) / shop.items_per_page }
                    ));
                    
                    ui.separator();
                    ui.heading("快捷操作");
                    
                    if ui.button("🛒 切换商城显示").clicked() {
                        shop_open = !shop_open;
                    }
                    
                    if ui.button("💰 增加 10000 金币").clicked() {
                        shop.player_gold += 10000;
                        println!("💰 金币增加到: {}", shop.player_gold);
                    }
                    
                    if ui.button("💎 增加 1000 元宝").clicked() {
                        shop.player_ingot += 1000;
                        println!("💎 元宝增加到: {}", shop.player_ingot);
                    }
                    
                    if ui.button("📍 重置位置").clicked() {
                        shop.position = egui_macroquad::egui::pos2(300.0, 150.0);
                        println!("📍 商城位置已重置");
                    }
                    
                    ui.separator();
                    ui.heading("分类切换");
                    
                    ui.label("主要分类:");
                    ui.horizontal_wrapped(|ui| {
                        use client_macroquad::scenes::dialogs::game::GameShopSection;
                        for section in GameShopSection::ALL {
                            if ui.button(section.display_name()).clicked() {
                                shop.selected_section = *section;
                                shop.selected_class = client_macroquad::scenes::dialogs::game::GameShopClass::All;
                                shop.current_page = 0;
                                shop.selected_item = None;
                                shop.item_viewer = None;
                                shop.filter_items();
                            }
                        }
                    });
                    
                    ui.add_space(5.0);
                    ui.label("职业筛选:");
                    ui.horizontal_wrapped(|ui| {
                        use client_macroquad::scenes::dialogs::game::GameShopClass;
                        for class in GameShopClass::ALL {
                            if ui.button(class.display_name()).clicked() {
                                shop.selected_class = *class;
                                shop.current_page = 0;
                                shop.selected_item = None;
                                shop.item_viewer = None;
                                shop.filter_items();
                            }
                        }
                    });
                    
                    ui.separator();
                    ui.heading("预览控制");
                    
                    if shop.item_viewer.is_some() {
                        ui.label("✅ 预览器已打开");
                        if ui.button("❌ 关闭预览器").clicked() {
                            shop.item_viewer = None;
                            shop.selected_item = None;
                        }
                    } else {
                        ui.label("❌ 预览器已关闭");
                    }
                    
                    ui.separator();
                    ui.label("💡 提示：点击商品查看详情");
                });
            
            // FPS 显示
            egui_macroquad::egui::Window::new("📊 性能")
                .default_pos(egui_macroquad::egui::pos2(10.0, 520.0))
                .default_size(egui_macroquad::egui::vec2(300.0, 100.0))
                .resizable(false)
                .show(ctx, |ui| {
                    let current_time = get_time();
                    let frame_time = (current_time - last_time) as f32;
                    last_time = current_time;
                    
                    frame_times.push(frame_time);
                    if frame_times.len() > 60 {
                        frame_times.remove(0);
                    }
                    
                    let avg_frame_time = frame_times.iter().sum::<f32>() / frame_times.len() as f32;
                    let fps = 1.0 / avg_frame_time;
                    
                    ui.label(format!("FPS: {:.1}", fps));
                    ui.label(format!("帧时间: {:.2}ms", avg_frame_time * 1000.0));
                    ui.label(format!("egui耗时: {:.2}ms", (get_time() - egui_start) * 1000.0));
                });
        });

        // 绘制 egui
        egui_macroquad::draw();

        // 键盘快捷键
        if is_key_pressed(KeyCode::Escape) && !shop_open {
            break; // 商城关闭时，ESC退出程序
        }
        
        if is_key_pressed(KeyCode::F1) {
            shop_open = !shop_open;
        }

        next_frame().await;
    }
    
    println!("\n👋 测试结束");
}
