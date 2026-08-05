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
                    // C# DrawObjects / macroquad：blend 瓦片分两类
                    //  - fileIndex 14/27/100..199：3 格上 + 顶对齐 + 偏移
                    //  - 其他 blend（含路灯 image 2723..=2732）：底边对齐 + 偏移
                    // Bevy 原实现对所有 blend 统一 3 格上且漏了 2723..=2732 → 路灯位置错位（#88）
                    let is_3cell_anchor = file_index == 14
                        || file_index == 27
                        || (100..199).contains(&file_index);
                    let should_apply_offset = if blend {
                        is_3cell_anchor || (2723..=2732).contains(&base_image_index)
                    } else {
                        file_index == 28
                    };
                    if let Some(info) = libraries.get_map_image(file_index, base_image_index) {
                        let off_x = if should_apply_offset { info.offset_x as f32 } else { 0.0 };
                        let off_y = if should_apply_offset { info.offset_y as f32 } else { 0.0 };
                        let left = x as f32 * TILE_WIDTH as f32 + off_x;
                        let (anchor_y, top_anchored) = if blend {
                            if is_3cell_anchor {
                                (-(base_y_world - 3.0 * TILE_HEIGHT as f32 + off_y), true)
                            } else {
                                (-(base_y_world + off_y), false)
                            }
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


/// #88：生成一个 chunk 的地图灯光（C# DrawLights Map Lights 公式/颜色）
pub(crate) fn spawn_light_chunk(
    commands: &mut Commands,
    libraries: &mut Libraries,
    blend_materials: &mut Assets<MapBlendMaterial>,
    blend_quad: &Handle<Mesh>,
    light_tex: &Handle<Image>,
    map: &MapReader,
    cx: i32,
    cy: i32,
) -> usize {
    let mut count = 0usize;
    let x0 = (cx * CHUNK_TILES as i32).max(0) as usize;
    let y0 = (cy * CHUNK_TILES as i32).max(0) as usize;
    let x1 = ((cx + 1) * CHUNK_TILES as i32).min(map.width) as usize;
    let y1 = ((cy + 1) * CHUNK_TILES as i32).min(map.height) as usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let cell = &map.map_cells[x][y];
            let l = cell.light;
            if l == 0 || l >= 10 {
                continue;
            }
            let li = ((l as usize % 10) * 3).min(9);
            let (lw, lh) = LIGHT_SIZES[li];
            // C#：front 动画格叠加库偏移
            let mut off_x = 0.0f32;
            let mut off_y = 0.0f32;
            if cell.front_animation_frame > 0 {
                if let Some((file_index, image_index)) = cell.front_tile() {
                    if let Some(info) = libraries.get_map_image(file_index, image_index) {
                        off_x = info.offset_x as f32;
                        off_y = info.offset_y as f32;
                    }
                }
            }
            // C# DrawLights 中心 = (格左+off_x-14+OffSetX, 格底+off_y-21)（屏幕 y 向下）→ Bevy 世界 y 取负
            // OffSetX=10：C# DrawLights p.X 比 DrawObjects drawX 多 OffSetX，光斑需右移对齐路灯（#88）
            let cx_w = x as f32 * TILE_WIDTH + off_x - 14.0 + LIGHT_SCREEN_OFFSET_X;
            let cy_w = -((y + 1) as f32 * TILE_HEIGHT + off_y - 21.0);
            // C# 灯光颜色按 Light/10：1=白 2=蓝 3=橙 4=绿，默认白；强度 0.4 避免过曝
            let (cr, cg, cb) = match l / 10 {
                2 => (120.0, 180.0, 255.0),
                3 => (255.0, 180.0, 120.0),
                4 => (22.0, 160.0, 5.0),
                _ => (255.0, 255.0, 255.0),
            };
            let mat = blend_materials.add(crate::map_tile_anim::MapBlendMaterial {
                color: bevy::prelude::LinearRgba::new(cr * 0.4 / 255.0, cg * 0.4 / 255.0, cb * 0.4 / 255.0, 1.0),
                texture: light_tex.clone(),
            });
            commands.spawn((
                MapLight,
                LightChunkKey(cx, cy),
                bevy::prelude::Mesh2d(blend_quad.clone()),
                bevy::prelude::MeshMaterial2d(mat),
                Transform::from_xyz(cx_w, cy_w, 0.9)
                    .with_scale(Vec3::new(lw, lh, 1.0)),
                Visibility::default(),
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
    chunks: Query<(Entity, &ChunkKey, &Sprite)>,
    front_chunks: Query<(Entity, &FrontChunkKey, Option<&MeshMaterial2d<crate::map_tile_anim::MapBlendMaterial>>)>,
    light_tex: Res<MapLightTexture>,
    lights: Query<(
        Entity,
        &LightChunkKey,
        Option<&MeshMaterial2d<crate::map_tile_anim::MapBlendMaterial>>,
    )>,
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
        chunks.iter().map(|(_, k, _)| (k.0, k.1, k.2)).collect();

    // 卸载窗口外（#113 内存：同时释放 chunk 纹理资产——此前只 despawn 实体，
    // 纹理句柄永久留在 Assets，玩家移动时内存无限增长）
    for (e, k, sprite) in chunks.iter() {
        if !wanted.contains(&(k.0, k.1, k.2)) {
            assets.remove(&sprite.image);
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
        front_chunks.iter().map(|(_, k, _)| (k.0, k.1)).collect();
    for (e, k, mat) in front_chunks.iter() {
        if !wanted_front.contains(&(k.0, k.1)) {
            // #113 内存：混合瓦片材质随 chunk 卸载释放（共享贴图仍在 FrontImageCache）
            if let Some(mat) = mat {
                blend_materials.remove(&mat.0);
            }
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

    // #88：灯光流式——随相机窗口生成/卸载（与 front 一致；此前只在 setup 生成一次，
    // 玩家出生点远离地图中心时灯光全在地图中心，位置不对）
    let mut wanted_light = std::collections::HashSet::new();
    for cy in (cam_cy - 3)..=(cam_cy + 3) {
        for cx in (cam_cx - 3)..=(cam_cx + 3) {
            if cx >= 0 && cy >= 0 && cx < chunks_x && cy < chunks_y {
                wanted_light.insert((cx, cy));
            }
        }
    }
    let existing_light: std::collections::HashSet<_> =
        lights.iter().map(|(_, k, _)| (k.0, k.1)).collect();
    for (e, k, mat) in lights.iter() {
        if !wanted_light.contains(&(k.0, k.1)) {
            // #113 内存：灯光材质随 chunk 卸载释放
            if let Some(mat) = mat {
                blend_materials.remove(&mat.0);
            }
            commands.entity(e).despawn();
        }
    }
    let mut light_added = 0usize;
    for (cx, cy) in &wanted_light {
        if existing_light.contains(&(*cx, *cy)) {
            continue;
        }
        light_added += spawn_light_chunk(
            &mut commands,
            &mut game_libs.0,
            &mut blend_materials,
            &blend_quad,
            &light_tex.0,
            &map_reader,
            *cx,
            *cy,
        );
    }
    if light_added > 0 {
        tracing::info!("💡 灯光流式加载 {} 个", light_added);
    }
}
