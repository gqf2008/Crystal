// ============================================================================
#![allow(clippy::type_complexity)]
// 主对话框 HUD（M8）
// 布局参考：Client/MirScenes/Dialogs/MainDialogs.cs + Client-Macroquad
//   src/scenes/dialogs/game/main_dialog.rs（draw_health_mana_orbs / draw_exp_bar / draw_buttons）
// 纹理：Prguse[分辨率] 背景、Prguse[4] 血蓝球、Prguse[7/8] 经验条、
//       按钮（1900..1914 角色/背包/技能/任务/设置，1960.. 菜单，826.. 商城）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::actor::LocalPlayer;
use crate::actor::PlayerName;
use crate::game::dialogs::character::CharPage;
use crate::game::dialogs::dura_status::{dura_btn_y, MINIMAP_X};
use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::keyboard_layout::{key_name, KeyboardState};
use crate::game::dialogs::minimap::MiniMapMode;
use crate::game::dialogs::option::OptionState;
use crate::game::dialogs::{DialogKind, DialogManager};
use crate::game::player_state::{
    AutoPotion, Gold, Inventory, PetModeState, Progression, StatusFlags, Vitals,
};
use crate::game::sets::GameSet;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiButton;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiEntity, UiFont, UiImageCache,
};

/// HUD 状态（网络 handler 写入，HUD 系统读取）
#[derive(Resource)]
pub struct HudState {
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub max_exp: i64,
    pub level: u16,
    pub gold: u32,
    /// #248 声望/功勋
    pub credit: u32,
    /// #268 基础属性（S.BaseStatsInfo）
    pub base_stats: Vec<i32>,
    pub name: String,
    /// 本地玩家 object_id（UserInformation 提供）
    pub player_object_id: Option<u32>,
    /// 角色职业（显示用）
    pub class: u8,
    /// 角色性别（UserInformation 提供；C# MirGender，CanUseItem 用，#1544）
    pub gender: u8,
    /// 是否骑乘中（MountUpdated 本地玩家，#1544：骑乘时仅 Scroll/Potion/Torch 可用）
    pub riding: bool,
    /// 坐骑类型（MountUpdated 本地玩家，#1564：骑乘音效区分 Tiger/Wolf）
    pub mount_type: i16,
    /// 是否钓鱼中（FishingUpdate 本地玩家，#1544：钓鱼时不可使用物品）
    pub fishing: bool,
    /// #1616：本地玩家麻痹/冰冻毒（C# CheckInput：Paralysis/LRParalysis/Frozen 锁定输入）
    pub paralysis: bool,
    /// #1550：陷阱岩石（C# User.InTrapRock：陷阱中不可走/跑）
    pub in_trap_rock: bool,
    /// #1552：冲刺（C# User.Sprint，SwiftFeet Buff）——CanRun 3 格
    pub sprint: bool,
    /// #1552：潜行（C# User.Sneaking，MoonLight/DarkBody Buff）——不可跑 + 半透明
    pub sneaking: bool,
    /// 自动喝药开关（HP < 35% 自动使用背包药品）
    pub auto_pot_hp: bool,
    /// 玩家死亡（Death 包置位，Revived 清除；死亡时禁用输入/显示遮罩）
    pub dead: bool,
    /// 收到轮回术复活请求（#222）
    pub reincarnation_offered: bool,
    /// 死亡弹窗已点击“否”关闭（C# ShowReviveMessage 只弹一次；死亡后重置）
    pub death_popup_dismissed: bool,
    /// 自动喝药冷却（避免连发）
    pub pot_cooldown: f32,
    /// 背包（网络 UserInformation 写入）
    pub inventory: crate::game::dialogs::inventory::InventoryState,
    /// 装备（12 槽）
    pub equipment: Vec<Option<InvItem>>,
    /// #1388：宠物模式（C# PModeLabel；S.ChangePMode 更新）
    pub pet_mode: mir2_shared::enums::PetMode,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            hp: 1,
            max_hp: 1000,
            mp: 1,
            max_mp: 600,
            exp: 0,
            max_exp: 100,
            level: 1,
            gold: 0,
            credit: 0,
            base_stats: Vec::new(),
            name: String::new(),
            player_object_id: None,
            class: 0,
            gender: 0,
            riding: false,
            mount_type: 0,
            fishing: false,
            paralysis: false,
            in_trap_rock: false,
            sprint: false,
            sneaking: false,
            auto_pot_hp: true,
            dead: false,
            reincarnation_offered: false,
            death_popup_dismissed: false,
            pot_cooldown: 0.0,
            inventory: Default::default(),
            equipment: vec![None; 14], // #1136：服务端补 Torch/Belt/Stone 共 14 槽
            pet_mode: mir2_shared::enums::PetMode::Both,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HudButtonKind {
    Character,
    Inventory,
    Skills,
    QuestLog,
    Option,
    Menu,
    GameShop,
    Hero,
}

#[derive(Component)]
pub struct HudButton(pub HudButtonKind);

/// #1331：HUD 英雄按钮（C# HeroMenuButton，有英雄才显示）
#[derive(Component)]
pub struct HeroBtn;

/// #1357：HUD 英雄状态小面板（C# HeroInfoPanel：名字/等级/HP/MP/经验）
#[derive(Component)]
pub struct HeroPanel;
#[derive(Component)]
pub struct HeroPanelText(usize);

/// HUD 显示数据快照（#70 试点：挂 HUD 根实体；值变化时才写组件，
/// hud_update_system 用 Changed<HudData> 门控，血条/文字只在数据变化帧更新）
#[derive(Component, Default, PartialEq, Clone)]
pub struct HudData {
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub max_exp: i64,
    pub level: u16,
    pub gold: u32,
    pub name: String,
}

/// #1392：HUD 负重/空格标签（C# WeightLabel=剩余负重，SpaceLabel=背包空格数）
fn hud_space_weight_system(
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut wt: Query<&mut Text2d, (With<HudWeightText>, Without<HudSpaceText>)>,
    mut sp: Query<&mut Text2d, (With<HudSpaceText>, Without<HudWeightText>)>,
) {
    // #2633 批次4 步5：负重/空格读 Inventory 组件；实体缺失视同空背包（同原 hud 默认 0）
    let (max_weight, weight, space) = inv_q
        .single()
        .map(|inv| {
            (
                inv.max_weight,
                inv.weight,
                inv.items.iter().filter(|s| s.is_none()).count(),
            )
        })
        .unwrap_or((0, 0, 0));
    let rem = max_weight.saturating_sub(weight);
    // C# WeightLabel = (BagWeight - CurrentBagWeight).ToString()：仅剩余负重（不带 /max）
    let w = format!("{}", rem);
    for mut t in &mut wt {
        if t.0 != w {
            t.0 = w.clone();
        }
    }
    let space = space.to_string();
    for mut t in &mut sp {
        if t.0 != space {
            t.0 = space.clone();
        }
    }
}

/// #1357：HUD 英雄状态小面板（C# HeroInfoPanel：名字 Lv/HP/MP/经验，有英雄才显示）
fn hero_panel_system(
    hero: Res<crate::game::dialogs::hero::HeroState>,
    mut texts: Query<(&mut Text2d, &HeroPanelText)>,
    mut widgets: Query<&mut Visibility, (With<HeroPanel>, Without<HeroPanelText>)>,
    mut text_vis: Query<&mut Visibility, (With<HeroPanelText>, Without<HeroPanel>)>,
) {
    let show = hero.current.is_some();
    for mut v in &mut widgets {
        *v = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut v in &mut text_vis {
        *v = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if !show {
        return;
    }
    let cur = hero.current.as_ref().unwrap();
    for (mut text, kind) in &mut texts {
        text.0 = match kind.0 {
            0 => format!("{} Lv.{}", cur.name, cur.level),
            1 => format!("HP {}", hero.hero_hp),
            2 => format!("MP {}", hero.hero_mp),
            _ => format!("经验 {}/{}", hero.hero_exp, hero.hero_max_exp),
        };
    }
}

/// #1331：英雄按钮显隐（C# HeroMenuButton.Visible = 有英雄）
fn hero_btn_system(hero: Res<crate::game::dialogs::hero::HeroState>, mut btns: Query<&mut Visibility, With<HeroBtn>>) {
    let show = hero.current.is_some();
    for mut v in &mut btns {
        *v = if show { Visibility::Visible } else { Visibility::Hidden };
    }
}

/// HUD 按钮悬停提示（C# MirButton Hint：名称 + 快捷键；source=11 与对话框 tooltip 隔离）
fn hud_tooltip_system(
    kb: Res<KeyboardState>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    windows: Query<&Window>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
    buttons: Query<(&UiButton, &HudButton)>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((cam, gtf)) = ui_cameras.single() else { return };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else { return };
    let cursor = Vec2::new(world.x, -world.y);

    let mut hit: Option<(String, String)> = None;
    for (btn, kind) in &buttons {
        let (x, y, w, h) = btn.rect;
        if cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h {
            hit = hud_button_hint(kind.0, &kb);
            break;
        }
    }
    match hit {
        Some((name, key)) => {
            let line = if key.is_empty() {
                name.clone()
            } else {
                format!("{}（{}）", name, key)
            };
            tooltip.update(11, true, name, vec![line], cursor.x, cursor.y);
        }
        None => tooltip.update(11, false, String::new(), Vec::new(), 0.0, 0.0),
    }
}

/// HUD 按钮名称 + 绑定快捷键（无键位绑定的按钮只显示名称）
fn hud_button_hint(kind: HudButtonKind, kb: &KeyboardState) -> Option<(String, String)> {
    let (name, action): (&str, Option<&str>) = match kind {
        HudButtonKind::Character => ("角色", Some("角色")),
        HudButtonKind::Inventory => ("背包", Some("背包")),
        HudButtonKind::Skills => ("技能", Some("技能")),
        HudButtonKind::QuestLog => ("任务", Some("任务")),
        HudButtonKind::Option => ("设置", Some("设置")),
        HudButtonKind::Menu => ("菜单", None),
        HudButtonKind::GameShop => ("商城", Some("商城")),
        HudButtonKind::Hero => ("英雄", None),
    };
    let key = action
        .and_then(|a| kb.bindings.iter().find(|b| b.action == a))
        .map(|b| key_name(b.key))
        .unwrap_or_default();
    Some((name.to_string(), key))
}

/// HUD 按钮 → 对话框开关（M9：接入 DialogManager）
fn hud_button_system(mut mgr: ResMut<DialogManager>, mut page: ResMut<CharPage>, buttons: Query<(&UiButton, &HudButton)>) {
    for (btn, kind) in &buttons {
        if btn.clicked {
            tracing::info!("🎛️ HUD 按钮点击: {:?}", kind.0);
            match kind.0 {
                HudButtonKind::Inventory => mgr.toggle(DialogKind::Inventory),
                HudButtonKind::Character => mgr.toggle(DialogKind::Character),
                HudButtonKind::Skills => {
                    // C# MainDialogs.SkillButton → CharacterDialog.ShowSkillPage()
                    if mgr.is_open(DialogKind::Character) && page.0 == 3 {
                        mgr.close(DialogKind::Character);
                    } else {
                        mgr.open(DialogKind::Character);
                        page.0 = 3;
                    }
                }
                HudButtonKind::QuestLog => mgr.toggle(DialogKind::QuestLog),
                HudButtonKind::Option => mgr.toggle(DialogKind::Settings),
                HudButtonKind::Menu => mgr.toggle(DialogKind::Menu),
                HudButtonKind::GameShop => mgr.toggle(DialogKind::GameShop),
                HudButtonKind::Hero => mgr.toggle(DialogKind::Hero),
            }
        }
    }
}

/// 动态部件标记（每帧按 HudState 更新）
#[derive(Component)]
struct OrbBase(f32);
#[derive(Component)]
struct HpHpFill;
#[derive(Component)]
struct MpMpFill;
/// 经验条填充：存完整条宽（C# ExperienceBar.Size.Width），避免按已收缩 rect 连乘导致越涨越窄
#[derive(Component)]
struct ExpFill(f32);
#[derive(Component)]
struct HpHpText;
#[derive(Component)]
struct MpMpText;
/// hp_view=false 两行格式标签（C# TopLabel/BottomLabel：HealthOrb 相对 (9,20)/(9,50)，85x30 框内水平居中）
#[derive(Component)]
struct TopHudText;
#[derive(Component)]
struct BottomHudText;
#[derive(Component)]
struct ExpText;
#[derive(Component)]
struct LevelText;
#[derive(Component)]
struct GoldText;
#[derive(Component)]
struct NameText;

/// 死亡遮罩（全屏半透明 + 文字 + 复活按钮，#46）
#[derive(Component)]
struct DeathOverlay;
#[derive(Component)]
struct DeathReviveBtn;

/// 轮回术拒绝按钮（#222）
#[derive(Component)]
struct DeathReincDeclineBtn;
#[derive(Component)]
struct DeathText;

const ORB_HEIGHT: f32 = 80.0;
const ORB_TOP: f32 = 30.0;
const EXP_TOP: f32 = 143.0;
const BUTTON_TOP: f32 = 76.0;

// C# MainDialog 标签位置（MainDialogs.cs 构造器，对话框相对坐标；1024 分辨率）
/// LevelLabel @ (5,108)：纯等级数字
pub const HUD_LEVEL_X: f32 = 5.0;
pub const HUD_LEVEL_Y: f32 = 108.0;
/// CharacterName @ (6,120) 90x16
pub const HUD_NAME_X: f32 = 6.0;
pub const HUD_NAME_Y: f32 = 120.0;
/// CharacterName 框尺寸 Size(90,16)：C# DrawFormat=HCenter|VCenter 在框内双向居中
pub const HUD_NAME_W: f32 = 90.0;
pub const HUD_NAME_H: f32 = 16.0;
/// GoldLabel @ (Width-105, 119)
pub const HUD_GOLD_DX: f32 = 105.0;
pub const HUD_GOLD_Y: f32 = 119.0;
/// HealthOrb 标签：Label_SizeChanged 水平居中于球心 x=50；HealthLabel/ManaLabel 球体相对 y=27/42
pub const HUD_ORB_CX: f32 = 50.0;
pub const HUD_HP_ORB_Y: f32 = 27.0;
pub const HUD_MP_ORB_Y: f32 = 42.0;
/// C# TopLabel/BottomLabel 水平中心：Location.X(9) + Size.Width(85)/2 = 51.5（DrawFormat=HorizontalCenter）
pub const HUD_2LINE_CX: f32 = 51.5;
/// C# TopLabel/BottomLabel 距 HealthOrb 的 y：Location (9,20)/(9,50)（HealthOrb @ (0,30)）
pub const HUD_TOP_LABEL_DY: f32 = 20.0;
pub const HUD_BOTTOM_LABEL_DY: f32 = 50.0;
/// ExperienceLabel @ (ExperienceBar.Width/2 - 20, -10)（经验条相对，条上方 10px）
pub const HUD_EXP_LABEL_DX: f32 = 20.0;
pub const HUD_EXP_LABEL_DY: f32 = 10.0;

/// 模式标签 X（C# MiniMapDialog.Process :2082-2087：MiniMapDialog.X - 3 = 898 - 3）
pub const MODE_LABEL_X: f32 = MINIMAP_X - 3.0; // 895
/// 三标签 y 偏移（C# Process: S=H+150 / A=H+165 / P=H+180；
/// 绝对 y = 小地图高 + offset - 152，其中 152 = ScreenHeight(768) - MainDialog.Y(616)）
pub const S_MODE_DY: f32 = -2.0;
pub const A_MODE_DY: f32 = 13.0;
pub const P_MODE_DY: f32 = 28.0;

/// 模式标签绝对 y（随小地图大/小模式，C# Process 每帧重定位；复用 dura_btn_y 的大/小高选择）
pub fn mode_label_y(minimap_big: bool, dy: f32) -> f32 {
    dy + dura_btn_y(minimap_big)
}

/// 模式标签可见性（C# 构造 Visible=Settings.ModeView，仅 INI，无游戏内开关）
fn mode_visibility(mode_view: bool) -> Visibility {
    if mode_view {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

/// 生成单个模式标签（S/A/P 共用：x=MiniMap.X-3，y=小地图高+dy，12px，z=4，挂 marker + 门控可见性）
/// 黑描边（C# MainDialogs.cs:356/366/376 仅设 OutLineColour 未关 OutLine → 构造默认
/// OutLine=true（MirLabel.cs:181-182）= 有描边，#2563）
fn spawn_mode_label(
    commands: &mut Commands,
    font: &Handle<Font>,
    text: &str,
    minimap_big: bool,
    dy: f32,
    color: Color,
    vis: Visibility,
    marker: impl Component,
) -> Entity {
    let e = spawn_ui_text(
        commands,
        font,
        text,
        MODE_LABEL_X,
        mode_label_y(minimap_big, dy),
        12.0,
        color,
        4.0,
    );
    commands.entity(e).insert((marker, vis));
    crate::ui::outlined_text::outline_on(
        commands,
        e,
        text,
        font.clone(),
        12.0,
        bevy::sprite::Anchor::TOP_LEFT,
        false,
    );
    e
}

/// 攻击模式指示（C# AModeLabel，右上小地图正下方）
#[derive(Component)]
pub struct AttackModeText;
/// 宠物模式指示（C# PModeLabel，右上小地图正下方）
#[derive(Component)]
pub struct PModeText;
/// 技能模式指示（C# SModeLabel，右上小地图正下方）
#[derive(Component)]
pub struct SModeText;
/// #1392：HUD 负重标签（C# WeightLabel）
#[derive(Component)]
pub struct HudWeightText;
/// #1392：HUD 空格标签（C# SpaceLabel）
#[derive(Component)]
pub struct HudSpaceText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        // #2633 批次4：原 hud_server_events（518 行上帝系统）已按域拆为 4 个写系统——
        // player_vitals_events / player_status_events（game/player_state.rs）、
        // inventory_events（dialogs/inventory.rs）、belt_restock_events（dialogs/potion_belt.rs），
        // 均入 GameSet::PlayerState（.before(Hud)，维持「写方在读方前」），并保留 HudState 双写过渡。
        app.add_systems(OnEnter(AppState::Game), spawn_hud);
        app.add_systems(OnExit(AppState::Game), cleanup_hud);
        // #2632：放宽 11 系统 .chain() 全串行——只保留确有数据依赖的排序，其余解链并行。
        // 保留的依赖（写方须排在读方前，晚一帧读会引入一帧滞后）：
        //   · ui_button_system 每帧写 UiButton.clicked → hud_button / death_overlay 读；
        //   · sync_hud_data 写 HudData → hud_update_system 以 Changed<HudData> 门控消费；
        //   · auto_potion_system 读 StatusFlags.dead 与 death_overlay_system 写 StatusFlags.dead
        //     共享玩家实体组件（#2633 步3 由 ResMut<HudState> 迁来），保持原「先读 dead、
        //     后写 dead」的相对先后。
        // 其余（attack_mode/hero_btn/hero_panel/space_weight/tooltip）读写的组件互不相交，
        // 顺序不影响输出，解链允许并行。
        app.add_systems(
            Update,
            (
                (ui_button_system, hud_button_system, death_overlay_system).chain(),
                (sync_hud_data, hud_update_system).chain(),
                auto_potion_system.before(death_overlay_system),
                attack_mode_text_system,
                hero_btn_system,
                hero_panel_system,
                hud_space_weight_system,
                hud_tooltip_system,
            )
                .run_if(in_state(AppState::Game))
                .in_set(GameSet::Hud),
        );
    }
}

fn cleanup_hud(mut commands: Commands, roots: Query<Entity, With<UiEntity>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_hud(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    opt: Res<OptionState>,
    mmap: Res<MiniMapMode>,
) {
if !crate::ui::sprite_ui::ui_enabled("hud") {
    return;
}

    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 分辨率索引：窗口 1024 宽 → 1（与 macroquad 一致：800→0，1024→1，其他→2）
    let resolution_index = 1usize;
    let bg_info = libs
        .0
        .get_image(LibraryName::Prguse, resolution_index)
        .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
        .unwrap_or((1024.0, 150.0));
    let (bg_w, bg_h) = bg_info;
    let main_x = (1024.0 - bg_w) / 2.0;
    let main_y = 768.0 - bg_h;

    // #70：HUD 数据根实体（无渲染，仅承载 HudData；值变化时触发 Changed 门控更新）
    commands.spawn((UiEntity, HudData::default()));

    // 背景
    // HUD 底条 Prguse[1] 数据本身 41% 不透明（黑底透明+装饰）：用原始 alpha（黑→透明 workaround
    // 对数据已是透明的黑像素无影响），恢复 C# 半透明装饰效果，而不是实心黑带。
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        resolution_index,
    ) {
        // z=2.0：>=2 避开深度剔除（<2 的 UI 精灵不渲染）
        spawn_ui_sprite(&mut commands, h, main_x, main_y, 2.0, 1.0);
    }

    // 血/蓝球填充（Prguse[4]：左半红 HP、右半蓝 MP）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 4) {
        let orb_x = main_x;
        let orb_y = main_y + ORB_TOP;
        // HP 球（左半）
        commands.spawn((
            UiEntity,
            OrbBase(-orb_y),
            HpHpFill,
            Sprite {
                image: h.clone(),
                rect: Some(Rect::new(0.0, 0.0, 50.0, ORB_HEIGHT)),
                custom_size: Some(Vec2::new(50.0, ORB_HEIGHT)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(orb_x, -orb_y, 2.0),
            Visibility::default(),
        ));
        // MP 球（右半）
        commands.spawn((
            UiEntity,
            OrbBase(-orb_y),
            MpMpFill,
            Sprite {
                image: h,
                rect: Some(Rect::new(51.0, 0.0, 101.0, ORB_HEIGHT)),
                custom_size: Some(Vec2::new(50.0, ORB_HEIGHT)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(orb_x + 51.0, -orb_y, 2.0),
            Visibility::default(),
        ));
    }

    // 经验条（Prguse[8]；800 宽用 7）
    // C# ExperienceBar @ (9,143)，Size = Prguse[8] 实测 1004x8（1024 分辨率）
    let (exp_bar_w, exp_bar_h) = libs
        .0
        .get_image(LibraryName::Prguse, 8)
        .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
        .unwrap_or((100.0, 5.0));
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 8) {
        let bar_x = main_x + 9.0;
        let bar_y = main_y + EXP_TOP;
        commands.spawn((
            UiEntity,
            ExpFill(exp_bar_w),
            Sprite {
                image: h,
                rect: Some(Rect::new(0.0, 0.0, exp_bar_w, exp_bar_h)),
                custom_size: Some(Vec2::new(exp_bar_w, exp_bar_h)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(bar_x, -bar_y, 2.0),
            Visibility::default(),
        ));
    }

    // 文本（位置逐项对齐 C# MainDialog 标签的对话框相对坐标）
    let orb_x = main_x;
    let orb_y = main_y + ORB_TOP;
    // C# HealthLabel/ManaLabel：父为 HealthOrb(0,30)，标签球体相对 (0,27)/(0,42)，
    // 由 Label_SizeChanged 水平居中于球心 x=50（x=50-width/2）→ 用 TOP_CENTER 锚定自动居中。
    spawn_centered_text(
        &mut commands,
        &font,
        HpHpText,
        Anchor::TOP_CENTER,
        orb_x + HUD_ORB_CX,
        orb_y + HUD_HP_ORB_Y,
        "",
    );
    spawn_centered_text(
        &mut commands,
        &font,
        MpMpText,
        Anchor::TOP_CENTER,
        orb_x + HUD_ORB_CX,
        orb_y + HUD_MP_ORB_Y,
        "",
    );
    // C# TopLabel/BottomLabel（仅 HPView=false 显示）：框 85x30、HorizontalCenter
    // → TOP_CENTER 锚点于框水平中心 x=51.5、框顶 y（HealthOrb 相对 (9,20)/(9,50)）
    spawn_centered_text(
        &mut commands,
        &font,
        TopHudText,
        Anchor::TOP_CENTER,
        orb_x + HUD_2LINE_CX,
        orb_y + HUD_TOP_LABEL_DY,
        "",
    );
    spawn_centered_text(
        &mut commands,
        &font,
        BottomHudText,
        Anchor::TOP_CENTER,
        orb_x + HUD_2LINE_CX,
        orb_y + HUD_BOTTOM_LABEL_DY,
        "",
    );
    // C# ExperienceLabel.Location = (ExperienceBar.Width/2 - 20, -10)（经验条相对：居中偏左、条上方 10px）
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        ExpText,
        main_x + 9.0 + exp_bar_w / 2.0 - HUD_EXP_LABEL_DX,
        main_y + EXP_TOP - HUD_EXP_LABEL_DY,
        "",
    );
    // C# LevelLabel @ (5,108)：纯等级数字
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        LevelText,
        main_x + HUD_LEVEL_X,
        main_y + HUD_LEVEL_Y,
        "",
    );
    // C# GoldLabel @ (Width-105, 119)
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        GoldText,
        main_x + bg_w - HUD_GOLD_DX,
        main_y + HUD_GOLD_Y,
        "",
    );
    // C# CharacterName @ (6,120) Size(90,16)，DrawFormat=HCenter|VCenter 在框内双向居中
    // → 用 CENTER 锚定框心 (6+90/2, 120+16/2)=(51,128)，短名字也在 90px 框内居中、内容变化自动重居中
    spawn_centered_text(
        &mut commands,
        &font,
        NameText,
        Anchor::CENTER,
        main_x + HUD_NAME_X + HUD_NAME_W / 2.0,
        main_y + HUD_NAME_Y + HUD_NAME_H / 2.0,
        "",
    );

    // 主对话框按钮（C# 位置：Size.Width - 119/-96/-73/-50/-27，y=+76）
    let button_y = main_y + BUTTON_TOP;
    let buttons: [(HudButtonKind, usize, usize, usize, f32); 5] = [
        (HudButtonKind::Character, 1900, 1901, 1902, bg_w - 119.0),
        (HudButtonKind::Inventory, 1903, 1904, 1905, bg_w - 96.0),
        (HudButtonKind::Skills, 1906, 1907, 1908, bg_w - 73.0),
        (HudButtonKind::QuestLog, 1909, 1910, 1911, bg_w - 50.0),
        (HudButtonKind::Option, 1912, 1913, 1914, bg_w - 27.0),
    ];
    for (kind, n, h, p, xoff) in buttons {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            n,
            h,
            p,
            main_x + xoff,
            button_y,
            3.0,
            23.0,
            23.0,
        ) {
            commands.entity(e).insert(HudButton(kind));
        }
    }
    // 菜单按钮（C#：Width-55, 35）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        1960,
        1961,
        1962,
        main_x + bg_w - 55.0,
        main_y + 35.0,
        3.0,
        23.0,
        23.0,
    ) {
        commands.entity(e).insert(HudButton(HudButtonKind::Menu));
    }
    // 商城按钮（C#：Width-105, 35）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        826,
        827,
        828,
        main_x + bg_w - 105.0,
        main_y + 35.0,
        3.0,
        23.0,
        23.0,
    ) {
        commands
            .entity(e)
            .insert(HudButton(HudButtonKind::GameShop));
    }
    // 英雄按钮（C# MainDialog HeroMenuButton：Prguse 2164/2165/2166，(Width-160, 65)，20x20）
    // #1331：点击打开英雄面板；有英雄（HeroState.current）才显示
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        2164,
        2165,
        2166,
        main_x + bg_w - 160.0,
        main_y + 65.0,
        3.0,
        20.0,
        20.0,
    ) {
        commands
            .entity(e)
            .insert((HudButton(HudButtonKind::Hero), HeroBtn, Visibility::Hidden));
    }

    // #1357：英雄状态小面板（C# HeroInfoPanel Prguse[14] @(95,48)，有英雄才显示）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 14) {
        let e = spawn_ui_sprite(&mut commands, h, main_x + 95.0, main_y + 48.0, 3.0, 1.0);
        commands.entity(e).insert((HeroPanel, Visibility::Hidden));
    }
    let panel_texts: [(&str, f32, f32); 4] = [
        ("", 26.0, 8.0),
        ("", 8.0, 28.0),
        ("", 8.0, 44.0),
        ("", 8.0, 60.0),
    ];
    for (i, (_, dx, dy)) in panel_texts.iter().enumerate() {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            main_x + 95.0 + dx, main_y + 48.0 + dy,
            11.0, Color::WHITE, 3.2,
        );
        commands.entity(e).insert((HeroPanelText(i), Visibility::Hidden));
    }

    // 死亡弹窗（对齐 C# GameScene.ShowReviveMessage → MirMessageBox(YesNo)）：
    // Prguse[360] 居中 (284,289)，文案 DiedTip，是/否按钮 Title[206-208]/[210-212]
    // （按钮纹理自带“是/否”文字，不再额外绘制文字）；轮回术请求时复用同一弹窗。
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DeathOverlay,
        Sprite {
            image: white,
            custom_size: Some(Vec2::new(1024.0, 768.0)),
            color: Color::srgba(0.0, 0.0, 0.0, 0.5),
            ..default()
        },
        Transform::from_xyz(512.0, -384.0, 10.0),
        Visibility::Hidden,
    ));
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, 284.0, 289.0, 10.5, 1.0);
        commands.entity(e).insert((DeathOverlay, Visibility::Hidden));
    }
    let death_txt = spawn_ui_text(
        &mut commands,
        &font,
        "你已经死亡，是否要在城镇复活？",
        319.0,
        324.0,
        16.0,
        Color::WHITE,
        11.0,
    );
    commands.entity(death_txt).insert((DeathText, DeathOverlay, Visibility::Hidden));
    // 是（TownRevive / 轮回术接受）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        544.0, 446.0, 11.0, 76.0, 25.0,
    ) {
        commands.entity(e).insert((DeathReviveBtn, DeathOverlay, Visibility::Hidden));
    }
    // 否（关闭弹窗 / 轮回术拒绝）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        644.0, 446.0, 11.0, 76.0, 25.0,
    ) {
        commands.entity(e).insert((DeathReincDeclineBtn, DeathOverlay, Visibility::Hidden));
    }

    // 模式标签（C# AMode/PMode/SModeLabel）：右上小地图正下方，顶→底 S/A/P。
    // X = MiniMap.X-3 = 895；y 随小地图大/小模式（C# Process 每帧重定位，attack_mode_text_system 跟随）。
    // 颜色对齐 C# 命名色：AMode=Yellow、SMode=LimeGreen、PMode=Orange。
    // 仅当 Settings.ModeView（仅 INI，无游戏内开关）为 true 时可见（C# 构造 Visible=Settings.ModeView）。
    let mode_vis = mode_visibility(opt.mode_view);
    let big = mmap.big;
    spawn_mode_label(&mut commands, &font, "技能:Ctrl", big, S_MODE_DY, Color::srgb(0.196, 0.804, 0.196), mode_vis, SModeText);
    spawn_mode_label(&mut commands, &font, "模式:和平", big, A_MODE_DY, Color::srgb(1.0, 1.0, 0.0), mode_vis, AttackModeText);
    spawn_mode_label(&mut commands, &font, "宠物:跟随", big, P_MODE_DY, Color::srgb(1.0, 0.647, 0.0), mode_vis, PModeText);
    // #1392：负重/空格（C# WeightLabel/SpaceLabel @(Width-105/Width-30, 101)）
    let wt = spawn_ui_text(&mut commands, &font, "0/0", main_x + bg_w - 105.0, main_y + 101.0, 11.0, Color::WHITE, 4.0);
    commands.entity(wt).insert(HudWeightText);
    let sp = spawn_ui_text(&mut commands, &font, "0", main_x + bg_w - 30.0, main_y + 101.0, 11.0, Color::WHITE, 4.0);
    commands.entity(sp).insert(HudSpaceText);
}

#[allow(clippy::too_many_arguments)]
fn spawn_text(
    commands: &mut Commands,
    font: &Handle<Font>,
    _images: &mut Assets<Image>,
    _cache: &mut UiImageCache,
    _marker: impl Component,
    x: f32,
    y: f32,
    text: &str,
) {
    let e = spawn_ui_text(commands, font, text, x, y, 12.0, Color::WHITE, 4.0);
    commands.entity(e).insert(_marker);
}

/// 居中标签（内容变化自动重居中，复刻 C# 居中语义）。`anchor` 决定居中方式、`(x,y)` 传锚点：
/// - TOP_CENTER：C# Label_SizeChanged「x=中心-width/2、y 不变」（HP/MP 球标签，水平居中于球心）。
/// - CENTER：C# DrawFormat=HCenter|VCenter 框内双向居中（CharacterName 90x16 框，锚点=框心）。
#[allow(clippy::too_many_arguments)]
fn spawn_centered_text(
    commands: &mut Commands,
    font: &Handle<Font>,
    _marker: impl Component,
    anchor: Anchor,
    x: f32,
    y: f32,
    text: &str,
) {
    let e = commands
        .spawn((
            UiEntity,
            Text2d::new(text),
            anchor,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(x, -y, 4.0),
            Visibility::default(),
        ))
        .id();
    commands.entity(e).insert(_marker);
}

/// 自动喝药（M10）：HP < 35% 且冷却结束 → 使用背包药品（UseItem）
///
/// #2633 批次4 步3：hp/max_hp→`Vitals`、dead→`StatusFlags`、inventory→`Inventory`、
/// auto_pot_hp/pot_cooldown→`AutoPotion`（enabled/cooldown，本系统是其唯一读者/写者，
/// 设计 §4.5，可整体迁离 HudState；`hud.auto_pot_hp`/`hud.pot_cooldown` 自此无读写、
/// 待步9 统一删）。实体缺失跳过——喝药本就需实体在场，缺席不动作（原 HudState 默认
/// 放行语义无对应组件）。
fn auto_potion_system(
    net: Res<crate::network::NetConnection>,
    time: Res<Time>,
    mut player: Query<(&Vitals, &StatusFlags, &Inventory, &mut AutoPotion), With<LocalPlayer>>,
) {
    let Ok((vitals, flags, inventory, mut auto_pot)) = player.single_mut() else {
        return;
    };
    auto_pot.cooldown -= time.delta_secs();
    if flags.dead || !auto_pot.enabled || auto_pot.cooldown > 0.0 {
        return;
    }
    let pct = vitals.hp as f32 / vitals.max_hp.max(1) as f32;
    if pct < 0.35 {
        // #1592：优先 HP 药（shape==0），无则退化为任意药水（避免喝蓝药不回复 HP）
        let potion = crate::game::dialogs::inventory::pick_auto_hp_potion(
            inventory.items.iter().flatten(),
        );
        if let Some(potion) = potion {
            net.send_packet(&mir2_shared::packets::client::item::UseItem {
                unique_id: potion.unique_id,
            });
            tracing::info!(
                "💊 自动喝药 {} (uid={})（HP {}/{}）",
                potion.name,
                potion.unique_id,
                vitals.hp,
                vitals.max_hp
            );
            auto_pot.cooldown = 3.0;
        }
    }
}

/// 模式标签单条更新：文本变化即写 + y 偏离目标 >0.5 才重定位（C# Process 每帧重定位的 0.5px 阈值版本）
/// 文本变化时同帧同步 4 个描边副本（#2563；写方直同步，规避 sync_outline_system 排序依赖）
fn update_mode_label(
    t: &mut Text2d,
    tf: &mut Transform,
    children: Option<&Children>,
    shadows: &mut Query<
        &mut Text2d,
        (
            With<crate::ui::outlined_text::OutlineShadow>,
            Without<AttackModeText>,
            Without<PModeText>,
            Without<SModeText>,
        ),
    >,
    want: &str,
    y: f32,
) {
    if t.0 != want {
        t.0 = want.to_string();
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut shadow) = shadows.get_mut(child) {
                    shadow.0 = want.to_string();
                }
            }
        }
    }
    if (tf.translation.y - y).abs() > 0.5 {
        tf.translation.y = y;
    }
}

/// 每帧按 HudState 更新血/蓝/经验条与文本（单查询避免 Bevy B0001 冲突）
/// 模式指示更新（#156 攻击 / #1388 宠物+技能）：值变化即写，无变化跳过
fn attack_mode_text_system(
    mode: Res<crate::game::combat::AttackModeState>,
    // #2633 批次4 步3：pet_mode→`PetModeState`；实体缺失回退 Both（同原 HudState 默认）。
    pet: Query<&PetModeState, With<LocalPlayer>>,
    opt: Res<OptionState>,
    mmap: Res<MiniMapMode>,
    mut am: Query<
        (&mut Text2d, &mut Transform, Option<&Children>),
        (With<AttackModeText>, Without<PModeText>, Without<SModeText>),
    >,
    mut pm: Query<
        (&mut Text2d, &mut Transform, Option<&Children>),
        (With<PModeText>, Without<AttackModeText>, Without<SModeText>),
    >,
    mut sm: Query<
        (&mut Text2d, &mut Transform, Option<&Children>),
        (With<SModeText>, Without<AttackModeText>, Without<PModeText>),
    >,
    // 描边副本（#2563：With<OutlineShadow> 且无三 marker，与 am/pm/sm 可证互斥）
    mut shadows: Query<
        &mut Text2d,
        (
            With<crate::ui::outlined_text::OutlineShadow>,
            Without<AttackModeText>,
            Without<PModeText>,
            Without<SModeText>,
        ),
    >,
) {
    let a = format!("模式:{}", crate::game::combat::attack_mode_name(mode.mode));
    let ay = -mode_label_y(mmap.big, A_MODE_DY);
    for (mut t, mut tf, children) in &mut am {
        update_mode_label(&mut t, &mut tf, children, &mut shadows, &a, ay);
    }
    let pet_mode = pet.single().map(|p| p.0).unwrap_or(mir2_shared::enums::PetMode::Both);
    let p = match pet_mode {
        mir2_shared::enums::PetMode::Both => "宠物:攻击和跟随".to_string(),
        mir2_shared::enums::PetMode::MoveOnly => "宠物:仅跟随".to_string(),
        mir2_shared::enums::PetMode::AttackOnly => "宠物:仅攻击".to_string(),
        mir2_shared::enums::PetMode::None => "宠物:不行动".to_string(),
        mir2_shared::enums::PetMode::FocusMasterTarget => "宠物:跟随目标".to_string(),
        _ => "宠物:未知".to_string(),
    };
    let py = -mode_label_y(mmap.big, P_MODE_DY);
    for (mut t, mut tf, children) in &mut pm {
        update_mode_label(&mut t, &mut tf, children, &mut shadows, &p, py);
    }
    let s = if opt.skill_mode_ctrl { "技能:Ctrl".to_string() } else { "技能:~".to_string() };
    let sy = -mode_label_y(mmap.big, S_MODE_DY);
    for (mut t, mut tf, children) in &mut sm {
        update_mode_label(&mut t, &mut tf, children, &mut shadows, &s, sy);
    }
}

/// #2633 批次4 步3/步7：hp/max_hp/mp/max_mp→`Vitals`、exp/max_exp/level→`Progression`、
/// gold→`Gold`、name→复用 `PlayerName`（步7 迁；hud.name 仍双写，步9 删）。
/// R3：保留 `Changed<HudData>` 跳帧门控——仍「值变才写 HudData」（`if *data != new`），
/// 不改成每帧无条件写；R4：一律读组件当前值，不加 `Changed<组件>` 过滤。
/// 实体缺失跳过（登录前无 LocalPlayer，HudData 保持默认，与组件默认值一致）。
fn sync_hud_data(
    mut roots: Query<&mut HudData>,
    player: Query<(&Vitals, &Progression, &Gold, &PlayerName), With<LocalPlayer>>,
) {
    let Ok(mut data) = roots.single_mut() else { return };
    let Ok((vitals, progression, gold, player_name)) = player.single() else { return };
    let new = HudData {
        hp: vitals.hp,
        max_hp: vitals.max_hp,
        mp: vitals.mp,
        max_mp: vitals.max_mp,
        exp: progression.exp,
        max_exp: progression.max_exp,
        level: progression.level,
        gold: gold.0,
        name: player_name.0.clone(),
    };
    if *data != new {
        *data = new;
    }
}

/// #2633 批次4 步3/步7：血/蓝/经验/等级/金币/名字改读 `Vitals`/`Progression`/`Gold`/
/// `PlayerName` 组件（步7 迁 name；hud.name 双写保留，步9 删）。门控不变：仍靠 `Changed<HudData>`
/// 跳帧（#70），R4 读当前值不加组件 Changed 过滤。实体缺失跳过（HudData 默认帧不更新）。
fn hud_update_system(
    opt: Res<crate::game::dialogs::option::OptionState>,
    hud_datas: Query<&HudData, Changed<HudData>>,
    player: Query<(&Vitals, &Progression, &Gold, &PlayerName), With<LocalPlayer>>,
    mut fills: Query<(
        &mut Sprite,
        &mut Transform,
        Option<&OrbBase>,
        Option<&HpHpFill>,
        Option<&MpMpFill>,
        Option<&ExpFill>,
    )>,
    mut texts: Query<(
        &mut Text2d,
        Option<&HpHpText>,
        Option<&MpMpText>,
        Option<&TopHudText>,
        Option<&BottomHudText>,
        Option<&ExpText>,
        Option<&LevelText>,
        Option<&GoldText>,
        Option<&NameText>,
    )>,
) {
    // #70：数据未变化帧直接跳过（Changed<HudData> 门控）
    if hud_datas.single().is_err() {
        return;
    }
    let Ok((vitals, progression, player_gold, player_name)) = player.single() else { return };
    let hp_pct = (vitals.hp as f32 / vitals.max_hp.max(1) as f32).clamp(0.0, 1.0);
    let mp_pct = (vitals.mp as f32 / vitals.max_mp.max(1) as f32).clamp(0.0, 1.0);
    let exp_pct = (progression.exp as f32 / progression.max_exp.max(1) as f32).clamp(0.0, 1.0);

    for (mut sprite, mut tf, orb_base, hp, mp, exp) in &mut fills {
        if hp.is_some() {
            let h = ORB_HEIGHT * hp_pct;
            sprite.rect = Some(Rect::new(0.0, ORB_HEIGHT - h, 50.0, ORB_HEIGHT));
            sprite.custom_size = Some(Vec2::new(50.0, h));
            // 底部对齐：基准 Y（主对话框内血球顶）向下偏移 (ORB_HEIGHT - h)
            if let Some(base) = orb_base {
                tf.translation.y = base.0 - (ORB_HEIGHT - h);
            }
        } else if mp.is_some() {
            let h = ORB_HEIGHT * mp_pct;
            sprite.rect = Some(Rect::new(51.0, ORB_HEIGHT - h, 101.0, ORB_HEIGHT));
            sprite.custom_size = Some(Vec2::new(50.0, h));
            if let Some(base) = orb_base {
                tf.translation.y = base.0 - (ORB_HEIGHT - h);
            }
        } else if let Some(exp_fill) = exp {
            // C# ExperienceBar_BeforeDraw：section.Width = (Size.Width-3)*percent、Height=Size.Height。
            // 用组件存的完整条宽（非当前已收缩 rect），修复"按收缩 rect 连乘 → 经验条越涨越窄"。
            let th = sprite.rect.map(|r| r.max.y - r.min.y).unwrap_or(8.0);
            let w = (exp_fill.0 - 3.0).max(0.0) * exp_pct;
            sprite.rect = Some(Rect::new(0.0, 0.0, w, th));
            sprite.custom_size = Some(Vec2::new(w, th));
        }
    }

    for (mut t, hp, mp, top, bottom, exp, lv, gold, name) in &mut texts {
        // 值变化才更新，避免每帧重排文本（ICU4X 报错 + CPU 开销，#31）
        let new = if hp.is_some() {
            // C# :436-457：HPView=true → HealthLabel="HP cur/max"；
            // false → HealthLabel/ManaLabel 清空，两行 Top/Bottom 标签接管
            if opt.hp_view {
                format!("HP {}/{}", vitals.hp, vitals.max_hp)
            } else {
                String::new()
            }
        } else if mp.is_some() {
            if opt.hp_view {
                // C# "MP {0}/{1} " 带尾随空格（影响居中测量，与 C# 一致）
                format!("MP {}/{} ", vitals.mp, vitals.max_mp)
            } else {
                String::new()
            }
        } else if top.is_some() {
            // C# :452 TopLabel（仅 HPView=false）：" {HP}    {MP} \n---------------"
            if opt.hp_view {
                String::new()
            } else {
                hud_orb_top_text(vitals.hp, vitals.mp)
            }
        } else if bottom.is_some() {
            // C# :453 BottomLabel（仅 HPView=false）：" {maxHP}    {maxMP} "
            if opt.hp_view {
                String::new()
            } else {
                hud_orb_bottom_text(vitals.max_hp, vitals.max_mp)
            }
        } else if exp.is_some() {
            // C# ExperienceLabel = "{0:#0.##%}"（最多两位小数、去尾零）
            format_exp_percent(exp_pct)
        } else if lv.is_some() {
            // C# LevelLabel = User.Level.ToString()（纯数字，"Lv" 由底栏美术自带）
            format!("{}", progression.level)
        } else if gold.is_some() {
            // C# GoldLabel = Gold.ToString("###,###,##0")（千分位）
            format_gold(player_gold.0)
        } else if name.is_some() {
            player_name.0.clone()
        } else {
            continue;
        };
        if t.0 != new {
            t.0 = new;
        }
    }
}

/// C# "{0:#0.##%}"：百分比最多两位小数、去尾零（0.5→"50%"、0.255→"25.5%"、0.1234→"12.34%"）
fn format_exp_percent(frac: f32) -> String {
    let s = format!("{:.2}", frac * 100.0);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{}%", s)
}

/// C# MainDialogs.cs:452 HPView=false TopLabel 两行文本：" {hp}    {mp} \n---------------"
pub fn hud_orb_top_text(hp: i32, mp: i32) -> String {
    format!(" {hp}    {mp} \n---------------")
}

/// C# MainDialogs.cs:453 HPView=false BottomLabel：" {max_hp}    {max_mp} "
pub fn hud_orb_bottom_text(max_hp: i32, max_mp: i32) -> String {
    format!(" {max_hp}    {max_mp} ")
}

/// C# Gold.ToString("###,###,##0")：三位分节千分位（1234567→"1,234,567"）
fn format_gold(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 死亡遮罩显隐 + 复活按钮（#46）
///
/// #2633 批次4 步3：dead/reincarnation_offered 读改 `StatusFlags`；`death_popup_dismissed`
/// 是纯 UI 残留，仍留 `HudState`（步9 才清）。**写仍双写** `StatusFlags` + `hud.dead`/
/// `hud.reincarnation_offered`：auto/world、auto/navigation、auto/combat、player_control
/// 等「旗标读者」本步尚未迁移、仍读 `hud.dead`（设计 §11批3 步4 才迁），若此处只写
/// `StatusFlags`，点「复活」后这些读者会在服务器确认前一直读到陈旧 `hud.dead=true`
/// （输入仍被锁/自动战斗仍判死亡），破坏行为等价；保留双写则所有读者值与之前一致。
/// 实体缺失视同未死亡/无轮回请求（同原 HudState 默认 false），遮罩不显示。
fn death_overlay_system(
    mut hud: ResMut<HudState>,
    net: Res<crate::network::NetConnection>,
    mut flags_q: Query<&mut StatusFlags, With<LocalPlayer>>,
    // 背景/遮罩/文字/是/否按钮全部带 DeathOverlay，统一随死亡显隐
    mut overlay: Query<&mut Visibility, With<DeathOverlay>>,
    mut death_texts: Query<&mut Text2d, (With<DeathText>, Without<DeathReviveBtn>, Without<DeathReincDeclineBtn>)>,
    yes_btns: Query<&UiButton, (With<DeathReviveBtn>, Without<DeathReincDeclineBtn>)>,
    no_btns: Query<&UiButton, (With<DeathReincDeclineBtn>, Without<DeathReviveBtn>)>,
) {
    // C# ShowReviveMessage：死亡后弹一次；点“否”关闭后不再弹（除非再次死亡）
    let (dead, reinc_offered) = flags_q
        .single()
        .map(|f| (f.dead, f.reincarnation_offered))
        .unwrap_or((false, false));
    let show = dead && !hud.death_popup_dismissed;
    for mut vis in overlay.iter_mut() {
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut t in &mut death_texts {
        let new = if reinc_offered {
            "你想要复活吗？"
        } else {
            "你已经死亡，是否要在城镇复活？"
        };
        if t.0 != new {
            t.0 = new.to_string();
        }
    }
    if !show {
        return;
    }
    // 是：轮回术请求 → AcceptReincarnation；否则 TownRevive（C# YesButton 语义）
    for btn in &yes_btns {
        if btn.clicked {
            if reinc_offered {
                net.send_packet(&mir2_shared::packets::client::misc::AcceptReincarnation);
                tracing::info!("🌀 接受轮回术复活");
            } else {
                net.send_packet(&mir2_shared::packets::client::misc::TownRevive);
                tracing::info!("⛪ 点击复活（TownRevive）");
            }
            if let Ok(mut f) = flags_q.single_mut() {
                f.dead = false;
                f.reincarnation_offered = false;
            }
            // 双写：未迁移旗标读者（auto/*、player_control）仍读 hud.dead（见函数注释）
            hud.dead = false;
            hud.reincarnation_offered = false;
            hud.death_popup_dismissed = false;
        }
    }
    // 否：轮回术请求 → CancelReincarnation；关闭弹窗（玩家保持死亡，C# Dispose 语义）
    for btn in &no_btns {
        if btn.clicked {
            if reinc_offered {
                net.send_packet(&mir2_shared::packets::client::misc::CancelReincarnation);
                tracing::info!("🌀 拒绝轮回术复活");
            }
            if let Ok(mut f) = flags_q.single_mut() {
                f.reincarnation_offered = false;
            }
            hud.reincarnation_offered = false;
            hud.death_popup_dismissed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C# GoldLabel = Gold.ToString("###,###,##0")（千分位）
    #[test]
    fn format_gold_thousands() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(999), "999");
        assert_eq!(format_gold(1000), "1,000");
        assert_eq!(format_gold(1234567), "1,234,567");
        assert_eq!(format_gold(12345678), "12,345,678");
    }

    /// C# ExperienceLabel = "{0:#0.##%}"（最多两位小数、去尾零）
    #[test]
    fn format_exp_percent_trims() {
        assert_eq!(format_exp_percent(0.0), "0%");
        assert_eq!(format_exp_percent(0.5), "50%");
        assert_eq!(format_exp_percent(0.255), "25.5%");
        assert_eq!(format_exp_percent(0.1234), "12.34%");
        assert_eq!(format_exp_percent(1.0), "100%");
    }

    /// C# MainDialogs.cs:452-453 两行格式字面值（HPView=false）
    #[test]
    fn orb_two_line_matches_csharp() {
        assert_eq!(hud_orb_top_text(100, 50), " 100    50 \n---------------");
        assert_eq!(hud_orb_bottom_text(200, 100), " 200    100 ");
    }

    /// HPView 分支（C# :436-457）：true → 球标签 HP/MP 行、两行标签空；
    /// false → 球标签空、Top/Bottom 两行 C# 字面值
    #[test]
    fn hud_hp_view_two_line_format() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        fn text_of<M: Component>(world: &mut World) -> String {
            let mut q = world.query_filtered::<&Text2d, With<M>>();
            q.iter(world).next().expect("应有标签").0.clone()
        }

        fn texts(hp_view: bool) -> (String, String, String, String) {
            let mut world = World::new();
            world.insert_resource(GameLibraries(Libraries::new(resolve_data_path())));
            world.insert_resource(Assets::<Image>::default());
            world.insert_resource(UiImageCache::default());
            world.insert_resource(Assets::<Font>::default());
            world.insert_resource(UiFont::default());
            let mut opt = OptionState::default();
            opt.hp_view = hp_view;
            world.insert_resource(opt);
            // #2633 步3/步7：hud_update_system 改读玩家组件（Vitals/Progression/Gold/PlayerName），
            // HudState 仅剩身份字段双写；spawn 本地玩家实体并写组件驱动显示（R9 预演）。
            world.insert_resource(HudState::default());
            world.spawn((
                LocalPlayer,
                Vitals { hp: 100, max_hp: 200, mp: 50, max_mp: 100 },
                Progression::default(),
                Gold(0),
                PlayerName(String::new()),
            ));
            world.insert_resource(MiniMapMode::default());
            world.run_system_once(spawn_hud).expect("spawn_hud 应成功");
            world
                .run_system_once(hud_update_system)
                .expect("hud_update_system 应成功");
            (
                text_of::<HpHpText>(&mut world),
                text_of::<MpMpText>(&mut world),
                text_of::<TopHudText>(&mut world),
                text_of::<BottomHudText>(&mut world),
            )
        }

        let (hp, mp, top, bottom) = texts(true);
        assert_eq!(hp, "HP 100/200");
        assert_eq!(mp, "MP 50/100 ");
        assert_eq!(top, "");
        assert_eq!(bottom, "");
        let (hp, mp, top, bottom) = texts(false);
        assert_eq!(hp, "");
        assert_eq!(mp, "");
        assert_eq!(top, " 100    50 \n---------------");
        assert_eq!(bottom, " 200    100 ");
    }

    /// mode_visibility：INI 门控的纯函数（C# 构造 Visible=Settings.ModeView）
    #[test]
    fn mode_visibility_maps_mode_view() {
        assert_eq!(mode_visibility(false), Visibility::Hidden);
        assert_eq!(mode_visibility(true), Visibility::Visible);
    }

    /// 模式标签可见性门控：C# 构造 `Visible=Settings.ModeView`（仅 INI，默认 false）。
    /// 真实 spawn_hud：mode_view=false → 三标签 Hidden；true → Visible。
    #[test]
    fn mode_labels_gated_by_mode_view() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        fn mode_vis(mode_view: bool) -> [Visibility; 3] {
            let mut world = World::new();
            world.insert_resource(GameLibraries(Libraries::new(resolve_data_path())));
            world.insert_resource(Assets::<Image>::default());
            world.insert_resource(UiImageCache::default());
            world.insert_resource(Assets::<Font>::default());
            world.insert_resource(UiFont::default());
            let mut opt = OptionState::default();
            opt.mode_view = mode_view;
            world.insert_resource(opt);
            world.insert_resource(MiniMapMode::default());
            world.run_system_once(spawn_hud).expect("spawn_hud 应成功");
            let mut sq = world.query_filtered::<&Visibility, With<SModeText>>();
            let s = sq.iter(&world).copied().next().expect("应有 SModeText");
            let mut aq = world.query_filtered::<&Visibility, With<AttackModeText>>();
            let a = aq
                .iter(&world)
                .copied()
                .next()
                .expect("应有 AttackModeText");
            let mut pq = world.query_filtered::<&Visibility, With<PModeText>>();
            let p = pq.iter(&world).copied().next().expect("应有 PModeText");
            [s, a, p]
        }

        assert_eq!(
            mode_vis(false),
            [Visibility::Hidden; 3],
            "默认 mode_view=false 三标签应隐藏（对齐 C# Settings.ModeView 默认 false）"
        );
        assert_eq!(
            mode_vis(true),
            [Visibility::Visible; 3],
            "mode_view=true 三标签应可见"
        );
    }

    /// 模式标签随小地图大/小模式重定位（C# MiniMapDialog.Process :2082-2087 每帧定位）。
    /// X=MiniMap.X-3=895；大模式 y=152/167/182、小模式 y=43/58/73（S/A/P 顶→底），Bevy Transform.y 取负。
    #[test]
    fn mode_labels_follow_minimap_mode() {
        use bevy::ecs::system::RunSystemOnce;

        fn ty(world: &World, e: Entity) -> f32 {
            world.get::<Transform>(e).unwrap().translation.y
        }

        let mut world = World::new();
        world.insert_resource(crate::game::combat::AttackModeState::default());
        world.insert_resource(HudState::default());
        world.insert_resource(OptionState::default());
        world.insert_resource(MiniMapMode::default()); // 默认大模式
        let sm = world
            .spawn((
                SModeText,
                Text2d::new("技能:Ctrl"),
                Transform::from_xyz(MODE_LABEL_X, 0.0, 4.0),
            ))
            .id();
        let am = world
            .spawn((
                AttackModeText,
                Text2d::new("模式:和平"),
                Transform::from_xyz(MODE_LABEL_X, 0.0, 4.0),
            ))
            .id();
        let pm = world
            .spawn((
                PModeText,
                Text2d::new("宠物:跟随"),
                Transform::from_xyz(MODE_LABEL_X, 0.0, 4.0),
            ))
            .id();

        world
            .run_system_once(attack_mode_text_system)
            .expect("系统应成功");
        assert_eq!(ty(&world, sm), -152.0, "大模式 SMode y");
        assert_eq!(ty(&world, am), -167.0, "大模式 AMode y");
        assert_eq!(ty(&world, pm), -182.0, "大模式 PMode y");

        world.resource_mut::<MiniMapMode>().big = false;
        world
            .run_system_once(attack_mode_text_system)
            .expect("系统应成功");
        assert_eq!(ty(&world, sm), -43.0, "小模式 SMode y");
        assert_eq!(ty(&world, am), -58.0, "小模式 AMode y");
        assert_eq!(ty(&world, pm), -73.0, "小模式 PMode y");
    }

    /// 模式标签描边（#2563：C# MainDialogs.cs:356/366/376 仅设 OutLineColour 未关
    /// OutLine → MirLabel 构造默认 OutLine=true = 有描边）：spawn 挂 4 个黑色副本，
    /// 文本更新系统同帧同步副本
    #[test]
    fn mode_labels_have_outline_shadows() {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::ecs::world::CommandQueue;

        // 无子实体的裸标签：系统安全（Option<&Children> = None）
        let mut world = World::new();
        world.insert_resource(crate::game::combat::AttackModeState::default());
        world.insert_resource(HudState::default());
        world.insert_resource(OptionState::default());
        world.insert_resource(MiniMapMode::default());
        let bare = world
            .spawn((
                AttackModeText,
                Text2d::new("模式:和平"),
                Transform::from_xyz(MODE_LABEL_X, 0.0, 4.0),
            ))
            .id();
        world
            .run_system_once(attack_mode_text_system)
            .expect("裸标签系统应成功");

        // spawn 路径：4 个描边副本 + 文本一致
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let e = spawn_mode_label(
            &mut commands,
            &Handle::default(),
            "技能:Ctrl",
            true,
            S_MODE_DY,
            Color::WHITE,
            Visibility::Visible,
            SModeText,
        );
        queue.apply(&mut world);
        let children: Vec<Entity> = world
            .entity(e)
            .get::<Children>()
            .expect("应有描边子实体")
            .iter()
            .collect();
        assert_eq!(children.len(), 4, "4 方向描边副本");
        let mut shadows =
            world.query_filtered::<&Text2d, With<crate::ui::outlined_text::OutlineShadow>>();
        for c in &children {
            let t = shadows.get(&world, *c).expect("子实体应为描边副本");
            assert_eq!(t.0, "技能:Ctrl", "副本文本与正文一致");
        }
        let _ = bare;

        // 文本更新 → 副本同帧同步（attack_mode_text_system 直同步路径：
        // 切 skill_mode_ctrl 使 SMode 文本变化）
        world.resource_mut::<OptionState>().skill_mode_ctrl = false;
        world
            .run_system_once(attack_mode_text_system)
            .expect("更新应成功");
        for c in &children {
            assert_eq!(
                shadows.get(&world, *c).unwrap().0,
                "技能:~",
                "副本同步新文本"
            );
        }
    }
}
