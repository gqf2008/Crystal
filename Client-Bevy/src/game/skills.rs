// ============================================================================
// 技能系统（M13 续）：已学技能列表 + F1-F8 施放 + 快捷键绑定
// UI 交互参考：C# MainDialogs.cs（magic.Key == 1..8 = F1..F8 快捷施放）
// 网络参考：SharedRust packets/client/combat.rs::Magic / MagicKey
// ============================================================================

use bevy::prelude::*;
use mir2_shared::data::client_data::ClientMagic;
use mir2_shared::enums::Spell;

use crate::network::NetworkContext;
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

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MagicsState>();
        app.add_systems(Update, skill_bar_system.run_if(in_state(AppState::Game)));
    }
}

/// F1-F8 施放绑定技能（原版 C#：F1-F8 → UserMagic(key) → Magic 包）
fn skill_bar_system(
    keys: Res<ButtonInput<KeyCode>>,
    magics: Res<MagicsState>,
    net: Res<NetworkContext>,
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
    // 玩家当前瓦片位置与朝向（服务器权威坐标，UserLocation 持续更新）
    let (x, y, dir) = net.self_position.unwrap_or((0, 0, 4));
    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell: magic.spell,
        direction: mir2_shared::enums::MirDirection::try_from(dir)
            .unwrap_or(mir2_shared::enums::MirDirection::Down),
        target_id: 0,
        location: mir2_shared::Point { x, y },
    });
    tracing::info!("✨ F{} 施放 {} ({:?}) @ ({},{})", slot + 1, magic.name, magic.spell, x, y);
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
