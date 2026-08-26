// ============================================================================
// 技能系统（M13 续）：已学技能列表 + F1-F8 施放 + 快捷键绑定
// UI 交互参考：C# MainDialogs.cs（magic.Key == 1..8 = F1..F8 快捷施放）
// 网络参考：SharedRust packets/client/combat.rs::Magic / MagicKey
// ============================================================================

use bevy::prelude::*;
use mir2_shared::data::client_data::ClientMagic;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::sets::GameSet;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};
use crate::ui::sprite_ui::{
    spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont, UiImageCache,
};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_label, spawn_panel, spawn_scroll_bar_ui,
    UiScrollList,
};
use mir2_shared::enums::Spell;

use crate::actor::ActorAnim;
use crate::game::player_state::StatusFlags;
use crate::network::{NetConnection, SessionState};
use crate::scenes::AppState;

/// 开关技能列表（#242：C# GameScene UseMagic 的 toggle 分支）
pub const TOGGLE_SPELLS: [Spell; 4] = [
    Spell::Thrusting,
    Spell::HalfMoon,
    Spell::CrossHalfMoon,
    Spell::DoubleSlash,
];

/// 技能键名（C# MainDialogs.cs:3279/3423-3426 KeyLabel 公式）：
/// `Prefixes[(key-1)/8]` = {"", "CTRL", "Shift"} +（key>8 时换行）`F{(key-1)%8+1}`。
/// Bevy 列表/快捷栏为单行文本，C# 的 "CTRL␊F1" 两行形式单行化为 "Ctrl F1"。
/// key=0 → 空（未绑定）；key≥25 超 C# Prefixes 范围 → 空（防御）。
pub fn skill_key_name(key: u8) -> String {
    match key {
        0 => String::new(),
        1..=8 => format!("F{}", key),
        9..=16 => format!("Ctrl F{}", key - 8),
        17..=24 => format!("Shift F{}", key - 16),
        _ => String::new(),
    }
}

/// #1610：C# PlayerObject.cs Attack1 动作——按开关状态选择随 C.Attack 发送的 Spell
/// （HalfMoon → Spell.HalfMoon、CrossHalfMoon → Spell.CrossHalfMoon、DoubleSlash → Spell.DoubleSlash；否则 None）
pub fn toggled_attack_spell(spell_toggles: &[(Spell, bool)]) -> Spell {
    let on = |s: Spell| spell_toggles.iter().any(|(sp, v)| *sp == s && *v);
    if on(Spell::HalfMoon) {
        Spell::HalfMoon
    } else if on(Spell::CrossHalfMoon) {
        Spell::CrossHalfMoon
    } else if on(Spell::DoubleSlash) {
        Spell::DoubleSlash
    } else {
        Spell::None
    }
}

/// 是否开关技能（#242）
pub fn is_toggle_spell(spell: Spell) -> bool {
    TOGGLE_SPELLS.contains(&spell)
}

/// #1376：技能冷却（S.MagicDelay → spell → (剩余秒, 总秒)）
#[derive(Resource, Default)]
pub struct MagicCooldowns {
    pub map: Vec<(Spell, f32, f32)>,
}

impl MagicCooldowns {
    pub fn set(&mut self, spell: Spell, delay_ms: i64) {
        let total = delay_ms.max(0) as f32 / 1000.0;
        if let Some(e) = self.map.iter_mut().find(|(s, _, _)| *s == spell) {
            *e = (spell, total, total);
        } else {
            self.map.push((spell, total, total));
        }
    }
    /// 剩余比例 0..1（无冷却=0）
    pub fn fraction(&self, spell: Spell) -> f32 {
        self.map
            .iter()
            .find(|(s, _, _)| *s == spell)
            .map(|(_, r, t)| {
                if *t > 0.0 {
                    (*r / *t).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0)
    }

    /// 剩余秒（无冷却=None；C# ProcessSkillDelay timeLeft）
    pub fn remaining(&self, spell: Spell) -> Option<f32> {
        self.map
            .iter()
            .find(|(s, _, _)| *s == spell)
            .map(|(_, r, _)| *r)
    }

    /// 总冷却秒（无冷却=None；C# ProcessSkillDelay Delay）
    pub fn total(&self, spell: Spell) -> Option<f32> {
        self.map
            .iter()
            .find(|(s, _, _)| *s == spell)
            .map(|(_, _, t)| *t)
    }
}

/// #1376：冷却递减（Time 驱动）
fn magic_cooldown_system(time: Res<Time>, mut cd: ResMut<MagicCooldowns>) {
    cd.map.retain_mut(|(_, r, _)| {
        *r -= time.delta_secs();
        *r > 0.0
    });
}

/// 已学技能列表（NewMagic 包写入）
#[derive(Resource, Default)]
pub struct MagicsState {
    pub magics: Vec<ClientMagic>,
    /// #242 开关技能状态（spell → 开/关，S.SpellToggle 同步；Spell 无 Hash 用 Vec）
    pub spell_toggles: Vec<(Spell, bool)>,
}

impl MagicsState {
    /// 查询开关技能状态（#242）
    pub fn toggle_state(&self, spell: Spell) -> bool {
        self.spell_toggles
            .iter()
            .find(|(s, _)| *s == spell)
            .map(|(_, v)| *v)
            .unwrap_or(false)
    }

    /// 翻转开关技能状态并返回新状态（#242）
    pub fn toggle_spell(&mut self, spell: Spell) -> bool {
        let new = !self.toggle_state(spell);
        if let Some(entry) = self.spell_toggles.iter_mut().find(|(s, _)| *s == spell) {
            entry.1 = new;
        } else {
            self.spell_toggles.push((spell, new));
        }
        new
    }
}

impl MagicsState {
    /// 新增/覆盖技能（服务端以 spell 为唯一键）
    pub fn upsert(&mut self, m: ClientMagic) {
        if let Some(e) = self.magics.iter_mut().find(|x| x.spell == m.spell) {
            *e = m;
        } else {
            self.magics.push(m);
        }
    }

    /// 按 spell 查技能
    pub fn by_spell(&self, spell: Spell) -> Option<&ClientMagic> {
        self.magics.iter().find(|m| m.spell == spell)
    }

    /// 按快捷键 1..8 查绑定技能（原版 C#：m.Key == 槽位，F1=1）
    pub fn by_key(&self, key: u8) -> Option<&ClientMagic> {
        if let Some(m) = self.magics.iter().find(|m| m.key == key) {
            return Some(m);
        }
        // 兜底：未绑定时按技能列表顺序取
        if (1..=8).contains(&key) {
            self.magics.get(key as usize - 1)
        } else {
            None
        }
    }

    /// 绑定快捷键（原版 C# AssignKeyPanel.SaveButton 语义）：
    /// 先清除所有占用该键的技能，再设置目标技能；返回目标旧键（发包用）
    pub fn assign_key(&mut self, spell: Spell, key: u8) -> Option<u8> {
        let old = self.by_spell(spell).map(|m| m.key);
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
}

#[derive(Component)]
pub struct SkillsWidget;

#[derive(Component)]
pub struct SkillsClose;

#[derive(Component)]
pub struct SkillsLine(usize);

/// 技能快捷栏根实体（整栏随拖动移动、随设置开关显隐；对齐 C# SkillBarDialog 整体 Show/Hide）。
/// .0 = 栏号（C# BarIndex 0/1；C# 共两条栏，Settings.SkillBar=true 时全部显示）
#[derive(Component)]
pub struct SkillBarRoot(pub usize);

/// 技能快捷栏格子锚点（子实体挂图标/冷却/键名标签）。
/// .0 = 绝对格号 bar*8+i（0..15）；对应 C# m.Key = 值+1（bar1:1..8、bar2:9..16）
#[derive(Component)]
pub struct SkillBarSlot(pub usize);

/// 技能快捷栏图标（格子子实体；C# Cells[i] = MagIcon[magic.Icon*2] 自然尺寸）
#[derive(Component)]
pub struct SkillBarIcon(pub usize);

/// 技能快捷栏冷却遮罩（格子子实体；C# CoolDowns[i] = Prguse2[1260+frame] 60% 透明）
#[derive(Component)]
pub struct SkillBarCooldown(pub usize);

/// 技能快捷栏按键标签（格子子实体；C# KeyNameLabels[i]，格内有技能时置空）
#[derive(Component)]
pub struct SkillBarKey(pub usize);

/// 技能栏位置状态（C# SkillBarDialog ×2：Movable + SkillbarLocation[2,2] 持久化）
#[derive(Resource)]
pub struct SkillBarState {
    /// 各栏左上角屏幕坐标（C# Settings.SkillbarLocation，默认 bar0={0,0}、bar1={216,0}，
    /// Settings.cs:163；DialogProcess 每帧应用 L1325-1332）
    pub pos: [(f32, f32); 2],
    /// 拖动中：(栏号, 按下点与该栏 pos 的偏移)
    pub drag_offset: Option<(usize, (f32, f32))>,
    /// 按下时命中的格子（绝对 0..15 = bar*8+i；C# 格子是子控件：按在格子上走点击流程，不触发拖动）
    pub pressed_slot: Option<usize>,
    /// 格子点击施法请求（C# Cells[i].Click → UseSpell(i+1+8*BarIndex)，键位即 C# m.Key 1..16），
    /// 绝对 0..15，由 skill_bar_system 消费（键 = 值+1）
    pub pending_cast: Option<usize>,
}

/// 无 INI 时的默认位置（C# Settings.cs:163 `SkillbarLocation = {{0,0},{216,0}}`）
pub const SKILLBAR_DEFAULT_POS: [(f32, f32); 2] = [(0.0, 0.0), (216.0, 0.0)];
/// 某栏存档无效时的回落位置。C# 源码字面：DialogProcess L1329 `continue` 跳过赋值 →
/// 栏保持构造器 Location=(0, BarIndex*20)（MainDialogs.cs:1533）。但 C# 对象初始化器
/// `new SkillBarDialog { BarIndex = 1 }` 的 BarIndex 赋值发生在构造器**之后**，构造器
/// 执行时恒为 0，故 C# 运行时两栏实际都回落 (0,0)（重叠）。此处取 (0, bar*20) 是
/// **有意偏离**字面运行语义，避免两栏完全重叠不可用；仅 bar1 存档越界的极端场景可见差异。
const SKILLBAR_CTOR_POS: [(f32, f32); 2] = [(0.0, 0.0), (0.0, 20.0)];

impl Default for SkillBarState {
    fn default() -> Self {
        Self {
            pos: SKILLBAR_DEFAULT_POS,
            drag_offset: None,
            pressed_slot: None,
            pending_cast: None,
        }
    }
}

impl SkillBarState {
    /// 从 Mir2Config.ini 解析（C# [Game] Skillbar{i}X/Skillbar{i}Y，i∈{0,1}，Settings.cs:267-270）
    pub fn from_ini(content: &str) -> Self {
        use crate::game::dialogs::settings_file::ini_str;
        let mut s = Self::default();
        for bar in 0..2usize {
            if let Some(v) = ini_str(content, "Game", &format!("Skillbar{bar}X"))
                .and_then(|v| v.parse::<f32>().ok())
            {
                s.pos[bar].0 = v;
            }
            if let Some(v) = ini_str(content, "Game", &format!("Skillbar{bar}Y"))
                .and_then(|v| v.parse::<f32>().ok())
            {
                s.pos[bar].1 = v;
            }
            // C# GameScene.DialogProcess（L1328-1331）：存档越界（x > Resolution-100=924 或
            // y > 700，严格大于）则 continue 跳过赋值 → 栏保持构造器 Location=(0, BarIndex*20)。
            // 负值兜底：C# 运行时拖动钳制（OnMouseMove L910-913）保证永不产生负坐标；
            // Bevy 旧版本无钳制可能存过负值 → 一并按无效处理回落（与越界同一语义）。
            // 非有限值兜底："NaN"/"inf" 能被 parse::<f32>() 成功解析，而 NaN 的全序比较全为
            // false，单靠越界判定拦不住 → 栏会被定位到 NaN 永不可见（#2517）。
            if !s.pos[bar].0.is_finite()
                || !s.pos[bar].1.is_finite()
                || s.pos[bar].0 > 924.0
                || s.pos[bar].1 > 700.0
                || s.pos[bar].0 < 0.0
                || s.pos[bar].1 < 0.0
            {
                s.pos[bar] = SKILLBAR_CTOR_POS[bar];
            }
        }
        s
    }

    /// 启动时加载（C# Settings.Load）
    pub fn load() -> Self {
        Self::from_ini(&crate::game::dialogs::settings_file::load_ini())
    }

    /// 保存（C# Settings.Save：SkillbarLocation → [Game] Skillbar{i}X/Y 两栏，merge 写回）
    pub fn save(&self) {
        use crate::game::dialogs::settings_file::{set_ini_value, write_ini};
        let mut content = crate::game::dialogs::settings_file::load_ini();
        for bar in 0..2usize {
            content = set_ini_value(
                &content,
                "Game",
                &format!("Skillbar{bar}X"),
                &self.pos[bar].0.round().to_string(),
            );
            content = set_ini_value(
                &content,
                "Game",
                &format!("Skillbar{bar}Y"),
                &self.pos[bar].1.round().to_string(),
            );
        }
        write_ini(&content);
        tracing::debug!("⚙️ 技能栏位置已保存到 Mir2Config.ini");
    }
}

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MagicsState>();
        app.init_resource::<MagicCooldowns>();
        app.insert_resource(SkillBarState::load());
        app.add_systems(OnEnter(AppState::Game), spawn_skills_window);
        app.add_systems(OnEnter(AppState::Game), spawn_skill_bar);
        app.add_systems(OnExit(AppState::Game), cleanup_skills_window);
        // #2632：原先 5 次独立 add_systems 各挂 .run_if(in_state(Game))，归并为
        // Skills 集统一门控。这些系统彼此本无显式排序，合并进同一元组（非 .chain()）
        // 不增删任何 ordering，保持行为等价。
        app.configure_sets(Update, GameSet::Skills.run_if(in_state(AppState::Game)));
        app.add_systems(
            Update,
            // #148 技能快捷键改由 dialog_hotkey_system 按键位设置处理（可重绑）
            (
                skills_window_system,
                skill_bar_show_system,
                skill_bar_icon_system,
                skill_bar_cooldown_system,
                ui_button_system,
                skills_server_events,
                skill_bar_system,
                magic_cooldown_system,
                skill_bar_pointer_system,
            )
                .in_set(GameSet::Skills),
        );
    }
}

/// F1-F8 施放绑定技能（原版 C#：F1-F8 → UserMagic(key) → Magic 包）
/// M37：有选中攻击目标时朝目标施放（弹道类魔法 target_id + 目标位置），
/// 无目标时朝当前朝向施放（fallback）。
fn skill_bar_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    // #2633 批次4 步4：fishing/paralysis 施法门读改 StatusFlags（本系统不再用 HudState）
    flags: Query<&StatusFlags, With<crate::actor::LocalPlayer>>,
    mut bar: ResMut<SkillBarState>,
    mut magics: ResMut<MagicsState>,
    net: Res<NetConnection>,
    session: Res<SessionState>,
    control: Res<crate::game::player_control::ControlState>,
    mut chat: ResMut<crate::game::chat::ChatState>,
    actors: Query<(&crate::actor::NetObjectId, &Transform), Without<crate::actor::LocalPlayer>>,
    mut players: Query<
        (Entity, &Transform, &mut ActorAnim),
        (
            With<crate::actor::LocalPlayer>,
            With<crate::actor::NetObjectId>,
        ),
    >,
) {
    const F_KEYS: [KeyCode; 8] = [
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
    ];
    // 格子点击施法请求（C# Cells[i].Click → UseSpell(i+1+8*BarIndex)）与 F 键同一路径。
    // 键位（C# KeyBindSettings:242-276）：bar1 = F1..F8；bar2 = Ctrl+F1..F8 → 键 9..16
    let pending = bar.pending_cast.take();
    // #1600/#1616：C# GameScene.CheckInput——钓鱼/麻痹/冰冻锁定施法输入
    // #2633 步4：读改 StatusFlags；实体缺失视同未锁（同原 hud 默认 false，放行施法）
    let cast_locked = flags.single().map(|f| f.fishing || f.paralysis).unwrap_or(false);
    if cast_locked {
        return;
    }
    let ctrl_held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let slot = pending.or_else(|| {
        F_KEYS
            .iter()
            .position(|k| keys.just_pressed(*k))
            .map(|i| if ctrl_held { i + 8 } else { i })
    });
    let Some(slot) = slot else {
        return;
    };
    let Some(magic) = magics.by_key(slot as u8 + 1).cloned() else {
        return;
    };
    // #242：开关技能（刺杀/半月/十字斩/双斩）→ 本地切换 + C.SpellToggle（C# UseMagic toggle 分支）
    if is_toggle_spell(magic.spell) {
        let new_state = magics.toggle_spell(magic.spell);
        net.send_packet(&mir2_shared::packets::client::combat::SpellToggle {
            spell: magic.spell,
            can_use: new_state,
        });
        let state_txt = if new_state { "开启" } else { "关闭" };
        chat.add_line(
            format!("{} {}", state_txt, magic.name),
            Color::srgb(0.4, 1.0, 0.4),
            crate::game::chat::ChatChannel::System,
        );
        tracing::info!(
            "🔄 技能开关 {} {} ({:?})",
            state_txt,
            magic.name,
            magic.spell
        );
        return;
    }
    // 玩家当前瓦片位置（以本地玩家实体 Transform 为准，服务器坐标更稳）
    // #1596：single_mut 拿到本地实体，施法时清除寻路路径（C# CanMove=false）
    let Ok((pe, ptf, mut anim)) = players.single_mut() else {
        return;
    };
    let (px, py) = crate::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
    // 有选中目标 → 朝目标施放
    let mut target_id = 0u32;
    let mut tx = px;
    let mut ty = py;
    let mut cast_dir = mir2_shared::enums::MirDirection::try_from(
        session.self_position.map(|(_, _, d)| d).unwrap_or(4),
    )
    .unwrap_or(mir2_shared::enums::MirDirection::Down);
    if let Some(tid) = control.attack_target {
        if let Some((_, tf)) = actors.iter().find(|(id, _)| id.0 == tid) {
            let (ttx, tty) =
                crate::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
            target_id = tid;
            tx = ttx;
            ty = tty;
            cast_dir = crate::game::movement::direction_from_delta(
                (ttx - px).signum(),
                (tty - py).signum(),
            )
            .unwrap_or(cast_dir);
        }
    }
    // #1596：C# 施法替代移动动作（CanMove=false）——清除寻路路径并回站立
    commands
        .entity(pe)
        .remove::<crate::game::movement::LocalMove>();
    anim.action = mir2_shared::enums::MirAction::Standing;
    anim.frame_index = 0;

    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        object_id: 0, // #2573：C# Magic.ObjectID（0=本人；英雄派发待真实对象化）
        spell_target_lock: false,
        spell: magic.spell,
        direction: cast_dir,
        target_id,
        location: mir2_shared::Point { x: tx, y: ty },
    });
    tracing::info!(
        "✨ {} 施放 {} ({:?}) 目标={} @ ({},{})",
        if slot >= 8 {
            format!("Ctrl + F{}", slot - 7)
        } else {
            format!("F{}", slot + 1)
        },
        magic.name,
        magic.spell,
        target_id,
        tx,
        ty
    );
}
#[cfg(test)]
mod tests {
    use super::*;

    /// C# 技能键名公式锚点（MainDialogs.cs:3279 Prefixes={"","CTRL","Shift"} +
    /// :3423-3426 `Prefixes[(key-1)/8] + (key>8?换行:"") + F{(key-1)%8+1}`）：
    /// 1..8="F*"；9..16="Ctrl F*"（C# 两行 "CTRL␊F*" 的单行化）；17..24="Shift F*"。
    /// 旧实现技能列表只认 1..8，9..16 显示为无键（#2550）。
    #[test]
    fn skill_key_name_matches_csharp() {
        assert_eq!(skill_key_name(0), "");
        assert_eq!(skill_key_name(1), "F1");
        assert_eq!(skill_key_name(8), "F8");
        assert_eq!(skill_key_name(9), "Ctrl F1");
        assert_eq!(skill_key_name(16), "Ctrl F8");
        assert_eq!(skill_key_name(17), "Shift F1");
        assert_eq!(skill_key_name(24), "Shift F8");
        // ≥25 超 C# Prefixes 下标（C# 会越界）→ 防御性空串
        assert_eq!(skill_key_name(25), "");
    }

    #[test]
    fn skill_bar_state_parse() {
        let content = "[Game]\nSkillbar0X=123\nSkillbar0Y=456\nSkillbar1X=300\nSkillbar1Y=40\n";
        let s = SkillBarState::from_ini(content);
        assert_eq!(s.pos, [(123.0, 456.0), (300.0, 40.0)]);
    }

    #[test]
    fn skill_bar_state_defaults_when_missing() {
        let s = SkillBarState::from_ini("");
        let d = SkillBarState::default();
        assert_eq!(s.pos, d.pos);
        // C# Settings.cs:163 SkillbarLocation = {{0,0},{216,0}}（两栏并排左上角）
        assert_eq!(d.pos[0], (0.0, 0.0));
        assert_eq!(d.pos[1], (216.0, 0.0));
    }

    /// 旧版（单栏）存档只有 Skillbar0X/Y：bar0 旧值原样保留，bar1 缺失用默认 (216,0)。
    /// 键名未变（git 历史旧版就是 Skillbar0X/Skillbar0Y）→ 旧存档无损升级。
    #[test]
    fn legacy_single_bar_ini_upgrades_losslessly() {
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=182\nSkillbar0Y=64\n");
        assert_eq!(s.pos[0], (182.0, 64.0), "旧 bar0 存档保留");
        assert_eq!(s.pos[1], (216.0, 0.0), "bar1 无存档用默认");
    }

    /// C# GameScene.DialogProcess（L1325-1332）：某栏存档越界 → continue 跳过赋值，
    /// 该栏保持构造器 Location=(0, BarIndex*20)（MainDialogs.cs:1533）；另一栏不受影响
    #[test]
    fn skill_bar_state_out_of_bounds_falls_back() {
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=925\nSkillbar0Y=100\n");
        assert_eq!(s.pos[0], (0.0, 0.0), "x>924 回落 bar0 构造默认 (0,0)");
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=100\nSkillbar0Y=701\n");
        assert_eq!(s.pos[0], (0.0, 0.0), "y>700 回落 bar0 构造默认 (0,0)");
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=924\nSkillbar0Y=700\n");
        assert_eq!(s.pos[0], (924.0, 700.0), "边界值保留（C# 为严格 >）");
        // 负值兜底：旧版本无拖动钳制可能存过负坐标 → 视为无效回落默认（栏永不可拖出屏幕）
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=-50\nSkillbar0Y=100\n");
        assert_eq!(s.pos[0], (0.0, 0.0), "负 x 应回落默认");
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=100\nSkillbar0Y=-10\n");
        assert_eq!(s.pos[0], (0.0, 0.0), "负 y 应回落默认");
        // bar1 越界 → 回落 bar1 构造默认 (0,20)，且不影响 bar0
        let s = SkillBarState::from_ini(
            "[Game]\nSkillbar0X=100\nSkillbar0Y=50\nSkillbar1X=999\nSkillbar1Y=50\n",
        );
        assert_eq!(s.pos[0], (100.0, 50.0), "bar0 有效值不受 bar1 影响");
        assert_eq!(s.pos[1], (0.0, 20.0), "bar1 越界回落构造默认 (0,20)");
    }

    /// 键名标签文本 = C# GetKey(BarIndex, i)（默认键位：bar1 F1..F8；bar2 Ctrl+F1..F8，
    /// 格式 "修饰符 + 键名"，KeyBindSettings.cs:383-407）
    #[test]
    fn skill_key_labels_match_csharp_defaults() {
        assert_eq!(skill_key_label(0, 0), "F1");
        assert_eq!(skill_key_label(0, 7), "F8");
        assert_eq!(skill_key_label(1, 0), "C+F1");
        assert_eq!(skill_key_label(1, 7), "C+F8");
    }

    /// C# MirControl.OnMouseMove（L901-913）：拖动位置钳制在父容器（全屏）内，栏不可拖出屏幕。
    /// 上界 = 1024-216 / 768-28 = (808,740)，下界 0。此处断言钳制常量与栏/屏尺寸的 C# 关系。
    #[test]
    fn skill_bar_drag_clamp_bounds_match_csharp() {
        assert_eq!(SKILL_BAR_MAX_X, 1024.0 - SKILL_BAR_W, "X 上界 = 屏宽-栏宽");
        assert_eq!(SKILL_BAR_MAX_Y, 768.0 - SKILL_BAR_H, "Y 上界 = 屏高-栏高");
        assert_eq!((SKILL_BAR_MAX_X, SKILL_BAR_MAX_Y), (808.0, 740.0));
        // 栏在钳制范围内始终完整可见
        assert!(SKILL_BAR_MAX_X + SKILL_BAR_W <= 1024.0);
        assert!(SKILL_BAR_MAX_Y + SKILL_BAR_H <= 768.0);
    }

    /// C# Update()：Cells[i].Index = magic.Icon * 2（MagIcon 成对帧）
    #[test]
    fn magic_icon_index_is_doubled() {
        assert_eq!(magic_icon_index(0), 0);
        assert_eq!(magic_icon_index(5), 10);
        assert_eq!(magic_icon_index(111), 222);
    }

    /// C# ProcessSkillDelay（L1735-1741）：delayPerFrame=(int)(Delay/22)、startFrame=22-(int)(timeLeft/delayPerFrame)，
    /// startFrame∈[0,22]（末帧 22 → Prguse2[1282]，timeLeft<100ms 由调用方隐藏）。
    #[test]
    fn cooldown_frame_sweeps_with_remaining() {
        // Delay=5000ms → per_frame=(int)(5000/22)=227
        assert_eq!(
            cooldown_frame(5.0, 5.0),
            0,
            "刚施放（timeLeft=Delay）从第0帧开始"
        );
        assert_eq!(
            cooldown_frame(2.5, 5.0),
            11,
            "剩余一半 2500/227=11 → 22-11=11"
        );
        // C# 审查反例：Delay=5000、timeLeft=4773 → (int)(4773/227)=21 → startFrame=1
        assert_eq!(
            cooldown_frame(4.773, 5.0),
            1,
            "C# 反例 timeLeft=4773 → 第1帧"
        );
        assert_eq!(
            cooldown_frame(0.1, 5.0),
            22,
            "timeLeft=100ms → 22-0=末帧22（idx 1282）"
        );
        assert_eq!(
            cooldown_frame(0.0, 5.0),
            22,
            "timeLeft=0 → 钳位末帧22（调用方负责隐藏）"
        );
    }

    /// C# Cells[i] bbox：@(i*25+15, 3) 24x22；命中格子不触发拖动；两栏绝对格号 bar*8+i
    #[test]
    fn skill_slot_hit_test() {
        let bar = SkillBarState::default();
        // 第 0 格 @(15,3) 24x22：中心命中
        assert_eq!(skill_slot_at(&bar, Vec2::new(27.0, 14.0)), Some(0));
        // 第 1 格 @(40,3)
        assert_eq!(skill_slot_at(&bar, Vec2::new(52.0, 14.0)), Some(1));
        // 第 7 格 @(190,3) 右缘
        assert_eq!(skill_slot_at(&bar, Vec2::new(213.0, 24.0)), Some(7));
        // bar2 默认 (216,0)：第 0 格 @(216+15,3)=(231,3) → 绝对格号 8
        assert_eq!(skill_slot_at(&bar, Vec2::new(243.0, 14.0)), Some(8));
        // bar2 第 7 格 @(216+190,3)=(406,3) → 绝对格号 15
        assert_eq!(skill_slot_at(&bar, Vec2::new(429.0, 24.0)), Some(15));
        // 格间缝隙（x=39 在第0格右缘与第1格左缘之间）不命中
        assert_eq!(skill_slot_at(&bar, Vec2::new(39.5, 14.0)), None);
        // 格子上方（y<3，按钮/标签带）不命中格子
        assert_eq!(skill_slot_at(&bar, Vec2::new(27.0, 1.0)), None);
    }

    fn magic(spell: Spell, key: u8) -> ClientMagic {
        ClientMagic {
            name: String::new(),
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
            key,
            experience: 0,
            delay: 0,
            range: 0,
            cast_time: 0,
        }
    }

    /// 原版 C# SaveButton：清除占用同键的技能，再设置目标，返回旧键
    #[test]
    fn assign_key_clears_conflicts_and_returns_old() {
        let mut s = MagicsState::default();
        s.upsert(magic(Spell::Fencing, 1));
        s.upsert(magic(Spell::Slaying, 0));
        s.upsert(magic(Spell::Thrusting, 3));

        // Fencing 从 F1 改绑 F3：占用 F3 的 Thrusting 应被清 0
        let old = s.assign_key(Spell::Fencing, 3);
        assert_eq!(old, Some(1));
        assert_eq!(s.by_spell(Spell::Fencing).unwrap().key, 3);
        assert_eq!(s.by_spell(Spell::Thrusting).unwrap().key, 0);
        assert_eq!(s.by_spell(Spell::Slaying).unwrap().key, 0);

        // 绑定到 0（None）
        let old = s.assign_key(Spell::Fencing, 0);
        assert_eq!(old, Some(3));
        assert_eq!(s.by_spell(Spell::Fencing).unwrap().key, 0);
    }

    /// 未找到技能时返回 None，不影响其他技能
    #[test]
    fn assign_key_unknown_spell_is_noop() {
        let mut s = MagicsState::default();
        s.upsert(magic(Spell::Fencing, 1));
        let old = s.assign_key(Spell::Slaying, 5);
        assert_eq!(old, None);
        assert_eq!(s.by_spell(Spell::Fencing).unwrap().key, 1);
    }

    /// #2517："NaN"/"inf" 能被 parse::<f32>() 成功解析，而 NaN 的全序比较全为 false，
    /// 单靠越界判定拦不住 → 栏会被定位到 NaN 永不可见。非有限值必须回落默认。
    #[test]
    fn skill_bar_state_non_finite_falls_back() {
        for v in ["NaN", "nan", "inf", "-inf", "Infinity"] {
            let cx = format!("[Game]\nSkillbar0X={v}\nSkillbar0Y=100\n");
            assert_eq!(
                SkillBarState::from_ini(&cx).pos[0],
                (0.0, 0.0),
                "Skillbar0X={v} 非有限/越界应回落默认"
            );
            let cy = format!("[Game]\nSkillbar0X=100\nSkillbar0Y={v}\n");
            assert_eq!(
                SkillBarState::from_ini(&cy).pos[0],
                (0.0, 0.0),
                "Skillbar0Y={v} 非有限/越界应回落默认"
            );
            // bar1 同样防护，回落其构造默认 (0,20)
            let c1 = format!("[Game]\nSkillbar1X={v}\nSkillbar1Y=100\n");
            assert_eq!(
                SkillBarState::from_ini(&c1).pos[1],
                (0.0, 20.0),
                "Skillbar1X={v} 非有限/越界应回落 bar1 构造默认"
            );
        }
    }

    /// #2517 回归：技能栏所有可渲染子/孙控件都必须挂 UiEntity。
    /// RenderLayers 不随层级传播（Bevy 0.19 check_visibility 无父级回溯），UI 相机只画
    /// layer 1/2；漏挂 UiEntity 的 Sprite/Text2d 留在默认 layer 0 → 被 UI 相机剔除、
    /// 只被地图相机画到世界原点（整栏不可见）。空 Libraries 下格网/底图等 lib 精灵不生成，
    /// 但标题文字/格子/图标/冷却/键名标签都会生成，足以守住这条不变量。
    #[test]
    fn skill_bar_children_carry_ui_entity() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .init_resource::<UiImageCache>()
            .init_resource::<UiFont>()
            .insert_resource(GameLibraries(crate::resources::libraries::Libraries::new(
                "Data",
            )))
            .insert_resource(SkillBarState::default());
        app.world_mut()
            .run_system_once(spawn_skill_bar)
            .expect("spawn_skill_bar 应可运行");

        let mut root_q = app
            .world_mut()
            .query_filtered::<Entity, With<SkillBarRoot>>();
        assert_eq!(
            root_q.iter(app.world()).count(),
            2,
            "应生成 2 个技能栏根实体（C# 两条 SkillBarDialog）"
        );

        let mut q = app
            .world_mut()
            .query::<(Has<Sprite>, Has<Text2d>, Has<UiEntity>)>();
        let mut renderable = 0usize;
        for (has_sprite, has_text, has_ui) in q.iter(app.world()) {
            if has_sprite || has_text {
                renderable += 1;
                assert!(has_ui, "可渲染技能栏控件缺 UiEntity（会被 UI 相机剔除）");
            }
        }
        assert!(renderable > 0, "应至少生成 1 个可渲染控件");
    }
}

/// 消费服务端技能事件（网络层只广播 ServerEvent）
fn skills_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut magics: ResMut<MagicsState>,
    // #2633 批次4 步7：本地判定改读 `NetObjectId`（HudState 已于步9 删除）；
    // 实体缺失视同非本地（原 hud.player_object_id=None 默认）
    local_q: Query<&crate::actor::NetObjectId, With<crate::actor::LocalPlayer>>,
) {
    use crate::network::server_event::ServerEvent;
    let local_id = local_q.single().ok().map(|id| id.0);
    for ev in events.read() {
        match ev {
            ServerEvent::MagicLearned { magic } => {
                magics.upsert(magic.clone());
            }
            ServerEvent::MagicLeveled {
                object_id,
                spell,
                level,
                experience,
                ..
            } => {
                // #220：仅本地玩家的升级更新 MagicsState（英雄的由 hero.rs 按 object_id 路由）
                if local_id != Some(*object_id) {
                    continue;
                }
                // C# S.MagicLeveled：更新技能等级/经验（技能窗口即时刷新）
                if let Some(m) = magics.magics.iter_mut().find(|m| m.spell == *spell) {
                    if m.level != *level || m.experience != *experience {
                        m.level = *level;
                        m.experience = *experience;
                        tracing::info!("⬆️ 技能 {:?} → Lv.{}", spell, level);
                    }
                }
            }
            ServerEvent::UserInformation { magics: ms, .. } => {
                for m in ms {
                    magics.upsert(m.clone());
                }
            }
            ServerEvent::MagicRemoved { spell } => {
                // #258：移除技能（遗忘）
                let before = magics.magics.len();
                magics.magics.retain(|m| m.spell != *spell);
                magics.spell_toggles.retain(|(s, _)| s != spell);
                if magics.magics.len() != before {
                    tracing::info!("🗑️ 技能已移除: {:?}", spell);
                }
            }
            ServerEvent::SpellToggled { spell, can_use } => {
                // #242：S.SpellToggle → 更新开关状态
                let before = magics.toggle_state(*spell);
                if let Some(entry) = magics.spell_toggles.iter_mut().find(|(s, _)| *s == *spell) {
                    entry.1 = *can_use;
                } else {
                    magics.spell_toggles.push((*spell, *can_use));
                }
                if before != *can_use {
                    tracing::info!("🔄 技能开关（服务端）{:?} -> {}", spell, can_use);
                }
            }
            _ => {}
        }
    }
}

// ============================================================================
// 技能窗口（#136 C# MagicWindow）：显示已学技能列表（名称/等级/快捷键）
// ============================================================================

const SKILLS_DX: f32 = 360.0;
const SKILLS_DY: f32 = 180.0;

fn spawn_skills_window(
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

    // 面板背景 Title[508]（C# CharacterDialog 技能页背景；248x284 @ (360,180)）
    let panel = if let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 508) {
        let p = spawn_panel(&mut commands, bg, SKILLS_DX, SKILLS_DY, 248.0, 284.0, 30);
        commands.entity(p).insert((
            DialogRoot(DialogKind::Skills),
            SkillsWidget,
            // #89 技能列表滚轮（10 行 × 20px）
            UiScrollList {
                rect_rel: (12.0, 36.0, 270.0, 200.0),
                row_h: 20.0,
                visible: 10,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (288.0, 36.0, 4.0, 200.0),
                thumb: None,
                z: 9,
            },
        ));
        p
    } else {
        // 兜底：纹理缺失时退回半透明深色面板
        let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
        let p = spawn_panel(&mut commands, white, SKILLS_DX, SKILLS_DY, 300.0, 360.0, 30);
        commands.entity(p).insert((
            DialogRoot(DialogKind::Skills),
            SkillsWidget,
            UiScrollList {
                rect_rel: (12.0, 36.0, 270.0, 200.0),
                row_h: 20.0,
                visible: 10,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (288.0, 36.0, 4.0, 200.0),
                thumb: None,
                z: 9,
            },
        ));
        p
    };

    commands.entity(panel).with_children(|p| {
        // 滚动条（面板子节点）
        spawn_scroll_bar_ui(p, (288.0, 36.0, 4.0, 200.0), 9);
        // 标题
        spawn_label(p, &font, "技能", 12.0, 8.0, 15.0, Color::srgb(1.0, 0.9, 0.3), 9);
        // 关闭
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 272.0, 3.0, 20.0, 20.0, 10).insert(SkillsClose);
        }
        // 列表（10 行 × 20px）@(12,36+20i)
        for i in 0..10usize {
            spawn_label(p, &font, "", 12.0, 36.0 + i as f32 * 20.0, 12.0, Color::WHITE, 9)
                .insert(SkillsLine(i));
        }
    });
}

fn cleanup_skills_window(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 显示/隐藏 + 技能列表渲染 + 关闭
fn skills_window_system(
    mut mgr: ResMut<DialogManager>,
    magics: Res<MagicsState>,
    close: Query<(Entity, &Interaction), With<SkillsClose>>,
    mut widgets: Query<&mut Visibility, With<SkillsWidget>>,
    mut lines: Query<(&mut Text, &SkillsLine)>,
    mut scroll: Query<&mut UiScrollList, With<SkillsWidget>>,
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
    let open = mgr.is_open(DialogKind::Skills);
    for mut vis in &mut widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Skills);
        }
    }
    {
        let mut sl = scroll.single_mut();
        if let Ok(sl) = sl.as_mut() {
            sl.set_total(magics.magics.len());
        }
    }
    let off = scroll.single().map(|s| s.offset).unwrap_or(0);
    for (mut text, line) in &mut lines {
        text.0 = match magics.magics.get(off + line.0) {
            Some(m) => {
                // 键名后缀（C# KeyLabel：0=无、1..8="F*"、9..16="Ctrl F*"、17..24="Shift F*"）
                let key = match skill_key_name(m.key) {
                    k if k.is_empty() => k,
                    k => format!(" [{}]", k),
                };
                // #242：开关技能显示当前状态
                let toggle = if is_toggle_spell(m.spell) {
                    let on = magics.toggle_state(m.spell);
                    if on {
                        "【开】"
                    } else {
                        "【关】"
                    }
                } else {
                    ""
                };
                format!("{} Lv.{}{}{}", m.name, m.level, key, toggle)
            }
            None => String::new(),
        };
    }
}

// ============================================================================
// 技能快捷栏（#2487 源码级对齐 C# SkillBarDialog，MainDialogs.cs L1516-1744；重做被回滚的 #2483）
//
// C# 布局（实测精灵）：底图 Prguse[2190]=216x28 @(0,0)；
//   BeforeDraw 画格网 Prguse[2193]=204x28 @(+12,0) 50% 透明（L1659，z 在底图之下）；
//   切换绑定按钮 Prguse[2247]=16x28 @(0,0)（L1542-1550）；栏位数字 8pt 白 @(0,1)（L1584-1593）；
//   技能格 @(i*25+15, 3)（L1565），图标 MagIcon[magic.Icon*2] 自然尺寸（实测 24x22，L1694）；
//   键名标签 8pt 白 @(i*25+13, 0)，格内有技能时置空（L1595-1607, L1698）；
//   冷却遮罩 Prguse2[1260+frame]（22 帧）@格位 60% 透明（L1573-1581, L1704-1743）；
//   格子点击 → UseSpell(i+1)（L1568-1571）；Movable=true 拖动整栏（L1535）。
// ============================================================================

/// 栏尺寸 = 底图 Prguse[2190] 实测 216x28
pub const SKILL_BAR_W: f32 = 216.0;
pub const SKILL_BAR_H: f32 = 28.0;
/// 拖动钳制上界（C# MirControl.OnMouseMove L901-913，Parent=GameScene 全屏分支，无 -1）：
/// X ≤ Parent.Width - W = 1024-216 = 808；Y ≤ Parent.Height - H = 768-28 = 740；下界 0。
pub const SKILL_BAR_MAX_X: f32 = 808.0;
pub const SKILL_BAR_MAX_Y: f32 = 740.0;
/// 格网 Prguse[2193] 相对栏的偏移（C# BeforeDraw：DisplayLocation + (12, 0)）
pub const SKILL_GRID_OFFSET_X: f32 = 12.0;
/// 技能格 24x22（MagIcon/冷却帧实测自然尺寸）、步进 25、首格 @(15,3)
pub const SKILL_SLOT_W: f32 = 24.0;
pub const SKILL_SLOT_H: f32 = 22.0;
pub const SKILL_SLOT_STEP: f32 = 25.0;
pub const SKILL_SLOT_X: f32 = 15.0;
pub const SKILL_SLOT_Y: f32 = 3.0;
/// 键名标签 @(i*25+13, 0)（相对格左缘 -2、栏顶 0）
pub const SKILL_KEY_X: f32 = 13.0;
/// 冷却帧 Prguse2[1260..=1282]（C# Index=1260+startFrame，startFrame∈[0,22]，共 23 帧，实测均 24x22）
pub const SKILL_COOLDOWN_BASE: usize = 1260;
pub const SKILL_COOLDOWN_FRAMES: usize = 22;

fn skill_slot_x(i: usize) -> f32 {
    SKILL_SLOT_X + i as f32 * SKILL_SLOT_STEP
}

/// C# Update()：Cells[i].Index = magic.Icon * 2（MagIcon 成对帧：偶=常态，奇=按下/灰）
fn magic_icon_index(icon: u8) -> usize {
    icon as usize * 2
}

/// C# ProcessSkillDelay（L1735-1741）整数毫秒公式：
/// `delayPerFrame = (int)(Delay/22)`；`startFrame = 22 - (int)(timeLeft/delayPerFrame)`；`Index = 1260+startFrame`。
/// startFrame ∈ [0,22]（timeLeft→Delay 时 0、timeLeft∈[100ms,delayPerFrame) 时 22），故帧索引 1260..=1282 共 23 帧
/// （Prguse2[1260..1282] 实测均为 24x22）。剩余/总为浮点秒（逐帧累减），转毫秒取整后与 C# 截断方向一致，
/// 端点精确、中段至多因浮点累减误差 ±1 帧（纯视觉）。
fn cooldown_frame(remaining_secs: f32, total_secs: f32) -> usize {
    let delay_ms = (total_secs * 1000.0).round() as i64;
    let time_left_ms = (remaining_secs * 1000.0).round() as i64;
    if delay_ms <= 0 {
        return 0;
    }
    let per_frame = (delay_ms / SKILL_COOLDOWN_FRAMES as i64).max(1);
    let start = SKILL_COOLDOWN_FRAMES as i64 - (time_left_ms / per_frame);
    start.clamp(0, SKILL_COOLDOWN_FRAMES as i64) as usize
}

fn spawn_skill_bar(
    mut commands: Commands,
    mut libs: ResMut<crate::map_renderer::GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    bar: Res<SkillBarState>,
) {
    if !crate::ui::sprite_ui::ui_enabled("skill") {
        return;
    }

    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    // C# GameScene:346-349 创建两条 SkillBarDialog（BarIndex 0/1），Settings.SkillBar=true
    // 时 DialogProcess 全部显示；默认位置 Settings.SkillbarLocation {{0,0},{216,0}} 并排左上角
    for bar_idx in 0..2usize {
        spawn_one_skill_bar(
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            &font,
            bar_idx,
            bar.pos[bar_idx],
        );
    }
}

/// C# SkillBarDialog.Update()（L1661-1707）：键名标签文本 = GetKey(BarIndex, i)。
/// 默认键位（KeyBindSettings.cs:242-276）：bar1 = F1..F8 无修饰；bar2 = Ctrl+F1..F8。
/// GetKey 格式（:383-407）：修饰符 + " + " + 键名 → "Ctrl + F1"。
pub fn skill_key_label(bar_idx: usize, i: usize) -> String {
    if bar_idx == 0 {
        format!("F{}", i + 1)
    } else {
        // C# 两行 "CTRL\nF*"；单行化用 "C+F*" 才能放进 24px 格子（"Ctrl + F*" 会溢出相邻格）
        format!("C+F{}", i + 1)
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_one_skill_bar(
    commands: &mut Commands,
    libs: &mut crate::map_renderer::GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut crate::ui::sprite_ui::UiImageCache,
    font: &Handle<Font>,
    bar_idx: usize,
    pos: (f32, f32),
) {
    // 根实体：整栏随拖动移动、随设置开关显隐（子控件全部挂在根下，z 相对叠加）
    let root = commands
        .spawn((
            UiEntity,
            SkillBarRoot(bar_idx),
            Transform::from_xyz(pos.0, -pos.1, 2.4),
            Visibility::Visible,
        ))
        .id();
    commands.entity(root).with_children(|p| {
        // ⚠️ 每个子/孙控件都必须挂 UiEntity：RenderLayers 不随层级传播（Bevy 0.19
        // check_visibility_cpu_culling 无父级回溯），仅 Added<UiEntity> 的实体才会被
        // mark_ui_render_layers 挂到 layer 1，而 UI 相机只画 layer 1/2。漏挂 → 子控件留在
        // 默认 layer 0 → 被 UI 相机剔除、只被地图相机画到世界原点，整栏不可见（#2517）。
        // C# BeforeDraw（L1659）：格网 Prguse[2193] @(+12,0) 50% 透明，画在底图之下
        if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 2193) {
            p.spawn((
                UiEntity,
                Sprite {
                    image: h,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(SKILL_GRID_OFFSET_X, 0.0, -0.05),
            ));
        }
        // 底图 Prguse[2190] @(0,0)
        if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 2190) {
            p.spawn((
                UiEntity,
                Sprite::from_image(h),
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        }
        // 切换绑定按钮 Prguse[2247]=16x28 @(0,0)（L1542-1550；C# 点击仅重绘，切换逻辑已注释）
        if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 2247) {
            p.spawn((
                UiEntity,
                Sprite::from_image(h),
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, 0.0, 0.05),
            ));
        }
        // 栏位数字（C# BindNumberLabel：Text = (BarIndex+1)，Update L1670 → "1"/"2"）
        // 8pt 白 @(0,1)（L1584-1593；C# 8pt ≈ 11px）
        p.spawn((
            UiEntity,
            Text2d::new((bar_idx + 1).to_string()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(0.0, -1.0, 0.15),
        ));
        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        for i in 0..8usize {
            // 绝对格号 = bar*8 + i（键位 = +1；C# m.Key bar1:1..8、bar2:9..16，Update L1690/1697）
            let abs = bar_idx * 8 + i;
            // 技能格锚点 @(i*25+15, 3)（L1565）：空槽无暗盒，视觉来自 2193 格网
            p.spawn((
                UiEntity,
                SkillBarSlot(abs),
                Transform::from_xyz(skill_slot_x(i), -SKILL_SLOT_Y, 0.1),
                Visibility::Visible,
            ))
            .with_children(|c| {
                // 技能图标：MagIcon[icon*2] 自然尺寸（由 skill_bar_icon_system 填图，不设 custom_size）
                c.spawn((
                    UiEntity,
                    SkillBarIcon(abs),
                    Sprite {
                        image: white.clone(),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(0.0, 0.0, 0.1),
                    Visibility::Hidden,
                ));
                // 冷却遮罩：Prguse2[1260+frame] 60% 透明（由 skill_bar_cooldown_system 驱动）
                c.spawn((
                    UiEntity,
                    SkillBarCooldown(abs),
                    Sprite {
                        image: white.clone(),
                        color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(0.0, 0.0, 0.2),
                    Visibility::Hidden,
                ));
                // 键名标签 @(i*25+13, 0) → 相对格 (-2, +3)；8pt 白；有技能时隐藏。
                // 文本 = GetKey(BarIndex, i)：bar1 "F1".."F8"、bar2 "Ctrl + F1".."Ctrl + F8"
                c.spawn((
                    UiEntity,
                    SkillBarKey(abs),
                    Text2d::new(skill_key_label(bar_idx, i)),
                    bevy::sprite::Anchor::TOP_LEFT,
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(8.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_xyz(SKILL_KEY_X - SKILL_SLOT_X, SKILL_SLOT_Y, 0.3),
                ));
            });
        }
    });
}

/// 命中技能格绝对格号（C# Cells[i] bbox：@(i*25+15, 3) 24x22；两条栏各自检测，
/// 返回 bar*8+i）
fn skill_slot_at(bar: &SkillBarState, cursor: Vec2) -> Option<usize> {
    for b in 0..2usize {
        for i in 0..8usize {
            let x = bar.pos[b].0 + skill_slot_x(i);
            let y = bar.pos[b].1 + SKILL_SLOT_Y;
            if cursor.x >= x
                && cursor.x <= x + SKILL_SLOT_W
                && cursor.y >= y
                && cursor.y <= y + SKILL_SLOT_H
            {
                return Some(b * 8 + i);
            }
        }
    }
    None
}

/// 技能栏指针交互（C# SkillBarDialog Movable + Cells[i].Click→UseSpell）：
/// 按在格子上=点击流程（松开仍在同格 → 施法；C# 格子是子控件，先吃鼠标事件，不触发拖动）；
/// 按在栏体空白=拖动整栏，松开保存 [Game] Skillbar0X/Y。
/// 设置开关 SkillBar=false 时整栏隐藏且不响应（C# GameScene.DialogProcess Hide）。
fn skill_bar_pointer_system(
    mut bar: ResMut<SkillBarState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    opt: Res<crate::game::dialogs::option::OptionState>,
    magics: Res<MagicsState>,
    mut roots: Query<(&mut Transform, &SkillBarRoot)>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
) {
    // 与 spawn_skill_bar 同一道门：UI_BITS 关掉 skill 时栏未生成，指针系统不应空转/误存（#2517）
    if !crate::ui::sprite_ui::ui_enabled("skill") {
        return;
    }
    if !crate::game::dialogs::option::view_should_show(
        crate::game::dialogs::option::OptionViewKind::SkillBar,
        &opt,
    ) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    // UI 相机是 ScalingMode::Fixed{1024,768}：窗口缩放/最大化后 cursor_position 的窗口逻辑像素
    // ≠ UI 逻辑坐标，必须经 viewport_to_world_2d 换算（对齐 ui_button_system，#2517）。
    // 光标在窗口外时为 None：不能提前 return——否则拖到窗外松开鼠标时，drag_offset 清理与 save
    // 都被跳过，拖拽态残留（#2517）。故换算成 Option，光标相关的命中/移动仅在 Some 时做。
    let cursor: Option<Vec2> = window
        .cursor_position()
        .and_then(|c| {
            ui_cameras
                .single()
                .ok()
                .and_then(|(cam, gtf)| cam.viewport_to_world_2d(gtf, c).ok())
        })
        .map(|w| Vec2::new(w.x, -w.y));
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(cursor) = cursor {
            // C# 空格 Index=-1 → AutoSize 尺寸 0x0 永不命中，鼠标事件落回对话框本体触发 Movable 拖动；
            // 仅占用格（绑定技能、显示图标）才走点击流程。故按下点命中空格时按“未命中格”处理 → 拖动。
            bar.pressed_slot =
                skill_slot_at(&bar, cursor).filter(|&i| magics.by_key(i as u8 + 1).is_some());
            // 命中哪条栏的栏体（非格子区）→ 拖那条栏（C# Movable 每条独立）
            let hit_bar = (0..2usize).find(|&b| {
                cursor.x >= bar.pos[b].0
                    && cursor.x <= bar.pos[b].0 + SKILL_BAR_W
                    && cursor.y >= bar.pos[b].1
                    && cursor.y <= bar.pos[b].1 + SKILL_BAR_H
            });
            if bar.pressed_slot.is_none() {
                if let Some(b) = hit_bar {
                    bar.drag_offset = Some((b, (cursor.x - bar.pos[b].0, cursor.y - bar.pos[b].1)));
                    tracing::debug!("⚙️ 技能栏{b} 开始拖动");
                }
            }
        }
    }
    if let Some((b, off)) = bar.drag_offset {
        if mouse.pressed(MouseButton::Left) {
            // C# OnMouseMove：拖动位置钳制在父容器（全屏）内，栏永远不可拖出屏幕（修复：拖出后丢失）
            if let Some(cursor) = cursor {
                bar.pos[b] = (
                    (cursor.x - off.0).clamp(0.0, SKILL_BAR_MAX_X),
                    (cursor.y - off.1).clamp(0.0, SKILL_BAR_MAX_Y),
                );
            }
        } else {
            // 松开（含光标已拖出窗口的松开）：结束拖动并保存（C# OnMoving 每帧写 Settings，退出统一落盘）
            bar.drag_offset = None;
            bar.save();
            tracing::info!(
                "⚙️ 技能栏{b} 位置 -> ({:.0},{:.0}) 已保存",
                bar.pos[b].0,
                bar.pos[b].1
            );
        }
    }
    // 松开：按下与松开都在同一格 → 点击施法（C# MirImageControl Click 于 MouseUp 触发）
    if mouse.just_released(MouseButton::Left) {
        if let Some(i) = bar.pressed_slot.take() {
            if cursor.and_then(|c| skill_slot_at(&bar, c)) == Some(i) {
                bar.pending_cast = Some(i);
            }
        }
    }
    // 整栏跟随拖动（根实体移动，子控件随层级联动；按 SkillBarRoot.0 栏号对位）
    for (mut tf, root) in &mut roots {
        let pos = bar.pos[root.0];
        tf.translation.x = pos.0;
        tf.translation.y = -pos.1;
    }
}

/// #2604：设置开关 SkillBar 应用到双栏根实体（C# GameScene.DialogProcess
/// 整体 Show/Hide，:1325-1332）。子控件继承根的 InheritedVisibility——
/// 图标/键名系统无需各自判断（曾无任何系统应用该设置，R 键还误接已删的
/// belt.rs 显隐资源空转）
fn skill_bar_show_system(
    opt: Res<crate::game::dialogs::option::OptionState>,
    mut roots: Query<&mut Visibility, With<SkillBarRoot>>,
) {
    let show = if crate::game::dialogs::option::view_should_show(
        crate::game::dialogs::option::OptionViewKind::SkillBar,
        &opt,
    ) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut roots {
        if *vis != show {
            *vis = show;
        }
    }
}

/// 技能快捷栏更新：显示绑定技能图标（C# Update()：图标 MagIcon[icon*2]；有技能时键名标签置空）
fn skill_bar_icon_system(
    magics: Res<MagicsState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut icons: Query<(&mut Sprite, &mut Visibility, &SkillBarIcon), Without<SkillBarKey>>,
    mut keys: Query<(&mut Visibility, &SkillBarKey), Without<SkillBarIcon>>,
) {
    for (mut sprite, mut vis, slot) in &mut icons {
        let magic = magics.by_key(slot.0 as u8 + 1);
        match magic {
            Some(m) => {
                let handle = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::MagIcon,
                    magic_icon_index(m.icon),
                );
                match handle {
                    Some(h) => {
                        if sprite.image != h {
                            sprite.image = h;
                        }
                        *vis = Visibility::Visible;
                    }
                    None => *vis = Visibility::Hidden,
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
    // 键名标签：格内有技能则隐藏（C# KeyNameLabels[i].Text = ""）
    for (mut vis, key) in &mut keys {
        let occupied = magics.by_key(key.0 as u8 + 1).is_some();
        let target = if occupied {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if *vis != target {
            *vis = target;
        }
    }
}

/// 技能栏冷却遮罩（C# ProcessSkillDelay L1704-1743：Prguse2[1260+startFrame]，startFrame∈[0,22] 扫过，
/// 60% 透明；timeLeft < 100ms 时隐藏）
fn skill_bar_cooldown_system(
    magics: Res<MagicsState>,
    cds: Res<MagicCooldowns>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut overlays: Query<(&mut Sprite, &mut Visibility, &SkillBarCooldown)>,
) {
    for (mut sprite, mut vis, slot) in &mut overlays {
        let magic = magics.by_key(slot.0 as u8 + 1);
        let state = magic.map(|m| (cds.remaining(m.spell), cds.total(m.spell)));
        match state {
            // C#：timeLeft ∈ (0,100ms) → Visible=false；冷却结束（条目移除）→ None
            Some((Some(r), Some(_))) if r < 0.1 => *vis = Visibility::Hidden,
            Some((Some(r), Some(t))) if r > 0.0 => {
                let idx = SKILL_COOLDOWN_BASE + cooldown_frame(r, t);
                match ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::Prguse2,
                    idx,
                ) {
                    Some(h) => {
                        if sprite.image != h {
                            sprite.image = h;
                        }
                        *vis = Visibility::Visible;
                    }
                    None => *vis = Visibility::Hidden,
                }
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}
