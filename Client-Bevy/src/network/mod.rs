// ============================================================================
// 网络模块（M5：mock 打通流程；M7：真实 TCP 接入 ServerRust）
// ============================================================================

pub mod codec;
pub mod mock;
pub mod tcp;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use mir2_shared::enums::ServerPacketIds;
use mir2_shared::packets::base::{Packet, PacketHeader};
use mir2_shared::SelectInfo;
use std::path::Path;

use crate::game::chat::ChatState;
use crate::game::combat::CombatEvents;
use crate::game::dialogs::group::GroupState;
use crate::game::dialogs::mail::{MailDetail, MailEntry, MailState};
use crate::game::dialogs::inventory::InvItem;
use crate::game::dialogs::npc::NpcDialogState;
use crate::game::dialogs::npc_goods::{GoodsEntry, NpcGoodsState};
use crate::game::dialogs::sell_panel::SellPanelState;
 use crate::game::dialogs::storage::StorageState;
use crate::game::dialogs::trade::{TradeItem as UiTradeItem, TradeState};
use crate::game::hud::HudState;
use crate::game::movement::{NetMotion, NetMotions};
use crate::game::skills::MagicsState;
use crate::game::weather::WeatherState;
use crate::map_renderer::GameData;
use crate::scenes::AppState;

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

/// 网络上下文（Bevy Resource）
#[derive(Resource)]
pub struct NetworkContext {
    /// 发往服务器的内层包（PacketHeader+body）
    pub to_server: Option<Sender<Vec<u8>>>,
    /// Mock 模式：从服务器接收的外帧编码字节
    pub from_server: Option<Receiver<Vec<u8>>>,
    /// 真实 TCP 模式：服务器事件（完整内层包 / 断线）
    pub tcp_events: Option<Receiver<tcp::TcpEvent>>,
    pub mode: NetworkMode,
    pub state: NetState,
    /// 角色列表（LoginSuccess 携带）
    pub characters: Vec<SelectInfo>,
    /// 新建角色成功后置 true，选角界面需要重建槽位
    pub select_reload: bool,
    /// 新建角色被服务器拒绝时的提示
    pub character_error: Option<String>,
    /// 选中的角色
    pub selected_index: Option<i32>,
    /// 登录错误信息（登录失败 / 连接失败 / 断线）
    pub login_error: Option<String>,
    /// 登录成功标志（LoginScene 播放 ChrSel 动画后进选角）
    pub login_success: bool,
    /// 注册新账号错误信息
    pub new_account_error: Option<String>,
    /// 注册新账号成功（UI 关闭对话框并提示）
    pub new_account_success: bool,
    /// 修改密码错误信息
    pub change_password_error: Option<String>,
    /// 修改密码成功
    pub change_password_success: bool,
    /// 与服务器断开的原因
    pub disconnected: Option<String>,
    /// ClientVersion 的 16 字节版本哈希（服务端 CheckVersion 时需匹配）
    pub client_version_hash: [u8; 16],
    /// 是否已发送 ClientVersion（每次连接只发一次）
    pub client_version_sent: bool,
    /// 本地玩家 object_id（UserInformation 提供；mock 模式为 None=第一个 ObjectPlayer）
    pub local_player_id: Option<u32>,
    /// 服务器 UserLocation 权威位置（瓦片坐标 + 朝向），由移动系统消费
    pub self_position: Option<(i32, i32, u8)>,
}

impl Default for NetworkContext {
    fn default() -> Self {
        Self {
            to_server: None,
            from_server: None,
            tcp_events: None,
            mode: NetworkMode::Mock,
            state: NetState::Offline,
            characters: Vec::new(),
            select_reload: false,
            character_error: None,
            selected_index: None,
            login_error: None,
            login_success: false,
            new_account_error: None,
            new_account_success: false,
            change_password_error: None,
            change_password_success: false,
            disconnected: None,
            client_version_hash: [0u8; 16],
            client_version_sent: false,
            local_player_id: None,
            self_position: None,
        }
    }
}

impl NetworkContext {
    /// 发送客户端包（serialize 内层 → 发送）
    pub fn send_packet<P: Packet>(&self, packet: &P) {
        if let Some(tx) = &self.to_server {
            let mut inner = Vec::new();
            if mir2_shared::packets::base::serialize_packet(&mut inner, packet).is_ok() {
                let _ = tx.send(inner);
            }
        }
    }
}

/// 待生成的网络对象（MapChanged 后由 Game 状态消费）
#[derive(Debug, Clone)]
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

#[derive(Resource, Default)]
pub struct NetObjects {
    pub pending: Vec<NetObject>,
    /// 待移除的服务器对象 ID（ObjectRemove）
    pub to_remove: Vec<u32>,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let (mode, addr) = resolve_net_mode();
        app.insert_resource(NetMode(mode));
        app.insert_resource(NetServerAddr(addr));
        app.init_resource::<NetworkContext>();
        app.init_resource::<NetObjects>();
        app.add_systems(Startup, setup_network);
        app.add_systems(Update, network_system);
    }
}

/// 真实 TCP 服务器地址资源
#[derive(Resource)]
pub struct NetServerAddr(pub String);

/// 启动网络（按模式：mock 或真实 TCP）
fn setup_network(mut net: ResMut<NetworkContext>, mode: Res<NetMode>, addr: Res<NetServerAddr>) {
    match mode.0 {
        NetworkMode::Mock => {
            let (to_server, from_client) = crossbeam_channel::bounded::<Vec<u8>>(1024);
            let (to_client, from_server) = crossbeam_channel::bounded::<Vec<u8>>(1024);
            net.to_server = Some(to_server);
            net.from_server = Some(from_server);
            mock::spawn_mock(to_client, from_client);
            tracing::info!("🌐 Mock 网络已启动（本地模拟服务器）");
        }
        NetworkMode::Real => match tcp::connect(&addr.0, net.client_version_hash) {
            Ok(conn) => {
                net.to_server = Some(conn.to_server);
                net.tcp_events = Some(conn.from_server);
                net.mode = NetworkMode::Real;
                tracing::info!("🌐 真实 TCP 已连接: {}", addr.0);
            }
            Err(e) => {
                tracing::error!("🔌 连接服务器 {} 失败: {}", addr.0, e);
                net.login_error = Some(format!("无法连接服务器 {}：{}", addr.0, e));
                net.disconnected = Some(format!("{}", e));
            }
        },
    }
}

/// 网络系统参数（Bevy 16 参数上限：合并对话框状态）
#[derive(SystemParam)]
struct NetworkPanels<'w> {
    storage: ResMut<'w, StorageState>,
    sell_panel: ResMut<'w, SellPanelState>,
    group: ResMut<'w, GroupState>,
    mail: ResMut<'w, MailState>,
    trade: ResMut<'w, TradeState>,
    mgr: ResMut<'w, crate::game::dialogs::DialogManager>,
}

/// 网络系统：拉取服务器数据 → 解析包 → 分发处理
fn network_system(
    mut net: ResMut<NetworkContext>,
    mut game_data: ResMut<GameData>,
    mut net_objects: ResMut<NetObjects>,
    mut motions: ResMut<NetMotions>,
    mut hud: ResMut<HudState>,
    mut chat: ResMut<ChatState>,
    mut npc_dialog: ResMut<NpcDialogState>,
    mut npc_goods: ResMut<NpcGoodsState>,
    mut combat_evt: ResMut<CombatEvents>,
    mut weather: ResMut<WeatherState>,
    mut magics: ResMut<MagicsState>,
    mut panels: NetworkPanels,
    mut next: ResMut<NextState<AppState>>,
) {
    // 真实 TCP：TcpEvent（完整内层包 / 断线）
    if let Some(rx) = net.tcp_events.clone() {
        while let Ok(ev) = rx.try_recv() {
            match ev {
                tcp::TcpEvent::Packet(payload) => {
                    handle_packet(
                        &mut net,
                        &mut game_data,
                        &mut net_objects,
                        &mut motions,
                        &mut hud,
                        &mut chat,
                        &mut npc_dialog,
                        &mut npc_goods,
                        &mut combat_evt,
                        &mut weather,
                        &mut magics,
                        &mut *panels.storage,
                        &mut *panels.sell_panel,
                        &mut *panels.group,
                        &mut *panels.mail,
                        &mut *panels.trade,
                        &mut *panels.mgr,
                        &mut next,
                        &payload,
                    );
                }
                tcp::TcpEvent::Disconnected { reason } => {
                    tracing::warn!("🔌 与服务器断开: {}", reason);
                    net.state = NetState::Offline;
                    net.disconnected = Some(reason.clone());
                    net.login_error = Some(format!("与服务器断开连接：{}", reason));
                    net.login_success = false;
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
                        &mut game_data,
                        &mut net_objects,
                        &mut motions,
                        &mut hud,
                        &mut chat,
                        &mut npc_dialog,
                        &mut npc_goods,
                        &mut combat_evt,
                        &mut weather,
                        &mut magics,
                        &mut *panels.storage,
                        &mut *panels.sell_panel,
                        &mut *panels.group,
                        &mut *panels.mail,
                        &mut *panels.trade,
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

/// 把服务端 UserItem 转成客户端背包条目（含 ItemType/Shape 用于使用/装备判断）
fn to_inv_item(item: &mir2_shared::data::item::UserItem) -> InvItem {
    InvItem {
        unique_id: item.unique_id,
        item_index: item.item_index,
        name: item
            .info
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_else(|| format!("#{}", item.item_index)),
        image: item.info.as_ref().map(|i| i.image).unwrap_or(0),
        count: item.count,
        item_type: item.info.as_ref().map(|i| i.item_type as u8).unwrap_or(0),
        shape: item.info.as_ref().map(|i| i.shape).unwrap_or(0),
    }
}

/// 解析服务端 ReceiveMail（同 opcode 双格式）：
/// - 条目包：mail_id, sender, subject, timestamp, read, collected, gold, item_count
/// - 全文包：mail_id, sender, subject, body, timestamp, read, collected, gold, item_count, items...
/// 先尝试全文格式，失败再按条目格式（条目包按全文解析时 timestamp 首字节必然导致 7-bit 长度越界）
fn parse_receive_mail(payload: &[u8]) -> Option<(MailEntry, Option<MailDetail>)> {
    use mir2_shared::binary::read_dotnet_string;
    use byteorder::{LittleEndian, ReadBytesExt};

    fn parse_content(
        payload: &[u8],
    ) -> Option<(MailEntry, Option<MailDetail>)> {
        let mut cur = std::io::Cursor::new(payload);
        let mail_id = cur.read_u64::<LittleEndian>().ok()?;
        let sender = read_dotnet_string(&mut cur).ok()?;
        let subject = read_dotnet_string(&mut cur).ok()?;
        let body = read_dotnet_string(&mut cur).ok()?;
        let _timestamp = cur.read_i64::<LittleEndian>().ok()?;
        let read_flag = cur.read_u8().ok()? != 0;
        let _collected = cur.read_u8().ok()? != 0;
        let gold = cur.read_u32::<LittleEndian>().ok()?;
        let item_count = cur.read_u8().ok()? as usize;
        let mut items = Vec::new();
        for _ in 0..item_count {
            let _uid = cur.read_u64::<LittleEndian>().ok()?;
            let _idx = cur.read_u32::<LittleEndian>().ok()?;
            let name = read_dotnet_string(&mut cur).ok()?;
            let _count = cur.read_u16::<LittleEndian>().ok()?;
            let _cd = cur.read_u16::<LittleEndian>().ok()?;
            let _md = cur.read_u16::<LittleEndian>().ok()?;
            items.push(name);
        }
        if payload.len() as u64 != cur.position() {
            return None;
        }
        Some((
            MailEntry {
                mail_id,
                sender: sender.clone(),
                subject: subject.clone(),
                unread: !read_flag,
                gold,
            },
            Some(MailDetail {
                mail_id,
                sender,
                subject,
                body,
                gold,
                items,
            }),
        ))
    }

    fn parse_entry(payload: &[u8]) -> Option<(MailEntry, Option<MailDetail>)> {
        let mut cur = std::io::Cursor::new(payload);
        let mail_id = cur.read_u64::<LittleEndian>().ok()?;
        let sender = read_dotnet_string(&mut cur).ok()?;
        let subject = read_dotnet_string(&mut cur).ok()?;
        let _timestamp = cur.read_i64::<LittleEndian>().ok()?;
        let read_flag = cur.read_u8().ok()? != 0;
        let _collected = cur.read_u8().ok()? != 0;
        let gold = cur.read_u32::<LittleEndian>().ok()?;
        let _item_count = cur.read_u8().ok()?;
        if payload.len() as u64 != cur.position() {
            return None;
        }
        Some((
            MailEntry {
                mail_id,
                sender,
                subject,
                unread: !read_flag,
                gold,
            },
            None,
        ))
    }

    parse_content(payload).or_else(|| parse_entry(payload))
}

/// 处理单个内层包
fn handle_packet(
    net: &mut NetworkContext,
    game_data: &mut GameData,
    net_objects: &mut NetObjects,
    motions: &mut NetMotions,
    hud: &mut HudState,
    chat: &mut ChatState,
    npc_dialog: &mut NpcDialogState,
    npc_goods: &mut NpcGoodsState,
    combat_evt: &mut CombatEvents,
    weather: &mut WeatherState,
    magics: &mut MagicsState,
    storage: &mut StorageState,
    sell_panel: &mut SellPanelState,
    group: &mut GroupState,
    mail: &mut MailState,
    trade: &mut TradeState,
    mgr: &mut crate::game::dialogs::DialogManager,
    next: &mut NextState<AppState>,
    payload: &[u8],
) {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return;
    };
    match header.opcode {
        // ---- M7: 握手 ----
        x if x == ServerPacketIds::Connected as i16 => {
            tracing::info!("🔌 服务器已连接（Connected），发送 ClientVersion");
            if !net.client_version_sent {
                net.client_version_sent = true;
                net.send_packet(&mir2_shared::packets::client::connection::ClientVersion {
                    version_hash: net.client_version_hash.to_vec(),
                });
            }
        }
        x if x == ServerPacketIds::ClientVersion as i16 => {
            if let Ok(p) = connection::ClientVersion::read_body(&mut cur) {
                tracing::info!("🔑 ClientVersion 校验结果: {}", p.result);
            }
        }

        // ---- M7: 认证 ----
        x if x == ServerPacketIds::NewAccount as i16 => {
            if let Ok(p) = login::NewAccount::read_body(&mut cur) {
                let msg = match p.result {
                    8 => {
                        net.new_account_success = true;
                        net.new_account_error = None;
                        "注册成功，请登录".to_string()
                    }
                    0 => "服务器暂时关闭注册".to_string(),
                    1 => "账号格式错误（3-15位字母数字）".to_string(),
                    2 => "密码格式错误（5-15位字母数字）".to_string(),
                    3 => "邮箱格式错误".to_string(),
                    4 => "用户名过长".to_string(),
                    5 => "密保问题过长".to_string(),
                    6 => "密保答案过长".to_string(),
                    7 => "账号已存在".to_string(),
                    _ => format!("注册失败（{}）", p.result),
                };
                tracing::info!("📝 NewAccount result={} {}", p.result, msg);
                if p.result != 8 {
                    net.new_account_error = Some(msg);
                }
            }
        }
        x if x == ServerPacketIds::ChangePassword as i16 => {
            if let Ok(p) = login::ChangePassword::read_body(&mut cur) {
                let msg = match p.result {
                    6 => {
                        net.change_password_success = true;
                        net.change_password_error = None;
                        "密码修改成功".to_string()
                    }
                    0 => "服务器关闭修改密码".to_string(),
                    1 => "账号格式错误".to_string(),
                    2 => "当前密码格式错误".to_string(),
                    3 => "新密码格式错误".to_string(),
                    4 => "账号不存在".to_string(),
                    5 => "当前密码错误".to_string(),
                    _ => format!("修改密码失败（{}）", p.result),
                };
                tracing::info!("🔑 ChangePassword result={} {}", p.result, msg);
                if p.result != 6 {
                    net.change_password_error = Some(msg);
                }
            }
        }
        x if x == ServerPacketIds::Login as i16 => {
            if let Ok(p) = login::Login::read_body(&mut cur) {
                let msg = match p.result {
                    0 => "服务器禁止登录".to_string(),
                    1 => "账号格式错误".to_string(),
                    2 => "密码格式错误".to_string(),
                    3 => "账号不存在".to_string(),
                    4 => "密码错误".to_string(),
                    _ => format!("登录失败（{}）", p.result),
                };
                tracing::warn!("⛔ 登录失败 result={} {}", p.result, msg);
                net.state = NetState::Offline;
                net.login_error = Some(msg);
            }
        }
        x if x == ServerPacketIds::LoginSuccess as i16 => {
            if let Ok(p) = login::LoginSuccess::read_body(&mut cur) {
                tracing::info!("✅ 登录成功，角色 {} 个", p.characters.len());
                net.characters = p.characters;
                net.select_reload = false;
                net.state = NetState::Select;
                net.login_error = None;
                net.login_success = true;
            }
        }

        // ---- 角色管理 ----
        x if x == ServerPacketIds::StartGame as i16 => {
            if let Ok(p) = login::StartGame::read_body(&mut cur) {
                tracing::info!("✅ 开始游戏 result={}", p.result);
                net.state = NetState::InGame;
            }
        }
        x if x == ServerPacketIds::NewCharacter as i16 => {
            if let Ok(p) = account::NewCharacter::read_body(&mut cur) {
                tracing::info!("⛔ 新建角色被拒绝 result={}", p.result);
                net.character_error = Some(match p.result {
                    4 => "最多只能创建4个角色！".to_string(),
                    _ => "创建角色失败！".to_string(),
                });
            }
        }
        x if x == ServerPacketIds::NewCharacterSuccess as i16 => {
            if let Ok(p) = account::NewCharacterSuccess::read_body(&mut cur) {
                tracing::info!("✅ 新建角色成功: {}", p.character.name);
                net.characters.push(SelectInfo {
                    index: p.character.index,
                    name: p.character.name.clone(),
                    level: p.character.level,
                    class: p.character.class,
                    gender: p.character.gender,
                    last_access: p.character.last_access,
                });
                net.select_reload = true;
            }
        }
        x if x == ServerPacketIds::DeleteCharacter as i16 => {
            if let Ok(p) = account::DeleteCharacter::read_body(&mut cur) {
                tracing::warn!("⛔ 删除角色被拒绝 result={}", p.result);
                net.character_error = Some(match p.result {
                    0 => "删除失败：不能删除当前在线角色".to_string(),
                    1 => "删除失败：角色不存在".to_string(),
                    _ => format!("删除角色失败（{}）", p.result),
                });
            }
        }
        x if x == ServerPacketIds::DeleteCharacterSuccess as i16 => {
            if let Ok(p) = account::DeleteCharacterSuccess::read_body(&mut cur) {
                tracing::info!("🗑️ 删除角色成功 idx={}", p.character_index);
                net.characters.retain(|c| c.index != p.character_index);
                net.selected_index = None;
                net.select_reload = true;
            }
        }

        // ---- 地图与对象 ----
        x if x == ServerPacketIds::MapChanged as i16 => {
            if let Ok(p) = map::MapChanged::read_body(&mut cur) {
                tracing::info!(
                    "🗺️ MapChanged: {} ({},{})",
                    p.file_name,
                    p.location_x,
                    p.location_y
                );
                game_data.desired_map = Some(p.file_name);
                game_data.player_spawn =
                    Some((p.location_x as f32, p.location_y as f32, p.direction));
                weather.code = p.weather;
                next.set(AppState::Game);
            }
        }
        x if x == ServerPacketIds::ObjectPlayer as i16 => {
            match objects::ObjectPlayer::read_body(&mut cur) {
            Ok(p) => {
                net_objects.pending.push(NetObject::Player {
                    object_id: p.object_id,
                    name: p.name,
                    class: p.class,
                    gender: p.gender,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    direction: p.direction as u8,
                    hair: p.hair,
                    weapon: p.weapon,
                    weapon_effect: p.weapon_effect,
                    armour: p.armour,
                    wing_effect: p.wing_effect,
                });
            }
            Err(e) => {
                tracing::warn!("⚠️ ObjectPlayer 解析失败: {} (len={})", e, payload.len());
            }
            }
        }
        x if x == ServerPacketIds::ObjectMonster as i16 => {
            if let Ok(p) = objects::ObjectMonster::read_body(&mut cur) {
                net_objects.pending.push(NetObject::Monster {
                    object_id: p.object_id,
                    name: p.name,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    image: p.image,
                    direction: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectNpc as i16 => {
            if let Ok(p) = objects::ObjectNpc::read_body(&mut cur) {
                net_objects.pending.push(NetObject::Npc {
                    object_id: p.object_id,
                    name: p.name,
                    image: p.image,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    direction: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectRemove as i16 => {
            if let Ok(p) = objects::ObjectRemove::read_body(&mut cur) {
                tracing::debug!("🗑️ ObjectRemove id={}", p.object_id);
                net_objects.to_remove.push(p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectItem as i16 => {
            match drops::ObjectItem::read_body(&mut cur) {
            Ok(p) => {
                let name = p
                    .item
                    .info
                    .as_ref()
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| format!("#{}", p.item.item_index));
                tracing::info!(
                    "📦 地面物品: {} (uid={}) @ ({},{})",
                    name,
                    p.item.unique_id,
                    p.location_x,
                    p.location_y
                );
                net_objects.pending.push(NetObject::GroundItem {
                    object_id: p.object_id,
                    item: to_inv_item(&p.item),
                    location_x: p.location_x,
                    location_y: p.location_y,
                });
            }
            Err(e) => tracing::warn!("⚠️ ObjectItem 解析失败: {} (len={})", e, payload.len()),
            }
        }
        // ---- M8: 玩家状态 ----
        x if x == ServerPacketIds::UserInformation as i16 => {
            match user::UserInformation::read_body(&mut cur) {
                Ok(p) => {
                    tracing::info!(
                        "👤 UserInformation: {} Lv.{} hp={} mp={} exp={}/{} gold={}",
                        p.name,
                        p.level,
                        p.hp,
                        p.mp,
                        p.experience,
                        p.max_experience,
                        p.gold
                    );
                    hud.name = p.name.clone();
                    hud.level = p.level;
                    hud.hp = p.hp;
                    hud.mp = p.mp;
                    hud.exp = p.experience;
                    hud.max_exp = p.max_experience.max(1);
                    hud.gold = p.gold;
                    hud.class = p.class as u8;
                    hud.player_object_id = Some(p.object_id);
                    net.local_player_id = Some(p.object_id);
                    net.self_position = Some((p.location_x, p.location_y, p.direction as u8));

                    // 背包（40 格）
                    if let Some(inv) = &p.inventory {
                        let items: Vec<Option<InvItem>> = inv
                            .iter()
                            .take(40)
                            .map(|slot| slot.as_ref().map(to_inv_item))
                            .collect();
                        hud.inventory.items = items;
                        hud.inventory.gold = p.gold;
                        tracing::info!(
                            "🎒 背包 {} 格（{} 件物品）",
                            hud.inventory.items.len(),
                            hud.inventory.items.iter().flatten().count()
                        );
                    }

                    // 装备（12 槽）
                    if let Some(equip) = &p.equipment {
                        hud.equipment = equip
                            .iter()
                            .map(|slot| slot.as_ref().map(to_inv_item))
                            .collect();
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ UserInformation 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        x if x == ServerPacketIds::HealthChanged as i16 => {
            if let Ok(p) = combat::HealthChanged::read_body(&mut cur) {
                hud.hp = p.hp as i32;
                hud.mp = p.mp as i32;
            }
        }
        x if x == ServerPacketIds::UserLocation as i16 => {
            if let Ok(p) = user::UserLocation::read_body(&mut cur) {
                net.self_position = Some((p.location_x, p.location_y, p.direction as u8));
            }
        }
        x if x == ServerPacketIds::GainedGold as i16 => {
            if let Ok(p) = drops::GainedGold::read_body(&mut cur) {
                hud.gold = p.gold;
            }
        }
        x if x == ServerPacketIds::GainExperience as i16 => {
            if let Ok(p) = experience::GainExperience::read_body(&mut cur) {
                hud.exp += p.amount as i64;
            }
        }
        x if x == ServerPacketIds::LevelChanged as i16 => {
            if let Ok(p) = experience::LevelChanged::read_body(&mut cur) {
                hud.level = p.level;
                hud.exp = p.experience;
                hud.max_exp = p.max_experience.max(1);
            }
        }

        // ---- M8: 对象移动与聊天 ----
        x if x == ServerPacketIds::ObjectTurn as i16 => {
            if let Ok(p) = objects::ObjectTurn::read_body(&mut cur) {
                motions.pending.push(NetMotion::Turn {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectWalk as i16 => {
            if let Ok(p) = objects::ObjectWalk::read_body(&mut cur) {
                motions.pending.push(NetMotion::Walk {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectRun as i16 => {
            if let Ok(p) = objects::ObjectRun::read_body(&mut cur) {
                motions.pending.push(NetMotion::Run {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::Chat as i16 => {
            if let Ok(p) = chat::Chat::read_body(&mut cur) {
                let color = chat_color(p.chat_type);
                chat.add_line(p.message, color);
            }
        }
        x if x == ServerPacketIds::ObjectChat as i16 => {
            if let Ok(p) = chat::ObjectChat::read_body(&mut cur) {
                let color = chat_color(p.chat_type);
                chat.add_line(p.text, color);
            }
        }

        // ---- M9: NPC 对话 ----
        x if x == ServerPacketIds::NPCResponse as i16 => {
            match npc_interaction::NPCResponse::read_body(&mut cur) {
            Ok(p) => {
                tracing::info!("🧙 NPC 对话: {} 行", p.page.len());
                npc_dialog.lines = p.page;
                npc_dialog.visible = true;
            }
            Err(e) => tracing::warn!("⚠️ NPCResponse 解析失败: {} (len={})", e, payload.len()),
            }
        }

        // ---- M10: 战斗反馈 ----
        x if x == ServerPacketIds::ObjectStruck as i16 => {
            if let Ok(p) = combat::ObjectStruck::read_body(&mut cur) {
                combat_evt.strikes.push((p.object_id, p.direction));
            }
        }
        x if x == ServerPacketIds::ObjectDied as i16 => {
            if let Ok(p) = combat::ObjectDied::read_body(&mut cur) {
                combat_evt.deaths.push((p.object_id, p.death_type));
            }
        }
        x if x == ServerPacketIds::DamageIndicator as i16 => {
            if let Ok(p) = combat::DamageIndicator::read_body(&mut cur) {
                combat_evt
                    .damages
                    .push((p.object_id, p.damage, p.damage_type));
            }
        }

        // ---- M9: NPC 商店 ----
        x if x == ServerPacketIds::NPCGoods as i16 => {
            // C# 协议：NPCGoods 的 body 是 gzip 压缩的（C# ServerPackets.NPCGoods.Compressed == true，
            // Rust SharedRust 对应 is_compressed()==true），必须解压后再解析。
            let mut body = Vec::new();
            let mut gz = flate2::read::GzDecoder::new(std::io::Cursor::new(
                &payload[PacketHeader::HEADER_SIZE..],
            ));
            match std::io::Read::read_to_end(&mut gz, &mut body) {
                Ok(_) => {
                    let mut cur = std::io::Cursor::new(body);
                    match npc_interaction::NPCGoods::read_body(&mut cur) {
                        Ok(p) => {
                            // C# 语义：Sell/Repair/SpecialRepair 面板 → 打开出售/修理面板（NPCDropDialog），
                            // 其余（Buy/BuySub/Craft）→ 商品对话框
                            if matches!(
                                p.panel_type,
                                mir2_shared::enums::PanelType::Sell
                                    | mir2_shared::enums::PanelType::Repair
                                    | mir2_shared::enums::PanelType::SpecialRepair
                            ) {
                                npc_goods.visible = false;
                                sell_panel.mode = Some(p.panel_type);
                                sell_panel.target = None;
                                sell_panel.visible = true;
                                tracing::info!("🧰 NPC 面板: {:?}", p.panel_type);
                                // C# NPCDropDialog.Show() 同时打开背包
                                if !mgr.is_open(crate::game::dialogs::DialogKind::Inventory) {
                                    mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
                                }
                                return;
                            }
                            let goods: Vec<GoodsEntry> = p
                                .list
                                .iter()
                                .map(|item| GoodsEntry {
                                    item_index: item.item_index,
                                    unique_id: item.unique_id,
                                    name: item
                                        .info
                                        .as_ref()
                                        .map(|i| i.name.clone())
                                        .unwrap_or_else(|| format!("#{}", item.item_index)),
                                    price: item.info.as_ref().map(|i| i.price).unwrap_or(0),
                                    count: item.count,
                                })
                                .collect();
                            tracing::info!("🏪 NPC 商品: {} 件 (rate={})", goods.len(), p.rate);
                            npc_goods.goods = goods;
                            npc_goods.selected = None;
                            npc_goods.visible = true;
                        }
                        Err(e) => {
                            tracing::warn!("⚠️ NPCGoods 解析失败: {} (len={})", e, payload.len())
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ NPCGoods gzip 解压失败: {} (len={})", e, payload.len())
                }
            }
        }

        // ---- M13: 物品操作响应 ----
        x if x == ServerPacketIds::MoveItem as i16 => {
            if let Ok(p) = item_operations::MoveItem::read_body(&mut cur) {
                if p.success && p.grid == mir2_shared::enums::MirGridType::Inventory {
                    let from = p.from as usize;
                    let to = p.to as usize;
                    if from < hud.inventory.items.len() && to < hud.inventory.items.len() {
                        hud.inventory.items.swap(from, to);
                        tracing::info!("📦 移动物品 {} -> {}", p.from, p.to);
                    }
                }
            }
        }

        // ---- M13: 装备/使用响应（本地同步） ----
        x if x == ServerPacketIds::EquipItem as i16 => {
            if let Ok(p) = item_operations::EquipItem::read_body(&mut cur) {
                if p.success {
                    let to = p.to as usize;
                    // 从背包移除并放入装备槽
                    let from_idx = hud
                        .inventory
                        .items
                        .iter()
                        .position(|s| s.as_ref().map(|it| it.unique_id) == Some(p.unique_id));
                    if let Some(from_idx) = from_idx {
                        let item = hud.inventory.items[from_idx].take();
                        if let Some(item) = item {
                            let name = item.name.clone();
                            if to < hud.equipment.len() {
                                let old = hud.equipment[to].take();
                                hud.equipment[to] = Some(item);
                                // 旧装备放回背包空格
                                if let Some(old) = old {
                                    if let Some(empty) = hud.inventory.items.iter_mut().find(|s| s.is_none()) {
                                        *empty = Some(old);
                                    }
                                }
                            }
                            tracing::info!("⚔️ 装备成功: {} -> 槽 {}", name, p.to);
                        }
                    }
                }
            }
        }
        x if x == ServerPacketIds::RemoveItem as i16 => {
            if let Ok(p) = item_operations::RemoveItem::read_body(&mut cur) {
                if p.success {
                    let item = hud
                        .equipment
                        .iter_mut()
                        .flatten()
                        .find(|it| it.unique_id == p.unique_id)
                        .cloned();
                    if let Some(item) = item {
                        for slot in hud.equipment.iter_mut() {
                            if slot.as_ref().map(|it| it.unique_id) == Some(p.unique_id) {
                                *slot = None;
                                break;
                            }
                        }
                        if let Some(empty) = hud.inventory.items.iter_mut().find(|s| s.is_none()) {
                            *empty = Some(item);
                        }
                        tracing::info!("🛡️ 卸下装备 uid={}", p.unique_id);
                    }
                }
            }
        }
        x if x == ServerPacketIds::UseItem as i16 => {
            if let Ok(p) = item_operations::UseItem::read_body(&mut cur) {
                let idx = hud
                    .inventory
                    .items
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.unique_id) == Some(p.unique_id));
                if let Some(idx) = idx {
                    let count = hud.inventory.items[idx].as_ref().map(|it| it.count).unwrap_or(0);
                    if count > 1 {
                        if let Some(it) = hud.inventory.items[idx].as_mut() {
                            it.count -= 1;
                        }
                    } else {
                        hud.inventory.items[idx] = None;
                    }
                    tracing::info!("💊 使用物品 uid={} 剩余 {}", p.unique_id, count.saturating_sub(1));
                }
            }
        }
        x if x == ServerPacketIds::SplitItem as i16 => {
            // 拆分响应后服务端会跟完整 UserInformation 刷新（权威重建背包）
            if let Ok(p) = item::SplitItem::read_body(&mut cur) {
                tracing::info!("🔪 拆分响应: grid={:?} uid={} count={}", p.grid, p.unique_id, p.count);
            }
        }
        x if x == ServerPacketIds::DropItem as i16 => {
            if let Ok(p) = item_operations::DropItem::read_body(&mut cur) {
                tracing::info!("🗑️ 丢弃响应: uid={} count={} success={}", p.unique_id, p.count, p.success);
            }
        }
        x if x == ServerPacketIds::MergeItem as i16 => {
            if let Ok(p) = item_operations::MergeItem::read_body(&mut cur) {
                tracing::info!("🧬 合并响应: from={} to={} success={}", p.id_from, p.id_to, p.success);
            }
        }
        x if x == ServerPacketIds::SellItem as i16 => {
            if let Ok(p) = item::SellItem::read_body(&mut cur) {
                tracing::info!("💰 出售响应: uid={} count={} success={}", p.unique_id, p.count, p.success);
            }
        }

        // ---- M13: 技能 ----

        // ---- M18: 仓库 ----
        x if x == ServerPacketIds::UserStorage as i16 => {
            match player::UserStorage::read_body(&mut cur) {
                Ok(p) => {
                    let items: Vec<Option<InvItem>> = p
                        .storage
                        .iter()
                        .map(|s| s.as_ref().map(to_inv_item))
                        .collect();
                    tracing::info!(
                        "🏬 仓库 {} 格（{} 件物品）",
                        items.len(),
                        items.iter().flatten().count()
                    );
                    storage.items = items;
                    storage.visible = true;
                    // 原版 C#：仓库打开时同时显示背包
                    if !mgr.is_open(crate::game::dialogs::DialogKind::Storage) {
                        mgr.open.push(crate::game::dialogs::DialogKind::Storage);
                    }
                    if !mgr.is_open(crate::game::dialogs::DialogKind::Inventory) {
                        mgr.open.push(crate::game::dialogs::DialogKind::Inventory);
                    }
                }
                Err(e) => tracing::warn!("⚠️ UserStorage 解析失败: {} (len={})", e, payload.len()),
            }
        }

        // ---- M23: 交易 ----
        x if x == ServerPacketIds::TradeRequest as i16 => {
            use mir2_shared::binary::read_dotnet_string;
            match read_dotnet_string(&mut cur) {
                Ok(name) => {
                    if trade.visible {
                        // 打开包（服务器权威 partner）
                        trade.partner_name = name.clone();
                        tracing::info!("🤝 交易窗口: 与 {} 交易", name);
                    } else if trade.is_initiator {
                        // 发起者收到的第一个包就是 open（同 opcode）
                        trade.visible = true;
                        trade.partner_name = name.clone();
                        tracing::info!("🤝 交易窗口已打开（发起者）: {}", name);
                    } else if trade.invite.is_none() {
                        // 邀请包
                        trade.invite = Some(name.clone());
                        tracing::info!("🤝 收到交易邀请: {}", name);
                    }
                }
                Err(e) => tracing::warn!("⚠️ TradeRequest 解析失败: {} (len={})", e, payload.len()),
            }
        }
        x if x == ServerPacketIds::TradeGold as i16 => {
            // 服务端 wire：[amount: u64 LE]
            if payload.len() >= 12 {
                let amount = u64::from_le_bytes(payload[4..12].try_into().unwrap_or([0; 8]));
                trade.their_gold = amount;
                tracing::info!("💰 对方交易金币: {}", amount);
            }
        }
        x if x == ServerPacketIds::TradeConfirm as i16 => {
            // 服务端 wire：[side_a.locked u8][side_b.locked u8]（a=发起者）
            if payload.len() >= 6 {
                let a = payload[4] != 0;
                let b = payload[5] != 0;
                if trade.is_initiator {
                    trade.my_locked = a;
                    trade.their_locked = b;
                } else {
                    trade.my_locked = b;
                    trade.their_locked = a;
                }
                tracing::info!(
                    "🔒 交易锁定状态: 我={} 对方={}",
                    trade.my_locked,
                    trade.their_locked
                );
                if a && b {
                    tracing::info!("🎉 交易完成！");
                    trade.visible = false;
                    trade.invite = None;
                    trade.pending_deposit = None;
                }
            }
        }
        x if x == ServerPacketIds::TradeCancel as i16 => {
            if trade.visible {
                tracing::info!("🚫 交易已取消/关闭");
            }
            trade.visible = false;
            trade.invite = None;
            trade.pending_deposit = None;
        }
        x if x == ServerPacketIds::TradeItem as i16 => {
            // 服务端 wire：[uid u64][grid u8][count u16][is_add u8]（对方物品更新）
            if payload.len() >= 15 {
                let uid = u64::from_le_bytes(payload[4..12].try_into().unwrap_or([0; 8]));
                let grid = payload[12] as usize;
                let count = u16::from_le_bytes(payload[13..15].try_into().unwrap_or([0; 2]));
                let is_add = payload[15] != 0;
                if is_add {
                    // 对方新增物品：保留已有条目的显示信息（服务端只发 uid/grid/count）
                    if let Some(slot) = trade.their_items.get_mut(grid) {
                        let prev = slot.take();
                        *slot = Some(UiTradeItem {
                            uid,
                            item_index: prev.as_ref().map(|p| p.item_index).unwrap_or(0),
                            name: prev
                                .as_ref()
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| format!("#{}", uid)),
                            image: prev.as_ref().map(|p| p.image).unwrap_or(0),
                            count: if count > 0 { count } else { 1 },
                        });
                    }
                    tracing::info!("📦 对方放入交易物品 uid={} 槽={} x{}", uid, grid, count);
                } else {
                    if let Some(slot) = trade.their_items.get_mut(grid) {
                        *slot = None;
                    }
                    trade.their_items.retain(|s| s.as_ref().map(|i| i.uid) != Some(uid));
                    tracing::info!("↩️ 对方取回物品 uid={}", uid);
                }
            }
        }
        x if x == ServerPacketIds::DepositTradeItem as i16 => {
            // 服务端响应：[from_slot i32][success u8]
            if payload.len() >= 9 {
                let success = payload[8] != 0;
                if success {
                    if let Some((from, to)) = trade.pending_deposit.take() {
                        if let Some(item) = hud.inventory.items.get(from).and_then(|s| s.as_ref()) {
                            if let Some(slot) = trade.my_items.get_mut(to) {
                                *slot = Some(UiTradeItem::from(item));
                            }
                        }
                        trade.my_locked = false;
                        tracing::info!("✅ 物品已放入交易槽 {}", to);
                    }
                } else {
                    trade.pending_deposit = None;
                    tracing::warn!("❌ 放入交易失败");
                }
            }
        }

        // ---- M22: 邮件 ----
        x if x == ServerPacketIds::ReceiveMail as i16 => {
            match parse_receive_mail(&payload[PacketHeader::HEADER_SIZE..]) {
                Some((entry, detail)) => {
                    // 去重：同 mail_id 已存在则替换（全文包会更新未读标记）
                    if let Some(existing) = mail.mails.iter_mut().find(|m| m.mail_id == entry.mail_id) {
                        *existing = entry;
                    } else {
                        mail.mails.insert(0, entry);
                    }
                    if let Some(d) = detail {
                        mail.detail = Some(d);
                        tracing::info!(
                            "📧 邮件详情: {} - {} 金币={}",
                            mail.detail.as_ref().map(|x| x.sender.as_str()).unwrap_or("?"),
                            mail.detail.as_ref().map(|x| x.subject.as_str()).unwrap_or("?"),
                            mail.detail.as_ref().map(|x| x.gold).unwrap_or(0)
                        );
                    } else {
                        tracing::info!(
                            "📧 新邮件: {} - {}{}",
                            mail.mails[0].sender,
                            mail.mails[0].subject,
                            if mail.mails[0].unread { "（未读）" } else { "" }
                        );
                    }
                }
                None => tracing::warn!("⚠️ ReceiveMail 解析失败: (len={})", payload.len()),
            }
        }

        // ---- M21: 组队 ----
        x if x == ServerPacketIds::GroupMembersMap as i16 => {
            match group::GroupMembersMap::read_body(&mut cur) {
                Ok(p) => {
                    group.members = p.members;
                    tracing::info!(
                        "👥 组队成员: {}",
                        group
                            .members
                            .iter()
                            .map(|m| format!(
                                "{}{}",
                                if m.is_leader { "★" } else { "" },
                                m.name
                            ))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                Err(e) => {
                    tracing::warn!("⚠️ GroupMembersMap 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        x if x == ServerPacketIds::GroupInvite as i16 => {
            match group::GroupInvite::read_body(&mut cur) {
                Ok(p) => {
                    group.invite = Some(crate::game::dialogs::group::GroupInviteInfo {
                        inviter_name: p.name.clone(),
                        inviter_id: p.inviter_id,
                    });
                    tracing::info!("👥 收到组队邀请: {} (id={})", p.name, p.inviter_id);
                }
                Err(e) => {
                    tracing::warn!("⚠️ GroupInvite 解析失败: {} (len={})", e, payload.len())
                }
            }
        }
        x if x == ServerPacketIds::DeleteGroup as i16 => {
            group.members.clear();
            group.invite = None;
            tracing::info!("👥 组队已解散");
        }
        x if x == ServerPacketIds::DeleteMember as i16 => {
            if let Ok(p) = group::DeleteMember::read_body(&mut cur) {
                group.members.retain(|m| m.name != p.name);
                tracing::info!("👥 成员离开: {}", p.name);
            }
        }
        x if x == ServerPacketIds::NewMagic as i16 => {
            if let Ok(p) = magic::NewMagic::read_body(&mut cur) {
                if !p.hero {
                    magics.upsert(p.magic.clone());
                    tracing::info!(
                        "📖 学会技能: {} ({:?}) key={}",
                        p.magic.name,
                        p.magic.spell,
                        p.magic.key
                    );
                }
            }
        }
        x if x == ServerPacketIds::MagicDelay as i16 => {
            if let Ok(p) = MagicDelay::read_body(&mut cur) {
                tracing::debug!("⏳ 技能冷却: object={} spell={:?} delay={}ms", p.object_id, p.spell, p.delay);
            }
        }
        x if x == ServerPacketIds::MagicCast as i16 => {
            if let Ok(p) = MagicCast::read_body(&mut cur) {
                tracing::info!("🪄 MagicCast: spell={:?}", p.spell);
            }
        }

        x if x == ServerPacketIds::KeepAlive as i16 => {
            // 服务器心跳：回一个 KeepAlive
            net.send_packet(&mir2_shared::packets::client::connection::KeepAlive { time: 0 });
        }
        other => {
            tracing::debug!("未处理服务器包 opcode {:04X}", other);
        }
    }
}

/// 聊天颜色（参考 C# ReceiveChat / macroquad chat_dialog 配色）
fn chat_color(t: mir2_shared::enums::ChatType) -> bevy::prelude::Color {
    use mir2_shared::enums::ChatType;
    match t {
        ChatType::Normal => bevy::prelude::Color::WHITE,
        ChatType::Shout | ChatType::Shout2 | ChatType::Shout3 => {
            bevy::prelude::Color::srgb(1.0, 0.75, 0.3)
        }
        ChatType::System | ChatType::System2 | ChatType::Announcement => {
            bevy::prelude::Color::srgb(1.0, 0.95, 0.4)
        }
        ChatType::Hint => bevy::prelude::Color::srgb(0.4, 1.0, 0.4),
        ChatType::Group => bevy::prelude::Color::srgb(0.5, 0.9, 1.0),
        ChatType::WhisperIn | ChatType::WhisperOut => bevy::prelude::Color::srgb(1.0, 0.5, 1.0),
        ChatType::Guild => bevy::prelude::Color::srgb(0.8, 0.6, 1.0),
        ChatType::LevelUp => bevy::prelude::Color::srgb(1.0, 0.9, 0.2),
        ChatType::Mentor | ChatType::Trainer | ChatType::Relationship => {
            bevy::prelude::Color::srgb(0.6, 1.0, 0.8)
        }
        _ => bevy::prelude::Color::WHITE,
    }
}

