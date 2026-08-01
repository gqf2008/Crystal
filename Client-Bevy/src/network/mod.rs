// ============================================================================
// 网络模块（里程碑 5：mock 模式打通 登录→选角→进游戏→对象生成）
// ============================================================================

pub mod codec;
pub mod mock;

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use mir2_shared::enums::ServerPacketIds;
use mir2_shared::packets::base::{Packet, PacketHeader};
use mir2_shared::SelectInfo;

use crate::map_renderer::GameData;
use crate::scenes::AppState;

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
#[derive(Resource, Default)]
pub struct NetworkContext {
    /// 发往服务器的内层包（PacketHeader+body）
    pub to_server: Option<Sender<Vec<u8>>>,
    /// 从服务器接收的外帧编码字节
    pub from_server: Option<Receiver<Vec<u8>>>,
    pub state: NetState,
    /// 角色列表（LoginSuccess 携带）
    pub characters: Vec<SelectInfo>,
    /// 选中的角色
    pub selected_index: Option<i32>,
    /// 登录错误信息
    pub login_error: Option<String>,
    /// 登录成功标志（LoginScene 播放 ChrSel 动画后进选角）
    pub login_success: bool,
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
}

#[derive(Resource, Default)]
pub struct NetObjects {
    pub pending: Vec<NetObject>,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkContext>();
        app.init_resource::<NetObjects>();
        app.add_systems(Startup, setup_mock_network);
        app.add_systems(Update, network_system);
    }
}

/// 启动 mock 网络（本机模拟服务器）
fn setup_mock_network(mut net: ResMut<NetworkContext>) {
    let (to_server, from_client) = crossbeam_channel::bounded::<Vec<u8>>(1024);
    let (to_client, from_server) = crossbeam_channel::bounded::<Vec<u8>>(1024);
    net.to_server = Some(to_server);
    net.from_server = Some(from_server);
    mock::spawn_mock(to_client, from_client);
    tracing::info!("🌐 Mock 网络已启动（本地模拟服务器）");
}

/// 网络系统：解码外帧 → 解析包 → 分发处理
fn network_system(
    mut net: ResMut<NetworkContext>,
    mut game_data: ResMut<GameData>,
    mut net_objects: ResMut<NetObjects>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(rx) = net.from_server.clone() else {
        return;
    };
    let mut buf: Vec<u8> = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        buf.extend_from_slice(&bytes);
        // 尽可能多地解码帧
        loop {
            match codec::decode(&buf) {
                Some(Ok((payload, consumed))) => {
                    buf.drain(..consumed);
                    handle_packet(&mut net, &mut game_data, &mut net_objects, &mut next, &payload);
                    if buf.is_empty() {
                        break;
                    }
                }
                Some(Err(e)) => {
                    tracing::warn!("⚠️ 帧解码失败: {}", e);
                    buf.clear();
                    break;
                }
                None => break, // 数据不足
            }
        }
    }
}

/// 处理单个内层包
fn handle_packet(
    net: &mut NetworkContext,
    game_data: &mut GameData,
    net_objects: &mut NetObjects,
    next: &mut NextState<AppState>,
    payload: &[u8],
) {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return;
    };
    match header.opcode {
        x if x == ServerPacketIds::LoginSuccess as i16 => {
            if let Ok(p) = login::LoginSuccess::read_body(&mut cur) {
                tracing::info!("✅ 登录成功，角色 {} 个", p.characters.len());
                net.characters = p.characters;
                net.state = NetState::Select;
                net.login_error = None;
                net.login_success = true;
            }
        }
        x if x == ServerPacketIds::StartGame as i16 => {
            if let Ok(p) = login::StartGame::read_body(&mut cur) {
                tracing::info!("✅ 开始游戏 result={}", p.result);
                net.state = NetState::InGame;
            }
        }
        x if x == ServerPacketIds::MapChanged as i16 => {
            if let Ok(p) = map::MapChanged::read_body(&mut cur) {
                tracing::info!("🗺️ MapChanged: {} ({},{})", p.file_name, p.location_x, p.location_y);
                game_data.desired_map = Some(p.file_name);
                game_data.player_spawn = Some((p.location_x as f32, p.location_y as f32, p.direction));
                next.set(AppState::Game);
            }
        }
        x if x == ServerPacketIds::ObjectPlayer as i16 => {
            if let Ok(p) = objects::ObjectPlayer::read_body(&mut cur) {
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
