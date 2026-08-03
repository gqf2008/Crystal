//! 调试：打印关键纹理索引是否存在（HUD/对话框用）
use client_bevy::resources::libraries::{Libraries, LibraryName};

fn main() {
    let mut libs = Libraries::new("Data");
    libs.ensure_initialized();

    let checks: &[(LibraryName, usize)] = &[
        (LibraryName::Title, 504),
        (LibraryName::Title, 500),
        (LibraryName::Title, 506),
        (LibraryName::Title, 567),
        (LibraryName::Prguse, 2090),
        (LibraryName::Prguse, 2091),
    ];
    for (lib, i) in checks {
        match libs.get_image(*lib, *i) {
            Some(info) => {
                let w = info.width.max(0) as usize;
                let h = info.height.max(0) as usize;
                let mut px = String::new();
                if let Some(rgba) = &info.rgba {
                    let at = |x: usize, y: usize| -> String {
                        let idx = (y * w + x) * 4;
                        if idx + 3 < rgba.len() {
                            format!("{:02x}{:02x}{:02x}{:02x}", rgba[idx], rgba[idx+1], rgba[idx+2], rgba[idx+3])
                        } else {
                            "??".to_string()
                        }
                    };
                    px = format!(" px(0,0)={} px(10,10)={} px(mid)={}", at(0,0), at(10.min(w-1), 10.min(h-1)), at(w/2, h/2));
                }
                println!("{:?}[{}] {}x{} rgba={}{}", lib, i, w, h, info.rgba.as_ref().map(|r| r.len()).unwrap_or(0), px);
            }
            None => println!("{:?}[{}] MISSING", lib, i),
        }
    }
}
