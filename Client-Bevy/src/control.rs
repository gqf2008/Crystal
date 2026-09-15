// ============================================================================
// control.rs 客户端内置控制接口（TCP JSON-RPC，供 MCP/agent 控制玩家）
// 监听 127.0.0.1:9000，每行一条 JSON-RPC。
//   move {dx,dy,run}     相对玩家瓦片偏移移动（dx/dy 为瓦片数）
//   screenshot {path}    保存当前帧截图
//   state {}             返回玩家位置/朝向
//   nearby {}            返回周围实体（含 object_id）
//   attack {object_id}   攻击指定对象
//   interact {object_id} 与指定 NPC 对话
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
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
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
enum ControlCommand {
    Move {
        dx: i32,
        dy: i32,
        run: bool,
    },
    Screenshot {
        path: String,
    },
    GetState {
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
}

/// dialog 命令的动作（#2586）
enum DialogAction {
    Open,
    Close,
    Toggle,
}

#[derive(Resource)]
struct ControlRx(Receiver<ControlCommand>);

/// #2767 光标探针：自动化环境（无焦点/共享桌面）里 `Window::cursor_position()` 不可用，
/// 悬停类系统改读这里注入的视口坐标；`None` = 用真实光标。
#[derive(Resource, Default)]
pub struct CursorProbe {
    pub pos: Option<Vec2>,
}

/// 悬停用的光标位置：探针优先，其次真实窗口光标（纯函数便于单测）。
pub fn resolve_cursor(probe: Option<Vec2>, window: Option<Vec2>) -> Option<Vec2> {
    probe.or(window)
}

/// #2767：控制接口用到的实体查询打包（原先 16 个系统参数已是 Bevy 上限，
/// 再加「光标探针 + 相机」就编译失败）。
#[derive(SystemParam)]
struct ControlQueries<'w, 's> {
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
    dialog_roots: Query<'w, 's, (&'static DialogRoot, &'static Visibility)>,
    /// #2791：`hero_manage` 是状态驱动窗（不经 `DialogManager.open`，见 dialogs/mod.rs
    /// 的 `DialogKind::HeroManage`），RPC 直接切 `HeroState.managing`
    hero: ResMut<'w, crate::game::dialogs::hero::HeroState>,
    /// #2801 单元②③：任务详情窗状态（`quest_detail` RPC 直接指定任务/分页首行/询问框）
    quest_detail: ResMut<'w, crate::game::dialogs::quest_log::QuestDetailState>,
    /// #2892 批C：`MirInputBox` 是状态驱动窗（服务端 `S.GuildNameRequest`/`S.GuildRequestWar`
    /// 打开），RPC 直接切 `InputBoxState.open` 以便实机取证
    input_box: ResMut<'w, crate::game::dialogs::input_box::InputBoxState>,
    map_cameras: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (With<Camera2d>, Without<crate::ui::sprite_ui::UiEntity>),
    >,
}

pub struct ControlPlugin;

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = bounded::<ControlCommand>(64);
        app.insert_resource(ControlRx(rx));
        // #2767：光标探针（悬停类系统的自动化入口）
        app.init_resource::<CursorProbe>();
        std::thread::spawn(move || control_listener(tx));
        app.add_systems(
            Update,
            apply_control_commands.run_if(in_state(AppState::Game)),
        );
    }
}

fn control_listener(tx: Sender<ControlCommand>) {
    let listener = match TcpListener::bind("127.0.0.1:9000") {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("[control] 绑定 9000 失败: {e}");
            return;
        }
    };
    tracing::info!("[control] 监听 127.0.0.1:9000");
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
                if tx.send(ControlCommand::Nearby { reply: reply_tx }).is_ok() {
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
        "hero_skill" => D::HeroSkill,
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
        | D::HeroSkill
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
    }
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
    mut player_menu: ResMut<crate::game::player_menu::PlayerMenuState>,
    mut page_res: ResMut<crate::game::dialogs::character::CharPage>,
    mut q: ControlQueries,
) {
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
                for (root, vis) in &q.dialog_roots {
                    if *vis == Visibility::Visible {
                        *map.entry(format!("{:?}", root.0)).or_insert(0) += 1;
                    }
                }
                let _ = reply.send(format!("{map:?}"));
            }
            ControlCommand::Nearby { reply } => {
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
                    if d < 600.0 {
                        let vp = viewport(tf);
                        arr.push(json!({"kind": "monster", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32), "vp": vp.map(|(x, y)| json!({"x": x, "y": y}))}));
                    }
                }
                for (tf, name, oid) in q.npcs.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        let vp = viewport(tf);
                        arr.push(json!({"kind": "npc", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32), "vp": vp.map(|(x, y)| json!({"x": x, "y": y}))}));
                    }
                }
                for (tf, name, oid) in q.others.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        let vp = viewport(tf);
                        arr.push(json!({"kind": "player", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32), "vp": vp.map(|(x, y)| json!({"x": x, "y": y}))}));
                    }
                }
                for (tf, item, oid) in q.items.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
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
                    "direction": anim.direction,
                    "chat_input_active": chat.input_active,
                    "chat_input_text": chat.input_text,
                    "ime_enabled": ime.enabled(),
                    "ime_composing": ime.composing_text(),
                })
                .to_string();
                let _ = reply.send(s);
            }
            ControlCommand::Attack { object_id } => {
                control_state.attack_target = Some(object_id);
                control_state.last_attack = 0.0;
                if let Ok((pe, _, _)) = q.players.single() {
                    commands.entity(pe).remove::<LocalMove>();
                }
                tracing::info!("🎮 control attack: {object_id}");
            }
            ControlCommand::Interact { object_id } => {
                control_state.npc_id = Some(object_id);
                control_state.last_npc_call = time.elapsed_secs();
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("🎮 control interact: {object_id}");
            }
            ControlCommand::NpcCall { object_id, key } => {
                tracing::info!("🎮 control npc_call: {object_id} {key}");
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC { object_id, key });
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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// parse_dialog_kind 覆盖除 GuestTrade 外全部变体 + 未知返回 None（#2586）
    /// GuestTrade 由网络 trade 会话驱动无独立开关，刻意不做 RPC 映射（批M 审查）
    #[test]
    fn parse_dialog_kind_covers_all_variants() {
        let all = [
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
        // #2599：trust_merchant/npc_drop 是历史别名（→ Market/Npc，真实现移壳后保留工具兼容），
        // 与 market/npc 重复映射——互异断言计数时先去掉这 2 个别名。
        // 名单与 witness 一致：每个可解析名都有 RPC 映射；DialogKind 共 47 个变体，
        // GuestTrade 刻意排除——枚举级穷尽由 has_rpc_mapping 的无通配 match 编译期保证）
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
            46,
            "48 个名字（含 trust_merchant/npc_drop 两个别名）应映射到 46 个不同变体"
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
}
