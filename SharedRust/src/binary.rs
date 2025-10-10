use std::io::{Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::data::stats::{SharedError, SharedResult};

pub const DOTNET_STRING_MAX: usize = 10000; // Maximum 10KB strings to prevent buffer overflow attacks

pub fn read_bool<R: Read>(reader: &mut R) -> SharedResult<bool> {
    Ok(reader.read_u8()? != 0)
}

pub fn write_bool<W: Write>(writer: &mut W, value: bool) -> SharedResult<()> {
    writer.write_u8(if value { 1 } else { 0 })?;
    Ok(())
}

pub fn read_dotnet_string<R: Read>(reader: &mut R) -> SharedResult<String> {
    let length = read_7bit_encoded_int(reader)?;
    if length > DOTNET_STRING_MAX {
        return Err(SharedError::StringTooLong { length });
    }
    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer)?;
    let value = String::from_utf8(buffer)?;
    Ok(value)
}

pub fn write_dotnet_string<W: Write>(writer: &mut W, value: &str) -> SharedResult<()> {
    let bytes = value.as_bytes();
    write_7bit_encoded_int(writer, bytes.len())?;
    writer.write_all(bytes)?;
    Ok(())
}

pub fn read_7bit_encoded_int<R: Read>(reader: &mut R) -> SharedResult<usize> {
    let mut count: usize = 0;
    let mut shift = 0;

    loop {
        if shift >= 35 {
            return Err(SharedError::Invalid7BitEncodedInt);
        }
        let byte = reader.read_u8()?;
        count |= ((byte & 0x7F) as usize) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
    }

    Ok(count)
}

pub fn write_7bit_encoded_int<W: Write>(writer: &mut W, mut value: usize) -> SharedResult<()> {
    if value > DOTNET_STRING_MAX {
        return Err(SharedError::StringTooLong { length: value });
    }

    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        writer.write_u8(byte)?;
        if value == 0 {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn bool_roundtrip() -> SharedResult<()> {
        let mut buffer = Vec::new();
        write_bool(&mut buffer, true)?;
        write_bool(&mut buffer, false)?;
        let mut cursor = Cursor::new(buffer);
        assert!(read_bool(&mut cursor)?);
        assert!(!read_bool(&mut cursor)?);
        Ok(())
    }

    #[test]
    fn string_roundtrip() -> SharedResult<()> {
        let expected = "Mir2 Rust ✨";
        let mut buffer = Vec::new();
        write_dotnet_string(&mut buffer, expected)?;
        let mut cursor = Cursor::new(buffer);
        let actual = read_dotnet_string(&mut cursor)?;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn seven_bit_encoding_matches_reference() -> SharedResult<()> {
        let values = [0usize, 1, 127, 128, 16_777_215, 1_000_000_000];
        for value in values {
            let mut buffer = Vec::new();
            write_7bit_encoded_int(&mut buffer, value)?;
            let mut cursor = Cursor::new(buffer);
            let decoded = read_7bit_encoded_int(&mut cursor)?;
            assert_eq!(decoded, value);
        }
        Ok(())
    }
}
