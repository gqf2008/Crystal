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

/// Get player frame for a specific action
pub fn get_player_frame(action: MirAction) -> Option<&'static Frame> {
    PLAYER_FRAMES.get(&action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_frames_basic_sanity() {
        let standing = get_player_frame(MirAction::Standing).expect("standing frame");
        assert_eq!(standing.start, 0);
        assert_eq!(standing.count, 4);
        assert_eq!(standing.interval, 500);

        assert!(get_player_frame(MirAction::Walking).is_some());
        assert!(get_player_frame(MirAction::Attack1).is_some());
    }
}
