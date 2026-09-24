// ============================================================================
// control.rs 客户端内置控制接口（TCP JSON-RPC，供 MCP/agent 控制玩家）
// 监听 127.0.0.1:<端口>（--control-port 可配，默认 9000；双客户端并行验证时各用一端口），
// 每行一条 JSON-RPC。
//   move {dx,dy,run}     相对玩家瓦片偏移移动（dx/dy 为瓦片数）
//   screenshot {path}    保存当前帧截图
//   state {}             返回玩家位置/朝向
//   nearby {}            返回周围实体（含 object_id）
//   attack {object_id}   攻击指定对象
//   interact {object_id} 与指定 NPC 对话
//   cursor {x,y|clear}  注入/清除光标探针（悬停类系统读它；None=真实光标）
//   wheel {x,y,delta}   在 (x,y) 注入一行滚轮（UI 逻辑坐标，正=向下滚=offset 增）
//   scroll {}           返回全部 UiScrollList 真值：轨道矩形 x/y/w/h、列表矩形
//                       rx/ry/rw/rh（**滚轮命中用后者**）、offset/total/visible/
//                       step/z、shown。矩形口径与滚轮命中的绝对原点算法一致
//                       （theme::scroll_origin，沿 ChildOf 累加 Node.left/top），
//                       与 dialog_rect 的「布局后 ComputedNode」口径**不同**，勿混用
//   npc_call {object_id,key} 对 NPC 发 CallNPC(key)（e2e 页面跳转驱动；后台窗口无法注入鼠标）
//   pickup {object_id}  拾取指定地面物品
//   chat {message}    发送聊天/GM 命令（@MAKE 等）
//   dialog {kind,action?}  打开/关闭/切换对话框（默认 toggle；验收截图巡回用，#2586）
//   cursor {x,y} | {clear:true}  注入/清除「光标探针」（#2767：自动化环境 winit 收不到真实
//                               光标 → 悬停类系统用探针坐标驱动；`nearby` 的 vp 字段给出目标视口坐标）
//   player_menu {object_id}  以该玩家的视口坐标打开右键菜单（#2771：悬停 Hint 的实机验证入口）
//   quest_detail {quest_id[,top_line][,confirm]}  打开任务详情窗展示指定任务（#2801：等价于点
//                 任务日记「已接任务」行 + 点消息区滚动键 + 点取消键；`top_line` 指定消息区
//                 首行（翻页取证，与滚轮/滚动键同一状态字段）、`confirm=true` 弹取消询问框；
//                 quest_id<=0 = 关闭）
// ============================================================================

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::input::ButtonState;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::PrimaryWindow;
use crossbeam_channel::{bounded, Receiver, Sender};
use serde_json::{json, Value};

use crate::actor::{
    ActorAnim, GroundItem, LocalPlayer, Monster, MonsterName, NetObjectId, Npc, NpcName, Player,
    PlayerName,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::movement::{world_to_tile, LocalMove};
use crate::game::pathfinding;
use crate::game::player_control::ControlState;
use crate::map_renderer::{GameData, GameLibraries};
use crate::network::NetConnection;
use crate::scenes::AppState;

/// 控制命令（控制线程 → Bevy 主循环）
/// 解析 `attack_mode` RPC 的模式名（玩家验收能力，2026-09-22）。
///
/// 只接受下面写死的 6 个名字；其余一律 `None`（RPC 侧回 `error` 而不是**静默保持和平模式**）——
/// 静默回退会让"以为切了模式其实没切"重新变成不可见缺陷，正是 P1 难定位的原因之一。
/// `quest_probe` 的判据内核（纯函数，可单测）：只挑 `taken == true` 的条目。
///
/// 为什么单独抽出来：判据必须**来自状态**（`QuestLogState`），且对同一状态可重复读出同一结果。
/// 反例是曾被证伪的 `quest_detail {id}`——它对任意 id 都回 ok，用它当"条目数"会得到恒定值。
/// ⑤ 邮件仓库的判据内核（纯函数）：占用格数 = `Some` 的格数。
///
/// 判据来源是**状态**（`Inventory.items` / `StorageState.items`），不是 UI 计数标签；
/// 同一状态连读必须一致——空/非空两态都要成立（同 quest_probe 的仪器自检口径）。
pub fn used_slots<T>(items: &[Option<T>]) -> usize {
    items.iter().filter(|s| s.is_some()).count()
}

/// `nearby` 的扫描半径（像素）：缺省 600（与原硬编码一致）。
/// 非正数 / NaN / 无穷一律视作缺省——`radius=0` 会让夹具"一个实体都看不到"
/// 却看着像调用成功（假绿），比报错更难查。
pub fn parse_nearby_radius(raw: Option<f64>) -> f32 {
    match raw {
        Some(v) if v.is_finite() && v > 0.0 => v as f32,
        _ => 600.0,
    }
}

/// 构造发信包：收件人/正文/金币原样带过去，附件固定空、不贴票。
/// 单独抽成纯函数是为了给「⑤ 邮件闭环」留一条可单测的判据——
/// 实测踩过：动作 RPC 收下参数却把金币吞掉（发出去的信 collected 后收不到钱）。
pub fn build_send_mail(
    to: &str,
    message: &str,
    gold: u32,
) -> mir2_shared::packets::client::mail::SendMail {
    mir2_shared::packets::client::mail::SendMail {
        name: to.to_string(),
        message: message.to_string(),
        gold,
        items_idx: [0u64; 5],
        stamped: false,
    }
}

/// 构造交任务包：`quest_index` 与可选奖励下标**不得互换**（服务端按位置读字段：
/// `[quest_index i32][selected_item_index i32]`）。抽成纯函数是为了给 ④ 留一条可单测判据——
/// 写反了服务端会去交"下标那个任务"，症状是「交了个不相干的任务 / 报任务不存在」。
pub fn build_finish_quest(
    quest_index: i32,
    selected_item_index: i32,
) -> mir2_shared::packets::client::quest::FinishQuest {
    mir2_shared::packets::client::quest::FinishQuest {
        quest_index,
        selected_item_index,
    }
}

/// 构造购买包：`item_index` 走的是**商品行的 unique_id**（C# `BuyItem.ItemIndex = SelectedItem.UniqueID`；
/// 常规商店服务端把 unique_id 填成 item_index，二手货才是实例 id），数量原样带过。
/// 抽成纯函数是为了钉住「数量不会被吞成 1」——吞了会少扣钱少发货，日志却像成功。
pub fn build_buy_item(item_index: u64, count: u16) -> mir2_shared::packets::client::npc::BuyItem {
    mir2_shared::packets::client::npc::BuyItem {
        item_index,
        count,
        panel_type: mir2_shared::enums::PanelType::Buy,
    }
}

/// 构造出售包：与**背包里 Alt+左键快速出售**同一条路径（`dialogs/inventory.rs:2459`
/// 发 `SellItem{unique_id, count}`）——只多一层动作 RPC，便于验收夹具按状态判成交。
/// `unique_id` 必须是**背包实例的 unique_id**（不是 item_index：出售按实例定位、要拆堆叠/清实例）。
pub fn build_sell_item(unique_id: u64, count: u16) -> mir2_shared::packets::client::npc::SellItem {
    mir2_shared::packets::client::npc::SellItem { unique_id, count }
}

/// 构造丢弃包：与背包「拖出去 / 丢弃确认框 Yes」同一条路径
/// （`game/dialogs/inventory.rs` 发 `DropItem{unique_id,count,hero_inventory:false}`）。
///
/// 为什么需要它：① 战斗闭环的「掉落拾取」半边只靠怪物掉率（本库 Scarecrow 单杀命中掉落行的
/// 期望 ≈0.25、Deer 只有 2 行），判据会长期停在 `N/A`；② 丢弃是**真实玩家动作**，
/// 走它就能确定性验证「地面出现物品 → 拾取回包」这条链。
pub fn build_drop_item(unique_id: u64, count: u32) -> mir2_shared::packets::client::item::DropItem {
    mir2_shared::packets::client::item::DropItem {
        unique_id,
        count,
        hero_inventory: false,
    }
}

/// 已占用格列表 `(格号, 名称)`——存取闭环的夹具靠它拿到**准确的 From/To 格号**
/// （`StoreItem`/`TakeBackItem` 的 from/to 就是格号，猜格号会得到"回包 success 但两边都不动"）。
pub fn occupied_cells(
    items: &[Option<crate::game::dialogs::inventory::InvItem>],
) -> Vec<(usize, String)> {
    items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|it| (i, it.name.clone())))
        .collect()
}

/// 已占用格列表 `(格号, 名称, unique_id)`——出售判据需要**背包实例 unique_id**
/// （`C.SellItem.UniqueID` 按实例定位；拿 item_index 会卖错实例或找不到物品）。
/// unique_id 缺失/为 0 统一记 0，夹具据此跳过该格。
pub fn occupied_cells_with_uid(
    items: &[Option<crate::game::dialogs::inventory::InvItem>],
) -> Vec<(usize, String, u64)> {
    items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|it| (i, it.name.clone(), it.unique_id)))
        .collect()
}

/// 任务格已占用列表 `(格号, 名称, 数量)`——ItemTasks 任务的判据要落在**任务格**上：
/// 服务端 `Q`（任务物品掉落）直接把物品放进任务格并推进进度
/// （`world/mod.rs try_give_quest_item`），所以「任务物品到手」= 任务格里出现该物品。
pub fn quest_cells(
    items: &[Option<crate::game::dialogs::inventory::InvItem>],
) -> Vec<(usize, String, u16)> {
    items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|it| (i, it.name.clone(), it.count)))
        .collect()
}

/// 「客户端这一格 == 服务端权威那一格」——**只认瓦片相等**，`None`（还没收到过 UserLocation）
/// 一律算不同步。
///
/// 为什么要有这个判据（2026-09-24 实测）：客户端本地预测天然领先服务端一步（移动包在"到达
/// 那一步"时才发），而近战由服务端按「服务端玩家格 + 客户端发来的方向」结算
/// （`world/combat.rs`：`target_x = result.x + MON_DIR_DX[dir]`），方向又由客户端用**自己**的
/// 格差算 ⇒ 原点差一格，挥砍就落在空地上。夹具要"等同步再打"，就得有个不问猜测的状态源。
pub fn in_sync_with_server(client_tile: (i32, i32), server_tile: Option<(i32, i32)>) -> bool {
    matches!(server_tile, Some(s) if s == client_tile)
}

/// `C.Harvest` 构造（2026-09-24）：方向是唯一字段——`HarvestMonster.Harvest` 用它取"正前方 3×3"里的尸体。
pub fn build_harvest(direction: u8) -> mir2_shared::packets::client::combat::Harvest {
    use mir2_shared::packets::base::Packet as _;
    let _ = mir2_shared::packets::client::combat::Harvest::OPCODE;
    mir2_shared::packets::client::combat::Harvest {
        direction: mir2_shared::enums::MirDirection::try_from(direction % 8)
            .unwrap_or(mir2_shared::enums::MirDirection::Up),
    }
}

pub fn taken_quest_ids(entries: &[crate::game::dialogs::quest_log::QuestEntry]) -> Vec<i32> {
    entries.iter().filter(|e| e.taken).map(|e| e.id).collect()
}

fn parse_attack_mode(name: &str) -> Option<mir2_shared::enums::AttackMode> {
    use mir2_shared::enums::AttackMode;
    match name.trim().to_ascii_lowercase().as_str() {
        "peace" => Some(AttackMode::Peace),
        "group" => Some(AttackMode::Group),
        "guild" => Some(AttackMode::Guild),
        "enemy_guild" | "enemyguild" => Some(AttackMode::EnemyGuild),
        "red_brown" | "redbrown" => Some(AttackMode::RedBrown),
        "all" => Some(AttackMode::All),
        _ => None,
    }
}

enum ControlCommand {
    Move {
        dx: i32,
        dy: i32,
        run: bool,
    },
    /// 玩家视角验收能力：走到指定**世界坐标**（同 `nearby` 的 x/y）——内部用与服务端同款的
    /// `pathfinding::find_path` 生成 `LocalMove`，因此能绕开建筑，够到被挡住的 NPC/落点。
    /// （`Move` 只走单格方向；`pickup` 的寻路只服务掉落物。）
    WalkTo {
        x: f32,
        y: f32,
        run: bool,
    },
    /// 玩家视角验收能力：切换攻击模式（等价于 Ctrl+H 循环，但可直接指定）——
    /// 设 `AttackModeState` 并发 `ChangeAMode`，否则"和平"模式下打不死怪无法自动复现/验证。
    SetAttackMode {
        mode: mir2_shared::enums::AttackMode,
    },
    Screenshot {
        path: String,
    },
    GetState {
        reply: Sender<String>,
    },
    /// 玩家验收能力（2026-09-22）：只读战斗探针——把"这一次攻击到底有没有落到目标身上"
    /// 变成可读真值（锁定目标 / 距离 / 是否在射程 / 目标血条百分比 / 近期战斗事件流）。
    /// 存在理由：`nearby` 的成员变化分不清"目标离开视野"与"被打死"，实测因此误判过一次。
    /// 只读任务探针（2026-09-22）：暴露客户端侧「已接任务 id 列表 + 完成标记」，
    /// 判据取自状态而非 UI 代理量——实测 `quest_detail {id}` 对任意 id 都回 ok，
    /// 用它当「条目数」会得到恒定值（仪器无效），与 P1 的 nearby 成员变化同类。
    /// 接受任务（2026-09-22）：照 `auto/world.rs:122`/`:932` 的既有写法发 `C.AcceptQuest`。
    /// 真实签名是 `AcceptQuest { npc_index, quest_index }`——**需要 npc_index**，
    /// 这也是 `quest_detail {confirm:true}` 只改 UI 状态、`taken` 不变的原因（它不发这个包）。
    /// 只读背包探针（2026-09-22）：占用/总格数 + 重量（判据取状态）
    BagProbe {
        reply: Sender<String>,
    },
    /// 只读仓库探针：占用/总格数 + 可见性/页（判据取状态）
    StorageProbe {
        reply: Sender<String>,
    },
    /// 只读法术特效探针（2026-09-25）：当前存活的**渲染侧**特效实体读数
    /// （施法帧动画 `SpellFxAnim` + 施法/远程弹道 `SpellMissileAnim`）。
    ///
    /// 存在理由：owner 反馈「魔法效果完全不对」的修复（把染色白方块换成原版
    /// `Magic/Magic2/Magic3` 帧表）此前**只有单元测试钉表**，没有实机判据——
    /// 本探针把「渲染侧真正 spawn 的库/起始帧/帧数」暴露成可断言状态（不是日志文本）。
    SpellFxProbe {
        reply: Sender<String>,
    },
    /// 只读 NPC 窗探针（2026-09-23）：每行文本 + 每条**行内链接的精确命中矩形**。
    /// 存在理由：NPC 窗是自绘文本、行内链接形如 `<Access/@Storage> Storage`，
    /// 链接段只覆盖行首那几个字——夹具按"行中心/行右半"点会静默无反应（⑤ 开仓库栽在这里）。
    /// 与 `npc_ui_system` 的点击分发共用同一套几何度量（`npc::npc_link_targets`）。
    NpcRows {
        reply: Sender<String>,
    },
    AcceptQuest {
        npc_index: u32,
        quest_index: i32,
    },
    /// 只读 UI 探针（2026-09-23，P3-2 用）：列出**覆盖到该逻辑坐标**的所有 UI 节点，
    /// 带祖先链 / display / visibility / z。用途：界面瑕疵定位时直接问"这块像素是谁画的"，
    /// 而不是靠猜组件——P3-2（行会 NOTICE 页残留 Status 页黑区）就靠它定位绘制方。
    UiNodesAt {
        x: f32,
        y: f32,
        reply: Sender<String>,
    },
    /// ⑤ 存取动作（现成包 `C.StoreItem`=15 / `C.TakeBackItem`=16）：
    /// 等价于 C# 的「选中背包格 → 点仓库格」/反向，直接发生成包，
    /// 让存取闭环**不依赖窗口内格子像素定位**（与 `accept_quest` 同一模式）。
    StorageStore {
        from: i32,
        to: i32,
    },
    StorageTake {
        from: i32,
        to: i32,
    },
    /// ⑤ 邮件动作（现成包 `C.SendMail`/`C.ReadMail`/`C.CollectParcel`）：
    /// 与撰写/阅读窗点击路径发的是同一个包，把邮件闭环从「窗口内像素定位」里解耦。
    /// 注意：**不能给自己发**（原版 C# 规则，服务端 `mail.rs` 直接拒绝），
    /// 合法判据是 A→B→B 登录收取。
    MailSend {
        to: String,
        message: String,
        gold: u32,
    },
    MailRead {
        mail_id: u64,
    },
    MailCollect {
        mail_id: u64,
    },
    /// 只读邮件探针：客户端侧邮件列表/详情（判据取状态而非 UI 代理量）
    MailProbe {
        reply: Sender<String>,
    },
    /// ③ 只读商店探针：客户端侧商品行（item_index/unique_id/名称/价格/数量）
    NpcGoodsProbe {
        reply: Sender<String>,
    },
    /// 只读**游戏商城**探针（owner 队列 `shop-class-tabs`）：当前三段筛选状态
    /// （C# `ClassFilter`/`TypeFilter`/`SectionFilter`）+ 当前页真实展示的行
    /// （过滤口径与渲染共用 `filter_shop_items` ⇒ 判据取状态，不猜 UI）。
    ShopProbe {
        reply: Sender<String>,
    },
    /// ③ 购买动作（现成包 `C.BuyItem`）：与商品窗「购买」按钮同一路径。
    /// 服务端仍按原版校验：必须先打开购买页（`[@BUYSELL]/[@BUY]/...`）且商品在该 NPC 销售列表内。
    BuyItem {
        item_index: u64,
        count: u16,
    },
    /// ③ 出售动作（现成包 `C.SellItem`）：与背包里 **Alt+左键快速出售** 同一路径
    /// （`game/dialogs/inventory.rs` 在 `npc_goods.visible` 时发 `SellItem{unique_id,count}`）。
    /// 服务端仍按原版校验：必须先与买卖 NPC 对话且在其 DataRange(16) 内。
    SellItem {
        unique_id: u64,
        count: u16,
    },
    /// ⑨ 丢弃动作（现成包 `C.DropItem`）：与背包「拖出/丢弃确认框 Yes」同一路径。
    /// 用于让「地面掉落 → 拾取」这条链有**确定性**入口（怪物掉率是概率的，靠打怪等掉落
    /// 会让判据长期停在 N/A）。
    DropItem {
        unique_id: u64,
        count: u32,
    },
    /// ② 复活动作（现成包 `C.TownRevive`，空体）：与死亡提示框的「回城复活」按钮同一路径。
    /// 判据是状态翻转（dead→false、hp 0→>0、位置回到绑定点），不是"点了没报错"。
    TownRevive,
    /// ④ 任务闭环动作（现成包 `C.FinishQuest`）：交任务/领奖励。
    /// 与 `accept_quest` 同一模式——把「交任务」从任务日志窗的像素定位里解耦。
    /// 服务端仍按原版规则校验：进度必须满（无任务目标的任务视为**空进度=已完成**）、
    /// 有 finish NPC 链接时玩家必须在同图 DataRange(16) 内、背包要放得下物品奖励。
    FinishQuest {
        quest_index: i32,
        selected_item_index: i32,
    },
    QuestProbe {
        reply: Sender<String>,
    },
    CombatProbe {
        reply: Sender<String>,
    },
    /// #2961 项5 验收：注入一次滚轮（x,y 为 UI 逻辑坐标；delta 为行数，正=向下滚=offset 增）
    Wheel {
        x: f32,
        y: f32,
        delta: f32,
    },
    /// #2961 项5 验收：读全部滚动列表真值（轨道绝对矩形 + offset/total/visible/z）
    GetScroll {
        reply: Sender<String>,
    },
    /// 诊断：返回当前打开的对话框列表
    GetDialogs {
        reply: Sender<String>,
    },
    /// 诊断：返回当前 Visible 的 DialogRoot kind（含未 open 却可见的=泄漏）
    GetVisible {
        reply: Sender<String>,
    },
    Nearby {
        reply: Sender<String>,
        /// 扫描半径（像素）。默认 600 与原实现一致；⑮ 定位仓库 NPC 时用大半径一次列出全图实体。
        radius: f32,
    },
    Attack {
        object_id: u32,
    },
    Interact {
        object_id: u32,
    },
    /// e2e：直接对当前 NPC 发 CallNPC(key)——实机鼠标点击在后台窗口不可注入
    /// （winit 丢弃注入的 WM_MOUSEMOVE → cursor=None），页面跳转只能走 RPC 驱动。
    /// key 必须是 "[@SECTION]" 括号全格式（与实机点击路径 extract_npc_key 产物
    /// 一致；服务端/mock 按该格式匹配，裸 "main" 会静默无效）。
    /// 有意绕过 interact 路径的 C# 5 秒 NPCTime 冷却（player_control.rs
    /// npc_call_allowed）——e2e 驱动不受游戏节奏限制
    NpcCall {
        object_id: u32,
        key: String,
    },
    Pickup {
        object_id: u32,
    },
    /// 只读聊天探针（2026-09-24）：返回最近 N 行聊天/系统消息（文本 + 频道）。
    /// 存在理由：服务端**拒绝类反馈只走 S.Chat 系统消息**（"请到对应 NPC 处接取任务" / "该任务已完成" /
    /// "等级不足" / "附近没有可采集的猎物"…），日志里没有——没有它就只能靠猜（本轮 ④ 的 `accept_quest`
    /// 假红查了很久：RPC 回 `ok:true` 但服务端其实拒绝了）。
    ChatProbe {
        limit: usize,
        reply: Sender<String>,
    },
    /// 采集/剥皮（2026-09-24）：照 `C.Harvest` 发方向；可采集怪（HarvestMonster）的尸体必须走这条路
    /// 才能拿到产出——④ ItemTasks 的 Q 物品在可采集怪身上就靠它交付（详见 combat.rs `roll_harvest_drops`）。
    /// `direction = None` → 用客户端当前朝向。
    Harvest {
        direction: Option<u8>,
    },
    Chat {
        message: String,
    },
    Dialog {
        kind: DialogKind,
        action: DialogAction,
    },
    /// #2767：注入/清除光标探针（视口逻辑坐标 0..1024/0..768）
    Cursor {
        pos: Option<Vec2>,
        reply: Sender<String>,
    },
    /// #2771：打开玩家右键菜单（实机验证菜单 Hint；坐标取该玩家视口位置）
    PlayerMenu {
        object_id: u32,
        reply: Sender<String>,
    },
    /// #2801 单元②③：打开任务详情窗展示指定任务/分页首行/取消询问框（等价于点任务日记
    /// 「已接任务」行、点消息区滚动键、点取消键；自动化无光标时点击链路不可用，
    /// 见 `dialogs/quest_log.rs`）
    QuestDetail {
        quest_id: i32,
        top_line: usize,
        confirm: bool,
        reply: Sender<String>,
    },
    /// #2775：切换角色窗页（0=装备 1=状态 2=State 3=技能）——技能页 Hint 的实机验证入口
    CharPage {
        page: usize,
        reply: Sender<String>,
    },
    /// #2781：切换聊天窗口档位（0/1/2 → 4/7/11 行）——控制栏「大小」按钮的实机验证入口
    ChatSize {
        size: usize,
        reply: Sender<String>,
    },
    /// 合成鼠标点击（全 UI 交互验证）：Move→Press→(可选 drag_to Move)→Release
    /// 四帧注入，PointerInput（bevy_ui Interaction 按钮链路）+ MouseButtonInput
    /// （ButtonInput<MouseButton> 拖动链路）+ 窗口光标位置三通道同步注入
    Click {
        pos: Vec2,
        drag_to: Option<Vec2>,
        reply: Sender<String>,
    },
    /// 返回指定对话框根面板的屏幕矩形（逻辑坐标），供 click 计算点击点
    DialogRect {
        kind: DialogKind,
        reply: Sender<String>,
    },
    /// 诊断：inspect 全部 CloseButton 实体的组件清单（抓 Visibility 改写者）
    DiagCloseBtn,
}

/// dialog 命令的动作（#2586）
enum DialogAction {
    Open,
    Close,
    Toggle,
}

#[derive(Resource)]
struct ControlRx(Receiver<ControlCommand>);

/// diag_closebtn RPC 的一次性触发标记（exclusive 系统消费后移除）
#[derive(Resource)]
struct DiagCloseBtnReq;

/// #2767 光标探针：自动化环境（无焦点/共享桌面）里 `Window::cursor_position()` 不可用，
/// 悬停类系统改读这里注入的视口坐标；`None` = 用真实光标。
#[derive(Resource, Default)]
pub struct CursorProbe {
    pub pos: Option<Vec2>,
}

/// click RPC 的逐帧注入状态机：`drive_pending_click`（First 调度）每帧推进一步，
/// 保证消息在 PreUpdate 的 picking/input 消费前落位（同帧生效）。
#[derive(Resource)]
struct PendingClick {
    pos: Vec2,
    drag_to: Option<Vec2>,
    phase: u8,
    /// 完成/失败路径 take() 发送；Option 配合 Drop 兜底（见 impl Drop）
    reply: Option<Sender<String>>,
    reply_hits: Vec<String>,
}

/// #2956：任何丢弃路径（被新 click 覆盖、资源被移除）都给调用方一条终态回执——
/// 否则 RPC 侧 2s 超时拿到 `{}`，与「成功但无 hits」无法区分。
/// 回执通道 bounded(1) 且正常路径已 take()，try_send 永不阻塞游戏线程。
impl Drop for PendingClick {
    fn drop(&mut self) {
        if let Some(tx) = self.reply.take() {
            let _ = tx.try_send(json!({"ok": false, "error": "busy"}).to_string());
        }
    }
}

/// 悬停用的光标位置：探针优先，其次真实窗口光标（纯函数便于单测）。
pub fn resolve_cursor(probe: Option<Vec2>, window: Option<Vec2>) -> Option<Vec2> {
    probe.or(window)
}

/// 悬停/点击命中用的光标来源（探针优先，其次真实窗口光标）。
///
/// 打包成 `SystemParam` 是因为 NPC 对话系统本就顶在 16 参数上限上，再加参数编译不过。
/// 语义与 `resolve_cursor` 一致：**正常游玩**（无探针）读真实光标，行为不变；
/// **自动化**（click/cursor RPC 注入探针）时命中判定不再依赖"窗口有焦点"。
/// 为什么 NPC 窗非走这条路不可：它按行/按段自绘文本（不是 bevy_ui 按钮），
/// 命中判定自己读光标，所以 `Interaction`/`HoverMap` 那套注入**到不了它**。
#[derive(SystemParam)]
pub struct CursorSource<'w, 's> {
    probe: Res<'w, CursorProbe>,
    windows: Query<'w, 's, &'static Window>,
}

impl CursorSource<'_, '_> {
    /// 注入的光标优先；无探针时退回真实窗口光标（无窗口/鼠标在窗外 → None）。
    pub fn pos(&self) -> Option<Vec2> {
        let real = self.windows.single().ok().and_then(|w| w.cursor_position());
        resolve_cursor(self.probe.pos, real)
    }
}

/// #2767：控制接口用到的实体查询打包（原先 16 个系统参数已是 Bevy 上限，
/// 再加「光标探针 + 相机」就编译失败）。
#[derive(SystemParam)]
struct ControlQueries<'w, 's> {
    /// `combat_probe` 用：对象头顶血条百分比（由 `S.ObjectHealth` 写入 `ActorHp`）。
    /// 这是"攻击是否真的落到目标身上"的直接证据，不依赖视野成员变化。
    hp: Query<'w, 's, (&'static NetObjectId, &'static crate::game::combat::ActorHp)>,
    /// `state` 用：会话里的服务器权威位置留痕（`UserLocation`）——移动同步判据见
    /// `SessionState::last_server_position` 的注释。
    session: Res<'w, crate::network::SessionState>,
    /// `quest_probe` 用：客户端侧任务日记状态（已接/已完成标记）
    quest_log: Res<'w, crate::game::dialogs::quest_log::QuestLogState>,
    /// `chat_probe` 用：聊天过滤设置——`transparent`（C# Settings.TransparentChat）决定面板底色
    /// 是否半透明；owner 缺陷②的实机判据要读它，而 `apply_control_commands` 的参数表已到
    /// 16 个 SystemParam 上限（再多会因 `ObserverSystem` 实现上限编译失败），故挂在 `ControlQueries`。
    chat_filter: Res<'w, crate::game::chat::ChatFilter>,
    /// `chat_probe` 用：主窗口（逻辑尺寸 + `scale_factor`）。
    /// 截图落盘的是**物理像素**（窗口逻辑尺寸 × scale_factor），而 UI 命中/绘制用的是逻辑坐标——
    /// 像素判据（l5t 的"面板展开后是否不透明"）必须按这个比例换算，否则采样区根本不是面板。
    window: Query<'w, 's, &'static bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    /// ⑤ 探针用：本地玩家背包组件（占用/总格数、重量）
    bag: Query<'w, 's, &'static crate::game::player_state::Inventory, With<LocalPlayer>>,
    /// `combat_probe` 用：本地玩家状态标志——`auto_attack_system` 的 run_if 是
    /// `player_input_enabled`（= 非 dead/fishing/paralysis），命中该门控时攻击**一次都不会发**，
    /// 而日志里看不出任何异常（P1 实测踩到过）。
    flags: Query<'w, 's, &'static crate::game::player_state::StatusFlags, With<LocalPlayer>>,
    /// `combat_probe` 用（#3089 非空转判据）：已应用战斗事件计数（只读）。
    /// 放在 `ControlQueries` 里而不是 `apply_control_commands` 的参数表上——
    /// 那台系统已经是 16 个 SystemParam 的上限，多加一个会因 `ObserverSystem` 实现上限而编译失败。
    applied: Res<'w, crate::game::combat::RealHitProbe>,
    players: Query<
        'w,
        's,
        (Entity, &'static Transform, &'static ActorAnim),
        (With<LocalPlayer>, With<NetObjectId>),
    >,
    monsters: Query<
        'w,
        's,
        (
            &'static Transform,
            &'static MonsterName,
            &'static NetObjectId,
        ),
        (With<Monster>, Without<LocalPlayer>),
    >,
    npcs: Query<
        'w,
        's,
        (&'static Transform, &'static NpcName, &'static NetObjectId),
        (With<Npc>, Without<LocalPlayer>),
    >,
    others: Query<
        'w,
        's,
        (
            &'static Transform,
            &'static PlayerName,
            &'static NetObjectId,
        ),
        (With<Player>, Without<LocalPlayer>),
    >,
    items: Query<
        'w,
        's,
        (
            &'static Transform,
            &'static GroundItem,
            &'static NetObjectId,
        ),
        (With<GroundItem>, Without<LocalPlayer>),
    >,
    dialog_roots: Query<
        'w,
        's,
        (
            &'static DialogRoot,
            &'static Node,
            &'static Visibility,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    /// dialog_rect RPC：标准关闭钮定位（theme::CloseButton 标记 + 布局后矩形）
    close_buttons: Query<
        'w,
        's,
        (
            Entity,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
            Option<&'static Visibility>,
        ),
        With<crate::ui::theme::CloseButton>,
    >,
    /// dialog_rect RPC：关闭钮 → 根面板的祖先链（scroll RPC 的绝对原点累加也用它）
    child_of: Query<'w, 's, &'static ChildOf>,
    /// #2961 项5 验收：全部滚动列表；轨道矩形换算绝对坐标用（口径同
    /// `theme::scroll_list_ui_system` 的 origin()）
    scroll_lists: Query<
        'w,
        's,
        (
            Entity,
            &'static crate::ui::theme::UiScrollList,
            Option<&'static InheritedVisibility>,
        ),
        Without<crate::ui::theme::UiScrollThumb>,
    >,
    ui_nodes: Query<'w, 's, &'static Node, Without<crate::ui::theme::UiScrollThumb>>,
    /// `spell_fx_probe` 用：渲染侧存活的施法帧动画（库/起始帧/帧数/跟随对象）
    spell_fx: Query<'w, 's, &'static crate::game::spell_effects::SpellFxAnim>,
    /// `spell_fx_probe` 用：渲染侧存活的施法/远程弹道（库/起始帧/帧数）
    spell_missiles: Query<'w, 's, &'static crate::game::effects::SpellMissileAnim>,
    /// dialog_rect RPC：物理→逻辑坐标换算用的窗口 scale_factor
    primary_window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    /// dialog_rect 诊断：任意实体的 Visibility 读取（关闭钮祖先链诊断）
    all_visibility: Query<'w, 's, &'static Visibility>,
    /// #2791：`hero_manage` 是状态驱动窗（不经 `DialogManager.open`，见 dialogs/mod.rs
    /// 的 `DialogKind::HeroManage`），RPC 直接切 `HeroState.managing`
    hero: ResMut<'w, crate::game::dialogs::hero::HeroState>,
    /// #2801 单元②③：任务详情窗状态（`quest_detail` RPC 直接指定任务/分页首行/询问框）
    quest_detail: ResMut<'w, crate::game::dialogs::quest_log::QuestDetailState>,
    /// #2892 批C：`MirInputBox` 是状态驱动窗（服务端 `S.GuildNameRequest`/`S.GuildRequestWar`
    /// 打开），RPC 直接切 `InputBoxState.open` 以便实机取证
    input_box: ResMut<'w, crate::game::dialogs::input_box::InputBoxState>,
    /// Storage 窗由 `StorageState.visible`（服务端 `S.StorageOpened`）+ `DialogManager.open`
    /// 双门控（dialogs/storage.rs `storage_open`），RPC open/close 两边都要切
    storage: ResMut<'w, crate::game::dialogs::storage::StorageState>,
    map_cameras: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (With<Camera2d>, Without<crate::ui::sprite_ui::UiEntity>),
    >,
    /// `npc_rows` RPC + `npc_call`/`interact` 记账：NPC 窗文本状态（行文本 = 命中判定的输入；
    /// `npc_object_id` = 对话内选项点击发 CallNPC 的地址，必须在这里写入）
    npc_state: ResMut<'w, crate::game::dialogs::npc::NpcDialogState>,
    /// `npc_rows` RPC：渲染行实体 → 行原点（`Node.left/top`，与点击分发同一来源）。
    /// 走实体而不是重算布局：夹具算出的点击点必须与客户端自己的命中判定同源。
    npc_lines: Query<
        'w,
        's,
        (&'static crate::game::dialogs::npc::NpcLine, &'static Node),
        With<crate::game::dialogs::npc::NpcDialogWidget>,
    >,
    /// `ui_nodes_at` 只读探针：任意 UI 节点的布局矩形 + 显隐 + z（定位"谁画的这块像素"）
    ui_all: Query<
        'w,
        's,
        (
            Entity,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            Option<&'static Name>,
            Option<&'static Visibility>,
            Option<&'static Node>,
            Option<&'static ZIndex>,
            Option<&'static crate::game::dialogs::guild::GuildPageRoot>,
        ),
    >,
    /// `mail_probe` RPC：客户端侧邮件列表/详情（ReceiveMail 写入，判据取状态）
    mail: Res<'w, crate::game::dialogs::mail::MailState>,
    /// 本地玩家金币（`GoldGained`/`UserInformation` 写入）——邮件收取/交易类闭环的
    /// 判据就是它的 delta，读它比读 HUD 像素或 DB 落后值都可靠
    gold: Query<'w, 's, &'static crate::game::player_state::Gold, With<LocalPlayer>>,
    /// 本地玩家等级/经验（`UserInformation`/`ExpGained` 写入）——任务奖励判据的另一半
    progression: Query<'w, 's, &'static crate::game::player_state::Progression, With<LocalPlayer>>,
    /// `npc_goods_probe` RPC：客户端侧商品行（服务端 GoodsList 写入，判据取状态）
    goods: Res<'w, crate::game::dialogs::npc_goods::NpcGoodsState>,
    shop: Res<'w, crate::game::dialogs::game_shop::GameShopState>,
    /// `state` RPC：HP/死亡标志（复活闭环判据）
    vitals: Query<'w, 's, &'static crate::game::player_state::Vitals, With<LocalPlayer>>,
    state_flags: Query<'w, 's, &'static crate::game::player_state::StatusFlags, With<LocalPlayer>>,
}

/// 控制端口默认值（--control-port 未指定或非法时回退）
const DEFAULT_CONTROL_PORT: u16 = 9000;

/// 解析 --control-port <u16>：合法值用之；缺省/非法（非数字、超范围、缺参数）
/// warn 并回退 9000。纯函数便于单测；跟随仓库分散 env::args 解析风格
/// （比照 network/mod.rs resolve_net_mode、auto/navigation.rs --e2e-user）。
fn parse_control_port(args: &[String]) -> u16 {
    let Some(i) = args.iter().position(|a| a == "--control-port") else {
        return DEFAULT_CONTROL_PORT;
    };
    let Some(raw) = args.get(i + 1) else {
        tracing::warn!("[control] --control-port 缺参数，回退 {DEFAULT_CONTROL_PORT}");
        return DEFAULT_CONTROL_PORT;
    };
    match raw.parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            tracing::warn!("[control] --control-port 非法值 {raw:?}，回退 {DEFAULT_CONTROL_PORT}");
            DEFAULT_CONTROL_PORT
        }
    }
}

pub struct ControlPlugin;

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = bounded::<ControlCommand>(64);
        app.insert_resource(ControlRx(rx));
        // #2767：光标探针（悬停类系统的自动化入口）
        app.init_resource::<CursorProbe>();
        let port = parse_control_port(&std::env::args().collect::<Vec<_>>());
        std::thread::spawn(move || control_listener(tx, port));
        app.add_systems(
            Update,
            apply_control_commands.run_if(in_state(AppState::Game)),
        );
        // #2956：非 Game 态到达的命令立即回错排空——否则滞留 channel，
        // 进 Game 后对已完全不同的场景按旧坐标补点
        app.add_systems(
            First,
            drain_control_outside_game.run_if(bevy::prelude::not(in_state(AppState::Game))),
        );
        // 合成点击驱动：First 调度，抢在 PreUpdate picking/input 消费前写消息；
        // 不加 run_if——资源不在即空转。注意 #2956：非 Game 态的新 click 命令在
        // First 就被 drain_control_outside_game 回 not in game 排空，到不了这里；
        // 本系统跨状态存活的只有「Game 内创建、状态切换时在途」的 click。
        app.add_systems(First, drive_pending_click);
        // diag_closebtn：exclusive inspect（组件清单含写入者特征 marker）
        app.add_systems(First, diag_closebtn_inspect.after(drive_pending_click));
        #[cfg(debug_assertions)]
        {
            app.add_systems(PreUpdate, closebtn_vis_change_watch_pre);
            app.add_systems(PostUpdate, closebtn_vis_change_watch_post);
            app.add_systems(PostUpdate, vis_batch_watch_post);
        }
        // 注（2026-09-24 移除）：这里曾注册过一个 debug-only 的 `Visibility` on_insert 诊断钩子
        // （抓「关闭钮 Visibility 被谁写入」，每次都 `Backtrace::force_capture()` 打全文回溯）。
        // 它跑在 UI 实体 spawn 的命令应用路径上（`spawn_at_with_caller → trigger_on_insert`），
        // 实机上层**间歇 panic**：stack 结束在这条 closure 里，进程直接退出 ⇒ 客户端进游戏即崩、
        // 没有本地玩家（夹具侧看到 `state`/`bag_probe` 全空）。同时它给每次进场刷 396KB 日志。
        // 关闭钮的可见性问题早已在其它 PR 修掉，这条诊断不必再常驻；门禁见
        // `spawning_close_button_ui_does_not_panic_via_component_hooks`。
    }
}

fn control_listener(tx: Sender<ControlCommand>, port: u16) {
    let addr = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("[control] 绑定 {addr} 失败: {e}");
            return;
        }
    };
    tracing::info!("[control] 监听 {addr}");
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let tx = tx.clone();
        std::thread::spawn(move || handle_conn(stream, tx));
    }
}

fn handle_conn(mut stream: std::net::TcpStream, tx: Sender<ControlCommand>) {
    let reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(line) else {
            let _ = writeln!(stream, "{{\"jsonrpc\":\"2.0\",\"error\":\"parse\"}}");
            continue;
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

        let result: Value = match method {
            "move" => {
                let dx = params.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = params.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let run = params.get("run").and_then(|v| v.as_bool()).unwrap_or(true);
                let _ = tx.send(ControlCommand::Move { dx, dy, run });
                json!({"ok": true})
            }
            "walk_to" => {
                // 玩家验收能力：走到世界坐标 {x,y}；也接受瓦片坐标 {tx,ty}。
                // 瓦片→世界的换算**必须**用 `movement::tile_to_world`（它才是
                // `world_to_tile` 的真逆变换：x = tx*W + W/2、y = -(ty*H + H)）。
                // 2026-09-23 修：此前这里硬编码 `(tx*48, ty*48)`，两个轴都不对
                // （x 差半个格、y 连符号都反）——实测 `tx=178,ty=221` 被算成目标瓦片
                // `(178,−333)`，于是 walk_to 的瓦片入口形同虚设，只能传世界像素。
                let world = walk_world_from_params(&params);
                match world {
                    Some((x, y)) => {
                        let run = params.get("run").and_then(|v| v.as_bool()).unwrap_or(true);
                        let _ = tx.send(ControlCommand::WalkTo { x, y, run });
                        json!({"ok": true, "x": x, "y": y})
                    }
                    None => json!({"error": "missing x/y (or tx/ty)"}),
                }
            }
            "attack_mode" => {
                let name = params.get("mode").and_then(|v| v.as_str()).unwrap_or("");
                match parse_attack_mode(name) {
                    Some(mode) => {
                        let _ = tx.send(ControlCommand::SetAttackMode { mode });
                        json!({"ok": true, "mode": format!("{mode:?}")})
                    }
                    None => json!({
                        "error": "unknown attack mode",
                        "accepted": ["peace", "group", "guild", "enemy_guild", "red_brown", "all"]
                    }),
                }
            }
            "screenshot" => {
                let path = params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("../tools/ctrl_shot.png")
                    .to_string();
                let _ = tx.send(ControlCommand::Screenshot { path });
                json!({"ok": true})
            }
            "nearby" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                let radius = parse_nearby_radius(params.get("radius").and_then(|v| v.as_f64()));
                if tx
                    .send(ControlCommand::Nearby {
                        reply: reply_tx,
                        radius,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "dialogs" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::GetDialogs { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "visible" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::GetVisible { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    json!({"visible": s})
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "combat_probe" => {
                // 只读：锁定目标 / 距离 / 血条百分比 / 近期战斗事件（判据来自服务端事件流）
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::CombatProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "chat_probe" => {
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::ChatProbe {
                        limit,
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "bag_probe" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::BagProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "spell_fx_probe" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::SpellFxProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            // 只读 UI 探针：列出覆盖该逻辑坐标的所有 UI 节点（P3-2 界面定位用）
            "ui_nodes_at" => {
                let x = params.get("x").and_then(|v| v.as_f64()).unwrap_or(-1.0) as f32;
                let y = params.get("y").and_then(|v| v.as_f64()).unwrap_or(-1.0) as f32;
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::UiNodesAt {
                        x,
                        y,
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "storage_probe" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::StorageProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "npc_rows" => {
                // NPC 窗每行文本 + 行内链接的精确命中矩形（含建议点击点 cx/cy）
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx.send(ControlCommand::NpcRows { reply: reply_tx }).is_ok() {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            // ⑤ 存取动作：`from`/`to` 都是**格号**（背包格号 ↔ 仓库格号，见 bag_probe/
            // storage_probe 的 `occupied`）。等价于 C# 的「选中背包格 → 点仓库格」
            // 与其反向，直接发 `C.StoreItem`/`C.TakeBackItem`——把存取闭环从
            // 「窗口内格子像素定位」里解耦出来（与 accept_quest 同一模式）。
            "storage_store" | "storage_take" => {
                let from = params.get("from").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
                let to = params.get("to").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
                if from < 0 || to < 0 {
                    json!({"error": "missing from/to（格号，需 >= 0）"})
                } else if method == "storage_store" {
                    let _ = tx.send(ControlCommand::StorageStore { from, to });
                    json!({"ok": true, "action": "store", "from": from, "to": to})
                } else {
                    let _ = tx.send(ControlCommand::StorageTake { from, to });
                    json!({"ok": true, "action": "take", "from": from, "to": to})
                }
            }
            // ⑤ 邮件动作：mail_send {to,message,gold} / mail_read {mail_id} /
            // mail_collect {mail_id}——与撰写/阅读窗点击路径发同一个包。
            "mail_send" => {
                let to = params
                    .get("to")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let message = params
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let gold = params.get("gold").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                if to.is_empty() {
                    json!({"error": "missing to（收件人角色名；原版规则：不能给自己发）"})
                } else {
                    // 回执要用收件人名，故先克隆一份进命令
                    let _ = tx.send(ControlCommand::MailSend {
                        to: to.clone(),
                        message,
                        gold,
                    });
                    json!({"ok": true, "action": "send", "to": to, "gold": gold})
                }
            }
            "mail_read" | "mail_collect" => {
                let mail_id = params.get("mail_id").and_then(|v| v.as_u64()).unwrap_or(0);
                if mail_id == 0 {
                    json!({"error": "missing mail_id"})
                } else if method == "mail_read" {
                    let _ = tx.send(ControlCommand::MailRead { mail_id });
                    json!({"ok": true, "action": "read", "mail_id": mail_id})
                } else {
                    let _ = tx.send(ControlCommand::MailCollect { mail_id });
                    json!({"ok": true, "action": "collect", "mail_id": mail_id})
                }
            }
            "mail_probe" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::MailProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            // ② 回城复活（现成包 C.TownRevive）：与死亡提示框的「回城复活」按钮同一路径
            "revive_town" => {
                let _ = tx.send(ControlCommand::TownRevive);
                json!({"ok": true, "action": "town_revive"})
            }
            // ③ 商店：npc_goods_probe（只读商品行）/ buy_item {item_index, count}
            "npc_goods_probe" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::NpcGoodsProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            // 商城三段筛选（owner 队列 `shop-class-tabs`）：只读回当前筛选 + 当前页真实行
            "shop_probe" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::ShopProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "buy_item" => {
                let item_index = params
                    .get("item_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u16;
                if item_index == 0 {
                    json!({"error": "missing item_index（商品行的 unique_id；常规商店 = item_index）"})
                } else {
                    let _ = tx.send(ControlCommand::BuyItem { item_index, count });
                    json!({"ok": true, "item_index": item_index, "count": count})
                }
            }
            "sell_item" => {
                let unique_id = params
                    .get("unique_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u16;
                if unique_id == 0 {
                    json!({"error": "missing unique_id（背包实例的 unique_id，非 item_index）"})
                } else {
                    let _ = tx.send(ControlCommand::SellItem { unique_id, count });
                    json!({"ok": true, "unique_id": unique_id, "count": count})
                }
            }
            "drop_item" => {
                let unique_id = params
                    .get("unique_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                if unique_id == 0 {
                    json!({"error": "missing unique_id（背包实例的 unique_id）"})
                } else {
                    let _ = tx.send(ControlCommand::DropItem { unique_id, count });
                    json!({"ok": true, "unique_id": unique_id, "count": count})
                }
            }
            // ④ 交任务：finish_quest {quest_index, selected_item_index?（默认 -1 = 不选奖励）}
            "finish_quest" => {
                let quest_index = params
                    .get("quest_index")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let selected_item_index = params
                    .get("selected_item_index")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1) as i32;
                if quest_index <= 0 {
                    json!({"error": "missing quest_index"})
                } else {
                    let _ = tx.send(ControlCommand::FinishQuest {
                        quest_index,
                        selected_item_index,
                    });
                    json!({"ok": true, "quest_index": quest_index, "selected_item_index": selected_item_index})
                }
            }
            "accept_quest" => {
                let npc_index = params
                    .get("npc_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let quest_index = params
                    .get("quest_index")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                if quest_index <= 0 {
                    json!({"error": "missing quest_index"})
                } else {
                    let _ = tx.send(ControlCommand::AcceptQuest {
                        npc_index,
                        quest_index,
                    });
                    json!({"ok": true, "npc_index": npc_index, "quest_index": quest_index})
                }
            }
            "quest_probe" => {
                // 只读：已接任务列表（状态判据，非 UI 代理量）
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::QuestProbe { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "state" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::GetState { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "chat" => {
                let message = params
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !message.is_empty() {
                    let _ = tx.send(ControlCommand::Chat { message });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing message"})
                }
            }
            "cursor" => {
                // #2767：{x,y} 注入视口坐标；{clear:true} 或省略坐标 = 恢复真实光标
                let pos = match (
                    params.get("x").and_then(|v| v.as_f64()),
                    params.get("y").and_then(|v| v.as_f64()),
                ) {
                    (Some(x), Some(y)) => Some(Vec2::new(x as f32, y as f32)),
                    _ => None,
                };
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::Cursor {
                        pos,
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "wheel" => {
                // #2961 项5：{x,y} UI 逻辑坐标（缺省屏幕中心），{delta} 行数（正=向下滚）
                let x = params.get("x").and_then(|v| v.as_f64()).unwrap_or(512.0) as f32;
                let y = params.get("y").and_then(|v| v.as_f64()).unwrap_or(384.0) as f32;
                let delta = params.get("delta").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                let _ = tx.send(ControlCommand::Wheel { x, y, delta });
                json!({"ok": true, "x": x, "y": y, "delta": delta})
            }
            "scroll" => {
                // #2961 项5：读全部 UiScrollList 真值（轨道绝对矩形 + offset/total/visible/z）
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::GetScroll { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "player_menu" => {
                // #2771：{object_id} → 以该玩家视口坐标打开右键菜单；返回菜单左上角，便于脚本算悬停点
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::PlayerMenu {
                        object_id,
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "quest_detail" => {
                // #2801 单元②③：{quest_id[,top_line][,confirm]} → 打开任务详情窗并直接指定
                // 「展示哪个任务 / 消息区首行 / 是否弹取消询问框」——分别等价于点日记已接行、
                // 点/滚消息区、点取消键（都写同一份 `QuestDetailState`）；quest_id<=0 关闭
                let quest_id = params.get("quest_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let top_line =
                    params.get("top_line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let confirm = params
                    .get("confirm")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::QuestDetail {
                        quest_id,
                        top_line,
                        confirm,
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "char_page" => {
                // #2775：{page} 切到角色窗某页（0=装备 1=状态 2=State 3=技能）并打开角色窗，
                // 供自动化验证技能页 Hint（页签只能点，无热键）
                let page = params.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::CharPage {
                        page,
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "chat_size" => {
                // #2781：{size} 0/1/2 → 聊天窗口 4/7/11 行（等价点控制栏「大小」按钮）
                let size = params.get("size").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::ChatSize {
                        size,
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "pickup" => {
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                if object_id > 0 {
                    let _ = tx.send(ControlCommand::Pickup { object_id });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing object_id"})
                }
            }
            "harvest" => {
                let direction = params
                    .get("direction")
                    .and_then(|v| v.as_u64())
                    .map(|d| d as u8);
                let _ = tx.send(ControlCommand::Harvest { direction });
                json!({"ok": true})
            }
            "attack" => {
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                if object_id > 0 {
                    let _ = tx.send(ControlCommand::Attack { object_id });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing object_id"})
                }
            }
            "interact" => {
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                if object_id > 0 {
                    let _ = tx.send(ControlCommand::Interact { object_id });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing object_id"})
                }
            }
            "npc_call" => {
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if object_id > 0 && !key.is_empty() {
                    let _ = tx.send(ControlCommand::NpcCall { object_id, key });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing object_id or key"})
                }
            }
            "click" => {
                // 合成鼠标点击：{x, y} 单击；{x, y, drag_to:{x,y}} 按下拖到目标再松开。
                // 逻辑坐标（与 cursor 探针同坐标系）。返回悬停命中栈（诊断用）。
                let xy = match (
                    params.get("x").and_then(|v| v.as_f64()),
                    params.get("y").and_then(|v| v.as_f64()),
                ) {
                    (Some(x), Some(y)) => Some(Vec2::new(x as f32, y as f32)),
                    _ => None,
                };
                match xy {
                    Some(pos) => {
                        let drag_to = match params.get("drag_to") {
                            Some(d) => match (
                                d.get("x").and_then(|v| v.as_f64()),
                                d.get("y").and_then(|v| v.as_f64()),
                            ) {
                                (Some(dx), Some(dy)) => Some(Vec2::new(dx as f32, dy as f32)),
                                _ => None,
                            },
                            None => None,
                        };
                        let (reply_tx, reply_rx) = bounded::<String>(1);
                        if tx
                            .send(ControlCommand::Click {
                                pos,
                                drag_to,
                                reply: reply_tx,
                            })
                            .is_ok()
                        {
                            let s = reply_rx
                                .recv_timeout(std::time::Duration::from_secs(3))
                                .unwrap_or_else(|_| "{}".to_string());
                            serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                        } else {
                            json!({"error": "control channel closed"})
                        }
                    }
                    None => json!({"error": "missing x/y"}),
                }
            }
            "diag_closebtn" => {
                let _ = tx.send(ControlCommand::DiagCloseBtn);
                json!({"ok": true})
            }
            "dialog_rect" => {
                // {kind} → 根面板屏幕矩形 {x,y,w,h,visible}（逻辑坐标），供 click 算点击点
                let kind = params.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                match parse_dialog_kind(kind) {
                    Some(k) => {
                        let (reply_tx, reply_rx) = bounded::<String>(1);
                        if tx
                            .send(ControlCommand::DialogRect {
                                kind: k,
                                reply: reply_tx,
                            })
                            .is_ok()
                        {
                            let s = reply_rx
                                .recv_timeout(std::time::Duration::from_secs(2))
                                .unwrap_or_else(|_| "{}".to_string());
                            serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                        } else {
                            json!({"error": "control channel closed"})
                        }
                    }
                    None => json!({"error": format!("unknown dialog kind: {kind}")}),
                }
            }
            "dialog" => {
                let kind = params.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                let action = params
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("toggle")
                    .to_ascii_lowercase();
                match (parse_dialog_kind(kind), action.as_str()) {
                    (Some(k), "open") => {
                        let _ = tx.send(ControlCommand::Dialog {
                            kind: k,
                            action: DialogAction::Open,
                        });
                        json!({"ok": true, "kind": kind, "action": "open"})
                    }
                    (Some(k), "close") => {
                        let _ = tx.send(ControlCommand::Dialog {
                            kind: k,
                            action: DialogAction::Close,
                        });
                        json!({"ok": true, "kind": kind, "action": "close"})
                    }
                    (Some(k), "toggle") => {
                        let _ = tx.send(ControlCommand::Dialog {
                            kind: k,
                            action: DialogAction::Toggle,
                        });
                        json!({"ok": true, "kind": kind, "action": "toggle"})
                    }
                    (Some(_), a) => {
                        json!({"error": format!("unknown action: {a} (open/close/toggle)")})
                    }
                    (None, _) if kind.is_empty() => {
                        json!({"error": "missing kind (snake_case, e.g. inventory)"})
                    }
                    (None, _) => json!({"error": format!("unknown dialog kind: {kind}")}),
                }
            }
            _ => json!({"error": format!("unknown method: {method}")}),
        };

        let resp = json!({"jsonrpc": "2.0", "id": id, "result": result});
        let _ = writeln!(stream, "{resp}");
        let _ = stream.flush();
    }
}

/// snake_case 对话框名 → DialogKind（#2586）。
///
/// 覆盖除 `GuestTrade`/`Memo`/`FishingStatus` 外的全部 46 个变体（`DialogKind` 共 49 个）——
/// 三者都没有独立开关语义：`GuestTrade` 由网络 trade 会话与 Trade 成对驱动（dialogs/trade.rs）、
/// `Memo` 由好友窗「备注」动作打开、`FishingStatus` 随钓鱼流程 `S.FishingUpdate.Fishing` 显隐，
/// 故不做 RPC 映射（调用会回 unknown dialog kind）。
/// 另有 2 个历史别名（#2599 移除 M9 占位空壳后保留工具兼容）：
/// `trust_merchant` → Market（C# TrustMerchantDialog 的真身是 market.rs）、
/// `npc_drop` → Npc（C# NPCDropDialog 的真身是挂 Npc 根下的 sell_panel.rs）。
/// **新增 DialogKind 变体时必须同步本函数、[`has_rpc_mapping`] 与测试名单**
/// （[`has_rpc_mapping`] 的穷尽 match 会让漏改编译失败）。
fn parse_dialog_kind(s: &str) -> Option<DialogKind> {
    use DialogKind as D;
    Some(match s {
        "inventory" => D::Inventory,
        "character" => D::Character,
        "quest_log" => D::QuestLog,
        "settings" => D::Settings,
        "menu" => D::Menu,
        "game_shop" => D::GameShop,
        "minimap" => D::Minimap,
        "npc" => D::Npc,
        "group" => D::Group,
        "friend" => D::Friend,
        "trade" => D::Trade,
        "inspect" => D::Inspect,
        "npc_goods" => D::NpcGoods,
        "guild" => D::Guild,
        "mail" => D::Mail,
        "ranking" => D::Ranking,
        "mentor" => D::Mentor,
        "relationship" => D::Relationship,
        "mount" => D::Mount,
        "report" => D::Report,
        "hero_inventory" => D::HeroInventory,
        "hero_equipment" => D::HeroEquipment,
        // #2892 批58：C# 无独立英雄技能窗（`HeroDialog.SkillPage`）——`hero_skill` 保留为
        // **别名**（→ `HeroEquipment`，与 `npc_drop`/`trust_merchant` 同类），仅用于实机取证的窗口开关
        "hero_skill" => D::HeroEquipment,
        "creature" => D::Creature,
        // #2599 历史别名：真实现见 market.rs / sell_panel.rs
        "trust_merchant" => D::Market,
        "item_rental" => D::ItemRental,
        "guild_territory" => D::GuildTerritory,
        "help" => D::Help,
        "notice" => D::Notice,
        "buff" => D::Buff,
        "fishing" => D::Fishing,
        "socket" => D::Socket,
        "refine" => D::Refine,
        "craft" => D::Craft,
        "dura_status" => D::DuraStatus,
        "npc_drop" => D::Npc,
        "roll" => D::Roll,
        "npc_awake" => D::NpcAwake,
        "timer" => D::Timer,
        "keyboard_layout" => D::KeyboardLayout,
        "big_map" => D::BigMap,
        "chat_notice" => D::ChatNotice,
        "market" => D::Market,
        "storage" => D::Storage,
        // #2720：C# `ItemRentalDialog`（浏览已租出物品）
        "item_rental_browse" => D::ItemRentalBrowse,
        // #2791：C# `HeroManageDialog`（`S.ManageHeroes` 弹出的英雄管理窗；RPC 直接切
        // `HeroState.managing`，见 `apply_control_commands` 的 `ControlCommand::Dialog` 分支）
        "hero_manage" => D::HeroManage,
        // #2801：C# `QuestDetailDialog`（任务详情窗，`Prguse[960]`；由任务日记行左键打开，
        // 也可由 RPC 直接开关以便实机取证）
        "quest_detail" => D::QuestDetail,
        // #2892 批C：C# `MirInputBox`（服务端发起式取名；由 `S.GuildNameRequest` /
        // `S.GuildRequestWar` 打开，RPC 仅用于实机取证时确认根节点存在）
        "input_box" => D::InputBox,
        _ => return None,
    })
}

/// 该 DialogKind 是否有 RPC 映射（= parse_dialog_kind 可达）。
///
/// **无通配臂的穷尽 match**：新增 DialogKind 变体而漏改这里会编译失败，堵住
/// 「测试名单自证互异、测不出枚举遗漏」的盲区（批M 审查发现 GuestTrade 即因此漏掉）。
fn has_rpc_mapping(kind: DialogKind) -> bool {
    use DialogKind as D;
    match kind {
        D::Inventory
        | D::Character
        | D::QuestLog
        | D::Settings
        | D::Menu
        | D::GameShop
        | D::Minimap
        | D::Npc
        | D::Group
        | D::Friend
        | D::Trade
        | D::Inspect
        | D::NpcGoods
        | D::Guild
        | D::Mail
        | D::Ranking
        | D::Mentor
        | D::Relationship
        | D::Mount
        | D::Report
        | D::HeroInventory
        | D::HeroEquipment
        | D::Creature
        | D::ItemRental
        | D::GuildTerritory
        | D::Help
        | D::Notice
        | D::Buff
        | D::Fishing
        | D::Socket
        | D::Refine
        | D::Craft
        | D::DuraStatus
        | D::Roll
        | D::NpcAwake
        | D::Timer
        | D::KeyboardLayout
        | D::BigMap
        | D::ChatNotice
        | D::Market
        | D::ItemRentalBrowse
        | D::Storage
        | D::HeroManage
        | D::QuestDetail
        | D::InputBox => true,
        // #2892 批D 单元①：备注窗由好友窗的「备注」动作打开（C# `MemoDialog.Show()`），
        // 无独立 RPC 开关
        D::Memo => false,
        // #2926：钓鱼状态窗与 `Fishing` 成对显隐（由钓鱼流程驱动），无独立 RPC 开关
        D::FishingStatus => false,
        // GuestTrade 刻意排除：网络 trade 会话驱动，无独立开关（见 parse_dialog_kind 文档）
        D::GuestTrade => false,
        // #3103：两张写邮件窗与 `Mail` 成对显隐（由 `MailState.compose` / `compose_parcel` 驱动），
        // 无独立 RPC 开关——`visible_win`/`close` 这类 RPC 仍作用于 `Mail`（列表窗）
        D::MailCompose => false,
    }
}

/// click RPC 的逐帧合成输入驱动（First 调度：抢在 PreUpdate 的 picking/input 消费前
/// 写消息，同帧生效）。三通道同步：
/// - `PointerInput`（Move/Press/Release）→ bevy_picking → bevy_ui `Interaction`（按钮链路）
/// - `MouseButtonInput` → `ButtonInput<MouseButton>`（dialog_drag/window_drag 拖动链路）
/// - `Window::set_physical_cursor_position` → `window.cursor_position()`（拖动/悬停读取方）
/// phase: 0=Move 到起点, 1=Press（并读 HoverMap 记录命中栈）, 2=拖到 drag_to（可跳过）, 3=Release+回执
/// diag_closebtn RPC 触发后：打印每个 CloseButton 实体的全组件清单（Debug 值）
fn diag_closebtn_inspect(world: &mut World) {
    if world.remove_resource::<DiagCloseBtnReq>().is_none() {
        return;
    }
    {
        let vis = world
            .get_resource::<crate::game::dialogs::storage::StorageState>()
            .map(|s| s.visible);
        let open = world
            .get_resource::<DialogManager>()
            .map(|m| m.is_open(DialogKind::Storage));
        tracing::warn!("🧬 storage 门控: state.visible={vis:?} mgr.open={open:?}");
    }
    let mut q = world.query_filtered::<Entity, With<crate::ui::theme::CloseButton>>();
    let ents: Vec<Entity> = q.iter(world).collect();
    for e in ents {
        let Ok(infos) = world.inspect_entity(e) else {
            continue;
        };
        let names: Vec<String> = infos
            .map(|i| {
                let n = i.name().to_string();
                n.rsplit("::").next().unwrap_or(&n).to_string()
            })
            .collect();
        tracing::warn!("🧬 closebtn {e:?} components: {names:?}");
    }
}

/// 🐛 诊断：Visibility 变化侦测（Changed 过滤器在 PreUpdate 与 PostUpdate 各挂一份，
/// 对比同一帧内写入发生的位置——实机交互验证抓「关闭钮被压 Hidden」用）。
/// 由 `diag_closebtn` RPC arm 的 `VisBatchWatch(N)` 帧窗口门控——常态 debug 构建不占日志。
#[cfg(debug_assertions)]
pub fn closebtn_vis_change_watch_pre(
    frames: Option<Res<VisBatchWatch>>,
    q: Query<(Entity, &Visibility), (With<crate::ui::theme::CloseButton>, Changed<Visibility>)>,
) {
    if frames.map(|f| f.0 == 0).unwrap_or(true) {
        return;
    }
    for (e, v) in &q {
        tracing::warn!("👁[Pre] closebtn {e:?} Visibility CHANGED -> {v:?}");
    }
}

/// 同上，PostUpdate 版（帧窗同 `VisBatchWatch`）
#[cfg(debug_assertions)]
pub fn closebtn_vis_change_watch_post(
    frames: Option<Res<VisBatchWatch>>,
    q: Query<(Entity, &Visibility), (With<crate::ui::theme::CloseButton>, Changed<Visibility>)>,
) {
    if frames.map(|f| f.0 == 0).unwrap_or(true) {
        return;
    }
    for (e, v) in &q {
        tracing::warn!("👁[Post] closebtn {e:?} Visibility CHANGED -> {v:?}");
    }
}

/// 🐛 诊断：开窗后 N 帧内打印全 world 的 Visibility 变化批量（识别写入查询的目标集）
#[cfg(debug_assertions)]
#[derive(Resource)]
pub struct VisBatchWatch(pub u32);

/// VisBatchWatch 资源在场时，每帧打印 Changed<Visibility> 实体（限 60 条）
#[cfg(debug_assertions)]
pub fn vis_batch_watch_post(
    frames: Option<ResMut<VisBatchWatch>>,
    q: Query<(Entity, &Visibility), Changed<Visibility>>,
) {
    let Some(mut frames) = frames else { return };
    if frames.0 == 0 {
        return;
    }
    frames.0 -= 1;
    let batch: Vec<String> = q.iter().map(|(e, v)| format!("{e:?}={v:?}")).collect();
    tracing::warn!("👁[Batch] {} changes: {:?}", batch.len(), batch);
}

/// #2956：非 Game 态排空控制队列——带回执的命令立即回 `not in game`，
/// 无回执的静默丢弃。否则命令滞留 channel，进 Game 后对已完全不同的场景补执行
/// （典型：登录态发的 click 进图后按旧坐标点到别的窗口上）。
fn drain_control_outside_game(control: Res<ControlRx>) {
    while let Ok(cmd) = control.0.try_recv() {
        if let Some(reply) = control_reply(&cmd) {
            let _ = reply.try_send(json!({"ok": false, "error": "not in game"}).to_string());
        }
    }
}

/// 取命令携带的回执通道（所有带 reply 的变体统一在此枚举；无回执变体返回 None）
fn control_reply(cmd: &ControlCommand) -> Option<&Sender<String>> {
    match cmd {
        ControlCommand::GetState { reply }
        | ControlCommand::GetDialogs { reply }
        | ControlCommand::GetVisible { reply }
        | ControlCommand::Nearby { reply, .. }
        | ControlCommand::Cursor { reply, .. }
        | ControlCommand::PlayerMenu { reply, .. }
        | ControlCommand::QuestDetail { reply, .. }
        | ControlCommand::CharPage { reply, .. }
        | ControlCommand::ChatSize { reply, .. }
        | ControlCommand::Click { reply, .. }
        | ControlCommand::DialogRect { reply, .. }
        | ControlCommand::GetScroll { reply }
        | ControlCommand::ShopProbe { reply } => Some(reply),
        _ => None,
    }
}

fn drive_pending_click(world: &mut World) {
    let Some(mut pending) = world.remove_resource::<PendingClick>() else {
        return;
    };
    let Some((window_ent, scale)) = (|| {
        let mut q = world.query_filtered::<(Entity, &Window), With<PrimaryWindow>>();
        let (e, w) = q.single(world).ok()?;
        Some((e, w.scale_factor()))
    })() else {
        if let Some(tx) = pending.reply.take() {
            let _ = tx.send(json!({"ok": false, "error": "no window"}).to_string());
        }
        return;
    };
    let set_cursor = |world: &mut World, pos: Vec2| {
        if let Some(mut w) = world.get_mut::<Window>(window_ent) {
            w.set_physical_cursor_position(Some(bevy::math::DVec2::new(
                (pos.x * scale) as f64,
                (pos.y * scale) as f64,
            )));
        }
    };
    let target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(window_ent)
            .normalize(Some(window_ent))
            .expect("Entity 归一化恒有值"),
    );
    let loc = |pos: Vec2| Location {
        target: target.clone(),
        position: pos,
    };
    match pending.phase {
        0 => {
            set_cursor(world, pending.pos);
            world.write_message(PointerInput::new(
                PointerId::Mouse,
                loc(pending.pos),
                PointerAction::Move { delta: Vec2::ZERO },
            ));
            pending.phase = 1;
        }
        1 => {
            // 读 HoverMap 命中栈（诊断：回报点击落到了什么上），按 depth 降序取前 3
            let mut hits: Vec<String> = Vec::new();
            if let Some(map) = world
                .get_resource::<HoverMap>()
                .and_then(|h| h.get(&PointerId::Mouse))
            {
                let mut v: Vec<(f32, Entity)> = map.iter().map(|(e, h)| (h.depth, *e)).collect();
                v.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                for (_, e) in v.into_iter().take(3) {
                    // 归属链：上溯 ChildOf 找 DialogRoot（诊断点击到底落在哪个窗口/画布上）
                    let mut chain = String::new();
                    let mut cur = e;
                    for _ in 0..32 {
                        let Some(co) = world.get::<ChildOf>(cur) else {
                            break;
                        };
                        let parent = co.parent();
                        let mut roots = world.query::<&DialogRoot>();
                        if let Ok(root) = roots.get(world, parent) {
                            chain = format!("root={:?}", root.0);
                            break;
                        }
                        cur = parent;
                    }
                    let mut cnQ = world.query::<&ComputedNode>();
                    let size = cnQ
                        .get(world, e)
                        .map(|n| format!("{:.0}x{:.0}", n.size().x, n.size().y))
                        .unwrap_or_else(|_| "?".to_string());
                    let name = world
                        .get::<Name>(e)
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| format!("{e:?}"));
                    hits.push(format!("{name} {size} [{chain}]"));
                }
            }
            world.write_message(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window: window_ent,
            });
            world.write_message(PointerInput::new(
                PointerId::Mouse,
                loc(pending.pos),
                PointerAction::Press(PointerButton::Primary),
            ));
            pending.reply_hits = hits;
            pending.phase = 2;
        }
        2 => {
            if let Some(to) = pending.drag_to {
                set_cursor(world, to);
                world.write_message(PointerInput::new(
                    PointerId::Mouse,
                    loc(to),
                    PointerAction::Move { delta: Vec2::ZERO },
                ));
            }
            pending.phase = 3;
        }
        _ => {
            #[cfg(debug_assertions)]
            {
                let phys = world
                    .get::<Window>(window_ent)
                    .and_then(|w| w.physical_cursor_position());
                let pressed = world
                    .get_resource::<ButtonInput<MouseButton>>()
                    .map(|b| b.pressed(MouseButton::Left))
                    .unwrap_or(false);
                let mut q = world.query::<(Entity, &Interaction)>();
                let states: Vec<String> = q
                    .iter(world)
                    .filter(|(_, i)| **i != Interaction::None)
                    .map(|(e, i)| format!("{e:?}={i:?}"))
                    .collect();
                tracing::info!(
                    "🔍 click phase3: phys={phys:?} pressed={pressed} interactions={states:?}"
                );
            }
            world.write_message(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Released,
                window: window_ent,
            });
            world.write_message(PointerInput::new(
                PointerId::Mouse,
                loc(pending.drag_to.unwrap_or(pending.pos)),
                PointerAction::Release(PointerButton::Primary),
            ));
            // Drop 类型不能移出字段——hits 先 take 出来再组回执
            let hits = std::mem::take(&mut pending.reply_hits);
            if let Some(tx) = pending.reply.take() {
                let _ = tx.send(json!({"ok": true, "hits": hits}).to_string());
            }
            // #2956：点击完成撤掉探针——否则悬停系统（Hint/NPC 行悬停）永久读到
            // 陈旧点击点，真实玩家光标被旁路；drag 场景探针也不应停在 press 点
            if let Some(mut probe) = world.get_resource_mut::<CursorProbe>() {
                probe.pos = None;
            }
            return; // 完成：资源已移除，不再回插
        }
    }
    world.insert_resource(pending);
}

fn apply_control_commands(
    mut commands: Commands,
    control: Res<ControlRx>,
    mut control_state: ResMut<ControlState>,
    mut mgr: ResMut<DialogManager>,
    net: Res<NetConnection>,
    time: Res<Time>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    mut chat: ResMut<crate::game::chat::ChatState>,
    ime: Res<crate::ui::pinyin_ime::PinyinIme>,
    mut cursor_probe: ResMut<CursorProbe>,
    mut wheels: MessageWriter<MouseWheel>,
    // #2978 审查 P1：`wheel` 注入的探针必须在滚轮消息被消费后**撤销**——否则它常驻，
    // 滚轮命中与 9 个悬停类系统会永久旁路真实光标（#2956 在 click 上修掉的同一类缺陷）。
    // `apply_control_commands` 与 `scroll_list_ui_system` 在 Update 无排序边，不能同帧撤，
    // 故给 2 帧窗口（消息双缓冲下足够被消费）。
    mut wheel_clear: Local<u8>,
    mut player_menu: ResMut<crate::game::player_menu::PlayerMenuState>,
    mut page_res: ResMut<crate::game::dialogs::character::CharPage>,
    mut q: ControlQueries,
) {
    if *wheel_clear > 0 {
        *wheel_clear -= 1;
        if *wheel_clear == 0 {
            cursor_probe.pos = None;
        }
    }
    while let Ok(cmd) = control.0.try_recv() {
        match cmd {
            ControlCommand::Move { dx, dy, run } => {
                let Ok((pe, ptf, _)) = q.players.single() else {
                    continue;
                };
                let Some(map) = &game_data.map else { continue };
                let from = world_to_tile(ptf.translation.x, ptf.translation.y);
                let target = (from.0 + dx, from.1 + dy);
                if target == from {
                    continue;
                }
                libs.0.ensure_initialized();
                match pathfinding::find_path(map, from, target) {
                    Some(p) if !p.is_empty() => {
                        commands.entity(pe).insert(LocalMove {
                            path: p.into(),
                            step_timer_ms: 0.0,
                            run,
                            last: None,
                            step_origin: None,
                            turn_acc: 0.0,
                        });
                        tracing::info!(
                            "🎮 control move: ({},{}) -> ({},{}) run={run}",
                            from.0,
                            from.1,
                            target.0,
                            target.1
                        );
                    }
                    _ => tracing::debug!(
                        "🎮 control move unreachable: ({},{}) -> ({},{})",
                        from.0,
                        from.1,
                        target.0,
                        target.1
                    ),
                }
            }
            ControlCommand::Screenshot { path } => {
                tracing::info!("🎮 control screenshot: {path}");
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
            }
            ControlCommand::Dialog { kind, action } => {
                // #2791：英雄管理窗由业务状态驱动（C# `HeroManageDialog.Show()`），
                // 不进 `DialogManager.open`——RPC 直接切 `HeroState.managing`
                if kind == DialogKind::HeroManage {
                    match action {
                        DialogAction::Open => q.hero.managing = true,
                        DialogAction::Close => {
                            q.hero.managing = false;
                            q.hero.confirm_slot = None;
                        }
                        DialogAction::Toggle => q.hero.managing = !q.hero.managing,
                    }
                } else if kind == DialogKind::InputBox {
                    // #2892 批C：`MirInputBox` 由业务状态驱动（服务端发起），RPC 直接切状态
                    match action {
                        DialogAction::Open => {
                            q.input_box.open = true;
                            q.input_box.purpose =
                                crate::game::dialogs::input_box::InputPurpose::None;
                        }
                        DialogAction::Close => q.input_box.open = false,
                        DialogAction::Toggle => q.input_box.open = !q.input_box.open,
                    }
                } else if kind == DialogKind::Storage {
                    // Storage 双门控：`StorageState.visible` 与 `DialogManager.open`
                    // 都要切，否则窗口 open 了根仍 Hidden（交互 sweep FAIL_NO_BTN）
                    match action {
                        DialogAction::Open => {
                            q.storage.visible = true;
                            mgr.open(kind);
                        }
                        DialogAction::Close => {
                            q.storage.visible = false;
                            mgr.close(kind);
                        }
                        DialogAction::Toggle => {
                            q.storage.visible = !q.storage.visible;
                            mgr.toggle(kind);
                        }
                    }
                } else {
                    match action {
                        DialogAction::Open => mgr.open(kind),
                        DialogAction::Close => mgr.close(kind),
                        DialogAction::Toggle => mgr.toggle(kind),
                    }
                }
                tracing::info!("🎮 control dialog: {kind:?} -> open={}", mgr.is_open(kind));
            }
            ControlCommand::QuestDetail {
                quest_id,
                top_line,
                confirm,
                reply,
            } => {
                // #2801 单元②③：等价于点日记已接行（打开入口）+ 点/滚消息区（`top_line`）+
                // 点取消键（`confirm`）；`quest_id <= 0` 关闭窗口
                if quest_id <= 0 {
                    mgr.close(DialogKind::QuestDetail);
                    q.quest_detail.quest_id = None;
                    let _ = reply.send(json!({"ok": true, "closed": true}).to_string());
                } else {
                    q.quest_detail.quest_id = Some(quest_id);
                    q.quest_detail.top_line = top_line;
                    q.quest_detail.confirm_cancel = confirm;
                    q.quest_detail.selected_reward = None;
                    mgr.open(DialogKind::QuestDetail);
                    let _ = reply.send(
                        json!({
                            "ok": true,
                            "quest_id": quest_id,
                            "top_line": top_line,
                            "confirm": confirm
                        })
                        .to_string(),
                    );
                }
                tracing::info!(
                    "🎮 control quest_detail: quest={quest_id} top_line={top_line} confirm={confirm}"
                );
            }
            ControlCommand::GetDialogs { reply } => {
                let list: Vec<String> = mgr.open.iter().map(|k| format!("{k:?}")).collect();
                tracing::info!("🎮 [DIALOGS] open={:?}", mgr.open);
                let _ = reply.send(format!(r#"{{"dialogs":{list:?}}}"#));
            }
            ControlCommand::GetVisible { reply } => {
                let mut map: std::collections::BTreeMap<String, usize> = Default::default();
                for (root, _node, vis, ..) in &q.dialog_roots {
                    if *vis == Visibility::Visible {
                        *map.entry(format!("{:?}", root.0)).or_insert(0) += 1;
                    }
                }
                let _ = reply.send(format!("{map:?}"));
            }
            ControlCommand::Nearby { reply, radius } => {
                let Ok((_, ptf, _)) = q.players.single() else {
                    let _ = reply.send("{}".to_string());
                    continue;
                };
                let px = ptf.translation.x;
                let py = ptf.translation.y;
                // #2767：附带视口坐标（世界 → 逻辑视口），自动化脚本据此驱动 `cursor` 探针做悬停验证
                let viewport = |tf: &Transform| -> Option<(f32, f32)> {
                    let (cam, gtf) = q.map_cameras.single().ok()?;
                    let vp = cam.world_to_viewport(gtf, tf.translation).ok()?;
                    Some((vp.x, vp.y))
                };
                let mut arr = Vec::new();
                for (tf, name, oid) in q.monsters.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < radius {
                        let vp = viewport(tf);
                        arr.push(json!({"kind": "monster", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32), "vp": vp.map(|(x, y)| json!({"x": x, "y": y}))}));
                    }
                }
                for (tf, name, oid) in q.npcs.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < radius {
                        let vp = viewport(tf);
                        arr.push(json!({"kind": "npc", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32), "vp": vp.map(|(x, y)| json!({"x": x, "y": y}))}));
                    }
                }
                for (tf, name, oid) in q.others.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < radius {
                        let vp = viewport(tf);
                        arr.push(json!({"kind": "player", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32), "vp": vp.map(|(x, y)| json!({"x": x, "y": y}))}));
                    }
                }
                for (tf, item, oid) in q.items.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < radius {
                        let vp = viewport(tf);
                        arr.push(json!({"kind": "item", "name": item.name, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32), "vp": vp.map(|(x, y)| json!({"x": x, "y": y}))}));
                    }
                }
                arr.sort_by_key(|v| v.get("dist").and_then(|d| d.as_i64()).unwrap_or(0));
                let _ = reply.send(json!({"count": arr.len(), "entities": arr}).to_string());
            }
            ControlCommand::Cursor { pos, reply } => {
                cursor_probe.pos = pos;
                let s = match pos {
                    Some(v) => json!({"ok": true, "cursor": {"x": v.x, "y": v.y}}),
                    None => json!({"ok": true, "cursor": null}),
                }
                .to_string();
                let _ = reply.send(s);
            }
            ControlCommand::Click {
                pos,
                drag_to,
                reply,
            } => {
                // 悬停系统同步看到探针光标；逐帧注入交给 First 调度的 drive_pending_click。
                // #2956：覆盖在途 PendingClick 由其 Drop 兜底回 busy（含同帧两条 click
                // 经 commands 延迟插入相互覆盖的情形），调用方不再静默超时。
                cursor_probe.pos = Some(pos);
                commands.insert_resource(PendingClick {
                    pos,
                    drag_to,
                    phase: 0,
                    reply: Some(reply),
                    reply_hits: Vec::new(),
                });
            }
            ControlCommand::DiagCloseBtn => {
                commands.insert_resource(DiagCloseBtnReq);
                #[cfg(debug_assertions)]
                commands.insert_resource(VisBatchWatch(4));
            }
            ControlCommand::DialogRect { kind, reply } => {
                // 语义：定位该窗口的**标准关闭钮**（spawn_close_button 的 CloseButton
                // 标记），返回其中心的逻辑坐标（点击点）。根面板是全屏弹性容器时
                // Node left/top 无意义，布局后矩形才可靠——用 ComputedNode +
                // UiGlobalTransform（与 ui_picking 同一坐标系：物理÷scale=逻辑）。
                let scale = q
                    .primary_window
                    .single()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0);
                let mut found = None;
                #[cfg(debug_assertions)]
                for (btn, node, tf, iv, vis) in q.close_buttons.iter() {
                    let mut cur = btn;
                    let mut chain = String::new();
                    let mut anc = String::new();
                    for _ in 0..32 {
                        let Ok(co) = q.child_of.get(cur) else { break };
                        let parent = co.parent();
                        let pv = q.dialog_roots.get(parent).ok().map(|(_, _, v, ..)| *v);
                        let pv2 = q.all_visibility.get(parent).ok().copied();
                        anc.push_str(&format!(" >{parent:?} vis={pv2:?}"));
                        if let Ok((root, _n, rvis, ..)) = q.dialog_roots.get(parent) {
                            chain = format!("{:?}/{:?}", root.0, rvis);
                            break;
                        }
                        cur = parent;
                    }
                    tracing::info!(
                        "🔍 closebtn {btn:?} tf=({:.1},{:.1}) size={:?} iv={} vis={:?} owner={} anc={}",
                        tf.translation.x,
                        tf.translation.y,
                        node.size(),
                        iv.get(),
                        vis,
                        chain,
                        anc
                    );
                }
                // 判别只用「祖先 DialogRoot 的 Visibility」（关闭钮自身 InheritedVisibility
                // 在部分对话框上不可靠——根 Visible 时仍报 false，忽略 iv 后 finder 全类别可用）
                for (btn, node, tf, _iv, _vis) in q.close_buttons.iter() {
                    // 沿 ChildOf 上溯找 DialogRoot（限 32 层防环）；**先查自身**——
                    // dura_status 的常驻切换钮 DialogRoot 就挂在钮自己身上
                    let mut cur = btn;
                    let mut owner: Option<DialogKind> = None;
                    let mut root_node: Option<&Node> = None;
                    // 布局后矩形（已除 scale）：存数值避免借用生命周期问题
                    let mut root_rect_live: Option<(f32, f32, f32, f32)> = None;
                    for _ in 0..32 {
                        if let Ok((root, n, vis, cn, gtf)) = q.dialog_roots.get(cur) {
                            if *vis == Visibility::Visible {
                                owner = Some(root.0);
                                root_node = Some(n);
                                let sz = cn.size() / scale;
                                let tl = gtf.translation / scale;
                                root_rect_live =
                                    Some((tl.x - sz.x * 0.5, tl.y - sz.y * 0.5, sz.x, sz.y));
                            }
                            break;
                        }
                        let Ok(co) = q.child_of.get(cur) else { break };
                        cur = co.parent();
                    }
                    if owner == Some(kind) {
                        let c = tf.translation / scale;
                        let size = node.size();
                        // rx/ry/rw/rh：根面板的逻辑矩形（拖动测试取空区按点此，
                        // 不可靠的纯法宝关闭钮坐标测不了空白背景）
                        // P3-2b/⑤ 共同前置（2026-09-23）：根矩形必须取**布局后**的矩形。
                        // 旧写法走 `node_rect(&Node)`（CSS left/top），对靠布局定位的根（如 NPC 窗）
                        // 恒返回 (0,0) → 行级点击落到屏幕左上（跨图与开仓库都栽在这）。
                        // 现改用 ComputedNode::size + UiGlobalTransform::translation（与 ui_picking 同坐标系，÷scale=逻辑）。
                        let (rx, ry, rw, rh) = match root_rect_live {
                            Some(r) => r,
                            _ => root_node
                                .map(|n| crate::game::dialogs::node_rect(n))
                                .unwrap_or((0.0, 0.0, 0.0, 0.0)),
                        };
                        found = Some(json!({
                            "ok": true, "kind": format!("{kind:?}"),
                            "cx": c.x, "cy": c.y, "w": size.x, "h": size.y,
                            "rx": rx, "ry": ry, "rw": rw, "rh": rh,
                        }));
                        break;
                    }
                }
                let s = found
                    .unwrap_or_else(|| json!({"ok": false, "error": "close button not found"}))
                    .to_string();
                let _ = reply.send(s);
            }
            ControlCommand::PlayerMenu { object_id, reply } => {
                // #2771：以该玩家的视口坐标当作右键点（C# 右键玩家 → 菜单左上角 = 光标位置）
                let found = q
                    .others
                    .iter()
                    .find(|(_, _, oid)| oid.0 == object_id)
                    .map(|(tf, name, _)| (tf.translation, name.0.clone()));
                let s = match found {
                    Some((pos, name)) => {
                        let vp = q
                            .map_cameras
                            .single()
                            .ok()
                            .and_then(|(cam, gtf)| cam.world_to_viewport(gtf, pos).ok());
                        match vp {
                            Some(vp) => {
                                player_menu.visible = true;
                                player_menu.name = name.clone();
                                player_menu.object_id = object_id;
                                player_menu.x = vp.x;
                                player_menu.y = vp.y;
                                json!({"ok": true, "name": name, "x": vp.x, "y": vp.y})
                            }
                            None => json!({"error": "world_to_viewport failed"}),
                        }
                    }
                    None => json!({"error": "object not found"}),
                }
                .to_string();
                let _ = reply.send(s);
            }
            ControlCommand::CharPage { page, reply } => {
                // #2775：切页 + 打开角色窗（技能页 Hint 的实机验证入口；C# 页签只能点）
                page_res.0 = page;
                mgr.open(crate::game::dialogs::DialogKind::Character);
                let s = json!({"ok": true, "page": page}).to_string();
                let _ = reply.send(s);
            }
            ControlCommand::ChatSize { size, reply } => {
                // #2781：设置聊天窗口档位（行数随之同步；几何由 chat_size_system 应用）
                let size = size.min(2);
                chat.size = size;
                chat.visible_lines = crate::game::chat::chat_size_lines(size);
                let s = json!({"ok": true, "size": size, "lines": chat.visible_lines}).to_string();
                let _ = reply.send(s);
            }
            ControlCommand::GetState { reply } => {
                let Ok((_, ptf, anim)) = q.players.single() else {
                    let _ = reply.send("{}".to_string());
                    continue;
                };
                let tile = world_to_tile(ptf.translation.x, ptf.translation.y);
                // #2595：聊天/IME 真值——e2e 验证中文输入链路的地面真值（像素比对
                // 对细字/黑条不可靠，注入激活路径又有 ToUnicode 退化问题）
                let s = json!({
                    "x": ptf.translation.x,
                    "y": ptf.translation.y,
                    "tile_x": tile.0,
                    "tile_y": tile.1,
                    // 2 跨图闭环判据：换图后这里必须变成目标地图名（此前只能从日志 MapChanged 读，
                    // 判据不是状态源；`desired_map` 由网络 MapChanged 写入）
                    "map": game_data.desired_map.clone().unwrap_or_default(),
                    // 复活闭环判据：dead 翻转 + hp 归零/回升
                    "hp": q.vitals.single().map(|v| v.hp).unwrap_or(-1),
                    "max_hp": q.vitals.single().map(|v| v.max_hp).unwrap_or(-1),
                    "dead": q.state_flags.single().map(|f| f.dead).unwrap_or(false),
                    "direction": anim.direction,
                    "chat_input_active": chat.input_active,
                    "chat_input_text": chat.input_text,
                    "ime_enabled": ime.enabled(),
                    "ime_composing": ime.composing_text(),
                    // 移动同步判据（2026-09-24）：客户端本地预测天然领先服务端 1 步
                    // （移动包在"到达那一步"时才发），而近战由服务端按「服务端玩家格 + 方向」结算
                    // ⇒ 验收夹具必须在攻击前等到 `in_sync == true`，不能用 sleep 猜。
                    "server_tile_x": q.session.last_server_position.map(|p| p.0),
                    "server_tile_y": q.session.last_server_position.map(|p| p.1),
                    "in_sync": in_sync_with_server(tile, q.session.last_server_position),
                })
                .to_string();
                let _ = reply.send(s);
            }
            ControlCommand::WalkTo { x, y, run } => {
                // 同 `Move` 臂：寻路 + LocalMove（能绕建筑）；目标是世界坐标 → 瓦片
                let Ok((pe, ptf, _)) = q.players.single() else {
                    continue;
                };
                let Some(map) = &game_data.map else {
                    tracing::warn!("🎮 control walk_to: 地图未加载，忽略");
                    continue;
                };
                let from = world_to_tile(ptf.translation.x, ptf.translation.y);
                let target = world_to_tile(x, y);
                if target == from {
                    tracing::info!("🎮 control walk_to: 已在目标 ({},{})", target.0, target.1);
                    continue;
                }
                libs.0.ensure_initialized();
                match pathfinding::find_path(map, from, target) {
                    Some(p) if !p.is_empty() => {
                        let len = p.len();
                        commands.entity(pe).insert(LocalMove {
                            path: p.into(),
                            step_timer_ms: 0.0,
                            run,
                            last: None,
                            step_origin: None,
                            turn_acc: 0.0,
                        });
                        control_state.attack_target = None;
                        control_state.pickup_target = None;
                        tracing::info!(
                            "🎮 control walk_to: ({},{}) -> ({},{}) run={run} 路径 {len} 格",
                            from.0,
                            from.1,
                            target.0,
                            target.1
                        );
                    }
                    _ => tracing::warn!(
                        "🎮 control walk_to: ({},{}) -> ({},{}) 无可行路径（墙/越界）",
                        from.0,
                        from.1,
                        target.0,
                        target.1
                    ),
                }
            }
            ControlCommand::CombatProbe { reply } => {
                // 只读探针：不写任何状态。答复里 events 是**服务端事件流**的近期片段。
                let target = control_state.attack_target;
                let player = q.players.single().ok();
                let (tx_tile, tdist, tname) = match (target, player) {
                    (Some(id), Some((_, ptf, _))) => {
                        match q.monsters.iter().find(|(_, _, oid)| oid.0 == id) {
                            Some((tf, name, _)) => {
                                let p = world_to_tile(ptf.translation.x, ptf.translation.y);
                                let t = world_to_tile(tf.translation.x, tf.translation.y);
                                let cheb = (t.0 - p.0).abs().max((t.1 - p.1).abs());
                                (Some(t), Some(cheb), Some(name.0.clone()))
                            }
                            None => (None, None, None),
                        }
                    }
                    _ => (None, None, None),
                };
                let hp_percent = target.and_then(|id| {
                    q.hp.iter()
                        .find(|(oid, _)| oid.0 == id)
                        .map(|(_, hp)| hp.percent)
                });
                let events: Vec<serde_json::Value> = control_state
                    .combat_log
                    .iter()
                    .map(|e| {
                        json!({
                            "kind": e.kind,
                            "id": e.object_id,
                            "value": e.value,
                            "actor": e.actor_id,
                        })
                    })
                    .collect();
                let payload = json!({
                    "ok": true,
                    "players": q.players.iter().count(),
                    "flags": q.flags.iter().next().map(|f| json!({
                        "dead": f.dead,
                        "fishing": f.fishing,
                        "paralysis": f.paralysis,
                    })),
                    "input_enabled": match q.flags.single() {
                        Ok(f) => !(f.dead || f.fishing || f.paralysis),
                        Err(bevy::ecs::query::QuerySingleError::NoEntities(_)) => true,
                        Err(_) => false,
                    },
                    "attack_target": target,
                    "target_name": tname,
                    "target_tile": tx_tile.map(|t| vec![t.0, t.1]),
                    "target_dist_tiles": tdist,
                    "in_melee_range": tdist.map(|d| d <= 1),
                    "hp_percent": hp_percent,
                    "attack_mode": control_state.last_attack_mode.map(|m| format!("{m:?}")),
                    "attack_interval": control_state.attack_interval,
                    "since_last_attack": control_state.last_attack,
                    // 非空转判据（#3089）：已应用战斗事件计数——实机夹具用它断言
                    // 「Struck/PlayerStruck/Died 真的到达并被 apply_combat_events 处理过」。
                    // 只读，来自 game::combat::RealHitProbe。
                    "applied": {
                        "struck": q.applied.struck_applied,
                        "player_struck": q.applied.player_struck_applied,
                        "died": q.applied.died_applied,
                    },
                    "events": events,
                });
                tracing::info!(
                    "🎮 control combat_probe: target={:?} dist={:?} hp={:?} events={}",
                    target,
                    tdist,
                    hp_percent,
                    control_state.combat_log.len()
                );
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::ChatProbe { limit, reply } => {
                // 只读：直接读 ChatState 里最近 limit 行（最新在末尾）
                let n = limit.clamp(1, 200);
                let lines: Vec<serde_json::Value> = chat
                    .lines
                    .iter()
                    .rev()
                    .take(n)
                    .map(|(text, _color, chan, _uid)| {
                        json!({"text": text, "channel": format!("{chan:?}")})
                    })
                    .collect();
                // owner 四缺陷（①滚动 / ②透明 / ③对齐 / ④尺寸还原）的实机判据要读**滚动状态与几何**，
                // 不能只看最近几行文本：`tools/acceptance/l5t_chat_dialog4.ps1` 就是靠这些字段断言的。
                // 全部只读，取自 `ChatState` 与 `game::chat` 的纯函数（与绘制/命中同源，避免两套口径漂移）。
                let (px, py, pw, ph) = crate::game::chat::chat_panel_rect(chat.size);
                let payload = json!({
                    "ok": true,
                    "count": lines.len(),
                    "lines": lines,
                    "size": chat.size,
                    "visible_lines": chat.visible_lines,
                    "scroll_up": chat.scroll_up,
                    "total_lines": chat.lines.len(),
                    "max_scroll": chat.lines.len().saturating_sub(chat.visible_lines),
                    "transparent": q.chat_filter.transparent,
                    "panel": {"x": px, "y": py, "w": pw, "h": ph},
                    "panel_top": py,
                    "bar_top": crate::game::chat::chat_bar_top(chat.size),
                    // 物理像素 = 逻辑坐标 × scale（截图判据用；见 `ControlQueries::window` 注释）
                    "window": q.window.single().ok().map(|w| json!({
                        "w": w.width(),
                        "h": w.height(),
                        "scale": w.scale_factor(),
                    })),
                });
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::BagProbe { reply } => {
                let payload = match q.bag.single() {
                    Ok(inv) => json!({
                        "ok": true,
                        "used": used_slots(&inv.items),
                        "total": inv.items.len(),
                        "quest_used": used_slots(&inv.quest_inventory),
                        "quest_total": inv.quest_inventory.len(),
                        // 任务格内容（ItemTasks 判据：任务物品是否真的进了任务格）
                        "quest_occupied": quest_cells(&inv.quest_inventory)
                            .into_iter()
                            .map(|(c, n, cnt)| json!({"cell": c, "name": n, "count": cnt}))
                            .collect::<Vec<_>>(),
                        "weight": inv.weight,
                        "max_weight": inv.max_weight,
                        // 金币：交易/存取/邮件收取闭环的 delta 判据
                        "gold": q.gold.single().map(|g| g.0).unwrap_or(0),
                        // 等级/经验：任务/打怪奖励闭环的 delta 判据
                        "level": q.progression.single().map(|p| p.level).unwrap_or(0),
                        "exp": q.progression.single().map(|p| p.exp).unwrap_or(0),
                        "max_exp": q.progression.single().map(|p| p.max_exp).unwrap_or(0),
                        // 格号 → 名称：夹具据此挑存取源格（不用猜）
                        "occupied": occupied_cells_with_uid(&inv.items)
                            .into_iter()
                            .map(|(c, n, uid)| json!({"cell": c, "name": n, "unique_id": uid}))
                            .collect::<Vec<_>>(),
                    }),
                    Err(_) => json!({"ok": false, "error": "no local player inventory"}),
                };
                tracing::info!("🎮 control bag_probe: {payload}");
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::SpellFxProbe { reply } => {
                // 只读：渲染侧真正 spawn 的特效实体（不是日志文本）。
                // 输出按 (kind, library, base, follow) 排序——Query 迭代序不稳定，
                // 夹具要靠「连读两次一致」做仪器自检，所以必须确定性（见
                // LESSON_HashMap派生JSON输出必须先排序保证确定性）。
                let mut rows: Vec<(String, String, u64, u64, usize, usize)> = Vec::new();
                for fx in q.spell_fx.iter() {
                    rows.push((
                        "cast".to_string(),
                        format!("{:?}", fx.library),
                        fx.base as u64,
                        fx.follow_object_id as u64,
                        fx.frames,
                        (fx.dur * 1000.0) as usize,
                    ));
                }
                for m in q.spell_missiles.iter() {
                    rows.push((
                        "missile".to_string(),
                        format!("{:?}", m.library),
                        m.base as u64,
                        0,
                        m.frames,
                        (m.frame_ms * 1000.0) as usize,
                    ));
                }
                rows.sort();
                let active: Vec<Value> = rows
                    .into_iter()
                    .map(|(kind, library, base, follow, frames, ms)| {
                        json!({
                            "kind": kind,
                            "library": library,
                            "base": base,
                            "frames": frames,
                            "follow_object_id": follow,
                            "ms": ms,
                        })
                    })
                    .collect();
                let payload = json!({ "ok": true, "count": active.len(), "active": active });
                tracing::info!("🎮 control spell_fx_probe: count={}", payload["count"]);
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::UiNodesAt { x, y, reply } => {
                // 逻辑坐标 → 与 ui_picking 同口径：ComputedNode.size / UiGlobalTransform.translation 再除 scale
                let scale = q
                    .primary_window
                    .single()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0);
                let mut hits: Vec<(f32, serde_json::Value)> = Vec::new();
                for (e, cn, gtf, name, vis, node, z, page_root) in q.ui_all.iter() {
                    let sz = cn.size() / scale;
                    let tl = gtf.translation / scale;
                    let (rx, ry) = (tl.x - sz.x * 0.5, tl.y - sz.y * 0.5);
                    if x < rx || x > rx + sz.x || y < ry || y > ry + sz.y {
                        continue;
                    }
                    // 祖先链（带名字，最多 8 层）——定位"挂在哪个页容器/面板下"
                    let mut chain: Vec<String> = Vec::new();
                    let mut cur = e;
                    for _ in 0..8 {
                        let Ok(co) = q.child_of.get(cur) else { break };
                        let parent = co.parent();
                        let pname = q
                            .ui_all
                            .get(parent)
                            .ok()
                            .and_then(|(_, _, _, n, v, nd, _, pr)| {
                                let label = n
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|| format!("{parent:?}"));
                                let disp =
                                    nd.map(|d| format!("{:?}", d.display)).unwrap_or_default();
                                let vv = v.map(|v| format!("{v:?}")).unwrap_or_default();
                                let pg = q
                                    .ui_all
                                    .get(parent)
                                    .ok()
                                    .and_then(|(_, _, _, _, _, _, _, pr)| {
                                        pr.map(|p| format!("{:?}", p.0))
                                    })
                                    .map(|p| format!(" page={p}"))
                                    .unwrap_or_default();
                                Some(format!("{label} disp={disp} vis={vv}{pg}"))
                            })
                            .unwrap_or_else(|| format!("{parent:?}"));
                        chain.push(pname);
                        cur = parent;
                    }
                    let zi = z.map(|z| z.0).unwrap_or(0);
                    let area = sz.x * sz.y;
                    hits.push((
                        area,
                        json!({
                            "entity": format!("{e:?}"),
                            "name": name.map(|n| n.to_string()),
                            "rect": [rx, ry, sz.x, sz.y],
                            "display": node.map(|n| format!("{:?}", n.display)),
                            "visibility": vis.map(|v| format!("{v:?}")),
                            "z": zi,
                            "guild_page": page_root.map(|p| format!("{:?}", p.0)),
                            "ancestors": chain,
                        }),
                    ));
                }
                // 小的在后（更可能盖在上面）；同面积按 z 降序
                hits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let arr: Vec<serde_json::Value> = hits.into_iter().map(|(_, v)| v).collect();
                let payload = json!({"ok": true, "x": x, "y": y, "count": arr.len(), "nodes": arr});
                tracing::info!(
                    "🎮 control ui_nodes_at({x},{y}): {} 个节点",
                    payload["count"]
                );
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::StorageProbe { reply } => {
                let payload = json!({
                    "ok": true,
                    "used": used_slots(&q.storage.items),
                    "total": q.storage.items.len(),
                    "visible": q.storage.visible,
                    "page": format!("{:?}", q.storage.page),
                    "occupied": occupied_cells(&q.storage.items)
                        .into_iter()
                        .map(|(c, n)| json!({"cell": c, "name": n}))
                        .collect::<Vec<_>>(),
                });
                tracing::info!("🎮 control storage_probe: {payload}");
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::NpcRows { reply } => {
                // 行原点直接取渲染行的 Node.left/top（= 点击分发读的同一份几何）
                let rows: Vec<(usize, f32, f32)> = q
                    .npc_lines
                    .iter()
                    .map(|(line, node)| {
                        let px = |v: Val| match v {
                            Val::Px(v) => v,
                            _ => 0.0,
                        };
                        (line.0, px(node.left), px(node.top))
                    })
                    .collect();
                let targets =
                    crate::game::dialogs::npc::npc_link_targets(&q.npc_state.lines, 0, &rows);
                let links: Vec<serde_json::Value> = targets
                    .iter()
                    .map(|t| {
                        let (cx, cy) = t.center();
                        json!({
                            "row": t.row, "text": t.text, "key": t.key,
                            "x0": t.x0, "y0": t.y0, "x1": t.x1, "y1": t.y1,
                            "cx": cx, "cy": cy,
                        })
                    })
                    .collect();
                let payload = json!({
                    "ok": true,
                    "visible": q.npc_state.visible,
                    "npc_object_id": q.npc_state.npc_object_id,
                    "lines": q.npc_state.lines,
                    "links": links,
                });
                tracing::info!("🎮 control npc_rows: {} links", targets.len());
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::AcceptQuest {
                npc_index,
                quest_index,
            } => {
                net.send_packet(&mir2_shared::packets::client::quest::AcceptQuest {
                    npc_index,
                    quest_index,
                });
                tracing::info!("🎮 control accept_quest: npc={npc_index} quest={quest_index}");
            }
            ControlCommand::StorageStore { from, to } => {
                // 与 storage.rs 点击路径发的**同一个包**（背包格 → 仓库格）
                net.send_packet(&mir2_shared::packets::client::item::StoreItem { from, to });
                tracing::info!("🎮 control storage_store: from={from} to={to}");
            }
            ControlCommand::StorageTake { from, to } => {
                // 与 storage.rs 点击路径发的**同一个包**（仓库格 → 背包格）
                net.send_packet(&mir2_shared::packets::client::item::TakeBackItem { from, to });
                tracing::info!("🎮 control storage_take: from={from} to={to}");
            }
            ControlCommand::MailSend { to, message, gold } => {
                // 与撰写窗「发送」按钮发的**同一个包**（C# MailComposeParcelDialog）
                net.send_packet(&build_send_mail(&to, &message, gold));
                tracing::info!("🎮 control mail_send: to={to} gold={gold}");
            }
            ControlCommand::MailRead { mail_id } => {
                net.send_packet(&mir2_shared::packets::client::mail::ReadMail { mail_id });
                tracing::info!("🎮 control mail_read: id={mail_id}");
            }
            ControlCommand::MailCollect { mail_id } => {
                net.send_packet(&mir2_shared::packets::client::mail::CollectParcel { mail_id });
                tracing::info!("🎮 control mail_collect: id={mail_id}");
            }
            ControlCommand::FinishQuest {
                quest_index,
                selected_item_index,
            } => {
                // 与任务日志窗「交付」按钮发的**同一个包**（C# FinishQuest[quest_index][selected_item_index]）
                net.send_packet(&build_finish_quest(quest_index, selected_item_index));
                tracing::info!(
                    "🎮 control finish_quest: quest={quest_index} sel={selected_item_index}"
                );
            }
            ControlCommand::NpcGoodsProbe { reply } => {
                let goods: Vec<serde_json::Value> = q
                    .goods
                    .goods
                    .iter()
                    .enumerate()
                    .map(|(row, g)| {
                        json!({
                            "row": row, "item_index": g.item_index, "unique_id": g.unique_id,
                            "name": g.name, "price": g.price, "count": g.count,
                        })
                    })
                    .collect();
                let payload = json!({
                    "ok": true,
                    "visible": q.goods.visible,
                    "title": q.goods.title,
                    "is_buyback": q.goods.is_buyback,
                    "count": goods.len(),
                    "goods": goods,
                });
                tracing::info!("🎮 control npc_goods_probe: {} rows", goods.len());
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::ShopProbe { reply } => {
                use crate::game::dialogs::game_shop::{filter_shop_items, now_unix};
                let filtered = filter_shop_items(
                    &q.shop.items,
                    &q.shop.search,
                    &q.shop.class_filter,
                    &q.shop.category,
                    &q.shop.section_filter,
                    now_unix(),
                );
                const PAGE_SIZE: usize = 8;
                let pages = filtered.len().div_ceil(PAGE_SIZE).max(1);
                let page = q.shop.page.min(pages - 1);
                let rows: Vec<serde_json::Value> = (page * PAGE_SIZE..(page + 1) * PAGE_SIZE)
                    .filter_map(|i| filtered.get(i).map(|idx| &q.shop.items[*idx]))
                    .enumerate()
                    .map(|(row, it)| {
                        json!({
                            "row": row, "item_index": it.item_index, "name": it.name,
                            "class": it.class, "category": it.category,
                            "deal": it.deal, "top_item": it.top_item, "date": it.date,
                            "gold": it.gold_price, "credit": it.credit_price,
                            // 试穿预览判据（ItemInfo 按需回包缓存）：类型/shape/需性别
                            "item_type": q.shop.item_infos.get(&it.item_index).map(|i| i.item_type).unwrap_or(0),
                            "shape": q.shop.item_infos.get(&it.item_index).map(|i| i.shape).unwrap_or(-1),
                            "previewable": q
                                .shop
                                .item_infos
                                .get(&it.item_index)
                                .map(|i| crate::game::dialogs::game_shop::viewer_previewable(i.item_type))
                                .unwrap_or(false),
                        })
                    })
                    .collect();
                let payload = json!({
                    "ok": true,
                    "class_filter": q.shop.class_filter,
                    "section_filter": q.shop.section_filter,
                    "category": q.shop.category,
                    "categories": q.shop.categories,
                    "page": page,
                    "pages": pages,
                    "total_items": q.shop.items.len(),
                    "filtered": filtered.len(),
                    // 试穿预览状态（None = 未打开）
                    "viewer": q.shop.viewer.map(|v| json!({
                        "item_index": v.item_index,
                        "direction": v.direction,
                        "pos": [v.pos.0, v.pos.1],
                    })),
                    "rows": rows,
                });
                tracing::info!(
                    "🎮 control shop_probe: class={} section={} filtered={}",
                    q.shop.class_filter,
                    q.shop.section_filter,
                    filtered.len()
                );
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::BuyItem { item_index, count } => {
                // 与商品窗「购买」按钮发的**同一个包**（C# 客户端 BuyItem.ItemIndex = SelectedItem.UniqueID）
                net.send_packet(&build_buy_item(item_index, count));
                tracing::info!("🎮 control buy_item: item={item_index} count={count}");
            }
            ControlCommand::SellItem { unique_id, count } => {
                // 与背包 Alt+左键快速出售**同一个包**（`dialogs/inventory.rs` 同款）
                net.send_packet(&build_sell_item(unique_id, count));
                tracing::info!("🎮 control sell_item: unique_id={unique_id} count={count}");
            }
            ControlCommand::DropItem { unique_id, count } => {
                // 与背包拖出/丢弃确认 Yes **同一个包**（`dialogs/inventory.rs` 同款）
                net.send_packet(&build_drop_item(unique_id, count));
                tracing::info!("🎮 control drop_item: unique_id={unique_id} count={count}");
            }
            ControlCommand::TownRevive => {
                // 与死亡提示框「回城复活」按钮发的**同一个包**（C# TownRevive，空体）
                net.send_packet(&mir2_shared::packets::client::misc::TownRevive);
                tracing::info!("🎮 control revive_town");
            }
            ControlCommand::MailProbe { reply } => {
                let mails: Vec<serde_json::Value> = q
                    .mail
                    .mails
                    .iter()
                    .map(|m| {
                        json!({
                            "mail_id": m.mail_id, "sender": m.sender, "subject": m.subject,
                            "unread": m.unread, "gold": m.gold, "collected": m.collected,
                        })
                    })
                    .collect();
                let detail = q.mail.detail.as_ref().map(|d| {
                    json!({
                        "mail_id": d.mail_id, "sender": d.sender, "subject": d.subject,
                        "body": d.body, "gold": d.gold, "items": d.items, "collected": d.collected,
                    })
                });
                let payload =
                    json!({ "ok": true, "count": mails.len(), "mails": mails, "detail": detail });
                tracing::info!("🎮 control mail_probe: {} mails", mails.len());
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::QuestProbe { reply } => {
                let ids = taken_quest_ids(&q.quest_log.quests);
                let taken: Vec<serde_json::Value> = q
                    .quest_log
                    .quests
                    .iter()
                    .filter(|e| e.taken)
                    // `tasks` = 任务进度行（C# QuestDialog 的「进度」段；KillTasks 形如
                    // "TigerSnake 3/10"）。**KillTasks 的判据取自它**：客户端展示的进度是
                    // 服务端 ChangeQuest 下发的真值，不是 UI 代理量（与 `taken` 同源）。
                    .map(|e| {
                        json!({
                            "id": e.id,
                            "completed": e.completed,
                            "tasks": e.tasks,
                        })
                    })
                    .collect();
                debug_assert_eq!(ids.len(), taken.len());
                let payload = json!({
                    "ok": true,
                    "taken_count": taken.len(),
                    "taken": taken,
                    "entries": q.quest_log.quests.len(),
                });
                tracing::info!("🎮 control quest_probe: taken={}", payload["taken_count"]);
                let _ = reply.send(payload.to_string());
            }
            ControlCommand::SetAttackMode { mode } => {
                // 本系统参数已达 Bevy 上限（16），不能直接挂 ResMut<AttackModeState>：
                // 经 ControlState 传请求，由 combat::apply_pending_attack_mode 消费并发包。
                control_state.pending_attack_mode = Some(mode);
                tracing::info!("🎮 control attack_mode 请求: {mode:?}");
            }
            ControlCommand::Attack { object_id } => {
                control_state.attack_target = Some(object_id);
                // P1 修复：旧写法清零计时器（与"立即攻击"语义相反）→ 连点会永远攻击不出去
                control_state.mark_attack_ready();
                if let Ok((pe, _, _)) = q.players.single() {
                    commands.entity(pe).remove::<LocalMove>();
                }
                tracing::info!("🎮 control attack: {object_id}");
            }
            ControlCommand::Interact { object_id } => {
                control_state.npc_id = Some(object_id);
                control_state.last_npc_call = time.elapsed_secs();
                // 对话内选项点击靠这个字段发 CallNPC（缺了就发 object_id 0，服务端丢弃）
                q.npc_state.npc_object_id = object_id;
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("🎮 control interact: {object_id}");
            }
            ControlCommand::NpcCall { object_id, key } => {
                tracing::info!("🎮 control npc_call: {object_id} {key}");
                q.npc_state.npc_object_id = object_id;
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC { object_id, key });
            }
            ControlCommand::Harvest { direction } => {
                // 缺省用本地玩家当前朝向（尸体在正前方 3×3 内即可，服务端 `try_harvest_corpse` 判定）
                let dir = match direction {
                    Some(d) if d < 8 => d,
                    _ => q
                        .players
                        .single()
                        .map(|(_, _, anim)| anim.direction)
                        .unwrap_or(0),
                };
                net.send_packet(&build_harvest(dir));
                tracing::info!("🎮 control harvest: dir={dir}");
            }
            ControlCommand::Pickup { object_id } => {
                let Ok((pe, ptf, _)) = q.players.single() else {
                    continue;
                };
                let Some((item_tf, _, _)) = q.items.iter().find(|(_, _, id)| id.0 == object_id)
                else {
                    tracing::warn!("🎮 control pickup: item {object_id} not found");
                    continue;
                };
                let from = world_to_tile(ptf.translation.x, ptf.translation.y);
                let item_tile = world_to_tile(item_tf.translation.x, item_tf.translation.y);
                let adjacent =
                    (item_tile.0 - from.0).abs() <= 1 && (item_tile.1 - from.1).abs() <= 1;
                if adjacent {
                    net.send_packet(&mir2_shared::packets::client::item::PickUp {});
                    control_state.attack_target = None;
                    tracing::info!("🎮 control pickup: {object_id}");
                } else if let Some(map) = &game_data.map {
                    if let Some(p) = pathfinding::find_path(map, from, item_tile) {
                        if p.is_empty() {
                            tracing::debug!("🎮 control pickup unreachable: {object_id}");
                        } else {
                            let len = p.len();
                            commands.entity(pe).insert(LocalMove {
                                path: p.into(),
                                step_timer_ms: 0.0,
                                run: true,
                                last: None,
                                step_origin: None,
                                turn_acc: 0.0,
                            });
                            control_state.attack_target = None;
                            control_state.pickup_target = Some(object_id);
                            tracing::info!("🎮 control pickup walk: {object_id} ({len} tiles)");
                        }
                    }
                }
            }
            ControlCommand::Wheel { x, y, delta } => {
                // 命中判定走 CursorProbe（theme::scroll_list_ui_system 已改读探针）
                cursor_probe.pos = Some(Vec2::new(x, y));
                wheels.write(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.0,
                    y: delta,
                    window: Entity::PLACEHOLDER,
                    phase: TouchPhase::Moved,
                });
                *wheel_clear = 2;
                tracing::info!("🎮 control wheel: ({x},{y}) delta={delta}");
            }
            ControlCommand::GetScroll { reply } => {
                // 轨道绝对原点走 theme::scroll_origin（**与滚轮/滑块命中同一份算法**，
                // 否则脚本算的命中点会与实际判定错位）
                let mut out: Vec<Value> = Vec::new();
                for (e, list, vis) in q.scroll_lists.iter() {
                    let (ox, oy) = crate::ui::theme::scroll_origin(e, &q.child_of, &q.ui_nodes);
                    let (tx, ty, tw, th) = list.track_rel;
                    let (rx, ry, rw, rh) = list.rect_rel;
                    // 轨道矩形（画在哪）+ 列表本体矩形（**滚轮命中用哪个**）都给，
                    // 脚本据此算注入点；`step` 一并给，免得脚本另猜行数/格
                    out.push(json!({
                        "entity": e.to_bits(),
                        "x": ox + tx, "y": oy + ty, "w": tw, "h": th,
                        "rx": ox + rx, "ry": oy + ry, "rw": rw, "rh": rh,
                        "offset": list.offset, "total": list.total,
                        "visible": list.visible, "step": list.step, "z": list.z,
                        // 列表实体自身是否可见（#2968 闸门：隐藏列表不吃滚轮）；
                        // 缺组件（极简测试世界无可见性传播）按 true 处理，与系统同口径
                        "shown": vis.map(|v| v.get()).unwrap_or(true),
                    }));
                }
                let _ = reply.send(json!({ "lists": out }).to_string());
            }
            ControlCommand::Chat { message } => {
                tracing::info!("🎮 control chat: {}", message);
                net.send_packet(&mir2_shared::packets::client::chat::Chat {
                    message,
                    linked_items: Vec::new(),
                });
            }
        }
    }
}

/// `walk_to` 的参数解析（纯函数，便于门禁）：世界坐标直接透传；
/// 瓦片坐标走 `movement::tile_to_world`（= `world_to_tile` 的逆变换），不自己拼 48px。
pub(crate) fn walk_world_from_params(params: &serde_json::Value) -> Option<(f32, f32)> {
    if let (Some(x), Some(y)) = (
        params.get("x").and_then(|v| v.as_f64()),
        params.get("y").and_then(|v| v.as_f64()),
    ) {
        return Some((x as f32, y as f32));
    }
    let tx = params.get("tx").and_then(|v| v.as_f64())?;
    let ty = params.get("ty").and_then(|v| v.as_f64())?;
    let w = crate::game::movement::tile_to_world(tx as i32, ty as i32);
    Some((w.x, w.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 门禁（2026-09-23 修）：`walk_to` 的瓦片入口必须与 `world_to_tile` 同源——
    /// 传 {tx,ty} 时算出的世界坐标，反查回瓦片必须还是那对瓦片。
    ///
    /// 阳性对照（落地时实做）：把 `walk_world_from_params` 里的 `tile_to_world`
    /// 换回旧的 `(tx*48, ty*48)` → 本测试立即红（反查得到 (178,-333) 这类离谱瓦片）。
    #[test]
    fn walk_to_tx_ty_round_trips_through_world_to_tile() {
        use crate::game::movement::{tile_to_world, world_to_tile};
        for (tx, ty) in [(178, 221), (0, 0), (3, 700), (699, 1)] {
            let p = serde_json::json!({ "tx": tx, "ty": ty });
            let (wx, wy) = walk_world_from_params(&p).expect("tx/ty 必须解析成功");
            let expect = tile_to_world(tx, ty);
            assert!(
                (wx - expect.x).abs() < 0.01 && (wy - expect.y).abs() < 0.01,
                "tx/ty 必须走 tile_to_world：({tx},{ty}) 得到 ({wx},{wy})，期望 ({},{})",
                expect.x,
                expect.y
            );
            assert_eq!(
                world_to_tile(wx, wy),
                (tx, ty),
                "送进去的瓦片，反查回来必须还是它（这才是 walk_to 瓦片入口的意义）"
            );
        }
        // 世界坐标仍然直接透传
        let p = serde_json::json!({ "x": 100.0, "y": -200.0 });
        assert_eq!(walk_world_from_params(&p), Some((100.0, -200.0)));
        // 两者都没给 → None（调用方回 error，不静默）
        assert_eq!(walk_world_from_params(&serde_json::json!({})), None);
    }

    /// #2956：被覆盖的 click 必须收到终态回执（busy），而非让调用方 2s 超时拿 `{}`。
    /// 覆盖路径 = 第二条 click 经 commands.insert_resource 替换在途资源 → 旧值 Drop。
    #[test]
    fn pending_click_overwrite_replies_busy() {
        let mut world = World::new();
        let (tx1, rx1) = bounded::<String>(1);
        let (tx2, _rx2) = bounded::<String>(1);
        let mk = |tx: Sender<String>| PendingClick {
            pos: Vec2::ZERO,
            drag_to: None,
            phase: 0,
            reply: Some(tx),
            reply_hits: Vec::new(),
        };
        world.insert_resource(mk(tx1));
        world.insert_resource(mk(tx2)); // 覆盖：旧 click 的调用方必须拿到终态回执
        let s = rx1
            .recv_timeout(std::time::Duration::from_millis(200))
            .expect("被覆盖的 click 应收到 busy 回执");
        assert!(s.contains("busy"), "回执应含 busy: {s}");
    }

    /// #2956：click 完成后 (a) 恰好一条 ok 回执（Drop 不再补 busy 造成协议串行）；
    /// (b) CursorProbe 撤掉——悬停系统回到真实光标，不再永久读陈旧点击点。
    #[test]
    fn drive_pending_click_finish_replies_once_and_clears_probe() {
        let mut app = App::new();
        app.add_message::<PointerInput>();
        app.add_message::<MouseButtonInput>();
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.world_mut().insert_resource(CursorProbe {
            pos: Some(Vec2::new(10.0, 20.0)),
        });
        let (tx, rx) = bounded::<String>(1);
        app.world_mut().insert_resource(PendingClick {
            pos: Vec2::new(10.0, 20.0),
            drag_to: None,
            phase: 3, // 直接进完成帧：释放 + 回执 + 探针清理
            reply: Some(tx),
            reply_hits: vec!["btn".to_string()],
        });
        drive_pending_click(app.world_mut());
        assert_eq!(
            app.world().resource::<CursorProbe>().pos,
            None,
            "点击完成后探针应撤掉（None = 用真实光标）"
        );
        let s = rx
            .recv_timeout(std::time::Duration::from_millis(200))
            .expect("完成应回 ok");
        assert!(s.contains("\"ok\":true"), "回执应 ok: {s}");
        assert!(
            rx.try_recv().is_err(),
            "同一次 click 不得有第二条回执（Drop 串行污染）"
        );
    }

    /// #2956：非 Game 态命令立即回 not in game 并排空——不得滞留到进 Game 后补执行。
    #[test]
    fn drain_outside_game_replies_not_in_game_and_empties_queue() {
        use bevy::ecs::system::RunSystemOnce;
        let (cmd_tx, cmd_rx) = bounded::<ControlCommand>(64);
        let (rtx1, rrx1) = bounded::<String>(1);
        let (rtx2, rrx2) = bounded::<String>(1);
        cmd_tx
            .send(ControlCommand::Click {
                pos: Vec2::ZERO,
                drag_to: None,
                reply: rtx1,
            })
            .unwrap();
        cmd_tx
            .send(ControlCommand::GetState { reply: rtx2 })
            .unwrap();
        // 无回执变体：静默丢弃，不 panic
        cmd_tx
            .send(ControlCommand::Move {
                dx: 1,
                dy: 0,
                run: false,
            })
            .unwrap();
        let mut world = World::new();
        world.insert_resource(ControlRx(cmd_rx));
        world
            .run_system_once(drain_control_outside_game)
            .expect("drain 应成功");
        for (name, rx) in [("click", &rrx1), ("state", &rrx2)] {
            let s = rx
                .recv_timeout(std::time::Duration::from_millis(200))
                .unwrap_or_else(|_| panic!("{name} 应收到 not in game 回执"));
            assert!(s.contains("not in game"), "{name} 回执: {s}");
        }
        assert!(
            world.resource::<ControlRx>().0.try_recv().is_err(),
            "队列应已排空"
        );
    }

    /// #2767：悬停光标解析——探针优先（自动化环境无真实光标），否则用窗口光标；
    /// 两者都无 → `None`（悬停系统据此早退）。
    #[test]
    fn resolve_cursor_prefers_probe() {
        let probe = Some(Vec2::new(300.0, 200.0));
        let window = Some(Vec2::new(1.0, 2.0));
        assert_eq!(resolve_cursor(probe, window), probe);
        assert_eq!(resolve_cursor(probe, None), probe);
        assert_eq!(resolve_cursor(None, window), window);
        assert_eq!(resolve_cursor(None, None), None);
    }

    /// `parse_dialog_kind` 可解析的全部名字（48 个名字 → 45 个变体：含 trust_merchant /
    /// npc_drop / hero_skill 三个别名）。与交互巡回清单
    /// （`tools/acceptance/interact_sweep_manifest.json`）共用同一套名字空间。
    const RPC_KIND_NAMES: [&str; 48] = [
        "inventory",
        "character",
        "quest_log",
        "settings",
        "menu",
        "game_shop",
        "minimap",
        "npc",
        "group",
        "friend",
        "trade",
        "inspect",
        "npc_goods",
        "guild",
        "mail",
        "ranking",
        "mentor",
        "relationship",
        "mount",
        "report",
        "hero_inventory",
        "hero_equipment",
        "hero_skill",
        "creature",
        "trust_merchant",
        "item_rental",
        "guild_territory",
        "help",
        "notice",
        "buff",
        "fishing",
        "socket",
        "refine",
        "craft",
        "dura_status",
        "npc_drop",
        "roll",
        "npc_awake",
        "timer",
        "keyboard_layout",
        "big_map",
        "chat_notice",
        "market",
        "storage",
        "item_rental_browse",
        "hero_manage",
        "quest_detail",
        "input_box",
    ];

    /// parse_dialog_kind 覆盖除 GuestTrade 外全部变体 + 未知返回 None（#2586）
    /// GuestTrade 由网络 trade 会话驱动无独立开关，刻意不做 RPC 映射（批M 审查）
    #[test]
    fn parse_dialog_kind_covers_all_variants() {
        let all = RPC_KIND_NAMES;
        // #2599：trust_merchant/npc_drop 是历史别名（→ Market/Npc，真实现移壳后保留工具兼容），
        // 与 market/npc 重复映射——互异断言计数时先去掉这 2 个别名。
        // 名单与 witness 一致：每个可解析名都有 RPC 映射；DialogKind 共 48 个变体（#2892 批58 删 HeroSkill），
        // GuestTrade/Memo/FishingStatus 刻意排除——枚举级穷尽由 has_rpc_mapping 的无通配 match 编译期保证）
        let parsed: Vec<DialogKind> = all.iter().map(|s| parse_dialog_kind(s).unwrap()).collect();
        let uniq: Vec<&DialogKind> = {
            let mut seen: Vec<&DialogKind> = parsed.iter().collect();
            seen.sort_by_key(|k| format!("{k:?}"));
            seen.dedup_by_key(|k| format!("{k:?}"));
            seen
        };
        assert_eq!(all.len(), 48);
        assert_eq!(
            uniq.len(),
            45,
            "48 个名字（含 trust_merchant/npc_drop/hero_skill 三个别名）应映射到 45 个不同变体"
        );
        // 名单与 witness 一致：每个可解析名都有 RPC 映射
        assert!(
            parsed.iter().all(|k| has_rpc_mapping(*k)),
            "名单内全部变体应 has_rpc_mapping"
        );
        // GuestTrade 刻意排除（网络会话驱动）
        assert!(!has_rpc_mapping(DialogKind::GuestTrade));

        // 未知/空/大小写敏感
        assert!(parse_dialog_kind("").is_none());
        assert!(parse_dialog_kind("nope").is_none());
        assert!(
            parse_dialog_kind("Inventory").is_none(),
            "snake_case 小写约定"
        );
    }

    /// 交互巡回覆盖清单（`tools/acceptance/interact_sweep_manifest.json`）与 RPC 窗口登记对账。
    ///
    /// 背景：`parse_dialog_kind` 只是给了窗口一个 RPC 开关，**能开 ≠ 有人验过**。
    /// `tools/acceptance/ui_interact_sweep.ps1` 是唯一做「open → 点关闭钮 → 断言真关掉」
    /// 交互级验证的地方，它跑哪些窗口完全由这份清单决定——清单漏登记，那个窗口就从此
    /// 没人验（#2953/#2955 那批交互缺陷正是这一类）。本测试把「新增 RPC 窗口必须登记
    /// （进 sweep，或进 excluded 并写明理由与覆盖路径）」变成 `cargo test --lib` 拦得住的约束。
    ///
    /// 覆盖按**变体**算（别名 trust_merchant/npc_drop/hero_skill 不额外计数）。
    /// 已知边界（两条都留给人审，测试只挡"漏登记"与"空理由"）：
    /// 1. 变体名单取自 [`RPC_KIND_NAMES`]，其完整性由
    ///    [`parse_dialog_kind_covers_all_variants`] 的 `assert_eq!(all.len(), 48)` 守着——
    ///    新增变体却完全不碰那张名单时本测试看不见（那条路径要靠 `has_rpc_mapping` 的
    ///    穷尽 match 逼人去改，改到这里就会看到清单对账）。
    /// 2. `excluded` 的理由是自由文本：把新窗口塞进 excluded 并编个理由确实能过（只是 ≥8 字），
    ///    是否真被别的路径覆盖仍要人看——状态驱动窗口（npc/trade/npc_goods/roll）没法塞进
    ///    逐窗循环：它们的显隐每帧由游戏状态同步，RPC 开一下会被立刻覆盖，硬塞只会得到假 FAIL。
    #[test]
    fn interact_sweep_manifest_covers_all_rpc_kinds() {
        // include_str!：清单文件被删/改名 = 编译失败，而不是静默跳过一个不存在的门禁
        let raw = include_str!("../../tools/acceptance/interact_sweep_manifest.json");
        let m: Value = serde_json::from_str(raw).expect("交互巡回清单必须是合法 JSON");
        let names = |key: &str| -> Vec<String> {
            m[key]
                .as_array()
                .unwrap_or_else(|| panic!("清单缺数组字段 {key}"))
                .iter()
                .map(|v| v.as_str().expect("清单项应为字符串").to_string())
                .collect()
        };
        let sweep = names("sweep");
        let no_btn = names("no_close_by_design");
        let excluded: Vec<String> = m["excluded"]
            .as_object()
            .expect("清单缺 excluded 对象")
            .iter()
            .map(|(k, v)| {
                assert!(
                    v.as_str().is_some_and(|s| s.trim().chars().count() >= 8),
                    "excluded[{k}] 必须写明理由（≥8 字）：no RPC 开关的窗口也要说清由谁覆盖"
                );
                k.clone()
            })
            .collect();

        assert!(!sweep.is_empty(), "sweep 不应为空");
        for n in sweep.iter().chain(excluded.iter()) {
            assert!(
                parse_dialog_kind(n).is_some(),
                "清单里的 `{n}` 不是可解析的窗口名（parse_dialog_kind 返回 None）"
            );
        }
        let uniq = {
            let mut s = sweep.clone();
            s.sort();
            s.dedup();
            s.len()
        };
        assert_eq!(uniq, sweep.len(), "sweep 有重复项");
        for n in &no_btn {
            assert!(
                sweep.contains(n),
                "no_close_by_design 的 `{n}` 不在 sweep 里"
            );
        }
        for n in &excluded {
            assert!(!sweep.contains(n), "`{n}` 同时在 sweep 与 excluded 里");
        }

        let covered: Vec<DialogKind> = sweep
            .iter()
            .chain(excluded.iter())
            .filter_map(|n| parse_dialog_kind(n))
            .collect();
        let missing: Vec<String> = RPC_KIND_NAMES
            .iter()
            .filter_map(|n| parse_dialog_kind(n))
            .filter(|k| has_rpc_mapping(*k) && !covered.contains(k))
            .map(|k| format!("{k:?}"))
            .collect();
        assert!(
            missing.is_empty(),
            "有 RPC 开关却既不在 sweep 也不在 excluded：{missing:?}——\
             请登记进 tools/acceptance/interact_sweep_manifest.json\
             （可逐窗开关的进 sweep；状态/会话驱动的进 excluded 并写明理由与覆盖路径）"
        );
    }

    /// --control-port 未指定：回退默认 9000（含参数表里根本没有该 flag）。
    #[test]
    fn parse_control_port_defaults_to_9000() {
        let args: Vec<String> = vec!["client_bevy".into()];
        assert_eq!(parse_control_port(&args), 9000);
        let args: Vec<String> = vec!["client_bevy".into(), "--real-net".into()];
        assert_eq!(parse_control_port(&args), 9000);
    }

    /// --control-port 指定合法 u16：用之（双客户端并行各听一端口的前置）。
    #[test]
    fn parse_control_port_uses_given_value() {
        let args: Vec<String> = vec!["client_bevy".into(), "--control-port".into(), "9001".into()];
        assert_eq!(parse_control_port(&args), 9001);
    }

    /// --control-port 非法值（非数字/超范围/缺参数）：warn 并回退 9000。
    #[test]
    fn parse_control_port_invalid_falls_back_to_9000() {
        for bad in ["abc", "65536", "-1", "9.5", "", "--real-net"] {
            let args: Vec<String> = vec!["client_bevy".into(), "--control-port".into(), bad.into()];
            assert_eq!(
                parse_control_port(&args),
                9000,
                "非法值 {bad:?} 应回退 9000"
            );
        }
        // flag 在末尾、值缺失：同样回退
        let args: Vec<String> = vec!["client_bevy".into(), "--control-port".into()];
        assert_eq!(parse_control_port(&args), 9000);
    }

    /// --control-port 边界：0 与 65535 均为合法 u16，照常接受。
    #[test]
    fn parse_control_port_accepts_boundary_values() {
        for (raw, want) in [("0", 0u16), ("65535", 65535u16)] {
            let args: Vec<String> = vec!["client_bevy".into(), "--control-port".into(), raw.into()];
            assert_eq!(parse_control_port(&args), want, "边界值 {raw} 应接受");
        }
    }

    /// `nearby` 半径解析门禁（2026-09-23）：缺省 600（与原硬编码一致），显式正值生效，
    /// 非正/NaN/无穷回退 600。
    ///
    /// 阳性对照（实做）：把 `_ => 600.0` 改成 `_ => raw.unwrap_or(0.0) as f32`
    /// → 本测试立即红（`None`/`0`/`-5` 会得到 0 = 一个实体都扫不到）。
    #[test]
    fn nearby_radius_defaults_and_rejects_nonpositive() {
        assert_eq!(
            parse_nearby_radius(None),
            600.0,
            "缺省必须与原硬编码 600 一致"
        );
        assert_eq!(
            parse_nearby_radius(Some(100_000.0)),
            100_000.0,
            "显式半径生效"
        );
        assert_eq!(parse_nearby_radius(Some(1.0)), 1.0);
        assert_eq!(
            parse_nearby_radius(Some(0.0)),
            600.0,
            "radius=0 会静默扫不到实体，必须回退"
        );
        assert_eq!(parse_nearby_radius(Some(-5.0)), 600.0);
        assert_eq!(parse_nearby_radius(Some(f64::NAN)), 600.0);
        assert_eq!(parse_nearby_radius(Some(f64::INFINITY)), 600.0);
    }

    /// `occupied_cells` 门禁（2026-09-23）：格号必须是**原始下标**（存取包 from/to 用它），
    /// 空格的 None 不得占位。阳性对照：把 `enumerate` 换成过滤后的 `.map`（丢下标）
    /// 或把 `filter_map` 改成 `map` → 本测试红。
    #[test]
    fn occupied_cells_keeps_real_cell_indices() {
        use crate::game::dialogs::inventory::InvItem;
        let mk = |name: &str| InvItem {
            name: name.to_string(),
            ..Default::default()
        };
        let items: Vec<Option<InvItem>> = vec![None, Some(mk("Saddle")), None, Some(mk("Gold"))];
        let got = occupied_cells(&items);
        assert_eq!(
            got,
            vec![(1, "Saddle".to_string()), (3, "Gold".to_string())],
            "必须保留真实格号（1 与 3），不能压缩成 0/1"
        );
    }

    /// ⑤ 邮件闭环门禁：`mail_send` 构造的包必须**原样**带上收件人/正文/金币，
    /// 附件为空且不贴票（附件与贴票另有 UI 路径，这里不碰）。
    ///
    /// 为什么值得一测：服务端按 `name` 找收件人、按 `gold` 入箱；动作 RPC 若把金币吞成 0，
    /// 收件人 collected 翻转后**收不到钱**，而日志看上去"发送成功"——闭环判据（金币 delta）
    /// 会给出假红，查很久才发现是夹具/动作侧吞参数。
    ///
    /// 阳性对照（实做）：把 `gold` 改成常量 0 → 本测试立即红。
    #[test]
    fn build_send_mail_carries_recipient_message_and_gold() {
        let pkt = build_send_mail("bevy2char", "e2e 邮件正文", 123);
        assert_eq!(pkt.name, "bevy2char");
        assert_eq!(pkt.message, "e2e 邮件正文");
        assert_eq!(pkt.gold, 123, "金币必须原样发出（否则收件人收不到钱）");
        assert_eq!(pkt.items_idx, [0u64; 5], "本动作不带附件");
        assert!(!pkt.stamped, "本动作不贴票");
        // 空收件人由 RPC 层拒绝（服务端按 name 查人，空名只会静默失败）
        assert!(pkt.name.is_empty() || !pkt.name.trim().is_empty());
    }

    /// ④ 任务交付门禁：`finish_quest` 的两个字段**不得互换**（服务端按位置读
    /// `[quest_index][selected_item_index]`）。写反了会去交"下标那个任务"，
    /// 症状是「交了个不相干任务 / 报任务不存在」，而日志看着像调用成功。
    ///
    /// 阳性对照（实做）：把两个字段调换 → 本测试立即红。
    #[test]
    fn build_finish_quest_keeps_field_order() {
        let pkt = build_finish_quest(86, -1);
        assert_eq!(pkt.quest_index, 86, "第一个字段必须是任务号");
        assert_eq!(
            pkt.selected_item_index, -1,
            "第二个字段是可选奖励下标（-1=不选）"
        );
        let pkt2 = build_finish_quest(27, 2);
        assert_eq!((pkt2.quest_index, pkt2.selected_item_index), (27, 2));
    }

    /// ③ 购买门禁：商品号与**数量**必须原样发出，面板类型固定 Buy。
    /// 数量被吞成 1 的后果是「少扣钱少发货」，而回执与日志都像成功——判据（金币 delta）
    /// 会按单价对不上，得排查很久才想到是动作侧吞参数。
    ///
    /// 阳性对照（实做）：把 `count` 改成常量 1 → 本测试立即红。
    #[test]
    fn build_sell_item_carries_unique_id_and_count() {
        // 出售按**背包实例 unique_id** 定位（不是 item_index）：拿 item_index 去卖会
        // 卖错实例或报找不到物品，而 RPC 回执仍是 ok——所以字段必须原样钉住。
        // 阳性对照（实做）：把 unique_id 换成 item_index（或写死 0）→ 本测试立即红。
        let pkt = build_sell_item(4242, 3);
        assert_eq!(pkt.unique_id, 4242, "必须是背包实例 unique_id");
        assert_eq!(pkt.count, 3, "数量必须原样发出（吞成 1 会少卖少收钱）");
        assert_eq!(build_sell_item(7, 1).count, 1);
    }

    /// ⑨ 丢弃门禁：与背包拖出/确认框 Yes 同款包体（`unique_id` 按实例定位、`hero_inventory=false`）。
    /// 阳性对照（实做）：把 `hero_inventory` 改成 true → 本测试立即红（服务端会按英雄背包找物品，
    /// 报"物品不存在"，而 RPC 回执仍是 ok）。
    #[test]
    fn build_drop_item_targets_backpack_instance() {
        let pkt = build_drop_item(9001, 2);
        assert_eq!(pkt.unique_id, 9001, "按背包实例 unique_id 定位");
        assert_eq!(pkt.count, 2, "数量必须原样发出");
        assert!(
            !pkt.hero_inventory,
            "控制入口只丢角色背包（英雄背包另有路径）"
        );
    }

    #[test]
    fn build_buy_item_carries_count_and_panel_type() {
        let pkt = build_buy_item(317, 3);
        assert_eq!(
            pkt.item_index, 317,
            "商品号必须原样（常规商店 = 商品行 unique_id）"
        );
        assert_eq!(pkt.count, 3, "数量必须原样发出");
        assert_eq!(pkt.panel_type, mir2_shared::enums::PanelType::Buy);
        assert_eq!(build_buy_item(317, 1).count, 1);
    }

    /// ② 复活门禁：`revive_town` 必须发**服务端真正分派的那个 opcode**。
    /// 服务端按 `ClientPacketIds::TownRevive` 分派（gate/actor.rs:1078），发错包会静默无反应，
    /// 而夹具只看"死没死/活没活"是能骗过去的——所以这里直接钉 opcode。
    ///
    /// 阳性对照（实做）：把动作臂改发别的包（如 `ReviveHero`）→ 本测试立即红。
    #[test]
    fn town_revive_opcode_matches_server_dispatch() {
        use mir2_shared::packets::base::Packet;
        assert_eq!(
            mir2_shared::packets::client::misc::TownRevive::OPCODE,
            mir2_shared::enums::ClientPacketIds::TownRevive as i16,
            "revive_town 必须发 TownRevive（服务端就按这个 opcode 分派）"
        );
    }

    /// `attack_mode` RPC 的模式名解析（2026-09-22 玩家验收能力）。
    ///
    /// 阳性对照（落地时实做）：把 `_ => None` 改成 `_ => Some(Peace)`（静默回退）后，
    /// 下面 "拒绝未知模式" 的断言立即变红——这一类静默回退正是 P1「以为切了模式其实没切」的土壤。
    #[test]

    /// 仪器门禁（2026-09-22）：`quest_probe` 的判据内核必须**只取 taken 条目**，
    /// 且对同一状态**连续两次读数一致**（来源是状态，不是 UI 代理量）。
    ///
    /// 阳性对照（落地时实做）：把 `filter(|e| e.taken)` 去掉（把未接条目也算进去）
    /// → 本测试立即红；恢复后绿。
    #[test]

    /// ⑤ 仪器门禁（2026-09-22）：占用格数必须由状态算出，且空/非空两态可重复。
    ///
    /// 阳性对照：把 `filter(|s| s.is_some())` 去掉（把空格也算占用）→ 本测试立即红。
    #[test]
    fn used_slots_counts_only_occupied_and_is_stable() {
        let empty: Vec<Option<u8>> = vec![None, None, None];
        assert_eq!(used_slots(&empty), 0, "全空必须读 0");
        let mixed: Vec<Option<u8>> = vec![None, Some(1), Some(2), None];
        assert_eq!(used_slots(&mixed), 2, "只应数 Some");
        assert_eq!(
            used_slots(&mixed),
            used_slots(&mixed),
            "同一状态连读必须一致"
        );
    }
    fn taken_quest_ids_is_state_sourced_and_stable() {
        use crate::game::dialogs::quest_log::QuestEntry;
        let mk = |id: i32, taken: bool, completed: bool| QuestEntry {
            id,
            taken,
            completed,
            ..Default::default()
        };
        // 空态：没有已接任务（反例：quest_detail 那种"任意 id 都 ok"的实现会给出非空恒定值）
        let empty: Vec<QuestEntry> = vec![];
        assert!(taken_quest_ids(&empty).is_empty(), "空列表必须读出空");
        // 非空态：只取 taken
        let mixed = vec![
            mk(27, true, false),
            mk(28, false, false),
            mk(142, true, true),
        ];
        assert_eq!(taken_quest_ids(&mixed), vec![27, 142], "只应取 taken 条目");
        // 同一状态连读两次必须一致（与实机仪器自检同口径）
        assert_eq!(
            taken_quest_ids(&mixed),
            taken_quest_ids(&mixed),
            "同一状态连读必须一致"
        );
    }

    /// ④ 仪器门禁（2026-09-24）：ItemTasks 任务的判据必须读**任务格**（`quest_inventory`）——
    /// 服务端 `Q` 掉落（`world/mod.rs try_give_quest_item`）把任务物品**直接放进任务格**、
    /// 不落地也不进背包，所以「背包里有这个物品」不能当作完成任务，反之任务格为空也未必是没掉。
    ///
    /// 阳性对照：把 `filter_map` 换成 `map`（空格的 None 也算一格）→ 本测试立即红。
    #[test]
    fn quest_cells_reads_quest_bag_only_and_is_stable() {
        use crate::game::dialogs::inventory::InvItem;
        let mk = |name: &str, count: u16| InvItem {
            name: name.to_string(),
            count,
            ..Default::default()
        };
        // 空态：任务格还没东西（接取前 / 掉落还没到）
        let empty: Vec<Option<InvItem>> = vec![None, None, None];
        assert!(quest_cells(&empty).is_empty(), "全空必须读出空");
        // 非空态：格号是**原始下标**（任务格下标与背包无关），数量原样带出
        let quest_bag: Vec<Option<InvItem>> = vec![None, Some(mk("RedSnakeTeeth", 2)), None];
        assert_eq!(
            quest_cells(&quest_bag),
            vec![(1, "RedSnakeTeeth".to_string(), 2)],
            "必须保留真实格号与数量"
        );
        // 同一状态连读两次必须一致（与实机仪器自检同口径）
        assert_eq!(
            quest_cells(&quest_bag),
            quest_cells(&quest_bag),
            "同一状态连读必须一致"
        );
        // 反向对照：「背包里有货」不等于「任务格里有货」——两个来源必须区分，
        // 否则夹具会把普通背包物品当成 ItemTasks 已达成（假绿）。
        let backpack: Vec<Option<InvItem>> = vec![Some(mk("RedSnakeTeeth", 2))];
        assert_eq!(occupied_cells(&backpack).len(), 1, "背包侧读数不受影响");
        assert!(
            quest_cells(&vec![None, None]).is_empty(),
            "背包有货时任务格仍必须是空"
        );
    }

    /// 门禁（2026-09-24 实机缺陷：客户端偶发**进场即崩**）：带 `CloseButton` 的 UI 实体 spawn 时
    /// **不得**因为组件钩子 panic。曾经的 debug-only `Visibility` on_insert 诊断钩子（打印全文回溯）
    /// 就挂在这条命令应用路径上，间歇 panic ⇒ 进程退出、没有本地玩家、玩家侧卡登录/黑屏。
    /// 阳性对照：把那个钩子加回 `ControlPlugin::build` → 本测试立即红（钩子内 panic 会冒泡出来）。
    #[test]
    fn spawning_close_button_ui_does_not_panic_via_component_hooks() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // 真插件（含钩子注册路径）；端口绑定失败只打日志、不影响本测试
        app.add_plugins(ControlPlugin);
        app.update();
        let e = app
            .world_mut()
            .spawn((crate::ui::theme::CloseButton, Visibility::Visible))
            .id();
        app.update();
        assert!(
            app.world().get_entity(e).is_ok(),
            "CloseButton + Visibility 的 spawn 必须正常落地（钩子 panic 会让这里拿不到实体）"
        );
    }

    /// 采集门禁（2026-09-24）：`build_harvest` 必须把方向原样带进 `C.Harvest`——可采集怪的 Q 物品
    /// 只能靠剥皮交付（`roll_harvest_drops` 修好后），方向错了服务端那 3×3 就找不到尸体。
    /// 阳性对照：把方向换成常量 `Up`（或丢掉 `% 8` 归一）→ 越界/错误方向断言红。
    #[test]
    fn build_harvest_carries_direction() {
        use mir2_shared::enums::MirDirection;
        assert_eq!(build_harvest(0).direction, MirDirection::Up);
        assert_eq!(build_harvest(2).direction, MirDirection::Right);
        assert_eq!(build_harvest(6).direction, MirDirection::Left);
        // 越界方向必须归一而不是 panic（服务端只认 0..8）
        assert_eq!(build_harvest(9).direction, MirDirection::UpRight);
        assert_eq!(build_harvest(255).direction, MirDirection::UpLeft);
    }

    /// 移动同步门禁（2026-09-24）：`in_sync_with_server` 只在**瓦片完全相等**时给 true，
    /// 未知（没收到过 UserLocation）必须给 false——夹具就是靠它决定"能不能挥砍"的，
    /// 放宽成"未知也算同步"会让近战在落后一格时空挥（本轮的实测缺陷形态）。
    ///
    /// 阳性对照：把实现改成 `server_tile.is_some()`（不比较瓦片）→ 本测试立即红。
    #[test]
    fn in_sync_requires_exact_tile_match() {
        assert!(
            !in_sync_with_server((300, 300), None),
            "没收到过 UserLocation 时必须判不同步"
        );
        assert!(
            in_sync_with_server((300, 300), Some((300, 300))),
            "同格才算同步"
        );
        assert!(
            !in_sync_with_server((300, 300), Some((299, 300))),
            "差一格（本地预测领先一步）必须判不同步"
        );
        assert!(
            !in_sync_with_server((300, 300), Some((301, 301))),
            "差一格斜向同样不同步"
        );
        // 同一输入连读必须一致（探针口径）
        assert_eq!(
            in_sync_with_server((412, 96), Some((412, 96))),
            in_sync_with_server((412, 96), Some((412, 96)))
        );
    }

    fn parse_attack_mode_maps_names_and_rejects_unknown() {
        use mir2_shared::enums::AttackMode;
        let cases = [
            ("peace", AttackMode::Peace),
            ("group", AttackMode::Group),
            ("guild", AttackMode::Guild),
            ("all", AttackMode::All),
            ("enemy_guild", AttackMode::EnemyGuild),
            ("enemyguild", AttackMode::EnemyGuild),
            ("red_brown", AttackMode::RedBrown),
            ("redbrown", AttackMode::RedBrown),
            ("  GUILD  ", AttackMode::Guild),
        ];
        for (raw, want) in cases {
            assert_eq!(
                parse_attack_mode(raw),
                Some(want),
                "{raw} 应解析为 {want:?}"
            );
        }
        // 注意 "group " 这类**首尾空白**是合法的（解析前 trim），故不放这里；
        // 这里只放真正未知的名字。
        for bad in ["", "   ", "peaceful", "全部", "guild_", "allx", "0"] {
            assert_eq!(
                parse_attack_mode(bad),
                None,
                "未知模式 {bad:?} 必须被拒绝（不得静默回退成某个模式）"
            );
        }
    }
}
