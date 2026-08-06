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
    GetPlayerState, HasItem, RemoveItemByIndexWithDura, SetHair, SetPlayerPosition,
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
    /// MAP/PARAM1/PARAM2/PARAM3 脚本上下文（MONGEN 等指令用，对齐 C# Param1/2/3）
    map_name: Option<String>,
    param1: i32,
    param2: i32,
    param3: i32,
    /// COMPOSEMAIL 暂存的邮件草稿（ADDMAILGOLD/ADDMAILITEM/SENDMAIL 链）
    mail: Option<crate::actors::mail::MailMessage>,
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
        // CHECKITEM <name|index> <count> [dura] — 背包物品数量（>=，对齐 C# CheckType.CheckItem）
        // C#：count 缺省为 1；dura 存在且可解析时只统计 current_dura >= dura*1000 的物品
        // （NPCSegment.cs CheckItem，与 CHECKHEROITEM 一致）。
        "CHECKITEM" => {
            let (idx, cnt) = parse_item_count(args, world);
            let min_dura = args.get(2).and_then(|s| s.parse::<u32>().ok());
            if idx == 0 {
                false
            } else {
                let total: u16 = player.inventory.backpack.iter().flatten()
                    .filter(|s| s.item.item_index == idx)
                    .filter(|s| min_dura.map(|d| (s.item.current_dura as u32) >= d * 1000).unwrap_or(true))
                    .map(|s| s.item.count)
                    .sum();
                total >= cnt
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
        // CHECKQUEST <index> <ACTIVE|COMPLETE> — 任务状态（对齐 C# CheckType.CheckQuest：
        // ACTIVE=进行中；其他（如 COMPLETE）=已完成；兼容数字 1=进行中、2=已完成）
        "CHECKQUEST" => {
            let idx = arg0().parse::<i32>().unwrap_or(0);
            let state = arg1().trim().to_uppercase();
            let actual = quest_state(world, session_id, idx).await;
            match state.as_str() {
                "1" | "ACTIVE" => actual == 1,
                _ => actual == 2,
            }
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
        // CHECKBUFF <type> — 检查是否拥有指定 Buff（对齐 C# CheckType.CheckBuff：HasBuff；
        // BuffType 带数据，用 discriminant 比较类型）
        "CHECKBUFF" => {
            match parse_buff_type(arg0()) {
                Some(want_bt) => {
                    let want_disc = std::mem::discriminant(&want_bt);
                    player.buffs.iter().any(|b| std::mem::discriminant(&b.buff_type) == want_disc)
                }
                None => false,
            }
        }
        // CHECKTIMER <key> <op> <seconds> — 检查计时器剩余秒数（对齐 C# CheckType.CheckTimer；
        // 先查全局 Envir.Timers["_-"+key]，再查玩家个人计时器；无计时器视为 0）
        "CHECKTIMER" => {
            let key = arg0().parse::<i32>().unwrap_or(0);
            let (op, want) = parse_op_amount(&args[1..]);
            let expire = world.npc_timers.get(&GLOBAL_TIMER_SESSION).and_then(|m| m.get(&key)).copied()
                .or_else(|| world.npc_timers.get(&session_id).and_then(|m| m.get(&key)).copied());
            let remaining = npc_timer_remaining_secs(world.tick_count, expire);
            compare_i64(remaining, op, want)
        }
        // CHECKDAY <DayOfWeek> — 当前星期几（对齐 C# CheckType.CheckDay）
        "CHECKDAY" => {
            now_weekday_upper().eq_ignore_ascii_case(arg0().trim())
        }
        // CHECKHOUR <hour> — 当前小时（对齐 C# CheckType.CheckHour）
        "CHECKHOUR" => {
            let want = arg0().parse::<u32>().unwrap_or(u32::MAX);
            now_hour() == want
        }
        // CHECKMINUTE <minute> — 当前分钟（对齐 C# CheckType.CheckMinute）
        "CHECKMINUTE" => {
            let want = arg0().parse::<u32>().unwrap_or(u32::MAX);
            now_minute() == want
        }
        // CHECKNAMELIST <file> — 玩家名在名单文件（对齐 C# CheckType.CheckNameList）
        "CHECKNAMELIST" => {
            let file_path = arg0();
            if file_path.is_empty() { false }
            else {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                path.starts_with(&base) && name_list_contains(&path, &player.name)
            }
        }
        // CHECKGUILDNAME <file> — 行会名在名单文件（对齐 C# CheckType.CheckGuildNameList；需在行会）
        "CHECKGUILDNAME" | "CHECKGUILDNAMELIST" => {
            let file_path = arg0();
            if file_path.is_empty() { false }
            else if let Some(guild_name) = &player.guild_name {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                path.starts_with(&base) && name_list_contains(&path, guild_name)
            } else {
                false
            }
        }
        // CHECKHUM <op> <count> <map> <instance> — 地图玩家数（对齐 C# CheckType.CheckHum；instance 忽略）
        "CHECKHUM" => {
            let (op, want) = parse_op_amount(args);
            let map_name = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let count = if let Some(mi) = map_index_by_name(world, map_name) {
                let mut n = 0i64;
                for (sid, r) in &world.players {
                    if *sid == session_id { n += 1; continue; }
                    if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                        if os.map_index == mi { n += 1; }
                    }
                }
                n
            } else {
                -1
            };
            compare_i64(count, op, want)
        }
        // CHECKEXACTMON <怪物名> <op> <count> <map> <instance> — 指定怪物数量（对齐 C# CheckType.CheckExactMon）
        // C#：d.Name.Replace(" ","") 与 param[0] 忽略大小写比较（怪物名去空格）；
        // 未知怪物名（GetMonsterInfo null）→ failed（-1）。
        "CHECKEXACTMON" => {
            let monster_name = arg0();
            let (op, want) = parse_op_amount(&args[1..]);
            let map_name = args.get(3).map(|s| s.as_str()).unwrap_or("");
            let count = if let Some(mi) = map_index_by_name(world, map_name) {
                // 怪物名是否存在（去空格、忽略大小写）
                let known = world.monster_name_index.keys()
                    .any(|k| k.replace(' ', "").eq_ignore_ascii_case(monster_name));
                if !known {
                    -1
                } else {
                    world.monsters.values()
                        .filter(|m| m.map_index == mi && m.name.replace(' ', "").eq_ignore_ascii_case(monster_name))
                        .count() as i64
                }
            } else {
                -1
            };
            compare_i64(count, op, want)
        }
        // CHECKMON <op> <count> <map> <instance> — 地图怪物数（对齐 C# CheckType.CheckMon）
        "CHECKMON" => {
            let (op, want) = parse_op_amount(args);
            let map_name = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let count = if let Some(mi) = map_index_by_name(world, map_name) {
                world.monsters.values().filter(|m| m.map_index == mi).count() as i64
            } else {
                -1
            };
            compare_i64(count, op, want)
        }
        // PETLEVEL <op> <level> — 宠物等级比较（对齐 C# CheckType.PetLevel；无宠物视为失败）
        "PETLEVEL" => {
            let (op, want) = parse_op_amount(args);
            let level = player.creature_log.active_creature.as_ref().map(|c| c.level as i64).unwrap_or(-1);
            compare_i64(level, op, want)
        }
        // PETCOUNT <op> <count> — 宠物数量（active + owned，对齐 C# CheckType.PetCount）
        "PETCOUNT" => {
            let (op, want) = parse_op_amount(args);
            let pets = player.creature_log.owned_creatures.len() as i64
                + if player.creature_log.active_creature.is_some() { 1 } else { 0 };
            compare_i64(pets, op, want)
        }
        // CHECKPET [宠物名] — 是否有宠物（对齐 C# CheckType.CheckPet）
        // C# 语义：CHECKPET <宠物名> 遍历所有 Pets 比较 Info.Name（大小写不敏感），任一匹配即真。
        // Rust 无参时保留旧行为（任意已激活宠物）；有参时遍历 active + owned，
        // 比较 custom_name 或类型 Debug 名（BabyPanda…），均忽略大小写。
        "CHECKPET" => {
            let name = arg0().trim();
            if name.is_empty() {
                player.creature_log.active_creature.is_some()
            } else {
                let want = name.to_lowercase();
                let pet_matches = |c: &crate::actors::creature::IntelligentCreature| {
                    c.custom_name.as_deref().map(|n| n.to_lowercase() == want).unwrap_or(false)
                        || format!("{:?}", c.creature_type).to_lowercase() == want
                };
                let active_match = player.creature_log.active_creature.as_ref()
                    .map(|c| pet_matches(c)).unwrap_or(false);
                active_match || player.creature_log.owned_creatures.iter().any(pet_matches)
            }
        }
        // CHECKWEDDINGRING — 已婚且左戒可制作结婚戒指（对齐 C# CheckType.CheckWeddingRing
        // → player.CheckMakeWeddingRing：已婚 + 左戒已装备 + 左戒未绑定结婚戒指
        // （C# -1 / Rust 约定 0）+ 戒指类型允许结婚戒指（!Bind.NoWeddingRing））
        "CHECKWEDDINGRING" => {
            if player.spouse_name.is_none() {
                false
            } else if let Some(ring) = player.inventory.get_equipment(crate::actors::inventory::EquipmentSlot::RingL) {
                if ring.wedding_ring != 0 {
                    // 已是结婚戒指，不能再制作
                    false
                } else {
                    // 戒指类型允许结婚戒指（C# ItemInfo.Bind.NoWeddingRing = 0x0800）
                    let no_wedding = mir2_shared::enums::BindMode::NO_WEDDING_RING.bits() as i32;
                    world.item_infos.get(&ring.item_index)
                        .map(|info| (info.bind_mode & no_wedding) == 0)
                        .unwrap_or(true)
                }
            } else {
                false
            }
        }
        // CHECKRANGE <x> <y> <range> — 玩家在 (x,y) 半径内（对齐 C# CheckType.CheckRange / Functions.InRange 欧氏）
        "CHECKRANGE" => {
            let x = arg0().parse::<i32>().unwrap_or(i32::MIN);
            let y = arg1().parse::<i32>().unwrap_or(i32::MIN);
            let range = args.get(2).map(|s| s.as_str()).unwrap_or("").parse::<i64>().unwrap_or(-1);
            if range < 0 { false }
            else {
                let dx = (player.x as i64) - x as i64;
                let dy = (player.y as i64) - y as i64;
                dx * dx + dy * dy <= range * range
            }
        }
        // CHECKMAPLIGHT <Day|Night|Dawn|Evening> — 全局昼夜状态（对齐 C# CheckType.CheckMapLight / Envir.Lights）
        "CHECKMAPLIGHT" => {
            let want = arg0().trim().to_uppercase();
            let cur = format!("{:?}", world.current_light).to_uppercase();
            !want.is_empty() && cur == want
        }
        // CHECKRELATIONSHIP — 是否已婚（对齐 C# CheckType.CheckRelationship：player.Info.Married != 0）
        "CHECKRELATIONSHIP" => {
            player.spouse_name.is_some()
        }
        // HEROLEVEL <op> <level> — 当前英雄等级（对齐 C# CheckType.HeroLevel）
        "HEROLEVEL" => {
            let (op, want) = parse_op_amount(args);
            let level = current_hero(world, session_id, player).map(|h| h.level as i64).unwrap_or(-1);
            compare_i64(level, op, want)
        }
        // CHECKHEROCLASS <class> — 当前英雄职业（对齐 C# CheckType.CheckHeroClass）
        "CHECKHEROCLASS" => {
            if let Some(cls) = parse_class(arg0()) {
                current_hero(world, session_id, player).map(|h| h.class == cls).unwrap_or(false)
            } else {
                false
            }
        }
        // CHECKHEROGENDER <male|female|0|1> — 当前英雄性别（对齐 C# CheckType.CheckHeroGender）
        "CHECKHEROGENDER" => {
            if let Some(g) = parse_gender(arg0()) {
                current_hero(world, session_id, player).map(|h| h.gender == g).unwrap_or(false)
            } else {
                false
            }
        }
        // CHECKCONQUEST <index> — 领地不在战争中（对齐 C# CheckType.CheckConquest：failed=WarIsOn）
        "CHECKCONQUEST" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            world.conquest_instances.iter()
                .find(|c| c.id == index)
                .map(|c| c.state != crate::actors::world::conquest::WarState::InProgress)
                .unwrap_or(false)
        }
        // CONQUESTOWNER <index> — 玩家行会为该领地所有者（对齐 C# CheckType.ConquestOwner）
        "CONQUESTOWNER" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if let Some(guild_name) = &player.guild_name {
                world.conquest_instances.iter()
                    .find(|c| c.id == index)
                    .map(|c| c.owner_guild.as_deref() == Some(guild_name.as_str()))
                    .unwrap_or(false)
            } else {
                false
            }
        }
        // AFFORDGATE <index> <id> — 行会金币足够修复城门（对齐 C# CheckType.AffordGate）
        "AFFORDGATE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if player.guild_name.is_none() || index < 0 || id < 0 { false }
            else {
                let oid = world.find_siege_structure(index, crate::actors::world::conquest::SiegeStructureType::CastleGate, id);
                let cost = oid.and_then(|o| world.siege_structures.get(&o)).map(|s| s.repair_cost()).unwrap_or(0);
                let gold = world.social_ref.ask(crate::actors::social::NpcGetGuildGold { session_id }).await.unwrap_or(0);
                cost > 0 && gold >= cost
            }
        }
        // AFFORDWALL <index> <id> — 行会金币足够修复城墙（对齐 C# CheckType.AffordWall）
        "AFFORDWALL" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if player.guild_name.is_none() || index < 0 || id < 0 { false }
            else {
                let oid = world.find_siege_structure(index, crate::actors::world::conquest::SiegeStructureType::Wall, id);
                let cost = oid.and_then(|o| world.siege_structures.get(&o)).map(|s| s.repair_cost()).unwrap_or(0);
                let gold = world.social_ref.ask(crate::actors::social::NpcGetGuildGold { session_id }).await.unwrap_or(0);
                cost > 0 && gold >= cost
            }
        }
        // AFFORDGUARD <index> <id> — 守卫（箭塔）负担检查（对齐 C# CheckType.AffordGuard，简化用 ArcherTower）
        "AFFORDGUARD" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if player.guild_name.is_none() || index < 0 || id < 0 { false }
            else {
                let oid = world.find_siege_structure(index, crate::actors::world::conquest::SiegeStructureType::ArcherTower, id);
                let cost = oid.and_then(|o| world.siege_structures.get(&o)).map(|s| s.repair_cost()).unwrap_or(0);
                let gold = world.social_ref.ask(crate::actors::social::NpcGetGuildGold { session_id }).await.unwrap_or(0);
                cost > 0 && gold >= cost
            }
        }
        // AFFORDSIEGE <index> <id> — 攻城器负担检查（对齐 C# CheckType.AffordSiege）
        // C# NPCSegment.cs:2698 实际查 GateList（与 AffordGate 相同），严格对齐
        "AFFORDSIEGE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if player.guild_name.is_none() || index < 0 || id < 0 { false }
            else {
                let oid = world.find_siege_structure(index, crate::actors::world::conquest::SiegeStructureType::CastleGate, id);
                let cost = oid.and_then(|o| world.siege_structures.get(&o)).map(|s| s.repair_cost()).unwrap_or(0);
                let gold = world.social_ref.ask(crate::actors::social::NpcGetGuildGold { session_id }).await.unwrap_or(0);
                cost > 0 && gold >= cost
            }
        }
        // CHECKCREDIT <op> <amount> — 账户积分比较（对齐 C# CheckType.CheckCredit）
        "CHECKCREDIT" => {
            let (op, want) = parse_op_amount(args);
            let credit = if let Some(record) = world.players.get(&session_id) {
                crate::db::get_account_credit(&world.db_pool, &record.account_username).await.unwrap_or(0) as i64
            } else {
                0
            };
            compare_i64(credit, op, want)
        }
        // ISNEWHUMAN — 账户只有 1 个角色（新玩家，对齐 C# CheckType.IsNewHuman）
        "ISNEWHUMAN" => {
            if let Some(record) = world.players.get(&session_id) {
                let count = crate::db::list_character_summaries(&world.db_pool, &record.account_username).await.unwrap_or_default().len();
                count <= 1
            } else {
                false
            }
        }
        // GROUPCHECKNEARBY — 所有组员在 NPC 附近 9 格（对齐 C# CheckType.GroupCheckNearby）
        "GROUPCHECKNEARBY" => {
            // 当前 NPC 位置
            let Some(&npc_oid) = world.session_npc.get(&session_id) else { return false };
            let Some(npc) = world.npcs.get(&npc_oid) else { return false };
            let Some(gid) = player.group_id else { return false };
            let mut all_nearby = true;
            for (sid, r) in &world.players {
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.group_id == Some(gid) {
                        let dx = (os.x - npc.x).abs() as i64;
                        let dy = (os.y - npc.y).abs() as i64;
                        // C# Functions.InRange 欧氏距离：dx²+dy² <= range²（9*9=81）
                        if dx * dx + dy * dy > 81 {
                            all_nearby = false;
                            break;
                        }
                    }
                }
            }
            all_nearby
        }
        // CONQUESTAVAILABLE <index> — 有行会且无人宣战（对齐 C# CheckType.ConquestAvailable：AttackerID == -1）
        "CONQUESTAVAILABLE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if player.guild_name.is_none() {
                false
            } else {
                world.conquest_instances.iter()
                    .find(|c| c.id == index)
                    .map(|c| c.attacker_guild.is_none() && c.state == crate::actors::world::conquest::WarState::Idle)
                    .unwrap_or(false)
            }
        }
        // CHECKHEROITEM <item> [count] [dura] — 英雄背包物品数量（>=，对齐 C# CheckType.CheckHeroItem）
        // C#：count 缺省为 1（parts.Length < 3 → "1"）；dura 存在且可解析时只统计
        // current_dura >= dura*1000 的物品（NPCSegment.cs CheckHeroItem）。
        "CHECKHEROITEM" => {
            let item_name = arg0();
            let count = args.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
            let min_dura = args.get(2).and_then(|s| s.parse::<u32>().ok());
            let idx = world.item_infos.values().find(|i| i.name.eq_ignore_ascii_case(item_name)).map(|i| i.index);
            if let Some(idx) = idx {
                let total: u32 = player.hero_inventory.backpack.iter().flatten()
                    .filter(|s| s.item.item_index == idx)
                    .filter(|s| min_dura.map(|d| (s.item.current_dura as u32) >= d * 1000).unwrap_or(true))
                    .map(|s| s.item.count as u32)
                    .sum();
                total >= count
            } else {
                false
            }
        }
        // CHECKPERMISSION <GuildRankOptions> — 玩家行会职务权限（对齐 C# CheckType.CheckPermission）
        "CHECKPERMISSION" => {
            let bit = match arg0().trim().to_uppercase().as_str() {
                "CANCHANGERANK" => 1u8,
                "CANRECRUIT" => 2u8,
                "CANKICK" => 4u8,
                "CANSTOREITEM" => 8u8,
                "CANRETRIEVEITEM" => 16u8,
                "CANALTERALLIANCE" => 32u8,
                "CANCHANGENOTICE" => 64u8,
                "CANACTIVATEBUFF" => 128u8,
                _ => 0u8,
            };
            if bit == 0 {
                false
            } else {
                let options = world.social_ref.ask(crate::actors::social::NpcGetGuildMemberOptions { session_id }).await.unwrap_or(0);
                options & bit != 0
            }
        }
        // HASGT — 行会是否拥有领地（对齐 C# CheckType.HasGT）
        "HASGT" => {
            if let Some(guild_name) = &player.guild_name {
                world.conquest_instances.iter().any(|c| c.owner_guild.as_deref() == Some(guild_name.as_str()))
            } else {
                false
            }
        }
        // CHECKGUILDGOLD <op> <amount> — 行会金币比较（对齐 C# CheckType.CheckGuildGold）
        "CHECKGUILDGOLD" => {
            let (op, want) = parse_op_amount(args);
            let gold = world.social_ref.ask(crate::actors::social::NpcGetGuildGold { session_id }).await.unwrap_or(0);
            compare_i64(gold as i64, op, want)
        }
        // CHECKCALC <left> <op> <right> — 整数比较（对齐 C# CheckType.CheckCalc）
        "CHECKCALC" => {
            let left = arg0().parse::<i64>().unwrap_or(0);
            let op = arg1();
            let right = args.get(2).map(|s| s.as_str()).unwrap_or("").parse::<i64>().unwrap_or(0);
            compare_i64(left, op, right)
        }
        // GROUPLEADER — 是否队长（对齐 C# CheckType.Groupleader）
        "GROUPLEADER" => {
            world.social_ref.ask(crate::actors::social::NpcIsGroupLeader { session_id }).await.unwrap_or(false)
        }
        // INGUILD / GUILDNAME <name>
        // INGUILD [公会名] — 在行会；指定公会名时需精确匹配（对齐 C# CheckType.InGuild）
        "INGUILD" => {
            let guild_name = arg0().trim();
            if guild_name.is_empty() {
                player.guild_name.is_some()
            } else {
                player.guild_name.as_deref() == Some(guild_name)
            }
        }
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
        // GROUPCOUNT <op> <n> — 组队成员数比较（对齐 C# CheckType.GroupCount：
        // 统计同 group_id 玩家数（含自己）；无组（GroupMembers == null）→ 恒失败）
        "GROUPCOUNT" => {
            let (op, amount) = parse_op_amount(args);
            let Some(gid) = player.group_id else {
                return false;
            };
            let mut cnt = 0i64;
            for (sid, r) in &world.players {
                if *sid == session_id {
                    cnt += 1;
                    continue;
                }
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.group_id == Some(gid) {
                        cnt += 1;
                    }
                }
            }
            compare_i64(cnt, op, amount)
        }
        // HASBAGSPACE <op> <count> — 背包空格数比较（对齐 C# CheckType.HasBagSpace：
        // 统计空槽数，按 op 与 count 比较；无参/缺参时按 >= 0 恒真，等价 C# 缺参丢弃检查）
        "HASBAGSPACE" => {
            let (op, want) = parse_op_amount(args);
            let empty = player.inventory.backpack.iter().filter(|s| s.is_none()).count() as i64;
            compare_i64(empty, op, want)
        },
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
        // GIVEGOLD <amount>（对齐 C# GiveGold：金币上限 uint.MaxValue）
        "GIVEGOLD" => {
            let amt = arg0().parse::<u64>().unwrap_or(0);
            if amt > 0 {
                let cap = u32::MAX as u64;
                let give = if let Some(st) = current_player_state(world, session_id).await {
                    amt.min(cap.saturating_sub(st.inventory.gold.min(cap)))
                } else {
                    amt.min(cap)
                };
                if give > 0 {
                    send_player_msg(world, session_id, AddGold { amount: give }).await;
                }
            }
        }
        // TAKEGOLD <amount>（对齐 C# TakeGold：请求超过现有金币时扣光；无参则不扣）
        "TAKEGOLD" => {
            let amt = arg0().parse::<u64>().unwrap_or(0);
            if amt > 0 {
                let take = if let Some(st) = current_player_state(world, session_id).await {
                    amt.min(st.inventory.gold)
                } else {
                    amt
                };
                if take > 0 {
                    send_player_msg(world, session_id, DeductGold { amount: take }).await;
                }
            }
        }
        // GIVEITEM <name|index> <count>
        "GIVEITEM" => {
            let (idx, cnt) = parse_item_count_action(args, world);
            if idx > 0 {
                give_item(world, session_id, idx, cnt).await;
            }
        }
        // TAKEITEM <name|index> <count> [dura] — 移除背包物品（对齐 C# ActionType.TakeItem；
        // dura 存在且可解析时只移除 current_dura >= dura*1000 的物品）
        "TAKEITEM" => {
            let (idx, cnt) = parse_item_count_action(args, world);
            if idx > 0 {
                let min_dura = args.get(2).and_then(|s| s.parse::<u32>().ok());
                take_item(world, session_id, idx, cnt, min_dura).await;
            }
        }
        // SETPKPOINT <points> —— 设置 PK 值（对齐 C# ActionType.SetPkPoint）
        "SETPKPOINT" => {
            let points = arg0().parse::<i32>().unwrap_or(0);
            send_player_msg(world, session_id, crate::actors::player::SetPkPoints { points }).await;
            debug!("NPC SETPKPOINT: {}", points);
        }
        // REDUCEPKPOINT <amount> —— 减少 PK 值（对齐 C# ActionType.ReducePkPoint）
        "REDUCEPKPOINT" => {
            let amount = arg0().parse::<i32>().unwrap_or(0);
            if amount > 0 {
                send_player_msg(world, session_id, crate::actors::player::ReducePkPoints { amount }).await;
                debug!("NPC REDUCEPKPOINT: -{}", amount);
            }
        }
        // GIVEGUILDGOLD <amount> —— 行会仓库增加金币（对齐 C# ActionType.GiveGuildGold）
        "GIVEGUILDGOLD" => {
            let amount = arg0().parse::<u32>().unwrap_or(0);
            if amount > 0 {
                let _ = world.social_ref.ask(crate::actors::social::NpcGuildGoldChange { session_id, amount, change_type: 3 }).await;
            }
        }
        // TAKEGUILDGOLD <amount> —— 行会仓库减少金币（对齐 C# ActionType.TakeGuildGold）
        "TAKEGUILDGOLD" => {
            let amount = arg0().parse::<u32>().unwrap_or(0);
            if amount > 0 {
                let _ = world.social_ref.ask(crate::actors::social::NpcGuildGoldChange { session_id, amount, change_type: 2 }).await;
            }
        }
        // INCREASEPKPOINT <amount> —— 增加 PK 值（对齐 C# ActionType.IncreasePkPoint）
        "INCREASEPKPOINT" | "ADDPKPOINT" => {
            let amount = arg0().parse::<i32>().unwrap_or(0);
            if amount > 0 {
                send_player_msg(world, session_id, crate::actors::player::AddPkPoints { points: amount }).await;
                debug!("NPC INCREASEPKPOINT: +{}", amount);
            }
        }
        // CALL <script_id> —— 调用另一个 NPC 脚本的 [@MAIN] 段（对齐 C# ActionType.Call + DelayedAction 立即）
        // 通过 ProcessDelayedActions 队列执行（避免 async 递归；C# 同样是 DelayedAction）
        "CALL" => {
            let script_id = arg0().parse::<i32>().unwrap_or(0);
            if script_id <= 0 {
                warn!("NPC CALL: invalid script id '{}'", arg0());
            } else if let Some(&npc_oid) = world.session_npc.get(&session_id) {
                world.npc_delayed_actions.entry(session_id).or_default().push(
                    crate::actors::world::DelayedNpcAction {
                        expire_tick: world.tick_count,
                        npc_object_id: npc_oid,
                        section: "main".to_string(),
                        target_db_index: Some(script_id),
                    },
                );
                debug!("NPC CALL: script {} [@MAIN] queued", script_id);
            } else {
                warn!("NPC CALL: no current NPC for session {}", session_id);
            }
        }
        // DROP <掉落表文件> —— 按 NPC 掉落表给玩家发奖励（对齐 C# ActionType.Drop + DropInfo.Load/AttemptDrop）
        "DROP" => {
            let file_path = arg0();
            if file_path.is_empty() {
                warn!("NPC DROP: missing file");
            } else {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                if path.starts_with(&base) {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            let drops = parse_drop_table(&content);
                            let mut given_gold = 0u64;
                            let mut given_items = 0usize;
                            for d in &drops {
                                // C# AttemptDrop：1/分母 概率
                                if fastrand::f64() > d.chance { continue; }
                                // GROUP 嵌套（C# GroupedDrop）：遍历子项各自概率，first/random/全部选择
                                if let Some(group) = &d.group {
                                    let mut hit_gold = 0u64;
                                    let mut hit_items: Vec<String> = Vec::new();
                                    for sub in &group.drops {
                                        if fastrand::f64() > sub.chance { continue; }
                                        if let Some(g) = sub.gold {
                                            let lo = g / 2;
                                            let hi = g + g / 2;
                                            hit_gold += if hi > lo { fastrand::u32(lo..=hi) } else { lo } as u64;
                                        }
                                        if let Some(item) = &sub.item_name {
                                            hit_items.push(item.clone());
                                            if group.first {
                                                break;
                                            }
                                        }
                                    }
                                    // GROUP*：随机选一个命中物品
                                    if group.random && hit_items.len() > 1 {
                                        let idx = fastrand::usize(0..hit_items.len());
                                        hit_items = vec![hit_items[idx].clone()];
                                    }
                                    if hit_gold > 0 {
                                        send_player_msg(world, session_id, AddGold { amount: hit_gold }).await;
                                        given_gold += hit_gold;
                                    }
                                    for item_name in &hit_items {
                                        if let Some(info) = world.item_infos.values().find(|i| i.name.eq_ignore_ascii_case(item_name)) {
                                            give_item(world, session_id, info.index, 1).await;
                                            given_items += 1;
                                        } else {
                                            warn!("NPC DROP: item '{}' not found", item_name);
                                        }
                                    }
                                    continue;
                                }
                                if let Some(gold) = d.gold {
                                    // C# 金币 0.5~1.5 倍随机
                                    let lo = gold / 2;
                                    let hi = gold + gold / 2;
                                    let amount = if hi > lo { fastrand::u32(lo..=hi) } else { lo };
                                    if amount > 0 {
                                        send_player_msg(world, session_id, AddGold { amount: amount as u64 }).await;
                                        given_gold += amount as u64;
                                    }
                                }
                                if let Some(item_name) = &d.item_name {
                                    if let Some(info) = world.item_infos.values().find(|i| i.name.eq_ignore_ascii_case(item_name)) {
                                        give_item(world, session_id, info.index, 1).await;
                                        given_items += 1;
                                    } else {
                                        warn!("NPC DROP: item '{}' not found", item_name);
                                    }
                                }
                            }
                            debug!("NPC DROP: {} ({} entries, gold={}, items={})", file_path, drops.len(), given_gold, given_items);
                        }
                        Err(e) => warn!("NPC DROP: failed {}: {}", path.display(), e),
                    }
                } else {
                    warn!("NPC DROP: path escape denied: {}", file_path);
                }
            }
        }
        // GIVEHP <amount> —— 恢复 HP（对齐 C# ActionType.GiveHP / ChangeHP）
        "GIVEHP" => {
            let amount = arg0().parse::<i32>().unwrap_or(0);
            if amount > 0 {
                send_player_msg(world, session_id, crate::actors::player::Heal { amount }).await;
                debug!("NPC GIVEHP: +{}", amount);
            }
        }
        // GIVEMP <amount> —— 恢复 MP（对齐 C# ActionType.GiveMP / ChangeMP）
        "GIVEMP" => {
            let amount = arg0().parse::<i32>().unwrap_or(0);
            if amount > 0 {
                send_player_msg(world, session_id, crate::actors::player::RestoreMp { amount }).await;
                debug!("NPC GIVEMP: +{}", amount);
            }
        }
        // CLEARPETS —— 清除所有宠物（对齐 C# ActionType.ClearPets）
        "CLEARPETS" => {
            if let Some(mut st) = current_player_state(world, session_id).await {
                if st.creature_log.active_creature.is_some() || !st.creature_log.owned_creatures.is_empty() {
                    st.creature_log.active_creature = None;
                    st.creature_log.owned_creatures.clear();
                    send_player_msg(world, session_id, crate::actors::player::SetCreature { creature_log: st.creature_log }).await;
                    debug!("NPC CLEARPETS: all pets cleared");
                }
            }
        }
        // GIVECREDIT <n> —— 增加账户积分（对齐 C# ActionType.GiveCredit）
        "GIVECREDIT" => {
            let amount = arg0().parse::<i64>().unwrap_or(0);
            if amount > 0 {
                world.npc_change_credit(session_id, amount).await;
            }
        }
        // TAKECREDIT <n> —— 减少账户积分（对齐 C# ActionType.TakeCredit，下限 0）
        "TAKECREDIT" => {
            let amount = arg0().parse::<i64>().unwrap_or(0);
            if amount > 0 {
                world.npc_change_credit(session_id, -amount).await;
            }
        }
        // GIVEPEARLS <n> —— 增加珍珠（对齐 C# ActionType.GivePearls）
        "GIVEPEARLS" => {
            let amount = arg0().parse::<u32>().unwrap_or(0);
            if amount > 0 {
                send_player_msg(world, session_id, crate::actors::player::GainPearls { amount }).await;
                debug!("NPC GIVEPEARLS: +{}", amount);
            }
        }
        // TAKEPEARLS <n> —— 减少珍珠（对齐 C# ActionType.TakePearls）
        "TAKEPEARLS" => {
            let amount = arg0().parse::<u32>().unwrap_or(0);
            if amount > 0 {
                send_player_msg(world, session_id, crate::actors::player::LosePearls { amount }).await;
                debug!("NPC TAKEPEARLS: -{}", amount);
            }
        }
        // REVIVEHERO —— 复活当前英雄（对齐 C# ActionType.ReviveHero）
        "REVIVEHERO" => {
            world.npc_revive_hero(session_id).await;
        }
        // SEALHERO —— 封印当前英雄（对齐 C# ActionType.SealHero）
        "SEALHERO" => {
            world.npc_seal_hero(session_id).await;
        }
        // CONQUESTSIEGE <index> <id> —— 生成攻城器（对齐 C# ActionType.ConquestSiege；id 忽略，每次生成新结构）
        "CONQUESTSIEGE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if index >= 0 {
                world.npc_spawn_siege_structure(session_id, index, crate::actors::world::conquest::SiegeStructureType::Catapult).await;
            }
        }
        // CONQUESTGUARD <index> <id> —— 生成守卫（对齐 C# ActionType.ConquestGuard；简化生成箭塔结构）
        "CONQUESTGUARD" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if index >= 0 {
                world.npc_spawn_siege_structure(session_id, index, crate::actors::world::conquest::SiegeStructureType::ArcherTower).await;
            }
        }
        // CONQUESTGATE <index> <id> —— 修复城门（对齐 C# ActionType.ConquestGate）
        "CONQUESTGATE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if index >= 0 && id >= 0 {
                world.npc_repair_siege_structure(session_id, index, id, crate::actors::world::conquest::SiegeStructureType::CastleGate).await;
            }
        }
        // CONQUESTWALL <index> <id> —— 修复城墙（对齐 C# ActionType.ConquestWall）
        "CONQUESTWALL" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if index >= 0 && id >= 0 {
                world.npc_repair_siege_structure(session_id, index, id, crate::actors::world::conquest::SiegeStructureType::Wall).await;
            }
        }
        // CONQUESTREPAIRALL <index> —— GM 修复全部（对齐 C# ActionType.ConquestRepairAll）
        "CONQUESTREPAIRALL" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if index >= 0 {
                world.npc_repair_all(session_id, index).await;
            }
        }
        // OPENGATE <index> <id> —— 打开城门（对齐 C# ActionType.OpenGate）
        "OPENGATE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if index >= 0 && id >= 0 {
                world.npc_open_close_gate(session_id, index, id, true).await;
            }
        }
        // CLOSEGATE <index> <id> —— 关闭城门（对齐 C# ActionType.CloseGate）
        "CLOSEGATE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let id = arg1().parse::<i32>().unwrap_or(-1);
            if index >= 0 && id >= 0 {
                world.npc_open_close_gate(session_id, index, id, false).await;
            }
        }
        // TAKECONQUESTGOLD <index> —— 所有者取走攻城金库（对齐 C# ActionType.TakeConquestGold）
        "TAKECONQUESTGOLD" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if index >= 0 {
                world.npc_take_conquest_gold(session_id, index).await;
            }
        }
        // SETCONQUESTRATE <index> <rate> —— 所有者设置税率（对齐 C# ActionType.SetConquestRate）
        "SETCONQUESTRATE" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            let rate = arg1().parse::<u8>().unwrap_or(0);
            if index >= 0 {
                world.npc_set_conquest_rate(session_id, index, rate).await;
            }
        }
        // STARTCONQUEST <index> —— 开/停战争（对齐 C# ActionType.StartConquest）
        "STARTCONQUEST" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if index >= 0 {
                world.npc_start_conquest(session_id, index).await;
            }
        }
        // SCHEDULECONQUEST <index> —— 宣战（对齐 C# ActionType.ScheduleConquest）
        "SCHEDULECONQUEST" => {
            let index = arg0().parse::<i32>().unwrap_or(-1);
            if index >= 0 {
                world.npc_schedule_conquest(session_id, index).await;
            }
        }
        // DELETEHERO —— 删除当前英雄（对齐 C# ActionType.DeleteHero）
        "DELETEHERO" => {
            world.npc_delete_hero(session_id).await;
        }
        // HEROGIVESKILL <技能名|ID> <level> —— 英雄学技能（对齐 C# ActionType.HeroGiveSkill）
        "HEROGIVESKILL" => {
            let skill_name = arg0();
            if let Some(spell) = resolve_magic_id(&world.magic_infos, skill_name) {
                let level = arg1().parse::<u8>().unwrap_or(0).min(3);
                send_player_msg(world, session_id, crate::actors::player::LearnHeroMagicWithLevel { spell, level }).await;
                debug!("NPC HEROGIVESKILL: spell={} level={}", spell, level);
            } else {
                warn!("NPC HEROGIVESKILL: unknown skill '{}'", skill_name);
            }
        }
        // HEROREMOVESKILL <技能名|ID> —— 移除英雄技能（对齐 C# ActionType.HeroRemoveSkill）
        "HEROREMOVESKILL" => {
            let skill_name = arg0();
            if let Some(spell) = resolve_magic_id(&world.magic_infos, skill_name) {
                send_player_msg(world, session_id, crate::actors::player::RemoveHeroMagicWithId { spell }).await;
                debug!("NPC HEROREMOVESKILL: spell={}", spell);
            } else {
                warn!("NPC HEROREMOVESKILL: unknown skill '{}'", skill_name);
            }
        }
        // CANGAINEXP <true|false> —— 设置是否可获得经验（对齐 C# ActionType.CanGainExp）
        "CANGAINEXP" => {
            let can = arg0().eq_ignore_ascii_case("true") || arg0() == "1";
            send_player_msg(world, session_id, crate::actors::player::SetCanGainExp { can }).await;
            debug!("NPC CANGAINEXP: {}", can);
        }
        // CHANGELEVEL <level> —— 设置等级（对齐 C# ActionType.ChangeLevel：设等级 + 经验 0 + LevelUp）
        "CHANGELEVEL" => {
            let level = arg0().parse::<u16>().unwrap_or(0);
            if level > 0 {
                send_player_msg(world, session_id, crate::actors::player::ChangeLevel { level }).await;
                debug!("NPC CHANGELEVEL: {}", level);
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
        // BUYGT —— 会长购买领地（对齐 C# ActionType.BuyGT）
        "BUYGT" => {
            world.npc_gt_buy(session_id).await;
        }
        // TELEPORTGT —— 传送到行会领地（对齐 C# ActionType.TeleportGT）
        "TELEPORTGT" => {
            world.npc_gt_teleport(session_id).await;
        }
        // EXTENDGT —— 会长延长领地租期（对齐 C# ActionType.ExtendGT）
        "EXTENDGT" => {
            world.npc_gt_extend(session_id).await;
        }
        // DISPLAYGTRENTALDAYS —— 显示领地剩余天数（对齐 C# ActionType.DisplayGTRentalDays）
        "DISPLAYGTRENTALDAYS" => {
            world.npc_gt_display_days(session_id).await;
        }
        // GTALLRECALL —— 会长召回全部在线成员（对齐 C# ActionType.GTAllRecall）
        "GTALLRECALL" => {
            world.npc_gt_recall_all(session_id).await;
        }
        // GTRECALL <name> —— 会长召回指定成员（对齐 C# ActionType.GTRecall）
        "GTRECALL" => {
            let member_name = unquote(arg0()).to_string();
            if !member_name.is_empty() {
                world.npc_gt_recall(session_id, &member_name).await;
            }
        }
        // GTSALE <price> —— 会长挂售领地（对齐 C# ActionType.GTSale）
        "GTSALE" => {
            let price = arg0().parse::<u64>().unwrap_or(0);
            if price > 0 {
                world.npc_gt_sale(session_id, price).await;
            }
        }
        // GTCANCELSALE —— 取消挂售（对齐 C# ActionType.GTCancelSale）
        "GTCANCELSALE" => {
            world.npc_gt_cancel_sale(session_id).await;
        }
        // ENTERMAP —— 传送到 NeedMove 暂存传送点（对齐 C# ActionType.EnterMap + NPCData["NPCMoveMap"]）
        "ENTERMAP" => {
            if let Some((map_index, x, y)) = world.session_last_movement.remove(&session_id) {
                teleport_player(world, session_id, map_index, x, y).await;
                debug!("NPC ENTERMAP: -> map {} ({},{})", map_index, x, y);
            } else {
                send_system_message(&world.gate_ref, session_id, "没有待传送的入口");
            }
        }
        // INSTANCEMOVE <map> <instance> <x> <y> —— 副本实例传送（对齐 C# ActionType.InstanceMove；
        // Rust 暂无独立副本实例，instance 忽略，等同传送到指定地图坐标）
        "INSTANCEMOVE" => {
            let map_name = arg0();
            let _instance = arg1().parse::<i32>().unwrap_or(0);
            let x = arg2().parse::<i32>().unwrap_or(0);
            let y = arg3().parse::<i32>().unwrap_or(0);
            if let Some(map_index) = world.map_infos.values()
                .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                .map(|m| m.index as u16)
            {
                teleport_player(world, session_id, map_index, x, y).await;
                debug!("NPC INSTANCEMOVE: map={} ({},{}) instance={}", map_name, x, y, _instance);
            } else {
                warn!("NPC INSTANCEMOVE: map '{}' not found", map_name);
            }
        }
        // RECALL —— 传送到当前 NPC 位置（对齐 mod.rs 旧处理器 RECALL）
        "RECALL" => {
            teleport_player(world, session_id, npc.map_index, npc.x, npc.y).await;
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
        // MONGEN <name> <count> —— 在目标坐标刷怪（坐标优先 PARAM2/PARAM3，缺省玩家/NPC 位置）
        "MONGEN" | "MAKEMON" | "MONSTER" => {
            let mob_name = arg0();
            let count = arg1().parse::<u32>().unwrap_or(1);
            if mob_name.is_empty() {
                warn!("NPC action MONGEN: missing monster name");
            } else {
                let player_state = current_player_state(world, session_id).await;
                let (tx, ty) = if flow.param2 > 0 && flow.param3 > 0 {
                    (flow.param2, flow.param3)
                } else if let Some(st) = &player_state {
                    (st.x, st.y)
                } else {
                    (npc.x, npc.y)
                };
                let map_index = if let Some(map_name) = &flow.map_name {
                    world.map_infos.values()
                        .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                        .map(|m| m.index as u16)
                        .unwrap_or(npc.map_index)
                } else if let Some(st) = &player_state {
                    st.map_index
                } else {
                    npc.map_index
                };
                let spawned = world.spawn_monster_named(mob_name, tx, ty, count, map_index).await;
                debug!("NPC MONGEN: '{}' x{} at ({},{}) map {} spawned={}", mob_name, count, tx, ty, map_index, spawned);
            }
        }
        // MONCLEAR <map名|index> —— 清除指定地图所有怪物（对齐 C# ActionType.MonClear）
        "MONCLEAR" | "MONCLEARALL" => {
            let map_ref = arg0();
            let map_index = if let Ok(idx) = map_ref.parse::<u16>() {
                idx
            } else {
                world.map_infos.values()
                    .find(|m| m.file_name.eq_ignore_ascii_case(map_ref))
                    .map(|m| m.index as u16)
                    .unwrap_or(0)
            };
            if map_index > 0 {
                let cleared = world.clear_monsters_on_map(map_index).await;
                debug!("NPC MONCLEAR: map {} cleared={}", map_index, cleared);
            } else {
                warn!("NPC MONCLEAR: map '{}' not found", map_ref);
            }
        }
        // GIVEBUFF <type> <duration_seconds> —— 给玩家加 Buff（对齐 C# ActionType.GiveBuff）
        "GIVEBUFF" => {
            let buff_name = arg0();
            let secs = arg1().parse::<u32>().unwrap_or(0);
            if let Some(bt) = parse_buff_type(buff_name) {
                if secs > 0 {
                    // 世界循环 100ms/tick：1 秒 = 10 ticks（对齐 C# Settings.Second * duration）
                    let ticks = secs.saturating_mul(10);
                    send_player_msg(world, session_id, crate::actors::player::ApplyBuff {
                        buff: crate::combat::buff::BuffInstance::new(bt, ticks, 1),
                    }).await;
                    debug!("NPC GIVEBUFF: '{}' {}s -> {} ticks", buff_name, secs, ticks);
                }
            } else {
                warn!("NPC GIVEBUFF: unknown buff type '{}'", buff_name);
            }
        }
        // REMOVEBUFF <type> —— 移除 Buff（对齐 C# ActionType.RemoveBuff）
        "REMOVEBUFF" => {
            let buff_name = arg0();
            if let Some(bt) = parse_buff_type(buff_name) {
                send_player_msg(world, session_id, crate::actors::player::RemoveBuff { buff_type: bt }).await;
                debug!("NPC REMOVEBUFF: '{}'", buff_name);
            } else {
                warn!("NPC REMOVEBUFF: unknown buff type '{}'", buff_name);
            }
        }
        // GIVESKILL <skill_name|spell_id> <level> —— 学技能（对齐 C# ActionType.GiveSkill，Level 最多 3）
        "GIVESKILL" => {
            let skill_name = arg0();
            if let Some(spell) = resolve_magic_id(&world.magic_infos, skill_name) {
                let level = arg1().parse::<u8>().unwrap_or(0).min(3);
                send_player_msg(world, session_id, crate::actors::player::LearnMagicWithLevel { spell, level }).await;
                debug!("NPC GIVESKILL: spell={} level={}", spell, level);
            } else {
                warn!("NPC GIVESKILL: unknown skill '{}'", skill_name);
            }
        }
        // REMOVESKILL <skill_name|spell_id> —— 移除技能（对齐 C# ActionType.RemoveSkill + S.RemoveMagic）
        "REMOVESKILL" => {
            let skill_name = arg0();
            if let Some(spell) = resolve_magic_id(&world.magic_infos, skill_name) {
                send_player_msg(world, session_id, crate::actors::player::RemoveMagicWithId { spell }).await;
                debug!("NPC REMOVESKILL: spell={}", spell);
            } else {
                warn!("NPC REMOVESKILL: unknown skill '{}'", skill_name);
            }
        }
        // UNEQUIPITEM [槽位名] —— 卸下装备（对齐 C# ActionType.UnequipItem：无参卸全部，指定槽位名卸单个）
        "UNEQUIPITEM" => {
            let slot_arg = arg0().to_lowercase();
            let mut slots = Vec::new();
            if slot_arg.is_empty() {
                for i in 0..crate::actors::inventory::EquipmentSlot::COUNT {
                    if let Some(slot) = crate::actors::inventory::EquipmentSlot::from_i32(i as i32) {
                        slots.push(slot);
                    }
                }
            } else if let Some(slot) = (0..crate::actors::inventory::EquipmentSlot::COUNT).find_map(|i| {
                let slot = crate::actors::inventory::EquipmentSlot::from_i32(i as i32)?;
                (format!("{:?}", slot).to_lowercase() == slot_arg).then_some(slot)
            }) {
                slots.push(slot);
            } else {
                warn!("NPC UNEQUIPITEM: unknown slot '{}'", arg0());
            }
            let mut removed_any = false;
            for slot in slots {
                if let Some(record) = world.players.get(&session_id) {
                    let ok = record
                        .actor_ref
                        .ask(crate::actors::player::InventoryUnequipItem { slot })
                        .await
                        .unwrap_or(false);
                    if ok {
                        removed_any = true;
                        if let Some(state) = world.recalculate_and_set_stat_bonuses(session_id).await {
                            world.broadcast_equipment_visuals(session_id, &state).await;
                        }
                    }
                }
            }
            debug!("NPC UNEQUIPITEM: session={} removed={}", session_id, removed_any);
        }
        // CHANGECLASS <Warrior|...>
        "CHANGECLASS" => {
            if let Some(cls) = parse_class(arg0()) {
                send_player_msg(world, session_id, ChangeClass { class: cls }).await;
            }
        }
        // GIVEPET <type_id> —— 给宠物（对齐 mod.rs 旧处理器 GIVEPET）
        "GIVEPET" => {
            let type_id = arg0().parse::<u8>().unwrap_or(0);
            let creature_type = crate::actors::creature::CreatureType::from(type_id);
            if creature_type != crate::actors::creature::CreatureType::None {
                if let Some(mut st) = current_player_state(world, session_id).await {
                    let mut log = st.creature_log;
                    let mut creature = crate::actors::creature::IntelligentCreature::new(creature_type);
                    creature.enabled = true;
                    log.set_creature(creature);
                    send_player_msg(world, session_id, crate::actors::player::SetCreature { creature_log: log }).await;
                    send_system_message(&world.gate_ref, session_id, "获得新宠物！");
                    debug!("NPC GIVEPET: type={:?}", creature_type);
                }
            }
        }
        // GIVEPETFOOD <amount> —— 恢复宠物饥饿（对齐 mod.rs 旧处理器 GIVEPETFOOD）
        "GIVEPETFOOD" => {
            let amount = arg0().parse::<u8>().unwrap_or(20);
            let restored = if let Some(record) = world.players.get(&session_id) {
                record.actor_ref.ask(crate::actors::player::RestoreCreatureHunger { amount }).await.unwrap_or(false)
            } else {
                false
            };
            if restored {
                send_system_message(&world.gate_ref, session_id, &format!("宠物吃了食物，饥饿值恢复 {} 点", amount));
            } else {
                send_system_message(&world.gate_ref, session_id, "你没有召唤宠物");
            }
        }
        // REMOVEPET [宠物名] —— 移除宠物（对齐 C# ActionType.RemovePet：按名字大小写不敏感匹配；
        // 无参时保留旧行为移除当前激活宠物；有参时先匹配 active，再匹配 owned_creatures）
        "REMOVEPET" => {
            if let Some(mut st) = current_player_state(world, session_id).await {
                let name = arg0().trim();
                let removed = if name.is_empty() {
                    st.creature_log.active_creature.take().is_some()
                } else {
                    let want = name.to_lowercase();
                    let pet_matches = |c: &crate::actors::creature::IntelligentCreature| {
                        c.custom_name.as_deref().map(|n| n.to_lowercase() == want).unwrap_or(false)
                            || format!("{:?}", c.creature_type).to_lowercase() == want
                    };
                    let active_removed = st.creature_log.active_creature.as_ref().map(|c| pet_matches(c)).unwrap_or(false);
                    if active_removed {
                        st.creature_log.active_creature = None;
                        true
                    } else {
                        let before = st.creature_log.owned_creatures.len();
                        st.creature_log.owned_creatures.retain(|c| !pet_matches(c));
                        st.creature_log.owned_creatures.len() < before
                    }
                };
                if removed {
                    send_player_msg(world, session_id, crate::actors::player::SetCreature { creature_log: st.creature_log }).await;
                    debug!("NPC REMOVEPET: pet removed (name='{}')", name);
                }
            }
        }
        // MAKEWEDDINGRING —— 制作结婚戒指（对齐 C# ActionType.MakeWeddingRing）
        "MAKEWEDDINGRING" => {
            let ok = if let Some(record) = world.players.get(&session_id) {
                record.actor_ref.ask(crate::actors::player::MakeWeddingRing).await.unwrap_or(false)
            } else {
                false
            };
            if !ok {
                send_system_message(&world.gate_ref, session_id, "需要已婚并佩戴未绑定的左戒指才能制作结婚戒指");
            }
        }
        // FORCEDIVORCE —— 强制离婚（对齐 C# ActionType.ForceDivorce）
        "FORCEDIVORCE" => {
            let _ = world.social_ref.ask(crate::actors::social::NpcForceDivorce { session_id }).await;
        }
        // CHANGEGENDER <male|female|0|1> —— 修改性别（对齐 C# ActionType.ChangeGender）
        "CHANGEGENDER" => {
            if let Some(gender) = parse_gender(arg0()) {
                send_player_msg(world, session_id, crate::actors::player::SetGender { gender }).await;
                debug!("NPC CHANGEGENDER: {:?}", gender);
            } else {
                warn!("NPC CHANGEGENDER: unknown gender '{}'", arg0());
            }
        }
        // CHANGEHAIR <style>
        "CHANGEHAIR" => {
            let h = arg0().parse::<u8>().unwrap_or(0);
            send_player_msg(world, session_id, SetHair { hair: h }).await;
        }
        // PLAYSOUND <sound_id> —— 播放声音（对齐 C# ActionType.PlaySound + S.PlaySound）
        "PLAYSOUND" => {
            let sound_id = arg0().parse::<i32>().unwrap_or(0);
            let packet = mir2_shared::packets::server::ui_events::PlaySound { sound_id };
            let mut body = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet).is_ok() {
                let _ = world.gate_ref.tell(SendToClient { session_id, data: body }).await;
                debug!("NPC PLAYSOUND: id={}", sound_id);
            }
        }
        // OPENBROWSER <url> —— 打开浏览器（对齐 C# ActionType.OpenBrowser + S.OpenBrowser）
        "OPENBROWSER" => {
            let url = unquote(arg0()).to_string();
            let packet = mir2_shared::packets::server::ui_events::OpenBrowser { url: url.clone() };
            let mut body = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet).is_ok() {
                let _ = world.gate_ref.tell(SendToClient { session_id, data: body }).await;
                debug!("NPC OPENBROWSER: {}", url);
            }
        }
        // GLOBALMESSAGE "msg" —— 全服广播（对齐 C# ActionType.GlobalMessage）
        "GLOBALMESSAGE" | "GLOBAL" => {
            let msg = unquote(arg0()).to_string();
            if !msg.is_empty() {
                broadcast_system_message(&world.gate_ref, &world.players, &msg);
            }
        }
        // LOCAL <msg> —— 本图系统消息广播（对齐 mod.rs 旧处理器 LOCAL）
        "LOCAL" => {
            let msg = unquote(arg0()).to_string();
            if !msg.is_empty() {
                if let Some(st) = current_player_state(world, session_id).await {
                    let map_index = st.map_index;
                    let mut targets = Vec::new();
                    for (sid, r) in &world.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.map_index == map_index {
                                targets.push(*sid);
                            }
                        }
                    }
                    for sid in targets {
                        send_system_message(&world.gate_ref, sid, &format!("[本地] {}", msg));
                    }
                }
            }
        }
        // LOCALMESSAGE "msg" <type>
        "LOCALMESSAGE" | "MESSAGE" | "SYSMSG" => {
            let msg = unquote(arg0()).to_string();
            let _kind = arg1().parse::<u8>().unwrap_or(0);
            send_system_message(&world.gate_ref, session_id, &msg);
        }
        // MAP <map_name> —— 设置 MONGEN 等指令的目标地图（对齐 C# Param1）
        "MAP" => {
            flow.map_name = Some(unquote(arg0()).to_string());
        }
        // PARAM1/PARAM2/PARAM3 <value> —— 脚本坐标参数（对齐 C# ActionType.Param1/2/3）
        "PARAM1" => {
            flow.param1 = arg0().parse::<i32>().unwrap_or(0);
        }
        "PARAM2" => {
            flow.param2 = arg0().parse::<i32>().unwrap_or(0);
        }
        "PARAM3" => {
            flow.param3 = arg0().parse::<i32>().unwrap_or(0);
        }
        // TIMERECALL <秒> [section] —— 延迟执行当前 NPC 脚本段（对齐 C# ActionType.TimeRecall + DelayedAction）
        "TIMERECALL" => {
            let secs = arg0().parse::<i64>().unwrap_or(0).max(0);
            let section = if arg1().is_empty() { "main".to_string() } else { arg1().to_string() };
            if let Some(&npc_oid) = world.session_npc.get(&session_id) {
                let expire_tick = world.tick_count.saturating_add(secs as u64 * 10);
                world.npc_delayed_actions.entry(session_id).or_default().push(
                    crate::actors::world::DelayedNpcAction { expire_tick, npc_object_id: npc_oid, section: section.clone(), target_db_index: None },
                );
                debug!("NPC TIMERECALL: session={} section='{}' in {}s (expire {})", session_id, section, secs, expire_tick);
            } else {
                warn!("NPC TIMERECALL: no current NPC for session {}", session_id);
            }
        }
        // TIMERECALLGROUP <秒> [section] —— 给所有组员注册延迟执行（对齐 C# ActionType.TimeRecallGroup）
        "TIMERECALLGROUP" => {
            let secs = arg0().parse::<i64>().unwrap_or(0).max(0);
            let section = if arg1().is_empty() { "main".to_string() } else { arg1().to_string() };
            if let Some(&npc_oid) = world.session_npc.get(&session_id) {
                let Some(st) = current_player_state(world, session_id).await else { return };
                let gid = st.group_id;
                let mut targets = vec![session_id];
                for (sid, r) in &world.players {
                    if *sid == session_id { continue; }
                    if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                        if os.group_id == gid {
                            targets.push(*sid);
                        }
                    }
                }
                let expire_tick = world.tick_count.saturating_add(secs as u64 * 10);
                for sid in &targets {
                    world.npc_delayed_actions.entry(*sid).or_default().push(
                        crate::actors::world::DelayedNpcAction { expire_tick, npc_object_id: npc_oid, section: section.clone(), target_db_index: None },
                    );
                }
                debug!("NPC TIMERECALLGROUP: {} players section='{}' in {}s", targets.len(), section, secs);
            }
        }
        // BREAKTIMERECALL —— 取消该玩家所有 NPC 延迟执行（对齐 C# ActionType.BreakTimeRecall）
        "BREAKTIMERECALL" => {
            world.npc_delayed_actions.remove(&session_id);
            debug!("NPC BREAKTIMERECALL: session={}", session_id);
        }
        // DELAYGOTO <秒> <section> —— 延迟跳转到脚本段（对齐 C# ActionType.DelayGoto）
        "DELAYGOTO" => {
            let secs = arg0().parse::<i64>().unwrap_or(0).max(0);
            let section = arg1().to_string();
            if section.is_empty() {
                warn!("NPC DELAYGOTO: missing section");
            } else if let Some(&npc_oid) = world.session_npc.get(&session_id) {
                let expire_tick = world.tick_count.saturating_add(secs as u64 * 10);
                world.npc_delayed_actions.entry(session_id).or_default().push(
                    crate::actors::world::DelayedNpcAction { expire_tick, npc_object_id: npc_oid, section, target_db_index: None },
                );
            }
        }
        // SETTIMER <key> <seconds> [type] [global] —— 注册计时器（对齐 C# ActionType.SetTimer）
        // C#：global=true 存 Envir.Timers["_-"+key]（全局共享、不发 S.SetTimer）；false 存玩家个人并发 S.SetTimer。
        // Rust：全局计时器存保留 session（GLOBAL_TIMER_SESSION），tick_npc_timers 正常到期清理。
        "SETTIMER" => {
            let key = arg0().parse::<i32>().unwrap_or(0);
            let secs = arg1().parse::<i64>().unwrap_or(0).max(0);
            let _kind = arg2().parse::<u8>().unwrap_or(0);
            let global = arg3().eq_ignore_ascii_case("true") || arg3() == "1";
            // 世界循环 100ms/tick：1 秒 = 10 ticks
            let expire_tick = world.tick_count.saturating_add(secs as u64 * 10);
            let owner = if global { GLOBAL_TIMER_SESSION } else { session_id };
            world.npc_timers.entry(owner).or_default().insert(key, expire_tick);
            if !global {
                let packet = mir2_shared::packets::server::ui_events::SetTimer {
                    timer_id: key,
                    seconds: secs as i32,
                };
                let mut body = Vec::new();
                if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet).is_ok() {
                    let _ = world.gate_ref.tell(SendToClient { session_id, data: body }).await;
                }
            }
            debug!("NPC SETTIMER: key={} {}s global={} (expire tick {})", key, secs, global, expire_tick);
        }
        // EXPIRETIMER <key> —— 移除计时器（对齐 C# ActionType.ExpireTimer：全局和个人都移除；发 S.ExpireTimer）
        "EXPIRETIMER" | "CLEARTIMER" => {
            let key = arg0().parse::<i32>().unwrap_or(0);
            if let Some(timers) = world.npc_timers.get_mut(&session_id) {
                timers.remove(&key);
            }
            if let Some(timers) = world.npc_timers.get_mut(&GLOBAL_TIMER_SESSION) {
                timers.remove(&key);
            }
            let packet = mir2_shared::packets::server::ui_events::ExpireTimer { timer_id: key };
            let mut body = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet).is_ok() {
                let _ = world.gate_ref.tell(SendToClient { session_id, data: body }).await;
            }
            debug!("NPC EXPIRETIMER: key={}", key);
        }
        // REFRESHEFFECTS —— 刷新等级特效（对齐 C# ActionType.RefreshEffects + S.ObjectLevelEffects）
        "REFRESHEFFECTS" => {
            if let Some(st) = current_player_state(world, session_id).await {
                let map_index = st.map_index;
                let packet = mir2_shared::packets::server::movement::ObjectLevelEffects {
                    object_id: st.object_id,
                    level_effects: 0, // Rust 端暂无 LevelEffects 计算（C# SetLevelEffects），先发 0 占位
                };
                let mut body = Vec::new();
                if mir2_shared::packets::base::serialize_packet(&mut std::io::Cursor::new(&mut body), &packet).is_ok() {
                    // 广播给同图玩家（对齐 C# player.Broadcast）
                    let mut targets = Vec::new();
                    for (sid, r) in &world.players {
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.map_index == map_index {
                                targets.push(*sid);
                            }
                        }
                    }
                    for sid in targets {
                        let _ = world.gate_ref.tell(SendToClient { session_id: sid, data: body.clone() }).await;
                    }
                }
            }
        }
        // GROUPGOTO <section> —— 组队跳转脚本段（对齐 C# ActionType.GroupGoto：DelayedAction 立即调度，所有组员到点执行）
        "GROUPGOTO" => {
            let section = arg0().to_string();
            if section.is_empty() {
                warn!("NPC GROUPGOTO: missing section");
            } else if let Some(&npc_oid) = world.session_npc.get(&session_id) {
                if let Some(st) = current_player_state(world, session_id).await {
                    let gid = st.group_id;
                    let mut targets = vec![session_id];
                    for (sid, r) in &world.players {
                        if *sid == session_id { continue; }
                        if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                            if os.group_id == gid {
                                targets.push(*sid);
                            }
                        }
                    }
                    // C# 用 Envir.Time（立即）；Rust 端下个 ProcessDelayedActions tick 执行
                    let expire_tick = world.tick_count;
                    for sid in &targets {
                        world.npc_delayed_actions.entry(*sid).or_default().push(
                            crate::actors::world::DelayedNpcAction { expire_tick, npc_object_id: npc_oid, section: section.clone(), target_db_index: None },
                        );
                    }
                    debug!("NPC GROUPGOTO: {} players section='{}'", targets.len(), section);
                }
            }
        }
        // GROUPTELEPORT <map> <x> <y> —— 组队传送（对齐 C# ActionType.GroupTeleport：目标地图+坐标；x/y 缺省用玩家位置）
        "GROUPTELEPORT" => {
            let map_name = arg0();
            let tx = arg2().parse::<i32>().unwrap_or(0);
            let ty = arg3().parse::<i32>().unwrap_or(0);
            let target_map = world.map_infos.values()
                .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
                .map(|m| m.index as u16);
            if let Some(map_index) = target_map {
                if let Some(st) = current_player_state(world, session_id).await {
                    let gid = st.group_id;
                    let (fx, fy) = if tx > 0 && ty > 0 { (tx, ty) } else { (st.x, st.y) };
                    // 先收集同组在线成员，避免借用冲突
                    let mut group_sessions = Vec::new();
                    for (sid, r) in &world.players {
                        if *sid == session_id { continue; }
                        if let Ok(Some(mst)) = r.actor_ref.ask(GetPlayerState).await {
                            if mst.group_id == gid {
                                group_sessions.push(*sid);
                            }
                        }
                    }
                    teleport_player(world, session_id, map_index, fx, fy).await;
                    for sid in &group_sessions {
                        teleport_player(world, *sid, map_index, fx, fy).await;
                    }
                    debug!("NPC GROUPTELEPORT: {} members -> map {} ({},{})", group_sessions.len() + 1, map_index, fx, fy);
                }
            } else {
                warn!("NPC GROUPTELEPORT: map '{}' not found", map_name);
            }
        }
        // ADDGUILD <行会名> —— 加入行会（对齐 C# ActionType.AddToGuild：自动接受邀请）
        "ADDGUILD" | "ADDTOGUILD" => {
            let guild_name = unquote(arg0()).to_string();
            if !guild_name.is_empty() {
                let _ = world.social_ref.ask(crate::actors::social::NpcAddToGuild { session_id, guild_name }).await;
            }
        }
        // REMOVEFROMGUILD —— 离开行会（对齐 C# ActionType.RemoveFromGuild）
        "REMOVEFROMGUILD" | "REMOVEGUILD" => {
            let _ = world.social_ref.ask(crate::actors::social::NpcRemoveFromGuild { session_id }).await;
        }
        // GROUPRECALL —— 组队召回（对齐 C# ActionType.GroupRecall：NPC 版无限制，直接召回组员到玩家位置）
        "GROUPRECALL" | "RECALLGROUP" => {
            let _ = world.social_ref.ask(crate::actors::social::NpcGroupRecall { session_id }).await;
        }
        // ADDNAMELIST <file> —— 玩家名追加到名单文件（对齐 C# ActionType.AddNameList）
        "ADDNAMELIST" => {
            let file_path = arg0();
            if file_path.is_empty() { warn!("NPC ADDNAMELIST: missing file"); }
            else if let Some(st) = current_player_state(world, session_id).await {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                if path.starts_with(&base) {
                    name_list_add(&path, &st.name);
                    debug!("NPC ADDNAMELIST: {} -> {}", st.name, file_path);
                } else {
                    warn!("NPC ADDNAMELIST: path escape denied: {}", file_path);
                }
            }
        }
        // DELNAMELIST <file> —— 从名单文件删除玩家名（对齐 C# ActionType.DelNameList）
        "DELNAMELIST" => {
            let file_path = arg0();
            if file_path.is_empty() { warn!("NPC DELNAMELIST: missing file"); }
            else if let Some(st) = current_player_state(world, session_id).await {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                if path.starts_with(&base) {
                    name_list_remove(&path, &st.name);
                    debug!("NPC DELNAMELIST: {} <- {}", st.name, file_path);
                } else {
                    warn!("NPC DELNAMELIST: path escape denied: {}", file_path);
                }
            }
        }
        // CLEARNAMELIST <file> —— 清空名单文件（对齐 C# ActionType.ClearNameList）
        "CLEARNAMELIST" => {
            let file_path = arg0();
            if file_path.is_empty() { warn!("NPC CLEARNAMELIST: missing file"); }
            else {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                if path.starts_with(&base) {
                    name_list_clear(&path);
                    debug!("NPC CLEARNAMELIST: {}", file_path);
                } else {
                    warn!("NPC CLEARNAMELIST: path escape denied: {}", file_path);
                }
            }
        }
        // ADDGUILDNAME <file> —— 行会名追加（对齐 C# ActionType.AddGuildNameList；需在行会）
        "ADDGUILDNAME" | "ADDGUILDNAMELIST" => {
            let file_path = arg0();
            if file_path.is_empty() { warn!("NPC ADDGUILDNAME: missing file"); }
            else if let Some(st) = current_player_state(world, session_id).await {
                if let Some(guild_name) = &st.guild_name {
                    let base = world.script_dir.clone();
                    let path = base.join(file_path);
                    if path.starts_with(&base) {
                        name_list_add(&path, guild_name);
                        debug!("NPC ADDGUILDNAME: {} -> {}", guild_name, file_path);
                    } else {
                        warn!("NPC ADDGUILDNAME: path escape denied: {}", file_path);
                    }
                } else {
                    send_system_message(&world.gate_ref, session_id, "你不在任何行会中");
                }
            }
        }
        // DELGUILDNAME <file> —— 从名单删除行会名（对齐 C# ActionType.DelGuildNameList）
        "DELGUILDNAME" => {
            let file_path = arg0();
            if file_path.is_empty() { warn!("NPC DELGUILDNAME: missing file"); }
            else if let Some(st) = current_player_state(world, session_id).await {
                if let Some(guild_name) = &st.guild_name {
                    let base = world.script_dir.clone();
                    let path = base.join(file_path);
                    if path.starts_with(&base) {
                        name_list_remove(&path, guild_name);
                        debug!("NPC DELGUILDNAME: {} <- {}", guild_name, file_path);
                    } else {
                        warn!("NPC DELGUILDNAME: path escape denied: {}", file_path);
                    }
                } else {
                    send_system_message(&world.gate_ref, session_id, "你不在任何行会中");
                }
            }
        }
        // CLEARGUILDNAME <file> —— 清空名单（对齐 C# ActionType.ClearGuildNameList；需在行会）
        "CLEARGUILDNAME" => {
            let file_path = arg0();
            if file_path.is_empty() { warn!("NPC CLEARGUILDNAME: missing file"); }
            else if let Some(st) = current_player_state(world, session_id).await {
                if st.guild_name.is_some() {
                    let base = world.script_dir.clone();
                    let path = base.join(file_path);
                    if path.starts_with(&base) {
                        name_list_clear(&path);
                        debug!("NPC CLEARGUILDNAME: {}", file_path);
                    } else {
                        warn!("NPC CLEARGUILDNAME: path escape denied: {}", file_path);
                    }
                } else {
                    send_system_message(&world.gate_ref, session_id, "你不在任何行会中");
                }
            }
        }
        // GETRANDOMTEXT <filePath> <变量名> —— 从文本文件随机选一行写入脚本变量（对齐 C# ActionType.GetRandomText）
        "GETRANDOMTEXT" => {
            let file_path = arg0();
            let var = normalize_custom_var(arg1());
            if file_path.is_empty() || var.is_empty() {
                warn!("NPC GETRANDOMTEXT: missing args (filePath/var)");
            } else {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                if path.starts_with(&base) {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
                            if !lines.is_empty() {
                                let idx = fastrand::usize(0..lines.len());
                                custom_vars.insert(var, lines[idx].to_string());
                                debug!("NPC GETRANDOMTEXT: {} -> line {}", file_path, idx);
                            }
                        }
                        Err(e) => warn!("NPC GETRANDOMTEXT: failed {}: {}", path.display(), e),
                    }
                } else {
                    warn!("NPC GETRANDOMTEXT: path escape denied: {}", file_path);
                }
            }
        }
        // SAVEVALUE <filePath> <header> <key> <value> —— 写 INI 全局变量（对齐 C# ActionType.SaveValue）
        "SAVEVALUE" => {
            let file_path = arg0();
            let header = arg1();
            let key = arg2();
            let value = unquote(arg3()).to_string();
            if file_path.is_empty() || header.is_empty() || key.is_empty() {
                warn!("NPC SAVEVALUE: missing args (filePath/header/key)");
            } else {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                if path.starts_with(&base) {
                    match ini_write(&path, header, key, &value) {
                        Ok(()) => debug!("NPC SAVEVALUE: {} [{}] {}={}", file_path, header, key, value),
                        Err(e) => warn!("NPC SAVEVALUE: failed {}: {}", path.display(), e),
                    }
                } else {
                    warn!("NPC SAVEVALUE: path escape denied: {}", file_path);
                }
            }
        }
        // LOADVALUE <变量名> <filePath> <header> <key> —— 读 INI 到脚本变量（对齐 C# ActionType.LoadValue）
        "LOADVALUE" => {
            let var = normalize_custom_var(arg0());
            let file_path = arg1();
            let header = arg2();
            let key = arg3();
            if var.is_empty() || file_path.is_empty() || header.is_empty() || key.is_empty() {
                warn!("NPC LOADVALUE: missing args (var/filePath/header/key)");
            } else {
                let base = world.script_dir.clone();
                let path = base.join(file_path);
                if path.starts_with(&base) {
                    if let Some(value) = ini_read(&path, header, key) {
                        custom_vars.insert(var, value);
                        debug!("NPC LOADVALUE: {} = {} ({} [{}] {})", arg0(), key, file_path, header, key);
                    }
                } else {
                    warn!("NPC LOADVALUE: path escape denied: {}", file_path);
                }
            }
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
        // COMPOSEMAIL "body" <sender> —— 创建邮件草稿（对齐 C# ComposeMail；配合 ADDMAILGOLD/ADDMAILITEM/SENDMAIL）
        "COMPOSEMAIL" => {
            let body = unquote(arg0()).to_string();
            let sender = if arg1().is_empty() { "系统".to_string() } else { arg1().to_string() };
            flow.mail = Some(crate::actors::mail::MailMessage {
                mail_id: crate::actors::mail::generate_mail_id(),
                sender_name: sender,
                receiver_name: String::new(),
                subject: "系统邮件".to_string(),
                body,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                read: false,
                collected: false,
                locked: false,
                gold: 0,
                items: Vec::new(),
            });
        }
        // ADDMAILGOLD <amount> —— 邮件附金币
        "ADDMAILGOLD" => {
            let amount = arg0().parse::<u64>().unwrap_or(0);
            if let Some(m) = flow.mail.as_mut() {
                m.gold = m.gold.saturating_add(amount);
            }
        }
        // ADDMAILITEM <item_name> <count> —— 邮件附物品（最多 5 个，按 stack_size 拆分，对齐 C# AddMailItem）
        "ADDMAILITEM" => {
            let item_name = arg0();
            let count = arg1().parse::<u16>().unwrap_or(1);
            if item_name.is_empty() {
                warn!("NPC ADDMAILITEM: missing item name");
            } else if let Some(m) = flow.mail.as_mut() {
                if m.items.len() >= 5 {
                    warn!("NPC ADDMAILITEM: mail attachments full (max 5)");
                } else if let Some(info) = world.item_infos.values().find(|i| i.name.eq_ignore_ascii_case(item_name)).cloned() {
                    let mut remaining = count;
                    let stack = info.stack_size.max(1) as u16;
                    while remaining > 0 && m.items.len() < 5 {
                        let take = remaining.min(stack);
                        remaining -= take;
                        m.items.push(mir2_shared::data::item::UserItem {
                            unique_id: crate::actors::inventory::generate_item_uid(),
                            item_index: info.index,
                            count: take,
                            current_dura: info.durability as u16,
                            max_dura: info.durability as u16,
                            identified: info.is_identified(),
                            ..Default::default()
                        });
                    }
                } else {
                    warn!("NPC ADDMAILITEM: item '{}' not found", item_name);
                }
            }
        }
        // SENDMAIL <recipient> —— 发送邮件草稿（对齐 C# SendMail；收件人为任意玩家名）
        "SENDMAIL" => {
            let recipient = arg0();
            if let Some(mut mail) = flow.mail.take() {
                if recipient.is_empty() {
                    send_system_message(&world.gate_ref, session_id, "SENDMAIL 缺少收件人");
                } else {
                    mail.receiver_name = recipient.to_string();
                    send_npc_mail(world, session_id, mail).await;
                }
            } else {
                send_system_message(&world.gate_ref, session_id, "请先用 COMPOSEMAIL 撰写邮件");
            }
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

/// CHECKTIMER 剩余秒数：无计时器视为 0（对齐 C# timer==null → remainingTime=0）；tick 100ms
fn npc_timer_remaining_secs(now_tick: u64, expire_tick: Option<u64>) -> i64 {
    match expire_tick {
        Some(exp) => exp.saturating_sub(now_tick) as i64 / 10,
        None => 0,
    }
}

/// 全局计时器保留 session（C# Envir.Timers 全局计时器；tick_npc_timers 按值清理）
const GLOBAL_TIMER_SESSION: u64 = u64::MAX;

/// 当前星期几（C# DayOfWeek 全名，大写；使用服务器本地时间 chrono::Local）
fn now_weekday_upper() -> String {
    use chrono::Datelike;
    match chrono::Local::now().weekday() {
        chrono::Weekday::Mon => "MONDAY",
        chrono::Weekday::Tue => "TUESDAY",
        chrono::Weekday::Wed => "WEDNESDAY",
        chrono::Weekday::Thu => "THURSDAY",
        chrono::Weekday::Fri => "FRIDAY",
        chrono::Weekday::Sat => "SATURDAY",
        chrono::Weekday::Sun => "SUNDAY",
    }.to_string()
}

/// 当前小时（本地时间）
fn now_hour() -> u32 {
    use chrono::Timelike;
    chrono::Local::now().hour()
}

/// 当前分钟（本地时间）
fn now_minute() -> u32 {
    use chrono::Timelike;
    chrono::Local::now().minute()
}

/// 地图名（file_name）→ map_index（大小写不敏感）
fn map_index_by_name(world: &WorldActor, map_name: &str) -> Option<u16> {
    world.map_infos.values()
        .find(|m| m.file_name.eq_ignore_ascii_case(map_name))
        .map(|m| m.index as u16)
}

/// 名单文件：是否包含指定行（精确匹配，对齐 C# CheckNameList Contains）
fn name_list_contains(path: &std::path::Path, name: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.lines().any(|l| l == name))
        .unwrap_or(false)
}

/// C# DropInfo.FromLine 解析后的掉落条目
#[derive(Debug, Clone, PartialEq)]
struct ParsedDrop {
    /// 命中概率（1/分母）
    chance: f64,
    /// 金币奖励（Gold 行）
    gold: Option<u32>,
    /// 物品名（Item 行，需再按名查 item_infos）
    item_name: Option<String>,
    /// GROUP 嵌套（C# GroupedDrop）
    group: Option<DropGroup>,
}

/// C# GroupDropInfo：一组子掉落（`{ }` 块内）
#[derive(Debug, Clone, PartialEq)]
struct DropGroup {
    /// `GROUP*`：从命中子项中随机选一个
    random: bool,
    /// `GROUP^`：首个命中的子项后停止
    first: bool,
    /// 子掉落列表
    drops: Vec<ParsedDrop>,
}

/// 解析单行掉落（C# DropInfo.FromLine）：`1/100 <物品名|Gold 金额|GROUP100[*|^]>`
fn parse_drop_line(line: &str) -> Option<ParsedDrop> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    // C# FromLine：parts[0].Substring(2) 去掉 "1/" 前缀取分母
    let denom = parts[0].strip_prefix("1/").or_else(|| parts[0].strip_prefix("1\\"))?;
    let denom = denom.parse::<f64>().ok()?;
    if denom <= 0.0 {
        return None;
    }
    let chance = (1.0 / denom).min(1.0);
    if parts[1].eq_ignore_ascii_case("Gold") {
        let gold = parts.get(2).and_then(|s| s.parse::<u32>().ok())?;
        if gold == 0 {
            return None;
        }
        Some(ParsedDrop { chance, gold: Some(gold), item_name: None, group: None })
    } else if parts[1].to_uppercase().starts_with("GROUP") {
        Some(ParsedDrop {
            chance,
            gold: None,
            item_name: None,
            group: Some(DropGroup {
                random: parts[1].ends_with('*'),
                first: parts[1].ends_with('^'),
                drops: Vec::new(),
            }),
        })
    } else {
        Some(ParsedDrop { chance, gold: None, item_name: Some(parts[1].to_string()), group: None })
    }
}

/// 解析 C# 掉落表文件（对齐 DropInfo.Load/FromLine + ParseGroup）：
/// 行格式 `1/100 <物品名|Gold 金额|GROUP100[*|^]>`；`;` 注释/空行跳过；
/// GROUP 行后 `{ 子行... }` 块解析为子掉落；无效行跳过
fn parse_drop_table(content: &str) -> Vec<ParsedDrop> {
    let lines: Vec<&str> = content.lines().collect();
    let mut drops = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim();
        i += 1;
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let Some(mut d) = parse_drop_line(line) else { continue };
        // GROUP 块：对齐 C# ParseGroup（`{` 后子行直到 `}`）
        if d.group.is_some() {
            // 找 `{`；无块则跳过该 GROUP 行且不消费后续行（对齐 C# 无 start 不填充）
            let start_idx = i;
            let mut found_open = false;
            while i < lines.len() {
                if lines[i].trim() == "{" {
                    found_open = true;
                    break;
                }
                i += 1;
            }
            if !found_open {
                i = start_idx;
                continue;
            }
            i += 1; // 跳过 {
            while i < lines.len() && lines[i].trim() != "}" {
                let sub = lines[i].trim();
                i += 1;
                if sub.is_empty() || sub.starts_with(';') {
                    continue;
                }
                if let Some(mut sub_drop) = parse_drop_line(sub) {
                    // 子项不支持再嵌套 GROUP（C# 同语义）
                    sub_drop.group = None;
                    if let Some(g) = d.group.as_mut() {
                        g.drops.push(sub_drop);
                    }
                }
            }
            if d.group.as_ref().map(|g| g.drops.is_empty()).unwrap_or(true) {
                continue; // 空组跳过
            }
        }
        drops.push(d);
    }
    drops
}

/// 名单文件：追加一行（不存在才加，对齐 C# AddNameList 精确匹配）
fn name_list_add(path: &std::path::Path, name: &str) {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    if existing.lines().any(|l| l == name) {
        return;
    }
    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(name);
    out.push('\n');
    let _ = std::fs::write(path, out);
}

/// 名单文件：删除匹配行（对齐 C# DelNameList）
fn name_list_remove(path: &std::path::Path, name: &str) {
    let Ok(content) = std::fs::read_to_string(path) else { return };
    let filtered: Vec<&str> = content.lines().filter(|l| *l != name).collect();
    let _ = std::fs::write(path, filtered.join("\n") + if filtered.is_empty() { "" } else { "\n" });
}

/// 名单文件：清空（对齐 C# ClearNameList）
fn name_list_clear(path: &std::path::Path) {
    let _ = std::fs::write(path, "");
}

/// 读取 INI 文件 [header] 下 key 的值（对齐 C# InIReader.ReadString；大小写不敏感）
fn ini_read(path: &std::path::Path, header: &str, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_header = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_header = line[1..line.len() - 1].eq_ignore_ascii_case(header);
        } else if in_header {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// 写入 INI 文件 [header] 下 key=value（存在则更新，否则在块内追加；块不存在则新建）
fn ini_write(path: &std::path::Path, header: &str, key: &str, value: &str) -> std::io::Result<()> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut idx = 0usize;
    while idx < lines.len() {
        let t = lines[idx].trim();
        if t.starts_with('[') && t.ends_with(']') {
            if t[1..t.len() - 1].eq_ignore_ascii_case(header) {
                idx += 1;
                while idx < lines.len() && !lines[idx].trim().starts_with('[') {
                    if let Some((k, _)) = lines[idx].split_once('=') {
                        if k.trim().eq_ignore_ascii_case(key) {
                            lines[idx] = format!("{}={}", key, value);
                            return std::fs::write(path, lines.join("\n"));
                        }
                    }
                    idx += 1;
                }
                lines.insert(idx, format!("{}={}", key, value));
                return std::fs::write(path, lines.join("\n"));
            }
        }
        idx += 1;
    }
    // header 不存在：追加新块
    let mut out = content;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("[{}]\n{}={}\n", header, key, value));
    std::fs::write(path, out)
}

/// 当前英雄（对齐 C# player.CurrentHero）：按 hero_index 从 player_heroes 查找
fn current_hero(world: &WorldActor, session_id: u64, player: &PlayerState) -> Option<HeroInfo> {
    world.player_heroes
        .get(&session_id)
        .and_then(|hs| hs.iter().find(|h| h.index as u8 == player.hero_index))
        .cloned()
}

/// CHANGEGENDER 性别解析：支持 male/female/0/1（大小写不敏感）
fn parse_gender(s: &str) -> Option<mir2_shared::enums::MirGender> {
    match s.trim().to_lowercase().as_str() {
        "male" | "0" => Some(mir2_shared::enums::MirGender::Male),
        "female" | "1" => Some(mir2_shared::enums::MirGender::Female),
        _ => None,
    }
}

/// GIVEBUFF buff 类型解析：支持 C# BuffType 枚举名（Hiding/SoulShield/...）+ Rust 变体名，大小写不敏感
fn parse_buff_type(s: &str) -> Option<crate::combat::buff::BuffType> {
    use crate::combat::buff::BuffType;
    match s.trim().to_uppercase().as_str() {
        "HPREGEN" => Some(BuffType::HpRegen { amount_per_tick: 0 }),
        "MPREGEN" => Some(BuffType::MpRegen { amount_per_tick: 0 }),
        "ATTACKBOOST" | "FURY" | "ATTACK" => Some(BuffType::AttackBoost { bonus: 0 }),
        "DEFENSEBOOST" | "DEFENSE" => Some(BuffType::DefenseBoost { bonus: 0 }),
        "ACDEFENSEBOOST" | "BLESSEDARMOUR" | "BLESSEDARMOR" => Some(BuffType::AcDefenseBoost { bonus: 0 }),
        "MACDEFENSEBOOST" | "SOULSHIELD" => Some(BuffType::MacDefenseBoost { bonus: 0 }),
        "DAMAGEREDUCTION" | "MAGICSHIELD" | "ELEMENTALBARRIER" => Some(BuffType::DamageReduction { percent: 0 }),
        "POISON" | "POISONSHOT" => Some(BuffType::Poison { damage_per_tick: 0 }),
        "SILENCE" => Some(BuffType::Silence),
        "STUN" => Some(BuffType::Stun),
        "INVISIBILITY" | "HIDING" | "MOONLIGHT" | "DARKBODY" => Some(BuffType::Invisibility),
        "ATTACKSPEEDBOOST" | "HASTE" => Some(BuffType::AttackSpeedBoost { percent: 0 }),
        "MOVESPEEDBOOST" | "SWIFTFEET" | "LIGHTBODY" => Some(BuffType::MoveSpeedBoost { percent: 0 }),
        "AGILITYBOOST" => Some(BuffType::AgilityBoost { bonus: 0 }),
        "CRITICALRATEBOOST" | "RAGE" => Some(BuffType::CriticalRateBoost { bonus: 0 }),
        "MPREGENBOOST" | "CONCENTRATION" => Some(BuffType::MpRegenBoost { bonus: 0 }),
        "MAXMPBOOST" | "MAGICBOOSTER" => Some(BuffType::MaxMpBoost { bonus: 0 }),
        "REFLECT" | "ENERGYSHIELD" | "COUNTERATTACK" => Some(BuffType::Reflect { percent: 0 }),
        "TAUNT" | "LIONROAR" => Some(BuffType::Taunt),
        // 法术批已实现的 C# BuffType 别名：ProtectionField/ImmortalSkin/UltimateEnhancer
        "PROTECTIONFIELD" => Some(BuffType::DamageReduction { percent: 0 }),
        "IMMORTALSKIN" => Some(BuffType::AcDefenseBoost { bonus: 0 }),
        "ULTIMATEENHANCER" => Some(BuffType::McBoost { bonus: 0 }),
        // C# Curse 降低目标输出，Rust 端用 Slow 近似负面效果
        "SLOW" | "CURSE" => Some(BuffType::Slow { percent: 0 }),
        "FROZEN" => Some(BuffType::Frozen),
        _ => None,
    }
}

/// GIVESKILL 技能解析：优先数字（C# spell id），否则按 magic_infos.name 大小写不敏感匹配
fn resolve_magic_id(magic_infos: &HashMap<u32, crate::db::MagicInfo>, s: &str) -> Option<i32> {
    let s = s.trim();
    if let Ok(id) = s.parse::<i32>() {
        return if magic_infos.contains_key(&(id as u32)) { Some(id) } else { None };
    }
    magic_infos.values()
        .find(|m| m.name.eq_ignore_ascii_case(s))
        .map(|m| m.spell)
}

/// SENDMAIL：投递邮件（在线直接 AddMail + 通知；离线落库，登录时读回）
async fn send_npc_mail(world: &WorldActor, session_id: u64, mail: crate::actors::mail::MailMessage) {
    if let Some(target) = world.find_session_by_name_ignore_case(&mail.receiver_name).await {
        if let Some(record) = world.players.get(&target) {
            let _ = record.actor_ref.ask(crate::actors::player::AddMail { mail: mail.clone() }).await;
            crate::actors::social_packets::send_mail_received_packet(&world.gate_ref, target, &mail);
            send_system_message(&world.gate_ref, session_id, "邮件已发送");
            debug!("NPC SENDMAIL delivered online: -> {}", mail.receiver_name);
            return;
        }
    }
    if let Err(e) = crate::db::insert_mail(&world.db_pool, &mail.receiver_name, &mail).await {
        warn!("NPC SENDMAIL: failed to save offline mail for {}: {}", mail.receiver_name, e);
        send_system_message(&world.gate_ref, session_id, "邮件发送失败，请稍后重试");
    } else {
        send_system_message(&world.gate_ref, session_id, "邮件已发送（玩家离线，将在登录时收到）");
    }
}

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

async fn take_item(world: &WorldActor, session_id: u64, item_index: i32, count: u16, min_dura: Option<u32>) {
    let Some(record) = world.players.get(&session_id) else { return };
    let _ = record
        .actor_ref
        .ask(RemoveItemByIndexWithDura { item_index, count, min_dura })
        .await;
}

async fn set_player_flag(world: &WorldActor, session_id: u64, key: String, val: i32) {
    let Some(record) = world.players.get(&session_id) else { return };
    if let Ok(Some(mut state)) = record.actor_ref.ask(GetPlayerState).await {
        state.flags.insert(key, val);
        let _ = record.actor_ref.ask(SetPlayerState { state }).await;
    }
}

/// 传送：MOVE map_index x y（pub(crate)：NPC 脚本 + 行会领地 TELEPORTGT 共用完整跨图逻辑）
pub(crate) async fn teleport_player(world: &mut WorldActor, session_id: u64, map_index: u16, x: i32, y: i32) {
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

    #[test]
    fn buff_type_parse_accepts_csharp_and_rust_names() {
        use crate::combat::buff::BuffType;
        assert!(matches!(parse_buff_type("Hiding"), Some(BuffType::Invisibility)));
        assert!(matches!(parse_buff_type("hiding"), Some(BuffType::Invisibility)));
        assert!(matches!(parse_buff_type("HpRegen"), Some(BuffType::HpRegen { .. })));
        assert!(matches!(parse_buff_type("POISON"), Some(BuffType::Poison { .. })));
        assert!(matches!(parse_buff_type("SoulShield"), Some(BuffType::MacDefenseBoost { .. })));
        assert!(matches!(parse_buff_type("SwiftFeet"), Some(BuffType::MoveSpeedBoost { .. })));
        assert!(parse_buff_type("NotABuff").is_none());
    }

    #[test]
    fn resolve_magic_id_accepts_numeric_id_and_name() {
        let mut infos = HashMap::new();
        infos.insert(4, crate::db::MagicInfo {
            name: "Fencing".to_string(),
            spell: 4,
            base_cost: 0,
            level_cost: 0,
            icon: 0,
            level1: 0,
            level2: 0,
            level3: 0,
            need1: 0,
            need2: 0,
            need3: 0,
            delay_base: 0,
            delay_reduction: 0,
            power_base: 0,
            power_bonus: 0,
            mpower_base: 0,
            mpower_bonus: 0,
            range: 0,
            multiplier_base: 0.0,
            multiplier_bonus: 0.0,
        });
        assert_eq!(resolve_magic_id(&infos, "4"), Some(4));
        assert_eq!(resolve_magic_id(&infos, "Fencing"), Some(4));
        assert_eq!(resolve_magic_id(&infos, "fencing"), Some(4));
        assert_eq!(resolve_magic_id(&infos, "99"), None);
        assert_eq!(resolve_magic_id(&infos, "FireBall"), None);
    }

    #[test]
    fn parse_gender_accepts_names_and_ids() {
        assert_eq!(parse_gender("male"), Some(mir2_shared::enums::MirGender::Male));
        assert_eq!(parse_gender("FEMALE"), Some(mir2_shared::enums::MirGender::Female));
        assert_eq!(parse_gender("0"), Some(mir2_shared::enums::MirGender::Male));
        assert_eq!(parse_gender("1"), Some(mir2_shared::enums::MirGender::Female));
        assert_eq!(parse_gender("x"), None);
    }

    #[test]
    fn timer_remaining_secs_handles_missing_and_expired() {
        // 无计时器 → 0
        assert_eq!(npc_timer_remaining_secs(100, None), 0);
        // 剩余 50 tick = 5 秒
        assert_eq!(npc_timer_remaining_secs(100, Some(150)), 5);
        // 已到期 → 0
        assert_eq!(npc_timer_remaining_secs(100, Some(90)), 0);
        // 正好到点 → 0
        assert_eq!(npc_timer_remaining_secs(100, Some(100)), 0);
    }

    #[test]
    fn ini_read_write_roundtrip() {
        let dir = std::env::temp_dir().join(format!("npc_ini_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.ini");
        let _ = std::fs::remove_file(&path);

        // 新建
        ini_write(&path, "Section", "Key", "v1").unwrap();
        assert_eq!(ini_read(&path, "section", "key").as_deref(), Some("v1"));
        // 更新
        ini_write(&path, "Section", "Key", "v2").unwrap();
        assert_eq!(ini_read(&path, "Section", "Key").as_deref(), Some("v2"));
        // 同块追加
        ini_write(&path, "Section", "Other", "x").unwrap();
        assert_eq!(ini_read(&path, "section", "other").as_deref(), Some("x"));
        // 新块追加
        ini_write(&path, "OtherSection", "K", "1").unwrap();
        assert_eq!(ini_read(&path, "othersection", "k").as_deref(), Some("1"));
        // 不存在的 key
        assert_eq!(ini_read(&path, "Section", "Nope"), None);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn name_list_add_remove_clear() {
        let dir = std::env::temp_dir().join(format!("npc_namelist_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("names.txt");
        let _ = std::fs::remove_file(&path);

        name_list_add(&path, "Alice");
        name_list_add(&path, "Bob");
        // 重复不加
        name_list_add(&path, "Alice");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().filter(|l| *l == "Alice").count(), 1);
        assert_eq!(content.lines().filter(|l| *l == "Bob").count(), 1);

        name_list_remove(&path, "Alice");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.lines().any(|l| l == "Alice"));
        assert!(content.lines().any(|l| l == "Bob"));

        name_list_clear(&path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn name_list_contains_matches_exact() {
        let dir = std::env::temp_dir().join(format!("npc_contains_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("list.txt");
        std::fs::write(&path, "Alice\nBob\n").unwrap();
        assert!(name_list_contains(&path, "Alice"));
        assert!(name_list_contains(&path, "Bob"));
        assert!(!name_list_contains(&path, "alice")); // C# 精确匹配
        assert!(!name_list_contains(&path, "Charlie"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn weekday_is_uppercase_weekday_name() {
        let w = now_weekday_upper();
        assert!(w == "MONDAY" || w == "TUESDAY" || w == "WEDNESDAY" || w == "THURSDAY"
            || w == "FRIDAY" || w == "SATURDAY" || w == "SUNDAY");
        assert!(now_hour() <= 23);
        assert!(now_minute() <= 59);
    }

    #[test]
    fn drop_table_parses_csharp_format() {
        let content = "; 注释\n\n1/100 金创药\n1/10 Gold 500\n1/2 GROUP100\ninvalid\n1/1000 强效金创药\n";
        let drops = parse_drop_table(content);
        assert_eq!(drops.len(), 3);
        assert!((drops[0].chance - 0.01).abs() < 1e-9);
        assert_eq!(drops[0].item_name.as_deref(), Some("金创药"));
        assert!((drops[1].chance - 0.1).abs() < 1e-9);
        assert_eq!(drops[1].gold, Some(500));
        assert_eq!(drops[1].item_name, None);
        // GROUP 无块跳过
        assert!((drops[2].chance - 0.001).abs() < 1e-9);
        assert_eq!(drops[2].item_name.as_deref(), Some("强效金创药"));
    }

    #[test]
    fn drop_table_parses_group_blocks() {
        let content = "1/100 GROUP100\n{\n1/10 金创药\n1/20 Gold 100\n}\n1/50 强效金创药\n";
        let drops = parse_drop_table(content);
        assert_eq!(drops.len(), 2);
        let g = drops[0].group.as_ref().unwrap();
        assert_eq!(g.drops.len(), 2);
        assert_eq!(g.drops[0].item_name.as_deref(), Some("金创药"));
        assert_eq!(g.drops[1].gold, Some(100));
        assert_eq!(drops[1].item_name.as_deref(), Some("强效金创药"));
    }

    #[test]
    fn drop_group_random_first_flags() {
        let content = "1/2 GROUP100*\n{\n1/10 A\n1/10 B\n}\n1/2 GROUP200^\n{\n1/10 C\n1/10 D\n}\n";
        let drops = parse_drop_table(content);
        assert_eq!(drops.len(), 2);
        assert!(drops[0].group.as_ref().unwrap().random);
        assert!(!drops[0].group.as_ref().unwrap().first);
        assert!(!drops[1].group.as_ref().unwrap().random);
        assert!(drops[1].group.as_ref().unwrap().first);
    }
}

