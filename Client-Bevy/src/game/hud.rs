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

use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::{DialogKind, DialogManager};
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
    pub name: String,
    /// 本地玩家 object_id（UserInformation 提供）
    pub player_object_id: Option<u32>,
    /// 角色职业（显示用）
    pub class: u8,
    /// 自动喝药开关（HP < 35% 自动使用背包药品）
    pub auto_pot_hp: bool,
    /// 玩家死亡（Death 包置位，Revived 清除；死亡时禁用输入/显示遮罩）
    pub dead: bool,
    /// 自动喝药冷却（避免连发）
    pub pot_cooldown: f32,
    /// 背包（网络 UserInformation 写入）
    pub inventory: crate::game::dialogs::inventory::InventoryState,
    /// 装备（12 槽）
    pub equipment: Vec<Option<InvItem>>,
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
            name: String::new(),
            player_object_id: None,
            class: 0,
            auto_pot_hp: true,
            dead: false,
            pot_cooldown: 0.0,
            inventory: Default::default(),
            equipment: vec![None; 12],
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
}

#[derive(Component)]
pub struct HudButton(pub HudButtonKind);

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

/// HUD 按钮 → 对话框开关（M9：接入 DialogManager）
fn hud_button_system(mut mgr: ResMut<DialogManager>, buttons: Query<(&UiButton, &HudButton)>) {
    for (btn, kind) in &buttons {
        if btn.clicked {
            tracing::info!("🎛️ HUD 按钮点击: {:?}", kind.0);
            match kind.0 {
                HudButtonKind::Inventory => mgr.toggle(DialogKind::Inventory),
                HudButtonKind::Character => mgr.toggle(DialogKind::Character),
                HudButtonKind::Skills => mgr.toggle(DialogKind::Character),
                HudButtonKind::QuestLog => mgr.toggle(DialogKind::QuestLog),
                HudButtonKind::Option => mgr.toggle(DialogKind::Settings),
                HudButtonKind::Menu => mgr.toggle(DialogKind::Menu),
                HudButtonKind::GameShop => mgr.toggle(DialogKind::GameShop),
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
#[derive(Component)]
struct ExpFill;
#[derive(Component)]
struct HpHpText;
#[derive(Component)]
struct MpMpText;
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
#[derive(Component)]
struct DeathText;

const ORB_HEIGHT: f32 = 80.0;
const ORB_TOP: f32 = 30.0;
const EXP_TOP: f32 = 143.0;
const BUTTON_TOP: f32 = 76.0;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            hud_server_events.run_if(in_state(crate::scenes::AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_hud);
        app.add_systems(OnExit(AppState::Game), cleanup_hud);
        app.add_systems(
            Update,
            (
                ui_button_system,
                hud_button_system,
                auto_potion_system,
                sync_hud_data,
                hud_update_system,
                death_overlay_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
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
) {
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
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        resolution_index,
    ) {
        spawn_ui_sprite(&mut commands, h, main_x, main_y, 1.0, 1.0);
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
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 8) {
        let bar_x = main_x + 9.0;
        let bar_y = main_y + EXP_TOP;
        let (tw, th) = libs
            .0
            .get_image(LibraryName::Prguse, 8)
            .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
            .unwrap_or((100.0, 5.0));
        commands.spawn((
            UiEntity,
            ExpFill,
            Sprite {
                image: h,
                rect: Some(Rect::new(0.0, 0.0, tw, th)),
                custom_size: Some(Vec2::new(tw, th)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(bar_x, -bar_y, 2.0),
            Visibility::default(),
        ));
    }

    // 文本
    let orb_x = main_x;
    let orb_y = main_y + ORB_TOP;
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        HpHpText,
        orb_x + 9.0,
        orb_y + 18.0,
        "",
    );
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        MpMpText,
        orb_x + 60.0,
        orb_y + 18.0,
        "",
    );
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        ExpText,
        main_x + 9.0 + 50.0,
        main_y + EXP_TOP - 2.0,
        "",
    );
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        LevelText,
        main_x + 9.0,
        main_y + 2.0,
        "",
    );
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        GoldText,
        main_x + bg_w - 90.0,
        main_y + 2.0,
        "",
    );
    spawn_text(
        &mut commands,
        &font,
        &mut images,
        &mut cache,
        NameText,
        main_x + 9.0,
        main_y + 14.0,
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

    // 死亡遮罩（#46）：全屏半透明 + 提示 + 复活按钮，默认隐藏
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DeathOverlay,
        Sprite {
            image: white,
            custom_size: Some(Vec2::new(1024.0, 768.0)),
            color: Color::srgba(0.0, 0.0, 0.0, 0.65),
            ..default()
        },
        Transform::from_xyz(512.0, -384.0, 10.0),
        Visibility::Hidden,
    ));
    let death_txt = spawn_ui_text(
        &mut commands,
        &font,
        "你已死亡，点击复活返回安全区",
        262.0,
        330.0,
        20.0,
        Color::srgb(1.0, 0.3, 0.3),
        11.0,
    );
    commands.entity(death_txt).insert((DeathText, DeathOverlay));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        206,
        207,
        208,
        462.0,
        380.0,
        11.0,
        100.0,
        30.0,
    ) {
        commands.entity(e).insert((DeathReviveBtn, DeathOverlay));
    }
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

/// 自动喝药（M10）：HP < 35% 且冷却结束 → 使用背包药品（UseItem）
fn auto_potion_system(
    mut hud: ResMut<HudState>,
    net: Res<crate::network::NetConnection>,
    time: Res<Time>,
) {
    hud.pot_cooldown -= time.delta_secs();
    if hud.dead || !hud.auto_pot_hp || hud.pot_cooldown > 0.0 {
        return;
    }
    let pct = hud.hp as f32 / hud.max_hp.max(1) as f32;
    if pct < 0.35 {
        // 数据驱动：从背包找第一个药品（ItemType::Potion）
        let potion = hud.inventory.items.iter().flatten().find(|it| {
            mir2_shared::enums::ItemType::try_from(it.item_type)
                == Ok(mir2_shared::enums::ItemType::Potion)
        });
        if let Some(potion) = potion {
            net.send_packet(&mir2_shared::packets::client::item::UseItem {
                unique_id: potion.unique_id,
            });
            tracing::info!(
                "💊 自动喝药 {} (uid={})（HP {}/{}）",
                potion.name,
                potion.unique_id,
                hud.hp,
                hud.max_hp
            );
            hud.pot_cooldown = 3.0;
        }
    }
}

/// 每帧按 HudState 更新血/蓝/经验条与文本（单查询避免 Bevy B0001 冲突）
/// #70：HudState（Resource）→ HudData（组件）快照同步，仅在实际值变化时写组件
fn sync_hud_data(mut roots: Query<&mut HudData>, hud: Res<HudState>) {
    let Ok(mut data) = roots.single_mut() else { return };
    let new = HudData {
        hp: hud.hp,
        max_hp: hud.max_hp,
        mp: hud.mp,
        max_mp: hud.max_mp,
        exp: hud.exp,
        max_exp: hud.max_exp,
        level: hud.level,
        gold: hud.gold,
        name: hud.name.clone(),
    };
    if *data != new {
        *data = new;
    }
}

fn hud_update_system(
    hud: Res<HudState>,
    hud_datas: Query<&HudData, Changed<HudData>>,
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
    let hp_pct = (hud.hp as f32 / hud.max_hp.max(1) as f32).clamp(0.0, 1.0);
    let mp_pct = (hud.mp as f32 / hud.max_mp.max(1) as f32).clamp(0.0, 1.0);
    let exp_pct = (hud.exp as f32 / hud.max_exp.max(1) as f32).clamp(0.0, 1.0);

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
        } else if exp.is_some() {
            let (tw, th) = match sprite.rect {
                Some(r) => (r.max.x - r.min.x, r.max.y - r.min.y),
                None => (100.0, 5.0),
            };
            let w = tw * exp_pct;
            sprite.rect = Some(Rect::new(0.0, 0.0, w, th));
            sprite.custom_size = Some(Vec2::new(w, th));
        }
    }

    for (mut t, hp, mp, exp, lv, gold, name) in &mut texts {
        // 值变化才更新，避免每帧重排文本（ICU4X 报错 + CPU 开销，#31）
        let new = if hp.is_some() {
            format!("{}", hud.hp)
        } else if mp.is_some() {
            format!("{}", hud.mp)
        } else if exp.is_some() {
            format!("{:.1}%", exp_pct * 100.0)
        } else if lv.is_some() {
            format!("Lv.{}", hud.level)
        } else if gold.is_some() {
            format!("{}", hud.gold)
        } else if name.is_some() {
            hud.name.clone()
        } else {
            continue;
        };
        if t.0 != new {
            t.0 = new;
        }
    }
}

/// 死亡遮罩显隐 + 复活按钮（#46）
fn death_overlay_system(
    mut hud: ResMut<HudState>,
    net: Res<crate::network::NetConnection>,
    mut overlay: Query<&mut Visibility, With<DeathOverlay>>,
    revive_btns: Query<&UiButton, With<DeathReviveBtn>>,
) {
    let show = hud.dead;
    for mut vis in overlay.iter_mut() {
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !show {
        return;
    }
    for btn in &revive_btns {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::misc::TownRevive);
            tracing::info!("⛪ 点击复活（TownRevive）");
            // 乐观清除，服务端 Revived 会再次确认
            hud.dead = false;
        }
    }
}


/// 消费服务端事件更新 HUD 状态（网络层只发 ServerEvent，不再直接改 HudState）
fn hud_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut hud: ResMut<HudState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::HealthChanged { hp, mp } => {
                hud.hp = *hp;
                hud.mp = *mp;
            }
            ServerEvent::GoldGained { gold } => {
                hud.gold = hud.gold.saturating_add(*gold);
            }
            ServerEvent::ExperienceGained { amount } => {
                hud.exp += *amount;
                tracing::info!("✨ 获得经验 +{}（当前 {}/{}）", amount, hud.exp, hud.max_exp);
            }
            ServerEvent::LevelChanged { level, exp, max_exp } => {
                hud.level = *level;
                hud.exp = *exp;
                hud.max_exp = (*max_exp).max(1);
                tracing::info!("⬆️ 升级 Lv.{} exp={}/{}", level, exp, max_exp);
            }
            ServerEvent::Chat { .. }
            | ServerEvent::NpcDialog { .. }
            | ServerEvent::Roll { .. }
            | ServerEvent::AwakeningMaterials { .. }
            | ServerEvent::AwakeningResult { .. }
            | ServerEvent::StorageOpened { .. }
            | ServerEvent::GuildInGuild { .. }
            | ServerEvent::GuildData { .. }
            | ServerEvent::GuildStorage { .. }
            | ServerEvent::GroupMembers { .. }
            | ServerEvent::GroupInvite { .. }
            | ServerEvent::GroupDeleted
            | ServerEvent::GroupMemberLeft { .. }
            | ServerEvent::MentorInvite { .. }
            | ServerEvent::MentorUpdate { .. }
            | ServerEvent::FriendUpdated { .. }
            | ServerEvent::Rankings { .. }
            | ServerEvent::GuildNotice { .. }
            | ServerEvent::QuestChanged { .. }
            | ServerEvent::QuestCompleted { .. }
            | ServerEvent::BuffAdded { .. }
            | ServerEvent::BuffRemoved { .. }
            | ServerEvent::InspectPlayer { .. }
            | ServerEvent::CreatureList { .. }
            | ServerEvent::HeroChanged { .. }
            | ServerEvent::MarriageInvite { .. }
            | ServerEvent::MarriageStatus { .. }
            | ServerEvent::DivorceRequest
            | ServerEvent::RentalRequestReceived
            | ServerEvent::RentalItemUpdate { .. }
            | ServerEvent::RentalFee { .. }
            | ServerEvent::RentalPeriod { .. }
            | ServerEvent::RentalDeposit { .. }
            | ServerEvent::RentalRetrieve { .. }
            | ServerEvent::RentalLocked
            | ServerEvent::RentalPartnerLocked
            | ServerEvent::RentalCanConfirm { .. }
            | ServerEvent::RentalConfirmed { .. }
            | ServerEvent::RentalCancelled
            | ServerEvent::MarketPages { .. }
            | ServerEvent::MarketListings { .. }
            | ServerEvent::MarketConsign { .. }
            | ServerEvent::MarketSuccess { .. }
            | ServerEvent::MarketFail { .. }
            | ServerEvent::ShopCatalog { .. }
            | ServerEvent::ShopStock { .. }
            | ServerEvent::TerritoryList { .. }
            | ServerEvent::TerritoryWar { .. }
            | ServerEvent::TradeGold { .. }
            | ServerEvent::TradeCancelled
            | ServerEvent::FishingUpdate { .. }
            | ServerEvent::MailReceived { .. }
            | ServerEvent::TradeRequested { .. }
            | ServerEvent::TradeConfirm { .. }
            | ServerEvent::TradeItemUpdate { .. }
            | ServerEvent::TradeDeposit { .. }
            | ServerEvent::GuildMemberChanged { .. }
            | ServerEvent::GuildInvited { .. }
            | ServerEvent::RankingsCleared
            | ServerEvent::WeatherChanged { .. }
            | ServerEvent::MapInfo { .. }
            | ServerEvent::MagicLearned { .. }
            | ServerEvent::CraftResult { .. }
            | ServerEvent::NpcGoods { .. }
            | ServerEvent::NpcSellPanel { .. } => {}
            ServerEvent::InventoryMoved { from, to } => {
                if *from < hud.inventory.items.len() && *to < hud.inventory.items.len() {
                    hud.inventory.items.swap(*from, *to);
                }
            }
            ServerEvent::ItemEquipped { unique_id, to } => {
                // 从背包移除并放入装备槽；旧装备放回背包空格
                let from_idx = hud
                    .inventory
                    .items
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.unique_id) == Some(*unique_id));
                if let Some(from_idx) = from_idx {
                    let item = hud.inventory.items[from_idx].take();
                    if let Some(item) = item {
                        if *to < hud.equipment.len() {
                            let old = hud.equipment[*to].take();
                            hud.equipment[*to] = Some(item);
                            if let Some(old) = old {
                                if let Some(empty) =
                                    hud.inventory.items.iter_mut().find(|s| s.is_none())
                                {
                                    *empty = Some(old);
                                }
                            }
                        }
                    }
                }
            }
            ServerEvent::ItemRemoved { unique_id } => {
                // 卸下装备：清空装备槽并放回背包空格
                let mut item = None;
                for slot in hud.equipment.iter_mut() {
                    if slot.as_ref().map(|it| it.unique_id) == Some(*unique_id) {
                        item = slot.take();
                        break;
                    }
                }
                if let Some(item) = item {
                    if let Some(empty) = hud.inventory.items.iter_mut().find(|s| s.is_none()) {
                        *empty = Some(item);
                    }
                }
            }
            ServerEvent::UserInformation {
                name,
                level,
                hp,
                mp,
                exp,
                max_exp,
                gold,
                class,
                object_id,
                inventory,
                equipment,
                ..
            } => {
                hud.name = name.clone();
                hud.level = *level;
                hud.hp = *hp;
                hud.mp = *mp;
                hud.exp = *exp;
                hud.max_exp = (*max_exp).max(1);
                hud.gold = *gold;
                hud.class = *class;
                hud.player_object_id = Some(*object_id);
                hud.inventory.items = inventory.clone();
                hud.inventory.gold = *gold;
                hud.equipment = equipment.clone();
            }
            ServerEvent::PlayerDied => {
                hud.dead = true;
            }
            ServerEvent::PlayerRevived => {
                hud.dead = false;
            }
            ServerEvent::ItemUsed { unique_id } => {
                let idx = hud
                    .inventory
                    .items
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.unique_id) == Some(*unique_id));
                if let Some(idx) = idx {
                    let count = hud.inventory.items[idx].as_ref().map(|it| it.count).unwrap_or(0);
                    if count > 1 {
                        if let Some(it) = hud.inventory.items[idx].as_mut() {
                            it.count -= 1;
                        }
                    } else {
                        hud.inventory.items[idx] = None;
                    }
                    tracing::info!("💊 使用物品 uid={} 剩余 {}", unique_id, count.saturating_sub(1));
                }
            }
        }
    }
}
