// ============================================================================
// BuffDialog - Buff/Debuff 状态显示对话框
// ============================================================================
//
// 功能:
// - 显示玩家当前的 Buff 和 Debuff 列表
// - 显示图标、剩余时间、效果说明
// - 鼠标悬停显示详细信息
// - 支持右键点击取消可移除的 Buff
//
// 位置: 屏幕上方中央
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, DrawParam, Color, Rect, Drawable};
use mir2_shared::enums::BuffType;

/// Buff 显示项
#[derive(Debug, Clone)]
pub struct BuffItem {
    pub buff_type: BuffType,
    pub visible: bool,          // 是否可见
    pub object_id: u32,         // 归属对象ID
    pub expire_time: i64,       // 过期时间 (Unix timestamp in milliseconds)
    pub infinite: bool,         // 是否永久
    pub paused: bool,           // 是否暂停
    pub icon_index: u32,        // 图标索引
    pub name: String,           // Buff 名称
    pub description: String,    // Buff 描述
}

impl BuffItem {
    /// 计算剩余时间 (秒)
    pub fn remaining_seconds(&self) -> i64 {
        if self.infinite {
            return i64::MAX;
        }
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        
        let remaining_ms = self.expire_time - now;
        (remaining_ms / 1000).max(0)
    }
    
    /// 格式化剩余时间
    pub fn format_time(&self) -> String {
        if self.infinite {
            return "∞".to_string();
        }
        
        let seconds = self.remaining_seconds();
        if seconds <= 0 {
            return "0:00".to_string();
        }
        
        let minutes = seconds / 60;
        let secs = seconds % 60;
        
        if minutes > 60 {
            let hours = minutes / 60;
            format!("{}h", hours)
        } else if minutes > 0 {
            format!("{}:{:02}", minutes, secs)
        } else {
            format!("0:{:02}", secs)
        }
    }
    
    /// 是否已过期
    pub fn is_expired(&self) -> bool {
        if self.infinite {
            return false;
        }
        self.remaining_seconds() <= 0
    }
}

/// Buff 显示对话框
#[derive(Debug, Clone)]
pub struct BuffDialog {
    /// Buff 列表
    pub buffs: Vec<BuffItem>,
    
    /// Debuff 列表
    pub debuffs: Vec<BuffItem>,
    
    /// 显示位置 (屏幕上方中央)
    pub x: f32,
    pub y: f32,
    
    /// 图标大小
    pub icon_size: f32,
    
    /// 图标间距
    pub icon_spacing: f32,
    
    /// 当前鼠标悬停的 Buff 索引
    pub hovered_index: Option<usize>,
    
    /// 是否显示 Debuff
    pub show_debuffs: bool,
}

impl BuffDialog {
    /// 创建新的 Buff 对话框
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            buffs: Vec::new(),
            debuffs: Vec::new(),
            x: screen_width / 2.0,
            y: 10.0,  // 距离顶部10像素
            icon_size: 32.0,
            icon_spacing: 4.0,
            hovered_index: None,
            show_debuffs: true,
        }
    }
    
    /// 添加 Buff
    pub fn add_buff(&mut self, buff: BuffItem) {
        // 检查是否已存在
        if let Some(existing) = self.buffs.iter_mut().find(|b| b.buff_type == buff.buff_type) {
            *existing = buff;
            return;
        }
        
        // 按照类型排序插入
        self.buffs.push(buff);
        self.buffs.sort_by_key(|b| b.buff_type as u8);
    }
    
    /// 移除 Buff
    pub fn remove_buff(&mut self, buff_type: BuffType) {
        self.buffs.retain(|b| b.buff_type != buff_type);
    }
    
    /// 添加 Debuff
    pub fn add_debuff(&mut self, debuff: BuffItem) {
        if let Some(existing) = self.debuffs.iter_mut().find(|d| d.buff_type == debuff.buff_type) {
            *existing = debuff;
            return;
        }
        
        self.debuffs.push(debuff);
        self.debuffs.sort_by_key(|d| d.buff_type as u8);
    }
    
    /// 移除 Debuff
    pub fn remove_debuff(&mut self, buff_type: BuffType) {
        self.debuffs.retain(|d| d.buff_type != buff_type);
    }
    
    /// 清空所有 Buff
    pub fn clear(&mut self) {
        self.buffs.clear();
        self.debuffs.clear();
    }
    
    /// 更新过期的 Buff
    pub fn update(&mut self) {
        self.buffs.retain(|b| !b.is_expired());
        self.debuffs.retain(|d| !d.is_expired());
    }
    
    /// 检查鼠标悬停
    pub fn update_hover(&mut self, mouse_x: f32, mouse_y: f32) {
        self.hovered_index = None;
        
        // 检查 Buffs
        let buff_count = self.buffs.len();
        let total_width = (buff_count as f32) * (self.icon_size + self.icon_spacing);
        let start_x = self.x - total_width / 2.0;
        
        for (i, _buff) in self.buffs.iter().enumerate() {
            let icon_x = start_x + (i as f32) * (self.icon_size + self.icon_spacing);
            let icon_y = self.y;
            
            if mouse_x >= icon_x && mouse_x <= icon_x + self.icon_size 
               && mouse_y >= icon_y && mouse_y <= icon_y + self.icon_size {
                self.hovered_index = Some(i);
                return;
            }
        }
        
        // 检查 Debuffs (显示在 Buffs 下方)
        if self.show_debuffs {
            let debuff_count = self.debuffs.len();
            let total_width = (debuff_count as f32) * (self.icon_size + self.icon_spacing);
            let start_x = self.x - total_width / 2.0;
            let debuff_y = self.y + self.icon_size + 10.0;
            
            for (i, _debuff) in self.debuffs.iter().enumerate() {
                let icon_x = start_x + (i as f32) * (self.icon_size + self.icon_spacing);
                
                if mouse_x >= icon_x && mouse_x <= icon_x + self.icon_size 
                   && mouse_y >= debuff_y && mouse_y <= debuff_y + self.icon_size {
                    self.hovered_index = Some(buff_count + i);
                    return;
                }
            }
        }
    }
    
    /// 处理鼠标点击 (右键取消 Buff)
    pub fn handle_click(&self, mouse_x: f32, mouse_y: f32, is_right_click: bool) -> Option<BuffType> {
        if !is_right_click {
            return None;
        }
        
        // 检查 Buffs
        let buff_count = self.buffs.len();
        let total_width = (buff_count as f32) * (self.icon_size + self.icon_spacing);
        let start_x = self.x - total_width / 2.0;
        
        for (i, buff) in self.buffs.iter().enumerate() {
            let icon_x = start_x + (i as f32) * (self.icon_size + self.icon_spacing);
            let icon_y = self.y;
            
            if mouse_x >= icon_x && mouse_x <= icon_x + self.icon_size 
               && mouse_y >= icon_y && mouse_y <= icon_y + self.icon_size {
                // 只有可移除的 Buff 才能右键取消
                if !buff.infinite && buff.visible {
                    return Some(buff.buff_type);
                }
            }
        }
        
        None
    }
    
    /// 绘制对话框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 绘制 Buffs
        self.draw_buff_list(ctx, canvas, &self.buffs, self.y, Color::from_rgb(100, 200, 255))?;
        
        // 绘制 Debuffs (在 Buffs 下方)
        if self.show_debuffs {
            let debuff_y = self.y + self.icon_size + 10.0;
            self.draw_buff_list(ctx, canvas, &self.debuffs, debuff_y, Color::from_rgb(255, 100, 100))?;
        }
        
        // 绘制悬停提示
        if let Some(hovered_index) = self.hovered_index {
            self.draw_tooltip(ctx, canvas, hovered_index)?;
        }
        
        Ok(())
    }
    
    /// 绘制 Buff 列表
    fn draw_buff_list(&self, ctx: &mut Context, canvas: &mut Canvas, buffs: &[BuffItem], y: f32, border_color: Color) -> GameResult {
        if buffs.is_empty() {
            return Ok(());
        }
        
        let buff_count = buffs.len();
        let total_width = (buff_count as f32) * (self.icon_size + self.icon_spacing);
        let start_x = self.x - total_width / 2.0;
        
        for (i, buff) in buffs.iter().enumerate() {
            let icon_x = start_x + (i as f32) * (self.icon_size + self.icon_spacing);
            
            // 绘制背景框
            let bg_rect = Rect::new(icon_x, y, self.icon_size, self.icon_size);
            let bg_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::fill(),
                bg_rect,
                Color::from_rgba(0, 0, 0, 180),
            )?;
            canvas.draw(&bg_mesh, DrawParam::default());
            
            // 绘制边框
            let border_mesh = ggez::graphics::Mesh::new_rectangle(
                ctx,
                ggez::graphics::DrawMode::stroke(2.0),
                bg_rect,
                border_color,
            )?;
            canvas.draw(&border_mesh, DrawParam::default());
            
            // TODO: 绘制 Buff 图标 (需要图像库支持)
            // let icon_texture = get_buff_icon(buff.icon_index);
            // canvas.draw(icon_texture, DrawParam::new().dest([icon_x, y]));
            
            // 绘制剩余时间
            if !buff.infinite {
                let time_text = ggez::graphics::Text::new(buff.format_time());
                let time_dims = time_text.measure(ctx)?;
                let time_x = icon_x + (self.icon_size - time_dims.x) / 2.0;
                let time_y = y + self.icon_size - 12.0;
                canvas.draw(
                    &time_text,
                    DrawParam::default()
                        .dest([time_x, time_y])
                        .color(Color::WHITE),
                );
            }
            
            // 暂停标记
            if buff.paused {
                let pause_text = ggez::graphics::Text::new("||");
                let pause_x = icon_x + 2.0;
                let pause_y = y + 2.0;
                canvas.draw(
                    &pause_text,
                    DrawParam::default()
                        .dest([pause_x, pause_y])
                        .color(Color::from_rgb(255, 255, 0)),
                );
            }
        }
        
        Ok(())
    }
    
    /// 绘制悬停提示
    fn draw_tooltip(&self, ctx: &mut Context, canvas: &mut Canvas, hovered_index: usize) -> GameResult {
        let buff_count = self.buffs.len();
        
        let buff = if hovered_index < buff_count {
            &self.buffs[hovered_index]
        } else if hovered_index < buff_count + self.debuffs.len() {
            &self.debuffs[hovered_index - buff_count]
        } else {
            return Ok(());
        };
        
        // 构建提示文本
        let mut tooltip_text = format!("{}\n", buff.name);
        if !buff.description.is_empty() {
            tooltip_text.push_str(&format!("{}\n", buff.description));
        }
        if !buff.infinite {
            tooltip_text.push_str(&format!("剩余时间: {}", buff.format_time()));
        } else {
            tooltip_text.push_str("永久效果");
        }
        
        let text = ggez::graphics::Text::new(tooltip_text);
        let text_dims = text.measure(ctx)?;
        
        // 计算提示框位置 (在图标下方)
        let icon_x = if hovered_index < buff_count {
            let total_width = (buff_count as f32) * (self.icon_size + self.icon_spacing);
            let start_x = self.x - total_width / 2.0;
            start_x + (hovered_index as f32) * (self.icon_size + self.icon_spacing)
        } else {
            let debuff_count = self.debuffs.len();
            let total_width = (debuff_count as f32) * (self.icon_size + self.icon_spacing);
            let start_x = self.x - total_width / 2.0;
            start_x + ((hovered_index - buff_count) as f32) * (self.icon_size + self.icon_spacing)
        };
        
        let icon_y = if hovered_index < buff_count {
            self.y
        } else {
            self.y + self.icon_size + 10.0
        };
        
        let tooltip_x = icon_x;
        let tooltip_y = icon_y + self.icon_size + 5.0;
        let tooltip_padding = 5.0;
        
        // 绘制提示框背景
        let bg_rect = Rect::new(
            tooltip_x - tooltip_padding,
            tooltip_y - tooltip_padding,
            text_dims.x + tooltip_padding * 2.0,
            text_dims.y + tooltip_padding * 2.0,
        );
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 230),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());
        
        // 绘制边框
        let border_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            bg_rect,
            Color::from_rgb(200, 200, 200),
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        // 绘制文本
        canvas.draw(
            &text,
            DrawParam::default()
                .dest([tooltip_x, tooltip_y])
                .color(Color::WHITE),
        );
        
        Ok(())
    }
    
    /// 获取指定 BuffType 的名称
    pub fn get_buff_name(buff_type: BuffType) -> String {
        match buff_type {
            BuffType::None => "无".to_string(),
            BuffType::TemporalFlux => "时光扭曲".to_string(),
            BuffType::Hiding => "隐身".to_string(),
            BuffType::Haste => "加速".to_string(),
            BuffType::SwiftFeet => "疾行".to_string(),
            BuffType::Fury => "狂怒".to_string(),
            BuffType::SoulShield => "灵魂护盾".to_string(),
            BuffType::BlessedArmour => "神圣战甲".to_string(),
            BuffType::LightBody => "轻身术".to_string(),
            BuffType::UltimateEnhancer => "终极强化".to_string(),
            BuffType::ProtectionField => "防护光环".to_string(),
            BuffType::Rage => "暴怒".to_string(),
            BuffType::Curse => "诅咒".to_string(),
            BuffType::MoonLight => "月光".to_string(),
            BuffType::DarkBody => "黑暗之躯".to_string(),
            BuffType::Concentration => "专注".to_string(),
            BuffType::VampireShot => "吸血射击".to_string(),
            BuffType::PoisonShot => "毒箭".to_string(),
            BuffType::CounterAttack => "反击".to_string(),
            BuffType::MentalState => "精神力".to_string(),
            BuffType::EnergyShield => "能量护盾".to_string(),
            BuffType::MagicBooster => "魔法增幅".to_string(),
            BuffType::PetEnhancer => "宠物强化".to_string(),
            BuffType::ImmortalSkin => "不朽之躯".to_string(),
            BuffType::MagicShield => "魔法盾".to_string(),
            BuffType::ElementalBarrier => "元素结界".to_string(),
            BuffType::GameMaster => "GM".to_string(),
            BuffType::Exp => "经验加成".to_string(),
            BuffType::Drop => "掉落加成".to_string(),
            BuffType::Gold => "金币加成".to_string(),
            _ => format!("{:?}", buff_type),
        }
    }
}

/// Buff 对话框组件 (用于 ECS)
#[derive(Debug, Clone)]
pub struct BuffDialogComponent {
    pub dialog: BuffDialog,
}

impl BuffDialogComponent {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            dialog: BuffDialog::new(screen_width, screen_height),
        }
    }
}
