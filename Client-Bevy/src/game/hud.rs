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
use crate::game::dialogs::keyboard_layout::{key_name, KeyboardState};
use crate::game::dialogs::minimap::MiniMapMode;
use crate::game::dialogs::option::OptionState;
use crate::game::dialogs::{DialogKind, DialogManager};
use crate::game::player_state::{
    AutoPotion, Gold, Inventory, PetModeState, Progression, StatusFlags, Vitals,
};
use crate::game::sets::GameSet;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiButton;
use crate::ui::sprite_ui::{
    shared_cjk_font, spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiCjkFont,
    UiEntity, UiFont, UiImageCache,
};

/// #2633 批次4 步9：死亡弹窗 UI 态（原 `HudState.death_popup_dismissed`，设计 §1/§8 纯 UI 残余）。
/// 语义不变：死亡/复活时重置 false、死亡弹窗点"否"置 true（C# ShowReviveMessage 只弹一次）。
/// 由 player_status_events（PlayerDied/PlayerRevived 重置）与 death_overlay_system（点否置位）读写，
/// 与玩家状态无关，故保留为 UI 资源而非玩家组件。
#[derive(Resource, Default)]
pub struct DeathDialogState {
    pub dismissed: bool,
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

// ---------------------------------------------------------------------------
// #2892 批C：C# `HeroInfoPanel`（`HeroDialogs.cs:464-700`）子控件
//   · 面板 `Prguse[14]` 135x78 @(95,48)
//   · 头像 `Prguse[1400]` 52x45 @(14,19)；危险 `1750`（血量≤20% 每 400ms 闪烁）/ 死亡 `1379`
//   · 名字容器 `Prguse[10]` 104x31 @(26,60)：等级 (3,-1) 17x14 居中、名字 (2,14) 97x14 居中
//   · 血量容器 `Prguse[11]` 72x45 @(57,26)：三条 52x8 条 `Prguse[1951/1952/1953]` @(18,6/19/32)
//   · 文本：HP @(71,28) 55x18、MP @(71,41) 55x18、EXP @(71,54) 65x18
// ---------------------------------------------------------------------------
pub const HERO_PANEL_ORIGIN: (f32, f32) = (95.0, 48.0);
pub const HERO_PANEL_SIZE: (f32, f32) = (135.0, 78.0);
pub const HERO_AVATAR_POS: (f32, f32) = (14.0, 19.0);
pub const HERO_NAME_BOX_POS: (f32, f32) = (26.0, 60.0);
pub const HERO_LEVEL_POS: (f32, f32) = (3.0, -1.0);
pub const HERO_LEVEL_SIZE: (f32, f32) = (17.0, 14.0);
pub const HERO_NAME_POS: (f32, f32) = (2.0, 14.0);
pub const HERO_NAME_SIZE: (f32, f32) = (97.0, 14.0);
pub const HERO_HEALTH_BOX_POS: (f32, f32) = (57.0, 26.0);
/// 三条百分比条的相对位置（HP/MP/EXP，容器内）
pub const HERO_BAR_POS: [(f32, f32); 3] = [(18.0, 6.0), (18.0, 19.0), (18.0, 32.0)];
pub const HERO_BAR_SIZE: (f32, f32) = (52.0, 8.0);
pub const HERO_HP_LABEL_POS: (f32, f32) = (71.0, 28.0);
pub const HERO_MP_LABEL_POS: (f32, f32) = (71.0, 41.0);
pub const HERO_EXP_LABEL_POS: (f32, f32) = (71.0, 54.0);
/// C# `Avatar_BeforeDraw`：血量 ≤ 20% 时危险头像每 400ms 闪一次
pub const HERO_AVATAR_BLINK_SECS: f32 = 0.4;

#[derive(Component)]
pub struct HeroPanelAvatarBase;
#[derive(Component)]
pub struct HeroPanelAvatarDanger;
#[derive(Component)]
pub struct HeroPanelAvatarDead;
/// 百分比条（0=HP 1=MP 2=EXP）；`full_w` = C# `Size.Width`（52）
#[derive(Component)]
pub struct HeroPanelBar {
    pub kind: usize,
    pub full_w: f32,
}
#[derive(Component)]
pub struct HeroPanelLevel;
#[derive(Component)]
pub struct HeroPanelName;
#[derive(Component)]
pub struct HeroPanelHp;
#[derive(Component)]
pub struct HeroPanelMp;
#[derive(Component)]
pub struct HeroPanelExp;

/// 英雄面板子树标记（一次查询统一控显隐，避免多查询争用 `Visibility`）
#[derive(Component)]
pub struct HeroPanelChild;

/// C# `HealthBar_/ManaBar_/ExperienceBar_BeforeDraw`：`sectionWidth = (int)(Size.Width * percent)`
pub fn hero_bar_width(full_w: f32, percent: f32) -> f32 {
    (full_w * percent.clamp(0.0, 1.0)).floor()
}

/// C# `Hplabel.Text = HP + "/" + Stats[Stat.HP]`（MP 同）
pub fn hero_vital_text(value: i32, max: i32) -> String {
    format!("{}/{}", value.max(0), max.max(0))
}

/// C# `ExLabel.Text = string.Format("{0:F2}%", Experience / MaxExperience * 100)`
pub fn hero_exp_text(exp: i64, max_exp: i64) -> String {
    let pct = if max_exp > 0 {
        exp as f64 / max_exp as f64 * 100.0
    } else {
        0.0
    };
    format!("{pct:.2}%")
}

/// #2892 批C：C# `HeroBehaviourPanel`（`HeroDialogs.cs:751-793`）——
/// `Size = 64x17`、`DrawImage = false`、`Location = MainDialog + (165,37)`；
/// 4 个 16x17 图标 `Prguse[1840..1843]`，**当前行为**显示 `Prguse[1844..1847]` 禁用帧。
pub const HERO_BEHAVIOUR_ORIGIN: (f32, f32) = (165.0, 37.0);
pub const HERO_BEHAVIOUR_ICON: (f32, f32) = (16.0, 17.0);
pub const HERO_BEHAVIOUR_ICON_BASE: usize = 1840;
pub const HERO_BEHAVIOUR_DISABLED_BASE: usize = 1844;

/// 行为按钮（C# `HeroBehaviourPanel.BehaviourButtons[i]`）
#[derive(Component)]
pub struct HeroBehaviourBtn {
    /// 行为序号 0..3（C# `HeroBehaviour` 枚举值）
    pub index: usize,
    /// 可用帧 `Prguse[1840+i]`
    pub normal: Handle<Image>,
    /// 禁用帧 `Prguse[1844+i]`（C# `DisabledIndex`）
    pub disabled: Handle<Image>,
}

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

/// #2892 批C：C# `HeroInfoPanel`（`HeroDialogs.cs:464-700`）——
/// 显隐、头像三态（`Avatar_BeforeDraw`）、三条百分比条（`*_BeforeDraw`）、HP/MP/EXP 文本。
#[allow(clippy::type_complexity)]
fn hero_panel_system(
    hero: Res<crate::game::dialogs::hero::HeroState>,
    time: Res<Time>,
    mut widgets: Query<
        (
            &mut Visibility,
            Option<&HeroPanel>,
            Option<&HeroPanelAvatarBase>,
            Option<&HeroPanelAvatarDanger>,
            Option<&HeroPanelAvatarDead>,
            Option<&HeroPanelBar>,
        ),
        With<HeroPanelChild>,
    >,
    mut bars: Query<(&HeroPanelBar, &mut Sprite)>,
    mut texts: Query<
        (
            &mut Text2d,
            Option<&HeroPanelLevel>,
            Option<&HeroPanelName>,
            Option<&HeroPanelHp>,
            Option<&HeroPanelMp>,
            Option<&HeroPanelExp>,
        ),
        With<HeroPanelChild>,
    >,
    mut blink: Local<(f32, bool)>,
) {
    let show = hero.current.is_some();
    if !show {
        *blink = (0.0, false);
    }
    let dead = hero.spawn_state == mir2_shared::enums::HeroSpawnState::Dead;
    let hp_pct = if hero.hero_max_hp > 0 {
        hero.hero_hp as f32 / hero.hero_max_hp as f32
    } else {
        0.0
    };
    // C# `Avatar_BeforeDraw`：≤20% 时每 400ms 闪一次危险头像；>20% 恒隐藏；死亡显死亡头像
    let danger_on = if show && !dead && hp_pct <= 0.2 {
        blink.0 += time.delta_secs();
        if blink.0 >= HERO_AVATAR_BLINK_SECS {
            blink.0 -= HERO_AVATAR_BLINK_SECS;
            blink.1 = !blink.1;
        }
        blink.1
    } else {
        blink.1 = false;
        false
    };
    for (mut vis, panel, base, danger, dead_avatar, bar) in &mut widgets {
        let want = if !show {
            Visibility::Hidden
        } else if panel.is_some() || base.is_some() || bar.is_some() {
            Visibility::Visible
        } else if danger.is_some() {
            if danger_on {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        } else if dead_avatar.is_some() {
            if dead {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        } else {
            Visibility::Visible
        };
        if *vis != want {
            *vis = want;
        }
    }
    if !show {
        return;
    }
    // 百分比条（`sectionWidth = (int)(Size.Width * percent)`；percent<=0 时 C# 直接 return）
    let mp_pct = if hero.hero_max_mp > 0 {
        hero.hero_mp as f32 / hero.hero_max_mp as f32
    } else {
        0.0
    };
    let exp_pct = if hero.hero_max_exp > 0 {
        hero.hero_exp as f32 / hero.hero_max_exp as f32
    } else {
        0.0
    };
    for (bar, mut sprite) in &mut bars {
        let pct = match bar.kind {
            0 => hp_pct,
            1 => mp_pct,
            _ => exp_pct,
        };
        let w = hero_bar_width(bar.full_w, pct);
        sprite.rect = Some(Rect::new(0.0, 0.0, w, HERO_BAR_SIZE.1));
        sprite.custom_size = Some(Vec2::new(w, HERO_BAR_SIZE.1));
    }
    // 文本（C# `LevelLabel` / `NameLabel` / `Hplabel` / `Mplabel` / `ExLabel`）
    let cur = hero.current.as_ref().expect("show 已判定有英雄");
    for (mut text, level, name, hp, mp, exp) in &mut texts {
        let want = if level.is_some() {
            cur.level.to_string()
        } else if name.is_some() {
            cur.name.clone()
        } else if hp.is_some() {
            hero_vital_text(hero.hero_hp, hero.hero_max_hp)
        } else if mp.is_some() {
            hero_vital_text(hero.hero_mp, hero.hero_max_mp)
        } else if exp.is_some() {
            hero_exp_text(hero.hero_exp, hero.hero_max_exp)
        } else {
            continue;
        };
        if text.0 != want {
            text.0 = want;
        }
    }
}

/// #1331：英雄按钮显隐（C# HeroMenuButton.Visible = 有英雄）
fn hero_btn_system(
    hero: Res<crate::game::dialogs::hero::HeroState>,
    mut btns: Query<&mut Visibility, With<HeroBtn>>,
) {
    let show = hero.current.is_some();
    for mut v in &mut btns {
        *v = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// #2892 批C：英雄行为条（C# `HeroBehaviourPanel`）——
/// 显隐按出战状态（`Visible = p.State > Unsummoned`，`GameScene.cs:6190`）、
/// 当前行为显禁用帧（`UpdateBehaviour`：`Enabled = (byte)behaviour != i`）、
/// 点击发 `C.SetHeroBehaviour`（`SetBehaviour`，`:784-787`）。
fn hero_behaviour_system(
    hero: Res<crate::game::dialogs::hero::HeroState>,
    net: Res<NetConnection>,
    mut btns: Query<(
        &HeroBehaviourBtn,
        &UiButton,
        &mut crate::ui::sprite_ui::ButtonFrames,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    use mir2_shared::enums::HeroBehaviour;
    let summoned = hero.spawn_state as u8 > mir2_shared::enums::HeroSpawnState::Unsummoned as u8;
    let show = hero.current.is_some() && summoned;
    let current = hero.behaviour as u8 as usize;
    for (btn, ui, mut frames, mut sprite, mut vis) in &mut btns {
        let want_vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want_vis {
            *vis = want_vis;
        }
        // 帧：当前行为 → 禁用帧（C# `DisabledIndex`）
        let want = if btn.index == current {
            btn.disabled.clone()
        } else {
            btn.normal.clone()
        };
        if frames.normal != want {
            frames.normal = want.clone();
            frames.hover = want.clone();
            frames.pressed = want.clone();
            sprite.image = want;
        }
        // 点击：当前行为按钮在 C# 是 `Enabled = false`，不响应
        if ui.clicked && btn.index != current {
            if let Ok(behaviour) = HeroBehaviour::try_from(btn.index as u8) {
                net.send_packet(&mir2_shared::packets::client::hero::SetHeroBehaviour {
                    behaviour,
                });
                tracing::info!("🧝 设置英雄行为: {:?}", behaviour);
            }
        }
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((cam, gtf)) = ui_cameras.single() else {
        return;
    };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else {
        return;
    };
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
fn hud_button_system(
    mut mgr: ResMut<DialogManager>,
    mut page: ResMut<CharPage>,
    buttons: Query<(&UiButton, &HudButton)>,
) {
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
        // 均入 GameSet::PlayerState（.before(Hud)，维持「写方在读方前」）；步9 双写删除。
        app.init_resource::<DeathDialogState>();
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
                // #2892 批C：英雄行为条（C# HeroBehaviourPanel）
                hero_behaviour_system,
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
    mut cjk_font: ResMut<UiCjkFont>,
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
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

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
        commands
            .entity(e)
            .insert((HeroPanel, HeroPanelChild, Visibility::Hidden));
    }
    // #2892 批C：面板子控件按 C# `HeroInfoPanel`（`HeroDialogs.cs:464-700`）
    let panel_x = main_x + HERO_PANEL_ORIGIN.0;
    let panel_y = main_y + HERO_PANEL_ORIGIN.1;
    // 头像三态（基 `Prguse[1400]` / 危险 `1750` / 死亡 `1379`，同坐标 (14,19)）
    for (idx, kind) in [(1400usize, 0u8), (1750, 1), (1379, 2)] {
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, idx) {
            let e = spawn_ui_sprite(
                &mut commands,
                h,
                panel_x + HERO_AVATAR_POS.0,
                panel_y + HERO_AVATAR_POS.1,
                3.1,
                1.0,
            );
            let mut ec = commands.entity(e);
            ec.insert((HeroPanelChild, Visibility::Hidden));
            match kind {
                0 => ec.insert(HeroPanelAvatarBase),
                1 => ec.insert(HeroPanelAvatarDanger),
                _ => ec.insert(HeroPanelAvatarDead),
            };
        }
    }
    // 名字容器 `Prguse[10]` @(26,60)
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 10) {
        let e = spawn_ui_sprite(
            &mut commands,
            h,
            panel_x + HERO_NAME_BOX_POS.0,
            panel_y + HERO_NAME_BOX_POS.1,
            3.1,
            1.0,
        );
        commands
            .entity(e)
            .insert((HeroPanelChild, Visibility::Hidden));
    }
    // 等级 / 名字（C# `TextFormatFlags.HorizontalCenter` → `Anchor::TOP_CENTER`，
    // x = 容器左边 + 框内偏移 + 框宽/2）
    let level_x = panel_x + HERO_NAME_BOX_POS.0 + HERO_LEVEL_POS.0 + HERO_LEVEL_SIZE.0 / 2.0;
    let level_y = panel_y + HERO_NAME_BOX_POS.1 + HERO_LEVEL_POS.1;
    let e = crate::ui::sprite_ui::spawn_ui_text_anchored(
        &mut commands,
        &font,
        "",
        Anchor::TOP_CENTER,
        level_x,
        level_y,
        10.0,
        Color::WHITE,
        3.2,
    );
    commands
        .entity(e)
        .insert((HeroPanelLevel, HeroPanelChild, Visibility::Hidden));
    let name_x = panel_x + HERO_NAME_BOX_POS.0 + HERO_NAME_POS.0 + HERO_NAME_SIZE.0 / 2.0;
    let name_y = panel_y + HERO_NAME_BOX_POS.1 + HERO_NAME_POS.1;
    let e = crate::ui::sprite_ui::spawn_ui_text_anchored(
        &mut commands,
        &font,
        "",
        Anchor::TOP_CENTER,
        name_x,
        name_y,
        10.0,
        Color::WHITE,
        3.2,
    );
    commands
        .entity(e)
        .insert((HeroPanelName, HeroPanelChild, Visibility::Hidden));
    // 血量容器 `Prguse[11]` @(57,26) + 三条 52x8 百分比条（`Prguse[1951..1953]`）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 11) {
        let e = spawn_ui_sprite(
            &mut commands,
            h,
            panel_x + HERO_HEALTH_BOX_POS.0,
            panel_y + HERO_HEALTH_BOX_POS.1,
            3.1,
            1.0,
        );
        commands
            .entity(e)
            .insert((HeroPanelChild, Visibility::Hidden));
    }
    for i in 0..3usize {
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            1951 + i,
        ) {
            let e = spawn_ui_sprite(
                &mut commands,
                h,
                panel_x + HERO_HEALTH_BOX_POS.0 + HERO_BAR_POS[i].0,
                panel_y + HERO_HEALTH_BOX_POS.1 + HERO_BAR_POS[i].1,
                3.2,
                1.0,
            );
            commands.entity(e).insert((
                HeroPanelBar {
                    kind: i,
                    full_w: HERO_BAR_SIZE.0,
                },
                HeroPanelChild,
                Visibility::Hidden,
            ));
        }
    }
    // HP / MP / EXP 文本（C# `Hplabel`/`Mplabel`/`ExLabel`）
    for (kind, pos) in [
        (0u8, HERO_HP_LABEL_POS),
        (1, HERO_MP_LABEL_POS),
        (2, HERO_EXP_LABEL_POS),
    ] {
        let e = spawn_ui_text(
            &mut commands,
            &font,
            "",
            panel_x + pos.0,
            panel_y + pos.1,
            10.0,
            Color::WHITE,
            3.3,
        );
        // #2817：C# `HeroInfoPanel` 的文本都是 `MirLabel` → 带描边
        crate::ui::outlined_text::outline_on(
            &mut commands,
            e,
            "",
            font.clone(),
            10.0,
            Anchor::TOP_LEFT,
            false,
        );
        match kind {
            0 => commands
                .entity(e)
                .insert((HeroPanelHp, HeroPanelChild, Visibility::Hidden)),
            1 => commands
                .entity(e)
                .insert((HeroPanelMp, HeroPanelChild, Visibility::Hidden)),
            _ => commands
                .entity(e)
                .insert((HeroPanelExp, HeroPanelChild, Visibility::Hidden)),
        };
    }

    // #2892 批C：英雄行为条（C# `HeroBehaviourPanel`，`HeroDialogs.cs:751-793`）——
    // 4 个 16x17 图标 @ x=0/16/32/48、y=HUD+37；可用帧 `Prguse[1840..1843]`、
    // 当前行为显禁用帧 `Prguse[1844..1847]`（C# `DisabledIndex` + `Enabled=false`）
    for i in 0..4usize {
        let Some(normal) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            HERO_BEHAVIOUR_ICON_BASE + i,
        ) else {
            continue;
        };
        let Some(disabled) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            HERO_BEHAVIOUR_DISABLED_BASE + i,
        ) else {
            continue;
        };
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            HERO_BEHAVIOUR_ICON_BASE + i,
            HERO_BEHAVIOUR_ICON_BASE + i,
            HERO_BEHAVIOUR_ICON_BASE + i,
            main_x + HERO_BEHAVIOUR_ORIGIN.0 + i as f32 * HERO_BEHAVIOUR_ICON.0,
            main_y + HERO_BEHAVIOUR_ORIGIN.1,
            3.0,
            HERO_BEHAVIOUR_ICON.0,
            HERO_BEHAVIOUR_ICON.1,
        ) {
            commands.entity(e).insert((
                HeroBehaviourBtn {
                    index: i,
                    normal,
                    disabled,
                },
                Visibility::Hidden,
                // C# `HeroDialogs.cs:774`：Hint = `HeroBehaviourFormat`（英雄行为：{0}）
                crate::ui::tooltip::TooltipHint(crate::game::dialogs::hero::behaviour_hint(i)),
            ));
        }
    }

    // 死亡弹窗（对齐 C# GameScene.ShowReviveMessage → MirMessageBox(YesNo)）：
    // Prguse[360] 居中 (284,289)，文案 DiedTip，是/否按钮 Title[206-208]/[210-212]
    // （按钮纹理自带“是/否”文字，不再额外绘制文字）；轮回术请求时复用同一弹窗。
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
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
        commands
            .entity(e)
            .insert((DeathOverlay, Visibility::Hidden));
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
    commands
        .entity(death_txt)
        .insert((DeathText, DeathOverlay, Visibility::Hidden));
    // #2817：死亡提示是 C# `MirMessageBox` 的 `MessageLabel`（`MirLabel`）→ 带描边
    crate::ui::outlined_text::outline_on(
        &mut commands,
        death_txt,
        "你已经死亡，是否要在城镇复活？",
        font.clone(),
        16.0,
        Anchor::TOP_LEFT,
        false,
    );
    // 是（TownRevive / 轮回术接受）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        206,
        207,
        208,
        544.0,
        446.0,
        11.0,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((DeathReviveBtn, DeathOverlay, Visibility::Hidden));
    }
    // 否（关闭弹窗 / 轮回术拒绝）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        210,
        211,
        212,
        644.0,
        446.0,
        11.0,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((DeathReincDeclineBtn, DeathOverlay, Visibility::Hidden));
    }

    // 模式标签（C# AMode/PMode/SModeLabel）：右上小地图正下方，顶→底 S/A/P。
    // X = MiniMap.X-3 = 895；y 随小地图大/小模式（C# Process 每帧重定位，attack_mode_text_system 跟随）。
    // 颜色对齐 C# 命名色：AMode=Yellow、SMode=LimeGreen、PMode=Orange。
    // 仅当 Settings.ModeView（仅 INI，无游戏内开关）为 true 时可见（C# 构造 Visible=Settings.ModeView）。
    let mode_vis = mode_visibility(opt.mode_view);
    let big = mmap.big;
    spawn_mode_label(
        &mut commands,
        &cjk,
        "技能:Ctrl",
        big,
        S_MODE_DY,
        Color::srgb(0.196, 0.804, 0.196),
        mode_vis,
        SModeText,
    );
    spawn_mode_label(
        &mut commands,
        &cjk,
        "模式:和平",
        big,
        A_MODE_DY,
        Color::srgb(1.0, 1.0, 0.0),
        mode_vis,
        AttackModeText,
    );
    spawn_mode_label(
        &mut commands,
        &cjk,
        "宠物:跟随",
        big,
        P_MODE_DY,
        Color::srgb(1.0, 0.647, 0.0),
        mode_vis,
        PModeText,
    );
    // #1392：负重/空格（C# WeightLabel/SpaceLabel @(Width-105/Width-30, 101)）
    let wt = spawn_ui_text(
        &mut commands,
        &font,
        "0/0",
        main_x + bg_w - 105.0,
        main_y + 101.0,
        11.0,
        Color::WHITE,
        4.0,
    );
    commands.entity(wt).insert(HudWeightText);
    // #2817：C# `WeightLabel`/`SpaceLabel` 都是 `MirLabel` → 带描边
    crate::ui::outlined_text::outline_on(
        &mut commands,
        wt,
        "0/0",
        font.clone(),
        11.0,
        Anchor::TOP_LEFT,
        false,
    );
    let sp = spawn_ui_text(
        &mut commands,
        &font,
        "0",
        main_x + bg_w - 30.0,
        main_y + 101.0,
        11.0,
        Color::WHITE,
        4.0,
    );
    commands.entity(sp).insert(HudSpaceText);
    crate::ui::outlined_text::outline_on(
        &mut commands,
        sp,
        "0",
        font.clone(),
        11.0,
        Anchor::TOP_LEFT,
        false,
    );
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
    // #2817：C# HUD 标签都是 `MirLabel`（`MainDialogs.cs:24` 的
    // HealthLabel/…/ExperienceLabel/GoldLabel/WeightLabel/SpaceLabel），
    // 构造器默认 `_outLine = true`（`MirLabel.cs:181-182`）→ 需要 4 向黑描边
    crate::ui::outlined_text::outline_on(
        commands,
        e,
        text,
        font.clone(),
        12.0,
        Anchor::TOP_LEFT,
        false,
    );
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
    // #2817：同上（HP/MP 球标签、双行标签、角色名都是 C# `MirLabel`）
    crate::ui::outlined_text::outline_on(commands, e, text, font.clone(), 12.0, anchor, false);
}

/// 自动喝药（M10）：HP < 35% 且冷却结束 → 使用背包药品（UseItem）
///
/// #2633 批次4 步3/9：hp/max_hp→`Vitals`、dead→`StatusFlags`、inventory→`Inventory`、
/// auto_pot_hp/pot_cooldown→`AutoPotion`（enabled/cooldown，本系统是其唯一读者/写者，
/// 设计 §4.5，整体迁离 HudState）。实体缺失跳过——喝药本就需实体在场，缺席不动作。
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
        let potion =
            crate::game::dialogs::inventory::pick_auto_hp_potion(inventory.items.iter().flatten());
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
    let pet_mode = pet
        .single()
        .map(|p| p.0)
        .unwrap_or(mir2_shared::enums::PetMode::Both);
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
    let s = if opt.skill_mode_ctrl {
        "技能:Ctrl".to_string()
    } else {
        "技能:~".to_string()
    };
    let sy = -mode_label_y(mmap.big, S_MODE_DY);
    for (mut t, mut tf, children) in &mut sm {
        update_mode_label(&mut t, &mut tf, children, &mut shadows, &s, sy);
    }
}

/// #2633 批次4 步3/步7：hp/max_hp/mp/max_mp→`Vitals`、exp/max_exp/level→`Progression`、
/// gold→`Gold`、name→复用 `PlayerName`（步7 迁；HudState 已于步9 删除）。
/// R3：保留 `Changed<HudData>` 跳帧门控——仍「值变才写 HudData」（`if *data != new`），
/// 不改成每帧无条件写；R4：一律读组件当前值，不加 `Changed<组件>` 过滤。
/// 实体缺失跳过（登录前无 LocalPlayer，HudData 保持默认，与组件默认值一致）。
fn sync_hud_data(
    mut roots: Query<&mut HudData>,
    player: Query<(&Vitals, &Progression, &Gold, &PlayerName), With<LocalPlayer>>,
) {
    let Ok(mut data) = roots.single_mut() else {
        return;
    };
    let Ok((vitals, progression, gold, player_name)) = player.single() else {
        return;
    };
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
/// `PlayerName` 组件（步7 迁 name；HudState 已于步9 删除）。门控不变：仍靠 `Changed<HudData>`
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
    let Ok((vitals, progression, player_gold, player_name)) = player.single() else {
        return;
    };
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
pub(crate) fn format_gold(n: u32) -> String {
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
/// #2633 批次4 步3/9：dead/reincarnation_offered 读改 `StatusFlags`；`death_popup_dismissed`
/// 迁入 `DeathDialogState.dismissed`（纯 UI 态，设计 §1/§8）。实体缺失视同未死亡/无轮回请求
/// （同原 HudState 默认 false），遮罩不显示。
fn death_overlay_system(
    mut death_ui: ResMut<DeathDialogState>,
    net: Res<crate::network::NetConnection>,
    mut flags_q: Query<&mut StatusFlags, With<LocalPlayer>>,
    // 背景/遮罩/文字/是/否按钮全部带 DeathOverlay，统一随死亡显隐
    mut overlay: Query<&mut Visibility, With<DeathOverlay>>,
    mut death_texts: Query<
        &mut Text2d,
        (
            With<DeathText>,
            Without<DeathReviveBtn>,
            Without<DeathReincDeclineBtn>,
        ),
    >,
    yes_btns: Query<&UiButton, (With<DeathReviveBtn>, Without<DeathReincDeclineBtn>)>,
    no_btns: Query<&UiButton, (With<DeathReincDeclineBtn>, Without<DeathReviveBtn>)>,
) {
    // C# ShowReviveMessage：死亡后弹一次；点“否”关闭后不再弹（除非再次死亡）
    let (dead, reinc_offered) = flags_q
        .single()
        .map(|f| (f.dead, f.reincarnation_offered))
        .unwrap_or((false, false));
    let show = dead && !death_ui.dismissed;
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
            death_ui.dismissed = false;
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
            death_ui.dismissed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2892 批C：C# `HeroBehaviourPanel`（`HeroDialogs.cs:751-793`）几何——
    /// （见下方 `hero_behaviour_geometry_matches_csharp`）
    #[test]
    fn hero_info_panel_geometry_and_formats() {
        assert_eq!(HERO_PANEL_ORIGIN, (95.0, 48.0));
        assert_eq!(HERO_PANEL_SIZE, (135.0, 78.0));
        assert_eq!(HERO_AVATAR_POS, (14.0, 19.0));
        assert_eq!(HERO_NAME_BOX_POS, (26.0, 60.0));
        assert_eq!(
            (HERO_LEVEL_POS, HERO_LEVEL_SIZE),
            ((3.0, -1.0), (17.0, 14.0))
        );
        assert_eq!((HERO_NAME_POS, HERO_NAME_SIZE), ((2.0, 14.0), (97.0, 14.0)));
        assert_eq!(HERO_HEALTH_BOX_POS, (57.0, 26.0));
        assert_eq!(HERO_BAR_POS, [(18.0, 6.0), (18.0, 19.0), (18.0, 32.0)]);
        assert_eq!(HERO_BAR_SIZE, (52.0, 8.0));
        assert_eq!(
            [HERO_HP_LABEL_POS, HERO_MP_LABEL_POS, HERO_EXP_LABEL_POS],
            [(71.0, 28.0), (71.0, 41.0), (71.0, 54.0)]
        );
        // C# `sectionWidth = (int)(Size.Width * percent)`（percent 上限 1）
        assert_eq!(hero_bar_width(52.0, 0.0), 0.0);
        assert_eq!(hero_bar_width(52.0, 0.5), 26.0);
        assert_eq!(hero_bar_width(52.0, 0.2), 10.0);
        assert_eq!(hero_bar_width(52.0, 1.0), 52.0);
        assert_eq!(hero_bar_width(52.0, 1.5), 52.0, "percent 超 1 应截断");
        assert_eq!(hero_bar_width(52.0, -1.0), 0.0);
        // 文本（C# `HP + "/" + Stats[HP]`、`{0:F2}%`）
        assert_eq!(hero_vital_text(300, 600), "300/600");
        assert_eq!(hero_vital_text(-5, 0), "0/0");
        assert_eq!(hero_exp_text(1000, 5000), "20.00%");
        assert_eq!(hero_exp_text(1, 3), "33.33%");
        assert_eq!(hero_exp_text(0, 0), "0.00%");
    }

    /// #2892 批C：`HeroInfoPanel` 系统级——条宽/文本/显隐跟随 `HeroState`
    #[test]
    fn hero_panel_bars_and_labels_follow_hero_state() {
        use crate::game::dialogs::hero::HeroState;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut hero = HeroState::default();
        hero.current = Some(mir2_shared::data::client_data::ClientHeroInformation {
            index: 1,
            name: "英雄甲".to_string(),
            level: 42,
            class: mir2_shared::enums::MirClass::Warrior,
            gender: mir2_shared::enums::MirGender::Male,
        });
        hero.hero_hp = 300;
        hero.hero_max_hp = 600; // 50%
        hero.hero_mp = 0;
        hero.hero_max_mp = 200; // 0%
        hero.hero_exp = 1000;
        hero.hero_max_exp = 5000; // 20%
        app.insert_resource(hero);
        app.add_systems(Update, hero_panel_system);

        let panel = app
            .world_mut()
            .spawn((HeroPanel, HeroPanelChild, Visibility::Hidden))
            .id();
        let mut bars = Vec::new();
        for i in 0..3usize {
            bars.push(
                app.world_mut()
                    .spawn((
                        HeroPanelBar {
                            kind: i,
                            full_w: HERO_BAR_SIZE.0,
                        },
                        HeroPanelChild,
                        Sprite::default(),
                        Visibility::Hidden,
                    ))
                    .id(),
            );
        }
        let level = app
            .world_mut()
            .spawn((
                Text2d::new(""),
                HeroPanelLevel,
                HeroPanelChild,
                Visibility::Hidden,
            ))
            .id();
        let name = app
            .world_mut()
            .spawn((
                Text2d::new(""),
                HeroPanelName,
                HeroPanelChild,
                Visibility::Hidden,
            ))
            .id();
        let hp = app
            .world_mut()
            .spawn((
                Text2d::new(""),
                HeroPanelHp,
                HeroPanelChild,
                Visibility::Hidden,
            ))
            .id();
        let mp = app
            .world_mut()
            .spawn((
                Text2d::new(""),
                HeroPanelMp,
                HeroPanelChild,
                Visibility::Hidden,
            ))
            .id();
        let exp = app
            .world_mut()
            .spawn((
                Text2d::new(""),
                HeroPanelExp,
                HeroPanelChild,
                Visibility::Hidden,
            ))
            .id();

        app.update();
        assert_eq!(
            *app.world().entity(panel).get::<Visibility>().unwrap(),
            Visibility::Visible,
            "有英雄 → 面板可见"
        );
        let width = |app: &App, e: Entity| {
            app.world()
                .entity(e)
                .get::<Sprite>()
                .unwrap()
                .rect
                .map(|r| r.max.x)
                .unwrap_or(0.0)
        };
        assert_eq!(width(&app, bars[0]), 26.0, "HP 50% → 26px");
        assert_eq!(
            width(&app, bars[1]),
            0.0,
            "MP 0% → 0px（C# percent<=0 直接 return）"
        );
        assert_eq!(width(&app, bars[2]), 10.0, "EXP 20% → 10px");
        assert_eq!(app.world().entity(level).get::<Text2d>().unwrap().0, "42");
        assert_eq!(
            app.world().entity(name).get::<Text2d>().unwrap().0,
            "英雄甲"
        );
        assert_eq!(app.world().entity(hp).get::<Text2d>().unwrap().0, "300/600");
        assert_eq!(app.world().entity(mp).get::<Text2d>().unwrap().0, "0/200");
        assert_eq!(app.world().entity(exp).get::<Text2d>().unwrap().0, "20.00%");

        // 无英雄 → 全部隐藏
        app.world_mut()
            .resource_mut::<crate::game::dialogs::hero::HeroState>()
            .current = None;
        app.update();
        for e in [panel, level, name, hp, mp, exp] {
            assert_eq!(
                *app.world().entity(e).get::<Visibility>().unwrap(),
                Visibility::Hidden,
                "无英雄 → 面板与文本都隐藏"
            );
        }
        for e in &bars {
            assert_eq!(
                *app.world().entity(*e).get::<Visibility>().unwrap(),
                Visibility::Hidden,
                "无英雄 → 条也隐藏"
            );
        }
    }

    /// #2892 批C：C# `HeroBehaviourPanel`（`HeroDialogs.cs:751-793`）几何——
    /// 64x17、HUD+(165,37)、4 个 16x17 图标、可用/禁用帧基址 1840/1844
    #[test]
    fn hero_behaviour_geometry_matches_csharp() {
        assert_eq!(HERO_BEHAVIOUR_ORIGIN, (165.0, 37.0));
        assert_eq!(HERO_BEHAVIOUR_ICON, (16.0, 17.0));
        assert_eq!(HERO_BEHAVIOUR_ICON_BASE, 1840);
        assert_eq!(HERO_BEHAVIOUR_DISABLED_BASE, 1844);
        // 面板 `Size = 64x17` = 4 × 16 宽
        assert_eq!(HERO_BEHAVIOUR_ICON.0 * 4.0, 64.0);
    }

    /// #2892 批C：行为条显隐 / 禁用帧 / 点击发包（C# `UpdateBehaviour` + `SetBehaviour`）
    #[test]
    fn hero_behaviour_visibility_frames_and_click() {
        use crate::ui::sprite_ui::ButtonFrames;
        use mir2_shared::enums::{HeroBehaviour, HeroSpawnState};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(NetConnection::default());
        let mut hero = crate::game::dialogs::hero::HeroState::default();
        hero.current = Some(mir2_shared::data::client_data::ClientHeroInformation {
            index: 1,
            name: "英雄".to_string(),
            level: 10,
            class: mir2_shared::enums::MirClass::Warrior,
            gender: mir2_shared::enums::MirGender::Male,
        });
        hero.spawn_state = HeroSpawnState::Summoned;
        hero.behaviour = HeroBehaviour::Follow; // 当前 = 2
        app.insert_resource(hero);
        app.add_systems(Update, hero_behaviour_system);

        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        app.world_mut().resource_mut::<NetConnection>().to_server = Some(tx);
        // 独立的图片句柄：用于断言「当前行为显禁用帧」
        app.insert_resource(Assets::<Image>::default());

        // 4 个按钮（handles 用 default 占位；断言用索引区分）
        let mut buttons = Vec::new();
        for i in 0..4usize {
            let normal = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::default());
            let disabled = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::default());
            let e = app
                .world_mut()
                .spawn((
                    HeroBehaviourBtn {
                        index: i,
                        normal: normal.clone(),
                        disabled: disabled.clone(),
                    },
                    UiButton {
                        rect: (0.0, 0.0, 16.0, 17.0),
                        clicked: false,
                    },
                    ButtonFrames {
                        normal,
                        hover: Handle::default(),
                        pressed: Handle::default(),
                    },
                    Sprite::default(),
                    Visibility::Hidden,
                ))
                .id();
            buttons.push(e);
        }
        app.update();

        // 出战 + 有英雄 → 4 个都可见；当前行为（2）显禁用帧（index → disabled 句柄）
        for (i, e) in buttons.iter().enumerate() {
            let vis = *app.world().entity(*e).get::<Visibility>().unwrap();
            assert_eq!(vis, Visibility::Visible, "出战状态应显示行为条（按钮 {i}）");
            let btn = app.world().entity(*e).get::<HeroBehaviourBtn>().unwrap();
            let frames = app.world().entity(*e).get::<ButtonFrames>().unwrap();
            if i == 2 {
                assert_eq!(
                    frames.normal, btn.disabled,
                    "当前行为（Follow=2）应显禁用帧 Prguse[1844+2]"
                );
            } else {
                assert_eq!(
                    frames.normal, btn.normal,
                    "非当前行为应显可用帧 Prguse[1840+i]"
                );
            }
        }
        app.world_mut().entity_mut(buttons[2]).insert(UiButton {
            rect: (0.0, 0.0, 16.0, 17.0),
            clicked: true,
        });
        app.update();
        assert!(
            rx.try_recv().is_err(),
            "当前行为的按钮在 C# 是 Enabled=false，点击不得发包"
        );

        // 点其它行为 → 发 C.SetHeroBehaviour
        app.world_mut().entity_mut(buttons[1]).insert(UiButton {
            rect: (0.0, 0.0, 16.0, 17.0),
            clicked: true,
        });
        app.update();
        let raw = rx.try_recv().expect("点非当前行为应发 C.SetHeroBehaviour");
        let pkt: mir2_shared::packets::client::hero::SetHeroBehaviour =
            mir2_shared::packets::base::deserialize_packet(&mut std::io::Cursor::new(raw))
                .expect("应为 SetHeroBehaviour 包");
        assert_eq!(pkt.behaviour, HeroBehaviour::CounterAttack);

        // 收回英雄（Unsummoned）→ 全部隐藏
        app.world_mut()
            .resource_mut::<crate::game::dialogs::hero::HeroState>()
            .spawn_state = HeroSpawnState::Unsummoned;
        app.update();
        for e in &buttons {
            assert_eq!(
                *app.world().entity(*e).get::<Visibility>().unwrap(),
                Visibility::Hidden,
                "Unsummoned 时行为条必须隐藏（C# `State > Unsummoned`）"
            );
        }
    }

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
            world.insert_resource(UiCjkFont::default());
            let mut opt = OptionState::default();
            opt.hp_view = hp_view;
            world.insert_resource(opt);
            // #2633 步3/步7/步9：hud_update_system 改读玩家组件（Vitals/Progression/Gold/PlayerName），
            // HudState 已删；spawn 本地玩家实体并写组件驱动显示（R9 预演）。
            world.spawn((
                LocalPlayer,
                Vitals {
                    hp: 100,
                    max_hp: 200,
                    mp: 50,
                    max_mp: 100,
                },
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

    /// #2817 单元③：HUD 标签全部按 C# `MirLabel`（默认 `_outLine = true`）带 4 向黑描边。
    /// 逐标签断言「有 `OutlinedText` + 4 个 `OutlineShadow` 子实体」，防以后漏挂。
    #[test]
    fn hud_labels_are_outlined() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(GameLibraries(Libraries::new(resolve_data_path())));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(UiImageCache::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(UiFont::default());
        world.insert_resource(UiCjkFont::default());
        world.insert_resource(OptionState::default());
        world.insert_resource(MiniMapMode::default());
        world.run_system_once(spawn_hud).expect("spawn_hud 应成功");

        fn assert_outlined<M: Component>(world: &mut World, name: &str) {
            let mut q = world.query_filtered::<(Entity, &Children), With<M>>();
            let (e, children) = q
                .iter(world)
                .next()
                .unwrap_or_else(|| panic!("应有 {name}"));
            let children: Vec<Entity> = children.iter().collect();
            assert!(
                world
                    .entity(e)
                    .contains::<crate::ui::outlined_text::OutlinedText>(),
                "{name} 应带描边主体标记"
            );
            let shadows = children
                .into_iter()
                .filter(|c| {
                    world
                        .entity(*c)
                        .contains::<crate::ui::outlined_text::OutlineShadow>()
                })
                .count();
            assert_eq!(shadows, 4, "{name} 应有 4 个描边副本");
        }

        assert_outlined::<HpHpText>(&mut world, "HealthLabel");
        assert_outlined::<MpMpText>(&mut world, "ManaLabel");
        assert_outlined::<TopHudText>(&mut world, "TopLabel");
        assert_outlined::<BottomHudText>(&mut world, "BottomLabel");
        assert_outlined::<ExpText>(&mut world, "ExperienceLabel");
        assert_outlined::<LevelText>(&mut world, "LevelLabel");
        assert_outlined::<GoldText>(&mut world, "GoldLabel");
        assert_outlined::<NameText>(&mut world, "CharacterName");
        assert_outlined::<HudWeightText>(&mut world, "WeightLabel");
        assert_outlined::<HudSpaceText>(&mut world, "SpaceLabel");
        assert_outlined::<AttackModeText>(&mut world, "AModeLabel");
        assert_outlined::<DeathText>(&mut world, "死亡提示");
    }

    /// #2817 单元③ 负向断言：聊天文本在 C# 里是**唯一**显式 `OutLine = false` 的标签
    /// （`MainDialogs.cs:962/1040`）→ Bevy 侧 `spawn_ui_text` 路径不得带描边。
    #[test]
    fn chat_text_path_stays_unoutlined() {
        use crate::ui::sprite_ui::spawn_ui_text;

        let mut world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let e = spawn_ui_text(
            &mut commands,
            &Handle::default(),
            "聊天行",
            0.0,
            0.0,
            12.0,
            Color::WHITE,
            4.0,
        );
        queue.apply(&mut world);
        assert!(
            !world
                .entity(e)
                .contains::<crate::ui::outlined_text::OutlinedText>(),
            "聊天文本（C# OutLine=false）不应带描边"
        );
        assert!(
            world
                .query_filtered::<Entity, With<crate::ui::outlined_text::OutlineShadow>>()
                .iter(&world)
                .count()
                == 0,
            "聊天文本不应生成描边副本"
        );
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
            world.insert_resource(UiCjkFont::default());
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
