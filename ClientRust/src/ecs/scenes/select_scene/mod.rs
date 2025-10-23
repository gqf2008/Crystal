// SelectScene - Character selection scene
// Mirrors Client/MirScenes/SelectScene.cs

// UI 组件子模块（每个组件负责自己的绘制和事件处理）
pub mod new_character_dialog;
pub mod delete_character_dialog;
pub mod credits_dialog;

// 业务逻辑子模块（按照单一职责原则分离）
mod ui_actions;        // UI 交互逻辑（按钮点击、对话框打开、游戏启动等）
mod network_handler;   // 网络事件处理（服务器响应处理）

pub use new_character_dialog::NewCharacterDialog;
pub use delete_character_dialog::DeleteCharacterDialog;
pub use credits_dialog::CreditsDialog;

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use hecs::World;
use tokio::sync::mpsc;

use super::{Scene, SceneType};
use crate::network::NetworkCommand;
use crate::ecs::ui::{ButtonGroup, ButtonWidget};  // 🆕 按钮部件
use mir2_shared::SelectInfo;

// 设计分辨率常量 (与 LoginScene 保持一致)
const DESIGN_WIDTH: f32 = 1024.0;
const DESIGN_HEIGHT: f32 = 768.0;

/// 
/// Mirrors C# SelectScene:
/// ```csharp
/// public class SelectScene : MirScene
/// {
///     public List<SelectInfo> Characters = new List<SelectInfo>();
///     private int _selected;
///     private NewCharacterDialog _character;
///     // ... UI controls
/// }
/// ```
pub struct SelectScene {
    // Character list (mirrors C# Characters)
    /// Mirrors C# `public List<SelectInfo> Characters`
    pub characters: Vec<SelectInfo>,
    
    /// Mirrors C# `private int _selected`
    pub selected_index: i32,
    
    // Dialogs
    /// Mirrors C# `private NewCharacterDialog _character`
    pub new_character_dialog: Option<NewCharacterDialog>,
    /// Delete character dialog
    pub delete_character_dialog: Option<DeleteCharacterDialog>,
    /// Credits dialog
    pub credits_dialog: Option<CreditsDialog>,
    
    // Network
    command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
    
    // Scene transition
    pub pending_scene_change: Option<SceneType>,
    
    // 🆕 按钮组管理器(替代手动状态管理)
    bottom_buttons: ButtonGroup,
    
    // UI state (保留用于对话框)
    hovered_button: Option<BottomButton>,  // TODO: 可以删除,由 ButtonGroup 管理
    pressed_button: Option<BottomButton>,  // TODO: 可以删除,由 ButtonGroup 管理
    
    // Character preview animation
    character_animation_frame: usize,
    character_animation_timer: f32,
    
    // Window dimensions
    window_width: f32,
    window_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomButton {
    StartGame,
    NewCharacter,
    DeleteCharacter,
    Credits,
    ExitGame,
}

impl SelectScene {
    /// Create new select scene with character list
    /// 
    /// Mirrors C# constructor:
    /// ```csharp
    /// public SelectScene(List<SelectInfo> characters)
    /// {
    ///     Characters = characters;
    ///     SortList();
    ///     // ... initialize UI
    /// }
    /// ```
    pub fn new(characters: Vec<SelectInfo>) -> Self {
        // 创建底部按钮组
        let mut bottom_buttons = ButtonGroup::new();
        
        // 按钮布局常量
        const BUTTON_Y: f32 = 736.0;
        const BUTTON_WIDTH: f32 = 96.0;
        const BUTTON_HEIGHT: f32 = 32.0;
        const BUTTON_SPACING: f32 = 150.0;
        const BUTTON_START_X: f32 = 100.0;
        
        // 添加5个底部按钮 (使用 Builder 模式添加工具提示)
        bottom_buttons.add(
            ButtonWidget::new(1, BUTTON_START_X, BUTTON_Y, BUTTON_WIDTH, BUTTON_HEIGHT, 340)
                .with_tooltip("开始游戏 (Enter)")
        );
        bottom_buttons.add(
            ButtonWidget::new(2, BUTTON_START_X + BUTTON_SPACING, BUTTON_Y, BUTTON_WIDTH, BUTTON_HEIGHT, 343)
                .with_tooltip("新建角色")
        );
        bottom_buttons.add(
            ButtonWidget::new(3, BUTTON_START_X + BUTTON_SPACING * 2.0, BUTTON_Y, BUTTON_WIDTH, BUTTON_HEIGHT, 346)
                .with_tooltip("删除角色 (Delete)")
        );
        bottom_buttons.add(
            ButtonWidget::new(4, BUTTON_START_X + BUTTON_SPACING * 3.0, BUTTON_Y, BUTTON_WIDTH, BUTTON_HEIGHT, 349)
                .with_tooltip("制作人员")
        );
        bottom_buttons.add(
            ButtonWidget::new(5, BUTTON_START_X + BUTTON_SPACING * 4.0, BUTTON_Y, BUTTON_WIDTH, BUTTON_HEIGHT, 352)
                .with_tooltip("退出游戏 (ESC)")
        );
        
        let mut scene = Self {
            characters,
            selected_index: 0,
            new_character_dialog: None,
            delete_character_dialog: None,
            credits_dialog: None,
            command_tx: None,
            pending_scene_change: None,
            bottom_buttons,  // 🆕 使用 ButtonGroup
            hovered_button: None,
            pressed_button: None,
            character_animation_frame: 0,
            character_animation_timer: 0.0,
            window_width: 1024.0,
            window_height: 768.0,
        };
        scene.sort_list();
        scene
    }
    
    /// 设置网络命令发送器
    pub fn set_command_sender(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>) {
        self.command_tx = Some(tx);
    }
    
    /// Sort character list by last access time
    /// 
    /// Mirrors C# SortList():
    /// ```csharp
    /// public void SortList()
    /// {
    ///     if (Characters != null)
    ///         Characters.Sort((c1, c2) => c2.LastAccess.CompareTo(c1.LastAccess));
    /// }
    /// ```
    fn sort_list(&mut self) {
        self.characters.sort_by(|a, b| b.last_access.cmp(&a.last_access));
    }
    
    /// Select character by index
    /// 
    /// Mirrors C# CharacterButton click handler:
    /// ```csharp
    /// _selected = index;
    /// UpdateInterface();
    /// ```
    pub fn select_character(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.characters.len() {
            self.selected_index = index;
            println!("Selected character: {}", self.characters[index as usize].name);
            // TODO: UpdateInterface() - update UI display
        }
    }
    
    /// 将窗口坐标转换为设计坐标 (与 LoginScene 保持一致)
    /// 
    /// 窗口可能是任意大小，但我们使用固定的 1024x768 设计坐标系。
    /// 这个方法将鼠标的窗口坐标转换为设计坐标，考虑4:3比例和居中偏移。
    fn window_to_design_coords(&self, ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let window_size = ctx.gfx.drawable_size();
        let window_width = window_size.0;
        let window_height = window_size.1;
        
        // 计算4:3视口
        let aspect_ratio = 4.0 / 3.0;
        let current_ratio = window_width / window_height;
        
        let (viewport_width, viewport_height) = if current_ratio > aspect_ratio {
            (window_height * aspect_ratio, window_height)
        } else {
            (window_width, window_width / aspect_ratio)
        };
        
        let offset_x = (window_width - viewport_width) / 2.0;
        let offset_y = (window_height - viewport_height) / 2.0;
        
        // 转换：窗口坐标 -> 视口坐标 -> 设计坐标
        let viewport_x = window_x - offset_x;
        let viewport_y = window_y - offset_y;
        
        let design_x = (viewport_x / viewport_width) * DESIGN_WIDTH;
        let design_y = (viewport_y / viewport_height) * DESIGN_HEIGHT;
        
        (design_x, design_y)
    }
}

// ========================================================================
// 死代码已删除：以下方法从未被调用，违反了单一职责原则
// - draw_new_character_dialog() (~227行)
// - draw_delete_character_dialog() (~45行)
// - draw_delete_confirmation() (~26行)
// - draw_delete_name_input() (~96行)
//
// 这些方法引用了已删除的字段（self.window_width, self.window_height）
// 对话框的绘制应该在各自的组件内部实现（new_character_dialog.rs 等）
// 
// UI操作方法已移至专门模块：
// - handle_button_click() → ui_actions.rs
// - handle_dialog_button_click() → ui_actions.rs
// - start_game(), open_*_dialog() 等 → ui_actions.rs
// ========================================================================

impl Default for SelectScene {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Scene for SelectScene {
    fn update(
        &mut self, 
        ctx: &mut Context, 
        _world: &mut World,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>
    ) -> GameResult<Option<SceneType>> {
        // 更新 NewCharacterDialog 动画和计时器
        if let Some(dialog) = &mut self.new_character_dialog {
            dialog.update(ctx.time.delta().as_secs_f32());
        }
        
        // 更新角色预览动画 (16帧, 250ms/帧 = 4 FPS)
        self.character_animation_timer += ctx.time.delta().as_secs_f32();
        if self.character_animation_timer >= 0.25 {
            self.character_animation_timer -= 0.25;
            let old_frame = self.character_animation_frame;
            self.character_animation_frame = (self.character_animation_frame + 1) % 16;
            
            // 调试：监控帧15→0的循环重启
            if old_frame == 15 && self.character_animation_frame == 0 {
                tracing::debug!("Animation loop restart: frame 15 -> 0");
            }
        }
        
        // 检查场景切换
        if let Some(scene_type) = self.pending_scene_change.take() {
            return Ok(Some(scene_type));
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, _world: &World) -> GameResult {
        use ggez::graphics::{DrawParam, Color, PxScale, Text, Rect, Mesh, DrawMode};
        use crate::graphics::libraries::{get_library, LibraryName};
        
        // 设置画布使用设计分辨率坐标系（1024x768）
        // ggez会自动缩放到窗口大小，保持4:3比例
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(0.0, 0.0, DESIGN_WIDTH, DESIGN_HEIGHT));
        
        // 1. 绘制背景 Prguse_65 (在设计坐标系中直接铺满)
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 65, 0.0, 0.0, Color::WHITE, false);
            }
        }
        
        // 2. 绘制标题 Title_40 (C#位置: 468, 20)
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 40, 468.0, 20.0, Color::WHITE, false);
            }
        }
        
        // 2.5 绘制服务器标签 (C#位置: 432, 60, Size: 155x17, 水平居中)
        // C#: DrawFormat = TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter
        // 文本在155像素宽区域内居中，中心点 = 432 + 155/2 = 509.5
        let mut server_label = Text::new("Legend of Mir 2");
        server_label.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(14.0));
        
        // 测量文本宽度以实现居中对齐
        let text_width = server_label.measure(ctx).map(|r| r.x).unwrap_or(120.0);
        let center_x = 432.0 + 155.0 / 2.0;  // 区域中心点
        let x = center_x - text_width / 2.0;  // 文本起始点
        
        canvas.draw(&server_label, DrawParam::default()
            .dest([x, 60.0])  // Y坐标使用C#原始值60
            .color(Color::from_rgb(200, 200, 200)));  // 浅灰色
        
        // 3. 绘制角色槽位和角色信息 (右侧垂直布局)
        // C# 代码中的原始位置: (637, 194), (637, 298), (637, 402), (637, 506)
        let character_button_positions = [
            (637.0, 194.0),
            (637.0, 298.0),
            (637.0, 402.0),
            (637.0, 506.0),
        ];
        
        for (i, character) in self.characters.iter().enumerate() {
            if i >= 4 { break; }  // 最多4个角色
            
            let (slot_x, slot_y) = character_button_positions[i];
            
            // 绘制槽位背景 (选中/未选中)
            let slot_index = if self.selected_index == i as i32 {
                665 + (character.class as i32)  // 选中状态: 665-669
            } else {
                660 + (character.class as i32)  // 未选中状态: 660-664
            };
            
            if let Some(lib_arc) = get_library(LibraryName::Title) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    let _ = lib.draw_with_color(ctx, canvas, slot_index as usize, slot_x, slot_y, Color::WHITE, false);
                }
            }
            
            // 绘制角色名称和等级信息 (使用文本)
            // C# NameLabel: Location = (107, 9), Size = (170, 18)
            let mut name_text = Text::new(&character.name);
            name_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(14.0));
            canvas.draw(&name_text, DrawParam::default()
                .dest([slot_x + 107.0, slot_y + 9.0])  // C#原始相对位置
                .color(Color::WHITE));
            
            // C# LevelLabel: Location = (107, 28), Size = (30, 18)
            let mut level_text = Text::new(format!("Lv.{}", character.level));
            level_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            canvas.draw(&level_text, DrawParam::default()
                .dest([slot_x + 107.0, slot_y + 28.0])  // C#原始相对位置
                .color(Color::from_rgb(200, 200, 200)));
            
            // C# ClassLabel: Location = (178, 28), Size = (100, 18)
            let class_name = match character.class {
                mir2_shared::enums::MirClass::Warrior => "Warrior",
                mir2_shared::enums::MirClass::Wizard => "Wizard",
                mir2_shared::enums::MirClass::Taoist => "Taoist",
                mir2_shared::enums::MirClass::Assassin => "Assassin",
                mir2_shared::enums::MirClass::Archer => "Archer",
            };
            let mut class_text = Text::new(class_name);
            class_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            canvas.draw(&class_text, DrawParam::default()
                .dest([slot_x + 178.0, slot_y + 28.0])  // C#原始相对位置
                .color(Color::from_rgb(200, 200, 200)));
        }
        
        // 4. 绘制空槽位 (如果角色少于4个)
        for i in self.characters.len()..4 {
            let (slot_x, slot_y) = character_button_positions[i];
            
            if let Some(lib_arc) = get_library(LibraryName::Prguse) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    let _ = lib.draw_with_color(ctx, canvas, 44, slot_x, slot_y, Color::WHITE, false);
                }
            }
        }
        
        // 5. 绘制选中角色的预览动画 (左侧中央)
        // C# CharacterDisplay: Location = new Point(260, 420)
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            let character = &self.characters[self.selected_index as usize];
            
            // 获取角色动画基础索引
            let base_index = match (character.class, character.gender) {
                (mir2_shared::enums::MirClass::Warrior, mir2_shared::enums::MirGender::Male) => 20,
                (mir2_shared::enums::MirClass::Warrior, mir2_shared::enums::MirGender::Female) => 300,
                (mir2_shared::enums::MirClass::Wizard, mir2_shared::enums::MirGender::Male) => 40,
                (mir2_shared::enums::MirClass::Wizard, mir2_shared::enums::MirGender::Female) => 320,
                (mir2_shared::enums::MirClass::Taoist, mir2_shared::enums::MirGender::Male) => 60,
                (mir2_shared::enums::MirClass::Taoist, mir2_shared::enums::MirGender::Female) => 340,
                (mir2_shared::enums::MirClass::Assassin, mir2_shared::enums::MirGender::Male) => 80,
                (mir2_shared::enums::MirClass::Assassin, mir2_shared::enums::MirGender::Female) => 360,
                (mir2_shared::enums::MirClass::Archer, mir2_shared::enums::MirGender::Male) => 100,
                (mir2_shared::enums::MirClass::Archer, mir2_shared::enums::MirGender::Female) => 140,  // C# 确认使用 140
            };
            
            let anim_index = base_index + self.character_animation_frame as i32;
            let anim_key = format!("ChrSel_{}", anim_index);
            
            // 角色预览位置（左侧中央，C#原始坐标）
            let preview_x = 260.0;
            let preview_y = 420.0;
            
            // 使用新的库系统绘制角色预览
            if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
                if let Ok(mut lib) = lib_arc.try_lock() {
                    // draw_with_color 最后一个参数 use_offset=true 会自动应用偏移量
                    let _ = lib.draw_with_color(ctx, canvas, anim_index as usize, preview_x, preview_y, Color::WHITE, true);
                    
                    // 法师需要绘制混合效果（C# AfterDraw事件）
                    if character.class == mir2_shared::enums::MirClass::Wizard {
                        let blend_index = (anim_index + 560) as usize;
                        let _ = lib.draw_with_color(ctx, canvas, blend_index, preview_x, preview_y, Color::from_rgba(255, 255, 255, 180), true);
                    }
                }
            }
            
            // C# LastAccessLabel: Location = (265, 609), Size = (180, 21)
            // C# LastAccessLabelLabel: Location = (-65, 0), Parent = LastAccessLabel, Text = "Last Online:"
            // 绘制"Last Online:"标签
            let mut last_online_label = Text::new("Last Online:");
            last_online_label.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            
            // 测量标签宽度以准确计算时间位置
            let label_width = last_online_label.measure(ctx).map(|r| r.x).unwrap_or(85.0);
            
            canvas.draw(&last_online_label, DrawParam::default()
                .dest([200.0, 615.0])  // C#原始Y位置609向上调整到615
                .color(Color::from_rgb(200, 200, 200)));
            
            // 绘制最后登录时间值（紧跟在标签之后，间距5像素）
            // C#: LastAccessLabel.Text = Characters[_selected].LastAccess == DateTime.MinValue ? "Never" : Characters[_selected].LastAccess.ToString();
            // 格式化时间为短格式 (例如: "2025-10-07 14:30" 而不是完整ISO格式)
            let last_access = character.last_access.format("%Y-%m-%d %H:%M").to_string();
            let mut last_access_text = Text::new(&last_access);
            last_access_text.set_font("AlibabaPuHuiTi")
                .set_scale(PxScale::from(13.0));
            canvas.draw(&last_access_text, DrawParam::default()
                .dest([200.0 + label_width + 5.0, 615.0])  // 标签宽度 + 5像素间距
                .color(Color::WHITE));
        }
        
        // 6. 绘制底部按钮 (使用 ButtonGroup 自动状态管理)
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                for button in &self.bottom_buttons.buttons {
                    let texture_index = button.get_texture_index();
                    let color = button.get_color();
                    let _ = lib.draw_with_color(
                        ctx,
                        canvas,
                        texture_index as usize,
                        button.x,
                        button.y,
                        color,
                        false
                    );
                }
            }
        }
        
        // 7. 绘制工具提示 (悬停在按钮上时)
        for button in &self.bottom_buttons.buttons {
            if let Some(tooltip_text) = button.get_tooltip() {
                let mut tooltip = Text::new(tooltip_text);
                tooltip.set_font("AlibabaPuHuiTi");
                tooltip.set_scale(14.0);
                
                // 计算提示框位置 (按钮上方)
                let tooltip_x = button.x;
                let tooltip_y = button.y - 25.0;
                
                // 绘制半透明背景
                let text_bounds = tooltip.measure(ctx).unwrap_or(ggez::glam::Vec2::new(100.0, 20.0).into());
                let bg_rect = Rect::new(
                    tooltip_x - 5.0,
                    tooltip_y - 5.0,
                    text_bounds.x + 10.0,
                    text_bounds.y + 10.0
                );
                
                if let Ok(mesh) = Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    bg_rect,
                    Color::from_rgba(0, 0, 0, 200)
                ) {
                    canvas.draw(&mesh, DrawParam::default());
                }
                
                // 绘制提示文字
                canvas.draw(&tooltip, DrawParam::default()
                    .dest([tooltip_x, tooltip_y])
                    .color(Color::from_rgb(255, 255, 200)));
                
                break; // 只显示一个提示
            }
        }
        
        // TODO: 8. 绘制 NewCharacterDialog (最上层)
        // 暂时禁用对话框绘制，等待完整迁移到新的库系统
        
        // TODO: 9. 绘制 DeleteCharacterDialog (最上层)
        // 暂时禁用对话框绘制
        
        // TODO: 10. 绘制 CreditsDialog (最上层)
        // 暂时禁用对话框绘制
        
        Ok(())
    }
    
    fn on_mouse_down(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        use ggez::winit::event::MouseButton;
        
        // 🔧 转换窗口坐标为设计坐标
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);
        
        // 调试：输出转换后的坐标
        tracing::debug!("🖱️ SelectScene 鼠标点击: 窗口({:.0}, {:.0}) -> 设计({:.0}, {:.0}), 按钮: {:?}", 
            x, y, design_x, design_y, button);
        
        // 只处理左键点击
        if button != MouseButton::Left {
            return Ok(());
        }
        
        // 1. 处理对话框点击（优先级最高）
        if let Some(_dialog) = &mut self.new_character_dialog {
            // TODO: 实现 NewCharacterDialog 的点击处理（使用设计坐标）
            // if dialog.handle_mouse_click(design_x, design_y, true) {
            //     return Ok(()); // 对话框消费了点击事件
            // }
        }
        
        if let Some(_dialog) = &mut self.delete_character_dialog {
            // TODO: 实现 DeleteCharacterDialog 的点击处理
        }
        
        if let Some(_dialog) = &mut self.credits_dialog {
            // TODO: 实现 CreditsDialog 的点击处理
        }
        
        // 2. 处理角色槽位点击 (右侧垂直布局) - 使用设计坐标
        // C# 代码中的原始位置: (637, 194), (637, 298), (637, 402), (637, 506)
        // 槽位大小约 80x80
        let character_button_positions = [
            (637.0, 194.0),
            (637.0, 298.0),
            (637.0, 402.0),
            (637.0, 506.0),
        ];
        
        for (i, &(slot_x, slot_y)) in character_button_positions.iter().enumerate() {
            if i >= self.characters.len() {
                break; // 没有更多角色
            }
            
            // 检查点击是否在槽位范围内 (80x80) - 使用设计坐标
            if design_x >= slot_x && design_x <= slot_x + 80.0 &&
               design_y >= slot_y && design_y <= slot_y + 80.0 {
                // 点击了角色槽位
                self.select_character(i as i32);
                tracing::info!("🖱️ 选中角色: {}", self.characters[i].name);
                return Ok(());
            }
        }
        
        // 3. 处理底部按钮点击 (使用 ButtonGroup) - 使用设计坐标
        if let Some(button_id) = self.bottom_buttons.on_mouse_down(design_x, design_y) {
            tracing::info!("🖱️ 点击按钮 ID: {} at 设计({:.0}, {:.0})", button_id, design_x, design_y);
            
            // 根据按钮ID分发事件
            match button_id {
                1 => self.handle_button_click(BottomButton::StartGame, network_tx),
                2 => self.handle_button_click(BottomButton::NewCharacter, network_tx),
                3 => self.handle_button_click(BottomButton::DeleteCharacter, network_tx),
                4 => self.handle_button_click(BottomButton::Credits, network_tx),
                5 => self.handle_button_click(BottomButton::ExitGame, network_tx),
                _ => {}
            }
            return Ok(());
        }
        
        Ok(())
    }
    
    fn on_mouse_up(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        _button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // 🔧 转换窗口坐标为设计坐标
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);
        
        // 使用 ButtonGroup 处理释放 (如果需要触发点击) - 使用设计坐标
        if let Some(button_id) = self.bottom_buttons.on_mouse_up(design_x, design_y) {
            tracing::info!("✅ 按钮点击完成: {}", button_id);
            // 按下和释放都在同一按钮内才触发(已在 on_mouse_down 处理)
        }
        
        // 清除旧的按下状态 (兼容性)
        self.pressed_button = None;
        Ok(())
    }
    
    fn on_mouse_move(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 🔧 转换窗口坐标为设计坐标
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);
        
        // 处理对话框鼠标移动 - 使用设计坐标
        if let Some(_dialog) = &mut self.new_character_dialog {
            // TODO: 实现对话框拖拽（使用设计坐标）
            // dialog.handle_mouse_drag(design_x, design_y);
        }
        
        // 使用 ButtonGroup 自动更新悬停状态 - 使用设计坐标
        self.bottom_buttons.update_hover(design_x, design_y);
        
        // 兼容旧代码:同步 hovered_button 字段 (TODO: 删除)
        self.hovered_button = None;
        for (i, button) in self.bottom_buttons.buttons.iter().enumerate() {
            if button.state == crate::ecs::ui::ButtonState::Hovered || 
               button.state == crate::ecs::ui::ButtonState::Pressed {
                self.hovered_button = Some(match i {
                    0 => BottomButton::StartGame,
                    1 => BottomButton::NewCharacter,
                    2 => BottomButton::DeleteCharacter,
                    3 => BottomButton::Credits,
                    4 => BottomButton::ExitGame,
                    _ => continue,
                });
                break;
            }
        }
        
        Ok(())
    }
    
    fn on_key_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        input: ggez::input::keyboard::KeyInput,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        use ggez::winit::keyboard::KeyCode;
        
        // 检查物理键码
        if let ggez::winit::event::KeyEvent {
            physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
            ..
        } = &input.event {
            match keycode {
                KeyCode::Escape => {
                    // ESC 键关闭对话框或退出
                    if self.new_character_dialog.is_some() {
                        self.new_character_dialog = None;
                        tracing::info!("❌ 关闭新建角色对话框");
                    } else if self.delete_character_dialog.is_some() {
                        self.delete_character_dialog = None;
                        tracing::info!("❌ 关闭删除角色对话框");
                    } else if self.credits_dialog.is_some() {
                        self.credits_dialog = None;
                        tracing::info!("❌ 关闭制作人员对话框");
                    } else {
                        // TODO: 返回登录场景或退出游戏
                        tracing::info!("❌ ESC 键: 退出角色选择");
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    // Enter 键开始游戏
                    if !self.characters.is_empty() && self.selected_index >= 0 {
                        self.handle_button_click(BottomButton::StartGame, network_tx);
                    }
                }
                KeyCode::Digit1 | KeyCode::Numpad1 => {
                    if !self.characters.is_empty() {
                        self.select_character(0);
                        tracing::info!("⌨️ 键盘选中角色1: {}", self.characters[0].name);
                    }
                }
                KeyCode::Digit2 | KeyCode::Numpad2 => {
                    if self.characters.len() > 1 {
                        self.select_character(1);
                        tracing::info!("⌨️ 键盘选中角色2: {}", self.characters[1].name);
                    }
                }
                KeyCode::Digit3 | KeyCode::Numpad3 => {
                    if self.characters.len() > 2 {
                        self.select_character(2);
                        tracing::info!("⌨️ 键盘选中角色3: {}", self.characters[2].name);
                    }
                }
                KeyCode::Digit4 | KeyCode::Numpad4 => {
                    if self.characters.len() > 3 {
                        self.select_character(3);
                        tracing::info!("⌨️ 键盘选中角色4: {}", self.characters[3].name);
                    }
                }
                _ => {}
            }
        }
        
        Ok(None)
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ========================================================================
// 所有辅助方法已移至专门的模块：
// - handle_button_click() → ui_actions.rs
// - handle_dialog_button_click() → ui_actions.rs  
// - start_game(), open_*_dialog() 等 → ui_actions.rs
// - handle_network_event() → network_handler.rs
// 
// mod.rs 现在只保留：
// - 核心数据结构 (SelectScene)
// - Scene trait 实现 (update, draw, on_*)
// - 基本状态管理方法 (select_character, set_command_sender 等)
// ========================================================================
