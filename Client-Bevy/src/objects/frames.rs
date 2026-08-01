// Frames.rs - Animation frame management
// Mirrors Client/MirObjects/Frames.cs
//
// Minimal subset for macroquad client:
// - Player frame table
// - Frame struct (start/count/interval + optional effect layer)

use mir2_shared::enums::MirAction;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Frame definition for animations
///
/// Mirrors C# Client/MirObjects/Frames.cs Frame class
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Starting frame index
    pub start: i32,
    /// Number of frames in animation
    pub count: i32,
    /// Number of frames to skip
    pub skip: i32,
    /// Time between frames (milliseconds)
    pub interval: i32,

    /// Starting frame for effect layer
    pub effect_start: i32,
    /// Number of effect frames
    pub effect_count: i32,
    /// Number of effect frames to skip
    pub effect_skip: i32,
    /// Time between effect frames (milliseconds)
    pub effect_interval: i32,

    /// Play animation in reverse
    pub reverse: bool,
    /// Use alpha blending
    pub blend: bool,
}

impl Frame {
    pub fn new(
        start: i32,
        count: i32,
        skip: i32,
        interval: i32,
        effect_start: i32,
        effect_count: i32,
        effect_skip: i32,
        effect_interval: i32,
    ) -> Self {
        Self {
            start,
            count,
            skip,
            interval,
            effect_start,
            effect_count,
            effect_skip,
            effect_interval,
            reverse: false,
            blend: false,
        }
    }

    pub fn basic(start: i32, count: i32, skip: i32, interval: i32) -> Self {
        Self::new(start, count, skip, interval, 0, 0, 0, 0)
    }

    pub fn offset(&self) -> i32 {
        self.count + self.skip
    }

    pub fn effect_offset(&self) -> i32 {
        self.effect_count + self.effect_skip
    }

    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    pub fn with_blend(mut self, blend: bool) -> Self {
        self.blend = blend;
        self
    }
}

/// FrameSet - Collection of frames for different actions
pub type FrameSet = HashMap<MirAction, Frame>;

/// Static frame data for Player
///
/// Mirrors C# FrameSet.Player
pub static PLAYER_FRAMES: LazyLock<FrameSet> = LazyLock::new(|| {
    let mut frames = HashMap::new();

    // Common actions
    frames.insert(MirAction::Standing, Frame::new(0, 4, 0, 500, 0, 8, 0, 250));
    frames.insert(MirAction::Walking, Frame::new(32, 6, 0, 100, 64, 6, 0, 100));
    frames.insert(MirAction::Running, Frame::new(80, 6, 0, 100, 112, 6, 0, 100));
    frames.insert(MirAction::Stance, Frame::new(128, 1, 0, 1000, 160, 1, 0, 1000));
    frames.insert(MirAction::Stance2, Frame::new(300, 1, 5, 1000, 332, 1, 5, 1000));

    frames.insert(MirAction::Attack1, Frame::new(136, 6, 0, 100, 168, 6, 0, 100));
    frames.insert(MirAction::Attack2, Frame::new(184, 6, 0, 100, 216, 6, 0, 100));
    frames.insert(MirAction::Attack3, Frame::new(232, 8, 0, 100, 264, 8, 0, 100));
    frames.insert(MirAction::Attack4, Frame::new(416, 6, 0, 100, 448, 6, 0, 100));

    frames.insert(MirAction::Spell, Frame::new(296, 6, 0, 100, 328, 6, 0, 100));
    frames.insert(MirAction::Harvest, Frame::new(344, 2, 0, 300, 376, 2, 0, 300));
    frames.insert(MirAction::Struck, Frame::new(360, 3, 0, 100, 392, 3, 0, 100));
    frames.insert(MirAction::Die, Frame::new(384, 4, 0, 100, 416, 4, 0, 100));
    frames.insert(MirAction::Dead, Frame::new(387, 1, 3, 1000, 419, 1, 3, 1000));
    frames.insert(MirAction::Revive, Frame::new(384, 4, 0, 100, 416, 4, 0, 100).with_reverse(true));
    frames.insert(MirAction::Mine, Frame::new(184, 6, 0, 100, 216, 6, 0, 100));
    frames.insert(MirAction::Lunge, Frame::new(139, 1, 5, 1000, 300, 1, 5, 1000));

    // Assassin
    frames.insert(MirAction::Sneek, Frame::new(464, 6, 0, 100, 496, 6, 0, 100));
    frames.insert(MirAction::DashAttack, Frame::new(80, 3, 3, 100, 112, 3, 3, 100));

    // Archer
    frames.insert(MirAction::WalkingBow, Frame::new(0, 6, 0, 100, 0, 6, 0, 100));
    frames.insert(MirAction::RunningBow, Frame::new(48, 6, 0, 100, 48, 6, 0, 100));
    frames.insert(MirAction::AttackRange1, Frame::new(96, 8, 0, 100, 96, 8, 0, 100));
    frames.insert(MirAction::AttackRange2, Frame::new(160, 8, 0, 100, 160, 8, 0, 100));
    frames.insert(MirAction::AttackRange3, Frame::new(224, 8, 0, 100, 224, 8, 0, 100));
    frames.insert(MirAction::Jump, Frame::new(288, 8, 0, 100, 288, 8, 0, 100));

    // Mounts
    frames.insert(MirAction::MountStanding, Frame::new(416, 4, 0, 500, 448, 4, 0, 500));
    frames.insert(MirAction::MountWalking, Frame::new(448, 8, 0, 100, 480, 8, 0, 500));
    frames.insert(MirAction::MountRunning, Frame::new(512, 6, 0, 100, 544, 6, 0, 100));
    frames.insert(MirAction::MountStruck, Frame::new(560, 3, 0, 100, 592, 3, 0, 100));
    frames.insert(MirAction::MountAttack, Frame::new(584, 6, 0, 100, 616, 6, 0, 100));

    // Fishing
    frames.insert(MirAction::FishingCast, Frame::new(632, 8, 0, 100, 0, 0, 0, 0));
    frames.insert(MirAction::FishingWait, Frame::new(696, 6, 0, 120, 0, 0, 0, 0));
    frames.insert(MirAction::FishingReel, Frame::new(744, 8, 0, 100, 0, 0, 0, 0));

    frames
});

/// Static frame data for Default Monster
///
/// Mirrors C# FrameSet.DefaultMonster (Client/MirObjects/Frames.cs)
pub static DEFAULT_MONSTER_FRAMES: LazyLock<FrameSet> = LazyLock::new(|| {
    let mut frames = HashMap::new();

    frames.insert(MirAction::Standing, Frame::basic(0, 4, 0, 500));
    frames.insert(MirAction::Walking, Frame::basic(32, 6, 0, 100));
    frames.insert(MirAction::Attack1, Frame::basic(80, 6, 0, 100));
    frames.insert(MirAction::Struck, Frame::basic(128, 2, 0, 200));
    frames.insert(MirAction::Die, Frame::basic(144, 10, 0, 100));
    frames.insert(MirAction::Dead, Frame::basic(153, 1, 9, 1000));
    frames.insert(MirAction::Revive, Frame::basic(144, 10, 0, 100).with_reverse(true));

    frames
});

/// Static frame data for Default NPC
///
/// Mirrors C# FrameSet.DefaultNPC (Client/MirObjects/Frames.cs)
pub static DEFAULT_NPC_FRAMES: LazyLock<FrameSet> = LazyLock::new(|| {
    let mut frames = HashMap::new();
    frames.insert(MirAction::Standing, Frame::basic(0, 4, 0, 450));
    frames.insert(MirAction::Harvest, Frame::basic(12, 10, 0, 200));
    frames
});

/// Special monster framesets
///
/// Mirrors C# Client/MirObjects/Frames.cs:
/// - FrameSet.DragonStatue
/// - FrameSet.GreatFoxSpirit
/// - FrameSet.HellBomb
/// - FrameSet.CaveStatue
pub static DRAGON_STATUE_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut list: Vec<FrameSet> = Vec::new();

    // DragonStatue 1..6
    for start in [300, 301, 302, 320, 321, 322] {
        let mut frame = FrameSet::new();
        frame.insert(MirAction::Standing, Frame::basic(start, 1, -1, 1000));
        frame.insert(MirAction::AttackRange1, Frame::basic(start, 1, -1, 120));
        frame.insert(MirAction::Struck, Frame::basic(start, 1, -1, 200));
        list.push(frame);
    }

    list
});

pub static GREAT_FOX_SPIRIT_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut list: Vec<FrameSet> = Vec::new();

    // GreatFoxSpirit level 0..4
    // Each level shifts standing/attack/struck by +60.
    for base in [0, 60, 120, 180, 240] {
        let mut frame = FrameSet::new();
        frame.insert(MirAction::Standing, Frame::basic(base, 20, -20, 100));
        frame.insert(MirAction::Attack1, Frame::basic(base + 22, 8, -8, 120));
        frame.insert(MirAction::Struck, Frame::basic(base + 20, 2, -2, 200));
        frame.insert(MirAction::Die, Frame::basic(300, 18, -18, 120));
        frame.insert(MirAction::Dead, Frame::basic(317, 1, -1, 1000));
        frame.insert(MirAction::Revive, Frame::basic(300, 18, -18, 150).with_reverse(true));
        list.push(frame);
    }

    list
});

pub static HELL_BOMB_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut list: Vec<FrameSet> = Vec::new();

    // HellBomb1/2/3
    for start in [52, 70, 88] {
        let mut frame = FrameSet::new();
        frame.insert(MirAction::Standing, Frame::basic(start, 9, -9, 100).with_blend(true));
        frame.insert(MirAction::Attack1, Frame::basic(999, 1, -1, 120).with_blend(true));
        frame.insert(MirAction::Struck, Frame::basic(start, 9, -9, 100).with_blend(true));
        list.push(frame);
    }

    list
});

pub static CAVE_STATUE_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut list: Vec<FrameSet> = Vec::new();

    // CaveStatue 1..2
    // NOTE: C# marks Blend=false explicitly; default is false here.
    let mut f1 = FrameSet::new();
    f1.insert(MirAction::Standing, Frame::basic(0, 1, -1, 100));
    f1.insert(MirAction::Struck, Frame::basic(0, 1, -1, 100));
    f1.insert(MirAction::Die, Frame::basic(2, 8, -8, 100));
    f1.insert(MirAction::Dead, Frame::basic(9, 1, -1, 100));
    list.push(f1);

    let mut f2 = FrameSet::new();
    f2.insert(MirAction::Standing, Frame::basic(18, 1, -1, 100));
    f2.insert(MirAction::Struck, Frame::basic(18, 1, -1, 100));
    f2.insert(MirAction::Die, Frame::basic(20, 8, -8, 100));
    f2.insert(MirAction::Dead, Frame::basic(27, 1, -1, 100));
    list.push(f2);

    list
});

/// Get player frame for a specific action
pub fn get_player_frame(action: MirAction) -> Option<&'static Frame> {
    PLAYER_FRAMES.get(&action)
}

pub fn get_default_monster_frame(action: MirAction) -> Option<&'static Frame> {
    DEFAULT_MONSTER_FRAMES.get(&action)
}

pub fn get_default_npc_frame(action: MirAction) -> Option<&'static Frame> {
    DEFAULT_NPC_FRAMES.get(&action)
}

/// Get monster frame for a specific monster type and action.
///
/// Mirrors C# MonsterObject.Load() frame selection logic.
pub fn get_monster_frame(
    monster_type: u16,
    action: MirAction,
    direction: mir2_shared::enums::MirDirection,
    stage: u8,
) -> Option<&'static Frame> {
    use mir2_shared::enums::Monster as MonsterKind;

    match MonsterKind::try_from(monster_type) {
        Ok(MonsterKind::GreatFoxSpirit) => {
            let idx = (stage as usize).min(GREAT_FOX_SPIRIT_FRAMES.len().saturating_sub(1));
            GREAT_FOX_SPIRIT_FRAMES.get(idx).and_then(|fs| fs.get(&action))
        }
        Ok(MonsterKind::DragonStatue) => {
            let idx = (direction as u8 as usize).min(DRAGON_STATUE_FRAMES.len().saturating_sub(1));
            DRAGON_STATUE_FRAMES.get(idx).and_then(|fs| fs.get(&action))
        }
        Ok(MonsterKind::HellBomb1) | Ok(MonsterKind::HellBomb2) | Ok(MonsterKind::HellBomb3) => {
            let base = MonsterKind::HellBomb1 as u16;
            let raw = monster_type.saturating_sub(base) as usize;
            let idx = raw.min(HELL_BOMB_FRAMES.len().saturating_sub(1));
            HELL_BOMB_FRAMES.get(idx).and_then(|fs| fs.get(&action))
        }
        Ok(MonsterKind::CaveStatue) => {
            let idx = (direction as u8 as usize).min(CAVE_STATUE_FRAMES.len().saturating_sub(1));
            CAVE_STATUE_FRAMES.get(idx).and_then(|fs| fs.get(&action))
        }
        _ => get_default_monster_frame(action),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::{MirDirection, Monster as MonsterKind};

    #[test]
    fn player_frames_basic_sanity() {
        let standing = get_player_frame(MirAction::Standing).expect("standing frame");
        assert_eq!(standing.start, 0);
        assert_eq!(standing.count, 4);
        assert_eq!(standing.interval, 500);

        assert!(get_player_frame(MirAction::Walking).is_some());
        assert!(get_player_frame(MirAction::Attack1).is_some());
    }

    #[test]
    fn special_monster_frames_dragon_statue_by_direction() {
        // C#：FrameSet.DragonStatue[(byte)Direction]
        let expected = [300, 301, 302, 320, 321, 322];

        for (dir_u8, &start) in expected.iter().enumerate() {
            let dir = MirDirection::try_from(dir_u8 as u8).expect("valid direction");
            let frame = get_monster_frame(
                MonsterKind::DragonStatue as u16,
                MirAction::Standing,
                dir,
                0,
            )
            .expect("dragon statue standing frame");
            assert_eq!(frame.start, start);
            assert_eq!(frame.count, 1);
            assert_eq!(frame.skip, -1);
        }

        // 超出 list 长度时：Rust 侧做了 clamp 兜底（避免 panic）
        let dir7 = MirDirection::try_from(7).expect("dir7");
        let frame = get_monster_frame(
            MonsterKind::DragonStatue as u16,
            MirAction::Standing,
            dir7,
            0,
        )
        .expect("dragon statue standing frame");
        assert_eq!(frame.start, 322);
    }

    #[test]
    fn special_monster_frames_great_fox_spirit_by_stage() {
        // C#：FrameSet.GreatFoxSpirit[Stage]
        let dir0 = MirDirection::try_from(0).unwrap();

        let f0 = get_monster_frame(
            MonsterKind::GreatFoxSpirit as u16,
            MirAction::Standing,
            dir0,
            0,
        )
        .expect("gfs standing");
        assert_eq!(f0.start, 0);
        assert_eq!(f0.count, 20);
        assert_eq!(f0.skip, -20);

        let f4 = get_monster_frame(
            MonsterKind::GreatFoxSpirit as u16,
            MirAction::Attack1,
            dir0,
            4,
        )
        .expect("gfs attack1");
        assert_eq!(f4.start, 262);
        assert_eq!(f4.count, 8);
        assert_eq!(f4.skip, -8);

        // 超出 stage：clamp 到最后一套（level 4）
        let f9 = get_monster_frame(
            MonsterKind::GreatFoxSpirit as u16,
            MirAction::Standing,
            dir0,
            9,
        )
        .expect("gfs standing");
        assert_eq!(f9.start, 240);
    }

    #[test]
    fn special_monster_frames_hell_bomb_by_base_image() {
        let dir0 = MirDirection::try_from(0).unwrap();

        let hb1 = get_monster_frame(
            MonsterKind::HellBomb1 as u16,
            MirAction::Standing,
            dir0,
            0,
        )
        .expect("hb1 standing");
        assert_eq!(hb1.start, 52);
        assert!(hb1.blend);

        let hb2 = get_monster_frame(
            MonsterKind::HellBomb2 as u16,
            MirAction::Standing,
            dir0,
            0,
        )
        .expect("hb2 standing");
        assert_eq!(hb2.start, 70);

        let hb3 = get_monster_frame(
            MonsterKind::HellBomb3 as u16,
            MirAction::Standing,
            dir0,
            0,
        )
        .expect("hb3 standing");
        assert_eq!(hb3.start, 88);

        let hb_attack = get_monster_frame(
            MonsterKind::HellBomb3 as u16,
            MirAction::Attack1,
            dir0,
            0,
        )
        .expect("hb attack1");
        assert_eq!(hb_attack.start, 999);
        assert!(hb_attack.blend);
    }

    #[test]
    fn special_monster_frames_cave_statue_by_direction() {
        let dir0 = MirDirection::try_from(0).unwrap();
        let dir1 = MirDirection::try_from(1).unwrap();

        let s1 = get_monster_frame(
            MonsterKind::CaveStatue as u16,
            MirAction::Standing,
            dir0,
            0,
        )
        .expect("cave statue 1 standing");
        assert_eq!(s1.start, 0);

        let s2 = get_monster_frame(
            MonsterKind::CaveStatue as u16,
            MirAction::Standing,
            dir1,
            0,
        )
        .expect("cave statue 2 standing");
        assert_eq!(s2.start, 18);
    }
}
