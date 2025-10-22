// ============================================================================
// 技能栏对话框
// ============================================================================
//
// 功能:
// - 显示8个技能槽(可配置多个技能栏)
// - 技能图标、快捷键名称、冷却时间
// - 点击使用技能
// - 拖拽技能到槽位
//
// 参考: Client/MirScenes/Dialogs/MainDialogs.cs (SkillBarDialog)
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Rect, DrawParam};

/// 技能栏对话框
pub struct SkillBarDialog {
    /// 是否可见
    visible: bool,
    
    /// 位置
    x: f32,
    y: f32,
    
    /// 技能栏索引 (支持多个技能栏 0, 1, 2...)
    bar_index: u8,
    
    /// 技能槽 (最多8个)
    skills: [Option<SkillSlot>; 8],
    
    /// 悬停的槽位索引
    hovered_slot: Option<usize>,
}

/// 技能槽
#[derive(Clone, Debug)]
pub struct SkillSlot {
    /// 技能图标索引
    pub icon_index: u16,
    
    /// 技能名称
    pub name: String,
    
    /// MP消耗
    pub mp_cost: u16,
    
    /// 冷却时间(毫秒)
    pub cooldown_ms: u32,
    
    /// 施法时间戳(用于计算冷却)
    pub cast_time: u64,
    
    /// 快捷键名称
    pub key_name: String,
    
    /// 技能ID(用于释放)
    pub spell: u8,
    
    /// 技能等级
    pub level: u8,
}

/// 技能栏操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillBarAction {
    /// 关闭
    Close,
    
    /// 使用技能
    UseSkill { slot_index: usize },
    
    /// 切换技能绑定
    SwitchBinds,
}

impl SkillBarDialog {
    /// 创建新技能栏
    pub fn new(bar_index: u8) -> Self {
        Self {
            visible: false,
            x: 100.0,
            y: 100.0 + (bar_index as f32 * 35.0), // 多个技能栏垂直排列
            bar_index,
            skills: Default::default(),
            hovered_slot: None,
        }
    }
    
    /// 设置位置
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }
    
    /// 切换显示/隐藏
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    
    /// 显示
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    /// 隐藏
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 更新技能栏数据
    pub fn update_skills(&mut self, magics: &[(u8, u8, u8)], current_time: u64) {
        // magics: (key, spell, level)
        // 清空所有槽位
        self.skills = Default::default();
        
        // 计算槽位偏移量
        let offset = (self.bar_index as usize) * 8;
        
        // 填充技能
        for &(key, spell, level) in magics {
            // key 是 1-based，需要转换为槽位索引
            if key == 0 {
                continue;
            }
            
            let key_index = key as usize - 1; // 转为 0-based
            
            // 检查是否属于当前技能栏
            if key_index < offset || key_index >= offset + 8 {
                continue;
            }
            
            let slot_index = key_index - offset;
            
            // 设置快捷键名称
            let key_name = if self.bar_index == 0 {
                format!("F{}", slot_index + 1)
            } else {
                format!("C-F{}", slot_index + 1)
            };
            
            // TODO: 从ClientMagic获取详细信息
            self.skills[slot_index] = Some(SkillSlot {
                icon_index: spell as u16,
                name: format!("Skill {}", spell),
                mp_cost: 10, // TODO: 从配置读取
                cooldown_ms: 1000, // TODO: 从配置读取
                cast_time: 0,
                key_name,
                spell,
                level,
            });
        }
    }
    
    /// 更新冷却时间
    pub fn update_cooldown(&mut self, slot_index: usize, cast_time: u64) {
        if let Some(ref mut skill) = self.skills[slot_index] {
            skill.cast_time = cast_time;
        }
    }
    
    /// 获取冷却进度 (0.0 - 1.0)
    pub fn get_cooldown_progress(&self, slot_index: usize, current_time: u64) -> f32 {
        if let Some(ref skill) = self.skills[slot_index] {
            if skill.cast_time == 0 {
                return 0.0;
            }
            
            let elapsed = current_time.saturating_sub(skill.cast_time);
            if elapsed >= skill.cooldown_ms as u64 {
                return 0.0;
            }
            
            let progress = elapsed as f32 / skill.cooldown_ms as f32;
            1.0 - progress // 反转: 1.0 = 刚施放, 0.0 = 冷却完成
        } else {
            0.0
        }
    }
    
    /// 检查点击位置
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> Option<SkillBarAction> {
        if !self.visible {
            return None;
        }
        
        // 检查切换按钮 (左侧)
        if self.is_in_switch_button(x, y) {
            return Some(SkillBarAction::SwitchBinds);
        }
        
        // 检查技能槽点击
        if let Some(slot_index) = self.get_skill_slot_at(x, y) {
            if self.skills[slot_index].is_some() {
                return Some(SkillBarAction::UseSkill { slot_index });
            }
        }
        
        None
    }
    
    /// 更新悬停状态
    pub fn update_hover(&mut self, x: f32, y: f32) {
        if !self.visible {
            self.hovered_slot = None;
            return;
        }
        
        self.hovered_slot = self.get_skill_slot_at(x, y);
    }
    
    /// 获取指定槽位的技能
    pub fn get_skill(&self, slot_index: usize) -> Option<&SkillSlot> {
        if slot_index >= 8 {
            return None;
        }
        self.skills[slot_index].as_ref()
    }
    
    /// 检查是否在切换按钮内
    fn is_in_switch_button(&self, x: f32, y: f32) -> bool {
        let rect = Rect::new(self.x, self.y, 16.0, 28.0);
        rect.contains([x, y])
    }
    
    /// 获取鼠标位置对应的技能槽索引
    fn get_skill_slot_at(&self, x: f32, y: f32) -> Option<usize> {
        for i in 0..8 {
            let slot_x = self.x + 15.0 + (i as f32 * 25.0);
            let slot_y = self.y + 3.0;
            let rect = Rect::new(slot_x, slot_y, 24.0, 24.0);
            
            if rect.contains([x, y]) {
                return Some(i);
            }
        }
        None
    }
    
    /// 渲染
    pub fn draw(&self, _ctx: &mut Context, canvas: &mut Canvas, current_time: u64) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        
        // 背景框
        let background_rect = ggez::graphics::Mesh::new_rectangle(
            _ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x, self.y, 215.0, 30.0),
            Color::from_rgba(30, 30, 30, 200),
        )?;
        canvas.draw(&background_rect, DrawParam::default());
        
        // 边框
        let border_rect = ggez::graphics::Mesh::new_rectangle(
            _ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            Rect::new(self.x, self.y, 215.0, 30.0),
            Color::from_rgb(100, 100, 100),
        )?;
        canvas.draw(&border_rect, DrawParam::default());
        
        // 技能栏索引标签
        let index_text = ggez::graphics::Text::new(format!("{}", self.bar_index + 1));
        canvas.draw(
            &index_text,
            DrawParam::default()
                .dest([self.x + 2.0, self.y + 8.0])
                .color(Color::WHITE),
        );
        
        // 渲染技能槽
        for i in 0..8 {
            let slot_x = self.x + 15.0 + (i as f32 * 25.0);
            let slot_y = self.y + 3.0;
            
            // 槽位背景
            let slot_bg = ggez::graphics::Mesh::new_rectangle(
                _ctx,
                ggez::graphics::DrawMode::fill(),
                Rect::new(slot_x, slot_y, 24.0, 24.0),
                if self.hovered_slot == Some(i) {
                    Color::from_rgba(80, 80, 80, 255)
                } else {
                    Color::from_rgba(50, 50, 50, 255)
                },
            )?;
            canvas.draw(&slot_bg, DrawParam::default());
            
            // 技能图标
            if let Some(ref skill) = self.skills[i] {
                // TODO: 绘制真实技能图标
                // 目前用简单颜色块代替
                let icon_color = match skill.spell % 5 {
                    0 => Color::from_rgb(255, 100, 100),
                    1 => Color::from_rgb(100, 255, 100),
                    2 => Color::from_rgb(100, 100, 255),
                    3 => Color::from_rgb(255, 255, 100),
                    _ => Color::from_rgb(255, 100, 255),
                };
                
                let icon_rect = ggez::graphics::Mesh::new_rectangle(
                    _ctx,
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(slot_x + 2.0, slot_y + 2.0, 20.0, 20.0),
                    icon_color,
                )?;
                canvas.draw(&icon_rect, DrawParam::default());
                
                // 快捷键名称
                let key_text = ggez::graphics::Text::new(&skill.key_name);
                canvas.draw(
                    &key_text,
                    DrawParam::default()
                        .dest([slot_x + 2.0, slot_y - 10.0])
                        .color(Color::WHITE)
                        .scale([0.7, 0.7]),
                );
                
                // 冷却覆盖层
                let cooldown_progress = self.get_cooldown_progress(i, current_time);
                if cooldown_progress > 0.0 {
                    // 半透明黑色覆盖层
                    let cooldown_height = 20.0 * cooldown_progress;
                    let cooldown_overlay = ggez::graphics::Mesh::new_rectangle(
                        _ctx,
                        ggez::graphics::DrawMode::fill(),
                        Rect::new(slot_x + 2.0, slot_y + 2.0, 20.0, cooldown_height),
                        Color::from_rgba(0, 0, 0, 150),
                    )?;
                    canvas.draw(&cooldown_overlay, DrawParam::default());
                    
                    // 冷却百分比文字
                    let cooldown_text = ggez::graphics::Text::new(format!("{:.0}%", cooldown_progress * 100.0));
                    canvas.draw(
                        &cooldown_text,
                        DrawParam::default()
                            .dest([slot_x + 6.0, slot_y + 8.0])
                            .color(Color::from_rgb(255, 200, 200))
                            .scale([0.6, 0.6]),
                    );
                }
            } else {
                // 空槽位 - 显示快捷键提示
                let key_name = if self.bar_index == 0 {
                    format!("F{}", i + 1)
                } else {
                    format!("C-F{}", i + 1)
                };
                
                let key_text = ggez::graphics::Text::new(&key_name);
                canvas.draw(
                    &key_text,
                    DrawParam::default()
                        .dest([slot_x + 4.0, slot_y + 8.0])
                        .color(Color::from_rgba(150, 150, 150, 200))
                        .scale([0.7, 0.7]),
                );
            }
            
            // 槽位边框
            let slot_border = ggez::graphics::Mesh::new_rectangle(
                _ctx,
                ggez::graphics::DrawMode::stroke(1.0),
                Rect::new(slot_x, slot_y, 24.0, 24.0),
                Color::from_rgb(100, 100, 100),
            )?;
            canvas.draw(&slot_border, DrawParam::default());
        }
        
        Ok(())
    }
}
