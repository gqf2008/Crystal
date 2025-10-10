// MirObjects - Game object system
// Mirrors the structure of Client/MirObjects/

mod drawable;  // NEW: DrawableMapObject trait for all drawable objects
pub(crate) mod frames;
#[cfg(test)]
mod frames_test;
mod map_object;
mod player_object;  // NEW: PlayerObject base class (Phase 1)
mod user_object;
mod monster_object;
mod npc_object;
mod item_object;
mod hero_object;
mod spell_object;
mod effect;
mod damage;
mod pathfinder;
mod map_code; // MapReader and CellInfo - 对应 Client/MirObjects/MapCode.cs
mod stats_ext;  // NEW: Stats system extensions
mod object_factory; // NEW: Factory for creating objects from server packets

pub use drawable::DrawableMapObject;  // NEW: Drawable trait
pub use frames::{AnimationAdvanceSummary, AnimationStep, Frame};
pub use map_object::{
    ActionResult, AttackOutcome, BuffDelta, MapObject, MapObjectType, ObjectActionOutcome,
    ObjectAttackOutcome, ObjectDeathOutcome, ObjectStruckOutcome, ObjectUpdateOutcome,
    StruckOutcome, SyncResult,
};

// NEW: Phase 1 - PlayerObject base class
pub use player_object::{PlayerObject, QueuedAction};

pub use user_object::UserObject;

// NEW: Stats extensions
pub use stats_ext::StatsExt;

// Re-export SharedRust types used by UserObject
// (follows C# dependency: Client depends on Shared)
pub use mir2_shared::data::item::ItemSets;  // C# Shared/Data/ItemData.cs ItemSets

// Re-export from mir2_shared (avoid duplication)
pub use mir2_shared::{
    data::client_data::{ClientMagic, ClientIntelligentCreature, ClientQuestProgress, ClientMail},
    enums::{EquipmentSlot, IntelligentCreatureType},
};
pub use monster_object::{MonsterObject, Monster, MonsterSoundType};
pub use npc_object::{NPCObject, NpcImage};
pub use item_object::ItemObject;
pub use hero_object::{HeroObject, HeroState};
pub use spell_object::SpellObject;
pub use effect::{Effect, EffectLayer, BlendMode};
pub use damage::{Damage, DamageType, Color};
pub use pathfinder::PathFinder;
pub use map_code::{CellInfo, MapReader}; // 对应 Client/MirObjects/MapCode.cs
pub use object_factory::ObjectFactory; // NEW: Object creation from packets
