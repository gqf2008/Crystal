// ============================================================================
// 游戏场景模块（M8：HUD + 聊天 + 玩家控制 + 移动）
// UI 交互参考：Client/MirScenes/GameScene.cs + Dialogs/MainDialogs.cs
// 绘制/网络参考：Client-Macroquad/src（main_dialog / player_control / network）
// ============================================================================

pub mod chat;
pub mod combat;
pub mod day_night;
pub mod effects;
pub mod dialogs;
pub mod hud;
pub mod movement;
pub mod pathfinding;
pub mod player_control;
pub mod skills;
pub mod sound;
pub mod weather;

use bevy::prelude::*;

use crate::scenes::AppState;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<hud::HudState>();
        app.init_resource::<chat::ChatState>();
        app.init_resource::<player_control::ControlState>();
        // 网络系统直接引用的对话框状态（与插件解耦，避免资源未注册）
        app.init_resource::<dialogs::npc::NpcDialogState>();
        app.init_resource::<dialogs::npc_goods::NpcGoodsState>();
        // 游戏场景 UI 相机（HUD/聊天共用，先于各 UI 插件创建）
        app.add_systems(OnEnter(AppState::Game), open_minimap_default);
        app.add_systems(Update, crate::ui::sprite_ui::ui_follow_camera);

        app.add_plugins((
            hud::HudPlugin,
            chat::ChatPlugin,
            dialogs::DialogsPlugin,
            movement::MovementPlugin,
            player_control::PlayerControlPlugin,
            skills::SkillsPlugin,
            combat::CombatPlugin,
            weather::WeatherPlugin,
            sound::SoundPlugin,
            day_night::DayNightPlugin,
            effects::EffectsPlugin,
        ));
    }
}

/// 小地图默认显示（原版为常驻控件）
fn open_minimap_default(mut mgr: ResMut<dialogs::DialogManager>) {
    if !mgr.is_open(dialogs::DialogKind::Minimap) {
        mgr.open.push(dialogs::DialogKind::Minimap);
    }
}
