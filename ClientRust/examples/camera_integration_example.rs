/// # 相机系统集成示例
/// 
/// 演示如何将 Camera 集成到游戏场景中
///
/// 运行: cargo run --example camera_integration_example

use std::time::Duration;

// 假设的导入 (实际需根据项目结构调整)
// use crystal::ecs::coordinate_system::{CoordinateSystem, ViewportConfig, Camera, MapUtils};
// use crystal::ecs::components::Position;

// ====== 示例1: 基础集成 ======

pub struct GameScene {
    camera: Camera,
    player_position: (f32, f32),
    delta_time: f32,
}

impl GameScene {
    pub fn new() -> Self {
        // 1. 创建视野配置 (屏幕分辨率)
        let viewport = ViewportConfig::new(1024.0, 768.0);
        
        // 2. 创建坐标系统
        let coord_system = CoordinateSystem::new(viewport);
        
        // 3. 创建相机
        let mut camera = Camera::new(coord_system);
        
        // 4. 设置地图边界 (假设 500x500 的地图)
        camera.set_map_bounds(0, 0, 499, 499);
        
        // 5. 设置跟随平滑度 (0.15 = 较平滑)
        camera.set_follow_smoothness(0.15);
        
        Self {
            camera,
            player_position: (1200.0, 1600.0), // 玩家初始世界坐标
            delta_time: 1.0 / 60.0,             // 60 FPS
        }
    }
    
    pub fn update(&mut self) {
        // 每帧更新相机 (自动跟随玩家)
        self.camera.update(self.delta_time, self.player_position);
    }
    
    pub fn render(&self) {
        // 获取相机最终位置 (包含震动偏移)
        let camera_pos = self.camera.get_final_position();
        
        println!("相机位置: {:?}", camera_pos);
        
        // 渲染所有可见实体
        for entity_pos in self.get_all_entities() {
            if self.camera.is_visible(entity_pos) {
                let screen_pos = self.camera.world_to_screen(entity_pos);
                // 在 screen_pos 渲染实体...
                println!("  实体在屏幕: {:?}", screen_pos);
            }
        }
    }
    
    fn get_all_entities(&self) -> Vec<(f32, f32)> {
        // 模拟获取所有实体位置
        vec![
            (1200.0, 1600.0), // 玩家
            (1250.0, 1650.0), // 怪物1
            (1100.0, 1550.0), // 怪物2
        ]
    }
}

// ====== 示例2: 战斗系统集成 ======

pub struct CombatSystem {
    camera: Camera,
}

impl CombatSystem {
    pub fn on_player_attack(&mut self) {
        // 普通攻击: 轻微震动
        self.camera.shake(3.0, 0.2);
    }
    
    pub fn on_critical_hit(&mut self) {
        // 暴击: 中等震动
        self.camera.shake(8.0, 0.4);
    }
    
    pub fn on_skill_cast(&mut self) {
        // 技能释放: 强烈震动
        self.camera.shake(12.0, 0.6);
    }
    
    pub fn on_boss_death(&mut self) {
        // Boss 死亡: 超强震动
        self.camera.shake(20.0, 1.0);
    }
}

// ====== 示例3: 过场动画 ======

pub struct CutsceneManager {
    camera: Camera,
    player_pos: (f32, f32),
}

impl CutsceneManager {
    /// 开场动画: 相机从地图中心飞向玩家
    pub fn play_intro(&mut self) {
        println!("播放开场动画...");
        
        // 1. 相机立即跳转到地图中心
        let map_center = (12000.0, 16000.0); // 假设地图中心
        self.camera.jump_to(map_center);
        
        // 2. 平滑过渡到玩家位置 (3秒)
        self.camera.transition_to(self.player_pos, 3.0);
        
        println!("相机将在3秒内移动到玩家位置");
    }
    
    /// Boss 登场: 相机快速移动到Boss位置
    pub fn boss_appears(&mut self, boss_pos: (f32, f32)) {
        println!("Boss登场!");
        
        // 1. 切换到自由模式 (不跟随玩家)
        self.camera.set_free_mode();
        
        // 2. 相机移动到Boss (2秒)
        self.camera.transition_to(boss_pos, 2.0);
        
        // 3. 镜头震动增强效果
        self.camera.shake(15.0, 0.8);
        
        // 4. 2秒后回到跟随模式 (实际需要用定时器)
        // self.schedule_callback(2.0, || self.camera.set_follow_mode());
    }
}

// ====== 示例4: 技能系统集成 ======

pub struct SkillSystem {
    camera: Camera,
}

impl SkillSystem {
    /// 传送技能: 相机跟随玩家瞬移
    pub fn teleport(&mut self, from: (f32, f32), to: (f32, f32)) {
        println!("玩家传送: {:?} -> {:?}", from, to);
        
        // 选项1: 立即跳转 (硬切)
        // self.camera.jump_to(to);
        
        // 选项2: 快速过渡 (0.3秒)
        self.camera.transition_to(to, 0.3);
        
        // 传送震动效果
        self.camera.shake(10.0, 0.4);
    }
    
    /// 火球术: 相机短暂跟踪火球
    pub fn cast_fireball(&mut self, target: (f32, f32)) {
        println!("释放火球术!");
        
        // 1. 释放时震动
        self.camera.shake(5.0, 0.3);
        
        // 2. 临时跟踪火球轨迹 (实际需要更复杂的逻辑)
        // self.camera.set_free_mode();
        // self.camera.transition_to(target, 0.5);
    }
}

// ====== 示例5: 调试模式 ======

pub struct DebugCamera {
    camera: Camera,
    free_mode: bool,
}

impl DebugCamera {
    pub fn toggle_free_camera(&mut self) {
        self.free_mode = !self.free_mode;
        
        if self.free_mode {
            println!("切换到自由相机模式");
            self.camera.set_free_mode();
        } else {
            println!("切换回跟随模式");
            self.camera.set_follow_mode();
        }
    }
    
    pub fn handle_keyboard(&mut self, key: &str) {
        if !self.free_mode {
            return;
        }
        
        let move_speed = 10.0;
        let (x, y) = self.camera.position;
        
        match key {
            "w" | "up" => self.camera.position = (x, y - move_speed),
            "s" | "down" => self.camera.position = (x, y + move_speed),
            "a" | "left" => self.camera.position = (x - move_speed, y),
            "d" | "right" => self.camera.position = (x + move_speed, y),
            _ => {}
        }
    }
}

// ====== 示例6: 鼠标交互 ======

pub struct MouseHandler {
    camera: Camera,
}

impl MouseHandler {
    /// 处理鼠标点击 (例如: 地面点击移动)
    pub fn on_mouse_click(&self, screen_x: f32, screen_y: f32) {
        // 屏幕坐标转换为世界坐标
        let world_pos = self.camera.screen_to_world((screen_x, screen_y));
        
        // 世界坐标转换为格子坐标
        let grid_pos = CoordinateSystem::world_to_grid(world_pos.0, world_pos.1);
        
        println!("鼠标点击: 屏幕({}, {}) -> 世界({:?}) -> 格子({:?})", 
                 screen_x, screen_y, world_pos, grid_pos);
        
        // 发送移动指令...
    }
    
    /// 处理鼠标悬停 (例如: 显示怪物信息)
    pub fn on_mouse_hover(&self, screen_x: f32, screen_y: f32) {
        let world_pos = self.camera.screen_to_world((screen_x, screen_y));
        
        // 检查该位置是否有实体
        if let Some(entity) = self.get_entity_at(world_pos) {
            println!("鼠标悬停在实体上: {:?}", entity);
        }
    }
    
    fn get_entity_at(&self, world_pos: (f32, f32)) -> Option<String> {
        // 模拟检查实体
        Some("怪物 (等级10)".to_string())
    }
}

// ====== 示例7: 性能优化 - 视野剔除 ======

pub struct RenderOptimizer {
    camera: Camera,
}

impl RenderOptimizer {
    /// 只渲染可见对象 (视野剔除)
    pub fn render_visible_objects(&self, all_monsters: &[Monster]) {
        let mut rendered = 0;
        let mut culled = 0;
        
        for monster in all_monsters {
            if self.camera.is_visible(monster.position) {
                // 渲染怪物
                self.render_monster(monster);
                rendered += 1;
            } else {
                // 剔除不可见对象
                culled += 1;
            }
        }
        
        println!("渲染: {} 个对象, 剔除: {} 个对象", rendered, culled);
    }
    
    fn render_monster(&self, monster: &Monster) {
        let screen_pos = self.camera.world_to_screen(monster.position);
        println!("  渲染怪物: {} 在屏幕 {:?}", monster.name, screen_pos);
    }
}

// 辅助结构
pub struct Monster {
    name: String,
    position: (f32, f32),
}

// ====== 示例8: 完整游戏循环 ======

pub struct Game {
    camera: Camera,
    player_pos: (f32, f32),
    monsters: Vec<Monster>,
    accumulated_time: f32,
}

impl Game {
    pub fn new() -> Self {
        let viewport = ViewportConfig::new(1920.0, 1080.0);
        let coord_system = CoordinateSystem::new(viewport);
        let mut camera = Camera::new(coord_system);
        
        camera.set_map_bounds(0, 0, 500, 500);
        camera.set_follow_smoothness(0.2);
        
        Self {
            camera,
            player_pos: (12000.0, 16000.0),
            monsters: vec![
                Monster { name: "史莱姆".to_string(), position: (12100.0, 16050.0) },
                Monster { name: "骷髅".to_string(), position: (11900.0, 15950.0) },
                Monster { name: "Boss".to_string(), position: (15000.0, 20000.0) },
            ],
            accumulated_time: 0.0,
        }
    }
    
    pub fn update(&mut self, delta_time: f32) {
        self.accumulated_time += delta_time;
        
        // 模拟玩家移动
        self.player_pos.0 += 50.0 * delta_time;
        
        // 更新相机
        self.camera.update(delta_time, self.player_pos);
        
        // 每3秒触发一次震动 (模拟战斗)
        if self.accumulated_time > 3.0 {
            self.camera.shake(8.0, 0.4);
            self.accumulated_time = 0.0;
            println!("战斗震动!");
        }
    }
    
    pub fn render(&self) {
        println!("\n==== 渲染帧 ====");
        println!("玩家位置: {:?}", self.player_pos);
        println!("相机位置: {:?}", self.camera.get_final_position());
        
        // 渲染可见怪物
        for monster in &self.monsters {
            if self.camera.is_visible(monster.position) {
                let screen_pos = self.camera.world_to_screen(monster.position);
                println!("  {} 在屏幕 {:?}", monster.name, screen_pos);
            }
        }
    }
    
    pub fn run(&mut self, frames: u32) {
        println!("开始游戏循环 (模拟 {} 帧)...\n", frames);
        
        let delta_time = 1.0 / 60.0; // 60 FPS
        
        for frame in 0..frames {
            println!("--- 帧 {} ---", frame);
            self.update(delta_time);
            
            if frame % 30 == 0 {
                self.render(); // 每30帧渲染一次 (实际每帧都渲染)
            }
        }
        
        println!("\n游戏循环结束");
    }
}

// ====== 主函数 ======

fn main() {
    println!("=== 相机系统集成示例 ===\n");
    
    // 示例1: 基础集成
    println!("【示例1: 基础集成】");
    let mut scene = GameScene::new();
    scene.update();
    scene.render();
    
    println!("\n");
    
    // 示例2: 完整游戏循环
    println!("【示例2: 完整游戏循环】");
    let mut game = Game::new();
    game.run(180); // 模拟3秒 (60 FPS * 3)
}

// 注意: 这些示例需要实际的依赖才能编译运行
// 实际集成时需要根据项目结构调整导入路径
