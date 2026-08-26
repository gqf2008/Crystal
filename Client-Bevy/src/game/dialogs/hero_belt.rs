// ============================================================================
// 英雄药水腰带（#2602 批R，C# HeroBeltDialog）
// 参考：Client/MirScenes/Dialogs/HeroDialogs.cs:248-384
//   - 背景 Prguse[1921]（实测 100x38）@(MainDialog.X+475, SH-150)=(475,618)；
//     BeforeDraw 叠 Prguse[1934]（92x38，0.5 alpha）
//   - 2 格 32x32 @(12+i*35,3)，GridType=HeroInventory ItemSlot=x —— 即
//     英雄背包前 2 格（HeroBeltIdx=2，UserObject.cs:38；腰带占位是纯客户端
//     约定，服务端 hero_inventory 46 槽无保留——#2602 已核）
//   - 键标 "7"/"8" @(8+i*35,2)；Belt7/Belt8 快捷键使用（C# GameScene.cs:759-766，
//     UseItem 按 unique_id 全背包查，腰带格与网格格同路径）
//   - 旋转钮 1926-1928 @(82,3)；关闭钮 1923-1925 @(82,19)
//   - Flip 纵向：背景 1943 @(0,446)、格 @(3,i*35+12)、键标 @(-1,11+i*35)、
//     旋转 1938-1940 @(19,82)、关闭 1935-1937 @(3,82)、叠层 1946
//   - 格点击/双击/转移由 hero_inventory::hero_inv_click_system 统一处理
//     （hero_slot_at 扩展覆盖腰带格，槽位即 0/1）
// 有意偏差（附 #2602 记录）：
//   - 腰带用尽后从英雄背包自动补货（C# MirItemCell.cs:574-581 走
//     C.MoveItem(Grid=HeroInventory)）暂缓——服务端 MoveItem 英雄分支待核
//   - C# HeroBeltDialog Movable=true 且随 MainDialog 移动（MainDialogs.cs
//     :1284-1285）；本移植位置固定、不参与 dialog_drag_system（未挂
//     DialogRoot；主对话框底栏本也固定）
//   - 键标白字与 C# 一致（MirControl.cs:707 构造器默认 _foreColour=White）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::hero::HeroState;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
    ImageButton,
};

/// 格数（C# HeroBeltDialog.Grid[2]）
pub const BELT_SLOTS: usize = 2;
pub const CELL_SIZE: f32 = 32.0;
const CELL_SPACING: f32 = 35.0;
/// 横向（C# :261：(MainDialog.X+475, ScreenHeight-150)）
pub const BELT_X: f32 = 475.0;
pub const BELT_Y: f32 = 618.0;
/// 纵向（C# :333：Flip Location (0,446)）
pub const BELT_VERT_X: f32 = 0.0;
pub const BELT_VERT_Y: f32 = 446.0;

/// 横/纵布局（C# Flip :327-372）
#[derive(Resource, Default)]
pub struct HeroBeltVertical(pub bool);

/// 显隐（C# :260 Visible=true 默认；与主腰带不同键位开关——C# 无独立切换键，
/// 仅关闭钮隐藏，Z 是主腰带 Belt 键）
#[derive(Resource)]
pub struct HeroBeltVisible(pub bool);

impl Default for HeroBeltVisible {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component)]
pub struct HeroBeltWidget;

#[derive(Component)]
pub struct HeroBeltBg;

#[derive(Component)]
pub struct HeroBeltBgOverlay;

#[derive(Component)]
pub struct HeroBeltIcon(usize);

#[derive(Component)]
pub struct HeroBeltCount(usize);

/// 键标 7/8（C# Key[i] Text=(i+7)）
#[derive(Component)]
pub struct HeroBeltNumber(usize);

#[derive(Component)]
pub struct HeroBeltRotate;

#[derive(Component)]
pub struct HeroBeltClose;

/// 格实体（横纵布局时重定位；子图标/数量随父级）
#[derive(Component)]
pub struct HeroBeltSlotCell(pub usize);

/// 横向：格 @(12+i*35,3)、键标 @(8+i*35,2)、旋转 @(82,3)、关闭 @(82,19)
pub fn h_slot(i: usize) -> (f32, f32) {
    (BELT_X + 12.0 + i as f32 * CELL_SPACING, BELT_Y + 3.0)
}
fn h_num(i: usize) -> (f32, f32) {
    (BELT_X + 8.0 + i as f32 * CELL_SPACING, BELT_Y + 2.0)
}
/// 纵向：格 @(3,i*35+12)、键标 @(-1,11+i*35)、旋转 @(19,82)、关闭 @(3,82)
pub fn v_slot(i: usize) -> (f32, f32) {
    (
        BELT_VERT_X + 3.0,
        BELT_VERT_Y + 12.0 + i as f32 * CELL_SPACING,
    )
}
fn v_num(i: usize) -> (f32, f32) {
    (
        BELT_VERT_X - 1.0,
        BELT_VERT_Y + 11.0 + i as f32 * CELL_SPACING,
    )
}

pub struct HeroBeltPlugin;

impl Plugin for HeroBeltPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroBeltVertical>();
        app.init_resource::<HeroBeltVisible>();
        app.init_resource::<HeroBeltUseArmed>();
        app.add_systems(OnEnter(AppState::Game), spawn_hero_belt);
        app.add_systems(OnExit(AppState::Game), cleanup_hero_belt);
        app.add_systems(
            Update,
            (hero_belt_ui_system, hero_belt_icon_system, hero_belt_refill_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_hero_belt(mut commands: Commands, roots: Query<Entity, With<HeroBeltWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_hero_belt(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[1921]（横 100x38 @ (475,618)）/ 纵向 Prguse[1943]（40x101 @ (0,446)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1921) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, BELT_X, BELT_Y, 100.0, 38.0, 15);
    commands
        .entity(panel)
        .insert((HeroBeltWidget, HeroBeltBg, Visibility::Visible));

    commands.entity(panel).with_children(|p| {
        // 半透明叠层 1934（92x38，C# BeforeDraw 0.5 alpha）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1934) {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(92.0),
                    height: Val::Px(38.0),
                    ..default()
                },
                ImageNode::new(h).with_color(Color::srgba(1.0, 1.0, 1.0, 0.5)),
                HeroBeltWidget,
                HeroBeltBgOverlay,
                ZIndex(1),
            ));
        }

        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        // 2 格（C# :302-315：ItemSlot=x 即英雄背包 0/1）+ 键标 7/8
        for i in 0..BELT_SLOTS {
            spawn_container(p, 12.0 + i as f32 * CELL_SPACING, 3.0, CELL_SIZE, CELL_SIZE, 2)
                .insert((
                    HeroBeltWidget,
                    HeroBeltSlotCell(i),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                ))
                .with_children(|c| {
                    spawn_image(c, white.clone(), 2.0, 2.0, CELL_SIZE - 4.0, CELL_SIZE - 4.0, 3)
                        .insert((HeroBeltWidget, HeroBeltIcon(i), Visibility::Hidden));
                    spawn_label(c, &font, "", 16.0, 20.0, 10.0, Color::WHITE, 3)
                        .insert((HeroBeltWidget, HeroBeltCount(i), Visibility::Hidden));
                });
        }
        for i in 0..BELT_SLOTS {
            spawn_label(p, &font, &(i + 7).to_string(), 8.0 + i as f32 * CELL_SPACING, 2.0, 10.0, Color::WHITE, 3)
                .insert((HeroBeltWidget, HeroBeltNumber(i)));
        }
        // 旋转钮 1926-1928 @(82,3)（横向）/ 1938-1940 @(19,82)（纵向，Flip 换帧）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1926),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1927),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1928),
        ) {
            spawn_icon_button(p, n, h, pr, 82.0, 3.0, 16.0, 14.0, 3)
                .insert((HeroBeltWidget, HeroBeltRotate));
        }
        // 关闭钮 1923-1925 @(82,19)（横向）/ 1935-1937 @(3,82)（纵向，Flip 换帧）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1923),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1924),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1925),
        ) {
            spawn_icon_button(p, n, h, pr, 82.0, 19.0, 16.0, 14.0, 3)
                .insert((HeroBeltWidget, HeroBeltClose));
        }
    });
}

/// 显隐 + 横纵布局 + 旋转/关闭（布局数学对齐 C# Flip :327-372）
/// 单查询（Option 组件区分角色）避免 Bevy 16 参数上限。
#[allow(clippy::type_complexity)]
fn hero_belt_ui_system(
    mut visible: ResMut<HeroBeltVisible>,
    mut vertical: ResMut<HeroBeltVertical>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    // With<HeroBeltWidget> 限定，避免误碰全屏其它实体（#1362 同坑）——
    // 漏过滤时下面的 Visibility 循环会把全游戏实体强制 Visible，白色占位全显形 = 白屏
    mut items: Query<(
        Entity,
        &mut Node,
        &mut Visibility,
        Option<&mut ImageNode>,
        Option<&Interaction>,
        Option<&mut ImageButton>,
        Option<&HeroBeltBg>,
        Option<&HeroBeltBgOverlay>,
        Option<&HeroBeltSlotCell>,
        Option<&HeroBeltNumber>,
        Option<&HeroBeltRotate>,
        Option<&HeroBeltClose>,
    ), With<HeroBeltWidget>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    for (_, _, mut vis, _, _, _, _, _, _, _, _, _) in &mut items {
        *vis = if visible.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible.0 {
        return;
    }
    let vert = vertical.0;
    // 面板：横 (475,618) 100x38 / 纵 (0,446) 40x101
    let (px, py, pw, ph) = if vert {
        (BELT_VERT_X, BELT_VERT_Y, 40.0, 101.0)
    } else {
        (BELT_X, BELT_Y, 100.0, 38.0)
    };

    for (e, mut node, _, mut img, inter, mut btn, bg, overlay, slot, num, rot, cls) in &mut items {
        if bg.is_some() {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, if vert { 1943 } else { 1921 }) {
                if let Some(img) = img.as_mut() {
                    if img.image != h {
                        img.image = h;
                    }
                }
            }
            node.left = Val::Px(px);
            node.top = Val::Px(py);
            node.width = Val::Px(pw);
            node.height = Val::Px(ph);
        } else if overlay.is_some() {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, if vert { 1946 } else { 1934 }) {
                if let Some(img) = img.as_mut() {
                    if img.image != h {
                        img.image = h;
                    }
                    // C# BeforeDraw 0.5F alpha（叠层纹理本身不透明）
                    img.color = Color::srgba(1.0, 1.0, 1.0, 0.5);
                }
            }
            node.left = Val::Px(0.0);
            node.top = Val::Px(0.0);
            node.width = Val::Px(if vert { 40.0 } else { 92.0 });
            node.height = Val::Px(if vert { 92.0 } else { 38.0 });
        } else if let Some(n) = num {
            let (x, y) = if vert { v_num(n.0) } else { h_num(n.0) };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
        } else if let Some(s) = slot {
            // 格（横向 @(12+i*35,3)；纵向 @(3,i*35+12)，C# :313/:336）
            let (x, y) = if vert { v_slot(s.0) } else { h_slot(s.0) };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
        } else if rot.is_some() {
            // 旋转（横向 @(82,3)；纵向 @(19,82)，C# :343-346）；Flip 换帧组
            let (x, y) = if vert {
                (BELT_VERT_X + 19.0, BELT_VERT_Y + 82.0)
            } else {
                (BELT_X + 82.0, BELT_Y + 3.0)
            };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
            if let Some(btn) = btn.as_mut() {
                if vert {
                    swap_btn_frames(btn, &mut libs, &mut images, (1938, 1939, 1940));
                } else {
                    swap_btn_frames(btn, &mut libs, &mut images, (1926, 1927, 1928));
                }
            }
            if let Some(inter) = inter {
                if edge(e, inter, &mut prev_inter) {
                    vertical.0 = !vertical.0;
                    tracing::info!("🔁 英雄腰带旋转: {}", if !vert { "纵向" } else { "横向" });
                }
            }
        } else if cls.is_some() {
            // 关闭（横向 @(82,19)；纵向 @(3,82)，C# :338-341）；Flip 换帧组
            let (x, y) = if vert {
                (BELT_VERT_X + 3.0, BELT_VERT_Y + 82.0)
            } else {
                (BELT_X + 82.0, BELT_Y + 19.0)
            };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
            if let Some(btn) = btn.as_mut() {
                if vert {
                    swap_btn_frames(btn, &mut libs, &mut images, (1935, 1936, 1937));
                } else {
                    swap_btn_frames(btn, &mut libs, &mut images, (1923, 1924, 1925));
                }
            }
            if let Some(inter) = inter {
                if edge(e, inter, &mut prev_inter) {
                    visible.0 = false;
                    tracing::info!("🧪 关闭英雄腰带");
                }
            }
        }
    }
}

/// 换 ImageButton 三帧（横向/纵向组；image_button_system 按 normal/hover/pressed 刷）
fn swap_btn_frames(
    btn: &mut ImageButton,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    ids: (usize, usize, usize),
) {
    let (n, h, p) = ids;
    if let Some(i) = load_lib_image(libs, images, LibraryName::Prguse, n) {
        btn.normal = i;
    }
    if let Some(i) = load_lib_image(libs, images, LibraryName::Prguse, h) {
        btn.hover = i;
    }
    if let Some(i) = load_lib_image(libs, images, LibraryName::Prguse, p) {
        btn.pressed = i;
    }
}

/// 渲染：图标/数量（英雄背包前 2 格 = 腰带）
fn hero_belt_icon_system(
    hero: Res<HeroState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    // B0001：icons 与 counts 的 &mut Visibility 需 Without 隔离（#1362 同坑）
    mut icons: Query<(&mut ImageNode, &mut Visibility, &HeroBeltIcon), Without<HeroBeltCount>>,
    mut counts: Query<(&mut Text, &mut Visibility, &HeroBeltCount), Without<HeroBeltIcon>>,
) {
    for (mut node, mut vis, icon) in &mut icons {
        if let Some(item) = hero.inventory.get(icon.0).and_then(|s| s.as_ref()) {
            if let Some(h) = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Items,
                item.image as usize,
            ) {
                node.image = h;
            }
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
    for (mut text, mut vis, count) in &mut counts {
        if let Some(item) = hero.inventory.get(count.0).and_then(|s| s.as_ref()) {
            text.0 = if item.count > 1 {
                item.count.to_string()
            } else {
                String::new()
            };
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

/// Belt7/Belt8 快捷键使用的物品 uid（keyboard_layout::secondary_hotkey_system
/// 调用；UseItem 按 uid 全背包查，直接复用 hero_inv 的 use_item_core 链路）
pub fn hero_belt_item_uid(hero: &HeroState, i: usize) -> Option<u64> {
    hero.inventory
        .get(i)
        .and_then(|s| s.as_ref())
        .map(|it| it.unique_id)
}

/// 腰带补货武装标志（审查 MAJOR 窄修）：仅在发出 UseItem(HeroInventory)
/// 且物品在腰带格 0/1 时置 true——补货只覆盖"用尽最后一瓶"，不覆盖
/// 取回/切换英雄等其它清空路径（否则与 TakeBackHeroItem 拉锯回填死循环）
#[derive(Resource, Default)]
pub struct HeroBeltUseArmed(pub bool);

/// 腰带自动补货纯函数（#2611，C# MirItemCell.cs:574-581）：to_slot 格从
/// 「有 prev_index 物品」变为空 → 英雄背包区（2..）找第一件**同 item_index**
/// → 返回 (from, to)（落点=触发格，C# To=ItemSlot）；无匹配返回 None
pub fn belt_refill_move(prev_index: i32, to_slot: usize, hero: &HeroState) -> Option<(i32, i32)> {
    if hero.inventory.get(to_slot).and_then(|s| s.as_ref()).is_some() {
        return None; // 触发格未空
    }
    for from in BELT_SLOTS..hero.inventory.len() {
        if let Some(it) = hero.inventory.get(from).and_then(|s| s.as_ref()) {
            if it.item_index == prev_index {
                return Some((from as i32, to_slot as i32));
            }
        }
    }
    None
}

/// 补货系统：仅 armed（UseItem 发出）时的 Some→None 跃迁触发——
/// C# 补货挂在 UseItem 点击内（Item.Count==1 才发），帧跃迁门控配 armed
/// 标志等价收窄；发 C.MoveItem(Grid=HeroInventory)（服务端 #2611 已加分支）
fn hero_belt_refill_system(
    hero: Res<HeroState>,
    net: Res<crate::network::NetConnection>,
    mut armed: ResMut<HeroBeltUseArmed>,
    mut prev: Local<[Option<i32>; BELT_SLOTS]>,
) {
    if !armed.0 {
        // 未武装也要同步记忆，避免武装瞬间的历史跃迁误触发
        for slot in 0..BELT_SLOTS {
            prev[slot] = hero
                .inventory
                .get(slot)
                .and_then(|s| s.as_ref())
                .map(|it| it.item_index);
        }
        return;
    }
    let mut transition_seen = false;
    for slot in 0..BELT_SLOTS {
        let cur = hero
            .inventory
            .get(slot)
            .and_then(|s| s.as_ref())
            .map(|it| it.item_index);
        // 用尽跃迁：上一帧有、本帧空 → 按上一帧物品类型补回触发格
        if prev[slot].is_some() && cur.is_none() {
            transition_seen = true;
            if let Some((from, to)) = belt_refill_move(prev[slot].unwrap_or(0), slot, &hero) {
                net.send_packet(&mir2_shared::packets::client::item::MoveItem {
                    grid: mir2_shared::enums::MirGridType::HeroInventory,
                    from,
                    to,
                });
                tracing::info!("🧪 腰带补货: 英雄背包{} -> 腰带{}", from, to);
            }
        }
        prev[slot] = cur;
    }
    // 武装只活一次跃迁评估（无论是否命中）——封掉"耗尽但无同类"的
    // 闩锁泄漏（审查复审：闩锁下后续取回/切英雄清空会误触回填）
    if transition_seen {
        armed.0 = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::dialogs::hero::HeroState;

    fn hero_with(inv: Vec<Option<usize>>) -> HeroState {
        // inv: Some(item_index)/None 逐槽；转成 HeroState（借用 InvItem 字段构造
        // 太重——用真实 InvItem 最小构造）
        let mut h = HeroState::default();
        h.inventory = inv
            .into_iter()
            .map(|s| {
                s.map(|idx| crate::game::dialogs::inventory::InvItem {
                    unique_id: idx as u64 + 1,
                    item_index: idx as i32,
                    ..Default::default()
                })
            })
            .collect();
        h
    }

    /// 补货纯函数（C# 语义：用尽格按上一帧物品类型从背包区找同类，落回触发格）
    #[test]
    fn belt_refill_finds_same_index_in_backpack() {
        // 腰带 0 空、背包区 2 有同类 7 → from=2, to=0
        let h = hero_with(vec![None, None, Some(7), Some(9)]);
        assert_eq!(belt_refill_move(7, 0, &h), Some((2, 0)));
        // 背包区无同类 → 不补
        let h = hero_with(vec![None, None, Some(9)]);
        assert_eq!(belt_refill_move(7, 0, &h), None);
        // 触发格未空 → 不动
        let h = hero_with(vec![Some(7), Some(7), Some(9)]);
        assert_eq!(belt_refill_move(7, 0, &h), None);
        // 落点=触发格（审查 MINOR：腰带 0 空、触发格 1 用尽 → 补回 1 而非 0）
        let h = hero_with(vec![None, None, Some(7)]);
        assert_eq!(belt_refill_move(7, 1, &h), Some((2, 1)));
    }

    /// 布局常量对齐 C#（HeroDialogs.cs:261/:333；格子公式 :313/:336）
    #[test]
    fn layout_matches_csharp() {
        // 横向原点 (475,618)；格 0/1 @ (487,621)/(522,621)；键标 (483,620)/(518,620)
        assert_eq!((BELT_X, BELT_Y), (475.0, 618.0));
        assert_eq!(h_slot(0), (487.0, 621.0));
        assert_eq!(h_slot(1), (522.0, 621.0));
        assert_eq!(h_num(0), (483.0, 620.0));
        assert_eq!(h_num(1), (518.0, 620.0));
        // 纵向原点 (0,446)；格 (3,458)/(3,493)；键标 x=-1（C# 字面值 :370）
        assert_eq!((BELT_VERT_X, BELT_VERT_Y), (0.0, 446.0));
        assert_eq!(v_slot(0), (3.0, 458.0));
        assert_eq!(v_slot(1), (3.0, 493.0));
        assert_eq!(v_num(0).0, -1.0);
        assert_eq!(v_num(0).1, 457.0);
    }
}
