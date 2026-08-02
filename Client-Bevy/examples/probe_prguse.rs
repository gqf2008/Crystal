//! 调试：打印 Prguse 库关键纹理索引是否存在（HUD 用）
use client_bevy::resources::libraries::{Libraries, LibraryName};

fn main() {
    let mut libs = Libraries::new("Data");
    libs.ensure_initialized();
    let idxs: [usize; 15] = [0, 1, 2, 4, 7, 8, 76, 826, 1084, 1900, 1903, 1906, 1909, 1912, 1960];
    for i in idxs {
        match libs.get_image(LibraryName::Prguse, i) {
            Some(info) => println!("Prguse[{}] OK {}x{}", i, info.width, info.height),
            None => println!("Prguse[{}] MISSING", i),
        }
    }
}
