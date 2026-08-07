use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

// 网络包解码分派（#72 拆分；#1148 再按域拆分）：handle_login 处理服务端包 登录/选角/登出 分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_login(    net: &mut NetConnection,
    session: &mut SessionState,
    auth: &mut AuthFeedback,
    game_data: &mut GameData,
    net_objects: &mut MessageWriter<NetObject>,
    net_removals: &mut MessageWriter<NetObjectRemoved>,
    motions: &mut MessageWriter<NetMotion>,
    combat_evt: &mut MessageWriter<CombatEvent>,
    effects: &mut MessageWriter<PendingEffect>,
    server_events: &mut MessageWriter<ServerEvent>,
    control: &mut ControlState,
    next: &mut NextState<AppState>,
    payload: &[u8],) -> bool {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return false;
    };
    let opcode = header.opcode;
    const HANDLED: &[i16] = &[ServerPacketIds::Connected as i16, ServerPacketIds::Disconnect as i16, ServerPacketIds::ClientVersion as i16, ServerPacketIds::NewAccount as i16, ServerPacketIds::ChangePassword as i16, ServerPacketIds::Login as i16, ServerPacketIds::LoginSuccess as i16, ServerPacketIds::StartGame as i16, ServerPacketIds::NewCharacter as i16, ServerPacketIds::NewCharacterSuccess as i16, ServerPacketIds::DeleteCharacter as i16, ServerPacketIds::DeleteCharacterSuccess as i16, ServerPacketIds::LogOutSuccess as i16, ServerPacketIds::LoginBanned as i16, ServerPacketIds::StartGameBanned as i16, ServerPacketIds::StartGameDelay as i16, ServerPacketIds::LogOutFailed as i16, ServerPacketIds::ReturnToLogin as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        x if x == ServerPacketIds::Connected as i16 => {
            tracing::info!("🔌 服务器已连接（Connected），发送 ClientVersion");
            if !net.client_version_sent {
                net.client_version_sent = true;
                net.send_packet(&mir2_shared::packets::client::connection::ClientVersion {
                    version_hash: net.client_version_hash.to_vec(),
                });
            }
        }
        x if x == ServerPacketIds::Disconnect as i16 => {
            // C# S.Disconnect.Reason：0=服务器关闭 1=顶号 2=包错误 3=崩溃
            let reason = match connection::Disconnect::read_body(&mut cur) {
                Ok(p) => p.reason,
                Err(_) => 0, // 空 body 容错（早期 ServerRust）
            };
            let msg = match reason {
                1 => "账号已在其他地方登录".to_string(),
                2 => "网络数据包错误".to_string(),
                3 => "服务器崩溃".to_string(),
                _ => "服务器已关闭连接".to_string(),
            };
            auth.login_error = Some(msg);
            net.auto_reconnect = false;
            net.disconnected = Some(format!("server-disconnect:{}", reason));
            next.set(AppState::Login);
            tracing::warn!("🚪 服务端断开: reason={}", reason);
        }
        x if x == ServerPacketIds::ClientVersion as i16 => {
            if let Ok(p) = connection::ClientVersion::read_body(&mut cur) {
                tracing::info!("🔑 ClientVersion 校验结果: {}", p.result);
            }
        }
        x if x == ServerPacketIds::NewAccount as i16 => {
            if let Ok(p) = login::NewAccount::read_body(&mut cur) {
                let msg = match p.result {
                    8 => {
                        auth.new_account_success = true;
                        auth.new_account_error = None;
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
                    auth.new_account_error = Some(msg);
                }
            }
        }
        x if x == ServerPacketIds::ChangePassword as i16 => {
            if let Ok(p) = login::ChangePassword::read_body(&mut cur) {
                let msg = match p.result {
                    6 => {
                        auth.change_password_success = true;
                        auth.change_password_error = None;
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
                    auth.change_password_error = Some(msg);
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
                auth.login_error = Some(msg);
                net.reconnecting = false;
            }
        }
        x if x == ServerPacketIds::LoginSuccess as i16 => {
            if let Ok(p) = login::LoginSuccess::read_body(&mut cur) {
                tracing::info!("✅ 登录成功，角色 {} 个", p.characters.len());
                session.characters = p.characters;
                session.select_reload = false;
                net.state = NetState::Select;
                auth.login_error = None;
                auth.login_success = true;
                // M58：重连成功后自动进入之前的角色
                if net.reconnecting {
                    net.reconnecting = false;
                    let saved = net.saved_character.lock().ok().map(|g| g.clone()).flatten();
                    if let Some(idx) = saved {
                        net.send_packet(&mir2_shared::packets::client::account::StartGame {
                            character_index: idx,
                        });
                        tracing::info!("🔌 自动重连成功，自动进入角色 idx={}", idx);
                    }
                }
            }
        }
        x if x == ServerPacketIds::StartGame as i16 => {
            if let Ok(p) = login::StartGame::read_body(&mut cur) {
                tracing::info!("✅ 开始游戏 result={}", p.result);
                net.state = NetState::InGame;
            }
        }
        x if x == ServerPacketIds::NewCharacter as i16 => {
            if let Ok(p) = account::NewCharacter::read_body(&mut cur) {
                tracing::info!("⛔ 新建角色被拒绝 result={}", p.result);
                session.character_error = Some(match p.result {
                    4 => "最多只能创建4个角色！".to_string(),
                    _ => "创建角色失败！".to_string(),
                });
            }
        }
        x if x == ServerPacketIds::NewCharacterSuccess as i16 => {
            if let Ok(p) = account::NewCharacterSuccess::read_body(&mut cur) {
                tracing::info!("✅ 新建角色成功: {}", p.character.name);
                session.characters.push(SelectInfo {
                    index: p.character.index,
                    name: p.character.name.clone(),
                    level: p.character.level,
                    class: p.character.class,
                    gender: p.character.gender,
                    last_access: p.character.last_access,
                });
                session.select_reload = true;
            }
        }
        x if x == ServerPacketIds::DeleteCharacter as i16 => {
            if let Ok(p) = account::DeleteCharacter::read_body(&mut cur) {
                tracing::warn!("⛔ 删除角色被拒绝 result={}", p.result);
                session.character_error = Some(match p.result {
                    0 => "删除失败：不能删除当前在线角色".to_string(),
                    1 => "删除失败：角色不存在".to_string(),
                    _ => format!("删除角色失败（{}）", p.result),
                });
            }
        }
        x if x == ServerPacketIds::DeleteCharacterSuccess as i16 => {
            if let Ok(p) = account::DeleteCharacterSuccess::read_body(&mut cur) {
                tracing::info!("🗑️ 删除角色成功 idx={}", p.character_index);
                session.characters.retain(|c| c.index != p.character_index);
                session.selected_index = None;
                session.select_reload = true;
            }
        }
        x if x == ServerPacketIds::LogOutSuccess as i16 => {
            // C# S.LogOutSuccess：登出成功，返回选角界面
            if let Ok(_p) = player::LogOutSuccess::read_body(&mut cur) {
                server_events.write(ServerEvent::LogOutSuccess);
                next.set(AppState::Select);
                tracing::info!("🚪 登出成功，返回选角");
            }
        }
        x if x == ServerPacketIds::LoginBanned as i16 => {
            if let Ok(p) = login::LoginBanned::read_body(&mut cur) {
                auth.login_error = Some(format!("账号被封禁：{}（到期 ticks={}）", p.reason, p.expiry_date));
                server_events.write(ServerEvent::LoginBanned {
                    reason: p.reason.clone(),
                    expiry_date: p.expiry_date,
                });
                tracing::warn!("🚫 登录封禁: {}", p.reason);
            }
        }
        x if x == ServerPacketIds::StartGameBanned as i16 => {
            if let Ok(p) = login::StartGameBanned::read_body(&mut cur) {
                server_events.write(ServerEvent::StartGameBanned {
                    reason: p.reason.clone(),
                    expiry_date: p.expiry_date,
                });
                tracing::warn!("🚫 进游戏封禁: {}", p.reason);
            }
        }
        x if x == ServerPacketIds::StartGameDelay as i16 => {
            if let Ok(p) = login::StartGameDelay::read_body(&mut cur) {
                server_events.write(ServerEvent::StartGameDelay { milliseconds: p.milliseconds });
                tracing::info!("⏳ 进游戏延迟: {}ms", p.milliseconds);
            }
        }
        x if x == ServerPacketIds::LogOutFailed as i16 => {
            if let Ok(_p) = login::LogOutFailed::read_body(&mut cur) {
                server_events.write(ServerEvent::LogOutFailed);
                tracing::warn!("🚪 登出失败");
            }
        }
        x if x == ServerPacketIds::ReturnToLogin as i16 => {
            if let Ok(_p) = login::ReturnToLogin::read_body(&mut cur) {
                server_events.write(ServerEvent::ReturnToLogin);
                next.set(AppState::Login);
                tracing::info!("🚪 服务端要求返回登录界面");
            }
        }
        _ => {}
    }
    handled
}
