// ============================================================================
// UI 渲染系统 - 负责绘制所有UI元素
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, DrawParam, Rect, Mesh, DrawMode, Text, PxScale, TextFragment};
use hecs::World;

use super::components::*;

/// UI 渲染系统
pub struct UIRenderer;

impl UIRenderer {
    /// 渲染所有UI元素
    pub fn render(ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 查询 CharacterStatus 组件
        for (_, status) in world.query::<&CharacterStatus>().iter() {
            // 渲染血条
            if let Some((_, health_bar)) = world.query::<&HealthBar>().iter().next() {
                if health_bar.visible {
                    Self::draw_health_bar(ctx, canvas, health_bar, status)?;
                }
            }
            
            // 渲染魔法条
            if let Some((_, mana_bar)) = world.query::<&ManaBar>().iter().next() {
                if mana_bar.visible {
                    Self::draw_mana_bar(ctx, canvas, mana_bar, status)?;
                }
            }
            
            // 渲染经验条
            if let Some((_, exp_bar)) = world.query::<&ExpBar>().iter().next() {
                if exp_bar.visible {
                    Self::draw_exp_bar(ctx, canvas, exp_bar, status)?;
                }
            }
            
            // 渲染技能栏
            if let Some((_, skill_bar)) = world.query::<&SkillBar>().iter().next() {
                if skill_bar.visible {
                    Self::draw_skill_bar(ctx, canvas, skill_bar)?;
                }
            }
            
            // 渲染聊天窗口
            if let Some((_, chat)) = world.query::<&ChatWindow>().iter().next() {
                if chat.visible {
                    Self::draw_chat_window(ctx, canvas, chat)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// 绘制血条
    fn draw_health_bar(
        ctx: &mut Context,
        canvas: &mut Canvas,
        bar: &HealthBar,
        status: &CharacterStatus,
    ) -> GameResult {
        let percent = status.health_percent();
        
        // 背景（深灰色）
        let bg = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(bar.x, bar.y, bar.width, bar.height),
            Color::from_rgb(40, 40, 40),
        )?;
        canvas.draw(&bg, DrawParam::default());
        
        // 血条本体
        if percent > 0.0 {
            let fill_width = bar.width * percent;
            let fg = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(bar.x, bar.y, fill_width, bar.height),
                HealthBar::get_color(percent),
            )?;
            canvas.draw(&fg, DrawParam::default());
        }
        
        // 边框
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(1.0),
            Rect::new(bar.x, bar.y, bar.width, bar.height),
            Color::WHITE,
        )?;
        canvas.draw(&border, DrawParam::default());
        
        // 文字（HP: 120/150）
        if bar.show_text {
            let text = format!("HP: {}/{}", status.health, status.max_health);
            let mut text_obj = Text::new(text);
            text_obj.set_scale(PxScale::from(14.0));
            
            let text_x = bar.x + bar.width / 2.0 - 30.0;
            let text_y = bar.y + 2.0;
            
            canvas.draw(
                &text_obj,
                DrawParam::default()
                    .dest([text_x, text_y])
                    .color(Color::WHITE),
            );
        }
        
        Ok(())
    }
    
    /// 绘制魔法条
    fn draw_mana_bar(
        ctx: &mut Context,
        canvas: &mut Canvas,
        bar: &ManaBar,
        status: &CharacterStatus,
    ) -> GameResult {
        let percent = status.mana_percent();
        
        // 背景
        let bg = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(bar.x, bar.y, bar.width, bar.height),
            Color::from_rgb(40, 40, 40),
        )?;
        canvas.draw(&bg, DrawParam::default());
        
        // 魔法条本体
        if percent > 0.0 {
            let fill_width = bar.width * percent;
            let fg = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(bar.x, bar.y, fill_width, bar.height),
                ManaBar::get_color(percent),
            )?;
            canvas.draw(&fg, DrawParam::default());
        }
        
        // 边框
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(1.0),
            Rect::new(bar.x, bar.y, bar.width, bar.height),
            Color::WHITE,
        )?;
        canvas.draw(&border, DrawParam::default());
        
        // 文字（MP: 50/50）
        if bar.show_text {
            let text = format!("MP: {}/{}", status.mana, status.max_mana);
            let mut text_obj = Text::new(text);
            text_obj.set_scale(PxScale::from(14.0));
            
            let text_x = bar.x + bar.width / 2.0 - 30.0;
            let text_y = bar.y + 2.0;
            
            canvas.draw(
                &text_obj,
                DrawParam::default()
                    .dest([text_x, text_y])
                    .color(Color::WHITE),
            );
        }
        
        Ok(())
    }
    
    /// 绘制经验条
    fn draw_exp_bar(
        ctx: &mut Context,
        canvas: &mut Canvas,
        bar: &ExpBar,
        status: &CharacterStatus,
    ) -> GameResult {
        let percent = status.exp_percent();
        
        // 背景
        let bg = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(bar.x, bar.y, bar.width, bar.height),
            Color::from_rgba(20, 20, 20, 180),
        )?;
        canvas.draw(&bg, DrawParam::default());
        
        // 经验条本体
        if percent > 0.0 {
            let fill_width = bar.width * percent;
            let fg = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(bar.x, bar.y, fill_width, bar.height),
                ExpBar::get_color(percent),
            )?;
            canvas.draw(&fg, DrawParam::default());
        }
        
        // 顶部边框
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(1.0),
            Rect::new(bar.x, bar.y, bar.width, bar.height),
            Color::from_rgba(255, 255, 255, 100),
        )?;
        canvas.draw(&border, DrawParam::default());
        
        // 百分比文字
        if bar.show_percent {
            let text = format!("Lv.{} - EXP: {:.1}%", status.level, percent * 100.0);
            let mut text_obj = Text::new(text);
            text_obj.set_scale(PxScale::from(12.0));
            
            let text_x = bar.x + bar.width / 2.0 - 60.0;
            let text_y = bar.y + 1.0;
            
            canvas.draw(
                &text_obj,
                DrawParam::default()
                    .dest([text_x, text_y])
                    .color(Color::WHITE),
            );
        }
        
        Ok(())
    }
    
    /// 绘制技能栏
    fn draw_skill_bar(
        ctx: &mut Context,
        canvas: &mut Canvas,
        bar: &SkillBar,
    ) -> GameResult {
        let mut x = bar.x;
        
        for (i, skill_opt) in bar.skills.iter().enumerate() {
            // 绘制技能槽背景
            let bg = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(x, bar.y, bar.slot_size, bar.slot_size),
                Color::from_rgb(40, 40, 40),
            )?;
            canvas.draw(&bg, DrawParam::default());
            
            // 绘制边框
            let border = Mesh::new_rectangle(
                ctx,
                DrawMode::stroke(1.0),
                Rect::new(x, bar.y, bar.slot_size, bar.slot_size),
                Color::WHITE,
            )?;
            canvas.draw(&border, DrawParam::default());
            
            // 如果有技能，绘制技能信息
            if let Some(skill) = skill_opt {
                // TODO: 绘制技能图标
                
                // 如果在冷却中，绘制冷却遮罩
                if skill.is_cooling_down() {
                    let cooldown_height = bar.slot_size * skill.cooldown_percent();
                    let cooldown_mask = Mesh::new_rectangle(
                        ctx,
                        DrawMode::fill(),
                        Rect::new(x, bar.y, bar.slot_size, cooldown_height),
                        Color::from_rgba(0, 0, 0, 128),
                    )?;
                    canvas.draw(&cooldown_mask, DrawParam::default());
                    
                    // 绘制冷却时间
                    let cooldown_text = format!("{:.1}", skill.remaining_cooldown);
                    let mut text_obj = Text::new(cooldown_text);
                    text_obj.set_scale(PxScale::from(10.0));
                    
                    canvas.draw(
                        &text_obj,
                        DrawParam::default()
                            .dest([x + bar.slot_size / 2.0 - 10.0, bar.y + bar.slot_size / 2.0 - 5.0])
                            .color(Color::WHITE),
                    );
                }
            }
            
            // 绘制快捷键提示 (F1-F8)
            let key_text = format!("F{}", i + 1);
            let mut text_obj = Text::new(key_text);
            text_obj.set_scale(PxScale::from(10.0));
            
            canvas.draw(
                &text_obj,
                DrawParam::default()
                    .dest([x + 2.0, bar.y + bar.slot_size - 12.0])
                    .color(Color::from_rgba(200, 200, 200, 200)),
            );
            
            x += bar.slot_size + bar.slot_spacing;
        }
        
        Ok(())
    }
    
    /// 绘制聊天窗口
    fn draw_chat_window(
        ctx: &mut Context,
        canvas: &mut Canvas,
        chat: &ChatWindow,
    ) -> GameResult {
        // 背景（半透明）
        let bg = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(chat.x, chat.y, chat.width, chat.height),
            Color::from_rgba(0, 0, 0, 128),
        )?;
        canvas.draw(&bg, DrawParam::default());
        
        // 边框
        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(1.0),
            Rect::new(chat.x, chat.y, chat.width, chat.height),
            Color::from_rgba(255, 255, 255, 128),
        )?;
        canvas.draw(&border, DrawParam::default());
        
        // 绘制最近的消息（从下往上）
        let line_height = 16.0;
        let max_lines = (chat.height / line_height) as usize;
        let start_index = if chat.messages.len() > max_lines {
            chat.messages.len() - max_lines
        } else {
            0
        };
        
        let mut y = chat.y + chat.height - line_height - 5.0;
        
        for message in chat.messages[start_index..].iter().rev() {
            if y < chat.y {
                break;
            }
            
            // 消息文本
            let text = if message.sender.is_empty() {
                message.content.clone()
            } else {
                format!("{}: {}", message.sender, message.content)
            };
            
            let mut text_obj = Text::new(text);
            text_obj.set_scale(PxScale::from(12.0));
            
            canvas.draw(
                &text_obj,
                DrawParam::default()
                    .dest([chat.x + 5.0, y])
                    .color(message.msg_type.get_color()),
            );
            
            y -= line_height;
        }
        
        Ok(())
    }
}
