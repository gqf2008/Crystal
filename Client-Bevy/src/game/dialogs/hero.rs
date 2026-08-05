// ============================================================================
// 英雄对话框（M48 + #190 英雄管理 UI）
// 参考：C# HeroDialog + HeroManageDialog + NewHeroDialog
// 网络：
//   S: ManageHeroes（英雄列表）/ NewHero（创建结果）/ ChangeHero（切换）
//   C: ChangeHero[hero_index u8] / NewHero[name, gender, class]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::dialogs::text_input::{TextInputDisplay, TextInputField, TextInputRect};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};

/// 英雄状态
#[derive(Resource)]
pub struct HeroState {
    pub hero_index: u8,
    pub message: String,
    /// 英雄列表（C# S.ManageHeroes）
    pub heroes: Vec<mir2_shared::data::client_data::ClientHeroInformation>,
    /// 当前英雄
    pub current: Option<mir2_shared::data::client_data::ClientHeroInformation>,
    /// 创建面板是否打开
    pub creating: bool,
    /// 创建结果提示
    pub create_msg: String,
    /// 创建面板选中的职业/性别
    pub create_class: mir2_shared::enums::MirClass,
    pub create_gender: mir2_shared::enums::MirGender,
    /// 英雄行为（C# HeroBehaviour：0=攻击 1=反击 2=跟随 3=自定义）
    pub behaviour: mir2_shared::enums::HeroBehaviour,
    /// 英雄自动药阈值（0=关闭；C# HeroInventoryDialog AutoHPPercent）
    pub auto_pot_hp: u8,
    pub auto_pot_mp: u8,
    /// 英雄背包/装备（C# S.HeroInformation，#203）
    pub inventory: Vec<Option<crate::game::dialogs::inventory::InvItem>>,
    pub equipment: Vec<Option<crate::game::dialogs::inventory::InvItem>>,
    pub hero_hp: i32,
    pub hero_mp: i32,
    pub hero_exp: i64,
    pub hero_max_exp: i64,
    pub auto_pot: bool,
    pub hp_item_index: i32,
    pub mp_item_index: i32,
}

impl Default for HeroState {
    fn default() -> Self {
        Self {
            hero_index: 0,
            message: String::new(),
            heroes: Vec::new(),
            current: None,
            creating: false,
            create_msg: String::new(),
            create_class: mir2_shared::enums::MirClass::Warrior,
            create_gender: mir2_shared::enums::MirGender::Male,
            behaviour: mir2_shared::enums::HeroBehaviour::Attack,
            auto_pot_hp: 0,
            auto_pot_mp: 0,
            inventory: Vec::new(),
            equipment: Vec::new(),
            hero_hp: 0,
            hero_mp: 0,
            hero_exp: 0,
            hero_max_exp: 0,
            auto_pot: false,
            hp_item_index: -1,
            mp_item_index: -1,
        }
    }
}

#[derive(Component)]
pub struct HeroWidget;

#[derive(Component)]
pub struct HeroClose;

#[derive(Component)]
pub struct HeroSwitchMain;

#[derive(Component)]
pub struct HeroSwitch1;

/// 打开英雄背包（#203）
#[derive(Component)]
pub struct HeroOpenInventory;

/// 打开英雄装备（#206）
#[derive(Component)]
pub struct HeroOpenEquipment;

#[derive(Component)]
pub struct HeroLine(usize);

/// 创建英雄按钮（C# NewHeroDialog）
#[derive(Component)]
pub struct HeroCreateBtn;

/// 自动药阈值按钮（C# HeroInventoryDialog HPButton/MPButton）
#[derive(Component)]
pub struct HeroAutoHpCycle;
#[derive(Component)]
pub struct HeroAutoMpCycle;
#[derive(Component)]
pub struct HeroAutoPotLabel;

/// 英雄行为按钮（C# HeroBehaviourPanel：Prguse 1840..1843）
#[derive(Component)]
pub struct HeroBehaviourBtn(usize);

/// 创建面板
#[derive(Component)]
pub struct HeroCreatePanel;

/// 职业/性别循环选择按钮
#[derive(Component)]
pub struct HeroClassCycle;

#[derive(Component)]
pub struct HeroGenderCycle;

#[derive(Component)]
pub struct HeroClassLabel;

#[derive(Component)]
pub struct HeroGenderLabel;

#[derive(Component)]
pub struct HeroCreateOk;

#[derive(Component)]
pub struct HeroCreateCancel;

#[derive(Component)]
pub struct HeroCreateMsg;

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroState>();
        app.add_systems(
            Update,
            hero_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_hero);
        app.add_systems(OnExit(AppState::Game), cleanup_hero);
        app.add_systems(
            Update,
            (hero_ui_system, hero_button_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_hero(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_hero(
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
            DialogRoot(DialogKind::Hero),
            HeroWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            HeroClose,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    // 列表行（0..4）
    for i in 0..5usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            HeroLine(i),
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    // 切换主角色 / 英雄 1
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 230.0, 8.3, 90.0, 25.0,
    ) {
        commands.entity(e).insert((
            HeroSwitchMain,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        410.0, 230.0, 8.3, 90.0, 25.0,
    ) {
        commands.entity(e).insert((
            HeroSwitch1,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    // 创建英雄按钮（C# NewHeroDialog）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 262.0, 8.3, 90.0, 25.0,
    ) {
        commands.entity(e).insert((
            HeroCreateBtn,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    let _ = spawn_ui_text(&mut commands, &font, "创建英雄", 314.0, 266.0, 12.0, Color::WHITE, 8.4);
    // 英雄行为（C# HeroBehaviourPanel：Prguse 1840..1843，16x17）
    let _ = spawn_ui_text(&mut commands, &font, "行为:", 410.0, 266.0, 12.0, Color::WHITE, 8.4);
    for i in 0..4usize {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            LibraryName::Prguse, 1840 + i, 1840 + i, 1840 + i,
            440.0 + i as f32 * 18.0, 262.0, 8.3, 16.0, 17.0,
        ) {
            commands.entity(e).insert((
                HeroBehaviourBtn(i),
                DialogRoot(DialogKind::Hero),
                HeroWidget,
            ));
        }
    }

    // 自动药阈值（C# HeroInventoryDialog HPButton/MPButton，Title 560/563）
    let _ = spawn_ui_text(&mut commands, &font, "自动药:", 300.0, 300.0, 12.0, Color::WHITE, 8.4);
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 560, 561, 562,
        360.0, 296.0, 8.3, 60.0, 25.0,
    ) {
        commands.entity(e).insert((
            HeroAutoHpCycle,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 563, 564, 565,
        430.0, 296.0, 8.3, 60.0, 25.0,
    ) {
        commands.entity(e).insert((
            HeroAutoMpCycle,
            DialogRoot(DialogKind::Hero),
            HeroWidget,
        ));
    }
    let ap = spawn_ui_text(&mut commands, &font, "", 364.0, 300.0, 12.0, Color::srgb(1.0, 0.9, 0.4), 8.4);
    commands.entity(ap).insert((HeroAutoPotLabel, DialogRoot(DialogKind::Hero), HeroWidget));
    // 英雄背包按钮（#203：打开 HeroInventoryDialog）
    let inv_btn = spawn_ui_text(&mut commands, &font, "英雄背包", 300.0, 330.0, 12.0, Color::srgb(0.8, 0.9, 1.0), 8.3);
    commands.entity(inv_btn).insert((
        HeroOpenInventory,
        UiButton {
            rect: (300.0, 330.0, 90.0, 18.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Hero),
        HeroWidget,
    ));
    // 英雄装备按钮（#206：打开 HeroEquipmentDialog）
    let eq_btn = spawn_ui_text(&mut commands, &font, "英雄装备", 410.0, 330.0, 12.0, Color::srgb(0.8, 0.9, 1.0), 8.3);
    commands.entity(eq_btn).insert((
        HeroOpenEquipment,
        UiButton {
            rect: (410.0, 330.0, 90.0, 18.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Hero),
        HeroWidget,
    ));
    // 创建面板
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        DialogRoot(DialogKind::Hero),
        HeroCreatePanel,
        Sprite {
            image: white.clone(),
            color: Color::srgba(0.1, 0.1, 0.15, 0.96),
            custom_size: Some(Vec2::new(330.0, 200.0)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(280.0, -296.0, 9.0),
        Visibility::Hidden,
    ));
    let _ = spawn_ui_text(&mut commands, &font, "名字:", 290.0, 312.0, 12.0, Color::WHITE, 9.1);
    let name_box = commands
        .spawn((
            UiEntity,
            DialogRoot(DialogKind::Hero),
            HeroCreatePanel,
            TextInputField(0),
            TextInputRect(330.0, 308.0, 260.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(260.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(330.0, -308.0, 9.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(name_box).with_children(|p| {
        p.spawn((
            TextInputDisplay(0),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(4.0, -2.0, 9.2),
        ));
    });
    // 职业 / 性别 循环选择
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 340.0, 9.3, 130.0, 22.0,
    ) {
        commands.entity(e).insert((
            HeroClassCycle,
            DialogRoot(DialogKind::Hero),
            HeroCreatePanel,
        ));
    }
    let cl = spawn_ui_text(&mut commands, &font, "职业: 战士", 308.0, 344.0, 12.0, Color::WHITE, 9.4);
    commands.entity(cl).insert((HeroClassLabel, DialogRoot(DialogKind::Hero), HeroCreatePanel));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        450.0, 340.0, 9.3, 100.0, 22.0,
    ) {
        commands.entity(e).insert((
            HeroGenderCycle,
            DialogRoot(DialogKind::Hero),
            HeroCreatePanel,
        ));
    }
    let gl = spawn_ui_text(&mut commands, &font, "性别: 男", 458.0, 344.0, 12.0, Color::WHITE, 9.4);
    commands.entity(gl).insert((HeroGenderLabel, DialogRoot(DialogKind::Hero), HeroCreatePanel));
    // 确定 / 取消 / 结果提示
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 376.0, 9.3, 70.0, 23.0,
    ) {
        commands.entity(e).insert((
            HeroCreateOk,
            DialogRoot(DialogKind::Hero),
            HeroCreatePanel,
        ));
    }
    let _ = spawn_ui_text(&mut commands, &font, "确定", 315.0, 380.0, 12.0, Color::WHITE, 9.4);
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 376.0, 9.3, 70.0, 23.0,
    ) {
        commands.entity(e).insert((
            HeroCreateCancel,
            DialogRoot(DialogKind::Hero),
            HeroCreatePanel,
        ));
    }
    let _ = spawn_ui_text(&mut commands, &font, "取消", 405.0, 380.0, 12.0, Color::WHITE, 9.4);
    let msg = spawn_ui_text(&mut commands, &font, "", 300.0, 410.0, 12.0, Color::srgb(1.0, 0.9, 0.4), 9.5);
    commands.entity(msg).insert((HeroCreateMsg, DialogRoot(DialogKind::Hero), HeroCreatePanel));
}

/// 显隐 + 列表渲染（按钮逻辑在 hero_button_system）
#[allow(clippy::too_many_arguments)]
fn hero_ui_system(
    mgr: Res<DialogManager>,
    state: Res<HeroState>,
    mut widgets: Query<&mut Visibility, With<HeroWidget>>,
    mut panel: Query<&mut Visibility, (With<HeroCreatePanel>, Without<HeroWidget>)>,
    mut lines: Query<(&mut Text2d, &HeroLine), (Without<HeroClassLabel>, Without<HeroGenderLabel>, Without<HeroCreateMsg>)>,
    mut class_label: Query<&mut Text2d, (With<HeroClassLabel>, Without<HeroLine>, Without<HeroGenderLabel>, Without<HeroCreateMsg>)>,
    mut gender_label: Query<&mut Text2d, (With<HeroGenderLabel>, Without<HeroLine>, Without<HeroClassLabel>, Without<HeroCreateMsg>)>,
    mut create_msg: Query<&mut Text2d, (With<HeroCreateMsg>, Without<HeroLine>, Without<HeroClassLabel>, Without<HeroGenderLabel>)>,
    mut auto_pot_label: Query<&mut Text2d, (With<HeroAutoPotLabel>, Without<HeroLine>, Without<HeroClassLabel>, Without<HeroGenderLabel>, Without<HeroCreateMsg>)>,
) {
    let open = mgr.is_open(DialogKind::Hero);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for mut vis in &mut panel {
        *vis = if state.creating { Visibility::Visible } else { Visibility::Hidden };
    }
    // 列表
    let current_name = state
        .current
        .as_ref()
        .map(|h| h.name.clone())
        .unwrap_or_else(|| "主角色".to_string());
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "英雄管理".to_string(),
            1 => format!("当前: {}", current_name),
            2 => state
                .heroes
                .first()
                .map(|h| format!("{}  Lv.{}  {}", h.name, h.level, class_name(h.class)))
                .unwrap_or_else(|| "（无英雄，点“创建英雄”创建）".to_string()),
            3 => state.message.clone(),
            4 => state.create_msg.clone(),
            _ => String::new(),
        };
    }
    for mut t in &mut class_label {
        let s = format!("职业: {}", class_name(state.create_class));
        if t.0 != s {
            t.0 = s;
        }
    }
    for mut t in &mut gender_label {
        let s = format!("性别: {}", gender_name(state.create_gender));
        if t.0 != s {
            t.0 = s;
        }
    }
    for mut t in &mut auto_pot_label {
        let s = autopot_text(state.auto_pot_hp, state.auto_pot_mp);
        if t.0 != s {
            t.0 = s;
        }
    }
    for mut t in &mut create_msg {
        if t.0 != state.create_msg {
            t.0 = state.create_msg.clone();
        }
    }
}

/// 英雄按钮点击（关闭/切换/创建面板）
#[allow(clippy::too_many_arguments)]
fn hero_button_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<HeroState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<&UiButton, With<HeroClose>>,
    main_btn: Query<&UiButton, With<HeroSwitchMain>>,
    hero1_btn: Query<&UiButton, With<HeroSwitch1>>,
    nav_btns: Query<(&UiButton, Option<&HeroOpenInventory>, Option<&HeroOpenEquipment>)>,
    create_btn: Query<&UiButton, With<HeroCreateBtn>>,
    class_btn: Query<&UiButton, With<HeroClassCycle>>,
    gender_btn: Query<&UiButton, With<HeroGenderCycle>>,
    ok_btn: Query<&UiButton, With<HeroCreateOk>>,
    cancel_btn: Query<&UiButton, With<HeroCreateCancel>>,
    behaviour_btns: Query<(&UiButton, &HeroBehaviourBtn)>,
    hp_btn: Query<&UiButton, With<HeroAutoHpCycle>>,
    mp_btn: Query<&UiButton, With<HeroAutoMpCycle>>,
) {
    if !mgr.is_open(DialogKind::Hero) {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Hero);
        }
    }
    for btn in &main_btn {
        if btn.clicked {
            net.send_packet(&crate::network::ChangeHeroWire { hero_index: 0 });
            state.message = "切换主角色…".to_string();
            tracing::info!("🦸 切换主角色");
        }
    }
    for btn in &hero1_btn {
        if btn.clicked {
            net.send_packet(&crate::network::ChangeHeroWire { hero_index: 1 });
            state.message = "切换英雄 1…".to_string();
            tracing::info!("🦸 切换英雄 1");
        }
    }
    for (btn, inv, eq) in &nav_btns {
        if btn.clicked {
            if inv.is_some() {
                mgr.toggle(DialogKind::HeroInventory);
                tracing::info!("🎒 英雄背包: {}", if mgr.is_open(DialogKind::HeroInventory) { "打开" } else { "关闭" });
            } else if eq.is_some() {
                mgr.toggle(DialogKind::HeroEquipment);
                tracing::info!("🦸 英雄装备: {}", if mgr.is_open(DialogKind::HeroEquipment) { "打开" } else { "关闭" });
            }
        }
    }
    for btn in &create_btn {
        if btn.clicked {
            state.creating = !state.creating;
            state.create_msg.clear();
            if input.texts.len() < 1 {
                input.texts.resize(1, String::new());
            }
            input.active = None;
        }
    }
    for btn in &class_btn {
        if btn.clicked && state.creating {
            state.create_class = next_class(state.create_class);
        }
    }
    for btn in &gender_btn {
        if btn.clicked && state.creating {
            state.create_gender = next_gender(state.create_gender);
        }
    }
    for btn in &ok_btn {
        if btn.clicked && state.creating {
            let name = input.texts.get(0).cloned().unwrap_or_default();
            net.send_packet(&mir2_shared::packets::client::hero::NewHero {
                name: name.trim().to_string(),
                gender: state.create_gender,
                class: state.create_class,
            });
            state.create_msg = "创建中…".to_string();
            tracing::info!("🦸 创建英雄: {}", name);
        }
    }
    for (btn, b) in &behaviour_btns {
    for btn in &hp_btn {
        if btn.clicked {
            state.auto_pot_hp = next_autopot(state.auto_pot_hp);
            net.send_packet(&mir2_shared::packets::client::hero::SetAutoPotValue { stat: STAT_HP, value: state.auto_pot_hp as u32 });
        }
    }
    for btn in &mp_btn {
        if btn.clicked {
            state.auto_pot_mp = next_autopot(state.auto_pot_mp);
            net.send_packet(&mir2_shared::packets::client::hero::SetAutoPotValue { stat: STAT_MP, value: state.auto_pot_mp as u32 });
        }
    }
        if btn.clicked {
            let behaviour = match b.0 {
                1 => mir2_shared::enums::HeroBehaviour::CounterAttack,
                2 => mir2_shared::enums::HeroBehaviour::Follow,
                3 => mir2_shared::enums::HeroBehaviour::Custom,
                _ => mir2_shared::enums::HeroBehaviour::Attack,
            };
            net.send_packet(&mir2_shared::packets::client::hero::SetHeroBehaviour { behaviour });
            state.message = format!("行为: {}", behaviour_name(behaviour));
            tracing::info!("🦸 设置英雄行为: {:?}", behaviour);
        }
    }
    for btn in &cancel_btn {
        if btn.clicked && state.creating {
            state.creating = false;
            input.active = None;
        }
    }
}
/// 职业显示名（C# MirClass 顺序）
fn class_name(c: mir2_shared::enums::MirClass) -> &'static str {
    use mir2_shared::enums::MirClass::*;
    match c {
        Warrior => "战士",
        Wizard => "法师",
        Taoist => "道士",
        Assassin => "刺客",
        Archer => "弓箭手",
        _ => "未知",
    }
}

/// 下一个职业（循环）
fn next_class(c: mir2_shared::enums::MirClass) -> mir2_shared::enums::MirClass {
    use mir2_shared::enums::MirClass::*;
    match c {
        Warrior => Wizard,
        Wizard => Taoist,
        Taoist => Assassin,
        Assassin => Archer,
        _ => Warrior,
    }
}

/// 性别显示名
fn gender_name(g: mir2_shared::enums::MirGender) -> &'static str {
    use mir2_shared::enums::MirGender::*;
    match g {
        Male => "男",
        Female => "女",
        _ => "?",
    }
}

/// 下一个性别（循环）
fn next_gender(g: mir2_shared::enums::MirGender) -> mir2_shared::enums::MirGender {
    use mir2_shared::enums::MirGender::*;
    match g {
        Male => Female,
        _ => Male,
    }
}

/// 消费服务端英雄事件（网络层只广播 ServerEvent）
fn hero_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut hero: ResMut<HeroState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::HeroChanged { index } => {
                hero.hero_index = *index;
                hero.message = if *index == 0 {
                    "已切换主角色".to_string()
                } else {
                    format!("已切换英雄 {}", index)
                };
            }
            ServerEvent::HeroManageReceived { heroes, current } => {
                hero.heroes = heroes.clone();
                hero.current = current.clone();
                hero.message = format!("英雄列表: {} 个", heroes.len());
            }
            ServerEvent::NewHeroResult { result } => {
                hero.create_msg = match *result {
                    1 => "英雄名字不符合要求".to_string(),
                    4 => "无法创建更多英雄".to_string(),
                    10 => "英雄创建成功".to_string(),
                    _ => format!("创建英雄失败（{}）", result),
                };
                if *result == 10 {
                    hero.creating = false;
                }
                hero.message = hero.create_msg.clone();
            }
            ServerEvent::HeroBehaviourSet { behaviour } => {
                if let Ok(b) = mir2_shared::enums::HeroBehaviour::try_from(*behaviour) {
                    hero.behaviour = b;
                    hero.message = format!("行为: {}", behaviour_name(b));
                }
            }
            ServerEvent::HeroAutoPotSet { stat, value } => {
                if *stat == STAT_HP {
                    hero.auto_pot_hp = (*value).min(100) as u8;
                } else if *stat == STAT_MP {
                    hero.auto_pot_mp = (*value).min(100) as u8;
                }
                hero.message = format!("自动药: {}", autopot_text(hero.auto_pot_hp, hero.auto_pot_mp));
            }
            ServerEvent::HeroInformation {
                inventory,
                equipment,
                hp,
                mp,
                exp,
                max_exp,
                auto_pot,
                auto_hp_percent,
                auto_mp_percent,
                hp_item_index,
                mp_item_index,
                ..
            } => {
                hero.inventory = inventory.clone();
                hero.equipment = equipment.clone();
                hero.hero_hp = *hp;
                hero.hero_mp = *mp;
                hero.hero_exp = *exp;
                hero.hero_max_exp = *max_exp;
                hero.auto_pot = *auto_pot;
                if *auto_hp_percent > 0 {
                    hero.auto_pot_hp = *auto_hp_percent;
                }
                if *auto_mp_percent > 0 {
                    hero.auto_pot_mp = *auto_mp_percent;
                }
                hero.hp_item_index = *hp_item_index;
                hero.mp_item_index = *mp_item_index;
                hero.message = "英雄信息已同步".to_string();
                tracing::info!(
                    "🦸 英雄信息: 背包 {} 格 装备 {} 格 HP={} MP={}",
                    inventory.len(),
                    equipment.len(),
                    hp,
                    mp
                );
            }
            _ => {}
        }
    }
}



/// 英雄行为显示名（C# HeroBehaviour）
fn behaviour_name(b: mir2_shared::enums::HeroBehaviour) -> &'static str {
    use mir2_shared::enums::HeroBehaviour::*;
    match b {
        Attack => "攻击",
        CounterAttack => "反击",
        Follow => "跟随",
        Custom => "自定义",
        _ => "未知",
    }
}

// C# Stat 枚举：HP=12, MP=13（服务端同）
pub(crate) const STAT_HP: u8 = 12;
pub(crate) const STAT_MP: u8 = 13;

/// 自动药阈值循环：0 → 30 → 50 → 70 → 90 → 0
pub(crate) fn next_autopot(v: u8) -> u8 {
    match v {
        0 => 30,
        30 => 50,
        50 => 70,
        70 => 90,
        _ => 0,
    }
}

/// 自动药显示文本
fn autopot_text(hp: u8, mp: u8) -> String {
    format!("HP {}%  MP {}%", hp, mp)
}
