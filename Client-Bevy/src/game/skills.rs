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
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};
use mir2_shared::enums::Spell;

use crate::network::{NetConnection, SessionState};
use crate::scenes::AppState;

/// 开关技能列表（#242：C# GameScene UseMagic 的 toggle 分支）
pub const TOGGLE_SPELLS: [Spell; 4] = [
    Spell::Thrusting,
    Spell::HalfMoon,
    Spell::CrossHalfMoon,
    Spell::DoubleSlash,
];

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

/// 技能快捷栏（F1-F8 图标）
#[derive(Component)]
pub struct SkillBarSlot(pub usize);

/// 技能快捷栏图标（子实体）
#[derive(Component)]
pub struct SkillBarIcon(pub usize);

/// 技能快捷栏按键标签（子实体）
#[derive(Component)]
pub struct SkillBarKey(pub usize);

/// 技能栏位置状态（C# MagicBar SkillbarLocation：可拖动 + 持久化）
#[derive(Resource)]
pub struct SkillBarState {
    /// 第 0 格左上角屏幕坐标（C# SkillbarLocation[0]）
    pub pos: (f32, f32),
    /// 拖动中：按下点与 pos 的偏移
    pub drag_offset: Option<(f32, f32)>,
}

impl Default for SkillBarState {
    fn default() -> Self {
        // C# Settings.SkillbarLocation[0] = {0, 0}（左上角）。
        // 之前默认放底部中央 (360,726) 会与左下聊天面板 (230,671)-(862,739) 重叠 → 界面“乱”。
        Self {
            pos: (0.0, 0.0),
            drag_offset: None,
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
            (skills_window_system, skill_bar_icon_system, ui_button_system)
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
            skill_bar_drag_system.run_if(in_state(AppState::Game)),
        );
    }
}

/// F1-F8 施放绑定技能（原版 C#：F1-F8 → UserMagic(key) → Magic 包）
/// M37：有选中攻击目标时朝目标施放（弹道类魔法 target_id + 目标位置），
/// 无目标时朝当前朝向施放（fallback）。
fn skill_bar_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut magics: ResMut<MagicsState>,
    net: Res<NetConnection>,
    session: Res<SessionState>,
    control: Res<crate::game::player_control::ControlState>,
    mut chat: ResMut<crate::game::chat::ChatState>,
    actors: Query<(&crate::actor::NetObjectId, &Transform), Without<crate::actor::LocalPlayer>>,
    players: Query<&Transform, (With<crate::actor::LocalPlayer>, With<crate::actor::NetObjectId>)>,
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
    let Some(slot) = F_KEYS.iter().position(|k| keys.just_pressed(*k)) else {
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
    let (px, py) = players
        .single()
        .ok()
        .map(|tf| crate::game::movement::world_to_tile(tf.translation.x, tf.translation.y))
        .or_else(|| session.self_position.map(|(x, y, _)| (x, y)))
        .unwrap_or((0, 0));
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
// 技能快捷栏（#150 C# MagicBar 对齐）：底部 F1-F8 图标 + 按键标签
// ============================================================================

const SKILL_BAR_Y: f32 = 768.0 - 42.0;
const SKILL_SLOT_W: f32 = 34.0;
const SKILL_SLOT_H: f32 = 28.0;

fn spawn_skill_bar(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    bar: Res<SkillBarState>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for i in 0..8usize {
        let x = bar.pos.0 + i as f32 * (SKILL_SLOT_W + 4.0);
        let slot = commands
            .spawn((
                UiEntity,
                SkillBarSlot(i),
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.45),
                    custom_size: Some(Vec2::new(SKILL_SLOT_W, SKILL_SLOT_H)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(x, -bar.pos.1, 2.5),
                Visibility::Visible,
            ))
            .id();
        commands.entity(slot).with_children(|p| {
            // 技能图标（MagIcon[m.icon]）
            p.spawn((
                SkillBarIcon(i),
                Sprite {
                    image: white.clone(),
                    custom_size: Some(Vec2::new(SKILL_SLOT_W - 4.0, SKILL_SLOT_H - 4.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(2.0, -2.0, 2.6),
                Visibility::Hidden,
            ));
            // F 键标签
            p.spawn((
                SkillBarKey(i),
                Text2d::new(format!("F{}", i + 1)),
                bevy::sprite::Anchor::TOP_LEFT,
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(9.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.4)),
                Transform::from_xyz(1.0, -1.0, 2.7),
            ));
        });
    }
}

/// 技能栏拖动（C# MagicBar Movable）：按住栏体移动，松开保存 [Game] Skillbar0X/Y
fn skill_bar_drag_system(
    mut bar: ResMut<SkillBarState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut slots: Query<(&mut Transform, &SkillBarSlot)>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let bar_w = 8.0 * (SKILL_SLOT_W + 4.0) - 4.0;
    let in_bar = cursor.x >= bar.pos.0
        && cursor.x <= bar.pos.0 + bar_w
        && cursor.y >= bar.pos.1
        && cursor.y <= bar.pos.1 + SKILL_SLOT_H;
    if mouse.just_pressed(MouseButton::Left) && in_bar {
        bar.drag_offset = Some((cursor.x - bar.pos.0, cursor.y - bar.pos.1));
        tracing::debug!("⚙️ 技能栏开始拖动");
    }
    if let Some(off) = bar.drag_offset {
        if mouse.pressed(MouseButton::Left) {
            bar.pos = (cursor.x - off.0, cursor.y - off.1);
        } else {
            bar.drag_offset = None;
            bar.save();
            tracing::info!("⚙️ 技能栏位置 -> ({:.0},{:.0}) 已保存", bar.pos.0, bar.pos.1);
        }
    }
    // 实时应用位置到各槽位（含拖动中）
    for (mut tf, slot) in &mut slots {
        tf.translation.x = bar.pos.0 + slot.0 as f32 * (SKILL_SLOT_W + 4.0);
        tf.translation.y = -bar.pos.1;
    }
}

/// 技能快捷栏更新：显示绑定技能图标
fn skill_bar_icon_system(
    magics: Res<MagicsState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut icons: Query<(&mut Sprite, &mut Visibility, &SkillBarIcon)>,
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
                    m.icon as usize,
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
}



