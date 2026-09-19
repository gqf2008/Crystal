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
    dialog_roots: Query<'w, 's, (&'static DialogRoot, &'static Node, &'static Visibility)>,
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
        // 🐛 实机交互验证诊断：抓「关闭钮 Visibility 被谁写入」——on_insert 钩子打印回溯
        #[cfg(debug_assertions)]
        {
            app.world_mut()
                .register_component_hooks::<Visibility>()
                .on_insert(|mut world, ctx| {
                    let has_close = world
                        .entity(ctx.entity)
                        .contains::<crate::ui::theme::CloseButton>();
                    if has_close {
                        let vis = world.entity(ctx.entity).get::<Visibility>().copied();
                        let bt = std::backtrace::Backtrace::force_capture();
                        tracing::warn!(
                            "🪝 closebtn {:?} Visibility INSERT {:?}\n{}",
                            ctx.entity,
                            vis,
                            bt
                        );
                    }
                });
        }
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
        | ControlCommand::Nearby { reply }
        | ControlCommand::Cursor { reply, .. }
        | ControlCommand::PlayerMenu { reply, .. }
        | ControlCommand::QuestDetail { reply, .. }
        | ControlCommand::CharPage { reply, .. }
        | ControlCommand::ChatSize { reply, .. }
        | ControlCommand::Click { reply, .. }
        | ControlCommand::DialogRect { reply, .. }
        | ControlCommand::GetScroll { reply } => Some(reply),
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
                for (root, _node, vis) in &q.dialog_roots {
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
                        let pv = q.dialog_roots.get(parent).ok().map(|(_, _, v)| *v);
                        let pv2 = q.all_visibility.get(parent).ok().copied();
                        anc.push_str(&format!(" >{parent:?} vis={pv2:?}"));
                        if let Ok((root, _n, rvis)) = q.dialog_roots.get(parent) {
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
                    for _ in 0..32 {
                        if let Ok((root, n, vis)) = q.dialog_roots.get(cur) {
                            if *vis == Visibility::Visible {
                                owner = Some(root.0);
                                root_node = Some(n);
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
                        let (rx, ry, rw, rh) = root_node
                            .map(|n| crate::game::dialogs::node_rect(n))
                            .unwrap_or((0.0, 0.0, 0.0, 0.0));
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
