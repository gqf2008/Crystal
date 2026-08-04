// ============================================================================
// 掷骰子对话框（M57）
// 参考：C# RollDialog（Client/MirScenes/Dialogs/RollDialog.cs）
//   - 服务端 Roll 包（270）触发：Type(0=骰子/1=尤茨) Page Result AutoRoll
//   - 骰子：Prguse[281+result] @ 屏幕中央；尤茨：Items[2587+result]
//   - autoRoll=true 到达自动掷；否则点击掷
//   - 掷完回调：CallNPC "[page]"（C# ReturnResult）
// 网络：服务端 NPC 脚本 ROLLDIE/ROLLYUT 动作发送 Roll 包
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_image, UiFont, UiImageCache,
};

/// 掷骰状态（网络 Roll 包填充）
#[derive(Resource, Default)]
pub struct RollState {
    pub visible: bool,
    /// 0=骰子 1=尤茨
    pub r#type: i32,
    /// 掷完回调的 NPC 页
    pub page: String,
    /// 结果 1-6
    pub result: i32,
    pub auto_roll: bool,
    /// 回调 NPC object_id（来自当前 NPC 对话框）
    pub npc_id: u32,
    /// 显示起始时间（用于 autoRoll 计时）
    pub started_at: f32,
    pub finished: bool,
}

#[derive(Component)]
pub struct RollWidget;

#[derive(Component)]
pub struct RollResultImage;

#[derive(Component)]
pub struct RollPrompt;

pub struct RollPlugin;

impl Plugin for RollPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RollState>();
        app.add_systems(OnEnter(AppState::Game), spawn_roll);
        app.add_systems(OnExit(AppState::Game), cleanup_roll);
        app.add_systems(
            Update,
            roll_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_roll(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_roll(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 结果图（骰子默认图）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let img = spawn_ui_sprite(&mut commands, white.clone(), 512.0 - 38.0, 384.0 - 40.0, 9.0, 1.0);
    commands.entity(img).insert((
        Sprite {
            image: white,
            custom_size: Some(Vec2::new(65.0, 65.0)),
            ..default()
        },
        RollResultImage,
        DialogRoot(DialogKind::Roll),
        RollWidget,
        Visibility::Hidden,
    ));

    // 提示文字
    let txt = spawn_ui_text(
        &mut commands, &font, "点击继续",
        512.0 - 40.0, 384.0 + 45.0, 12.0, Color::WHITE, 9.1,
    );
    commands.entity(txt).insert((
        RollPrompt,
        DialogRoot(DialogKind::Roll),
        RollWidget,
        Visibility::Hidden,
    ));
}

fn roll_ui_system(
    time: Res<Time>,
    mut state: ResMut<RollState>,
    net: ResMut<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut widgets: Query<&mut Visibility, With<RollWidget>>,
    mut img: Query<(&mut Sprite, &RollResultImage), Without<RollPrompt>>,
    mut prompt: Query<&mut Text2d, (With<RollPrompt>, Without<RollResultImage>)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut logged: Local<bool>,
) {
    if !state.visible {
        for mut vis in &mut widgets {
            *vis = Visibility::Hidden;
        }
        *logged = false;
        return;
    }
    for mut vis in &mut widgets {
        *vis = Visibility::Visible;
    }

    // 首次显示：记录起始时间 + 设置结果图
    if state.started_at == 0.0 {
        state.started_at = time.elapsed_secs();
    }
    if !*logged {
        let (lib, idx, x, y, w, ih) = if state.r#type == 1 {
            (LibraryName::Items, 2587 + state.result.clamp(1, 6), 512.0 - 90.0, 384.0 - 65.0, 180.0, 130.0)
        } else {
            (LibraryName::Prguse, 281 + state.result.clamp(1, 6), 512.0 - 38.0, 384.0 - 40.0, 65.0, 65.0)
        };
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, lib, idx as usize) {
            for (mut sprite, _) in &mut img {
                sprite.image = h.clone();
                sprite.custom_size = Some(Vec2::new(w, ih));
                sprite.rect = None;
            }
            let _ = (x, y);
        }
        for mut text in &mut prompt {
            text.0 = if state.auto_roll { "掷骰中..." } else { "点击掷骰" }.to_string();
        }
        tracing::info!(
            "🎲 掷骰子: type={} result={} page={} auto={}",
            state.r#type,
            state.result,
            state.page,
            state.auto_roll
        );
        *logged = true;
    }

    // 完成条件：autoRoll 计时 2 秒 / 手动点击
    let finish = if state.auto_roll {
        time.elapsed_secs() - state.started_at >= 2.0
    } else {
        mouse.just_pressed(MouseButton::Left)
    };
    if finish && !state.finished {
        state.finished = true;
        net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
            object_id: state.npc_id,
            key: format!("[{}]", state.page),
        });
        tracing::info!("🎲 掷骰完成，回调 NPC {} 页 [{}]", state.npc_id, state.page);
        state.visible = false;
        state.started_at = 0.0;
    }
}
