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

/// 已学技能列表（NewMagic 包写入）
#[derive(Resource, Default)]
pub struct MagicsState {
    pub magics: Vec<ClientMagic>,
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

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MagicsState>();
        app.add_systems(OnEnter(AppState::Game), spawn_skills_window);
        app.add_systems(OnExit(AppState::Game), cleanup_skills_window);
        app.add_systems(
            Update,
            // #148 技能快捷键改由 dialog_hotkey_system 按键位设置处理（可重绑）
            (skills_window_system, ui_button_system)
                .run_if(in_state(AppState::Game)),
        );
                app.add_systems(
            Update,
            skills_server_events.run_if(in_state(crate::scenes::AppState::Game)),
        );
app.add_systems(Update, skill_bar_system.run_if(in_state(AppState::Game)));
    }
}

/// F1-F8 施放绑定技能（原版 C#：F1-F8 → UserMagic(key) → Magic 包）
/// M37：有选中攻击目标时朝目标施放（弹道类魔法 target_id + 目标位置），
/// 无目标时朝当前朝向施放（fallback）。
fn skill_bar_system(
    keys: Res<ButtonInput<KeyCode>>,
    magics: Res<MagicsState>,
    net: Res<NetConnection>,
    session: Res<SessionState>,
    control: Res<crate::game::player_control::ControlState>,
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
    let Some(magic) = magics.by_key(slot as u8 + 1) else {
        return;
    };
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
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::MagicLearned { magic } => {
                magics.upsert(magic.clone());
            }
            ServerEvent::UserInformation { magics: ms, .. } => {
                for m in ms {
                    magics.upsert(m.clone());
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

    // 面板（半透明深色）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let panel = commands
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
        .id();
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
                format!("{} Lv.{}{}", m.name, m.level, key)
            }
            None => String::new(),
        };
    }
}
