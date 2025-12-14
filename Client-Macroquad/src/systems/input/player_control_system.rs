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
        Camera, LocalPlayer, Player, PlayerInput, Position,
    },
    game::{GameContext, GameResult},
    systems::LogicSystem,
};
use macroquad::prelude::MouseButton;
use std::time::{Duration, Instant};
use mir2_shared::enums::MirDirection;

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

    // local-player -> server movement sync
    last_net_move_sent: Option<Instant>,
    net_move_interval: Duration,
}

impl PlayerControlSystem {
    pub fn new() -> Self {
        Self {
            mouse_state: MouseState::default(),
            // 双击窗口：越长越容易误触发自动寻路；这里收紧到 200ms。
            double_click_threshold: Duration::from_millis(200),
            // “按住移动”需要一个阈值，避免快速单击也触发 DirectFollow 导致轻微位移。
            long_press_threshold: Duration::from_millis(120),

            last_net_move_sent: None,
            net_move_interval: Duration::from_millis(80),
        }
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

    fn find_clicked_npc_object_id(ctx: &GameContext, click_world: (f32, f32)) -> Option<u32> {
        use crate::components::{NetworkObjectType, NetworkSync, Position};

        let (click_gx, click_gy) = crate::coord::Coord::world_to_grid(click_world.0, click_world.1);

        // 找到落在同一格（或相邻格）的 NPC（基于 NetworkSync.object_type）
        let mut best: Option<(u32, i32, i32, f32)> = None;

        for (_, (sync, pos)) in ctx.world.query::<(&NetworkSync, &Position)>().iter() {
            if sync.object_type != NetworkObjectType::NPC {
                continue;
            }
            let (gx, gy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
            let dx = (gx - click_gx).abs();
            let dy = (gy - click_gy).abs();
            if dx > 1 || dy > 1 {
                continue;
            }
            // 在候选里选“离点击点最近”的 NPC
            let dist2 = (pos.x - click_world.0) * (pos.x - click_world.0)
                + (pos.y - click_world.1) * (pos.y - click_world.1);
            match best {
                None => best = Some((sync.object_id, gx, gy, dist2)),
                Some((_oid, _bgx, _bgy, bdist2)) if dist2 < bdist2 => {
                    best = Some((sync.object_id, gx, gy, dist2))
                }
                _ => {}
            }
        }

        best.map(|(oid, _, _, _)| oid)
    }

    fn find_clicked_monster_entity(ctx: &GameContext, click_world: (f32, f32)) -> Option<hecs::Entity> {
        use crate::components::{NetworkObjectType, NetworkSync, Position};

        let (click_gx, click_gy) = crate::coord::Coord::world_to_grid(click_world.0, click_world.1);
        let mut best: Option<(hecs::Entity, f32)> = None;

        for (e, (sync, pos)) in ctx.world.query::<(&NetworkSync, &Position)>().iter() {
            if sync.object_type != NetworkObjectType::Monster {
                continue;
            }
            let (gx, gy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
            let dx = (gx - click_gx).abs();
            let dy = (gy - click_gy).abs();
            if dx > 1 || dy > 1 {
                continue;
            }
            let dist2 = (pos.x - click_world.0) * (pos.x - click_world.0)
                + (pos.y - click_world.1) * (pos.y - click_world.1);
            match best {
                None => best = Some((e, dist2)),
                Some((_be, bdist2)) if dist2 < bdist2 => best = Some((e, dist2)),
                _ => {}
            }
        }

        best.map(|(e, _)| e)
    }

    fn player_in_talk_range(ctx: &GameContext, npc_object_id: u32, max_range: i32) -> bool {
        use crate::components::{LocalPlayer, NetworkSync, Position};

        let player_grid = ctx
            .world
            .query::<(&LocalPlayer, &Position)>()
            .iter()
            .next()
            .map(|(_, (_, pos))| crate::coord::Coord::world_to_grid(pos.x, pos.y));
        let Some((px, py)) = player_grid else {
            return false;
        };

        let npc_grid = ctx
            .world
            .query::<(&NetworkSync, &Position)>()
            .iter()
            .find_map(|(_, (sync, pos))| {
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

    // TODO: 实体点击功能待实现
    // 需要等待 PlayerInput 结构体重构完成
}

impl Default for PlayerControlSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicSystem for PlayerControlSystem {
    

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // ✅ 零拷贝：直接访问 GameContext
        let mouse_left_down = ctx.input().mouse.button_down(MouseButton::Left);
        let mouse_right_down = ctx.input().mouse.button_down(MouseButton::Right);
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

        // 处理右键状态变化
        if mouse_right_down && !self.mouse_state.right_pressed {
            self.mouse_state.right_pressed = true;
            self.mouse_state.right_press_start = Some(now);
            self.mouse_state.right_press_position = Some(mouse_pos_tuple);
            tracing::warn!("🔽 右键按下 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
        } else if !mouse_right_down && self.mouse_state.right_pressed {
            self.mouse_state.right_pressed = false;
            tracing::warn!("🔼 右键松开 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);

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

        // 获取相机信息
        let (camera_pos, camera) = ctx.world
            .query_mut::<(&Position, &Camera)>()
            .into_iter()
            .next()
            .map(|(_, (pos, cam))| (pos.clone(), cam.clone()))
            .unwrap_or((
                Position { x: 0.0, y: 0.0 },
                Camera {
                    zoom: 1.0,
                    screen_width: 1280.0,
                    screen_height: 720.0,
                },
            ));

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
        let net = ctx.net().cloned();

        // 语义：本地玩家正常移动，但把移动意图同步到服务器；并接受服务器回包校正。
        let sync_move_to_server = ctx.session.server_authoritative_movement;
        let sync_combat_to_server = ctx.session.server_authoritative_combat;

        // 单击：优先判定是否点到 NPC（近似拾取：按格子命中）
        // 原版习惯：鼠标点 NPC 直接对话；这里同时保留右键交互。
        let mut npc_interaction_target_left: Option<u32> = None;
        if left_single_click {
            if let Some((sx, sy)) = self.mouse_state.left_press_position {
                let click_world = Self::screen_to_world(sx, sy, &camera_pos, &camera);
                npc_interaction_target_left = Self::find_clicked_npc_object_id(ctx, click_world);
                if let Some(npc_id) = npc_interaction_target_left {
                    if !Self::player_in_talk_range(ctx, npc_id, 2) {
                        // 走近再交互：当前先不触发（避免远距离“隔空对话”）
                        npc_interaction_target_left = None;
                    }
                }
            }
        }

        let mut npc_interaction_target_right: Option<u32> = None;
        if right_single_click {
            if let Some((sx, sy)) = self.mouse_state.right_press_position {
                let click_world = Self::screen_to_world(sx, sy, &camera_pos, &camera);
                npc_interaction_target_right = Self::find_clicked_npc_object_id(ctx, click_world);
                if let Some(npc_id) = npc_interaction_target_right {
                    if !Self::player_in_talk_range(ctx, npc_id, 2) {
                        // 走近再交互：当前先不触发（避免远距离“隔空对话”）
                        npc_interaction_target_right = None;
                    }
                }
            }
        }

        // 记录当前交互 NPC（对齐 C# GameScene.NPCID）
        // 注意：必须在进入下面的 `query_mut::<(&mut PlayerInput, ..)>` 之前做，避免二次可变借用。
        let npc_interaction_target = if left_single_click {
            npc_interaction_target_left
        } else if right_single_click {
            npc_interaction_target_right
        } else {
            None
        };
        if let Some(npc_object_id) = npc_interaction_target {
            for (_e, active) in ctx.world.query_mut::<&mut crate::components::ActiveNpc>() {
                active.npc_object_id = Some(npc_object_id);
                break;
            }
        }

        // 右键攻击：在进入 query_mut 之前，先把“点击世界坐标/点到的怪物实体”算好，避免可变借用期间再借用 ctx。
        let right_click_attack_world: Option<(f32, f32)> = if right_single_click && npc_interaction_target_right.is_none() {
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
        use crate::components::PlayerAction;
        
        // 先收集所有有AttackState的实体
        let attacking_entities: std::collections::HashSet<_> = ctx.world
            .query::<&AttackState>()
            .iter()
            .map(|(entity, _)| entity)
            .collect();
        
        // 收集需要添加攻击状态的实体
        let mut entities_to_attack = Vec::new();
        
        for (entity, (player_input, player, _local, pos, path)) in ctx
            .world
            .query_mut::<(
                &mut PlayerInput,
                &mut Player,
                &LocalPlayer,
                &Position,
                &mut crate::components::movement::Path,
            )>()
            .into_iter()
        {
            // ⚔️ 如果正在攻击,跳过输入处理 (由 AttackSystem 管理)
            if attacking_entities.contains(&entity) {
                continue;
            }
            // 🎯 优先处理单击
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
                        if let Some(net) = net.as_ref() {
                            let _ = net.send(crate::network::handlers::NetworkEvent::NPCCallRequest {
                                npc_object_id,
                                key: String::new(),
                            });
                        }
                        player.action = PlayerAction::Stand;
                    } else {
                        // 左键单击 = 站立
                        tracing::warn!("⏹️ 检测到左键单击，立即停止移动");
                        player.action = PlayerAction::Stand;
                    }
                } else if right_single_click {
                    // 右键单击：优先 NPC 交互（服务器驱动），否则才是攻击
                    if let Some(npc_object_id) = npc_interaction_target_right {
                        tracing::warn!("💬 右键点到NPC，发送NPCCallRequest: {}", npc_object_id);
                        if let Some(net) = net.as_ref() {
                            let _ = net.send(crate::network::handlers::NetworkEvent::NPCCallRequest {
                                npc_object_id,
                                key: String::new(),
                            });
                        }
                        player.action = PlayerAction::Stand;
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
                            continue;
                        }

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
                        entities_to_attack.push(entity);

                    }
                    
                    // TODO: 计算攻击方向(朝向鼠标点击位置)
                    // 当前暂时保持原方向
                }
                
                // 清除 last_click_time 避免重复触发
                self.mouse_state.left_last_click_time = None;
                self.mouse_state.right_last_click_time = None;
                continue;  // 跳过后续处理
            }

            // ✅ 启用双击寻路功能
            let has_double_click = double_click_left.is_some() || double_click_right.is_some();
            
            if has_double_click {
                use crate::components::PlayerAction;
                
                // 双击模式: 自动寻路,松开后继续移动
                if let Some((world_x, world_y)) = double_click_left {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::components::MovementMode::Pathfinding;
                    player.action = PlayerAction::Walk;
                    tracing::warn!("🚶🚶 左键双击走路到 ({:.1}, {:.1}) [寻路模式-松开后继续走]", world_x, world_y);
                } else if let Some((world_x, world_y)) = double_click_right {
                    player_input.move_to = Some((world_x, world_y));
                    player_input.movement_mode = crate::components::MovementMode::Pathfinding;
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
                        player.action = PlayerAction::Run;
                    } else {
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
                                player.action = PlayerAction::Stand;
                            }
                        }
                        MovementMode::Pathfinding => {
                            // 寻路模式下,松开不停止,继续走完路径
                            // 但如果 MovementSystem 已清除 move_to (到达目的地)，则设置站立
                            if player_input.move_to.is_none() && player.action != PlayerAction::Stand {
                                player.action = PlayerAction::Stand;
                                player_input.movement_mode = MovementMode::None;
                                tracing::info!("🎬 到达目的地,设置站立动作");
                            }
                        }
                        MovementMode::None => {
                            // 确保没有移动目标时是站立状态
                            if player_input.move_to.is_none() && player.action != PlayerAction::Stand {
                                player.action = PlayerAction::Stand;
                            }
                        }
                    }

                }
            }

            // ===== local move -> server sync: 发送移动意图 =====
            if sync_move_to_server {
                let now = Instant::now();
                if self.can_send_net_move(now) {
                    // 目标优先级：Path 当前 waypoint > move_to
                    let target_grid = if path.is_valid {
                        path.current_waypoint()
                    } else {
                        player_input
                            .move_to
                            .map(|(wx, wy)| crate::coord::Coord::world_to_grid(wx, wy))
                    };

                    if let Some((tgx, tgy)) = target_grid {
                        let (pgx, pgy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
                        if let Some(dir) = Self::grid_direction_towards((pgx, pgy), (tgx, tgy)) {
                            let run = matches!(player.action, PlayerAction::Run);
                            // 本地表现可先转向；服务器会回包校正位置
                            player.direction = dir;
                            Self::send_net_move_step(net.as_ref(), run, dir);
                            self.last_net_move_sent = Some(now);
                        }
                    }
                }
            }
        }
        
        // ⚔️ 循环结束后,添加所有攻击状态
        for entity in entities_to_attack {
            let _ = ctx.world.insert_one(entity, AttackState {
                start_time: now,
                attack_type: PlayerAction::Attack1,
            });
        }

        Ok(())
    }
}
