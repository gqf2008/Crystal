// 诊断工具：验证地图解析 + 地图库图像加载（控制台程序，无 GUI）
use client_bevy::resources::libraries::{resolve_data_path, Libraries};
use client_bevy::resources::map_reader::{resolve_map_path, MapReader};

fn main() {
    let map_name = std::env::args().nth(1).unwrap_or_else(|| "0100".to_string());
    let path = resolve_map_path(&map_name);
    println!("map path: {}", path);
    let map = MapReader::new(&path).expect("map load failed");
    println!("map size: {}x{}", map.width, map.height);

    let mut back = 0usize;
    let mut middle = 0usize;
    let mut front = 0usize;
    for x in 0..map.width {
        for y in 0..map.height {
            let c = &map.map_cells[x as usize][y as usize];
            if c.back_tile().is_some() { back += 1; }
            if c.middle_tile().is_some() { middle += 1; }
            if c.front_tile().is_some() { front += 1; }
        }
    }
    println!("cells with tiles: back={} middle={} front={}", back, middle, front);

    let data_path = resolve_data_path();
    println!("data path: {}", data_path.display());
    let mut libs = Libraries::new(data_path);
    libs.init_map_libraries();
    let (s, m) = libs.stats();
    println!("libs: single={} map={}", s, m);

    // 采样：每层前 10 个瓦片
    let layers: [(&str, fn(&client_bevy::resources::map_reader::CellInfo) -> Option<(i16, i32)>); 3] = [
        ("back", client_bevy::resources::map_reader::CellInfo::back_tile),
        ("middle", client_bevy::resources::map_reader::CellInfo::middle_tile),
        ("front", client_bevy::resources::map_reader::CellInfo::front_tile),
    ];
    for (layer_name, getter) in layers {
        let mut sampled = 0;
        'outer: for x in 0..map.width {
            for y in 0..map.height {
                let c = &map.map_cells[x as usize][y as usize];
                if let Some((li, ii)) = getter(c) {
                    match libs.get_map_image_debug(li, ii) {
                        Ok(info) => {
                            println!(
                                "  {} tile ({},{}): lib={} img={} -> OK {}x{} rgba={}",
                                layer_name, x, y, li, ii,
                                info.width, info.height, info.rgba.is_some()
                            );
                        }
                        Err(e) => {
                            println!("  {} tile ({},{}): lib={} img={} -> {}", layer_name, x, y, li, ii, e);
                        }
                    }
                    sampled += 1;
                    if sampled >= 10 { break 'outer; }
                }
            }
        }
        if sampled == 0 {
            println!("  {}: no tiles found in map!", layer_name);
        }
    }

    // 验证 MapLibs 关键槽位
    for idx in [0i16, 1, 2, 100, 110, 120, 121, 190, 200] {
        let lib = libs.get_map_library(idx);
        println!(
            "MapLibs[{}] = {}",
            idx,
            match lib {
                Some(l) => format!("loaded ({} images)", l.count()),
                None => "MISSING".to_string(),
            }
        );
    }
}
