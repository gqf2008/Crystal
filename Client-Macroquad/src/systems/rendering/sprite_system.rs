mod character;
// weapon 模块目前是占位（未接线）。装备相关绘制在 character 模块里完成。
mod weapon;

use crate::components::{Camera, FrontOcclusion, Position, RenderPass, RenderStage};
use crate::systems::RenderSystem;
// use ggez::{graphics::GraphicsContext, GameResult};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::sync::OnceLock;

#[derive(ecs_macros::RenderSystem)]
pub struct SpriteRenderSystem {
    add_blend_material: Material,
}

impl SpriteRenderSystem {
    pub fn new() -> Self {
        // 创建 ADD 混合材质 (dst + src * alpha)
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

    fn sprite_diag_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| std::env::var_os("CRYSTAL_SPRITE_DIAG").is_some())
    }
}

impl SpriteRenderSystem {
    /// 获取相机变换参数
    #[allow(dead_code)]
    fn get_camera_transform(world: &hecs::World) -> Option<(f32, f32, f32)> {
        let mut query = world.query::<(&Camera, &Position)>();
        if let Some((camera, cam_pos)) = query.iter().next() {
            Some((cam_pos.x, cam_pos.y, camera.zoom))
        } else {
            None
        }
    }

    /// 世界坐标 → 屏幕坐标
    #[allow(dead_code)]
    fn world_to_screen(
        world_x: f32,
        world_y: f32,
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> (f32, f32) {
        let relative_x = (world_x - cam_x) * zoom;
        let relative_y = (world_y - cam_y) * zoom;
        (
            screen_width / 2.0 + relative_x,
            screen_height / 2.0 + relative_y,
        )
    }
}

impl RenderSystem for SpriteRenderSystem {
    fn draw(
        &mut self,
        world: &hecs::World,
    ) -> crate::game::GameResult {
        // 诊断：如果连红点都没有，优先确认：
        // 1) SpriteRenderSystem 是否被调度到；2) 世界里是否存在 Player/LocalPlayer；3) 是否有 Camera。
        // 默认关闭（避免影响帧率），需要时用环境变量 CRYSTAL_SPRITE_DIAG=1 打开。
        // 即使打开，也只打印一次，避免刷屏。
        static SPRITE_DIAG_ONCE: OnceLock<()> = OnceLock::new();
        static SPRITE_DIAG_WHEN_PLAYER_EXISTS: OnceLock<()> = OnceLock::new();
        static SPRITE_DIAG_POST_FRONT_ONCE: OnceLock<()> = OnceLock::new();
        static SPRITE_DIAG_POST_FRONT_WHEN_PLAYER_EXISTS: OnceLock<()> = OnceLock::new();

        let pass = world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|pass| *pass)
            .unwrap_or_default();

        if Self::sprite_diag_enabled() {
            let _ = SPRITE_DIAG_ONCE.set(()).map(|_| {
                let player_count = world.query::<&crate::components::Player>().iter().count();
                let local_player_count = world.query::<&crate::components::LocalPlayer>().iter().count();
                let cam_count = world.query::<&crate::components::Camera>().iter().count();
                let pos_count = world.query::<&crate::components::Position>().iter().count();
                let occluded = world
                    .query::<&crate::components::FrontOcclusion>()
                    .iter()
                    .next()
                    .map(|o| o.local_player_occluded)
                    .unwrap_or(false);
                println!(
                    "[DIAG][SpriteRenderSystem] draw called: stage={:?} alpha={} local_only={} players={} local_players={} cameras={} positions={} occluded={}",
                    pass.stage,
                    pass.alpha,
                    pass.local_only,
                    player_count,
                    local_player_count,
                    cam_count,
                    pos_count,
                    occluded
                );
            });
        }

        // 诊断：确认 PostFront pass 是否执行。
        if Self::sprite_diag_enabled() && pass.stage == RenderStage::PostFront {
            let player_count = world.query::<&crate::components::Player>().iter().count();
            let local_player_count = world.query::<&crate::components::LocalPlayer>().iter().count();
            let occluded = world
                .query::<&crate::components::FrontOcclusion>()
                .iter()
                .next()
                .map(|o| o.local_player_occluded)
                .unwrap_or(false);

            let _ = SPRITE_DIAG_POST_FRONT_ONCE.set(()).map(|_| {
                println!(
                    "[DIAG][SpriteRenderSystem] PostFront pass: players={} local_players={} occluded={} (ghost should draw if occluded)",
                    player_count,
                    local_player_count,
                    occluded
                );
            });

            if player_count > 0 {
                let _ = SPRITE_DIAG_POST_FRONT_WHEN_PLAYER_EXISTS.set(()).map(|_| {
                    println!(
                        "[DIAG][SpriteRenderSystem] PostFront with player: players={} local_players={} occluded={} => will_draw_ghost={}",
                        player_count,
                        local_player_count,
                        occluded,
                        occluded
                    );
                });
            }
        }

        // 如果首帧 draw 时玩家还没生成，后续玩家出现时再打一次关键坐标。
        if Self::sprite_diag_enabled() && pass.stage == RenderStage::Normal {
            let player_count = world.query::<&crate::components::Player>().iter().count();
            if player_count > 0 {
                let _ = SPRITE_DIAG_WHEN_PLAYER_EXISTS.set(()).map(|_| {
                    let first_player_pos = world
                        .query::<(&crate::components::Player, &crate::components::Position)>()
                        .iter()
                        .next()
                        .map(|(_p, pos)| (pos.x, pos.y));

                    let cam_pos = world
                        .query::<(&crate::components::Camera, &crate::components::Position)>()
                        .iter()
                        .next()
                        .map(|(_c, pos)| (pos.x, pos.y));

                    let occluded = world
                        .query::<&crate::components::FrontOcclusion>()
                        .iter()
                        .next()
                        .map(|o| o.local_player_occluded)
                        .unwrap_or(false);

                    println!(
                        "[DIAG][SpriteRenderSystem] players now exist: players={} first_player_pos={:?} camera_pos={:?} occluded={}",
                        player_count,
                        first_player_pos,
                        cam_pos,
                        occluded
                    );
                });
            }
        }

        match pass.stage {
            RenderStage::Normal => {
                // 正常世界渲染
                self.draw_character(world, &self.add_blend_material, pass.alpha, pass.local_only)?;
            }
            RenderStage::PostFront => {
                // PostFront：如果本地玩家被前景遮挡，则画一层 ghost（半透明本地玩家）
                let occluded = world
                    .query::<&FrontOcclusion>()
                    .iter()
                    .next()
                    .map(|o| o.local_player_occluded)
                    .unwrap_or(false);

                if occluded {
                    // 前景遮挡：画一层半透明的本地玩家（类似原版“被遮挡仍可见”的观感）。
                    const PLAYER_GHOST_ALPHA: f32 = 0.55;
                    self.draw_character(world, &self.add_blend_material, PLAYER_GHOST_ALPHA, true)?;
                }
            }
            RenderStage::Ui => {
                // UI stage 不画世界精灵
            }
        }
        Ok(())
    }
}
