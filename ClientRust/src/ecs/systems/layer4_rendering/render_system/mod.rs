// ============================================================================
// Render System - 渲染系统模块化
// ============================================================================

mod debug;
mod item;
mod monster;
mod npc;
mod player;
mod tiles;
mod ui;  // UI渲染方法 (RenderSystem::draw_ui)


use ggez::{Context, GameResult};
use ggez::graphics::{self, Canvas, DrawParam, Color, BlendMode, BlendComponent, BlendFactor, BlendOperation, Text, TextFragment, PxScale, Rect, Mesh};
use hecs::World;
use crate::ecs::components::{Camera, QuestIcon, Position, RenderConfig, Player, MapTile, VisibleArea};
use crate::ecs::{TileLayer, CELL_HEIGHT};

/// 实体类型枚举（用于Y-sorting）
#[derive(Debug, Clone, Copy)]
enum EntityType {
    Monster(hecs::Entity),
    NPC(hecs::Entity),
    Player(hecs::Entity),
    FrontTile(hecs::Entity),
}

/// 渲染系统主结构
pub struct RenderSystem;

impl RenderSystem {
    /// 🎯 统一渲染入口：渲染整个游戏世界
    pub fn draw_game_world(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
        visible_area_entity: hecs::Entity,
        debug_counters_entity: hecs::Entity,
    ) -> GameResult {
        // ==================== 第一步: 渲染地面层 (Back + Middle) ====================
        let mut ground_config = config.clone();
        ground_config.show_front = false;
        
        Self::draw_tiles(
            ctx,
            canvas,
            world,
            pos,
            camera,
            &ground_config,
            visible_area_entity,
        )?;
        
        // ==================== 第二步: 渲染 Front 层建筑物 + 角色/怪物（按 Y 排序）====================
        // 🎯 原版逻辑：按 Y 行逐行绘制，每行内 Front 层在对象之前绘制
        // 这样实现正确的遮挡关系：上方的对象被下方的建筑物遮挡
        Self::draw_front_and_entities_sorted(ctx, canvas, world, pos, camera, config, Some(visible_area_entity))?;
        
        // ==================== 第四步: 顶层信息 ====================
        Self::draw_items(ctx, canvas, world, pos, camera)?;
        Self::draw_monster_info(ctx, canvas, world, pos, camera)?;
        
        // ==================== 调试层 ====================
        if config.show_grid {
            Self::draw_grid(ctx, canvas, world, pos, camera)?;
        }
        if config.show_obstacles {
            Self::draw_obstacles(ctx, canvas, world, pos, camera)?;
        }
        if config.show_path {
            Self::draw_path(ctx, canvas, world, pos, camera)?;
        }
        
        Ok(())
    }
    
    /// 收集所有需要 Y-sorting 的实体
    fn collect_sorted_entities(
        world: &World,
        config: &RenderConfig,
        visible_area_entity: hecs::Entity,
        debug_counters_entity: hecs::Entity,
    ) -> GameResult<Vec<(i32, EntityType)>> {
        let mut entities = Vec::new();
        
        // 收集怪物
        let mut monster_count = 0;
        for (entity, pos) in world.query::<&Position>().iter() {
            if world.get::<&crate::ecs::components::MonsterData>(entity).is_ok() {
                // 🎯 使用怪物底部的Y坐标
                let monster_y = pos.y as i32 + CELL_HEIGHT;
                entities.push((monster_y, EntityType::Monster(entity)));
                monster_count += 1;
            }
        }
        
        // 收集NPC
        let mut npc_count = 0;
        for (entity, pos) in world.query::<&Position>().iter() {
            if world.get::<&crate::ecs::components::NPCData>(entity).is_ok() {
                // 🎯 使用NPC底部的Y坐标
                let npc_y = pos.y as i32 + CELL_HEIGHT;
                entities.push((npc_y, EntityType::NPC(entity)));
                npc_count += 1;
            }
        }
        
        // 收集玩家
        for (entity, pos) in world.query::<&Position>().iter() {
            if world.get::<&Player>(entity).is_ok() {
                // 🎯 使用角色底部的Y坐标（pos.y 是格子顶部，+ CELL_HEIGHT 是底部）
                let player_y = pos.y as i32 + CELL_HEIGHT;
                entities.push((player_y, EntityType::Player(entity)));
            }
        }
        
        // 收集Front层瓦片（参与Y-sorting）
        let mut front_tile_count = 0;
        if let Ok(visible_area) = world.get::<&VisibleArea>(visible_area_entity) {
            for &entity in &visible_area.visible_entities {
                if let Ok(tile) = world.get::<&MapTile>(entity) {
                    if matches!(tile.layer, TileLayer::Front) && config.show_front {
                        // 🎯 使用瓦片格子底部的Y坐标作为排序依据
                        // 这样建筑物会根据其所在格子的Y位置进行排序
                        // grid_y 越大 = 越靠后 = 排序值越大
                        let tile_y = (tile.grid_y + 1) * CELL_HEIGHT;
                        entities.push((tile_y, EntityType::FrontTile(entity)));
                        front_tile_count += 1;
                    }
                }
            }
        }
        
        // 📊 记录 Y-sorting 日志
        if let Ok(mut debug) = world.get::<&mut crate::ecs::components::DebugCounters>(debug_counters_entity) {
            if debug.should_log_y_sorting() {
                tracing::info!(
                    "🎯 Y-sorting: {} monsters, {} NPCs, {} front tiles",
                    monster_count, npc_count, front_tile_count
                );
            }
        }
        
        // Y-sorting: 按Y坐标排序（从小到大，远处先绘制）
        entities.sort_by_key(|(y, _)| *y);
        
        Ok(entities)
    }
    
    /// 按Y排序顺序渲染所有实体
    fn render_sorted_entities(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        entities: &[(i32, EntityType)],
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult {
        for (_y, entity_type) in entities {
            match entity_type {
                EntityType::Monster(entity) => {
                    if let Ok(entity_pos) = world.get::<&Position>(*entity) {
                        Self::draw_single_monster(ctx, canvas, world, *entity, &entity_pos, pos, camera, config)?;
                    }
                }
                EntityType::NPC(entity) => {
                    if let Ok(entity_pos) = world.get::<&Position>(*entity) {
                        Self::draw_single_npc(ctx, canvas, world, *entity, &entity_pos, pos, camera, config)?;
                    }
                }
                EntityType::Player(entity) => {
                    if let (Ok(player), Ok(player_pos)) = (
                        world.get::<&Player>(*entity),
                        world.get::<&Position>(*entity)
                    ) {
                        Self::draw_player_with_world(ctx, canvas, world, &player, &player_pos, pos, camera)?;
                    }
                }
                EntityType::FrontTile(entity) => {
                    if let Ok(tile) = world.get::<&MapTile>(*entity) {
                        // � Front 层瓦片默认使用 ALPHA 混合
                        canvas.set_blend_mode(graphics::BlendMode::ALPHA);
                        Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config, 1.0, world)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 🎯 按 Y 坐标排序绘制 Front 层和所有实体（原版逻辑）
    /// 
    /// 原版 C# 逻辑：所有对象（包括 Front 层瓦片、角色）按照底部Y坐标排序后绘制
    /// 
    /// 这样实现正确的 Y-sorting：
    /// - Y 坐标小的对象先绘制（在后面）
    /// - Y 坐标大的对象后绘制（在前面）
    /// - 实现正确的前后遮挡关系
    fn draw_front_and_entities_sorted(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
        visible_area_entity: Option<hecs::Entity>,
    ) -> GameResult {
        use crate::ecs::CELL_HEIGHT;
        
        // 定义渲染对象类型
        enum RenderObject {
            FrontTile(hecs::Entity),
            Monster(hecs::Entity, Position),
            NPC(hecs::Entity, Position),
            Player(hecs::Entity, Position),
        }
        
        // 收集所有需要渲染的对象及其 Y 坐标
        let mut render_objects: Vec<(i32, RenderObject)> = Vec::new();
        
        // 1. 收集可见区域内的 Front 层瓦片（性能优化）
        if config.show_front {
            if let Some(visible_entity) = visible_area_entity {
                if let Ok(visible_area) = world.get::<&VisibleArea>(visible_entity) {
                    for &entity in &visible_area.visible_entities {
                        if let Ok(tile) = world.get::<&MapTile>(entity) {
                            if matches!(tile.layer, TileLayer::Front) {
                                // Front 层瓦片使用格子底部的 Y 坐标
                                let tile_bottom_y = (tile.grid_y + 1) * CELL_HEIGHT;
                                render_objects.push((tile_bottom_y, RenderObject::FrontTile(entity)));
                            }
                        }
                    }
                }
            } else {
                // 如果没有VisibleArea，回退到查询所有瓦片（兼容模式）
                for (entity, tile) in world.query::<&MapTile>().iter() {
                    if matches!(tile.layer, TileLayer::Front) {
                        let tile_bottom_y = (tile.grid_y + 1) * CELL_HEIGHT;
                        render_objects.push((tile_bottom_y, RenderObject::FrontTile(entity)));
                    }
                }
            }
        }
        
        // 2. 收集所有怪物
        for (entity, entity_pos) in world.query::<&Position>().iter() {
            if world.get::<&crate::ecs::components::MonsterData>(entity).is_ok() {
                // 使用角色底部的 Y 坐标（角色站立点）
                let entity_bottom_y = entity_pos.y as i32 + CELL_HEIGHT;
                render_objects.push((entity_bottom_y, RenderObject::Monster(entity, entity_pos.clone())));
            }
        }
        
        // 3. 收集所有 NPC
        for (entity, entity_pos) in world.query::<&Position>().iter() {
            if world.get::<&crate::ecs::components::NPCData>(entity).is_ok() {
                let entity_bottom_y = entity_pos.y as i32 + CELL_HEIGHT;
                render_objects.push((entity_bottom_y, RenderObject::NPC(entity, entity_pos.clone())));
            }
        }
        
        // 4. 收集所有玩家
        for (entity, (_, entity_pos)) in world.query::<(&Player, &Position)>().iter() {
            let entity_bottom_y = entity_pos.y as i32 + CELL_HEIGHT;
            render_objects.push((entity_bottom_y, RenderObject::Player(entity, entity_pos.clone())));
        }
        
        // 5. 按 Y 坐标排序（从小到大，先绘制后面的对象）
        render_objects.sort_by_key(|(y, _)| *y);
        
        // 6. 按顺序绘制所有对象
        for (_, render_obj) in render_objects {
            match render_obj {
                RenderObject::FrontTile(entity) => {
                    if let Ok(tile) = world.get::<&MapTile>(entity) {
                        Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config, 1.0, world)?;
                    }
                }
                RenderObject::Monster(entity, entity_pos) => {
                    Self::draw_single_monster(ctx, canvas, world, entity, &entity_pos, pos, camera, config)?;
                }
                RenderObject::NPC(entity, entity_pos) => {
                    Self::draw_single_npc(ctx, canvas, world, entity, &entity_pos, pos, camera, config)?;
                }
                RenderObject::Player(entity, entity_pos) => {
                    Self::draw_player_with_occlusion(ctx, canvas, world, entity, &entity_pos, pos, camera, config)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// 绘制玩家（带遮挡检测）
    fn draw_player_with_occlusion(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        entity: hecs::Entity,
        player_pos: &Position,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult {
        // 默认使用 ALPHA 混合
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);
        
        // 计算角色所在的格子坐标
        let grid_x = (player_pos.x / crate::ecs::CELL_WIDTH as f32) as i32;
        let grid_y = (player_pos.y / crate::ecs::CELL_HEIGHT as f32) as i32;
        
        // 查询该格子及周围的 Front 层瓦片，决定是否被遮挡
        for (_, tile) in world.query::<&MapTile>().iter() {
            if (tile.grid_x - grid_x).abs() <= 1
                && (tile.grid_y - grid_y).abs() <= 1
                && matches!(tile.layer, TileLayer::Front) 
            {
                use crate::graphics::get_map_library;
                if let Some(lib) = get_map_library(tile.library_index) {
                    if let Ok(mut lib_guard) = lib.lock() {
                        if let Ok(info) = lib_guard.get_or_create_texture(ctx, tile.image_index as usize) {
                            if let Some(ref texture) = info.image {
                                let tile_world_y = tile.grid_y as f32 * crate::ecs::CELL_HEIGHT as f32;
                                let tile_bottom_y = tile_world_y + texture.height() as f32;
                                
                                // 🎯 正确的遮挡判断：玩家Y坐标 > 瓦片底部Y，说明玩家在建筑物后面（被遮挡）
                                if player_pos.y > tile_bottom_y {
                                    canvas.set_blend_mode(graphics::BlendMode {
                                        color: graphics::BlendComponent {
                                            src_factor: graphics::BlendFactor::SrcAlpha,
                                            dst_factor: graphics::BlendFactor::OneMinusSrcAlpha,
                                            operation: graphics::BlendOperation::Add,
                                        },
                                        alpha: graphics::BlendComponent {
                                            src_factor: graphics::BlendFactor::One,
                                            dst_factor: graphics::BlendFactor::OneMinusSrcAlpha,
                                            operation: graphics::BlendOperation::Add,
                                        },
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 绘制玩家
        if let Ok(player) = world.get::<&Player>(entity) {
            // � 绘制玩家（已设置好混合模式，draw_player_with_world会自动查询MovementAnimation）
            Self::draw_player_with_world(ctx, canvas, world, &*player, player_pos, pos, camera)?;
        }
        
        // 恢复默认混合模式
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);
        
        Ok(())
    }

    /// 绘制所有角色、怪物、NPC（不包括Front层瓦片）
    /// 🎯 参考 atlas.rs：在建筑物绘制完后，根据位置动态切换混合模式
    fn draw_all_entities(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult {
        use crate::graphics::libraries::get_library;
        
        // 绘制所有怪物
        for (entity, entity_pos) in world.query::<&Position>().iter() {
            if world.get::<&crate::ecs::components::MonsterData>(entity).is_ok() {
                Self::draw_single_monster(ctx, canvas, world, entity, &entity_pos, pos, camera, config)?;
            }
        }
        
        // 绘制所有NPC
        for (entity, entity_pos) in world.query::<&Position>().iter() {
            if world.get::<&crate::ecs::components::NPCData>(entity).is_ok() {
                Self::draw_single_npc(ctx, canvas, world, entity, &entity_pos, pos, camera, config)?;
            }
        }
        
        // 绘制所有玩家（关键：动态切换混合模式）
        for (entity, (player, player_pos)) in world.query::<(&Player, &Position)>().iter() {
            // 🎯 默认使用 ALPHA 混合（正常显示）
            canvas.set_blend_mode(graphics::BlendMode::ALPHA);
            
            // 计算角色所在的格子坐标
            let grid_x = (player_pos.x / crate::ecs::CELL_WIDTH as f32) as i32;
            let grid_y = (player_pos.y / crate::ecs::CELL_HEIGHT as f32) as i32;
            
            // 查询该格子及周围的 Front 层瓦片，决定是否被遮挡
            for (_, tile) in world.query::<&MapTile>().iter() {
                // 检查周围2x2格子范围内的 Front 层瓦片
                if (tile.grid_x - grid_x).abs() <= 1
                    && (tile.grid_y - grid_y).abs() <= 1
                    && matches!(tile.layer, TileLayer::Front) 
                {
                    // 获取纹理信息
                    use crate::graphics::get_map_library;
                    
                    if let Some(lib) = get_map_library(tile.library_index) {
                        if let Ok(mut lib_guard) = lib.lock() {
                            if let Ok(info) = lib_guard.get_or_create_texture(ctx, tile.image_index as usize) {
                                if let Some(ref texture) = info.image {
                                    // 计算瓦片世界坐标（纹理左上角）
                                    let tile_world_y = tile.grid_y as f32 * crate::ecs::CELL_HEIGHT as f32;
                                    
                                    // 计算纹理底部Y坐标
                                    let tile_bottom_y = tile_world_y + texture.height() as f32;
                                    
                                    // 🎯 关键判断：角色被遮挡时使用 ADD 混合
                                    // 当角色Y + 85 < 瓦片底部Y 时，说明角色在建筑物后面（被遮挡）
                                    if player_pos.y + 85.0 < tile_bottom_y {
                                        // 被遮挡：使用 ADD 混合，产生半透明效果让玩家能看到角色
                                        canvas.set_blend_mode(graphics::BlendMode::ADD);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // 绘制角色
            Self::draw_player_with_world(ctx, canvas, world, &player, &player_pos, pos, camera)?;
        }
        
        Ok(())
    }

    /// 绘制NPC名字(带半透明黑色背景)
    pub(crate) fn draw_npc_name(
        ctx: &Context,
        canvas: &mut Canvas,
        name: &str,
        center_x: f32,
        y: f32,
        camera: &Camera,
    ) {
        // 创建文本
        let text_fragment = TextFragment::new(name)
            .scale(PxScale::from(14.0 * camera.zoom))
            .color(Color::from_rgb(255, 255, 0)); // 黄色
        let text = Text::new(text_fragment);

        // 计算文本尺寸
        let text_dims = text.measure(ctx).unwrap();
        let text_width = text_dims.x;
        let text_height = text_dims.y;

        // 居中对齐
        let text_x = center_x - text_width / 2.0;

        // 绘制半透明黑色背景
        let bg_padding = 4.0 * camera.zoom;
        let bg_rect = Rect::new(
            text_x - bg_padding,
            y - bg_padding,
            text_width + bg_padding * 2.0,
            text_height + bg_padding * 2.0,
        );

        if let Ok(bg_mesh) = Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 180),
        ) {
            canvas.draw(&bg_mesh, DrawParam::default());
        }

        // 绘制文本
        canvas.draw(&text, DrawParam::default().dest([text_x, y]));
    }

    /// 绘制任务图标
    pub(crate) fn draw_quest_icon(
        ctx: &Context,
        canvas: &mut Canvas,
        icon: QuestIcon,
        center_x: f32,
        y: f32,
        camera: &Camera,
    ) {
        let (symbol, color) = match icon {
            QuestIcon::None => return,                                      // 无图标
            QuestIcon::Available => ("!", Color::from_rgb(255, 255, 0)),    // 黄色感叹号
            QuestIcon::Complete => ("?", Color::from_rgb(255, 255, 0)),     // 黄色问号
            QuestIcon::Incomplete => ("?", Color::from_rgb(150, 150, 150)), // 灰色问号
        };

        // 创建图标文本
        let text_fragment = TextFragment::new(symbol)
            .scale(PxScale::from(24.0 * camera.zoom))
            .color(color);
        let text = Text::new(text_fragment);

        // 居中绘制
        let text_dims = text.measure(ctx).unwrap();
        let text_x = center_x - text_dims.x / 2.0;

        canvas.draw(&text, DrawParam::default().dest([text_x, y]));
    }

    /// 创建 ADD 混合模式 (火焰/特效)
    pub fn create_blend_mode() -> BlendMode {
        BlendMode {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        }
    }

    /// 将i32 ARGB颜色转换为ggez::Color
    pub(crate) fn argb_to_color(argb: i32) -> Color {
        if argb == 0 {
            return Color::WHITE; // 默认白色(无染色)
        }

        let a = ((argb >> 24) & 0xFF) as u8;
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;

        Color::from_rgba(r, g, b, a)
    }
}
