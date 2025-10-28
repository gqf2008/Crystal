// ============================================================================
// Player Rendering Module - 角色渲染模块
// ============================================================================

use super::RenderSystem;
use crate::ecs::components::{Player, Position, Camera, PlayerAppearance, LocalPlayer, MovementAnimation, PlayerAction};
use ggez::{Context, GameResult, graphics::{self, Canvas, DrawParam, Color}};
use hecs::World;

impl RenderSystem {
    /// 绘制角色（简化版本，不包含Front层遮挡检测）
    /// 
    /// 🎬 使用动画帧插值（MovementAnimation）计算绘制位置
    /// 参考: Client/MirObjects/PlayerObject.cs Line 1000-1050
    pub fn draw_player(
        ctx: &mut Context,
        canvas: &mut Canvas,
        player: &Player,
        player_pos: &Position,
        camera_pos: &Position,
        camera: &Camera,
        movement_anim: Option<&MovementAnimation>,  // 🆕 可选的动画插值组件
    ) -> GameResult<()> {
        use crate::graphics::libraries::{get_library, LibraryName};
        use crate::ecs::systems::CameraSystem;
        
        // 🎨 使用 CArmours(0) 库绘制角色（默认装备）
        // CArmours 库帧布局（参考 player_object.rs）:
        //   - Standing: 0-31   (8方向 * 4帧)
        //   - Walking:  32-79  (8方向 * 6帧)
        //   - Running:  80-127 (8方向 * 6帧)
        //   - Attack1:  128-175 (8方向 * 6帧)
        //
        // 公式: DrawFrame = action_frame_start + direction * frames_per_direction + frame_index
        //       FinalFrame = DrawFrame + ArmourOffSet (Male=0, Female=808)
        
        // 计算 DrawFrame
        let action_frame_start = player.action.frame_start();
        let frames_per_direction = player.action.frame_count();
        let direction_offset = (player.direction as i32) * frames_per_direction;
        let draw_frame = action_frame_start + direction_offset + player.frame_index;
        
        // 暂不考虑性别偏移（默认男性，偏移=0）
        let armour_offset = 0;
        let final_frame = draw_frame + armour_offset;
        
        // 🐛 DEBUG: 首次绘制打印帧信息
        static mut FIRST_DRAW: bool = true;
        unsafe {
            if FIRST_DRAW {
                let dir_name = match player.direction {
                    0 => "Up(上)",
                    1 => "UpRight(右上)",
                    2 => "Right(右)",
                    3 => "DownRight(右下)",
                    4 => "Down(下)",
                    5 => "DownLeft(左下)",
                    6 => "Left(左)",
                    7 => "UpLeft(左上)",
                    _ => "Unknown",
                };
                
                println!("\n🎨 === 角色帧计算调试 ===");
                println!("动作: {:?}", player.action);
                println!("方向: {} - {}", player.direction, dir_name);
                println!("当前帧索引: {}/{}", player.frame_index, frames_per_direction);
                println!("动作起始帧: {}", action_frame_start);
                println!("方向偏移: {} (方向{} * 每方向{}帧)", direction_offset, player.direction, frames_per_direction);
                println!("DrawFrame: {} + {} + {} = {}", action_frame_start, direction_offset, player.frame_index, draw_frame);
                println!("性别偏移: {}", armour_offset);
                println!("FinalFrame: {} + {} = {}", draw_frame, armour_offset, final_frame);
                println!("使用库: CArmours(0)");
                println!("========================\n");
                FIRST_DRAW = false;
            }
        }
        
        // ✅ 获取角色纹理 - 使用正确的角色库（不是地图库！）
        if let Some(mlib) = get_library(LibraryName::CArmours(0)) {
            if let Ok(mut mlib) = mlib.lock() {
                // 获取尺寸和偏移量
                let (char_w, char_h) = mlib
                    .get_size(final_frame as usize)
                    .unwrap_or((48, 64));
                
                let (_offset_x, _offset_y) = mlib
                    .get_offset(final_frame as usize)
                    .unwrap_or((0, 0));
                
                // 获取纹理
                match mlib.get_or_create_texture(ctx, final_frame as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                            // � 动画帧插值位置计算 (原版C#机制)
                            // 参考: PlayerObject.cs Line 1000-1050
                            //
                            // 如果有MovementAnimation组件，使用插值计算绘制位置:
                            //   DrawLocation = Movement * CellSize - OffSetMove
                            //
                            // 如果没有，使用Position（兼容旧代码）
                            
                            let (draw_world_x, draw_world_y) = if let Some(anim) = movement_anim {
                                // 🎯 使用动画帧插值计算位置
                                use crate::ecs::Coordinates;
                                
                                // Movement位置（目标格子中心）- 使用grid_to_world_center转换
                                let (movement_world_x, movement_world_y) = Coordinates::grid_to_world_center(
                                    anim.movement_grid.0,
                                    anim.movement_grid.1
                                );
                                
                                // 应用offset_move插值
                                let draw_x = movement_world_x - anim.offset_move.0;
                                let draw_y = movement_world_y - anim.offset_move.1;
                                
                                // 🎯 调试：每60帧打印一次渲染位置
                                static mut RENDER_FRAME_COUNT: u32 = 0;
                                unsafe {
                                    RENDER_FRAME_COUNT += 1;
                                    if RENDER_FRAME_COUNT % 60 == 0 {
                                        println!("🖼️ [渲染] movement_grid=({},{}) → world_center=({:.1},{:.1}) - offset=({:.1},{:.1}) = draw=({:.1},{:.1})",
                                            anim.movement_grid.0, anim.movement_grid.1,
                                            movement_world_x, movement_world_y,
                                            anim.offset_move.0, anim.offset_move.1,
                                            draw_x, draw_y);
                                    }
                                }
                                
                                (draw_x, draw_y)
                            } else {
                                // 📍 兼容模式：使用Position
                                (player_pos.x, player_pos.y)
                            };
                            
                            // 🎯 纹理位置计算:
                            // draw_world 现在是格子中心(红点)
                            // 纹理底边应该对齐格子底边，X轴居中
                            use crate::ecs::{CELL_HEIGHT};
                            let green_bottom_y = draw_world_y + (CELL_HEIGHT as f32 / 2.0);
                            let world_x = draw_world_x - (char_w as f32 / 2.0);
                            let world_y = green_bottom_y - char_h as f32;
                            
                            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                                camera_pos, 
                                camera, 
                                world_x,
                                world_y
                            );
                            
                            // 🎯 角色与Front层使用ADD混合
                            // 当角色在树木、建筑等Front层物体下方时，使用ADD混合实现半透明遮挡效果
                            // 使用 60% 亮度让 ADD 混合效果更明显
                            canvas.set_blend_mode(Self::create_blend_mode());
                            canvas.draw(
                                texture,
                                DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(Color::from_rgba(153, 153, 153, 255)),  // 60% 亮度
                            );
                            // 恢复默认混合模式
                            canvas.set_blend_mode(graphics::BlendMode::ALPHA);

                            // 🔍 遮挡调试：绘制青色边框，表示角色正在使用ADD混合模式
                            let rect = graphics::Rect::new(
                                screen_x,
                                screen_y,
                                char_w as f32 * camera.zoom,
                                char_h as f32 * camera.zoom,
                            );
                            let mesh = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::stroke(2.0),
                                rect,
                                Color::from_rgb(0, 255, 255),  // 青色边框
                            )?;
                            canvas.draw(&mesh, DrawParam::default());
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        
        Ok(())
    }

    /// 绘制角色武器
    /// 
    /// # 参数
    /// - `appearance`: 角色外观(包含武器索引)
    /// - `player`: 玩家组件(动作、方向、帧)
    /// - `camera_pos/camera`: 相机信息
    /// - `player_pos`: 角色位置
    /// - `body_frame`: 身体帧索引(用于计算武器帧偏移)
    /// - `world_x/world_y`: 身体纹理的世界坐标
    /// - `char_w/char_h`: 身体纹理尺寸
    #[allow(clippy::too_many_arguments)]
    fn draw_weapon(
        ctx: &mut Context,
        canvas: &mut Canvas,
        appearance: &PlayerAppearance,
        player: &Player,
        camera_pos: &Position,
        camera: &Camera,
        _player_pos: &Position,
        _body_frame: i32,
        world_x: f32,
        world_y: f32,
        _char_w: i16,
        _char_h: i16,
    ) -> GameResult<()> {
        // 如果没有装备武器,跳过
        if appearance.weapon < 0 {
            return Ok(());
        }
        
        use crate::graphics::libraries::{get_library_from_array, LibraryArray};
        use crate::ecs::systems::CameraSystem;
        use mir2_shared::enums::MirGender;
        
        // 🗡️ CWeapon库 (武器纹理) - 使用LibraryArray
        let weapon_index = appearance.weapon as usize;
        
        // 计算武器帧索引 (与身体相同的动作和方向)
        let action_frame_start = player.action.frame_start();
        let frames_per_direction = player.action.frame_count();
        let direction_offset = (player.direction as i32) * frames_per_direction;
        let draw_frame = action_frame_start + direction_offset + player.frame_index;
        
        // 🚺 性别帧偏移
        let weapon_offset = match appearance.gender {
            MirGender::Male => 0,
            MirGender::Female => 808,
        };
        let final_weapon_frame = draw_frame + weapon_offset;
        
        // 获取武器纹理
        if let Some(wlib) = get_library_from_array(LibraryArray::CWeapons, weapon_index) {
            if let Ok(mut wlib) = wlib.lock() {
                match wlib.get_or_create_texture(ctx, final_weapon_frame as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                            // 武器与身体使用相同的位置 (原版逻辑)
                            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                                camera_pos, 
                                camera, 
                                world_x,
                                world_y
                            );
                            
                            // 🎯 武器也使用 ADD 混合模式，与角色保持一致
                            // 使用 60% 亮度
                            canvas.set_blend_mode(Self::create_blend_mode());
                            canvas.draw(
                                texture,
                                DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(Color::from_rgba(153, 153, 153, 255)),  // 60% 亮度
                            );
                            canvas.set_blend_mode(graphics::BlendMode::ALPHA);
                            
                            // 🌟 绘制武器特效 (CWeaponEffect库)
                            if appearance.weapon_effect > 0 {
                                let effect_index = appearance.weapon_effect as usize;
                                if let Some(elib) = get_library_from_array(LibraryArray::CWeaponEffect, effect_index) {
                                    if let Ok(mut elib) = elib.lock() {
                                        match elib.get_or_create_texture(ctx, final_weapon_frame as usize) {
                                            Ok(effect_info) => {
                                                if let Some(ref effect_texture) = effect_info.image {
                                                    // 武器特效使用混合模式 (0.4透明度)
                                                    canvas.set_blend_mode(Self::create_blend_mode());
                                                    canvas.draw(
                                                        effect_texture,
                                                        DrawParam::default()
                                                            .dest([screen_x, screen_y])
                                                            .scale([camera.zoom, camera.zoom])
                                                            .color(Color::from_rgba(255, 255, 255, 102)), // 40% alpha = 102
                                                    );
                                                    canvas.set_blend_mode(graphics::BlendMode::ALPHA);
                                                }
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        
        Ok(())
    }

    /// 绘制角色（带Front层重叠检测）
    pub fn draw_player_with_world(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        player: &Player,
        player_pos: &Position,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::graphics::libraries::{get_library, LibraryName};
        use crate::ecs::systems::CameraSystem;
        
        // 🎬 尝试获取MovementAnimation组件用于动画帧插值
        let mut movement_anim_query = world.query::<(&LocalPlayer, &MovementAnimation)>();
        let movement_anim = movement_anim_query.iter().next().map(|(_, (_, anim))| anim);
        
        use crate::ecs::components::{MapTile, TileLayer};
        use mir2_shared::enums::{MirClass, MirGender};
        
        // 🎨 尝试获取 PlayerAppearance 组件（如果存在）
        let appearance = world.query::<&PlayerAppearance>()
            .iter()
            .next()
            .map(|(_, app)| app.clone());
        
        // 如果没有外观组件，使用默认值
        let (class, gender, armour_index) = if let Some(app) = &appearance {
            (app.class, app.gender, app.armour)
        } else {
            // 默认：战士，男性，盔甲索引0
            (MirClass::Warrior, MirGender::Male, 0)
        };
        
        // 🎨 根据职业、动作和性别选择库
        // 原版C#逻辑:
        //   - 战士/道士/法师: 所有动作使用 CArmours
        //   - 弓箭手: 跑步/射箭使用 ARArmours (altAnim=true)
        let library_index = armour_index.max(0);
        
        // 🏹 弓箭手特殊处理：跑步使用ARArmours库
        let (library_name, use_alt_anim) = match class {
            MirClass::Archer => {
                match player.action {
                    PlayerAction::Run => {
                        // 弓箭手跑步使用ARArmours库
                        (LibraryName::ARArmours(library_index as usize), true)
                    }
                    _ => {
                        // 其他动作使用CArmours库
                        (LibraryName::CArmours(library_index as usize), false)
                    }
                }
            }
            _ => {
                // 战士/道士/法师：所有动作使用CArmours库
                (LibraryName::CArmours(library_index as usize), false)
            }
        };
        
        // 计算 DrawFrame
        let action_frame_start = player.action.frame_start();
        let frames_per_direction = player.action.frame_count();
        let direction_offset = (player.direction as i32) * frames_per_direction;
        let draw_frame = action_frame_start + direction_offset + player.frame_index;
        
        // 🚺 性别帧偏移（原版逻辑）
        let armour_offset = if use_alt_anim {
            // 弓箭手ARArmours库使用不同的偏移
            match gender {
                MirGender::Male => 0,
                MirGender::Female => 352,  // 女性ARArmours偏移
            }
        } else {
            // 普通CArmours库偏移
            match gender {
                MirGender::Male => 0,
                MirGender::Female => 808,  // 女性CArmours偏移
            }
        };
        let final_frame = draw_frame + armour_offset;
        
        // 🐛 DEBUG: 打印详细的动作信息
        static mut LAST_ACTION: Option<PlayerAction> = None;
        static mut FRAME_LOG_COUNT: i32 = 0;
        unsafe {
            let should_log = LAST_ACTION.is_none() || LAST_ACTION.unwrap() != player.action || FRAME_LOG_COUNT < 3;
            if should_log {
                println!("\n🎭 === 角色动画帧计算 ===");
                println!("职业: {:?}, 性别: {:?}, 盔甲: {}", class, gender, armour_index);
                println!("动作: {:?} (frame_start={})", player.action, action_frame_start);
                println!("方向: {} ({}帧/方向)", player.direction, frames_per_direction);
                println!("当前帧: {}/{}", player.frame_index, frames_per_direction);
                println!("DrawFrame: {} + {} + {} = {}", action_frame_start, direction_offset, player.frame_index, draw_frame);
                println!("使用库: {:?} (altAnim={})", library_name, use_alt_anim);
                println!("性别偏移: {}", armour_offset);
                println!("FinalFrame: {} + {} = {}", draw_frame, armour_offset, final_frame);
                println!("========================\n");
                FRAME_LOG_COUNT += 1;
                if LAST_ACTION.is_none() || LAST_ACTION.unwrap() != player.action {
                    LAST_ACTION = Some(player.action);
                    FRAME_LOG_COUNT = 0;
                }
            }
        }
        
        tracing::debug!("🎭 角色渲染: class={:?}, gender={:?}, armour={}, action={:?}, draw_frame={}, offset={}, final={}", 
            class, gender, armour_index, player.action, draw_frame, armour_offset, final_frame);
        
        // 🎯 遮挡检测：检测角色是否被Front层瓦片遮挡
        // 遮挡条件：
        // 1. Front层瓦片的世界Y坐标 <= 角色脚底Y坐标（瓦片在前面绘制）
        // 2. Front层瓦片在屏幕空间与角色有重叠
        use crate::ecs::Coordinates;
        use crate::ecs::{CELL_WIDTH, CELL_HEIGHT};
        
        let mut _has_front_overlap = false;
        
        // 角色脚底的世界坐标和格子坐标
        let player_world_x = player_pos.x;
        let player_world_y = player_pos.y;
        let (player_grid_x, player_grid_y) = Coordinates::world_to_grid(player_world_x, player_world_y);
        
        // 预先获取角色的尺寸信息（用于碰撞检测）
        // 🎯 使用稍大的检测范围，避免边缘临界状态导致闪烁
        let char_height = 80.0; // 角色大约高度（加大检测范围）
        let char_width = 64.0;  // 角色大约宽度（加大检测范围）
        
        for (_, tile) in world.query::<&MapTile>().iter() {
            // 只检查Front层
            if !matches!(tile.layer, TileLayer::Front) {
                continue;
            }
            
            // 瓦片的世界坐标（左上角）
            let tile_world_x = (tile.grid_x * CELL_WIDTH) as f32;
            let tile_world_y = (tile.grid_y * CELL_HEIGHT) as f32;
            
            // 条件1: Front层瓦片的Y坐标 <= 角色的Y坐标（格子空间）
            // 即瓦片在角色前面或同一行
            if tile.grid_y > player_grid_y {
                continue; // 瓦片在角色后面，不会遮挡
            }
            
            // 条件2: 在世界空间检查X方向重叠
            // Front层瓦片通常比较大（树木、建筑），需要获取实际尺寸
            // 简化处理：假设Front层瓦片至少覆盖 2x2 格子 (96x64)
            let tile_width = CELL_WIDTH as f32 * 2.0;  // 假设宽度
            let tile_height = CELL_HEIGHT as f32 * 3.0; // 假设高度（建筑物可能更高）
            
            // 角色的包围盒（以脚底为基准）
            let char_left = player_world_x - char_width / 2.0;
            let char_right = player_world_x + char_width / 2.0;
            let char_top = player_world_y - char_height;
            let char_bottom = player_world_y;
            
            // 瓦片的包围盒
            let tile_left = tile_world_x;
            let tile_right = tile_world_x + tile_width;
            let tile_top = tile_world_y;
            let tile_bottom = tile_world_y + tile_height;
            
            // AABB碰撞检测
            let x_overlap = char_right > tile_left && char_left < tile_right;
            let y_overlap = char_bottom > tile_top && char_top < tile_bottom;
            
            if x_overlap && y_overlap {
                _has_front_overlap = true;
                break;
            }
        }
        // 纹理尺寸和偏移量 (用于后续 AABB 计算)
        let mut char_w = 48;
        let mut char_h = 64;
        let world_x;
        let world_y;
        
        // ✅ 获取角色纹理 - 根据职业和性别使用对应的库
        if let Some(mlib) = get_library(library_name) {
            if let Ok(mut mlib) = mlib.lock() {
                // 获取尺寸和偏移量
                (char_w, char_h) = mlib
                    .get_size(final_frame as usize)
                    .unwrap_or((48, 64));
                
                let (_offset_x, _offset_y) = mlib
                    .get_offset(final_frame as usize)
                    .unwrap_or((0, 0));
                
                // 🎬 动画帧插值位置计算 (原版C#机制)
                // 如果有MovementAnimation组件，使用插值计算绘制位置
                let (draw_world_x, draw_world_y) = if let Some(anim) = movement_anim {
                    // 🎯 使用动画帧插值计算位置
                    // Movement位置（目标格子中心）
                    let movement_world_x = anim.movement_grid.0 as f32 * CELL_WIDTH as f32;
                    let movement_world_y = anim.movement_grid.1 as f32 * CELL_HEIGHT as f32;
                    
                    // 应用offset_move插值
                    let draw_x = movement_world_x - anim.offset_move.0;
                    let draw_y = movement_world_y - anim.offset_move.1;
                    
                    (draw_x, draw_y)
                } else {
                    // 📍 兼容模式：使用Position
                    (player_pos.x, player_pos.y)
                };
                
                // 计算纹理位置
                let green_bottom_y = draw_world_y + (CELL_HEIGHT as f32 / 2.0);
                world_x = draw_world_x + (CELL_WIDTH as f32 / 2.0) - (char_w as f32 / 2.0);
                world_y = green_bottom_y - char_h as f32;
                
                // 🗡️ 左侧方向:先绘制武器(在身体后面)
                // player.direction 是 u8: 0=Up, 1=UpRight, 2=Right, 3=DownRight, 4=Down, 5=DownLeft, 6=Left, 7=UpLeft
                let draw_weapon_back = matches!(player.direction, 
                    0 | 6 | 7 | 5); // Up, Left, UpLeft, DownLeft
                
                if draw_weapon_back {
                    if let Some(ref app) = appearance {
                        Self::draw_weapon(ctx, canvas, app, player, camera_pos, camera, 
                            player_pos, final_frame, world_x, world_y, char_w, char_h)?;
                    }
                }
                
                // 获取纹理
                match mlib.get_or_create_texture(ctx, final_frame as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                            // 🎯 纹理位置计算:
                            // player_pos 现在是格子中心(红点)
                            // 纹理底边应该对齐格子底边
                            // 
                            // 格子底边Y = player_pos.y + CELL_HEIGHT/2
                            // 纹理底边Y = world_y + char_h
                            // 所以: world_y = player_pos.y + CELL_HEIGHT/2 - char_h
                            // 
                            // ⚠️ X方向: 原工程中角色在格子右侧，所以需要向右偏移
                            // 格子中心 + 半格宽度 = 格子右边缘
                            
                            let (screen_x, screen_y) = CameraSystem::world_to_screen(
                                camera_pos, 
                                camera, 
                                world_x,
                                world_y
                            );
                            
                            // 🎯 角色始终使用 ALPHA 混合模式绘制
                            canvas.set_blend_mode(graphics::BlendMode::ALPHA);
                            
                            canvas.draw(
                                texture,
                                DrawParam::default()
                                    .dest([screen_x, screen_y])
                                    .scale([camera.zoom, camera.zoom])
                                    .color(Color::WHITE),
                            );
                            
                            // 🗡️ 绘制武器 (CWeapon库)
                            // 根据方向决定武器层级:
                            // - 左侧方向: 武器在身体后面(已在前面绘制)
                            // - 右侧方向: 武器在身体前面(在这里绘制)
                            // player.direction 是 u8: 0=Up, 1=UpRight, 2=Right, 3=DownRight, 4=Down, 5=DownLeft, 6=Left, 7=UpLeft
                            let draw_weapon_front = matches!(player.direction, 
                                1 | 2 | 3 | 4); // UpRight, Right, DownRight, Down
                            
                            if draw_weapon_front {
                                if let Some(ref app) = appearance {
                                    Self::draw_weapon(ctx, canvas, app, player, camera_pos, camera, 
                                        player_pos, final_frame, world_x, world_y, char_w, char_h)?;
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            } else {
                // 没有纹理时,先尝试绘制武器(左侧方向)
                let draw_weapon_back = matches!(player.direction, 
                    0 | 6 | 7 | 5); // Up, Left, UpLeft, DownLeft
                
                if draw_weapon_back {
                    if let Some(app) = &appearance {
                        Self::draw_weapon(ctx, canvas, app, player, camera_pos, camera, 
                            player_pos, 0, 0.0, 0.0, 48, 64)?;
                    }
                }
            }
        } else {
            // 身体纹理获取失败前,先绘制武器(左侧方向)
            let draw_weapon_back = matches!(player.direction, 
                0 | 6 | 7 | 5); // Up, Left, UpLeft, DownLeft
            
            if draw_weapon_back {
                if let Some(app) = &appearance {
                    Self::draw_weapon(ctx, canvas, app, player, camera_pos, camera, 
                        player_pos, 0, 0.0, 0.0, 48, 64)?;
                }
            }
        }
        
        /* 🐛 旧的调试绘制代码 - 已禁用，使用新的遮挡调试边框
        // 🐛 调试绘制:显示碰撞检测和渲染相关的边界
        
        // 1. 绘制人物所在格子边界(绿色) - 用于移动碰撞检测
        //    服务器检查: ValidPoint(格子是否可行走) + cell.Objects(格子内对象阻挡)
        //    红点(player_pos)应该在绿框的中心位置
        let grid_world_x = player_pos.x - (CELL_WIDTH as f32 / 2.0);  // 格子左边 = 中心 - 半格宽
        let grid_world_y = player_pos.y - (CELL_HEIGHT as f32 / 2.0);  // 格子顶边 = 中心 - 半格高
        let (grid_screen_x, grid_screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            grid_world_x,
            grid_world_y,
        );
        
        let grid_rect = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            graphics::Rect::new(
                grid_screen_x,
                grid_screen_y,
                CELL_WIDTH as f32 * camera.zoom,
                CELL_HEIGHT as f32 * camera.zoom,
            ),
            Color::from_rgb(0, 255, 0), // 绿色边框
        )?;
        canvas.draw(&grid_rect, DrawParam::default());
        
        // 2. 绘制人物包围盒(黄色) - 应该完全包裹人物纹理
        //    AABB用于与Front层瓦片做遮挡检测
        //    黄框底边应该与绿框底边对齐，X轴居中对齐
        //    绿框底边Y = player_pos.y + CELL_HEIGHT/2
        //    黄框底边Y = char_top + char_h = 绿框底边Y
        //    所以: char_top = player_pos.y + CELL_HEIGHT/2 - char_h
        
        let green_bottom_y = player_pos.y + (CELL_HEIGHT as f32 / 2.0);  // 绿框底边
        let char_left = player_pos.x - (char_w as f32 / 2.0);  // 黄框X居中对齐
        let char_top = green_bottom_y - char_h as f32;  // 黄框底边对齐绿框底边
        let (char_screen_x, char_screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            char_left,
            char_top,
        );
        
        let char_rect = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            graphics::Rect::new(
                char_screen_x,
                char_screen_y,
                char_w as f32 * camera.zoom,
                char_h as f32 * camera.zoom,
            ),
            Color::from_rgb(255, 255, 0), // 黄色边框
        )?;
        canvas.draw(&char_rect, DrawParam::default());
        
        // 3. 绘制人物脚底中心点(红色圆点) - Position组件的实际位置
        //    这是角色的锚点,用于计算纹理渲染位置和格子坐标
        let (foot_screen_x, foot_screen_y) = CameraSystem::world_to_screen(
            camera_pos,
            camera,
            player_pos.x,
            player_pos.y,
        );
        
        let foot_circle = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            [foot_screen_x, foot_screen_y],
            4.0 * camera.zoom,
            0.1,
            Color::from_rgb(255, 0, 0), // 红色圆点
        )?;
        canvas.draw(&foot_circle, DrawParam::default());
        
        // 4. 绘制坐标文字
        let coord_text = graphics::Text::new(format!("({}, {})", player_grid_x, player_grid_y));
        canvas.draw(
            &coord_text,
            DrawParam::default()
                .dest([grid_screen_x + 5.0, grid_screen_y + 5.0])
                .color(Color::from_rgb(255, 255, 255))
                .scale([0.8, 0.8]),
        );
        */
        
        Ok(())
    }
}
