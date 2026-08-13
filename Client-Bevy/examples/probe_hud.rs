//! 实测 HUD 底栏相关精灵尺寸/偏移（C# MainDialog 对齐基线）
use client_bevy::resources::libraries::{Libraries, LibraryName};
fn main() {
    let data = client_bevy::resources::libraries::resolve_data_path();
    let mut libs = Libraries::new(&data);
    libs.ensure_initialized();
    println!("data = {}", data.display());
    let mut d = |l: LibraryName, i: usize, tag: &str| match libs.get_image(l, i) {
        Some(x) => println!("{:?}[{}] {} {}x{} offset=({},{})", l, i, tag, x.width, x.height, x.offset_x, x.offset_y),
        None => println!("{:?}[{}] {} MISSING", l, i, tag),
    };
    d(LibraryName::Prguse, 0, "底栏800");
    d(LibraryName::Prguse, 1, "底栏1024");
    d(LibraryName::Prguse, 2, "底栏其他");
    d(LibraryName::Prguse, 4, "血蓝球");
    d(LibraryName::Prguse, 6, "纯血球(战士<26)");
    d(LibraryName::Prguse, 7, "经验条800");
    d(LibraryName::Prguse, 8, "经验条1024");
    d(LibraryName::Prguse, 76, "负重条<=50%");
}
