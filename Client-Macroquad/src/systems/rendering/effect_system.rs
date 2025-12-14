use crate::components::{FloatingText, Health, HoverHighlight, LibrarySprite, NameColor, Position, RenderPass, SpriteBlendMode};
use crate::game::GameResult;
use crate::systems::RenderSystem;
use crate::ui::text_renderer::{draw_text_with_outline, measure_text_cn};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

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
        // ghost pass 只画本地玩家，不画特效
        if _world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| pass.local_only)
            .unwrap_or(false)
        {
            return Ok(());
        }

        let alpha = _world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|(_, pass)| pass.alpha)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);

        let hovered_npc_object_id = _world
            .query::<&HoverHighlight>()
            .iter()
            .next()
            .and_then(|(_, hh)| hh.npc_object_id);

        let cam_zoom = _world
            .query::<&crate::components::Camera>()
            .iter()
            .next()
            .map(|(_, c)| c.zoom)
            .unwrap_or(1.0)
            .max(0.0001);

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

        // ===== 1) NPC 名字常显（对齐 C# NPCObject.DrawName / MapObject.DrawName） =====
        // 规则：
        // - 名字含 "_" 则分行；首行使用 NameColor，其余白色
        // - 描边黑色
        // - 位置以“格子中心点”为水平中心（pos.x），纵向参考 C# 的 32px 偏移
        for (_entity, (npc, pos)) in _world.query::<(&crate::components::NPC, &Position)>().iter() {
            // ghost pass 不画；由 early return 已处理
            let name = npc.name.as_str();
            if name.is_empty() {
                continue;
            }

            let name_color_argb = _world
                .get::<&NameColor>(_entity)
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
            let base_y = pos.y - 40.0; // 近似对齐 C#：DisplayRectangle.Y - 32 + 8
            let multi_offset = ((lines.len().saturating_sub(1)) as f32 * 10.0) / 2.0;

            for (i, line) in lines.iter().enumerate() {
                let dims = measure_text_cn(line, font_size);
                let x = pos.x - dims.width / 2.0;
                let y = base_y - multi_offset + (i as f32 * 12.0);

                let color = if i == 0 {
                    argb_i32_to_color(name_color_argb, 1.0 * alpha)
                } else {
                    Color::new(1.0, 1.0, 1.0, alpha)
                };
                let outline = Color::new(0.0, 0.0, 0.0, alpha);
                draw_text_with_outline(line, x, y, font_size, color, outline);
            }
        }

        // ===== 1.5) 怪物名字 + 血条（最小对齐：可见性优先） =====
        // 说明：原版会根据 Target/Dead/Hidden 等状态控制显示。
        // 这里先实现“有 Health 就画血条”，并常显怪物名，便于验证掉血/命中闭环。
        for (_entity, (monster, pos, hp)) in _world
            .query::<(&crate::components::Monster, &Position, &Health)>()
            .iter()
        {
            if monster.name.is_empty() {
                continue;
            }

            let font_size = 16.0;
            let name = monster.name.as_str();
            let dims = measure_text_cn(name, font_size);
            let x = pos.x - dims.width / 2.0;
            let y = pos.y - 54.0;
            draw_text_with_outline(
                name,
                x,
                y,
                font_size,
                Color::new(1.0, 1.0, 1.0, alpha),
                Color::new(0.0, 0.0, 0.0, alpha),
            );

            // 血条：宽 46px，高 6px（跟随 world 坐标；macroquad Camera2D 会一起变换）
            let bar_w = 46.0;
            let bar_h = 6.0;
            let bar_x = pos.x - bar_w / 2.0;
            let bar_y = pos.y - 46.0;

            let max = hp.max.max(1) as f32;
            let cur = hp.current.max(0) as f32;
            let pct = (cur / max).clamp(0.0, 1.0);

            // 背景 + 边框
            draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.0, 0.0, 0.0, (0.55 * alpha).clamp(0.0, 1.0)));
            draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 1.0 / cam_zoom, Color::new(0.0, 0.0, 0.0, alpha));

            // 填充
            draw_rectangle(
                bar_x + 1.0 / cam_zoom,
                bar_y + 1.0 / cam_zoom,
                (bar_w - 2.0 / cam_zoom) * pct,
                bar_h - 2.0 / cam_zoom,
                Color::new(0.9, 0.1, 0.1, alpha),
            );
        }

        // ===== 1.6) 漂浮文本（伤害数字） =====
        for (_entity, (ft, pos)) in _world.query::<(&FloatingText, &Position)>().iter() {
            if ft.text.is_empty() {
                continue;
            }
            let font_size = 18.0;
            let dims = measure_text_cn(&ft.text, font_size);
            let x = pos.x - dims.width / 2.0;
            let y = pos.y;
            draw_text_with_outline(
                &ft.text,
                x,
                y,
                font_size,
                Color::new(1.0, 1.0, 1.0, alpha),
                Color::new(0.0, 0.0, 0.0, alpha),
            );
        }

        for (_entity, (spr, pos)) in _world.query::<(&LibrarySprite, &Position)>().iter() {
            let tint = Color::new(1.0, 1.0, 1.0, alpha);

            // 1) ADD 混合：当作“特效层”渲染（放在 SpriteRenderSystem 之后）
            if matches!(spr.blend_mode, SpriteBlendMode::Additive) {
                gl_use_material(&self.add_blend_material);
                let _ = draw_layer(spr, pos, tint, Vec2::ZERO);
                gl_use_default_material();
                continue;
            }

            // 2) NPC 悬停轮廓（只做描边，不重复绘制本体）
            let Some(hover_oid) = hovered_npc_object_id else {
                continue;
            };
            if !matches!(spr.blend_mode, SpriteBlendMode::Alpha) {
                continue;
            }
            let Ok(sync) = _world.get::<&crate::components::NetworkSync>(_entity) else {
                continue;
            };
            if sync.object_type != crate::components::NetworkObjectType::NPC || sync.object_id != hover_oid {
                continue;
            }

            // 轮廓粗细按屏幕像素近似：world_offset = px / zoom
            let o1 = 3.5 / cam_zoom;
            let o2 = 6.0 / cam_zoom;
            let outline_tint = Color::new(0.1, 1.0, 0.1, (0.95 * alpha).clamp(0.0, 1.0));
            let offsets1 = [
                vec2(-o1, 0.0),
                vec2(o1, 0.0),
                vec2(0.0, -o1),
                vec2(0.0, o1),
                vec2(-o1, -o1),
                vec2(o1, -o1),
                vec2(-o1, o1),
                vec2(o1, o1),
            ];
            let offsets2 = [
                vec2(-o2, 0.0),
                vec2(o2, 0.0),
                vec2(0.0, -o2),
                vec2(0.0, o2),
                vec2(-o2, -o2),
                vec2(o2, -o2),
                vec2(-o2, o2),
                vec2(o2, o2),
            ];

            for off in offsets1 {
                let _ = draw_layer(spr, pos, outline_tint, off);
            }
            for off in offsets2 {
                let _ = draw_layer(spr, pos, outline_tint, off);
            }
        }

        Ok(())
    }
}