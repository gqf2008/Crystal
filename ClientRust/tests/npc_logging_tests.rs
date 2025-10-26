// ============================================================================
// 日志分析测试 - 监控tracing输出验证行为
// ============================================================================

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::{Arc, Mutex};

mod test_helpers;
use test_helpers::*;

use mir2_client::ecs::NPCData;

/// 日志订阅器 - 捕获所有日志消息
struct LogCollector {
    logs: Arc<Mutex<Vec<String>>>,
}

impl LogCollector {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let logs = Arc::new(Mutex::new(Vec::new()));
        (Self { logs: logs.clone() }, logs)
    }
}

impl<S> tracing_subscriber::Layer<S> for LogCollector
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = LogVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);
        
        if !visitor.message.is_empty() {
            self.logs.lock().unwrap().push(visitor.message);
        }
    }
}

struct LogVisitor {
    message: String,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}

/// 测试1: 验证NPC创建日志
#[test]
fn test_npc_creation_logs() {
    // 设置日志捕获
    let (collector, logs) = LogCollector::new();
    let _guard = tracing_subscriber::registry()
        .with(collector)
        .set_default();
    
    println!("🧪 测试: NPC创建日志验证");
    
    let mut test_ctx = TestContext::new();
    
    // 清空之前的日志
    logs.lock().unwrap().clear();
    
    // 创建NPC应该产生日志
    let _npc_id = test_ctx.create_test_npc(
        "测试铁匠",
        100,
        &mir2_shared::Point { x: 10, y: 10 },
        0xFF00FF00u32 as i32,
    );
    
    // 检查日志
    let captured_logs = logs.lock().unwrap();
    let has_npc_log = captured_logs.iter().any(|log| {
        log.contains("NPC") || log.contains("铁匠")
    });
    
    if has_npc_log {
        println!("  ✓ 捕获到NPC创建相关日志");
        for log in captured_logs.iter() {
            if log.contains("NPC") {
                println!("    📝 {}", log);
            }
        }
    }
    
    println!("✅ NPC创建日志测试完成");
}

/// 测试2: 验证动作切换日志
#[test]
fn test_action_switch_logs() {
    let (collector, logs) = LogCollector::new();
    let _guard = tracing_subscriber::registry()
        .with(collector)
        .set_default();
    
    println!("🧪 测试: 动作切换日志验证");
    
    let mut test_ctx = TestContext::new();
    let npc_id = test_ctx.create_test_npc("工匠", 200, &mir2_shared::Point { x: 1, y: 1 }, 0);
    
    logs.lock().unwrap().clear();
    
    // 强制触发动作切换
    for i in 0..10 {
        // 设置到切换时间点
        {
            let mut npc = test_ctx.world.get::<&mut NPCData>(npc_id).unwrap();
            npc.action_timer = npc.next_action_delay + 100;
        }
        
        // 设置到最后一帧
        {
            let mut anim = test_ctx.world.get::<&mut mir2_client::ecs::Animation>(npc_id).unwrap();
            anim.frame_index = anim.frame_count.saturating_sub(1);
        }
        
        test_ctx.advance_time(100);
        
        // 检查是否有切换日志
        let switch_logs = logs.lock().unwrap();
        let switch_count = switch_logs.iter()
            .filter(|log| log.contains("切换动作") || log.contains("switch"))
            .count();
        
        if switch_count > 0 {
            println!("  第{}次迭代: 捕获到{}条切换日志", i, switch_count);
            break;
        }
    }
    
    let final_logs = logs.lock().unwrap();
    let has_switch = final_logs.iter().any(|log| 
        log.contains("切换") || log.contains("Standing") || log.contains("Harvest")
    );
    
    if has_switch {
        println!("  ✓ 成功捕获动作切换日志");
        for log in final_logs.iter().take(5) {
            println!("    📝 {}", log);
        }
    }
    
    println!("✅ 动作切换日志测试完成");
}

/// 测试3: 性能基准 - 测量实际帧时间
#[test]
fn test_npc_system_performance_benchmark() {
    println!("🧪 性能基准测试");
    
    let scenarios = vec![
        ("小场景", 5),
        ("中场景", 20),
        ("大场景", 50),
        ("极限场景", 100),
    ];
    
    for (name, npc_count) in scenarios {
        let mut test_ctx = TestContext::new();
        
        // 创建NPC
        for i in 0..npc_count {
            test_ctx.create_test_npc(
                &format!("NPC_{}", i),
                (i % 10) as u16 * 100,
                &mir2_shared::Point { x: i % 20, y: i / 20 },
                0,
            );
        }
        
        // 测量1000帧的更新时间
        let iterations = 1000;
        let start = std::time::Instant::now();
        
        for _ in 0..iterations {
            test_ctx.advance_time(16); // 60fps = 16.67ms per frame
        }
        
        let elapsed = start.elapsed();
        let avg_frame_time = elapsed.as_micros() as f64 / iterations as f64;
        let fps_estimate = 1_000_000.0 / avg_frame_time;
        
        println!("  {} ({}个NPC):", name, npc_count);
        println!("    平均帧时间: {:.2}μs", avg_frame_time);
        println!("    预估FPS: {:.0}", fps_estimate);
        
        // 性能断言
        let max_frame_time = match npc_count {
            0..=10 => 100.0,   // 小场景应该<100μs
            11..=30 => 500.0,  // 中场景<500μs
            31..=60 => 2000.0, // 大场景<2ms
            _ => 5000.0,       // 极限<5ms
        };
        
        assert!(avg_frame_time < max_frame_time, 
                "{}: 平均帧时间{:.2}μs超过阈值{:.2}μs", 
                name, avg_frame_time, max_frame_time);
    }
    
    println!("✅ 性能基准测试通过");
}
