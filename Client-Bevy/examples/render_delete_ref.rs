// 渲染删除角色确认框原版参考（纯 CPU 合成，对齐 C# MirMessageBox/MirInputBox 坐标）
#![allow(clippy::manual_checked_ops, clippy::manual_saturating_arithmetic)]
use client_bevy::resources::libraries::{resolve_data_path, Libraries, LibraryName};
use image::{Rgba, RgbaImage};

fn main() {
    let data = resolve_data_path();
    println!("data: {}", data.display());
    let mut libs = Libraries::new(data);
    libs.ensure_initialized();

    // ===== 第一步：MirMessageBox YesNo (Prguse[360]) =====
    {
        let mut img = RgbaImage::new(1024, 768);
        let info = libs.get_image(LibraryName::Prguse, 360).unwrap();
        let (w, h) = (info.width as i32, info.height as i32);
        println!("Prguse[360] = {}x{}", w, h);
        let dx = (1024 - w) / 2;
        let dy = (768 - h) / 2;
        paste_lib(&mut libs, &mut img, LibraryName::Prguse, 360, dx, dy);
        // Yes Title[206]@(260,157)  No Title[210]@(360,157)
        paste_lib(&mut libs, &mut img, LibraryName::Title, 206, dx + 260, dy + 157);
        paste_lib(&mut libs, &mut img, LibraryName::Title, 210, dx + 360, dy + 157);
        img.save("../tools/ref_del_ask.png").unwrap();
        println!("saved ref_del_ask.png");
    }

    // ===== 第二步：MirInputBox (Prguse[660]) =====
    {
        let mut img = RgbaImage::new(1024, 768);
        let info = libs.get_image(LibraryName::Prguse, 660).unwrap();
        let (w, h) = (info.width as i32, info.height as i32);
        println!("Prguse[660] = {}x{}", w, h);
        let dx = (1024 - w) / 2;
        let dy = (768 - h) / 2;
        paste_lib(&mut libs, &mut img, LibraryName::Prguse, 660, dx, dy);
        // 输入框边框示意 (23,86) 240x19 绿框
        draw_rect(&mut img, dx + 23, dy + 86, 240, 19, [0, 255, 0, 255]);
        // OK Title[200]@(60,123)  Cancel Title[203]@(160,123)
        paste_lib(&mut libs, &mut img, LibraryName::Title, 200, dx + 60, dy + 123);
        paste_lib(&mut libs, &mut img, LibraryName::Title, 203, dx + 160, dy + 123);
        img.save("../tools/ref_del_input.png").unwrap();
        println!("saved ref_del_input.png");
    }

    // 单独打印按钮尺寸，便于核对 Bevy 端命中矩形
    for idx in [200usize, 203, 206, 210] {
        if let Some(info) = libs.get_image(LibraryName::Title, idx) {
            println!("Title[{}] = {}x{}", idx, info.width, info.height);
        }
    }
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

fn draw_rect(img: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
    for i in 0..w {
        put(img, x + i, y, c);
        put(img, x + i, y + h - 1, c);
    }
    for j in 0..h {
        put(img, x, y + j, c);
        put(img, x + w - 1, y + j, c);
    }
}

fn put(img: &mut RgbaImage, x: i32, y: i32, c: [u8; 4]) {
    if x >= 0 && y >= 0 && x < img.width() as i32 && y < img.height() as i32 {
        img.put_pixel(x as u32, y as u32, Rgba(c));
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
            if a == 0 {
                continue;
            }
            let (dr, dg, db, da) = {
                let d = img.get_pixel(px as u32, py as u32).0;
                (d[0] as u32, d[1] as u32, d[2] as u32, d[3] as u32)
            };
            let sa = a as u32;
            let inv = (255u32).checked_sub(sa).unwrap_or(0);
            let out_a = sa + da * inv / 255;
            let r = if out_a > 0 {
                (rgba[src] as u32 * sa + dr * da * inv / 255) / out_a
            } else {
                0
            };
            let g = if out_a > 0 {
                (rgba[src + 1] as u32 * sa + dg * da * inv / 255) / out_a
            } else {
                0
            };
            let b = if out_a > 0 {
                (rgba[src + 2] as u32 * sa + db * da * inv / 255) / out_a
            } else {
                0
            };
            img.put_pixel(px as u32, py as u32, Rgba([r as u8, g as u8, b as u8, out_a as u8]));
        }
    }
}
