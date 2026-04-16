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

pub fn serialize_packet<W: Write, P: Packet>(
    writer: &mut W,
    packet: &P,
) -> SharedResult<()> {
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
            eprintln!("❌ ERROR: header.length={} < HEADER_SIZE={}", header.length, PacketHeader::HEADER_SIZE);
            return Err(SharedError::InvalidPacketLength(header.length));
        }
    };
    
    // 🔒 安全检查: 防止巨量内存分配 (51GB 崩溃修复)
    const MAX_BODY_SIZE: usize = 1024 * 1024; // 1MB 上限
    if body_len > MAX_BODY_SIZE {
        eprintln!("❌ FATAL: body_len={} exceeds MAX_BODY_SIZE={}", body_len, MAX_BODY_SIZE);
        eprintln!("   header.length={}, opcode={}", header.length, header.opcode);
        return Err(SharedError::PacketTooLarge(body_len));
    }
    
    eprintln!("DEBUG: Allocating body vec of {} bytes (opcode={})", body_len, header.opcode);
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

    use crate::packets::server::{
        Chat as ServerChat, ObjectRemove, PlayerUpdate, HealthChanged,
    };
    use crate::packets::client::Chat as ClientChat;
    use crate::enums::{MirDirection, ChatType};

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
        let packet = ObjectRemove {
            object_id: 12345,
        };
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
        let packet = HealthChanged {
            hp: 500,
            mp: 200,
        };
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
}
