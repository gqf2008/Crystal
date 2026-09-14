// ============================================================================
// 租借浏览对话框（C# `ItemRentalDialog`，批10 #2720）
// 参考：Client/MirScenes/Dialogs/ItemRentalDialog.cs
//   - 面板 `Index = 1; Library = Prguse3`（原生 400x174），居中
//   - 标题 `Prguse3[0]` @(22,8)；页签 `Prguse3[2]` @(8,32) 72x23、`Prguse3[3]` @(81,32) 84x23
//   - 租借按钮 `Prguse3[4..6]` @(295,144) Size 85x29 → `C.ItemRentalRequest`
//   - 关闭 `Prguse2[360..362]` @(375,3) 24x21
//   - 3 行 @(0, 78+i*21) Size 383x21；行内三列：物品名(5,0)/承租人(137,0)/归还日期(264,0) 各 128x20
// 网络：打开时按 C# `RequestRentedItems`（60s 节流）发 `C.GetRentedItems`；
//       `S.GetRentedItems`（C# `ItemRentalInformation` 形状）→ 填 3 行
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_image, spawn_label, spawn_panel};

/// #2892 批B：C# `ItemRentalDialog` 面板与子控件精灵（`ItemRentalDialog.cs:16-105`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse3, 1);
pub const TITLE: (LibraryName, usize) = (LibraryName::Prguse3, 0);
pub const RENTED_TAB_SPRITE: (LibraryName, usize) = (LibraryName::Prguse3, 2);
pub const BORROWED_TAB_SPRITE: (LibraryName, usize) = (LibraryName::Prguse3, 3);
pub const RENT_BTN_SPRITES: [(LibraryName, usize); 3] = [
    (LibraryName::Prguse3, 4),
    (LibraryName::Prguse3, 5),
    (LibraryName::Prguse3, 6),
];
pub const CLOSE_SPRITES: [(LibraryName, usize); 3] = [
    (LibraryName::Prguse2, 360),
    (LibraryName::Prguse2, 361),
    (LibraryName::Prguse2, 362),
];
/// C# `ItemRentalDialog` 面板原生尺寸（Prguse3[1]）
pub const PANEL_W: f32 = 400.0;
pub const PANEL_H: f32 = 174.0;
/// C# 控件锚点
pub const TITLE_POS: (f32, f32) = (22.0, 8.0);
pub const RENTED_TAB: (f32, f32, f32, f32) = (8.0, 32.0, 72.0, 23.0);
pub const BORROWED_TAB: (f32, f32, f32, f32) = (81.0, 32.0, 84.0, 23.0);
pub const RENT_BTN_POS: (f32, f32) = (295.0, 144.0);
pub const RENT_BTN_SIZE: (f32, f32) = (85.0, 29.0);
pub const CLOSE_POS: (f32, f32) = (375.0, 3.0);
/// C# `ItemRow`：`Location = (0, 78 + i*21)`
pub const ROW_Y0: f32 = 78.0;
pub const ROW_DY: f32 = 21.0;
pub const RENTAL_ROWS: usize = 3;
/// 行内三列（C# `ItemRow` 内三个 MirLabel）
pub const ROW_COL_X: [f32; 3] = [5.0, 137.0, 264.0];
/// C# `RequestRentedItems()`：60 秒节流
const REQUEST_THROTTLE_SECS: f64 = 60.0;

/// 浏览窗状态（`S.GetRentedItems` 写入）
#[derive(Resource, Default)]
pub struct ItemRentalBrowseState {
    pub items: Vec<mir2_shared::packets::server::rental_system::RentalItemInfo>,
    /// 上次请求时间（C# `_lastRequestTime` 节流）
    pub last_request: Option<std::time::Instant>,
}

#[derive(Component)]
pub struct ItemRentalBrowseWidget;

#[derive(Component)]
pub struct ItemRentalBrowseClose;

/// 租借按钮（C# `rentItemButton` → `C.ItemRentalRequest`）
#[derive(Component)]
pub struct ItemRentalRentBtn;

/// 行内单元格（行, 列）
#[derive(Component)]
pub struct ItemRentalRowLabel(pub usize, pub usize);

pub struct ItemRentalBrowsePlugin;

impl Plugin for ItemRentalBrowsePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRentalBrowseState>();
        app.add_systems(OnEnter(AppState::Game), spawn_item_rental_browse);
        app.add_systems(OnExit(AppState::Game), cleanup_item_rental_browse);
        app.add_systems(
            Update,
            (rental_browse_server_events, rental_browse_ui_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_item_rental_browse(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// C# `ItemReturnDate.ToString(CultureInfo.InvariantCulture)` 等价（Unix 秒 → UTC 文本）
pub fn format_return_date(return_date: i64) -> String {
    chrono::DateTime::from_timestamp(return_date, 0)
        .map(|t| t.format("%Y/%m/%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn spawn_item_rental_browse(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let _font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 C# Prguse3[1]（400x174）居中
    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    let (px, py) = crate::game::dialogs::center_origin(PANEL_W, PANEL_H);
    let panel = spawn_panel(&mut commands, bg, px, py, PANEL_W, PANEL_H, 30);
    commands.entity(panel).insert((
        DialogRoot(DialogKind::ItemRentalBrowse),
        ItemRentalBrowseWidget,
    ));

    commands.entity(panel).with_children(|p| {
        // 标题 Prguse3[0]（52x18）+ 两个页签（C# 两页签 Enabled=false，仅作当前页标识）
        if let Some(title) = load_lib_image(&mut libs, &mut images, TITLE.0, TITLE.1) {
            spawn_image(p, title, TITLE_POS.0, TITLE_POS.1, 52.0, 18.0, 9);
        }
        if let Some(tab) = load_lib_image(
            &mut libs,
            &mut images,
            RENTED_TAB_SPRITE.0,
            RENTED_TAB_SPRITE.1,
        ) {
            spawn_image(
                p,
                tab,
                RENTED_TAB.0,
                RENTED_TAB.1,
                RENTED_TAB.2,
                RENTED_TAB.3,
                9,
            );
        }
        if let Some(tab) = load_lib_image(
            &mut libs,
            &mut images,
            BORROWED_TAB_SPRITE.0,
            BORROWED_TAB_SPRITE.1,
        ) {
            spawn_image(
                p,
                tab,
                BORROWED_TAB.0,
                BORROWED_TAB.1,
                BORROWED_TAB.2,
                BORROWED_TAB.3,
                9,
            );
        }
        // 租借按钮 Prguse3[4..6]（84x28；C# Size 85x29）@(295,144)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(
                &mut libs,
                &mut images,
                RENT_BTN_SPRITES[0].0,
                RENT_BTN_SPRITES[0].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                RENT_BTN_SPRITES[1].0,
                RENT_BTN_SPRITES[1].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                RENT_BTN_SPRITES[2].0,
                RENT_BTN_SPRITES[2].1,
            ),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                RENT_BTN_POS.0,
                RENT_BTN_POS.1,
                RENT_BTN_SIZE.0,
                RENT_BTN_SIZE.1,
                10,
            )
            .insert(ItemRentalRentBtn);
        }
        // 关闭 Prguse2[360..362]（24x21）@(375,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(
                &mut libs,
                &mut images,
                CLOSE_SPRITES[0].0,
                CLOSE_SPRITES[0].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                CLOSE_SPRITES[1].0,
                CLOSE_SPRITES[1].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                CLOSE_SPRITES[2].0,
                CLOSE_SPRITES[2].1,
            ),
        ) {
            spawn_icon_button(p, n, h, pr, CLOSE_POS.0, CLOSE_POS.1, 24.0, 21.0, 10)
                .insert(ItemRentalBrowseClose);
        }
        // 3 行 × 3 列标签（C# `ItemRow`）
        for row in 0..RENTAL_ROWS {
            let y = ROW_Y0 + row as f32 * ROW_DY;
            for col in 0..3usize {
                spawn_label(p, &cjk, "", ROW_COL_X[col], y, 12.0, Color::WHITE, 9)
                    .insert(ItemRentalRowLabel(row, col));
            }
        }
    });
}

/// `S.GetRentedItems` → 本地列表（C# `GameScene.GetRentedItems → ReceiveRentedItems`）
fn rental_browse_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut state: ResMut<ItemRentalBrowseState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::RentedItems { items } = ev {
            state.items = items.clone();
            tracing::info!("📦 租借浏览窗：{} 条", state.items.len());
        }
    }
}

/// 显隐 + 行渲染 + 打开时请求列表（60s 节流）+ 租借/关闭
#[allow(clippy::too_many_arguments)]
fn rental_browse_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<ItemRentalBrowseState>,
    net: Res<NetConnection>,
    close: Query<(Entity, &Interaction), With<ItemRentalBrowseClose>>,
    rent_btn: Query<(Entity, &Interaction), With<ItemRentalRentBtn>>,
    mut widgets: Query<&mut Visibility, With<ItemRentalBrowseWidget>>,
    mut labels: Query<(&mut Text, &ItemRentalRowLabel)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut was_open: Local<bool>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::ItemRentalBrowse);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // C# `Toggle()`：打开时 `RequestRentedItems()`（60s 节流）→ `C.GetRentedItems`
    if open && !*was_open {
        let now = std::time::Instant::now();
        let allowed = state
            .last_request
            .map(|last| now.duration_since(last).as_secs_f64() >= REQUEST_THROTTLE_SECS)
            .unwrap_or(true);
        if allowed {
            state.last_request = Some(now);
            net.send_packet(&mir2_shared::packets::client::item::GetRentedItems);
            tracing::info!("📦 请求已租出物品列表");
        }
    }
    *was_open = open;
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::ItemRentalBrowse);
        }
    }
    // C# `rentItemButton.Click` → `C.ItemRentalRequest`
    for (e, inter) in &rent_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalRequest);
            tracing::info!("📦 请求物品租借");
        }
    }
    // 行渲染：物品名 / 承租人 / 归还日期（C# `ItemRow.Update`）
    for (mut text, cell) in &mut labels {
        let new = state
            .items
            .get(cell.0)
            .map(|item| match cell.1 {
                0 => item.item_name.clone(),
                1 => item.renting_player_name.clone(),
                _ => format_return_date(item.return_date),
            })
            .unwrap_or_default();
        if text.0 != new {
            text.0 = new;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2720：面板/控件锚点对齐 C# `ItemRentalDialog`（ItemRentalDialog.cs:18-105）
    #[test]
    fn rental_browse_layout_matches_csharp_anchors() {
        assert_eq!((PANEL_W, PANEL_H), (400.0, 174.0));
        assert_eq!(TITLE_POS, (22.0, 8.0));
        assert_eq!(RENTED_TAB, (8.0, 32.0, 72.0, 23.0));
        assert_eq!(BORROWED_TAB, (81.0, 32.0, 84.0, 23.0));
        assert_eq!(RENT_BTN_POS, (295.0, 144.0));
        assert_eq!(CLOSE_POS, (375.0, 3.0));
        assert_eq!((ROW_Y0, ROW_DY), (78.0, 21.0));
        assert_eq!(ROW_COL_X, [5.0, 137.0, 264.0]);
        assert_eq!(RENTAL_ROWS, 3);
        // C# Center：面板居中
        assert_eq!(
            crate::game::dialogs::center_origin(PANEL_W, PANEL_H),
            (312.0, 297.0)
        );
    }

    /// #2720：归还日期格式化（Unix 秒 → UTC 文本；非法值给空串）
    #[test]
    fn rental_return_date_format() {
        assert_eq!(format_return_date(1_700_000_000), "2023/11/14 22:13:20");
        assert_eq!(format_return_date(i64::MIN), "");
    }
}
