use client_bevy::resources::map_reader::{resolve_map_path, MapReader};
fn main() {
    let map = MapReader::new(&resolve_map_path("5")).unwrap();
    let (cx, cy) = (map.width as i32 / 2, map.height as i32 / 2);
    let mut found = 0;
    'outer: for r in 0..40 { for dy in -r..=r { for dx in -r..=r {
        let (x, y) = (cx + dx, cy + dy);
        if x >= 0 && y >= 0 && (x as usize) < map.width as usize && (y as usize) < map.height as usize && map.map_cells[x as usize][y as usize].is_walkable() {
            let mut open = 0;
            for yy in (y-4)..=(y+4) { for xx in (x-4)..=(x+4) {
                if xx >= 0 && yy >= 0 && (xx as usize) < map.width as usize && (yy as usize) < map.height as usize && map.map_cells[xx as usize][yy as usize].is_walkable() { open += 1; }
            }}
            println!("walkable ({},{}) open={}", x, y, open);
            found += 1;
            if found >= 6 { break 'outer; }
        }
    }}}
}