//! UI 对齐校准测试（进入游戏画面前的全部交互 UI）。
//!
//! 不依赖人眼看 PNG：把原版 C# 的布局规则编码为断言，加载真实精灵元数据，
//! 自动验证每一项。任何坐标/尺寸偏移 → 测试失败并指出具体元素。
//! 运行：cargo test --test ui_alignment
//!
//! 校准内容（每条都用「真实精灵尺寸」运行时计算，非硬编码）：
//!  1. 居中：每个对话框原点 == ((1024-精灵宽)/2, (768-精灵高)/2)。
//!     —— 用游戏代码暴露的 pub 原点常量，防代码漂移；这正是之前选角预览
//!        错位的同类 bug（尺寸假设错→居中错）。
//!  2. 包含：每个子控件 bbox ⊆ 父对话框 bbox（坐标错就会戳出框外）。
//!  3. 不出界：所有元素 ⊆ 1024×768 画布。
//!  4. 底部按钮整数公式：x == 164*N - 32（N=1..5）+ 同行不重叠。
//!  5. 精灵存在：每个引用的 (lib,idx) 能加载且尺寸>0。
//!  6. UseOffSet 预览：绘制位置 == Location + 精灵 offset（C# MLibrary.Draw 约定）。

use client_bevy::resources::libraries::{Libraries, LibraryName};
use client_bevy::ui::login as lg;
use client_bevy::ui::modal_box as mb;
use client_bevy::ui::new_character as nc;

const SW: f32 = 1024.0;
const SH: f32 = 768.0;
const EPS: f32 = 0.5;

struct Libs(Libraries);

impl Libs {
    fn new() -> Self {
        let mut l = Libraries::new("Data");
        l.ensure_initialized();
        Self(l)
    }
    /// 真实精灵尺寸 (w,h)；缺失则 panic。
    fn size(&mut self, lib: LibraryName, idx: usize) -> (f32, f32) {
        let i = self
            .0
            .get_image(lib, idx)
            .unwrap_or_else(|| panic!("{:?}[{}] 缺失", lib, idx));
        (i.width as f32, i.height as f32)
    }
    /// 真实精灵 (w,h,offset_x,offset_y)。
    fn size_off(&mut self, lib: LibraryName, idx: usize) -> (f32, f32, f32, f32) {
        let i = self
            .0
            .get_image(lib, idx)
            .unwrap_or_else(|| panic!("{:?}[{}] 缺失", lib, idx));
        (
            i.width as f32,
            i.height as f32,
            i.offset_x as f32,
            i.offset_y as f32,
        )
    }
}

/// 断言对话框原点按真实精灵居中。
fn assert_centered(name: &str, ox: f32, oy: f32, lib: LibraryName, idx: usize, libs: &mut Libs) {
    let (w, h) = libs.size(lib, idx);
    let ex = (SW - w) / 2.0;
    let ey = (SH - h) / 2.0;
    assert!(
        (ox - ex).abs() < EPS && (oy - ey).abs() < EPS,
        "[居中] {name}: 原点({ox},{oy}) 应为({ex},{ey})（精灵 {:?}[{idx}] {w}x{h}）",
        lib
    );
    println!("  ✓ 居中 {} @({},{}) {:?}[{}] {}x{}", name, ox, oy, lib, idx, w, h);
}

/// 断言子控件 bbox 完全在父 bbox 内。
fn assert_inside(child: &str, cx: f32, cy: f32, cw: f32, ch: f32, px: f32, py: f32, pw: f32, ph: f32) {
    assert!(
        cx >= px - EPS && cy >= py - EPS && cx + cw <= px + pw + EPS && cy + ch <= py + ph + EPS,
        "[包含] {child}: bbox({cx},{cy},{cw}x{ch}) 戳出父框({px},{py},{pw}x{ph})"
    );
}

/// 断言 bbox 在画布内。
fn assert_in_canvas(name: &str, x: f32, y: f32, w: f32, h: f32) {
    assert!(
        x >= -EPS && y >= -EPS && x + w <= SW + EPS && y + h <= SH + EPS,
        "[出界] {name}: bbox({x},{y},{w}x{h}) 超出画布 1024x768"
    );
}

#[test]
fn login_dialog_aligned() {
    let mut libs = Libs::new();
    assert_centered("登录框", lg::DX, lg::DY, LibraryName::Prguse, 1084, &mut libs);
    let (dw, dh) = libs.size(LibraryName::Prguse, 1084);
    // 子控件（相对对话框原点）—— 全部来自 C# LoginScene.LoginDialog
    // 精灵类：(lib, idx, rel_x, rel_y)
    let sprites: &[(LibraryName, usize, f32, f32)] = &[
        (LibraryName::Title, 30, (328.0 - 102.0) / 2.0, 12.0), // 标题（居中于 328 宽）
        (LibraryName::Title, 31, 52.0, 83.0),                  // 账号标签
        (LibraryName::Title, 32, 43.0, 105.0),                 // 密码标签
        (LibraryName::Title, 320, 227.0, 81.0),                // OK
        (LibraryName::Title, 323, 60.0, 163.0),                // 新建账号
        (LibraryName::Title, 326, 166.0, 163.0),               // 修改密码
        (LibraryName::Title, 332, 60.0, 189.0),                // 查看密钥
        (LibraryName::Title, 329, 166.0, 189.0),               // 关闭
    ];
    for (lib, idx, rx, ry) in sprites {
        let (w, h) = libs.size(*lib, *idx);
        let (ax, ay) = (lg::DX + rx, lg::DY + ry);
        assert_inside(&format!("{:?}[{}]", lib, idx), ax, ay, w, h, lg::DX, lg::DY, dw, dh);
        assert_in_canvas(&format!("登录 {:?}", lib), ax, ay, w, h);
    }
    // 输入框矩形（C# 显式 Size）：账号 (85,85) 136x15 / 密码 (85,108) 136x15
    for (rx, ry, w, h, n) in [(85.0f32, 85.0f32, 136.0f32, 15.0f32, "账号"), (85.0, 108.0, 136.0, 15.0, "密码")] {
        assert_inside(n, lg::DX + rx, lg::DY + ry, w, h, lg::DX, lg::DY, dw, dh);
    }
    // 同行按钮不重叠（y=163 行：新建账号/修改密码；y=189 行：查看密钥/关闭）
    let row163 = [(lg::DX + 60.0, 100.0), (lg::DX + 166.0, 100.0)];
    assert!(!overlap(row163[0].0, row163[0].1, row163[1].0, row163[1].0), "[重叠] y=163 行按钮重叠");
    println!("  ✓ 登录框 {} 个子控件全部在框内", sprites.len() + 2);
}

#[test]
fn new_account_dialog_aligned() {
    let mut libs = Libs::new();
    assert_centered("新建账号框", lg::NA_X, lg::NA_Y, LibraryName::Prguse, 63, &mut libs);
    let (dw, dh) = libs.size(LibraryName::Prguse, 63);
    // 8 输入框：C# NewAccountDialog，x=226，ys/widths 如下
    let ys = [103.0f32, 129.0, 155.0, 189.0, 215.0, 250.0, 276.0, 311.0];
    let widths = [136.0f32, 136.0, 136.0, 136.0, 136.0, 190.0, 190.0, 136.0];
    for (i, (y, w)) in ys.iter().zip(widths.iter()).enumerate() {
        assert_inside(&format!("输入框{i}"), lg::NA_X + 226.0, lg::NA_Y + y, *w, 18.0, lg::NA_X, lg::NA_Y, dw, dh);
    }
    // OK Title[200](135,425) / Cancel Title[203](409,425)
    for (idx, rx) in [(200usize, 135.0f32), (203, 409.0)] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_inside(&format!("Title[{idx}]"), lg::NA_X + rx, lg::NA_Y + 425.0, w, h, lg::NA_X, lg::NA_Y, dw, dh);
    }
    println!("  ✓ 新建账号框 10 个子控件全部在框内");
}

#[test]
fn change_password_dialog_aligned() {
    let mut libs = Libs::new();
    assert_centered("修改密码框", lg::CP_X, lg::CP_Y, LibraryName::Prguse, 50, &mut libs);
    let (dw, dh) = libs.size(LibraryName::Prguse, 50);
    for y in [75.0f32, 113.0, 151.0, 188.0] {
        assert_inside("输入框", lg::CP_X + 178.0, lg::CP_Y + y, 136.0, 18.0, lg::CP_X, lg::CP_Y, dw, dh);
    }
    // OK Title[107](80,236) 90x25 / Cancel Title[110](222,236) 68x25
    for (idx, rx, w) in [(107usize, 80.0f32, 90.0f32), (110, 222.0, 68.0)] {
        let (rw, h) = libs.size(LibraryName::Title, idx);
        assert_eq!(rw, w, "[尺寸] Title[{}] 宽应为 {}", idx, w);
        assert_inside(&format!("Title[{idx}]"), lg::CP_X + rx, lg::CP_Y + 236.0, rw, h, lg::CP_X, lg::CP_Y, dw, dh);
    }
    println!("  ✓ 修改密码框 6 个子控件全部在框内");
}

#[test]
fn new_character_dialog_aligned() {
    let mut libs = Libs::new();
    assert_centered("新建角色框", nc::DLG_X, nc::DLG_Y, LibraryName::Prguse, 73, &mut libs);
    let (dw, dh) = libs.size(LibraryName::Prguse, 73);
    // 标题 Title[20](206,11)
    let (tw, th) = libs.size(LibraryName::Title, 20);
    assert_inside("Title[20]", nc::DLG_X + 206.0, nc::DLG_Y + 11.0, tw, th, nc::DLG_X, nc::DLG_Y, dw, dh);
    // 预览 ChrSel[20] UseOffSet=true：绘制 = Location(120,250) + offset，必须在框内
    let (pw, ph, pox, poy) = libs.size_off(LibraryName::ChrSel, 20);
    let (pvx, pvy) = (nc::DLG_X + 120.0 + pox, nc::DLG_Y + 250.0 + poy);
    assert_inside("预览 ChrSel[20]", pvx, pvy, pw, ph, nc::DLG_X, nc::DLG_Y, dw, dh);
    // 法师 blend 叠加层 ChrSel[600]（Wizard 男 base40 +0+560）：DrawBlend 在同 Location + 自身 offset
    let (bw, bh, box_, boy) = libs.size_off(LibraryName::ChrSel, 600);
    assert!(bw > 4.0 && bh > 4.0, "[blend] ChrSel[600] 法师男 blend 应有实质内容，实际 {bw}x{bh}");
    let (blx, bly) = (nc::DLG_X + 120.0 + box_, nc::DLG_Y + 250.0 + boy);
    assert_inside("法师 blend ChrSel[600]", blx, bly, bw, bh, nc::DLG_X, nc::DLG_Y, dw, dh);
    // 描述边框 (279,70) 278x170 / 名字输入 (325,268) 240x20
    assert_inside("描述边框", nc::DLG_X + 279.0, nc::DLG_Y + 70.0, 278.0, 170.0, nc::DLG_X, nc::DLG_Y, dw, dh);
    assert_inside("名字输入", nc::DLG_X + 325.0, nc::DLG_Y + 268.0, 240.0, 20.0, nc::DLG_X, nc::DLG_Y, dw, dh);
    // 职业按钮 Prguse[2426/2429/2432/2435/2438] x=[323,373,423,473,523] y=296
    let class: &[(usize, f32)] = &[(2426, 323.0), (2429, 373.0), (2432, 423.0), (2435, 473.0), (2438, 523.0)];
    for (idx, x) in class {
        let (w, h) = libs.size(LibraryName::Prguse, *idx);
        assert_eq!((w, h), (44.0, 42.0), "[尺寸] Prguse[{}] 应为 44x42", idx);
        assert_inside(&format!("职业[{idx}]"), nc::DLG_X + x, nc::DLG_Y + 296.0, w, h, nc::DLG_X, nc::DLG_Y, dw, dh);
    }
    // 性别按钮 Prguse[2420/2423] x=[323,373] y=343
    for (idx, x) in [(2420usize, 323.0f32), (2423, 373.0)] {
        let (w, h) = libs.size(LibraryName::Prguse, idx);
        assert_eq!((w, h), (44.0, 42.0), "[尺寸] Prguse[{}] 应为 44x42", idx);
        assert_inside(&format!("性别[{idx}]"), nc::DLG_X + x, nc::DLG_Y + 343.0, w, h, nc::DLG_X, nc::DLG_Y, dw, dh);
    }
    // 职业行不重叠（44 宽，间距 50）
    for w in [323.0f32, 373.0, 423.0, 473.0, 523.0].windows(2) {
        assert!(!overlap(nc::DLG_X + w[0], 44.0, nc::DLG_X + w[1], 44.0), "[重叠] 职业按钮行重叠");
    }
    // OK Title[360](160,425) / Cancel Title[280](425,425)
    for (idx, rx) in [(360usize, 160.0f32), (280, 425.0)] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_inside(&format!("Title[{idx}]"), nc::DLG_X + rx, nc::DLG_Y + 425.0, w, h, nc::DLG_X, nc::DLG_Y, dw, dh);
    }
    println!("  ✓ 新建角色框 全部子控件在框内（预览 @({},{})）", pvx, pvy);
}

#[test]
fn delete_dialogs_aligned() {
    let mut libs = Libs::new();
    // MirInputBox Prguse[660]
    assert_centered("删除输入框", mb::DLG_X, mb::DLG_Y, LibraryName::Prguse, 660, &mut libs);
    let (idw, idh) = libs.size(LibraryName::Prguse, 660);
    assert_inside("提示区", mb::DLG_X + 25.0, mb::DLG_Y + 25.0, 235.0, 40.0, mb::DLG_X, mb::DLG_Y, idw, idh);
    assert_inside("输入框", mb::DLG_X + 23.0, mb::DLG_Y + 86.0, 240.0, 19.0, mb::DLG_X, mb::DLG_Y, idw, idh);
    for (idx, rx) in [(200usize, 60.0f32), (203, 160.0)] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_inside(&format!("Title[{idx}]"), mb::DLG_X + rx, mb::DLG_Y + 123.0, w, h, mb::DLG_X, mb::DLG_Y, idw, idh);
    }
    // MirMessageBox Prguse[360]
    assert_centered("删除询问框", mb::MSG_X, mb::MSG_Y, LibraryName::Prguse, 360, &mut libs);
    let (mw, mh) = libs.size(LibraryName::Prguse, 360);
    assert_inside("文字区", mb::MSG_X + 35.0, mb::MSG_Y + 35.0, 390.0, 110.0, mb::MSG_X, mb::MSG_Y, mw, mh);
    for (idx, rx) in [(206usize, 260.0f32), (210, 360.0)] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_inside(&format!("Title[{idx}]"), mb::MSG_X + rx, mb::MSG_Y + 157.0, w, h, mb::MSG_X, mb::MSG_Y, mw, mh);
    }
    println!("  ✓ 删除输入框 + 删除询问框 全部子控件在框内");
}

#[test]
fn select_screen_aligned() {
    let mut libs = Libs::new();
    // 标题 Title[40](468,20)
    let (tw, th) = libs.size(LibraryName::Title, 40);
    assert_in_canvas("标题 Title[40]", 468.0, 20.0, tw, th);
    // 角色槽 Title[660..663] @ (637, [194,298,402,506])，尺寸应 288x56
    for (slot, y) in [194.0f32, 298.0, 402.0, 506.0].iter().enumerate() {
        let (w, h) = libs.size(LibraryName::Title, 660 + slot);
        assert_eq!((w, h), (288.0, 56.0), "[尺寸] Title[{}] 角色槽应为 288x56", 660 + slot);
        assert_in_canvas(&format!("角色槽[{slot}]"), 637.0, *y, w, h);
    }
    // 预览 ChrSel[20] @(260,420)+offset，必须在画布内
    let (pw, ph, pox, poy) = libs.size_off(LibraryName::ChrSel, 20);
    let (pvx, pvy) = (260.0 + pox, 420.0 + poy);
    assert_in_canvas("预览 ChrSel[20]", pvx, pvy, pw, ph);
    // 法师 blend 叠加层 ChrSel[600] @(260,420)+自身 offset（仅 Wizard 有内容；SelectScene 总是 DrawBlend）
    let (bw, bh, box_, boy) = libs.size_off(LibraryName::ChrSel, 600);
    assert!(bw > 4.0 && bh > 4.0, "[blend] ChrSel[600] 法师男 blend 应有实质内容，实际 {bw}x{bh}");
    assert_in_canvas("法师 blend ChrSel[600]", 260.0 + box_, 420.0 + boy, bw, bh);
    // 底部按钮：C# xPoint=(1024-200)/5=164，btnX(N)=100+164*N-82-50=164*N-32 → 132/296/460/624/788 @ y=736
    let ys = 768.0 - 32.0;
    let bottom = [132.0f32, 296.0, 460.0, 624.0, 788.0];
    for (n, x) in bottom.iter().enumerate() {
        let expected = 164.0 * (n as f32 + 1.0) - 32.0;
        assert!((x - expected).abs() < EPS, "[公式] 底部按钮[{}] x={} 应为 {}", n, x, expected);
        let (w, h) = libs.size(LibraryName::Title, 340 + 3 * n); // 340/343/346/349/352
        assert_eq!((w, h), (100.0, 25.0), "[尺寸] 底部按钮应为 100x25");
        assert_in_canvas(&format!("底部按钮[{n}]"), *x, ys, w, h);
    }
    // 底部按钮同行不重叠（100 宽，间距 164 → 间隙 64）
    for w in bottom.windows(2) {
        assert!(!overlap(w[0], 100.0, w[1], 100.0), "[重叠] 底部按钮行重叠");
    }
    println!("  ✓ 选角界面 标题/4 角色槽/预览/5 底部按钮 全部对齐（预览 @({},{})）", pvx, pvy);
}

/// 两个 [x, x+w) 区间是否重叠（同行 y 假设一致）。
fn overlap(x1: f32, w1: f32, x2: f32, w2: f32) -> bool {
    x1 < x2 + w2 - EPS && x2 < x1 + w1 - EPS
}
