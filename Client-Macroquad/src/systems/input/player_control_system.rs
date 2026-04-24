// ============================================================================
// Player Control System V2 - 玩家控制系统 (零拷贝版本)
// ============================================================================
//
// **架构升级** (2025-11-03):
// - 从 System trait 迁移到 SystemV2 trait
// - 使用 GameContext 实现零拷贝输入访问
// - 使用 GameContext 零拷贝访问输入
//
// **性能提升**:
// - 旧版本: ~500ns/帧 (克隆 MouseContext + KeyboardContext)
// - 新版本: ~20ns/帧 (直接引用访问)
// - 提升: 96%
//
// **职责**:
// - 双击检测 → 生成移动命令（自动寻路）
// - 长按检测 → 生成跟随命令（直接移动）
// - 右键单击 → 触发攻击 (添加 AttackState 组件)
// - 屏幕坐标 → 世界坐标转换
// - 将处理后的命令写入 PlayerInput 和 Path
//
// **数据流**:
// ```
// GameContext.ctx (零拷贝)
//     ↓ (直接访问)
// ctx.mouse, ctx.keyboard
//     ↓ (双击/长按检测)
// PlayerInput + Path 组件
//     ↓
// MovementSystem 读取并执行移动
// ```
//
// ============================================================================

use crate::{
    components::{
        Camera, HoverHighlight, LibrarySprite, NetworkObjectType, NetworkSync, Position, UiWorldInputBlock,
        LocalPlayer, Player, PlayerInput, MovementVelocity,
    },
    game::{GameContext, GameResult},
    systems::LogicSystem,
};
use macroquad::prelude::{get_time, MouseButton};
use std::time::{Duration, Instant};
use mir2_shared::enums::MirDirection;
use std::sync::OnceLock;

/// 鼠标状态追踪（用于双击和长按检测）
#[derive(Debug)]
struct MouseState {
    left_pressed: bool,
    left_press_start: Option<Instant>,
    left_press_position: Option<(f32, f32)>,
    left_last_click_time: Option<Instant>,
    left_pending_double_click: Option<(f32, f32)>,

    right_pressed: bool,
    right_press_start: Option<Instant>,
    right_press_position: Option<(f32, f32)>,
    right_last_click_time: Option<Instant>,
    right_pending_double_click: Option<(f32, f32)>,

    current_position: (f32, f32),
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            left_pressed: false,
            left_press_start: None,
            left_press_position: None,
            left_last_click_time: None,
            left_pending_double_click: None,
            right_pressed: false,
            right_press_start: None,
            right_press_position: None,
            right_last_click_time: None,
            right_pending_double_click: None,
            current_position: (0.0, 0.0),
        }
    }
}

#[derive(ecs_macros::LogicSystem)]
pub struct PlayerControlSystem {
    mouse_state: MouseState,
    double_click_threshold: Duration,
    long_press_threshold: Duration,

    // 记录上一帧 AI 模式，用于检测切换边沿（AI -> 手动）
    prev_ai_mode: bool,

    // 单击 NPC：不在范围时先走近；进入范围后自动触发一次对话
    pending_npc_call: Option<u32>,

    // local-player -> server movement sync
    last_net_move_sent: Option<Instant>,
    net_move_interval: Duration,
    last_net_move_grid: Option<(i32, i32)>,
}

impl PlayerControlSystem {
    const NPC_CALL_COOLDOWN_SECS: f64 = 0.35;

    // 与 Crystal 原版协议/服务端保持一致：
    // - Walk: 前进 1 格
    // - Run: 前进 2 格（骑马/疾风脚等可到 3，但 Macroquad 端暂按 2 处理）
    const NET_RUN_STEPS: i32 = 2;

    fn net_move_diag_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| std::env::var_os("CRYSTAL_NETMOVE_DIAG").is_some())
    }

    pub fn new() -> Self {
        Self {
            mouse_state: MouseState::default(),
            // 双击窗口：200ms 对右键双击偏紧，容易“有时打不出来”。这里适度放宽。
            double_click_threshold: Duration::from_millis(280),
            // “按住移动”阈值：120ms 对很多鼠标的“正常点击按住时长”偏紧，
            // 会把单击/双击误判为长按，从而导致双击偶发不触发。
            long_press_threshold: Duration::from_millis(200),

            prev_ai_mode: false,

            pending_npc_call: None,

            last_net_move_sent: None,
            // 服务端 `MirConnection` 的洪泛保护是按 5 秒窗口统计 socket receive 次数：
            // `_dataCounter > Settings.MaxPacket (默认 50)` 会直接断开并 IPBlock。
            // 我们每次移动都独立发一个 Walk/Run 包，Windows 下很可能对应一次 receive，
            // 因此移动同步需要保守一些（< 10 次/秒，且还要给 KeepAlive/战斗等包留空间）。
            // 说明：同步是“按格一步 Walk/Run”，而 Position 是连续像素。
            // interval 过大，会出现本地已跨过多个格子（尤其从格子中心出发，跨过中点只需半格），
            // 导致服务器收不到连续步进。
            // 原版客户端的移动“节拍”是 100ms（GameScene.MoveTime），并且移动包是“步进意图”。
            // 这里选择 100ms 以贴近原版；同时由于 Run 一次推进 2 格，实际 Run 包频率会显著低于逐格发送。
            net_move_interval: Duration::from_millis(100),
            last_net_move_grid: None,
        }
    }

    fn on_ai_disabled(&mut self, ctx: &mut GameContext) {
        use crate::components::{MovementMode, PlayerAction};

        // 1) 清理输入边缘状态：避免 AI 模式期间残留的 click 状态在切回手动后误触发。
        self.mouse_state = MouseState::default();
        self.pending_npc_call = None;

        // 2) 重置“本地移动 -> 服务器同步”的基准。
        //    切换时玩家可能仍在像素级移动/位置被服务器广播包更新，直接沿用旧基准容易出现
        //    dist>1 的跳变分支，从而产生突兀的同步行为。
        self.last_net_move_sent = None;
        // 关键修复：设置 last_net_move_grid 为当前玩家格子位置，而不是清空
        // 这样切换后第一步移动可以正常计算与上一格的距离，避免 dist>1 跳变导致瞬移
        // (实际赋值移到下面获取 player_e 后)

        // 3) 清掉 AI 留下的移动意图/路径/速度，保证手动接管后立刻“可控且稳定”。
        //    这能避免：
        //    - move_to 残留导致角色继续走/跑
        //    - path/velocity 残留导致动作/动画与输入不一致
        let Some(player_e) = ctx.world.iter().find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity())) else {
            return;
        };

        // 设置 last_net_move_grid 为当前玩家格子位置
        if let Ok(pos) = ctx.world.get::<&crate::components::Position>(player_e) {
            let current_grid = crate::coord::Coord::world_to_grid(pos.x, pos.y);
            self.last_net_move_grid = Some(current_grid);
        } else {
            self.last_net_move_grid = None;
        }

        let is_attacking = ctx.world.get::<&crate::components::AttackState>(player_e).is_ok();

        if let Ok(mut input) = ctx.world.get::<&mut crate::components::PlayerInput>(player_e) {
            input.move_to = None;
            input.movement_mode = MovementMode::None;
            input.run = false;
            input.attack_target = None;
        }

        if let Ok(mut path) = ctx.world.get::<&mut crate::components::movement::Path>(player_e) {
            path.clear();
        }
        if let Ok(mut mv) = ctx.world.get::<&mut crate::components::MovementVelocity>(player_e) {
            mv.stop();
        }
        if let Ok(mut m) = ctx.world.get::<&mut crate::components::Movement>(player_e) {
            m.set_state(crate::components::MovementState::Idle);
        }
        if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(player_e) {
            if !is_attacking && !p.action.is_attack() {
                p.action = PlayerAction::Stand;
            }
        }
    }

    fn find_object_world_pos(ctx: &GameContext, object_id: u32) -> Option<(f32, f32)> {
        use crate::components::{NetworkSync, Position};

        for (sync, pos) in ctx.world.query::<(&NetworkSync, &Position)>().iter() {
            if sync.object_id == object_id {
                return Some((pos.x, pos.y));
            }
        }
        None
    }

    fn grid_direction_towards(from: (i32, i32), to: (i32, i32)) -> Option<MirDirection> {
        let dx = (to.0 - from.0).clamp(-1, 1);
        let dy = (to.1 - from.1).clamp(-1, 1);
        if dx == 0 && dy == 0 {
            return None;
        }
        Some(match (dx, dy) {
            (0, -1) => MirDirection::Up,
            (1, -1) => MirDirection::UpRight,
            (1, 0) => MirDirection::Right,
            (1, 1) => MirDirection::DownRight,
            (0, 1) => MirDirection::Down,
            (-1, 1) => MirDirection::DownLeft,
            (-1, 0) => MirDirection::Left,
            (-1, -1) => MirDirection::UpLeft,
            _ => MirDirection::Up,
        })
    }

    fn can_send_net_move(&self, now: Instant) -> bool {
        match self.last_net_move_sent {
            None => true,
            Some(last) => now.duration_since(last) >= self.net_move_interval,
        }
    }

    fn send_net_move_step(net: Option<&crate::network::NetContext>, run: bool, dir: MirDirection) {
        let Some(net) = net else {
            return;
        };
        let _ = if run {
            net.send(crate::network::handlers::NetworkEvent::RunRequest { direction: dir })
        } else {
            net.send(crate::network::handlers::NetworkEvent::WalkRequest { direction: dir })
        };
    }

    

    

    /// 屏幕坐标 → 世界坐标
    fn screen_to_world(
        screen_x: f32,
        screen_y: f32,
        camera_pos: &Position,
        camera: &Camera,
    ) -> (f32, f32) {
        let world_x = camera_pos.x + (screen_x - camera.screen_width / 2.0) / camera.zoom;
        let world_y = camera_pos.y + (screen_y - camera.screen_height / 2.0) / camera.zoom;
        (world_x, world_y)
    }

    fn find_network_entity_by_object_id(
        ctx: &GameContext,
        object_type: NetworkObjectType,
        object_id: u32,
    ) -> Option<hecs::Entity> {
        if let Some(e) = ctx.world.iter().filter_map(|e| {
            let sync = e.get::<&NetworkSync>()?;
            if sync.object_type == object_type && sync.object_id == object_id {
                Some(e.entity())
            } else {
                None
            }
        }).next() {
            return Some(e);
        }
        None
    }

    fn hit_test_object_id(
        ctx: &GameContext,
        mouse_world: (f32, f32),
        object_type: NetworkObjectType,
    ) -> Option<u32> {
        // 贴近原版：优先像素级命中（VisiblePixel），命中里取 y 最大（最前景）
        let mut best_pixel: Option<(u32, f32)> = None;
        for (sync, spr, pos) in ctx.world.query::<(&NetworkSync, &LibrarySprite, &Position)>().iter() {
            if sync.object_type != object_type {
                continue;
            }

            let Some(info) = spr.library.get_texture(spr.texture_index()) else {
                continue;
            };

            let draw_x = pos.x + info.offset_x as f32;
            let draw_y = pos.y + info.offset_y as f32;
            let local_x = (mouse_world.0 - draw_x).floor() as i32;
            let local_y = (mouse_world.1 - draw_y).floor() as i32;
            if !info.visible_pixel(local_x, local_y) {
                continue;
            }

            match best_pixel {
                None => best_pixel = Some((sync.object_id, pos.y)),
                Some((_oid, best_y)) if pos.y > best_y => best_pixel = Some((sync.object_id, pos.y)),
                _ => {}
            }
        }
        if let Some((oid, _y)) = best_pixel {
            return Some(oid);
        }

        // 命中兜底：鼠标落在当前帧纹理矩形内
        let mut best_rect: Option<(u32, f32)> = None;
        for (sync, spr, pos) in ctx.world.query::<(&NetworkSync, &LibrarySprite, &Position)>().iter() {
            if sync.object_type != object_type {
                continue;
            }
            let Some(info) = spr.library.get_texture(spr.texture_index()) else {
                continue;
            };
            let Some(tex) = info.image else {
                continue;
            };

            let draw_x = pos.x + info.offset_x as f32;
            let draw_y = pos.y + info.offset_y as f32;
            let w = tex.width();
            let h = tex.height();
            if mouse_world.0 < draw_x
                || mouse_world.1 < draw_y
                || mouse_world.0 >= draw_x + w
                || mouse_world.1 >= draw_y + h
            {
                continue;
            }

            match best_rect {
                None => best_rect = Some((sync.object_id, pos.y)),
                Some((_oid, best_y)) if pos.y > best_y => best_rect = Some((sync.object_id, pos.y)),
                _ => {}
            }
        }

        best_rect.map(|(oid, _y)| oid)
    }

    fn hit_test_object_id_with_grid_fallback(
        ctx: &GameContext,
        mouse_world: (f32, f32),
        object_type: NetworkObjectType,
    ) -> Option<u32> {
        if let Some(oid) = Self::hit_test_object_id(ctx, mouse_world, object_type) {
            return Some(oid);
        }

        // 格子/距离兜底：用于纹理未加载或可见像素数据异常时仍能点到。
        let (click_gx, click_gy) = crate::coord::Coord::world_to_grid(mouse_world.0, mouse_world.1);
        let mut best: Option<(u32, f32)> = None;

        for (sync, pos) in ctx.world.query::<(&NetworkSync, &Position)>().iter() {
            if sync.object_type != object_type {
                continue;
            }
            let (gx, gy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
            let dx = (gx - click_gx).abs();
            let dy = (gy - click_gy).abs();
            if dx > 1 || dy > 1 {
                continue;
            }
            let dist2 = (dx * dx + dy * dy) as f32;
            match best {
                None => best = Some((sync.object_id, dist2)),
                Some((_oid, bdist2)) if dist2 < bdist2 => best = Some((sync.object_id, dist2)),
                _ => {}
            }
        }

        best.map(|(oid, _)| oid)
    }

    fn update_world_hover(ctx: &mut GameContext, camera_pos: &Position, camera: &Camera) {
        // HoverHighlight 被挂在 render-pass 实体上（约定：世界里只有一个）
        let Some(pass_entity) = ctx
            .world
            .iter()
            .find_map(|e| e.get::<&HoverHighlight>().map(|_| e.entity()))
        else {
            return;
        };

        // UI/输入屏蔽：UI 捕获或鼠标悬停 UI 时，不更新世界 hover。
        // 另外，如果本帧 `input_blocked`，FrameInput 会返回 (0,0)，必须直接跳过。
        let blocked_by_ui = ctx.input_blocked
            || ctx
            .world
            .get::<&UiWorldInputBlock>(pass_entity)
            .ok()
            .map(|b| b.mouse_captured || b.mouse_over_ui)
            .unwrap_or(false);

        let (npc_oid, monster_oid) = if blocked_by_ui {
            (None, None)
        } else {
            let mouse_pos = ctx.input().mouse.position();
            let (wx, wy) = Self::screen_to_world(mouse_pos.x, mouse_pos.y, camera_pos, camera);

            if let Some(oid) = Self::hit_test_object_id(ctx, (wx, wy), NetworkObjectType::NPC) {
                (Some(oid), None)
            } else {
                let mid = Self::hit_test_object_id(ctx, (wx, wy), NetworkObjectType::Monster);
                (None, mid)
            }
        };

        // 最后再写入 HoverHighlight，避免与上面的只读查询产生借用冲突
        if let Ok(mut hh) = ctx.world.get::<&mut HoverHighlight>(pass_entity) {
            hh.npc_object_id = npc_oid;
            hh.monster_object_id = monster_oid;
        }
    }

    fn find_clicked_npc_object_id(ctx: &GameContext, click_world: (f32, f32)) -> Option<u32> {
        Self::hit_test_object_id_with_grid_fallback(ctx, click_world, NetworkObjectType::NPC)
    }

    fn try_send_npc_main(ctx: &mut GameContext, npc_object_id: u32) {
        // 对齐交互手感：限制短时间内重复发包，但不影响连续点击/打开商店。
        let now = get_time();
        let mut allowed = true;

        // 共享冷却：优先复用 GameScene 挂在 render-pass 实体上的组件
        if let Some(cd) = ctx.world.query_mut::<&mut crate::components::NpcCallCooldown>().into_iter().next() {
            if now < cd.until {
                allowed = false;
            } else {
                cd.until = now + Self::NPC_CALL_COOLDOWN_SECS;
            }
        }

        if !allowed {
            return;
        }

        if let Some(net) = ctx.net() {
            let _ = net.send(crate::network::handlers::NetworkEvent::NPCCallRequest {
                npc_object_id,
                key: "[@Main]".to_string(),
            });
        }
    }

    fn find_clicked_monster_entity(ctx: &GameContext, click_world: (f32, f32)) -> Option<hecs::Entity> {
        let oid = Self::hit_test_object_id_with_grid_fallback(ctx, click_world, NetworkObjectType::Monster)?;
        Self::find_network_entity_by_object_id(ctx, NetworkObjectType::Monster, oid)
    }

    fn player_in_talk_range(ctx: &GameContext, npc_object_id: u32, max_range: i32) -> bool {
        use crate::components::{LocalPlayer, NetworkSync, Position};

        let player_grid = ctx
            .world
            .query::<(&LocalPlayer, &Position)>()
            .iter()
            .next()
            .map(|(_, pos)| crate::coord::Coord::world_to_grid(pos.x, pos.y));
        let Some((px, py)) = player_grid else {
            return false;
        };

        let npc_grid = ctx
            .world
            .query::<(&NetworkSync, &Position)>()
            .iter()
            .find_map(|(sync, pos)| {
                if sync.object_id == npc_object_id {
                    Some(crate::coord::Coord::world_to_grid(pos.x, pos.y))
                } else {
                    None
                }
            });
        let Some((nx, ny)) = npc_grid else {
            return false;
        };

        (nx - px).abs() <= max_range && (ny - py).abs() <= max_range
    }

    // 实体点击：NPC 交互通过 NPCDialog 驱动，怪物点击通过右键攻击处理。
    // 玩家/物品点击待后续补充。
}

impl Default for PlayerControlSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicSystem for PlayerControlSystem {
    

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let ai_mode = ctx.session.local_player_ai_enabled;

        // AI -> 手动：做一次性清理，避免残留 move_to/path/velocity 导致接管瞬间异常。
        if self.prev_ai_mode && !ai_mode {
            self.on_ai_disabled(ctx);
        }
        self.prev_ai_mode = ai_mode;

        // 模式互斥：挂机/AT/BT 控制开启时，本系统仍要做“本地移动→服务器同步”等维护逻辑，
        // 但必须完全抑制手动鼠标输入对 PlayerInput/Path 的写入。
        if ai_mode {
            self.mouse_state.left_pressed = false;
            self.mouse_state.left_press_start = None;
            self.mouse_state.left_press_position = None;
            self.mouse_state.left_last_click_time = None;
            self.mouse_state.left_pending_double_click = None;

            self.mouse_state.right_pressed = false;
            self.mouse_state.right_press_start = None;
            self.mouse_state.right_press_position = None;
            self.mouse_state.right_last_click_time = None;
            self.mouse_state.right_pending_double_click = None;

            self.pending_npc_call = None;
        }

        // ✅ 零拷贝：直接访问 GameContext
        let mouse_left_down = if ai_mode {
            false
        } else {
            ctx.input().mouse.button_down(MouseButton::Left)
        };
        let mouse_right_down = if ai_mode {
            false
        } else {
            ctx.input().mouse.button_down(MouseButton::Right)
        };
        let mouse_pos = ctx.input().mouse.position();

        // 更新鼠标位置 (Point2 使用 .x, .y 访问)
        let mouse_pos_tuple = (mouse_pos.x, mouse_pos.y);
        self.mouse_state.current_position = mouse_pos_tuple;

        let now = Instant::now();

        // 处理左键状态变化
        if mouse_left_down && !self.mouse_state.left_pressed {
            self.mouse_state.left_pressed = true;
            self.mouse_state.left_press_start = Some(now);
            self.mouse_state.left_press_position = Some(mouse_pos_tuple);
            tracing::warn!("🔽 左键按下 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
        } else if !mouse_left_down && self.mouse_state.left_pressed {
            self.mouse_state.left_pressed = false;
            tracing::warn!("🔼 左键松开 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);

            // 长按不计入“点击/双击”，避免跟随模式污染双击识别。
            let is_click_candidate = self
                .mouse_state
                .left_press_start
                .map(|t| now.duration_since(t) < self.long_press_threshold)
                .unwrap_or(false);

            if !is_click_candidate {
                self.mouse_state.left_last_click_time = None;
                self.mouse_state.left_pending_double_click = None;
                self.mouse_state.left_press_start = None;
            } else {
                // 检查是否是双击
                let is_double_click = if let Some(last_click) = self.mouse_state.left_last_click_time {
                    now.duration_since(last_click) < self.double_click_threshold
                } else {
                    false
                };

                if is_double_click {
                    tracing::warn!("🖱️🖱️ 检测到左键双击 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
                    // 双击：记录一次性事件，供本帧后续消费（触发寻路）
                    self.mouse_state.left_pending_double_click = self.mouse_state.left_press_position;
                    self.mouse_state.left_last_click_time = None;
                } else {
                    // 可能是单击，记录时间等待确认（双击窗口过后才确认）
                    self.mouse_state.left_last_click_time = Some(now);
                }
                self.mouse_state.left_press_start = None;
            }
        }

        // 处理右键状态变化
        if mouse_right_down && !self.mouse_state.right_pressed {
            self.mouse_state.right_pressed = true;
            self.mouse_state.right_press_start = Some(now);
            self.mouse_state.right_press_position = Some(mouse_pos_tuple);
            tracing::warn!("🔽 右键按下 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
        } else if !mouse_right_down && self.mouse_state.right_pressed {
            self.mouse_state.right_pressed = false;
            tracing::warn!("🔼 右键松开 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);

            // 长按不计入“点击/双击”，避免 DirectFollow/攻击等场景污染双击识别。
            let is_click_candidate = self
                .mouse_state
                .right_press_start
                .map(|t| now.duration_since(t) < self.long_press_threshold)
                .unwrap_or(false);

            if !is_click_candidate {
                self.mouse_state.right_last_click_time = None;
                self.mouse_state.right_pending_double_click = None;
                self.mouse_state.right_press_start = None;
            } else {
                // 检查是否是双击
                let is_double_click = if let Some(last_click) = self.mouse_state.right_last_click_time {
                    now.duration_since(last_click) < self.double_click_threshold
                } else {
                    false
                };

                if is_double_click {
                    tracing::warn!("🖱️🖱️ 检测到右键双击 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
                    // 双击：记录一次性事件，供本帧后续消费（触发寻路）
                    self.mouse_state.right_pending_double_click = self.mouse_state.right_press_position;
                    self.mouse_state.right_last_click_time = None;
                } else {
                    // 可能是单击，记录时间等待确认（双击窗口过后才确认）
                    self.mouse_state.right_last_click_time = Some(now);
                }
                self.mouse_state.right_press_start = None;
            }
        }

        // 获取相机信息
        let (camera_pos, camera) = ctx.world
            .query_mut::<(&Position, &Camera)>()
            .into_iter()
            .next()
            .map(|(pos, cam)| (*pos, cam.clone()))
            .unwrap_or((
                Position { x: 0.0, y: 0.0 },
                Camera {
                    zoom: 1.0,
                    screen_width: 1280.0,
                    screen_height: 720.0,
                },
            ));

        // 统一由 ECS 输入系统更新 hover 高亮
        Self::update_world_hover(ctx, &camera_pos, &camera);

        // 处理双击（移动到目标，使用寻路）
        // 说明：双击是“第二次点击松开”时产生的一次性事件；单击不会误触发。
        let double_click_left = self
            .mouse_state
            .left_pending_double_click
            .take()
            .map(|(screen_x, screen_y)| Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera));

        let double_click_right = self
            .mouse_state
            .right_pending_double_click
            .take()
            .map(|(screen_x, screen_y)| Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera));

        
        // 🎯 检查延迟单击：只有在双击窗口已过且确认不是双击时才处理单击
        let now = Instant::now();
        let left_single_click = if let Some(last_click) = self.mouse_state.left_last_click_time {
            // 超过双击窗口，确认是左键单击
            now.duration_since(last_click) >= self.double_click_threshold
        } else {
            false
        };
        
        let right_single_click = if let Some(last_click) = self.mouse_state.right_last_click_time {
            // 超过双击窗口，确认是右键单击
            now.duration_since(last_click) >= self.double_click_threshold
        } else {
            false
        };
        
        let has_single_click = left_single_click || right_single_click;

        // 克隆网络句柄（避免在持有 ctx.world 的可变借用时再借用 ctx）
        let net = ctx.net.clone();

        // NPC 对话请求：延迟到 player loop 之后再发（避免 hecs 可变借用冲突）
        let mut npc_call_immediate: Option<u32> = None;

        // 语义：本地玩家正常移动；可选择把移动意图同步到服务器。
        // - server_authoritative_movement=true: 同步 + 接受服务器回包校正
        // - sync_movement_intent_to_server=true: 仅同步（用于 Mock 命中判定），不做服务器纠偏
        let sync_move_to_server = ctx.session.server_authoritative_movement
            || ctx.session.sync_movement_intent_to_server;
        let sync_combat_to_server = ctx.session.server_authoritative_combat;

        // 单击：优先判定是否点到 NPC（近似拾取：按格子命中）
        // 原版习惯：鼠标点 NPC 直接对话；这里同时保留右键交互。
        let mut npc_interaction_target_left: Option<u32> = None;
        let mut npc_approach_target_left: Option<u32> = None;
        if left_single_click {
            if let Some((sx, sy)) = self.mouse_state.left_press_position {
                let click_world = Self::screen_to_world(sx, sy, &camera_pos, &camera);
                let clicked = Self::find_clicked_npc_object_id(ctx, click_world);
                if let Some(npc_id) = clicked {
                    if Self::player_in_talk_range(ctx, npc_id, 2) {
                        npc_interaction_target_left = Some(npc_id);
                    } else {
                        npc_approach_target_left = Some(npc_id);
                    }
                }
            }
        }

        let mut npc_interaction_target_right: Option<u32> = None;
        let mut npc_approach_target_right: Option<u32> = None;
        if right_single_click {
            if let Some((sx, sy)) = self.mouse_state.right_press_position {
                let click_world = Self::screen_to_world(sx, sy, &camera_pos, &camera);
                let clicked = Self::find_clicked_npc_object_id(ctx, click_world);
                if let Some(npc_id) = clicked {
                    if Self::player_in_talk_range(ctx, npc_id, 2) {
                        npc_interaction_target_right = Some(npc_id);
                    } else {
                        npc_approach_target_right = Some(npc_id);
                    }
                }
            }
        }

        // 记录当前交互 NPC（对齐 C# GameScene.NPCID）
        // 注意：必须在进入下面的 `query_mut::<(&mut PlayerInput, ..)>` 之前做，避免二次可变借用。
        let npc_clicked_target = if left_single_click {
            npc_interaction_target_left.or(npc_approach_target_left)
        } else if right_single_click {
            npc_interaction_target_right.or(npc_approach_target_right)
        } else {
            None
        };

        let _npc_interaction_target = if left_single_click {
            npc_interaction_target_left
        } else if right_single_click {
            npc_interaction_target_right
        } else {
            None
        };

        if let Some(npc_object_id) = npc_clicked_target {
            // hecs query_mut returns an iterator; we only need the first result
            #[allow(clippy::never_loop)]
            for active in ctx.world.query_mut::<&mut crate::components::ActiveNpc>() {
                active.npc_object_id = Some(npc_object_id);
                break;
            }
        }

        // 走近目标的世界坐标：必须在进入 query_mut 之前算好（避免 hecs 借用冲突）
        let npc_approach_left_world: Option<(u32, f32, f32)> = npc_approach_target_left
            .and_then(|oid| Self::find_object_world_pos(ctx, oid).map(|(x, y)| (oid, x, y)));
        let npc_approach_right_world: Option<(u32, f32, f32)> = npc_approach_target_right
            .and_then(|oid| Self::find_object_world_pos(ctx, oid).map(|(x, y)| (oid, x, y)));

        // 单击攻击：在进入 query_mut 之前，先把“点击世界坐标/点到的怪物实体”算好，避免可变借用期间再借用 ctx。
        // 注意：如果本次单击命中/接近 NPC，则不应同时触发攻击判定。
        let left_click_attack_world: Option<(f32, f32)> = if left_single_click
            && npc_interaction_target_left.is_none()
            && npc_approach_target_left.is_none()
        {
            self.mouse_state
                .left_press_position
                .map(|(sx, sy)| Self::screen_to_world(sx, sy, &camera_pos, &camera))
        } else {
            None
        };
        let left_click_attack_monster: Option<hecs::Entity> = left_click_attack_world
            .and_then(|click_world| Self::find_clicked_monster_entity(ctx, click_world));
        let left_click_attack_monster_grid: Option<(i32, i32)> = left_click_attack_monster
            .and_then(|e| ctx.world.get::<&crate::components::Position>(e).ok())
            .map(|pos| crate::coord::Coord::world_to_grid(pos.x, pos.y));

        let right_click_attack_world: Option<(f32, f32)> = if right_single_click
            && npc_interaction_target_right.is_none()
            && npc_approach_target_right.is_none()
        {
            self.mouse_state
                .right_press_position
                .map(|(sx, sy)| Self::screen_to_world(sx, sy, &camera_pos, &camera))
        } else {
            None
        };
        let right_click_attack_monster: Option<hecs::Entity> = right_click_attack_world
            .and_then(|click_world| Self::find_clicked_monster_entity(ctx, click_world));
        let right_click_attack_monster_grid: Option<(i32, i32)> = right_click_attack_monster
            .and_then(|e| ctx.world.get::<&crate::components::Position>(e).ok())
            .map(|pos| crate::coord::Coord::world_to_grid(pos.x, pos.y));
        
        // 更新本地玩家输入和动作状态
        use crate::components::AttackState;

        // 先收集所有有AttackState的实体
        let attacking_entities: std::collections::HashSet<_> = ctx.world
            .iter()
            .filter_map(|e| e.get::<&AttackState>().map(|_| e.entity()))
            .collect();

        // There is only one local player. Find it and process.
        if let Some(entity) = ctx.world.iter().find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity())) {
            let in_trap_rock = ctx.world.get::<&crate::components::InTrapRock>(entity)
                .map(|t| t.trapped)
                .unwrap_or(false);

            if !attacking_entities.contains(&entity) {
                if let Ok((player_input, player, pos, _path, velocity)) =
                    ctx.world.query_one_mut::<(&mut PlayerInput, &mut Player, &Position, &mut crate::components::movement::Path, &MovementVelocity)>(entity)
                {
            // 🎯 优先处理单击
            if has_single_click {
                // 停止移动
                player_input.move_to = None;
                player_input.movement_mode = crate::components::MovementMode::None;
                    // 原版体验：左键用于取消当前攻击目标（停止追砍）
                    if left_single_click {
                        player_input.attack_target = None;
                    }
                // 本地停下；不需要额外清 path（Movement/Pathfinding 会自行收敛），这里保持最小副作用。
                
                use crate::components::PlayerAction;
                
                if left_single_click {
                    // 左键单击：优先 NPC 对话（对齐原版体验）
                    if let Some(npc_object_id) = npc_interaction_target_left {
                        tracing::warn!("💬 左键点到NPC，发送NPCCallRequest: {}", npc_object_id);
                        npc_call_immediate = Some(npc_object_id);
                        player.action = PlayerAction::Stand;
                        self.pending_npc_call = None;
                    } else if let Some((npc_object_id, wx, wy)) = npc_approach_left_world {
                        // 走近再对话：先寻路靠近 NPC
                        player_input.move_to = Some((wx, wy));
                        player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                        player.action = PlayerAction::Walk;
                        self.pending_npc_call = Some(npc_object_id);
                    } else if let (Some(target_entity), Some((mgx, mgy))) = (left_click_attack_monster, left_click_attack_monster_grid) {
                        // 左键点怪：走过去后攻击（走路）
                        player_input.set_attack(target_entity);
                        player.action = PlayerAction::Attack1;
                        self.pending_npc_call = None;

                        let (pgx, pgy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
                        let dx = (mgx - pgx).abs();
                        let dy = (mgy - pgy).abs();
                        let in_melee_range = dx.max(dy) <= 1;

                        if !in_melee_range {
                            let step_x = (mgx - pgx).clamp(-1, 1);
                            let step_y = (mgy - pgy).clamp(-1, 1);
                            let agx = mgx - step_x;
                            let agy = mgy - step_y;
                            let (awx, awy) = crate::coord::Coord::grid_to_world_center(agx, agy);

                            player_input.move_to = Some((awx, awy));
                            player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                            player.action = PlayerAction::Walk;
                        }
                    } else {
                        // 左键单击 = 站立
                        tracing::warn!("⏹️ 检测到左键单击，立即停止移动");
                        player.action = PlayerAction::Stand;
                        self.pending_npc_call = None;
                    }
                } else if right_single_click {
                    // 右键单击：优先 NPC 交互（服务器驱动），否则才是攻击
                    if let Some(npc_object_id) = npc_interaction_target_right {
                        tracing::warn!("💬 右键点到NPC，发送NPCCallRequest: {}", npc_object_id);
                        // 原版主要是左键；此处仍允许右键交互，但同样走 [@Main]
                        if npc_call_immediate.is_none() {
                            npc_call_immediate = Some(npc_object_id);
                        }
                        player.action = PlayerAction::Stand;
                        self.pending_npc_call = None;
                    } else if let Some((npc_object_id, wx, wy)) = npc_approach_right_world {
                        player_input.move_to = Some((wx, wy));
                        player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                        player.action = PlayerAction::Run;
                        self.pending_npc_call = Some(npc_object_id);
                    } else {
                        // 右键单击 = 攻击动作
                        tracing::warn!("⚔️ 检测到右键单击，触发攻击");
                        player.action = PlayerAction::Attack1;

                        // 1) 点击到怪：设置 attack_target；若不在近战范围则自动走近
                        if let (Some(target_entity), Some((mgx, mgy))) = (right_click_attack_monster, right_click_attack_monster_grid) {
                            player_input.set_attack(target_entity);

                            let (pgx, pgy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
                            let dx = (mgx - pgx).abs();
                            let dy = (mgy - pgy).abs();
                            let in_melee_range = dx.max(dy) <= 1;

                            if !in_melee_range {
                                // 走到怪物附近一格（朝向玩家一侧的相邻格）
                                let step_x = (mgx - pgx).clamp(-1, 1);
                                let step_y = (mgy - pgy).clamp(-1, 1);
                                let agx = mgx - step_x;
                                let agy = mgy - step_y;
                                let (awx, awy) = crate::coord::Coord::grid_to_world_center(agx, agy);

                                player_input.move_to = Some((awx, awy));
                                player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                                player.action = PlayerAction::Run;
                            }

                            // 点击到怪：攻击动画由 CombatSystem 在“进入范围并实际出手”时添加。
                            // 这里不添加 AttackState，避免跑近过程中出现“挥刀后又站住”的不一致。
                            // IMPORTANT: 必须清除单击状态，否则本次单击会在后续帧被重复判定，导致刷屏/重复发包。
                            self.mouse_state.left_last_click_time = None;
                            self.mouse_state.right_last_click_time = None;
                            // Skip double-click/movement handling; go straight to server sync
                            // (was: continue in old for-loop; now single-entity, just exit this branch)
                        } else {

                        // 2) 朝向点击方向；没点到怪则直接挥空发一次 AttackRequest
                        if let Some(click_world) = right_click_attack_world {
                            let (pgx, pgy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
                            let (tgx, tgy) = crate::coord::Coord::world_to_grid(click_world.0, click_world.1);
                            if let Some(dir) = Self::grid_direction_towards((pgx, pgy), (tgx, tgy)) {
                                player.direction = dir;
                                if sync_combat_to_server && right_click_attack_monster.is_none() {
                                    if let Some(net) = net.as_ref() {
                                        let _ = net.send(crate::network::handlers::NetworkEvent::AttackRequest {
                                            direction: dir,
                                            spell: 0,
                                        });
                                    }
                                }
                            }
                        }

                        // 没点到怪：本地播放挥刀动画
                        // (will add attack state below)

                        } // end else (not clicking monster, do swing)
                    }
                    
                    // 攻击方向已由 grid_direction_towards 计算并设置到 player.direction
                }
                
                // 清除 last_click_time 避免重复触发
                self.mouse_state.left_last_click_time = None;
                self.mouse_state.right_last_click_time = None;
            }

            // ✅ 启用双击寻路功能（仅在未处理单击时）
            if !has_single_click {
                let has_double_click = double_click_left.is_some() || double_click_right.is_some();
            
            if has_double_click {
                use crate::components::PlayerAction;
                
                // 双击模式: 自动寻路,松开后继续移动
                if let Some((world_x, world_y)) = double_click_left {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                    player_input.run = false;
                    player.action = PlayerAction::Walk;
                    tracing::warn!("🚶🚶 左键双击走路到 ({:.1}, {:.1}) [寻路模式-松开后继续走]", world_x, world_y);
                } else if let Some((world_x, world_y)) = double_click_right {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                    player_input.run = true;
                    player.action = PlayerAction::Run;
                    tracing::warn!("🏃🏃 右键双击跑步到 ({:.1}, {:.1}) [寻路模式-松开后继续走]", world_x, world_y);
                }
            } else {
                use crate::components::{MovementMode, PlayerAction};
                
                // 没有双击,检查是否按住鼠标(直接跟随模式)
                let is_pressing_left = self.mouse_state.left_pressed;
                let is_pressing_right = self.mouse_state.right_pressed;
                
                // 只有“按住超过阈值”才进入 DirectFollow；快速单击不产生移动。
                let can_follow_left = is_pressing_left
                    && self
                        .mouse_state
                        .left_press_start
                        .map(|t| now.duration_since(t) >= self.long_press_threshold)
                        .unwrap_or(false);
                let can_follow_right = is_pressing_right
                    && self
                        .mouse_state
                        .right_press_start
                        .map(|t| now.duration_since(t) >= self.long_press_threshold)
                        .unwrap_or(false);

                if can_follow_left || can_follow_right {
                    // 🎯 鼠标按下：设置移动目标和动作状态
                    let (screen_x, screen_y) = self.mouse_state.current_position;
                    let (world_x, world_y) = Self::screen_to_world(screen_x, screen_y, &camera_pos, &camera);

                    player_input.movement_mode = MovementMode::DirectFollow;
                    player_input.move_to = Some((world_x, world_y));

                    // 🎬 设置动作（PlayerControlSystem 独占写入）
                    if can_follow_right {
                        player_input.run = true;
                        player.action = PlayerAction::Run;
                    } else {
                        player_input.run = false;
                        player.action = PlayerAction::Walk;
                    }
                } else {
                    // 🎯 鼠标松开：根据模式决定是否停止
                    match player_input.movement_mode {
                        MovementMode::DirectFollow => {
                            // 跟随模式下,松开立即停止
                            if player_input.move_to.is_some() {
                                tracing::warn!("⏹️⏹️ 松开鼠标,停止跟随 (mode={:?})", player_input.movement_mode);
                                player_input.move_to = None;
                                player_input.movement_mode = MovementMode::None;

                                // 🎬 设置站立动作（PlayerControlSystem 独占写入）
                                // 但若正在攻击（AttackState 存在），不要覆盖攻击动画。
                                let is_attacking = attacking_entities.contains(&entity);
                                if !is_attacking && !player.action.is_attack() {
                                    player.action = PlayerAction::Stand;
                                }
                            }
                        }
                        MovementMode::Pathfinding => {
                            // 寻路模式下,松开不停止,继续走完路径
                            // ✅ move_to 可能来自 AI/脚本（非鼠标双击），此时也需要维持走/跑动作，
                            // 否则会出现“位置在动但动画不播放”的平移效果。
                            let is_attacking = attacking_entities.contains(&entity);
                            if !is_attacking && player_input.move_to.is_some() && !player.action.is_attack() {
                                // 关键：不要用“有 move_to”直接驱动跑/走动画。
                                // 在碰撞/人墙场景下，move_to 会被保留用于下一帧重新算路，但 velocity/path 可能已被清空。
                                // 若仍强制播放 Run，会出现“原地奔跑”。
                                let has_velocity = velocity.x.abs() > 0.01 || velocity.y.abs() > 0.01;

                                // 只用“实际速度”驱动 Walk/Run：
                                // - Path/MoveTo 只是意图，可能长期存在（AI 追砍/重新算路）
                                // - Velocity 才代表这一帧是否真的在动
                                if has_velocity {
                                    let desired = if player_input.run {
                                        PlayerAction::Run
                                    } else {
                                        PlayerAction::Walk
                                    };

                                    // move_to 可能来自 AI/脚本：这里根据 PlayerInput.run 维持 Walk/Run。
                                    if matches!(player.action, PlayerAction::Stand | PlayerAction::Walk | PlayerAction::Run)
                                        && player.action != desired
                                    {
                                        player.action = desired;
                                    }
                                } else if player.action != PlayerAction::Stand {
                                    // 有移动意图但当前帧确实没在动：用 Stand，避免“原地跑”。
                                    player.action = PlayerAction::Stand;
                                }
                            }

                            // 但如果 MovementSystem 已清除 move_to (到达目的地)，则设置站立
                            if !is_attacking
                                && player_input.move_to.is_none()
                                && player.action != PlayerAction::Stand
                                && !player.action.is_attack()
                            {
                                player.action = PlayerAction::Stand;
                                player_input.movement_mode = MovementMode::None;
                                player_input.run = false;
                                tracing::info!("🎬 到达目的地,设置站立动作");
                            }
                        }
                        MovementMode::None => {
                            // 确保没有移动目标时是站立状态
                            let is_attacking = attacking_entities.contains(&entity);
                            if !is_attacking
                                && player_input.move_to.is_none()
                                && player.action != PlayerAction::Stand
                                && !player.action.is_attack()
                            {
                                player.action = PlayerAction::Stand;
                            }

                            // 兜底：有 move_to 但 mode=None 且这一帧没在动时，也不要继续播放 Run/Walk。
                            // 这种状态常见于“攻击结束/模式切换”边界，move_to 可能被上层保留，但本帧速度为 0。
                            if !is_attacking
                                && player_input.move_to.is_some()
                                && !player.action.is_attack()
                                && player.action != PlayerAction::Stand
                            {
                                let has_velocity = velocity.x.abs() > 0.01 || velocity.y.abs() > 0.01;
                                if !has_velocity {
                                    player.action = PlayerAction::Stand;
                                }
                            }
                        }
                    }

                } // end else (mouse release handling)
            } // end if-else (has_double_click / else)
            } // end if !has_single_click

            if in_trap_rock && (player_input.move_to.is_some()
                || player_input.movement_mode != crate::components::MovementMode::None
                || _path.is_valid
                || !matches!(player.action, crate::components::PlayerAction::Stand))
            {
                player_input.move_to = None;
                player_input.movement_mode = crate::components::MovementMode::None;
                _path.clear();
                if !player.action.is_attack() {
                    player.action = crate::components::PlayerAction::Stand;
                }
            }

            // ===== local move -> server sync: 同步”已发生的格子位移” =====
            // 关键点：
            // - 本地移动是连续像素移动；MockServer 的 Move/Walk/RunRequest 语义是“推进一格”。
            // - 如果按固定时间间隔发送，会导致 Mock 端走得比本地快，累计偏差后触发客户端的大偏差纠偏（表现为瞬移）。
            // - 这里改为：只有当本地玩家“跨入新格子”时，才给服务器发送一步，从而保持双方格子同步。
            if sync_move_to_server && !in_trap_rock {
                let now = Instant::now();
                let (pgx, pgy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);

                match self.last_net_move_grid {
                    None => {
                        // 首帧只建立基准，不发包（Mock StartGame 会把 player_grid 初始化到出生点）。
                        self.last_net_move_grid = Some((pgx, pgy));
                    }
                    Some((sgx, sgy)) => {
                        if (sgx, sgy) == (pgx, pgy) {
                            // 同一格：不需要发步进。
                        } else {
                            // 与原版 Crystal 对齐：
                            // - Walk: 前进 1 格
                            // - Run: 前进 2 格（服务端 HumanObject.Run steps=2/3）
                            // 因此这里不能简单“每格都发 Run”，否则服务端会按 2 格推进并导致坐标漂移。
                            // 策略：每次允许发送时“追赶一步”，并根据剩余 delta 决定发 Walk(1) 还是 Run(2)。
                            let dx = pgx - sgx;
                            let dy = pgy - sgy;

                            // 传送/复活/强制对齐等：差距过大时不尝试补步，直接重置基准。
                            let is_large_jump = dx.abs() > 3 || dy.abs() > 3;
                            if is_large_jump {
                                if Self::net_move_diag_enabled() {
                                    tracing::info!(
                                        "[NETMOVE] reset baseline (large jump): last=({},{}), cur=({},{}), d=({},{}), run={} mode={:?}",
                                        sgx,
                                        sgy,
                                        pgx,
                                        pgy,
                                        dx,
                                        dy,
                                        player_input.run,
                                        player_input.movement_mode
                                    );
                                }
                                self.last_net_move_grid = Some((pgx, pgy));
                            } else if self.can_send_net_move(now) {
                                let step_x = dx.clamp(-1, 1);
                                let step_y = dy.clamp(-1, 1);

                                // 不要用 player.action 推断跑/走：它可能因为“这一帧没速度”被设为 Stand。
                                // 原版的跑步并不是“更快的一格”，而是“一次两格”。
                                let want_run = player_input.run;

                                let run_is_possible = want_run
                                    && ((step_x == 0) || dx.abs() >= Self::NET_RUN_STEPS)
                                    && ((step_y == 0) || dy.abs() >= Self::NET_RUN_STEPS);

                                let steps = if run_is_possible {
                                    Self::NET_RUN_STEPS
                                } else {
                                    1
                                };

                                let next_gx = sgx + step_x * steps;
                                let next_gy = sgy + step_y * steps;

                                if let Some(dir) =
                                    Self::grid_direction_towards((sgx, sgy), (next_gx, next_gy))
                                {
                                    let run = run_is_possible;
                                    if Self::net_move_diag_enabled() {
                                        tracing::info!(
                                            "[NETMOVE] step: last=({},{})->next=({},{})->cur=({},{}), dir={:?}, run={} steps={} mode={:?}",
                                            sgx,
                                            sgy,
                                            next_gx,
                                            next_gy,
                                            pgx,
                                            pgy,
                                            dir,
                                            run,
                                            steps,
                                            player_input.movement_mode
                                        );
                                    }
                                    Self::send_net_move_step(net.as_ref(), run, dir);
                                    self.last_net_move_sent = Some(now);
                                    self.last_net_move_grid = Some((next_gx, next_gy));
                                }
                            }
                        }
                    }
                }
            }
        } // end if let Ok(query_one_mut)
        } // end if !attacking_entities
        } // end if let Some(entity)

        // 发送 NPC 主对话请求（共享 5 秒冷却）
        if let Some(npc_object_id) = npc_call_immediate {
            Self::try_send_npc_main(ctx, npc_object_id);
        }

        // 点过 NPC 但当时不在范围：走近后自动触发一次对话请求
        if let Some(npc_id) = self.pending_npc_call {
            if Self::player_in_talk_range(ctx, npc_id, 2) {
                Self::try_send_npc_main(ctx, npc_id);
                self.pending_npc_call = None;
            }
        }
        
        Ok(())
    }
}
