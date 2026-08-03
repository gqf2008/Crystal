//! 验证 UI 布局：把进入游戏画面前的【所有】交互 UI 按代码坐标合成到 PNG。
//! 完全 CPU blit，绕过 flaky GPU（0xffffffff 崩溃），直接看布局对不对。
//! 运行：cargo run --example verify_ui_layout （工作目录 = Client-Bevy）
//!
//! 约定与游戏一致：屏幕坐标左上角原点、y 向下，逻辑分辨率 1024×768；
//! 精灵按 Anchor::TOP_LEFT（左上角落在 (x,y)）。仅 CharacterDisplay（角色预览）
//! 用 UseOffSet=true，绘制左上角 = Location + 精灵 offset（对齐 MLibrary.Draw）。
//! 文字不画（CPU 端无字体栅格）；输入框/边框用半透明矩形示意。
use client_bevy::resources::libraries::{Libraries, LibraryName};
use image::{ImageBuffer, Rgba, RgbaImage};

const W: u32 = 1024;
const H: u32 = 768;

/// 把 (lib, idx) 精灵以左上角 (x,y) alpha 叠加到 buf（越界裁剪）。
fn blit(buf: &mut RgbaImage, libs: &mut Libraries, lib: LibraryName, idx: usize, x: i32, y: i32) {
    let Some(info) = libs.get_image(lib, idx) else {
        eprintln!("  [skip] {:?}[{}] 缺失", lib, idx);
        return;
    };
    let Some(rgba) = info.rgba.as_ref() else {
        eprintln!("  [skip] {:?}[{}] 无像素数据", lib, idx);
        return;
    };
    let iw = info.width.max(0) as u32;
    let ih = info.height.max(0) as u32;
    if iw == 0 || ih == 0 || rgba.len() < (iw * ih * 4) as usize {
        return;
    }
    let src: RgbaImage = match ImageBuffer::from_raw(iw, ih, rgba.clone()) {
        Some(s) => s,
        None => return,
    };
    for sy in 0..ih {
        for sx in 0..iw {
            let px = x + sx as i32;
            let py = y + sy as i32;
            if !(0..W as i32).contains(&px) || !(0..H as i32).contains(&py) {
                continue;
            }
            let p = src.get_pixel(sx, sy);
            if p[3] == 0 {
                continue;
            }
            let a = p[3] as f32 / 255.0;
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

/// 读取 (lib,idx) 的 (width,height,offset_x,offset_y)。
fn info_of(libs: &mut Libraries, lib: LibraryName, idx: usize) -> (i32, i32, i32, i32) {
    libs.get_image(lib, idx)
        .map(|i| (i.width as i32, i.height as i32, i.offset_x as i32, i.offset_y as i32))
        .unwrap_or((0, 0, 0, 0))
}

fn new_buf() -> RgbaImage {
    ImageBuffer::from_pixel(W, H, Rgba([10, 10, 14, 255]))
}

const BG: Rgba<u8> = Rgba([10, 10, 14, 255]);

fn content(buf: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> usize {
    let mut n = 0;
    for sy in y..(y + h).min(H) {
        for sx in x..(x + w).min(W) {
            let p = buf.get_pixel(sx, sy);
            if (p[0] as i32 - BG[0] as i32).abs() > 12
                || (p[1] as i32 - BG[1] as i32).abs() > 12
                || (p[2] as i32 - BG[2] as i32).abs() > 12
            {
                n += 1;
            }
        }
    }
    n
}

fn check(label: &str, n: usize) {
    println!("  {:<46} {}", label, if n > 0 { "✓ 有内容" } else { "✗ 空" });
}

/// 半透明填充矩形（输入框底色）。
fn rect_filled(buf: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, c: Rgba<u8>) {
    for sy in 0..h {
        for sx in 0..w {
            let px = x + sx;
            let py = y + sy;
            if !(0..W as i32).contains(&px) || !(0..H as i32).contains(&py) {
                continue;
            }
            let a = c[3] as f32 / 255.0;
            let dst = buf.get_pixel_mut(px as u32, py as u32);
            *dst = Rgba([
                (c[0] as f32 * a + dst[0] as f32 * (1.0 - a)) as u8,
                (c[1] as f32 * a + dst[1] as f32 * (1.0 - a)) as u8,
                (c[2] as f32 * a + dst[2] as f32 * (1.0 - a)) as u8,
                255,
            ]);
        }
    }
}

/// 1px 边框（校验边框示意）。
fn rect_border(buf: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, c: Rgba<u8>) {
    rect_filled(buf, x, y, w, 1, c);
    rect_filled(buf, x, y + h - 1, w, 1, c);
    rect_filled(buf, x, y, 1, h, c);
    rect_filled(buf, x + w - 1, y, 1, h, c);
}

fn save(buf: &RgbaImage, name: &str) {
    let _ = image::DynamicImage::ImageRgba8(buf.clone()).into_rgb8().save(name);
    println!("[save] {}", name);
}

fn main() {
    let mut libs = Libraries::new("Data");
    libs.ensure_initialized();

    // ===================== 1. 登录界面 =====================
    let dx = 348.0f32;
    let dy = 274.0f32;
    let mut buf = new_buf();
    blit(&mut buf, &mut libs, LibraryName::ChrSel, 0, 0, 0); // 背景
    blit(&mut buf, &mut libs, LibraryName::Prguse, 1084, dx as i32, dy as i32); // 对话框 328x220
    let t30w = info_of(&mut libs, LibraryName::Title, 30).0 as f32;
    blit(&mut buf, &mut libs, LibraryName::Title, 30, (dx + (328.0 - t30w) / 2.0) as i32, (dy + 12.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 31, (dx + 52.0) as i32, (dy + 83.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 32, (dx + 43.0) as i32, (dy + 105.0) as i32);
    // 账号/密码输入框 136x15
    rect_filled(&mut buf, (dx + 85.0) as i32, (dy + 85.0) as i32, 136, 15, Rgba([180, 180, 200, 90]));
    rect_border(&mut buf, (dx + 85.0) as i32, (dy + 85.0) as i32, 136, 15, Rgba([0, 200, 0, 200]));
    rect_filled(&mut buf, (dx + 85.0) as i32, (dy + 108.0) as i32, 136, 15, Rgba([180, 180, 200, 90]));
    rect_border(&mut buf, (dx + 85.0) as i32, (dy + 108.0) as i32, 136, 15, Rgba([0, 200, 0, 200]));
    // 按钮
    for (idx, ox, oy) in [
        (320usize, 227f32, 81f32),  // OK
        (323, 60.0, 163.0),         // 新建账号
        (326, 166.0, 163.0),        // 修改密码
        (332, 60.0, 189.0),         // 查看密钥
        (329, 166.0, 189.0),        // 关闭
    ] {
        blit(&mut buf, &mut libs, LibraryName::Title, idx, (dx + ox) as i32, (dy + oy) as i32);
    }
    println!("[1/7] 登录界面 抽检：");
    check("对话框 (348,274,328,220)", content(&buf, 348, 274, 328, 220));
    check("标题 Title[30] (461,286,102,24)", content(&buf, 461, 286, 102, 24));
    check("OK 按钮 (575,355,48,48)", content(&buf, 575, 355, 48, 48));
    check("账号输入框 (433,359,136,15)", content(&buf, 433, 359, 136, 15));
    check("关闭按钮 (514,463,100,25)", content(&buf, 514, 463, 100, 25));
    save(&buf, "layout_login.png");

    // ===================== 2. 新建账号对话框 =====================
    // Prguse[63] 588x460 → origin (218,154)；8 输入框；OK Title[200](135,425)/Cancel Title[203](409,425)
    let nx = 218.0f32;
    let ny = 154.0f32;
    let mut buf = new_buf();
    blit(&mut buf, &mut libs, LibraryName::ChrSel, 0, 0, 0);
    blit(&mut buf, &mut libs, LibraryName::Prguse, 63, nx as i32, ny as i32);
    let ys = [103.0f32, 129.0, 155.0, 189.0, 215.0, 250.0, 276.0, 311.0];
    let widths = [136.0f32, 136.0, 136.0, 136.0, 136.0, 190.0, 190.0, 136.0];
    for i in 0..8 {
        rect_filled(&mut buf, (nx + 226.0) as i32, (ny + ys[i]) as i32, widths[i] as i32, 18, Rgba([180, 180, 200, 90]));
        rect_border(&mut buf, (nx + 226.0) as i32, (ny + ys[i]) as i32, widths[i] as i32, 18, Rgba([120, 120, 120, 200]));
    }
    blit(&mut buf, &mut libs, LibraryName::Title, 200, (nx + 135.0) as i32, (ny + 425.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 203, (nx + 409.0) as i32, (ny + 425.0) as i32);
    println!("[2/7] 新建账号对话框 抽检：");
    check("对话框 Prguse[63] (218,154,588,460)", content(&buf, 218, 154, 588, 460));
    check("账号输入(顶) (444,257,136,18)", content(&buf, 444, 257, 136, 18));
    check("问题输入(190宽) (444,404,190,18)", content(&buf, 444, 404, 190, 18));
    check("OK (353,579,76,25)", content(&buf, 353, 579, 76, 25));
    check("Cancel (627,579,76,25)", content(&buf, 627, 579, 76, 25));
    save(&buf, "layout_new_account.png");

    // ===================== 3. 修改密码对话框 =====================
    // Prguse[50] 348x268 → origin (338,250)；4 输入框；OK Title[107](80,236)/Cancel Title[110](222,236)
    let cx = 338.0f32;
    let cy = 250.0f32;
    let mut buf = new_buf();
    blit(&mut buf, &mut libs, LibraryName::ChrSel, 0, 0, 0);
    blit(&mut buf, &mut libs, LibraryName::Prguse, 50, cx as i32, cy as i32);
    let cys = [75.0f32, 113.0, 151.0, 188.0];
    for y in cys {
        rect_filled(&mut buf, (cx + 178.0) as i32, (cy + y) as i32, 136, 18, Rgba([180, 180, 200, 90]));
        rect_border(&mut buf, (cx + 178.0) as i32, (cy + y) as i32, 136, 18, Rgba([120, 120, 120, 200]));
    }
    blit(&mut buf, &mut libs, LibraryName::Title, 107, (cx + 80.0) as i32, (cy + 236.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 110, (cx + 222.0) as i32, (cy + 236.0) as i32);
    println!("[3/7] 修改密码对话框 抽检：");
    check("对话框 Prguse[50] (338,250,348,268)", content(&buf, 338, 250, 348, 268));
    check("账号输入 (516,325,136,18)", content(&buf, 516, 325, 136, 18));
    check("OK Title[107] (418,486,90,25)", content(&buf, 418, 486, 90, 25));
    check("Cancel Title[110] (560,486,68,25)", content(&buf, 560, 486, 68, 25));
    save(&buf, "layout_change_password.png");

    // ===================== 4. 选角界面 =====================
    let mut buf = new_buf();
    blit(&mut buf, &mut libs, LibraryName::Prguse, 65, 0, 0); // 背景 1024x768
    blit(&mut buf, &mut libs, LibraryName::Title, 40, 468, 20); // 标题
    let positions = [(637.0f32, 194.0f32), (637.0, 298.0), (637.0, 402.0), (637.0, 506.0)];
    for (slot, (x, y)) in positions.iter().enumerate() {
        blit(&mut buf, &mut libs, LibraryName::Title, 660 + slot, *x as i32, *y as i32);
    }
    // 预览 ChrSel[20] UseOffSet=true
    let (_, _, pox, poy) = info_of(&mut libs, LibraryName::ChrSel, 20);
    let (pvx, pvy) = (260 + pox, 420 + poy);
    blit(&mut buf, &mut libs, LibraryName::ChrSel, 20, pvx, pvy);
    let bottom = [(340usize, 132.0f32), (343, 296.0), (346, 460.0), (349, 624.0), (352, 788.0)];
    for (idx, x) in bottom {
        blit(&mut buf, &mut libs, LibraryName::Title, idx, x as i32, 736);
    }
    println!("[4/7] 选角界面 抽检：");
    check("标题 Title[40] (468,20,84,19)", content(&buf, 468, 20, 84, 19));
    check("角色槽[0] (637,194,288,56)", content(&buf, 637, 194, 288, 56));
    check("角色槽[3] (637,506,288,56)", content(&buf, 637, 506, 288, 56));
    check(&format!("预览 ChrSel[20] @({},{})", pvx, pvy), content(&buf, pvx.max(0) as u32, pvy.max(0) as u32, 180, 260));
    check("开始按钮 (132,736,100,25)", content(&buf, 132, 736, 100, 25));
    save(&buf, "layout_select.png");

    // ===================== 5. 新建角色对话框 =====================
    // Prguse[73] 588x460 → origin (218,154)
    let mut buf = new_buf();
    blit(&mut buf, &mut libs, LibraryName::Prguse, 65, 0, 0); // 选角背景
    blit(&mut buf, &mut libs, LibraryName::Prguse, 73, nx as i32, ny as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 20, (nx + 206.0) as i32, (ny + 11.0) as i32); // 标题
    // 预览 ChrSel[20] @(120,250)+offset，UseOffSet=true
    let (pvx, pvy) = ((nx + 120.0) as i32 + pox, (ny + 250.0) as i32 + poy);
    blit(&mut buf, &mut libs, LibraryName::ChrSel, 20, pvx, pvy);
    // 描述边框 (279,70) 278x170
    rect_border(&mut buf, (nx + 279.0) as i32, (ny + 70.0) as i32, 278, 170, Rgba([100, 100, 100, 220]));
    // 名字输入 (325,268) 240x20
    rect_filled(&mut buf, (nx + 325.0) as i32, (ny + 268.0) as i32, 240, 20, Rgba([180, 180, 200, 90]));
    rect_border(&mut buf, (nx + 325.0) as i32, (ny + 268.0) as i32, 240, 20, Rgba([0, 200, 0, 200]));
    // 职业按钮（初始：战士选中=2427，其余未选）y=296
    let class_btns = [(2427usize, 323.0f32), (2429, 373.0), (2432, 423.0), (2435, 473.0), (2438, 523.0)];
    for (idx, x) in class_btns {
        blit(&mut buf, &mut libs, LibraryName::Prguse, idx, (nx + x) as i32, (ny + 296.0) as i32);
    }
    // 性别按钮（初始：男选中=2421，女未选=2423）y=343
    blit(&mut buf, &mut libs, LibraryName::Prguse, 2421, (nx + 323.0) as i32, (ny + 343.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Prguse, 2423, (nx + 373.0) as i32, (ny + 343.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 360, (nx + 160.0) as i32, (ny + 425.0) as i32); // OK
    blit(&mut buf, &mut libs, LibraryName::Title, 280, (nx + 425.0) as i32, (ny + 425.0) as i32); // Cancel
    println!("[5/7] 新建角色对话框 抽检：");
    check("对话框 Prguse[73] (218,154,588,460)", content(&buf, 218, 154, 588, 460));
    check("标题 Title[20] (424,165,187,20)", content(&buf, 424, 165, 187, 20));
    check(&format!("预览 @({},{})", pvx, pvy), content(&buf, pvx.max(0) as u32, pvy.max(0) as u32, 150, 250));
    check("名字输入 (543,422,240,20)", content(&buf, 543, 422, 240, 20));
    check("战士按钮(选中) (541,450,44,42)", content(&buf, 541, 450, 44, 42));
    check("弓手按钮 (741,450,44,42)", content(&buf, 741, 450, 44, 42));
    check("OK Title[360] (378,579,60,25)", content(&buf, 378, 579, 60, 25));
    save(&buf, "layout_new_character.png");

    // ===================== 6. 删除询问框（MirMessageBox YesNo）=====================
    // Prguse[360] 456x190 → origin (284,289)；Yes Title[206](260,157)/No Title[210](360,157)
    let mx = 284.0f32;
    let my = 289.0f32;
    let mut buf = new_buf();
    blit(&mut buf, &mut libs, LibraryName::Prguse, 65, 0, 0);
    blit(&mut buf, &mut libs, LibraryName::Prguse, 360, mx as i32, my as i32);
    rect_border(&mut buf, (mx + 35.0) as i32, (my + 35.0) as i32, 390, 110, Rgba([100, 100, 100, 200])); // 文字区
    blit(&mut buf, &mut libs, LibraryName::Title, 206, (mx + 260.0) as i32, (my + 157.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 210, (mx + 360.0) as i32, (my + 157.0) as i32);
    println!("[6/7] 删除询问框 抽检：");
    check("对话框 Prguse[360] (284,289,456,190)", content(&buf, 284, 289, 456, 190));
    check("Yes Title[206] (544,446,76,25)", content(&buf, 544, 446, 76, 25));
    check("No Title[210] (644,446,76,25)", content(&buf, 644, 446, 76, 25));
    save(&buf, "layout_delete_ask.png");

    // ===================== 7. 删除确认输入框（MirInputBox）=====================
    // Prguse[660] 288x156 → origin (368,306)；输入(23,86)240x19；OK Title[200](60,123)/Cancel Title[203](160,123)
    let ix = 368.0f32;
    let iy = 306.0f32;
    let mut buf = new_buf();
    blit(&mut buf, &mut libs, LibraryName::Prguse, 65, 0, 0);
    blit(&mut buf, &mut libs, LibraryName::Prguse, 660, ix as i32, iy as i32);
    rect_border(&mut buf, (ix + 25.0) as i32, (iy + 25.0) as i32, 235, 40, Rgba([100, 100, 100, 200])); // 提示文字区
    rect_filled(&mut buf, (ix + 23.0) as i32, (iy + 86.0) as i32, 240, 19, Rgba([180, 180, 200, 90]));
    rect_border(&mut buf, (ix + 23.0) as i32, (iy + 86.0) as i32, 240, 19, Rgba([0, 255, 0, 220])); // 绿边框
    blit(&mut buf, &mut libs, LibraryName::Title, 200, (ix + 60.0) as i32, (iy + 123.0) as i32);
    blit(&mut buf, &mut libs, LibraryName::Title, 203, (ix + 160.0) as i32, (iy + 123.0) as i32);
    println!("[7/7] 删除确认输入框 抽检：");
    check("对话框 Prguse[660] (368,306,288,156)", content(&buf, 368, 306, 288, 156));
    check("输入框 (391,392,240,19)", content(&buf, 391, 392, 240, 19));
    check("OK Title[200] (428,429,76,25)", content(&buf, 428, 429, 76, 25));
    check("Cancel Title[203] (528,429,76,25)", content(&buf, 528, 429, 76, 25));
    save(&buf, "layout_delete_confirm.png");

    println!("\n完成。7 张 PNG：layout_login / _new_account / _change_password / _select / _new_character / _delete_ask / _delete_confirm");
}
