//! 技能页（角色窗技能页）行 Hint 文案——C# `MainDialogs.cs:3427-3751`
//! `CharacterDialog.RefreshInterface` 里 `SkillButton.Hint = GetLocalization(*SkillDescription, ...)` 的等价表。
//!
//! 由 `Client/Localization/Chinese.json` 的 `*SkillDescription` 键与 C# 的 `case Spell.X:` 配对生成
//! （顺序即 C# 顺序），**逐字保留**原版换行与占位符：
//! `{0}` = 当前技能等级，`{1}` = 该等级数值（Level1/2/3，满级为 0），`{2}` = 基础法力消耗。
//! C# 各 case 传参个数不同（无 `{2}` 的模板只传 2 个），本实现统一按模板替换——未出现的占位符不参与。

use mir2_shared::enums::Spell;

/// 技能描述模板（`None` = C# 该 Spell 无 case，不弹 Hint）
pub fn skill_hint_template(spell: Spell) -> Option<&'static str> {
    let t = match spell {
        Spell::Fencing => "基本剑术\n\n被动技能\n\n命中率会根据练习等级提高。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::Slaying => "攻杀剑术\n\n被动技能\n\n击中的准确度和破坏力会随着熟练等级的提升而增加。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::Thrusting => "刺杀剑术\n\n切换技能\n\n延长攻击距离，破坏力会随着熟练等级的提升而增加。\n当前技能等级 {0}\n下一等级 {1}",
        Spell::Rage => "狂暴\n\n增益技能\n法力消耗 {2}\n\n激发内力在一段时间内提升力量。攻击力和持续时间取决于技能等级。技能使用后需等待才能再次施展。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::ProtectionField => "护身气幕\n\n增益技能\n法力消耗 {2}\n\n凝聚内力覆盖全身，提升抵御敌人攻击的能力。防御力和持续时间取决于技能等级。技能使用后需等待才能再次施展。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::HalfMoon => "半月弯刀\n\n切换技能\n每次攻击法力消耗: {2}\n\n以高速挥动武器产生冲击波，对施法者周围半圆范围敌人造成伤害。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::FlamingSword => "烈火剑法\n\n主动技能\n法力消耗: {2}\n\n在下一次攻击中召唤火之精灵，对目标造成毁灭性打击。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::ShoulderDash => "野蛮冲撞\n\n主动技能\n法力消耗: {2}\n\n战士蓄力用肩膀撞击目标，将其推开，若目标撞上障碍物则造成额外伤害。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::CrossHalfMoon => "狂风斩\n\n切换技能\n每次攻击法力消耗: {2}\n\n战士施展双重半月斩，击伤紧邻的所有怪物。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::TwinDrakeBlade => "双龙斩\n\n主动技能\n法力消耗 {2}\n\n连续施展多重强力攻击，有低概率短暂眩晕目标。被眩晕的怪物额外受到50%伤害。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::Entrapment => "捕蝇剑\n\n主动技能\n法力消耗: {2}\n\n使怪物麻痹并将其拉向施法者。\n当前技能等级 {0}\n下一等级 {1}",
        Spell::LionRoar => "狮子吼\n\n主动技能\n法力消耗: {2}\n\n麻痹施法者周围的敌人，持续时间随技能等级提升。\n当前技能等级 {0}\n下一等级 {1}",
        Spell::CounterAttack => "反击\n\n增益技能\n法力消耗 {2}\n\n短时间提升物防和魔防，并有几率格挡攻击并进行反击。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::ImmortalSkin => "不灭护体\n\n增益技能\n法力消耗 {2}\n\n提升防御力，减少受到的伤害。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::Fury => "嗜血\n\n增益技能\n法力消耗 {2}\n\n在一定时间内提升战士的命中率。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::SlashingBurst => "日闪\n\n主动技能\n法力消耗: {2}\n\n允许战士跳过物体或怪物跨越1格距离。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::BladeAvalanche => "空破闪\n\n主动技能\n法力消耗 {2}\n\n向施法者前方三方向投掷刀刃，形成致命的金属风暴。\n\n当前技能等级 {0}\n下一等级 {1}",
        Spell::FireBall => "火球术\n\n瞬发技能\n法力消耗 {2}\n\n汇聚火元素形成火球，投掷给怪物造成伤害。\n\n当前技能等级 {0}\n下一等级 {1}ThunderboltSkillDescription",
        Spell::ThunderBolt => "雷电术\n\nInstant Casting\nMana Cost {2}\n\nStrikes the foe with a lightning bolt \ninflicting high damage.\n\nCurrent Skill Level {0}\nNext Level {1}",
        Spell::GreatFireBall => "大火球\n\n瞬发技能\n魔法消耗 {2}\n\n火球术的进阶版，大火球术对目标造成更高的伤害。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Repulsion => "抗拒火环\n\n瞬发技能\n魔法消耗 {2}\n\n借助火焰之力，将周围的敌人推开。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::HellFire => "地狱火\n\n瞬发技能\n魔法消耗 {2}\n\n释放一道火焰攻击，击中前方的怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Lightning => "疾光电影\n\n瞬发技能\n魔法消耗 {2}\n\n释放一道闪电，攻击前方的怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::ElectricShock => "诱惑之光\n\n瞬发技能\n魔法消耗 {2}\n\n释放强力电击波，击中怪物使其无法移动，或让其混乱为你作战。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Teleport => "瞬息移动\n\n瞬发技能\n魔法消耗 {2}\n\n瞬间传送到随机位置。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FireWall => "火墙\n\n瞬发技能\n魔法消耗 {2}\n\n在指定位置生成一堵火墙，攻击经过的怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FireBang => "爆裂火焰\n\n瞬发技能\n魔法消耗 {2}\n\n在指定位置引发爆燃，烧灼范围内的所有怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::ThunderStorm => "地狱雷光\n\n瞬发技能\n魔法消耗 {2}\n\n在施法者周围制造雷暴，攻击范围内的所有亡灵敌人。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::MagicShield => "魔法护盾\n\n瞬发技能\n魔法消耗 {2}\n\n为施法者创造一个吸收伤害的防护屏障。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::TurnUndead => "圣言术\n\n瞬发技能\n魔法消耗 {2}\n\n有几率瞬间击杀符合等级要求的亡灵目标。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::IceStorm => "冰咆哮\n\n瞬发技能\n魔法消耗 {2}\n\n在指定区域制造暴风雪，攻击范围内的怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FlameDisruptor => "灭天火\n\n瞬发技能\n魔法消耗 {2}\n\n将地底的火焰引至地表，攻击怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FrostCrunch => "寒冰掌\n\n瞬发技能\n魔法消耗 {2}\n\n冻结怪物周围空气中的元素，使其减速。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Mirroring => "分身术\n\n瞬间施法\n法力消耗 {2}\n\n创造一个自己的镜像，与自己一同攻击\n怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FlameField => "火龙气焰\n\n瞬间施法\n法力消耗 {2}\n\n释放强大的火焰法术，对周围敌人造成伤害。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Vampirism => "噬血术\n\n瞬间施法\n法力消耗 {2}\n\n消耗法力夺取怪物生命值来\n提升自身生命值。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Blizzard => "天霜冰环\n\n引导施法\n法力消耗 {2}\n\n集中内力并扩散至全身各处，\n增强对敌人的防护。防御力和持续时间\n取决于技能等级。技能使用后需等待一段时间才能再次施展。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::MeteorStrike => "流星打击\n\n引导施法\n法力消耗 {2}\n\n从天而降的火焰砸击5x5范围内的全部怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::IceThrust => "冰刺术\n\n瞬间施法\n法力消耗 {2}\n\n制造冰柱攻击怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::MagicBooster => "魔力强化\n\n持续效果\n法力消耗 {2}\n\n提升魔法伤害，但额外消耗法力。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FastMove => "快速移动\n\n引导施法\n法力消耗 {2}\n\n利用蓄力技能提升移动速度。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::StormEscape => "风暴逃脱\n\n引导施法\n法力消耗 {2}\n\n麻痹附近敌人并瞬移至指定位置。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Blink => "闪现\n\n瞬间施法\n法力消耗 {2}\n\n随机瞬移至附近位置。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::SpiritSword => "精神力战法\n\n提升近战命中目标的概率。\n被动技能\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Healing => "治愈术\n\n瞬间施法\n法力消耗 {2}\n\n治疗单一目标，\n持续恢复生命值。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Poisoning => "施毒术\n\n瞬间施法\n法力消耗 {2}\n\n需要物品：剧毒粉末\n\n向怪物投掷毒药以削弱其能力。\n使用绿色毒药削弱生命值。\n使用红色毒药削弱防御力。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::SoulFireBall => "灵魂火符\n\n瞬间施法\n法力消耗 {2}\n\n需要物品：护身符\n\n将力量注入符咒并投向怪物，符咒将爆裂成火焰。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::SoulShield => "灵魂护盾\n\n瞬间施法\n法力消耗 {2}\n\n需要物品：护身符\n\n为施法者和队友施加祝福，强化其魔法防御力。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::BlessedArmour => "祝福铠甲\n\n瞬间施法\n法力消耗 {2}\n\n需要物品：护身符\n\n为施法者和队友施加祝福，提升其防御力。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::TrapHexagon => "困魔咒\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n用这种魔法力量困住怪物，\n使其无法移动。受到外部伤害后，\n怪物将恢复移动能力。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::SummonSkeleton => "召唤骷髅\n\n瞬发技能\n法力消耗 {2}\n\n召唤一个强大的范围攻击骷髅，\n它将与你并肩作战。\n\n需要物品：护身符。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::Hiding => "隐身术\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n在短时间内怪物无法发现你，\n但一旦你开始移动，\n怪物将会注意到你。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::MassHiding => "群体隐身术\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n在短时间内怪物无法发现你和你的队友，\n但一旦你或队友开始移动，\n怪物将会注意到你们。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::Revelation => "心灵启示\n\n瞬发技能\n法力消耗 {2}\n\n能够读取其他人的生命值。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::MassHealing => "群体治疗术\n\n瞬发技能\n法力消耗 {2}\n\n用法力笼罩指定区域内的所有受伤玩家，\n为他们恢复生命值。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::SummonShinsu => "召唤神兽\n\n瞬发技能\n法力消耗 {2}\n\n召唤一只神兽，与您并肩作战。\n需要物品：护身符。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::UltimateEnhancer => "终极强化\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n吸收周围能量以提升自身属性。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::EnergyRepulsor => "气功波\n\n瞬发技能\n法力消耗 {2}\n\n聚集能量发动强力冲击，\n将周围怪物击退。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::Purification => "净化术\n\n瞬发技能\n法力消耗 {2}\n\n使用此技能帮助他人解除中毒\n和麻痹状态。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::SummonHolyDeva => "召唤圣灵\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n召唤一位圣灵，它将释放强力雷电\n攻击怪物。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::Curse => "诅咒术\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符 + 毒药\n\n降低目标的攻击速度、物理攻击、\n魔法攻击与道术攻击力。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::Hallucination => "迷魂术\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n让怪物陷入幻觉，并攻击\n沿途遇到的任何目标。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::Reincarnation => "复活术\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n复活一名已死亡的玩家。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::PoisonCloud => "毒雾\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：绿毒\n\n投掷护身符，在目标区域生成\n强力的毒云。\n\n当前技能等级 {0}\n下一级等级 {1}",
        Spell::EnergyShield => "能量护盾\n\n瞬发技能\n法力消耗 {2}\n\n需要物品：护身符\n\n可施加于自己或友方目标。\n反弹所受伤害的一定比例给攻击者。\n\n当前技能等级 {0}\n\n下一级等级 {1}",
        Spell::Plague => "瘟疫术\n\n瞬间施放\n魔法消耗 {2}\n\n需要物品：护符 + 毒药\n\n降低目标的魔法值并施加各种负面状态\n示例：眩晕、诅咒、中毒和减速。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::HealingCircle => "阴阳五行阵\n\n瞬间施放\n魔法消耗 {2}\n\n治疗范围内的友方目标，并对敌人造成法术伤害。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::PetEnhancer => "宠物强化\n\n瞬间施放\n魔法消耗 {2}\n\n增强宠物的防御和力量。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FatalSword => "绝命剑法\n\n被动技能\n\n增加对怪物的攻击伤害。\n并稍微提升准确度。\n被动技能\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::DoubleSlash => "风剑术\n\n切换技能\n每次攻击魔法消耗 {2}\n\n快速连续斩击怪物两次。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Haste => "急速\n\n增益技能\n魔法消耗 {2}\n\n提高攻击速度。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::FlashDash => "拔刀术\n\n主动技能\n魔法消耗 {2}\n\n以快速斩击攻击怪物并\n令其麻痹。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::HeavenlySword => "天剑\n\n主动技能\n魔法消耗 {2}\n\n攻击半径两步范围内的怪物。\n当前技能等级 {0}\n下一级 {1}",
        Spell::FireBurst => "烈风击\n\n主动技能\n魔法消耗 {2}\n\n将围绕你的敌人击退。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Trap => "捕缚术\n\n瞬间施放\n冷却时间 60 秒\n\n魔法消耗 {2}\n\n将怪物困住一段时间。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::MoonLight => "月光\n\n增益技能\n魔法消耗 {2}\n\n通过隐身来躲避怪物的视线\n使用此技能攻击怪物时造成更大伤害。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::MPEater => "吸气\n\n被动技能\n\n吸收怪物的魔法值以恢复施法者的魔法值。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::SwiftFeet => "迅雷疾足\n\n增益技能\n魔法消耗 {2}\n\n激活时提高奔跑速度。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::LightBody => "轻身术\n\n增益技能\n魔法消耗 {2}\n\n使用此技能使身体更轻盈并提高移动速度。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::PoisonSword => "猛毒剑气\n\n主动技能\n魔法消耗 {2}\n\n用剑斩击怪物使其中毒。中毒效果将随时间造成持续伤害。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::DarkBody => "暗影替身\n\n主动技能\n魔法消耗 {2}\n\n制造一个自己的幻象去攻击怪物，同时自己进入隐身状态。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::CrescentSlash => "月华乱舞\n\n魔法消耗 {2}\n\n爆发剑之力量攻击周围所有怪物。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Hemorrhage => "血凤击\n\n被动技能\n\n有几率造成致命伤害并附加流血效果。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::MoonMist => "月影迷雾\n\n增益技能\n魔法消耗 {2}\n\n可使自己从怪物视线中隐藏\n首次攻击将比平常更强。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Focus => "聚焦\n\n被动技能\n\n提升物理攻击命中率。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::StraightShot => "直射\n\n主动技能\n法力消耗 {2} \n\n为箭矢注入法力造成额外伤害。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::DoubleShot => "连射\n\n主动技能\n法力消耗 {2} \n\n快速连续射出两支箭。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::ExplosiveTrap => "爆炸陷阱\n\n陷阱技能\n法力消耗 {2} \n\n布置一排陷阱，敌人碰触后会爆炸。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::DelayedExplosion => "延爆箭\n\n主动技能\n法力消耗 {2} \n\n发射一支会在短暂延迟后爆炸的箭矢。\n使用元素造成额外伤害。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Meditation => "冥想\n\n被动技能\n\n攻击怪物时可收集元素。\n最多可获得 4 个元素。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::BackStep => "后撤步\n\n主动技能\n法力消耗 {2} \n\n迅速向后跃离危险。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::ElementalShot => "元素射击\n\n主动技能\n法力消耗 {2} \n\n高伤害魔法攻击。每个元素可增加伤害。\n若无元素则生成 2 个元素。\n弓手等级高于目标时可将其击退。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Concentration => "专注\n\n增益技能\n法力消耗 {2} \n\n技能激活时提高收集元素的几率。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::Stonetrap => "石陷阱\n\n陷阱技能\n法力消耗 {2}\n\n布置一个石陷阱。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::ElementalBarrier => "元素屏障\n\n增益技能\n法力消耗 {2}\n\n以元素屏障保护施法者。\n施法时元素越多，伤害减免越高。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::SummonVampire => "召唤吸血蜘蛛\n\n召唤技能\n法力消耗 {2}\n\n召唤一只吸血蜘蛛与你并肩作战。\n吸血蜘蛛会吸取敌人生命为施法者恢复。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::VampireShot => "吸血箭\n\n主动技能\n法力消耗 {2}\n\n射出一支吸血箭，吸取敌人生命为施法者恢复。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::SummonToad => "召唤蟾蜍\n\n召唤技能\n法力消耗 {2}\n\n召唤一只蟾蜍与你并肩作战。\n蟾蜍无法移动，若主人离开其视野范围则会爆炸。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::PoisonShot => "毒箭\n\n主动技能\n法力消耗 {2}\n\n射出一支毒箭使敌人中毒。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::CrippleShot => "致残射击\n\n主动技能\n法力消耗 {2}\n\n射出一支削弱箭令敌人减速。\n毒箭增益可使削弱射击产生3×3范围的毒性攻击。\n吸血射击增益可使削弱射击攻击两次并吸取生命。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::SummonSnakes => "召唤毒蛇\n\n召唤技能\n法力消耗 {2}\n\n召唤一个图腾，生成一群毒蛇。\n毒蛇会吸引周围所有怪物并攻击，\n有几率使目标麻痹。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::NapalmShot => "燃烧箭\n\n主动技能\n法力消耗 {2}\n\n射出一支箭，在目标周围5×5范围内爆炸。\n\n当前技能等级 {0}\n下一级 {1}",
        Spell::OneWithNature => "天人合一\n\n增益技能\n法力消耗 {2}\n\n召唤一个环绕施法者的元素之环，\n对5×5范围内的所有目标造成伤害。\n\n当前技能等级 {0}\n下一级等级 {1}",
        _ => return None,
    };
    Some(t)
}

/// 展开模板：`{0}`=等级、`{1}`=该等级数值、`{2}`=基础消耗（C# `SkillButton.Hint`）
pub fn skill_hint(spell: Spell, level: u8, level_value: u32, base_cost: u32) -> Option<String> {
    let t = skill_hint_template(spell)?;
    Some(
        t.replace("{0}", &level.to_string())
            .replace("{1}", &level_value.to_string())
            .replace("{2}", &base_cost.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2775：模板逐字来自 C#/Chinese.json（抽查，含换行与 {2} 占位符）
    #[test]
    fn templates_match_csharp_localization() {
        assert!(skill_hint_template(Spell::Fencing)
            .unwrap()
            .starts_with("基本剑术\n\n被动技能"));
        let rage = skill_hint_template(Spell::Rage).unwrap();
        assert!(rage.contains("法力消耗 {2}"), "带消耗的模板保留 {{2}}");
        assert!(rage.contains("下一等级 {1}"));
    }

    /// #2775：展开 = C# `GetLocalization(template, level, levelValue, baseCost)`
    #[test]
    fn skill_hint_substitutes_all_placeholders() {
        let t = skill_hint(Spell::Rage, 2, 7, 25).unwrap();
        assert!(t.contains("当前技能等级 2"), "{t}");
        assert!(t.contains("法力消耗 25"), "{t}");
        assert!(t.contains("下一等级 7"), "{t}");
        assert!(!t.contains("{0}") && !t.contains("{1}") && !t.contains("{2}"));
    }

    /// #2775：C# 无 case 的技能不弹提示（default 分支不赋值）
    #[test]
    fn spells_without_csharp_case_have_no_hint() {
        assert!(skill_hint_template(Spell::None).is_none());
        assert!(skill_hint(Spell::None, 0, 0, 0).is_none());
    }
}
