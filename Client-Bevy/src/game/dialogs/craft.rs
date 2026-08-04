// ============================================================================
// 合成对话框（M41）
// 参考：C# NPC 合成页面 + ServerRust get_craft_recipes / CraftItemRequest
// 网络（ServerRust gate 实际 wire）：
//   C: CraftItem[recipe_id u32][materials_count u32]
//   S: CraftItem[recipe_id u32][count u16][success u8] + 系统聊天消息
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};

/// 配方（服务端 get_craft_recipes 硬编码 3 条）
#[derive(Debug, Clone, Copy)]
pub struct RecipeInfo {
    pub recipe_id: u32,
    pub product_index: i32,
    pub ingredients: &'static str,
}

pub const RECIPES: [RecipeInfo; 3] = [
    RecipeInfo {
        recipe_id: 1,
        product_index: 100,
        ingredients: "木材x3 + 铁矿石x2",
    },
    RecipeInfo {
        recipe_id: 2,
        product_index: 101,
        ingredients: "草药x2 + 清水x1",
    },
    RecipeInfo {
        recipe_id: 3,
        product_index: 102,
        ingredients: "铁矿石x5",
    },
];

/// 合成状态（CraftItem 响应写入）
#[derive(Resource, Default)]
pub struct CraftState {
    pub selected: Option<usize>,
    pub message: String,
    pub last_result: Option<(u32, u16, bool)>,
}

#[derive(Component)]
pub struct CraftWidget;

#[derive(Component)]
pub struct CraftClose;

#[derive(Component)]
pub struct CraftBtn;

#[derive(Component)]
pub struct CraftLine(usize);

pub struct CraftPlugin;

impl Plugin for CraftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftState>();
        app.add_systems(OnEnter(AppState::Game), spawn_craft);
        app.add_systems(OnExit(AppState::Game), cleanup_craft);
        app.add_systems(
            Update,
            (craft_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_craft(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_craft(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Craft),
            CraftWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            CraftClose,
            DialogRoot(DialogKind::Craft),
            CraftWidget,
        ));
    }
    // 配方 3 行 + 状态 3 行
    for i in 0..6usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            CraftLine(i),
            DialogRoot(DialogKind::Craft),
            CraftWidget,
        ));
    }
    // 合成按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        360.0, 260.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            CraftBtn,
            DialogRoot(DialogKind::Craft),
            CraftWidget,
        ));
    }
}

/// 显隐 + 渲染 + 选择 + 合成
#[allow(clippy::too_many_arguments)]
fn craft_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<CraftState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<CraftClose>>,
    craft_btn: Query<&UiButton, With<CraftBtn>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<CraftWidget>>,
    mut lines: Query<(&mut Text2d, &CraftLine)>,
) {
    let open = mgr.is_open(DialogKind::Craft);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Craft);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 3 => {
                let r = &RECIPES[i];
                format!(
                    "{}: 成品#{}（{}）成功率80%",
                    r.recipe_id,
                    r.product_index,
                    r.ingredients
                )
            }
            3 => format!(
                "选中配方: {}",
                state
                    .selected
                    .map(|i| format!("配方 {}", RECIPES[i].recipe_id))
                    .unwrap_or_else(|| "无".to_string())
            ),
            4 => state.message.clone(),
            5 => "点击配方行选中 → 点合成".to_string(),
            _ => String::new(),
        };
    }
    // 配方行点击选中
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 0..3usize {
                    let y = 120.0 + i as f32 * 22.0;
                    if cursor.x >= 298.0 && cursor.x <= 600.0 && cursor.y >= y && cursor.y <= y + 20.0 {
                        state.selected = Some(i);
                        tracing::info!("🔧 选中配方 {}", RECIPES[i].recipe_id);
                        break;
                    }
                }
            }
        }
    }
    // 合成
    for btn in &craft_btn {
        if btn.clicked {
            if let Some(i) = state.selected {
                let r = &RECIPES[i];
                net.send_packet(&crate::network::CraftItemWire {
                    recipe_id: r.recipe_id,
                    materials: 0,
                });
                state.message = format!("合成配方 {} 中…", r.recipe_id);
                tracing::info!("🔧 合成配方 {}（成品#{}）", r.recipe_id, r.product_index);
            } else {
                state.message = "请先点击选中一个配方".to_string();
            }
        }
    }
}
