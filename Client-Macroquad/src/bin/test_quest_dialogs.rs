/// Quest对话框测试程序
/// 
/// 测试功能：
/// - QuestListDialog - NPC任务列表对话框
/// - QuestDetailDialog - 任务详情对话框
/// - QuestLogDialog - 任务日志对话框
/// - QuestTrackingDialog - 任务追踪对话框
/// 
/// 快捷键：
/// - 1: 打开/关闭任务列表对话框
/// - 2: 打开/关闭任务详情对话框
/// - 3: 打开/关闭任务日志对话框
/// - 4: 打开/关闭任务追踪对话框
/// - R: 重新加载示例任务数据
/// - ESC: 退出程序

use macroquad::prelude::*;
use egui_macroquad::egui;

use client_macroquad::scenes::dialogs::game::{
    QuestListDialog, QuestDetailDialog, QuestLogDialog, QuestTrackingDialog,
};
use client_macroquad::scenes::dialogs::game::quest_log_dialog::{
    QuestInfo, QuestStatus, QuestRewards, QuestItem,
};
use client_macroquad::scenes::dialogs::Dialog;
use client_macroquad::resources::{set_data_path, preload_libraries, LibraryName};

struct QuestDialogTest {
    quest_list_dialog: QuestListDialog,
    quest_detail_dialog: QuestDetailDialog,
    quest_log_dialog: QuestLogDialog,
    quest_tracking_dialog: QuestTrackingDialog,
    
    quest_list_open: bool,
    quest_detail_open: bool,
    quest_log_open: bool,
    quest_tracking_open: bool,
    
    sample_quests: Vec<QuestInfo>,
}

impl QuestDialogTest {
    fn new() -> Self {
        let sample_quests = Self::create_sample_quests();
        
        Self {
            quest_list_dialog: QuestListDialog::new(),
            quest_detail_dialog: QuestDetailDialog::new(),
            quest_log_dialog: QuestLogDialog::new(),
            quest_tracking_dialog: QuestTrackingDialog::new(),
            
            quest_list_open: false,
            quest_detail_open: false,
            quest_log_open: false,
            quest_tracking_open: true,
            
            sample_quests,
        }
    }
    
    fn create_sample_quests() -> Vec<QuestInfo> {
        vec![
            QuestInfo {
                id: 1,
                name: "消灭稻草人".to_string(),
                description: "新手村外的稻草人威胁着村民的安全，请帮忙消灭10只稻草人。\n\n稻草人虽然看起来人畜无害，但它们会突然袭击路过的村民。为了保护村民的安全，村长希望你能清理掉村外的稻草人。".to_string(),
                npc_name: "村长".to_string(),
                status: QuestStatus::Accepted,
                progress: 3,
                max_progress: 10,
                level_required: 1,
                rewards: QuestRewards {
                    experience: 500,
                    gold: 100,
                    items: vec![
                        QuestItem {
                            icon_index: 0,
                            name: "小血瓶".to_string(),
                            count: 5,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 2,
                name: "收集鸡毛".to_string(),
                description: "铁匠需要20根鸡毛来制作羽毛箭，请到鸡舍收集。\n\n村里的铁匠正在研究新型的羽毛箭，但是缺少足够的材料。他希望你能帮忙收集一些鸡毛。".to_string(),
                npc_name: "铁匠".to_string(),
                status: QuestStatus::Accepted,
                progress: 15,
                max_progress: 20,
                level_required: 5,
                rewards: QuestRewards {
                    experience: 800,
                    gold: 200,
                    items: vec![
                        QuestItem {
                            icon_index: 10,
                            name: "铁剑".to_string(),
                            count: 1,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 3,
                name: "探索古墓".to_string(),
                description: "传说中的古墓出现了异常，需要勇敢的冒险者前去调查。\n\n最近有村民报告说，在村子北边的古墓里传出了奇怪的声音。法师认为可能有什么东西在古墓中苏醒了，需要有经验的冒险者去调查。".to_string(),
                npc_name: "法师".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 1,
                level_required: 15,
                rewards: QuestRewards {
                    experience: 2000,
                    gold: 1000,
                    items: vec![
                        QuestItem {
                            icon_index: 30,
                            name: "魔法戒指".to_string(),
                            count: 1,
                        },
                        QuestItem {
                            icon_index: 31,
                            name: "神秘卷轴".to_string(),
                            count: 3,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 4,
                name: "击败骷髅战士".to_string(),
                description: "地牢深处的骷髅战士复活了，需要强大的战士将其重新消灭。\n\n传说中被封印的骷髅战士突破了封印，开始在地牢中游荡。队长希望你能前去消灭这个威胁。".to_string(),
                npc_name: "队长".to_string(),
                status: QuestStatus::Completed,
                progress: 1,
                max_progress: 1,
                level_required: 20,
                rewards: QuestRewards {
                    experience: 5000,
                    gold: 2000,
                    items: vec![
                        QuestItem {
                            icon_index: 50,
                            name: "战士头盔".to_string(),
                            count: 1,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 5,
                name: "寻找失踪的商人".to_string(),
                description: "一位商人在前往城镇的路上失踪了，请帮忙寻找他的下落。\n\n商人公会报告说，一位重要的商人在三天前出发前往邻近城镇，但至今未到达。公会长希望你能沿着商道寻找他的踪迹。".to_string(),
                npc_name: "公会长".to_string(),
                status: QuestStatus::Available,
                progress: 0,
                max_progress: 3,
                level_required: 10,
                rewards: QuestRewards {
                    experience: 1500,
                    gold: 500,
                    items: vec![
                        QuestItem {
                            icon_index: 40,
                            name: "商人徽章".to_string(),
                            count: 1,
                        }
                    ],
                },
            },
            QuestInfo {
                id: 6,
                name: "采集草药".to_string(),
                description: "药师需要特殊的草药来制作治疗药水，请到森林中采集。".to_string(),
                npc_name: "药师".to_string(),
                status: QuestStatus::Accepted,
                progress: 8,
                max_progress: 15,
                level_required: 3,
                rewards: QuestRewards {
                    experience: 600,
                    gold: 150,
                    items: vec![
                        QuestItem {
                            icon_index: 60,
                            name: "高级血瓶".to_string(),
                            count: 3,
                        },
                        QuestItem {
                            icon_index: 61,
                            name: "高级蓝瓶".to_string(),
                            count: 3,
                        }
                    ],
                },
            },
        ]
    }
    
    fn reload_sample_quests(&mut self) {
        self.sample_quests = Self::create_sample_quests();
        println!("✅ 重新加载示例任务数据");
        
        // 更新追踪对话框
        self.quest_tracking_dialog.set_tracking_quests(
            self.sample_quests.iter()
                .filter(|q| q.status == QuestStatus::Accepted)
                .cloned()
                .collect()
        );
    }
    
    fn handle_input(&mut self) {
        if is_key_pressed(KeyCode::Key1) {
            self.quest_list_open = !self.quest_list_open;
            if self.quest_list_open {
                // 只显示可接受的任务
                let available_quests: Vec<_> = self.sample_quests.iter()
                    .filter(|q| matches!(q.status, QuestStatus::Available | QuestStatus::Completed))
                    .cloned()
                    .collect();
                self.quest_list_dialog.show_with_quests(available_quests);
            }
            println!("📋 任务列表对话框: {}", if self.quest_list_open { "打开" } else { "关闭" });
        }
        
        if is_key_pressed(KeyCode::Key2) {
            self.quest_detail_open = !self.quest_detail_open;
            if self.quest_detail_open {
                // 显示第一个已接受的任务
                if let Some(quest) = self.sample_quests.iter()
                    .find(|q| q.status == QuestStatus::Accepted)
                    .cloned() {
                    self.quest_detail_dialog.display_quest(quest);
                }
            }
            println!("📖 任务详情对话框: {}", if self.quest_detail_open { "打开" } else { "关闭" });
        }
        
        if is_key_pressed(KeyCode::Key3) {
            self.quest_log_open = !self.quest_log_open;
            println!("📜 任务日志对话框: {}", if self.quest_log_open { "打开" } else { "关闭" });
        }
        
        if is_key_pressed(KeyCode::Key4) {
            self.quest_tracking_open = !self.quest_tracking_open;
            println!("🎯 任务追踪对话框: {}", if self.quest_tracking_open { "打开" } else { "关闭" });
        }
        
        if is_key_pressed(KeyCode::R) {
            self.reload_sample_quests();
        }
    }
    
    fn draw_help_panel(&self, ctx: &egui::Context) {
        egui::Window::new("Quest对话框测试 - 帮助")
            .fixed_pos(egui::pos2(10.0, 10.0))
            .default_width(300.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("🎮 快捷键");
                ui.separator();
                
                ui.label("1 - 打开/关闭任务列表对话框");
                ui.label("2 - 打开/关闭任务详情对话框");
                ui.label("3 - 打开/关闭任务日志对话框");
                ui.label("4 - 打开/关闭任务追踪对话框");
                ui.label("R - 重新加载示例数据");
                ui.label("ESC - 退出程序");
                
                ui.add_space(10.0);
                ui.separator();
                ui.heading("📊 当前状态");
                
                ui.horizontal(|ui| {
                    ui.label("任务列表:");
                    ui.colored_label(
                        if self.quest_list_open { egui::Color32::GREEN } else { egui::Color32::GRAY },
                        if self.quest_list_open { "已打开" } else { "已关闭" }
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("任务详情:");
                    ui.colored_label(
                        if self.quest_detail_open { egui::Color32::GREEN } else { egui::Color32::GRAY },
                        if self.quest_detail_open { "已打开" } else { "已关闭" }
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("任务日志:");
                    ui.colored_label(
                        if self.quest_log_open { egui::Color32::GREEN } else { egui::Color32::GRAY },
                        if self.quest_log_open { "已打开" } else { "已关闭" }
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("任务追踪:");
                    ui.colored_label(
                        if self.quest_tracking_open { egui::Color32::GREEN } else { egui::Color32::GRAY },
                        if self.quest_tracking_open { "已打开" } else { "已关闭" }
                    );
                });
                
                ui.add_space(10.0);
                ui.label(format!("示例任务数: {}", self.sample_quests.len()));
                ui.label(format!("进行中: {}", self.sample_quests.iter().filter(|q| q.status == QuestStatus::Accepted).count()));
                ui.label(format!("可接受: {}", self.sample_quests.iter().filter(|q| q.status == QuestStatus::Available).count()));
                ui.label(format!("已完成: {}", self.sample_quests.iter().filter(|q| q.status == QuestStatus::Completed).count()));
                
                ui.add_space(10.0);
                ui.separator();
                ui.heading("🎨 纹理资源");
                ui.label("已加载:");
                ui.label("  • Prguse (对话框背景)");
                ui.label("  • Prguse2 (滚动条)");
                ui.label("  • Title (标题和按钮)");
                ui.label("  • Items (物品图标)");
            });
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Quest对话框测试程序".to_string(),
        window_width: 1280,
        window_height: 800,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("===========================================");
    println!("  Quest对话框测试程序");
    println!("===========================================");
    println!();
    println!("快捷键:");
    println!("  1 - 打开/关闭任务列表对话框");
    println!("  2 - 打开/关闭任务详情对话框");
    println!("  3 - 打开/关闭任务日志对话框");
    println!("  4 - 打开/关闭任务追踪对话框");
    println!("  R - 重新加载示例数据");
    println!("  ESC - 退出程序");
    println!();
    println!("===========================================");
    
    // 设置资源路径并预加载纹理库
    println!("🔄 正在加载纹理资源...");
    println!("📂 数据路径: ./Data/");
    set_data_path("./Data/");
    
    // 预加载quest对话框需要的纹理库
    preload_libraries(&[
        LibraryName::Prguse,   // 背景、按钮、图标
        LibraryName::Prguse2,  // 滚动条、关闭按钮
        LibraryName::Title,    // 标题、功能按钮
        LibraryName::Items,    // 物品图标
    ]);
    
    println!("✅ 纹理资源加载完成");
    println!();
    
    let mut test = QuestDialogTest::new();
    
    // 初始化追踪对话框
    test.quest_tracking_dialog.set_tracking_quests(
        test.sample_quests.iter()
            .filter(|q| q.status == QuestStatus::Accepted)
            .cloned()
            .collect()
    );
    
    loop {
        clear_background(Color::from_rgba(45, 52, 54, 255));
        
        // 处理输入
        if is_key_pressed(KeyCode::Escape) {
            println!("👋 退出程序");
            break;
        }
        
        test.handle_input();
        
        // 绘制egui UI
        egui_macroquad::ui(|ctx| {
            // 绘制帮助面板
            test.draw_help_panel(ctx);
            
            // 显示对话框
            test.quest_list_dialog.show(ctx, &mut test.quest_list_open);
            test.quest_detail_dialog.show(ctx, &mut test.quest_detail_open);
            test.quest_log_dialog.show(ctx, &mut test.quest_log_open);
            test.quest_tracking_dialog.show(ctx, &mut test.quest_tracking_open);
        });
        
        // 绘制背景信息
        draw_text(
            "Quest Dialog Test - Press ESC to quit",
            10.0,
            screen_height() - 20.0,
            20.0,
            LIGHTGRAY,
        );
        
        egui_macroquad::draw();
        
        next_frame().await;
    }
}
