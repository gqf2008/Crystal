// 简单测试：显示前10x10格子的内容
use mir2_client::objects::MapReader;

fn main() -> std::io::Result<()> {
    println!("=== 地图格子内容检查 ===\n");
    
    let reader = MapReader::new("Map/0.map")?;
    println!("✅ 地图: {}x{}\n", reader.width, reader.height);
    
    println!("前10x10格子:");
    for y in 0..10.min(reader.height) {
        println!("\n--- 行 {} ---", y);
        for x in 0..10.min(reader.width) {
            if let Some(cell) = reader.get_cell(x, y) {
                if cell.back_image > 0 || cell.middle_image > 0 || cell.front_image > 0 {
                    println!("({}, {}): Back={}/{}, Mid={}/{}, Front={}/{}", 
                        x, y,
                        cell.back_image, cell.back_index,
                        cell.middle_image, cell.middle_index,
                        cell.front_image, cell.front_index
                    );
                }
            }
        }
    }
    
    Ok(())
}
