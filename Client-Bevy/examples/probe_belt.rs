//! 一次性探针：Prguse[1932]（药水腰带背景 240x38）槽位区域像素采样
use client_bevy::resources::libraries::{Libraries, LibraryName};

fn main() {
    let mut libs = Libraries::new("Data");
    libs.ensure_initialized();
    for idx in [1932usize, 1933] {
        match libs.get_image(LibraryName::Prguse, idx) {
            Some(info) => {
                let w = info.width.max(0) as usize;
                let h = info.height.max(0) as usize;
                println!(
                    "Prguse[{}] {}x{} has_rgba={}",
                    idx,
                    info.width,
                    info.height,
                    info.rgba.is_some()
                );
                if let Some(rgba) = &info.rgba {
                    let at = |x: usize, y: usize| -> String {
                        let i = (y * w + x) * 4;
                        if i + 3 < rgba.len() {
                            format!(
                                "{:02x}{:02x}{:02x}{:02x}",
                                rgba[i],
                                rgba[i + 1],
                                rgba[i + 2],
                                rgba[i + 3]
                            )
                        } else {
                            "??".into()
                        }
                    };
                    // 槽位 1 中心约 (12+17, 3+17)=(29,20)；槽间空隙 (5,20)；面板角 (2,2)
                    println!("  corner(2,2)={} gap(5,20)={} slot1c(29,20)={} slot2c(64,20)={} mid(120,19)={}",
                        at(2,2), at(5,20), at(29,20), at(64,20), at(120,19));
                }
            }
            None => println!("Prguse[{}] MISSING", idx),
        }
    }
}
