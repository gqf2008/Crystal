// Gate 帧编解码：长度前缀 + XOR 加密
// 对应 C# LoginGate/ClientSession.cs 中的协议解析逻辑

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

/// XOR 密钥（与客户端 config.ini 中的 GateKey 对应）
const DEFAULT_XOR_KEY: u8 = 0xAA;

/// 编码：长度前缀(2字节) + XOR 加密
pub fn encode(data: &[u8], out: &mut Vec<u8>) {
    let len = data.len() as u16;
    out.reserve(2 + len as usize);

    // 写入长度（小端）
    let mut cursor = Cursor::new(Vec::with_capacity(2));
    cursor.write_u16::<LittleEndian>(len).unwrap();
    out.extend_from_slice(&cursor.into_inner());

    // XOR 加密
    out.extend(data.iter().map(|b| b ^ DEFAULT_XOR_KEY));
}

/// 解码：读取长度前缀 + XOR 解密
/// 返回解码后的数据和消耗的字节数
pub fn decode(buf: &[u8]) -> Option<(Vec<u8>, usize)> {
    if buf.len() < 2 {
        return None;
    }

    // 读取长度
    let mut cursor = Cursor::new(buf);
    let len = cursor.read_u16::<LittleEndian>().ok()? as usize;

    // 检查是否有足够数据
    let total = 2 + len;
    if buf.len() < total {
        return None;
    }

    // XOR 解密
    let payload = buf[2..total]
        .iter()
        .map(|b| b ^ DEFAULT_XOR_KEY)
        .collect();

    Some((payload, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let data = vec![1, 2, 3, 4, 5];
        let mut encoded = Vec::new();
        encode(&data, &mut encoded);

        assert_eq!(encoded.len(), 7); // 2字节长度 + 5字节数据

        let (decoded, consumed) = decode(&encoded).unwrap();
        assert_eq!(decoded, data);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_decode_incomplete() {
        // 只有长度前缀，没有数据体
        let buf = vec![5, 0]; // len=5, but no payload
        assert!(decode(&buf).is_none());
    }

    #[test]
    fn test_decode_empty() {
        assert!(decode(&[]).is_none());
    }
}
