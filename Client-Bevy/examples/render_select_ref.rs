use client_bevy::resources::libraries::{resolve_data_path, Libraries, LibraryName};
use image::{Rgba, RgbaImage};

fn main() {
    let data = resolve_data_path();
    let mut libs = Libraries::new(data);
    libs.ensure_initialized();
    let (w, h) = (1024u32, 768u32);
    let mut img = RgbaImage::new(w, h);
    // Background Prguse[65] at (0,0)
    paste_lib(&mut libs, &mut img, LibraryName::Prguse, 65, 0, 0);
    // Title[40] at (468,20)
    paste_lib(&mut libs, &mut img, LibraryName::Title, 40, 468, 20);
    // Character slot frames Title[660..663] at (637,194/298/402/506)
    let ys = [194i32, 298, 402, 506];
    for (slot, y) in ys.iter().enumerate() {
        paste_lib(&mut libs, &mut img, LibraryName::Title, 660 + slot, 637, *y);
    }
    // Bottom buttons: C# indices 340/343/346/349/352 at x=132/296/460/624/788, y=736
    let xs = [132i32, 296, 460, 624, 788];
    for (n, x) in xs.iter().enumerate() {
        paste_lib(&mut libs, &mut img, LibraryName::Title, 340 + 3 * n, *x, 736);
    }
    img.save("../tools/ref_select.png").unwrap();
    println!("saved ../tools/ref_select.png");
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
            if px < 0 || py < 0 || px >= img.width() as i32 || py >= img.height() as i32 { continue; }
            let a = rgba[src + 3];
            if a == 0 { continue; }
            let (dr, dg, db, da) = {
                let d = img.get_pixel(px as u32, py as u32).0;
                (d[0] as u32, d[1] as u32, d[2] as u32, d[3] as u32)
            };
            let sa = a as u32;
            let inv = 255u32 - sa;
            let out_a = sa + da * inv / 255;
            let r = if out_a > 0 { (rgba[src] as u32 * sa + dr * da * inv / 255) / out_a } else { 0 };
            let g = if out_a > 0 { (rgba[src+1] as u32 * sa + dg * da * inv / 255) / out_a } else { 0 };
            let b = if out_a > 0 { (rgba[src+2] as u32 * sa + db * da * inv / 255) / out_a } else { 0 };
            img.put_pixel(px as u32, py as u32, Rgba([r as u8, g as u8, b as u8, out_a as u8]));
        }
    }
}
