use std::fs::File;
use std::io::Read;

fn main() {
    let mut f = File::open("Map/0.map").unwrap();
    let mut data = Vec::new();
    f.read_to_end(&mut data).unwrap();
    
    let width = i16::from_le_bytes([data[0], data[1]]) as i32;
    let height = i16::from_le_bytes([data[2], data[3]]) as i32;
    
    println!("地图尺寸: {}x{}", width, height);
    println!("检查前10个单元格的Middle数据:");
    
    let mut offset = 4;
    for i in 0..10 {
        let back_img = i16::from_le_bytes([data[offset], data[offset+1]]);
        let middle_img = i16::from_le_bytes([data[offset+2], data[offset+3]]);
        let front_img = i16::from_le_bytes([data[offset+4], data[offset+5]]);
        let back_idx = data[offset+6];
        let middle_idx = data[offset+7];
        let front_idx = data[offset+8];
        
        println!("单元格[{}]: back_img={} back_idx={} middle_img={} middle_idx={} front_img={} front_idx={}", 
                 i, back_img, back_idx, middle_img, middle_idx, front_img, front_idx);
        
        offset += 13;
    }
}
