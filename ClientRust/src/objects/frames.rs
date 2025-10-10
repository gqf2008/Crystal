// Frames.rs - Animation frame management
// Mirrors Client/MirObjects/Frames.cs

use mir2_shared::enums::MirAction;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::io::{Read, Result as IoResult};
use byteorder::{LittleEndian, ReadBytesExt};

/// Frame definition for animations
/// 
/// Mirrors C# Client/MirObjects/Frames.cs Frame class
/// 
/// A Frame defines how an animation plays:
/// - start: Starting frame index in the sprite sheet
/// - count: Number of frames to play
/// - skip: Number of frames to skip after animation ends
/// - interval: Time between frames (milliseconds)
/// - effect_*: Same properties for effect layer (wings, weapons, etc.)
/// - reverse: Play animation in reverse
/// - blend: Use alpha blending
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
    /// Create a new Frame
    /// 
    /// Mirrors C# Frame(int start, int count, int skip, int interval, ...)
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
    
    /// Create a basic frame without effects
    /// 
    /// Simplified constructor for common case
    pub fn basic(start: i32, count: i32, skip: i32, interval: i32) -> Self {
        Self::new(start, count, skip, interval, 0, 0, 0, 0)
    }
    
    /// Offset for frame iteration (count + skip)
    /// 
    /// Mirrors C# Frame.OffSet property
    pub fn offset(&self) -> i32 {
        self.count + self.skip
    }
    
    /// Offset for effect frame iteration
    /// 
    /// Mirrors C# Frame.EffectOffSet property
    pub fn effect_offset(&self) -> i32 {
        self.effect_count + self.effect_skip
    }
    
    /// Set reverse flag (builder pattern)
    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
    
    /// Set blend flag (builder pattern)
    pub fn with_blend(mut self, blend: bool) -> Self {
        self.blend = blend;
        self
    }
    
    /// Read Frame from a binary reader
    /// 
    /// Mirrors C# Frame(BinaryReader reader) constructor
    /// 
    /// # Format
    /// The binary format is:
    /// - Start: i32 (4 bytes)
    /// - Count: i32 (4 bytes)
    /// - Skip: i32 (4 bytes)
    /// - Interval: i32 (4 bytes)
    /// - EffectStart: i32 (4 bytes)
    /// - EffectCount: i32 (4 bytes)
    /// - EffectSkip: i32 (4 bytes)
    /// - EffectInterval: i32 (4 bytes)
    /// - Reverse: bool (1 byte)
    /// - Blend: bool (1 byte)
    /// 
    /// Total: 34 bytes
    pub fn from_reader<R: Read>(reader: &mut R) -> IoResult<Self> {
        let start = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_i32::<LittleEndian>()?;
        let skip = reader.read_i32::<LittleEndian>()?;
        let interval = reader.read_i32::<LittleEndian>()?;
        let effect_start = reader.read_i32::<LittleEndian>()?;
        let effect_count = reader.read_i32::<LittleEndian>()?;
        let effect_skip = reader.read_i32::<LittleEndian>()?;
        let effect_interval = reader.read_i32::<LittleEndian>()?;
        let reverse = reader.read_u8()? != 0;  // C# bool is 1 byte
        let blend = reader.read_u8()? != 0;
        
        Ok(Self {
            start,
            count,
            skip,
            interval,
            effect_start,
            effect_count,
            effect_skip,
            effect_interval,
            reverse,
            blend,
        })
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            start: 0,
            count: 1,
            skip: 0,
            interval: 100,
            effect_start: 0,
            effect_count: 0,
            effect_skip: 0,
            effect_interval: 0,
            reverse: false,
            blend: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct AnimationStep {
    pub frames_advanced: u32,
    pub completed_cycles: u32,
}

#[derive(Debug, Default)]
pub struct AnimationAdvanceSummary {
    pub objects_updated: usize,
    pub frames_advanced: u64,
    pub cycles_completed: u64,
}

impl AnimationAdvanceSummary {
    pub fn record_step(&mut self, step: &AnimationStep) {
        if step.frames_advanced > 0 {
            self.objects_updated += 1;
            self.frames_advanced += step.frames_advanced as u64;
        }
        self.cycles_completed += step.completed_cycles as u64;
    }
}

#[derive(Debug, Clone)]
pub(super) struct AnimationState {
    action: MirAction,
    frame_index: u8,
    frame_count: u8,
    frame_time_ms: u32,
    repeat: bool,
    elapsed_ms: u32,
}

impl Default for AnimationState {
    fn default() -> Self {
        let spec = animation_spec(MirAction::Standing);
        AnimationState {
            action: MirAction::Standing,
            frame_index: 0,
            frame_count: spec.frame_count,
            frame_time_ms: spec.frame_time_ms,
            repeat: spec.repeat,
            elapsed_ms: 0,
        }
    }
}

impl AnimationState {
    pub(super) fn current_action(&self) -> MirAction {
        self.action
    }

    /// Update animation state based on object state flags.
    /// Returns true if the action changed.
    pub(super) fn update_from_state(
        &mut self,
        dead: bool,
        hidden: bool,
        fishing: bool,
        riding_mount: bool,
    ) -> bool {
        let desired_action = if dead {
            MirAction::Dead
        } else if hidden {
            MirAction::Hide
        } else if fishing {
            MirAction::FishingWait
        } else if riding_mount {
            MirAction::MountStanding
        } else {
            MirAction::Standing
        };

        self.ensure_action(desired_action)
    }

    pub(super) fn ensure_action(&mut self, action: MirAction) -> bool {
        if self.action == action {
            return false;
        }

        self.set_action(action);
        true
    }

    pub(super) fn set_action(&mut self, action: MirAction) {
        let spec = animation_spec(action);
        self.action = action;
        self.frame_index = 0;
        self.frame_count = spec.frame_count.max(1);
        self.frame_time_ms = spec.frame_time_ms.max(1);
        self.repeat = spec.repeat;
        self.elapsed_ms = 0;
    }

    pub(super) fn tick(&mut self, delta_ms: u32) -> AnimationStep {
        let mut step = AnimationStep::default();

        if self.frame_count <= 1 {
            return step;
        }

        self.elapsed_ms = self.elapsed_ms.saturating_add(delta_ms);

        while self.elapsed_ms >= self.frame_time_ms {
            self.elapsed_ms -= self.frame_time_ms;
            self.frame_index += 1;
            step.frames_advanced += 1;

            if self.frame_index >= self.frame_count {
                step.completed_cycles += 1;
                if self.repeat {
                    self.frame_index = 0;
                } else {
                    self.frame_index = self.frame_count - 1;
                    self.elapsed_ms = 0;
                    break;
                }
            }
        }

        step
    }
}

#[derive(Debug, Clone, Copy)]
struct AnimationSpec {
    frame_count: u8,
    frame_time_ms: u32,
    repeat: bool,
}

fn animation_spec(action: MirAction) -> AnimationSpec {
    match action {
        MirAction::Standing | MirAction::Stance | MirAction::Stance2 | MirAction::MountStanding => {
            AnimationSpec {
                frame_count: 4,
                frame_time_ms: 400,
                repeat: true,
            }
        }
        MirAction::Walking
        | MirAction::Running
        | MirAction::MountWalking
        | MirAction::MountRunning
        | MirAction::WalkingBow
        | MirAction::RunningBow => AnimationSpec {
            frame_count: 6,
            frame_time_ms: 120,
            repeat: true,
        },
        MirAction::Attack1
        | MirAction::Attack2
        | MirAction::Attack3
        | MirAction::Attack4
        | MirAction::Attack5
        | MirAction::AttackRange1
        | MirAction::AttackRange2
        | MirAction::AttackRange3
        | MirAction::Special
        | MirAction::Spell
        | MirAction::Harvest
        | MirAction::DashAttack
        | MirAction::Lunge => AnimationSpec {
            frame_count: 6,
            frame_time_ms: 90,
            repeat: false,
        },
        MirAction::Struck | MirAction::MountStruck => AnimationSpec {
            frame_count: 3,
            frame_time_ms: 150,
            repeat: false,
        },
        MirAction::Die | MirAction::Dead | MirAction::Skeleton => AnimationSpec {
            frame_count: 10,
            frame_time_ms: 200,
            repeat: false,
        },
        MirAction::FishingCast | MirAction::FishingWait | MirAction::FishingReel => AnimationSpec {
            frame_count: 6,
            frame_time_ms: 220,
            repeat: true,
        },
        _ => AnimationSpec {
            frame_count: 6,
            frame_time_ms: 150,
            repeat: true,
        },
    }
}

/// FrameSet - Collection of frames for different actions
/// 
/// Mirrors C# Client/MirObjects/Frames.cs FrameSet class
/// Contains frame definitions for all actions of a character/monster/NPC
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

/// Static frame data for Default NPC
/// 
/// Mirrors C# FrameSet.DefaultNPC
pub static DEFAULT_NPC_FRAMES: LazyLock<FrameSet> = LazyLock::new(|| {
    let mut frames = HashMap::new();
    frames.insert(MirAction::Standing, Frame::basic(0, 4, 0, 450));
    frames.insert(MirAction::Harvest, Frame::basic(12, 10, 0, 200));
    frames
});

/// Static frame data for Default Monster
/// 
/// Mirrors C# FrameSet.DefaultMonster
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

/// Static frame data for DragonStatue variations
/// 
/// Mirrors C# FrameSet.DragonStatue
pub static DRAGON_STATUE_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut variations = Vec::new();
    
    // DragonStatue 1
    let mut frames1 = HashMap::new();
    frames1.insert(MirAction::Standing, Frame::basic(300, 1, -1, 1000));
    frames1.insert(MirAction::AttackRange1, Frame::basic(300, 1, -1, 120));
    frames1.insert(MirAction::Struck, Frame::basic(300, 1, -1, 200));
    variations.push(frames1);
    
    // DragonStatue 2
    let mut frames2 = HashMap::new();
    frames2.insert(MirAction::Standing, Frame::basic(301, 1, -1, 1000));
    frames2.insert(MirAction::AttackRange1, Frame::basic(301, 1, -1, 120));
    frames2.insert(MirAction::Struck, Frame::basic(301, 1, -1, 200));
    variations.push(frames2);
    
    // DragonStatue 3
    let mut frames3 = HashMap::new();
    frames3.insert(MirAction::Standing, Frame::basic(302, 1, -1, 1000));
    frames3.insert(MirAction::AttackRange1, Frame::basic(302, 1, -1, 120));
    frames3.insert(MirAction::Struck, Frame::basic(302, 1, -1, 200));
    variations.push(frames3);
    
    // DragonStatue 4
    let mut frames4 = HashMap::new();
    frames4.insert(MirAction::Standing, Frame::basic(320, 1, -1, 1000));
    frames4.insert(MirAction::AttackRange1, Frame::basic(320, 1, -1, 120));
    frames4.insert(MirAction::Struck, Frame::basic(320, 1, -1, 200));
    variations.push(frames4);
    
    // DragonStatue 5
    let mut frames5 = HashMap::new();
    frames5.insert(MirAction::Standing, Frame::basic(321, 1, -1, 1000));
    frames5.insert(MirAction::AttackRange1, Frame::basic(321, 1, -1, 120));
    frames5.insert(MirAction::Struck, Frame::basic(321, 1, -1, 200));
    variations.push(frames5);
    
    // DragonStatue 6
    let mut frames6 = HashMap::new();
    frames6.insert(MirAction::Standing, Frame::basic(322, 1, -1, 1000));
    frames6.insert(MirAction::AttackRange1, Frame::basic(322, 1, -1, 120));
    frames6.insert(MirAction::Struck, Frame::basic(322, 1, -1, 200));
    variations.push(frames6);
    
    variations
});

/// Static frame data for GreatFoxSpirit variations
/// 
/// Mirrors C# FrameSet.GreatFoxSpirit
pub static GREAT_FOX_SPIRIT_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut variations = Vec::new();
    
    // GreatFoxSpirit level 0
    let mut frames0 = HashMap::new();
    frames0.insert(MirAction::Standing, Frame::basic(0, 20, -20, 100));
    frames0.insert(MirAction::Attack1, Frame::basic(22, 8, -8, 120));
    frames0.insert(MirAction::Struck, Frame::basic(20, 2, -2, 200));
    frames0.insert(MirAction::Die, Frame::basic(300, 18, -18, 120));
    frames0.insert(MirAction::Dead, Frame::basic(317, 1, -1, 1000));
    frames0.insert(MirAction::Revive, Frame::basic(300, 18, -18, 150).with_reverse(true));
    variations.push(frames0);
    
    // GreatFoxSpirit level 1
    let mut frames1 = HashMap::new();
    frames1.insert(MirAction::Standing, Frame::basic(60, 20, -20, 100));
    frames1.insert(MirAction::Attack1, Frame::basic(82, 8, -8, 120));
    frames1.insert(MirAction::Struck, Frame::basic(80, 2, -2, 200));
    frames1.insert(MirAction::Die, Frame::basic(300, 18, -18, 120));
    frames1.insert(MirAction::Dead, Frame::basic(317, 1, -1, 1000));
    frames1.insert(MirAction::Revive, Frame::basic(300, 18, -18, 150).with_reverse(true));
    variations.push(frames1);
    
    // GreatFoxSpirit level 2
    let mut frames2 = HashMap::new();
    frames2.insert(MirAction::Standing, Frame::basic(120, 20, -20, 100));
    frames2.insert(MirAction::Attack1, Frame::basic(142, 8, -8, 120));
    frames2.insert(MirAction::Struck, Frame::basic(140, 2, -2, 200));
    frames2.insert(MirAction::Die, Frame::basic(300, 18, -18, 120));
    frames2.insert(MirAction::Dead, Frame::basic(317, 1, -1, 1000));
    frames2.insert(MirAction::Revive, Frame::basic(300, 18, -18, 150).with_reverse(true));
    variations.push(frames2);
    
    // GreatFoxSpirit level 3
    let mut frames3 = HashMap::new();
    frames3.insert(MirAction::Standing, Frame::basic(180, 20, -20, 100));
    frames3.insert(MirAction::Attack1, Frame::basic(202, 8, -8, 120));
    frames3.insert(MirAction::Struck, Frame::basic(200, 2, -2, 200));
    frames3.insert(MirAction::Die, Frame::basic(300, 18, -18, 120));
    frames3.insert(MirAction::Dead, Frame::basic(317, 1, -1, 1000));
    frames3.insert(MirAction::Revive, Frame::basic(300, 18, -18, 150).with_reverse(true));
    variations.push(frames3);
    
    // GreatFoxSpirit level 4
    let mut frames4 = HashMap::new();
    frames4.insert(MirAction::Standing, Frame::basic(240, 20, -20, 100));
    frames4.insert(MirAction::Attack1, Frame::basic(262, 8, -8, 120));
    frames4.insert(MirAction::Struck, Frame::basic(260, 2, -2, 200));
    frames4.insert(MirAction::Die, Frame::basic(300, 18, -18, 120));
    frames4.insert(MirAction::Dead, Frame::basic(317, 1, -1, 1000));
    frames4.insert(MirAction::Revive, Frame::basic(300, 18, -18, 150).with_reverse(true));
    variations.push(frames4);
    
    variations
});

/// Static frame data for HellBomb variations
/// 
/// Mirrors C# FrameSet.HellBomb
pub static HELL_BOMB_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut variations = Vec::new();
    
    // HellBomb1
    let mut frames1 = HashMap::new();
    frames1.insert(MirAction::Standing, Frame::basic(52, 9, -9, 100).with_blend(true));
    frames1.insert(MirAction::Attack1, Frame::basic(999, 1, -1, 120).with_blend(true));
    frames1.insert(MirAction::Struck, Frame::basic(52, 9, -9, 100).with_blend(true));
    variations.push(frames1);
    
    // HellBomb2
    let mut frames2 = HashMap::new();
    frames2.insert(MirAction::Standing, Frame::basic(70, 9, -9, 100).with_blend(true));
    frames2.insert(MirAction::Attack1, Frame::basic(999, 1, -1, 120).with_blend(true));
    frames2.insert(MirAction::Struck, Frame::basic(70, 9, -9, 100).with_blend(true));
    variations.push(frames2);
    
    // HellBomb3
    let mut frames3 = HashMap::new();
    frames3.insert(MirAction::Standing, Frame::basic(88, 9, -9, 100).with_blend(true));
    frames3.insert(MirAction::Attack1, Frame::basic(999, 1, -1, 120).with_blend(true));
    frames3.insert(MirAction::Struck, Frame::basic(88, 9, -9, 100).with_blend(true));
    variations.push(frames3);
    
    variations
});

/// Static frame data for CaveStatue variations
/// 
/// Mirrors C# FrameSet.CaveStatue
pub static CAVE_STATUE_FRAMES: LazyLock<Vec<FrameSet>> = LazyLock::new(|| {
    let mut variations = Vec::new();
    
    // CaveStatue1
    let mut frames1 = HashMap::new();
    frames1.insert(MirAction::Standing, Frame::basic(0, 1, -1, 100).with_blend(false));
    frames1.insert(MirAction::Struck, Frame::basic(0, 1, -1, 100).with_blend(false));
    frames1.insert(MirAction::Die, Frame::basic(2, 8, -8, 100).with_blend(false));
    frames1.insert(MirAction::Dead, Frame::basic(9, 1, -1, 100).with_blend(false));
    variations.push(frames1);
    
    // CaveStatue2
    let mut frames2 = HashMap::new();
    frames2.insert(MirAction::Standing, Frame::basic(18, 1, -1, 100).with_blend(false));
    frames2.insert(MirAction::Struck, Frame::basic(18, 1, -1, 100).with_blend(false));
    frames2.insert(MirAction::Die, Frame::basic(20, 8, -8, 100).with_blend(false));
    frames2.insert(MirAction::Dead, Frame::basic(27, 1, -1, 100).with_blend(false));
    variations.push(frames2);
    
    variations
});

/// Get frame for a specific action from a frameset
/// 
/// Helper function to safely retrieve frame data
pub fn get_frame(frameset: &FrameSet, action: MirAction) -> Option<&Frame> {
    frameset.get(&action)
}

/// Get player frame for a specific action
pub fn get_player_frame(action: MirAction) -> Option<&'static Frame> {
    PLAYER_FRAMES.get(&action)
}

/// Get default NPC frame for a specific action
pub fn get_default_npc_frame(action: MirAction) -> Option<&'static Frame> {
    DEFAULT_NPC_FRAMES.get(&action)
}

/// Get default monster frame for a specific action
pub fn get_default_monster_frame(action: MirAction) -> Option<&'static Frame> {
    DEFAULT_MONSTER_FRAMES.get(&action)
}
