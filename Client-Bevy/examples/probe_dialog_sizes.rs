//! 探针：打印进入游戏前所有 UI 精灵的真实尺寸/offset，核实居中假设。
//! 运行：cargo run --example probe_dialog_sizes
use client_bevy::resources::libraries::{Libraries, LibraryName};

fn row(libs: &mut Libraries, lib: LibraryName, idx: usize) {
    match libs.get_image(lib, idx) {
        Some(i) => println!(
            "{:?}[{:>5}]  {:>4}x{:<4}  offset=({:>5},{:>5})",
            lib, idx, i.width, i.height, i.offset_x, i.offset_y
        ),
        None => println!("{:?}[{:>5}]  缺失", lib, idx),
    }
}

fn main() {
    let mut libs = Libraries::new("Data");
    libs.ensure_initialized();

    println!("=== 对话框背景（居中假设依赖这些尺寸）===");
    for idx in [1084, 63, 50, 73, 660, 360, 65] {
        row(&mut libs, LibraryName::Prguse, idx);
    }
    println!("\n=== Title 标题/按钮 ===");
    for idx in [30, 20, 40, 31, 32] {
        row(&mut libs, LibraryName::Title, idx);
    }
    println!("\n=== Title 角色槽 ===");
    for idx in [660, 661, 662, 663, 664, 665] {
        row(&mut libs, LibraryName::Title, idx);
    }
    println!("\n=== Title 各按钮（命中框尺寸假设）===");
    for idx in [320, 323, 326, 329, 332, 340, 343, 346, 349, 352, 200, 203, 206, 210, 280, 360, 107, 110] {
        row(&mut libs, LibraryName::Title, idx);
    }
    println!("\n=== Prguse 职业按钮/空槽 ===");
    for idx in [44, 2420, 2423, 2426, 2429, 2432, 2435, 2438] {
        row(&mut libs, LibraryName::Prguse, idx);
    }
    println!("\n=== ChrSel 背景/预览 ===");
    for idx in [0, 20] {
        row(&mut libs, LibraryName::ChrSel, idx);
    }

    println!("\n=== ChrSel blend 叠加层（Index+560，各职业/性别起始帧）===");
    // base: 战士20/法师40/道士60/刺客80/弓手100(男)140(女)；女=base+280
    // blend 帧索引 = base + 560
    let bases = [20, 40, 60, 80, 100, 140, 300, 320, 340, 360];
    for &b in &bases {
        row(&mut libs, LibraryName::ChrSel, b + 560);
    }
    println!("\n=== ChrSel 战士男 blend 全 16 帧（确认整段存在）===");
    for i in 0..16 {
        row(&mut libs, LibraryName::ChrSel, 20 + i + 560);
    }
}
