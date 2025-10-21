// ============================================================================
// Animation & Door Systems - 动画和门系统
// ============================================================================

use hecs::World;
use std::time::Instant;
use crate::ecs::components::{MapTile, TileLayer, AnimatedTile, Door, DoorState};

/// 动画系统
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn update(world: &mut World, animation_count: i32) {
        for (_entity, (tile, anim)) in world.query_mut::<(&mut MapTile, &AnimatedTile)>() {
            let total_frames = anim.frame_count as i32 + (anim.frame_count as i32 * anim.frame_interval as i32);
            let frame_offset = (animation_count % total_frames) / (1 + anim.frame_interval as i32);
            tile.image_index = anim.base_image_index + frame_offset;
        }
    }
}

/// 门系统
pub struct DoorSystem;

impl DoorSystem {
    pub fn update(world: &mut World) {
        for (_entity, (tile, door)) in world.query_mut::<(&mut MapTile, &mut Door)>() {
            match door.state {
                DoorState::Opening => {
                    if door.last_tick.elapsed().as_millis() > 100 {
                        door.current_frame += 1;
                        if door.current_frame >= 8 {
                            door.current_frame = 8;
                            door.state = DoorState::Open;
                        }
                        door.last_tick = Instant::now();
                    }
                }
                DoorState::Closing => {
                    if door.last_tick.elapsed().as_millis() > 100 {
                        door.current_frame -= 1;
                        if door.current_frame <= 0 {
                            door.current_frame = 0;
                            door.state = DoorState::Closed;
                        }
                        door.last_tick = Instant::now();
                    }
                }
                _ => {}
            }
            
            // 更新瓦片图像索引
            if door.current_frame > 0 {
                tile.image_index += (door.current_frame + 1) * door.door_offset;
            }
        }
    }
}
