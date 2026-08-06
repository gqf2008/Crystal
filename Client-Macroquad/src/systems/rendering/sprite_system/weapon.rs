// ============================================================================
// Weapon Render Module - 武器渲染模块（SpriteRenderSystem 内部分拆）
// ============================================================================
//
// 说明：
// - “武器 + 武器特效”在渲染时与人物帧号/朝向遮挡(PostFront)/骑乘帧映射强耦合。
// - 因此它仍然由 `SpriteRenderSystem::render_character()` 控制绘制顺序，
//   但把重复的绘制逻辑抽到本文件，主要目的是控制 `character.rs` 文件体积。

use super::SpriteRenderSystem;
use crate::components::Position;
use crate::resources::LibraryName;
use macroquad::prelude::*;

impl SpriteRenderSystem {
    /// 绘制武器层 + 武器特效（加色混合）。
    ///
    /// - `candidates` 是候选帧号（按优先级从高到低）；
    /// - `draw_layer`/`draw_layer_additive` 由调用方提供，复用同一套“取贴图+offset+draw”逻辑；
    /// - 返回值：本次是否绘制了任意一层（武器或特效）。
    pub(super) fn draw_weapon_with_effect(
        draw_layer: &impl Fn(LibraryName, i32, &Position, Color) -> bool,
        draw_layer_additive: &impl Fn(&Material, LibraryName, i32, &Position, Color) -> bool,
        add_blend_material: &Material,
        actor_pos: &Position,
        tint: Color,
        weapon_lib: LibraryName,
        weapon_index: usize,
        weapon_effect_index_opt: Option<usize>,
        candidates: [i32; 6],
    ) -> bool {
        let mut drew_any = false;

        let mut weapon_drawn = false;
        let mut weapon_frame_used: Option<i32> = None;
        for f in candidates {
            if draw_layer(weapon_lib, f, actor_pos, tint) {
                weapon_drawn = true;
                weapon_frame_used = Some(f);
                break;
            }
        }

        // 兜底：首次扫描前 64 帧找一个能画出来的帧（避免骑乘帧/资源布局不一致导致完全无武器）。
        if !weapon_drawn {
            static WEAPON_PROBE_ONCE: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
            let probe = WEAPON_PROBE_ONCE.get_or_init(|| {
                for i in 0..64 {
                    if weapon_lib
                        .get_texture(i)
                        .and_then(|info| info.image)
                        .is_some()
                    {
                        return Some(i as i32);
                    }
                }
                None
            });
            if let Some(f) = *probe {
                if draw_layer(weapon_lib, f, actor_pos, tint) {
                    weapon_drawn = true;
                    weapon_frame_used = Some(f);
                }
            }
        }

        static WEAPON_DRAW_DIAG_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        if Self::sprite_diag_enabled() {
            let _ = WEAPON_DRAW_DIAG_ONCE.set(()).map(|_| {
                println!(
                    "[DIAG][SpriteRenderSystem] weapon_drawn={} weapon_index={} frame_used={:?}",
                    weapon_drawn, weapon_index, weapon_frame_used
                );
            });
        }

        drew_any |= weapon_drawn;

        if let Some(effect_index) = weapon_effect_index_opt {
            let effect_tint = Color::new(
                tint.r,
                tint.g,
                tint.b,
                (tint.a * Self::WEAPON_EFFECT_ALPHA).clamp(0.0, 1.0),
            );
            let effect_lib = LibraryName::CWeaponEffect(effect_index);
            let mut effect_drawn = false;
            let mut effect_frame_used: Option<i32> = None;
            for f in candidates {
                if draw_layer_additive(add_blend_material, effect_lib, f, actor_pos, effect_tint) {
                    effect_drawn = true;
                    effect_frame_used = Some(f);
                    break;
                }
            }
            static WEAPON_EFFECT_DIAG_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            if Self::sprite_diag_enabled() {
                let _ = WEAPON_EFFECT_DIAG_ONCE.set(()).map(|_| {
                    println!(
                        "[DIAG][SpriteRenderSystem] weapon_effect_drawn={} effect_index={} frame_used={:?}",
                        effect_drawn, effect_index, effect_frame_used
                    );
                });
            }

            drew_any |= effect_drawn;
        }

        drew_any
    }
}
