use client_bevy::resources::map_reader::{resolve_map_path, MapReader};
fn main() {
    for name in ["n0","0","3","4","5","6","7","8","9","10","11","12","13","14","15","20","21"] {
        let Ok(map) = MapReader::new(&resolve_map_path(name)) else { continue };
        let (mut fa, mut fa_blend, mut ta, mut mid_anim, mut light, mut light_19, mut light_ge10) = (0,0,0,0,0,0,0);
        for x in 0..map.width as usize {
            for y in 0..map.height as usize {
                let c = &map.map_cells[x][y];
                if c.front_animation_frame > 0 { fa += 1; if c.front_animation_frame & 0x80 != 0 { fa_blend += 1; } }
                if c.tile_animation_frames > 0 { ta += 1; }
                if c.middle_animation_frame > 0 { mid_anim += 1; }
                if c.light > 0 { light += 1; if c.light < 10 { light_19 += 1; } else { light_ge10 += 1; } }
            }
        }
        println!("map {} {}x{}: front_anim={} blend={} tileanim={} midanim={} light={} light_1_9={} light_ge10={}",
            name, map.width, map.height, fa, fa_blend, ta, mid_anim, light, light_19, light_ge10);
    }
}