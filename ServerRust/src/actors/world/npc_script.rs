//! C# NPCScript 兼容引擎
//!
//! 解析 C# 格式的 NPC 脚本（`[@section]/#IF/#SAY/#ACT/GOTO`）并在 Rust 端执行。
//! 对应 C# Server.MirObjects.NPCObject / NPCScript / NPCSegment 的解析与执行逻辑。
//!
//! 文件格式（节选）：
//! ```text
//! [@main]
//! #IF
//! CHECKGOLD > 500
//! #ACT
//! MOVE 0 289 617
//! TAKEGOLD 500
//! #ELSEACT
//! GOTO @B1
//! #ELSESAY
//! 你没有足够的金币。
//!
//! [@B1]
//! #SAY
//! 你需要500金币才能传送。
//! <返回/@main>
//! ```
//!
//! 一个 Section 可包含多个 Segment。每个 Segment 由可选的 `#IF` 块、
//! `#SAY`/`#ACT`（条件成立时）以及 `#ELSESAY`/`#ELSEACT`（条件不成立时）组成。
//! 没有 `#IF` 的 Segment 视为无条件，checks 为空即恒为真。
//!
//! 注意：部分公开 API（如 `parse_buttons`、字段 `name`）预留给后续调用方
//! 与调试使用，故整个模块允许 dead_code。

#![allow(dead_code)]

use super::*;
use crate::actors::player::{
    AddExperience, AddGold, AddItemToInventory, ChangeClass, CheckQuestState, DeductGold,
    GetPlayerState, HasItem, HasItemSpace, RemoveItemByIndex, SetHair, SetPlayerPosition,
    SetPlayerState,
};
use mir2_shared::enums::MirClass;
use std::collections::HashMap;

// =============================================================================
// 数据结构
// =============================================================================

/// 解析后的整份脚本
#[derive(Debug, Clone, Default)]
pub struct ParsedScript {
    /// 保持原始大小写的 section 定义顺序
    pub sections: Vec<Section>,
    /// 小写 section 名 → 在 sections 中的索引
    pub index: HashMap<String, usize>,
}

/// 一个 `[@name]` 段
#[derive(Debug, Clone, Default)]
pub struct Section {
    /// 原始段名（不含 `@` 和方括号），如 `main`、`Main-1`、`B1`
    pub name: String,
    /// 段内多个连续片段（每个 #IF/#SAY/#ACT 组合算一个片段）
    pub segments: Vec<Segment>,
}

/// 一个执行片段：一组 check + 命中/未命中两套行为
#[derive(Debug, Clone, Default)]
pub struct Segment {
    /// 条件指令列表（AND 逻辑；空表示无条件恒真）
    pub checks: Vec<Check>,
    /// 条件成立时执行的动作
    pub actions: Vec<Action>,
    /// 条件成立时显示的文本（保留原始按钮标记 `<text/@target>`）
    pub say: Vec<String>,
    /// 条件不成立时执行的动作
    pub else_actions: Vec<Action>,
    /// 条件不成立时显示的文本
    pub else_say: Vec<String>,
}

/// 一条 Check 指令：`CHECKGOLD > 500` → { check_type: "CHECKGOLD", args: [">", "500"] }
#[derive(Debug, Clone, Default)]
pub struct Check {
    pub check_type: String,
    pub args: Vec<String>,
}

/// 一条 Action 指令：`MOVE 0 289 617` → { action_type: "MOVE", args: ["0","289","617"] }
#[derive(Debug, Clone, Default)]
pub struct Action {
    pub action_type: String,
    pub args: Vec<String>,
}

/// 执行某段后产出的对话页内容 + 控制流信号
#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    /// 发给客户端的对话行（已做变量替换，含按钮 `<text/@target>`）
    pub say_lines: Vec<String>,
    /// GOTO 跳转目标（section 名，不含 `@`）；首个命中的优先
    pub goto: Option<String>,
}

/// 执行过程中由 actions 产生的副作用控制信号
#[derive(Debug, Clone, Default)]
struct FlowControl {
    break_loop: bool,
    goto: Option<String>,
}

/// 当前正在写入的 SAY/ACT 容器模式（解析器内部状态）
#[derive(Clone, Copy, PartialEq)]
enum ParseMode {
    Idle,
    If,
    Say,
    Act,
    ElseSay,
    ElseAct,
}

// =============================================================================
// 解析器
// =============================================================================

impl ParsedScript {
    /// 把整份脚本文本解析为 ParsedScript。
    ///
    /// - 识别 `[@name]` 段头（大小写不敏感，归一化为小写索引）。
    /// - 识别 `#IF/#ACT/#SAY/#ELSEACT/#ELSESAY/#OR/#ELSE/#ENDIF` 等指令行。
    /// - `;` 开头的行视为注释，整行忽略。
    /// - 其余非空行按当前模式分发为 check/action/say。
    /// - `#INSERT [path] @section`：仅记录日志，不真正加载外部文件
    ///   （文件包含需要 IO，调用方在需要时单独处理）。
    pub fn parse(content: &str) -> ParsedScript {
        let mut script = ParsedScript::default();
        let mut cur_sec: Option<usize> = None;
        let mut seg = Segment::default();
        let mut mode = ParseMode::Idle;

        for raw in content.lines() {
            let line = raw.trim();

            // 空行：在 SAY/ELSESAY 模式下保留为视觉间隔，其余模式忽略
            if line.is_empty() {
                if matches!(mode, ParseMode::Say | ParseMode::ElseSay) {
                    push_say(&mut seg, mode, String::new());
                }
                continue;
            }

            // 注释：`;` 开头整行忽略
            if line.starts_with(';') {
                continue;
            }

            // 段头：[@xxx]
            if line.starts_with('[') && line.ends_with(']') && line.len() >= 3 {
                flush_segment(&mut seg, &mut cur_sec, &mut script);
                mode = ParseMode::Idle;
                let inner = &line[1..line.len() - 1];
                let name = inner.strip_prefix('@').map(|s| s.to_string()).unwrap_or_else(|| inner.to_string());
                let lower = name.to_lowercase();
                if let Some(&_idx) = script.index.get(&lower) {
                    warn!("NPC script: duplicate section [@{}] ignored", name);
                    cur_sec = script.index.get(&lower).copied();
                    continue;
                }
                let idx = script.sections.len();
                script.sections.push(Section { name, segments: Vec::new() });
                script.index.insert(lower, idx);
                cur_sec = Some(idx);
                continue;
            }

            // 指令行：#xxx
            if let Some(rest) = line.strip_prefix('#') {
                let upper = rest.trim().to_uppercase();
                let keyword = upper.split_whitespace().next().unwrap_or("").to_string();

                match keyword.as_str() {
                    "IF" => {
                        flush_segment(&mut seg, &mut cur_sec, &mut script);
                        mode = ParseMode::If;
                    }
                    "OR" => {
                        // OR 行当作额外 check 加入当前 segment（简化为 AND；真正 OR 罕见，TODO）
                        mode = ParseMode::If;
                    }
                    "ACT" => mode = ParseMode::Act,
                    "SAY" => mode = ParseMode::Say,
                    "ELSEACT" => mode = ParseMode::ElseAct,
                    "ELSESAY" => mode = ParseMode::ElseSay,
                    "ELSE" => mode = ParseMode::ElseAct,
                    "ENDIF" | "END" => {
                        flush_segment(&mut seg, &mut cur_sec, &mut script);
                        mode = ParseMode::Idle;
                    }
                    "INSERT" => {
                        debug!("NPC script #INSERT directive skipped: {}", line);
                    }
                    "LABEL" => {}
                    "BREAK" => {
                        push_action(&mut seg, mode, Action {
                            action_type: "BREAK".into(),
                            args: Vec::new(),
                        });
                    }
                    _ => {
                        let args = tokenize_args(rest);
                        push_action(&mut seg, mode, Action {
                            action_type: keyword.clone(),
                            args,
                        });
                        debug!("NPC script: unknown directive #{}", keyword);
                    }
                }
                continue;
            }

            // 普通行：按当前模式分发
            match mode {
                ParseMode::Idle => {
                    // Idle 模式下的裸文本：视为无条件 SAY（自动进入 Say 模式）
                    mode = ParseMode::Say;
                    push_say(&mut seg, mode, line.to_string());
                }
                ParseMode::Say | ParseMode::ElseSay => {
                    push_say(&mut seg, mode, line.to_string());
                }
                ParseMode::If => {
                    let tokens = tokenize_args(line);
                    if let Some(ct) = tokens.first().cloned() {
                        let args = tokens.into_iter().skip(1).collect();
                        seg.checks.push(Check { check_type: ct.to_uppercase(), args });
                    }
                }
                ParseMode::Act | ParseMode::ElseAct => {
                    let tokens = tokenize_args(line);
                    if let Some(at) = tokens.first().cloned() {
                        let args = tokens.into_iter().skip(1).collect();
                        push_action(&mut seg, mode, Action { action_type: at.to_uppercase(), args });
                    }
                }
            }
        }

        // 收尾
        flush_segment(&mut seg, &mut cur_sec, &mut script);
        script
    }

    /// 按名查找 section（大小写不敏感，可带或不带前导 `@`）
    pub fn find(&self, name: &str) -> Option<&Section> {
        let lower = name.to_lowercase();
        let lower = lower.strip_prefix('@').unwrap_or(&lower);
        self.index.get(lower).and_then(|&i| self.sections.get(i))
    }

    /// 入口段（@main/@MAIN）
    pub fn main_section(&self) -> Option<&Section> {
        self.find("main")
    }

    /// 执行指定 section，返回应显示的对话行与可选的 GOTO 目标。
    ///
    /// - 遍历段内每个 segment：先求值 checks（AND 逻辑）。
    /// - checks 全过 → 执行 actions + 收集 say；否则执行 else_actions + 收集 else_say。
    /// - actions 中的 BREAK 中断 segment 循环。
    /// - actions 中的 GOTO 设置返回目标并立即停止。
    /// - say 文本会做变量替换。
    pub async fn execute_section(
        &self,
        section: &Section,
        world: &mut WorldActor,
        session_id: u64,
        npc: &NpcState,
        custom_vars: &mut HashMap<String, String>,
    ) -> ExecutionResult {
        let mut result = ExecutionResult::default();

        for seg in &section.segments {
            let player_state = match current_player_state(world, session_id).await {
                Some(s) => s,
                None => return result, // 玩家已离线，停止
            };
            let passed = eval_checks(world, session_id, &player_state, &seg.checks).await;
            let (actions, say_src) = if passed {
                (seg.actions.clone(), seg.say.clone())
            } else {
                (seg.else_actions.clone(), seg.else_say.clone())
            };

            // 收集 say（变量替换，用 segment 开头获取的 player_state，避免每行 actor 往返）
            for raw in &say_src {
                let line = replace_vars(raw, &player_state, &npc.name, custom_vars);
                result.say_lines.push(line);
            }

            // 执行 actions
            let mut flow = FlowControl::default();
            for act in &actions {
                exec_action(world, session_id, npc, act, &mut flow, custom_vars).await;
                if flow.break_loop || flow.goto.is_some() {
                    break;
                }
            }

            if let Some(target) = flow.goto.take() {
                result.goto = Some(target);
                return result;
            }
            if flow.break_loop {
                break;
            }
        }

        result
    }
}

/// 把当前 segment 提交进所属 section（非空才提交）
fn flush_segment(seg: &mut Segment, cur_sec: &mut Option<usize>, script: &mut ParsedScript) {
    let nonempty = !seg.checks.is_empty()
        || !seg.actions.is_empty()
        || !seg.say.is_empty()
        || !seg.else_actions.is_empty()
        || !seg.else_say.is_empty();
    if nonempty {
        if let Some(idx) = *cur_sec {
            let taken = std::mem::take(seg);
            if let Some(s) = script.sections.get_mut(idx) {
                s.segments.push(taken);
            }
        }
    }
    *seg = Segment::default();
}

fn push_say(seg: &mut Segment, mode: ParseMode, line: String) {
    match mode {
        ParseMode::ElseSay => seg.else_say.push(line),
        _ => seg.say.push(line),
    }
}

fn push_action(seg: &mut Segment, mode: ParseMode, action: Action) {
    match mode {
        ParseMode::ElseAct => seg.else_actions.push(action),
        _ => seg.actions.push(action),
    }
}

/// 按空白切分 token，但保留双引号内的空白
/// 例：`LOCALMESSAGE "hello world" 0` → ["LOCALMESSAGE", "hello world", "0"]
fn tokenize_args(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    for ch in line.chars() {
        match ch {
            '"' => in_quote = !in_quote,
            c if c.is_whitespace() && !in_quote => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

// =============================================================================
// 变量替换
// =============================================================================

/// 系统变量与自定义变量替换。
///
/// 支持：`<$USERNAME>` `<$LEVEL>` `<$NPCNAME>` `<$HP>` `<$MAXHP>` `<$MP>` `<$MAXMP>`
/// `<$GAMEGOLD>` `<$CLASS>` `<$PKPOINT>` `<$GENDER>`。
/// 自定义变量 `%A0`/`%B0` 从 custom_vars 取值，未找到则原样保留。
pub fn replace_vars(
    text: &str,
    player: &PlayerState,
    npc_name: &str,
    custom_vars: &HashMap<String, String>,
) -> String {
    let mut out = text.to_string();

    out = out.replace("<$USERNAME>", &player.name);
    out = out.replace("<$username>", &player.name);
    out = out.replace("<$LEVEL>", &player.level.to_string());
    out = out.replace("<$level>", &player.level.to_string());
    out = out.replace("<$NPCNAME>", npc_name);
    out = out.replace("<$npcname>", npc_name);
    out = out.replace("<$HP>", &player.hp.to_string());
    out = out.replace("<$hp>", &player.hp.to_string());
    out = out.replace("<$MAXHP>", &player.max_hp.to_string());
    out = out.replace("<$maxhp>", &player.max_hp.to_string());
    out = out.replace("<$MP>", &player.mp.to_string());
    out = out.replace("<$mp>", &player.mp.to_string());
    out = out.replace("<$MAXMP>", &player.max_mp.to_string());
    out = out.replace("<$maxmp>", &player.max_mp.to_string());
    out = out.replace("<$GAMEGOLD>", &player.inventory.gold.to_string());
    out = out.replace("<$gamegold>", &player.inventory.gold.to_string());
    out = out.replace("<$CLASS>", class_name(player.class));
    out = out.replace("<$class>", class_name(player.class));
    out = out.replace("<$PKPOINT>", &player.pk_points.to_string());
    out = out.replace("<$pkpoint>", &player.pk_points.to_string());
    out = out.replace("<$GENDER>", gender_name(player.gender));
    out = out.replace("<$gender>", gender_name(player.gender));

    for (k, v) in custom_vars {
        if !k.is_empty() {
            out = out.replace(k.as_str(), v.as_str());
        }
    }

    out
}

fn class_name(c: MirClass) -> &'static str {
    match c {
        MirClass::Warrior => "Warrior",
        MirClass::Wizard => "Wizard",
        MirClass::Taoist => "Taoist",
        MirClass::Assassin => "Assassin",
        MirClass::Archer => "Archer",
    }
}

fn gender_name(g: mir2_shared::enums::MirGender) -> &'static str {
    match g {
        mir2_shared::enums::MirGender::Male => "Male",
        mir2_shared::enums::MirGender::Female => "Female",
    }
}

/// 把 MirClass 字符串名/数字解析回枚举（CHECKCLASS / CHANGECLASS 用）
pub fn parse_class(name: &str) -> Option<MirClass> {
    match name.to_lowercase().as_str() {
        "warrior" | "0" => Some(MirClass::Warrior),
        "wizard" | "1" => Some(MirClass::Wizard),
        "taoist" | "2" => Some(MirClass::Taoist),
        "assassin" | "3" => Some(MirClass::Assassin),
        "archer" | "4" => Some(MirClass::Archer),
        _ => None,
    }
}

// =============================================================================
// Check 求值
// =============================================================================

/// 求值一组 check（AND 逻辑；空集 = 恒真）
async fn eval_checks(
    world: &mut WorldActor,
    session_id: u64,
    player: &PlayerState,
    checks: &[Check],
) -> bool {
    if checks.is_empty() {
        return true;
    }
    for c in checks {
        if !eval_one_check(world, session_id, player, c).await {
            return false;
        }
    }
    true
}

async fn eval_one_check(
    world: &mut WorldActor,
    session_id: u64,
    player: &PlayerState,
    c: &Check,
) -> bool {
    let args = &c.args;
    let arg0 = || args.first().map(|s| s.as_str()).unwrap_or("");
    let arg1 = || args.get(1).map(|s| s.as_str()).unwrap_or("");

    match c.check_type.as_str() {
        // CHECKGOLD <op> <amount>  /  CHECKGOLD <amount>（默认 >=）
        "CHECKGOLD" => {
            let (op, amount) = parse_op_amount(args);
            compare_i64(player.inventory.gold as i64, op, amount)
        }
        // CHECKITEM <name|index> <count>
        "CHECKITEM" => {
            let (idx, cnt) = parse_item_count(args, world);
            if idx == 0 {
                false
            } else {
                has_item(world, session_id, idx, cnt).await
            }
        }
        // CHECKCLASS <Warrior|Wizard|...|class_index>
        // C# NPCSegment.cs:2222 用单类名匹配，不支持位掩码
        "CHECKCLASS" => {
            let a = arg0();
            if let Some(cls) = parse_class(a) {
                // 类名字符串（Warrior/Wizard/Taoist/Assassin/Archer）
                player.class == cls
            } else if let Ok(idx) = a.parse::<u8>() {
                // 数字 → 类索引（C# 语义：0=Warrior,1=Wizard,2=Taoist,3=Assassin,4=Archer）
                if let Ok(cls) = mir2_shared::enums::MirClass::try_from(idx) {
                    player.class == cls
                } else {
                    false
                }
            } else {
                false
            }
        }
        // CHECKLEVEL <op> <level>  / CHECKLEVEL <min> <max>
        "CHECKLEVEL" => {
            let (op, amount) = parse_op_amount(args);
            if op.is_empty() && args.len() >= 2 {
                let lo = args[0].parse::<i64>().unwrap_or(0);
                let hi = args[1].parse::<i64>().unwrap_or(i64::MAX);
                (player.level as i64) >= lo && (player.level as i64) <= hi
            } else {
                compare_i64(player.level as i64, op, amount)
            }
        }
        // CHECKQUEST <index> <state>  state: 1=in progress, 2=completed
        "CHECKQUEST" => {
            let idx = arg0().parse::<i32>().unwrap_or(0);
            let want = arg1().parse::<u8>().unwrap_or(1);
            let actual = quest_state(world, session_id, idx).await;
            actual == want
        }
        // CHECK：通用 flag 检查  CHECK [535] 1
        "CHECK" => {
            if let Some(flag) = parse_flag(arg0()) {
                let want = arg1().parse::<i32>().unwrap_or(1);
                player.flags.get(&format!("NPC_FLAG_{}", flag)).copied().unwrap_or(0) == want
            } else {
                false
            }
        }
        // RANDOM <n>  → 1/n 概率为真
        "RANDOM" => {
            let n = arg0().parse::<u32>().unwrap_or(1).max(1);
            fastrand::u32(1..=n) == 1
        }
        // ISADMIN
        "ISADMIN" => player.is_gm,
        // CHECKPKPOINT <op> <amount>
        "CHECKPKPOINT" => {
            let (op, amount) = parse_op_amount(args);
            compare_i64(player.pk_points as i64, op, amount)
        }
        // CHECKBUFF <type> — 检查是否拥有任意 buff（BuffType 为枚举+数据，简化为非空判断）
        "CHECKBUFF" => {
            let _want = arg0();
            !player.buffs.is_empty()
        }
        // INGUILD / GUILDNAME <name>
        "INGUILD" => player.guild_name.is_some(),
        // CHECKMAP <map_name|index>
        "CHECKMAP" => {
            let want = arg0();
            if let Ok(idx) = want.parse::<u16>() {
                player.map_index == idx
            } else {
                world
                    .map_infos
                    .values()
                    .any(|m| m.file_name.eq_ignore_ascii_case(want) && m.index as u16 == player.map_index)
            }
        }
        // CHECKGENDER <Male|Female|0|1>
        "CHECKGENDER" => {
            let want = arg0();
            let want_byte = match want.to_lowercase().as_str() {
                "male" | "0" => Some(0u8),
                "female" | "1" => Some(1u8),
                _ => want.parse::<u8>().ok(),
            };
            want_byte.map(|w| player.gender as u8 == w).unwrap_or(false)
        }
        // GROUPCOUNT <op> <n>
        "GROUPCOUNT" => {
            let (op, amount) = parse_op_amount(args);
            let cnt = player.group_id.map(|_| 1i64).unwrap_or(0);
            compare_i64(cnt, op, amount)
        }
        // HASBAGSPACE
        "HASBAGSPACE" => has_item_space(world, session_id).await,
        _ => {
            debug!("NPC check '{}' not implemented, treating as PASS", c.check_type);
            true
        }
    }
}

// =============================================================================
// Action 执行
// =============================================================================

async fn exec_action(
    world: &mut WorldActor,
    session_id: u64,
    npc: &NpcState,
    act: &Action,
    flow: &mut FlowControl,
    custom_vars: &mut HashMap<String, String>,
) {
    let args = &act.args;
    let arg0 = || args.first().map(|s| s.as_str()).unwrap_or("");
    let arg1 = || args.get(1).map(|s| s.as_str()).unwrap_or("");
    let arg2 = || args.get(2).map(|s| s.as_str()).unwrap_or("");
    let arg3 = || args.get(3).map(|s| s.as_str()).unwrap_or("");

    match act.action_type.as_str() {
        // GIVEGOLD <amount>
        "GIVEGOLD" => {
            let amt = arg0().parse::<u64>().unwrap_or(0);
            send_player_msg(world, session_id, AddGold { amount: amt }).await;
        }
        // TAKEGOLD <amount>（无参则不扣，兼容脚本）
        "TAKEGOLD" => {
            let amt = arg0().parse::<u64>().unwrap_or(0);
            if amt > 0 {
                send_player_msg(world, session_id, DeductGold { amount: amt }).await;
            }
        }
        // GIVEITEM <name|index> <count>
        "GIVEITEM" => {
            let (idx, cnt) = parse_item_count_action(args, world);
            if idx > 0 {
                give_item(world, session_id, idx, cnt).await;
            }
        }
        // TAKEITEM <name|index> <count>
        "TAKEITEM" => {
            let (idx, cnt) = parse_item_count_action(args, world);
            if idx > 0 {
                take_item(world, session_id, idx, cnt).await;
            }
        }
        // GIVEEXP <amount>
        "GIVEEXP" | "ADDEXP" | "ADD EXP" => {
            let amt = arg0().parse::<i32>().unwrap_or(0);
            let boosted = world.apply_global_exp_multiplier(amt);
            send_player_msg(world, session_id, AddExperience { amount: boosted }).await;
        }
        // GOTO @section
        "GOTO" => {
            let target = arg0().trim_start_matches('@').to_string();
            flow.goto = Some(target);
        }
        // BREAK
        "BREAK" => {
            flow.break_loop = true;
        }

        // ROLLDIE <page> <autoRoll> / ROLLYUT <page> <autoRoll>（C# ActionType.RollDie/RollYut）
        // 掷骰后发 Roll 包（270），客户端动画结束回 CallNPC "[page]"，结果存 %NPCRollResult
        "ROLLDIE" | "ROLLYUT" => {
            let page = unquote(arg0()).to_string();
            let auto_roll = arg1().eq_ignore_ascii_case("true") || arg1() == "1";
            let result = fastrand::i32(1..=6);
            custom_vars.insert("NPCRollResult".into(), result.to_string());
            let r#type = if act.action_type == "ROLLYUT" { 1 } else { 0 };
            let packet = mir2_shared::packets::server::ui_events::Roll {
                r#type,
                page: page.clone(),
                result,
                auto_roll,
            };
            let mut body = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet).is_ok() {
                let _ = world.gate_ref.tell(SendToClient {
                    session_id,
                    data: body,
                }).await;
            }
            info!("NPC {}: ROLL type={} result={} page={} auto={}", npc.name, r#type, result, page, auto_roll);
        }
        // MOVE <map_index> <x> <y>（C# 格式）
        "MOVE" | "MAPMOVE" | "TELEPORT" => {
            let map_idx = arg0().parse::<u16>().unwrap_or(0);
            let x = arg1().parse::<i32>().unwrap_or(0);
            let y = arg2().parse::<i32>().unwrap_or(0);
            teleport_player(world, session_id, map_idx, x, y).await;
        }
        // SET [flag] <value>
        "SET" => {
            if let Some(flag) = parse_flag(arg0()) {
                let val = arg1().parse::<i32>().unwrap_or(1);
                set_player_flag(world, session_id, format!("NPC_FLAG_{}", flag), val).await;
            }
        }
        // MONGEN <name> <count>  (TODO: 真实刷怪)
        "MONGEN" | "MAKEMON" | "MONSTER" => {
            warn!("NPC action MONGEN '{}' not fully implemented (TODO)", arg0());
        }
        // GIVEBUFF <type> <duration>  (TODO)
        "GIVEBUFF" => {
            warn!("NPC action GIVEBUFF '{}' not fully implemented (TODO)", arg0());
        }
        // GIVESKILL <skill_name> <level>  (TODO)
        "GIVESKILL" => {
            warn!("NPC action GIVESKILL '{}' not fully implemented (TODO)", arg0());
        }
        // CHANGECLASS <Warrior|...>
        "CHANGECLASS" => {
            if let Some(cls) = parse_class(arg0()) {
                send_player_msg(world, session_id, ChangeClass { class: cls }).await;
            }
        }
        // CHANGEHAIR <style>
        "CHANGEHAIR" => {
            let h = arg0().parse::<u8>().unwrap_or(0);
            send_player_msg(world, session_id, SetHair { hair: h }).await;
        }
        // LOCALMESSAGE "msg" <type>
        "LOCALMESSAGE" | "MESSAGE" | "SYSMSG" => {
            let msg = unquote(arg0()).to_string();
            let _kind = arg1().parse::<u8>().unwrap_or(0);
            send_system_message(&world.gate_ref, session_id, &msg);
        }
        // GROUPRECALL  (TODO: 组队召回)
        "GROUPRECALL" | "RECALLGROUP" => {
            warn!("NPC action GROUPRECALL not fully implemented (TODO)");
        }
        // MOV <var> <value>   var 形如 A0/B0/C0...（内部存为 %A0）
        "MOV" => {
            let var = normalize_custom_var(arg0());
            let val = unquote(arg1()).to_string();
            custom_vars.insert(var, val);
        }
        // CALC <dst> <op> <src1> <src2>   例：CALC A0 + A1 B0
        "CALC" => {
            let dst = normalize_custom_var(arg0());
            let op = arg1();
            let a = resolve_num(arg2(), custom_vars);
            let b = resolve_num(arg3(), custom_vars);
            let res = match op {
                "+" => a.wrapping_add(b),
                "-" => a.wrapping_sub(b),
                "*" => a.wrapping_mul(b),
                "/" if b != 0 => a / b,
                "%" if b != 0 => a % b,
                _ => a,
            };
            custom_vars.insert(dst, res.to_string());
        }
        // COMPOSEMAIL "body" <sender>  (TODO)
        "COMPOSEMAIL" => {
            warn!("NPC action COMPOSEMAIL not fully implemented (TODO)");
        }
        // EXIT / CLOSE / RETURN — 终止对话
        "EXIT" | "CLOSE" | "RETURN" => {
            flow.break_loop = true;
        }
        _ => {
            debug!("NPC action '{}' not implemented, ignored", act.action_type);
        }
    }
    let _ = npc; // 保留参数供未来动作使用
}

// =============================================================================
// 辅助：玩家消息发送 / 物品 / 传送 / flag
// =============================================================================

async fn send_player_msg<M>(world: &WorldActor, session_id: u64, msg: M)
where
    PlayerActor: kameo::message::Message<M>,
    M: Send + 'static,
{
    if let Some(record) = world.players.get(&session_id) {
        let _ = record.actor_ref.ask(msg).await;
    }
}

/// 拉取当前玩家状态快照；玩家不存在返回 None
async fn current_player_state(world: &WorldActor, session_id: u64) -> Option<PlayerState> {
    let record = world.players.get(&session_id)?;
    record.actor_ref.ask(GetPlayerState).await.ok().flatten()
}

async fn has_item(world: &WorldActor, session_id: u64, item_index: i32, count: u16) -> bool {
    if let Some(record) = world.players.get(&session_id) {
        record
            .actor_ref
            .ask(HasItem { item_index, count })
            .await
            .unwrap_or(false)
    } else {
        false
    }
}

async fn has_item_space(world: &WorldActor, session_id: u64) -> bool {
    if let Some(record) = world.players.get(&session_id) {
        record
            .actor_ref
            .ask(HasItemSpace)
            .await
            .unwrap_or(false)
    } else {
        false
    }
}

async fn quest_state(world: &WorldActor, session_id: u64, quest_index: i32) -> u8 {
    if let Some(record) = world.players.get(&session_id) {
        record
            .actor_ref
            .ask(CheckQuestState { quest_index })
            .await
            .unwrap_or(0)
    } else {
        0
    }
}

async fn give_item(world: &WorldActor, session_id: u64, item_index: i32, count: u16) {
    let Some(record) = world.players.get(&session_id) else { return };
    let info = world.item_infos.get(&item_index);
    let max_dura = info.map(|i| i.durability as u16).unwrap_or(0);
    let identified = info.map(|i| i.is_identified()).unwrap_or(false);
    let stack_size = info.map(|i| i.stack_size).unwrap_or(1).max(1) as u16;

    let remaining = count.max(1);
    // 按 stack_size 分批创建堆叠物品（对齐 C# GiveItem 遵守 StackSize）
    let mut left = remaining;
    while left > 0 {
        let batch = left.min(stack_size);
        let item = mir2_shared::data::item::UserItem {
            unique_id: crate::actors::inventory::generate_item_uid(),
            item_index,
            count: batch,
            current_dura: max_dura,
            max_dura: max_dura,
            identified,
            ..Default::default()
        };
        let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
        left -= batch;
    }
}

async fn take_item(world: &WorldActor, session_id: u64, item_index: i32, count: u16) {
    let Some(record) = world.players.get(&session_id) else { return };
    let _ = record
        .actor_ref
        .ask(RemoveItemByIndex { item_index, count })
        .await;
}

async fn set_player_flag(world: &WorldActor, session_id: u64, key: String, val: i32) {
    let Some(record) = world.players.get(&session_id) else { return };
    if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
        state.flags.insert(key, val);
        let _ = record.actor_ref.ask(SetPlayerState { state }).await;
    }
}

/// 传送：MOVE map_index x y
async fn teleport_player(world: &mut WorldActor, session_id: u64, map_index: u16, x: i32, y: i32) {
    let dest = world.map_infos.get(&(map_index as i32)).cloned();
    let Some(dest_mi) = dest else {
        // 无地图配置：仍尝试在同图改坐标
        if let Some(record) = world.players.get(&session_id) {
            let _ = record
                .actor_ref
                .ask(SetPlayerPosition { x, y, direction: 4, map_index: Some(map_index), is_mounted: None })
                .await;
        }
        return;
    };

    let dest_file = dest_mi.file_name.clone();
    let dest_title = dest_mi.title.clone();
    let _ = world.get_or_load_map(&dest_file, map_index);

    if let Some(record) = world.players.get(&session_id) {
        let _ = record
            .actor_ref
            .ask(SetPlayerPosition { x, y, direction: 4, map_index: Some(map_index), is_mounted: None })
            .await;
        let map_pkt = build_map_changed_packet(map_index, &dest_file, &dest_title, x, y, false);
        let _ = world.gate_ref.tell(SendToClient { session_id, data: map_pkt }).await;
        let mut body = Vec::new();
        body.extend_from_slice(&x.to_le_bytes());
        body.extend_from_slice(&y.to_le_bytes());
        body.push(4u8);
        let _ = world.gate_ref.tell(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::UserLocation as i16, &body),
        }).await;
    }
}

// =============================================================================
// 参数解析辅助
// =============================================================================

/// 从 args 中解析比较运算符与数值。
/// 形如 `> 500` / `>= 500` / `= 500` → (">", 500)
/// 单独 `500` → ("", 500)，调用方按 >= 处理
fn parse_op_amount(args: &[String]) -> (&'static str, i64) {
    match args.len() {
        0 => ("", 0),
        1 => ("", args[0].parse::<i64>().unwrap_or(0)),
        _ => {
            // 第一参若为运算符，第二参为数值；否则第一参是裸数值（默认 >=）
            let op = match args[0].as_str() {
                ">" | "gt" => ">",
                ">=" | "ge" => ">=",
                "<" | "lt" => "<",
                "<=" | "le" => "<=",
                "=" | "==" | "eq" => "==",
                "!=" | "<>" | "ne" => "!=",
                _ => "",
            };
            if op.is_empty() {
                ("", args[0].parse::<i64>().unwrap_or(0))
            } else {
                let val = args[1].parse::<i64>().unwrap_or(0);
                (op, val)
            }
        }
    }
}

fn compare_i64(lhs: i64, op: &str, rhs: i64) -> bool {
    match op {
        ">" | "gt" => lhs > rhs,
        ">=" | "ge" => lhs >= rhs,
        "<" | "lt" => lhs < rhs,
        "<=" | "le" => lhs <= rhs,
        "=" | "==" | "eq" => lhs == rhs,
        "!=" | "<>" | "ne" => lhs != rhs,
        "" => lhs >= rhs, // 默认 >=（CHECKGOLD/CHECKLEVEL 常见用法）
        _ => lhs >= rhs,
    }
}

/// CHECKITEM 用：解析物品与数量。第一参可是 index 或 name（name 在 world.item_infos 反查）
fn parse_item_count(args: &[String], world: &WorldActor) -> (i32, u16) {
    if args.is_empty() {
        return (0, 1);
    }
    let first = &args[0];
    let idx = if let Ok(i) = first.parse::<i32>() {
        i
    } else {
        let lower = first.to_lowercase();
        world
            .item_infos
            .values()
            .find(|info| info.name.to_lowercase() == lower)
            .map(|info| info.index)
            .unwrap_or(0)
    };
    let cnt = args.get(1).and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
    (idx, cnt)
}

/// Action 的物品/数量解析
fn parse_item_count_action(args: &[String], world: &WorldActor) -> (i32, u16) {
    parse_item_count(args, world)
}

/// 解析 `[535]` 形式的 flag 编号
fn parse_flag(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        inner.parse::<i32>().ok()
    } else {
        s.parse::<i32>().ok()
    }
}

/// 去掉首尾引号
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// 把 `A0`/`%A0` 归一化为 `%A0`（自定义变量内部 key）
fn normalize_custom_var(s: &str) -> String {
    if s.starts_with('%') {
        s.to_string()
    } else {
        format!("%{}", s)
    }
}

/// 解析数值参数：可能是字面量数字，也可能是 `%var` 引用
fn resolve_num(s: &str, custom_vars: &HashMap<String, String>) -> i64 {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('%') {
        let key = format!("%{}", rest);
        custom_vars.get(&key).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0)
    } else {
        s.parse::<i64>().unwrap_or(0)
    }
}

// =============================================================================
// 按钮 / 入口辅助
// =============================================================================

/// 从一段文本里解析按钮 `<显示文字/@target>` 列表
pub fn parse_buttons(text: &str) -> Vec<(String, String)> {
    let mut buttons = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(close) = text[i + 1..].find('>') {
                let inner = &text[i + 1..i + 1 + close];
                if let Some(slash) = inner.find('/') {
                    let label = inner[..slash].to_string();
                    let target = inner[slash + 1..].to_string();
                    if !target.is_empty() {
                        buttons.push((label, target));
                    }
                }
                i = i + 1 + close + 1;
                continue;
            }
        }
        i += 1;
    }
    buttons
}

/// 判断给定脚本文本是否为 C# 格式（首条非空非注释行以 `[` 段头或 `#` 指令开头）。
/// 用于 npc.rs 选择走新引擎还是旧的 `<CMD>` 解析。
pub fn is_csharp_format(script_text: &str) -> bool {
    for line in script_text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') {
            continue;
        }
        return t.starts_with('[') || t.starts_with('#');
    }
    false
}

// =============================================================================
// 单元测试（纯解析逻辑，不依赖 WorldActor）
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
; 注释行
[@MAIN]
#IF
CHECKPKPOINT > 2
#SAY
I will not help an evil person like you...
<Close/@exit>
#ELSEACT
GOTO @Main-1

[@Main-1]
#SAY
Hello i'm Jason.
I transport men and goods.
<BorderVillage/@move1>

[@move1]
#IF
CHECKGOLD > 500
#ACT
MOVE 0 289 617
TAKEGOLD 500
#ELSEACT
GOTO @B1

[@B1]
#SAY
You don't have enough Gold!
"#;

    #[test]
    fn parses_all_sections_case_insensitive() {
        let script = ParsedScript::parse(SAMPLE);
        // 4 sections: MAIN, Main-1, move1, B1
        assert_eq!(script.sections.len(), 4);
        // 大小写不敏感查找
        assert!(script.find("main").is_some());
        assert!(script.find("MAIN").is_some());
        assert!(script.find("Main-1").is_some());
        assert!(script.find("move1").is_some());
        assert!(script.find("b1").is_some());
        // 带 @ 前缀也行
        assert!(script.find("@main").is_some());
        assert!(script.main_section().is_some());
    }

    #[test]
    fn parses_if_act_elseact_segments() {
        let script = ParsedScript::parse(SAMPLE);
        let main = script.find("main").unwrap();
        // MAIN 段应有一个带 #IF 的 segment
        assert!(!main.segments.is_empty());
        let seg0 = &main.segments[0];
        // checks: CHECKPKPOINT > 2
        assert_eq!(seg0.checks.len(), 1);
        assert_eq!(seg0.checks[0].check_type, "CHECKPKPOINT");
        assert_eq!(seg0.checks[0].args, vec![">", "2"]);
        // say 命中时
        assert!(seg0.say.iter().any(|s| s.contains("evil person")));
        // else_actions: GOTO @Main-1
        assert_eq!(seg0.else_actions.len(), 1);
        assert_eq!(seg0.else_actions[0].action_type, "GOTO");
        assert_eq!(seg0.else_actions[0].args, vec!["@Main-1"]);
    }

    #[test]
    fn parses_act_block_with_multiple_actions() {
        let script = ParsedScript::parse(SAMPLE);
        let mv = script.find("move1").unwrap();
        let seg = &mv.segments[0];
        // #IF CHECKGOLD > 500
        assert_eq!(seg.checks[0].check_type, "CHECKGOLD");
        // #ACT: MOVE + TAKEGOLD
        assert_eq!(seg.actions.len(), 2);
        assert_eq!(seg.actions[0].action_type, "MOVE");
        assert_eq!(seg.actions[0].args, vec!["0", "289", "617"]);
        assert_eq!(seg.actions[1].action_type, "TAKEGOLD");
        assert_eq!(seg.actions[1].args, vec!["500"]);
        // #ELSEACT: GOTO @B1
        assert_eq!(seg.else_actions[0].action_type, "GOTO");
    }

    #[test]
    fn parses_unconditional_say_section() {
        let script = ParsedScript::parse(SAMPLE);
        // Main-1 没有 #IF，是无条件 SAY
        let m1 = script.find("Main-1").unwrap();
        let seg = &m1.segments[0];
        assert!(seg.checks.is_empty());
        assert!(seg.say.iter().any(|s| s.contains("Jason")));
    }

    #[test]
    fn is_csharp_format_detects_sections() {
        assert!(is_csharp_format(SAMPLE));
        assert!(is_csharp_format("[@Main]\n#SAY\nhi"));
        assert!(!is_csharp_format("<CHECKLEVEL 1 50>\n<END>"));
        // 注释/空行开头不影响判定
        assert!(is_csharp_format("\n; comment\n[@x]\n#SAY\ny"));
    }

    #[test]
    fn parse_buttons_extracts_label_and_target() {
        let buttons = parse_buttons("Teleport to: <BorderVillage/@move1> {(500 Gold)/GOLD}\n<Close/@exit>");
        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0].0, "BorderVillage");
        assert_eq!(buttons[0].1, "@move1"); // @ 前缀保留（C# 原始格式）
        assert_eq!(buttons[1].0, "Close");
        assert_eq!(buttons[1].1, "@exit");
    }

    #[test]
    fn op_amount_parsing() {
        let a = [">".to_string(), "500".to_string()];
        assert_eq!(parse_op_amount(&a), (">", 500));
        let b = ["500".to_string()];
        assert_eq!(parse_op_amount(&b), ("", 500));
        let c: Vec<String> = vec![];
        assert_eq!(parse_op_amount(&c), ("", 0));
        let d = ["<=".to_string(), "10".to_string()];
        assert_eq!(parse_op_amount(&d), ("<=", 10));
    }

    #[test]
    fn compare_i64_supports_all_ops() {
        assert!(compare_i64(10, ">", 5));
        assert!(compare_i64(10, ">=", 10));
        assert!(compare_i64(5, "<", 10));
        assert!(compare_i64(5, "<=", 5));
        assert!(compare_i64(5, "==", 5));
        assert!(compare_i64(5, "!=", 6));
        // 默认 >=
        assert!(compare_i64(10, "", 5));
        assert!(!compare_i64(3, "", 5));
    }

    #[test]
    fn parse_flag_handles_brackets() {
        assert_eq!(parse_flag("[535]"), Some(535));
        assert_eq!(parse_flag("535"), Some(535));
        assert_eq!(parse_flag("abc"), None);
    }

    #[test]
    fn tokenize_preserves_quoted_whitespace() {
        let t = tokenize_args(r#"LOCALMESSAGE "hello world" 0"#);
        assert_eq!(t, vec!["LOCALMESSAGE", "hello world", "0"]);
    }

    #[test]
    fn custom_var_normalize_and_resolve() {
        let mut vars = HashMap::new();
        vars.insert("%A0".to_string(), "42".to_string());
        assert_eq!(normalize_custom_var("A0"), "%A0");
        assert_eq!(normalize_custom_var("%B0"), "%B0");
        assert_eq!(resolve_num("%A0", &vars), 42);
        assert_eq!(resolve_num("10", &vars), 10);
        assert_eq!(resolve_num("%B0", &vars), 0);
    }

    #[test]
    fn handles_crlf_and_brace_annotations_from_real_script() {
        // 真实脚本片段：CRLF 行尾 + {(500 Gold)/GOLD} 花括号注解（视为普通文本）
        let crlf = "[@tele]\r\n#SAY\r\nWhich place?\r\n \r\nTeleport to: <BorderVillage/@move1> {(500 Gold)/GOLD}\r\n";
        let script = ParsedScript::parse(crlf);
        let tele = script.find("tele").expect("tele section");
        let seg = &tele.segments[0];
        assert!(seg.checks.is_empty());
        // 所有非空文本行都应进 say
        let joined = seg.say.join("|");
        assert!(joined.contains("Which place?"));
        assert!(joined.contains("BorderVillage"));
        // 花括号注解保留为文本（不在 < > 内，不当作按钮）
        assert!(joined.contains("{(500 Gold)/GOLD}"));
        // 按钮仅识别 <BorderVillage/@move1>
        let all_text = seg.say.join("\n");
        let btns = parse_buttons(&all_text);
        assert_eq!(btns.len(), 1);
        assert_eq!(btns[0].0, "BorderVillage");
    }

    #[test]
    fn ignores_insert_directive_and_comments() {
        let s = "; a comment\n#INSERT [SystemScripts\\00Default\\Login.txt] @Main\n[@Main]\n#SAY\nhi\n";
        let script = ParsedScript::parse(s);
        assert_eq!(script.sections.len(), 1);
        let main = script.find("main").unwrap();
        assert!(main.segments[0].say.iter().any(|l| l.contains("hi")));
        // INSERT 不应产生任何 section
        assert!(script.find("INSERT").is_none());
    }

    #[test]
    fn multiple_segments_in_one_section() {
        // 一个段内有多个 #IF 块（每个 #IF 开启新 segment）
        let s = "[@x]\n#IF\nCHECKGOLD > 100\n#SAY\nrich\n#IF\nCHECKGOLD < 10\n#SAY\npoor\n";
        let script = ParsedScript::parse(s);
        let x = script.find("x").unwrap();
        assert_eq!(x.segments.len(), 2);
        assert_eq!(x.segments[0].checks[0].check_type, "CHECKGOLD");
        assert!(x.segments[0].say.iter().any(|l| l.contains("rich")));
        assert_eq!(x.segments[1].checks[0].check_type, "CHECKGOLD");
        assert!(x.segments[1].say.iter().any(|l| l.contains("poor")));
    }
}

