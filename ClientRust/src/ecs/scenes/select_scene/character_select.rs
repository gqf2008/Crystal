// character_select.rs - 角色选择主界面 UI 组件
// 负责角色列表显示、角色预览动画、用户交互等

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, DrawParam, Color, PxScale, Text};
use mir2_shared::{SelectInfo, MirClass};
use crate::graphics::libraries::{get_library, LibraryName};
use crate::ecs::ui::ButtonGroup;


/// 角色选择主界面 UI 组件
/// 
/// ⚠️ **架构权衡**: 
/// - 理想ECS: SelectScene应该直接从World查询角色数据来绘制
/// - 当前实现: 缓存Vec<SelectInfo>以简化渲染逻辑
/// - **World是权威数据源**, 这里只是从World加载的只读缓存
/// - TODO: 重构成纯UI状态 + draw时传入角色数据
pub struct CharacterSelect {
    /// 🔄 从World加载的角色数据缓存(只读)
    characters: Vec<SelectInfo>,
    
    /// 当前选中的角色索引 (-1 表示未选中)
    selected_index: i32,
    
    /// 角色预览动画帧索引 (0-15 循环)
    animation_frame: usize,
    
    /// 动画计时器
    animation_timer: f32,
    
    /// 动画帧间隔 (秒)
    animation_interval: f32,
}

impl CharacterSelect {
    /// 创建空的角色选择组件
    /// 🆕 ECS架构: 初始为空,SelectScene.update()会从World加载数据
    pub fn new() -> Self {
        tracing::info!("🎭 创建CharacterSelect (将从World加载数据)");
        Self {
            characters: Vec::new(),
            selected_index: -1,
            animation_frame: 0,
            animation_timer: 0.0,
            animation_interval: 0.1,  // 100ms 每帧
        }
    }
    
    /// 更新动画状态
    pub fn update(&mut self, delta: f32) {
        self.animation_timer += delta;
        if self.animation_timer >= self.animation_interval {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 16;
            
            if self.animation_frame == 0 {
                tracing::debug!("Animation loop restart: frame 15 -> 0");
            }
        }
    }
    
    /// 设置选中的角色
    pub fn select_character(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.characters.len() {
            self.selected_index = index;
        }
    }
    
    /// 获取当前选中的角色
    pub fn get_selected_character(&self) -> Option<&SelectInfo> {
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            Some(&self.characters[self.selected_index as usize])
        } else {
            None
        }
    }
    
    /// 获取角色列表的引用
    pub fn get_characters(&self) -> &[SelectInfo] {
        &self.characters
    }
    
    /// 获取选中索引
    pub fn get_selected_index(&self) -> i32 {
        self.selected_index
    }
    
    /// 检查点击是否在角色槽位上
    pub fn check_slot_click(&self, x: f32, y: f32) -> Option<usize> {
        Self::check_character_slot_click(x, y, self.characters.len())
    }
    
    /// 添加新角色到列表开头
    pub fn add_character(&mut self, character: SelectInfo) {
        self.characters.insert(0, character);
    }
    
    /// 清空角色列表
    pub fn clear_characters(&mut self) {
        self.characters.clear();
        self.selected_index = -1;
    }
    
    /// 检查角色列表是否为空
    pub fn is_empty(&self) -> bool {
        self.characters.is_empty()
    }
    
    /// 获取角色列表长度
    pub fn len(&self) -> usize {
        self.characters.len()
    }
    
    /// 根据角色索引删除角色
    pub fn remove_character_by_index(&mut self, character_index: i32) -> bool {
        if let Some(pos) = self.characters.iter().position(|c| c.index == character_index) {
            self.characters.remove(pos);
            
            // 调整选中索引
            if self.selected_index >= self.characters.len() as i32 {
                self.selected_index = if self.characters.is_empty() {
                    -1
                } else {
                    (self.characters.len() - 1) as i32
                };
            }
            
            true
        } else {
            false
        }
    }
    
    /// 获取角色列表长度
    pub fn character_count(&self) -> usize {
        self.characters.len()
    }
}

impl CharacterSelect {
    /// 绘制整个角色选择界面
    pub fn draw(&self, ctx: &mut ggez::graphics::GraphicsContext, canvas: &mut Canvas, button_group: &ButtonGroup) -> GameResult {
        // 1. 绘制背景和标题
        self.draw_background(ctx, canvas)?;
        
        // 2. 绘制角色槽位列表
        self.draw_character_slots(ctx, canvas)?;
        
        // 3. 绘制选中角色的预览动画
        if let Some(character) = self.get_selected_character() {
            self.draw_character_preview(ctx, canvas, character)?;
        }
        
        // 4. 绘制底部按钮和工具提示（传入按钮组引用）
        Self::draw_bottom_buttons(ctx, canvas, button_group)?;
        Self::draw_button_tooltips(ctx, canvas, button_group)?;
        
        Ok(())
    }
    
    /// 绘制背景和标题
    fn draw_background(&self, ctx: &mut ggez::graphics::GraphicsContext, canvas: &mut Canvas) -> GameResult {
        // 1. 绘制背景 Prguse_65
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Some(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 65, 0.0, 0.0, Color::WHITE, false);
            }
        }
        
        // 2. 绘制标题 Title_40 (C#位置: 468, 20)
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Some(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 40, 468.0, 20.0, Color::WHITE, false);
            }
        }
        
        // 2.5 绘制服务器标签 (C#位置: 432, 60, Size: 155x17, 水平居中)
        let mut server_label = Text::new("Legend of Mir 2");
        server_label.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(14.0));
        
        // 测量文本宽度以实现居中对齐
        let text_width = server_label.measure(ctx).map(|r| r.x).unwrap_or(120.0);
        let center_x = 432.0 + 155.0 / 2.0;  // 区域中心点
        let x = center_x - text_width / 2.0;  // 文本起始点
        
        canvas.draw(&server_label, DrawParam::default()
            .dest([x, 60.0])
            .color(Color::from_rgb(200, 200, 200)));
        
        Ok(())
    }
    
    /// 绘制角色槽位列表 (右侧垂直布局)
    fn draw_character_slots(&self, ctx: &mut ggez::graphics::GraphicsContext, canvas: &mut Canvas) -> GameResult {
        // C# 代码中的原始位置: (637, 194), (637, 298), (637, 402), (637, 506)
        let character_button_positions = [
            (637.0, 194.0),
            (637.0, 298.0),
            (637.0, 402.0),
            (637.0, 506.0),
        ];
        
        // 绘制已有角色
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
                if let Some(mut lib) = lib_arc.try_lock() {
                    let _ = lib.draw_with_color(ctx, canvas, slot_index as usize, slot_x, slot_y, Color::WHITE, false);
                }
            }
            
            // 绘制角色名称和等级信息
            Self::draw_character_info(ctx, canvas, character, slot_x, slot_y)?;
        }
        
        // 绘制空槽位
        for i in self.characters.len()..4 {
            let (slot_x, slot_y) = character_button_positions[i];
            
            if let Some(lib_arc) = get_library(LibraryName::Prguse) {
                if let Some(mut lib) = lib_arc.try_lock() {
                    let _ = lib.draw_with_color(ctx, canvas, 44, slot_x, slot_y, Color::WHITE, false);
                }
            }
        }
        
        Ok(())
    }
    
    /// 绘制单个角色的信息文本
    fn draw_character_info(
        _ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut Canvas,
        character: &SelectInfo,
        slot_x: f32,
        slot_y: f32,
    ) -> GameResult {
        // C# NameLabel: Location = (107, 9), Size = (170, 18)
        let mut name_text = Text::new(&character.name);
        name_text.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(14.0));
        canvas.draw(&name_text, DrawParam::default()
            .dest([slot_x + 107.0, slot_y + 9.0])
            .color(Color::WHITE));
        
        // C# LevelLabel: Location = (107, 28), Size = (30, 18)
        let mut level_text = Text::new(format!("Lv.{}", character.level));
        level_text.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(13.0));
        canvas.draw(&level_text, DrawParam::default()
            .dest([slot_x + 107.0, slot_y + 28.0])
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
            .dest([slot_x + 178.0, slot_y + 28.0])
            .color(Color::from_rgb(200, 200, 200)));
        
        Ok(())
    }
    
    /// 绘制选中角色的预览动画 (左侧中央)
    fn draw_character_preview(
        &self,
        ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut Canvas,
        character: &SelectInfo,
    ) -> GameResult {
        // C# CharacterDisplay: Location = new Point(260, 420)
        let preview_x = 260.0;
        let preview_y = 420.0;
        
        // 获取角色动画基础索引
        let base_index = Self::get_character_base_index(character.class, character.gender);
        let anim_index = base_index + self.animation_frame as i32;
        
        // 使用库系统绘制角色预览（use_offset=true 保持清晰）
        if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
            if let Some(mut lib) = lib_arc.try_lock() {
                // 绘制角色主体动画
                let _ = lib.draw_with_color(ctx, canvas, anim_index as usize, preview_x, preview_y, Color::WHITE, true);
                
                // 如果是法师，叠加绘制光效（使用半透明白色）
                if character.class == MirClass::Wizard {
                    let blend_index = (anim_index + 560) as usize;
                    let _ = lib.draw_with_color(ctx, canvas, blend_index, preview_x, preview_y, Color::from_rgba(255, 255, 255, 180), true);
                }
            }
        }
        
        // 绘制最后登录时间
        Self::draw_last_access(ctx, canvas, character)?;
        
        Ok(())
    }
    
    /// 获取角色动画基础索引
    fn get_character_base_index(class: mir2_shared::enums::MirClass, gender: mir2_shared::enums::MirGender) -> i32 {
        match (class, gender) {
            (mir2_shared::enums::MirClass::Warrior, mir2_shared::enums::MirGender::Male) => 20,
            (mir2_shared::enums::MirClass::Warrior, mir2_shared::enums::MirGender::Female) => 300,
            (mir2_shared::enums::MirClass::Wizard, mir2_shared::enums::MirGender::Male) => 40,
            (mir2_shared::enums::MirClass::Wizard, mir2_shared::enums::MirGender::Female) => 320,
            (mir2_shared::enums::MirClass::Taoist, mir2_shared::enums::MirGender::Male) => 60,
            (mir2_shared::enums::MirClass::Taoist, mir2_shared::enums::MirGender::Female) => 340,
            (mir2_shared::enums::MirClass::Assassin, mir2_shared::enums::MirGender::Male) => 80,
            (mir2_shared::enums::MirClass::Assassin, mir2_shared::enums::MirGender::Female) => 360,
            (mir2_shared::enums::MirClass::Archer, mir2_shared::enums::MirGender::Male) => 100,
            (mir2_shared::enums::MirClass::Archer, mir2_shared::enums::MirGender::Female) => 140,
        }
    }
    
    /// 绘制最后登录时间
    fn draw_last_access(
        ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut Canvas,
        character: &SelectInfo,
    ) -> GameResult {
        // C# LastAccessLabel: Location = (265, 609), Size = (180, 21)
        let mut last_online_label = Text::new("Last Online:");
        last_online_label.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(13.0));
        
        // 测量标签宽度
        let label_width = last_online_label.measure(ctx).map(|r| r.x).unwrap_or(85.0);
        
        canvas.draw(&last_online_label, DrawParam::default()
            .dest([200.0, 615.0])
            .color(Color::from_rgb(200, 200, 200)));
        
        // 绘制时间值
        let last_access = character.last_access.format("%Y-%m-%d %H:%M").to_string();
        let mut last_access_text = Text::new(&last_access);
        last_access_text.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(13.0));
        canvas.draw(&last_access_text, DrawParam::default()
            .dest([200.0 + label_width + 5.0, 615.0])
            .color(Color::WHITE));
        
        Ok(())
    }
    
    /// 绘制底部按钮
    pub fn draw_bottom_buttons(
        ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut Canvas,
        button_group: &ButtonGroup,
    ) -> GameResult {
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Some(mut lib) = lib_arc.try_lock() {
                for button in &button_group.buttons {
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
        
        Ok(())
    }
    
    /// 绘制按钮工具提示
    pub fn draw_button_tooltips(
        ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut Canvas,
        button_group: &ButtonGroup,
    ) -> GameResult {
        use ggez::graphics::{Rect, Mesh, DrawMode};
        
        for button in &button_group.buttons {
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
        
        Ok(())
    }
    
    /// 检查点击是否在角色槽位内
    pub fn check_character_slot_click(x: f32, y: f32, character_count: usize) -> Option<usize> {
        let character_button_positions = [
            (637.0, 194.0),
            (637.0, 298.0),
            (637.0, 402.0),
            (637.0, 506.0),
        ];
        
        for (i, &(slot_x, slot_y)) in character_button_positions.iter().enumerate() {
            if i >= character_count {
                break;
            }
            
            // 检查点击是否在槽位范围内 (宽度300像素，高度80像素)
            if x >= slot_x && x <= slot_x + 300.0 &&
               y >= slot_y && y <= slot_y + 80.0 {
                return Some(i);
            }
        }
        
        None
    }
}

