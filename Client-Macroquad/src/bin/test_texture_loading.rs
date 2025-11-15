// 测试纹理加载
use client_macroquad::resources::{self, LibraryName};

fn main() {
    println!("🧪 测试纹理数据加载系统");
    println!("================================\n");

    // 初始化库
    println!("📦 初始化资源库...");
    if let Err(e) = resources::initialize_all_libraries("Data") {
        eprintln!("❌ 初始化失败: {}", e);
        return;
    }
    println!("✅ 资源库初始化成功\n");

    // 测试加载几个纹理数据
    let test_cases = vec![
        (LibraryName::Prguse, 360, "Prguse 360"),
        (LibraryName::Title, 30, "Title 30"),
        (LibraryName::ChrSel, 0, "ChrSel 0"),
    ];

    println!("🎨 测试纹理数据加载:");
    for (lib, index, name) in test_cases {
        print!("  {} ... ", name);
        match resources::get_or_create_texture(lib, index) {
            Some(info) => {
                // 检查 RGBA 数据是否存在（纹理会在有窗口上下文时创建）
                if let Some(data) = info.get_rgba_data() {
                    println!("✅ 成功 ({}x{}, {} bytes)", 
                        info.width, info.height, data.len());
                } else if info.image.is_some() {
                    println!("✅ 成功（纹理已创建）({}x{})", info.width, info.height);
                } else {
                    println!("❌ 失败: 无RGBA数据且无纹理");
                }
            }
            None => {
                println!("❌ 失败: 无法加载");
            }
        }
    }

    println!("\n📊 测试完成!");
    println!("💡 注意: 实际纹理需要在 macroquad 窗口上下文中创建");
}
