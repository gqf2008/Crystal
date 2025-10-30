// ============================================================================
// 并行调度器性能基准测试
// ============================================================================
//
// 对比测试:
// 1. 串行执行 (Sequential) vs 并行执行 (Parallel)
// 2. 不同实体数量 (100, 1000, 10000)
// 3. Layer 3/4/5 的并行加速比
//
// ============================================================================

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hecs::World;
use mir2_client::ecs::{
    ParallelScheduler, ExecutionMode,
    components::*,
};

/// 创建测试 World，填充不同类型的实体 (使用真实组件定义)
fn create_test_world(entity_count: usize) -> (World, Option<MapData>) {
    use std::time::{Instant, Duration};
    
    let mut world = World::new();
    
    // 添加 MapData (LocalPredictionSystem 需要)
    let map_data = MapData {
        width: 100,
        height: 100,
        cells: vec![],
    };
    world.spawn((map_data.clone(),));
    
    // 创建玩家实体 (使用真实 Player 组件)
    for i in 0..entity_count {
        world.spawn((
            Position { x: i as f32 * 10.0, y: i as f32 * 10.0 },
            Player {
                direction: 0,
                action: PlayerAction::Stand,
                frame_index: 0,
                frame_time: 0,
                speed: 1.0,
                target_x: i as f32 * 10.0,
                target_y: i as f32 * 10.0,
                is_moving: false,
                path: vec![],
                path_index: 0,
                move_mode: MoveMode::Idle,
                last_move_time: Instant::now(),
                move_delay: Duration::from_millis(600),
                waiting_server_confirm: false,
                collision_detected: false,
                collision_target_grid: None,
                can_run: false,
                last_run_time: Instant::now(),
                run_cooldown: Duration::from_millis(900),
            },
        ));
    }
    
    (world, Some(map_data))
}

/// Benchmark: 串行执行
fn bench_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_execution");
    
    for entity_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let (mut world, _) = create_test_world(count);
                let mut scheduler = ParallelScheduler::new(ExecutionMode::Sequential);
                
                // 创建伪 Context（无法在 benchmark 中创建真实 ggez::Context）
                // 实际测试中会跳过需要 Context 的系统
                
                b.iter(|| {
                    // 只测试 Layer 2-5（跳过 Layer 1 的 Input/Network）
                    // 注意：这里无法创建真实的 ggez::Context，所以跳过需要它的系统
                    
                    // 模拟系统执行（直接调用系统而不是通过调度器）
                    use mir2_client::ecs::systems::*;
                    
                    // Layer 2
                    MovementSystemV2::update(&mut world, 0.016);
                    ReconciliationSystem::update(&mut world, 0.016);
                    InterpolationSystem::update(&mut world, 0.016);
                    
                    // Layer 3
                    AnimationStateSystem::update(&mut world, 0.016);
                    MonsterAnimationStateSystem::update(&mut world);
                    NPCActionSystem::update(&mut world, 16);
                    
                    // Layer 4
                    TileAnimationSystem::update(&mut world, 1);
                    AnimationPlaybackSystem::update(&mut world, 16);
                    MovementInterpolationSystem::update(&mut world);
                    
                    // Layer 5
                    MouseEventSystem::update_mouse_input(&mut world);
                    MonsterSystem::update(&mut world, 0.016);
                    CameraSystem::update(&mut world);
                    
                    black_box(&world);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark: 并行执行
fn bench_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_execution");
    
    for entity_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let (mut world, _) = create_test_world(count);
                
                b.iter(|| {
                    // 模拟并行执行
                    use mir2_client::ecs::systems::*;
                    use parking_lot::RwLock;
                    
                    // Layer 2 (串行)
                    MovementSystemV2::update(&mut world, 0.016);
                    ReconciliationSystem::update(&mut world, 0.016);
                    InterpolationSystem::update(&mut world, 0.016);
                    
                    // Layer 3 (并行)
                    let world_lock = RwLock::new(&mut world);
                    rayon::scope(|s| {
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            AnimationStateSystem::update(&mut **w, 0.016);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            MonsterAnimationStateSystem::update(&mut **w);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            NPCActionSystem::update(&mut **w, 16);
                        });
                    });
                    let world = world_lock.into_inner();
                    
                    // Layer 4 (并行)
                    let world_lock = RwLock::new(world);
                    rayon::scope(|s| {
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            TileAnimationSystem::update(&mut **w, 1);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            AnimationPlaybackSystem::update(&mut **w, 16);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            MovementInterpolationSystem::update(&mut **w);
                        });
                    });
                    let world = world_lock.into_inner();
                    
                    // Layer 5 (并行)
                    let world_lock = RwLock::new(world);
                    rayon::scope(|s| {
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            MouseEventSystem::update_mouse_input(&mut **w);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            MonsterSystem::update(&mut **w, 0.016);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            CameraSystem::update(&mut **w);
                        });
                    });
                    let world = world_lock.into_inner();
                    
                    black_box(world);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark: 并行加速比 (Speedup)
fn bench_speedup(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_speedup");
    group.sample_size(10); // 减少样本数量，因为 10000 实体测试较慢
    
    for entity_count in [1000, 10000].iter() {
        // Sequential baseline
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            entity_count,
            |b, &count| {
                let (mut world, _) = create_test_world(count);
                
                b.iter(|| {
                    use mir2_client::ecs::systems::*;
                    
                    MovementSystemV2::update(&mut world, 0.016);
                    AnimationStateSystem::update(&mut world, 0.016);
                    MonsterAnimationStateSystem::update(&mut world);
                    NPCActionSystem::update(&mut world, 16);
                    TileAnimationSystem::update(&mut world, 1);
                    AnimationPlaybackSystem::update(&mut world, 16);
                    
                    black_box(&world);
                });
            },
        );
        
        // Parallel version
        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            entity_count,
            |b, &count| {
                let (mut world, _) = create_test_world(count);
                
                b.iter(|| {
                    use mir2_client::ecs::systems::*;
                    use parking_lot::RwLock;
                    
                    MovementSystemV2::update(&mut world, 0.016);
                    
                    let world_lock = RwLock::new(&mut world);
                    rayon::scope(|s| {
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            AnimationStateSystem::update(&mut **w, 0.016);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            MonsterAnimationStateSystem::update(&mut **w);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            NPCActionSystem::update(&mut **w, 16);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            TileAnimationSystem::update(&mut **w, 1);
                        });
                        s.spawn(|_| {
                            let mut w = world_lock.write();
                            AnimationPlaybackSystem::update(&mut **w, 16);
                        });
                    });
                    
                    black_box(world_lock.into_inner());
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_sequential, bench_parallel, bench_speedup);
criterion_main!(benches);
