//! 渲染系统 - 统一处理所有实体的渲染
//! 
//! 替代原来的draw_xxx方法，基于组件自动渲染

use hecs::World;
use crate::graphics::{get_library, LibraryName, Canvas};
use super::super::components::*;

/// 渲染LoginScene中的所有实体
pub fn render_all(
    world: &World,
    ctx: &mut ggez::Context,
    canvas: &mut Canvas,
) -> anyhow::Result<()> {
    // 0. 绘制半透明背景遮罩（如果有对话框打开）
    if world.query::<&DialogEntity>().iter().next().is_some() {
        use ggez::graphics::{Mesh, DrawMode, Rect, Color, DrawParam};
        if let Ok(bg_mesh) = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(0.0, 0.0, 1024.0, 768.0),
            Color::from_rgba(0, 0, 0, 128),  // 半透明黑色
        ) {
            canvas.draw(&bg_mesh, DrawParam::default());
        }
    }
    
    // 1. 渲染静态精灵（背景、对话框底图等）
    render_sprites(world, ctx, canvas)?;
    
    // 2. 渲染动画精灵（背景动画等）
    render_animated_sprites(world, ctx, canvas)?;
    
    // 3. 渲染按钮
    render_buttons(world, ctx, canvas)?;
    
    // 4. 渲染输入框
    render_text_inputs(world, ctx, canvas)?;
    
    // 5. 渲染标签文本
    render_labels(world, ctx, canvas)?;
    
    Ok(())
}

/// 渲染所有静态精灵
fn render_sprites(
    world: &World,
    ctx: &mut ggez::Context,
    canvas: &mut Canvas,
) -> anyhow::Result<()> {
    for (_entity, (pos, sprite, visible)) in world
        .query::<(&Position, &Sprite, &Visible)>()
        .iter()
    {
        if !visible.0 || !sprite.visible {
            continue;
        }

        if let Some(library) = get_library(sprite.library.clone()) {
            let mut library_lock = library.lock().unwrap();
            let _ = library_lock.draw_with_color(
                ctx,
                canvas,
                sprite.index as usize,
                pos.x,
                pos.y,
                ggez::graphics::Color::WHITE,
                false,
            );
        }
    }
    Ok(())
}

/// 渲染所有动画精灵
fn render_animated_sprites(
    world: &World,
    ctx: &mut ggez::Context,
    canvas: &mut Canvas,
) -> anyhow::Result<()> {
    for (_entity, (pos, anim_sprite, visible)) in world
        .query::<(&Position, &AnimatedSprite, &Visible)>()
        .iter()
    {
        if !visible.0 {
            continue;
        }

        if let Some(library) = get_library(anim_sprite.library.clone()) {
            let mut library_lock = library.lock().unwrap();
            let index = anim_sprite.current_index();
            let _ = library_lock.draw_with_color(
                ctx,
                canvas,
                index as usize,
                pos.x,
                pos.y,
                ggez::graphics::Color::WHITE,
                false,
            );
        }
    }
    Ok(())
}

/// 渲染所有按钮
fn render_buttons(
    world: &World,
    ctx: &mut ggez::Context,
    canvas: &mut Canvas,
) -> anyhow::Result<()> {
    for (_entity, (pos, button, sprite, visible)) in world
        .query::<(&Position, &Button, &Sprite, &Visible)>()
        .iter()
    {
        if !visible.0 {
            continue;
        }

        if let Some(library) = get_library(sprite.library.clone()) {
            let mut library_lock = library.lock().unwrap();
            let index = button.current_index();
            let _ = library_lock.draw_with_color(
                ctx,
                canvas,
                index as usize,
                pos.x,
                pos.y,
                ggez::graphics::Color::WHITE,
                false,
            );

            // 调试：绘制按钮边界框（可选）
            #[cfg(debug_assertions)]
            if let Ok(bounds) = world.get::<&Bounds>(_entity) {
                draw_debug_bounds(ctx, canvas, &bounds, button.hovered);
            }
        }
    }
    Ok(())
}

/// 渲染所有输入框
fn render_text_inputs(
    world: &World,
    ctx: &mut ggez::Context,
    canvas: &mut Canvas,
) -> anyhow::Result<()> {
    use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor, Mesh, DrawMode, Rect};

    for (_entity, (pos, size, text_input, visible)) in world
        .query::<(&Position, &Size, &TextInput, &Visible)>()
        .iter()
    {
        if !visible.0 {
            continue;
        }

        // 绘制输入框背景
        let bg_color = if text_input.focused {
            GgezColor::from_rgb(255, 255, 200) // 聚焦时淡黄色
        } else if text_input.valid {
            GgezColor::from_rgb(240, 240, 240) // 有效时浅灰色
        } else {
            GgezColor::from_rgb(255, 200, 200) // 无效时淡红色
        };

        if let Ok(bg_mesh) = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(pos.x, pos.y, size.width, size.height),
            bg_color,
        ) {
            canvas.draw(&bg_mesh, DrawParam::default());
        }

        // 绘制边框
        let border_color = if text_input.focused {
            GgezColor::from_rgb(255, 200, 0) // 聚焦时金色边框
        } else {
            GgezColor::from_rgb(128, 128, 128) // 普通灰色边框
        };

        if let Ok(border_mesh) = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            Rect::new(pos.x, pos.y, size.width, size.height),
            border_color,
        ) {
            canvas.draw(&border_mesh, DrawParam::default());
        }

        // 绘制文本
        let display_text = if text_input.password {
            "*".repeat(text_input.text.len())
        } else {
            text_input.text.clone()
        };

        let text = Text::new(
            TextFragment::new(&display_text)
                .font("AlibabaPuHuiTi")
                .scale(18.0)
        );
        canvas.draw(
            &text,
            DrawParam::default()
                .dest([pos.x + 5.0, pos.y + 3.0])
                .color(GgezColor::BLACK),
        );

        // 绘制光标（如果聚焦）
        if text_input.focused {
            let cursor_x = pos.x + 5.0 + text_input.text.len() as f32 * 9.0;
            if let Ok(cursor) = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(cursor_x, pos.y + 3.0, 2.0, 14.0),
                GgezColor::BLACK,
            ) {
                canvas.draw(&cursor, DrawParam::default());
            }
        }
    }
    Ok(())
}

/// 渲染所有标签文本
fn render_labels(
    world: &World,
    ctx: &mut ggez::Context,
    canvas: &mut Canvas,
) -> anyhow::Result<()> {
    use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor};

    for (_entity, (pos, label, visible)) in world
        .query::<(&Position, &Label, &Visible)>()
        .iter()
    {
        if !visible.0 {
            continue;
        }

        let text = Text::new(
            TextFragment::new(&label.text)
                .font(&label.font)
                .scale(label.size)
        );

        let color = GgezColor::from_rgba(
            label.color[0],
            label.color[1],
            label.color[2],
            label.color[3],
        );

        canvas.draw(&text, DrawParam::default().dest([pos.x, pos.y]).color(color));
    }
    Ok(())
}

/// 调试：绘制边界框
#[cfg(debug_assertions)]
fn draw_debug_bounds(
    ctx: &mut ggez::Context,
    canvas: &mut Canvas,
    bounds: &Bounds,
    hovered: bool,
) {
    use ggez::graphics::{Mesh, DrawMode, Rect, DrawParam, Color};

    let color = if hovered {
        Color::from_rgb(255, 0, 0) // 悬停时红色
    } else {
        Color::from_rgba(0, 255, 0, 128) // 普通时半透明绿色
    };

    if let Ok(rect_mesh) = Mesh::new_rectangle(
        ctx,
        DrawMode::stroke(1.0),
        Rect::new(bounds.x, bounds.y, bounds.width, bounds.height),
        color,
    ) {
        canvas.draw(&rect_mesh, DrawParam::default());
    }
}
