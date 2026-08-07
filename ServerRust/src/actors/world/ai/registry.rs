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
    // 环境物体（静态/被动）不走 Boss behavior，优先判断
    if is_static_object(&name) || is_passive_object(&name) {
        return Box::new(DefaultBehavior::new());
    }
    // 召唤物分身（名称带后缀）不匹配 Boss behavior，避免无限召唤
    if name.contains("分身") || name.contains(" clone") || name.contains("summoned ") {
        return Box::new(DefaultBehavior::new());
    }
    // 去除可能的空格/后缀，匹配 Boss 名称
    // 注意：EvilMirBody 是龙系统静态身体（CanMove/CanAttack=false），不是 EvilMir Boss 本体
    if (name.contains("evilmir") || name.contains("evil mir") || name.contains("邪恶巨龙"))
        && !name.contains("evilmirbody")
    {
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
    if name.contains("cavemaggot") || name.contains("cave maggot") || name.contains("洞穴蛆") {
        return Box::new(bosses::cave_maggot::CaveMaggotBehavior::new());
    }
    if name.contains("bonespearman") || name.contains("bone spearman") || name.contains("骷髅枪兵") {
        return Box::new(bosses::bone_spearman::BoneSpearmanBehavior::new());
    }
    if name.contains("sandworm") || name.contains("sand worm") || name.contains("沙虫") {
        return Box::new(bosses::sand_worm::SandWormBehavior::new());
    }
    if name.contains("shamanzombie") || name.contains("shaman zombie") || name.contains("巫师僵尸") || name.contains("萨满僵尸") {
        return Box::new(bosses::shaman_zombie::ShamanZombieBehavior::new());
    }
    if name.contains("straycat") || name.contains("stray cat") || name.contains("流浪猫") {
        return Box::new(bosses::stray_cat::StrayCatBehavior::new());
    }
    if name.contains("darkbeast") || name.contains("dark beast") || name.contains("暗兽") {
        return Box::new(bosses::dark_beast::DarkBeastBehavior::new());
    }
    // 注意：SnowWolf 判定放在 SnowWolfKing 之后（先匹配到 King 会提前 return）
    if name.contains("snowwolf") || name.contains("snow wolf") || name.contains("雪狼") {
        return Box::new(bosses::snow_wolf::SnowWolfBehavior::new());
    }
    if name.contains("mudzombie") || name.contains("mud zombie") || name.contains("泥僵尸") || name.contains("泥沼僵尸") {
        return Box::new(bosses::mud_zombie::MudZombieBehavior::new());
    }
    if name.contains("incarnatedghoul") || name.contains("incarnated ghoul") || name.contains("附身尸") {
        return Box::new(bosses::incarnated_ghoul::IncarnatedGhoulBehavior::new());
    }
    if name.contains("incarnatedzt") || name.contains("incarnated zt") || name.contains("附身祖玛") {
        return Box::new(bosses::incarnated_zt::IncarnatedZTBehavior::new());
    }
    if name.contains("mantis") || name.contains("螳螂") {
        return Box::new(bosses::mantis::MantisBehavior::new());
    }
    if name.contains("sandsnail") || name.contains("sand snail") || name.contains("沙蜗牛") {
        return Box::new(bosses::sand_snail::SandSnailBehavior::new());
    }
    if name.contains("scalybeast") || name.contains("scaly beast") || name.contains("鳞兽") {
        return Box::new(bosses::scaly_beast::ScalyBeastBehavior::new());
    }
    if name.contains("lightturtle") || name.contains("light turtle") || name.contains("光龟") {
        return Box::new(bosses::light_turtle::LightTurtleBehavior::new());
    }
    if name.contains("hellcannibal") || name.contains("hell cannibal") || name.contains("地狱食人花") {
        return Box::new(bosses::hell_cannibal::HellCannibalBehavior::new());
    }
    if name == "nadz" || name.contains("纳兹") {
        return Box::new(bosses::nadz::NadzBehavior::new());
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
    if name.contains("stonetrap") || name.contains("stone trap") || name.contains("石阵") {
        return Box::new(bosses::stone_trap::StoneTrapBehavior::new());
    }
    if name.contains("redmoonevil") || name.contains("red moon evil") || name.contains("红月恶魔") {
        return Box::new(bosses::red_moon_evil::RedMoonEvilBehavior::new());
    }
    if name.contains("toxicghoul") || name.contains("toxic ghoul") || name.contains("毒尸") {
        return Box::new(bosses::toxic_ghoul::ToxicGhoulBehavior::new());
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
    // ===== 第三批：25 个独特机制怪物（含 guard 变体，须在通用 guard 捕获前匹配）=====
    if name.contains("waterdragon") || name.contains("water dragon") || name.contains("水龙") {
        return Box::new(bosses::water_dragon::WaterDragonBehavior::new());
    }
    if name.contains("rightguard") || name.contains("right guard") || name.contains("右护卫") {
        return Box::new(bosses::right_guard::RightGuardBehavior::new());
    }
    if name.contains("minotaurking") || name.contains("minotaur king") || name.contains("牛头王") {
        return Box::new(bosses::minotaur_king::MinotaurKingBehavior::new());
    }
    if name.contains("trollking") || name.contains("troll king") || name.contains("巨魔王") {
        return Box::new(bosses::troll_king::TrollKingBehavior::new());
    }
    if name == "darkdevil" || name.contains("dark devil") || name.contains("暗黑恶魔") {
        return Box::new(bosses::dark_devil::DarkDevilBehavior::new());
    }
    if name.contains("darkdevourer") || name.contains("dark devourer") || name.contains("暗黑吞噬者") {
        return Box::new(bosses::dark_devourer::DarkDevourerBehavior::new());
    }
    if name.contains("hellknight") || name.contains("hell knight") || name.contains("地狱骑士") {
        return Box::new(bosses::hell_knight::HellKnightBehavior::new());
    }
    if name.contains("holydeva") || name.contains("holy deva") || name.contains("圣兽") {
        return Box::new(bosses::holy_deva::HolyDevaBehavior::new());
    }
    if name == "shinsu" || name.contains("shinsu") || name.contains("神兽") {
        return Box::new(bosses::shinsu::ShinsuBehavior::new());
    }
    if name.contains("harvestmonster") || name.contains("harvest monster") || name.contains("可采集") {
        return Box::new(bosses::harvest_monster::HarvestMonsterBehavior::new());
    }
    if name.contains("hoodedsummoner") || name.contains("hooded summoner") || name.contains("兜帽召唤师") {
        return Box::new(bosses::hooded_summoner::HoodedSummonerBehavior::new());
    }
    if name.contains("hornedsorceror") || name.contains("horned sorceror") || name.contains("horned sorcerer") || name.contains("角法师") {
        return Box::new(bosses::horned_sorceror::HornedSorcerorBehavior::new());
    }
    if name.contains("earthgolem") || name.contains("earth golem") || name.contains("地魔像") {
        return Box::new(bosses::earth_golem::EarthGolemBehavior::new());
    }
    if name.contains("vampirespider") || name.contains("vampire spider") || name.contains("吸血蜘蛛") {
        return Box::new(bosses::vampire_spider::VampireSpiderBehavior::new());
    }
    if name.contains("venomspider") || name.contains("venom spider") || name.contains("毒液蜘蛛") {
        return Box::new(bosses::venom_spider::VenomSpiderBehavior::new());
    }
    if name.contains("thunderelement") || name.contains("thunder element") || name.contains("雷元素") {
        return Box::new(bosses::thunder_element::ThunderElementBehavior::new());
    }
    if name == "kirin" || name.contains("kirin") || name.contains("麒麟") {
        return Box::new(bosses::kirin::KirinBehavior::new());
    }
    if name.contains("trollbomber") || name.contains("troll bomber") || name.contains("巨魔投弹手") {
        return Box::new(bosses::troll_bomber::TrollBomberBehavior::new());
    }
    if name.contains("plaguecrab") || name.contains("plague crab") || name.contains("瘟疫螃蟹") {
        return Box::new(bosses::plague_crab::PlagueCrabBehavior::new());
    }
    if name.contains("humanwizard") || name.contains("human wizard") || name.contains("人形法师") {
        return Box::new(bosses::human_wizard::HumanWizardBehavior::new());
    }
    if name.contains("humanassassin") || name.contains("human assassin") || name.contains("人形刺客") {
        return Box::new(bosses::human_assassin::HumanAssassinBehavior::new());
    }
    if name.contains("kingguard") || name.contains("king guard") || name.contains("王城护卫") {
        return Box::new(bosses::king_guard::KingGuardBehavior::new());
    }
    if name.contains("rhinopriest") || name.contains("rhino priest") || name.contains("犀牛祭司") {
        return Box::new(bosses::rhino_priest::RhinoPriestBehavior::new());
    }
    if name.contains("icephantom") || name.contains("ice phantom") || name.contains("冰幻影") {
        return Box::new(bosses::ice_phantom::IcePhantomBehavior::new());
    }
    if name.contains("ancientbringer") || name.contains("ancient bringer") || name.contains("远古召唤者") {
        return Box::new(bosses::ancient_bringer::AncientBringerBehavior::new());
    }
    if name == "guard" || (name.starts_with("guard") && !name.contains("town") && !name.contains("guardian") && !name.contains("vanguard") && !name.contains("bodyguard")) {
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
    // ===== 第二批：17 个独特机制怪物 =====
    if name.contains("hornedwarrior") || name.contains("horned warrior") || name.contains("角魔战士") {
        return Box::new(bosses::horned_warrior::HornedWarriorBehavior::new());
    }
    if name.contains("manectricking") || name.contains("manectric king") || name.contains("雷电王") {
        return Box::new(bosses::manectric_king::ManectricKingBehavior::new());
    }
    if name.contains("seedingsgeneral") || name.contains("seedings general") || name.contains("幼苗将军") {
        return Box::new(bosses::seedings_general::SeedingsGeneralBehavior::new());
    }
    if name.contains("tucsongeneral") || name.contains("tucson general") || name.contains("图森将军") {
        return Box::new(bosses::tucson_general::TucsonGeneralBehavior::new());
    }
    if name.contains("whitemammoth") || name.contains("white mammoth") || name.contains("白色猛犸") {
        return Box::new(bosses::white_mammoth::WhiteMammothBehavior::new());
    }
    if name.contains("hornedarcher") || name.contains("horned archer") || name.contains("角魔弓手") {
        return Box::new(bosses::horned_archer::HornedArcherBehavior::new());
    }
    if name == "khazard" || name.contains("khazard") || name.contains("卡扎德") {
        return Box::new(bosses::khazard::KhazardBehavior::new());
    }
    if name.contains("kinghydrax") || name.contains("king hydrax") || name.contains("海德拉王") {
        return Box::new(bosses::king_hydrax::KingHydraxBehavior::new());
    }
    if name.contains("crystalspider") || name.contains("crystal spider") || name.contains("水晶蜘蛛") {
        return Box::new(bosses::crystal_spider::CrystalSpiderBehavior::new());
    }
    if name.contains("elementguard") || name.contains("element guard") || name.contains("元素守卫") {
        return Box::new(bosses::element_guard::ElementGuardBehavior::new());
    }
    if name.contains("wingedtigerlord") || name.contains("winged tiger lord") || name.contains("飞虎王") {
        return Box::new(bosses::winged_tiger_lord::WingedTigerLordBehavior::new());
    }
    if name.contains("greatfoxspirit") || name.contains("great fox spirit") || name.contains("巨狐之灵") {
        return Box::new(bosses::great_fox_spirit::GreatFoxSpiritBehavior::new());
    }
    if name.contains("stonegolem") || name.contains("stone golem") || name.contains("石头傀儡") {
        return Box::new(bosses::stone_golem::StoneGolemBehavior::new());
    }
    if name.contains("tucsonmage") || name.contains("tucson mage") || name.contains("图森法师") {
        return Box::new(bosses::tucson_mage::TucsonMageBehavior::new());
    }
    if name.contains("omamage") || name.contains("oma mage") || name.contains("奥玛法师") {
        return Box::new(bosses::oma_mage::OmaMageBehavior::new());
    }
    if name.contains("flamingmutant") || name.contains("flaming mutant") || name.contains("燃烧突变体") {
        return Box::new(bosses::flaming_mutant::FlamingMutantBehavior::new());
    }
    if name.contains("darkcaptain") || name.contains("dark captain") || name.contains("黑暗队长") {
        return Box::new(bosses::dark_captain::DarkCaptainBehavior::new());
    }
    Box::new(DefaultBehavior::new())
}

/// 判断是否为已注册的 Boss（用于 tick_monsters 分发）
pub fn is_registered_boss(monster_name: &str) -> bool {
    let name = monster_name.to_lowercase();
    // 环境物体优先排除（静态/被动不走 Boss 分发）
    if is_static_object(&name) || is_passive_object(&name) {
        return false;
    }
    ((name.contains("evilmir") || name.contains("evil mir") || name.contains("邪恶巨龙"))
        && !name.contains("evilmirbody"))
        || name.contains("hornedcommander") || name.contains("horned commander") || name.contains("角魔统帅")
        || name.contains("helllord") || name.contains("hell lord") || name.contains("地狱领主")
        || name.contains("treequeen") || name.contains("tree queen") || name.contains("树后")
        || name.contains("yimoogi") || name.contains("异魔蛇") || name.contains("蛇母")
        || name.contains("darkomaking") || name.contains("dark oma king") || name.contains("暗黑奥玛") || name.contains("奥玛之王")
        || name.contains("generalmeowmeow") || name.contains("general meow meow") || name.contains("喵喵将军")
        || name.contains("zumataurus") || name.contains("zuma taurus") || name.contains("祖玛教主") || name.contains("祖玛金牛")
        || name.contains("evilcentipede") || name.contains("evil centipede") || name.contains("地蜈蚣") || name.contains("触角恶魔")
        || name.contains("cannibalplant") || name.contains("cannibal plant") || name.contains("食人花")
        || name.contains("cavemaggot") || name.contains("cave maggot") || name.contains("洞穴蛆")
        || name.contains("bonespearman") || name.contains("bone spearman") || name.contains("骷髅枪兵")
        || name.contains("sandworm") || name.contains("sand worm") || name.contains("沙虫")
        || name.contains("shamanzombie") || name.contains("shaman zombie") || name.contains("巫师僵尸") || name.contains("萨满僵尸")
        || name.contains("straycat") || name.contains("stray cat") || name.contains("流浪猫")
        || name.contains("darkbeast") || name.contains("dark beast") || name.contains("暗兽")
        || name.contains("snowwolf") || name.contains("snow wolf") || name.contains("雪狼")
        || name.contains("mudzombie") || name.contains("mud zombie") || name.contains("泥僵尸") || name.contains("泥沼僵尸")
        || name.contains("incarnatedghoul") || name.contains("incarnated ghoul") || name.contains("附身尸")
        || name.contains("incarnatedzt") || name.contains("incarnated zt") || name.contains("附身祖玛")
        || name.contains("mantis") || name.contains("螳螂")
        || name.contains("sandsnail") || name.contains("sand snail") || name.contains("沙蜗牛")
        || name.contains("scalybeast") || name.contains("scaly beast") || name.contains("鳞兽")
        || name.contains("lightturtle") || name.contains("light turtle") || name.contains("光龟")
        || name.contains("hellcannibal") || name.contains("hell cannibal") || name.contains("地狱食人花")
        || name == "nadz" || name.contains("纳兹")
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
        || name.contains("stonetrap") || name.contains("stone trap") || name.contains("石阵")
        || name.contains("redmoonevil") || name.contains("red moon evil") || name.contains("红月恶魔")
        || name.contains("toxicghoul") || name.contains("toxic ghoul") || name.contains("毒尸")
        || name.contains("bugbagmaggot") || name.contains("虫袋蛆")
        || name.contains("bombspider") || name.contains("bomb spider") || name.contains("炸弹蜘蛛")
        || name == "hugger"
        || name == "guard" || (name.starts_with("guard") && !name.contains("town") && !name.contains("guardian") && !name.contains("vanguard") && !name.contains("bodyguard"))
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
        // 第二批：17 个独特机制怪物
        || name.contains("hornedwarrior") || name.contains("horned warrior") || name.contains("角魔战士")
        || name.contains("manectricking") || name.contains("manectric king") || name.contains("雷电王")
        || name.contains("seedingsgeneral") || name.contains("seedings general") || name.contains("幼苗将军")
        || name.contains("tucsongeneral") || name.contains("tucson general") || name.contains("图森将军")
        || name.contains("whitemammoth") || name.contains("white mammoth") || name.contains("白色猛犸")
        || name.contains("hornedarcher") || name.contains("horned archer") || name.contains("角魔弓手")
        || name == "khazard" || name.contains("卡扎德")
        || name.contains("kinghydrax") || name.contains("king hydrax") || name.contains("海德拉王")
        || name.contains("crystalspider") || name.contains("crystal spider") || name.contains("水晶蜘蛛")
        || name.contains("elementguard") || name.contains("element guard") || name.contains("元素守卫")
        || name.contains("wingedtigerlord") || name.contains("winged tiger lord") || name.contains("飞虎王")
        || name.contains("greatfoxspirit") || name.contains("great fox spirit") || name.contains("巨狐之灵")
        || name.contains("stonegolem") || name.contains("stone golem") || name.contains("石头傀儡")
        || name.contains("tucsonmage") || name.contains("tucson mage") || name.contains("图森法师")
        || name.contains("omamage") || name.contains("oma mage") || name.contains("奥玛法师")
        || name.contains("flamingmutant") || name.contains("flaming mutant") || name.contains("燃烧突变体")
        || name.contains("darkcaptain") || name.contains("dark captain") || name.contains("黑暗队长")
        // 第三批：25 个独特机制怪物
        || name.contains("waterdragon") || name.contains("water dragon") || name.contains("水龙")
        || name.contains("rightguard") || name.contains("right guard") || name.contains("右护卫")
        || name.contains("minotaurking") || name.contains("minotaur king") || name.contains("牛头王")
        || name.contains("trollking") || name.contains("troll king") || name.contains("巨魔王")
        || name == "darkdevil" || name.contains("dark devil") || name.contains("暗黑恶魔")
        || name.contains("darkdevourer") || name.contains("dark devourer") || name.contains("暗黑吞噬者")
        || name.contains("hellknight") || name.contains("hell knight") || name.contains("地狱骑士")
        || name.contains("holydeva") || name.contains("holy deva") || name.contains("圣兽")
        || name == "shinsu" || name.contains("神兽")
        || name.contains("harvestmonster") || name.contains("harvest monster") || name.contains("可采集")
        || name.contains("hoodedsummoner") || name.contains("hooded summoner") || name.contains("兜帽召唤师")
        || name.contains("hornedsorceror") || name.contains("horned sorceror") || name.contains("horned sorcerer") || name.contains("角法师")
        || name.contains("earthgolem") || name.contains("earth golem") || name.contains("地魔像")
        || name.contains("vampirespider") || name.contains("vampire spider") || name.contains("吸血蜘蛛")
        || name.contains("venomspider") || name.contains("venom spider") || name.contains("毒液蜘蛛")
        || name.contains("thunderelement") || name.contains("thunder element") || name.contains("雷元素")
        || name == "kirin" || name.contains("麒麟")
        || name.contains("trollbomber") || name.contains("troll bomber") || name.contains("巨魔投弹手")
        || name.contains("plaguecrab") || name.contains("plague crab") || name.contains("瘟疫螃蟹")
        || name.contains("humanwizard") || name.contains("human wizard") || name.contains("人形法师")
        || name.contains("humanassassin") || name.contains("human assassin") || name.contains("人形刺客")
        || name.contains("kingguard") || name.contains("king guard") || name.contains("王城护卫")
        || name.contains("rhinopriest") || name.contains("rhino priest") || name.contains("犀牛祭司")
        || name.contains("icephantom") || name.contains("ice phantom") || name.contains("冰幻影")
        || name.contains("ancientbringer") || name.contains("ancient bringer") || name.contains("远古召唤者")
}

/// 判断是否为「静态环境物体」（CanMove=false 且 CanAttack=false，对齐 C# 的 Tree/Wall/CaveStatue 等）
///
/// 这些物体不可移动、不可攻击：tick_monsters 中应跳过全部 AI（攻击 + 移动），只保留受击/死亡判定。
/// 对应 C# 实现：MonsterObject 子类覆写 `CanMove { false }` + `CanAttack { false }` + Walk(){return false}。
///
/// 注意：`name` 传入前应已 lowercase（内部亦会 to_lowercase 以容错）。
pub fn is_static_object(monster_name: &str) -> bool {
    let name = monster_name.to_lowercase();
    // C# 中 CanMove=false && CanAttack=false 的物体（完全不可动、不攻击）
    name == "tree"
        || name == "wall"
        || name == "cavestatue" || name.contains("cave statue") || name.contains("洞穴雕像")
        || name == "boulderspirit" || name.contains("boulder spirit") || name.contains("巨石之灵")
        || name == "evilmirbody" || name.contains("evil mir body") || name.contains("邪龙身躯")
        || name == "icepillar" || name.contains("ice pillar") || name.contains("冰柱")
        || name == "woodbox" || name.contains("wood box") || name.contains("木箱")
        || name == "cargobox" || name.contains("cargo box") || name.contains("货箱")
        || name == "restlessjar" || name.contains("restless jar") || name.contains("躁动之坛")
        || name == "purplefaeflower" || name.contains("purple fae flower") || name.contains("紫妖花")
        || name == "blockingobject" || name.contains("blocking object") || name.contains("阻挡物")
}

/// 判断是否为「被动环境物体」（可移动但不主动攻击，对齐 C# 的 Deer/Doe/Football 等）
///
/// 这些物体会移动/漫游，但不会主动追击或攻击玩家。
/// tick_monsters 中应跳过攻击与追击逻辑（target 查找仍可保留，但不应产生伤害）。
///
/// 注意：`name` 传入前应已 lowercase（内部亦会 to_lowercase 以容错）。
pub fn is_passive_object(monster_name: &str) -> bool {
    let name = monster_name.to_lowercase();
    // 可移动但不主动攻击的动物/物体
    name == "deer" || name.contains("鹿")
        || name == "doe" || name.contains("母鹿")
        || name == "football" || name.contains("足球")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archer_guard_not_boss() {
        // #471 修复：ArcherGuard 等普通守卫不应被当作 Boss（否则跳过 tick 死亡判定 → 无敌）
        assert!(!is_registered_boss("ArcherGuard"));
        assert!(!is_registered_boss("archerguard"));
    }

    #[test]
    fn plain_guard_still_boss_behavior() {
        assert!(is_registered_boss("Guard"));
        assert!(is_registered_boss("Guard2"));
    }
}
