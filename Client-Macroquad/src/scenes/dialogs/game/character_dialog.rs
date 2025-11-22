/// 角色对话框 - 包含装备、技能、状态页面
/// 对应原工程 CharacterDialog.cs
/// 
/// 功能：
/// - 角色装备显示和管理
/// - 技能树显示和升级
/// - 角色属性和状态显示
/// - 支持标签页切换

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 角色对话框标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTab {
    Character,  // 角色装备页
    Skills,     // 技能页  
    Status,     // 状态页
}

/// 装备栏位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
    Weapon,     // 武器
    Armor,      // 衣服
    Helmet,     // 头盔
    Torch,      // 照明物
    Necklace,   // 项链
    BraceletL,  // 左手镯
    BraceletR,  // 右手镯
    RingL,      // 左戒指
    RingR,      // 右戒指
    Amulet,     // 护身符
    Belt,       // 腰带
    Boots,      // 鞋子
    Stone,      // 宝石
    Mount,      // 坐骑
}

/// 装备物品数据
#[derive(Debug, Clone, Copy)]
pub struct EquipmentItem {
    pub icon_index: usize,
    pub durability: (u32, u32), // (当前, 最大)
    pub upgraded: bool,         // 是否强化
}

/// 技能数据
#[derive(Debug, Clone)]  
pub struct SkillInfo {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub max_level: u32,
    pub icon_index: usize,
    pub experience: u64,
    pub next_exp: u64,
}

/// 角色对话框
pub struct CharacterDialog {
    position: egui::Pos2,
    
    /// 当前标签页
    active_tab: CharacterTab,
    
    /// 窗口拖拽状态
    dragging: bool,
    drag_offset: egui::Vec2,
    
    /// 装备数据 (14个装备栏位)
    equipment: [Option<EquipmentItem>; 14],
    
    /// 技能数据
    skills: Vec<SkillInfo>,
    
    /// 角色属性
    character_stats: CharacterStats,
}

/// 角色属性数据
#[derive(Debug, Clone)]
pub struct CharacterStats {
    pub level: u32,
    pub experience: u64,
    pub next_exp: u64,
    pub health: (u32, u32),     // (当前, 最大)
    pub mana: (u32, u32),       // (当前, 最大)
    pub dc: (u32, u32),         // 攻击力 (最小, 最大)
    pub mc: (u32, u32),         // 魔法 (最小, 最大)
    pub sc: (u32, u32),         // 道术 (最小, 最大)
    pub ac: (u32, u32),         // 防御 (最小, 最大)
    pub mac: (u32, u32),        // 魔防 (最小, 最大)
    pub accuracy: u32,          // 准确
    pub agility: u32,           // 敏捷
    pub luck: u32,              // 幸运
}

impl CharacterDialog {
    pub fn new() -> Self {
        // 模拟一些装备数据
        let mut equipment = [None; 14];
        equipment[0] = Some(EquipmentItem {
            icon_index: 1,  // 武器
            durability: (80, 100),
            upgraded: true,
        });
        equipment[1] = Some(EquipmentItem {
            icon_index: 20, // 衣服
            durability: (95, 100),
            upgraded: false,
        });
        equipment[2] = Some(EquipmentItem {
            icon_index: 40, // 头盔
            durability: (100, 100),
            upgraded: false,
        });
        
        // 模拟技能数据
        let skills = vec![
            SkillInfo {
                id: 1,
                name: "基本剑术".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 1,
                experience: 0,
                next_exp: 0,
            },
            SkillInfo {
                id: 2,
                name: "攻杀剑术".to_string(),
                level: 2,
                max_level: 3,
                icon_index: 2,
                experience: 150,
                next_exp: 300,
            },
            SkillInfo {
                id: 3,
                name: "刺杀剑术".to_string(),
                level: 1,
                max_level: 3,
                icon_index: 3,
                experience: 50,
                next_exp: 100,
            },
        ];
        
        let character_stats = CharacterStats {
            level: 35,
            experience: 125680,
            next_exp: 150000,
            health: (380, 380),
            mana: (120, 120),
            dc: (15, 25),
            mc: (0, 0),
            sc: (0, 0),
            ac: (8, 15),
            mac: (2, 8),
            accuracy: 12,
            agility: 15,
            luck: 0,
        };
        
        Self {
            position: egui::pos2(100.0, 100.0),
            active_tab: CharacterTab::Character,
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
            equipment,
            skills,
            character_stats,
        }
    }
    
    
    /// 显示角色页
    pub fn show_character_page(&mut self) {
        self.active_tab = CharacterTab::Character;
    }
    
    /// 显示技能页
    pub fn show_skill_page(&mut self) {
        self.active_tab = CharacterTab::Skills;
    }
    
    /// 显示状态页  
    pub fn show_status_page(&mut self) {
        self.active_tab = CharacterTab::Status;
    }
    
    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 角色对话框背景 - 使用不同的纹理索引
        let bg_index = match self.active_tab {
            CharacterTab::Character => 504,  // 角色页背景
            CharacterTab::Skills => 505,     // 技能页背景
            CharacterTab::Status => 506,     // 状态页背景
        };
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, bg_index) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                return bg_rect;
            }
        }
        
        // 降级：绘制默认背景
        let default_rect = egui::Rect::from_min_size(self.position, egui::vec2(300.0, 400.0));
        ui.painter().rect_filled(
            default_rect,
            5.0,
            egui::Color32::from_rgb(40, 40, 45),
        );
        ui.painter().rect_stroke(
            default_rect,
            5.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );
        
        default_rect
    }
    
    /// 绘制标签页按钮
    fn draw_tab_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标签页按钮位置 (基于原工程布局)
        let tab_y = bg_rect.min.y + 30.0;
        let tab_buttons = [
            (CharacterTab::Character, "角色", bg_rect.min.x + 10.0),
            (CharacterTab::Skills, "技能", bg_rect.min.x + 70.0),
            (CharacterTab::Status, "状态", bg_rect.min.x + 130.0),
        ];
        
        for (tab, label, x) in tab_buttons {
            let is_active = self.active_tab == tab;
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(x, tab_y),
                egui::vec2(50.0, 25.0)
            );
            
            let response = ui.interact(button_rect, egui::Id::new(format!("tab_{:?}", tab)), egui::Sense::click());
            
            // 绘制按钮背景
            let bg_color = if is_active {
                egui::Color32::from_rgb(80, 120, 160)
            } else if response.hovered() {
                egui::Color32::from_rgb(60, 60, 70)
            } else {
                egui::Color32::from_rgb(50, 50, 55)
            };
            
            ui.painter().rect_filled(button_rect, 3.0, bg_color);
            ui.painter().rect_stroke(
                button_rect,
                3.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                egui::epaint::StrokeKind::Outside,
            );
            
            // 绘制按钮文字
            ui.painter().text(
                button_rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
            
            if response.clicked() {
                self.active_tab = tab;
                println!("🔄 切换到标签页: {:?}", tab);
            }
        }
    }
    
    /// 绘制装备页内容
    fn draw_character_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 装备栏位布局 (基于原工程精确位置)
        let equipment_slots = [
            (EquipmentSlot::Weapon, egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 80.0)),
            (EquipmentSlot::Armor, egui::pos2(bg_rect.min.x + 60.0, bg_rect.min.y + 80.0)),
            (EquipmentSlot::Helmet, egui::pos2(bg_rect.min.x + 100.0, bg_rect.min.y + 80.0)),
            (EquipmentSlot::Torch, egui::pos2(bg_rect.min.x + 140.0, bg_rect.min.y + 80.0)),
            (EquipmentSlot::Necklace, egui::pos2(bg_rect.min.x + 60.0, bg_rect.min.y + 120.0)),
            (EquipmentSlot::BraceletL, egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 160.0)),
            (EquipmentSlot::BraceletR, egui::pos2(bg_rect.min.x + 140.0, bg_rect.min.y + 160.0)),
            (EquipmentSlot::RingL, egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 200.0)),
            (EquipmentSlot::RingR, egui::pos2(bg_rect.min.x + 140.0, bg_rect.min.y + 200.0)),
        ];
        
        for (slot_index, (_slot_type, pos)) in equipment_slots.iter().enumerate() {
            self.draw_equipment_slot(ui, ctx, slot_index, *pos);
        }
        
        // 绘制角色属性面板
        self.draw_character_stats(ui, bg_rect);
    }
    
    /// 绘制装备栏位
    fn draw_equipment_slot(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, slot_index: usize, pos: egui::Pos2) {
        let slot_size = egui::vec2(32.0, 32.0);
        let slot_rect = egui::Rect::from_min_size(pos, slot_size);
        
        // 绘制栏位背景
        ui.painter().rect_filled(
            slot_rect,
            2.0,
            egui::Color32::from_rgba_premultiplied(40, 40, 40, 200),
        );
        ui.painter().rect_stroke(
            slot_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 绘制装备图标
        if let Some(equipment) = &self.equipment[slot_index] {
            if let Some(info) = LibraryName::Items.get_egui_texture(ctx, equipment.icon_index) {
                if let Some(item_texture) = info.egui_texture {
                    // 居中显示装备图标
                    let img_size = egui::vec2(info.width as f32, info.height as f32);
                    let center_offset = (slot_size - img_size) / 2.0;
                    let img_rect = egui::Rect::from_min_size(pos + center_offset, img_size);
                    
                    ui.painter().image(
                        item_texture.id(),
                        img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
                    // 显示耐久度条
                    if equipment.durability.1 > 0 {
                        let dur_percent = equipment.durability.0 as f32 / equipment.durability.1 as f32;
                        let dur_rect = egui::Rect::from_min_size(
                            egui::pos2(pos.x, pos.y + slot_size.y - 3.0),
                            egui::vec2(slot_size.x * dur_percent, 2.0)
                        );
                        
                        let dur_color = if dur_percent > 0.7 {
                            egui::Color32::GREEN
                        } else if dur_percent > 0.3 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::RED
                        };
                        
                        ui.painter().rect_filled(dur_rect, 0.0, dur_color);
                    }
                }
            }
        }
        
        // 处理栏位交互
        let response = ui.interact(slot_rect, egui::Id::new(format!("equip_slot_{}", slot_index)), egui::Sense::click());
        if response.clicked() {
            println!("🎯 点击装备栏位: {}", slot_index);
        }
    }
    
    /// 绘制角色属性
    fn draw_character_stats(&self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        let stats_x = bg_rect.min.x + 180.0;
        let stats_y = bg_rect.min.y + 80.0;
        let line_height = 18.0;
        
        let stats_text = [
            format!("等级: {}", self.character_stats.level),
            format!("经验: {}/{}", self.character_stats.experience, self.character_stats.next_exp),
            format!("生命: {}/{}", self.character_stats.health.0, self.character_stats.health.1),
            format!("魔法: {}/{}", self.character_stats.mana.0, self.character_stats.mana.1),
            format!("攻击: {}-{}", self.character_stats.dc.0, self.character_stats.dc.1),
            format!("防御: {}-{}", self.character_stats.ac.0, self.character_stats.ac.1),
            format!("魔防: {}-{}", self.character_stats.mac.0, self.character_stats.mac.1),
            format!("准确: {}", self.character_stats.accuracy),
            format!("敏捷: {}", self.character_stats.agility),
            format!("幸运: {}", self.character_stats.luck),
        ];
        
        for (i, text) in stats_text.iter().enumerate() {
            ui.painter().text(
                egui::pos2(stats_x, stats_y + i as f32 * line_height),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
    }
    
    /// 绘制技能页内容
    fn draw_skills_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 技能图标布局 (3x4网格)
        let skill_start_x = bg_rect.min.x + 20.0;
        let skill_start_y = bg_rect.min.y + 80.0;
        let skill_size = 40.0;
        let skill_spacing = 45.0;
        
        for (i, skill) in self.skills.iter().enumerate() {
            let row = i / 3;
            let col = i % 3;
            let pos = egui::pos2(
                skill_start_x + col as f32 * skill_spacing,
                skill_start_y + row as f32 * skill_spacing
            );
            
            // 绘制技能图标占位
            let skill_rect = egui::Rect::from_min_size(pos, egui::vec2(skill_size, skill_size));
            ui.painter().rect_filled(
                skill_rect, 
                3.0, 
                egui::Color32::from_rgb(60, 60, 70)
            );
        }
    }
    
    /// 绘制技能栏位
    fn draw_skill_slot(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, skill: &SkillInfo, pos: egui::Pos2) {
        let slot_size = egui::vec2(40.0, 40.0);
        let slot_rect = egui::Rect::from_min_size(pos, slot_size);
        
        // 绘制技能图标背景
        ui.painter().rect_filled(
            slot_rect,
            3.0,
            egui::Color32::from_rgba_premultiplied(30, 30, 35, 200),
        );
        ui.painter().rect_stroke(
            slot_rect,
            3.0,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 80, 90)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // 绘制技能图标 (使用Magic库)
        if let Some(info) = LibraryName::Magic.get_egui_texture(ctx, skill.icon_index) {
            if let Some(skill_texture) = info.egui_texture {
                let img_size = egui::vec2(32.0, 32.0);
                let center_offset = (slot_size - img_size) / 2.0;
                let img_rect = egui::Rect::from_min_size(pos + center_offset, img_size);
                
                ui.painter().image(
                    skill_texture.id(),
                    img_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 显示技能等级
        ui.painter().text(
            egui::pos2(pos.x + slot_size.x - 5.0, pos.y + 5.0),
            egui::Align2::RIGHT_TOP,
            format!("{}", skill.level),
            egui::FontId::proportional(10.0),
            egui::Color32::YELLOW,
        );
        
        // 处理技能交互
        let response = ui.interact(slot_rect, egui::Id::new(format!("skill_{}", skill.id)), egui::Sense::click());
        let is_clicked = response.clicked();
        if response.hovered() {
            // 显示技能详情tooltip
            let tooltip_text = format!("{}\n等级: {}/{}\n经验: {}/{}", 
                skill.name, skill.level, skill.max_level, skill.experience, skill.next_exp);
            response.on_hover_text(tooltip_text);
        }
        
        if is_clicked {
            println!("🔮 点击技能: {} (等级{})", skill.name, skill.level);
        }
    }
    
    /// 绘制状态页内容
    fn draw_status_page(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 详细的角色属性显示
        let content_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 20.0, bg_rect.min.y + 80.0),
            egui::vec2(bg_rect.width() - 40.0, bg_rect.height() - 120.0)
        );
        
        ui.painter().text(
            egui::pos2(content_rect.min.x, content_rect.min.y),
            egui::Align2::LEFT_TOP,
            "详细状态信息",
            egui::FontId::proportional(16.0),
            egui::Color32::YELLOW,
        );
        
        // 这里可以添加更详细的状态信息显示
        let detailed_info = format!(
            "角色等级: {}\n经验值: {}/{}\n\n属性点分配:\n力量: 0\n敏捷: 0\n体力: 0\n精神: 0",
            self.character_stats.level,
            self.character_stats.experience,
            self.character_stats.next_exp
        );
        
        ui.painter().text(
            egui::pos2(content_rect.min.x, content_rect.min.y + 30.0),
            egui::Align2::LEFT_TOP,
            detailed_info,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }
    
    /// 绘制关闭按钮
    fn draw_close_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        // 关闭按钮位置 (右上角)
        let close_pos = egui::pos2(bg_rect.max.x - 25.0, bg_rect.min.y + 5.0);
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 360) { // 关闭按钮纹理
            if let Some(close_texture) = info.egui_texture {
                let close_size = close_texture.size_vec2();
                let close_rect = egui::Rect::from_min_size(close_pos, close_size);
                
                let response = ui.interact(close_rect, egui::Id::new("character_close"), egui::Sense::click());
                
                let texture_id = if response.is_pointer_button_down_on() {
                    // 按下状态 - 使用按下纹理
                    LibraryName::Prguse.get_egui_texture(ctx, 362)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(close_texture.id())
                } else if response.hovered() {
                    // 悬停状态 - 使用悬停纹理
                    LibraryName::Prguse.get_egui_texture(ctx, 361)
                        .and_then(|info| info.egui_texture)
                        .map(|t| t.id())
                        .unwrap_or(close_texture.id())
                } else {
                    close_texture.id()
                };
                
                ui.painter().image(
                    texture_id,
                    close_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                let is_clicked = response.clicked();
                if response.hovered() {
                    response.on_hover_text("关闭");
                }
                
                return is_clicked;
            }
        }
        
        false
    }
    
    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标题栏区域作为拖拽区域
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(bg_rect.width(), 30.0),
        );
        
        let drag_response = ui.interact(title_area, egui::Id::new("character_drag"), egui::Sense::drag());
        
        if drag_response.drag_started() {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.dragging {
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.position = (pointer_pos.to_vec2() + self.drag_offset).to_pos2();
            }
        }
        
        if drag_response.drag_stopped() {
            self.dragging = false;
        }
    }
    
    /// 设置当前标签页
    pub fn set_current_tab(&mut self, tab_index: usize) {
        match tab_index {
            0 => self.active_tab = CharacterTab::Character,
            1 => self.active_tab = CharacterTab::Skills,
            2 => self.active_tab = CharacterTab::Status,
            _ => {}
        }
    }
}

impl Dialog for CharacterDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        
        // 使用 Area 创建自由浮动窗口
        egui::Area::new(egui::Id::new("character_dialog"))
            .fixed_pos(self.position)
            .movable(false)  // 使用自定义拖拽
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制标签页按钮
                self.draw_tab_buttons(ui, ctx, &bg_rect);
                
                // 根据当前标签页绘制内容
                match self.active_tab {
                    CharacterTab::Character => self.draw_character_page(ui, ctx, &bg_rect),
                    CharacterTab::Skills => self.draw_skills_page(ui, ctx, &bg_rect),
                    CharacterTab::Status => self.draw_status_page(ui, ctx, &bg_rect),
                }
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
                    *open = false;
                }
            });
    }
}