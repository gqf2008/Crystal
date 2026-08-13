//! 调试：实测技能快捷栏相关精灵的真实尺寸与偏移（C# SkillBarDialog 对齐基线）。
//! 运行：cargo run --example probe_skillbar（工作目录 = Client-Bevy）
use client_bevy::resources::libraries::{Libraries, LibraryName};

fn main() {
    let data = client_bevy::resources::libraries::resolve_data_path();
    let mut libs = Libraries::new(&data);
    libs.ensure_initialized();
    println!("data = {}", data.display());

    let dump = |libs: &mut Libraries, lib: LibraryName, idx: usize| match libs.get_image(lib, idx) {
        Some(info) => println!(
            "{:?}[{}] {}x{} offset=({},{}) rgba={}",
            lib,
            idx,
            info.width,
            info.height,
            info.offset_x,
            info.offset_y,
            info.rgba.as_ref().map(|r| r.len()).unwrap_or(0)
        ),
        None => println!("{:?}[{}] MISSING", lib, idx),
    };

    println!("--- C# SkillBarDialog 引用 ---");
    dump(&mut libs, LibraryName::Prguse, 2190); // 底图
    dump(&mut libs, LibraryName::Prguse, 2193); // BeforeDraw 格网 (+12,0, 50% alpha)
    dump(&mut libs, LibraryName::Prguse, 2247); // 切换绑定按钮 16x28

    println!("--- 冷却帧 Prguse2[1260..1263]（C# CoolDowns Index=1260+frame, 共22帧） ---");
    for i in 1260..=1263 {
        dump(&mut libs, LibraryName::Prguse2, i);
    }

    println!("--- MagIcon 样本（C# Index = Icon*2）---");
    for i in [0usize, 1, 2, 3, 10, 20, 50, 100, 150, 200, 222, 223] {
        dump(&mut libs, LibraryName::MagIcon, i);
    }
}
