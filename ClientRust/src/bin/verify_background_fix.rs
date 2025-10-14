/// 自动化验证工具：检查 GameScene 背景清理修复
/// 
/// 运行方式：
/// ```
/// cargo run --bin verify_background_fix
/// ```

use std::fs;
use std::path::Path;

fn main() {
    println!("\n🧪 ========== GameScene 背景清理自动化验证 ==========\n");
    
    let mut all_passed = true;
    
    // 1. 检查源代码修复
    println!("📝 检查代码修复...");
    if check_code_fix() {
        println!("✅ 代码修复已应用：发现背景清理代码\n");
    } else {
        println!("❌ 代码修复未应用：缺少背景清理代码\n");
        all_passed = false;
    }
    
    // 2. 检查文件结构
    println!("📁 检查文件结构...");
    if check_file_structure() {
        println!("✅ 文件结构完整\n");
    } else {
        println!("❌ 文件结构不完整\n");
        all_passed = false;
    }
    
    // 3. 代码质量检查
    println!("🔍 代码质量检查...");
    check_code_quality();
    println!();
    
    // 4. 生成报告
    println!("📊 生成测试报告...");
    generate_report(all_passed);
    
    // 5. 提供建议
    println!("\n🎯 下一步操作:");
    println!("1. 运行游戏进行手动测试:");
    println!("   cargo run --bin mir2_client");
    println!();
    println!("2. 观察游戏场景背景:");
    println!("   - 应该看到黑色背景");
    println!("   - 不应该看到登录界面残留");
    println!();
    
    if all_passed {
        println!("========================================");
        println!("✅ 自动化验证完成：所有检查通过！");
        println!("========================================\n");
    } else {
        println!("========================================");
        println!("❌ 自动化验证失败：请检查上述错误");
        println!("========================================\n");
        std::process::exit(1);
    }
}

/// 检查代码修复是否已应用
fn check_code_fix() -> bool {
    let game_scene_path = "src/scenes/game_scene.rs";
    
    if let Ok(content) = fs::read_to_string(game_scene_path) {
        // 检查关键代码
        let has_background_clear = content.contains("绘制全屏黑色背景") 
            || content.contains("清空画布");
        let has_mesh_rectangle = content.contains("Mesh::new_rectangle");
        let has_color_black = content.contains("Color::BLACK");
        
        if has_background_clear && has_mesh_rectangle && has_color_black {
            println!("   ✓ 发现背景清理注释");
            println!("   ✓ 发现 Mesh::new_rectangle 调用");
            println!("   ✓ 发现 Color::BLACK 使用");
            return true;
        } else {
            if !has_background_clear {
                println!("   ✗ 缺少背景清理注释");
            }
            if !has_mesh_rectangle {
                println!("   ✗ 缺少 Mesh::new_rectangle 调用");
            }
            if !has_color_black {
                println!("   ✗ 缺少 Color::BLACK 使用");
            }
            return false;
        }
    } else {
        println!("   ✗ 无法读取 {}", game_scene_path);
        return false;
    }
}

/// 检查文件结构
fn check_file_structure() -> bool {
    let files = vec![
        "src/scenes/game_scene.rs",
        "src/scenes/game_scene/camera.rs",
        "src/scenes/game_scene/map_renderer.rs",
        "src/program.rs",
        "Cargo.toml",
    ];
    
    let mut all_exist = true;
    for file in files {
        if Path::new(file).exists() {
            println!("   ✓ {}", file);
        } else {
            println!("   ✗ {} (不存在)", file);
            all_exist = false;
        }
    }
    
    all_exist
}

/// 代码质量检查
fn check_code_quality() {
    let game_scene_path = "src/scenes/game_scene.rs";
    
    if let Ok(content) = fs::read_to_string(game_scene_path) {
        // 检查是否有 TODO 或 FIXME
        let todo_count = content.matches("TODO").count();
        let fixme_count = content.matches("FIXME").count();
        
        if todo_count > 0 {
            println!("   ⚠️  发现 {} 个 TODO 标记", todo_count);
        }
        if fixme_count > 0 {
            println!("   ⚠️  发现 {} 个 FIXME 标记", fixme_count);
        }
        
        // 检查是否有 unsafe 代码
        let unsafe_count = content.matches("unsafe").count();
        if unsafe_count > 0 {
            println!("   ⚠️  发现 {} 处 unsafe 代码", unsafe_count);
        }
        
        // 检查注释率
        let total_lines = content.lines().count();
        let comment_lines = content.lines().filter(|line| {
            line.trim().starts_with("//") || line.trim().starts_with("///")
        }).count();
        let comment_ratio = (comment_lines as f32 / total_lines as f32) * 100.0;
        
        println!("   ℹ️  代码行数: {}", total_lines);
        println!("   ℹ️  注释行数: {} ({:.1}%)", comment_lines, comment_ratio);
        
        if comment_ratio > 20.0 {
            println!("   ✓ 注释率良好");
        } else {
            println!("   ⚠️  注释率偏低，建议增加注释");
        }
    }
}

/// 生成测试报告
fn generate_report(passed: bool) {
    use std::io::Write;
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let report = format!(
r#"
========================================
GameScene 背景清理验证报告
========================================

测试时间: {}
测试结果: {}

修复内容:
- 在 GameScene::draw() 开头添加全屏黑色背景
- 使用 Mesh::new_rectangle 绘制背景
- 确保每帧清空画布

预期效果:
- 进入游戏场景后，背景应该是纯黑色
- 不应该看到登录界面的残留
- 地图纹理应该清晰地绘制在黑色背景上

自动化测试限制:
- 无法验证实际渲染效果
- 需要人工视觉检查
- 建议进行回归测试

========================================
"#,
        timestamp,
        if passed { "✅ 通过" } else { "❌ 失败" }
    );
    
    // 保存报告
    let report_filename = format!("test_report_{}.txt", 
        chrono::Local::now().format("%Y%m%d_%H%M%S"));
    
    if let Ok(mut file) = fs::File::create(&report_filename) {
        let _ = file.write_all(report.as_bytes());
        println!("   ✓ 报告已保存到: {}", report_filename);
    } else {
        println!("   ✗ 无法保存报告");
    }
}
