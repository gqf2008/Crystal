// ============================================================================
// map_renderer 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use super::*;

pub(crate) fn setup_world(
    mut commands: Commands,
    mut assets: ResMut<Assets<Image>>,
    mut game_data: ResMut<GameData>,
    mut game_libs: ResMut<GameLibraries>,
    mut tile_cache: ResMut<TileImageCache>,
    mut front_images: ResMut<FrontImageCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut blend_materials: ResMut<Assets<MapBlendMaterial>>,
    // 只取地图相机（排除 UI 相机：UiEntity + Camera2d；否则两个相机 single_mut 失败 → 相机停在 (0,0) 显示左上角）
    mut camera: Query<
        &mut Transform,
        (With<Camera2d>, Without<crate::ui::sprite_ui::UiEntity>),
    >,
) {
    // 1. 加载图像库（MapLibs）
    game_libs.0.ensure_initialized();
    let libraries = &mut game_libs.0;
    tracing::info!(
        "📚 库状态: 单体 {} 个, MapLibs {} 个",
        libraries.stats().0,
        libraries.stats().1
    );

    // 2. 加载地图（网络 MapChanged 优先，其次命令行 --map）
    let map_name = game_data
        .desired_map
        .clone()
        .unwrap_or_else(map_arg);
    let map_path = resolve_map_path(&map_name);
    let map = match MapReader::new(&map_path) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("❌ 地图加载失败 {}: {}", map_path, e);
            commands.spawn(Camera2d);
            return;
        }
    };
    tracing::info!(
        "🗺️ 地图 {} 加载成功: {}x{}",
        map_path,
        map.width,
        map.height
    );

    // 灯光混合瓦片共享单位 quad（缩放 = 瓦片尺寸）
    let blend_quad = meshes.add(Rectangle::new(1.0, 1.0));

    // 3. 按块生成纹理（流式：只烘焙相机附近初始窗口，其余由 chunk_stream_system 按需加载）
    let mut spawned = 0usize;
    let chunks_x = div_ceil_i32(map.width, CHUNK_TILES as i32);
    let chunks_y = div_ceil_i32(map.height, CHUNK_TILES as i32);

    let cam_cx = (map.width as f32 * TILE_WIDTH / 2.0 / CHUNK_PIXEL_W as f32) as i32;
    let cam_cy = (map.height as f32 * TILE_HEIGHT / 2.0 / CHUNK_PIXEL_H as f32) as i32;
    let radius = 2i32;
    for layer in [Layer::Back, Layer::Middle] {
        for cy in (cam_cy - radius)..=(cam_cy + radius) {
            for cx in (cam_cx - radius)..=(cam_cx + radius) {
                if cx < 0 || cy < 0 || cx >= chunks_x || cy >= chunks_y {
                    continue;
                }
                if let Some(handle) =
                    build_chunk(libraries, &map, layer, cx, cy, &mut assets)
                {
                    let rect_x = (cx * CHUNK_TILES as i32) as f32 * TILE_WIDTH;
                    let rect_y = (cy * CHUNK_TILES as i32) as f32 * TILE_HEIGHT;
                    let px = rect_x + CHUNK_PIXEL_W as f32 / 2.0;
                    let py = -(rect_y + CHUNK_PIXEL_H as f32 / 2.0);
                    commands.spawn((
                        Sprite::from_image(handle),
                        Transform::from_xyz(px, py, layer.z()),
                        Visibility::default(),
                        MapFloorMark(layer),
                        ChunkKey(cx, cy, layer),
                    ));
                    spawned += 1;
                }
            }
        }
    }
    tracing::info!("🧩 地图块初始窗口生成: {} 个 Sprite", spawned);

    // 3.5 Front 层：按 chunk 窗口流式生成（#31 性能：不再全图 4 万精灵）
    // 逐瓦片精灵，z 按基准 Y（格子底边）与角色交错排序。
    let mut front_spawned = 0usize;
    for cy in (cam_cy - radius)..=(cam_cy + radius) {
        for cx in (cam_cx - radius)..=(cam_cx + radius) {
            if cx < 0 || cy < 0 || cx >= chunks_x || cy >= chunks_y {
                continue;
            }
            front_spawned += spawn_front_chunk(
                &mut commands, libraries, &mut assets, &mut tile_cache,
                &mut blend_materials, &blend_quad, &mut front_images, &map, cx, cy,
            );
        }
    }
    tracing::info!("🌳 Front 瓦片精灵初始窗口生成: {} 个", front_spawned);

    // 3.6 地图灯光（C# DrawLights Map Lights）：cell.Light 1..9 全量生成，
    // 白色径向渐变 + ADD 混合，z=0.9（场景之上、UI 之下，F 键可开关）
    let light_tex = make_light_texture(&mut assets, 128);
    // #88：灯光纹理共享给 chunk 流式（灯光随相机加载/卸载）
    commands.insert_resource(MapLightTexture(light_tex.clone()));
    let mut light_spawned = 0usize;
    // 灯光只生成相机附近窗口（对齐 C# 只画视口 ±24 格）。
    // 全图生成 + 夜晚全部 Visible 会导致数千个巨大光斑同时渲染 → 卡死/过曝。
    let lr = radius + 1;
    let lx0 = ((cam_cx - lr).max(0) * CHUNK_TILES as i32).min(map.width) as usize;
    let lx1 = ((cam_cx + lr + 1) * CHUNK_TILES as i32).min(map.width) as usize;
    let ly0 = ((cam_cy - lr).max(0) * CHUNK_TILES as i32).min(map.height) as usize;
    let ly1 = ((cam_cy + lr + 1) * CHUNK_TILES as i32).min(map.height) as usize;
    for y in ly0..ly1 {
        for x in lx0..lx1 {
            let cell = &map.map_cells[x][y];
            let l = cell.light;
            if l == 0 || l >= 10 {
                continue;
            }
            let li = ((l as usize % 10) * 3).min(9);
            let (lw, lh) = LIGHT_SIZES[li];
            // C#：若该格有 front 动画，叠加库偏移
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
            let cell_left = x as f32 * TILE_WIDTH as f32;
            let cell_bottom_world = -((y + 1) as f32 * TILE_HEIGHT as f32);
            // C# GameScene.DrawLights（Map Lights）：
            //   p = 格左缘 x*CellW、格底缘 (y+1)*CellH（+32）
            //   front 动画格再叠加库偏移 (off_x, off_y)
            //   p.Offset(-LightW/2 - 24 + 10, -LightH/2 - 16 - 5)
            //   => 纹理左上角 = (格左+off_x - W/2 - 14, 格底+off_y - H/2 - 21)
            //   => 中心 = (格左+off_x - 14, 格底+off_y - 21)（屏幕 y 向下）
            // Bevy 世界 y 取负：世界中心 = (cell_left+off_x-14+OffSetX, -(格底+off_y-21))
            // OffSetX=10：C# DrawLights p.X 比 DrawObjects drawX 多 OffSetX，光斑需右移对齐路灯（#88）
            let cx = cell_left + off_x - 14.0 + LIGHT_SCREEN_OFFSET_X;
            let cy = cell_bottom_world - off_y + 21.0;
            // C# 灯光颜色按 Light/10：1=白 2=蓝 3=橙 4=绿，默认白
            let (cr, cg, cb) = match l / 10 {
                2 => (120.0, 180.0, 255.0),
                3 => (255.0, 180.0, 120.0),
                4 => (22.0, 160.0, 5.0),
                _ => (255.0, 255.0, 255.0),
            };
            // C# 灯光乘在 darkness 压暗后的背景上（柔和）；Bevy 直接 ADD 全强度会过曝。
            // 强度取 0.4：夜晚温和提亮、白天隐藏（day_night_system 按 darkness 控制 alpha）
            let mat = blend_materials.add(crate::map_tile_anim::MapBlendMaterial {
                color: bevy::prelude::LinearRgba::new(cr * 0.4 / 255.0, cg * 0.4 / 255.0, cb * 0.4 / 255.0, 1.0),
                texture: light_tex.clone(),
            });
            commands.spawn((
                MapLight,
                LightChunkKey((x / CHUNK_TILES as usize) as i32, (y / CHUNK_TILES as usize) as i32),
                bevy::prelude::Mesh2d(blend_quad.clone()),
                bevy::prelude::MeshMaterial2d(mat),
                Transform::from_xyz(cx, cy, 0.9)
                    .with_scale(Vec3::new(lw, lh, 1.0)),
                Visibility::default(),
            ));
            light_spawned += 1;
        }
    }
    tracing::info!("💡 地图灯光生成完成: {} 个", light_spawned);

    // 3.7 C# DrawObjects：非 1x1/2x2 的静态 Middle（大树/建筑等）底边对齐单独画
    let mut obj_spawned = 0usize;
    for y in 0..map.height as usize {
        for x in 0..map.width as usize {
            let cell = &map.map_cells[x][y];
            if let Some((file_index, image_index)) = cell.middle_tile() {
                if let Some(info) = libraries.get_map_image(file_index, image_index) {
                    let (w, h) = (info.width.max(0) as u32, info.height.max(0) as u32);
                    if w > 0
                        && h > 0
                        && !((w == TILE_WIDTH as u32 && h == TILE_HEIGHT as u32)
                            || (w == TILE_WIDTH as u32 * 2 && h == TILE_HEIGHT as u32 * 2))
                    {
                        let left = x as f32 * TILE_WIDTH as f32;
                        let bottom = -((y + 1) as f32 * TILE_HEIGHT as f32);
                        let center_x = left + w as f32 / 2.0;
                        let center_y = bottom + h as f32 / 2.0;
                        if let Some(rgba) = info.rgba.clone() {
                            let mut img = make_image(rgba, w, h);
                            img.sampler = ImageSampler::nearest();
                            let handle = assets.add(img);
                            commands.spawn((
                                Sprite::from_image(handle),
                                Transform::from_xyz(
                                    center_x,
                                    center_y,
                                    depth_y((y + 1) as f32 * TILE_HEIGHT as f32),
                                ),
                                Visibility::default(),
                            ));
                            obj_spawned += 1;
                        }
                    }
                }
            }
        }
    }
    tracing::info!("🏛️ 对象层 Middle 大图生成完成: {} 个", obj_spawned);

    // 4. 相机定位（优先玩家出生点，否则地图中心）
    let center_x = map.width as f32 * TILE_WIDTH / 2.0;
    let center_y = -(map.height as f32 * TILE_HEIGHT / 2.0);
    // 相机固定放地图中心（用户要求：中心才能看到建筑；玩家在中心附近）
    let (cam_x, cam_y) = (center_x, center_y);

    if let Ok(mut cam_tf) = camera.single_mut() {
        cam_tf.translation = Vec3::new(cam_x, cam_y, 10.0);
        tracing::info!("[DIAG] 相机定位: ({:.0},{:.0})", cam_x, cam_y);
    } else {
        tracing::warn!("[DIAG] 相机定位失败！Camera2d 数量={}", camera.iter().count());
    }

    // 构建可行走网格（M8 寻路）
    let mut walkable = Vec::with_capacity(map.width as usize);
    for x in 0..map.width {
        let mut col = Vec::with_capacity(map.height as usize);
        for y in 0..map.height {
            col.push(map.map_cells[x as usize][y as usize].is_walkable());
        }
        walkable.push(col);
    }
    // 诊断：可走格统计（#57 排查 0.map 寻路失败）
    {
        let total = map.width as usize * map.height as usize;
        let walkable_count = walkable.iter().flatten().filter(|w| **w).count();
        tracing::info!("🚶 可行走网格: {}/{} 格可走（{:.1}%）", walkable_count, total, walkable_count as f64 * 100.0 / total.max(1) as f64);
    }

pub struct GameData {
    pub map: Option<LoadedMap>,
    /// 地图解析器（供 chunk 流式按需加载）
    pub map_reader: Option<std::sync::Arc<MapReader>>,
    /// 网络 MapChanged 指定的地图名（优先于命令行 --map）
    pub desired_map: Option<String>,
    /// 玩家出生位置（瓦片坐标 + 朝向），来自 MapChanged
    pub player_spawn: Option<(f32, f32, u8)>,
}
    game_data.map = Some(LoadedMap {
        name: map_name.clone(),
        width: map.width,
        height: map.height,
        walkable,
    });
    game_data.map_reader = Some(std::sync::Arc::new(map));

}

/// Startup：创建唯一的 2D 相机（登录界面需要相机渲染 egui；进入游戏后重定位）

pub fn build_chunk_rgba(
    libraries: &mut Libraries,
    map: &MapReader,
    layer: Layer,
    cx: i32,
    cy: i32,
) -> Option<Vec<u8>> {
    let mut canvas = vec![0u8; (CHUNK_PIXEL_W * CHUNK_PIXEL_H * 4) as usize];
    let mut any_drawn = false;

    let start_x = cx * CHUNK_TILES as i32;
    let start_y = cy * CHUNK_TILES as i32;
    let end_x = (start_x + CHUNK_TILES as i32).min(map.width);
    let end_y = (start_y + CHUNK_TILES as i32).min(map.height);

    // 关键：瓦片/物件的图像尺寸可能超出单格，跨块边界会被 canvas 裁剪，
    // 造成块边界出现 32px 透明缝隙（macroquad 是视图空间整幅绘制，无此问题）。
    // 这里按层多迭代边界外若干行/列，由 blit 自行裁剪，保证跨界瓦片完整衔接。
    // - Back(96x64)：底部/右侧伸出 1 格，顶部/左侧由相邻块补齐
    // - Middle：双向各留 1 格（覆盖稍高的中景瓦片）
    // - Front：高物件从格子底部向上延伸，向上留 16 行、向右留 8 列
    let (x_lo, x_hi, y_lo, y_hi) = match layer {
        Layer::Back => ((start_x - 1).max(0), (end_x + 1).min(map.width), start_y, (end_y + 1).min(map.height)),
        Layer::Middle => ((start_x - 1).max(0), (end_x + 1).min(map.width), (start_y - 1).max(0), (end_y + 1).min(map.height)),
        Layer::Front => ((start_x - 1).max(0), (end_x + 8).min(map.width), (start_y - 1).max(0), (end_y + 16).min(map.height)),
    };

    for x in x_lo..x_hi {
        for y in y_lo..y_hi {
            // Back 层是 2x2 格子共享的（macroquad render_back_layer 只遍历偶数坐标）。
            // 奇数格可能存着与偶数格不一致的图，画出来会造成与参考实现不同的叠放。
            if layer == Layer::Back && (x % 2 != 0 || y % 2 != 0) {
                continue;
            }
            let cell = &map.map_cells[x as usize][y as usize];
            let Some((file_index, image_index)) = layer.tile(cell) else {
                continue;
            };
            let Some(info) = libraries.get_map_image(file_index, image_index) else {
                continue;
            };
            // C# DrawFloor：地板层 Middle 只画 1x1/2x2，其余走对象层
            if layer == Layer::Middle {
                let (w, h) = (info.width, info.height);
                if !((w == TILE_WIDTH as i16 && h == TILE_HEIGHT as i16)
                    || (w == TILE_WIDTH as i16 * 2 && h == TILE_HEIGHT as i16 * 2))
                {
                    continue;
                }
            }
            let Some(rgba) = info.rgba.as_ref() else {
                continue;
            };
            // C# DrawFloor：地板层左上角对齐格子左上角
            let dx = (x - start_x) * TILE_WIDTH as i32;
            let dy = (y - start_y) * TILE_HEIGHT as i32;
            if blit(&mut canvas, dx, dy, &info, rgba) {
                any_drawn = true;
            }
        }
    }

    if !any_drawn {
        return None;
    }

    Some(canvas)
}

/// 把指定块的三层之一合成一张纹理。块内无任何瓦片时返回 None。
pub(crate) fn build_chunk(
    libraries: &mut Libraries,
    map: &MapReader,
    layer: Layer,
    cx: i32,
    cy: i32,
    assets: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let canvas = build_chunk_rgba(libraries, map, layer, cx, cy)?;
    let mut image = make_image(canvas, CHUNK_PIXEL_W, CHUNK_PIXEL_H);
    // 地图瓦片用最近邻过滤，避免缩放时发虚（与 macroquad MapLibs 的 Nearest 一致）
    image.sampler = ImageSampler::nearest();
    Some(assets.add(image))
}

/// 把图像 RGBA 拷贝到画布，返回是否有像素被写入
pub(crate) fn blit(
    canvas: &mut [u8],
    dx: i32,
    dy: i32,
    img: &ImageInfo,
    rgba: &[u8],
) -> bool {
    let w = img.width as i32;
    let h = img.height as i32;
    if w <= 0 || h <= 0 {
        return false;
    }
    let mut drawn = false;
    for yy in 0..h {
        let sy = dy + yy;
        if sy < 0 || sy >= CHUNK_PIXEL_H as i32 {
            continue;
        }
        for xx in 0..w {
            let sx = dx + xx;
            if sx < 0 || sx >= CHUNK_PIXEL_W as i32 {
                continue;
            }
            let src = ((yy * w + xx) * 4) as usize;
            if rgba[src + 3] == 0 {
                continue;
            }
            let dst = ((sy * CHUNK_PIXEL_W as i32 + sx) * 4) as usize;
            canvas[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
            drawn = true;
        }
    }
    drawn
}

/// 用原始 RGBA 数据构造 Bevy Image 资产
pub fn make_image(rgba: Vec<u8>, width: u32, height: u32) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
