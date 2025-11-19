// ============================================================================
// 所有UI组件统一测试程序
// 测试所有公共组件的功能和交互
// ============================================================================

use macroquad::prelude::*;
use egui_macroquad::egui;
use client_macroquad::ui::components::{
    TexturedButton, TexturedCheckBox, TexturedDialog, ShopItemViewer, DialogType,
    TexturedMessageBox, MessageBoxButtons, MessageBoxResult
};
use client_macroquad::ui::dialogs::{GameShopDialog, ShopItem};
use client_macroquad::resources::{ResourceManager, LibraryName};

#[derive(PartialEq, Clone, Copy)]
enum TestPage {
    Buttons,
    Checkboxes,
    Dialogs,
    MessageBox,
    Shop,
    ItemViewer,
}

struct ComponentTestApp {
    current_page: TestPage,
    font_loaded: bool,

    // 基础组件
    btn_normal: TexturedButton,
    btn_disabled: TexturedButton,
    checkbox_native: TexturedCheckBox,
    checkbox_textured: TexturedCheckBox,
    
    // 对话框
    dialog: TexturedDialog,
    message_box: TexturedMessageBox,
    shop_dialog: GameShopDialog,
    shop_viewer: ShopItemViewer,
    
    // 状态
    counter: i32,
    last_msg_result: String,
}

impl ComponentTestApp {
    fn new() -> Self {
        // 1. 基础按钮 (使用 Prguse2 库中的关闭按钮样式)
        let btn_normal = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(360, Some(361), Some(362), None) 
            .with_size(egui::vec2(20.0, 20.0))
            .with_tooltip("点击增加计数");

        let btn_disabled = TexturedButton::new()
            .with_library(LibraryName::Prguse2)
            .with_states(360, Some(361), Some(362), None)
            .with_size(egui::vec2(20.0, 20.0))
            .with_enabled(false)
            .with_tooltip("禁用按钮");

        // 2. 复选框
        let checkbox_native = TexturedCheckBox::new()
            .with_text("原生样式复选框")
            .with_checked(true);

        let checkbox_textured = TexturedCheckBox::new()
            .with_textures(LibraryName::Prguse, 2087, 2086) // 使用 Prguse 默认复选框索引
            .with_text("纹理复选框")
            .with_tooltip("带纹理的复选框");

        // 3. 基础对话框 (使用 Title 库中的聊天选项背景，避免看起来像 MessageBox)
        let dialog = TexturedDialog::new("test_dialog", "测试对话框")
            .with_type(DialogType::Normal)
            .with_background(LibraryName::Title, 466) // 使用 Title[466] (ChatOptionDialog)
            .with_rect(egui::pos2(300.0, 100.0), egui::vec2(224.0, 180.0));
        
        // 4. 消息框
        let message_box = TexturedMessageBox::new(
            "这是一个测试消息框。\n请点击按钮进行选择。",
            MessageBoxButtons::YesNoCancel
        );

        // 5. 商店对话框
        let mut shop_dialog = GameShopDialog::new();
        // 添加一些测试商品
        let items = vec![
            ShopItem {
                id: 1,
                name: "木剑".to_string(),
                description: "新手武器".to_string(),
                price: 100,
                icon_index: 100,
                category: "武器".to_string(),
                in_stock: true,
            },
            ShopItem {
                id: 2,
                name: "布衣".to_string(),
                description: "新手防具".to_string(),
                price: 200,
                icon_index: 101,
                category: "防具".to_string(),
                in_stock: true,
            },
            ShopItem {
                id: 3,
                name: "金创药(小)".to_string(),
                description: "恢复生命值".to_string(),
                price: 50,
                icon_index: 102,
                category: "消耗品".to_string(),
                in_stock: false, // 缺货测试
            },
        ];
        shop_dialog.load_items(items);

        // 6. 独立查看器
        let shop_viewer = ShopItemViewer::new()
            .with_total_items(5);

        Self {
            current_page: TestPage::Buttons,
            font_loaded: false,
            btn_normal,
            btn_disabled,
            checkbox_native,
            checkbox_textured,
            dialog,
            message_box,
            shop_dialog,
            shop_viewer,
            counter: 0,
            last_msg_result: "无".to_string(),
        }
    }

    fn load_fonts(&mut self, ctx: &egui::Context) {
        if self.font_loaded {
            return;
        }

        let mut fonts = egui::FontDefinitions::default();
        let font_path = "assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf";
        
        println!("Attempting to load font from: {}", font_path);
        
        match std::fs::read(font_path) {
            Ok(font_data) => {
                fonts.font_data.insert(
                    "my_font".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                );

                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "my_font".to_owned());

                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "my_font".to_owned());

                ctx.set_fonts(fonts);
                println!("Font loaded successfully!");
                self.font_loaded = true;
            },
            Err(e) => {
                eprintln!("Failed to load font: {}", e);
                // Fallback or just mark as loaded to stop trying
                self.font_loaded = true; 
            }
        }
    }

    fn update(&mut self) {
        // 这里可以处理一些非UI的逻辑
    }

    fn draw(&mut self) {
        egui_macroquad::ui(|ctx| {
            self.load_fonts(ctx);

            // 侧边栏 - 组件列表
            egui::SidePanel::left("component_list_panel")
                .resizable(false)
                .default_width(150.0)
                .show(ctx, |ui| {
                    ui.heading("组件列表");
                    ui.separator();
                    
                    if ui.selectable_label(self.current_page == TestPage::Buttons, "基础按钮 (Buttons)").clicked() {
                        self.current_page = TestPage::Buttons;
                    }
                    if ui.selectable_label(self.current_page == TestPage::Checkboxes, "复选框 (Checkboxes)").clicked() {
                        self.current_page = TestPage::Checkboxes;
                    }
                    if ui.selectable_label(self.current_page == TestPage::Dialogs, "对话框 (Dialogs)").clicked() {
                        self.current_page = TestPage::Dialogs;
                    }
                    if ui.selectable_label(self.current_page == TestPage::MessageBox, "消息框 (MsgBox)").clicked() {
                        self.current_page = TestPage::MessageBox;
                    }
                    if ui.selectable_label(self.current_page == TestPage::Shop, "游戏商店 (Shop)").clicked() {
                        self.current_page = TestPage::Shop;
                    }
                    if ui.selectable_label(self.current_page == TestPage::ItemViewer, "物品查看 (Viewer)").clicked() {
                        self.current_page = TestPage::ItemViewer;
                    }
                    
                    ui.separator();
                    ui.label("点击上方列表切换测试内容");
                });

            // 主内容区域
            egui::CentralPanel::default().show(ctx, |ui| {
                match self.current_page {
                    TestPage::Buttons => {
                        ui.heading("基础按钮测试");
                        ui.label("测试 TexturedButton 组件的不同状态");
                        ui.add_space(20.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("正常按钮:");
                            if self.btn_normal.draw(ui) {
                                self.counter += 1;
                            }
                            ui.label(format!("(点击计数: {})", self.counter));
                        });
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("禁用按钮:");
                            self.btn_disabled.draw(ui);
                            ui.label("(不可点击)");
                        });
                    },
                    TestPage::Checkboxes => {
                        ui.heading("复选框测试");
                        ui.label("测试 TexturedCheckBox 组件");
                        ui.add_space(20.0);
                        
                        ui.label("1. 原生样式:");
                        self.checkbox_native.draw(ui);
                        
                        ui.add_space(10.0);
                        
                        ui.label("2. 纹理样式 (模拟):");
                        self.checkbox_textured.draw(ui);
                    },
                    TestPage::Dialogs => {
                        ui.heading("对话框测试");
                        ui.label("测试 TexturedDialog 组件");
                        ui.label("请点击下方按钮显示/隐藏对话框");
                        ui.add_space(20.0);
                        
                        if ui.button(if self.dialog.visible { "隐藏对话框" } else { "显示对话框" }).clicked() {
                            if self.dialog.visible {
                                self.dialog.hide();
                            } else {
                                self.dialog.show();
                            }
                        }
                    },
                    TestPage::MessageBox => {
                        ui.heading("消息框测试");
                        ui.label("测试 TexturedMessageBox 组件");
                        ui.add_space(20.0);
                        
                        if ui.button("显示消息框").clicked() {
                            self.message_box.show();
                        }
                        
                        ui.add_space(10.0);
                        ui.label(format!("上次选择结果: {}", self.last_msg_result));
                    },
                    TestPage::Shop => {
                        ui.heading("游戏商店测试");
                        ui.label("测试 GameShopDialog 组件");
                        ui.add_space(20.0);
                        
                        if ui.button("打开商店").clicked() {
                            self.shop_dialog.show();
                        }
                    },
                    TestPage::ItemViewer => {
                        ui.heading("物品查看器测试");
                        ui.label("测试 ShopItemViewer 组件");
                        ui.add_space(20.0);
                        
                        self.shop_viewer.show();
                        self.shop_viewer.draw(ctx); // Viewer通常是直接绘制的
                    },
                }
            });

            // 独立绘制的弹窗组件 (Overlay)
            
            // 消息框 (最高优先级)
            let msg_result = self.message_box.draw(ctx);
            if msg_result != MessageBoxResult::None {
                self.last_msg_result = format!("{:?}", msg_result);
            }

            if self.current_page == TestPage::Dialogs {
                if self.dialog.draw_base(ctx) {
                    // 对话框关闭时的回调
                }
                if self.dialog.visible {
                    egui::Area::new(egui::Id::new("test_dialog_content"))
                        .fixed_pos(self.dialog.position + egui::vec2(20.0, 40.0))
                        .order(self.dialog.order)
                        .show(ctx, |ui| {
                            ui.label("这是一个测试对话框内容");
                            ui.label("你可以拖拽标题栏移动它");
                        });
                }
            }

            if self.current_page == TestPage::Shop {
                if let Some(action) = self.shop_dialog.draw(ctx) {
                    match action {
                        client_macroquad::ui::dialogs::GameShopAction::Close => {
                            // 商店内部有关闭逻辑，这里可以处理额外的关闭事件
                        },
                        client_macroquad::ui::dialogs::GameShopAction::BuyItem(id) => {
                            println!("购买了商品 ID: {}", id);
                        }
                        _ => {}
                    }
                }
            }
        });
    }
}

#[macroquad::main("UI Component Test")]
async fn main() {
    // 初始化资源管理器（虽然没有真实加载，但需要结构存在）
    // 注意：实际运行需要正确的资源路径和文件
    let _resource_manager = ResourceManager::new();
    
    let mut app = ComponentTestApp::new();

    loop {
        clear_background(GRAY);
        
        app.update();
        app.draw();
        
        egui_macroquad::draw();
        
        next_frame().await;
    }
}
