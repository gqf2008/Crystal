// ============================================================================
// MainDialog - 游戏主界面底部工具栏
// ============================================================================
// 
// 【功能说明】
// 1. 底部工具栏背景（根据分辨率适配）
// 2. 生命值/魔法值球显示
// 3. 经验条和负重条
// 4. 功能按钮组（背包、角色、技能、任务、选项、菜单、商城）
// 5. 角色信息显示（等级、金币、负重等）
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::{BeltDialog, ChatDialog, ChatControlBar, InventoryDialog};
use crate::scenes::dialogs::Dialog;

/// 主界面底部工具栏
pub struct MainDialog {
    /// 是否可见
    visible: bool,
    /// 当前分辨率索引 (0=800, 1=1024, 2=1280+)
    resolution_index: usize,
    /// 模拟数据 - 当前生命值
    hp: i32,
    /// 模拟数据 - 最大生命值
    max_hp: i32,
    /// 模拟数据 - 当前魔法值
    mp: i32,
    /// 模拟数据 - 最大魔法值
    max_mp: i32,
    /// 模拟数据 - 经验值百分比
    exp_percent: f32,
    /// 模拟数据 - 等级
    level: u32,
    /// 模拟数据 - 角色名
    character_name: String,
    /// 模拟数据 - 金币
    gold: u32,
    /// 模拟数据 - 当前负重
    weight: u32,
    /// 模拟数据 - 最大负重
    max_weight: u32,
    /// 模拟数据 - 背包空格数
    bag_space: u32,
    
    // 子对话框
    /// 血瓶快捷栏
    /// 快捷栏
    belt_dialog: BeltDialog,
    /// 聊天窗口
    chat_dialog: ChatDialog,
    /// 聊天控制栏
    chat_control_bar: ChatControlBar,
    /// 背包
    inventory_dialog: InventoryDialog,
}

impl MainDialog {
    pub fn new() -> Self {
        // 根据屏幕宽度决定分辨率索引
        let screen_width = macroquad::prelude::screen_width();
        let screen_height = macroquad::prelude::screen_height();
        let dpi_scale = macroquad::prelude::screen_dpi_scale();
        
        let screen_w = screen_width / dpi_scale;
        let screen_h = screen_height / dpi_scale;
        
        let resolution_index = if screen_width <= 800.0 {
            0
        } else if screen_width <= 1024.0 {
            1
        } else {
            2
        };
        
        // MainDialog 的 X 坐标（底部居中）
        let bg_info = LibraryName::Prguse.get_size(resolution_index).unwrap_or((1024, 150));
        let bg_width = bg_info.0 as f32;
        let main_dialog_x = (screen_w - bg_width) / 2.0;

        Self {
            visible: true,
            resolution_index,
            // 模拟数据
            hp: 850,
            max_hp: 1000,
            mp: 450,
            max_mp: 600,
            exp_percent: 0.65,
            level: 45,
            character_name: "测试角色".to_string(),
            gold: 123456,
            weight: 75,
            max_weight: 100,
            bag_space: 28,
            
            // 子对话框
            belt_dialog: BeltDialog::new(main_dialog_x, screen_h),
            chat_dialog: ChatDialog::new(main_dialog_x, screen_h, resolution_index),
            chat_control_bar: ChatControlBar::new(main_dialog_x, screen_h, resolution_index),
            inventory_dialog: InventoryDialog::new(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let screen_width = macroquad::prelude::screen_width() / macroquad::prelude::screen_dpi_scale();
        let screen_height = macroquad::prelude::screen_height() / macroquad::prelude::screen_dpi_scale();

        // 获取主背景纹理
        let bg_info = LibraryName::Prguse.get_size(self.resolution_index).unwrap_or((1024, 150));
        let bg_width = bg_info.0 as f32;
        let bg_height = bg_info.1 as f32;

        // 底部居中显示
        let dialog_x = (screen_width - bg_width) / 2.0;
        let dialog_y = screen_height - bg_height;

        // 绘制主界面
        let base_pos = egui::pos2(dialog_x, dialog_y);
        
        egui::Area::new(egui::Id::new("main_dialog"))
            .fixed_pos(base_pos)
            .movable(false)
            .interactable(true)
            .order(egui::Order::Middle)  // 保持在 Middle 层，让聊天组件（Foreground）优先响应
            .show(ctx, |ui| {
                // 创建基于固定位置的 rect
                let rect = egui::Rect::from_min_size(base_pos, egui::vec2(bg_width, bg_height));
                
                // 分配空间（但不使用返回的 rect，因为它可能有偏移）
                ui.allocate_rect(
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(bg_width, bg_height)),
                    egui::Sense::hover()
                );

                // 绘制主背景
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, self.resolution_index) {
                    if let Some(bg_texture) = info.egui_texture {
                        ui.painter().image(
                            bg_texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                // 绘制生命值球（左侧）
                self.draw_health_orb(ui, ctx, &rect);

                // 绘制魔法值球（右侧）
                self.draw_mana_orb(ui, ctx, &rect);

                // 绘制经验条
                self.draw_exp_bar(ui, ctx, &rect);

                // 绘制负重条
                self.draw_weight_bar(ui, ctx, &rect);

                // 绘制角色信息
                self.draw_character_info(ui, &rect);

                // 绘制功能按钮组
                self.draw_buttons(ui, ctx, &rect);
            });
    }

    /// 绘制生命值球
    fn draw_health_orb(&self, ui: &mut egui::Ui, ctx: &egui::Context, rect: &egui::Rect) {
        // 生命值球位置（左侧）
        // 原工程：X = MainDialog.X, Y = HealthOrb.DisplayLocation.Y + 80 - height
        // HealthOrb.Location = (0, 30)，所以 HealthOrb.DisplayLocation.Y = MainDialog.Y + 30
        let orb_x = rect.min.x;  // 直接使用 MainDialog.X，不加偏移
        let orb_y = rect.min.y + 30.0;  // HealthOrb 相对于 MainDialog 的 Y 偏移
        
        // 绘制生命值球纹理 Prguse[4]
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 4) {
            if let Some(orb_texture) = info.egui_texture {
                let tex_size = orb_texture.size_vec2();
                
                // 纹理实际高度（通常是80，但使用实际值更准确）
                let orb_height = tex_size.y.min(80.0);
                
                // 计算生命值高度（从底部向上填充）
                let hp_percent = (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0);
                let hp_height = orb_height * hp_percent;
                
                // 左半部分（红色生命值） - 宽度50像素
                let hp_src_y = orb_height - hp_height;
                let hp_src_rect = egui::Rect::from_min_max(
                    egui::pos2(0.0 / tex_size.x, hp_src_y / tex_size.y),
                    egui::pos2(50.0 / tex_size.x, orb_height / tex_size.y)
                );
                
                let hp_dst_rect = egui::Rect::from_min_size(
                    egui::pos2(orb_x, orb_y + hp_src_y),
                    egui::vec2(50.0, hp_height)
                );
                
                ui.painter().image(
                    orb_texture.id(),
                    hp_dst_rect,
                    hp_src_rect,
                    egui::Color32::WHITE,
                );
                
                // 右半部分（蓝色魔法值） - x从51开始，宽度50像素
                let mp_percent = (self.mp as f32 / self.max_mp as f32).clamp(0.0, 1.0);
                let mp_height = orb_height * mp_percent;
                let mp_src_y = orb_height - mp_height;
                
                let mp_src_rect = egui::Rect::from_min_max(
                    egui::pos2(51.0 / tex_size.x, mp_src_y / tex_size.y),
                    egui::pos2(101.0 / tex_size.x, orb_height / tex_size.y)
                );
                
                let mp_dst_rect = egui::Rect::from_min_size(
                    egui::pos2(orb_x + 51.0, orb_y + mp_src_y),
                    egui::vec2(50.0, mp_height)
                );
                
                ui.painter().image(
                    orb_texture.id(),
                    mp_dst_rect,
                    mp_src_rect,
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制数值文字
        let hp_text = format!("{}/{}", self.hp, self.max_hp);
        ui.painter().text(
            egui::pos2(orb_x + 50.0, orb_y + 27.0),
            egui::Align2::CENTER_CENTER,
            &hp_text,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );
        
        let mp_text = format!("{}/{}", self.mp, self.max_mp);
        ui.painter().text(
            egui::pos2(orb_x + 50.0, orb_y + 42.0),
            egui::Align2::CENTER_CENTER,
            &mp_text,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );
    }

    /// 绘制魔法值球
    fn draw_mana_orb(&self, _ui: &mut egui::Ui, _ctx: &egui::Context, _rect: &egui::Rect) {
        // 魔法值球已经在 draw_health_orb 中一起绘制了
    }

    /// 绘制经验条
    fn draw_exp_bar(&self, ui: &mut egui::Ui, ctx: &egui::Context, rect: &egui::Rect) {
        // 经验条位置
        let bar_x = rect.min.x + 9.0;
        let bar_y = rect.min.y + 143.0;
        
        // 根据分辨率选择纹理索引 (800用7，其他用8)
        let exp_texture_idx = if self.resolution_index == 0 { 7 } else { 8 };
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, exp_texture_idx) {
            if let Some(exp_texture) = info.egui_texture {
                let tex_size = exp_texture.size_vec2();
                
                // 计算经验百分比对应的宽度
                let bar_width = tex_size.x - 3.0;  // 减去3像素边距
                let fill_width = bar_width * self.exp_percent;
                
                // 源矩形：裁剪宽度
                let src_rect = egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(fill_width / tex_size.x, 1.0)
                );
                
                // 目标矩形
                let dst_rect = egui::Rect::from_min_size(
                    egui::pos2(bar_x, bar_y),
                    egui::vec2(fill_width, tex_size.y)
                );
                
                ui.painter().image(
                    exp_texture.id(),
                    dst_rect,
                    src_rect,
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 经验百分比文字
        let exp_text = format!("{:.1}%", self.exp_percent * 100.0);
        ui.painter().text(
            egui::pos2(bar_x + 40.0, bar_y + 5.0),
            egui::Align2::CENTER_CENTER,
            &exp_text,
            egui::FontId::proportional(9.0),
            egui::Color32::WHITE,
        );
    }

    /// 绘制负重条
    fn draw_weight_bar(&self, ui: &mut egui::Ui, ctx: &egui::Context, rect: &egui::Rect) {
        // 负重条位置（右侧，从右向左计算）
        let bar_x = rect.max.x - 105.0;
        let bar_y = rect.min.y + 103.0;
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 76) {
            if let Some(weight_texture) = info.egui_texture {
                let tex_size = weight_texture.size_vec2();
                
                // 计算负重百分比
                let weight_percent = self.weight as f32 / self.max_weight as f32;
                let weight_percent = weight_percent.min(1.0);
                
                // 计算填充宽度
                let bar_width = tex_size.x - 2.0;  // 减去2像素边距
                let fill_width = bar_width * weight_percent;
                
                // 源矩形：裁剪宽度
                let src_rect = egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(fill_width / tex_size.x, 1.0)
                );
                
                // 目标矩形
                let dst_rect = egui::Rect::from_min_size(
                    egui::pos2(bar_x, bar_y),
                    egui::vec2(fill_width, tex_size.y)
                );
                
                // 根据负重比例选择颜色
                let color = if weight_percent < 0.8 {
                    egui::Color32::WHITE  // 正常，使用原色
                } else if weight_percent < 1.0 {
                    egui::Color32::from_rgb(255, 255, 0)  // 接近超重，黄色
                } else {
                    egui::Color32::from_rgb(255, 100, 100)  // 超重，红色
                };
                
                ui.painter().image(
                    weight_texture.id(),
                    dst_rect,
                    src_rect,
                    color,
                );
            }
        }
        
        // 负重文字
        let weight_text = format!("{}/{}", self.weight, self.max_weight);
        ui.painter().text(
            egui::pos2(bar_x + 40.0, bar_y + 5.0),
            egui::Align2::CENTER_CENTER,
            &weight_text,
            egui::FontId::proportional(9.0),
            egui::Color32::WHITE,
        );
    }

    /// 绘制角色信息
    fn draw_character_info(&self, ui: &mut egui::Ui, rect: &egui::Rect) {
        let info_x = rect.min.x + 120.0;
        let info_y = rect.min.y + 15.0;

        // 角色名和等级
        let name_level = format!("{} Lv.{}", self.character_name, self.level);
        ui.painter().text(
            egui::pos2(info_x, info_y),
            egui::Align2::LEFT_TOP,
            &name_level,
            egui::FontId::proportional(16.0),
            egui::Color32::from_rgb(255, 215, 0),
        );

        // 金币
        let gold_text = format!("金币: {}", self.gold);
        ui.painter().text(
            egui::pos2(info_x, info_y + 20.0),
            egui::Align2::LEFT_TOP,
            &gold_text,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 215, 0),
        );

        // 背包空格
        let space_text = format!("空格: {}", self.bag_space);
        ui.painter().text(
            egui::pos2(info_x, info_y + 35.0),
            egui::Align2::LEFT_TOP,
            &space_text,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }

    /// 绘制功能按钮组
    fn draw_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: &egui::Rect) {
        let button_y = rect.min.y + 76.0;
        let button_start_x = rect.max.x - 120.0;
        let button_spacing = 23.0;

        // 按钮列表：背包、角色、技能、任务、选项
        let buttons = [
            (1903, 1904, 1905, "背包"),
            (1900, 1901, 1902, "角色"),
            (1906, 1907, 1908, "技能"),
            (1909, 1910, 1911, "任务"),
            (1912, 1913, 1914, "选项"),
        ];

        for (i, (normal_idx, hover_idx, pressed_idx, hint)) in buttons.iter().enumerate() {
            let btn_x = button_start_x + (i as f32 * button_spacing);
            self.draw_button(ui, ctx, btn_x, button_y, *normal_idx, *hover_idx, *pressed_idx, hint);
        }

        // 菜单按钮（位置稍上）
        let menu_x = rect.max.x - 55.0;
        let menu_y = rect.min.y + 35.0;
        self.draw_button(ui, ctx, menu_x, menu_y, 1960, 1961, 1962, "菜单");

        // 商城按钮
        let shop_x = rect.max.x - 105.0;
        let shop_y = rect.min.y + 35.0;
        self.draw_button(ui, ctx, shop_x, shop_y, 826, 827, 828, "商城");
    }

    /// 绘制单个按钮
    fn draw_button(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        x: f32,
        y: f32,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        hint: &str,
    ) {
        // 获取按钮纹理
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, normal_idx) {
            if let Some(texture) = info.egui_texture {
                let size = texture.size_vec2();
                let rect = egui::Rect::from_min_size(egui::pos2(x, y), size);
                
                let response = ui.interact(rect, egui::Id::new(format!("btn_{}", normal_idx)), egui::Sense::click());
                
                // 根据状态选择纹理
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                if let Some(btn_info) = LibraryName::Prguse.get_egui_texture(ctx, texture_idx) {
                    if let Some(btn_texture) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 点击事件和鼠标悬停提示
                let clicked = response.clicked();
                response.on_hover_text(hint);
                
                if clicked {
                    println!("🖱️ 点击了 {} 按钮", hint);
                    // 处理按钮点击
                    match hint {
                        "背包" => self.inventory_dialog.toggle(),
                        _ => {}
                    }
                }
            }
        }
    }
    
    /// 显示所有子对话框
    pub fn show_dialogs(&mut self, ctx: &egui::Context) {
        // 获取屏幕尺寸
        let screen_h = macroquad::prelude::screen_height() / macroquad::prelude::screen_dpi_scale();
        
        // 每个对话框使用独立的 open 变量，避免互相影响
        let mut chat_open = true;
        let mut control_bar_open = true;
        let mut belt_open = true;
        let mut inventory_open = self.inventory_dialog.is_visible(); // 使用背包对话框的实际可见状态
        
        // 先显示 ChatDialog（在最底层）
        self.chat_dialog.show(ctx, &mut chat_open);
        
        // 再显示 ChatControlBar（在中间层）
        let (size_clicked, _settings_clicked) = self.chat_control_bar.show(ctx, &mut control_bar_open);
        
        // 显示 BeltDialog（在最上层，不被其他组件遮挡）
        self.belt_dialog.show(ctx, &mut belt_open);
        
        // 显示 InventoryDialog（独立窗口）
        self.inventory_dialog.show(ctx, &mut inventory_open);
        
        // 如果 Size 按钮被点击，改变 ChatDialog 大小
        if size_clicked {
            self.chat_dialog.change_size(screen_h);
            
            // 同步更新 ChatControlBar 位置（保持在 ChatDialog 上方 15px）
            let chat_pos = self.chat_dialog.get_position();
            let control_bar_y = chat_pos.y - 15.0;
            self.chat_control_bar.set_position(egui_macroquad::egui::pos2(chat_pos.x, control_bar_y));
            
            // 同步更新 BeltDialog 位置（紧贴在 ChatControlBar 上方）
            // ChatControlBar 高度为 16px，BeltDialog 高度为 24px
            // BeltDialog 顶部 Y = ChatControlBar 顶部 Y - ChatControlBar 高度 - BeltDialog 高度
            let belt_y = control_bar_y - 16.0 - 24.0;  // control_bar 高度 16 + belt 高度 24
            self.belt_dialog.set_position(egui_macroquad::egui::pos2(chat_pos.x, belt_y));
        }
    }
}

// ============================================================================
// 实现 Dialog trait
// ============================================================================

impl Dialog for MainDialog {
    fn show(&mut self, ctx: &egui::Context, _open: &mut bool) {
        self.show(ctx);
    }
}
