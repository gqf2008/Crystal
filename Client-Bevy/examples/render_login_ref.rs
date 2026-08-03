// 渲染参考登录界面（纯 CPU 合成，对齐原版 C#/macroquad 坐标）
#![allow(clippy::manual_checked_ops, clippy::manual_saturating_arithmetic)]
use client_bevy::resources::libraries::{resolve_data_path, Libraries, LibraryName};
use image::{Rgba, RgbaImage};

fn main() {
    let data = resolve_data_path();
    println!("data: {}", data.display());
    let mut libs = Libraries::new(data);
    libs.ensure_initialized();

    let w = 1280u32;
    let h = 800u32;
    let mut img = RgbaImage::new(w, h);

    // 背景 ChrSel[0]（若有）
    if let Some(info) = libs.get_image(LibraryName::ChrSel, 0) {
        if let Some(rgba) = info.rgba {
            paste(&mut img, &rgba, info.width as u32, info.height as u32, 0, 0);
        }
    }

    // 对话框 Prguse[1084] at (476,290)
    let (dx, dy) = (476i32, 290i32);
    paste_lib(&mut libs, &mut img, LibraryName::Prguse, 1084, dx, dy);

    // Title[30] 居中
    if let Some(info) = libs.get_image(LibraryName::Title, 30) {
        let tw = info.width as i32;
        let x = dx + (328 - tw) / 2;
        paste_lib(&mut libs, &mut img, LibraryName::Title, 30, x, dy + 12);
    }
    // Title[31] 账号标签 (52,83)  Title[32] 密码标签 (43,105)
    paste_lib(&mut libs, &mut img, LibraryName::Title, 31, dx + 52, dy + 83);
    paste_lib(&mut libs, &mut img, LibraryName::Title, 32, dx + 43, dy + 105);

    // 按钮（正常态第一帧）
    paste_lib(&mut libs, &mut img, LibraryName::Title, 320, dx + 227, dy + 81);
    paste_lib(&mut libs, &mut img, LibraryName::Title, 323, dx + 60, dy + 163);
    paste_lib(&mut libs, &mut img, LibraryName::Title, 326, dx + 166, dy + 163);
    paste_lib(&mut libs, &mut img, LibraryName::Title, 332, dx + 60, dy + 189);
    paste_lib(&mut libs, &mut img, LibraryName::Title, 329, dx + 166, dy + 189);

    img.save("../tools/ref_login.png").unwrap();
    println!("saved ../../tools/ref_login.png");

    // 也保存对话框底图单独检查
    let mut dlg = RgbaImage::new(328, 220);
    let raw = libs.get_image(LibraryName::Prguse, 1084).unwrap();
    paste(&mut dlg, &raw.rgba.unwrap(), raw.width as u32, raw.height as u32, 0, 0);
    dlg.save("../tools/ref_dlg.png").unwrap();
    println!("dlg saved");
}

fn paste_lib(libs: &mut Libraries, img: &mut RgbaImage, name: LibraryName, idx: usize, x: i32, y: i32) {
    if let Some(info) = libs.get_image(name, idx) {
        if let Some(rgba) = info.rgba {
            paste(img, &rgba, info.width as u32, info.height as u32, x, y);
        } else {
            println!("no rgba: {:?}[{}]", name, idx);
        }
    } else {
        println!("missing: {:?}[{}]", name, idx);
    }
}

fn paste(img: &mut RgbaImage, rgba: &[u8], iw: u32, ih: u32, x: i32, y: i32) {
    for j in 0..ih {
        for i in 0..iw {
            let src = ((j * iw + i) * 4) as usize;
            let (px, py) = (x + i as i32, y + j as i32);
            if px < 0 || py < 0 || px >= img.width() as i32 || py >= img.height() as i32 {
                continue;
            }
            let a = rgba[src + 3];
            if a == 0 { continue; }
            let (dr, dg, db, da) = {
                let d = img.get_pixel(px as u32, py as u32).0;
                (d[0] as u32, d[1] as u32, d[2] as u32, d[3] as u32)
            };
            let sa = a as u32;
            let inv = (255u32).checked_sub(sa).unwrap_or(0);
            let out_a = sa + da * inv / 255;
            let r = if out_a > 0 { (rgba[src] as u32 * sa + dr * da * inv / 255) / out_a } else { 0 };
            let g = if out_a > 0 { (rgba[src + 1] as u32 * sa + dg * da * inv / 255) / out_a } else { 0 };
            let b = if out_a > 0 { (rgba[src + 2] as u32 * sa + db * da * inv / 255) / out_a } else { 0 };
            img.put_pixel(px as u32, py as u32, Rgba([r as u8, g as u8, b as u8, out_a as u8]));
        }
    }
}

