// ============================================================================
// 技能系统（M13 续）：已学技能列表 + F1-F8 施放 + 快捷键绑定
// UI 交互参考：C# MainDialogs.cs（magic.Key == 1..8 = F1..F8 快捷施放）
// 网络参考：SharedRust packets/client/combat.rs::Magic / MagicKey
// ============================================================================

use bevy::prelude::*;
use mir2_shared::data::client_data::ClientMagic;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};
use crate::ui::sprite_ui::{
    spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont, UiImageCache,
};
use mir2_shared::enums::Spell;

use crate::actor::ActorAnim;
use crate::network::{NetConnection, SessionState};
use crate::scenes::AppState;

/// 开关技能列表（#242：C# GameScene UseMagic 的 toggle 分支）
pub const TOGGLE_SPELLS: [Spell; 4] = [
    Spell::Thrusting,
    Spell::HalfMoon,
    Spell::CrossHalfMoon,
    Spell::DoubleSlash,
];

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
            .map(|(_, r, t)| if *t > 0.0 { (*r / *t).clamp(0.0, 1.0) } else { 0.0 })
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

/// 技能快捷栏根实体（整栏随拖动移动、随设置开关显隐；对齐 C# SkillBarDialog 整体 Show/Hide）
#[derive(Component)]
pub struct SkillBarRoot;

/// 技能快捷栏格子锚点（子实体挂图标/冷却/键名标签）
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

/// 技能栏位置状态（C# SkillBarDialog：Movable + SkillbarLocation 持久化）
#[derive(Resource)]
pub struct SkillBarState {
    /// 栏左上角屏幕坐标（C# SkillbarLocation[0]，默认 {0,0}）
    pub pos: (f32, f32),
    /// 拖动中：按下点与 pos 的偏移
    pub drag_offset: Option<(f32, f32)>,
    /// 按下时命中的格子（C# 格子是子控件：按在格子上走点击流程，不触发拖动）
    pub pressed_slot: Option<usize>,
    /// 格子点击施法请求（C# Cells[i].Click → UseSpell(i+1)），由 skill_bar_system 消费
    pub pending_cast: Option<usize>,
}

impl Default for SkillBarState {
    fn default() -> Self {
        // C# Settings.SkillbarLocation[0] = {0, 0}（左上角）。
        // 之前默认放底部中央 (360,726) 会与左下聊天面板 (230,671)-(862,739) 重叠 → 界面“乱”。
        Self {
            pos: (0.0, 0.0),
            drag_offset: None,
            pressed_slot: None,
            pending_cast: None,
        }
    }
}

impl SkillBarState {
    /// 从 Mir2Config.ini 解析（C# [Game] Skillbar0X/Skillbar0Y）
    pub fn from_ini(content: &str) -> Self {
        use crate::game::dialogs::settings_file::ini_str;
        let mut s = Self::default();
        if let Some(v) = ini_str(content, "Game", "Skillbar0X").and_then(|v| v.parse::<f32>().ok()) {
            s.pos.0 = v;
        }
        if let Some(v) = ini_str(content, "Game", "Skillbar0Y").and_then(|v| v.parse::<f32>().ok()) {
            s.pos.1 = v;
        }
        // C# GameScene.DialogProcess（L1329-1331）：存档越界则丢弃，回落构造默认 (0,0)。
        // 判定为严格大于：x > Resolution-100（逻辑分辨率 1024 → 924）或 y > 700。
        // 负值兜底：C# 运行时拖动钳制（OnMouseMove L910-913）保证永不产生负坐标，故其加载不查负；
        // Bevy 旧版本无钳制可能存过负值 → 一并按无效处理回落默认（与越界同一语义，永不可见的栏不可留）。
        // 非有限值兜底："NaN"/"inf" 能被 parse::<f32>() 成功解析，而 NaN 的全序比较全为 false，
        // 单靠下面的越界判定拦不住 → 栏会被定位到 NaN 永不可见（#2517）。NaN/±inf 一并回落默认。
        if !s.pos.0.is_finite()
            || !s.pos.1.is_finite()
            || s.pos.0 > 924.0
            || s.pos.1 > 700.0
            || s.pos.0 < 0.0
            || s.pos.1 < 0.0
        {
            s.pos = (0.0, 0.0);
        }
        s
    }

    /// 启动时加载（C# Settings.Load）
    pub fn load() -> Self {
        Self::from_ini(&crate::game::dialogs::settings_file::load_ini())
    }

    /// 保存（C# Settings.Save：SkillbarLocation → [Game] Skillbar0X/Y，merge 写回）
    pub fn save(&self) {
        use crate::game::dialogs::settings_file::{set_ini_value, write_ini};
        let mut content = crate::game::dialogs::settings_file::load_ini();
        content = set_ini_value(&content, "Game", "Skillbar0X", &self.pos.0.round().to_string());
        content = set_ini_value(&content, "Game", "Skillbar0Y", &self.pos.1.round().to_string());
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
        app.add_systems(
            Update,
            // #148 技能快捷键改由 dialog_hotkey_system 按键位设置处理（可重绑）
            (
                skills_window_system,
                skill_bar_icon_system,
                skill_bar_cooldown_system,
                ui_button_system,
            )
                .run_if(in_state(AppState::Game)),
        );
                app.add_systems(
            Update,
            skills_server_events.run_if(in_state(crate::scenes::AppState::Game)),
        );
app.add_systems(Update, skill_bar_system.run_if(in_state(AppState::Game)));
app.add_systems(Update, magic_cooldown_system.run_if(in_state(AppState::Game)));
        app.add_systems(
            Update,
            skill_bar_pointer_system.run_if(in_state(AppState::Game)),
        );
    }
}

/// F1-F8 施放绑定技能（原版 C#：F1-F8 → UserMagic(key) → Magic 包）
/// M37：有选中攻击目标时朝目标施放（弹道类魔法 target_id + 目标位置），
/// 无目标时朝当前朝向施放（fallback）。
fn skill_bar_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    hud: Res<crate::game::hud::HudState>,
    mut bar: ResMut<SkillBarState>,
    mut magics: ResMut<MagicsState>,
    net: Res<NetConnection>,
    session: Res<SessionState>,
    control: Res<crate::game::player_control::ControlState>,
    mut chat: ResMut<crate::game::chat::ChatState>,
    actors: Query<(&crate::actor::NetObjectId, &Transform), Without<crate::actor::LocalPlayer>>,
    mut players: Query<(Entity, &Transform, &mut ActorAnim), (With<crate::actor::LocalPlayer>, With<crate::actor::NetObjectId>)>,
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
    // 格子点击施法请求（C# Cells[i].Click → UseSpell(i+1)）与 F 键同一路径
    let pending = bar.pending_cast.take();
    // #1600/#1616：C# GameScene.CheckInput——钓鱼/麻痹/冰冻锁定施法输入
    if hud.fishing || hud.paralysis {
        return;
    }
    let Some(slot) = pending.or_else(|| F_KEYS.iter().position(|k| keys.just_pressed(*k))) else {
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
            let (ttx, tty) = crate::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
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
    commands.entity(pe).remove::<crate::game::movement::LocalMove>();
    anim.action = mir2_shared::enums::MirAction::Standing;
    anim.frame_index = 0;

    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell: magic.spell,
        direction: cast_dir,
        target_id,
        location: mir2_shared::Point { x: tx, y: ty },
    });
    tracing::info!(
        "✨ F{} 施放 {} ({:?}) 目标={} @ ({},{})",
        slot + 1,
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

    #[test]
    fn skill_bar_state_parse() {
        let content = "[Game]\nSkillbar0X=123\nSkillbar0Y=456\n";
        let s = SkillBarState::from_ini(content);
        assert_eq!(s.pos, (123.0, 456.0));
    }

    #[test]
    fn skill_bar_state_defaults_when_missing() {
        let s = SkillBarState::from_ini("");
        let d = SkillBarState::default();
        assert_eq!(s.pos, d.pos);
        assert_eq!(d.pos.0, 0.0);
        assert_eq!(d.pos.1, 0.0);
    }

    /// C# GameScene.DialogProcess（L1329-1331）：存档越界丢弃，回落默认 (0,0)；严格大于
    #[test]
    fn skill_bar_state_out_of_bounds_falls_back() {
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=925\nSkillbar0Y=100\n");
        assert_eq!(s.pos, (0.0, 0.0), "x>924 应回落默认");
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=100\nSkillbar0Y=701\n");
        assert_eq!(s.pos, (0.0, 0.0), "y>700 应回落默认");
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=924\nSkillbar0Y=700\n");
        assert_eq!(s.pos, (924.0, 700.0), "边界值保留（C# 为严格 >）");
        // 负值兜底：旧版本无拖动钳制可能存过负坐标 → 视为无效回落默认（栏永不可拖出屏幕）
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=-50\nSkillbar0Y=100\n");
        assert_eq!(s.pos, (0.0, 0.0), "负 x 应回落默认");
        let s = SkillBarState::from_ini("[Game]\nSkillbar0X=100\nSkillbar0Y=-10\n");
        assert_eq!(s.pos, (0.0, 0.0), "负 y 应回落默认");
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

    /// C# Cells[i] bbox：@(i*25+15, 3) 24x22；命中格子不触发拖动
    #[test]
    fn skill_slot_hit_test() {
        let bar = SkillBarState::default();
        // 第 0 格 @(15,3) 24x22：中心命中
        assert_eq!(skill_slot_at(&bar, Vec2::new(27.0, 14.0)), Some(0));
        // 第 1 格 @(40,3)
        assert_eq!(skill_slot_at(&bar, Vec2::new(52.0, 14.0)), Some(1));
        // 第 7 格 @(190,3) 右缘
        assert_eq!(skill_slot_at(&bar, Vec2::new(213.0, 24.0)), Some(7));
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
    /// 单靠越界判定拦不住 → 栏会被定位到 NaN 永不可见。非有限值必须回落默认 (0,0)。
    #[test]
    fn skill_bar_state_non_finite_falls_back() {
        for v in ["NaN", "nan", "inf", "-inf", "Infinity"] {
            let cx = format!("[Game]\nSkillbar0X={v}\nSkillbar0Y=100\n");
            assert_eq!(
                SkillBarState::from_ini(&cx).pos,
                (0.0, 0.0),
                "Skillbar0X={v} 非有限/越界应回落默认"
            );
            let cy = format!("[Game]\nSkillbar0X=100\nSkillbar0Y={v}\n");
            assert_eq!(
                SkillBarState::from_ini(&cy).pos,
                (0.0, 0.0),
                "Skillbar0Y={v} 非有限/越界应回落默认"
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
            1,
            "应生成 1 个技能栏根实体"
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
    hud: Res<crate::game::hud::HudState>,
) {
    use crate::network::server_event::ServerEvent;
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
                if hud.player_object_id != Some(*object_id) {
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板背景 Title[508]（C# CharacterDialog 技能页背景；248x284）
    let panel = if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 508) {
        commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Skills),
                SkillsWidget,
                Sprite::from_image(h),
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(SKILLS_DX, -SKILLS_DY, 6.0),
                Visibility::Hidden,
            ))
            .id()
    } else {
        // 兜底：纹理缺失时退回半透明深色面板
        let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
        commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Skills),
                SkillsWidget,
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.12, 0.12, 0.16, 0.95),
                    custom_size: Some(Vec2::new(300.0, 360.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(SKILLS_DX, -SKILLS_DY, 6.0),
                Visibility::Hidden,
            ))
            .id()
    };
    // 标题
    let t = spawn_ui_text(&mut commands, &font, "技能", SKILLS_DX + 12.0, SKILLS_DY + 8.0, 15.0, Color::srgb(1.0, 0.9, 0.3), 6.2);
    commands.entity(t).insert((DialogRoot(DialogKind::Skills), SkillsWidget));
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        SKILLS_DX + 272.0, SKILLS_DY + 3.0, 6.3, 20.0, 20.0,
    ) {
        commands.entity(e).insert((SkillsClose, DialogRoot(DialogKind::Skills), SkillsWidget));
    }
    // 列表（10 行 × 20px + 滚动条）
    let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (SKILLS_DX + 288.0, SKILLS_DY + 36.0, 4.0, 200.0), 6.3);
    commands.entity(track).insert((DialogRoot(DialogKind::Skills), SkillsWidget, Visibility::Visible));
    commands.entity(thumb).insert((DialogRoot(DialogKind::Skills), SkillsWidget, Visibility::Visible));
    commands.entity(panel).insert(ScrollList {
        rect_rel: (12.0, 36.0, 270.0, 200.0),
        row_h: 20.0,
        visible: 10,
        total: 0,
        offset: 0,
        step: 3,
        track_rel: (288.0, 36.0, 4.0, 200.0),
        thumb: Some(thumb),
        z: 8.0,
    });
    for i in 0..10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            SKILLS_DX + 12.0, SKILLS_DY + 36.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            SkillsLine(i),
            DialogRoot(DialogKind::Skills),
            SkillsWidget,
        ));
    }
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
    close: Query<&UiButton, With<SkillsClose>>,
    mut widgets: Query<&mut Visibility, With<SkillsWidget>>,
    mut lines: Query<(&mut Text2d, &SkillsLine)>,
    mut scroll: Query<&mut ScrollList, With<SkillsWidget>>,
) {
    let open = mgr.is_open(DialogKind::Skills);
    for mut vis in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
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
                let key = if m.key >= 1 && m.key <= 8 {
                    format!(" [F{}]", m.key)
                } else {
                    String::new()
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
    // 根实体：整栏随拖动移动、随设置开关显隐（子控件全部挂在根下，z 相对叠加）
    let root = commands
        .spawn((
            UiEntity,
            SkillBarRoot,
            Transform::from_xyz(bar.pos.0, -bar.pos.1, 2.4),
            Visibility::Visible,
        ))
        .id();
    commands.entity(root).with_children(|p| {
        // ⚠️ 每个子/孙控件都必须挂 UiEntity：RenderLayers 不随层级传播（Bevy 0.19
        // check_visibility_cpu_culling 无父级回溯），仅 Added<UiEntity> 的实体才会被
        // mark_ui_render_layers 挂到 layer 1，而 UI 相机只画 layer 1/2。漏挂 → 子控件留在
        // 默认 layer 0 → 被 UI 相机剔除、只被地图相机画到世界原点，整栏不可见（#2517）。
        // C# BeforeDraw（L1659）：格网 Prguse[2193] @(+12,0) 50% 透明，画在底图之下
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            2193,
        ) {
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
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            2190,
        ) {
            p.spawn((
                UiEntity,
                Sprite::from_image(h),
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        }
        // 切换绑定按钮 Prguse[2247]=16x28 @(0,0)（L1542-1550；C# 点击仅重绘，切换逻辑已注释）
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            2247,
        ) {
            p.spawn((
                UiEntity,
                Sprite::from_image(h),
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, 0.0, 0.05),
            ));
        }
        // 栏位数字 "1" 8pt 白 @(0,1)（L1584-1593；C# 8pt ≈ 11px）
        p.spawn((
            UiEntity,
            Text2d::new("1"),
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
            // 技能格锚点 @(i*25+15, 3)（L1565）：空槽无暗盒，视觉来自 2193 格网
            p.spawn((
                UiEntity,
                SkillBarSlot(i),
                Transform::from_xyz(skill_slot_x(i), -SKILL_SLOT_Y, 0.1),
                Visibility::Visible,
            ))
            .with_children(|c| {
                // 技能图标：MagIcon[icon*2] 自然尺寸（由 skill_bar_icon_system 填图，不设 custom_size）
                c.spawn((
                    UiEntity,
                    SkillBarIcon(i),
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
                    SkillBarCooldown(i),
                    Sprite {
                        image: white.clone(),
                        color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                        ..default()
                    },
                    bevy::sprite::Anchor::TOP_LEFT,
                    Transform::from_xyz(0.0, 0.0, 0.2),
                    Visibility::Hidden,
                ));
                // 键名标签 @(i*25+13, 0) → 相对格 (-2, +3)；8pt 白；有技能时隐藏
                c.spawn((
                    UiEntity,
                    SkillBarKey(i),
                    Text2d::new(format!("F{}", i + 1)),
                    bevy::sprite::Anchor::TOP_LEFT,
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_xyz(SKILL_KEY_X - SKILL_SLOT_X, SKILL_SLOT_Y, 0.3),
                ));
            });
        }
    });
}

/// 命中技能格索引（C# Cells[i] bbox：@(i*25+15, 3) 24x22）
fn skill_slot_at(bar: &SkillBarState, cursor: Vec2) -> Option<usize> {
    (0..8).find(|&i| {
        let x = bar.pos.0 + skill_slot_x(i);
        let y = bar.pos.1 + SKILL_SLOT_Y;
        cursor.x >= x
            && cursor.x <= x + SKILL_SLOT_W
            && cursor.y >= y
            && cursor.y <= y + SKILL_SLOT_H
    })
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
    mut roots: Query<&mut Transform, With<SkillBarRoot>>,
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
            let in_bar = cursor.x >= bar.pos.0
                && cursor.x <= bar.pos.0 + SKILL_BAR_W
                && cursor.y >= bar.pos.1
                && cursor.y <= bar.pos.1 + SKILL_BAR_H;
            if bar.pressed_slot.is_none() && in_bar {
                bar.drag_offset = Some((cursor.x - bar.pos.0, cursor.y - bar.pos.1));
                tracing::debug!("⚙️ 技能栏开始拖动");
            }
        }
    }
    if let Some(off) = bar.drag_offset {
        if mouse.pressed(MouseButton::Left) {
            // C# OnMouseMove：拖动位置钳制在父容器（全屏）内，栏永远不可拖出屏幕（修复：拖出后丢失）
            if let Some(cursor) = cursor {
                bar.pos = (
                    (cursor.x - off.0).clamp(0.0, SKILL_BAR_MAX_X),
                    (cursor.y - off.1).clamp(0.0, SKILL_BAR_MAX_Y),
                );
            }
        } else {
            // 松开（含光标已拖出窗口的松开）：结束拖动并保存
            bar.drag_offset = None;
            bar.save();
            tracing::info!(
                "⚙️ 技能栏位置 -> ({:.0},{:.0}) 已保存",
                bar.pos.0,
                bar.pos.1
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
    // 整栏跟随拖动（根实体移动，子控件随层级联动）
    for mut tf in &mut roots {
        tf.translation.x = bar.pos.0;
        tf.translation.y = -bar.pos.1;
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
