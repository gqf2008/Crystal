// ============================================================================
// ECS集成测试 - 验证SystemScheduler核心功能
// ============================================================================

use hecs::World;
use mir2_client::ecs::{
    SystemScheduler,
    components::{
        core::*,
        map::*,
        movement::*,
    },
};

/// 测试1: SystemScheduler基础功能
#[test]
fn test_scheduler_basic_execution() {
    let mut scheduler = SystemScheduler::new();
    let mut world = World::new();
    
    // 执行多帧
    for _ in 0..10 {
        scheduler.update(&mut world, 0.016).expect("系统执行失败");
    }
    
    // 验证所有系统都被执行
    let stats = scheduler.get_all_stats();
    assert_eq!(stats.len(), 15, "应该有15个系统");
    
    for stat in stats {
        assert_eq!(stat.execution_count, 10, "{} 应该执行10次", stat.name);
    }
    
    println!("✅ SystemScheduler基础执行测试通过");
}

/// 测试2: 系统执行顺序验证
#[test]
fn test_system_execution_order() {
    let mut scheduler = SystemScheduler::new();
    let mut world = World::new();
    
    // 执行一帧
    scheduler.update(&mut world, 0.016).expect("系统执行失败");
    
    // 验证系统按优先级执行
    let stats = scheduler.get_all_stats();
    
    // 找到PlayerControlSystem和MovementSystem
    let player_control_stats = stats.iter()
        .find(|s| s.name == "PlayerControlSystem")
        .expect("找不到PlayerControlSystem");
    let movement_stats = stats.iter()
        .find(|s| s.name == "MovementSystem")
        .expect("找不到MovementSystem");
    
    // 验证优先级: PlayerControlSystem(110) < MovementSystem(400)
    assert!(player_control_stats.priority < movement_stats.priority,
        "PlayerControlSystem应该先于MovementSystem执行");
    
    println!("✅ 系统执行顺序测试通过");
    println!("   PlayerControlSystem优先级: {}", player_control_stats.priority);
    println!("   MovementSystem优先级: {}", movement_stats.priority);
}

/// 测试3: 碰撞系统边界检测
#[test]
fn test_collision_boundary() {
    let mut scheduler = SystemScheduler::new();
    let mut world = World::new();
    
    // 创建实体在边界附近
    let entity = world.spawn((
        Position { x: 990.0, y: 500.0 },
        MovementVelocity::new(200.0),
        MapBounds { width: 1000, height: 1000 },
    ));
    
    // 设置向右移动的速度
    {
        let mut vel = world.get::<&mut MovementVelocity>(entity).unwrap();
        vel.set(100.0, 0.0);
    }
    
    // 执行多帧让实体移动到边界外
    for _ in 0..20 {
        scheduler.update(&mut world, 0.016).expect("系统执行失败");
    }
    
    // 验证位置被限制在边界内
    let final_pos = world.get::<&Position>(entity).unwrap();
    assert!(final_pos.x < 1000.0, "X位置应该被限制在边界内");
    assert!(final_pos.x >= 0.0, "X位置应该 >= 0");
    
    println!("✅ 碰撞边界检测测试通过: x={}", final_pos.x);
}

/// 测试4: 系统启用/禁用
#[test]
fn test_system_enable_disable() {
    let mut scheduler = SystemScheduler::new();
    let mut world = World::new();
    
    // 禁用MovementSystem
    scheduler.disable_system("MovementSystem");
    
    // 执行10帧
    for _ in 0..10 {
        scheduler.update(&mut world, 0.016).expect("系统执行失败");
    }
    
    // 验证MovementSystem未执行
    let movement_stats = scheduler.get_stats("MovementSystem").unwrap();
    assert_eq!(movement_stats.execution_count, 0, "MovementSystem应该未执行");
    
    // 验证其他系统正常执行
    let collision_stats = scheduler.get_stats("CollisionSystem").unwrap();
    assert_eq!(collision_stats.execution_count, 10, "CollisionSystem应该执行10次");
    
    // 重新启用
    scheduler.enable_system("MovementSystem");
    scheduler.update(&mut world, 0.016).expect("系统执行失败");
    
    let movement_stats = scheduler.get_stats("MovementSystem").unwrap();
    assert_eq!(movement_stats.execution_count, 1, "MovementSystem应该执行1次");
    
    println!("✅ 系统启用/禁用测试通过");
}

/// 测试5: 性能基准 - 100个实体
#[test]
fn test_performance_benchmark_100_entities() {
    let mut scheduler = SystemScheduler::new();
    let mut world = World::new();
    
    // 创建100个实体
    for i in 0..100 {
        world.spawn((
            Position { 
                x: (i % 10) as f32 * 100.0, 
                y: (i / 10) as f32 * 100.0 
            },
            MovementVelocity::new(100.0),
            MapBounds { width: 1000, height: 1000 },
        ));
    }
    
    // 测量100帧的执行时间
    let start = std::time::Instant::now();
    for _ in 0..100 {
        scheduler.update(&mut world, 0.016).expect("系统执行失败");
    }
    let elapsed = start.elapsed();
    
    let avg_frame_time = elapsed.as_micros() / 100;
    let target_frame_time = 16_666; // 16.666ms @ 60fps
    
    println!("\n=== 性能基准测试 (100实体) ===");
    println!("实体数量: 100");
    println!("测试帧数: 100");
    println!("总耗时: {:?}", elapsed);
    println!("平均帧时间: {}μs", avg_frame_time);
    println!("目标帧时间: {}μs (60fps)", target_frame_time);
    println!("性能余量: {:.1}%", 
        (1.0 - avg_frame_time as f64 / target_frame_time as f64) * 100.0);
    
    // 打印系统性能报告
    scheduler.print_performance_report();
    
    // 验证性能满足60fps
    assert!(
        avg_frame_time < target_frame_time,
        "平均帧时间 {}μs 应小于 {}μs (60fps)",
        avg_frame_time,
        target_frame_time
    );
    
    println!("✅ 性能基准测试通过");
}

/// 测试6: 统计信息重置
#[test]
fn test_stats_reset() {
    let mut scheduler = SystemScheduler::new();
    let mut world = World::new();
    
    // 执行几帧
    for _ in 0..5 {
        scheduler.update(&mut world, 0.016).expect("系统执行失败");
    }
    
    // 验证统计
    let stats_before = scheduler.get_stats("MovementSystem").unwrap();
    assert_eq!(stats_before.execution_count, 5);
    
    // 重置
    scheduler.reset_stats();
    
    let stats_after = scheduler.get_stats("MovementSystem").unwrap();
    assert_eq!(stats_after.execution_count, 0);
    assert_eq!(stats_after.total_time.as_nanos(), 0);
    
    println!("✅ 统计重置测试通过");
}

/// 测试7: 压力测试 - 1000个实体
#[test]
#[ignore] // 默认跳过，使用 cargo test -- --ignored 运行
fn test_stress_1000_entities() {
    let mut scheduler = SystemScheduler::new();
    let mut world = World::new();
    
    println!("创建1000个实体...");
    for i in 0..1000 {
        world.spawn((
            Position { 
                x: (i % 32) as f32 * 50.0, 
                y: (i / 32) as f32 * 50.0 
            },
            MovementVelocity::new(100.0),
            MapBounds { width: 2000, height: 2000 },
        ));
    }
    
    println!("执行100帧压力测试...");
    let start = std::time::Instant::now();
    for frame in 0..100 {
        scheduler.update(&mut world, 0.016).expect("系统执行失败");
        
        if frame % 10 == 0 {
            let elapsed = start.elapsed();
            let avg = elapsed.as_micros() / (frame + 1) as u128;
            println!("帧 {}: 平均帧时间 {}μs", frame, avg);
        }
    }
    
    let elapsed = start.elapsed();
    let avg_frame_time = elapsed.as_micros() / 100;
    
    println!("\n=== 压力测试结果 ===");
    println!("实体数量: 1000");
    println!("测试帧数: 100");
    println!("总耗时: {:?}", elapsed);
    println!("平均帧时间: {}μs", avg_frame_time);
    
    scheduler.print_performance_report();
    
    // 即使1000个实体，也应该在33ms内完成（30fps）
    assert!(
        avg_frame_time < 33_333,
        "1000实体平均帧时间 {}μs 应小于 33333μs (30fps)",
        avg_frame_time
    );
    
    println!("✅ 1000实体压力测试通过");
}
