// ============================================================================
// map_renderer 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use super::*;
use super::chunks_build::build_chunk;

pub(crate) fn spawn_front_chunk(
    commands: &mut Commands,
    libraries: &mut Libraries,
    assets: &mut Assets<Image>,
    tile_cache: &mut TileImageCache,
    blend_materials: &mut Assets<MapBlendMaterial>,
    blend_quad: &Handle<Mesh>,
    front_images: &mut FrontImageCache,
    map: &MapReader,
    cx: i32,
    cy: i32,
) -> usize {
    let mut count = 0usize;
    let f_start_x = cx * CHUNK_TILES as i32;
    let f_start_y = cy * CHUNK_TILES as i32;
    let f_end_x = (f_start_x + CHUNK_TILES as i32).min(map.width);
    let f_end_y = (f_start_y + CHUNK_TILES as i32).min(map.height);
    for x in f_start_x..f_end_x {
        for y in f_start_y..f_end_y {
            let cell = &map.map_cells[x as usize][y as usize];
            if cell.front_animation_frame > 0 {
                // 动画/灯光混合瓦片：单独生成（blend → ADD 混合材质）
                if let Some((file_index, base_image_index)) = cell.front_tile() {
                    let mut animation = cell.front_animation_frame;
                    let blend = (animation & 0x80) > 0;
                    if blend {
                        animation &= 0x7F;
                    }
                    let tick = cell.front_animation_tick;
                    let base_y_world = (y + 1) as f32 * TILE_HEIGHT as f32;
                    let should_apply_offset = if blend {
                        (100..199).contains(&file_index)
                    } else {
                        file_index == 28
                    };
                    if let Some(info) = libraries.get_map_image(file_index, base_image_index) {
                        let off_x = if should_apply_offset { info.offset_x as f32 } else { 0.0 };
                        let off_y = if should_apply_offset { info.offset_y as f32 } else { 0.0 };
                        let left = x as f32 * TILE_WIDTH as f32 + off_x;
                        let (anchor_y, top_anchored) = if blend {
                            (-(base_y_world - 3.0 * TILE_HEIGHT as f32 + off_y), true)
                        } else {
                            (-(base_y_world + off_y), false)
                        };
                        if blend {
                            if let Some(e) = spawn_blend_tile(
                                commands, libraries, assets, tile_cache,
                                blend_materials, blend_quad.clone(),
                                TileAnimKind::Front, file_index, base_image_index,
                                animation, tick, left, anchor_y, top_anchored,
                                depth_y(base_y_world),
                            ) {
                                commands.entity(e).insert(FrontChunkKey(cx, cy));
                            }
                        } else if let Some(e) = spawn_anim_tile(
                            commands, libraries, assets, tile_cache,
                            TileAnimKind::Front, file_index, base_image_index,
                            animation, tick, false, left, anchor_y, top_anchored,
                            depth_y(base_y_world),
                        ) {
                            commands.entity(e).insert(FrontChunkKey(cx, cy));
                        }
                    }
                }
                count += 1;
                continue;
            }
            let Some((file_index, image_index)) = cell.front_tile() else {
                continue;
            };
            let key = (file_index, image_index);
            let cached = front_images.0.get(&key).cloned();
            let (handle, w, h) = match cached {
                Some(c) => c,
                None => {
                    let Some(info) = libraries.get_map_image(file_index, image_index) else {
                        continue;
                    };
                    if info.width <= 0 || info.height <= 0 {
                        continue;
                    }
                    let Some(rgba) = info.rgba.clone() else {
                        continue;
                    };
                    let (w, h) = (info.width, info.height);
                    let mut img = make_image(rgba, w.max(0) as u32, h.max(0) as u32);
                    img.sampler = ImageSampler::nearest();
                    let handle = assets.add(img);
                    front_images.0.insert(key, (handle.clone(), w, h));
                    (handle, w, h)
                }
            };
            // 基准 Y = 格子底边 (y+1)*32
            let base_y = ((y + 1) * TILE_HEIGHT as i32) as f32;
            let left = (x as f32) * TILE_WIDTH;
            let top = base_y - h as f32;
            let center_x = left + w as f32 / 2.0;
            let (center_y, z) = if (w == TILE_WIDTH as i16 && h == TILE_HEIGHT as i16)
                || (w == TILE_WIDTH as i16 * 2 && h == TILE_HEIGHT as i16 * 2)
            {
                // C# DrawFloor：1x1/2x2 地面贴花左上角对齐，且 z 在地板之上、角色之下
                (-(y as f32 * TILE_HEIGHT + h as f32 / 2.0), 0.15)
            } else {
                // C# DrawObjects：高物件底边对齐，与角色按 Y 交错
                (-(base_y - h as f32 / 2.0), depth_y(base_y))
            };
            commands.spawn((
                Sprite::from_image(handle),
                Transform::from_xyz(center_x, center_y, z),
                Visibility::default(),
                FrontChunkKey(cx, cy),
                FrontTile {
                    base_y,
                    left,
                    top,
                    right: left + w as f32,
                    bottom: base_y,
                },
            ));
            count += 1;
        }
    }
    count
}


pub(crate) fn chunk_stream_system(
    mut commands: Commands,
    mut stream: ResMut<ChunkStream>,
    game_data: Res<GameData>,
    mut game_libs: ResMut<GameLibraries>,
    mut assets: ResMut<Assets<Image>>,
    mut tile_cache: ResMut<TileImageCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut blend_materials: ResMut<Assets<MapBlendMaterial>>,
    mut front_images: ResMut<FrontImageCache>,
    camera: Query<
        &Transform,
        (
            With<Camera2d>,
            Without<crate::ui::sprite_ui::UiEntity>,
            Without<crate::actor::LocalPlayer>,
        ),
    >,
    chunks: Query<(Entity, &ChunkKey)>,
    front_chunks: Query<(Entity, &FrontChunkKey)>,
) {
    let Some(map_reader) = game_data.map_reader.clone() else { return };
    let Ok(cam) = camera.single() else { return };
    let cam_cx = (cam.translation.x / CHUNK_PIXEL_W as f32) as i32;
    let cam_cy = ((-cam.translation.y) / CHUNK_PIXEL_H as f32) as i32;
    if stream.last_cam_chunk == Some((cam_cx, cam_cy)) {
        return;
    }
    stream.last_cam_chunk = Some((cam_cx, cam_cy));
    let chunks_x = div_ceil_i32(map_reader.width, CHUNK_TILES as i32);
    let chunks_y = div_ceil_i32(map_reader.height, CHUNK_TILES as i32);
    let radius = 2i32;

    let mut wanted = std::collections::HashSet::new();
    for layer in [Layer::Back, Layer::Middle] {
        for cy in (cam_cy - radius)..=(cam_cy + radius) {
            for cx in (cam_cx - radius)..=(cam_cx + radius) {
                if cx >= 0 && cy >= 0 && cx < chunks_x && cy < chunks_y {
                    wanted.insert((cx, cy, layer));
                }
            }
        }
    }
    let existing: std::collections::HashSet<_> =
        chunks.iter().map(|(_, k)| (k.0, k.1, k.2)).collect();

    // 卸载窗口外
    for (e, k) in chunks.iter() {
        if !wanted.contains(&(k.0, k.1, k.2)) {
            commands.entity(e).despawn();
        }
    }
    // 加载窗口内缺失
    let mut added = 0usize;
    for (cx, cy, layer) in &wanted {
        if existing.contains(&(*cx, *cy, *layer)) {
            continue;
        }
        if let Some(handle) = build_chunk(
            &mut game_libs.0,
            &map_reader,
            *layer,
            *cx,
            *cy,
            &mut assets,
        ) {
            let rect_x = (*cx * CHUNK_TILES as i32) as f32 * TILE_WIDTH;
            let rect_y = (*cy * CHUNK_TILES as i32) as f32 * TILE_HEIGHT;
            let px = rect_x + CHUNK_PIXEL_W as f32 / 2.0;
            let py = -(rect_y + CHUNK_PIXEL_H as f32 / 2.0);
            commands.spawn((
                Sprite::from_image(handle),
                Transform::from_xyz(px, py, layer.z()),
                Visibility::default(),
                MapFloorMark(*layer),
                ChunkKey(*cx, *cy, *layer),
            ));
            added += 1;
        }
    }
    if added > 0 {
        tracing::info!("🧩 chunk 流式加载 {} 个", added);
    }

    // Front 层流式：同窗口生成/卸载精灵（#31 性能，避免全图 4 万实体常驻）
    let blend_quad = meshes.add(Rectangle::new(1.0, 1.0));
    let mut wanted_front = std::collections::HashSet::new();
    for cy in (cam_cy - radius)..=(cam_cy + radius) {
        for cx in (cam_cx - radius)..=(cam_cx + radius) {
            if cx >= 0 && cy >= 0 && cx < chunks_x && cy < chunks_y {
                wanted_front.insert((cx, cy));
            }
        }
    }
    let existing_front: std::collections::HashSet<_> =
        front_chunks.iter().map(|(_, k)| (k.0, k.1)).collect();
    for (e, k) in front_chunks.iter() {
        if !wanted_front.contains(&(k.0, k.1)) {
            commands.entity(e).despawn();
        }
    }
    let mut front_added = 0usize;
    for (cx, cy) in &wanted_front {
        if existing_front.contains(&(*cx, *cy)) {
            continue;
        }
        front_added += spawn_front_chunk(
            &mut commands,
            &mut game_libs.0,
            &mut assets,
            &mut tile_cache,
            &mut blend_materials,
            &blend_quad,
            &mut front_images,
            &map_reader,
            *cx,
            *cy,
        );
    }
    if front_added > 0 {
        tracing::info!("🌳 front 流式加载 {} 个", front_added);
    }
}
