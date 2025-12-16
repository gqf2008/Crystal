// GameScene - 游戏主场景
//
// 快捷键:
//   I = 背包, C = 角色, B = 快捷栏, S = 商城
//   F1-F6 = 快捷栏技能
//   ESC = 返回角色选择

use crate::game::{GameContext, GameResult};
use crate::resources::initialize_all_libraries;
use crate::scenes::{Scene, SceneTransition};
use crate::systems::presentation::{DialogSystem, HUDSystem, MinimapSystem, UISystem};
use crate::systems::rendering::{EffectRenderSystem, SpriteRenderSystem, UIRenderSystem};
use crate::ui::ui_state::UiState;
use crate::{
    components::{
        Camera as EcsCamera, CameraMode, Draggable, LocalPlayer, MapData, Position, RenderConfig,
        RenderPass, RenderStage, ResourceInitState, SceneExitBlock, TimeTracker,
    },
    systems::{
        priority, AnimationSystem, CameraBoundsSystem, CameraFollowSystem, CameraSpaceGateSystem,
        CameraSystem, CollisionSystem, CombatSystem, FrameEndSystem, HealthRegenSystem,
        LocalPlayerAiSystem, MountStateSyncSystem, MovementSystem, PathfindingSystem, PlayerControlSystem, SkillSystem,
        SystemScheduler, TimeTickSystem,
    },
};
use macroquad::prelude::*;

/// 游戏主场景 - 集成所有混合对话框
pub struct GameScene {
    map_camera: Camera2D,
    map_camera_position: Vec2,
    map_zoom: f32,

    // ECS（ggez 版本同构）：先最小接入 update，不影响现有渲染链路
    ecs_ctx: GameContext,
    ecs_scheduler: SystemScheduler,
    ecs_camera_entity: Option<hecs::Entity>,
    ecs_local_player_entity: Option<hecs::Entity>,
    ecs_time_entity: Option<hecs::Entity>,
    ecs_render_pass_entity: Option<hecs::Entity>,
}

impl GameScene {
    /// 调试/验证入口：允许外部（例如 `test_game_scene`）直接操作 ECS 世界。
    ///
    /// 目的：做最小的“可视化验证器”（刷怪、切换 direction/stage），不引入额外 UI 面板。
    pub fn debug_ecs_ctx_mut(&mut self) -> &mut GameContext {
        &mut self.ecs_ctx
    }

    fn is_resources_initialized(&self) -> bool {
        let Some(pass_entity) = self.ecs_render_pass_entity else {
            return false;
        };
        self.ecs_ctx
            .world
            .get::<&ResourceInitState>(pass_entity)
            .map(|s| s.initialized)
            .unwrap_or(false)
    }

    fn can_escape_exit(&self) -> bool {
        let Some(pass_entity) = self.ecs_render_pass_entity else {
            return true;
        };
        self.ecs_ctx
            .world
            .get::<&SceneExitBlock>(pass_entity)
            .map(|b| !b.block_escape_exit)
            .unwrap_or(true)
    }

    pub fn new() -> Self {
        // Camera2D 初始值（真实参数会在地图加载后更新）
        let map_camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(
                2.0 / screen_width().max(1.0),
                2.0 / screen_height().max(1.0),
            ),
            offset: vec2(0.0, 0.0),
            render_target: None,
            rotation: 0.0,
            viewport: None,
        };

        let mut ecs_scheduler = SystemScheduler::new();
        ecs_scheduler
            // UI 输入阻塞必须早于 PlayerControlSystem 执行，因此将 UIRenderSystem.update 提前。
            // draw 仍然只在 RenderStage::Ui 时生效。
            .add_render_system_staged(
                UIRenderSystem::new(),
                priority::INPUT + 1,
                crate::systems::RenderStageMask::UI,
            )
            .add_system(crate::systems::NetworkSystem::default(), priority::NETWORK)
            .add_system(
                crate::systems::NetworkApplySystem::default(),
                priority::NETWORK_APPLY,
            )
            .add_system(
                crate::systems::MapBootstrapSystem::default(),
                priority::MAP_BOOTSTRAP,
            )
            .add_system(crate::systems::MapLoadSystem, priority::MAP_LOAD)
            .add_system(TimeTickSystem::default(), priority::GAME_EVENT)
            .add_system(LocalPlayerAiSystem::default(), priority::LOCAL_PLAYER_AI)
            .add_system(PlayerControlSystem::new(), priority::PLAYER_CONTROL)
            // 战斗/技能/自然回复：先接入闭环（目前 test_game_scene 默认不会触发）
            .add_system(CombatSystem::default(), priority::COMBAT)
            .add_system(SkillSystem::default(), priority::SKILL)
            .add_system(HealthRegenSystem, priority::REGEN)
            .add_system(PathfindingSystem::new(), priority::PATHFINDING)
            .add_system(MovementSystem, priority::MOVEMENT)
            .add_system(CollisionSystem::new(), priority::COLLISION)
            .add_system(
                crate::systems::PositionInterpolationSystem::default(),
                priority::REMOTE_INTERPOLATION,
            )
            .add_system(MountStateSyncSystem::new(), priority::MOUNT_STATE_SYNC)
            .add_system(AnimationSystem::new(), priority::ANIMATION)
            .add_system(
                crate::systems::presentation::HealthBarAnimSystem::default(),
                priority::HEALTH_BAR_ANIM,
            )
            .add_system(crate::systems::ParticleSystem, priority::PARTICLE)
            .add_system(
                crate::systems::presentation::SoundSystem::default(),
                priority::SOUND,
            )
            .add_system(
                crate::systems::FloatingTextSystem::default(),
                priority::PARTICLE,
            )
            .add_system(
                CameraSpaceGateSystem::default(),
                priority::CAMERA_SPACE_GATE,
            )
            .add_system(CameraFollowSystem, priority::CAMERA_FOLLOW)
            .add_system(CameraSystem::new(), priority::CAMERA)
            .add_system(CameraBoundsSystem::default(), priority::CAMERA_BOUNDS)
            // 表现层 UI 系统（规划：UI/HUD/Minimap/Dialog）
            .add_system(UISystem::new(), priority::UI)
            .add_system(HUDSystem::new(), priority::HUD)
            .add_system(MinimapSystem::new(), priority::MINIMAP)
            .add_system(DialogSystem::new(), priority::DIALOG)
            // ECS 渲染系统：先最小接入 SpriteRenderSystem（角色/坐骑/武器特效）
            .add_render_system_staged(
                crate::systems::rendering::MapRenderSystem::new(),
                priority::MAP_RENDER,
                crate::systems::RenderStageMask::NORMAL
                    .union(crate::systems::RenderStageMask::POST_FRONT),
            )
            .add_render_system_staged(
                SpriteRenderSystem::new(),
                priority::SPRITE_RENDER,
                crate::systems::RenderStageMask::NORMAL
                    .union(crate::systems::RenderStageMask::POST_FRONT),
            )
            .add_render_system_staged(
                EffectRenderSystem::new(),
                priority::EFFECT_RENDER,
                crate::systems::RenderStageMask::NORMAL
                    .union(crate::systems::RenderStageMask::POST_FRONT),
            )
            .add_render_system_staged(
                crate::systems::dbug::DebugSystem::new(),
                priority::DEBUG,
                crate::systems::RenderStageMask::POST_FRONT
                    .union(crate::systems::RenderStageMask::UI),
            )
            .add_system(FrameEndSystem::default(), priority::FRAME_END);

        let mut ecs_ctx = GameContext::new();
        // 提前创建 RenderPass 单例：保证 UI/Debug 叠加层在资源未初始化时也能渲染（例如“加载中”）。
        let ecs_render_pass_entity = ecs_ctx.world.spawn((
            RenderPass::default(),
            crate::components::FrontOcclusion::default(),
            crate::components::HoverHighlight::default(),
            crate::components::ActiveNpc::default(),
            crate::components::NpcCallCooldown::default(),
            crate::components::UiWorldInputBlock::default(),
            UiState::new(),
            ResourceInitState::default(),
            SceneExitBlock::default(),
        ));

        Self {
            map_camera,
            map_camera_position: vec2(0.0, 0.0),
            map_zoom: 1.0,

            ecs_ctx,
            ecs_scheduler,
            ecs_camera_entity: None,
            ecs_local_player_entity: None,
            ecs_time_entity: None,
            ecs_render_pass_entity: Some(ecs_render_pass_entity),
        }
    }

    /// 根据窗口尺寸自动调整有效缩放。
    ///
    /// 目的：统一缩放参数的来源，避免渲染/输入/相机 clamp 各用各的 zoom。
    ///
    /// 注意：这里不再做“随窗口尺寸自动放大 zoom”的视野钉死策略。
    /// 用户期望：窗口变大 = 视野变大；性能优化应在渲染层解决。
    fn effective_map_zoom(&self) -> f32 {
        if let Some(cam_entity) = self.ecs_camera_entity {
            if let Ok(cam) = self.ecs_ctx.world.get::<&EcsCamera>(cam_entity) {
                return cam.zoom;
            }
        }
        self.map_zoom
    }

    fn effective_map_camera_position(&self) -> Vec2 {
        if let Some(cam_entity) = self.ecs_camera_entity {
            if let Ok(pos) = self.ecs_ctx.world.get::<&Position>(cam_entity) {
                return vec2(pos.x, pos.y);
            }
        }
        self.map_camera_position
    }

    fn ensure_ecs_bootstrap(&mut self) {
        if !self.is_resources_initialized() {
            return;
        }

        // 1) 地图数据由 MapBootstrapSystem / MapLoadSystem 负责创建/更新。

        // 2) 相机实体（PlayerControlSystem 需要 Camera + Position 来做屏幕->世界坐标转换）
        let map_center = {
            let mut q = self.ecs_ctx.world.query::<&MapData>();
            q.iter()
                .next()
                .map(|(_, m)| vec2(m.width as f32 * 48.0 * 0.5, m.height as f32 * 32.0 * 0.5))
        };

        if self.ecs_camera_entity.is_none() {
            let (sw, sh) = self.ecs_ctx.drawable_size();
            let initial = map_center.unwrap_or(self.map_camera_position);
            let entity = self.ecs_ctx.world.spawn((
                EcsCamera::new(sw, sh),
                Draggable::default(),
                Position::new(initial.x, initial.y),
                CameraMode::FollowPlayer,
            ));
            self.ecs_camera_entity = Some(entity);
        } else if let (Some(cam_entity), Some(center)) = (self.ecs_camera_entity, map_center) {
            // 地图刚创建但还没收到玩家数据时，把相机定位到地图中心，避免一直盯在 (0,0)。
            // 一旦 LocalPlayer 出现，CameraFollowSystem 会接管。
            let has_local_player = self
                .ecs_ctx
                .world
                .query::<&LocalPlayer>()
                .iter()
                .next()
                .is_some();

            if !has_local_player {
                if let Ok(mut pos) = self.ecs_ctx.world.get::<&mut Position>(cam_entity) {
                    let is_default = pos.x.abs() < 0.001 && pos.y.abs() < 0.001;
                    if is_default {
                        pos.x = center.x;
                        pos.y = center.y;
                        self.map_camera_position = center;
                    }
                }
            }
        }

        // 3) 渲染/相机配置（CameraSystem 会读取 RenderConfig.enable_camera_drag）
        if self
            .ecs_ctx
            .world
            .query::<&RenderConfig>()
            .iter()
            .next()
            .is_none()
        {
            self.ecs_ctx.world.spawn((RenderConfig::default(),));
        }

        // 4) 时间跟踪实体（AnimationSystem 需要 animation_count 驱动帧变化）
        if self.ecs_time_entity.is_none() {
            let entity = self.ecs_ctx.world.spawn((TimeTracker::default(),));
            self.ecs_time_entity = Some(entity);
        }

        // 5) RenderPass/UiState/ResourceInitState：已在 new() 阶段创建单例。

        // 6) 本地玩家：由网络（真服/MockNetwork）落地；这里仅做“发现并缓存”，避免重复生成。
        if self.ecs_local_player_entity.is_none() {
            let mut q = self.ecs_ctx.world.query::<&LocalPlayer>();
            if let Some((e, _)) = q.iter().next() {
                self.ecs_local_player_entity = Some(e);
            }
        }
    }

    /// 触发一次 ECS 渲染 pass。
    ///
    /// 职责很窄：
    /// - 把本次 pass 的参数写入 `RenderPass` 单例（`alpha/local_only/stage`）
    /// - 调用调度器的 `draw()`，由各个 RenderSystem 按 `stage` 自行绘制
    fn draw_ecs_pass(&mut self, alpha: f32, local_only: bool, stage: RenderStage) -> GameResult {
        let Some(pass_entity) = self.ecs_render_pass_entity else {
            return Ok(());
        };

        if let Ok(mut pass) = self.ecs_ctx.world.get::<&mut RenderPass>(pass_entity) {
            pass.alpha = alpha;
            pass.local_only = local_only;
            pass.stage = stage;
        }

        self.ecs_scheduler.draw(&self.ecs_ctx.world)
    }

    fn update_map_camera(&mut self) {
        // 像素对齐（抗闪烁）：
        // Linear 过滤 + 子像素相机移动会导致阴影/暗部出现“深浅闪烁”(shimmering)。
        // 将渲染相机 target 对齐到 1/map_zoom 的网格（世界像素）上，可显著稳定采样。
        let zoom = self.effective_map_zoom().max(0.0001);
        let cam_pos = self.effective_map_camera_position();
        let snapped_target = vec2(
            (cam_pos.x * zoom).round() / zoom,
            (cam_pos.y * zoom).round() / zoom,
        );
        self.map_camera.target = snapped_target;
        let sw = screen_width().max(1.0);
        let sh = screen_height().max(1.0);
        self.map_camera.zoom = vec2(2.0 / sw * zoom, 2.0 / sh * zoom);
    }

    /// 初始化资源路径与地图库（UI 纹理由 UIRenderSystem 负责）
    pub fn load_textures(&mut self) {
        println!("🎮 GameScene: 初始化资源...");

        // 资源根目录：使用绝对路径，避免从不同工作目录启动时找不到 Data/
        // 例如：从仓库根目录 `cargo run -p client-macroquad --bin test_game_scene`
        // 或从 `Client-Macroquad/` 目录启动都应能正常加载。
        let data_dir = format!("{}/Data", env!("CARGO_MANIFEST_DIR"));
        crate::resources::resource_manager::set_data_path(&data_dir);
        // 初始化 Libraries（包含 UI/特效/物品/角色/武器/坐骑 + MapLibs）
        // 否则会出现“地图能画，但人物/装备/坐骑贴图为空”的现象。
        println!("📚 GameScene: 初始化游戏资源库...");
        let libs_ok = initialize_all_libraries(&data_dir).is_ok();

        if let Some(pass_entity) = self.ecs_render_pass_entity {
            if let Ok(mut s) = self
                .ecs_ctx
                .world
                .get::<&mut ResourceInitState>(pass_entity)
            {
                s.initialized = libs_ok;
            }
        }

        // 地图加载完成后，立即完成 ECS 最小引导（MapData/Camera/LocalPlayer）
        self.ensure_ecs_bootstrap();

        println!("✅ GameScene: 资源初始化完成");
    }

    // 地图的视觉加载/切换由 ECS MapRenderSystem 接管。
}

impl Default for GameScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for GameScene {
    fn name(&self) -> &str {
        "游戏场景"
    }

    fn on_enter(&mut self) -> GameResult {
        println!("🎬 进入游戏场景");

        // 场景间移交网络连接：SelectScene 已把 NetContext 放入全局，这里接管到 ECS。
        if let Some(net) = crate::network::take_global_net() {
            self.ecs_ctx.set_net(net);

			// 从 config.ini 同步运行时手感参数（例如远程插值时长）
			let cfg = crate::network::load_network_runtime_config();
			self.ecs_ctx.session.remote_player_walk_interp_secs = (cfg.remote_interp_walk_ms as f32) / 1000.0;
			self.ecs_ctx.session.remote_player_run_interp_secs = (cfg.remote_interp_run_ms as f32) / 1000.0;

            // 目前本地玩家移动仍由客户端 MovementSystem 驱动；开启 server_authoritative_movement
            // 会导致“本地移动 + 服务器回包纠偏”双重驱动，从而出现抖动/乱跳（坐骑更明显）。
            self.ecs_ctx.session.server_authoritative_movement = false;
            self.ecs_ctx.session.sync_movement_intent_to_server = true;
            self.ecs_ctx.session.server_authoritative_combat = true;
        }

        // 如果没有连接（例如 test_game_scene 直接进 GameScene），且配置允许 mock，则自动接入 MockNetwork。
        let mut auto_start_game = false;
        if self.ecs_ctx.net.is_none() {
            let cfg = crate::network::load_network_runtime_config();
            if cfg.use_mock {
                if let Ok(net) = crate::network::NetworkBuilder::new(cfg.server_addr)
                    .with_mock(true)
                    .build()
                {
                    self.ecs_ctx.set_net(net);

					// 从 config.ini 同步远程插值时长（用于 multi-remote AI）
					self.ecs_ctx.session.remote_player_walk_interp_secs = (cfg.remote_interp_walk_ms as f32) / 1000.0;
					self.ecs_ctx.session.remote_player_run_interp_secs = (cfg.remote_interp_run_ms as f32) / 1000.0;

                    // Mock 场景下默认使用本地移动（避免双驱动抖动）。
                    self.ecs_ctx.session.server_authoritative_movement = false;
                    self.ecs_ctx.session.sync_movement_intent_to_server = true;
                    self.ecs_ctx.session.server_authoritative_combat = true;
                    auto_start_game = true;
                }
            }
        }

        // 自动触发 StartGameRequest，让 MockNetwork 下发地图/玩家信息以及 NPC/怪物。
        if auto_start_game {
            if let Some(net) = self.ecs_ctx.net() {
                let _ = net.send(crate::network::handlers::NetworkEvent::StartGameRequest {
                    character_index: 0,
                });
            }
        }

        // 资源/纹理加载由外部在进入场景前调用 `load_textures()` 完成。
        Ok(())
    }

    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开游戏场景");

        // 退出时把连接放回全局，便于返回角色选择/复用。
        if let Some(net) = self.ecs_ctx.net.take() {
            crate::network::set_global_net(net);
        }

        Ok(())
    }

    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> {
        // ECS 整合：
        // - update 阶段驱动 ECS 世界
        // - Space 按下/松开：由 ECS 的 CameraSpaceGateSystem/CameraFollowSystem 控制相机模式
        // - 点击 UI 时：屏蔽 ECS 输入，避免 UI 操作导致角色乱走
        if self.is_resources_initialized() {
            self.ensure_ecs_bootstrap();

            self.ecs_ctx.delta_time = _dt;
            self.ecs_scheduler.update(&mut self.ecs_ctx, _dt)?;
        }

        // 快捷键交由 UIRenderSystem 统一处理（避免 GameScene 直连 UI 组件）

        // ESC 且没有打开的对话框 = 返回角色选择
        if is_key_pressed(KeyCode::Escape) {
            if self.can_escape_exit() {
                return Ok(SceneTransition::CharacterSelect);
            }
        }

        Ok(SceneTransition::None)
    }

    fn render(&mut self) -> GameResult {
        // 背景
        clear_background(Color::from_rgba(30, 45, 30, 255));

        // 世界渲染：
        // - Normal: Map(back+middle) + sprites/effects
        // - PostFront: Map(front) + ghost(local player) + overlays

        if self.is_resources_initialized() {
            self.update_map_camera();
            // 注意：部分渲染器（例如 MeshMapRenderer 的 chunk RenderTarget）会临时切换相机。
            // 为了保证每个 pass 的后续 RenderSystem 都在同一套世界相机下绘制，
            // 这里在每个 pass 入口重新设置一次相机。
            set_camera(&self.map_camera);
            self.draw_ecs_pass(1.0, false, RenderStage::Normal)?;
            set_camera(&self.map_camera);
            self.draw_ecs_pass(1.0, false, RenderStage::PostFront)?;
            set_default_camera();
        } else {
            set_default_camera();
        }

        // UI/覆盖层渲染：交由 ECS 渲染层（UIRenderSystem/DebugSystem）
        self.draw_ecs_pass(1.0, false, RenderStage::Ui)?;

        Ok(())
    }

    fn handle_input(&mut self) -> GameResult {
        Ok(())
    }
}
