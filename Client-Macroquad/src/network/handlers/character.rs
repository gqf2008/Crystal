// Character Handler - 角色相关数据包处理
// 
// 处理登录、角色创建、选择、删除等

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

/// Character handler - processes character-related packets
pub struct CharacterHandler;

impl PacketHandler for CharacterHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // Login (failed)
            x if x == ServerPacketIds::Login as u16 => {
                if let Ok(packet) = server::Login::read_body(&mut cursor) {
                    let reason = match packet.result {
                        0 => "登录已禁用".to_string(),
                        1 => "账号无效".to_string(),
                        2 => "密码无效".to_string(),
                        3 => "账号不存在".to_string(),
                        4 => "账号或密码错误".to_string(),
                        other => format!("登录失败: result={other}"),
                    };
                    tracing::warn!("❌ Login failed: {}", reason);
                    events.push(NetworkEvent::LoginFailed { reason });
                }
            }

            // LoginBanned
            x if x == ServerPacketIds::LoginBanned as u16 => {
                if let Ok(packet) = server::LoginBanned::read_body(&mut cursor) {
                    let reason = format!("登录被禁止: {}", packet.reason);
                    tracing::warn!("⛔ {}", reason);
                    events.push(NetworkEvent::LoginFailed { reason });
                }
            }

            // LoginSuccess
            x if x == ServerPacketIds::LoginSuccess as u16 => {
                if let Ok(packet) = server::LoginSuccess::read_body(&mut cursor) {
                    tracing::info!("✅ Login successful, received {} characters", packet.characters.len());
                    events.push(NetworkEvent::LoginSuccess { 
                        characters: packet.characters 
                    });
                }
            }

            // NewAccount (response)
            x if x == ServerPacketIds::NewAccount as u16 => {
                if let Ok(packet) = server::NewAccount::read_body(&mut cursor) {
                    match packet.result {
                        8 => {
                            tracing::info!("✅ NewAccount success");
                            events.push(NetworkEvent::NewAccountSuccess);
                        }
                        0 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "当前服务器禁止创建账号".to_string(),
                        }),
                        1 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "账号不合法".to_string(),
                        }),
                        2 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "密码不合法".to_string(),
                        }),
                        3 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "邮箱不合法".to_string(),
                        }),
                        4 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "用户名不合法".to_string(),
                        }),
                        5 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "密保问题不合法".to_string(),
                        }),
                        6 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "密保答案不合法".to_string(),
                        }),
                        7 => events.push(NetworkEvent::NewAccountFailed {
                            reason: "账号已存在".to_string(),
                        }),
                        other => events.push(NetworkEvent::NewAccountFailed {
                            reason: format!("创建账号失败: result={other}"),
                        }),
                    }
                }
            }

            // ChangePassword (response)
            x if x == ServerPacketIds::ChangePassword as u16 => {
                if let Ok(packet) = server::ChangePassword::read_body(&mut cursor) {
                    match packet.result {
                        6 => {
                            tracing::info!("✅ ChangePassword success");
                            events.push(NetworkEvent::ChangePasswordSuccess);
                        }
                        0 => events.push(NetworkEvent::ChangePasswordFailed {
                            reason: "修改密码已禁用".to_string(),
                        }),
                        1 => events.push(NetworkEvent::ChangePasswordFailed {
                            reason: "账号不合法".to_string(),
                        }),
                        2 => events.push(NetworkEvent::ChangePasswordFailed {
                            reason: "当前密码不合法".to_string(),
                        }),
                        3 => events.push(NetworkEvent::ChangePasswordFailed {
                            reason: "新密码不合法".to_string(),
                        }),
                        4 => events.push(NetworkEvent::ChangePasswordFailed {
                            reason: "账号不存在".to_string(),
                        }),
                        5 => events.push(NetworkEvent::ChangePasswordFailed {
                            reason: "当前密码错误".to_string(),
                        }),
                        other => events.push(NetworkEvent::ChangePasswordFailed {
                            reason: format!("修改密码失败: result={other}"),
                        }),
                    }
                }
            }

            // ChangePasswordBanned
            x if x == ServerPacketIds::ChangePasswordBanned as u16 => {
                if let Ok(packet) = server::ChangePasswordBanned::read_body(&mut cursor) {
                    let reason = format!(
                        "修改密码被禁止: {} (expiry_ticks={})",
                        packet.reason, packet.expiry_date
                    );
                    tracing::warn!("⛔ {}", reason);
                    events.push(NetworkEvent::ChangePasswordFailed { reason });
                }
            }
            
            // StartGame
            x if x == ServerPacketIds::StartGame as u16 => {
                if let Ok(packet) = server::StartGame::read_body(&mut cursor) {
                    tracing::info!("🎮 StartGame result: {}", packet.result);
                    events.push(NetworkEvent::StartGame { packet });
                }
            }

            // StartGameBanned
            x if x == ServerPacketIds::StartGameBanned as u16 => {
                if let Ok(packet) = server::StartGameBanned::read_body(&mut cursor) {
                    tracing::warn!(
                        "⛔ StartGame banned: {} (expiry {})",
                        packet.reason,
                        packet.expiry_date
                    );
                    events.push(NetworkEvent::StartGameBanned { packet });
                }
            }

            // StartGameDelay
            x if x == ServerPacketIds::StartGameDelay as u16 => {
                if let Ok(packet) = server::StartGameDelay::read_body(&mut cursor) {
                    tracing::info!("⏳ StartGameDelay: {}ms", packet.milliseconds);
                    events.push(NetworkEvent::StartGameDelay { packet });
                }
            }
            
            // NewCharacter (failed/response)
            x if x == ServerPacketIds::NewCharacter as u16 => {
                if let Ok(packet) = server::NewCharacter::read_body(&mut cursor) {
                    let message = match packet.result {
                        0 => "Creating new characters is currently disabled.".to_string(),
                        1 => "Your Character Name is not acceptable.".to_string(),
                        2 => "The gender you selected does not exist.\n Contact a GM for assistance.".to_string(),
                        3 => "The class you selected does not exist.\n Contact a GM for assistance.".to_string(),
                        4 => format!(
                            "You cannot make anymore then {} Characters.",
                            mir2_shared::MAX_CHARACTER_COUNT
                        ),
                        5 => "A Character with this name already exists.".to_string(),
                        _ => format!("Create character failed: result={}", packet.result),
                    };
                    events.push(NetworkEvent::SystemMessage { message });
                }
            }

            // NewCharacterSuccess
            x if x == ServerPacketIds::NewCharacterSuccess as u16 => {
                if let Ok(packet) = server::NewCharacterSuccess::read_body(&mut cursor) {
                    tracing::info!("👤 Character created: {}", packet.character.name);
                    events.push(NetworkEvent::CharacterCreated { character: packet.character });
                }
            }

            // DeleteCharacter (failed/response)
            x if x == ServerPacketIds::DeleteCharacter as u16 => {
                if let Ok(packet) = server::DeleteCharacter::read_body(&mut cursor) {
                    events.push(NetworkEvent::SystemMessage {
                        message: format!("删除角色失败: result={}", packet.result),
                    });
                }
            }

            // DeleteCharacterSuccess
            x if x == ServerPacketIds::DeleteCharacterSuccess as u16 => {
                if let Ok(packet) = server::DeleteCharacterSuccess::read_body(&mut cursor) {
                    tracing::info!("🗑️ Character deleted: index={}", packet.character_index);
                    events.push(NetworkEvent::CharacterDeleted {
                        index: packet.character_index as u32,
                    });
                }
            }
            
            // UserInformation (player data after login)
            x if x == ServerPacketIds::UserInformation as u16 => {
                if let Ok(packet) = server::UserInformation::read_body(&mut cursor) {
                    tracing::info!(
                        "📊 UserInformation: {} at ({}, {})",
                        packet.name,
                        packet.location_x,
                        packet.location_y
                    );

                    // 🎯 推送UserInformation事件（进入游戏后的完整状态）
                    events.push(NetworkEvent::UserInformation { packet });
                }
            }

            // ====================================================================
            // Player State
            // ====================================================================

            // PlayerUpdate
            x if x == ServerPacketIds::PlayerUpdate as u16 => {
                if let Ok(_packet) = server::PlayerUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerUpdated);
                    tracing::debug!("🔄 PlayerUpdate received");
                }
            }

            // ChangeAMode
            x if x == ServerPacketIds::ChangeAMode as u16 => {
                if let Ok(packet) = server::ChangeAMode::read_body(&mut cursor) {
                    events.push(NetworkEvent::AttackModeChanged { mode: packet.mode as u8 });
                    tracing::debug!("⚔️ AttackModeChanged: {:?}", packet.mode);
                }
            }

            // ChangePMode
            x if x == ServerPacketIds::ChangePMode as u16 => {
                if let Ok(packet) = server::ChangePMode::read_body(&mut cursor) {
                    events.push(NetworkEvent::PetModeChanged { mode: packet.mode as u8 });
                    tracing::debug!("🐾 PetModeChanged: {:?}", packet.mode);
                }
            }

            // ColourChanged
            x if x == ServerPacketIds::ColourChanged as u16 => {
                if let Ok(packet) = server::ColourChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerColourChanged { colour: packet.name_colour_argb as u32 });
                    tracing::debug!("🎨 PlayerColourChanged: {}", packet.name_colour_argb);
                }
            }

            // ObjectColourChanged
            x if x == ServerPacketIds::ObjectColourChanged as u16 => {
                if let Ok(packet) = server::ObjectColourChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectColourChanged {
                        object_id: packet.object_id,
                        colour: packet.name_colour_argb as u32,
                    });
                    tracing::debug!(
                        "🎨 ObjectColourChanged: object={} colour={}",
                        packet.object_id, packet.name_colour_argb
                    );
                }
            }

            // ObjectGuildNameChanged
            x if x == ServerPacketIds::ObjectGuildNameChanged as u16 => {
                if let Ok(packet) = server::ObjectGuildNameChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectGuildNameChanged2 {
                        object_id: packet.object_id,
                        guild_name: packet.guild_name.clone(),
                    });
                    tracing::debug!(
                        "🏰 ObjectGuildNameChanged: object={} guild={}",
                        packet.object_id, packet.guild_name
                    );
                }
            }

            // ObjectName
            x if x == ServerPacketIds::ObjectName as u16 => {
                if let Ok(_packet) = server::ObjectName::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerNameUpdated);
                    tracing::debug!("📛 PlayerNameUpdated");
                }
            }

            // UserName
            x if x == ServerPacketIds::UserName as u16 => {
                if let Ok(_packet) = server::UserName::read_body(&mut cursor) {
                    events.push(NetworkEvent::UserNameUpdated);
                    tracing::debug!("📛 UserNameUpdated");
                }
            }

            // ChatItemStats
            x if x == ServerPacketIds::ChatItemStats as u16 => {
                if let Ok(_packet) = server::ChatItemStats::read_body(&mut cursor) {
                    events.push(NetworkEvent::ChatItemStatsReceived);
                    tracing::debug!("📊 ChatItemStatsReceived");
                }
            }

            // ====================================================================
            // Logout / Reincarnation
            // ====================================================================

            // LogOutSuccess
            x if x == ServerPacketIds::LogOutSuccess as u16 => {
                if let Ok(_packet) = server::LogOutSuccess::read_body(&mut cursor) {
                    events.push(NetworkEvent::LogOutSuccess);
                    tracing::info!("🚪 LogOutSuccess");
                }
            }

            // LogOutFailed
            x if x == ServerPacketIds::LogOutFailed as u16 => {
                if let Ok(_packet) = server::LogOutFailed::read_body(&mut cursor) {
                    events.push(NetworkEvent::LogOutFailed);
                    tracing::warn!("🚪 LogOutFailed");
                }
            }

            // ReturnToLogin
            x if x == ServerPacketIds::ReturnToLogin as u16 => {
                if let Ok(_packet) = server::ReturnToLogin::read_body(&mut cursor) {
                    events.push(NetworkEvent::ReturnToLogin);
                    tracing::info!("🔙 ReturnToLogin");
                }
            }

            // CancelReincarnation
            x if x == ServerPacketIds::CancelReincarnation as u16 => {
                if let Ok(_packet) = server::CancelReincarnation::read_body(&mut cursor) {
                    events.push(NetworkEvent::ReincarnationCancelled);
                    tracing::debug!("🔮 Reincarnation cancelled");
                }
            }

            // RequestReincarnation
            x if x == ServerPacketIds::RequestReincarnation as u16 => {
                if let Ok(_packet) = server::RequestReincarnation::read_body(&mut cursor) {
                    events.push(NetworkEvent::ReincarnationRequested);
                    tracing::debug!("🔮 Reincarnation requested");
                }
            }

            // UserSlotsRefresh
            x if x == ServerPacketIds::UserSlotsRefresh as u16 => {
                if let Ok(_packet) = server::UserSlotsRefresh::read_body(&mut cursor) {
                    tracing::debug!("🔄 UserSlotsRefresh");
                }
            }

            _ => {
                tracing::debug!("⚠️ CharacterHandler: Unhandled opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_character_handler_creation() {
        let handler = CharacterHandler;
        // Test with an unhandled opcode to verify fallback path works
        let events = handler.handle(&PacketHeader::new(4, -1), &[]);
        assert_eq!(events.len(), 1);
    }
}
