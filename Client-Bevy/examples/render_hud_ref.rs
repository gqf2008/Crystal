//! 渲染 HUD 底栏标签位置对比图（#2497 PR 证据）：真实底栏美术 Prguse[1] 上叠加
//! 旧 Bevy 位置（红）vs 新 C# 对齐位置（绿）的文本框，肉眼确认标签落到美术预留区。
//! CPU blit，无 GPU 依赖。运行：cargo run --example render_hud_ref（工作目录 = Client-Bevy）
//! 输出：verify-out/hud_labels_compare.png
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

    let (bw, bh): (i32, i32) = match libs.get_image(LibraryName::Prguse, 1) {
        Some(i) => (i.width.max(0) as i32, i.height.max(0) as i32),
        None => (1024, 152),
    };
    let mut buf: RgbaImage =
        ImageBuffer::from_pixel(bw as u32, (bh * 2 + 20) as u32, Rgba([24, 24, 32, 255]));

    // 上行（y=0）：新 C# 对齐位置（绿框）；下行（y=bh+20）：旧 Bevy 位置（红框）
    blit(&mut buf, &mut libs, LibraryName::Prguse, 1, 0, 0);
    blit(&mut buf, &mut libs, LibraryName::Prguse, 1, 0, bh + 20);
    let green = [60u8, 220, 90];
    let red = [230u8, 60, 60];
    // (x, y, w, h) 底栏相对坐标
    let new_boxes = [
        (5, 108, 24, 12),            // Level
        (6, 120, 90, 16),            // Name
        (bw - 105, 119, 99, 13),     // Gold
        (9 + 502 - 20, 133, 40, 12), // Exp（条宽1004/2-20=482 → 9+482=491）
        (50 - 35, 57, 70, 12),       // HP（球心50 居中，~70宽）
        (50 - 35, 72, 70, 12),       // MP
        (bw - 105, 101, 40, 14),     // Weight（未变）
        (bw - 30, 101, 26, 14),      // Space（未变）
    ];
    let old_boxes = [
        (9, 2, 24, 12),          // Level 旧
        (9, 14, 90, 16),         // Name 旧
        (bw - 90, 2, 99, 13),    // Gold 旧
        (59, 141, 40, 12),       // Exp 旧
        (9, 48, 24, 12),         // HP 旧（并排左）
        (60, 48, 24, 12),        // MP 旧（并排右）
        (bw - 105, 101, 40, 14), // Weight（未变）
        (bw - 30, 101, 26, 14),  // Space（未变）
    ];
    for (x, y, w, h) in new_boxes {
        rect(&mut buf, x, y, w, h, green);
    }
    for (x, y, w, h) in old_boxes {
        rect(&mut buf, x, y + bh + 20, w, h, red);
    }

    std::fs::create_dir_all("verify-out").ok();
    buf.save("verify-out/hud_labels_compare.png")
        .expect("save png");
    println!("saved verify-out/hud_labels_compare.png (上=新 C# 对齐绿框, 下=旧 Bevy 红框)");
}
