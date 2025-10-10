// 独立测试 Frame::from_reader 功能

use std::io::Cursor;

// 从 frames.rs 复制的最小实现用于测试
use std::io::{Read, Result as IoResult};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub start: i32,
    pub count: i32,
    pub skip: i32,
    pub interval: i32,
    pub effect_start: i32,
    pub effect_count: i32,
    pub effect_skip: i32,
    pub effect_interval: i32,
    pub reverse: bool,
    pub blend: bool,
}

impl Frame {
    pub fn from_reader<R: Read>(reader: &mut R) -> IoResult<Self> {
        let start = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_i32::<LittleEndian>()?;
        let skip = reader.read_i32::<LittleEndian>()?;
        let interval = reader.read_i32::<LittleEndian>()?;
        let effect_start = reader.read_i32::<LittleEndian>()?;
        let effect_count = reader.read_i32::<LittleEndian>()?;
        let effect_skip = reader.read_i32::<LittleEndian>()?;
        let effect_interval = reader.read_i32::<LittleEndian>()?;
        let reverse = reader.read_u8()? != 0;
        let blend = reader.read_u8()? != 0;
        
        Ok(Self {
            start,
            count,
            skip,
            interval,
            effect_start,
            effect_count,
            effect_skip,
            effect_interval,
            reverse,
            blend,
        })
    }
}

fn main() {
    println!("测试 Frame::from_reader 实现\n");
    
    // 测试 1: 正常数据
    println!("测试 1: 读取正常帧数据");
    let data: Vec<u8> = vec![
        100, 0, 0, 0,  // Start: 100
        8, 0, 0, 0,    // Count: 8
        0, 0, 0, 0,    // Skip: 0
        120, 0, 0, 0,  // Interval: 120
        200, 0, 0, 0,  // EffectStart: 200
        10, 0, 0, 0,   // EffectCount: 10
        2, 0, 0, 0,    // EffectSkip: 2
        150, 0, 0, 0,  // EffectInterval: 150
        1,             // Reverse: true
        0,             // Blend: false
    ];
    
    let mut cursor = Cursor::new(data);
    match Frame::from_reader(&mut cursor) {
        Ok(frame) => {
            println!("✅ 成功读取帧数据:");
            println!("   start={}, count={}, skip={}, interval={}", 
                frame.start, frame.count, frame.skip, frame.interval);
            println!("   effect_start={}, effect_count={}, effect_skip={}, effect_interval={}", 
                frame.effect_start, frame.effect_count, frame.effect_skip, frame.effect_interval);
            println!("   reverse={}, blend={}", frame.reverse, frame.blend);
            
            assert_eq!(frame.start, 100);
            assert_eq!(frame.count, 8);
            assert_eq!(frame.skip, 0);
            assert_eq!(frame.interval, 120);
            assert_eq!(frame.effect_start, 200);
            assert_eq!(frame.effect_count, 10);
            assert_eq!(frame.effect_skip, 2);
            assert_eq!(frame.effect_interval, 150);
            assert!(frame.reverse);
            assert!(!frame.blend);
        }
        Err(e) => {
            panic!("❌ 读取失败: {}", e);
        }
    }
    
    // 测试 2: 负数 skip
    println!("\n测试 2: 读取负数 skip 值（如 DragonStatue）");
    let data2: Vec<u8> = vec![
        44, 1, 0, 0,           // Start: 300
        1, 0, 0, 0,            // Count: 1
        255, 255, 255, 255,    // Skip: -1
        232, 3, 0, 0,          // Interval: 1000
        0, 0, 0, 0,            // EffectStart: 0
        0, 0, 0, 0,            // EffectCount: 0
        0, 0, 0, 0,            // EffectSkip: 0
        0, 0, 0, 0,            // EffectInterval: 0
        0,                     // Reverse: false
        1,                     // Blend: true
    ];
    
    let mut cursor2 = Cursor::new(data2);
    match Frame::from_reader(&mut cursor2) {
        Ok(frame) => {
            println!("✅ 成功读取负数 skip:");
            println!("   start={}, count={}, skip={}, interval={}", 
                frame.start, frame.count, frame.skip, frame.interval);
            println!("   reverse={}, blend={}", frame.reverse, frame.blend);
            
            assert_eq!(frame.start, 300);
            assert_eq!(frame.count, 1);
            assert_eq!(frame.skip, -1);
            assert_eq!(frame.interval, 1000);
            assert!(!frame.reverse);
            assert!(frame.blend);
        }
        Err(e) => {
            panic!("❌ 读取失败: {}", e);
        }
    }
    
    // 测试 3: 不完整数据（错误处理）
    println!("\n测试 3: 不完整数据（应该返回错误）");
    let data3: Vec<u8> = vec![1, 2, 3, 4, 5];
    let mut cursor3 = Cursor::new(data3);
    match Frame::from_reader(&mut cursor3) {
        Ok(_) => {
            panic!("❌ 应该返回错误但成功了");
        }
        Err(e) => {
            println!("✅ 正确返回错误: {}", e);
        }
    }
    
    println!("\n🎉 所有测试通过！");
    println!("\nFrame::from_reader 实现正确，可以从二进制流读取帧数据。");
    println!("对应 C# 的 Frame(BinaryReader reader) 构造函数。");
}
