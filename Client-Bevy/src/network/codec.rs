// 客户端帧编解码：长度前缀(2字节 LE) + XOR 0xAA
// 与服务端 src/gate/codec.rs 完全对称

const XOR_KEY: u8 = 0xAA;
/// 单帧最大字节数，防止恶意长度导致 OOM（游戏包通常 < 1KB）
const MAX_FRAME_LEN: usize = 32 * 1024;

/// 编码：写入 [2-byte LE length][XOR(payload)] 到 out
pub fn encode(data: &[u8], out: &mut Vec<u8>) {
    let len = data.len() as u16;
    out.reserve(2 + len as usize);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend(data.iter().map(|b| b ^ XOR_KEY));
}

/// 从缓冲区内尝试解码一帧
/// 返回 (解码后的payload, 消耗的总字节数) 或 None(数据不足)
pub fn decode(buf: &[u8]) -> Option<Result<(Vec<u8>, usize), std::io::Error>> {
    if buf.len() < 2 {
        return None;
    }
    let len = u16::from_le_bytes([buf[0], buf[1]]) as usize;
    if len > MAX_FRAME_LEN {
        return Some(Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame length {} exceeds max {}", len, MAX_FRAME_LEN),
        )));
    }
    let total = 2 + len;
    if buf.len() < total {
        return None;
    }
    let payload = buf[2..total].iter().map(|b| b ^ XOR_KEY).collect();
    Some(Ok((payload, total)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let data = vec![1, 2, 3, 4, 5];
        let mut encoded = Vec::new();
        encode(&data, &mut encoded);
        assert_eq!(encoded.len(), 7);
        let result = decode(&encoded).unwrap().unwrap();
        assert_eq!(result.0, data);
        assert_eq!(result.1, encoded.len());
    }

    #[test]
    fn test_decode_incomplete() {
        let buf = vec![5, 0]; // len=5, no payload
        assert!(decode(&buf).is_none());
    }

    #[test]
    fn test_decode_empty() {
        assert!(decode(&[]).is_none());
    }

    #[test]
    fn test_decode_oversized_frame() {
        // declared len = 0x8001 (32769) > MAX_FRAME_LEN (32KB)
        let buf = vec![0x01, 0x80];
        let result = decode(&buf).unwrap();
        assert!(result.is_err());
    }
}
