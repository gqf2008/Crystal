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
}
