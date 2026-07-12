//! Boss behavior 注册表（按怪物名称匹配，对齐 C# Settings 的字符串配置）
//!
//! C# 用 `Settings.HornedCommanderMob = "HornedSorceror"` 等字符串配置 Boss 召唤物。
//! Rust 端 Boss 本体用名称匹配注册 behavior。

use super::behavior::MonsterBehavior;
use super::default::DefaultBehavior;
use super::bosses;

/// 根据怪物名称构建 behavior（Boss 返回专属 impl，其他返回 DefaultBehavior）
pub fn make_behavior(monster_name: &str) -> Box<dyn MonsterBehavior + Send + Sync> {
    let name = monster_name.to_lowercase();
    // 去除可能的空格/后缀，匹配 Boss 名称
    if name.contains("evilmir") || name.contains("evil mir") || name.contains("邪恶巨龙") {
        return Box::new(bosses::evil_mir::EvilMirBehavior::new());
    }
    if name.contains("hornedcommander") || name.contains("horned commander") || name.contains("角魔统帅") {
        return Box::new(bosses::horned_commander::HornedCommanderBehavior::new());
    }
    if name.contains("helllord") || name.contains("hell lord") || name.contains("地狱领主") {
        return Box::new(bosses::hell_lord::HellLordBehavior::new());
    }
    if name.contains("treequeen") || name.contains("tree queen") || name.contains("树后") {
        return Box::new(bosses::tree_queen::TreeQueenBehavior::new());
    }
    if name.contains("yimoogi") || name.contains("异魔蛇") || name.contains("蛇母") {
        return Box::new(bosses::yimoogi::YimoogiBehavior::new());
    }
    if name.contains("darkomaking") || name.contains("dark oma king") || name.contains("暗黑奥玛") || name.contains("奥玛之王") {
        return Box::new(bosses::dark_oma_king::DarkOmaKingBehavior::new());
    }
    if name.contains("generalmeowmeow") || name.contains("general meow meow") || name.contains("喵喵将军") {
        return Box::new(bosses::general_meow_meow::GeneralMeowMeowBehavior::new());
    }
    if name.contains("zumataurus") || name.contains("zuma taurus") || name.contains("祖玛教主") || name.contains("祖玛金牛") {
        return Box::new(bosses::zuma_taurus::ZumaTaurusBehavior::new());
    }
    // ===== 额外 Boss（10 个）=====
    if name.contains("evilcentipede") || name.contains("evil centipede") || name.contains("地蜈蚣") || name.contains("触角恶魔") {
        return Box::new(bosses::evil_centipede::EvilCentipedeBehavior::new());
    }
    if name.contains("cannibalplant") || name.contains("cannibal plant") || name.contains("食人花") {
        return Box::new(bosses::cannibal_plant::CannibalPlantBehavior::new());
    }
    if name.contains("omaking") || name.contains("oma king") || name.contains("奥玛王") {
        return Box::new(bosses::oma_king::OmaKingBehavior::new());
    }
    if name.contains("woomataurus") || name.contains("wooma taurus") || name.contains("沃玛教主") {
        return Box::new(bosses::wooma_taurus::WoomaTaurusBehavior::new());
    }
    if name.contains("flamequeen") || name.contains("flame queen") || name.contains("火焰女王") || name.contains("烈焰女王") {
        return Box::new(bosses::flame_queen::FlameQueenBehavior::new());
    }
    if name.contains("snowwolfking") || name.contains("snow wolf king") || name.contains("雪狼王") {
        return Box::new(bosses::snow_wolf_king::SnowWolfKingBehavior::new());
    }
    if name.contains("turtleking") || name.contains("turtle king") || name.contains("龟丞相") || name.contains("龟王") {
        return Box::new(bosses::turtle_king::TurtleKingBehavior::new());
    }
    if name.contains("behemoth") || name.contains("巨兽") {
        return Box::new(bosses::behemoth::BehemothBehavior::new());
    }
    if name.contains("leftguard") || name.contains("left guard") || name.contains("左护卫") {
        return Box::new(bosses::left_guard::LeftGuardBehavior::new());
    }
    if name.contains("hellkeeper") || name.contains("hell keeper") || name.contains("地狱守门人") {
        return Box::new(bosses::hell_keeper::HellKeeperBehavior::new());
    }

    // ===== 普通怪物专属 behavior（25 个，独特机制）=====
    if name.contains("zumamonster") || name.contains("zuma monster") || name.contains("祖玛怪") {
        return Box::new(bosses::zuma_monster::ZumaMonsterBehavior::new());
    }
    if name.contains("axeskeleton") || name.contains("axe skeleton") || name.contains("掷斧骷髅") || name.contains("斧骷髅") {
        return Box::new(bosses::axe_skeleton::AxeSkeletonBehavior::new());
    }
    if name.contains("spittingspider") || name.contains("spitting spider") || name.contains("吐丝蜘蛛") {
        return Box::new(bosses::spitting_spider::SpittingSpiderBehavior::new());
    }
    if name.contains("bugbagmaggot") || name.contains("bugbag maggot") || name.contains("虫袋蛆") {
        return Box::new(bosses::bug_bag_maggot::BugBagMaggotBehavior::new());
    }
    if name.contains("bombspider") || name.contains("bomb spider") || name.contains("炸弹蜘蛛") {
        return Box::new(bosses::bomb_spider::BombSpiderBehavior::new());
    }
    if name == "hugger" || name.contains("hugger") {
        return Box::new(bosses::hugger::HuggerBehavior::new());
    }
    if name == "guard" || name.contains("guard") && !name.contains("town") {
        return Box::new(bosses::guard::GuardBehavior::new());
    }
    if name.contains("townarcher") || name.contains("town archer") || name.contains("城镇弓箭手") {
        return Box::new(bosses::town_archer::TownArcherBehavior::new());
    }
    if name.contains("castlegate") || name.contains("castle gate") || name.contains("城门") {
        return Box::new(bosses::castle_gate::CastleGateBehavior::new());
    }
    if name.contains("digoutzombie") || name.contains("digout zombie") || name.contains("钻地僵尸") {
        return Box::new(bosses::dig_out_zombie::DigOutZombieBehavior::new());
    }
    if name.contains("revivingzombie") || name.contains("reviving zombie") || name.contains("复活僵尸") {
        return Box::new(bosses::reviving_zombie::RevivingZombieBehavior::new());
    }
    if name == "jar1" || name.contains("jar1") || name.contains("坛子") {
        return Box::new(bosses::jar1::Jar1Behavior::new());
    }
    if name == "armadillo" || name.contains("armadillo") || name.contains("犰狳") {
        return Box::new(bosses::armadillo::ArmadilloBehavior::new());
    }
    if name.contains("gastoad") || name.contains("gas toad") || name.contains("毒气蟾蜍") {
        return Box::new(bosses::gas_toad::GasToadBehavior::new());
    }
    if name.contains("stoningstatue") || name.contains("stoning statue") || name.contains("石化雕像") {
        return Box::new(bosses::stoning_statue::StoningStatueBehavior::new());
    }
    if name.contains("bonelord") || name.contains("bone lord") || name.contains("骨魔领主") {
        return Box::new(bosses::bone_lord::BoneLordBehavior::new());
    }
    if name.contains("catshaman") || name.contains("cat shaman") || name.contains("猫巫师") {
        return Box::new(bosses::cat_shaman::CatShamanBehavior::new());
    }
    if name.contains("rootspider") || name.contains("root spider") || name.contains("根须蜘蛛") {
        return Box::new(bosses::root_spider::RootSpiderBehavior::new());
    }
    if name.contains("flamemage") || name.contains("flame mage") || name.contains("火焰法师") {
        return Box::new(bosses::flame_mage::FlameMageBehavior::new());
    }
    if name == "tornado" || name.contains("tornado") || name.contains("龙卷风") {
        return Box::new(bosses::tornado::TornadoBehavior::new());
    }
    if name.contains("poisonhugger") || name.contains("poison hugger") || name.contains("毒抱怪") {
        return Box::new(bosses::poison_hugger::PoisonHuggerBehavior::new());
    }
    if name.contains("hornedmage") || name.contains("horned mage") || name.contains("角魔法师") {
        return Box::new(bosses::horned_mage::HornedMageBehavior::new());
    }
    if name.contains("witchdoctor") || name.contains("witch doctor") || name.contains("巫医") {
        return Box::new(bosses::witch_doctor::WitchDoctorBehavior::new());
    }
    if name.contains("kingscorpion") || name.contains("king scorpion") || name.contains("蝎子王") {
        return Box::new(bosses::king_scorpion::KingScorpionBehavior::new());
    }
    if name.contains("furbolgwarrior") || name.contains("furbolg warrior") || name.contains("熊人战士") {
        return Box::new(bosses::furbolg_warrior::FurbolgWarriorBehavior::new());
    }
    if name.contains("crazymanworm") || name.contains("crazy manworm") || name.contains("狂化人面虫") {
        return Box::new(bosses::crazy_manworm::CrazyManwormBehavior::new());
    }
    if name.contains("flamingwooma") || name.contains("flaming wooma") || name.contains("烈焰沃玛") {
        return Box::new(bosses::flaming_wooma::FlamingWoomaBehavior::new());
    }
    if name.contains("iceguard") || name.contains("ice guard") || name.contains("冰守卫") {
        return Box::new(bosses::ice_guard::IceGuardBehavior::new());
    }
    Box::new(DefaultBehavior::new())
}

/// 判断是否为已注册的 Boss（用于 tick_monsters 分发）
pub fn is_registered_boss(monster_name: &str) -> bool {
    let name = monster_name.to_lowercase();
    name.contains("evilmir") || name.contains("evil mir") || name.contains("邪恶巨龙")
        || name.contains("hornedcommander") || name.contains("horned commander") || name.contains("角魔统帅")
        || name.contains("helllord") || name.contains("hell lord") || name.contains("地狱领主")
        || name.contains("treequeen") || name.contains("tree queen") || name.contains("树后")
        || name.contains("yimoogi") || name.contains("异魔蛇") || name.contains("蛇母")
        || name.contains("darkomaking") || name.contains("dark oma king") || name.contains("暗黑奥玛") || name.contains("奥玛之王")
        || name.contains("generalmeowmeow") || name.contains("general meow meow") || name.contains("喵喵将军")
        || name.contains("zumataurus") || name.contains("zuma taurus") || name.contains("祖玛教主") || name.contains("祖玛金牛")
        || name.contains("evilcentipede") || name.contains("evil centipede") || name.contains("地蜈蚣") || name.contains("触角恶魔")
        || name.contains("cannibalplant") || name.contains("cannibal plant") || name.contains("食人花")
        || name.contains("omaking") || name.contains("oma king") || name.contains("奥玛王")
        || name.contains("woomataurus") || name.contains("wooma taurus") || name.contains("沃玛教主")
        || name.contains("flamequeen") || name.contains("flame queen") || name.contains("火焰女王") || name.contains("烈焰女王")
        || name.contains("snowwolfking") || name.contains("snow wolf king") || name.contains("雪狼王")
        || name.contains("turtleking") || name.contains("turtle king") || name.contains("龟丞相") || name.contains("龟王")
        || name.contains("behemoth") || name.contains("巨兽")
        || name.contains("leftguard") || name.contains("left guard") || name.contains("左护卫")
        || name.contains("hellkeeper") || name.contains("hell keeper") || name.contains("地狱守门人")
        // 普通怪物专属 behavior
        || name.contains("zumamonster") || name.contains("zuma monster") || name.contains("祖玛怪")
        || name.contains("axeskeleton") || name.contains("axe skeleton") || name.contains("掷斧骷髅") || name.contains("斧骷髅")
        || name.contains("spittingspider") || name.contains("spitting spider") || name.contains("吐丝蜘蛛")
        || name.contains("bugbagmaggot") || name.contains("虫袋蛆")
        || name.contains("bombspider") || name.contains("bomb spider") || name.contains("炸弹蜘蛛")
        || name == "hugger"
        || name == "guard" || (name.contains("guard") && !name.contains("town"))
        || name.contains("townarcher") || name.contains("town archer") || name.contains("城镇弓箭手")
        || name.contains("castlegate") || name.contains("castle gate") || name.contains("城门")
        || name.contains("digoutzombie") || name.contains("digout zombie") || name.contains("钻地僵尸")
        || name.contains("revivingzombie") || name.contains("reviving zombie") || name.contains("复活僵尸")
        || name == "jar1" || name.contains("坛子")
        || name == "armadillo" || name.contains("犰狳")
        || name.contains("gastoad") || name.contains("gas toad") || name.contains("毒气蟾蜍")
        || name.contains("stoningstatue") || name.contains("stoning statue") || name.contains("石化雕像")
        || name.contains("bonelord") || name.contains("bone lord") || name.contains("骨魔领主")
        || name.contains("catshaman") || name.contains("cat shaman") || name.contains("猫巫师")
        || name.contains("rootspider") || name.contains("root spider") || name.contains("根须蜘蛛")
        || name.contains("flamemage") || name.contains("flame mage") || name.contains("火焰法师")
        || name == "tornado" || name.contains("龙卷风")
        || name.contains("poisonhugger") || name.contains("毒抱怪")
        || name.contains("hornedmage") || name.contains("horned mage") || name.contains("角魔法师")
        || name.contains("witchdoctor") || name.contains("witch doctor") || name.contains("巫医")
        || name.contains("kingscorpion") || name.contains("king scorpion") || name.contains("蝎子王")
        || name.contains("furbolgwarrior") || name.contains("furbolg warrior") || name.contains("熊人战士")
        || name.contains("crazymanworm") || name.contains("crazy manworm") || name.contains("狂化人面虫")
        || name.contains("flamingwooma") || name.contains("flaming wooma") || name.contains("烈焰沃玛")
        || name.contains("iceguard") || name.contains("ice guard") || name.contains("冰守卫")
}
