use crate::ecs::components::InputEvent;
use crate::{
    ecs::WorldExt,
    network::{builder::CategorizedEvents, NetContext},
};
use crate::{network::NetworkBuilder, ClientSettings};

use ggez::{
    conf::Conf,
    context::{ContextFields, Has, HasMut},
    filesystem::Filesystem,
    graphics::GraphicsContext,
    input::{gamepad::GamepadContext, keyboard::KeyboardContext, mouse::MouseContext},
    timer::TimeContext,
};
use hecs::World;

/// 自定义游戏上下文
///
/// 包含 ggez 的所有核心组件 + ECS World + 网络上下文
pub struct GameContext {
    // ===== ggez 核心组件 =====
    pub fs: Filesystem,
    pub gfx: GraphicsContext,
    pub keyboard: KeyboardContext,
    pub mouse: MouseContext,
    pub gamepad: GamepadContext,
    pub time: TimeContext,
    pub fields: ContextFields,

    // ===== 游戏特定组件 =====
    pub settings: ClientSettings,
    /// ECS World - 存储所有实体和组件
    pub world: World,
    /// 网络事件（本帧收集的）
    pub net_events: CategorizedEvents,
    /// 帧临时输入事件缓冲（从 EventHandler 收集）
    pub frame_input_events: Vec<InputEvent>,
}

impl GameContext {

    pub fn builder(
        game_id: String,
        conf: Conf,
        fs: Filesystem,
        settings: ClientSettings,
    ) -> ggez::GameResult<(Self, ggez::winit::event_loop::EventLoop<()>)> {
        let event_loop = ggez::winit::event_loop::EventLoop::new()?;
        let timer_context = TimeContext::new();
        let graphics_context = GraphicsContext::new(&game_id, &event_loop, &conf, &fs)?;

        let mut world = World::new();
        let net_ctx = NetworkBuilder::new(settings.network.clone())
            .build()
            .map_err(|err| {
                ggez::GameError::CustomError(format!("Failed to create network: {}", err))
            })?;
        world.spawn_network(net_ctx);

        // NetContext 的实体 ID 已经在 spawn 时固定为 NETWORK_ENTITY

        let ctx = Self {
            fs,
            gfx: graphics_context,
            time: timer_context,
            keyboard: KeyboardContext::new(),
            mouse: MouseContext::new(),
            gamepad: GamepadContext::new()?,
            fields: ContextFields {
                conf,
                continuing: true,
                quit_requested: false,
            },
            world,
            frame_input_events: Vec::new(),
            net_events: CategorizedEvents::default(),
            settings,
        };

        Ok((ctx, event_loop))
    }

    /// 获取 ggez::Context 的可变引用
    ///
    /// 用于需要原始 ggez Context 的场景（如渲染）
    ///
    /// 安全性：CustomContext 通过实现 Has/HasMut trait 保证与 ggez::Context 兼容
    pub fn as_ggez_context(&mut self) -> &mut ggez::Context {
        unsafe { std::mem::transmute(self) }
    }

    pub fn collect_network_events(&mut self) {
        let events = self.network().recv_categorized();
        self.net_events = events;
    }

    /// 清空本帧的输入事件
    pub fn clear_frame_events(&mut self) {
        self.frame_input_events.clear();
    }

    /// 添加输入事件到帧缓冲
    pub fn push_input_event(&mut self, event: InputEvent) {
        self.frame_input_events.push(event);
    }
}

// ============================================================================
// 实现 ggez 的 Has/HasMut trait
// ============================================================================
// 这些实现允许 CustomContext 作为 ggez::Context 的替代品使用

impl Has<Filesystem> for GameContext {
    #[inline]
    fn retrieve(&self) -> &Filesystem {
        &self.fs
    }
}

impl Has<GraphicsContext> for GameContext {
    #[inline]
    fn retrieve(&self) -> &GraphicsContext {
        &self.gfx
    }
}

impl Has<KeyboardContext> for GameContext {
    #[inline]
    fn retrieve(&self) -> &KeyboardContext {
        &self.keyboard
    }
}

impl Has<MouseContext> for GameContext {
    #[inline]
    fn retrieve(&self) -> &MouseContext {
        &self.mouse
    }
}

impl Has<GamepadContext> for GameContext {
    #[inline]
    fn retrieve(&self) -> &GamepadContext {
        &self.gamepad
    }
}

impl Has<TimeContext> for GameContext {
    #[inline]
    fn retrieve(&self) -> &TimeContext {
        &self.time
    }
}

impl Has<ContextFields> for GameContext {
    #[inline]
    fn retrieve(&self) -> &ContextFields {
        &self.fields
    }
}

// ===== HasMut 实现 =====

impl HasMut<ContextFields> for GameContext {
    #[inline]
    fn retrieve_mut(&mut self) -> &mut ContextFields {
        &mut self.fields
    }
}

impl HasMut<GraphicsContext> for GameContext {
    #[inline]
    fn retrieve_mut(&mut self) -> &mut GraphicsContext {
        &mut self.gfx
    }
}

impl HasMut<TimeContext> for GameContext {
    #[inline]
    fn retrieve_mut(&mut self) -> &mut TimeContext {
        &mut self.time
    }
}

impl HasMut<KeyboardContext> for GameContext {
    #[inline]
    fn retrieve_mut(&mut self) -> &mut KeyboardContext {
        &mut self.keyboard
    }
}

impl HasMut<MouseContext> for GameContext {
    #[inline]
    fn retrieve_mut(&mut self) -> &mut MouseContext {
        &mut self.mouse
    }
}

impl HasMut<GamepadContext> for GameContext {
    #[inline]
    fn retrieve_mut(&mut self) -> &mut GamepadContext {
        &mut self.gamepad
    }
}

// ============================================================================
// 扩展：访问游戏特定资源
// ============================================================================

impl GameContext {
    /// 获取 ClientSettings
    pub fn settings(&self) -> Option<hecs::Ref<'_, ClientSettings>> {
        if let Some(entity) = crate::ecs::SETTING_ENTITY {
            self.world.get::<&ClientSettings>(entity).ok()
        } else {
            None
        }
    }

    /// 获取 NetContext
    pub fn network(&self) -> hecs::Ref<'_, NetContext> {
        self.world
            .get::<&NetContext>(crate::ecs::NETWORK_ENTITY.unwrap_or(hecs::Entity::DANGLING))
            .expect("NetContext not found in World")
    }

    // ===== 便捷方法：ECS 查询 =====

    /// 获取实体数量
    pub fn entity_count(&self) -> usize {
        self.world.len() as usize
    }

    /// 检查实体是否存在
    pub fn entity_exists(&self, entity: hecs::Entity) -> bool {
        self.world.contains(entity)
    }

    // ===== 便捷方法：时间相关 =====

    /// 获取帧间隔时间 (秒)
    pub fn delta_time(&self) -> f32 {
        self.time.delta().as_secs_f32()
    }

    /// 获取游戏运行总时间 (秒)
    pub fn time_since_start(&self) -> f64 {
        self.time.time_since_start().as_secs_f64()
    }

    /// 获取当前 FPS
    pub fn fps(&self) -> f32 {
        self.time.fps() as f32
    }

    // ===== 便捷方法：屏幕尺寸 =====

    /// 获取屏幕宽度
    pub fn screen_width(&self) -> f32 {
        self.gfx.drawable_size().0
    }

    /// 获取屏幕高度
    pub fn screen_height(&self) -> f32 {
        self.gfx.drawable_size().1
    }

    /// 获取屏幕尺寸 (宽, 高)
    pub fn screen_size(&self) -> (f32, f32) {
        self.gfx.drawable_size()
    }

    // ===== 便捷方法：鼠标 =====

    /// 鼠标左键是否按下
    pub fn mouse_left_pressed(&self) -> bool {
        self.mouse
            .button_pressed(ggez::input::mouse::MouseButton::Left)
    }

    /// 鼠标右键是否按下
    pub fn mouse_right_pressed(&self) -> bool {
        self.mouse
            .button_pressed(ggez::input::mouse::MouseButton::Right)
    }

    /// 鼠标中键是否按下
    pub fn mouse_middle_pressed(&self) -> bool {
        self.mouse
            .button_pressed(ggez::input::mouse::MouseButton::Middle)
    }

    /// 鼠标按钮是否按下
    pub fn mouse_button_pressed(
        &self,
        button: ggez::input::mouse::MouseButton,
    ) -> Option<(ggez::input::mouse::MouseButton, f32, f32)> {
        if self.mouse.button_pressed(button) {
            let pos = self.mouse.position();
            Some((button, pos.x, pos.y))
        } else {
            None
        }
    }

    /// 获取鼠标位置
    pub fn mouse_position(&self) -> (f32, f32) {
        let pos = self.mouse.position();
        (pos.x, pos.y)
    }

    /// 获取鼠标 X 坐标
    pub fn mouse_x(&self) -> f32 {
        self.mouse.position().x
    }

    /// 获取鼠标 Y 坐标
    pub fn mouse_y(&self) -> f32 {
        self.mouse.position().y
    }

    /// 鼠标是否在屏幕内
    pub fn mouse_in_bounds(&self) -> bool {
        let (x, y) = self.mouse_position();
        let (w, h) = self.gfx.drawable_size();
        x >= 0.0 && x < w && y >= 0.0 && y < h
    }

    /// 鼠标移动事件
    pub fn mouse_motion(&self) -> impl Iterator<Item = (f32, f32, f32, f32)> + '_ {
        self.frame_input_events
            .iter()
            .filter_map(|event| match event {
                InputEvent::MouseMove { x, y, dx, dy } => Some((*x, *y, *dx, *dy)),
                _ => None,
            })
    }

    /// 鼠标滚轮事件
    pub fn mouse_wheel(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        self.frame_input_events
            .iter()
            .filter_map(|event| match event {
                InputEvent::MouseWheel { x, y } => Some((*x, *y)),
                _ => None,
            })
    }

    /// 鼠标进入或离开窗口
    pub fn mouse_entered_or_leaved(&self) -> Option<bool> {
        self.frame_input_events
            .iter()
            .find_map(|event| match event {
                InputEvent::MouseEnterOrLeave { entered } => Some(*entered),
                _ => None,
            })
    }

    /// 鼠标进入窗口
    pub fn mouse_entered(&self) -> Option<bool> {
        self.frame_input_events
            .iter()
            .find_map(|event| match event {
                InputEvent::MouseEnterOrLeave { entered } => Some(*entered),
                _ => None,
            })
    }

    /// 鼠标离开窗口
    pub fn mouse_leaved(&self) -> Option<bool> {
        self.frame_input_events
            .iter()
            .find_map(|event| match event {
                InputEvent::MouseEnterOrLeave { entered } => Some(!*entered),
                _ => None,
            })
    }

    /// 获取鼠标到屏幕中心的距离
    pub fn mouse_distance_to_center(&self) -> f32 {
        let (mx, my) = self.mouse_position();
        let (w, h) = self.gfx.drawable_size();
        let cx = w / 2.0;
        let cy = h / 2.0;
        let dx = mx - cx;
        let dy = my - cy;
        (dx * dx + dy * dy).sqrt()
    }

    /// 获取鼠标相对于屏幕中心的角度（弧度）
    pub fn mouse_angle_from_center(&self) -> f32 {
        let (mx, my) = self.mouse_position();
        let (w, h) = self.gfx.drawable_size();
        let cx = w / 2.0;
        let cy = h / 2.0;
        (my - cy).atan2(mx - cx)
    }

    /// 检查鼠标是否在指定矩形区域内
    pub fn mouse_in_rect(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let (mx, my) = self.mouse_position();
        mx >= x && mx < x + w && my >= y && my < y + h
    }

    /// 检查鼠标是否在圆形区域内
    pub fn mouse_in_circle(&self, cx: f32, cy: f32, radius: f32) -> bool {
        let (mx, my) = self.mouse_position();
        let dx = mx - cx;
        let dy = my - cy;
        (dx * dx + dy * dy) <= (radius * radius)
    }

    /// 鼠标到点的距离
    pub fn mouse_distance_to(&self, x: f32, y: f32) -> f32 {
        let (mx, my) = self.mouse_position();
        let dx = x - mx;
        let dy = y - my;
        (dx * dx + dy * dy).sqrt()
    }

    // ===== 便捷方法：键盘 =====

    /// 迭代本帧的文本输入事件
    pub fn text_input(&self) -> impl Iterator<Item = char> + '_ {
        self.frame_input_events.iter().filter_map(|event| {
            if let InputEvent::Ime { character, .. } = event {
                Some(*character)
            } else {
                None
            }
        })
    }

    /// 获取当前按下的所有按键
    pub fn pressed_keys(
        &self,
    ) -> impl Iterator<
        Item = (
            ggez::input::keyboard::KeyCode,
            Option<ggez::winit::keyboard::SmolStr>,
        ),
    > + '_ {
        self.keyboard.pressed_physical_keys.iter().filter_map(|&k| {
            if let ggez::winit::keyboard::PhysicalKey::Code(key) = k {
                Some((key, None))
            } else {
                None
            }
        })
    }

    /// 检查指定键是否按下
    pub fn key_pressed(&self, key: ggez::input::keyboard::KeyCode) -> bool {
        self.keyboard
            .is_physical_key_pressed(&ggez::winit::keyboard::PhysicalKey::Code(key))
    }

    /// 检查 Shift 键是否按下
    pub fn shift_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::ShiftLeft)
            || self.key_pressed(ggez::input::keyboard::KeyCode::ShiftRight)
    }

    /// 检查 Ctrl 键是否按下
    pub fn ctrl_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::ControlLeft)
            || self.key_pressed(ggez::input::keyboard::KeyCode::ControlRight)
    }

    /// 检查 Alt 键是否按下
    pub fn alt_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::AltLeft)
            || self.key_pressed(ggez::input::keyboard::KeyCode::AltRight)
    }

    /// 检查空格键是否按下
    pub fn space_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::Space)
    }

    /// 检查回车键是否按下
    pub fn enter_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::Enter)
            || self.key_pressed(ggez::input::keyboard::KeyCode::NumpadEnter)
    }

    /// 检查 ESC 键是否按下
    pub fn escape_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::Escape)
    }

    /// 检查 Tab 键是否按下
    pub fn tab_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::Tab)
    }

    /// 检查退格键是否按下
    pub fn backspace_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::Backspace)
    }

    /// 检查删除键是否按下
    pub fn delete_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::Delete)
    }

    // ===== 方向键方法 =====

    /// 检查上箭头键是否按下
    pub fn arrow_up_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::ArrowUp)
    }

    /// 检查下箭头键是否按下
    pub fn arrow_down_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::ArrowDown)
    }

    /// 检查左箭头键是否按下
    pub fn arrow_left_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::ArrowLeft)
    }

    /// 检查右箭头键是否按下
    pub fn arrow_right_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::ArrowRight)
    }

    /// 获取方向键的方向向量 (x, y)，返回 (-1, 0, 1) 的组合
    pub fn arrow_direction(&self) -> (i32, i32) {
        let x = if self.arrow_right_pressed() {
            1
        } else if self.arrow_left_pressed() {
            -1
        } else {
            0
        };
        let y = if self.arrow_down_pressed() {
            1
        } else if self.arrow_up_pressed() {
            -1
        } else {
            0
        };
        (x, y)
    }

    // ===== WASD 方法 =====

    /// 检查 W 键是否按下
    pub fn w_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::KeyW)
    }

    /// 检查 A 键是否按下
    pub fn a_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::KeyA)
    }

    /// 检查 S 键是否按下
    pub fn s_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::KeyS)
    }

    /// 检查 D 键是否按下
    pub fn d_pressed(&self) -> bool {
        self.key_pressed(ggez::input::keyboard::KeyCode::KeyD)
    }

    /// 获取 WASD 的方向向量 (x, y)，返回 (-1, 0, 1) 的组合
    pub fn wasd_direction(&self) -> (i32, i32) {
        let x = if self.d_pressed() {
            1
        } else if self.a_pressed() {
            -1
        } else {
            0
        };
        let y = if self.s_pressed() {
            1
        } else if self.w_pressed() {
            -1
        } else {
            0
        };
        (x, y)
    }

    // ===== 数字键方法 =====

    /// 检查数字键 0-9 是否按下
    pub fn digit_pressed(&self, digit: u8) -> bool {
        if digit > 9 {
            return false;
        }

        use ggez::input::keyboard::KeyCode;
        let key = match digit {
            0 => KeyCode::Digit0,
            1 => KeyCode::Digit1,
            2 => KeyCode::Digit2,
            3 => KeyCode::Digit3,
            4 => KeyCode::Digit4,
            5 => KeyCode::Digit5,
            6 => KeyCode::Digit6,
            7 => KeyCode::Digit7,
            8 => KeyCode::Digit8,
            9 => KeyCode::Digit9,
            _ => return false,
        };

        self.key_pressed(key)
    }

    /// 检查小键盘数字键是否按下
    pub fn numpad_digit_pressed(&self, digit: u8) -> bool {
        if digit > 9 {
            return false;
        }

        use ggez::input::keyboard::KeyCode;
        let key = match digit {
            0 => KeyCode::Numpad0,
            1 => KeyCode::Numpad1,
            2 => KeyCode::Numpad2,
            3 => KeyCode::Numpad3,
            4 => KeyCode::Numpad4,
            5 => KeyCode::Numpad5,
            6 => KeyCode::Numpad6,
            7 => KeyCode::Numpad7,
            8 => KeyCode::Numpad8,
            9 => KeyCode::Numpad9,
            _ => return false,
        };

        self.key_pressed(key)
    }

    // ===== 功能键方法 =====

    /// 检查 F1-F12 功能键是否按下
    pub fn function_key_pressed(&self, num: u8) -> bool {
        if num < 1 || num > 12 {
            return false;
        }

        use ggez::input::keyboard::KeyCode;
        let key = match num {
            1 => KeyCode::F1,
            2 => KeyCode::F2,
            3 => KeyCode::F3,
            4 => KeyCode::F4,
            5 => KeyCode::F5,
            6 => KeyCode::F6,
            7 => KeyCode::F7,
            8 => KeyCode::F8,
            9 => KeyCode::F9,
            10 => KeyCode::F10,
            11 => KeyCode::F11,
            12 => KeyCode::F12,
            _ => return false,
        };

        self.key_pressed(key)
    }

    /// 获取按下的键数量
    pub fn pressed_key_count(&self) -> usize {
        self.keyboard.pressed_physical_keys.len()
    }

    /// 检查是否有任何键按下
    pub fn any_key_pressed(&self) -> bool {
        !self.keyboard.pressed_physical_keys.is_empty()
    }

    // ===== 组合键方法 =====

    /// 检查 Ctrl + 键 的组合
    pub fn ctrl_key(&self, key: ggez::input::keyboard::KeyCode) -> bool {
        self.ctrl_pressed() && self.key_pressed(key)
    }

    /// 检查 Shift + 键 的组合
    pub fn shift_key(&self, key: ggez::input::keyboard::KeyCode) -> bool {
        self.shift_pressed() && self.key_pressed(key)
    }

    /// 检查 Alt + 键 的组合
    pub fn alt_key(&self, key: ggez::input::keyboard::KeyCode) -> bool {
        self.alt_pressed() && self.key_pressed(key)
    }

    // ===== 常用组合快捷键 =====

    /// 检查是否按下 Ctrl+C（复制）
    pub fn is_copy(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyC)
    }

    /// 检查是否按下 Ctrl+V（粘贴）
    pub fn is_paste(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyV)
    }

    /// 检查是否按下 Ctrl+X（剪切）
    pub fn is_cut(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyX)
    }

    /// 检查是否按下 Ctrl+Z（撤销）
    pub fn is_undo(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyZ)
    }

    /// 检查是否按下 Ctrl+Y 或 Ctrl+Shift+Z（重做）
    pub fn is_redo(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyY)
            || (self.ctrl_pressed()
                && self.shift_pressed()
                && self.key_pressed(ggez::input::keyboard::KeyCode::KeyZ))
    }

    /// 检查是否按下 Ctrl+A（全选）
    pub fn is_select_all(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyA)
    }

    /// 检查是否按下 Ctrl+S（保存）
    pub fn is_save(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyS)
    }

    /// 检查是否按下 Ctrl+O（打开）
    pub fn is_open(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyO)
    }

    /// 检查是否按下 Ctrl+N（新建）
    pub fn is_new(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyN)
    }

    /// 检查是否按下 Ctrl+F（查找）
    pub fn is_find(&self) -> bool {
        self.ctrl_key(ggez::input::keyboard::KeyCode::KeyF)
    }

    // ===== 网络事件访问 =====

    /// 获取所有网络事件的引用
    pub fn net_events(&self) -> &CategorizedEvents {
        &self.net_events
    }

    /// 获取网络事件总数
    pub fn net_event_count(&self) -> usize {
        self.net_events.total_count()
    }

    /// 检查是否有网络事件
    pub fn has_net_events(&self) -> bool {
        !self.net_events.is_empty()
    }

    // ===== 连接事件 =====

    /// 获取连接事件
    pub fn connection_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.connection
    }

    /// 检查是否有连接事件
    pub fn has_connection_events(&self) -> bool {
        !self.net_events.connection.is_empty()
    }

    /// 检查是否已断开连接
    pub fn is_disconnected(&self) -> bool {
        self.net_events
            .connection
            .iter()
            .any(|e| matches!(e, crate::network::handlers::GameEvent::Disconnected { .. }))
    }

    // ===== 认证事件 =====

    /// 获取认证事件
    pub fn auth_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.auth
    }

    /// 检查是否有认证事件
    pub fn has_auth_events(&self) -> bool {
        !self.net_events.auth.is_empty()
    }

    /// 检查是否登录成功
    pub fn is_login_success(&self) -> bool {
        self.net_events
            .auth
            .iter()
            .any(|e| matches!(e, crate::network::handlers::GameEvent::LoginSuccess { .. }))
    }

    /// 检查是否登录失败
    pub fn is_login_failed(&self) -> bool {
        self.net_events
            .auth
            .iter()
            .any(|e| matches!(e, crate::network::handlers::GameEvent::LoginFailed { .. }))
    }

    // ===== 角色管理事件 =====

    /// 获取角色管理事件
    pub fn character_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.character
    }

    /// 检查是否有角色管理事件
    pub fn has_character_events(&self) -> bool {
        !self.net_events.character.is_empty()
    }

    /// 检查是否收到用户信息（角色信息）
    pub fn has_user_information(&self) -> bool {
        self.net_events.character.iter().any(|e| {
            matches!(
                e,
                crate::network::handlers::GameEvent::UserInformation { .. }
            )
        })
    }

    // ===== 玩家状态事件 =====

    /// 获取玩家状态事件
    pub fn player_state_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.player_state
    }

    /// 检查是否有玩家状态事件
    pub fn has_player_state_events(&self) -> bool {
        !self.net_events.player_state.is_empty()
    }

    // ===== 战斗事件 =====

    /// 获取战斗事件
    pub fn combat_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.combat
    }

    /// 检查是否有战斗事件
    pub fn has_combat_events(&self) -> bool {
        !self.net_events.combat.is_empty()
    }

    // ===== 聊天事件 =====

    /// 获取聊天事件
    pub fn chat_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.chat
    }

    /// 检查是否有聊天消息
    pub fn has_chat_events(&self) -> bool {
        !self.net_events.chat.is_empty()
    }

    // ===== 世界对象事件 =====

    /// 获取世界对象事件
    pub fn world_object_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.world_objects
    }

    /// 检查是否有世界对象事件
    pub fn has_world_object_events(&self) -> bool {
        !self.net_events.world_objects.is_empty()
    }

    // ===== 地图事件 =====

    /// 获取地图事件
    pub fn map_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.map
    }

    /// 检查是否有地图事件
    pub fn has_map_events(&self) -> bool {
        !self.net_events.map.is_empty()
    }

    /// 检查是否有地图切换事件
    pub fn has_map_changed(&self) -> bool {
        self.net_events
            .map
            .iter()
            .any(|e| matches!(e, crate::network::handlers::GameEvent::MapChanged { .. }))
    }

    // ===== 物品事件 =====

    /// 获取物品事件
    pub fn item_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.items
    }

    /// 检查是否有物品事件
    pub fn has_item_events(&self) -> bool {
        !self.net_events.items.is_empty()
    }

    // ===== NPC 事件 =====

    /// 获取 NPC 事件
    pub fn npc_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.npc
    }

    /// 检查是否有 NPC 事件
    pub fn has_npc_events(&self) -> bool {
        !self.net_events.npc.is_empty()
    }

    // ===== 其他事件 =====

    /// 获取其他事件
    pub fn other_events(&self) -> &[crate::network::handlers::GameEvent] {
        &self.net_events.other
    }

    /// 检查是否有其他事件
    pub fn has_other_events(&self) -> bool {
        !self.net_events.other.is_empty()
    }

    // ===== 事件过滤和查询 =====

    /// 遍历所有网络事件
    pub fn iter_all_net_events(
        &self,
    ) -> impl Iterator<Item = &crate::network::handlers::GameEvent> {
        self.net_events
            .connection
            .iter()
            .chain(self.net_events.auth.iter())
            .chain(self.net_events.character.iter())
            .chain(self.net_events.player_state.iter())
            .chain(self.net_events.combat.iter())
            .chain(self.net_events.chat.iter())
            .chain(self.net_events.world_objects.iter())
            .chain(self.net_events.map.iter())
            .chain(self.net_events.items.iter())
            .chain(self.net_events.npc.iter())
            .chain(self.net_events.other.iter())
    }

    /// 查找特定类型的事件
    pub fn find_event<F>(&self, predicate: F) -> Option<&crate::network::handlers::GameEvent>
    where
        F: Fn(&crate::network::handlers::GameEvent) -> bool,
    {
        self.iter_all_net_events().find(|e| predicate(e))
    }

    /// 过滤特定类型的事件
    pub fn filter_events<F>(&self, predicate: F) -> Vec<&crate::network::handlers::GameEvent>
    where
        F: Fn(&crate::network::handlers::GameEvent) -> bool,
    {
        self.iter_all_net_events()
            .filter(|e| predicate(e))
            .collect()
    }

    /// 获取本帧的输入事件列表
    pub fn input_events(&self) -> &[InputEvent] {
        &self.frame_input_events
    }

    // ===== 工具方法 =====

    /// 判断点是否在矩形内
    pub fn point_in_rect(
        &self,
        x: f32,
        y: f32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
    ) -> bool {
        x >= rect_x && x < rect_x + rect_w && y >= rect_y && y < rect_y + rect_h
    }

    /// 计算两点距离
    pub fn distance(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let dx = x2 - x1;
        let dy = y2 - y1;
        (dx * dx + dy * dy).sqrt()
    }
}
