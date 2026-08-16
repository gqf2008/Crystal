// ============================================================================
// 查看玩家对话框（M46 → #2607 批T 对齐 C# InspectDialog 14 格装备网格）
// C# MainDialogs.cs:2113-2526：背景 Prguse[430]@(536,0)、角色页 Prguse[340]@(8,70)、
//   14 格装备网格（EQUIP_SLOTS 同角色对话框坐标）、名字/行会标签、Close@(509,3)。
// 网络（#2607 协议加 slot+image）：
//   C: Inspect[object_id u32]
//   S: PlayerInspect[oid u32][name dotnet][guild dotnet][level u16][class u8]
//      [gender u8][count u8][per: slot u8][uid u64][index i32][image i32][dura i32][max_dura i32]
// 有意偏差（附 #2607 记录）：纸娃娃（StateItems 画甲/武器/头盔+发型）与
//   Group/Friend/Mail/Trade/Observe 五动作按钮（按钮精灵索引待 probe）后续批
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::character::EQUIP_SLOTS;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};

/// 背景 Prguse[430] 左贴右缘（C# :2153-2155 Location(536,0)）
pub const BG_X: f32 = 536.0;
pub const BG_Y: f32 = 0.0;
/// 角色页 Prguse[340] @(8,70)（C# :2159-2165）
pub const PAGE_REL: (f32, f32) = (8.0, 70.0);

/// 装备条目（#2607：slot=服务端旧序槽位，image=Items 库图标索引）
#[derive(Debug, Clone, Default)]
pub struct InspectItem {
    pub slot: u8,
    pub unique_id: u64,
    pub item_index: i32,
    pub image: i32,
    pub current_dura: i32,
    pub max_dura: i32,
}

/// 查看状态（PlayerInspect 写入）
#[derive(Resource, Default)]
pub struct InspectState {
    pub name: String,
    pub guild: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
    pub items: Vec<InspectItem>,
}

#[derive(Component)]
pub struct InspectWidget;

#[derive(Component)]
pub struct InspectClose;

/// 名字/行会标签（C# NameLabel/GuildLabel）
#[derive(Component)]
pub struct InspectNameText;

#[derive(Component)]
pub struct InspectGuildText;

/// 装备图标格（.0 = C# 格位 0..13，非服务端槽位）
#[derive(Component)]
pub struct InspectCellIcon(usize);

pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InspectState>();
        app.add_systems(OnEnter(AppState::Game), spawn_inspect);
        app.add_systems(OnExit(AppState::Game), cleanup_inspect);
        app.add_systems(
            Update,
            inspect_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (inspect_ui_system, inspect_icon_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_inspect(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_inspect(
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

    // 背景 + 角色页（C# :2153-2165）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 430) {
        let e = spawn_ui_sprite(&mut commands, h, BG_X, BG_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 340) {
        let e = spawn_ui_sprite(
            &mut commands,
            h,
            BG_X + PAGE_REL.0,
            BG_Y + PAGE_REL.1,
            6.1,
            1.0,
        );
        commands
            .entity(e)
            .insert((DialogRoot(DialogKind::Inspect), InspectWidget, Visibility::Hidden));
    }
    // 关闭 @(509,3)（C# :2472-2482 同格式）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        BG_X + 509.0,
        BG_Y + 3.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands.entity(e).insert((
            InspectClose,
            DialogRoot(DialogKind::Inspect),
            InspectWidget,
        ));
    }
    // 名字（加粗 10F 居中 150x20 @(30,45)，C# :2210-2220）+ 行会（同 @(30,60)）
    // 名字（加粗 10F 居中 150x20 @(30,45)，C# :2210-2220）+ 行会（@(30,60)）
    for (is_name, ry) in [(true, 45.0), (false, 60.0)] {
        let t = spawn_ui_text(
            &mut commands,
            &font,
            "",
            BG_X + 105.0,
            BG_Y + ry,
            10.0,
            Color::WHITE,
            8.0,
        );
        let mut ec = commands.entity(t);
        if is_name {
            ec.insert((
                InspectNameText,
                bevy::sprite::Anchor::TOP_CENTER,
                DialogRoot(DialogKind::Inspect),
                InspectWidget,
            ));
        } else {
            ec.insert((
                InspectGuildText,
                bevy::sprite::Anchor::TOP_CENTER,
                DialogRoot(DialogKind::Inspect),
                InspectWidget,
            ));
        }
    }
    // 14 格装备图标（格位坐标 = character::EQUIP_SLOTS，页内 @(8,70) 偏移；
    // C# :2362-2469 MirItemCell GridType=Inspect）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for (pos, (cx, cy)) in EQUIP_SLOTS.iter().enumerate() {
        let e = commands
            .spawn((
                UiEntity,
                InspectCellIcon(pos),
                DialogRoot(DialogKind::Inspect),
                InspectWidget,
                Sprite {
                    image: white.clone(),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(BG_X + PAGE_REL.0 + cx, -(BG_Y + PAGE_REL.1 + cy), 7.5),
                Visibility::Hidden,
            ))
            .id();
        let _ = e;
    }
}

/// 显隐 + 标签 + 关闭
fn inspect_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<InspectState>,
    close: Query<&UiButton, With<InspectClose>>,
    mut widgets: Query<&mut Visibility, (With<InspectWidget>, Without<InspectCellIcon>)>,
    mut icons: Query<&mut Visibility, (With<InspectCellIcon>, Without<InspectWidget>)>,
    mut names: Query<
        &mut Text2d,
        (With<InspectNameText>, Without<InspectGuildText>),
    >,
    mut guilds: Query<
        &mut Text2d,
        (With<InspectGuildText>, Without<InspectNameText>),
    >,
) {
    let open = mgr.is_open(DialogKind::Inspect);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in icons.iter_mut() {
        *vis = Visibility::Hidden;
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Inspect);
        }
    }
    let class = match state.class {
        0 => "战士",
        1 => "法师",
        2 => "道士",
        3 => "刺客",
        4 => "弓箭手",
        _ => "未知",
    };
    for mut t in &mut names {
        t.0 = format!("{}（{}）Lv.{}", state.name, class, state.level);
    }
    for mut t in &mut guilds {
        t.0 = if state.guild.is_empty() {
            String::new()
        } else {
            format!("<{}>", state.guild)
        };
    }
}

/// 图标渲染：服务端槽位（旧序）→ C# 格位（SERVER_SLOT_TO_POS）→ Items[image]
fn inspect_icon_system(
    state: Res<InspectState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut icons: Query<(&mut Sprite, &mut Visibility, &InspectCellIcon)>,
) {
    let mut by_pos = [None::<&InspectItem>; 14];
    for it in &state.items {
        let pos = crate::game::dialogs::character::SERVER_SLOT_TO_POS
            .get(it.slot as usize)
            .copied();
        if let Some(pos) = pos {
            by_pos[pos] = Some(it);
        }
    }
    for (mut sprite, mut vis, cell) in &mut icons {
        match by_pos.get(cell.0).copied().flatten() {
            Some(it) if it.image > 0 => {
                if let Some(h) = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::Items,
                    it.image as usize,
                ) {
                    sprite.image = h;
                    sprite.custom_size = None;
                    *vis = Visibility::Visible;
                    continue;
                }
                *vis = Visibility::Hidden;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}

/// 消费服务端查看事件（网络层只广播 ServerEvent）
fn inspect_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut mgr: ResMut<DialogManager>,
    mut inspect: ResMut<InspectState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::InspectPlayer { name, guild, level, class, gender, items } = ev {
            inspect.name = name.clone();
            inspect.guild = guild.clone();
            inspect.level = *level;
            inspect.class = *class;
            inspect.gender = *gender;
            inspect.items = items.clone();
            mgr.open(DialogKind::Inspect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 布局常量对齐 C#（MainDialogs.cs:2153-2165）
    #[test]
    fn layout_matches_csharp() {
        assert_eq!((BG_X, BG_Y), (536.0, 0.0));
        assert_eq!(PAGE_REL, (8.0, 70.0));
        // 首格 = Weapon(123,7)（C# :2362-2369，格坐标与角色对话框 EQUIP_SLOTS 同源）
        assert_eq!(EQUIP_SLOTS[0], (123.0, 7.0));
        assert_eq!(EQUIP_SLOTS.len(), 14);
    }
}
