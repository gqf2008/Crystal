use client_bevy::game::pathfinding;
use client_bevy::map_renderer::LoadedMap;
use client_bevy::resources::map_reader::{resolve_map_path, MapReader};
fn main() {
    let map_reader = MapReader::new(&resolve_map_path("3")).unwrap();
    let mut walkable = Vec::with_capacity(map_reader.width as usize);
    for x in 0..map_reader.width {
        let mut col = Vec::with_capacity(map_reader.height as usize);
        for y in 0..map_reader.height { col.push(map_reader.map_cells[x as usize][y as usize].is_walkable()); }
        walkable.push(col);
    }
    let map = LoadedMap { name: "3".into(), width: map_reader.width, height: map_reader.height, walkable };
    let p = pathfinding::find_path(&map, (500,400), (540,440)).unwrap();
    let mut prev = (500,400);
    let mut seq = Vec::new();
    for n in &p {
        seq.push(format!("({},{})", n.0-prev.0, n.1-prev.1));
        prev = *n;
    }
    println!("seq: {}", seq.join(" "));
}