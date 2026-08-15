// ============================================================================
// input_gate - 文本输入聚焦时游戏键位让路（C# WinForms 焦点路由等价）
// ============================================================================
// C# 原版：聊天框/输入框是 MirTextBox 包真实 WinForms TextBox（MainDialogs.cs:1096
// 激活 '@'/'!'/' '/Enter/'/'）。TextBox 聚焦时按键进文本框；CMain 表单无 KeyPreview
// → CMain_KeyDown → GameScene_KeyDown（对话框热键所在，GameScene.cs:504）整条链不触发。
// 唯一转发例外：ChatTextBox_KeyDown 把 F1-F12/Tab 显式转发回游戏
// （MainDialogs.cs:1160-1185；MirTextBox.cs:375-384 另转发 PrintScreen）。
// Escape 在文本框聚焦时 = 取消聚焦且 e.Handled（MirTextBox.cs:386-395），
// 不触发 Closeall。
//
// Bevy 无焦点路由：各系统直接读全局 ButtonInput<KeyCode> → 打字会误触对话框热键/
// 拾取/攻击模式/相机平移（用户实测：游戏内按 n/i 弹钓鱼+背包，见 #2595）。
// 本模块在 PreUpdate 汇总所有文本输入态到 TextInputGate，键位消费者按
// C# 转发表豁免 F1-F12/Tab 后让路。
// ============================================================================

use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

use crate::game::chat::ChatState;
use crate::game::dialogs::amount_box::AmountBoxState;
use crate::game::dialogs::creature::CreatureState;
use crate::game::dialogs::text_input::TextInputState;
use crate::scenes::AppState;

/// 任一文本输入聚焦中（true = 游戏键位须让路，F1-F12/Tab 除外）
#[derive(Resource, Default, Clone, Copy)]
pub struct TextInputGate(pub bool);

/// C# 聚焦文本框时仍转发回游戏键位的键（MainDialogs.cs:1166-1182：
/// F1..F12 + Tab；PrintScreen 走 KeyUp，Bevy 无此绑定不涉及）
pub fn forwarded_while_typing(kc: KeyCode) -> bool {
    matches!(
        kc,
        KeyCode::F1
            | KeyCode::F2
            | KeyCode::F3
            | KeyCode::F4
            | KeyCode::F5
            | KeyCode::F6
            | KeyCode::F7
            | KeyCode::F8
            | KeyCode::F9
            | KeyCode::F10
            | KeyCode::F11
            | KeyCode::F12
            | KeyCode::Tab
    )
}

/// PreUpdate 汇总所有文本输入态。覆盖：
/// - 聊天输入行（ChatState.input_active，chat.rs 回填）
/// - 通用输入框（TextInputState.active：好友/组队/大地图搜索/邮件/NPC 输入层等）
/// - 宠物改名/释放输入框（CreatureState.rename_open/release_open，C# MirInputBox）
/// - 数量输入框（AmountBoxState.visible，C# MirAmountBox 模态数字框）
/// ModalState（删除角色确认）是选角屏流程，不出现在 Game 态，不在此汇总。
/// 各输入态由 Update 各系统写入，PreUpdate 读到的是上一帧值——输入框开/关
/// 当帧的 1 帧滞后对离散按键无实际影响（Enter 开框与字母不同帧到达）。
fn update_text_input_gate(
    chat: Res<ChatState>,
    text: Res<TextInputState>,
    creature: Res<CreatureState>,
    amount: Res<AmountBoxState>,
    mut gate: ResMut<TextInputGate>,
) {
    gate.0 = chat.input_active
        || text.active.is_some()
        || creature.rename_open
        || creature.release_open
        || amount.visible;
}

pub struct TextInputGatePlugin;

impl Plugin for TextInputGatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputGate>();
        app.add_systems(
            PreUpdate,
            update_text_input_gate.run_if(in_state(AppState::Game)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ChatState>();
        app.init_resource::<TextInputState>();
        app.init_resource::<CreatureState>();
        app.init_resource::<AmountBoxState>();
        app.init_resource::<TextInputGate>();
        // 直插 Game 态（in_state 只读 State 资源，无需 StatesPlugin）
        app.insert_resource(State::new(AppState::Game));
        app.add_systems(PreUpdate, update_text_input_gate);
        app
    }

    /// 无任何输入聚焦 → 门关
    #[test]
    fn gate_off_by_default() {
        let mut app = gate_app();
        app.update();
        assert!(!app.world().resource::<TextInputGate>().0);
    }

    /// 聊天输入激活 → 门开
    #[test]
    fn chat_input_sets_gate() {
        let mut app = gate_app();
        app.world_mut().resource_mut::<ChatState>().input_active = true;
        app.update();
        assert!(app.world().resource::<TextInputGate>().0);
    }

    /// 通用输入框聚焦（好友/组队/大地图搜索/NPC 输入层同源）→ 门开
    #[test]
    fn generic_text_input_sets_gate() {
        let mut app = gate_app();
        app.world_mut().resource_mut::<TextInputState>().active = Some(0);
        app.update();
        assert!(app.world().resource::<TextInputGate>().0);
    }

    /// 宠物改名/释放输入框 → 门开
    #[test]
    fn creature_inputs_set_gate() {
        let mut app = gate_app();
        app.world_mut().resource_mut::<CreatureState>().rename_open = true;
        app.update();
        assert!(app.world().resource::<TextInputGate>().0);
        let mut app = gate_app();
        app.world_mut().resource_mut::<CreatureState>().release_open = true;
        app.update();
        assert!(app.world().resource::<TextInputGate>().0);
    }

    /// 数量输入框打开 → 门开
    #[test]
    fn amount_box_sets_gate() {
        let mut app = gate_app();
        app.world_mut().resource_mut::<AmountBoxState>().visible = true;
        app.update();
        assert!(app.world().resource::<TextInputGate>().0);
    }

    /// C# 转发表：F1-F12/Tab 放行，字母/空格/Escape 不放行
    #[test]
    fn forwarded_keys_match_csharp() {
        for kc in [
            KeyCode::F1,
            KeyCode::F5,
            KeyCode::F9,
            KeyCode::F12,
            KeyCode::Tab,
        ] {
            assert!(forwarded_while_typing(kc), "{kc:?} 应转发");
        }
        for kc in [
            KeyCode::KeyN,
            KeyCode::KeyI,
            KeyCode::Space,
            KeyCode::Escape,
            KeyCode::Enter,
        ] {
            assert!(!forwarded_while_typing(kc), "{kc:?} 不应转发");
        }
    }
}
