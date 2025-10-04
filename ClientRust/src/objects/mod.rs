// MirObjects - Game object system
// Mirrors the structure of Client/MirObjects/

mod frames;
mod map_object;
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

pub use frames::{AnimationAdvanceSummary, AnimationStep};
pub use map_object::{
    ActionResult, AttackOutcome, BuffDelta, MapObject, MapObjectType, ObjectActionOutcome,
    ObjectAttackOutcome, ObjectDeathOutcome, ObjectStruckOutcome, ObjectUpdateOutcome,
    StruckOutcome, SyncResult,
};
pub use user_object::{
    UserObject, ClientMagic, ItemSets, EquipmentSlot, ClientIntelligentCreature,
    IntelligentCreatureType, ClientQuestProgress, ClientMail, QueuedAction,
    QueuedActionType, SpecialItemMode,
};
pub use monster_object::{MonsterObject, Monster, MonsterSoundType};
pub use npc_object::{NPCObject, NpcImage};
pub use item_object::ItemObject;
pub use hero_object::{HeroObject, HeroState};
pub use spell_object::SpellObject;
pub use effect::{Effect, EffectLayer, BlendMode};
pub use damage::{Damage, DamageType, Color};
pub use pathfinder::PathFinder;
pub use map_code::{MapReader, CellInfo}; // 对应 Client/MirObjects/MapCode.cs
