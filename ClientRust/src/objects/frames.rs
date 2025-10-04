// Frames.rs - Animation frame management
// Mirrors Client/MirObjects/Frames.cs

use mir2_shared::enums::MirAction;

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
