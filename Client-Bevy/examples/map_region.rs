use client_bevy::resources::map_reader::{resolve_map_path, MapReader};
fn main() {
    let map = MapReader::new(&resolve_map_path("n0")).unwrap();
    for (y0, y1, label) in [(0u32,100,"top 0-100"),(100,200,"100-200"),(200,300,"200-300"),(300,400,"300-400"),(400,500,"400-500"),(500,600,"500-600"),(600,700,"600-700")] {
        let (mut back, mut mid, mut front) = (0usize,0usize,0usize);
        for x in 0..map.width as usize {
            for y in y0 as usize..y1 as usize {
                if map.map_cells[x][y].back_tile().is_some() { back += 1; }
                if map.map_cells[x][y].middle_tile().is_some() { mid += 1; }
                if map.map_cells[x][y].front_tile().is_some() { front += 1; }
            }
        }
        println!("{}: back={} mid={} front={}", label, back, mid, front);
    }
}
