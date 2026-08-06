use std::convert::TryFrom;
use std::io::{Cursor, Read, Write};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};

use crate::data::stats::{SharedError, SharedResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    pub length: u16,
    pub opcode: i16,
}

impl PacketHeader {
    pub const HEADER_SIZE: usize = 4;

    pub fn new(length: u16, opcode: i16) -> Self {
        Self { length, opcode }
    }

    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let length = reader.read_u16::<LittleEndian>()?;
        let opcode = reader.read_i16::<LittleEndian>()?;

        // 🔒 安全检查: 验证包头的合理性
        // 包长度必须至少等于包头大小(4字节)
        if (length as usize) < Self::HEADER_SIZE {
            return Err(SharedError::InvalidPacketLength(length));
        }

        // 包长度不应超过64KB(u16最大值,但实际游戏包应该更小)
        // 注: u16 天然上限 65535，此检查已省略（编译器优化）

        Ok(Self { length, opcode })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u16::<LittleEndian>(self.length)?;
        writer.write_i16::<LittleEndian>(self.opcode)?;
        Ok(())
    }
}

pub trait Packet: Sized {
    const OPCODE: i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self>;
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()>;

    fn is_compressed() -> bool {
        false
    }
}

pub fn serialize_packet<W: Write, P: Packet>(writer: &mut W, packet: &P) -> SharedResult<()> {
    let mut buffer = Vec::new();
    packet.write_body(&mut buffer)?;
    let body = if P::is_compressed() {
        compress_bytes(&buffer)?
    } else {
        buffer
    };

    let total_len = PacketHeader::HEADER_SIZE + body.len();
    let length = u16::try_from(total_len).map_err(|_| SharedError::PacketTooLarge(body.len()))?;
    let header = PacketHeader::new(length, P::OPCODE);

    // IMPORTANT: 写到网络时尽量一次 write_all。
    // 服务端的洪泛保护计数基于 socket receive 回调次数；多次小 write 在 TCP_NODELAY 下更容易被拆包，
    // 导致服务端计数异常升高并断开连接。
    let mut out = Vec::with_capacity(PacketHeader::HEADER_SIZE + body.len());
    header.write_to(&mut out)?;
    out.extend_from_slice(&body);
    writer.write_all(&out)?;
    Ok(())
}

pub fn deserialize_packet<R: Read, P: Packet>(reader: &mut R) -> SharedResult<P> {
    let header = PacketHeader::read_from(reader)?;
    if header.length < PacketHeader::HEADER_SIZE as u16 {
        return Err(SharedError::InvalidPacketLength(header.length));
    }

    if header.opcode != P::OPCODE {
        return Err(SharedError::OpcodeMismatch {
            expected: P::OPCODE,
            actual: header.opcode,
        });
    }

    // 🔒 安全检查: 使用 checked_sub 防止溢出
    let body_len = match (header.length as usize).checked_sub(PacketHeader::HEADER_SIZE) {
        Some(len) => len,
        None => {
            eprintln!(
                "❌ ERROR: header.length={} < HEADER_SIZE={}",
                header.length,
                PacketHeader::HEADER_SIZE
            );
            return Err(SharedError::InvalidPacketLength(header.length));
        }
    };

    // 🔒 安全检查: 防止巨量内存分配 (51GB 崩溃修复)
    const MAX_BODY_SIZE: usize = 1024 * 1024; // 1MB 上限
    if body_len > MAX_BODY_SIZE {
        eprintln!(
            "❌ FATAL: body_len={} exceeds MAX_BODY_SIZE={}",
            body_len, MAX_BODY_SIZE
        );
        eprintln!(
            "   header.length={}, opcode={}",
            header.length, header.opcode
        );
        return Err(SharedError::PacketTooLarge(body_len));
    }

    eprintln!(
        "DEBUG: Allocating body vec of {} bytes (opcode={})",
        body_len, header.opcode
    );
    let mut body = vec![0u8; body_len];
    reader.read_exact(&mut body)?;
    let payload = if P::is_compressed() {
        decompress_bytes(&body)?
    } else {
        body
    };
    let mut cursor = Cursor::new(payload);

    let packet = P::read_body(&mut cursor)?;
    Ok(packet)
}

pub fn extract_packet<P: Packet>(buffer: &[u8]) -> SharedResult<Option<(P, Vec<u8>)>> {
    if buffer.len() < PacketHeader::HEADER_SIZE {
        return Ok(None);
    }

    let length = LittleEndian::read_u16(&buffer[0..2]) as usize;
    if length < PacketHeader::HEADER_SIZE {
        return Err(SharedError::InvalidPacketLength(length as u16));
    }

    if buffer.len() < length {
        return Ok(None);
    }

    let opcode = LittleEndian::read_i16(&buffer[2..4]);
    if opcode != P::OPCODE {
        return Err(SharedError::OpcodeMismatch {
            expected: P::OPCODE,
            actual: opcode,
        });
    }

    let body_slice = &buffer[PacketHeader::HEADER_SIZE..length];
    let payload = if P::is_compressed() {
        decompress_bytes(body_slice)?
    } else {
        body_slice.to_vec()
    };

    let mut cursor = Cursor::new(payload);
    let packet = P::read_body(&mut cursor)?;
    let remainder = buffer[length..].to_vec();

    Ok(Some((packet, remainder)))
}

fn compress_bytes(data: &[u8]) -> SharedResult<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

fn decompress_bytes(data: &[u8]) -> SharedResult<Vec<u8>> {
    let mut decoder = GzDecoder::new(Cursor::new(data));
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

    use crate::packets::{client, server};
    use crate::{
        enums::{ChatType, MirClass, MirDirection, MirGender, MirGridType, Spell},
        map::Point,
    };

    #[derive(Debug, PartialEq, Eq)]
    struct SimplePacket {
        value: u32,
    }

    impl Packet for SimplePacket {
        const OPCODE: i16 = 42;

        fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
            let value = reader.read_u32::<LittleEndian>()?;
            Ok(Self { value })
        }

        fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
            writer.write_u32::<LittleEndian>(self.value)?;
            Ok(())
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct CompressedPacket {
        data: Vec<u8>,
    }

    impl Packet for CompressedPacket {
        const OPCODE: i16 = 77;

        fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
            let mut data = Vec::new();
            reader.read_to_end(&mut data)?;
            Ok(Self { data })
        }

        fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
            writer.write_all(&self.data)?;
            Ok(())
        }

        fn is_compressed() -> bool {
            true
        }
    }

    #[test]
    fn roundtrip_uncompressed() -> SharedResult<()> {
        let packet = SimplePacket { value: 0xDEADBEEF };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;

        let mut cursor = Cursor::new(bytes.clone());
        let decoded = deserialize_packet::<_, SimplePacket>(&mut cursor)?;
        assert_eq!(decoded, packet);
        assert_eq!(cursor.position() as usize, bytes.len());
        Ok(())
    }

    #[test]
    fn roundtrip_compressed() -> SharedResult<()> {
        let payload = b"This payload should compress well.".repeat(4);
        let packet = CompressedPacket {
            data: payload.clone(),
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;

        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, CompressedPacket>(&mut cursor)?;
        assert_eq!(decoded, packet);
        Ok(())
    }

    #[test]
    fn extract_packet_with_remainder() -> SharedResult<()> {
        let first = SimplePacket { value: 1 };
        let second = SimplePacket { value: 2 };

        let mut first_bytes = Vec::new();
        serialize_packet(&mut first_bytes, &first)?;
        let mut second_bytes = Vec::new();
        serialize_packet(&mut second_bytes, &second)?;

        let mut stream = first_bytes.clone();
        stream.extend_from_slice(&second_bytes);

        let (decoded_first, remainder) =
            extract_packet::<SimplePacket>(&stream)?.expect("first packet");
        assert_eq!(decoded_first, first);
        assert_eq!(remainder, second_bytes);

        let (decoded_second, remainder2) =
            extract_packet::<SimplePacket>(&remainder)?.expect("second packet");
        assert_eq!(decoded_second, second);
        assert!(remainder2.is_empty());
        Ok(())
    }

    // ============================================================
    // Real-world protocol packet roundtrip tests
    // ============================================================

    use crate::packets::client::Chat as ClientChat;
    use crate::packets::server::{Chat as ServerChat, HealthChanged, ObjectRemove, PlayerUpdate};

    #[test]
    fn roundtrip_chat_server() -> SharedResult<()> {
        let packet = ServerChat {
            message: "Hello World!".to_string(),
            chat_type: ChatType::Normal,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, ServerChat>(&mut cursor)?;
        assert_eq!(decoded.message, "Hello World!");
        Ok(())
    }

    #[test]
    fn roundtrip_chat_client() -> SharedResult<()> {
        let packet = ClientChat {
            message: "Test message".to_string(),
            linked_items: Vec::new(),
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, ClientChat>(&mut cursor)?;
        assert_eq!(decoded.message, "Test message");
        Ok(())
    }

    #[test]
    fn roundtrip_object_remove() -> SharedResult<()> {
        let packet = ObjectRemove { object_id: 12345 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, ObjectRemove>(&mut cursor)?;
        assert_eq!(decoded.object_id, 12345);
        Ok(())
    }

    #[test]
    fn roundtrip_player_update() -> SharedResult<()> {
        let packet = PlayerUpdate {
            object_id: 999,
            light: 3,
            weapon: 10,
            weapon_effect: 0,
            armor: 5,
            wings_effect: 0,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, PlayerUpdate>(&mut cursor)?;
        assert_eq!(decoded.object_id, 999);
        assert_eq!(decoded.light, 3);
        assert_eq!(decoded.weapon, 10);
        assert_eq!(decoded.armor, 5);
        Ok(())
    }

    #[test]
    fn roundtrip_health_changed() -> SharedResult<()> {
        let packet = HealthChanged { hp: 500, mp: 200 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, HealthChanged>(&mut cursor)?;
        assert_eq!(decoded.hp, 500);
        assert_eq!(decoded.mp, 200);
        Ok(())
    }

    use crate::packets::client::combat::{Attack as ClientAttack, SpellToggle};

    #[test]
    fn roundtrip_attack_client() -> SharedResult<()> {
        let packet = ClientAttack {
            direction: MirDirection::Right,
            spell: crate::enums::Spell::None,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, ClientAttack>(&mut cursor)?;
        assert_eq!(decoded.direction, MirDirection::Right);
        Ok(())
    }

    #[test]
    fn roundtrip_spell_toggle() -> SharedResult<()> {
        let packet = SpellToggle {
            spell: crate::enums::Spell::FireBall,
            can_use: true,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, SpellToggle>(&mut cursor)?;
        assert_eq!(decoded.spell, crate::enums::Spell::FireBall);
        assert!(decoded.can_use);
        Ok(())
    }

    // ============================================================
    // Helper
    // ============================================================

    fn roundtrip<P: Packet + PartialEq + std::fmt::Debug>(packet: &P) -> SharedResult<()> {
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, P>(&mut cursor)?;
        assert_eq!(decoded, *packet);
        Ok(())
    }

    // ============================================================
    // Connection packets
    // ============================================================

    #[test]
    fn roundtrip_client_version() -> SharedResult<()> {
        roundtrip(&client::ClientVersion {
            version_hash: vec![0x01, 0x02, 0x03, 0x04],
        })
    }

    #[test]
    fn roundtrip_client_disconnect() -> SharedResult<()> {
        roundtrip(&client::Disconnect)
    }

    #[test]
    fn roundtrip_client_keep_alive() -> SharedResult<()> {
        roundtrip(&client::KeepAlive { time: 1234567890 })
    }

    #[test]
    fn roundtrip_server_keep_alive() -> SharedResult<()> {
        let packet = server::KeepAlive { time: 9876543210 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::KeepAlive>(&mut cursor)?;
        assert_eq!(decoded.time, 9876543210);
        Ok(())
    }

    #[test]
    fn roundtrip_server_connected() -> SharedResult<()> {
        let packet = server::Connected;
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let _decoded = deserialize_packet::<_, server::Connected>(&mut cursor)?;
        Ok(())
    }

    #[test]
    fn roundtrip_server_client_version() -> SharedResult<()> {
        let packet = server::ClientVersion { result: 1 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::ClientVersion>(&mut cursor)?;
        assert_eq!(decoded.result, 1);
        Ok(())
    }

    #[test]
    fn roundtrip_server_disconnect() -> SharedResult<()> {
        let packet = server::Disconnect { reason: 2 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::Disconnect>(&mut cursor)?;
        assert_eq!(decoded.reason, 2);
        Ok(())
    }

    // ============================================================
    // Account packets
    // ============================================================

    #[test]
    fn roundtrip_new_account() -> SharedResult<()> {
        roundtrip(&client::NewAccount {
            account_id: "test_account".to_string(),
            password: "secret123".to_string(),
            birth_date_binary: 0,
            user_name: "Test User".to_string(),
            secret_question: "What is your pet?".to_string(),
            secret_answer: "Fluffy".to_string(),
            email_address: "test@example.com".to_string(),
        })
    }

    #[test]
    fn roundtrip_change_password() -> SharedResult<()> {
        roundtrip(&client::ChangePassword {
            account_id: "account".to_string(),
            current_password: "oldpass".to_string(),
            new_password: "newpass".to_string(),
        })
    }

    #[test]
    fn roundtrip_login() -> SharedResult<()> {
        roundtrip(&client::Login {
            account_id: "player1".to_string(),
            password: "password".to_string(),
        })
    }

    #[test]
    fn roundtrip_start_game() -> SharedResult<()> {
        roundtrip(&client::StartGame { character_index: 2 })
    }

    #[test]
    fn roundtrip_new_character() -> SharedResult<()> {
        let packet = server::NewCharacter { result: 0 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::NewCharacter>(&mut cursor)?;
        assert_eq!(decoded.result, 0);
        Ok(())
    }

    #[test]
    fn roundtrip_new_character_success() -> SharedResult<()> {
        use chrono::TimeZone;
        let packet = server::NewCharacterSuccess {
            character: crate::packets::CharacterSummary {
                index: 0,
                name: "Hero".to_string(),
                level: 42,
                class: MirClass::Wizard,
                gender: MirGender::Female,
                last_access: chrono::Utc::with_ymd_and_hms(&chrono::Utc, 2024, 1, 15, 10, 30, 0)
                    .unwrap(),
            },
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::NewCharacterSuccess>(&mut cursor)?;
        assert_eq!(decoded.character.index, 0);
        assert_eq!(decoded.character.name, "Hero");
        assert_eq!(decoded.character.level, 42);
        assert_eq!(decoded.character.class, MirClass::Wizard);
        assert_eq!(decoded.character.gender, MirGender::Female);
        Ok(())
    }

    #[test]
    fn roundtrip_delete_character() -> SharedResult<()> {
        let packet = server::DeleteCharacter { result: 1 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::DeleteCharacter>(&mut cursor)?;
        assert_eq!(decoded.result, 1);
        Ok(())
    }

    #[test]
    fn roundtrip_delete_character_success() -> SharedResult<()> {
        roundtrip(&server::DeleteCharacterSuccess { character_index: 3 })
    }

    // ============================================================
    // Movement packets
    // ============================================================

    #[test]
    fn roundtrip_turn() -> SharedResult<()> {
        roundtrip(&client::Turn {
            direction: MirDirection::DownLeft,
        })
    }

    #[test]
    fn roundtrip_walk() -> SharedResult<()> {
        roundtrip(&client::Walk {
            direction: MirDirection::Right,
        })
    }

    #[test]
    fn roundtrip_run() -> SharedResult<()> {
        roundtrip(&client::Run {
            direction: MirDirection::UpRight,
        })
    }

    #[test]
    fn roundtrip_user_back_step() -> SharedResult<()> {
        let packet = server::UserBackStep {
            location_x: 100,
            location_y: 200,
            direction: MirDirection::Left,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::UserBackStep>(&mut cursor)?;
        assert_eq!(decoded.location_x, 100);
        assert_eq!(decoded.location_y, 200);
        assert_eq!(decoded.direction, MirDirection::Left);
        Ok(())
    }

    #[test]
    fn roundtrip_object_back_step() -> SharedResult<()> {
        let packet = server::ObjectBackStep {
            object_id: 555,
            location_x: 50,
            location_y: 75,
            direction: MirDirection::Down,
            distance: 2,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::ObjectBackStep>(&mut cursor)?;
        assert_eq!(decoded.object_id, 555);
        assert_eq!(decoded.location_x, 50);
        assert_eq!(decoded.location_y, 75);
        assert_eq!(decoded.direction, MirDirection::Down);
        assert_eq!(decoded.distance, 2);
        Ok(())
    }

    #[test]
    fn roundtrip_user_dash_attack() -> SharedResult<()> {
        let packet = server::UserDashAttack {
            location_x: 300,
            location_y: 400,
            direction: MirDirection::Up,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::UserDashAttack>(&mut cursor)?;
        assert_eq!(decoded.location_x, 300);
        assert_eq!(decoded.location_y, 400);
        assert_eq!(decoded.direction, MirDirection::Up);
        Ok(())
    }

    #[test]
    fn roundtrip_object_dash_attack() -> SharedResult<()> {
        let packet = server::ObjectDashAttack {
            object_id: 777,
            location_x: 111,
            location_y: 222,
            direction: MirDirection::Right,
            distance: 3,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::ObjectDashAttack>(&mut cursor)?;
        assert_eq!(decoded.object_id, 777);
        assert_eq!(decoded.location_x, 111);
        assert_eq!(decoded.location_y, 222);
        assert_eq!(decoded.direction, MirDirection::Right);
        assert_eq!(decoded.distance, 3);
        Ok(())
    }

    #[test]
    fn roundtrip_user_attack_move() -> SharedResult<()> {
        let packet = server::UserAttackMove {
            location_x: 500,
            location_y: 600,
            direction: MirDirection::DownRight,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::UserAttackMove>(&mut cursor)?;
        assert_eq!(decoded.location_x, 500);
        assert_eq!(decoded.location_y, 600);
        assert_eq!(decoded.direction, MirDirection::DownRight);
        Ok(())
    }

    #[test]
    fn roundtrip_set_concentration() -> SharedResult<()> {
        let packet = server::SetConcentration {
            object_id: 123,
            enabled: true,
            interrupted: false,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::SetConcentration>(&mut cursor)?;
        assert_eq!(decoded.object_id, 123);
        assert!(decoded.enabled);
        assert!(!decoded.interrupted);
        Ok(())
    }

    #[test]
    fn roundtrip_set_elemental() -> SharedResult<()> {
        let packet = server::SetElemental {
            object_id: 456,
            enabled: true,
            value: 100,
            element: 2,
            expire_time: 999999,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::SetElemental>(&mut cursor)?;
        assert_eq!(decoded.object_id, 456);
        assert!(decoded.enabled);
        assert_eq!(decoded.value, 100);
        assert_eq!(decoded.element, 2);
        assert_eq!(decoded.expire_time, 999999);
        Ok(())
    }

    #[test]
    fn roundtrip_object_deco() -> SharedResult<()> {
        let packet = server::ObjectDeco {
            object_id: 789,
            deco: 42,
            remove: true,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::ObjectDeco>(&mut cursor)?;
        assert_eq!(decoded.object_id, 789);
        assert_eq!(decoded.deco, 42);
        assert!(decoded.remove);
        Ok(())
    }

    #[test]
    fn roundtrip_object_sneaking() -> SharedResult<()> {
        let packet = server::ObjectSneaking {
            object_id: 321,
            sneaking: true,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::ObjectSneaking>(&mut cursor)?;
        assert_eq!(decoded.object_id, 321);
        assert!(decoded.sneaking);
        Ok(())
    }

    #[test]
    fn roundtrip_object_level_effects() -> SharedResult<()> {
        let packet = server::ObjectLevelEffects {
            object_id: 654,
            level_effects: 0b1010,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::ObjectLevelEffects>(&mut cursor)?;
        assert_eq!(decoded.object_id, 654);
        assert_eq!(decoded.level_effects, 0b1010);
        Ok(())
    }

    // ============================================================
    // Combat packets
    // ============================================================

    #[test]
    fn roundtrip_attack() -> SharedResult<()> {
        roundtrip(&client::combat::Attack {
            direction: MirDirection::Up,
            spell: Spell::None,
        })
    }

    #[test]
    fn roundtrip_range_attack() -> SharedResult<()> {
        roundtrip(&client::combat::RangeAttack {
            direction: MirDirection::Right,
            location: Point::new(10, 20),
            target_id: 999,
            target_location: Point::new(30, 40),
        })
    }

    #[test]
    fn roundtrip_harvest() -> SharedResult<()> {
        roundtrip(&client::combat::Harvest {
            direction: MirDirection::Down,
        })
    }

    #[test]
    fn roundtrip_magic() -> SharedResult<()> {
        roundtrip(&client::combat::Magic {
            spell: Spell::FireBall,
            direction: MirDirection::Left,
            target_id: 12345,
            location: Point::new(100, 200),
        })
    }

    #[test]
    fn roundtrip_magic_key() -> SharedResult<()> {
        roundtrip(&client::combat::MagicKey {
            spell: Spell::ThunderBolt,
            key: 5,
            old_key: 3,
        })
    }

    #[test]
    fn roundtrip_object_attack() -> SharedResult<()> {
        let packet = server::combat::ObjectAttack {
            object_id: 111,
            location_x: 50,
            location_y: 60,
            direction: 2,
            spell: 1,
            level: 3,
            attack_type: 0,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectAttack>(&mut cursor)?;
        assert_eq!(decoded.object_id, 111);
        assert_eq!(decoded.location_x, 50);
        assert_eq!(decoded.location_y, 60);
        assert_eq!(decoded.direction, 2);
        assert_eq!(decoded.spell, 1);
        assert_eq!(decoded.level, 3);
        assert_eq!(decoded.attack_type, 0);
        Ok(())
    }

    #[test]
    fn roundtrip_struck() -> SharedResult<()> {
        let packet = server::combat::Struck { attacker_id: 222 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::Struck>(&mut cursor)?;
        assert_eq!(decoded.attacker_id, 222);
        Ok(())
    }

    #[test]
    fn roundtrip_object_struck() -> SharedResult<()> {
        let packet = server::combat::ObjectStruck {
            object_id: 333,
            attacker_id: 444,
            location_x: 70,
            location_y: 80,
            direction: 4,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectStruck>(&mut cursor)?;
        assert_eq!(decoded.object_id, 333);
        assert_eq!(decoded.attacker_id, 444);
        assert_eq!(decoded.location_x, 70);
        assert_eq!(decoded.location_y, 80);
        assert_eq!(decoded.direction, 4);
        Ok(())
    }

    #[test]
    fn roundtrip_damage_indicator() -> SharedResult<()> {
        let packet = server::combat::DamageIndicator {
            damage: 150,
            damage_type: 1,
            object_id: 555,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::DamageIndicator>(&mut cursor)?;
        assert_eq!(decoded.damage, 150);
        assert_eq!(decoded.damage_type, 1);
        assert_eq!(decoded.object_id, 555);
        Ok(())
    }

    #[test]
    fn roundtrip_pushed() -> SharedResult<()> {
        let packet = server::combat::Pushed {
            location_x: 10,
            location_y: 20,
            direction: 3,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::Pushed>(&mut cursor)?;
        assert_eq!(decoded.location_x, 10);
        assert_eq!(decoded.location_y, 20);
        assert_eq!(decoded.direction, 3);
        Ok(())
    }

    #[test]
    fn roundtrip_object_pushed() -> SharedResult<()> {
        let packet = server::combat::ObjectPushed {
            object_id: 666,
            location_x: 30,
            location_y: 40,
            direction: 5,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectPushed>(&mut cursor)?;
        assert_eq!(decoded.object_id, 666);
        assert_eq!(decoded.location_x, 30);
        assert_eq!(decoded.location_y, 40);
        assert_eq!(decoded.direction, 5);
        Ok(())
    }

    #[test]
    fn roundtrip_server_range_attack() -> SharedResult<()> {
        let packet = server::combat::RangeAttack {
            target_id: 777,
            target_x: 100,
            target_y: 200,
            spell: 5,
            spell_level: 2,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::RangeAttack>(&mut cursor)?;
        assert_eq!(decoded.target_id, 777);
        assert_eq!(decoded.target_x, 100);
        assert_eq!(decoded.target_y, 200);
        assert_eq!(decoded.spell, 5);
        assert_eq!(decoded.spell_level, 2);
        Ok(())
    }

    #[test]
    fn roundtrip_object_range_attack() -> SharedResult<()> {
        let packet = server::combat::ObjectRangeAttack {
            object_id: 888,
            location_x: 50,
            location_y: 60,
            direction: 1,
            target_id: 999,
            target_x: 70,
            target_y: 80,
            spell: 3,
            spell_level: 1,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectRangeAttack>(&mut cursor)?;
        assert_eq!(decoded.object_id, 888);
        assert_eq!(decoded.location_x, 50);
        assert_eq!(decoded.location_y, 60);
        assert_eq!(decoded.direction, 1);
        assert_eq!(decoded.target_id, 999);
        assert_eq!(decoded.target_x, 70);
        assert_eq!(decoded.target_y, 80);
        assert_eq!(decoded.spell, 3);
        assert_eq!(decoded.spell_level, 1);
        Ok(())
    }

    #[test]
    fn roundtrip_user_dash() -> SharedResult<()> {
        let packet = server::combat::UserDash {
            location_x: 11,
            location_y: 22,
            direction: 6,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::UserDash>(&mut cursor)?;
        assert_eq!(decoded.location_x, 11);
        assert_eq!(decoded.location_y, 22);
        assert_eq!(decoded.direction, 6);
        Ok(())
    }

    #[test]
    fn roundtrip_object_dash() -> SharedResult<()> {
        let packet = server::combat::ObjectDash {
            object_id: 1234,
            location_x: 33,
            location_y: 44,
            direction: 7,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectDash>(&mut cursor)?;
        assert_eq!(decoded.object_id, 1234);
        assert_eq!(decoded.location_x, 33);
        assert_eq!(decoded.location_y, 44);
        assert_eq!(decoded.direction, 7);
        Ok(())
    }

    #[test]
    fn roundtrip_user_dash_fail() -> SharedResult<()> {
        let packet = server::combat::UserDashFail {
            location_x: 55,
            location_y: 66,
            direction: 0,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::UserDashFail>(&mut cursor)?;
        assert_eq!(decoded.location_x, 55);
        assert_eq!(decoded.location_y, 66);
        assert_eq!(decoded.direction, 0);
        Ok(())
    }

    #[test]
    fn roundtrip_object_dash_fail() -> SharedResult<()> {
        let packet = server::combat::ObjectDashFail {
            object_id: 5678,
            location_x: 77,
            location_y: 88,
            direction: 2,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectDashFail>(&mut cursor)?;
        assert_eq!(decoded.object_id, 5678);
        assert_eq!(decoded.location_x, 77);
        assert_eq!(decoded.location_y, 88);
        assert_eq!(decoded.direction, 2);
        Ok(())
    }

    #[test]
    fn roundtrip_death() -> SharedResult<()> {
        let packet = server::combat::Death {
            location_x: 99,
            location_y: 111,
            direction: 3,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::Death>(&mut cursor)?;
        assert_eq!(decoded.location_x, 99);
        assert_eq!(decoded.location_y, 111);
        assert_eq!(decoded.direction, 3);
        Ok(())
    }

    #[test]
    fn roundtrip_object_died() -> SharedResult<()> {
        let packet = server::combat::ObjectDied {
            object_id: 9999,
            location_x: 200,
            location_y: 300,
            direction: 4,
            death_type: 1,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectDied>(&mut cursor)?;
        assert_eq!(decoded.object_id, 9999);
        assert_eq!(decoded.location_x, 200);
        assert_eq!(decoded.location_y, 300);
        assert_eq!(decoded.direction, 4);
        assert_eq!(decoded.death_type, 1);
        Ok(())
    }

    #[test]
    fn roundtrip_revived() -> SharedResult<()> {
        let packet = server::combat::Revived;
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let _decoded = deserialize_packet::<_, server::combat::Revived>(&mut cursor)?;
        Ok(())
    }

    #[test]
    fn roundtrip_object_revived() -> SharedResult<()> {
        let packet = server::combat::ObjectRevived {
            object_id: 4321,
            effect: 2,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::ObjectRevived>(&mut cursor)?;
        assert_eq!(decoded.object_id, 4321);
        assert_eq!(decoded.effect, 2);
        Ok(())
    }

    #[test]
    fn roundtrip_health_changed_server() -> SharedResult<()> {
        let packet = server::combat::HealthChanged { hp: 800, mp: 300 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::HealthChanged>(&mut cursor)?;
        assert_eq!(decoded.hp, 800);
        assert_eq!(decoded.mp, 300);
        Ok(())
    }

    #[test]
    fn roundtrip_hero_health_changed() -> SharedResult<()> {
        let packet = server::combat::HeroHealthChanged { hp: 400, mp: 150 };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::combat::HeroHealthChanged>(&mut cursor)?;
        assert_eq!(decoded.hp, 400);
        assert_eq!(decoded.mp, 150);
        Ok(())
    }

    // ============================================================
    // Item packets
    // ============================================================

    #[test]
    fn roundtrip_move_item() -> SharedResult<()> {
        roundtrip(&client::item::MoveItem {
            grid: MirGridType::Inventory,
            from: 5,
            to: 10,
        })
    }

    #[test]
    fn roundtrip_store_item() -> SharedResult<()> {
        roundtrip(&client::item::StoreItem { from: 3, to: 7 })
    }

    #[test]
    fn roundtrip_take_back_item() -> SharedResult<()> {
        roundtrip(&client::item::TakeBackItem { from: 7, to: 3 })
    }

    #[test]
    fn roundtrip_merge_item() -> SharedResult<()> {
        roundtrip(&client::item::MergeItem {
            grid_from: MirGridType::Inventory,
            grid_to: MirGridType::Storage,
            id_from: 1001,
            id_to: 1002,
        })
    }

    #[test]
    fn roundtrip_equip_item() -> SharedResult<()> {
        roundtrip(&client::item::EquipItem {
            grid: MirGridType::Inventory,
            unique_id: 12345,
            to: 2,
        })
    }

    #[test]
    fn roundtrip_remove_item() -> SharedResult<()> {
        roundtrip(&client::item::RemoveItem {
            grid: MirGridType::Equipment,
            unique_id: 67890,
            to: 5,
        })
    }

    #[test]
    fn roundtrip_remove_slot_item() -> SharedResult<()> {
        roundtrip(&client::item::RemoveSlotItem {
            grid: MirGridType::Equipment,
            unique_id: 11111,
            to: 3,
            from_slot: 1,
        })
    }

    #[test]
    fn roundtrip_split_item_client() -> SharedResult<()> {
        roundtrip(&client::item::SplitItem {
            grid: MirGridType::Inventory,
            unique_id: 22222,
            count: 50,
        })
    }

    #[test]
    fn roundtrip_use_item() -> SharedResult<()> {
        roundtrip(&client::item::UseItem { unique_id: 33333 })
    }

    #[test]
    fn roundtrip_drop_item() -> SharedResult<()> {
        roundtrip(&client::item::DropItem {
            unique_id: 44444,
            count: 10,
            hero_inventory: false,
        })
    }

    #[test]
    fn roundtrip_drop_gold() -> SharedResult<()> {
        roundtrip(&client::item::DropGold { amount: 5000 })
    }

    #[test]
    fn roundtrip_pick_up() -> SharedResult<()> {
        roundtrip(&client::item::PickUp)
    }

    #[test]
    fn roundtrip_get_rented_items() -> SharedResult<()> {
        roundtrip(&client::item::GetRentedItems)
    }

    #[test]
    fn roundtrip_item_rental_request() -> SharedResult<()> {
        roundtrip(&client::item::ItemRentalRequest)
    }

    #[test]
    fn roundtrip_item_rental_fee() -> SharedResult<()> {
        roundtrip(&client::item::ItemRentalFee { amount: 10000 })
    }

    #[test]
    fn roundtrip_item_rental_period() -> SharedResult<()> {
        roundtrip(&client::item::ItemRentalPeriod { days: 7 })
    }

    #[test]
    fn roundtrip_deposit_rental_item() -> SharedResult<()> {
        roundtrip(&client::item::DepositRentalItem { from: 1, to: 2 })
    }

    #[test]
    fn roundtrip_retrieve_rental_item() -> SharedResult<()> {
        roundtrip(&client::item::RetrieveRentalItem { from: 2, to: 1 })
    }

    #[test]
    fn roundtrip_cancel_item_rental() -> SharedResult<()> {
        roundtrip(&client::item::CancelItemRental)
    }

    #[test]
    fn roundtrip_item_rental_lock_fee() -> SharedResult<()> {
        roundtrip(&client::item::ItemRentalLockFee)
    }

    #[test]
    fn roundtrip_item_rental_lock_item() -> SharedResult<()> {
        roundtrip(&client::item::ItemRentalLockItem)
    }

    #[test]
    fn roundtrip_confirm_item_rental() -> SharedResult<()> {
        roundtrip(&client::item::ConfirmItemRental)
    }

    #[test]
    fn roundtrip_sell_item() -> SharedResult<()> {
        roundtrip(&server::item::SellItem {
            unique_id: 55555,
            count: 5,
            success: true,
        })
    }

    #[test]
    fn roundtrip_repair_item() -> SharedResult<()> {
        roundtrip(&server::item::RepairItem { unique_id: 66666 })
    }

    #[test]
    fn roundtrip_item_repaired() -> SharedResult<()> {
        roundtrip(&server::item::ItemRepaired {
            unique_id: 77777,
            max_dura: 100,
            current_dura: 85,
        })
    }

    #[test]
    fn roundtrip_split_item_server() -> SharedResult<()> {
        roundtrip(&server::item::SplitItem {
            grid: MirGridType::Inventory,
            unique_id: 88888,
            count: 20,
        })
    }

    #[test]
    fn roundtrip_split_item1() -> SharedResult<()> {
        roundtrip(&server::item::SplitItem1 {
            grid: MirGridType::Storage,
            unique_id: 99999,
            count: 15,
        })
    }

    #[test]
    fn roundtrip_item_slot_size_changed() -> SharedResult<()> {
        roundtrip(&server::item::ItemSlotSizeChanged {
            unique_id: 121212,
            slot_size: 4,
        })
    }

    #[test]
    fn roundtrip_item_seal_changed() -> SharedResult<()> {
        roundtrip(&server::item::ItemSealChanged {
            grid_type: MirGridType::Inventory,
            unique_id: 131313,
            expiry_date: 1700000000,
        })
    }

    #[test]
    fn roundtrip_craft_item() -> SharedResult<()> {
        roundtrip(&server::item::CraftItem {
            unique_id: 141414,
            count: 3,
            success: true,
        })
    }

    // ============================================================
    // Chat packets
    // ============================================================

    #[test]
    fn roundtrip_chat_client_with_items() -> SharedResult<()> {
        use crate::data::item::ChatItem;
        let packet = client::Chat {
            message: "Check out this item!".to_string(),
            linked_items: vec![ChatItem {
                unique_id: 98765,
                title: "Dragon Sword".to_string(),
                grid: MirGridType::Inventory,
            }],
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, client::Chat>(&mut cursor)?;
        assert_eq!(decoded.message, "Check out this item!");
        assert_eq!(decoded.linked_items.len(), 1);
        assert_eq!(decoded.linked_items[0].unique_id, 98765);
        assert_eq!(decoded.linked_items[0].title, "Dragon Sword");
        assert_eq!(decoded.linked_items[0].grid, MirGridType::Inventory);
        Ok(())
    }

    #[test]
    fn roundtrip_inspect() -> SharedResult<()> {
        roundtrip(&client::Inspect { object_id: 5555 })
    }

    #[test]
    fn roundtrip_observe() -> SharedResult<()> {
        roundtrip(&client::Observe {
            name: "Observer".to_string(),
        })
    }

    #[test]
    fn roundtrip_object_chat() -> SharedResult<()> {
        roundtrip(&server::ObjectChat {
            object_id: 7777,
            text: "Hello adventurer!".to_string(),
            chat_type: ChatType::Normal,
        })
    }

    // ============================================================
    // Magic/Combat packet roundtrips
    // ============================================================

    #[test]
    fn roundtrip_magic_cast() -> SharedResult<()> {
        let packet = server::magic_combat::MagicCast {
            spell: Spell::FireBall,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::magic_combat::MagicCast>(&mut cursor)?;
        assert_eq!(decoded.spell, Spell::FireBall);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_object_magic() -> SharedResult<()> {
        use server::magic_combat::ObjectMagic;
        let packet = ObjectMagic {
            object_id: 1001,
            location_x: 50,
            location_y: 60,
            direction: MirDirection::Down,
            spell: Spell::FireBall,
            target_id: 2001,
            target_x: 55,
            target_y: 65,
            cast: true,
            level: 2,
            self_broadcast: false,
            secondary_target_ids: vec![2002, 2003],
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, ObjectMagic>(&mut cursor)?;
        assert_eq!(decoded.object_id, 1001);
        assert_eq!(decoded.spell, Spell::FireBall);
        assert_eq!(decoded.level, 2);
        assert_eq!(decoded.secondary_target_ids.len(), 2);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_object_spell() -> SharedResult<()> {
        let packet = server::magic_combat::ObjectSpell {
            object_id: 9001,
            location_x: 30,
            location_y: 40,
            spell: Spell::FireWall,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::magic_combat::ObjectSpell>(&mut cursor)?;
        assert_eq!(decoded.object_id, 9001);
        assert_eq!(decoded.spell, Spell::FireWall);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_projectile() -> SharedResult<()> {
        let packet = server::magic_combat::ObjectProjectile {
            spell: Spell::FireBall,
            source: 1001,
            destination: 2001,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::magic_combat::ObjectProjectile>(&mut cursor)?;
        assert_eq!(decoded.spell, Spell::FireBall);
        assert_eq!(decoded.source, 1001);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_effect() -> SharedResult<()> {
        let packet = server::magic_combat::ObjectEffect {
            object_id: 1001,
            effect: crate::enums::SpellEffect::Healing,
            effect_type: 1,
            delay_time: 0,
            time: 500,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::magic_combat::ObjectEffect>(&mut cursor)?;
        assert_eq!(decoded.object_id, 1001);
        assert_eq!(decoded.time, 500);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_new_magic() -> SharedResult<()> {
        use server::magic::NewMagic;
        let packet = NewMagic {
            magic: crate::data::client_data::ClientMagic {
                name: "FireBall".into(),
                spell: Spell::FireBall,
                base_cost: 20,
                level_cost: 5,
                icon: 1,
                level1: 7,
                level2: 15,
                level3: 22,
                need1: 30,
                need2: 100,
                need3: 200,
                level: 2,
                key: 5,
                experience: 500,
                delay: 1800,
                range: 9,
                cast_time: 0,
            },
            hero: false,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, NewMagic>(&mut cursor)?;
        assert_eq!(decoded.magic.spell, Spell::FireBall);
        assert!(!decoded.hero);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_leveled() -> SharedResult<()> {
        let packet = server::magic::MagicLeveled {
            object_id: 1000,
            spell: Spell::FireBall,
            level: 2,
            experience: 500,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::magic::MagicLeveled>(&mut cursor)?;
        assert_eq!(decoded.spell, Spell::FireBall);
        assert_eq!(decoded.level, 2);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_delay() -> SharedResult<()> {
        let packet = server::magic_combat::MagicDelay {
            object_id: 1001,
            spell: Spell::FireBall,
            delay: 1800,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::magic_combat::MagicDelay>(&mut cursor)?;
        assert_eq!(decoded.object_id, 1001);
        assert_eq!(decoded.delay, 1800);
        Ok(())
    }

    #[test]
    fn roundtrip_magic_spell_toggle_hero() -> SharedResult<()> {
        let packet = server::magic::SpellToggle {
            spell: Spell::MagicShield,
            can_use: true,
            hero: true,
        };
        let mut bytes = Vec::new();
        serialize_packet(&mut bytes, &packet)?;
        let mut cursor = Cursor::new(bytes);
        let decoded = deserialize_packet::<_, server::magic::SpellToggle>(&mut cursor)?;
        assert_eq!(decoded.spell, Spell::MagicShield);
        assert!(decoded.can_use);
        assert!(decoded.hero);
        Ok(())
    }
}
