// 真实地图验证 + 寻路性能基准（#27）
// 用法: cargo run --example verify_maps
// 解析本地真实地图（n0/0/3/5），检查出生点可行走、绕障可达、700x700 寻路耗时
use client_bevy::game::pathfinding;
use client_bevy::map_renderer::LoadedMap;
use client_bevy::resources::map_reader::{resolve_map_path, MapReader};
use std::time::Instant;

fn main() {
    let maps = [
        ("n0", 354, 352),
        ("0", 347, 347),
        ("3", 500, 400),
        ("5", 400, 400),
    ];
    for (name, sx, sy) in maps {
        let Ok(reader) = MapReader::new(&resolve_map_path(name)) else {
            println!("{name}: 加载失败（数据缺失）");
            continue;
        };
        let mut walkable = Vec::with_capacity(reader.width as usize);
        for x in 0..reader.width {
            let mut col = Vec::with_capacity(reader.height as usize);
            for y in 0..reader.height {
                col.push(reader.map_cells[x as usize][y as usize].is_walkable());
            }
            walkable.push(col);
        }
        let map = LoadedMap {
            name: name.into(),
            width: reader.width,
            height: reader.height,
            walkable,
        };
        let spawn_walkable = map.is_walkable(sx, sy);
        let reachable = map.walkable.iter().flatten().filter(|w| **w).count();
        println!(
            "{name}: {}x{} 出生点({},{})可走={} 可行走格={} ({:.1}%)",
            map.width,
            map.height,
            sx,
            sy,
            spawn_walkable,
            reachable,
            reachable as f64 / (map.width * map.height) as f64 * 100.0
        );
        if !spawn_walkable {
            continue;
        }
        // 向 4 个方向各找一个可达点并寻路
        for (tx, ty) in [(sx + 10, sy), (sx - 10, sy), (sx, sy + 10), (sx, sy - 10)] {
            if !map.in_bounds(tx, ty) || !map.is_walkable(tx, ty) {
                continue;
            }
            let t0 = Instant::now();
            let path = pathfinding::find_path(&map, (sx, sy), (tx, ty));
            let dt = t0.elapsed();
            match path {
                Some(p) => println!(
                    "    -> ({tx},{ty}): {} 步, {:.2}ms",
                    p.len(),
                    dt.as_secs_f64() * 1000.0
                ),
                None => println!("    -> ({tx},{ty}): 不可达, {:.2}ms", dt.as_secs_f64() * 1000.0),
            }
        }
        // 性能：随机 100 条路径（仅在地图内随机取点，统计平均耗时）
        let mut rng: u64 = 0xC0FFEE;
        let mut next = move || {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (rng >> 33) as u32
        };
        let mut total = 0.0f64;
        let mut found = 0usize;
        let n = 100;
        for _ in 0..n {
            let tx = (next() % map.width as u32) as i32;
            let ty = (next() % map.height as u32) as i32;
            if !map.is_walkable(tx, ty) {
                continue;
            }
            let t0 = Instant::now();
            if pathfinding::find_path(&map, (sx, sy), (tx, ty)).is_some() {
                found += 1;
            }
            total += t0.elapsed().as_secs_f64() * 1000.0;
        }
        println!(
            "    随机 {n} 条可达目标：命中 {found}，平均寻路 {:.2}ms，最长未统计",
            total / n as f64
        );
    }
}
