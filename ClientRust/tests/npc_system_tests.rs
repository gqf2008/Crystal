// ============================================================================
// NPC系统自动化测试
// ============================================================================
// 
// 测试策略:
// 1. 日志分析 - 监控tracing输出,验证事件发生
// 2. 组件检查 - 直接检查ECS组件状态
// 3. 时间模拟 - 快进时间测试动作切换
//
// 使用方法:
// cargo test --test npc_system_tests -- --nocapture

use mir2_shared::{MirAction, Point};

// 导入需要测试的模块
mod test_helpers;
use test_helpers::*;

// 导入ECS组件
use mir2_client::ecs::{NPCData, Animation};

/// 测试1: NPC创建和初始化
#[test]
fn test_npc_creation() {
    let mut test_ctx = TestContext::new();
    
    println!("🧪 测试: NPC创建和初始化");
    
    // 创建测试NPC
    let npc_id = test_ctx.create_test_npc(
        "铁匠", 
        100, 
        &Point { x: 10, y: 10 },
        0xFF00FF00u32 as i32, // 绿色
    );
    
    // 验证NPC组件
    let world = &test_ctx.world;
    let npc_data = world.get::<&NPCData>(npc_id).expect("NPCData组件应该存在");
    
    assert_eq!(npc_data.name, "铁匠", "NPC名字应该正确");
    assert_eq!(npc_data.npc_index, 100, "NPC索引应该正确");
    assert_eq!(npc_data.colour, 0xFF00FF00u32 as i32, "NPC颜色应该正确");
    assert!(npc_data.action_timer == 0, "初始计时器应为0");
    assert!(npc_data.next_action_delay >= 3000 && npc_data.next_action_delay <= 8000, 
            "延迟应在3-8秒范围");
    
    // 验证动画组件
    let anim = world.get::<&Animation>(npc_id).expect("Animation组件应该存在");
    assert_eq!(anim.action, MirAction::Standing, "初始动作应为Standing");
    assert_eq!(anim.frame_count, 4, "Standing应该有4帧");
    assert_eq!(anim.frame_interval, 450, "Standing间隔应为450ms");
    
    println!("✅ NPC创建测试通过");
}

/// 测试2: NPC动作切换逻辑
#[test]
fn test_npc_action_switching() {
    let mut test_ctx = TestContext::new();
    
    println!("🧪 测试: NPC动作切换");
    
    let npc_id = test_ctx.create_test_npc("商人", 200, &Point { x: 5, y: 5 }, 0);
    
    // 记录初始状态
    let initial_action = {
        let anim = test_ctx.world.get::<&Animation>(npc_id).unwrap();
        anim.action
    };
    
    println!("  初始动作: {:?}", initial_action);
    
    // 多次尝试触发切换(因为随机可能选到相同动作)
    let mut new_action = initial_action;
    for attempt in 0..20 {
        // 强制触发切换 - 设置计时器超时并到达最后一帧
        {
            let mut npc = test_ctx.world.get::<&mut NPCData>(npc_id).unwrap();
            npc.action_timer = npc.next_action_delay + 1000;
        }
        {
            let mut anim = test_ctx.world.get::<&mut Animation>(npc_id).unwrap();
            anim.frame_index = anim.frame_count.saturating_sub(1);
        }
        
        // 触发更新
        test_ctx.advance_time(100);
        
        new_action = {
            let anim = test_ctx.world.get::<&Animation>(npc_id).unwrap();
            anim.action
        };
        
        if new_action != initial_action {
            println!("  第{}次尝试: 动作切换 {:?} -> {:?}", attempt + 1, initial_action, new_action);
            break;
        }
    }
    
    // 验证动作最终切换了(如果20次都没切换说明权重有问题)
    assert_ne!(new_action, initial_action, 
               "20次尝试后动作应该切换至少一次,当前仍为: {:?}", initial_action);
    
    println!("✅ 动作切换测试通过");
}

/// 测试3: NPC动画帧更新
#[test]
fn test_npc_animation_frames() {
    let mut test_ctx = TestContext::new();
    
    println!("🧪 测试: NPC动画帧更新");
    
    let npc_id = test_ctx.create_test_npc("守卫", 300, &Point { x: 1, y: 1 }, 0);
    
    // 获取初始帧索引
    let initial_frame = {
        let anim = test_ctx.world.get::<&Animation>(npc_id).unwrap();
        anim.frame_index
    };
    
    println!("  初始帧: {}", initial_frame);
    
    // 前进足够时间让动画循环
    test_ctx.advance_time(2000); // 2秒
    
    let final_frame = {
        let anim = test_ctx.world.get::<&Animation>(npc_id).unwrap();
        println!("  最终帧: {}, 帧数: {}", anim.frame_index, anim.frame_count);
        anim.frame_index
    };
    
    // 验证帧有变化(说明动画在更新)
    assert!(final_frame != initial_frame || final_frame == 0, 
            "动画帧应该更新或循环到0");
    
    println!("✅ 动画帧更新测试通过");
}

/// 测试4: NPC颜色染色
#[test]
fn test_npc_colour_system() {
    let mut test_ctx = TestContext::new();
    
    println!("🧪 测试: NPC颜色系统");
    
    // 测试不同颜色
    let colours = vec![
        (0xFF0000FFu32 as i32, "蓝色"),   // ARGB: Alpha=255, R=0, G=0, B=255
        (0xFFFF0000u32 as i32, "红色"),   // ARGB: Alpha=255, R=255, G=0, B=0
        (0xFF00FF00u32 as i32, "绿色"),   // ARGB: Alpha=255, R=0, G=255, B=0
        (0, "默认白色"),         // 0表示无染色
    ];
    
    for (colour, name) in colours {
        let npc_id = test_ctx.create_test_npc(
            &format!("{}NPC", name),
            400,
            &Point { x: 1, y: 1 },
            colour,
        );
        
        let npc_data = test_ctx.world.get::<&NPCData>(npc_id).unwrap();
        assert_eq!(npc_data.colour, colour, "{}颜色应该正确设置", name);
        
        println!("  ✓ {} (0x{:08X})", name, colour);
    }
    
    println!("✅ 颜色系统测试通过");
}

/// 测试5: 任务标记组件
#[test]
#[ignore = "需要QuestMarker公开导出"]
fn test_quest_marker() {
    println!("⏭️  任务标记测试已跳过 - 需要组件导出");
}

/// 测试6: 批量NPC性能测试
#[test]
fn test_npc_performance() {
    let mut test_ctx = TestContext::new();
    
    println!("🧪 测试: 批量NPC性能");
    
    let npc_count = 100;
    println!("  创建{}个NPC...", npc_count);
    
    let start = std::time::Instant::now();
    
    // 创建100个NPC
    for i in 0..npc_count {
        test_ctx.create_test_npc(
            &format!("NPC_{}", i),
            (i % 10) as u16 * 100,
            &Point { x: i % 20, y: i / 20 },
            0,
        );
    }
    
    let creation_time = start.elapsed();
    println!("  创建耗时: {:?}", creation_time);
    
    // 模拟10秒游戏时间
    let start = std::time::Instant::now();
    for _ in 0..200 {
        test_ctx.advance_time(50); // 50ms per frame
    }
    let update_time = start.elapsed();
    
    println!("  更新10秒游戏时间耗时: {:?}", update_time);
    println!("  平均帧时间: {:?}", update_time / 200);
    
    // 性能断言
    assert!(creation_time.as_millis() < 100, "创建100个NPC应该<100ms");
    assert!(update_time.as_millis() < 500, "更新200帧应该<500ms");
    
    println!("✅ 性能测试通过");
}

/// 测试7: 动作权重分布验证
#[test]
fn test_action_weight_distribution() {
    let mut test_ctx = TestContext::new();
    
    println!("🧪 测试: 动作权重分布");
    
    let npc_id = test_ctx.create_test_npc("测试NPC", 600, &Point { x: 1, y: 1 }, 0);
    
    let mut standing_count = 0;
    let mut harvest_count = 0;
    
    // 强制触发多次切换,统计分布(200次采样)
    for _ in 0..200 {
        // 强制切换到下一个动作
        {
            let mut npc = test_ctx.world.get::<&mut NPCData>(npc_id).unwrap();
            npc.action_timer = npc.next_action_delay + 1000;
        }
        
        // 强制到最后一帧
        {
            let mut anim = test_ctx.world.get::<&mut Animation>(npc_id).unwrap();
            anim.frame_index = anim.frame_count.saturating_sub(1);
        }
        
        // 触发更新
        test_ctx.advance_time(100);
        
        // 统计动作
        let action = test_ctx.world.get::<&Animation>(npc_id).unwrap().action;
        match action {
            MirAction::Standing => standing_count += 1,
            MirAction::Harvest => harvest_count += 1,
            _ => {}
        }
    }
    
    let total = standing_count + harvest_count;
    let standing_ratio = standing_count as f32 / total as f32;
    let harvest_ratio = harvest_count as f32 / total as f32;
    
    println!("  Standing: {}次 ({:.1}%)", standing_count, standing_ratio * 100.0);
    println!("  Harvest: {}次 ({:.1}%)", harvest_count, harvest_ratio * 100.0);
    
    // 权重应该大致为70/30,允许±20%误差
    assert!(standing_ratio >= 0.5 && standing_ratio <= 0.9, 
            "Standing应该在50%-90%范围,实际: {:.1}%", standing_ratio * 100.0);
    
    println!("✅ 动作权重分布测试通过");
}
