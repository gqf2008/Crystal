// ============================================================================
// 技能系统（M13 续）：已学技能列表 + F1-F8 施放
// UI 交互参考：C# MainDialogs.cs（magic.Key == 1..8 = F1..F8 快捷施放）
// 网络参考：SharedRust packets/client/combat.rs::Magic
// ============================================================================

use bevy::prelude::*;
use mir2_shared::data::client_data::ClientMagic;

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
