/// 查看 Prguse 纹理尺寸的工具
use client_macroquad::resources::mlibrary::MLibrary;
use std::path::PathBuf;

fn main() {
    let data_path = PathBuf::from("Data");
    
    // 加载 Prguse 库
    let prguse_path = data_path.join("Prguse.Lib");
    match MLibrary::open(&prguse_path) {
        Ok(mut lib) => {
            println!("✓ Loaded Prguse library");
            
            // 查看登录对话框 (1084)
            if let Ok(info) = lib.get_image_info(1084) {
                println!("📐 登录对话框 [1084]: {}x{}", info.width, info.height);
            }
            
            // 查看修改密码对话框 (50 - C# 原版使用)
            if let Ok(info) = lib.get_image_info(50) {
                println!("📐 修改密码对话框 [50]: {}x{}", info.width, info.height);
            }
            
            // 查看新建账号对话框 (63)
            if let Ok(info) = lib.get_image_info(63) {
                println!("📐 新建账号对话框 [63]: {}x{}", info.width, info.height);
            }
            
            // 查看 Prguse[64]
            if let Ok(info) = lib.get_image_info(64) {
                println!("📐 Prguse[64]: {}x{}", info.width, info.height);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to load Prguse: {:?}", e);
        }
    }
}
