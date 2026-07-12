//! 8 个核心 Boss behavior
//!
//! 覆盖全部独特机制：守点不动/睡眠/多阶段/召唤/分身/法术场/免疫/传送/定时器

pub mod evil_mir;
pub mod horned_commander;
pub mod hell_lord;
pub mod tree_queen;
pub mod yimoogi;
pub mod dark_oma_king;
pub mod general_meow_meow;
pub mod zuma_taurus;

// ===== 额外 Boss（10 个，独特机制）=====
pub mod evil_centipede;
pub mod cannibal_plant;
pub mod oma_king;
pub mod wooma_taurus;
pub mod flame_queen;
pub mod snow_wolf_king;
pub mod turtle_king;
pub mod behemoth;
pub mod left_guard;
pub mod hell_keeper;

// ===== 普通怪物专属 behavior（25 个，独特机制）=====
pub mod zuma_monster;
pub mod axe_skeleton;
pub mod spitting_spider;
pub mod bug_bag_maggot;
pub mod bomb_spider;
pub mod hugger;
pub mod guard;
pub mod town_archer;
pub mod castle_gate;
pub mod dig_out_zombie;
pub mod reviving_zombie;
pub mod jar1;
pub mod armadillo;
pub mod gas_toad;
pub mod stoning_statue;
pub mod bone_lord;
pub mod cat_shaman;
pub mod root_spider;
pub mod flame_mage;
pub mod tornado;
pub mod poison_hugger;
pub mod horned_mage;
pub mod witch_doctor;
pub mod king_scorpion;
pub mod furbolg_warrior;
pub mod crazy_manworm;
pub mod flaming_wooma;
pub mod ice_guard;
