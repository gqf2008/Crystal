// 离屏诊断：导出地图块纹理 PNG，验证瓦片合成对齐
// 用法: cargo run --example dump_chunk -- <map> <cx> <cy>
use client_bevy::map_renderer::{build_chunk_rgba, Layer};
use client_bevy::resources::libraries::{resolve_data_path, Libraries};
use client_bevy::resources::map_reader::{resolve_map_path, MapReader};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let map_name = args.get(1).cloned().unwrap_or_else(|| "n0".to_string());
    let cx: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(17);
    let cy: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(17);

    let path = resolve_map_path(&map_name);
    let map = MapReader::new(&path).expect("map load failed");
    println!("map {} {}x{}", path, map.width, map.height);

    let mut libs = Libraries::new(resolve_data_path());
    libs.init_map_libraries();
    let (_, m) = libs.stats();
    println!("map libs loaded: {}", m);

    for (layer, name) in [
        (Layer::Back, "back"),
        (Layer::Middle, "middle"),
        (Layer::Front, "front"),
    ] {
        match build_chunk_rgba(&mut libs, &map, layer, cx, cy) {
            Some(rgba) => {
                let out = format!("E:/tmp/chunk_{}_{}_{}_{}.png", map_name, cx, cy, name);
                // 合成到图像并保存
                let img = image::RgbaImage::from_raw(1536, 1024, rgba).expect("valid size");
                img.save(&out).expect("save png");
                println!("saved {} ({}x{})", out, 1536, 1024);
            }
            None => println!("chunk ({},{}) layer {} is empty", cx, cy, name),
        }
    }
}
