use crate::resources::libraries::LibraryName;
use crate::scenes::dialogs::Dialog;
use egui_macroquad::egui;
use macroquad::prelude::*;

/// 新建角色对话框事件
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NewCharacterEvent {
    None,
    Create,
    Cancel,
}

/// 新建角色对话框
pub struct NewCharacterDialog {
    id: String,
    pub name: String,
    pub class: u8,  // 0=Warrior, 1=Wizard, 2=Taoist, 3=Assassin, 4=Archer
    pub gender: u8, // 0=Male, 1=Female
    last_event: NewCharacterEvent,
    
    // 角色预览动画
    animation_frame: usize,
    animation_timer: f32,
}

impl NewCharacterDialog {
    pub fn new() -> Self {
        Self {
            id: "new_character_dialog".to_string(),
            name: String::new(),
            class: 0,
            gender: 0,
            last_event: NewCharacterEvent::None,
            animation_frame: 0,
            animation_timer: 0.0,
        }
    }
    
    /// 重置对话框
    pub fn reset(&mut self) {
        self.name.clear();
        self.class = 0;
        self.gender = 0;
        self.last_event = NewCharacterEvent::None;
        self.animation_frame = 0;
        self.animation_timer = 0.0;
    }
    
    /// 获取并清除最后的事件
    pub fn take_event(&mut self) -> NewCharacterEvent {
        let event = self.last_event;
        self.last_event = NewCharacterEvent::None;
        event
    }
    
    /// 更新动画帧（16帧循环，250ms/帧）
    pub fn update(&mut self, dt: f32) {
        self.animation_timer += dt;
        if self.animation_timer >= 0.25 {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 16;
        }
    }
    
    /// 绘制图像按钮
    fn draw_image_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        abs_pos: egui::Pos2,
    ) -> bool {
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, normal_idx) {
            if let Some(ref handle) = info.egui_texture {
                let texture_size = handle.size_vec2();
                let button_rect = egui::Rect::from_min_size(abs_pos, texture_size);
                
                let button_id = format!("{}_{}", self.id, normal_idx);
                let response = ui.interact(button_rect, egui::Id::new(button_id), egui::Sense::click());
                
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                if let Some(btn_info) = LibraryName::Title.get_egui_texture(ctx, texture_idx) {
                    if let Some(ref btn_handle) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_handle.id(),
                            button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                return response.clicked();
            }
        }
        false
    }
    
    /// 绘制 Prguse 库的按钮
    fn draw_prguse_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        abs_pos: egui::Pos2,
    ) -> bool {
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, normal_idx) {
            if let Some(ref handle) = info.egui_texture {
                let texture_size = handle.size_vec2();
                // 添加偏移量
                let button_rect = egui::Rect::from_min_size(
                    egui::pos2(abs_pos.x + info.x as f32, abs_pos.y + info.y as f32),
                    texture_size
                );
                
                let button_id = format!("{}_prguse_{}", self.id, normal_idx);
                let response = ui.interact(button_rect, egui::Id::new(button_id), egui::Sense::click());
                
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                if let Some(btn_info) = LibraryName::Prguse.get_egui_texture(ctx, texture_idx) {
                    if let Some(ref btn_handle) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_handle.id(),
                            button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                return response.clicked();
            }
        }
        false
    }
}

impl Dialog for NewCharacterDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        
        // ESC键关闭
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            return;
        }
        
        // 获取对话框背景尺寸 Prguse[73]
        let (dialog_w, dialog_h) = if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 73) {
            if let Some(ref handle) = info.egui_texture {
                let size = handle.size_vec2();
                (size.x, size.y)
            } else {
                (656.0, 537.0) // 默认尺寸
            }
        } else {
            (656.0, 537.0)
        };
        
        egui::Area::new(egui::Id::new(&self.id))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .interactable(true)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, egui::vec2(dialog_w, dialog_h)),
                    egui::Sense::hover(),
                ).rect;
                
                // 绘制背景 Prguse[73]
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 73) {
                    if let Some(ref handle) = info.egui_texture {
                        ui.painter().image(
                            handle.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 绘制标题 Title[20] at (206, 11)
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 20) {
                    if let Some(ref handle) = info.egui_texture {
                        let size = handle.size_vec2();
                        let title_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                rect.min.x + 206.0 + info.x as f32,
                                rect.min.y + 11.0 + info.y as f32
                            ),
                            size,
                        );
                        ui.painter().image(
                            handle.id(),
                            title_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 绘制角色预览动画
                // 索引计算参考原工程: Index = 20, AnimationCount = 16
                // 大部分职业: base = 20 + class*20 + gender*280
                // 但弓箭手(class=4)使用特殊索引：男100 / 女140
                let base_index: usize = if self.class == 4 {
                    // Archer 特殊处理
                    if self.gender == 0 { 100 } else { 140 }
                } else {
                    // 其他职业使用通用公式
                    20 + (self.class as usize * 20) + (self.gender as usize * 280)
                };
                let anim_index: usize = base_index + (self.animation_frame % 16);
                
                if let Some(info) = LibraryName::ChrSel.get_egui_texture(ctx, anim_index) {
                    if let Some(ref handle) = info.egui_texture {
                        let size = handle.size_vec2();
                        // 角色预览位置: (120, 250) + ImageInfo 偏移量
                        let preview_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                rect.min.x + 120.0 + info.x as f32,
                                rect.min.y + 250.0 + info.y as f32
                            ),
                            size,
                        );
                        ui.painter().image(
                            handle.id(),
                            preview_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        
                        // 法师光效 (index + 560)
                        if self.class == 1 {
                            let glow_index = anim_index + 560;
                            if let Some(glow_info) = LibraryName::ChrSel.get_egui_texture(ctx, glow_index) {
                                if let Some(ref glow_handle) = glow_info.egui_texture {
                                    let glow_size = glow_handle.size_vec2();
                                    let glow_rect = egui::Rect::from_min_size(
                                        egui::pos2(
                                            rect.min.x + 120.0 + glow_info.x as f32,
                                            rect.min.y + 250.0 + glow_info.y as f32
                                        ),
                                        glow_size,
                                    );
                                    ui.painter().image(
                                        glow_handle.id(),
                                        glow_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                                    );
                                }
                            }
                        }
                    }
                }
                
                // 角色名输入框 at (325, 268), size (240, 20)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 325.0, rect.min.y + 268.0),
                        egui::vec2(240.0, 20.0),
                    ),
                    egui::TextEdit::singleline(&mut self.name)
                        .hint_text("请输入角色名称")
                        .desired_width(240.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // 职业描述框 at (279, 70), size (278, 170)
                let description = match self.class {
                    0 => "Warriors are a class of great strength and vitality. They are not easily killed in battle and have the advantage of being able to use a variety of heavy weapons and Armour. Therefore, Warriors favor attacks that are based on melee physical damage. They are weak in ranged attacks, however the variety of equipment that are developed specifically for Warriors complement their weakness in ranged combat.",
                    1 => "Wizards are a class of low strength and stamina, but have the ability to use powerful spells. Their offensive spells are very effective, but because it takes time to cast these spells, they're likely to leave themselves open for enemy's attacks. Therefore, the physically weak wizards must aim to attack their enemies from a safe distance.",
                    2 => "Taoists are well disciplined in the study of Astronomy, Medicine, and others aside from Mu-Gong. Rather then directly engaging the enemies, their specialty lies in assisting their allies with support. Taoists can summon powerful creatures and have a high resistance to magic, and is a class with well balanced offensive and defensive abilities.",
                    3 => "Assassins are members of a secret organization and their history is relatively unknown. They're capable of hiding themselves and performing attacks while being unseen by others, which naturally makes them excellent at making fast kills. It is necessary for them to avoid being in battles with multiple enemies due to their weak vitality and strength.",
                    4 => "Archers are a class of great accuracy and strength, using their powerful skills with bows to deal extraordinary damage from range. Much like wizards, they rely on their keen instincts to dodge oncoming attacks as they tend to leave themselves open to frontal attacks. However, their physical prowess and deadly aim allows them to instil fear into anyone they hit.",
                    _ => "",
                };
                
                let desc_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 279.0, rect.min.y + 70.0),
                    egui::vec2(278.0, 170.0),
                );
                
                let font_id = egui::FontId::proportional(11.0);
                let text_color = egui::Color32::WHITE;
                let wrap_width = 268.0;
                
                let galley = ctx.fonts(|fonts| {
                    fonts.layout(
                        description.to_string(),
                        font_id,
                        text_color,
                        wrap_width,
                    )
                });
                
                ui.painter().galley(
                    desc_rect.min + egui::vec2(5.0, 5.0),
                    galley,
                    text_color,
                );
                
                // 性别选择按钮 (Male: Prguse[2421/2422], Female: Prguse[2423/2424/2425])
                // Male at (323, 343), Female at (373, 343)
                if self.draw_prguse_button(ui, ctx, 2421, 2421, 2422,
                    egui::pos2(rect.min.x + 323.0, rect.min.y + 343.0)) {
                    self.gender = 0;
                }
                
                if self.draw_prguse_button(ui, ctx, 2423, 2424, 2425,
                    egui::pos2(rect.min.x + 373.0, rect.min.y + 343.0)) {
                    self.gender = 1;
                }
                
                // 职业选择按钮 (Warrior/Wizard/Taoist/Assassin/Archer)
                // at (323, 296), (373, 296), (423, 296), (473, 296), (523, 296)
                if self.draw_prguse_button(ui, ctx, 2427, 2427, 2428,
                    egui::pos2(rect.min.x + 323.0, rect.min.y + 296.0)) {
                    self.class = 0; // Warrior
                }
                
                if self.draw_prguse_button(ui, ctx, 2429, 2430, 2431,
                    egui::pos2(rect.min.x + 373.0, rect.min.y + 296.0)) {
                    self.class = 1; // Wizard
                }
                
                if self.draw_prguse_button(ui, ctx, 2432, 2433, 2434,
                    egui::pos2(rect.min.x + 423.0, rect.min.y + 296.0)) {
                    self.class = 2; // Taoist
                }
                
                if self.draw_prguse_button(ui, ctx, 2435, 2436, 2437,
                    egui::pos2(rect.min.x + 473.0, rect.min.y + 296.0)) {
                    self.class = 3; // Assassin
                }
                
                if self.draw_prguse_button(ui, ctx, 2438, 2439, 2440,
                    egui::pos2(rect.min.x + 523.0, rect.min.y + 296.0)) {
                    self.class = 4; // Archer
                }
                
                // OK按钮: Title[360/361/362] at (160, 425)
                if self.draw_image_button(ui, ctx, 360, 361, 362,
                    egui::pos2(rect.min.x + 160.0, rect.min.y + 425.0)) {
                    self.last_event = NewCharacterEvent::Create;
                }
                
                // Cancel按钮: Title[280/281/282] at (425, 425)
                if self.draw_image_button(ui, ctx, 280, 281, 282,
                    egui::pos2(rect.min.x + 425.0, rect.min.y + 425.0)) {
                    self.last_event = NewCharacterEvent::Cancel;
                    *open = false;
                }
            });
    }
}
