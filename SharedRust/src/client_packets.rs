use std::io::{Cursor, Read, Write};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::ClientPacketIds;
use crate::packet::{deserialize_packet, PacketHeader, PacketMessage};
use crate::stats::{SharedError, SharedResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientVersion {
    pub version_hash: Vec<u8>,
}

impl PacketMessage for ClientVersion {
    const OPCODE: i16 = ClientPacketIds::ClientVersion as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let length = reader.read_i32::<LittleEndian>()?;
        let length = usize::try_from(length).map_err(|_| SharedError::NegativeLength {
            field: "version_hash",
            length,
        })?;
        let mut version_hash = vec![0; length];
        reader.read_exact(&mut version_hash)?;
        Ok(Self { version_hash })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        let length = i32::try_from(self.version_hash.len())
            .map_err(|_| SharedError::PacketTooLarge(self.version_hash.len()))?;
        writer.write_i32::<LittleEndian>(length)?;
        writer.write_all(&self.version_hash)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Disconnect;

impl PacketMessage for Disconnect {
    const OPCODE: i16 = ClientPacketIds::Disconnect as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepAlive {
    pub time: i64,
}

impl PacketMessage for KeepAlive {
    const OPCODE: i16 = ClientPacketIds::KeepAlive as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let time = reader.read_i64::<LittleEndian>()?;
        Ok(Self { time })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.time)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccount {
    pub account_id: String,
    pub password: String,
    pub birth_date_binary: i64,
    pub user_name: String,
    pub secret_question: String,
    pub secret_answer: String,
    pub email_address: String,
}

impl Default for NewAccount {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            password: String::new(),
            birth_date_binary: 0,
            user_name: String::new(),
            secret_question: String::new(),
            secret_answer: String::new(),
            email_address: String::new(),
        }
    }
}

impl PacketMessage for NewAccount {
    const OPCODE: i16 = ClientPacketIds::NewAccount as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            account_id: read_dotnet_string(reader)?,
            password: read_dotnet_string(reader)?,
            birth_date_binary: reader.read_i64::<LittleEndian>()?,
            user_name: read_dotnet_string(reader)?,
            secret_question: read_dotnet_string(reader)?,
            secret_answer: read_dotnet_string(reader)?,
            email_address: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.account_id)?;
        write_dotnet_string(writer, &self.password)?;
        writer.write_i64::<LittleEndian>(self.birth_date_binary)?;
        write_dotnet_string(writer, &self.user_name)?;
        write_dotnet_string(writer, &self.secret_question)?;
        write_dotnet_string(writer, &self.secret_answer)?;
        write_dotnet_string(writer, &self.email_address)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePassword {
    pub account_id: String,
    pub current_password: String,
    pub new_password: String,
}

impl Default for ChangePassword {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            current_password: String::new(),
            new_password: String::new(),
        }
    }
}

impl PacketMessage for ChangePassword {
    const OPCODE: i16 = ClientPacketIds::ChangePassword as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            account_id: read_dotnet_string(reader)?,
            current_password: read_dotnet_string(reader)?,
            new_password: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.account_id)?;
        write_dotnet_string(writer, &self.current_password)?;
        write_dotnet_string(writer, &self.new_password)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub account_id: String,
    pub password: String,
}

impl Default for Login {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            password: String::new(),
        }
    }
}

impl PacketMessage for Login {
    const OPCODE: i16 = ClientPacketIds::Login as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            account_id: read_dotnet_string(reader)?,
            password: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.account_id)?;
        write_dotnet_string(writer, &self.password)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartGame {
    pub character_index: i32,
}

impl PacketMessage for StartGame {
    const OPCODE: i16 = ClientPacketIds::StartGame as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let character_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { character_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        Ok(())
    }
}

pub fn decode_header(buffer: &[u8]) -> Option<PacketHeader> {
    if buffer.len() < PacketHeader::HEADER_SIZE {
        return None;
    }

    let length = LittleEndian::read_u16(&buffer[0..2]);
    let opcode = LittleEndian::read_i16(&buffer[2..4]);
    Some(PacketHeader::new(length, opcode))
}

pub fn decode_packet_from_parts<P: PacketMessage>(
    header: &PacketHeader,
    payload: &[u8],
) -> SharedResult<P> {
    let mut buffer = Vec::with_capacity(PacketHeader::HEADER_SIZE + payload.len());
    header.write_to(&mut buffer)?;
    buffer.extend_from_slice(payload);
    let mut cursor = Cursor::new(buffer);
    deserialize_packet::<_, P>(&mut cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{deserialize_packet, serialize_packet};
    use std::io::Cursor;

    #[test]
    fn client_version_round_trip() -> SharedResult<()> {
        let packet = ClientVersion {
            version_hash: vec![1, 2, 3, 4, 5],
        };
        let mut buffer = Vec::new();
        serialize_packet(&mut buffer, &packet)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = deserialize_packet::<_, ClientVersion>(&mut cursor)?;
        assert_eq!(decoded, packet);
        Ok(())
    }

    #[test]
    fn new_account_round_trip() -> SharedResult<()> {
        let packet = NewAccount {
            account_id: "MirPlayer".to_string(),
            password: "Secret123".to_string(),
            birth_date_binary: 638155392000000000,
            user_name: "TheHero".to_string(),
            secret_question: "Color?".to_string(),
            secret_answer: "Blue".to_string(),
            email_address: "hero@example.com".to_string(),
        };
        let mut buffer = Vec::new();
        serialize_packet(&mut buffer, &packet)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = deserialize_packet::<_, NewAccount>(&mut cursor)?;
        assert_eq!(decoded, packet);
        Ok(())
    }

    #[test]
    fn login_round_trip() -> SharedResult<()> {
        let packet = Login {
            account_id: "User".to_string(),
            password: "Pw".to_string(),
        };
        let mut buffer = Vec::new();
        serialize_packet(&mut buffer, &packet)?;
        let mut cursor = Cursor::new(buffer);
        let decoded = deserialize_packet::<_, Login>(&mut cursor)?;
        assert_eq!(decoded, packet);
        Ok(())
    }
}
