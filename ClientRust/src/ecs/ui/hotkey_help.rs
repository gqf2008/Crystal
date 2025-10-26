/// 按键帮助面板
/// 显示所有游戏快捷键说明

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, DrawParam, Text, Color, Rect, PxScale};

pub struct HotkeyHelpPanel {
    pub visible: bool,
    pub x: f32,
    pub y: f32,
}

impl HotkeyHelpPanel {
    pub fn new() -> Self {
        Self {
            visible: false, // 默认隐藏,按H键显示
            x: 20.0,
            y: 100.0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn draw(&self, _ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }

        let font_size = 20.0;
        let line_height = 24.0;
        let mut y = self.y;

        // 背景半透明黑色
        let bg_width = 600.0;
        let bg_height = 480.0;
        let bg_rect = Rect::new(self.x - 10.0, self.y - 10.0, bg_width, bg_height);
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            _ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 180),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());

        // 标题
        let title = "快捷键帮助 (按H键关闭)";
        let mut title_text = Text::new(title);
        title_text.set_scale(PxScale::from(font_size + 4.0));
        canvas.draw(
            &title_text,
            DrawParam::default()
                .dest([self.x, y])
                .color(Color::from_rgb(255, 255, 0)),
        );
        y += line_height + 10.0;

        // 快捷键列表
        let hotkeys = vec![
            ("═══ UI对话框 ═══", ""),
            ("I", "背包"),
            ("C", "角色"),
            ("S", "技能"),
            ("K", "学习技能"),
            ("Q", "任务"),
            ("T", "交易"),
            ("", ""),
            ("═══ 游戏操作 ═══", ""),
            ("N", "与最近NPC对话"),
            ("Space", "拾取物品"),
            ("Z", "整理背包"),
            ("Tab", "切换目标"),
            ("", ""),
            ("═══ 技能/物品 ═══", ""),
            ("F1-F8", "施放技能"),
            ("1-8", "使用物品"),
            ("", ""),
            ("═══ 调试工具 ═══", ""),
            ("B", "显示所有边框"),
            ("F9", "NPC边框(青色)"),
            ("F10", "Monster边框(紫色)"),
            ("F11", "特效边框(绿色)"),
            ("G", "网格"),
            ("O", "障碍物"),
            ("P", "寻路路径"),
        ];

        for (key, desc) in hotkeys.iter() {
            if key.is_empty() {
                y += line_height / 2.0;
                continue;
            }

            if desc.is_empty() {
                // 分类标题
                let mut section_text = Text::new(*key);
                section_text.set_scale(PxScale::from(font_size));
                canvas.draw(
                    &section_text,
                    DrawParam::default()
                        .dest([self.x, y])
                        .color(Color::from_rgb(100, 200, 255)),
                );
            } else {
                // 按键 + 说明
                let key_color = Color::from_rgb(255, 200, 100);
                let desc_color = Color::from_rgb(220, 220, 220);

                let mut key_text = Text::new(*key);
                key_text.set_scale(PxScale::from(font_size));
                canvas.draw(
                    &key_text,
                    DrawParam::default()
                        .dest([self.x + 20.0, y])
                        .color(key_color),
                );

                let mut desc_text = Text::new(*desc);
                desc_text.set_scale(PxScale::from(font_size));
                canvas.draw(
                    &desc_text,
                    DrawParam::default()
                        .dest([self.x + 140.0, y])
                        .color(desc_color),
                );
            }

            y += line_height;
        }

        Ok(())
    }
}
