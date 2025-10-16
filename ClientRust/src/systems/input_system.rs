// InputSystem - 统一处理鼠标、键盘输入
//
// 职责:
// - 统一的输入处理入口
// - 支持按键映射和组合键
// - 可以记录输入历史 (用于回放、作弊检测)

use ggez::input::keyboard::KeyCode;
use ggez::input::mouse::MouseButton;
use ggez::Context;
use mir2_shared::Point;

/// 按钮状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Released,    // 未按下
    Pressed,     // 刚按下 (单帧)
    Held,        // 持续按下
}

/// 鼠标状态
#[derive(Debug, Clone)]
pub struct MouseState {
    pub position: Point,              // 当前鼠标位置
    pub left_button: ButtonState,     // 左键状态
    pub right_button: ButtonState,    // 右键状态
    pub middle_button: ButtonState,   // 中键状态
    pub scroll_delta: f32,            // 滚轮增量
    
    // 前一帧的按钮状态 (用于检测 Pressed/Released)
    prev_left: bool,
    prev_right: bool,
    prev_middle: bool,
}

impl MouseState {
    fn new() -> Self {
        Self {
            position: Point::new(0, 0),
            left_button: ButtonState::Released,
            right_button: ButtonState::Released,
            middle_button: ButtonState::Released,
            scroll_delta: 0.0,
            prev_left: false,
            prev_right: false,
            prev_middle: false,
        }
    }
}

/// 键盘状态
#[derive(Debug, Clone)]
pub struct KeyboardState {
    // 当前按下的按键
    pressed_keys: Vec<KeyCode>,
}

impl KeyboardState {
    fn new() -> Self {
        Self {
            pressed_keys: Vec::new(),
        }
    }
    
    /// 检查某个按键是否按下
    /// 注意: 由于 ggez 的限制，此方法需要 Context，暂时返回 false
    /// 实际使用时应直接调用 ctx.keyboard.is_key_pressed()
    pub fn is_key_down(&self, _key: KeyCode) -> bool {
        // TODO: 保存键盘状态需要在 update() 中使用 is_key_pressed() 遍历所有需要的按键
        false
    }
}

/// 游戏动作枚举 (用于快捷键绑定)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    CloseAll,        // ESC - 关闭所有对话框
    OpenInventory,   // I - 打开背包
    OpenCharacter,   // C - 打开角色
    OpenSkills,      // S - 打开技能
    OpenQuest,       // Q - 打开任务
    // TODO: 添加更多动作
}

/// 快捷键配置
#[derive(Debug, Clone)]
pub struct KeybindConfig {
    bindings: std::collections::HashMap<GameAction, KeyCode>,
}

impl KeybindConfig {
    fn new() -> Self {
        let mut bindings = std::collections::HashMap::new();
        
        // 默认快捷键
        bindings.insert(GameAction::CloseAll, KeyCode::Escape);
        bindings.insert(GameAction::OpenInventory, KeyCode::KeyI);
        bindings.insert(GameAction::OpenCharacter, KeyCode::KeyC);
        bindings.insert(GameAction::OpenSkills, KeyCode::KeyS);
        bindings.insert(GameAction::OpenQuest, KeyCode::KeyQ);
        
        Self { bindings }
    }
    
    /// 获取动作对应的按键
    pub fn get_key(&self, action: GameAction) -> Option<KeyCode> {
        self.bindings.get(&action).copied()
    }
}

/// 输入系统
pub struct InputSystem {
    mouse_state: MouseState,
    keyboard_state: KeyboardState,
    keybind_config: KeybindConfig,
}

impl InputSystem {
    /// 创建新的输入系统
    pub fn new() -> Self {
        Self {
            mouse_state: MouseState::new(),
            keyboard_state: KeyboardState::new(),
            keybind_config: KeybindConfig::new(),
        }
    }
    
    /// 每帧主动读取输入状态
    pub fn update(&mut self, ctx: &Context) {
        // 更新鼠标位置
        let pos = ctx.mouse.position();
        self.mouse_state.position = Point::new(pos.x as i32, pos.y as i32);
        

        
        // 更新鼠标按钮状态
        let left_down = ctx.mouse.button_pressed(MouseButton::Left);
        let right_down = ctx.mouse.button_pressed(MouseButton::Right);
        let middle_down = ctx.mouse.button_pressed(MouseButton::Middle);
        
        self.mouse_state.left_button = self.get_button_state(
            left_down,
            self.mouse_state.prev_left,
        );
        self.mouse_state.right_button = self.get_button_state(
            right_down,
            self.mouse_state.prev_right,
        );
        self.mouse_state.middle_button = self.get_button_state(
            middle_down,
            self.mouse_state.prev_middle,
        );
        
        // 保存当前帧状态
        self.mouse_state.prev_left = left_down;
        self.mouse_state.prev_right = right_down;
        self.mouse_state.prev_middle = middle_down;
        
        // 更新键盘状态
        // 注意: ggez 不提供 pressed_keys() 方法，需要使用 is_key_pressed() 单独检查
        // 暂时不实现键盘状态列表，按需检查即可
        self.keyboard_state.pressed_keys.clear();
    }
    
    /// 计算按钮状态
    fn get_button_state(&self, current_down: bool, prev_down: bool) -> ButtonState {
        match (current_down, prev_down) {
            (true, false) => ButtonState::Pressed,   // 刚按下
            (true, true) => ButtonState::Held,       // 持续按下
            (false, _) => ButtonState::Released,     // 未按下或刚释放
        }
    }
    
    /// 检查快捷键是否触发
    pub fn check_keybind(&self, action: GameAction) -> bool {
        if let Some(key) = self.keybind_config.get_key(action) {
            self.keyboard_state.is_key_down(key)
        } else {
            false
        }
    }
    
    /// 获取鼠标状态 (只读)
    pub fn mouse(&self) -> &MouseState {
        &self.mouse_state
    }
    
    /// 获取键盘状态 (只读)
    pub fn keyboard(&self) -> &KeyboardState {
        &self.keyboard_state
    }
    
    // ==================== 游戏动作处理 ====================
    
    /// 获取移动输入 (左键走路,右键跑步)
    /// 返回: Some(是否跑步) 或 None(无移动输入)
    pub fn get_move_input(&self) -> Option<bool> {
        // 右键优先 = 跑步
        match self.mouse_state.right_button {
            ButtonState::Pressed | ButtonState::Held => return Some(true),
            _ => {}
        }
        
        // 左键 = 走路
        match self.mouse_state.left_button {
            ButtonState::Pressed | ButtonState::Held => return Some(false),
            _ => {}
        }
        
        None
    }
    
    /// 获取攻击/拾取输入 (左键按下)
    pub fn get_action_input(&self) -> bool {
        matches!(
            self.mouse_state.left_button,
            ButtonState::Pressed | ButtonState::Held
        )
    }
    
    /// 获取鼠标世界坐标 (需要传入 Camera)
    pub fn get_mouse_world_pos(&self, screen_to_world: impl Fn(Point) -> Point) -> Point {
        screen_to_world(self.mouse_state.position)
    }
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}
