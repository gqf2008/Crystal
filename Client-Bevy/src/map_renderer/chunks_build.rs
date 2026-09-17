// ============================================================================
// map_renderer 模块拆分（#72）
// ============================================================================

use super::*;
use bevy::prelude::*;

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
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<crate::ui::sprite_ui::UiEntity>)>,
    mut auth: ResMut<crate::ui::login::AuthFeedback>,
    mut next: ResMut<NextState<crate::scenes::AppState>>,
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
    let map_name = game_data.desired_map.clone().unwrap_or_else(map_arg);
    let map_path = resolve_map_path(&map_name);
    let map = match MapReader::new(&map_path) {
        Ok(m) => m,
        Err(e) => {
            // M4：加载失败必须玩家可见并退回登录，而不是只记日志静默黑屏。
            // 旧实现还在此多 spawn 一个 Camera2d——唯一地图相机已由 Startup 的
            // spawn_camera 创建，再 spawn 会让全库相机 single/single_mut 失败。
            tracing::error!("❌ 地图加载失败 {}: {}", map_path, e);
            auth.login_error = Some(format!("地图 {} 加载失败：{}", map_name, e));
            // set_if_neq：setup_world 仅两个入口——OnEnter(Game) 与游戏内换图重建
            // （map_rebuild_system，in_state(Game) 门控）——执行到本失败分支时当前态
            // 恒为 Game（OnEnter 触发时迁移已完成），Game→Login 必为真实迁移，
            // set 与 set_if_neq 行为相同。统一用 set_if_neq 只是防御：Bevy 0.19
            // 同态 set 非 no-op（会真实重跑 OnExit/OnEnter），未来若新增同态入口
            // 不至于静默重建场景
            (*next).set_if_neq(crate::scenes::AppState::Login);
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
                if let Some(handle) = build_chunk(libraries, &map, layer, cx, cy, &mut assets) {
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
                &mut commands,
                libraries,
                &mut assets,
                &mut tile_cache,
                &mut blend_materials,
                &blend_quad,
                &mut front_images,
                &map,
                cx,
                cy,
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
                color: bevy::prelude::LinearRgba::new(
                    cr * 0.4 / 255.0,
                    cg * 0.4 / 255.0,
                    cb * 0.4 / 255.0,
                    1.0,
                ),
                texture: light_tex.clone(),
            });
            commands.spawn((
                MapLight,
                LightChunkKey(
                    (x / CHUNK_TILES as usize) as i32,
                    (y / CHUNK_TILES as usize) as i32,
                ),
                bevy::prelude::Mesh2d(blend_quad.clone()),
                bevy::prelude::MeshMaterial2d(mat),
                Transform::from_xyz(cx, cy, 0.9).with_scale(Vec3::new(lw, lh, 1.0)),
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
                                // 对象层大图无 chunk 归属（跨块），挂标记供 OnExit/换图统一清理
                                MapMiddleObject,
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
        tracing::warn!(
            "[DIAG] 相机定位失败！Camera2d 数量={}",
            camera.iter().count()
        );
    }

    // 构建可行走网格（M8 寻路）+ 门索引网格（#1550）
    let mut walkable = Vec::with_capacity(map.width as usize);
    let mut doors = Vec::with_capacity(map.width as usize);
    for x in 0..map.width {
        let mut col = Vec::with_capacity(map.height as usize);
        let mut dcol = Vec::with_capacity(map.height as usize);
        for y in 0..map.height {
            let cell = &map.map_cells[x as usize][y as usize];
            col.push(cell.is_walkable());
            dcol.push(cell.door_index);
        }
        walkable.push(col);
        doors.push(dcol);
    }
    // 诊断：可走格统计（#57 排查 0.map 寻路失败）
    {
        let total = map.width as usize * map.height as usize;
        let walkable_count = walkable.iter().flatten().filter(|w| **w).count();
        tracing::info!(
            "🚶 可行走网格: {}/{} 格可走（{:.1}%）",
            walkable_count,
            total,
            walkable_count as f64 * 100.0 / total.max(1) as f64
        );
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
        doors,
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
        Layer::Back => (
            (start_x - 1).max(0),
            (end_x + 1).min(map.width),
            start_y,
            (end_y + 1).min(map.height),
        ),
        Layer::Middle => (
            (start_x - 1).max(0),
            (end_x + 1).min(map.width),
            (start_y - 1).max(0),
            (end_y + 1).min(map.height),
        ),
        Layer::Front => (
            (start_x - 1).max(0),
            (end_x + 8).min(map.width),
            (start_y - 1).max(0),
            (end_y + 16).min(map.height),
        ),
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
pub(crate) fn blit(canvas: &mut [u8], dx: i32, dy: i32, img: &ImageInfo, rgba: &[u8]) -> bool {
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

// ============================================================================
// 场景生命期：地图世界清理 / 换图重建（B1/S2）
// ============================================================================

/// S2/B1 共用：清掉当前地图的全部场景实体并重置 chunk 流式游标。
/// 覆盖：Back/Middle 合成块（ChunkKey）、Front 瓦片（FrontChunkKey，含动画/混合）、
/// 地图灯光（MapLight）、Middle 大图对象（MapMiddleObject）。
/// 块纹理资产随实体释放；Front 贴图由 FrontImageCache 跨图共享持有，保留不删。
/// chunk_stream_system 的 existing 集合来自实体查询——旧实体清掉后
/// 不会被当成「已加载」保留，无需额外同步。
pub(crate) fn clear_map_world(world: &mut World) {
    let mut despawn = Vec::new();
    let mut chunk_images = Vec::new();
    let mut materials = Vec::new();
    {
        // Back/Middle 合成块：纹理逐块独有，随实体释放（对齐 chunk_stream 卸载路径）
        let mut q = world.query_filtered::<(Entity, &Sprite), With<ChunkKey>>();
        for (e, sprite) in q.iter(world) {
            despawn.push(e);
            chunk_images.push(sprite.image.clone());
        }
    }
    {
        // Front 瓦片（静态/动画/混合）：贴图走 FrontImageCache 共享，只回收混合材质
        let mut q = world.query_filtered::<
            (Entity, Option<&MeshMaterial2d<MapBlendMaterial>>),
            With<FrontChunkKey>,
        >();
        for (e, mat) in q.iter(world) {
            despawn.push(e);
            if let Some(mat) = mat {
                materials.push(mat.0.clone());
            }
        }
    }
    {
        // 地图灯光
        let mut q = world
            .query_filtered::<(Entity, Option<&MeshMaterial2d<MapBlendMaterial>>), With<MapLight>>(
            );
        for (e, mat) in q.iter(world) {
            despawn.push(e);
            if let Some(mat) = mat {
                materials.push(mat.0.clone());
            }
        }
    }
    {
        // Middle 大图对象
        let mut q = world.query_filtered::<Entity, With<MapMiddleObject>>();
        for e in q.iter(world) {
            despawn.push(e);
        }
    }
    for e in despawn {
        let _ = world.despawn(e);
    }
    if let Some(mut assets) = world.get_resource_mut::<Assets<Image>>() {
        for h in chunk_images {
            assets.remove(&h);
        }
    }
    if let Some(mut mats) = world.get_resource_mut::<Assets<MapBlendMaterial>>() {
        for h in materials {
            mats.remove(&h);
        }
    }
    if let Some(mut stream) = world.get_resource_mut::<ChunkStream>() {
        // 强制 chunk_stream_system 下一帧按新相机/新图全量重估
        stream.last_cam_chunk = None;
    }
}

/// S2：离开 Game 场景（登出 / 断线回登录）统一清理地图实体。
/// 此前 map_renderer 只有流式卸载，OnExit(Game) 无任何清理——同进程重进游戏时
/// 旧块/灯光/大图全部残留叠加（OnEnter 只负责 spawn，不负责先清上一局）。
pub(crate) fn cleanup_map_world(world: &mut World) {
    clear_map_world(world);
}

/// B1：运行中换图重建。
/// MapChanged 只写 desired_map + next.set_if_neq(Game)；游戏内收到时 set_if_neq
/// 不写 Pending、OnExit/OnEnter(Game) 都不重跑（Bevy 0.19 同态 NextState::set
/// 反而会真实重跑 OnExit+OnEnter，必须用 set_if_neq），而 desired_map 此前
/// 只有 setup_world 一个消费者 → 世界永远停在第一张图。这里按值比较
/// （不靠变更检测 tick，避免同帧顺序坑）：
/// desired_map 与已加载地图名不一致 → 全清旧世界后以 setup_world 原逻辑重建
/// （含 walkable/doors、相机定位、初始窗口、GameData.map/map_reader 替换）。
/// 旧图网络实体（NetObjectId、非本地玩家）一并清掉：同态换图不经过
/// OnExit(Game)，despawn_local_player 不会跑，不清则旧图 NPC/怪物成幽灵。
/// 加载失败走 setup_world 的 M4 分支：错误可见 + 退回登录。
pub(crate) fn map_rebuild_system(world: &mut World) {
    let needs = {
        let gd = world.resource::<GameData>();
        match (&gd.desired_map, &gd.map) {
            (Some(desired), Some(loaded)) => desired != &loaded.name,
            // 首次进图 map 尚为 None，由 OnEnter(Game) 的 setup_world 负责
            _ => false,
        }
    };
    if !needs {
        return;
    }
    let name = world
        .resource::<GameData>()
        .desired_map
        .clone()
        .unwrap_or_default();
    tracing::info!("🗺️ 检测到换图 {}，清理旧世界并重建", name);
    clear_map_world(world);
    // 幽灵清理（复核发现）：同态换图不经过 OnExit(Game)，despawn_local_player
    // 不会跑——旧图的 NPC/怪物/远端玩家/地面物品/金币（NetObjectId、非本地玩家）
    // 必须随旧世界一起清掉，否则在新图同 object_id 实体到达前一直可见成幽灵。
    // 新图对象由服务端换图后全量重发。本地玩家必须显式排除——注意这是唯一
    // 防线而非「双保险」：spawn_local_player_with（actor/spawn_helpers.rs:177-178）
    // 给本地玩家同时挂 LocalPlayer 和 NetObjectId，误删 Without<LocalPlayer>
    // 排除条件换图即删玩家（过滤器写法对齐 actor::despawn_local_player 的清理面）。
    {
        let mut q = world.query_filtered::<
            Entity,
            (
                With<crate::actor::NetObjectId>,
                Without<crate::actor::LocalPlayer>,
            ),
        >();
        let ghosts: Vec<Entity> = q.iter(world).collect();
        for e in ghosts {
            let _ = world.despawn(e);
        }
    }
    // 换图落位（复核 minor）：旧图未走完的点击移动（LocalMove/MoveTween）若带入
    // 新图，会在新图继续走；且 apply_self_position 在 LocalMove 非空时直接丢弃
    // UserLocation 校正（movement.rs:240-282），坐标漂移永远校不回来。对齐 C#
    // MapChanged 直设 CurrentLocation 语义：重建时清移动状态并按 player_spawn 落位。
    {
        let spawn = world.resource::<GameData>().player_spawn;
        let mut q = world.query_filtered::<Entity, With<crate::actor::LocalPlayer>>();
        let players: Vec<Entity> = q.iter(world).collect();
        for e in players {
            let mut em = world.entity_mut(e);
            em.remove::<crate::game::movement::LocalMove>();
            em.remove::<crate::game::movement::MoveTween>();
            if let Some((tx, ty, _dir)) = spawn {
                let p = crate::game::movement::tile_to_world(tx as i32, ty as i32);
                if let Some(mut tf) = em.get_mut::<Transform>() {
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                    tf.translation.z = crate::actor::depth_z(-p.y);
                }
            }
        }
    }
    use bevy::ecs::system::RunSystemOnce;
    if let Err(e) = world.run_system_once(setup_world) {
        tracing::error!("🗺️ 换图重建失败: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::libraries::Libraries;
    use crate::scenes::AppState;
    use crate::ui::login::AuthFeedback;

    /// 造一个挂齐 setup_world 所需资源、且地图必然加载失败的 World：
    /// GameLibraries.initialized = true 跳过 ensure_initialized 的磁盘扫描，
    /// desired_map 指向不存在的地图文件 → 走 M4 失败分支（错误可见 + 回登录）。
    fn world_with_old_map(old_name: &str, desired: Option<&str>) -> World {
        let mut world = World::new();
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Mesh>::default());
        world.insert_resource(Assets::<MapBlendMaterial>::default());
        world.insert_resource(ChunkStream {
            last_cam_chunk: Some((3, 3)),
        });
        let mut libs = Libraries::new("__no_data_dir_for_test__");
        libs.initialized = true;
        world.insert_resource(GameLibraries(libs));
        world.init_resource::<TileImageCache>();
        world.init_resource::<FrontImageCache>();
        world.insert_resource(NextState::<AppState>::default());
        world.init_resource::<AuthFeedback>();
        world.insert_resource(GameData {
            map: Some(LoadedMap {
                name: old_name.to_string(),
                width: 1,
                height: 1,
                doors: vec![vec![0]],
                walkable: vec![vec![true]],
            }),
            map_reader: None,
            desired_map: desired.map(|s| s.to_string()),
            player_spawn: None,
        });
        world
    }

    /// 在世界里摆一套「旧地图」场景实体 + 一个无关实体，返回 5 个 Entity。
    fn spawn_old_world_entities(world: &mut World) -> (Entity, Entity, Entity, Entity, Entity) {
        let chunk = world
            .spawn((
                ChunkKey(0, 0, Layer::Back),
                Sprite::default(),
                MapFloorMark(Layer::Back),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        let front = world
            .spawn((
                FrontChunkKey(0, 0),
                Sprite::default(),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        let light = world
            .spawn((
                MapLight,
                LightChunkKey(0, 0),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        let middle_obj = world
            .spawn((
                MapMiddleObject,
                Sprite::default(),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        let keeper = world.spawn(Transform::default()).id();
        (chunk, front, light, middle_obj, keeper)
    }

    fn assert_map_entities_gone(world: &mut World, entities: [Entity; 4]) {
        for e in entities {
            assert!(world.get_entity(e).is_err(), "地图实体 {:?} 应被清理", e);
        }
        for (count, name) in [
            (
                world
                    .query_filtered::<Entity, With<ChunkKey>>()
                    .iter(world)
                    .count(),
                "ChunkKey",
            ),
            (
                world
                    .query_filtered::<Entity, With<FrontChunkKey>>()
                    .iter(world)
                    .count(),
                "FrontChunkKey",
            ),
            (
                world
                    .query_filtered::<Entity, With<MapLight>>()
                    .iter(world)
                    .count(),
                "MapLight",
            ),
            (
                world
                    .query_filtered::<Entity, With<MapMiddleObject>>()
                    .iter(world)
                    .count(),
                "MapMiddleObject",
            ),
        ] {
            assert_eq!(count, 0, "{} 应无残留", name);
        }
        assert_eq!(
            world.resource::<ChunkStream>().last_cam_chunk,
            None,
            "ChunkStream 游标应重置"
        );
    }

    /// B1 回归：游戏内 desired_map 变更 → 旧地图实体全清 + 流式游标重置 + 触发重建。
    /// （修复前：desired_map 无人消费，旧实体原样保留，本测试红。）
    #[test]
    fn desired_map_change_triggers_world_rebuild() {
        let mut world = world_with_old_map("old_map", Some("__missing_map_for_rebuild_test__"));
        let (chunk, front, light, middle_obj, keeper) = spawn_old_world_entities(&mut world);

        map_rebuild_system(&mut world);

        assert_map_entities_gone(&mut world, [chunk, front, light, middle_obj]);
        assert!(
            world.get_entity(keeper).is_ok(),
            "无关实体不应被地图清理误伤"
        );
        // 重建走到 setup_world 的 M4 失败分支（测试地图不存在）：错误必须玩家可见
        // 且请求退回登录，而不是静默黑屏
        assert!(
            world.resource::<AuthFeedback>().login_error.is_some(),
            "地图加载失败应给出玩家可见错误"
        );
        assert!(
            matches!(
                *world.resource::<NextState<AppState>>(),
                NextState::PendingIfNeq(AppState::Login)
            ),
            "地图加载失败应退回登录（set_if_neq → PendingIfNeq）"
        );
        // 失败分支不得再 spawn 重复相机（唯一地图相机由 Startup spawn_camera 创建）
        assert_eq!(
            world
                .query_filtered::<Entity, With<Camera2d>>()
                .iter(&world)
                .count(),
            0,
            "M4 失败分支不应 spawn 重复 Camera2d"
        );
    }

    /// B1 反向用例：desired_map 与已加载地图一致 → 不动世界。
    #[test]
    fn same_desired_map_does_not_rebuild() {
        let mut world = world_with_old_map("same_map", Some("same_map"));
        let (chunk, _, _, _, _) = spawn_old_world_entities(&mut world);

        map_rebuild_system(&mut world);

        assert!(world.get_entity(chunk).is_ok(), "同图不应触发重建清理");
        assert_eq!(
            world.resource::<ChunkStream>().last_cam_chunk,
            Some((3, 3)),
            "同图不应重置流式游标"
        );
    }

    /// S2 幽灵回归：换图重建必须连旧图的网络实体一起清掉（NPC/怪物/远端玩家/
    /// 地面物品/金币：带 NetObjectId、非本地玩家）。同态换图不经过 OnExit(Game)，
    /// despawn_local_player 不会跑；修复前重建只清地图块/灯光，旧 NetObjectId
    /// 实体在新图同 object_id 实体到达前一直可见成幽灵（本测试修复前红：
    /// NetObjectId 计数 = 1）。注意 Without<LocalPlayer> 是唯一防线而非双保险：
    /// 本地玩家同时带 LocalPlayer 和 NetObjectId（spawn_local_player_with，
    /// actor/spawn_helpers.rs:177-178），故本测试的玩家实体两者都挂——误删
    /// 排除条件本测试即红。
    #[test]
    fn rebuild_despawns_old_map_net_entities() {
        use crate::actor::{LocalPlayer, NetObjectId};

        let mut world = world_with_old_map("old_map", Some("__missing_map_for_rebuild_test__"));
        let ghost_npc = world.spawn(NetObjectId(1001)).id();
        let ghost_remote = world.spawn(NetObjectId(1002)).id();
        // 本地玩家同时挂 LocalPlayer + NetObjectId（真实结构，见 spawn_local_player_with）
        let player = world.spawn((LocalPlayer, NetObjectId(1000))).id();

        map_rebuild_system(&mut world);

        assert_eq!(
            world
                .query_filtered::<Entity, (With<NetObjectId>, Without<LocalPlayer>)>()
                .iter(&world)
                .count(),
            0,
            "换图重建后旧图 NetObjectId 实体应清零（幽灵实体）"
        );
        assert!(
            world.get_entity(ghost_npc).is_err() && world.get_entity(ghost_remote).is_err(),
            "旧图 NPC/远端玩家应被清理"
        );
        assert!(
            world.get_entity(player).is_ok(),
            "本地玩家不应被换图清理误伤"
        );
    }

    /// 2b 回归：换图重建必须清掉本地玩家未走完的 LocalMove/MoveTween 并按
    /// player_spawn 落位（对齐 C# MapChanged 直设 CurrentLocation 语义）。
    /// 修复前：移动组件原样保留、Transform 停留旧图位置——旧路径在新图继续走，
    /// 且 LocalMove 非空会抑制 apply_self_position 的 UserLocation 校正（本测试红）。
    #[test]
    fn rebuild_clears_local_move_and_repositions_player() {
        use crate::actor::LocalPlayer;
        use crate::game::movement::{tile_to_world, LocalMove, MoveTween};
        use mir2_shared::enums::MirAction;

        let mut world = world_with_old_map("old_map", Some("__missing_map_for_rebuild_test__"));
        world.resource_mut::<GameData>().player_spawn = Some((10.0, 20.0, 3));
        let player = world
            .spawn((
                LocalPlayer,
                Transform::from_xyz(1.0, 2.0, 3.0),
                LocalMove {
                    path: std::collections::VecDeque::from([(11, 21), (12, 22)]),
                    step_timer_ms: 42.0,
                    run: true,
                    last: Some((10, 20)),
                    step_origin: Some((9, 19)),
                    turn_acc: 1.0,
                },
                MoveTween {
                    from: Vec2::ZERO,
                    to: Vec2::ONE,
                    t: 0.5,
                    dur: 0.1,
                    action: MirAction::Walking,
                    dir: 2,
                },
            ))
            .id();

        map_rebuild_system(&mut world);

        let e = world.entity(player);
        assert!(
            e.get::<LocalMove>().is_none(),
            "换图后 LocalMove 必须清除（旧路径不得带入新图）"
        );
        assert!(
            e.get::<MoveTween>().is_none(),
            "换图后 MoveTween 必须清除"
        );
        let p = tile_to_world(10, 20);
        let tf = e.get::<Transform>().expect("本地玩家应有 Transform");
        assert_eq!(
            tf.translation,
            Vec3::new(p.x, p.y, crate::actor::depth_z(-p.y)),
            "换图后应按 player_spawn 落位（C# MapChanged 直设 CurrentLocation）"
        );
    }

    /// 复核严重项回归：换图帧（desired_map 变更）与新图 NetObject 消息同帧到达时，
    /// 网络对象生成链（ActorPlugin）必须排在 map_rebuild_system 之后——否则刚生成的
    /// 新图 NPC 会被重建的幽灵清理整批 despawn，服务端不会重发 → 新图 NPC 永久缺失。
    /// 走真实插件（MapRenderPlugin + ActorPlugin）+ 完整 Update 调度；撤掉
    /// actor/mod.rs 的 .after(map_rebuild_system) 排序锁后本测试红。
    #[test]
    fn same_frame_net_object_survives_map_rebuild() {
        use crate::actor::{ActorPlugin, NetObjectId, Npc};
        use crate::map_renderer::MapRenderPlugin;
        use crate::network::{NetObject, NetObjectRemoved, SessionState};

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::state::app::StatesPlugin,
        ));
        app.init_state::<AppState>();
        app.add_plugins((MapRenderPlugin, ActorPlugin));
        // spawn_net_objects_when_ready / setup_world 参数资源（无窗口/渲染最小集）
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<Font>::default());
        app.insert_resource(Assets::<bevy::audio::AudioSource>::default());
        let mut libs = Libraries::new("__no_data_dir_for_test__");
        libs.initialized = true; // 跳过 ensure_initialized 磁盘扫描
        app.insert_resource(GameLibraries(libs));
        app.init_resource::<crate::ui::sprite_ui::UiImageCache>();
        app.init_resource::<crate::ui::sprite_ui::UiFont>();
        app.init_resource::<crate::game::sound::SoundBank>();
        app.init_resource::<SessionState>();
        app.init_resource::<AuthFeedback>();
        app.add_message::<NetObject>();
        app.add_message::<NetObjectRemoved>();
        // 其余在役 Update 系统的参数资源（bevy 0.19 缺资源参数直接 panic）
        app.add_message::<crate::network::server_event::ServerEvent>();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.init_resource::<crate::game::input_gate::TextInputGate>();
        app.init_resource::<crate::control::CursorProbe>();
        app.init_resource::<crate::ui::tooltip::TooltipState>();
        // chunk_stream_system 需要（正常由 setup_world 成功路径创建；测试地图加载必失败）
        app.insert_resource(crate::map_renderer::MapLightTexture(Handle::default()));
        // 首次进图必然加载失败（地图文件不存在）→ M4 失败分支，避免真实 Data 依赖
        app.insert_resource(GameData {
            desired_map: Some("__missing_first_map__".to_string()),
            ..GameData::default()
        });

        // 直接置 Game 态（不走真实 OnEnter 迁移：setup_world 的 M4 失败分支会
        // 请求回登录，bevy_state 0.19 同帧链式应用迁移，Update 时已不在 Game）。
        // 本测试只需要 in_state(Game) 门控为真 + map_rebuild_system 真实跑起来。
        app.world_mut().insert_resource(State::new(AppState::Game));

        // 同帧：换图（desired_map 与已加载地图不一致）+ 一条新图 NPC 消息
        {
            let mut gd = app.world_mut().resource_mut::<GameData>();
            gd.map = Some(LoadedMap {
                name: "old_map".to_string(),
                width: 1,
                height: 1,
                doors: vec![vec![0]],
                walkable: vec![vec![true]],
            });
            gd.desired_map = Some("__missing_new_map__".to_string());
        }
        app.world_mut().write_message(NetObject::Npc {
            object_id: 9001,
            name: "同帧NPC".to_string(),
            image: 0,
            location_x: 0,
            location_y: 0,
            direction: 0,
        });
        app.update();

        let mut q = app.world_mut().query_filtered::<&NetObjectId, With<Npc>>();
        let ids: Vec<u32> = q.iter(app.world()).map(|id| id.0).collect();
        assert_eq!(
            ids,
            vec![9001],
            "同帧换图 + NetObject::Npc：NPC 必须存活（生成链须排在换图重建之后）"
        );
    }

    /// S2 回归：OnExit(Game) 清理后四类地图实体无残留、游标重置、无关实体保留。
    /// 走真实状态迁移（Game → Login），不是直接调清理函数。
    #[test]
    fn on_exit_game_cleans_all_map_entities() {
        let mut app = App::new();
        // App::new 只有 MainSchedulePlugin，状态迁移需要显式 StatesPlugin
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<AppState>();
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<MapBlendMaterial>::default());
        app.insert_resource(ChunkStream {
            last_cam_chunk: Some((7, 7)),
        });
        app.add_systems(OnExit(AppState::Game), cleanup_map_world);

        // 进入 Game（Intro → Game）
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Game);
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::Game
        );

        let (chunk, front, light, middle_obj, keeper) = spawn_old_world_entities(app.world_mut());

        // 退出 Game（Game → Login）→ OnExit 清理
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Login);
        app.update();

        assert_map_entities_gone(app.world_mut(), [chunk, front, light, middle_obj]);
        assert!(
            app.world_mut().get_entity(keeper).is_ok(),
            "无关实体不应被地图清理误伤"
        );
    }
}
