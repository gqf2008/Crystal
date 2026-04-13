// Wire format utilities: packet framing and string encoding

/// 构建完整的内层数据包（带 PacketHeader）
pub fn build_packet_bytes(opcode: i16, body: &[u8]) -> Vec<u8> {
    const HEADER_SIZE: usize = 4;
    let total_len = HEADER_SIZE + body.len();
    let length = total_len as u16;
    let mut out = Vec::with_capacity(total_len);
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// 写 DotNet 7-bit 编码字符串
pub fn write_dotnet_string(body: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    let mut v = bytes.len();
    loop {
        let mut b = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 { b |= 0x80; }
        body.push(b);
        if v == 0 { break; }
    }
    body.extend_from_slice(bytes);
}
