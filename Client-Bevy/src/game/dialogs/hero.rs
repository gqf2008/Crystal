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
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label, spawn_panel,
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
    /// 英雄魔法（#218）
    pub magics: Vec<mir2_shared::data::client_data::ClientMagic>,
    /// 英雄对象 id（#220：MagicLeveled 路由）
    pub object_id: u32,
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
            magics: Vec::new(),
            object_id: 0,
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

impl HeroState {
    /// #220：按 object_id 路由英雄技能升级（匹配才更新，返回是否命中）
    pub fn apply_magic_leveled(
        &mut self,
        object_id: u32,
        spell: mir2_shared::enums::Spell,
        level: u8,
        experience: u16,
    ) -> bool {
        if self.object_id != object_id {
            return false;
        }
        if let Some(m) = self.magics.iter_mut().find(|m| m.spell == spell) {
            m.level = level;
            m.experience = experience;
            true
        } else {
            false
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

/// 打开英雄技能（#218）
#[derive(Component)]
pub struct HeroOpenSkill;

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

/// 英雄阵亡复活按钮（#1216：C# HeroPanel 复活按钮，hero_hp<=0 时显示）
#[derive(Component)]
pub struct HeroReviveBtn;

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
            (hero_ui_system, hero_button_system, hero_revive_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// #1216：英雄阵亡复活按钮（C# HeroPanel 复活按钮）：hero_hp<=0 显示，点击发 ReviveHeroWire
fn hero_revive_system(
    mut state: ResMut<HeroState>,
    net: Res<NetConnection>,
    mgr: Res<DialogManager>,
    mut q: Query<(Entity, &Interaction, &mut Visibility), With<HeroReviveBtn>>,
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
    // 复活按钮属于 Hero 对话框：对话框未打开时不显示，避免屏幕残留孤按钮
    let hero_open = mgr.is_open(DialogKind::Hero);
    for (e, inter, mut vis) in &mut q {
        *vis = if hero_open && state.hero_hp <= 0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&crate::network::ReviveHeroWire);
            state.message = "已请求复活英雄…".to_string();
            tracing::info!("🦸 请求复活英雄");
        }
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
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 Prguse[170] @ (280,80)。加宽加高到 320x310：切换/创建/行为/自动药/
    // 导航按钮全在面板内（旧 sprite 布局底部按钮 rel y=250-298 悬空 207 高面板外）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 310.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Hero), HeroWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(HeroClose);
        }
        // 列表行（0..5，#1135 末行显示英雄实时状态）@(18,40+22i)
        for i in 0..6usize {
            spawn_label(p, &cjk, "", 18.0, 40.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(HeroLine(i));
        }
        // 切换主角色 / 英雄 1 @(20/130,150) 90x25
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 20.0, 150.0, 90.0, 25.0, 10)
                .insert(HeroSwitchMain);
            spawn_icon_button(p, n, h, pr, 20.0, 182.0, 90.0, 25.0, 10).insert(HeroCreateBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 130.0, 150.0, 90.0, 25.0, 10).insert(HeroSwitch1);
        }
        // 创建英雄说明 + 行为标签 + 行为按钮（C# HeroBehaviourPanel：Prguse 1840..1843，16x17）
        spawn_label(p, &cjk, "创建英雄", 34.0, 186.0, 12.0, Color::WHITE, 10);
        spawn_label(p, &cjk, "行为:", 130.0, 186.0, 12.0, Color::WHITE, 10);
        for i in 0..4usize {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1840 + i) {
                crate::ui::theme::spawn_image(p, h.clone(), 160.0 + i as f32 * 18.0, 182.0, 16.0, 17.0, 10)
                    .insert((
                        HeroBehaviourBtn(i),
                        Button,
                    ));
            }
        }
        // 复活按钮（默认隐藏，hero_hp<=0 时由 hero_revive_system 显示）
        spawn_container(p, 20.0, 170.0, 160.0, 20.0, 10)
            .insert((
                Button,
                HeroReviveBtn,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                Visibility::Hidden,
            ))
            .with_children(|c| {
                spawn_label(c, &font, "英雄已阵亡·点击复活", 0.0, 4.0, 12.0, Color::srgb(1.0, 0.4, 0.4), 11);
            });
        // 自动药阈值（C# HeroInventoryDialog HPButton/MPButton，Title 560/563）
        spawn_label(p, &cjk, "自动药:", 20.0, 220.0, 12.0, Color::WHITE, 10);
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 560),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 561),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 562),
        ) {
            spawn_icon_button(p, n, h, pr, 80.0, 216.0, 60.0, 25.0, 10).insert(HeroAutoHpCycle);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 563),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 564),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 565),
        ) {
            spawn_icon_button(p, n, h, pr, 150.0, 216.0, 60.0, 25.0, 10).insert(HeroAutoMpCycle);
        }
        spawn_label(p, &cjk, "", 84.0, 220.0, 12.0, Color::srgb(1.0, 0.9, 0.4), 10)
            .insert(HeroAutoPotLabel);
        // 英雄背包/装备/技能 文本按钮（打开对应对话框）
        for (x, y, marker, text) in [
            (20.0, 250.0, "inv", "英雄背包"),
            (130.0, 250.0, "eq", "英雄装备"),
            (20.0, 280.0, "skill", "英雄技能"),
        ] {
            let mut cmds = spawn_container(p, x, y, 90.0, 18.0, 10);
            cmds.insert((
                Button,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ));
            if marker == "inv" {
                cmds.insert(HeroOpenInventory);
            } else if marker == "eq" {
                cmds.insert(HeroOpenEquipment);
            } else {
                cmds.insert(HeroOpenSkill);
            }
            cmds.with_children(|c| {
                spawn_label(c, &font, text, 0.0, 3.0, 12.0, Color::srgb(0.8, 0.9, 1.0), 11);
            });
        }
    });

    // 创建面板（独立根节点 @(280,296) 330x200，GlobalZIndex 45 盖过主面板）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(280.0),
                top: Val::Px(296.0),
                width: Val::Px(330.0),
                height: Val::Px(200.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.96)),
            HeroCreatePanel,
            DialogRoot(DialogKind::Hero),
            GlobalZIndex(45),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            spawn_label(p, &cjk, "名字:", 10.0, 16.0, 12.0, Color::WHITE, 10);
            // 名字输入框（TextInput 0）@(50,12)，命中矩形 (330,308,260,20)
            spawn_container(p, 50.0, 12.0, 260.0, 20.0, 10)
                .insert((
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                    crate::game::dialogs::text_input::TextInputField(0),
                    crate::game::dialogs::text_input::TextInputRect(330.0, 308.0, 260.0, 20.0),
                ))
                .with_children(|ic| {
                    ic.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(4.0),
                            top: Val::Px(2.0),
                            ..default()
                        },
                        Text::new(String::new()),
                        TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ZIndex(11),
                        crate::game::dialogs::text_input::TextInputDisplay(0),
                    ));
                });
            // 职业 / 性别 循环选择
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n, h, pr, 20.0, 44.0, 130.0, 22.0, 10)
                    .insert(HeroClassCycle);
            }
            spawn_label(p, &cjk, "职业: 战士", 28.0, 48.0, 12.0, Color::WHITE, 11)
                .insert(HeroClassLabel);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 170.0, 44.0, 100.0, 22.0, 10)
                    .insert(HeroGenderCycle);
            }
            spawn_label(p, &cjk, "性别: 男", 178.0, 48.0, 12.0, Color::WHITE, 11)
                .insert(HeroGenderLabel);
            // 确定 / 取消 / 结果提示
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 20.0, 80.0, 70.0, 23.0, 10)
                    .insert(HeroCreateOk);
                spawn_label(p, &cjk, "确定", 35.0, 84.0, 12.0, Color::WHITE, 11);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 110.0, 80.0, 70.0, 23.0, 10)
                    .insert(HeroCreateCancel);
                spawn_label(p, &cjk, "取消", 125.0, 84.0, 12.0, Color::WHITE, 11);
            }
            spawn_label(p, &cjk, "", 20.0, 114.0, 12.0, Color::srgb(1.0, 0.9, 0.4), 11)
                .insert(HeroCreateMsg);
        });
}

/// 显隐 + 列表渲染（按钮逻辑在 hero_button_system）
#[allow(clippy::too_many_arguments)]
fn hero_ui_system(
    mgr: Res<DialogManager>,
    state: Res<HeroState>,
    mut widgets: Query<&mut Visibility, With<HeroWidget>>,
    mut panel: Query<&mut Visibility, (With<HeroCreatePanel>, Without<HeroWidget>)>,
    mut lines: Query<(&mut Text, &HeroLine), (Without<HeroClassLabel>, Without<HeroGenderLabel>, Without<HeroCreateMsg>)>,
    mut class_label: Query<&mut Text, (With<HeroClassLabel>, Without<HeroLine>, Without<HeroGenderLabel>, Without<HeroCreateMsg>)>,
    mut gender_label: Query<&mut Text, (With<HeroGenderLabel>, Without<HeroLine>, Without<HeroClassLabel>, Without<HeroCreateMsg>)>,
    mut create_msg: Query<&mut Text, (With<HeroCreateMsg>, Without<HeroLine>, Without<HeroClassLabel>, Without<HeroGenderLabel>)>,
    mut auto_pot_label: Query<&mut Text, (With<HeroAutoPotLabel>, Without<HeroLine>, Without<HeroClassLabel>, Without<HeroGenderLabel>, Without<HeroCreateMsg>)>,
) {
    let open = mgr.is_open(DialogKind::Hero);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        // 创建面板（HeroCreatePanel）不在 widgets 里，且只在 open 分支管理显隐；
        // 关闭对话框时必须隐藏，否则“名字:/确定/取消/职业/性别”等标签残留屏幕
        for mut vis in &mut panel {
            *vis = Visibility::Hidden;
        }
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
            5 => {
                // #1135：出战英雄实时状态（HeroHealthChanged/GainHeroExperience/HeroLevelChanged 驱动）
                if state.hero_index > 0 {
                    format!(
                        "状态: HP {}  MP {}  经验 {}/{}",
                        state.hero_hp, state.hero_mp, state.hero_exp, state.hero_max_exp.max(1)
                    )
                } else {
                    String::new()
                }
            }
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
/// 两组 Option 查询（QueryData 元组上限 15）避免 Bevy 16 参数上限。
#[allow(clippy::type_complexity)]
fn hero_button_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<HeroState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut nav_btns: Query<(
        Entity,
        &Interaction,
        Option<&HeroClose>,
        Option<&HeroSwitchMain>,
        Option<&HeroSwitch1>,
        Option<&HeroOpenInventory>,
        Option<&HeroOpenEquipment>,
        Option<&HeroOpenSkill>,
        Option<&HeroCreateBtn>,
        Option<&HeroCreateCancel>,
    )>,
    mut cycle_btns: Query<(
        Entity,
        &Interaction,
        Option<&HeroClassCycle>,
        Option<&HeroGenderCycle>,
        Option<&HeroCreateOk>,
        Option<&HeroAutoHpCycle>,
        Option<&HeroAutoMpCycle>,
        Option<&HeroBehaviourBtn>,
    )>,
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
    if !mgr.is_open(DialogKind::Hero) {
        return;
    }
    for (e, inter, close, main, hero1, inv, eq, skill, create, cancel) in &mut nav_btns {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if close.is_some() {
            mgr.close(DialogKind::Hero);
        } else if main.is_some() {
            net.send_packet(&crate::network::ChangeHeroWire { hero_index: 0 });
            state.message = "切换主角色…".to_string();
            tracing::info!("🦸 切换主角色");
        } else if hero1.is_some() {
            net.send_packet(&crate::network::ChangeHeroWire { hero_index: 1 });
            state.message = "切换英雄 1…".to_string();
            tracing::info!("🦸 切换英雄 1");
        } else if inv.is_some() {
            mgr.toggle(DialogKind::HeroInventory);
            tracing::info!("🎒 英雄背包: {}", if mgr.is_open(DialogKind::HeroInventory) { "打开" } else { "关闭" });
        } else if eq.is_some() {
            mgr.toggle(DialogKind::HeroEquipment);
            tracing::info!("🦸 英雄装备: {}", if mgr.is_open(DialogKind::HeroEquipment) { "打开" } else { "关闭" });
        } else if skill.is_some() {
            mgr.toggle(DialogKind::HeroSkill);
            tracing::info!("🦸 英雄技能: {}", if mgr.is_open(DialogKind::HeroSkill) { "打开" } else { "关闭" });
        } else if create.is_some() {
            state.creating = !state.creating;
            state.create_msg.clear();
            if input.texts.len() < 1 {
                input.texts.resize(1, String::new());
            }
            input.active = None;
        } else if cancel.is_some() && state.creating {
            state.creating = false;
            input.active = None;
        }
    }
    for (e, inter, class_btn, gender_btn, ok, hp_btn, mp_btn, behaviour) in &mut cycle_btns {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if class_btn.is_some() && state.creating {
            state.create_class = next_class(state.create_class);
        } else if gender_btn.is_some() && state.creating {
            state.create_gender = next_gender(state.create_gender);
        } else if ok.is_some() && state.creating {
            let name = input.texts.get(0).cloned().unwrap_or_default();
            net.send_packet(&mir2_shared::packets::client::hero::NewHero {
                name: name.trim().to_string(),
                gender: state.create_gender,
                class: state.create_class,
            });
            state.create_msg = "创建中…".to_string();
            tracing::info!("🦸 创建英雄: {}", name);
        } else if hp_btn.is_some() {
            state.auto_pot_hp = next_autopot(state.auto_pot_hp);
            net.send_packet(&mir2_shared::packets::client::hero::SetAutoPotValue { stat: STAT_HP, value: state.auto_pot_hp as u32 });
        } else if mp_btn.is_some() {
            state.auto_pot_mp = next_autopot(state.auto_pot_mp);
            net.send_packet(&mir2_shared::packets::client::hero::SetAutoPotValue { stat: STAT_MP, value: state.auto_pot_mp as u32 });
        } else if let Some(b) = behaviour {
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
            ServerEvent::MagicLeveled {
                object_id,
                spell,
                level,
                experience,
            } => {
                if hero.apply_magic_leveled(*object_id, *spell, *level, *experience) {
                    hero.message = format!("英雄技能 {:?} 升级 Lv.{}", spell, level);
                }
            }
            ServerEvent::HeroMagicLearned { magic } => {
                // #1128：英雄技能书学会 → upsert 到 HeroState.magics（面板即时刷新）
                if let Some(existing) = hero.magics.iter_mut().find(|m| m.spell == magic.spell) {
                    *existing = magic.clone();
                } else {
                    hero.magics.push(magic.clone());
                }
                hero.message = format!("英雄学会技能: {}", magic.name);
                tracing::info!("🦸 英雄学会技能: {} ({:?})", magic.name, magic.spell);
            }
            ServerEvent::HeroHealthChanged { hp, mp } => {
                // #1135：英雄 HP/MP 实时同步（C# S.HeroHealthChanged）
                hero.hero_hp = *hp as i32;
                hero.hero_mp = *mp as i32;
            }
            ServerEvent::GainHeroExperience { amount } => {
                // #1135：英雄经验增加（C# S.GainHeroExperience）
                hero.hero_exp = hero.hero_exp.saturating_add(*amount as i64);
                hero.message = format!("英雄经验 +{}", amount);
            }
            ServerEvent::HeroLevelChanged { level, exp, max_exp } => {
                // #1135：英雄升级（C# S.HeroLevelChanged）——同步面板与列表等级
                hero.hero_exp = *exp;
                hero.hero_max_exp = *max_exp;
                hero.message = format!("英雄升级 Lv.{}", level);
                if let Some(cur) = hero.current.as_mut() {
                    cur.level = *level;
                }
                let hero_idx = hero.hero_index;
                for h in hero.heroes.iter_mut() {
                    if h.index as u8 == hero_idx {
                        h.level = *level;
                    }
                }
            }
            ServerEvent::HeroInformation {
                object_id,
                inventory,
                equipment,
                magics,
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
                hero.magics = magics.clone();
                hero.object_id = *object_id;
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

#[cfg(test)]
mod tests {
    use super::HeroState;
    use mir2_shared::data::client_data::ClientMagic;
    use mir2_shared::enums::Spell;

    fn cm(spell: Spell) -> ClientMagic {
        ClientMagic {
            name: format!("{:?}", spell),
            spell,
            base_cost: 0,
            level_cost: 0,
            icon: 0,
            level1: 0,
            level2: 0,
            level3: 0,
            need1: 0,
            need2: 0,
            need3: 0,
            level: 0,
            key: 0,
            experience: 0,
            delay: 0,
            range: 1,
            cast_time: 0,
        }
    }

    #[test]
    fn hero_magic_leveled_routes_by_object_id() {
        let mut hero = HeroState::default();
        hero.object_id = 0x1000_0100;
        hero.magics.push(cm(Spell::FireBall));
        // 正确 object_id → 更新
        assert!(hero.apply_magic_leveled(0x1000_0100, Spell::FireBall, 2, 500));
        assert_eq!(hero.magics[0].level, 2);
        assert_eq!(hero.magics[0].experience, 500);
        // 玩家 object_id → 不命中
        assert!(!hero.apply_magic_leveled(100, Spell::FireBall, 3, 0));
        assert_eq!(hero.magics[0].level, 2);
    }
}
