//! 调试（批R #2602）：公告/英雄腰带/帮助页所需精灵实测尺寸
use client_bevy::resources::libraries::{Libraries, LibraryName};

fn main() {
    let mut libs = Libraries::new("Data");
    libs.ensure_initialized();

    let checks: &[(LibraryName, usize)] = &[
        // NoticeDialog（NoticeDialog.cs:31/51/63/75/97/119）
        (LibraryName::Prguse, 961),   // 公告背景
        (LibraryName::Prguse2, 470),  // UpButton
        (LibraryName::Prguse2, 473),  // DownButton
        (LibraryName::Prguse2, 205),  // PositionBar
        (LibraryName::Title, 193),    // OkButton
        // HeroBeltDialog（HeroDialogs.cs:256-300）
        (LibraryName::Prguse, 1921),  // 横向背景
        (LibraryName::Prguse, 1943),  // 纵向背景（Flip）
        (LibraryName::Prguse, 1934),  // 横向叠加底
        (LibraryName::Prguse, 1946),  // 纵向叠加底
        (LibraryName::Prguse, 1926),  // 旋转钮
        (LibraryName::Prguse, 1923),  // 关闭钮
        (LibraryName::Prguse, 1935),  // 纵向旋转钮（Flip）
        // HelpDialog（HelpDialog.cs）
        (LibraryName::Prguse, 920),   // 帮助背景
        (LibraryName::Title, 57),     // 标题图
        (LibraryName::Prguse2, 240),  // Previous
        (LibraryName::Prguse2, 243),  // Next
        (LibraryName::Help, 0),       // 图文页首页
        (LibraryName::Help, 41),      // 图文页末页
    ];
    for (lib, i) in checks {
        match libs.get_image(*lib, *i) {
            Some(info) => {
                let w = info.width.max(0) as usize;
                let h = info.height.max(0) as usize;
                println!(
                    "{:?}[{}] {}x{} offset=({},{})",
                    lib, i, w, h, info.offset_x, info.offset_y
                );
            }
            None => println!("{:?}[{}] MISSING", lib, i),
        }
    }
}
