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
pub mod cave_maggot;
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
pub mod bone_spearman;
pub mod sand_worm;
pub mod shaman_zombie;
pub mod stray_cat;
pub mod dark_beast;
pub mod snow_wolf;
pub mod mud_zombie;
pub mod incarnated_ghoul;
pub mod incarnated_zt;
pub mod mantis;
pub mod sand_snail;
pub mod scaly_beast;
pub mod light_turtle;
pub mod hell_cannibal;
pub mod nadz;
pub mod hugger;
pub mod guard;
pub mod town_archer;
pub mod castle_gate;
pub mod dig_out_zombie;
pub mod red_moon_evil;
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
// ===== 第二批：17 个独特机制怪物 =====
pub mod horned_warrior;
pub mod manectric_king;
pub mod seedings_general;
pub mod tucson_general;
pub mod white_mammoth;
pub mod horned_archer;
pub mod khazard;
pub mod king_hydrax;
pub mod crystal_spider;
pub mod element_guard;
pub mod winged_tiger_lord;
pub mod great_fox_spirit;
pub mod stone_golem;
pub mod stone_trap;
pub mod tucson_mage;
pub mod oma_mage;
pub mod flaming_mutant;
pub mod dark_captain;

// ===== 第三批：25 个独特机制怪物 =====
pub mod water_dragon;
pub mod right_guard;
pub mod minotaur_king;
pub mod troll_king;
pub mod dark_devil;
pub mod dark_devourer;
pub mod hell_knight;
pub mod holy_deva;
pub mod shinsu;
pub mod harvest_monster;
pub mod hooded_summoner;
pub mod horned_sorceror;
pub mod earth_golem;
pub mod vampire_spider;
pub mod venom_spider;
pub mod thunder_element;
pub mod kirin;
pub mod toxic_ghoul;
pub mod troll_bomber;
pub mod plague_crab;
pub mod human_wizard;
pub mod human_assassin;
pub mod king_guard;
pub mod rhino_priest;
pub mod ice_phantom;
pub mod ancient_bringer;
