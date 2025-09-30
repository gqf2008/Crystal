use std::convert::TryFrom;
use std::io::{Cursor, Read, Write};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};

use crate::stats::{SharedError, SharedResult};

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
        Ok(Self { length, opcode })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u16::<LittleEndian>(self.length)?;
        writer.write_i16::<LittleEndian>(self.opcode)?;
        Ok(())
    }
}

pub trait PacketMessage: Sized {
    const OPCODE: i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self>;
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()>;

    fn is_compressed() -> bool {
        false
    }
}

pub fn serialize_packet<W: Write, P: PacketMessage>(
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
    header.write_to(writer)?;
    writer.write_all(&body)?;
    Ok(())
}

pub fn deserialize_packet<R: Read, P: PacketMessage>(reader: &mut R) -> SharedResult<P> {
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

    let body_len = header.length as usize - PacketHeader::HEADER_SIZE;
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

pub fn extract_packet<P: PacketMessage>(buffer: &[u8]) -> SharedResult<Option<(P, Vec<u8>)>> {
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

    impl PacketMessage for SimplePacket {
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

    impl PacketMessage for CompressedPacket {
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
}
