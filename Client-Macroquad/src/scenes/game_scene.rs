// GameScene - 游戏主场景
// 
// 快捷键:
//   I = 背包, C = 角色, B = 快捷栏, S = 商城
//   F1-F6 = 快捷栏技能
//   ESC = 返回角色选择

use crate::game::{GameContext, GameResult};
use crate::scenes::dialogs::game::{
    MainDialog,
};
use crate::scenes::{Scene, SceneTransition};
use crate::ui::text_renderer::draw_text_cn;
use crate::{
    components::{
        AnimationFrame, Camera as EcsCamera, CameraMode, LocalPlayer, MapData, MirDirection,
        MovementVelocity, Path, Player, PlayerAction, PlayerAppearance, PlayerInput, Position,
        TimeTracker,
    },
    systems::{priority, AnimationSystem, CameraFollowSystem, MovementSystem, PathfindingSystem, PlayerControlSystem, SystemScheduler},
};
use crate::components::{WeaponAnimation, WeaponState};
use crate::{map_renderer::MeshMapRenderer, resources::{init_map_libraries, LibraryName, MapReader}};
use crate::objects::frames::get_player_frame;
use macroquad::prelude::*;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};

// 遮挡“露人形”参数（集中在这里，方便调观感）
const FRONT_OCCLUSION_SEARCH_RADIUS_X_TILES: i32 = 3;
const FRONT_OCCLUSION_SEARCH_RADIUS_Y_TILES: i32 = 10;

// 以玩家脚下点为基准，向上构造一个“身体探针”矩形，用于判断是否被前景贴图遮挡
const PLAYER_OCCLUSION_PROBE_WIDTH_PX: f32 = 26.0;
const PLAYER_OCCLUSION_PROBE_HEIGHT_PX: f32 = 56.0;

// 被遮挡时额外绘制的人形透明度
const PLAYER_GHOST_ALPHA: f32 = 0.45;

// 武器特效强度（DrawBlend 的 alpha 近似值）
const WEAPON_EFFECT_ALPHA: f32 = 0.4;

// 坐骑时，人物在坐骑上的上移（纯视觉，不影响碰撞/寻路）
const MOUNT_RIDER_OFFSET_Y_PX: f32 = -24.0;

/// 游戏主场景 - 集成所有混合对话框
pub struct GameScene {
    // 地图渲染
    map_reader: Option<MapReader>,
    map_renderer: MeshMapRenderer,
    map_camera: Camera2D,
    map_camera_position: Vec2,
    map_zoom: f32,
    map_dragging: bool,
    map_first_drag: bool,
    map_last_mouse_pos: Vec2,

    // 角色武器特效用的 ADD 混合材质（避免 alpha 混合导致“阴影圈/发灰边缘”）
    add_blend_material: Material,

    // 完整 UI（底部主界面 + 全部子对话框）
    main_dialog: MainDialog,

    // ECS（ggez 版本同构）：先最小接入 update，不影响现有渲染链路
    ecs_ctx: GameContext,
    ecs_scheduler: SystemScheduler,
    ecs_camera_entity: Option<hecs::Entity>,
    ecs_local_player_entity: Option<hecs::Entity>,
    ecs_map_entity: Option<hecs::Entity>,
    ecs_time_entity: Option<hecs::Entity>,

    ecs_animation_accum: f32,

    ui_consumed_last_frame: bool,
    ui_mouse_captured: bool,
    
    // 初始化状态
    initialized: bool,
}

impl GameScene {
    pub fn new() -> Self {
        // 创建 ADD 混合材质 (dst + src * alpha)
        let add_blend_material = load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../shaders/default.vert"),
                fragment: include_str!("../../shaders/default.frag"),
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

        // Camera2D 初始值（真实参数会在地图加载后更新）
        let map_camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(2.0 / screen_width().max(1.0), 2.0 / screen_height().max(1.0)),
            offset: vec2(0.0, 0.0),
            render_target: None,
            rotation: 0.0,
            viewport: None,
        };

        let mut ecs_scheduler = SystemScheduler::new();
        ecs_scheduler
            .add_system(PlayerControlSystem::new(), priority::PLAYER_CONTROL)
            .add_system(PathfindingSystem::new(), priority::PATHFINDING)
            .add_system(MovementSystem, priority::MOVEMENT)
            .add_system(AnimationSystem::new(), priority::ANIMATION)
            .add_system(CameraFollowSystem, priority::CAMERA_FOLLOW);

        Self {
            map_reader: None,
            map_renderer: MeshMapRenderer::new(48.0, 32.0),
            map_camera,
            map_camera_position: vec2(0.0, 0.0),
            map_zoom: 1.0,
            map_dragging: false,
            map_first_drag: true,
            map_last_mouse_pos: mouse_position().into(),

            add_blend_material,

            main_dialog: MainDialog::new(),

            ecs_ctx: GameContext::new(),
            ecs_scheduler,
            ecs_camera_entity: None,
            ecs_local_player_entity: None,
            ecs_map_entity: None,
            ecs_time_entity: None,

            ecs_animation_accum: 0.0,
            ui_consumed_last_frame: false,
            ui_mouse_captured: false,
            initialized: false,
        }
    }

    fn ensure_ecs_bootstrap(&mut self) {
        if !self.initialized {
            return;
        }

        // 1) 地图数据（用于寻路/碰撞等系统）
        if self.ecs_map_entity.is_none() {
            if let Some(map) = self.map_reader.as_ref() {
                let entity = self.ecs_ctx.world.spawn((MapData {
                    cells: map.map_cells.clone(),
                    width: map.width,
                    height: map.height,
                },));
                self.ecs_map_entity = Some(entity);
            }
        }

        // 2) 相机实体（PlayerControlSystem 需要 Camera + Position 来做屏幕->世界坐标转换）
        if self.ecs_camera_entity.is_none() {
            let (sw, sh) = self.ecs_ctx.drawable_size();
            let entity = self.ecs_ctx.world.spawn((
                EcsCamera::new(sw, sh),
                Position::new(self.map_camera_position.x, self.map_camera_position.y),
                CameraMode::FollowPlayer,
            ));
            self.ecs_camera_entity = Some(entity);
        }

        // 2.5) 时间跟踪实体（AnimationSystem 需要 animation_count 驱动帧变化）
        if self.ecs_time_entity.is_none() {
            let entity = self.ecs_ctx.world.spawn((TimeTracker::default(),));
            self.ecs_time_entity = Some(entity);
        }

        // 3) 本地玩家实体（最小移动链路）
        if self.ecs_local_player_entity.is_none() {
            let player = Player {
                direction: MirDirection::Up,
                action: PlayerAction::Stand,
            };

            // 给 test_game_scene 一个“看得见的”默认外观，方便验证多层渲染
            // （避免 weapon=-1 / weapon_effect=0 导致看起来像“没画”）
            let appearance = PlayerAppearance {
                hair: 7,
                armour: 30,
                weapon: 78,
                weapon_effect: 66,
                wing_effect: 1,
                ..PlayerAppearance::default()
            };

            let weapon_anim = WeaponAnimation::new(appearance.weapon.max(0) as u16);

            let entity = self.ecs_ctx.world.spawn((
                LocalPlayer,
                player,
                Position::new(self.map_camera_position.x, self.map_camera_position.y),
                appearance,
                AnimationFrame::default(),
                PlayerInput::default(),
                Path::new(),
                MovementVelocity::new(crate::components::movement::DEFAULT_MAX_SPEED),
                WeaponState::default(),
                weapon_anim,
            ));
            self.ecs_local_player_entity = Some(entity);
        }
    }

    fn sync_ecs_camera_from_map(&mut self) {
        let Some(cam_entity) = self.ecs_camera_entity else {
            return;
        };

        // 同步相机尺寸/缩放/位置到 ECS
        let (sw, sh) = self.ecs_ctx.drawable_size();
        if let Ok(mut cam) = self.ecs_ctx.world.get::<&mut EcsCamera>(cam_entity) {
            cam.screen_width = sw;
            cam.screen_height = sh;
            cam.zoom = self.map_zoom;
        }
        if let Ok(mut pos) = self.ecs_ctx.world.get::<&mut Position>(cam_entity) {
            pos.x = self.map_camera_position.x;
            pos.y = self.map_camera_position.y;
        }
    }

    fn apply_ecs_camera_to_map(&mut self) {
        let Some(cam_entity) = self.ecs_camera_entity else {
            return;
        };
        let Ok(pos) = self.ecs_ctx.world.get::<&Position>(cam_entity) else {
            return;
        };

        // 先拷贝数据，避免 hecs::Ref 持有借用导致后续无法可变借用 self
        let (x, y) = (pos.x, pos.y);
        drop(pos);

        self.map_camera_position.x = x;
        self.map_camera_position.y = y;
        self.clamp_map_camera_position();
    }

    fn draw_local_player_sprite(&mut self, alpha: f32) {
        let Some(player_entity) = self.ecs_local_player_entity else {
            return;
        };

        let Ok(mut q) = self
            .ecs_ctx
            .world
            .query_one::<(&Position, &Player, &PlayerAppearance, &AnimationFrame)>(player_entity)
        else {
            return;
        };

        let Some((pos, player, appearance, anim_frame)) = q.get() else {
            return;
        };

        // 取全局动画计数器（与 AnimationSystem 同源）
        let animation_count = self
            .ecs_time_entity
            .and_then(|e| self.ecs_ctx.world.get::<&TimeTracker>(e).ok().map(|t| t.animation_count))
            .unwrap_or(0);

        // 使用 AnimationSystem 计算的帧索引（更接近最终架构）
        let base_frame = anim_frame.character_frame;
        let weapon_frame = anim_frame.weapon_frame;

        let tint = Color::new(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0));

        // 性别偏移（C# HairOffSet/ArmourOffSet/WeaponOffSet）
        let body_hair_offset = if appearance.gender == crate::components::MirGender::Male {
            0
        } else {
            808
        };
        let weapon_offset = if appearance.gender == crate::components::MirGender::Male {
            0
        } else {
            416
        };

        let armour_index = appearance.armour.max(0) as usize;
        let hair_index = appearance.hair as usize;
        let weapon_index_opt = if appearance.weapon >= 0 {
            Some(appearance.weapon as usize)
        } else {
            None
        };
        let weapon_effect_index_opt = if appearance.weapon_effect > 0 {
            Some(appearance.weapon_effect as usize)
        } else {
            None
        };

        let armour_library = match appearance.class {
            crate::components::MirClass::Assassin => LibraryName::AArmours(armour_index),
            crate::components::MirClass::Archer => LibraryName::ARArmours(armour_index),
            _ => LibraryName::CArmours(armour_index),
        };
        let hair_library = match appearance.class {
            crate::components::MirClass::Assassin => LibraryName::AHair(hair_index),
            crate::components::MirClass::Archer => LibraryName::ARHair(hair_index),
            _ => LibraryName::CHair(hair_index),
        };
        let weapon_library = |idx: usize| match appearance.class {
            crate::components::MirClass::Archer => LibraryName::ARWeapons(idx),
            _ => LibraryName::CWeapons(idx),
        };

        let mut drew_any = false;

        let draw_layer = |lib: LibraryName, frame: i32, pos: &Position, tint: Color| -> bool {
            let frame_index = frame.max(0) as usize;
            let Some(info) = lib.get_texture(frame_index) else {
                return false;
            };
            let Some(tex) = info.image else {
                return false;
            };
            let draw_x = pos.x + info.offset_x as f32;
            let draw_y = pos.y + info.offset_y as f32;
            draw_texture_ex(
                &tex,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams { ..Default::default() },
            );
            true
        };

        let draw_layer_additive = |material: &Material,
                                  lib: LibraryName,
                                  frame: i32,
                                  pos: &Position,
                                  tint: Color|
         -> bool {
            let frame_index = frame.max(0) as usize;
            let Some(info) = lib.get_texture(frame_index) else {
                return false;
            };
            let Some(tex) = info.image else {
                return false;
            };
            let draw_x = pos.x + info.offset_x as f32;
            let draw_y = pos.y + info.offset_y as f32;

            gl_use_material(material);
            draw_texture_ex(
                &tex,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams { ..Default::default() },
            );
            gl_use_default_material();

            true
        };

        // 坐骑（MVP：用于 test_game_scene 视觉展示）
        // 注意：`objects::frames::PLAYER_FRAMES` 里的 Mount* start 值是“人物帧表”的索引，
        // 不能直接拿来索引 `Data/Mount/XX.Lib`，否则会越界导致“坐骑没了”。
        // 这里改为：以 Mount lib 的 0 起始索引推算帧，并带多级回退，保证至少能画出来。
        const DEFAULT_MOUNT_INDEX: usize = 0;
        let mounted = true;

        let mut mount_drawn = false;
        let actor_pos;

        if mounted {
            let mount_lib = LibraryName::Mounts(DEFAULT_MOUNT_INDEX);

            // 坐骑资源的方向顺序在不同客户端资源里存在差异。
            // 当前这套 Data/Mount 资源表现为与 MirDirection 同序（Up 开始顺时针）。
            // 如果这里做 +4 旋转，会导致整体 180° 反向。
            let dir = player.direction as u8 as i32;
            let mount_dir_index = dir.rem_euclid(8);

            // 常见坐骑资源布局：按动作分块排列，每块再按 8 方向排列。
            // Stand: 4 帧/方向  => base 0
            // Walk : 8 帧/方向  => base 8*4 = 32
            // Run  : 6 帧/方向  => base 32 + 8*8 = 96
            let (frames_per_dir, interval_ms, action_base) = match player.action {
                PlayerAction::Walk => (8, 100, 8 * 4),
                PlayerAction::Run => (6, 100, 8 * 4 + 8 * 8),
                _ => (4, 500, 0),
            };

            // 用 TimeTracker 驱动动画帧（与角色/地图一致的 100ms tick 源）
            let animation_tick = (animation_count as i32) * 100 / interval_ms;
            let current_frame = animation_tick % frames_per_dir;

            // 先按“动作分块 + 方向 + 帧”尝试
            let primary_index = action_base + mount_dir_index * frames_per_dir + current_frame;

            // 失败时回退：
            // 1) 同方向但用 stand 分块
            // 2) 同方向但不用分块（有些资源从 0 开始直接按方向排）
            // 3) 最后兜底到 0/1/2
            let candidates = [
                primary_index,
                0 + mount_dir_index * 4 + (current_frame % 4),
                mount_dir_index * frames_per_dir + current_frame,
                current_frame,
                0,
                1,
                2,
            ];

            for frame_index in candidates {
                if draw_layer(mount_lib, frame_index, pos, tint) {
                    mount_drawn = true;
                    drew_any = true;
                    break;
                }
            }
        }

        // 骑乘时，人物应使用 Mount* 动作帧（否则会看到“人在坐骑上还在跑路/跑步姿势”）。
        // 注意：Mount* 人物帧本身已经包含与坐骑对齐的绘制偏移。
        // 若我们再额外上移一次，会导致“人站在坐骑上/悬空”。
        let mut rider_base_frame = base_frame;
        let mut rider_weapon_frame = weapon_frame;
        let mut rider_uses_mount_frames = false;
        if mounted && mount_drawn {
            let mount_action = match player.action {
                PlayerAction::Walk => mir2_shared::enums::MirAction::MountWalking,
                PlayerAction::Run => mir2_shared::enums::MirAction::MountRunning,
                _ => mir2_shared::enums::MirAction::MountStanding,
            };

            if let Some(frame) = get_player_frame(mount_action) {
                let dir = player.direction as u8 as i32;

                let interval = frame.interval.max(1);
                let count = frame.count.max(1);
                let tick = (animation_count as i32) * 100 / interval;
                let current = tick % count;

                rider_base_frame = frame.start + (dir * frame.count) + current;

                // ✅ 与非骑乘一致：武器/武器特效跟随身体 DrawFrame，避免左右“乱晃”。
                rider_weapon_frame = rider_base_frame;

                rider_uses_mount_frames = true;
            }
        }

        // 只有真的画出了坐骑，才对人物位置做骑乘修正。
        // 若使用 Mount* 人物帧，不再额外上移；否则用旧偏移做兜底对齐。
        actor_pos = if mounted && mount_drawn {
            if rider_uses_mount_frames {
                *pos
            } else {
                Position::new(pos.x, pos.y + MOUNT_RIDER_OFFSET_Y_PX)
            }
        } else {
            *pos
        };

        // 翅膀（简单实现：优先用 rider_base_frame；取不到就回退到 0 帧）
        if appearance.wing_effect > 0 {
            let wing_lib = LibraryName::Wings(appearance.wing_effect as usize);
            if !draw_layer(wing_lib, rider_base_frame, &actor_pos, tint) {
                drew_any |= draw_layer(wing_lib, 0, &actor_pos, tint);
            } else {
                drew_any = true;
            }
        }

        // 武器层前后关系（参考原版）：右侧/下方方向武器在前，其余方向武器在后
        let weapon_front = matches!(
            player.direction,
            crate::components::MirDirection::UpRight
                | crate::components::MirDirection::Right
                | crate::components::MirDirection::DownRight
                | crate::components::MirDirection::Down
        );

        // weapon behind
        if !weapon_front {
            if let Some(weapon_index) = weapon_index_opt {
                drew_any |= draw_layer(
                    weapon_library(weapon_index),
                    rider_weapon_frame + weapon_offset,
                    &actor_pos,
                    tint,
                );

                if let Some(effect_index) = weapon_effect_index_opt {
                    // ✅ 原版 DrawBlend(0.4F) 更接近 ADD/发光效果，这里用 ADD 混合避免“阴影圈”。
                    let effect_tint = Color::new(
                        tint.r,
                        tint.g,
                        tint.b,
                        (tint.a * WEAPON_EFFECT_ALPHA).clamp(0.0, 1.0),
                    );
                    drew_any |= draw_layer_additive(
                        &self.add_blend_material,
                        LibraryName::CWeaponEffect(effect_index),
                        rider_weapon_frame + weapon_offset,
                        &actor_pos,
                        effect_tint,
                    );
                }
            }
        }

        // body
        drew_any |= draw_layer(
            armour_library,
            rider_base_frame + body_hair_offset,
            &actor_pos,
            tint,
        );

        // hair
        drew_any |= draw_layer(
            hair_library,
            rider_base_frame + body_hair_offset,
            &actor_pos,
            tint,
        );



        // weapon front
        if weapon_front {
            if let Some(weapon_index) = weapon_index_opt {
                drew_any |= draw_layer(
                    weapon_library(weapon_index),
                    rider_weapon_frame + weapon_offset,
                    &actor_pos,
                    tint,
                );

                if let Some(effect_index) = weapon_effect_index_opt {
                    let effect_tint = Color::new(
                        tint.r,
                        tint.g,
                        tint.b,
                        (tint.a * WEAPON_EFFECT_ALPHA).clamp(0.0, 1.0),
                    );
                    drew_any |= draw_layer_additive(
                        &self.add_blend_material,
                        LibraryName::CWeaponEffect(effect_index),
                        rider_weapon_frame + weapon_offset,
                        &actor_pos,
                        effect_tint,
                    );
                }
            }
        }

        // 回退：红点
        if !drew_any {
            draw_circle(pos.x, pos.y, 6.0, Color::new(1.0, 0.0, 0.0, tint.a));
        }
    }

    fn draw_ecs_path_overlay(&mut self) {
        let Some(player_entity) = self.ecs_local_player_entity else {
            return;
        };

        // 路径（格子→世界）
        if let (Ok(pos), Ok(path)) = (
            self.ecs_ctx.world.get::<&Position>(player_entity),
            self.ecs_ctx.world.get::<&Path>(player_entity),
        ) {
            if path.is_valid && !path.waypoints.is_empty() {
                let mut last = (pos.x, pos.y);
                for (gx, gy) in path.waypoints.iter().copied() {
                    let wx = gx as f32 * 48.0;
                    let wy = gy as f32 * 32.0;
                    draw_line(last.0, last.1, wx, wy, 2.0, Color::from_rgba(255, 255, 0, 180));
                    last = (wx, wy);
                }
            }
        }
    }

    fn is_local_player_occluded_by_front(&self) -> bool {
        let Some(map) = self.map_reader.as_ref() else {
            return false;
        };
        let Some(player_entity) = self.ecs_local_player_entity else {
            return false;
        };
        let Ok(pos) = self.ecs_ctx.world.get::<&Position>(player_entity) else {
            return false;
        };

        // 更真实遮挡判断：用 Front 贴图的“实际绘制矩形”与玩家身体区域相交来判断。
        // 这里用一个从脚下向上延伸的矩形作为人体探针（无需 shader、也不改前景本身）。
        let foot = vec2(pos.x, pos.y);
        let probe = Rect::new(
            foot.x - PLAYER_OCCLUSION_PROBE_WIDTH_PX * 0.5,
            foot.y - PLAYER_OCCLUSION_PROBE_HEIGHT_PX,
            PLAYER_OCCLUSION_PROBE_WIDTH_PX,
            PLAYER_OCCLUSION_PROBE_HEIGHT_PX,
        );

        self.map_renderer.front_layer_occludes_probe(
            map,
            probe,
            FRONT_OCCLUSION_SEARCH_RADIUS_X_TILES,
            FRONT_OCCLUSION_SEARCH_RADIUS_Y_TILES,
        )
    }

    fn update_map_camera(&mut self) {
        // 像素对齐（抗闪烁）：
        // Linear 过滤 + 子像素相机移动会导致阴影/暗部出现“深浅闪烁”(shimmering)。
        // 将渲染相机 target 对齐到 1/map_zoom 的网格（世界像素）上，可显著稳定采样。
        let zoom = self.map_zoom.max(0.0001);
        let snapped_target = vec2(
            (self.map_camera_position.x * zoom).round() / zoom,
            (self.map_camera_position.y * zoom).round() / zoom,
        );
        self.map_camera.target = snapped_target;
        let sw = screen_width().max(1.0);
        let sh = screen_height().max(1.0);
        self.map_camera.zoom = vec2(2.0 / sw * self.map_zoom, 2.0 / sh * self.map_zoom);
    }

    fn clamp_map_camera_position(&mut self) {
        let Some(map) = self.map_reader.as_ref() else {
            return;
        };

        // 视口半宽/半高（世界坐标，单位：像素）
        let half_w = (screen_width().max(1.0) / 2.0) / self.map_zoom;
        let half_h = (screen_height().max(1.0) / 2.0) / self.map_zoom;

        // 地图总大小（世界坐标，单位：像素）
        let map_w = map.width as f32 * 48.0;
        let map_h = map.height as f32 * 32.0;

        // 若视口比地图大，直接居中
        if map_w <= half_w * 2.0 {
            self.map_camera_position.x = map_w / 2.0;
        } else {
            self.map_camera_position.x = self.map_camera_position.x.clamp(half_w, map_w - half_w);
        }

        if map_h <= half_h * 2.0 {
            self.map_camera_position.y = map_h / 2.0;
        } else {
            self.map_camera_position.y = self.map_camera_position.y.clamp(half_h, map_h - half_h);
        }
    }

    fn handle_map_input(&mut self) {
        if self.map_reader.is_none() {
            return;
        }

        // 为了避免与 UI 点击/拖拽冲突：只有按住 Space 才启用地图交互
        let map_input_enabled = is_key_down(KeyCode::Space);

        if map_input_enabled {
            // 鼠标滚轮缩放
            let wheel_y = mouse_wheel().1;
            if wheel_y != 0.0 {
                let zoom_factor = if wheel_y > 0.0 { 1.1 } else { 0.9 };
                self.map_zoom = (self.map_zoom * zoom_factor).clamp(0.3, 5.0);
            }

            // 鼠标拖拽平移（Space + 左键）
            if is_mouse_button_pressed(MouseButton::Left) {
                self.map_dragging = true;
                self.map_last_mouse_pos = mouse_position().into();
                self.map_first_drag = true;
            }

            if is_mouse_button_released(MouseButton::Left) {
                self.map_dragging = false;
            }
        } else {
            // 没按 Space 时，永远不保持拖拽状态
            self.map_dragging = false;
        }

        // 安全机制：避免窗口失焦导致释放事件丢失
        if self.map_dragging && !is_mouse_button_down(MouseButton::Left) {
            self.map_dragging = false;
        }

        if self.map_dragging {
            let current_pos: Vec2 = mouse_position().into();
            let delta = current_pos - self.map_last_mouse_pos;

            // 第一次拖拽时，如果 delta 过大（例如点击用于激活窗口），忽略
            let delta_magnitude = (delta.x * delta.x + delta.y * delta.y).sqrt();
            if self.map_first_drag && delta_magnitude > 100.0 {
                self.map_last_mouse_pos = current_pos;
                self.map_first_drag = false;
            } else if delta_magnitude > 0.1 {
                self.map_first_drag = false;

                // 公式参考 map_viewer：屏幕像素 delta -> 世界坐标 delta，再除以 zoom
                let world_delta_x = delta.x / self.map_zoom;
                let world_delta_y = delta.y / self.map_zoom;
                self.map_camera_position.x -= world_delta_x;
                self.map_camera_position.y -= world_delta_y;

                // 防止拖出地图边界
                self.clamp_map_camera_position();

                self.map_last_mouse_pos = current_pos;
            }
        } else {
            // 不拖拽时也更新，避免下次拖拽出现巨大 delta
            self.map_last_mouse_pos = mouse_position().into();
        }
    }
    
    /// 异步加载所有对话框纹理
    pub async fn load_textures(&mut self) {
        println!("🎮 GameScene: 加载对话框纹理...");

        self.main_dialog.load_native_textures().await;

        // 加载地图（用于主场景背景渲染）
        // 说明：这里先用固定地图文件，后续接入网络/地图切换系统再动态更新。
        println!("🗺️ GameScene: 初始化地图库...");
        if let Err(e) = init_map_libraries() {
            println!("⚠️ GameScene: 地图库初始化失败: {}", e);
        }

        let map_path = "Map/n0.map";
        match MapReader::new(map_path) {
            Ok(reader) => {
                println!("✅ GameScene: 地图加载成功 {} ({}x{})", map_path, reader.width, reader.height);
                // 初始相机位置：地图中心
                self.map_camera_position = vec2(reader.width as f32 * 48.0 / 2.0, reader.height as f32 * 32.0 / 2.0);
                self.map_zoom = 1.0;
                self.map_reader = Some(reader);
                self.update_map_camera();
            }
            Err(e) => {
                println!("⚠️ GameScene: 地图加载失败 {}: {} (将使用占位网格背景)", map_path, e);
                self.map_reader = None;
            }
        }
        
        self.initialized = true;

        // 地图加载完成后，立即完成 ECS 最小引导（MapData/Camera/LocalPlayer）
        self.ensure_ecs_bootstrap();
        self.sync_ecs_camera_from_map();
        println!("✅ GameScene: 对话框纹理加载完成");
    }
    
    /// 处理快捷键
    fn handle_hotkeys(&mut self) {
        // 如果聊天输入框激活，不处理其他快捷键
        if self.main_dialog.is_any_input_active() {
            return;
        }

        // Enter = 激活聊天输入框
        if is_key_pressed(KeyCode::Enter) {
            self.main_dialog.activate_chat_input();
        }

        // M = 切换小地图显示
        if is_key_pressed(KeyCode::M) {
            self.main_dialog.toggle_minimap();
        }

        // Tab = 切换小地图大小
        if is_key_pressed(KeyCode::Tab) {
            self.main_dialog.toggle_minimap_size();
        }

        // ESC = 先关闭弹窗；若没弹窗则返回角色选择（在 update 中处理返回）
        if is_key_pressed(KeyCode::Escape) {
            if self.main_dialog.any_popup_open() {
                self.main_dialog.close_all_popups();
            }
        }
    }
    
    /// 绘制快捷键提示
    fn draw_help_text(&self) {
        let y = screen_height() - 25.0;
        draw_text_cn(
            "快捷键: Space+拖拽/滚轮=地图 | Enter=聊天 M=小地图 Tab=小地图大小 | ESC=返回角色选择",
            10.0, y, 14.0, Color::from_rgba(200, 200, 200, 180)
        );
    }
}

impl Default for GameScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for GameScene {
    fn name(&self) -> &str { "游戏场景" }
    
    fn on_enter(&mut self) -> GameResult {
        println!("🎬 进入游戏场景");
        // 注意: 纹理需要异步加载，这里无法调用 async 函数
        // 应该在进入场景前或通过 Loading 场景预加载
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开游戏场景");
        self.main_dialog.close_all_popups();
        self.main_dialog.deactivate_chat_input();
        Ok(())
    }
    
    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> {
        // 地图动画更新
        self.map_renderer.update(_dt);

        // 地图输入（空格模式）
        self.handle_map_input();

        // ECS 整合：
        // - update 阶段驱动 ECS 世界
        // - Space 按下时：地图相机为手动（由现有 map_input 控制）
        // - Space 松开时：地图相机会跟随 ECS Camera（CameraFollowSystem）
        // - 点击 UI 时：屏蔽 ECS 输入，避免 UI 操作导致角色乱走
        if self.initialized {
            self.ensure_ecs_bootstrap();

            // 驱动全局动画计数器（AnimationSystem 使用 animation_count 计算当前帧）
            if let Some(time_entity) = self.ecs_time_entity {
                if let Ok(mut tt) = self.ecs_ctx.world.get::<&mut TimeTracker>(time_entity) {
                    tt.frame_count = tt.frame_count.wrapping_add(1);
                    tt.last_frame_time = std::time::Instant::now();

                    // 与地图动画一致：每 100ms 递增一次（使用累加器避免 dt<0.1 时永远不动）
                    self.ecs_animation_accum += _dt;
                    while self.ecs_animation_accum >= 0.1 {
                        tt.animation_count = tt.animation_count.wrapping_add(1);
                        self.ecs_animation_accum -= 0.1;
                    }
                }
            }

            let space_down = is_key_down(KeyCode::Space);
            let (mx, my) = mouse_position();
            let mouse_pos = vec2(mx, my);
            let left_pressed = is_mouse_button_pressed(MouseButton::Left);
            let right_pressed = is_mouse_button_pressed(MouseButton::Right);
            let left_down = is_mouse_button_down(MouseButton::Left);
            let right_down = is_mouse_button_down(MouseButton::Right);
            let mouse_button_down = left_down || right_down;
            let wheel_y = mouse_wheel().1;
            let ui_over = self.main_dialog.is_mouse_over_ui(mouse_pos);

            // UI 鼠标捕获：在 UI 上按下鼠标后，直到松开都阻止 ECS 读取输入。
            // 解决“拖拽 UI 时后续帧仍触发角色移动/寻路”的问题。
            if (left_pressed || right_pressed) && ui_over {
                self.ui_mouse_captured = true;
            }
            if self.ui_mouse_captured && !mouse_button_down {
                self.ui_mouse_captured = false;
            }

            // 这个帧内是否阻止 ECS 读取输入
            self.ecs_ctx.input_blocked = self.main_dialog.is_any_input_active()
                || self.ui_mouse_captured
                || (wheel_y != 0.0 && ui_over)
                || self.ui_consumed_last_frame;

            // 相机模式/权威切换
            if let Some(cam_entity) = self.ecs_camera_entity {
                if let Ok(mut mode) = self.ecs_ctx.world.get::<&mut CameraMode>(cam_entity) {
                    *mode = if space_down { CameraMode::Manual } else { CameraMode::FollowPlayer };
                }
            }

            if space_down || self.map_dragging {
                // 手动相机：GameScene map_camera_position 主导 ECS camera
                self.sync_ecs_camera_from_map();
            }

            self.ecs_ctx.delta_time = _dt;
            self.ecs_scheduler.update(&mut self.ecs_ctx, _dt)?;

            if !space_down && !self.map_dragging {
                // 跟随相机：ECS camera 主导 GameScene map_camera_position
                self.apply_ecs_camera_to_map();
            }

            self.ecs_ctx.cleanup_dead_entities();
            self.ecs_ctx.events_mut().clear_frame();
        }

        // 处理快捷键
        self.handle_hotkeys();
        
        // ESC 且没有打开的对话框 = 返回角色选择
        if is_key_pressed(KeyCode::Escape) {
            if !self.main_dialog.any_popup_open() {
                return Ok(SceneTransition::CharacterSelect);
            }
        }
        
        Ok(SceneTransition::None)
    }

    fn render(&mut self) -> GameResult {
        // 绘制地图背景（若地图未加载则使用占位网格）
        clear_background(Color::from_rgba(30, 45, 30, 255));

        self.update_map_camera();
        if self.map_reader.is_some() {
            set_camera(&self.map_camera);
            // 1) 先渲染 Back+Middle（不画 Front）
            let original_show_front = self.map_renderer.show_front_layer;
            {
                let map = self.map_reader.as_ref().expect("map_reader exists");

                self.map_renderer.show_front_layer = false;
                // 颜色先保持原样，后续若需要再引入 RenderConfig
                let _tiles = self.map_renderer.render(
                    map,
                    self.map_camera_position.x,
                    self.map_camera_position.y,
                    screen_width(),
                    screen_height(),
                    self.map_zoom,
                    WHITE,
                );
                self.map_renderer.show_front_layer = original_show_front;
            }

            // 2) 先画角色（位于 Middle 与 Front 之间）
            self.draw_local_player_sprite(1.0);
            self.draw_ecs_path_overlay();

            // 3) 再渲染 Front（保持完全不透明）
            if original_show_front {
                let occluded = self.is_local_player_occluded_by_front();
                {
                    let map = self.map_reader.as_ref().expect("map_reader exists");
                    let _front_tiles = self.map_renderer.render_front_layer_with_focus(
                        map,
                        self.map_camera_position.x,
                        self.map_camera_position.y,
                        screen_width(),
                        screen_height(),
                        self.map_zoom,
                        WHITE,
                        None,
                        0,
                        0,
                        1.0,
                    );
                }

                // 4) 只有被前景遮挡时，额外画一遍半透明人形（不改变前景本身）
                if occluded {
                    self.draw_local_player_sprite(PLAYER_GHOST_ALPHA);
                }
            }
            set_default_camera();
        } else {
            // 占位网格背景（模拟地图）
            let grid_color = Color::from_rgba(50, 65, 50, 255);
            for i in 0..=((screen_width() / 48.0) as i32 + 1) {
                let x = i as f32 * 48.0;
                draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
            }
            for i in 0..=((screen_height() / 32.0) as i32 + 1) {
                let y = i as f32 * 32.0;
                draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
            }
        }
        
        // 提示文字
        if !self.initialized {
            draw_text_cn(
                "⏳ 正在加载游戏资源...",
                screen_width() / 2.0 - 100.0,
                screen_height() / 2.0,
                24.0, WHITE
            );
        } else {
            // 绘制完整 UI
            self.main_dialog.update_and_draw();
            let ui_consumed = self.main_dialog.show_dialogs();
            self.ui_consumed_last_frame = ui_consumed;
            
            // 绘制帮助提示
            self.draw_help_text();
        }
        
        Ok(())
    }
    
    fn handle_input(&mut self) -> GameResult { 
        Ok(()) 
    }
}
