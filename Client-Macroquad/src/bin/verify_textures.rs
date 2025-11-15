/// 验证纹理加载是否正确
/// 测试通过 resources::get_or_create_texture 获取的纹理是否包含实际的 Texture2D
use client_macroquad::resources::{self, LibraryName};

fn main() {
    println!("🧪 测试纹理加载...\n");

    // 初始化资源系统
    println!("📦 初始化资源系统...");
    if let Err(e) = resources::initialize_all_libraries("Data") {
        println!("❌ 初始化失败: {}", e);
        return;
    }
    println!("✓ 初始化完成\n");

    // 测试1: 背景图片 (ChrSel 0)
    println!("1️⃣ 测试 ChrSel[0]...");
    match resources::get_or_create_texture(LibraryName::ChrSel, 0) {
        Some(info) => {
            println!("   📏 图像尺寸: {}x{}", info.width, info.height);
            if let Some(ref texture) = info.image {
                println!("   ✅ 纹理已创建: {}x{}", texture.width(), texture.height());
            } else {
                println!("   ❌ 纹理字段为 None（可能因为没有窗口上下文）");
            }
        }
        None => {
            println!("   ❌ 无法加载图像（get_or_create_texture 返回 None）");
        }
    }

    // 测试2: UI 元素 (Prguse 360)
    println!("\n2️⃣ 测试 Prguse[360]...");
    if let Some(info) = resources::get_or_create_texture(LibraryName::Prguse, 360) {
        if let Some(ref texture) = info.image {
            println!("   ✅ 纹理已创建: {}x{}", texture.width(), texture.height());
        } else {
            println!("   ❌ 纹理字段为 None");
        }
    } else {
        println!("   ❌ 无法加载图像");
    }

    // 测试3: 标题 (Title 30)
    println!("\n3️⃣ 测试 Title[30]...");
    if let Some(info) = resources::get_or_create_texture(LibraryName::Title, 30) {
        if let Some(ref texture) = info.image {
            println!("   ✅ 纹理已创建: {}x{}", texture.width(), texture.height());
        } else {
            println!("   ❌ 纹理字段为 None");
        }
    } else {
        println!("   ❌ 无法加载图像");
    }

    println!("\n✅ 测试完成");
}
