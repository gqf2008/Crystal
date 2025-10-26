// ============================================================================
// 测试辅助工具模块
// ============================================================================

use hecs::World;
use mir2_shared::{MirAction, MirDirection, Point};
use mir2_client::ecs::{NPCData, Animation, Position, Direction};
use mir2_client::ecs::systems::animation::{AnimationSystem, NPCActionSystem};
use mir2_client::objects::frames::{DEFAULT_NPC_FRAMES, get_frame};

/// 测试上下文 - 封装测试所需的所有状态
pub struct TestContext {
    pub world: World,
    pub elapsed_time: u32, // 累积时间(毫秒)
}

impl TestContext {
    /// 创建新的测试上下文
    pub fn new() -> Self {
        Self {
            world: World::new(),
            elapsed_time: 0,
        }
    }
    
    /// 创建测试用NPC
    pub fn create_test_npc(
        &mut self,
        name: &str,
        npc_index: u16,
        location: &Point,
        colour: i32,
    ) -> hecs::Entity {
        let world_x = location.x as f32 * 48.0 + 24.0;
        let world_y = location.y as f32 * 32.0 + 16.0;
        
        // 获取Standing动作的帧配置
        let action = MirAction::Standing;
        let (frame_count, frame_interval) = if let Some(frame) = get_frame(&DEFAULT_NPC_FRAMES, action) {
            (frame.count as u8, frame.interval as u32)
        } else {
            (4, 450)
        };
        
        self.world.spawn((
            Position::new(world_x, world_y),
            Direction::new(MirDirection::Down),
            Animation::new(action, frame_count, frame_interval),
            NPCData {
                id: rand::random::<u32>(),
                name: name.to_string(),
                npc_index,
                dialogue_id: 0,
                colour,
                action_timer: 0,
                next_action_delay: rand::random::<u32>() % 5000 + 3000,
            },
        ))
    }
    
    /// 前进时间并更新所有系统
    pub fn advance_time(&mut self, delta_ms: u32) {
        self.elapsed_time += delta_ms;
        
        // 更新实体动画
        AnimationSystem::update_entities(&mut self.world, delta_ms);
        
        // 更新NPC动作切换
        NPCActionSystem::update(&mut self.world, delta_ms);
    }
    
    /// 获取所有NPC的动作统计
    pub fn get_action_stats(&self) -> ActionStats {
        let mut stats = ActionStats::default();
        
        for (_entity, anim) in self.world.query::<&Animation>().iter() {
            match anim.action {
                MirAction::Standing => stats.standing_count += 1,
                MirAction::Harvest => stats.harvest_count += 1,
                _ => stats.other_count += 1,
            }
        }
        
        stats
    }
    
    /// 获取所有NPC数量
    pub fn npc_count(&self) -> usize {
        self.world.query::<&NPCData>().iter().count()
    }
}

/// 动作统计
#[derive(Debug, Default)]
pub struct ActionStats {
    pub standing_count: usize,
    pub harvest_count: usize,
    pub other_count: usize,
}

impl ActionStats {
    pub fn total(&self) -> usize {
        self.standing_count + self.harvest_count + self.other_count
    }
}

/// 日志捕获器 - 用于验证日志输出
pub struct LogCapture {
    pub logs: Vec<String>,
}

impl LogCapture {
    pub fn new() -> Self {
        Self { logs: Vec::new() }
    }
    
    /// 检查是否包含特定日志
    pub fn contains(&self, pattern: &str) -> bool {
        self.logs.iter().any(|log| log.contains(pattern))
    }
    
    /// 统计匹配的日志数量
    pub fn count_matches(&self, pattern: &str) -> usize {
        self.logs.iter().filter(|log| log.contains(pattern)).count()
    }
}
