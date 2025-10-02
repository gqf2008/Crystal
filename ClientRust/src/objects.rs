use std::time::Instant;

use mir2_shared::{
    enums::{BuffType, MirAction, MirDirection, Spell},
    Point,
};

use crate::protocol::{HeroObject, ObjectMonster, PlayerObject};

#[derive(Debug, Clone)]
enum MapObjectKind {
    Player(PlayerObject),
    Hero(HeroObject),
    Monster(ObjectMonster),
}

impl MapObjectKind {
    fn object_id(&self) -> u32 {
        match self {
            MapObjectKind::Player(player) => player.object_id,
            MapObjectKind::Hero(hero) => hero.player.object_id,
            MapObjectKind::Monster(monster) => monster.object_id,
        }
    }

    fn player(&self) -> &PlayerObject {
        match self {
            MapObjectKind::Player(player) => player,
            MapObjectKind::Hero(hero) => &hero.player,
            MapObjectKind::Monster(_) => panic!("Monster does not have a PlayerObject field"),
        }
    }

    fn player_mut(&mut self) -> &mut PlayerObject {
        match self {
            MapObjectKind::Player(player) => player,
            MapObjectKind::Hero(hero) => &mut hero.player,
            MapObjectKind::Monster(_) => panic!("Monster does not have a PlayerObject field"),
        }
    }

    fn replace_with_player(&mut self, player: PlayerObject) {
        match self {
            MapObjectKind::Player(target) => *target = player,
            MapObjectKind::Hero(hero) => hero.player = player,
            MapObjectKind::Monster(_) => {}
        }
    }

    fn replace_with_hero(&mut self, hero: HeroObject) {
        *self = MapObjectKind::Hero(hero);
    }

    fn replace_with_monster(&mut self, monster: ObjectMonster) {
        *self = MapObjectKind::Monster(monster);
    }

    fn monster(&self) -> &ObjectMonster {
        match self {
            MapObjectKind::Monster(monster) => monster,
            _ => panic!("Only Monster objects have monster field"),
        }
    }

    fn monster_mut(&mut self) -> &mut ObjectMonster {
        match self {
            MapObjectKind::Monster(monster) => monster,
            _ => panic!("Only Monster objects have monster field"),
        }
    }

    fn object_type(&self) -> MapObjectType {
        match self {
            MapObjectKind::Player(_) => MapObjectType::Player,
            MapObjectKind::Hero(_) => MapObjectType::Hero,
            MapObjectKind::Monster(_) => MapObjectType::Monster,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectType {
    Player,
    Hero,
    Monster,
}

#[derive(Debug, Clone)]
pub struct MapObject {
    kind: MapObjectKind,
    buffs: BuffState,
    animation: AnimationState,
    location: Point,
    direction: MirDirection,
    last_update: Instant,
}

impl MapObject {
    pub fn from_player(player: PlayerObject) -> (Self, SyncResult) {
        let mut buffs = BuffState::default();
        let buff_delta = buffs.replace(player.buffs.as_slice());
        let mut animation = AnimationState::default();
        animation.update_for_player(&player);
        let action = animation.current_action();
        let location = player.location;
        let direction = player.direction;
        let object = MapObject {
            kind: MapObjectKind::Player(player),
            buffs,
            animation,
            location,
            direction,
            last_update: Instant::now(),
        };
        (
            object,
            SyncResult {
                buff_delta,
                action_before: action,
                action_after: action,
            },
        )
    }

    pub fn from_hero(hero: HeroObject) -> (Self, SyncResult) {
        let mut buffs = BuffState::default();
        let buff_delta = buffs.replace(hero.player.buffs.as_slice());
        let mut animation = AnimationState::default();
        animation.update_for_player(&hero.player);
        let action = animation.current_action();
        let location = hero.player.location;
        let direction = hero.player.direction;
        let object = MapObject {
            kind: MapObjectKind::Hero(hero),
            buffs,
            animation,
            location,
            direction,
            last_update: Instant::now(),
        };
        (
            object,
            SyncResult {
                buff_delta,
                action_before: action,
                action_after: action,
            },
        )
    }

    pub fn from_monster(monster: ObjectMonster) -> (Self, SyncResult) {
        let mut buffs = BuffState::default();
        let buff_delta = buffs.replace(monster.buffs.as_slice());
        let mut animation = AnimationState::default();
        let action = if monster.dead {
            MirAction::Dead
        } else {
            MirAction::Standing
        };
        animation.set_action(action);
        let location = monster.location;
        let direction = monster.direction;
        let object = MapObject {
            kind: MapObjectKind::Monster(monster),
            buffs,
            animation,
            location,
            direction,
            last_update: Instant::now(),
        };
        (
            object,
            SyncResult {
                buff_delta,
                action_before: action,
                action_after: action,
            },
        )
    }

    pub fn object_id(&self) -> u32 {
        self.kind.object_id()
    }

    pub fn object_type(&self) -> MapObjectType {
        self.kind.object_type()
    }

    pub fn location(&self) -> Point {
        self.location
    }

    pub fn direction(&self) -> MirDirection {
        self.direction
    }

    pub fn current_action(&self) -> MirAction {
        self.animation.current_action()
    }

    pub fn is_hidden(&self) -> bool {
        self.kind.player().hidden
    }

    pub fn is_dead(&self) -> bool {
        self.kind.player().dead
    }

    pub fn level(&self) -> u16 {
        self.kind.player().level
    }

    pub fn set_level(&mut self, level: u16) -> u16 {
        let previous = self.kind.player().level;
        self.kind.player_mut().level = level;
        previous
    }

    pub fn name_colour_argb(&self) -> i32 {
        self.kind.player().name_colour_argb
    }

    pub fn set_name_colour_argb(&mut self, new_colour: i32) -> i32 {
        let previous = self.kind.player().name_colour_argb;
        self.kind.player_mut().name_colour_argb = new_colour;
        previous
    }

    pub fn guild_name(&self) -> &str {
        self.kind.player().guild_name.as_str()
    }

    pub fn set_guild_name(&mut self, guild_name: String) -> String {
        let previous = self.kind.player().guild_name.clone();
        self.kind.player_mut().guild_name = guild_name;
        previous
    }

    pub fn sync_player(&mut self, player: PlayerObject) -> SyncResult {
        let action_before = self.animation.current_action();
        self.kind.replace_with_player(player);
        let buff_delta = self.buffs.replace(self.kind.player().buffs.as_slice());
        let _ = self.animation.update_for_player(self.kind.player());
        self.update_transform_from_kind();
        let action_after = self.animation.current_action();
        self.last_update = Instant::now();
        SyncResult {
            buff_delta,
            action_before,
            action_after,
        }
    }

    pub fn sync_hero(&mut self, hero: HeroObject) -> SyncResult {
        let action_before = self.animation.current_action();
        self.kind.replace_with_hero(hero);
        let buff_delta = self.buffs.replace(self.kind.player().buffs.as_slice());
        let _ = self.animation.update_for_player(self.kind.player());
        self.update_transform_from_kind();
        let action_after = self.animation.current_action();
        self.last_update = Instant::now();
        SyncResult {
            buff_delta,
            action_before,
            action_after,
        }
    }

    pub fn sync_monster(&mut self, monster: ObjectMonster) -> SyncResult {
        let action_before = self.animation.current_action();
        let buff_delta = self.buffs.replace(monster.buffs.as_slice());
        let new_action = if monster.dead {
            MirAction::Dead
        } else {
            action_before
        };
        self.animation.set_action(new_action);
        self.location = monster.location;
        self.direction = monster.direction;
        self.kind.replace_with_monster(monster);
        let action_after = self.animation.current_action();
        self.last_update = Instant::now();
        SyncResult {
            buff_delta,
            action_before,
            action_after,
        }
    }

    pub fn advance(&mut self, delta_ms: u32) -> AnimationStep {
        let step = self.animation.tick(delta_ms);
        if step.frames_advanced > 0 || step.completed_cycles > 0 {
            self.last_update = Instant::now();
        }
        step
    }

    pub fn apply_attack(
        &mut self,
        direction: MirDirection,
        location: Point,
        spell: Spell,
        level: u8,
        attack_type: u8,
    ) -> AttackOutcome {
        let action = self.attack_action_for_type(attack_type);
        let transition = self.apply_action(action, direction, location);
        AttackOutcome {
            transition,
            spell,
            level,
            attack_type,
        }
    }

    pub fn apply_struck(
        &mut self,
        direction: MirDirection,
        location: Point,
        attacker_id: u32,
    ) -> StruckOutcome {
        let transition = self.apply_action(MirAction::Struck, direction, location);
        StruckOutcome {
            transition,
            attacker_id,
        }
    }

    pub fn apply_action(
        &mut self,
        action: MirAction,
        direction: MirDirection,
        location: Point,
    ) -> ActionResult {
        let action_before = self.animation.current_action();
        let direction_before = self.direction;
        let location_before = self.location;
        let action_changed = self.animation.ensure_action(action);
        self.set_transform(direction, location);
        let action_after = self.animation.current_action();
        self.last_update = Instant::now();

        ActionResult {
            action_before,
            action_after,
            direction_before,
            direction_after: self.direction,
            location_before,
            location_after: self.location,
            action_changed,
        }
    }

    pub fn apply_death(&mut self, direction: MirDirection, location: Point) -> ActionResult {
        match &mut self.kind {
            MapObjectKind::Player(player) => {
                player.dead = true;
            }
            MapObjectKind::Hero(hero) => {
                hero.player.dead = true;
            }
            MapObjectKind::Monster(monster) => {
                monster.dead = true;
            }
        }
        self.apply_action(MirAction::Die, direction, location)
    }

    fn attack_action_for_type(&self, attack_type: u8) -> MirAction {
        match self.object_type() {
            MapObjectType::Player => MirAction::Attack1,
            MapObjectType::Hero => match attack_type {
                1 => MirAction::Attack2,
                2 => MirAction::Attack3,
                3 => MirAction::Attack4,
                4 => MirAction::Attack5,
                _ => MirAction::Attack1,
            },
            MapObjectType::Monster => match attack_type {
                1 => MirAction::Attack2,
                2 => MirAction::Attack3,
                3 => MirAction::Attack4,
                _ => MirAction::Attack1,
            },
        }
    }

    fn update_transform_from_kind(&mut self) {
        match &self.kind {
            MapObjectKind::Player(player) | MapObjectKind::Hero(HeroObject { player, .. }) => {
                self.location = player.location;
                self.direction = player.direction;
            }
            MapObjectKind::Monster(monster) => {
                self.location = monster.location;
                self.direction = monster.direction;
            }
        }
    }

    fn set_transform(&mut self, direction: MirDirection, location: Point) {
        match &mut self.kind {
            MapObjectKind::Player(player) => {
                player.direction = direction;
                player.location = location;
            }
            MapObjectKind::Hero(hero) => {
                hero.player.direction = direction;
                hero.player.location = location;
            }
            MapObjectKind::Monster(monster) => {
                monster.direction = direction;
                monster.location = location;
            }
        }
        self.direction = direction;
        self.location = location;
    }
}

#[derive(Debug, Clone, Default)]
struct BuffState {
    active: Vec<BuffType>,
}

impl BuffState {
    fn replace(&mut self, incoming: &[BuffType]) -> BuffDelta {
        let mut added = Vec::new();
        for buff in incoming {
            if !self.active.contains(buff) {
                added.push(*buff);
            }
        }

        let mut removed = Vec::new();
        for buff in &self.active {
            if !incoming.contains(buff) {
                removed.push(*buff);
            }
        }

        self.active = incoming.to_vec();

        BuffDelta { added, removed }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuffDelta {
    pub added: Vec<BuffType>,
    pub removed: Vec<BuffType>,
}

impl BuffDelta {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub buff_delta: BuffDelta,
    pub action_before: MirAction,
    pub action_after: MirAction,
}

impl SyncResult {
    pub fn action_changed(&self) -> bool {
        self.action_before != self.action_after
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ActionResult {
    pub action_before: MirAction,
    pub action_after: MirAction,
    pub direction_before: MirDirection,
    pub direction_after: MirDirection,
    pub location_before: Point,
    pub location_after: Point,
    pub action_changed: bool,
}

impl ActionResult {
    pub fn moved(&self) -> bool {
        self.location_before != self.location_after
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AttackOutcome {
    pub transition: ActionResult,
    pub spell: Spell,
    pub level: u8,
    pub attack_type: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct StruckOutcome {
    pub transition: ActionResult,
    pub attacker_id: u32,
}

#[derive(Debug, Clone)]
pub struct ObjectUpdateOutcome {
    pub created: bool,
    pub object_type: MapObjectType,
    pub sync: SyncResult,
}

#[derive(Debug, Clone)]
pub struct ObjectActionOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub result: ActionResult,
}

#[derive(Debug, Clone)]
pub struct ObjectAttackOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub attack: AttackOutcome,
}

#[derive(Debug, Clone)]
pub struct ObjectStruckOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub struck: StruckOutcome,
}

#[derive(Debug, Clone)]
pub struct ObjectDeathOutcome {
    pub object_id: u32,
    pub object_type: MapObjectType,
    pub death_type: u8,
    pub transition: Option<ActionResult>,
    pub removed: bool,
    pub location: Point,
    pub direction: MirDirection,
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

#[derive(Debug, Default)]
pub struct AnimationStep {
    pub frames_advanced: u32,
    pub completed_cycles: u32,
}

#[derive(Debug, Clone)]
struct AnimationState {
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
    fn current_action(&self) -> MirAction {
        self.action
    }

    fn update_for_player(&mut self, player: &PlayerObject) -> bool {
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

    fn ensure_action(&mut self, action: MirAction) -> bool {
        if self.action == action {
            return false;
        }

        self.set_action(action);
        true
    }

    fn set_action(&mut self, action: MirAction) {
        let spec = animation_spec(action);
        self.action = action;
        self.frame_index = 0;
        self.frame_count = spec.frame_count.max(1);
        self.frame_time_ms = spec.frame_time_ms.max(1);
        self.repeat = spec.repeat;
        self.elapsed_ms = 0;
    }

    fn tick(&mut self, delta_ms: u32) -> AnimationStep {
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
