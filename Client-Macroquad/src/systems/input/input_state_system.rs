// ============================================================================
// InputStateSystem - 输入状态追踪系统
// ============================================================================
//
// **职责**：
// - 在每帧开始时记录上一帧的输入状态
// - 为其他 System 提供边缘检测的数据基础
// - 统一管理所有输入状态追踪逻辑
//
// **设计原则**：
// - ✅ 单一职责：只负责输入状态追踪
// - ✅ 逻辑内聚：所有边缘检测的状态更新集中在此
// - ✅ 高优先级：优先级 10，在所有输入处理系统之前执行
//
// **执行时机（建议）**：
// - 让 InputStateSystem 在“读取输入的系统”之后执行（帧末尾），
//   这样它记录的就是“本帧状态”，供下一帧做边缘检测。
// - 如果未来引入 begin_frame/end_frame 两阶段调度，可把它放到 end_frame。
//
// **使用示例**：
// ```rust
// // 在其他 System 中查询 InputState
// let input_state = ctx.world.query::<&InputState>().iter().next().map(|(_, s)| s);
// if let Some(state) = input_state {
//     if ctx.input().key_pressed(KeyCode::O) && !state.prev_pressed_keys.contains(&KeyCode::O) {
//         // 键刚按下（边缘检测）
//     }
// }
// ```
//
// ============================================================================

use crate::{
    components::InputState,
    game::GameContext,
    systems::LogicSystem,
};
use crate::game::{GameResult, KeyCode};

/// 需要追踪的按键列表（用于边缘检测）
/// 
/// 只追踪需要边缘检测的键，避免不必要的开销
const KEYS_TO_TRACK: &[KeyCode] = &[
    // 调试键
    KeyCode::O,
    KeyCode::P,
    KeyCode::G,
    KeyCode::B,
    KeyCode::A,
    KeyCode::S,
    KeyCode::D,
    KeyCode::L,
    KeyCode::M,
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::Equal,
    KeyCode::KpAdd,
    KeyCode::Minus,
    KeyCode::KpSubtract,
    KeyCode::Escape,
    
    // UI 交互键（未来可能需要）
    KeyCode::Enter,
    KeyCode::KpEnter,
    KeyCode::Space,
    KeyCode::Tab,
    KeyCode::Backspace,
    KeyCode::Delete,
];

/// 输入状态追踪系统
#[derive(ecs_macros::LogicSystem)]
pub struct InputStateSystem;

impl InputStateSystem {
    pub fn new() -> Self {
        Self
    }
}

impl LogicSystem for InputStateSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 关键设计：在执行本系统时，把“当前帧输入状态”写入 prev_*，
        // 供下一帧其他系统进行边缘检测（key_just_pressed 等）
        
        // 查询唯一的 InputState 组件
        let mut query = ctx.world.query::<&mut InputState>();
        
        if let Some(state) = query.iter().next() {
            // ===== 更新键盘状态（为下一帧准备）=====
            state.prev_pressed_keys.clear();
            
            for &key in KEYS_TO_TRACK {
                if ctx.input().key_pressed(key) {
                    state.prev_pressed_keys.insert(key);
                }
            }
            
            // ===== 更新鼠标状态（为下一帧准备）=====
            state.prev_mouse_left = ctx.input().mouse_left_pressed();
            state.prev_mouse_right = ctx.input().mouse_right_pressed();
            state.prev_mouse_middle = ctx.input().mouse_middle_pressed();
        }
        
        Ok(())
    }
}
