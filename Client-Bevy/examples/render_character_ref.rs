//! 渲染角色对话框装备格位置对比图（#2503 PR 证据）：真实对话框美术（Title[504] 框 +
//! Prguse[340] 角色页 + Prguse[100] 职业图标）上，叠加旧 Bevy 位置（红，漏加页偏移 (8,90)、
//! 36x36）vs 新 C# 对齐位置（绿，含页偏移、36x32）的装备格框，肉眼确认装备格落进纸娃娃区。
//! CPU blit，无 GPU 依赖。运行：cargo run --example render_character_ref（工作目录 = Client-Bevy）
//! 输出：verify-out/character_equip_compare.png
use client_bevy::game::dialogs::character as ch;
use client_bevy::resources::libraries::{Libraries, LibraryName};
use image::{ImageBuffer, Rgba, RgbaImage};

fn blit(buf: &mut RgbaImage, libs: &mut Libraries, lib: LibraryName, idx: usize, x: i32, y: i32) {
    let Some(info) = libs.get_image(lib, idx) else {
        eprintln!("  [skip] {:?}[{}] 缺失", lib, idx);
        return;
    };
    let Some(rgba) = info.rgba.as_ref() else {
        return;
    };
    let iw = info.width.max(0) as u32;
    let ih = info.height.max(0) as u32;
    if iw == 0 || ih == 0 || rgba.len() < (iw * ih * 4) as usize {
        return;
    }
    for sy in 0..ih {
        for sx in 0..iw {
            let p = &rgba[((sy * iw + sx) * 4) as usize..][..4];
            if p[3] == 0 {
                continue;
            }
            let a = p[3] as f32 / 255.0;
            let px = x + sx as i32;
            let py = y + sy as i32;
            if px < 0 || py < 0 || px >= buf.width() as i32 || py >= buf.height() as i32 {
                continue;
            }
            let dst = buf.get_pixel_mut(px as u32, py as u32);
            *dst = Rgba([
                (p[0] as f32 * a + dst[0] as f32 * (1.0 - a)) as u8,
                (p[1] as f32 * a + dst[1] as f32 * (1.0 - a)) as u8,
                (p[2] as f32 * a + dst[2] as f32 * (1.0 - a)) as u8,
                255,
            ]);
        }
    }
}

fn rect(buf: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
    // 仅画边框（1px），不填充，避免盖住美术
    for xx in x..x + w {
        for (px, py) in [(xx, y), (xx, y + h - 1)] {
            if px >= 0 && py >= 0 && px < buf.width() as i32 && py < buf.height() as i32 {
                buf.put_pixel(px as u32, py as u32, Rgba([c[0], c[1], c[2], 255]));
            }
        }
    }
    for yy in y..y + h {
        for (px, py) in [(x, yy), (x + w - 1, yy)] {
            if px >= 0 && py >= 0 && px < buf.width() as i32 && py < buf.height() as i32 {
                buf.put_pixel(px as u32, py as u32, Rgba([c[0], c[1], c[2], 255]));
            }
        }
    }
}

fn main() {
    let data = client_bevy::resources::libraries::resolve_data_path();
    let mut libs = Libraries::new(&data);
    libs.ensure_initialized();

    // 画布 = 对话框（Title[504]）真实尺寸；全程用对话框相对坐标（原点 = 对话框左上角）
    let (dw, dh): (i32, i32) = match libs.get_image(LibraryName::Title, 504) {
        Some(i) => (i.width.max(0) as i32, i.height.max(0) as i32),
        None => (264, 420),
    };
    let mut buf: RgbaImage = ImageBuffer::from_pixel(dw as u32, dh as u32, Rgba([24, 24, 32, 255]));

    // 底图：对话框框 + 角色页背景 + 职业图标（均对话框相对）
    blit(&mut buf, &mut libs, LibraryName::Title, 504, 0, 0);
    blit(
        &mut buf,
        &mut libs,
        LibraryName::Prguse,
        340,
        ch::PAGE_X as i32,
        ch::PAGE_Y as i32,
    );
    blit(
        &mut buf,
        &mut libs,
        LibraryName::Prguse,
        100,
        ch::CLASS_IMG_X as i32,
        ch::CLASS_IMG_Y as i32,
    );

    let green = [60u8, 220, 90]; // 新 C# 对齐（含页偏移 (8,90)，36x32）
    let red = [230u8, 60, 60]; // 旧 Bevy（漏页偏移，36x36）
    let yellow = [230u8, 200, 60]; // 名字/行会居中框
    for (ox, oy) in ch::EQUIP_SLOTS {
        // 新：DIALOG 相对 = PAGE + 页内偏移，36x32
        rect(
            &mut buf,
            (ch::PAGE_X + ox) as i32,
            (ch::PAGE_Y + oy) as i32,
            ch::SLOT_W as i32,
            ch::SLOT_H as i32,
            green,
        );
        // 旧：漏加页偏移，36x36
        rect(&mut buf, ox as i32, oy as i32, 36, 36, red);
    }
    // 名字 (0,12)264x20 / 行会 (0,33)264x30 居中框（黄）
    rect(&mut buf, 0, 12, 264, 20, yellow);
    rect(&mut buf, 0, 33, 264, 30, yellow);

    std::fs::create_dir_all("verify-out").ok();
    buf.save("verify-out/character_equip_compare.png")
        .expect("save png");
    println!(
        "saved verify-out/character_equip_compare.png (绿=新 C# 对齐含页偏移(8,90) 36x32, 红=旧漏页偏移 36x36, 黄=名字/行会居中框)"
    );
}
