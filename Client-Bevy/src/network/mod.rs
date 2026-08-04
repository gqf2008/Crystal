// ============================================================================
// 网络模块（M5：mock 打通流程；M7：真实 TCP 接入 ServerRust）
// ============================================================================

pub mod codec;
pub mod mock;
pub mod server_event;
pub mod tcp;
pub mod packets;
use packets::handle_packet;
mod reconnect;
mod wire;
mod wire2;

pub use reconnect::NetServerAddr;
pub use wire::*;
pub use wire2::*;
use reconnect::setup_network;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use server_event::ServerEvent;
use mir2_shared::enums::ServerPacketIds;
use mir2_shared::packets::base::{Packet, PacketHeader};
use mir2_shared::SelectInfo;
use std::path::Path;

use crate::game::chat::ChatState;
use crate::game::combat::CombatEvent;
use crate::game::dialogs::friend::{FriendEntry, FriendState};
use crate::game::dialogs::guild::{GuildMember as UiGuildMember, GuildState, StorageItem};
use crate::game::dialogs::ranking::{RankEntry, RankingState};
use crate::game::dialogs::group::GroupState;
use crate::game::dialogs::mail::{MailDetail, MailEntry, MailState};
use crate::game::dialogs::mentor::MentorState;
use crate::game::dialogs::market::{MarketItem, MarketState};
use crate::game::dialogs::game_shop::{GameShopState, ShopItem as UiShopItem};
use crate::game::dialogs::guild_territory::{GuildTerritoryState, TerritoryRow};
use crate::game::dialogs::fishing::FishingState;
use crate::game::dialogs::refine::RefineState;
use crate::game::dialogs::craft::CraftState;
use crate::game::dialogs::item_rental::ItemRentalState;
use crate::game::dialogs::quest_log::{QuestEntry, QuestLogState};
use crate::game::dialogs::buff::{BuffEntry, BuffState};
use crate::game::dialogs::report::ReportState;
use crate::game::dialogs::inspect::{InspectItem, InspectState};
use crate::game::dialogs::creature::{CreatureEntry, CreatureState};
use crate::game::dialogs::hero::HeroState;
use crate::game::dialogs::relationship::RelationshipState;
use crate::game::effects::PendingEffect;
use crate::game::player_control::ControlState;
use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::npc::NpcDialogState;
use crate::game::dialogs::npc_goods::{GoodsEntry, NpcGoodsState};
use crate::game::dialogs::sell_panel::SellPanelState;
 use crate::game::dialogs::storage::StorageState;
use crate::game::dialogs::trade::{TradeItem as UiTradeItem, TradeState};
use crate::game::hud::HudState;
use crate::game::movement::NetMotion;
use crate::game::skills::MagicsState;
use crate::game::weather::WeatherState;
use crate::map_renderer::GameData;
use crate::scenes::AppState;
use crate::ui::login::AuthFeedback;

/// 网络模式（M7：--real-net 走真实 TCP，默认 mock 便于离线开发）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NetworkMode {
    #[default]
    Mock,
    Real,
}

/// 从 config.ini 读取网络配置（ServerAddr / UseMock / 记住的账号）
pub struct RuntimeConfig {
    pub server_addr: String,
    pub use_mock: bool,
    pub saved_account: String,
}

pub fn load_runtime_config() -> RuntimeConfig {
    let mut cfg = RuntimeConfig {
        server_addr: "127.0.0.1:7000".to_string(),
        use_mock: true,
        saved_account: String::new(),
    };
    // config.ini 优先：工作目录，其次 crate 根
    let mut content = std::fs::read_to_string("config.ini").ok();
    if content.is_none() {
        content =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("config.ini")).ok();
    }
    let Some(content) = content else { return cfg };
    let mut section = String::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let value = v.trim();
        if section.eq_ignore_ascii_case("Network") {
            if key.eq_ignore_ascii_case("ServerAddr") && !value.is_empty() {
                cfg.server_addr = value.to_string();
            } else if key.eq_ignore_ascii_case("UseMock") {
                cfg.use_mock = matches!(
                    value.to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "y" | "on"
                );
            }
        } else if section.eq_ignore_ascii_case("Login") && key.eq_ignore_ascii_case("Account") {
            cfg.saved_account = value.to_string();
        }
    }
    cfg
}

/// 网络模式资源（Startup 时由命令行参数决定）
#[derive(Resource)]
pub struct NetMode(pub NetworkMode);

/// 解析网络模式：--real-net [addr] 走真实 TCP；--mock 强制 mock（默认）
fn resolve_net_mode() -> (NetworkMode, String) {
    let runtime = load_runtime_config();
    let args: Vec<String> = std::env::args().collect();
    let mut addr = runtime.server_addr.clone();
    let mut real = !runtime.use_mock;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--real-net" => {
                real = true;
                // 可选下一个参数作为地址
                if let Some(next) = args.get(i + 1) {
                    if !next.starts_with("--") && next.contains(':') {
                        addr = next.clone();
                        i += 1;
                    }
                }
            }
            "--mock" => real = false,
            _ => {}
        }
        i += 1;
    }
    if real {
        tracing::info!("🌐 网络模式: 真实 TCP -> {}", addr);
        (NetworkMode::Real, addr)
    } else {
        tracing::info!("🌐 网络模式: Mock（本地模拟服务器，--real-net 切换真实 TCP）");
        (NetworkMode::Mock, addr)
    }
}

/// 网络状态
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NetState {
    #[default]
    Offline,
    LoggingIn,
    Select,
    InGame,
}

/// 网络连接资源（#66 拆分：只保留连接/传输职责）
#[derive(Resource)]
pub struct NetConnection {
    /// 发往服务器的内层包（PacketHeader+body）
    pub to_server: Option<Sender<Vec<u8>>>,
    /// Mock 模式：从服务器接收的外帧编码字节
    pub from_server: Option<Receiver<Vec<u8>>>,
    /// 真实 TCP 模式：服务器事件（完整内层包 / 断线）
    pub tcp_events: Option<Receiver<tcp::TcpEvent>>,
    pub mode: NetworkMode,
    pub state: NetState,
    /// 与服务器断开的原因
    pub disconnected: Option<String>,
    /// ClientVersion 的 16 字节版本哈希（服务端 CheckVersion 时需匹配）
    pub client_version_hash: [u8; 16],
    /// 是否已发送 ClientVersion（每次连接只发一次）
    pub client_version_sent: bool,
    /// M58 自动重连：断线后自动重连并重新登录（默认开启）
    pub auto_reconnect: bool,
    /// 是否处于自动重连流程
    pub reconnecting: bool,
    /// 重连倒计时（秒）
    pub reconnect_timer: f32,
    /// 重连延迟（指数退避，2→4→8...最大 30 秒）
    pub reconnect_delay: f32,
    /// 重连尝试次数
    pub reconnect_attempts: u32,
    /// 最近登录凭据（send_packet 捕获 Login 包，自动重连用）
    pub saved_login: std::sync::Arc<std::sync::Mutex<Option<(String, String)>>>,
    /// 最近选择的角色下标（自动重连后自动进游戏）
    pub saved_character: std::sync::Arc<std::sync::Mutex<Option<i32>>>,
}

impl Default for NetConnection {
    fn default() -> Self {
        Self {
            to_server: None,
            from_server: None,
            tcp_events: None,
            mode: NetworkMode::Mock,
            state: NetState::Offline,
            disconnected: None,
            client_version_hash: [0u8; 16],
            client_version_sent: false,
            auto_reconnect: true,
            reconnecting: false,
            reconnect_timer: 0.0,
            reconnect_delay: 2.0,
            reconnect_attempts: 0,
            saved_login: std::sync::Arc::new(std::sync::Mutex::new(None)),
            saved_character: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl NetConnection {
    /// 发送客户端包（serialize 内层 → 发送）
    /// M58：顺带捕获 Login/StartGame 用于自动重连
    pub fn send_packet<P: Packet + 'static>(&self, packet: &P) {
        if let Some(login) = (packet as &dyn std::any::Any)
            .downcast_ref::<mir2_shared::packets::client::account::Login>()
        {
            if let Ok(mut g) = self.saved_login.lock() {
                *g = Some((login.account_id.clone(), login.password.clone()));
            }
        }
        if let Some(start) = (packet as &dyn std::any::Any)
            .downcast_ref::<mir2_shared::packets::client::account::StartGame>()
        {
            if let Ok(mut g) = self.saved_character.lock() {
                *g = Some(start.character_index);
            }
        }
        if let Some(tx) = &self.to_server {
            let mut inner = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut inner, packet).is_ok() {
                tracing::debug!("📤 send_packet opcode={} len={}", P::OPCODE, inner.len());
                let _ = tx.send(inner);
            }
        }
    }
}

/// 会话状态（#66 拆分：角色列表/选中/本地玩家/位置同步等非传输职责）
#[derive(Resource, Default)]
pub struct SessionState {
    /// 角色列表（LoginSuccess 携带）
    pub characters: Vec<SelectInfo>,
    /// 新建角色成功后置 true，选角界面需要重建槽位
    pub select_reload: bool,
    /// 新建角色被服务器拒绝时的提示
    pub character_error: Option<String>,
    /// 选中的角色
    pub selected_index: Option<i32>,
    /// 本地玩家 object_id（UserInformation 提供；mock 模式为 None=第一个 ObjectPlayer）
    pub local_player_id: Option<u32>,
    /// 服务器 UserLocation 权威位置（瓦片坐标 + 朝向），由移动系统消费
    pub self_position: Option<(i32, i32, u8)>,
}

/// 行会仓库物品存取包（M32）
/// 注意：ServerRust gate 实际解析 wire 为
/// `[change_type u8][grid u8][unique_id u64][count u32]`
/// （与 SharedRust 客户端包结构 [u8][i32][i32] 不一致），
/// 以服务端 gate 解析为准手动构造。
#[derive(Message, Debug, Clone)]
pub enum NetObject {
    Player {
        object_id: u32,
        name: String,
        class: mir2_shared::MirClass,
        gender: mir2_shared::MirGender,
        location_x: i32,
        location_y: i32,
        direction: u8,
        hair: u8,
        weapon: i16,
        weapon_effect: i16,
        armour: i16,
        wing_effect: u8,
        /// M60 坐骑
        mount_type: i16,
        is_mounted: bool,
    },
    Monster {
        object_id: u32,
        name: String,
        location_x: i32,
        location_y: i32,
        image: u16,
        direction: u8,
    },
    Npc {
        object_id: u32,
        name: String,
        image: u16,
        location_x: i32,
        location_y: i32,
        direction: u8,
    },
    /// 地面物品（ObjectItem）
    GroundItem {
        object_id: u32,
        item: InvItem,
        location_x: i32,
        location_y: i32,
    },
}

/// 服务器对象移除消息（ObjectRemove）
#[derive(Message, Debug, Clone, Copy)]
pub struct NetObjectRemoved(pub u32);
pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let (mode, addr) = resolve_net_mode();
        app.insert_resource(NetMode(mode));
        app.insert_resource(NetServerAddr(addr));
        app.init_resource::<NetConnection>();
        app.init_resource::<SessionState>();
        app.init_resource::<AuthFeedback>();
        app.add_message::<NetObject>();
        app.add_message::<NetObjectRemoved>();
        app.add_message::<ServerEvent>();
        app.add_systems(Startup, setup_network);
        app.add_systems(Update, network_system);
    }
}

/// 真实 TCP 服务器地址资源
#[derive(SystemParam)]
pub(crate) struct NetworkPanels<'w> {
    storage: ResMut<'w, StorageState>,
    sell_panel: ResMut<'w, SellPanelState>,
    group: ResMut<'w, GroupState>,
    mail: ResMut<'w, MailState>,
    trade: ResMut<'w, TradeState>,
    friend: ResMut<'w, FriendState>,
    guild: ResMut<'w, GuildState>,
    ranking: ResMut<'w, RankingState>,
    mentor: ResMut<'w, MentorState>,
    market: ResMut<'w, MarketState>,
    shop: ResMut<'w, GameShopState>,
    territory: ResMut<'w, GuildTerritoryState>,
    control: ResMut<'w, ControlState>,
    fishing: ResMut<'w, FishingState>,
    refine: ResMut<'w, RefineState>,
    craft: ResMut<'w, CraftState>,
    rental: ResMut<'w, ItemRentalState>,
    quest_log: ResMut<'w, QuestLogState>,
    buff: ResMut<'w, BuffState>,
    report: ResMut<'w, ReportState>,
    inspect: ResMut<'w, InspectState>,
    creature: ResMut<'w, CreatureState>,
    hero: ResMut<'w, HeroState>,
    relationship: ResMut<'w, RelationshipState>,
    big_map: ResMut<'w, crate::game::dialogs::big_map::BigMapState>,
    awake: ResMut<'w, crate::game::dialogs::npc_awake::NpcAwakeState>,
    roll: ResMut<'w, crate::game::dialogs::roll::RollState>,
    mgr: ResMut<'w, crate::game::dialogs::DialogManager>,
}

/// 网络→游戏消息出口（对象/移动/战斗/特效；MessageWriter 替代手写 Vec 队列）
#[derive(SystemParam)]
pub(crate) struct NetworkOutbox<'w> {
    net_objects: MessageWriter<'w, NetObject>,
    net_removals: MessageWriter<'w, NetObjectRemoved>,
    motions: MessageWriter<'w, NetMotion>,
    combat: MessageWriter<'w, CombatEvent>,
    effects: MessageWriter<'w, PendingEffect>,
    /// 服务端事件（HUD/UI 等按需消费，解耦 network_system 直接改 UI State）
    server_events: MessageWriter<'w, ServerEvent>,
}

/// 网络系统：拉取服务器数据 → 解析包 → 分发处理
pub(crate) fn network_system(
    mut net: ResMut<NetConnection>,
    mut session: ResMut<SessionState>,
    mut auth: ResMut<AuthFeedback>,
    mut game_data: ResMut<GameData>,
    mut outbox: NetworkOutbox,
    mut hud: ResMut<HudState>,
    mut chat: ResMut<ChatState>,
    mut npc_dialog: ResMut<NpcDialogState>,
    mut npc_goods: ResMut<NpcGoodsState>,
    mut weather: ResMut<WeatherState>,
    mut magics: ResMut<MagicsState>,
    mut panels: NetworkPanels,
    mut next: ResMut<NextState<AppState>>,
    time: Res<Time>,
    addr: Res<NetServerAddr>,
) {
    // M58：断线自动重连（真实 TCP，指数退避）
    if net.mode == NetworkMode::Real && net.reconnecting && net.to_server.is_none() {
        net.reconnect_timer -= time.delta_secs();
        if net.reconnect_timer <= 0.0 {
            match tcp::connect(&addr.0, net.client_version_hash) {
                Ok(conn) => {
                    net.to_server = Some(conn.to_server);
                    net.tcp_events = Some(conn.from_server);
                    net.client_version_sent = false;
                    net.state = NetState::LoggingIn;
                    auth.login_error = Some("连接已恢复，正在重新登录...".to_string());
                    let creds = net.saved_login.lock().ok().map(|g| g.clone()).flatten();
                    if let Some((acct, pass)) = creds {
                        net.send_packet(&mir2_shared::packets::client::account::Login {
                            account_id: acct,
                            password: pass,
                        });
                        tracing::info!("🔌 自动重连成功，已重新发送登录请求");
                    } else {
                        net.reconnecting = false;
                        auth.login_error = Some("连接已恢复，请重新登录".to_string());
                    }
                }
                Err(e) => {
                    net.reconnect_attempts += 1;
                    net.reconnect_delay = (net.reconnect_delay * 2.0).min(30.0);
                    net.reconnect_timer = net.reconnect_delay;
                    auth.login_error = Some(format!(
                        "重连失败（{}），{:.0} 秒后重试（第 {} 次）",
                        e, net.reconnect_delay, net.reconnect_attempts
                    ));
                    tracing::warn!("🔌 重连失败: {}（第 {} 次）", e, net.reconnect_attempts);
                }
            }
        }
    }

    // 真实 TCP：TcpEvent（完整内层包 / 断线）
    if let Some(rx) = net.tcp_events.clone() {
        while let Ok(ev) = rx.try_recv() {
            match ev {
                tcp::TcpEvent::Packet(payload) => {
                    handle_packet(
                        &mut net,
                        &mut session,
                        &mut auth,
                        &mut game_data,
                        &mut outbox.net_objects,
                        &mut outbox.net_removals,
                        &mut outbox.motions,
                        &mut chat,
                        &mut npc_goods,
                        &mut outbox.combat,
                        &mut weather,
                        &mut *panels.storage,
                        &mut *panels.sell_panel,
                        &mut *panels.group,
                        &mut *panels.mail,
                        &mut *panels.trade,
                        &mut *panels.friend,
                        &mut *panels.guild,
                        &mut *panels.ranking,
                        &mut *panels.mentor,
                        &mut *panels.market,
                        &mut *panels.shop,
                        &mut *panels.territory,
                        &mut outbox.effects,
                        &mut outbox.server_events,
                        &mut *panels.control,
                        &mut *panels.fishing,
                        &mut *panels.refine,
                        &mut *panels.craft,
                        &mut *panels.rental,
                        &mut *panels.quest_log,
                        &mut *panels.buff,
                        &mut *panels.report,
                        &mut *panels.inspect,
                        &mut *panels.creature,
                        &mut *panels.hero,
                        &mut *panels.relationship,
                        &mut *panels.big_map,
                        &mut *panels.awake,
                        &mut *panels.roll,
                        &mut *panels.mgr,
                        &mut next,
                        &payload,
                    );
                }
                tcp::TcpEvent::Disconnected { reason } => {
                    tracing::warn!("🔌 与服务器断开: {}", reason);
                    net.state = NetState::Offline;
                    net.disconnected = Some(reason.clone());
                    auth.login_success = false;
                    net.to_server = None;
                    net.tcp_events = None;
                    if net.auto_reconnect {
                        net.reconnecting = true;
                        net.reconnect_delay = 2.0;
                        net.reconnect_timer = 2.0;
                        auth.login_error = Some(format!("连接断开，2 秒后自动重连...（{}）", reason));
                    } else {
                        auth.login_error = Some(format!("与服务器断开连接：{}", reason));
                    }
                }
            }
        }
        return;
    }

    // Mock：外帧解码 → 内层包 → 分发
    let Some(rx) = net.from_server.clone() else {
        return;
    };
    let mut buf: Vec<u8> = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        buf.extend_from_slice(&bytes);
        loop {
            match codec::decode(&buf) {
                Some(Ok((payload, consumed))) => {
                    buf.drain(..consumed);
                    handle_packet(
                        &mut net,
                        &mut session,
                        &mut auth,
                        &mut game_data,
                        &mut outbox.net_objects,
                        &mut outbox.net_removals,
                        &mut outbox.motions,
                        &mut chat,
                        &mut npc_goods,
                        &mut outbox.combat,
                        &mut weather,
                        &mut *panels.storage,
                        &mut *panels.sell_panel,
                        &mut *panels.group,
                        &mut *panels.mail,
                        &mut *panels.trade,
                        &mut *panels.friend,
                        &mut *panels.guild,
                        &mut *panels.ranking,
                        &mut *panels.mentor,
                        &mut *panels.market,
                        &mut *panels.shop,
                        &mut *panels.territory,
                        &mut outbox.effects,
                        &mut outbox.server_events,
                        &mut *panels.control,
                        &mut *panels.fishing,
                        &mut *panels.refine,
                        &mut *panels.craft,
                        &mut *panels.rental,
                        &mut *panels.quest_log,
                        &mut *panels.buff,
                        &mut *panels.report,
                        &mut *panels.inspect,
                        &mut *panels.creature,
                        &mut *panels.hero,
                        &mut *panels.relationship,
                        &mut *panels.big_map,
                        &mut *panels.awake,
                        &mut *panels.roll,
                        &mut *panels.mgr,
                        &mut next,
                        &payload,
                    );
                    if buf.is_empty() {
                        break;
                    }
                }
                Some(Err(e)) => {
                    tracing::warn!("⚠️ 帧解码失败: {}", e);
                    buf.clear();
                    break;
                }
                None => break,
            }
        }
    }
}
