// ============================================================================
// NPC 对话框（M9 第 2 批）
// 布局参考：macroquad npc_dialog.rs / C# NPCDialogs.cs
//   - 背景 Prguse[384/385]，位置 (0,0)
//   - 文本区 (8,34)，行距 18；[@XXX] 行是选项，点击发送 CallNPC
//   - 关闭按钮 Prguse2[360-362] 在 (413,3)
// 网络：NPCResponse（行列表）→ 显示；CallNPC 推进
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::DialogRoot;
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// NPC 对话框状态（网络写入）
#[derive(Resource, Default)]
pub struct NpcDialogState {
    pub visible: bool,
    pub npc_object_id: u32,
    pub lines: Vec<String>,
}

#[derive(Component)]
pub struct NpcDialogWidget;

#[derive(Component)]
pub struct NpcClose;

#[derive(Component)]
pub struct NpcLine(usize);

pub struct NpcDialogPlugin;

impl Plugin for NpcDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcDialogState>();
        app.add_systems(OnEnter(AppState::Game), spawn_npc_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_npc_dialog);
        app.add_systems(
            Update,
            (npc_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_npc_dialog(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_npc_dialog(
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

    // 背景 Prguse[384]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 384) {
        let e = spawn_ui_sprite(&mut commands, h, 0.0, 0.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
            Visibility::Hidden,
        ));
    }

    // 关闭按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        413.0, 3.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            NpcClose,
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
        ));
    }

    // 8 行文本
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            8.0, 34.0 + i as f32 * 18.0,
            13.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            NpcLine(i),
            DialogRoot(crate::game::dialogs::DialogKind::Npc),
            NpcDialogWidget,
        ));
    }
}

/// 显示/关闭 + 文本渲染 + 选项点击
fn npc_ui_system(
    mut npc: ResMut<NpcDialogState>,
    net: Res<NetworkContext>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<NpcClose>>,
    mut widgets: Query<&mut Visibility, With<NpcDialogWidget>>,
    mut lines: Query<(&mut Text2d, &NpcLine)>,
) {
    for mut vis in widgets.iter_mut() {
        *vis = if npc.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !npc.visible {
        return;
    }

    // 关闭
    for btn in &close {
        if btn.clicked {
            npc.visible = false;
        }
    }

    // 渲染行
    for (mut text, line) in &mut lines {
        if let Some(l) = npc.lines.get(line.0) {
            text.0 = l.clone();
        } else {
            text.0 = String::new();
        }
    }

    // 点击选项行（以 [@ 开头的行）
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    for (i, l) in npc.lines.iter().enumerate() {
        if i < 8 && is_clickable_npc_line(l) {
            let y = 34.0 + i as f32 * 18.0;
            if cursor.x >= 8.0 && cursor.x <= 400.0 && cursor.y >= y && cursor.y <= y + 16.0 {
                let key = extract_npc_key(l);
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: npc.npc_object_id,
                    key: key.clone(),
                });
                tracing::info!("🧙 NPC 选项: {} → {}", l.trim(), key);
                break;
            }
        }
    }
}

/// 可点击的 NPC 菜单行：[@XXX] 或 <文字/@XXX>（原版 C# 链接格式）
fn is_clickable_npc_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("[@") || t.contains("/@")
}

/// 提取菜单键（统一为 "[@XXX]" 格式，服务端按该格式匹配）
pub fn extract_npc_key(line: &str) -> String {
    let t = line.trim();
    if t.starts_with("[@") {
        let end = t.find(']').unwrap_or(t.len());
        t[..end].to_string()
    } else if let Some(slash) = t.find("/@") {
        let rest = &t[slash + 1..];
        let end = rest.find('>').unwrap_or(rest.len());
        format!("[@{}]", &rest[..end])
    } else {
        t.to_string()
    }
}
