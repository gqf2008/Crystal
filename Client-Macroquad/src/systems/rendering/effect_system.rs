use crate::components::{
    FloatingText, Health, HealthBarAnim, HoverHighlight, LibrarySprite, Monster, NameColor, NPC,
    NetworkObjectType, NetworkSync, Position, RenderPass, RenderStage, SpriteBlendMode,
};
use crate::game::GameResult;
use crate::systems::RenderSystem;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

fn argb_i32_to_color(argb: i32, alpha_mul: f32) -> Color {
    if argb == 0 {
        return Color::new(1.0, 1.0, 1.0, alpha_mul.clamp(0.0, 1.0));
    }
    let u = argb as u32;
    let a = ((u >> 24) & 0xFF) as f32 / 255.0;
    let r = ((u >> 16) & 0xFF) as f32 / 255.0;
    let g = ((u >> 8) & 0xFF) as f32 / 255.0;
    let b = (u & 0xFF) as f32 / 255.0;
    Color::new(r, g, b, (a * alpha_mul).clamp(0.0, 1.0))
}

fn draw_world_overlays(world: &hecs::World, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);

    let cam_zoom = world
        .query::<&crate::components::Camera>()
        .iter()
        .next()
        .map(|(_, c)| c.zoom)
        .unwrap_or(1.0)
        .max(0.0001);

    let (cam_pos_x, cam_pos_y, cam_screen_w, cam_screen_h) = world
        .query::<(&crate::components::Camera, &Position)>()
        .iter()
        .next()
        .map(|(_, (c, p))| (p.x, p.y, c.screen_width, c.screen_height))
        .unwrap_or((0.0, 0.0, screen_width(), screen_height()));
    let half_w = (cam_screen_w / 2.0) / cam_zoom;
    let half_h = (cam_screen_h / 2.0) / cam_zoom;
    let cull_margin = 120.0;
    let view_left = cam_pos_x - half_w - cull_margin;
    let view_right = cam_pos_x + half_w + cull_margin;
    let view_top = cam_pos_y - half_h - cull_margin;
    let view_bottom = cam_pos_y + half_h + cull_margin;

    let in_view = |x: f32, y: f32| -> bool {
        x >= view_left && x <= view_right && y >= view_top && y <= view_bottom
    };

    let draw_text_outline_world = |text: &str, x: f32, y: f32, font_size: f32, color: Color, outline: Color| {
        let d = (1.0 / cam_zoom).max(0.35 / cam_zoom);
        let offsets = [vec2(-d, 0.0), vec2(d, 0.0), vec2(0.0, -d), vec2(0.0, d)];
        for off in offsets {
            draw_text_cn(text, x + off.x, y + off.y, font_size, outline);
        }
        draw_text_cn(text, x, y, font_size, color);
    };

    let (hovered_npc_object_id, hovered_monster_object_id) = world
        .query::<&HoverHighlight>()
        .iter()
        .next()
        .map(|(_, hh)| (hh.npc_object_id, hh.monster_object_id))
        .unwrap_or((None, None));

    if let Some(hover_oid) = hovered_npc_object_id {
        for (_entity, (spr, pos, sync)) in world.query::<(&LibrarySprite, &Position, &NetworkSync)>().iter() {
            if sync.object_type != NetworkObjectType::NPC || sync.object_id != hover_oid {
                continue;
            }
            if !matches!(spr.blend_mode, SpriteBlendMode::Alpha) {
                continue;
            }

            let Some(info) = spr.library.get_texture(spr.texture_index()) else {
                continue;
            };
            let Some(tex) = info.image else {
                continue;
            };

            let base_x = pos.x + info.offset_x as f32;
            let base_y = pos.y + info.offset_y as f32;
            let d = (1.0 / cam_zoom).max(0.35 / cam_zoom);
            let outline = Color::new(1.0, 1.0, 1.0, (0.55 * alpha).clamp(0.0, 1.0));
            let offsets = [vec2(-d, 0.0), vec2(d, 0.0), vec2(0.0, -d), vec2(0.0, d)];
            for off in offsets {
                draw_texture_ex(
                    &tex,
                    base_x + off.x,
                    base_y + off.y,
                    outline,
                    DrawTextureParams { ..Default::default() },
                );
            }
        }
    }

    if let Some(hover_oid) = hovered_monster_object_id {
        for (_entity, (spr, pos, sync)) in world.query::<(&LibrarySprite, &Position, &NetworkSync)>().iter() {
            if sync.object_type != NetworkObjectType::Monster || sync.object_id != hover_oid {
                continue;
            }
            if !matches!(spr.blend_mode, SpriteBlendMode::Alpha) {
                continue;
            }

            let Some(info) = spr.library.get_texture(spr.texture_index()) else {
                continue;
            };
            let Some(tex) = info.image else {
                continue;
            };

            let base_x = pos.x + info.offset_x as f32;
            let base_y = pos.y + info.offset_y as f32;
            let d = (1.0 / cam_zoom).max(0.35 / cam_zoom);
            let outline = Color::new(1.0, 1.0, 1.0, (0.55 * alpha).clamp(0.0, 1.0));
            let offsets = [vec2(-d, 0.0), vec2(d, 0.0), vec2(0.0, -d), vec2(0.0, d)];
            for off in offsets {
                draw_texture_ex(
                    &tex,
                    base_x + off.x,
                    base_y + off.y,
                    outline,
                    DrawTextureParams { ..Default::default() },
                );
            }
        }
    }

    for (entity, (npc, pos)) in world.query::<(&NPC, &Position)>().iter() {
        if !in_view(pos.x, pos.y) {
            continue;
        }
        let name = npc.name.as_str();
        if name.is_empty() {
            continue;
        }

        let name_color_argb = world
            .get::<&NameColor>(entity)
            .ok()
            .map(|c| c.0)
            .unwrap_or(0);

        let lines: Vec<&str> = if name.contains('_') {
            name.split('_').filter(|s| !s.is_empty()).collect()
        } else {
            vec![name]
        };
        if lines.is_empty() {
            continue;
        }

        let font_size = 16.0;
        let base_y = pos.y - 40.0;
        let multi_offset = ((lines.len().saturating_sub(1)) as f32 * 10.0) / 2.0;

        for (i, line) in lines.iter().enumerate() {
            let dims = measure_text_cn(line, font_size);
            let x = pos.x - dims.width / 2.0;
            let y = base_y - multi_offset + (i as f32 * 12.0);

            let color = if i == 0 {
                argb_i32_to_color(name_color_argb, alpha)
            } else {
                Color::new(1.0, 1.0, 1.0, alpha)
            };
            let outline = Color::new(0.0, 0.0, 0.0, alpha);
            draw_text_outline_world(line, x, y, font_size, color, outline);
        }
    }

    for (entity, (monster, pos, hp)) in world.query::<(&Monster, &Position, &Health)>().iter() {
        if !in_view(pos.x, pos.y) {
            continue;
        }
        if monster.name.is_empty() {
            continue;
        }

        let font_size = 16.0;
        let name = monster.name.as_str();
        let dims = measure_text_cn(name, font_size);
        let x = pos.x - dims.width / 2.0;
        let y = pos.y - 54.0;
        draw_text_outline_world(
            name,
            x,
            y,
            font_size,
            Color::new(1.0, 1.0, 1.0, alpha),
            Color::new(0.0, 0.0, 0.0, alpha),
        );

        let bar_w = 46.0;
        let bar_h = 6.0;
        let bar_x = pos.x - bar_w / 2.0;
        let bar_y = pos.y - 46.0;

        let max = hp.max.max(1) as f32;
        let cur = world
            .get::<&HealthBarAnim>(entity)
            .ok()
            .map(|a| a.displayed)
            .unwrap_or(hp.current.max(0) as f32)
            .clamp(0.0, max);
        let pct = (cur / max).clamp(0.0, 1.0);

        draw_rectangle(
            bar_x,
            bar_y,
            bar_w,
            bar_h,
            Color::from_rgba(0, 0, 0, (0.55 * alpha * 255.0).clamp(0.0, 255.0) as u8),
        );
        draw_rectangle_lines(
            bar_x,
            bar_y,
            bar_w,
            bar_h,
            1.0 / cam_zoom,
            Color::new(0.0, 0.0, 0.0, alpha),
        );
        draw_rectangle(
            bar_x + 1.0 / cam_zoom,
            bar_y + 1.0 / cam_zoom,
            (bar_w - 2.0 / cam_zoom) * pct,
            bar_h - 2.0 / cam_zoom,
            Color::new(0.9, 0.1, 0.1, alpha),
        );
    }

    for (_entity, (ft, pos)) in world.query::<(&FloatingText, &Position)>().iter() {
        if !in_view(pos.x, pos.y) {
            continue;
        }
        if ft.text.is_empty() {
            continue;
        }
        let font_size = 18.0;
        let dims = measure_text_cn(&ft.text, font_size);
        let x = pos.x - dims.width / 2.0;
        let y = pos.y;
        draw_text_outline_world(
            &ft.text,
            x,
            y,
            font_size,
            Color::new(1.0, 1.0, 1.0, alpha),
            Color::new(0.0, 0.0, 0.0, alpha),
        );
    }
}
#[derive(ecs_macros::RenderSystem)]
pub struct EffectRenderSystem {
    add_blend_material: Material,
}

impl EffectRenderSystem {
    pub fn new() -> Self {
        let add_blend_material = load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../../shaders/default.vert"),
                fragment: include_str!("../../../shaders/default.frag"),
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

        Self { add_blend_material }
    }
}

impl RenderSystem for EffectRenderSystem {
    fn draw(&mut self, _world: &hecs::World) -> GameResult {
        let pass = _world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| *pass)
            .unwrap_or_default();

        // ghost pass 只画本地玩家，不画特效/叠加层
        if pass.local_only {
            return Ok(());
        }

        let alpha = pass.alpha.clamp(0.0, 1.0);

        // PostFront：只画世界叠加层
        if pass.stage == RenderStage::PostFront {
            draw_world_overlays(_world, alpha);
            return Ok(());
        }

        // UI：不画任何世界特效
        if pass.stage == RenderStage::Ui {
            return Ok(());
        }

        let draw_layer = |lib_sprite: &LibrarySprite, pos: &Position, tint: Color, offset: Vec2| -> bool {
            let Some(info) = lib_sprite.library.get_texture(lib_sprite.texture_index()) else {
                return false;
            };
            let Some(tex) = info.image else {
                return false;
            };
            let draw_x = pos.x + info.offset_x as f32 + offset.x;
            let draw_y = pos.y + info.offset_y as f32 + offset.y;
            draw_texture_ex(
                &tex,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams { ..Default::default() },
            );
            true
        };

        for (_entity, (spr, pos)) in _world.query::<(&LibrarySprite, &Position)>().iter() {
            let tint = Color::new(1.0, 1.0, 1.0, alpha);

            // 1) ADD 混合：当作“特效层”渲染（放在 SpriteRenderSystem 之后）
            if matches!(spr.blend_mode, SpriteBlendMode::Additive) {
                gl_use_material(&self.add_blend_material);
                let _ = draw_layer(spr, pos, tint, Vec2::ZERO);
                gl_use_default_material();
                continue;
            }
        }

        Ok(())
    }
}