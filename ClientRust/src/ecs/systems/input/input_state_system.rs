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
// **执行时机**：
// ```
// 帧开始
//   ↓
// InputStateSystem (优先级 10) - 记录上一帧状态
//   ↓
// PlayerControlSystem (优先级 100) - 使用 InputState 做边缘检测
//   ↓
// DebugSystem (渲染系统) - 使用 InputState 做边缘检测
//   ↓
// 其他系统...
// ```
//
// **使用示例**：
// ```rust
// // 在其他 System 中查询 InputState
// let input_state = ctx.world.query::<&InputState>().iter().next().map(|(_, s)| s);
// if let Some(state) = input_state {
//     if ctx.input().key_pressed(KeyCode::KeyO) && !state.prev_pressed_keys.contains(&KeyCode::KeyO) {
//         // 键刚按下（边缘检测）
//     }
// }
// ```
//
// ============================================================================

use crate::ecs::{
    components::InputState,
    GameContext,
    systems::LogicSystem,
};
use ggez::input::keyboard::KeyCode;
use ggez::GameResult;

/// 需要追踪的按键列表（用于边缘检测）
/// 
/// 只追踪需要边缘检测的键，避免不必要的开销
const KEYS_TO_TRACK: &[KeyCode] = &[
    // 调试键
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyG,
    KeyCode::KeyB,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::Equal,
    KeyCode::NumpadAdd,
    KeyCode::Minus,
    KeyCode::NumpadSubtract,
    KeyCode::Escape,
    
    // UI 交互键（未来可能需要）
    KeyCode::Enter,
    KeyCode::NumpadEnter,
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
        // ⚠️ 关键设计：先使用旧状态，帧末尾才更新
        // 
        // 执行顺序：
        // 1. InputStateSystem (优先级10) - 不做任何事（或者记录旧状态）
        // 2. PlayerControlSystem (优先级100) - 使用 InputState 做边缘检测
        // 3. DebugSystem (优先级9200) - 使用 InputState 做边缘检测
        // 4. 帧末尾 - InputStateSystem 再次执行，更新 InputState
        //
        // 但是ECS系统只会在每帧调用一次update...所以我们需要换个思路：
        // **在当前帧末尾更新 prev_xxx，供下一帧使用**
        //
        // 更好的方案：
        // InputStateSystem 在帧开始时不做任何事
        // 在帧末尾（所有系统执行完后）才更新状态
        // 但这需要修改调度器...
        //
        // 🔧 临时方案：InputStateSystem 优先级设为最低（在所有系统之后）
        // 这样它会在帧末尾执行，记录本帧状态供下一帧使用
        
        // 查询唯一的 InputState 组件
        let mut query = ctx.world.query::<&mut InputState>();
        
        if let Some((_, state)) = query.iter().next() {
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
