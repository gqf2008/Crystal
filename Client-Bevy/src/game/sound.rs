// ============================================================================
// 音效系统（M11）
// 数据：Client-Macroquad/Sound/SoundList.lst（sound_id → 文件名）+ *.wav
// 用法：play_sound(commands, assets, bank, id)
// 挂接：攻击 Swing(10050)、受击 Struck(10060)
// ============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings};
use bevy::prelude::*;

/// 音效库（SoundList.lst 映射）
#[derive(Resource, Default)]
pub struct SoundBank {
    pub map: HashMap<u32, String>,
    pub root: PathBuf,
}

impl SoundBank {
    /// 加载 SoundList.lst（参考 Client-Macroquad/src/systems/presentation/sound_system.rs）
    pub fn load(&mut self) {
        // 共享主仓库的 Sound 目录（独立 worktree 场景：Crystal-bevy → ../Crystal）
        let candidate = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Client-Macroquad/Sound"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Crystal/Client-Macroquad/Sound"),
        ];
        self.root = candidate
            .into_iter()
            .find(|p| p.join("SoundList.lst").exists())
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Crystal/Client-Macroquad/Sound"));
        let lst = self.root.join("SoundList.lst");
        let Ok(text) = std::fs::read_to_string(&lst) else {
            tracing::warn!("🔊 SoundList.lst 未找到: {}", lst.display());
            return;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if let Some((id, file)) = line.split_once(':') {
                if let Ok(id) = id.trim().parse::<u32>() {
                    self.map.insert(id, file.trim().to_string());
                }
            }
        }
        tracing::info!("🔊 SoundList 加载: {} 条音效", self.map.len());
    }

    pub fn file_for(&self, id: u32) -> Option<PathBuf> {
        self.map.get(&id).map(|f| self.root.join(f))
    }
}

/// 全局音效音量（0-100 百分比，C# Settings.Volume；option_view_system 同步）
pub static SOUND_VOLUME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(80);

/// 百分比 → 线性音量（0.0-1.0）
pub fn volume_from_percent(percent: u32) -> f32 {
    (percent as f32 / 100.0).clamp(0.0, 1.0)
}

fn volume_settings() -> PlaybackSettings {
    let v = SOUND_VOLUME.load(std::sync::atomic::Ordering::Relaxed);
    PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(volume_from_percent(v)))
}

/// #1564：近战挥击音效（C# PlayerObject.PlayAttackSound，PlayerObject.cs:4889）：
/// - 骑乘：mount_type<7 → TigerAttack1(10181)、<12 → WolfAttack1(10190)（C# Random 区间，取首值确定性返回）；
/// - 刺客（持武器）→ SwingShort(10050)；
/// - 弓手（持武器）→ 无挥击音（C# 直接 return）；
/// - 其余按武器形状映射 SwingWood/Short/Sword/Sword2/Axe/Long/Club/Fist；
/// - 无武器 → SwingFist(10056)（C# default 分支）。
pub fn attack_swing_sound(
    class: u8,
    riding: bool,
    mount_type: i16,
    weapon_shape: i16,
) -> Option<u32> {
    use mir2_shared::enums::MirClass;
    if riding {
        // C#：MountType < 7 → 10181..10184（Tiger）；< 12 → 10190..10193（Wolf）
        return Some(if mount_type < 7 { 10181 } else { 10190 });
    }
    if class == MirClass::Assassin as u8 {
        return Some(10050); // SwingShort
    }
    if class == MirClass::Archer as u8 {
        return None; // 弓手不播近战挥击音（C# return）
    }
    Some(match weapon_shape {
        0 | 23 | 28 | 40 => 10051, // SwingWood
        1 | 12 => 10050,           // SwingShort
        2 | 8 | 11 | 15 | 18 | 20 | 25 | 31 | 33 | 34 | 37 | 41 => 10052, // SwingSword
        3 | 5 | 7 | 9 | 13 | 19 | 24 | 26 | 29 | 32 | 35 => 10053,       // SwingSword2
        4 | 14 | 16 | 38 => 10054,                                        // SwingAxe
        6 | 10 | 17 | 22 | 27 | 30 | 36 | 39 => 10056,                    // SwingLong
        21 => 10055,                                                       // SwingClub
        _ => 10056,                                                        // SwingFist
    })
}

/// #1570：怪物基础音（C# MonsterObject.cs:217 BaseSound = (ushort)BaseImage * 10）
pub fn monster_base_sound(monster_type: u16) -> u32 {
    monster_type as u32 * 10
}

/// #1570：怪物攻击音（C# PlayAttackSound → BaseSound + 1）
pub fn monster_attack_sound(monster_type: u16) -> u32 {
    monster_base_sound(monster_type) + 1
}

/// #1570：怪物死亡音（C# PlayDieSound → BaseSound + 3）
pub fn monster_die_sound(monster_type: u16) -> u32 {
    monster_base_sound(monster_type) + 3
}

/// #1568：怪物受击音（C# MonsterObject.PlayStruckSound，MonsterObject.cs:3966）按攻击者武器形状：
/// StruckWooden(10061)/StruckShort(10060)/StruckSword(10062)/StruckSword2(10063)/StruckAxe(10064)/StruckClub(10065)；
/// 未匹配（如无武器）C# 无 default → 不发音（返回 None）。
pub fn monster_struck_sound(weapon_shape: i16) -> Option<u32> {
    Some(match weapon_shape {
        0 | 23 | 28 | 40 => 10061, // StruckWooden
        1 | 12 => 10060,           // StruckShort
        2 | 8 | 11 | 15 | 18 | 20 | 25 | 31 | 33 | 34 | 37 | 41 => 10062, // StruckSword
        3 | 5 | 7 | 9 | 13 | 19 | 24 | 26 | 29 | 32 | 35 => 10063,       // StruckSword2
        4 | 14 | 16 | 38 => 10064,                                        // StruckAxe
        6 | 10 | 17 | 22 | 27 | 30 | 36 | 39 => 10060,                    // StruckShort
        21 => 10065,                                                       // StruckClub
        _ => return None,
    })
}

/// #1564：玩家受击 flinch 音（C# PlayerObject.cs:749 FlinchSound：MaleFlinch 10138 / FemaleFlinch 10139）
pub fn player_flinch_sound(gender: u8) -> u32 {
    if gender == 0 {
        10138 // MaleFlinch
    } else {
        10139 // FemaleFlinch
    }
}

/// #1564：玩家死亡音（C# PlayerObject.cs:748 DieSound：MaleDie 10144 / FemaleDie 10145）
pub fn player_die_sound(gender: u8) -> u32 {
    if gender == 0 {
        10144 // MaleDie
    } else {
        10145 // FemaleDie
    }
}

/// 播放音效（读取 wav → AudioSource → 一次性播放）
pub fn play_sound(
    commands: &mut Commands,
    assets: &mut Assets<AudioSource>,
    bank: &SoundBank,
    id: u32,
) {
    let Some(path) = bank.file_for(id) else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let source = AudioSource {
        bytes: Arc::from(bytes),
    };
    let handle = assets.add(source);
    commands.spawn((
        AudioPlayer(handle),
        volume_settings(),
    ));
}

/// 音效缓存（#91：UI 高频点击音效复用 AudioSource，避免每次读盘）
#[derive(Resource, Default)]
pub struct SoundCache {
    pub map: HashMap<u32, Handle<AudioSource>>,
}

/// 播放音效（带缓存；未命中时读盘一次并缓存）
pub fn play_sound_cached(
    commands: &mut Commands,
    assets: &mut Assets<AudioSource>,
    bank: &SoundBank,
    cache: &mut SoundCache,
    id: u32,
) {
    let handle = if let Some(h) = cache.map.get(&id) {
        h.clone()
    } else {
        let Some(path) = bank.file_for(id) else {
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let source = AudioSource {
            bytes: Arc::from(bytes),
        };
        let h = assets.add(source);
        cache.map.insert(id, h.clone());
        h
    };
    commands.spawn((
        AudioPlayer(handle),
        volume_settings(),
    ));
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundBank>();
        app.init_resource::<SoundCache>();
        app.add_systems(Startup, load_sound_bank);
        // #230：S.PlaySound → 播放服务端指定音效
        app.add_systems(
            Update,
            play_server_sounds
                .after(crate::network::network_system)
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
    }
}

fn load_sound_bank(mut bank: ResMut<SoundBank>) {
    bank.load();
}

/// #230：消费 ServerEvent::PlaySound 并播放（带缓存）
fn play_server_sounds(
    mut commands: Commands,
    mut assets: ResMut<Assets<AudioSource>>,
    bank: Res<SoundBank>,
    mut cache: ResMut<SoundCache>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::PlaySound { sound_id } = ev {
            play_sound_cached(&mut commands, &mut assets, &bank, &mut cache, *sound_id);
            tracing::debug!("🔊 [SOUND] 播放服务端音效 #{}", sound_id);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_from_percent_clamps() {
        assert_eq!(volume_from_percent(0), 0.0);
        assert_eq!(volume_from_percent(80), 0.8);
        assert_eq!(volume_from_percent(100), 1.0);
        assert_eq!(volume_from_percent(200), 1.0);
    }

    #[test]
    fn attack_swing_sound_weapon_shapes_match_csharp() {
        // #1564：C# PlayerObject.PlayAttackSound 武器形状分组
        use mir2_shared::enums::MirClass;
        let war = MirClass::Warrior as u8;
        // 0/23/28/40 → SwingWood(10051)
        for shape in [0i16, 23, 28, 40] {
            assert_eq!(attack_swing_sound(war, false, 0, shape), Some(10051));
        }
        // 1/12 → SwingShort(10050)
        for shape in [1i16, 12] {
            assert_eq!(attack_swing_sound(war, false, 0, shape), Some(10050));
        }
        // 2 → SwingSword(10052)
        assert_eq!(attack_swing_sound(war, false, 0, 2), Some(10052));
        // 3 → SwingSword2(10053)
        assert_eq!(attack_swing_sound(war, false, 0, 3), Some(10053));
        // 4 → SwingAxe(10054)
        assert_eq!(attack_swing_sound(war, false, 0, 4), Some(10054));
        // 21 → SwingClub(10055)
        assert_eq!(attack_swing_sound(war, false, 0, 21), Some(10055));
        // 6 → SwingLong(10056)
        assert_eq!(attack_swing_sound(war, false, 0, 6), Some(10056));
        // 默认（无武器 -1）→ SwingFist(10056)
        assert_eq!(attack_swing_sound(war, false, 0, -1), Some(10056));
    }

    #[test]
    fn attack_swing_sound_class_and_riding() {
        // #1564：刺客 SwingShort；弓手无挥击音；骑乘坐骑音
        use mir2_shared::enums::MirClass;
        assert_eq!(
            attack_swing_sound(MirClass::Assassin as u8, false, 0, 5),
            Some(10050)
        );
        assert_eq!(
            attack_swing_sound(MirClass::Archer as u8, false, 0, 5),
            None,
            "弓手不播近战挥击音（C# return）"
        );
        // 骑乘：mount_type<7 → TigerAttack1(10181)；>=7 → WolfAttack1(10190)
        assert_eq!(attack_swing_sound(MirClass::Warrior as u8, true, 0, 5), Some(10181));
        assert_eq!(attack_swing_sound(MirClass::Warrior as u8, true, 7, 5), Some(10190));
    }

    #[test]
    fn monster_base_sound_matches_csharp() {
        // #1570：C# BaseSound = BaseImage * 10；攻击 +1 / 死亡 +3
        assert_eq!(monster_base_sound(0), 0);
        assert_eq!(monster_base_sound(1), 10);
        assert_eq!(monster_base_sound(42), 420);
        assert_eq!(monster_attack_sound(1), 11); // BaseSound+1
        assert_eq!(monster_die_sound(1), 13); // BaseSound+3
        assert_eq!(monster_die_sound(42), 423);
    }

    #[test]
    fn monster_struck_sound_weapon_shapes_match_csharp() {
        // #1568：C# MonsterObject.PlayStruckSound 形状分组
        for shape in [0i16, 23, 28, 40] {
            assert_eq!(monster_struck_sound(shape), Some(10061)); // StruckWooden
        }
        for shape in [1i16, 12, 6, 10, 17, 22, 27, 30, 36, 39] {
            assert_eq!(monster_struck_sound(shape), Some(10060)); // StruckShort
        }
        assert_eq!(monster_struck_sound(2), Some(10062)); // StruckSword
        assert_eq!(monster_struck_sound(3), Some(10063)); // StruckSword2
        assert_eq!(monster_struck_sound(4), Some(10064)); // StruckAxe
        assert_eq!(monster_struck_sound(21), Some(10065)); // StruckClub
        // 无武器（C# 无 default）→ 不发音
        assert_eq!(monster_struck_sound(-1), None);
    }

    #[test]
    fn player_flinch_and_die_sound_by_gender() {
        // #1564：C# FlinchSound/DieSound 按性别（0=Male）
        assert_eq!(player_flinch_sound(0), 10138); // MaleFlinch
        assert_eq!(player_flinch_sound(1), 10139); // FemaleFlinch
        assert_eq!(player_die_sound(0), 10144); // MaleDie
        assert_eq!(player_die_sound(1), 10145); // FemaleDie
    }
}
