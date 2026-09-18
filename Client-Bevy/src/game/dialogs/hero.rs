// ============================================================================
// 英雄对话框（M48 + #190 英雄管理 UI）
// 参考：C# HeroDialog + HeroManageDialog + NewHeroDialog
// 网络：
//   S: ManageHeroes（英雄列表）/ NewHero（创建结果）/ ChangeHero（切换）
//   C: ChangeHero[hero_index u8] / NewHero[name, gender, class]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_label, spawn_panel, CloseButton,
};

/// #2892 批B：管理窗面板精灵与 C# 原生尺寸/坐标（C# `HeroManageDialog.Index = 1688; Location = (350,350)`）
pub const MANAGE_PANEL: (LibraryName, usize) = (LibraryName::Prguse, 1688);
pub const MANAGE_PANEL_SIZE: (f32, f32) = (352.0, 161.0);
pub const MANAGE_PANEL_POS: (f32, f32) = (350.0, 350.0);

/// 英雄状态
#[derive(Resource)]
pub struct HeroState {
    pub hero_index: u8,
    pub message: String,
    /// 英雄列表（C# S.ManageHeroes）
    pub heroes: Vec<mir2_shared::data::client_data::ClientHeroInformation>,
    /// 当前英雄
    pub current: Option<mir2_shared::data::client_data::ClientHeroInformation>,
    /// C# `S.ManageHeroes.MaximumCount`：**含主角色**的总名额（`PlayerObject.cs:14664`），
    /// 客户端可用头像槽 = `max_count - 1`（`HeroDialogs.cs:840`）
    pub max_count: i32,
    /// C# `HeroManageDialog.Visible`：随 `S.ManageHeroes` 弹出（`GameScene.cs:6063-6065`），
    /// 关闭键 / ESC（C# `KeybindOptions.Closeall`，`GameScene.cs:668-708`）隐藏
    pub managing: bool,
    /// 正在确认的槽位（C# `MirMessageBox`「MakeActiveHero」，`HeroDialogs.cs:826-831`）
    pub confirm_slot: Option<usize>,
    /// 创建面板是否打开
    pub creating: bool,
    /// 创建结果提示
    pub create_msg: String,
    /// 创建面板选中的职业/性别
    pub create_class: mir2_shared::enums::MirClass,
    pub create_gender: mir2_shared::enums::MirGender,
    /// #2892 批57：英雄属性快照（状态页/状态二页；`S.HeroInformation` 下发）
    pub stats: crate::network::server_event::HeroStatsInfo,
    /// 英雄行为（C# HeroBehaviour：0=攻击 1=反击 2=跟随 3=自定义）
    pub behaviour: mir2_shared::enums::HeroBehaviour,
    /// #2892 批C：C# `HeroSpawnState`（`S.UpdateHeroSpawnState`）——
    /// `HeroBehaviourPanel.Visible = state > Unsummoned`（`GameScene.cs:6190`）
    pub spawn_state: mir2_shared::enums::HeroSpawnState,
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
    /// #2892 批C：英雄最大 HP/MP（HUD `HeroInfoPanel` 百分比条；C# `Stats[Stat.HP/MP]`）
    pub hero_max_hp: i32,
    pub hero_max_mp: i32,
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
            max_count: 1,
            managing: false,
            confirm_slot: None,
            creating: false,
            create_msg: String::new(),
            create_class: mir2_shared::enums::MirClass::Warrior,
            create_gender: mir2_shared::enums::MirGender::Male,
            behaviour: mir2_shared::enums::HeroBehaviour::Attack,
            // C# `HeroSpawnState` 默认无出战英雄（`None`/`Unsummoned` 都不显示行为条）
            spawn_state: mir2_shared::enums::HeroSpawnState::Unsummoned,
            auto_pot_hp: 0,
            auto_pot_mp: 0,
            stats: crate::network::server_event::HeroStatsInfo::default(),
            inventory: Vec::new(),
            equipment: Vec::new(),
            magics: Vec::new(),
            object_id: 0,
            hero_hp: 0,
            hero_mp: 0,
            hero_max_hp: 0,
            hero_max_mp: 0,
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
    /// 英雄技能快捷键分配（C# MagicButton.HeroMagic -> AssignKeyPanel KeyOffset=17）。
    /// 返回目标技能旧 key，供 C.MagicKey.OldKey 路由到英雄。
    pub fn assign_key(&mut self, spell: mir2_shared::enums::Spell, key: u8) -> Option<u8> {
        let old = self.magics.iter().find(|m| m.spell == spell).map(|m| m.key);
        for m in &mut self.magics {
            if m.spell != spell && m.key == key {
                m.key = 0;
            }
        }
        if let Some(m) = self.magics.iter_mut().find(|m| m.spell == spell) {
            m.key = key;
        }
        old
    }

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

/// 英雄管理窗面板（C# `HeroManageDialog`，`Prguse[1688]` 352x161 @(350,350)，`HeroDialogs.cs:800-806`）
#[derive(Component)]
pub struct HeroManageWidget;

/// 管理窗关闭键（C# `HeroManageDialog.CloseButton`，`Prguse2[360..362]` @(Size.Width-24,4)）
#[derive(Component)]
pub struct HeroManageClose;

/// 8 个英雄槽（C# `HeroManageDialog.Avatars[i]`，`HeroDialogs.cs:822-851`）
#[derive(Component)]
pub struct HeroManageSlot(pub usize);

/// 当前英雄头像（C# `HeroManageDialog.CurrentAvatar` @(15,61)）
#[derive(Component)]
pub struct HeroManageCurrent;

/// MakeActiveHero 询问框（C# `MirMessageBox`，`Prguse[360]` 456x190 居中 @(284,289)）
#[derive(Component)]
pub struct HeroManageConfirm;

#[derive(Component)]
pub struct HeroManageConfirmText;

#[derive(Component)]
pub struct HeroManageConfirmYes;

#[derive(Component)]
pub struct HeroManageConfirmNo;

/// 头像精灵表（C# `HeroAvatar(class,gender)+370`：index = [class][gender]；空槽框 `Prguse[1689]`）
#[derive(Resource, Default)]
pub struct HeroAvatarImages {
    pub images: [[Option<Handle<Image>>; 2]; 5],
    pub empty_frame: Option<Handle<Image>>,
}

impl HeroAvatarImages {
    /// C# `GameScene.HeroAvatar(job, gender)` = `1400 + job + 10*gender`，头像控件用 `+370`
    /// （`HeroDialogs.cs:883`）→ `1770 + class + 10*gender`
    pub fn avatar_index(class: u8, gender: u8) -> usize {
        1770 + class as usize + 10 * gender as usize
    }
}

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroState>();
        app.add_systems(Update, hero_server_events.run_if(in_state(AppState::Game)));
        // #2892 批C：`DialogKind::Hero` 的自造聚合窗已删除（其功能全部回到 C# 归属：
        // 列表/切换 = `HeroManageDialog`(本文件)、行为 = HUD `HeroBehaviourPanel`、
        // 自动药 = 英雄背包窗、创建 = `NewCharacterDialog` 英雄模式）——
        // 这里只保留 `HeroManageDialog` 窗口
        app.add_systems(OnEnter(AppState::Game), spawn_hero_manage);
        app.add_systems(OnExit(AppState::Game), cleanup_hero);
        app.add_systems(Update, hero_manage_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_hero(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 生成英雄管理窗（C# `HeroManageDialog`，`Prguse[1688]` 352x161 @(350,350)，`Movable = true`）
fn spawn_hero_manage(
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
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // ===== 英雄管理窗（C# `HeroManageDialog`，#2791 单元①）=====
    // `Prguse[1688]` 352x161 @(350,350)；随 `S.ManageHeroes` 弹出（`GameScene.cs:6063-6065`），
    // 关闭键 / ESC（C# `KeybindOptions.Closeall`）隐藏。窗口用独立 kind `HeroManage`
    // （C# 两窗各自 `Movable`；复用 `Hero` 会被 kind 级拖动/包围盒连带），并挂 `AlwaysVisible`
    // 走状态驱动（`enforce_dialog_visibility` 只做「未 open → Hidden」的单向兜底，见
    //  LESSON_UI状态驱动窗口须同步管理栈否则单向显隐兜底会反向隐藏）。
    let mut avatar_images = HeroAvatarImages {
        empty_frame: load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1689),
        ..Default::default()
    };
    for class in 0..5u8 {
        for gender in 0..2u8 {
            avatar_images.images[class as usize][gender as usize] = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Prguse,
                HeroAvatarImages::avatar_index(class, gender),
            );
        }
    }
    let empty_frame = avatar_images.empty_frame.clone();
    commands.insert_resource(avatar_images);
    if let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1688) {
        let manage = spawn_panel(
            &mut commands,
            bg,
            MANAGE_PANEL_POS.0,
            MANAGE_PANEL_POS.1,
            MANAGE_PANEL_SIZE.0,
            MANAGE_PANEL_SIZE.1,
            31,
        );
        commands.entity(manage).insert((
            HeroManageWidget,
            DialogRoot(DialogKind::HeroManage),
            crate::game::dialogs::AlwaysVisible,
        ));
        commands.entity(manage).with_children(|p| {
            // 关闭键 @(Size.Width-24, 4)
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
            ) {
                spawn_icon_button(p, n, h, pr, 328.0, 4.0, 24.0, 21.0, 10)
                    .insert((HeroManageClose, CloseButton));
            }
            // 当前英雄头像 @(15,61)（C# `SetCurrentHero`：Info 落地再 +5,+5）
            if let Some(frame) = empty_frame.clone() {
                spawn_image(p, frame, 15.0, 61.0, 60.0, 41.0, 5).insert(HeroManageCurrent);
            }
            // 8 槽头像 @(98+60*(i%4), 61+40*(i/4))（空槽框 `Prguse[1689]` 60x41）
            for i in 0..8usize {
                let x = 98.0 + 60.0 * (i % 4) as f32;
                let y = 61.0 + 40.0 * (i / 4) as f32;
                if let Some(frame) = empty_frame.clone() {
                    spawn_image(p, frame, x, y, 60.0, 41.0, 5).insert((
                        Button,
                        HeroManageSlot(i),
                        crate::ui::tooltip::UiHint {
                            text: String::new(),
                        },
                    ));
                }
            }
        });
    }
    // MakeActiveHero 询问框（C# `MirMessageBox`；同 group.rs 邀请框：`Prguse[360]` 456x190
    // 居中 @(284,289)，Yes `Title[206..208]` @(260,157)、No `Title[210..212]` @(360,157)）
    if let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let confirm = spawn_panel(&mut commands, bg, 284.0, 289.0, 456.0, 190.0, 47);
        commands.entity(confirm).insert((
            HeroManageConfirm,
            DialogRoot(DialogKind::HeroManage),
            crate::game::dialogs::AlwaysVisible,
        ));
        commands.entity(confirm).with_children(|p| {
            spawn_label(p, &cjk, "", 35.0, 35.0, 12.0, Color::WHITE, 9)
                .insert(HeroManageConfirmText);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10)
                    .insert(HeroManageConfirmYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10)
                    .insert(HeroManageConfirmNo);
            }
        });
    }
}

/// 显隐 + 列表渲染（按钮逻辑在 hero_button_system）
#[allow(clippy::too_many_arguments)]
/// 管理窗槽位原点（C# `HeroDialogs.cs:839`：`98 + 60*(i%4)`、`61 + 40*(i/4)`）
fn hero_slot_origin(i: usize) -> (f32, f32) {
    (98.0 + 60.0 * (i % 4) as f32, 61.0 + 40.0 * (i / 4) as f32)
}

/// 头像句柄（C# `GameScene.HeroAvatar(class,gender)+370`）
fn avatar_handle(
    avatars: &HeroAvatarImages,
    class: mir2_shared::enums::MirClass,
    gender: mir2_shared::enums::MirGender,
) -> Option<Handle<Image>> {
    avatars.images[class as usize][gender as usize].clone()
}

/// 头像 Hint（C# `HeroManageAvatar.Info` setter 里 `Hint = info.ToString()`，`HeroDialogs.cs:884`）。
/// C# `ClientHeroInformation.ToString()`（`Shared/Data/ClientData.cs:652-655`）=
/// `Name` + 换行 + `Level {Level} {gender} {class}`；性别/职业取**枚举名小写**（不本地化），
/// 例如「示范英雄\nLevel 20 male warrior」。
fn hero_manage_hint(h: &mir2_shared::data::client_data::ClientHeroInformation) -> String {
    format!(
        "{}\nLevel {} {} {}",
        h.name,
        h.level,
        hero_gender_name_lower(h.gender),
        hero_class_name_lower(h.class)
    )
}

/// C# `Enum.GetName(typeof(MirClass), class).ToLower()`（`ClientData.cs:654`）
fn hero_class_name_lower(class: mir2_shared::enums::MirClass) -> &'static str {
    match class {
        mir2_shared::enums::MirClass::Warrior => "warrior",
        mir2_shared::enums::MirClass::Wizard => "wizard",
        mir2_shared::enums::MirClass::Taoist => "taoist",
        mir2_shared::enums::MirClass::Assassin => "assassin",
        mir2_shared::enums::MirClass::Archer => "archer",
    }
}

/// C# `Enum.GetName(typeof(MirGender), gender).ToLower()`（`ClientData.cs:654`）
fn hero_gender_name_lower(gender: mir2_shared::enums::MirGender) -> &'static str {
    match gender {
        mir2_shared::enums::MirGender::Male => "male",
        mir2_shared::enums::MirGender::Female => "female",
    }
}

/// C# `ClientTextKeys.MakeActiveHero`（`Client/Localization/Chinese.json` Text.MakeActiveHero）
fn hero_manage_confirm_text(name: &str) -> String {
    format!("是否要将 {name} 设为你的当前英雄？")
}

/// 英雄管理窗（C# `HeroManageDialog`）：显隐 + 8 槽头像/Hint + 当前头像 + MakeActiveHero 确认
/// （#2791 单元①）。独立系统而非并入 `hero_ui_system`：系统参数上限 16，
/// 见 LESSON_Bevy系统参数上限16_加查询即编译失败须拆独立系统。
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn hero_manage_system(
    mut state: ResMut<HeroState>,
    net: Res<NetConnection>,
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<crate::game::chat::ChatState>,
    input: Res<crate::game::dialogs::text_input::TextInputState>,
    avatars: Res<HeroAvatarImages>,
    mut q: Query<
        (
            Entity,
            // 面板/图片（spawn_panel、spawn_image）没有 Interaction（只有 Button 才有），
            // 故取 Option：显隐/换图要覆盖它们，点击只在 Some 时判定
            Option<&Interaction>,
            &mut Visibility,
            &mut Node,
            &mut ImageNode,
            Option<&mut crate::ui::tooltip::UiHint>,
            Option<&HeroManageWidget>,
            Option<&HeroManageClose>,
            Option<&HeroManageSlot>,
            Option<&HeroManageCurrent>,
            Option<&HeroManageConfirm>,
            Option<&HeroManageConfirmYes>,
            Option<&HeroManageConfirmNo>,
        ),
        // 写入全部按标记分发，但裸查询仍匹配全 app 实体（每帧全表扫描+调度串行）——
        // 限定管理窗部件（同 #2954 char_skill 的踩法）
        Or<(
            With<HeroManageWidget>,
            With<HeroManageClose>,
            With<HeroManageSlot>,
            With<HeroManageCurrent>,
            With<HeroManageConfirm>,
            With<HeroManageConfirmYes>,
            With<HeroManageConfirmNo>,
        )>,
    >,
    mut confirm_text: Query<&mut Text, With<HeroManageConfirmText>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: Option<&Interaction>,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let Some(inter) = inter else {
            return false;
        };
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // C# `HeroDialogs.cs:840`：`i > MaximumHeroCount - 2` 的槽位不归英雄所有，
    // 恒显示空框且 `NotControl = true`（不可点、无 Hint）
    let last_usable = state.max_count - 2;
    let mut clicked_slot: Option<usize> = None;
    let show = if state.managing {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for (
        e,
        inter,
        mut vis,
        mut node,
        mut image,
        hint,
        widget,
        close,
        slot,
        current,
        confirm,
        yes,
        no,
    ) in &mut q
    {
        if widget.is_some() {
            *vis = show;
            continue;
        }
        if confirm.is_some() {
            *vis = if state.managing && state.confirm_slot.is_some() {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            continue;
        }
        if close.is_some() {
            if state.managing && edge(e, inter, &mut prev_inter) {
                state.managing = false;
                state.confirm_slot = None;
            }
            continue;
        }
        if yes.is_some() {
            if state.managing && edge(e, inter, &mut prev_inter) {
                if let Some(hero) = state
                    .confirm_slot
                    .and_then(|i| state.heroes.get(i))
                    .cloned()
                {
                    net.send_packet(&crate::network::ChangeHeroWire {
                        hero_index: hero.index as u8,
                    });
                    tracing::info!("🦸 切换英雄：{}（index {}）", hero.name, hero.index);
                }
                state.confirm_slot = None;
            }
            continue;
        }
        if no.is_some() {
            if state.managing && edge(e, inter, &mut prev_inter) {
                state.confirm_slot = None;
            }
            continue;
        }
        if let Some(slot) = slot {
            let i = slot.0 as i32;
            // 只有 `i <= max_count-2` 且 HeroStorage[i] 非空才是英雄槽
            let hero = state
                .heroes
                .get(slot.0)
                .filter(|_| i <= last_usable)
                .cloned();
            let (x, y) = hero_slot_origin(slot.0);
            match hero {
                Some(h) => {
                    // C# `Info` setter：平移 +5,+5 并换成头像精灵（36x30）
                    if let Some(handle) = avatar_handle(&avatars, h.class, h.gender) {
                        if image.image != handle {
                            image.image = handle;
                        }
                    }
                    node.left = Val::Px(x + 5.0);
                    node.top = Val::Px(y + 5.0);
                    node.width = Val::Px(36.0);
                    node.height = Val::Px(30.0);
                    *vis = show;
                    if let Some(mut hint) = hint {
                        let t = hero_manage_hint(&h);
                        if hint.text != t {
                            hint.text = t;
                        }
                    }
                    if state.managing && edge(e, inter, &mut prev_inter) {
                        clicked_slot = Some(slot.0);
                    }
                }
                None if i > last_usable => {
                    // 超名额：恒显空框（60x41，`Prguse[1689]`），不可点、无 Hint
                    if let Some(frame) = avatars.empty_frame.clone() {
                        if image.image != frame {
                            image.image = frame;
                        }
                    }
                    node.left = Val::Px(x);
                    node.top = Val::Px(y);
                    node.width = Val::Px(60.0);
                    node.height = Val::Px(41.0);
                    *vis = show;
                    if let Some(mut hint) = hint {
                        hint.text.clear();
                    }
                }
                None => {
                    // 有名额但英雄为空：C# `Info = null` → `Visible = false`
                    *vis = Visibility::Hidden;
                    if let Some(mut hint) = hint {
                        hint.text.clear();
                    }
                }
            }
            continue;
        }
        if current.is_some() {
            // C# `SetCurrentHero`：`CurrentAvatar.Location = (15,61)`，Info 落地再 +5,+5
            match state.current.as_ref().filter(|_| state.managing) {
                Some(h) => {
                    if let Some(handle) = avatar_handle(&avatars, h.class, h.gender) {
                        if image.image != handle {
                            image.image = handle;
                        }
                    }
                    node.left = Val::Px(20.0);
                    node.top = Val::Px(66.0);
                    node.width = Val::Px(36.0);
                    node.height = Val::Px(30.0);
                    *vis = Visibility::Visible;
                }
                None => {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
    // 点头像槽 → 弹 MakeActiveHero 询问框（C# `HeroDialogs.cs:826-831`）
    if let Some(i) = clicked_slot {
        state.confirm_slot = Some(i);
    }
    for mut t in &mut confirm_text {
        let s = match state.confirm_slot.and_then(|i| state.heroes.get(i)) {
            Some(h) => hero_manage_confirm_text(&h.name),
            None => String::new(),
        };
        if t.0 != s {
            t.0 = s;
        }
    }
    // ESC（C# `KeybindOptions.Closeall`）：询问框（模态）先吃一次，再关管理窗。
    // 与 `esc_close_dialogs_system` 的优先级一致：聊天输入开 / 输入框聚焦时让路。
    if state.managing
        && keys.just_pressed(KeyCode::Escape)
        && !chat.input_active
        && input.active.is_none()
    {
        if state.confirm_slot.is_some() {
            state.confirm_slot = None;
        } else {
            state.managing = false;
        }
    }
}

/// 消费服务端英雄事件（网络层只广播 ServerEvent）
fn hero_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut hero: ResMut<HeroState>,
    // #2892 批C：`S.HeroCreateRequest` → 打开原版「新建角色」对话框（英雄模式）
    mut new_char: ResMut<crate::ui::new_character::NewCharState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::HeroChanged { index } => {
                hero.hero_index = *index;
                // C# `GameScene.ChangeHero`（:6068-6074）：换人后同步 `CurrentAvatar` 与列表
                hero.current = hero
                    .heroes
                    .iter()
                    .find(|h| h.index as u8 == *index)
                    .cloned();
                hero.message = if *index == 0 {
                    "已切换主角色".to_string()
                } else {
                    format!("已切换英雄 {}", index)
                };
            }
            ServerEvent::HeroManageReceived {
                heroes,
                current,
                max_count,
            } => {
                hero.heroes = heroes.clone();
                hero.current = current.clone();
                hero.max_count = *max_count;
                // C# `ManageHeroes`（:6063-6065）：每次收到列表都 `Show()` 英雄管理窗
                hero.managing = true;
                hero.confirm_slot = None;
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
                    // #2892 批C：英雄创建成功 → 收起新建角色对话框（C# `NewHero` 回调里 `NewHeroDialog.Dispose()`）
                    new_char.visible = false;
                    new_char.hero_mode = false;
                    new_char.name.clear();
                }
                hero.message = hero.create_msg.clone();
            }
            ServerEvent::HeroBehaviourSet { behaviour } => {
                if let Ok(b) = mir2_shared::enums::HeroBehaviour::try_from(*behaviour) {
                    hero.behaviour = b;
                    hero.message = format!("行为: {}", behaviour_name(b));
                }
            }
            // #2892 批C：C# `S.UpdateHeroSpawnState` —— HUD 行为条显隐判据
            // （`HeroBehaviourPanel.Visible = p.State > Unsummoned`，`GameScene.cs:6190`）
            ServerEvent::HeroSpawnStateChanged { state: spawn } => {
                hero.spawn_state = *spawn;
                tracing::info!("🧝 英雄出战状态: {:?}", spawn);
            }
            // #2892 批C：C# `GameScene.HeroCreateRequest`（`:6044-6052`）——
            // 按 `CanCreateClass` 显隐职业钮后 `NewHeroDialog.Show()`（= 新建角色对话框英雄模式）
            ServerEvent::HeroCreateRequested { can_create_class } => {
                let mut allowed = [true; 5];
                for (i, v) in can_create_class.iter().enumerate().take(5) {
                    allowed[i] = *v;
                }
                new_char.hero_mode = true;
                new_char.can_create_class = allowed;
                new_char.name.clear();
                new_char.error = None;
                new_char.gender = mir2_shared::enums::MirGender::Male;
                // 默认职业：保持当前选择若可选，否则取第一个可选职业
                if !allowed
                    .get(new_char.class as usize)
                    .copied()
                    .unwrap_or(true)
                {
                    let fallback = [
                        mir2_shared::enums::MirClass::Warrior,
                        mir2_shared::enums::MirClass::Wizard,
                        mir2_shared::enums::MirClass::Taoist,
                        mir2_shared::enums::MirClass::Assassin,
                        mir2_shared::enums::MirClass::Archer,
                    ]
                    .into_iter()
                    .find(|c| allowed.get(*c as usize).copied().unwrap_or(false));
                    if let Some(c) = fallback {
                        new_char.class = c;
                    }
                }
                new_char.visible = true;
                tracing::info!("🧝 打开英雄创建对话框（可创建: {:?}）", allowed);
            }
            ServerEvent::HeroAutoPotSet { stat, value } => {
                if *stat == STAT_HP {
                    hero.auto_pot_hp = (*value).min(100) as u8;
                } else if *stat == STAT_MP {
                    hero.auto_pot_mp = (*value).min(100) as u8;
                }
                hero.message = format!(
                    "自动药: {}",
                    autopot_text(hero.auto_pot_hp, hero.auto_pot_mp)
                );
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
            ServerEvent::HeroLevelChanged {
                level,
                exp,
                max_exp,
            } => {
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
                max_hp,
                max_mp,
                exp,
                max_exp,
                auto_pot,
                auto_hp_percent,
                auto_mp_percent,
                hp_item_index,
                mp_item_index,
                stats,
                ..
            } => {
                hero.inventory = inventory.clone();
                hero.equipment = equipment.clone();
                hero.magics = magics.clone();
                hero.object_id = *object_id;
                hero.hero_hp = *hp;
                hero.hero_mp = *mp;
                // #2892 批C：HUD 百分比条用的最大值（C# `Stats[Stat.HP/MP]`）
                hero.hero_max_hp = *max_hp;
                hero.hero_max_mp = *max_mp;
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
                hero.stats = stats.clone();
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
        // #2775：C# `ClientTextKeys.HeroBehaviour_Custom` = 「自动」（此前写作「自定义」属自造文案）
        Custom => "自动",
        _ => "未知",
    }
}

/// #2775：英雄行为按钮 Hint（C# `HeroDialogs.cs:774` `HeroBehaviourFormat` =「英雄行为：{0}」，
/// `{0}` 取 `HeroBehaviour` 枚举的本地化名；按钮下标 i 即枚举值，C# `Enum.Parse` 同序）。
pub(crate) fn behaviour_hint(i: usize) -> String {
    let name = match i {
        0 => "攻击",
        1 => "反击",
        2 => "跟随",
        _ => "自动",
    };
    format!("英雄行为：{name}")
}

// C# Stat 枚举：HP=12, MP=13（服务端同）
pub(crate) const STAT_HP: u8 = 12;
pub(crate) const STAT_MP: u8 = 13;

/// 自动药显示文本
fn autopot_text(hp: u8, mp: u8) -> String {
    format!("HP {}%  MP {}%", hp, mp)
}

#[cfg(test)]
mod tests {
    /// 表征（#2954 同类防护）：hero_manage_system 的宽查询限定管理窗部件后，
    /// 无标记实体不得被触碰；HeroManageWidget 显隐仍跟随 managing。
    #[test]
    fn hero_manage_system_does_not_stomp_unrelated_widgets() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<super::HeroState>();
        app.insert_resource(crate::network::NetConnection::default());
        app.init_resource::<crate::game::chat::ChatState>();
        app.init_resource::<crate::game::dialogs::text_input::TextInputState>();
        app.init_resource::<super::HeroAvatarImages>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, super::hero_manage_system);

        let decoy = app
            .world_mut()
            .spawn((Visibility::Visible, Node::default(), ImageNode::default()))
            .id();
        let widget = app
            .world_mut()
            .spawn((
                Visibility::Hidden,
                Node::default(),
                ImageNode::default(),
                super::HeroManageWidget,
            ))
            .id();

        // managing=false → widget Hidden，decoy 不动
        app.update();
        assert_eq!(
            app.world().entity(decoy).get::<Visibility>().unwrap(),
            &Visibility::Visible,
            "无标记实体不得被 hero_manage_system 触碰"
        );
        assert_eq!(
            app.world().entity(widget).get::<Visibility>().unwrap(),
            &Visibility::Hidden,
            "managing=false → widget Hidden"
        );

        // managing=true → widget Visible，decoy 仍不动
        app.world_mut().resource_mut::<super::HeroState>().managing = true;
        app.update();
        assert_eq!(
            app.world().entity(widget).get::<Visibility>().unwrap(),
            &Visibility::Visible,
            "managing=true → widget Visible"
        );
        assert_eq!(
            app.world().entity(decoy).get::<Visibility>().unwrap(),
            &Visibility::Visible,
            "无标记实体在开窗路径下仍不得被触碰"
        );
    }

    use super::{
        behaviour_hint, behaviour_name, hero_manage_confirm_text, hero_manage_hint,
        hero_manage_system, hero_server_events, hero_slot_origin, HeroAvatarImages,
        HeroManageConfirmText, HeroManageConfirmYes, HeroManageSlot, HeroState,
    };
    use crate::game::dialogs::text_input::TextInputState;
    use crate::network::NetConnection;
    use bevy::prelude::*;
    use mir2_shared::data::client_data::{ClientHeroInformation, ClientMagic};
    use mir2_shared::enums::Spell;

    /// #2892 批C：自造 `DialogKind::Hero` 聚合窗删除后，英雄相关窗口只剩
    /// `HeroManageDialog`（C# 显式 `Movable = true`，`HeroDialogs.cs:804`）→ 必须**可拖**。
    #[test]
    fn hero_manage_window_is_draggable() {
        use crate::game::dialogs::{DialogKind, DialogRoot, NotDraggable};
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        // CI 无游戏资产（Data/ 不入库）→ 跳过（详见 libraries::data_assets_present）
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip hero_manage_window_is_draggable: 无 Data 资产");
            return;
        }
        let mut world = World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(Libraries::new(
            resolve_data_path(),
        )));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiCjkFont::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        // #2892 批C：自造 Hero 窗删除后 spawn_hero_manage 不再读键位
        world
            .run_system_once(super::spawn_hero_manage)
            .expect("spawn_hero_manage 应成功");

        let mut q = world.query::<(Entity, &DialogRoot)>();
        let roots: Vec<(Entity, DialogKind)> = q
            .iter(&world)
            .filter(|(_, r)| matches!(r.0, DialogKind::HeroManage))
            .map(|(e, r)| (e, r.0))
            .collect();
        assert!(
            !roots.is_empty(),
            "应生成 HeroManage 根（自造 Hero 窗已删除）"
        );
        for (e, _) in &roots {
            assert!(
                !world.entity(*e).contains::<NotDraggable>(),
                "HeroManage 根 {e:?} 应可拖（C# `HeroManageDialog.Movable = true`）"
            );
        }
    }

    /// #2775：行为按钮 Hint（C# `HeroDialogs.cs:774` `HeroBehaviourFormat` +
    /// `HeroBehaviour` 枚举本地化名）
    #[test]
    fn behaviour_hint_matches_csharp_enum_names() {
        assert_eq!(
            [
                behaviour_hint(0),
                behaviour_hint(1),
                behaviour_hint(2),
                behaviour_hint(3)
            ],
            [
                "英雄行为：攻击",
                "英雄行为：反击",
                "英雄行为：跟随",
                "英雄行为：自动"
            ],
            "0=攻击 1=反击 2=跟随 3=自动（C# Enum.Parse 顺序）"
        );
        // C# `ClientTextKeys.HeroBehaviour_Custom` = 自动（此前 Bevy 写作「自定义」属自造文案）
        assert_eq!(
            behaviour_name(mir2_shared::enums::HeroBehaviour::Custom),
            "自动"
        );
    }

    /// #2791 单元①：管理窗槽位原点 = C# `98 + 60*(i%4)` / `61 + 40*(i/4)`
    /// （`HeroDialogs.cs:839`，`Info` 落地后再 +5,+5）
    #[test]
    fn hero_manage_slot_origin_matches_csharp() {
        assert_eq!(hero_slot_origin(0), (98.0, 61.0));
        assert_eq!(hero_slot_origin(1), (158.0, 61.0));
        assert_eq!(hero_slot_origin(3), (278.0, 61.0));
        assert_eq!(hero_slot_origin(4), (98.0, 101.0));
        assert_eq!(hero_slot_origin(7), (278.0, 101.0));
    }

    /// #2791 单元①：头像精灵 index（C# `HeroAvatar(class,gender)+370`）与
    /// `Hint = info.ToString()`（枚举名小写、不本地化，`Shared/Data/ClientData.cs:652-655`）
    #[test]
    fn hero_manage_avatar_index_and_hint_match_csharp() {
        assert_eq!(HeroAvatarImages::avatar_index(0, 0), 1770, "战士男");
        assert_eq!(HeroAvatarImages::avatar_index(4, 0), 1774, "弓手男");
        assert_eq!(HeroAvatarImages::avatar_index(0, 1), 1780, "战士女");
        assert_eq!(HeroAvatarImages::avatar_index(4, 1), 1784, "弓手女");

        let hero = ClientHeroInformation {
            index: 1,
            name: "示范英雄".to_string(),
            level: 20,
            class: mir2_shared::enums::MirClass::Warrior,
            gender: mir2_shared::enums::MirGender::Male,
        };
        assert_eq!(hero_manage_hint(&hero), "示范英雄\nLevel 20 male warrior");
        assert_eq!(
            hero_manage_confirm_text(&hero.name),
            "是否要将 示范英雄 设为你的当前英雄？"
        );
    }

    /// #2791 单元①：系统级——槽位显隐/Hint 按 `max_count` 名额与列表（C# `HeroDialogs.cs:840-851`），
    /// 点击占用槽 → MakeActiveHero 确认，Yes → `C.ChangeHero{hero_index}`
    #[test]
    fn hero_manage_slots_visibility_hint_and_switch_flow() {
        use bevy::ecs::system::RunSystemOnce;

        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        let mut world = World::new();
        world.insert_resource(HeroState {
            // 总名额 3（含主角色）→ 可用槽 0..=1；槽 2..7 超名额
            max_count: 3,
            managing: true,
            heroes: vec![ClientHeroInformation {
                index: 1,
                name: "示范英雄".to_string(),
                level: 20,
                class: mir2_shared::enums::MirClass::Warrior,
                gender: mir2_shared::enums::MirGender::Male,
            }],
            ..Default::default()
        });
        world.insert_resource(NetConnection {
            to_server: Some(tx),
            ..Default::default()
        });
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<crate::game::chat::ChatState>();
        world.init_resource::<TextInputState>();
        world.insert_resource(HeroAvatarImages::default());

        let spawn_slot = |world: &mut World, i: usize, inter: Interaction| {
            world
                .spawn((
                    Button,
                    HeroManageSlot(i),
                    crate::ui::tooltip::UiHint {
                        text: String::new(),
                    },
                    inter,
                    Node::default(),
                    ImageNode::default(),
                ))
                .id()
        };
        let slot0 = spawn_slot(&mut world, 0, Interaction::None);
        let slot1 = spawn_slot(&mut world, 1, Interaction::None);
        let slot2 = spawn_slot(&mut world, 2, Interaction::None);
        world.spawn((Text::new(String::new()), HeroManageConfirmText));
        let yes = world
            .spawn((
                Button,
                HeroManageConfirmYes,
                Interaction::None,
                Node::default(),
                ImageNode::default(),
            ))
            .id();

        world
            .run_system_once(hero_manage_system)
            .expect("hero_manage_system 应成功");

        // 槽 0：占用 → 头像精灵 36x30 @(+5,+5)，Hint = info.ToString()
        let e = world.entity(slot0);
        assert_eq!(*e.get::<Visibility>().unwrap(), Visibility::Visible);
        assert_eq!(e.get::<Node>().unwrap().left, Val::Px(103.0));
        assert_eq!(e.get::<Node>().unwrap().top, Val::Px(66.0));
        assert_eq!(e.get::<Node>().unwrap().width, Val::Px(36.0));
        assert_eq!(
            e.get::<crate::ui::tooltip::UiHint>().unwrap().text,
            "示范英雄\nLevel 20 male warrior"
        );
        // 槽 1：有名额但空 → C# `Info = null` → 隐藏
        assert_eq!(
            *world.entity(slot1).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
        // 槽 2：超名额 → 恒显空框 60x41 @(218,61)、无 Hint
        let e2 = world.entity(slot2);
        assert_eq!(*e2.get::<Visibility>().unwrap(), Visibility::Visible);
        assert_eq!(e2.get::<Node>().unwrap().left, Val::Px(218.0));
        assert_eq!(e2.get::<Node>().unwrap().width, Val::Px(60.0));
        assert!(e2
            .get::<crate::ui::tooltip::UiHint>()
            .unwrap()
            .text
            .is_empty());

        // 点占用槽 → 弹 MakeActiveHero 询问框
        world.entity_mut(slot0).insert(Interaction::Pressed);
        world
            .run_system_once(hero_manage_system)
            .expect("hero_manage_system 应成功");
        assert_eq!(world.resource::<HeroState>().confirm_slot, Some(0));

        // 询问框 Yes → C.ChangeHero{hero_index = 1}
        // 先松开槽位（`run_system_once` 每次重建 Local，按住会再次触发点击）
        world.entity_mut(slot0).insert(Interaction::None);
        world.entity_mut(yes).insert(Interaction::Pressed);
        world
            .run_system_once(hero_manage_system)
            .expect("hero_manage_system 应成功");
        assert_eq!(world.resource::<HeroState>().confirm_slot, None);
        let sent = rx.try_recv().expect("应发出 C.ChangeHero");
        assert_eq!(
            *sent.last().unwrap(),
            1u8,
            "ChangeHero{{hero_index}} 末字节 = 英雄 index"
        );
    }

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

    /// #2892 批C：`S.HeroCreateRequest` → 打开原版新建角色对话框的**英雄模式**
    /// （C# `GameScene.cs:6044-6052` 显隐职业钮后 `NewHeroDialog.Show()`）
    #[test]
    fn hero_create_request_opens_new_char_dialog_in_hero_mode() {
        use crate::network::server_event::ServerEvent;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<HeroState>();
        app.init_resource::<crate::ui::new_character::NewCharState>();
        app.add_message::<ServerEvent>();
        app.add_systems(Update, hero_server_events);

        // 战士/刺客/弓箭手不可创建 → 职业钮隐藏，默认职业落到第一个可创建者（法师）
        app.world_mut()
            .write_message(ServerEvent::HeroCreateRequested {
                can_create_class: vec![false, true, true, false, false],
            });
        app.update();
        let nc = app
            .world()
            .resource::<crate::ui::new_character::NewCharState>();
        assert!(
            nc.hero_mode,
            "应进入英雄模式（标题 Title[847]、OK 发 C.NewHero）"
        );
        assert!(nc.visible, "应弹出对话框");
        assert_eq!(
            nc.can_create_class,
            [false, true, true, false, false],
            "职业可选性按包内容"
        );
        assert_eq!(
            nc.class,
            mir2_shared::enums::MirClass::Wizard,
            "默认职业应落到第一个可创建职业（Warrior 不可选 → Wizard）"
        );

        // 创建成功（result=10）→ 收起对话框并退出英雄模式
        app.world_mut()
            .write_message(ServerEvent::NewHeroResult { result: 10 });
        app.update();
        let nc = app
            .world()
            .resource::<crate::ui::new_character::NewCharState>();
        assert!(!nc.visible && !nc.hero_mode, "创建成功后应收起并复位模式");
    }
}
