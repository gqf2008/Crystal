// Frames.rs - Animation frame management
// Mirrors Client/MirObjects/Frames.cs

use mir2_shared::enums::MirAction;

use crate::network::protocol::PlayerObject;

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

    pub(super) fn update_for_player(&mut self, player: &PlayerObject) -> bool {
        let desired_action = if player.dead {
            MirAction::Dead
        } else if player.hidden {
            MirAction::Hide
        } else if player.fishing {
            MirAction::FishingWait
        } else if player.riding_mount {
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
