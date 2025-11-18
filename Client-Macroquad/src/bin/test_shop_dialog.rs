// ============================================================================
// 测试新的商店对话框组件系统
// ============================================================================

// FIXME: 此测试文件需要更新导入路径
// use client_macroquad::ui::dialogs::{GameShopDialog, GameShopAction, ShopItem};
use client_macroquad::scenes::dialogs::game::{GameShopDialog, GameShopAction};
use client_macroquad::scenes::dialogs::game::game_shop_dialog::ShopItem;
use macroquad::prelude::*;

#[macroquad::main("测试商店对话框")]
async fn main() {
    // 创建商店对话框
    let mut shop_dialog = GameShopDialog::new();
    
    // 加载测试商品数据
    let test_items = vec![
        ShopItem {
            id: 1,
            name: "龙纹剑".to_string(),
            description: "传说中的神器，攻击力+100".to_string(),
            price: 50000,
            icon_index: 1,
            category: "武器".to_string(),
            in_stock: true,
        },
        ShopItem {
            id: 2,
            name: "凤凰羽衣".to_string(),
            description: "美丽的法师袍，魔法防御+50".to_string(),
            price: 30000,
            icon_index: 20,
            category: "防具".to_string(),
            in_stock: true,
        },
        ShopItem {
            id: 3,
            name: "生命药水".to_string(),
            description: "瞬间恢复1000点生命值".to_string(),
            price: 1000,
            icon_index: 40,
            category: "药品".to_string(),
            in_stock: true,
        },
        ShopItem {
            id: 4,
            name: "传送卷轴".to_string(),
            description: "随机传送到安全地点".to_string(),
            price: 5000,
            icon_index: 60,
            category: "道具".to_string(),
            in_stock: false,
        },
    ];
    
    shop_dialog.load_items(test_items);
    shop_dialog.show();
    
    loop {
        clear_background(BLACK);
        
        // 处理按键
        if is_key_pressed(KeyCode::Escape) {
            if shop_dialog.is_visible() {
                shop_dialog.hide();
            } else {
                break;
            }
        }
        
        if is_key_pressed(KeyCode::Space) {
            shop_dialog.show();
        }
        
        // 绘制说明
        if !shop_dialog.is_visible() {
            draw_text("按 SPACE 打开商店", 20.0, 30.0, 20.0, WHITE);
            draw_text("按 ESC 退出", 20.0, 60.0, 20.0, WHITE);
        }
        
        // 绘制商店对话框
        egui_macroquad::ui(|ctx| {
            if let Some(action) = shop_dialog.draw(ctx) {
                match action {
                    GameShopAction::Close => {
                        println!("商店关闭");
                    },
                    GameShopAction::ItemSelected(index) => {
                        println!("选中商品索引: {}", index);
                    },
                    GameShopAction::BuyItem(item_id) => {
                        println!("购买商品 ID: {}", item_id);
                    },
                    GameShopAction::PageChanged(page) => {
                        println!("切换到页面: {}", page);
                    },
                }
            }
        });
        
        egui_macroquad::draw();
        next_frame().await;
    }
}