// Integration tests for network module
// This file tests the network functionality in isolation

use mir2_shared::packets::client;
use mir2_shared::packets::{serialize_packet, Packet};
use byteorder::{ByteOrder, LittleEndian};

#[test]
fn test_keepalive_packet_creation() {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    let keepalive = client::KeepAlive { time: timestamp };
    
    // Verify packet can be created
    assert!(keepalive.time > 0);
}

#[test]
fn test_keepalive_packet_serialization() {
    let keepalive = client::KeepAlive {
        time: 1234567890123,
    };
    
    let mut buffer = Vec::new();
    let result = serialize_packet(&mut buffer, &keepalive);
    
    // Verify serialization succeeds
    assert!(result.is_ok());
    
    // Verify packet structure (header + body)
    // Header: 2 bytes length + 2 bytes opcode = 4 bytes
    // Body: 8 bytes (i64 timestamp)
    // Total: 12 bytes
    assert_eq!(buffer.len(), 12);
    
    // Verify packet length field
    let packet_len = LittleEndian::read_u16(&buffer[0..2]);
    assert_eq!(packet_len, 12);
    
    // Verify opcode
    let opcode = LittleEndian::read_i16(&buffer[2..4]);
    assert_eq!(opcode, client::KeepAlive::OPCODE);
    
    // Verify timestamp
    let time = LittleEndian::read_i64(&buffer[4..12]);
    assert_eq!(time, 1234567890123);
}

#[test]
fn test_keepalive_opcode_value() {
    // Verify KeepAlive has correct opcode (should be 2 for client)
    assert_eq!(client::KeepAlive::OPCODE, 2);
}

#[test]
fn test_multiple_keepalive_serialization() {
    // Test serializing multiple KeepAlive packets
    for i in 0..10 {
        let keepalive = client::KeepAlive {
            time: i * 1000,
        };
        
        let mut buffer = Vec::new();
        let result = serialize_packet(&mut buffer, &keepalive);
        
        assert!(result.is_ok());
        assert_eq!(buffer.len(), 12);
        
        // Verify each packet has unique timestamp
        let time = LittleEndian::read_i64(&buffer[4..12]);
        assert_eq!(time, i * 1000);
    }
}

#[test]
fn test_system_time_conversion() {
    // Test that we can get current timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    // Timestamp should be reasonable (after 2020)
    let min_timestamp = 1577836800000i64; // 2020-01-01 00:00:00 UTC in milliseconds
    assert!(timestamp > min_timestamp);
    
    // Timestamp should be before 2100
    let max_timestamp = 4102444800000i64; // 2100-01-01 00:00:00 UTC in milliseconds
    assert!(timestamp < max_timestamp);
}
