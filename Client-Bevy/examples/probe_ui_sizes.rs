//! 探针：dump 登录/选角 UI 所有相关精灵的真实像素尺寸（用于替换硬编码尺寸）。
//! 运行：cargo run --example probe_ui_sizes （工作目录 = Client-Bevy）
use client_bevy::resources::libraries::{Libraries, LibraryName};

fn main() {
    let mut libs = Libraries::new("Data");
    libs.ensure_initialized();

    let checks: &[(LibraryName, usize, &str)] = &[
        // ---- 登录界面 ----
        (LibraryName::Prguse, 1084, "登录对话框底"),
        (LibraryName::ChrSel, 0, "登录背景 ChrSel[0]"),
        (LibraryName::Title, 30, "Login 标题横幅"),
        (LibraryName::Title, 31, "AccountID 标签"),
        (LibraryName::Title, 32, "Password 标签"),
        (LibraryName::Title, 320, "OK 按钮"),
        (LibraryName::Title, 323, "新建账号按钮"),
        (LibraryName::Title, 326, "改密按钮"),
        (LibraryName::Title, 329, "关闭按钮"),
        (LibraryName::Title, 332, "软键盘按钮"),
        // ---- 新建账号 / 改密 对话框底 ----
        (LibraryName::Prguse, 63, "新建账号对话框底"),
        (LibraryName::Prguse, 50, "改密对话框底"),
        (LibraryName::Title, 200, "OK(对话框)"),
        (LibraryName::Title, 203, "Cancel(对话框)"),
        (LibraryName::Title, 107, "改密 OK"),
        (LibraryName::Title, 110, "改密 Cancel"),
        // ---- 选角界面 ----
        (LibraryName::Prguse, 65, "选角背景"),
        (LibraryName::Title, 40, "选角标题"),
        (LibraryName::Prguse, 44, "空角色槽"),
        (LibraryName::Prguse, 45, "锁定槽(reserved)"),
        (LibraryName::Title, 660, "角色槽 战士帧"),
        (LibraryName::Title, 665, "角色槽 战士选中帧"),
        (LibraryName::Title, 340, "开始游戏按钮"),
        (LibraryName::Title, 343, "新建角色按钮"),
        (LibraryName::Title, 346, "删除角色按钮"),
        (LibraryName::Title, 349, "Credits 按钮"),
        (LibraryName::Title, 352, "退出按钮"),
        // ---- 角色预览（含混合叠加层）----
        (LibraryName::ChrSel, 20, "预览 战士男 base"),
        (LibraryName::ChrSel, 580, "预览 战士男 混合叠加(20+560)"),
        // ---- 新建角色对话框 ----
        (LibraryName::Prguse, 73, "新建角色对话框底"),
        (LibraryName::Title, 20, "新建角色标题"),
        (LibraryName::Title, 360, "新建角色 OK"),
        (LibraryName::Title, 280, "新建角色 Cancel"),
        (LibraryName::Prguse, 2420, "男按钮"),
        (LibraryName::Prguse, 2423, "女按钮"),
        (LibraryName::Prguse, 2426, "战士按钮"),
        (LibraryName::Prguse, 2438, "弓手按钮"),
        // ---- 删除确认 ----
        (LibraryName::Prguse, 360, "MessageBox 底"),
        (LibraryName::Title, 206, "Yes 按钮"),
        (LibraryName::Title, 210, "No 按钮"),
        (LibraryName::Prguse, 660, "InputBox 底"),
    ];

    println!("{:<8} {:<6} {:<10} {:<10} {}", "Lib", "Idx", "W", "H", "用途");
    println!("{}", "-".repeat(70));
    for (lib, i, desc) in checks {
        match libs.get_image(*lib, *i) {
            Some(info) => {
                let w = info.width.max(0);
                let h = info.height.max(0);
                let has_rgba = info.rgba.as_ref().map(|r| r.len()).unwrap_or(0);
                println!(
                    "{:<8} {:<6} {:<10} {:<10} {} (rgba={})",
                    format!("{:?}", lib),
                    i,
                    w,
                    h,
                    desc,
                    has_rgba
                );
            }
            None => println!("{:<8} {:<6} {:<10} {:<10} {} *** MISSING ***", format!("{:?}", lib), i, "-", "-", desc),
        }
    }
}
