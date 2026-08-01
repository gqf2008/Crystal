// 诊断工具：验证地图解析 + 地图库图像加载（控制台程序，无 GUI）
use client_bevy::resources::libraries::{resolve_data_path, ArrayLibType, Libraries};
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

    // 验证数组库（角色/怪物/NPC）
    println!("===== 数组库校验 =====");
    for (ty, idxs) in [
        (ArrayLibType::CArmours, vec![0usize]),
        (ArrayLibType::CHair, vec![0usize]),
        (ArrayLibType::CWeapons, vec![0usize]),
        (ArrayLibType::Monsters, vec![1usize, 5, 9]),
        (ArrayLibType::Npcs, vec![0usize]),
    ] {
        for idx in idxs {
            match libs.get_array_image_debug(ty, idx, 0) {
                Ok(info) => println!(
                    "✓ {}[{}] img0: {}x{} rgba={}",
                    ty.name(),
                    idx,
                    info.width,
                    info.height,
                    info.rgba.is_some()
                ),
                Err(e) => println!("✗ {}[{}] img0: {}", ty.name(), idx, e),
            }
        }
    }

    // 验证 Back 层 2x2 共享假设：奇数格子的 back_tile 是否与偶数格子一致
    {
        let mut odd_only = 0usize;
        let mut mismatch = 0usize;
        let mut even_total = 0usize;
        let mut odd_checked = 0usize;
        let mut load_ok = 0usize;
        let mut load_fail = 0usize;
        let mut sample = 0usize;
        for x in 0..map.width {
            for y in 0..map.height {
                let c = &map.map_cells[x as usize][y as usize];
                if x % 2 == 0 && y % 2 == 0 {
                    if c.back_tile().is_some() { even_total += 1; }
                    continue;
                }
                // 奇数格子
                let Some((li, ii)) = c.back_tile() else { continue };
                odd_checked += 1;
                let ex = x - (x % 2);
                let ey = y - (y % 2);
                let even_cell = &map.map_cells[ex as usize][ey as usize];
                match even_cell.back_tile() {
                    Some((eli, eii)) if eli == li && eii == ii => {}
                    Some(_) => { mismatch += 1; }
                    None => { odd_only += 1; }
                }
                if mismatch <= 6 && sample < 20 {
                    let ec = even_cell.back_tile();
                    println!(
                        "  MISMATCH ({},{}): odd lib={} img={}, even({},{}) -> {:?}",
                        x, y, li, ii, ex, ey,
                        ec
                    );
                }
                if sample < 8 {
                    let got = libs.get_map_image_debug(li, ii);
                    println!(
                        "  odd cell ({},{}): lib={} img={} -> {}",
                        x, y, li, ii,
                        match got { Ok(info) => format!("OK {}x{}", info.width, info.height), Err(e) => e }
                    );
                    sample += 1;
                }
                if li == 0 || li == 100 || li == 101 || li == 102 {
                    match libs.get_map_image_debug(li, ii) {
                        Ok(_) => load_ok += 1,
                        Err(_) => load_fail += 1,
                    }
                }
            }
        }
        println!(
            "Back 2x2 分析: even_total={} odd_checked={} odd_only(偶数格无瓦片)={} mismatch={} 关键库加载 ok={} fail={}",
            even_total, odd_checked, odd_only, mismatch, load_ok, load_fail
        );
    }

    // 检查地砖图像本身的 alpha 分布（96x64，底半是否透明）
    {
        for (lib, img) in [(104i16, 7952i32), (104, 7951), (104, 7950), (100, 0), (100, 1), (100, 2), (100, 3)] {
            match libs.get_map_image_debug(lib, img) {
                Ok(info) => {
                    if let Some(rgba) = &info.rgba {
                        let w = info.width as usize;
                        let h = info.height as usize;
                        let mut opaque_bottom = 0usize;
                        let mut total_bottom = 0usize;
                        for y in (h/2)..h {
                            for x in 0..w {
                                let a = rgba[(y * w + x) * 4 + 3];
                                if a > 0 { opaque_bottom += 1; }
                                total_bottom += 1;
                            }
                        }
                        println!(
                            "tile lib={} img={} {}x{} bottom-half opaque {:.1}%",
                            lib, img, w, h,
                            opaque_bottom as f64 * 100.0 / total_bottom as f64
                        );
                    }
                }
                Err(e) => println!("tile lib={} img={} ERR {}", lib, img, e),
            }
        }
    }

    // 检查 chunk(10,10) 底部边界附近的行（世界行 348-353）的 back 数据
    {
        for y in 348..=353 {
            let mut desc = format!("world row {}:", y);
            for x in 0..8 {
                let c = &map.map_cells[x as usize][y as usize];
                match c.back_tile() {
                    Some((li, ii)) => desc.push_str(&format!(" ({},{})", li, ii)),
                    None => desc.push_str(" (none)"),
                }
            }
            println!("{}", desc);
        }
    }

    // 统计 front 层瓦片数量与唯一贴图数（决定渲染方案）
    {
        use std::collections::HashSet;
        let mut tiles = 0usize;
        let mut uniq: HashSet<(i16, i32)> = HashSet::new();
        for x in 0..map.width {
            for y in 0..map.height {
                if let Some((li, ii)) = map.map_cells[x as usize][y as usize].front_tile() {
                    tiles += 1;
                    uniq.insert((li, ii));
                }
            }
        }
        println!("front: tiles={} unique_images={}", tiles, uniq.len());
        // 也统计 middle 的
        let mut mtiles = 0usize;
        let mut muniq: HashSet<(i16, i32)> = HashSet::new();
        for x in 0..map.width {
            for y in 0..map.height {
                if let Some((li, ii)) = map.map_cells[x as usize][y as usize].middle_tile() {
                    mtiles += 1;
                    muniq.insert((li, ii));
                }
            }
        }
        println!("middle: tiles={} unique_images={}", mtiles, muniq.len());
    }

    // 估算 front 瓦片条带化（32px/带）后的精灵数
    {
        let mut tiles = 0usize;
        let mut bands = 0usize;
        let mut tall = 0usize;
        for x in 0..map.width {
            for y in 0..map.height {
                let Some((li, ii)) = map.map_cells[x as usize][y as usize].front_tile() else { continue };
                tiles += 1;
                if let Some(info) = libs.get_map_image(li, ii) {
                    let h = info.height as usize;
                    bands += (h + 31) / 32;
                    if h > 48 { tall += 1; }
                }
            }
        }
        println!("front bands estimate: tiles={} bands={} tall(>48px)={}", tiles, bands, tall);
    }

    // 验证 UI 素材（登录/选角）
    {
        libs.ensure_initialized();
        use client_bevy::resources::libraries::LibraryName;
        for (name, idx) in [
            (LibraryName::Prguse, 1084usize),
            (LibraryName::Title, 30usize),
            (LibraryName::Title, 31usize),
            (LibraryName::Title, 32usize),
            (LibraryName::Title, 320usize),
            (LibraryName::Title, 323usize),
            (LibraryName::Title, 326usize),
            (LibraryName::Prguse, 63usize),
            (LibraryName::Title, 200usize),
            (LibraryName::Prguse, 50usize),
            (LibraryName::Title, 107usize),
            (LibraryName::Prguse, 65usize),
            (LibraryName::Title, 40usize),
            (LibraryName::Prguse, 44usize),
            (LibraryName::Title, 660usize),
            (LibraryName::Title, 665usize),
            (LibraryName::Title, 340usize),
            (LibraryName::Title, 343usize),
            (LibraryName::Title, 346usize),
        ] {
            match libs.get_image(name, idx) {
                Some(info) => println!("✓ {:?}[{}] {}x{}", name, idx, info.width, info.height),
                None => println!("✗ {:?}[{}] 加载失败", name, idx),
            }
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
