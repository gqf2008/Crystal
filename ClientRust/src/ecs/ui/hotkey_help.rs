/// 按键帮助面板 - 显示所有游戏快捷键说明
use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, DrawParam, Text, TextFragment, Color, Rect, PxScale};

pub struct HotkeyHelpPanel {
    pub visible: bool,
    font_name: Option<String>,
}

impl HotkeyHelpPanel {
    pub fn new() -> Self {
        Self { visible: false, font_name: None }
    }
    
    pub fn set_font(&mut self, font_name: String) {
        self.font_name = Some(font_name);
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn draw(&self, _ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }

        // 使用更宽的水平布局 - 两列显示
        let x = 10.0;
        let y = 10.0;
        let font_size = 16.0;
        let line_height = 20.0;
        let col1_x = x + 10.0;      // 第一列X位置
        let col2_x = x + 320.0;     // 第二列X位置
        let key_width = 80.0;       // 按键列宽度
        
        // 绘制更宽的半透明背景
        let bg_width = 630.0;
        let bg_height = 280.0;
        let bg_rect = Rect::new(x - 5.0, y - 5.0, bg_width, bg_height);
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(_ctx, ggez::graphics::DrawMode::fill(), bg_rect, Color::from_rgba(0, 0, 0, 200))?;
        canvas.draw(&bg_mesh, DrawParam::default());

        // 标题
        let title_fragment = if let Some(ref font) = self.font_name {
            TextFragment::new("快捷键帮助 (H键关闭)").font(font.as_str()).scale(PxScale::from(font_size + 4.0))
        } else {
            TextFragment::new("快捷键帮助 (H键关闭)").scale(PxScale::from(font_size + 4.0))
        };
        canvas.draw(&Text::new(title_fragment), DrawParam::default().dest([col1_x, y + 5.0]).color(Color::from_rgb(255, 255, 0)));

        // 快捷键列表 - 分成两列
        let col1_keys = vec![
            ("═══ UI对话框 ═══", "", true),
            ("I", "背包", false), ("C", "角色", false), ("S", "技能", false),
            ("K", "学习技能", false), ("Q", "任务", false), ("T", "交易", false),
            ("", "", false),
            ("═══ 游戏操作 ═══", "", true),
            ("N", "与NPC对话", false), ("Space", "拾取物品", false),
            ("Z", "整理背包", false), ("Tab", "切换目标", false),
        ];
        
        let col2_keys = vec![
            ("═══ 技能/物品 ═══", "", true),
            ("F1-F8", "施放技能", false), ("1-8", "使用物品", false),
            ("", "", false),
            ("═══ 调试工具 ═══", "", true),
            ("B", "显示所有边框", false), ("F9", "NPC边框(青)", false),
            ("F10", "Monster边框(紫)", false), ("F11", "特效边框(绿)", false),
            ("G", "网格", false), ("O", "障碍物", false), ("P", "寻路路径", false),
        ];

        // 绘制第一列
        let mut current_y = y + 35.0;
        for (key, desc, is_title) in col1_keys.iter() {
            if key.is_empty() { current_y += line_height / 2.0; continue; }
            
            if *is_title {
                let fragment = if let Some(ref font) = self.font_name {
                    TextFragment::new(*key).font(font.as_str()).scale(PxScale::from(font_size))
                } else { TextFragment::new(*key).scale(PxScale::from(font_size)) };
                canvas.draw(&Text::new(fragment), DrawParam::default().dest([col1_x, current_y]).color(Color::from_rgb(100, 200, 255)));
            } else {
                let key_fragment = if let Some(ref font) = self.font_name {
                    TextFragment::new(*key).font(font.as_str()).scale(PxScale::from(font_size))
                } else { TextFragment::new(*key).scale(PxScale::from(font_size)) };
                canvas.draw(&Text::new(key_fragment), DrawParam::default().dest([col1_x + 10.0, current_y]).color(Color::from_rgb(255, 200, 100)));

                let desc_fragment = if let Some(ref font) = self.font_name {
                    TextFragment::new(*desc).font(font.as_str()).scale(PxScale::from(font_size))
                } else { TextFragment::new(*desc).scale(PxScale::from(font_size)) };
                canvas.draw(&Text::new(desc_fragment), DrawParam::default().dest([col1_x + key_width, current_y]).color(Color::from_rgb(220, 220, 220)));
            }
            current_y += line_height;
        }

        // 绘制第二列
        current_y = y + 35.0;
        for (key, desc, is_title) in col2_keys.iter() {
            if key.is_empty() { current_y += line_height / 2.0; continue; }
            
            if *is_title {
                let fragment = if let Some(ref font) = self.font_name {
                    TextFragment::new(*key).font(font.as_str()).scale(PxScale::from(font_size))
                } else { TextFragment::new(*key).scale(PxScale::from(font_size)) };
                canvas.draw(&Text::new(fragment), DrawParam::default().dest([col2_x, current_y]).color(Color::from_rgb(100, 200, 255)));
            } else {
                let key_fragment = if let Some(ref font) = self.font_name {
                    TextFragment::new(*key).font(font.as_str()).scale(PxScale::from(font_size))
                } else { TextFragment::new(*key).scale(PxScale::from(font_size)) };
                canvas.draw(&Text::new(key_fragment), DrawParam::default().dest([col2_x + 10.0, current_y]).color(Color::from_rgb(255, 200, 100)));

                let desc_fragment = if let Some(ref font) = self.font_name {
                    TextFragment::new(*desc).font(font.as_str()).scale(PxScale::from(font_size))
                } else { TextFragment::new(*desc).scale(PxScale::from(font_size)) };
                canvas.draw(&Text::new(desc_fragment), DrawParam::default().dest([col2_x + key_width, current_y]).color(Color::from_rgb(220, 220, 220)));
            }
            current_y += line_height;
        }

        Ok(())
    }
}