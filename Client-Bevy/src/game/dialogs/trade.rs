// ============================================================================
// 交易对话框（M9 第 2 批收尾）
// 布局参考：macroquad trade_dialog.rs / C# TradeDialogs.cs
//   - 背景 Title[22]，标题 Title[18]，位置 (250,100)
//   - 左（自己）/右（对方）各 5x4 物品槽；金币文本；锁定/确认
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};

#[derive(Resource)]
pub struct TradeState {
    pub visible: bool,
    pub partner_name: String,
    pub my_items: Vec<Option<u32>>,
    pub their_items: Vec<Option<u32>>,
    pub my_gold: u64,
    pub their_gold: u64,
    pub my_locked: bool,
    pub their_locked: bool,
}

impl Default for TradeState {
    fn default() -> Self {
        Self {
            visible: false,
            partner_name: String::new(),
            my_items: vec![None; 20],
            their_items: vec![None; 20],
            my_gold: 0,
            their_gold: 0,
            my_locked: false,
            their_locked: false,
        }
    }
}

#[derive(Component)]
pub struct TradeWidget;

#[derive(Component)]
pub struct TradeClose;

#[derive(Component)]
pub struct TradeLock;

#[derive(Component)]
pub struct TradeGoldText;

pub struct TradePlugin;

impl Plugin for TradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TradeState>();
        app.add_systems(OnEnter(AppState::Game), spawn_trade);
        app.add_systems(OnExit(AppState::Game), cleanup_trade);
        app.add_systems(
            Update,
            (trade_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_trade(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_trade(
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

    // 背景 Title[22]（约 540x350）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 22) {
        let e = spawn_ui_sprite(&mut commands, h, 250.0, 100.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Trade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }
    // 标题 Title[18]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 18) {
        let e = spawn_ui_sprite(&mut commands, h, 268.0, 109.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Trade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        250.0 + 520.0, 103.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            TradeClose,
            DialogRoot(DialogKind::Trade),
            TradeWidget,
        ));
    }

    // 双方物品槽（左 5x4，右 5x4；36x32）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for side in 0..2usize {
        let base_x = if side == 0 { 260.0 } else { 250.0 + 280.0 };
        for i in 0..20usize {
            let x = base_x + (i % 5) as f32 * 37.0;
            let y = 140.0 + (i / 5) as f32 * 34.0;
            commands.spawn((
                UiEntity,
                DialogRoot(DialogKind::Trade),
                TradeWidget,
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                    custom_size: Some(Vec2::new(36.0, 32.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(x, -y, 6.3),
                Visibility::Hidden,
            ));
        }
    }

    // 金币文本
    let g = spawn_ui_text(&mut commands, &font, "0", 300.0, 320.0, 12.0, Color::srgb(1.0, 0.85, 0.3), 8.0);
    commands.entity(g).insert((
        TradeGoldText,
        DialogRoot(DialogKind::Trade),
        TradeWidget,
    ));

    // 锁定按钮（占位：Title[200] 系列）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 200, 201, 202,
        250.0 + 230.0, 100.0 + 320.0, 7.0, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            TradeLock,
            DialogRoot(DialogKind::Trade),
            TradeWidget,
        ));
    }
}

fn trade_ui_system(
    mut mgr: ResMut<DialogManager>,
    trade: Res<TradeState>,
    close: Query<&UiButton, (With<TradeClose>, Without<TradeLock>)>,
    lock: Query<&UiButton, (With<TradeLock>, Without<TradeClose>)>,
    mut widgets: Query<&mut Visibility, With<TradeWidget>>,
    mut gold_texts: Query<&mut Text2d, With<TradeGoldText>>,
) {
    let open = trade.visible || mgr.is_open(DialogKind::Trade);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Trade);
        }
    }
    for _btn in &lock {
        // 锁定交易（网络 TradeLock 后续接入）
        tracing::info!("🔒 交易锁定（待接入网络）");
    }
    if let Ok(mut t) = gold_texts.single_mut() {
        t.0 = format!("金币: {} | 对方: {}", trade.my_gold, trade.their_gold);
    }
}
