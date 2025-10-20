// ============================================================================
// GameScene with GGEZ + hecs ECS Architecture
// 示例: 展示如何使用新的 ECS 架构
// ============================================================================

use ggez::{
    Context, GameResult,
    graphics::{Canvas, Color, DrawParam, Image},
    input::keyboard::KeyCode,
};
use std::time::{Duration, Instant};

use crate::ecs::*;
use crate::graphics::libraries::get_map_library;
use mir2_shared::{enums::*, Point};

/// 游戏场景 (GGEZ + hecs 版本)
pub struct GameSceneECS {
    // ECS 世界
    pub world: GameWorld,

    // GGEZ 资源 (纹理缓存等)
    pub texture_cache: std::collections::HashMap<(i32, i32), Image>,

    // 相机
    pub camera_x: i32,
    pub camera_y: i32,

    // 时间
    pub last_update: Instant,
}

impl GameSceneECS {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let mut world = GameWorld::new();

        // 创建本地玩家
        let _player = world.spawn_local_player(
            "TestPlayer".to_string(),
            MirClass::Warrior,
            MirGender::Male,
            Point::new(100, 100),
        );

        // 创建一些怪物
        for i in 0..5 {
            world.spawn_monster(
                i + 1,
                format!("Monster{}", i),
                0,
                Point::new(100 + i as i32 * 5, 100 + i as i32 * 3),
            );
        }

        // 创建一个 NPC
        world.spawn_npc(
            100,
            "村长".to_string(),
            0,
            Point::new(95, 95),
        );

        Ok(Self {
            world,
            texture_cache: std::collections::HashMap::new(),
            camera_x: 100,
            camera_y: 100,
            last_update: Instant::now(),
        })
    }

    /// 更新游戏逻辑
    pub fn update(&mut self, ctx: &mut Context) -> GameResult {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update);
        self.last_update = now;

        // 运行各个系统
        MovementSystem::update(&mut self.world.world, delta);
        AnimationSystem::update(&mut self.world.world, delta);
        LifetimeSystem::update(&mut self.world.world, delta);
        AISystem::update(&mut self.world.world, delta);

        // 清理死亡/过期实体
        self.world.cleanup_dead_entities();

        // 更新相机跟随玩家
        if let Some(player_pos) = self.world.get_local_player_position() {
            self.camera_x = player_pos.x;
            self.camera_y = player_pos.y;
        }

        Ok(())
    }

    /// 渲染游戏画面
    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 收集可见实体
        let visible_entities = RenderSystem::collect_visible_entities(
            &self.world.world,
            self.camera_x,
            self.camera_y,
            40, // 可见范围 (格子)
            30,
        );

        // 渲染所有可见实体
        for (entity, pos, sprite, order) in visible_entities {
            self.draw_sprite(ctx, canvas, &pos, &sprite)?;
        }

        Ok(())
    }

    /// 绘制精灵 (使用 GGEZ)
    fn draw_sprite(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        pos: &Position,
        sprite: &SpriteComp,
    ) -> GameResult {
        // 计算屏幕坐标
        let screen_x = (pos.x - self.camera_x) * 48 + pos.offset_x + 800;
        let screen_y = (pos.y - self.camera_y) * 32 + pos.offset_y + 450;

        // 从 MLibrary 加载纹理 (缓存)
        let key = (sprite.library, sprite.index);
        if !self.texture_cache.contains_key(&key) {
            if let Some(lib) = get_map_library(sprite.library as usize) {
                if let Some(image_info) = lib.get_image_info(sprite.index as usize) {
                    // 创建 GGEZ Image
                    // TODO: 实际的纹理加载逻辑
                    // let image = Image::from_rgba8(...)?;
                    // self.texture_cache.insert(key, image);
                }
            }
        }

        // 绘制纹理
        if let Some(image) = self.texture_cache.get(&key) {
            let blend_mode = match sprite.blend_mode {
                BlendModeComp::Alpha => ggez::graphics::BlendMode::ALPHA,
                BlendModeComp::Add => ggez::graphics::BlendMode::ADD, // ⭐ ADD 混合!
                BlendModeComp::Multiply => ggez::graphics::BlendMode::MULTIPLY,
            };

            canvas.draw(
                image,
                DrawParam::new()
                    .dest([screen_x as f32, screen_y as f32])
                    .blend_mode(blend_mode),
            );
        }

        Ok(())
    }

    /// 处理键盘输入
    pub fn key_down_event(&mut self, key: KeyCode) {
        // 获取本地玩家实体
        if let Some(player_entity) = self.world.get_local_player() {
            // 处理移动
            let mut dx = 0;
            let mut dy = 0;

            match key {
                KeyCode::Up => dy = -1,
                KeyCode::Down => dy = 1,
                KeyCode::Left => dx = -1,
                KeyCode::Right => dx = 1,
                KeyCode::Space => {
                    // 释放技能 (ADD 混合特效)
                    if let Ok(pos) = self.world.world.get::<&Position>(player_entity) {
                        let current_pos = Point::new(pos.x, pos.y);
                        let target_pos = Point::new(pos.x + 5, pos.y);
                        
                        self.world.spawn_spell_effect(
                            1, // 技能ID
                            0, // 玩家ID
                            current_pos,
                            target_pos,
                            2000, // 2秒生命周期
                        );
                    }
                }
                _ => {}
            }

            // 更新玩家速度
            if dx != 0 || dy != 0 {
                if let Ok(mut vel) = self.world.world.get::<&mut Velocity>(player_entity) {
                    vel.dx = dx as f32 * 3.0;
                    vel.dy = dy as f32 * 3.0;
                }
            }
        }
    }

    /// 处理鼠标点击
    pub fn mouse_button_down_event(&mut self, x: f32, y: f32) {
        // 转换屏幕坐标到地图坐标
        let map_x = self.camera_x + ((x - 800.0) / 48.0) as i32;
        let map_y = self.camera_y + ((y - 450.0) / 32.0) as i32;

        // 检查点击位置的实体
        let entities = self.world.get_entities_at(Point::new(map_x, map_y));
        
        if !entities.is_empty() {
            println!("点击了实体: {:?}", entities);
            
            // 例如: 对怪物发起攻击
            for entity in entities {
                if self.world.world.get::<&Monster>(entity).is_ok() {
                    // 攻击怪物
                    CombatSystem::apply_damage(&mut self.world.world, entity, 10);
                }
            }
        }
    }

    /// 网络消息处理示例
    pub fn handle_network_packet(&mut self, packet_type: &str, data: &[u8]) {
        match packet_type {
            "ObjectPlayer" => {
                // 解析远程玩家数据
                // let (id, name, x, y, direction) = parse_player_packet(data);
                // NetworkSyncSystem::sync_remote_player(&mut self.world.world, id, x, y, direction, MirAction::Standing);
            }
            "ObjectMonster" => {
                // 解析怪物数据
                // let (id, x, y, hp, action) = parse_monster_packet(data);
                // NetworkSyncSystem::sync_monster(&mut self.world.world, id, x, y, hp, action);
            }
            "ObjectRemove" => {
                // 移除实体
                // let entity_id = parse_remove_packet(data);
                // if let Some(entity) = self.world.find_monster(entity_id) {
                //     self.world.despawn(entity);
                // }
            }
            _ => {}
        }
    }
}

// ============================================================================
// 使用示例总结
// ============================================================================
//
// 优势:
// 1. ✅ 实体管理清晰 (Player/Monster/NPC/Spell 都是 Entity + Components)
// 2. ✅ 逻辑解耦 (Movement/Combat/AI 等系统独立)
// 3. ✅ 保留 GGEZ 的简单渲染 (ADD 混合开箱即用)
// 4. ✅ 性能优秀 (hecs 轻量级高性能)
// 5. ✅ 代码更清晰 (比纯 OOP 更易维护)
//
// 对比 Bevy:
// - 学习曲线更平缓 (hecs API 简单)
// - 代码量更少 (没有 Bevy 的样板代码)
// - 渲染更灵活 (GGEZ 直接控制)
// - 功能够用 (对于 2D MMORPG 完全足够)
//
// ============================================================================
