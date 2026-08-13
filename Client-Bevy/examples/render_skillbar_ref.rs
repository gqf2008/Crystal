//! 渲染技能快捷栏对比图（#2487 PR 证据）：上行=新实现（C# SkillBarDialog 源码级），下行=旧实现。
//! CPU blit，无 GPU 依赖。运行：cargo run --example render_skillbar_ref（工作目录 = Client-Bevy）
//! 输出：verify-out/skillbar_compare.png
use client_bevy::resources::libraries::{Libraries, LibraryName};
use image::{ImageBuffer, Rgba, RgbaImage};

fn blit_scaled(
    buf: &mut RgbaImage,
    libs: &mut Libraries,
    lib: LibraryName,
    idx: usize,
    x: i32,
    y: i32,
    alpha: f32,
    scale: i32,
) {
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
            let p = &rgba[((sy * iw + sx) * 4) as usize..((sy * iw + sx) * 4 + 4) as usize];
            if p[3] == 0 {
                continue;
            }
            let a = (p[3] as f32 / 255.0) * alpha;
            for dx in 0..scale {
                for dy in 0..scale {
                    let px = x + (sx as i32 * scale) + dx;
                    let py = y + (sy as i32 * scale) + dy;
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
    }
}

fn fill_rect(buf: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, rgba: [u8; 4]) {
    let a = rgba[3] as f32 / 255.0;
    for yy in y..y + h {
        for xx in x..x + w {
            if xx < 0 || yy < 0 || xx >= buf.width() as i32 || yy >= buf.height() as i32 {
                continue;
            }
            let dst = buf.get_pixel_mut(xx as u32, yy as u32);
            *dst = Rgba([
                (rgba[0] as f32 * a + dst[0] as f32 * (1.0 - a)) as u8,
                (rgba[1] as f32 * a + dst[1] as f32 * (1.0 - a)) as u8,
                (rgba[2] as f32 * a + dst[2] as f32 * (1.0 - a)) as u8,
                255,
            ]);
        }
    }
}

fn main() {
    let data = client_bevy::resources::libraries::resolve_data_path();
    let mut libs = Libraries::new(&data);
    libs.ensure_initialized();

    // 画布：上行新实现 2x，下行旧实现 2x
    let s = 2;
    let mut buf: RgbaImage = ImageBuffer::from_pixel(640, 150, Rgba([24u8, 24, 32, 255]));

    // ---- 上行：新实现（C# 源码级）@ (10,10) ----
    let (nx, ny) = (10, 10);
    // C# BeforeDraw：格网 2193 @(+12,0) 50% 透明，画在底图之下
    blit_scaled(
        &mut buf,
        &mut libs,
        LibraryName::Prguse,
        2193,
        nx + 12 * s,
        ny,
        0.5,
        s,
    );
    // 底图 2190 @(0,0)
    blit_scaled(
        &mut buf,
        &mut libs,
        LibraryName::Prguse,
        2190,
        nx,
        ny,
        1.0,
        s,
    );
    // 切换绑定按钮 2247 @(0,0)
    blit_scaled(
        &mut buf,
        &mut libs,
        LibraryName::Prguse,
        2247,
        nx,
        ny,
        1.0,
        s,
    );
    // 4 个占用格放真实图标（MagIcon[icon*2] 自然尺寸），4 个空槽显示格网
    for (i, icon2) in [(0usize, 2usize), (1, 6), (2, 10), (3, 16)] {
        let cx = nx + (15 + 25 * i as i32) * s;
        let cy = ny + 3 * s;
        blit_scaled(
            &mut buf,
            &mut libs,
            LibraryName::MagIcon,
            icon2,
            cx,
            cy,
            1.0,
            s,
        );
    }

    // ---- 下行：旧实现（master 现状）@ (10,76) ----
    let (ox, oy) = (10, 76);
    // 底图 2190（216 宽）+ 8 个 34x28 45% 黑盒（38 步进，超出底图）+ 30x24 内缩图标
    blit_scaled(
        &mut buf,
        &mut libs,
        LibraryName::Prguse,
        2190,
        ox,
        oy,
        1.0,
        s,
    );
    for i in 0..8i32 {
        let bx = ox + i * 38 * s;
        fill_rect(&mut buf, bx, oy, 34 * s, 28 * s, [0, 0, 0, 115]);
    }
    for (i, icon) in [(0i32, 2usize), (1, 6), (2, 10), (3, 16)] {
        // 旧实现图标索引少了 *2（此处按旧行为 m.icon 直接索引展示差异）
        let _ = icon;
        let ix = ox + i * 38 * s + 2 * s;
        let iy = oy + 2 * s;
        blit_scaled(
            &mut buf,
            &mut libs,
            LibraryName::MagIcon,
            [1, 3, 5, 8][i as usize],
            ix,
            iy,
            1.0,
            s,
        );
    }

    std::fs::create_dir_all("verify-out").ok();
    buf.save("verify-out/skillbar_compare.png")
        .expect("save png");
    println!("saved verify-out/skillbar_compare.png (上=新 C# 源码级, 下=旧)");
}
