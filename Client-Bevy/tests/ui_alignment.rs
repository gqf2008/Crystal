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

#[test]
fn game_chat_death_aligned() {
    let mut libs = Libs::new();

    // C# ChatDialog：面板背景 Prguse[2221] 632x68 @(230,671)
    let (cw, ch) = libs.size(LibraryName::Prguse, 2221);
    assert_eq!((cw, ch), (632.0, 68.0), "[尺寸] 聊天面板背景应为 632x68");
    assert_in_canvas("聊天面板背景", 230.0, 671.0, cw, ch);

    // 输入行 C# ChatTextBox @(1,54)，即绝对 (231,725)，627x13
    assert_inside(
        "聊天输入背景",
        231.0, 725.0, 627.0, 13.0,
        230.0, 671.0, cw, ch,
    );

    // 滚动按钮 C# @x=618, y=1/9/39/45，必须在面板内
    for (y, h) in [(1.0f32, 8.0f32), (9.0, 6.0), (39.0, 6.0), (45.0, 8.0)] {
        assert_inside(
            "聊天滚动按钮",
            230.0 + 618.0, 671.0 + y, 12.0, h,
            230.0, 671.0, cw, ch,
        );
    }

    // C# ShowReviveMessage → MirMessageBox(YesNo)：背景 Prguse[360] 居中
    assert_centered("死亡弹窗", 284.0, 289.0, LibraryName::Prguse, 360, &mut libs);
    let (dw, dh) = libs.size(LibraryName::Prguse, 360);
    assert_inside("死亡弹窗-是", 544.0, 446.0, 76.0, 25.0, 284.0, 289.0, dw, dh);
    assert_inside("死亡弹窗-否", 644.0, 446.0, 76.0, 25.0, 284.0, 289.0, dw, dh);

    println!("  ✓ 游戏内聊天面板 + 死亡弹窗 布局对齐");
}

/// 技能快捷栏（#2487 对齐 C# SkillBarDialog，MainDialogs.cs L1516-1744）。
/// 全部坐标引用 `client_bevy::game::skills` 的 pub 常量（防代码漂移），
/// 尺寸用真实精灵元数据运行时校验（基线 = 本次实测：2190=216x28、2193=204x28、2247=16x28、MagIcon/冷却帧=24x22）。
#[test]
fn skill_bar_aligned() {
    use client_bevy::game::skills as sk;
    let mut libs = Libs::new();

    // C# 源码字面值（MainDialogs.cs L1516-1744）——独立于被测代码的真源，
    // 代码常量漂移任意一个都会在这里变红：
    assert_eq!(sk::SKILL_BAR_W, 216.0, "栏宽 = Prguse[2190] 宽（实测 216）");
    assert_eq!(sk::SKILL_BAR_H, 28.0, "栏高 = Prguse[2190] 高（实测 28）");
    assert_eq!(sk::SKILL_GRID_OFFSET_X, 12.0, "格网偏移 = C# BeforeDraw DisplayLocation.X+12（L1659）");
    assert_eq!(sk::SKILL_SLOT_X, 15.0, "首格 x = C# i*25+15（L1565）");
    assert_eq!(sk::SKILL_SLOT_Y, 3.0, "格 y = C# L1565");
    assert_eq!(sk::SKILL_SLOT_STEP, 25.0, "格步进 = C# i*25（L1565）");
    assert_eq!(sk::SKILL_SLOT_W, 24.0, "格宽 = MagIcon 实测 24");
    assert_eq!(sk::SKILL_SLOT_H, 22.0, "格高 = MagIcon 实测 22");
    assert_eq!(sk::SKILL_KEY_X, 13.0, "键名标签 x = C# i*25+13（L1603）");
    assert_eq!(sk::SKILL_COOLDOWN_BASE, 1260, "冷却帧基址 = C# Index=1260+startFrame（L1741）");
    assert_eq!(sk::SKILL_COOLDOWN_FRAMES, 22, "冷却帧数 = C# totalFrames=22（L1721）");

    // 底图 Prguse[2190]：栏 bbox（默认 @(0,0)，C# Settings.SkillbarLocation 默认 {0,0}）
    let (bw, bh) = libs.size(LibraryName::Prguse, 2190);
    assert_eq!((bw, bh), (sk::SKILL_BAR_W, sk::SKILL_BAR_H), "[尺寸] 栏常量应等于底图实测");
    assert_in_canvas("技能栏底图", 0.0, 0.0, bw, bh);

    // 格网 Prguse[2193] @(+12,0)（BeforeDraw）：必须完整落在底图内
    let (gw, gh) = libs.size(LibraryName::Prguse, 2193);
    assert_inside("技能栏格网", sk::SKILL_GRID_OFFSET_X, 0.0, gw, gh, 0.0, 0.0, bw, bh);

    // 切换绑定按钮 Prguse[2247] @(0,0)：C# Size(16,28)
    let (sw, sh) = libs.size(LibraryName::Prguse, 2247);
    assert_eq!((sw, sh), (16.0, 28.0), "[尺寸] 切换绑定按钮应为 16x28（C# Size + 实测）");
    assert_inside("切换绑定按钮", 0.0, 0.0, sw, sh, 0.0, 0.0, bw, bh);

    // 8 技能格 @(i*25+15, 3) 24x22（C# Cells Location + MagIcon 自然尺寸）：全部落在底图内且互不重叠
    let (iw, ih) = libs.size(LibraryName::MagIcon, 0);
    assert_eq!((iw, ih), (sk::SKILL_SLOT_W, sk::SKILL_SLOT_H), "[尺寸] 格常量应等于 MagIcon 实测");
    for i in 0..8usize {
        let x = sk::SKILL_SLOT_X + i as f32 * sk::SKILL_SLOT_STEP;
        assert_inside("技能格", x, sk::SKILL_SLOT_Y, iw, ih, 0.0, 0.0, bw, bh);
        // 键名标签 @(i*25+13, 0)（C# Size 25x25）：落在底图内
        assert_inside("键名标签", sk::SKILL_KEY_X + i as f32 * sk::SKILL_SLOT_STEP, 0.0, 25.0, 25.0, 0.0, 0.0, bw, bh);
    }
    // 冷却帧 Prguse2[1260..=1282]（C# Index=1260+startFrame，startFrame∈[0,22] 共 23 帧）全部存在且为格尺寸
    for f in 0..=sk::SKILL_COOLDOWN_FRAMES {
        let (fw, fh) = libs.size(LibraryName::Prguse2, sk::SKILL_COOLDOWN_BASE + f);
        assert_eq!((fw, fh), (iw, ih), "[尺寸] 冷却帧 {} 应与技能格同尺寸", f);
    }

    println!("  ✓ 技能快捷栏 底图/格网/切换钮/8 格/键名标签/23 冷却帧 布局对齐");
}

/// HUD 底栏文本标签（#2497 对齐 C# MainDialog，MainDialogs.cs L13+ 构造器/Update）。
/// 坐标引用 `client_bevy::game::hud` 的 pub 常量（防代码漂移），底栏/球/经验条用真实精灵尺寸
/// （基线 = 本次实测：Prguse[1]=1024x152、Prguse[4]=104x80、Prguse[8]=1004x8）。
#[test]
fn hud_labels_aligned() {
    use client_bevy::game::hud;
    let mut libs = Libs::new();

    // C# 源码字面值（MainDialogs.cs）——独立于被测代码的真源，常量漂移任意一个即红：
    // (常量值, C# 字面值, 说明)
    let consts: &[(f32, f32, &str)] = &[
        (hud::HUD_LEVEL_X, 5.0, "LevelLabel.x = C# (5,108)"),
        (hud::HUD_LEVEL_Y, 108.0, "LevelLabel.y = C# (5,108)"),
        (hud::HUD_NAME_X, 6.0, "CharacterName.x = C# (6,120)"),
        (hud::HUD_NAME_Y, 120.0, "CharacterName.y = C# (6,120)"),
        (hud::HUD_NAME_W, 90.0, "CharacterName 框宽 = C# 90"),
        (hud::HUD_NAME_H, 16.0, "CharacterName 框高 = C# 16"),
        (hud::HUD_GOLD_DX, 105.0, "GoldLabel 距右 = C# Width-105"),
        (hud::HUD_GOLD_Y, 119.0, "GoldLabel.y = C# 119"),
        (hud::HUD_ORB_CX, 50.0, "球体标签居中 x = C# 球心 50"),
        (hud::HUD_HP_ORB_Y, 27.0, "HealthLabel 球体相对 y = C# 27"),
        (hud::HUD_MP_ORB_Y, 42.0, "ManaLabel 球体相对 y = C# 42"),
        (
            hud::HUD_EXP_LABEL_DY,
            10.0,
            "ExperienceLabel 条上方 = C# -10",
        ),
        (
            hud::HUD_EXP_LABEL_DX,
            20.0,
            "ExperienceLabel 居中偏左 = C# -20",
        ),
    ];
    for (got, want, what) in consts {
        assert_eq!(got, want, "[常量] {what}");
    }

    // 底栏真实尺寸（Prguse[1]=1024x152）：main_x=(1024-w)/2, main_y=768-h；经验条宽 Prguse[8]=1004
    let (bw, bh) = libs.size(LibraryName::Prguse, 1);
    let (ew, _eh) = libs.size(LibraryName::Prguse, 8);
    let main_x = (SW - bw) / 2.0;
    let main_y = SH - bh;

    // 各标签落在底栏 bbox 内（文本按 12px 字号、给合理宽度框做包含校验）。
    // (名称, 栏内x, 栏内y, 估宽, 估高)；经验标签 x = 9+条宽/2-20、y = 143-10
    let exp_lx = 9.0 + ew / 2.0 - hud::HUD_EXP_LABEL_DX;
    let exp_ly = 143.0 - hud::HUD_EXP_LABEL_DY;
    let labels: &[(&str, f32, f32, f32, f32)] = &[
        ("LevelLabel", hud::HUD_LEVEL_X, hud::HUD_LEVEL_Y, 24.0, 12.0),
        (
            "CharacterName",
            hud::HUD_NAME_X,
            hud::HUD_NAME_Y,
            90.0,
            16.0,
        ),
        (
            "GoldLabel",
            bw - hud::HUD_GOLD_DX,
            hud::HUD_GOLD_Y,
            99.0,
            13.0,
        ),
        ("ExperienceLabel", exp_lx, exp_ly, 40.0, 12.0),
        (
            "HealthLabel",
            hud::HUD_ORB_CX - 30.0,
            30.0 + hud::HUD_HP_ORB_Y,
            60.0,
            12.0,
        ),
        (
            "ManaLabel",
            hud::HUD_ORB_CX - 30.0,
            30.0 + hud::HUD_MP_ORB_Y,
            60.0,
            12.0,
        ),
    ];
    for (name, rx, ry, w, h) in labels {
        assert_inside(
            name,
            main_x + rx,
            main_y + ry,
            *w,
            *h,
            main_x,
            main_y,
            bw,
            bh,
        );
    }

    println!("  ✓ HUD 底栏 Level/Name/Gold/Exp/HP/MP 标签位置对齐 C# MainDialog");
}

/// 角色对话框（#2503 对齐 C# CharacterDialog，CharacterDialog.cs / MirItemCell.cs）。
/// 装备格屏坐标 = 对话框(760,0) + CharacterPage(8,90) + 页内偏移；格子 C# 36x32。
/// 全部坐标引用 `client_bevy::game::dialogs::character` 的 pub 常量（防漂移），尺寸用真实精灵实测。
#[test]
fn character_dialog_aligned() {
    use client_bevy::game::dialogs::character as ch;
    let mut libs = Libs::new();

    // C# 源码字面值（CharacterDialog.cs / MirItemCell.cs）——独立于被测代码的真源，漂移即红：
    assert_eq!(
        ch::DIALOG_X,
        1024.0 - 264.0,
        "对话框原点 x = C# ScreenWidth-264"
    );
    assert_eq!(ch::DIALOG_Y, 0.0, "对话框原点 y = C# 0");
    assert_eq!(ch::PAGE_X, 8.0, "CharacterPage.x = C# (8,90)");
    assert_eq!(ch::PAGE_Y, 90.0, "CharacterPage.y = C# (8,90)");
    assert_eq!(ch::SLOT_W, 36.0, "装备格宽 = C# MirItemCell 36");
    assert_eq!(ch::SLOT_H, 32.0, "装备格高 = C# MirItemCell 32");
    assert_eq!(ch::NAME_CX, 132.0, "名字/行会框心 x = C# 264/2");
    assert_eq!(ch::NAME_CY, 22.0, "名字框心 y = C# 12+20/2");
    assert_eq!(ch::GUILD_CY, 48.0, "行会框心 y = C# 33+30/2");
    assert_eq!(ch::CLASS_IMG_X, 15.0, "ClassImage.x = C# (15,33)");
    assert_eq!(ch::CLASS_IMG_Y, 33.0, "ClassImage.y = C# (15,33)");
    // 装备格页内坐标抽查（C# 字面值，漂移即红）：Weapon/Stone/Mount
    assert_eq!(ch::EQUIP_SLOTS[0], (123.0, 7.0), "Weapon 页内 = C# (123,7)");
    assert_eq!(
        ch::EQUIP_SLOTS[12],
        (128.0, 242.0),
        "Stone 页内 = C# (128,242)"
    );
    assert_eq!(
        ch::EQUIP_SLOTS[13],
        (203.0, 62.0),
        "Mount 页内 = C# (203,62)"
    );

    // 对话框真实尺寸（Title[504]）
    let (dw, dh) = libs.size(LibraryName::Title, 504);

    // 14 装备格：屏坐标 = DIALOG+PAGE+页内偏移，36x32 落在对话框内
    for (ox, oy) in ch::EQUIP_SLOTS {
        let sx = ch::DIALOG_X + ch::PAGE_X + ox;
        let sy = ch::DIALOG_Y + ch::PAGE_Y + oy;
        assert_inside(
            "装备格",
            sx,
            sy,
            ch::SLOT_W,
            ch::SLOT_H,
            ch::DIALOG_X,
            ch::DIALOG_Y,
            dw,
            dh,
        );
    }

    // ClassImage Prguse[100] @ (15,33) 对话框相对，落在对话框内
    let (cw, chh) = libs.size(LibraryName::Prguse, 100);
    assert_inside(
        "ClassImage",
        ch::DIALOG_X + ch::CLASS_IMG_X,
        ch::DIALOG_Y + ch::CLASS_IMG_Y,
        cw,
        chh,
        ch::DIALOG_X,
        ch::DIALOG_Y,
        dw,
        dh,
    );

    // 名字/行会框心锚点（对话框相对）落在对话框内
    assert_inside(
        "名字框心",
        ch::DIALOG_X + ch::NAME_CX,
        ch::DIALOG_Y + ch::NAME_CY,
        1.0,
        1.0,
        ch::DIALOG_X,
        ch::DIALOG_Y,
        dw,
        dh,
    );
    assert_inside(
        "行会框心",
        ch::DIALOG_X + ch::NAME_CX,
        ch::DIALOG_Y + ch::GUILD_CY,
        1.0,
        1.0,
        ch::DIALOG_X,
        ch::DIALOG_Y,
        dw,
        dh,
    );

    println!("  ✓ 角色对话框 14 装备格(36x32)/ClassImage/名字/行会 布局对齐 C# CharacterDialog");
}

#[test]
fn inventory_bigmap_aligned() {
    use client_bevy::game::dialogs::big_map as bm;
    use client_bevy::game::dialogs::inventory as inv;
    let mut libs = Libs::new();

    // ---- 背包（C# InventoryDialog.cs）常量 == C# 字面值（防漂移）----
    // 窗口原点：C# 构造器未设 Location（InventoryDialog.cs:25-31）→ MirControl
    // 默认 (0,0)（_location 零值，MirControl.cs:300）。旧 (182,217) 是 WeightBar
    // 局部坐标（:37）被误当原点。
    assert_eq!(inv::DIALOG_X, 0.0, "背包窗口 x = C# 无 Location → 0");
    assert_eq!(inv::DIALOG_Y, 0.0, "背包窗口 y = C# 无 Location → 0");
    assert_eq!(inv::GOLD_TEXT_X, 40.0, "金币 x = C# GoldLabel (40,212)");
    assert_eq!(inv::GOLD_TEXT_Y, 212.0, "金币 y = C# GoldLabel (40,212)");
    assert_eq!(
        inv::WEIGHT_TEXT_X,
        268.0,
        "负重 x = C# WeightLabel (268,212)"
    );
    assert_eq!(
        inv::WEIGHT_TEXT_Y,
        212.0,
        "负重 y = C# WeightLabel (268,212)"
    );
    assert_eq!(
        inv::ADD_BTN_W,
        72.0,
        "扩容命中宽 = C# AddButton Size(72,23)"
    );
    assert_eq!(
        inv::ADD_BTN_H,
        23.0,
        "扩容命中高 = C# AddButton Size(72,23)"
    );

    // 背包对话框真实尺寸（Title[196]），子控件 bbox ⊆ 对话框
    let (idw, idh) = libs.size(LibraryName::Title, 196);
    let (ix, iy) = (inv::DIALOG_X, inv::DIALOG_Y); // 背包窗口原点（C# 无 Location → (0,0)）
    assert_inside(
        "背包扩容按钮",
        ix + 235.0,
        iy + 5.0,
        inv::ADD_BTN_W,
        inv::ADD_BTN_H,
        ix,
        iy,
        idw,
        idh,
    );
    // 金币/负重文本原点（~13px 字高）⊆ 对话框
    assert_inside(
        "金币文本",
        ix + inv::GOLD_TEXT_X,
        iy + inv::GOLD_TEXT_Y,
        60.0,
        14.0,
        ix,
        iy,
        idw,
        idh,
    );
    assert_inside(
        "负重文本",
        ix + inv::WEIGHT_TEXT_X,
        iy + inv::WEIGHT_TEXT_Y,
        40.0,
        14.0,
        ix,
        iy,
        idw,
        idh,
    );

    // ---- 大地图（C# BigMapDialog.cs）常量 == C# 字面值（防漂移）----
    assert_eq!(bm::PANEL_W, 760.0, "大地图面板宽 = Title[820] 实测 760");
    assert_eq!(bm::PANEL_H, 500.0, "大地图面板高 = Title[820] 实测 500");
    assert_eq!(bm::SEARCH_X, 59.0, "搜索框 x = C# SearchTextBox (59,H-27)");
    assert_eq!(bm::SEARCH_Y_FROM_BOTTOM, 27.0, "搜索框底距 = C# H-27");
    assert_eq!(bm::SEARCH_W, 130.0, "搜索框宽 = C# Size(130,10)");
    assert_eq!(bm::SEARCH_H, 10.0, "搜索框高 = C# Size(130,10)");

    // 搜索框 ⊆ 大地图面板 + ⊆ 画布
    let (mx, my) = ((SW - bm::PANEL_W) / 2.0, (SH - bm::PANEL_H) / 2.0);
    let sb_y = my + bm::PANEL_H - bm::SEARCH_Y_FROM_BOTTOM;
    assert_inside(
        "大地图搜索框",
        mx + bm::SEARCH_X,
        sb_y,
        bm::SEARCH_W,
        bm::SEARCH_H,
        mx,
        my,
        bm::PANEL_W,
        bm::PANEL_H,
    );
    assert_in_canvas(
        "大地图搜索框",
        mx + bm::SEARCH_X,
        sb_y,
        bm::SEARCH_W,
        bm::SEARCH_H,
    );

    println!("  ✓ 背包金币/负重(212)+扩容命中(72x23)+窗口原点(0,0)、大地图搜索框(59,H-27,130x10) 对齐 C#");
}

/// 英雄背包窗口原点（C# HeroInventoryDialog 构造器未设 Location，HeroDialogs.cs:24-31
/// → MirControl 默认 (0,0)；旧实现按屏幕居中是自创行为）。
#[test]
fn hero_inventory_origin_aligned() {
    use client_bevy::game::dialogs::hero_inventory as hinv;
    assert_eq!(hinv::DIALOG_X, 0.0, "英雄背包窗口 x = C# 无 Location → 0");
    assert_eq!(hinv::DIALOG_Y, 0.0, "英雄背包窗口 y = C# 无 Location → 0");
    println!("  ✓ 英雄背包窗口原点 (0,0) 对齐 C#（非居中）");
}

#[test]
fn login_select_meta_aligned() {
    let mut libs = Libs::new();

    // LoginScene.Version：左下角 Build 标签 @(5, ScreenHeight-20)
    assert_in_canvas("登录版本标签", 5.0, 748.0, 220.0, 18.0);

    // LoginScene.TestLabel：Prguse[79] @(ScreenWidth-116, 10)，仅测试配置可见
    let (tw, th) = libs.size(LibraryName::Prguse, 79);
    assert!(tw > 0.0 && th > 0.0, "[尺寸] Prguse[79] 登录 TestLabel 应存在");
    assert_in_canvas("登录 TestLabel", 908.0, 10.0, tw, th);

    // SelectScene.ServerLabel：@(432,60)，宽约 155
    assert_in_canvas("选角服务器名", 432.0, 60.0, 155.0, 17.0);

    // SelectScene.LastAccessLabel：标题 @(200,609)，值 @(265,609)
    assert_in_canvas("选角最后登录标题", 200.0, 609.0, 100.0, 21.0);
    assert_in_canvas("选角最后登录值", 265.0, 609.0, 180.0, 21.0);

    println!("  ✓ 登录/选角元信息（版本/TestLabel/服务器名/最后登录）位置对齐");
}

/// 菜单对话框 + 耐久切换钮（对齐 C# MenuDialog / DuraStatusDialog，MainDialogs.cs）。
/// 两者都曾被硬编码的错误精灵尺寸假设带偏（菜单 Title[567] 误为 44x224、底栏误为 150；
/// 耐久钮漏算 +20 相对偏移且 y 用既非大也非小的 124）。这里锚定真实精灵尺寸。
#[test]
fn menu_dura_aligned() {
    use client_bevy::game::dialogs::dura_status as ds;
    use client_bevy::game::dialogs::menu as mu;
    let mut libs = Libs::new();

    // ---- 菜单（C# MenuDialog，MainDialogs.cs:3024-3029）常量 == C# 字面值/实测 ----
    assert_eq!(mu::MENU_W, 36.0, "菜单宽 = Title[567] 实测 36");
    assert_eq!(mu::MENU_H, 282.0, "菜单高 = Title[567] 实测 282");
    assert_eq!(mu::MAIN_DIALOG_H, 152.0, "主底栏高 = Prguse[1] 实测 152");
    assert_eq!(mu::MENU_X, 988.0, "菜单 x = ScreenWidth-Width = 1024-36");
    assert_eq!(
        mu::MENU_Y,
        349.0,
        "菜单 y = MainDialog.Y(616)-Height(282)+15"
    );
    assert_eq!(mu::MENU_BTN_DX, 3.0, "按钮相对 x = C# 按钮 Location.X=3");
    // 菜单背景 Title[567] 实测尺寸 == 常量，且 ⊆ 画布
    let (mw, mh) = libs.size(LibraryName::Title, 567);
    assert_eq!(
        (mw, mh),
        (mu::MENU_W, mu::MENU_H),
        "[尺寸] 菜单常量应等于 Title[567] 实测"
    );
    assert_in_canvas("菜单背景", mu::MENU_X, mu::MENU_Y, mw, mh);

    // ---- 耐久切换钮（C# DuraStatusDialog，MainDialogs.cs:3911,3919）----
    assert_eq!(ds::MINIMAP_X, 898.0, "小地图 x = ScreenWidth-126");
    assert_eq!(
        ds::MINIMAP_H_BIG,
        154.0,
        "小地图大模式高 = Prguse[2090] 实测 154"
    );
    assert_eq!(
        ds::MINIMAP_H_SMALL,
        45.0,
        "小地图小模式高 = Prguse[2091] 实测 45"
    );
    assert_eq!(ds::BTN_X, 1004.0, "耐久钮 x = MiniMap.X+86+20");
    assert_eq!(ds::dura_btn_y(true), 154.0, "大模式钮 y = 小地图大高 154");
    assert_eq!(ds::dura_btn_y(false), 45.0, "小模式钮 y = 小地图小高 45");
    // 切换钮 Prguse[2113] 实测 20x19，大/小模式均 ⊆ 画布
    let (bw, bh) = libs.size(LibraryName::Prguse, 2113);
    assert_eq!(
        (bw, bh),
        (20.0, 19.0),
        "[尺寸] 耐久钮应为 20x19（C# Size(20,19) + 实测）"
    );
    assert_in_canvas("耐久钮(大模式)", ds::BTN_X, ds::dura_btn_y(true), bw, bh);
    assert_in_canvas("耐久钮(小模式)", ds::BTN_X, ds::dura_btn_y(false), bw, bh);

    println!("  ✓ 菜单背景(988,349) Title[567]=36x282、耐久钮(1004, 小地图高154/45) 对齐 C#");
}

/// 模式标签（C# AMode/PMode/SModeLabel，MainDialogs.cs:2082-2087 MiniMapDialog.Process 每帧定位）。
/// X = MiniMap.X-3 = 898-3 = 895；顶→底 S/A/P；y = 小地图高 + {-2,+13,+28}
/// （大模式 152/167/182、小模式 43/58/73；偏移 = Process 的 Height+{150,165,180} 再 -ScreenHeight(768)+MainDialog.Y(616)）。
#[test]
fn mode_labels_aligned() {
    use client_bevy::game::dialogs::dura_status as ds;
    use client_bevy::game::hud as h;

    // X == C# MiniMapDialog.X-3（与耐久钮同源 MiniMap.X = ScreenWidth-126 = 898）
    assert_eq!(h::MODE_LABEL_X, 895.0, "模式标签 x = MiniMap.X(898)-3");
    assert_eq!(
        h::MODE_LABEL_X,
        ds::MINIMAP_X - 3.0,
        "应与耐久钮同源 MiniMap.X"
    );
    // y 偏移 == C# Process 的 Height+{150,165,180} 再 -152（ScreenHeight-MainDialog.Y）
    assert_eq!(h::S_MODE_DY, -2.0, "SMode dy = 150-152");
    assert_eq!(h::A_MODE_DY, 13.0, "AMode dy = 165-152");
    assert_eq!(h::P_MODE_DY, 28.0, "PMode dy = 180-152");
    // 绝对 y（大/小模式）== C# 字面值（小地图高 154/45 + 偏移）
    assert_eq!(
        h::mode_label_y(true, h::S_MODE_DY),
        152.0,
        "大模式 SMode y=154-2"
    );
    assert_eq!(
        h::mode_label_y(true, h::A_MODE_DY),
        167.0,
        "大模式 AMode y=154+13"
    );
    assert_eq!(
        h::mode_label_y(true, h::P_MODE_DY),
        182.0,
        "大模式 PMode y=154+28"
    );
    assert_eq!(
        h::mode_label_y(false, h::S_MODE_DY),
        43.0,
        "小模式 SMode y=45-2"
    );
    assert_eq!(
        h::mode_label_y(false, h::A_MODE_DY),
        58.0,
        "小模式 AMode y=45+13"
    );
    assert_eq!(
        h::mode_label_y(false, h::P_MODE_DY),
        73.0,
        "小模式 PMode y=45+28"
    );
    // 顶→底顺序 S < A < P（C# 堆叠顺序；Bevy 旧版误为 S,P,A）
    assert!(
        h::mode_label_y(true, h::S_MODE_DY) < h::mode_label_y(true, h::A_MODE_DY)
            && h::mode_label_y(true, h::A_MODE_DY) < h::mode_label_y(true, h::P_MODE_DY),
        "[顺序] 模式标签应顶→底 S/A/P"
    );
    // ⊆ 画布（取栈顶 S 与栈底 P；宽按最长文本 ~100、高 ~12）
    for (name, big) in [("大", true), ("小", false)] {
        assert_in_canvas(
            &format!("模式标签S({name}模)"),
            h::MODE_LABEL_X,
            h::mode_label_y(big, h::S_MODE_DY),
            100.0,
            12.0,
        );
        assert_in_canvas(
            &format!("模式标签P({name}模)"),
            h::MODE_LABEL_X,
            h::mode_label_y(big, h::P_MODE_DY),
            100.0,
            12.0,
        );
    }

    println!("  ✓ 模式标签 x=895、顶→底 S/A/P、y=152/167/182(大) 43/58/73(小) 对齐 C# Process");
}

/// 罗盘容器位置 + 40 帧公式（C# CompassDialog.cs:14,52-61）
#[test]
fn compass_aligned() {
    use client_bevy::game::dialogs::compass as c;

    // 容器位置 == C# 字面值（ScreenWidth/2-25、ScreenHeight/2-120）
    assert_eq!(c::COMPASS_X, 487.0, "罗盘 x = 512-25");
    assert_eq!(
        c::COMPASS_X,
        1024.0 / 2.0 - 25.0,
        "罗盘 x = ScreenWidth/2-25"
    );
    assert_eq!(c::COMPASS_Y, 264.0, "罗盘 y = 384-120");
    assert_eq!(
        c::COMPASS_Y,
        768.0 / 2.0 - 120.0,
        "罗盘 y = ScreenHeight/2-120"
    );
    assert_eq!(c::COMPASS_BASE, 1470, "首帧 Prguse2[1470]");
    assert_eq!(c::COMPASS_FRAMES, 40, "40 帧 = 40/360*degree");
    // 帧公式字面值：玩家 (100,100) 8 方位（degree = (atan2(-xDiff,yDiff)*180/PI+360)%360）
    let cases: [((i32, i32), usize); 8] = [
        ((100, 90), 1470),  // N
        ((110, 90), 1475),  // NE
        ((110, 100), 1480), // E
        ((110, 110), 1485), // SE
        ((100, 110), 1490), // S
        ((90, 110), 1495),  // SW
        ((90, 100), 1500),  // W
        ((90, 90), 1505),   // NW
    ];
    for ((tx, ty), want) in cases {
        assert_eq!(
            c::compass_index(100, 100, tx, ty),
            want,
            "目标 ({tx},{ty}) 帧"
        );
    }
    // ⊆ 画布（最坏情况：容器 + 最大帧偏移 (9,10) + 最大帧 28x27，探针实测上限）
    assert_in_canvas("罗盘", c::COMPASS_X, c::COMPASS_Y, 9.0 + 28.0, 10.0 + 27.0);

    println!("  ✓ 罗盘容器 (487,264)、40 帧公式、偏移落位对齐 C# CompassDialog");
}

/// 血球两行标签位置（C# MainDialogs.cs:223-237 TopLabel/BottomLabel：HealthOrb 相对 (9,20)/(9,50)，85x30 框内水平居中）
#[test]
fn hud_two_line_labels_aligned() {
    use client_bevy::game::hud as h;

    assert_eq!(h::HUD_2LINE_CX, 51.5, "两行标签水平中心 = 9+85/2");
    assert_eq!(
        h::HUD_2LINE_CX,
        9.0 + 85.0 / 2.0,
        "Location.X(9) + Size.Width(85)/2"
    );
    assert_eq!(
        h::HUD_TOP_LABEL_DY,
        20.0,
        "TopLabel Location (9,20) 距 HealthOrb"
    );
    assert_eq!(
        h::HUD_BOTTOM_LABEL_DY,
        50.0,
        "BottomLabel Location (9,50) 距 HealthOrb"
    );
    // ⊆ 画布（HUD 底栏 MainDialog.Y=616、HealthOrb 相对 (0,30)；框 85x30）
    for (name, dy) in [
        ("TopLabel", h::HUD_TOP_LABEL_DY),
        ("BottomLabel", h::HUD_BOTTOM_LABEL_DY),
    ] {
        assert_in_canvas(
            &format!("血球{name}"),
            h::HUD_2LINE_CX - 42.5,
            616.0 + 30.0 + dy,
            85.0,
            30.0,
        );
    }

    println!("  ✓ 血球两行标签 x=51.5、dy=20/50 对齐 C# TopLabel/BottomLabel");
}

/// 两个 [x, x+w) 区间是否重叠（同行 y 假设一致）。
fn overlap(x1: f32, w1: f32, x2: f32, w2: f32) -> bool {
    x1 < x2 + w2 - EPS && x2 < x1 + w1 - EPS
}

/// 批10 收尾：Craft 与 Refine 两窗的**资源索引**回归（此前只有模块内的常量断言，
/// 这里用真实 .Lib 元数据核对精灵存在性与尺寸）。
/// Craft：面板 `Prguse[1109]` 337x215、标题 `Title[18]`、AUTO `Title[180..182]` 48x25、
/// CRAFT `Title[336..338]` 80x25、关闭 `Prguse2[360..362]` 24x21；
/// Refine：材料窗 `Prguse[1002]` 164x207、标题 `Title[18]`、投放窗 `Prguse[392]` 176x146。
#[test]
fn craft_refine_sprites_aligned() {
    use client_bevy::game::dialogs::craft as cf;
    use client_bevy::game::dialogs::refine as rf;
    let mut libs = Libs::new();

    // ---- Craft（C# CraftDialog，NPCDialogs.cs:2256）----
    let (cw, ch) = libs.size(LibraryName::Prguse, 1109);
    assert_eq!(
        (cw, ch),
        (cf::CRAFT_W, cf::CRAFT_H),
        "[尺寸] Craft 面板应 = Prguse[1109]"
    );
    let (tw, th) = libs.size(LibraryName::Title, 18);
    assert!(tw > 0.0 && th > 0.0, "[精灵] Craft 标题 Title[18] 应存在");
    for idx in [cf::CRAFT_AUTOFILL_INDEX, cf::CRAFT_AUTOFILL_INDEX + 1, cf::CRAFT_AUTOFILL_INDEX + 2]
    {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_eq!((w, h), (48.0, 25.0), "[尺寸] Title[{idx}] 应为 AUTO 键 48x25");
    }
    for idx in [cf::CRAFT_CONFIRM_INDEX, cf::CRAFT_CONFIRM_INDEX + 1, cf::CRAFT_CONFIRM_INDEX + 2] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_eq!((w, h), (80.0, 25.0), "[尺寸] Title[{idx}] 应为 CRAFT 键 80x25");
    }
    for idx in [360usize, 361, 362] {
        let (w, h) = libs.size(LibraryName::Prguse2, idx);
        assert_eq!((w, h), (24.0, 21.0), "[尺寸] Prguse2[{idx}] 应为关闭键 24x21");
    }
    assert_in_canvas("Craft 面板", 0.0, 0.0, cw, ch);
    assert!(cf::CRAFT_AUTOFILL_POS.0 + 48.0 <= cw, "AUTO 键越出面板");
    assert!(cf::CRAFT_CONFIRM_POS.0 + 80.0 <= cw, "CRAFT 键越出面板");
    assert!(cf::CRAFT_CLOSE_POS.0 + 24.0 <= cw, "关闭键越出面板");

    // ---- Refine（C# RefineDialog，NPCDialogs.cs:2726）----
    let (rw, rh) = libs.size(LibraryName::Prguse, 1002);
    assert_eq!(
        (rw, rh),
        (rf::REFINE_W, rf::REFINE_H),
        "[尺寸] Refine 材料窗应 = Prguse[1002]"
    );
    let (lx, ly) = rf::refine_cell_pos(rf::REFINE_MATERIAL_SLOTS - 1);
    assert!(
        lx + rf::REFINE_CELL_W <= rw && ly + rf::REFINE_CELL_H <= rh,
        "[包含] 最后一个材料格越出 Prguse[1002]"
    );
    // 投放窗（sell_panel 的 Refine/CheckRefine 模式）
    let (dw, dh) = libs.size(LibraryName::Prguse, 392);
    assert!(dw > 0.0 && dh > 0.0, "[精灵] 投放窗 Prguse[392] 应存在");

    println!("  ✓ Craft Prguse[1109]/Title[180..182]/[336..338]/Prguse2[360..362] 与 Refine Prguse[1002]/[392] 资源索引对齐 C#");
}

/// TrustMerchant 买/取回确认框（C# `MirMessageBox` YesNo）：`Prguse[360]` 456x190 居中
/// (284,289)、文本框 (35,35)、Yes `Title[206..208]`(260,157)、No `Title[210..212]`(360,157)，均 76x25。
#[test]
fn trust_merchant_confirm_box_aligned() {
    use client_bevy::game::dialogs::market as mk;
    let mut libs = Libs::new();
    let (w, h) = libs.size(LibraryName::Prguse, 360);
    assert_eq!(
        (w, h),
        (mk::TM_CONFIRM_W, mk::TM_CONFIRM_H),
        "[尺寸] 确认框背景应 = Prguse[360]"
    );
    assert_centered(
        "确认框",
        mk::TM_CONFIRM_POS.0,
        mk::TM_CONFIRM_POS.1,
        LibraryName::Prguse,
        360,
        &mut libs,
    );
    for idx in [206usize, 207, 208, 210, 211, 212] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_eq!(
            (w, h),
            (mk::TM_CONFIRM_BTN_W, mk::TM_CONFIRM_BTN_H),
            "[尺寸] Title[{idx}] 应等于确认框 Yes/No 键尺寸"
        );
    }
    // Yes/No 键都在 456x190 面板内
    let inside = |(x, y): (f32, f32)| {
        x + mk::TM_CONFIRM_BTN_W <= mk::TM_CONFIRM_W && y + mk::TM_CONFIRM_BTN_H <= mk::TM_CONFIRM_H
    };
    assert!(inside(mk::TM_CONFIRM_YES_POS));
    assert!(inside(mk::TM_CONFIRM_NO_POS));
    assert!(
        mk::TM_CONFIRM_YES_POS.0 < mk::TM_CONFIRM_NO_POS.0,
        "Yes 在 No 左侧"
    );
    // 文案（C# `ItemNotSoldGetBack` / `ConfirmBuyItemWithPrice`）
    assert_eq!(
        mk::market_retrieve_text("屠龙"),
        "屠龙尚未售出，确定要取回它吗？"
    );
    assert_eq!(
        mk::market_buy_text("屠龙", 12_345, "金币"),
        "确定要以12,345 金币购买屠龙吗？"
    );

    println!("  ✓ 确认框 Prguse[360] 456x190 @(284,289) + Yes/No Title[206..212] 76x25 对齐 C# MirMessageBox");
}

/// TrustMerchant 价格排序图标与 Mail 按钮：
/// `PriceFilterIcon` = `Prguse2[925/926]` 12x11 @(371,65)（落在「价格」表头 295..383 内），
/// `MailButton` = `Prguse[437..439]` 28x25 @(350,448)（仅市场页签，与仅寄售的 COLLECT 不同页签）。
#[test]
fn trust_merchant_price_filter_and_mail_aligned() {
    use client_bevy::game::dialogs::market as mk;
    let mut libs = Libs::new();

    for idx in [mk::TM_PRICE_ICON_LOW, mk::TM_PRICE_ICON_HIGH] {
        let (w, h) = libs.size(LibraryName::Prguse2, idx);
        assert_eq!(
            (w, h),
            (mk::TM_PRICE_ICON_W, mk::TM_PRICE_ICON_H),
            "[尺寸] Prguse2[{idx}] 应等于价格排序图标尺寸"
        );
    }
    for idx in [437usize, 438, 439] {
        let (w, h) = libs.size(LibraryName::Prguse, idx);
        assert_eq!(
            (w, h),
            (mk::TM_MAIL_W, mk::TM_MAIL_H),
            "[尺寸] Prguse[{idx}] 应等于 Mail 键尺寸"
        );
    }
    // 图标锚点（C# `TitlePriceLabel.Location + Size` 推导）与表头命中区
    assert_eq!(mk::TM_PRICE_HEADER_POS, (295.0, 60.0));
    assert_eq!(mk::TM_PRICE_ICON_POS, (371.0, 65.0));
    assert!(
        mk::TM_PRICE_ICON_POS.0 + mk::TM_PRICE_ICON_W <= 383.0,
        "图标越出「价格」表头"
    );
    // Mail 键位置与底栏「价格」列一致（C# 350,448）；与 COLLECT(300,448,仅寄售) 不同页签共存
    assert_eq!(mk::TM_MAIL_POS, (350.0, 448.0));
    assert_eq!(mk::TM_COLLECT_SOLD_POS.1, mk::TM_MAIL_POS.1, "同底栏行");
    assert!(mk::TM_MAIL_POS.0 >= mk::TM_COLLECT_SOLD_POS.0);
    assert_in_canvas(
        "市场写信键",
        mk::TM_MAIL_POS.0,
        mk::TM_MAIL_POS.1,
        mk::TM_MAIL_W,
        mk::TM_MAIL_H,
    );
    // 三态循环与图标帧（C# `CyclePriceFilter` / `UpdatePriceFilterIcon`）
    use client_bevy::game::dialogs::market::MarketPriceFilter as F;
    assert_eq!(F::Normal.next().next(), F::High);
    assert_eq!(F::High.next(), F::Normal);
    assert_eq!(F::Low.icon_frame(), Some(mk::TM_PRICE_ICON_LOW));
    assert_eq!(F::High.icon_frame(), Some(mk::TM_PRICE_ICON_HIGH));

    println!("  ✓ 价格排序图标 Prguse2[925/926] @(371,65) 与 Mail 键 Prguse[437..439] @(350,448) 对齐 C#");
}

/// TrustMerchant 列表行（C# `AuctionRow`：行 (127, 82+i*33) 354x32 + 34x32 图标区 +
/// 名称/价格/卖家/到期 4 标签 + 选中橙框；空数量用 `Prguse[540]` 占位）。
#[test]
fn trust_merchant_rows_aligned() {
    use bevy::prelude::Color;
    use client_bevy::game::dialogs::market as mk;
    let mut libs = Libs::new();

    // 行/图标区/步进（C# `AuctionRow.Size`/`IconArea`/`Rows[i].Location`）
    assert_eq!((mk::TM_ROW_X, mk::TM_ROW_Y), (127.0, 82.0));
    assert_eq!((mk::TM_ROW_W, mk::TM_ROW_H), (354.0, 32.0));
    assert_eq!(mk::TM_ROW_STEP, 33.0);
    assert_eq!((mk::TM_ROW_ICON_W, mk::TM_ROW_ICON_H), (34.0, 32.0));
    // 行内标签锚点（C# `NameLabel`(38,8)/`PriceLabel`(170,8)/`SellerLabel`(256,0)/`ExpireLabel`(256,14)）
    assert_eq!(mk::TM_ROW_NAME_POS, (38.0, 8.0));
    assert_eq!(mk::TM_ROW_PRICE_POS, (170.0, 8.0));
    assert_eq!(mk::TM_ROW_SELLER_POS, (256.0, 0.0));
    assert_eq!(mk::TM_ROW_EXPIRE_POS, (256.0, 14.0));
    // 选中框颜色（C# `BorderColour = FromArgb(255,200,100,0)`）
    assert_eq!(mk::TM_ROW_BORDER_COLOR, Color::srgb_u8(200, 100, 0));

    // 空数量占位图标 `Prguse[540]` 存在（实测 24x16）
    let (pw, ph) = libs.size(LibraryName::Prguse, mk::TM_ROW_PLACEHOLDER_FRAME);
    assert!(pw > 0.0 && ph > 0.0, "[精灵] Prguse[540] 应存在");
    assert_eq!((pw, ph), (24.0, 16.0), "[尺寸] Prguse[540] 应为 24x16");

    // 10 行都落在 `Title[786]` 面板内，且行宽不越过面板右边界
    let (pan_w, pan_h) = libs.size(LibraryName::Title, 786);
    let last_y = mk::TM_ROW_Y + 9.0 * mk::TM_ROW_STEP;
    assert!(mk::TM_ROW_X + mk::TM_ROW_W <= pan_w, "[包含] 行宽越界");
    assert!(last_y + mk::TM_ROW_H <= pan_h, "[包含] 末行越界");
    assert_in_canvas(
        "市场列表末行",
        mk::TM_ROW_X,
        last_y,
        mk::TM_ROW_W,
        mk::TM_ROW_H,
    );

    // 行文本/颜色（C# `AuctionRow.Update`）
    assert_eq!(mk::group_thousands(1_234_567), "1,234,567");
    assert_eq!(mk::row_price_text(2500, 1), "2,500 出价");
    assert_eq!(mk::row_price_color(10_000), Color::WHITE);
    assert_eq!(
        mk::row_seller_color("Sold", true),
        Color::srgb(1.0, 0.843, 0.0)
    );
    assert_eq!(mk::row_name_color(4), Color::WHITE);

    println!("  ✓ 市场行 (127,82+i*33) 354x32、图标区 34x32、Prguse[540] 占位与行内标签对齐 C#");
}

/// TrustMerchant 寄售/拍卖页签面板（C# `TMerchantDialog(type)`：`Title[787]` 背景 +
/// HelpLabel/ItemCell/PriceTextBox/SellItemButton/CollectSoldButton/SellNowButton + 5 个表头）。
#[test]
fn trust_merchant_consign_panel_aligned() {
    use client_bevy::game::dialogs::market as mk;
    let mut libs = Libs::new();

    // 两个页签背景都是 492x478（C# `Index = 786` / `787`）
    for idx in [786usize, 787] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_eq!(
            (w, h),
            (mk::TM_PANEL_W, mk::TM_PANEL_H),
            "[尺寸] Title[{idx}] 应为 492x478"
        );
    }
    // `SellItemButton`/`SellNowButton` 共用 Title[700..702]（52x25）
    for idx in [700usize, 701, 702] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_eq!(
            (w, h),
            (mk::TM_SELL_BTN_W, mk::TM_SELL_BTN_H),
            "[尺寸] Title[{idx}] 应等于寄售提交键尺寸"
        );
    }
    // `CollectSoldButton` Title[680..682]（72x25）
    for idx in [680usize, 681, 682] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_eq!(
            (w, h),
            (mk::TM_COLLECT_BTN_W, mk::TM_COLLECT_BTN_H),
            "[尺寸] Title[{idx}] 应等于领取已售键尺寸"
        );
    }
    // Buy 两套精灵都是 84x25
    for idx in [703usize, 704, 705, 706, 707, 708] {
        let (w, h) = libs.size(LibraryName::Title, idx);
        assert_eq!((w, h), (84.0, 25.0), "[尺寸] Title[{idx}] 应为 84x25");
    }

    // 锚点与 C# 字面值一致 + 全部落在面板内
    assert_eq!(mk::TM_HELP_POS, (8.0, 237.0));
    assert_eq!(mk::TM_CONSIGN_CELL_POS, (47.0, 104.0));
    assert_eq!(mk::TM_PRICE_POS, (15.0, 165.0));
    assert_eq!(mk::TM_SELL_ITEM_POS, (39.0, 188.0));
    assert_eq!(mk::TM_COLLECT_SOLD_POS, (300.0, 448.0));
    assert_eq!(mk::TM_SELL_NOW_POS, (324.0, 448.0));
    assert_in_canvas(
        "寄售物品格",
        mk::TM_CONSIGN_CELL_POS.0,
        mk::TM_CONSIGN_CELL_POS.1,
        mk::TM_CONSIGN_CELL_W,
        mk::TM_CONSIGN_CELL_H,
    );
    assert_in_canvas(
        "领取已售键",
        mk::TM_COLLECT_SOLD_POS.0,
        mk::TM_COLLECT_SOLD_POS.1,
        mk::TM_COLLECT_BTN_W,
        mk::TM_COLLECT_BTN_H,
    );
    // 寄售说明（115x205 @8,237）与表头（y=60/142）都不得越出 492x478 面板
    let (pw, ph) = libs.size(LibraryName::Title, 787);
    assert!(mk::TM_HELP_POS.0 + mk::TM_HELP_W <= pw);
    assert!(mk::TM_HELP_POS.1 + mk::TM_HELP_H <= ph);
    for (kind, x, y, w) in mk::TM_HEADERS {
        assert!(x + w <= pw, "[包含] 表头 {kind:?} 越界");
        assert!(y + 21.0 <= ph);
    }
    // 表头随页签换文案（寄售/拍卖）
    use client_bevy::game::dialogs::market::MarketHeader as H;
    use mir2_shared::enums::MarketPanelType;
    assert_eq!(
        mk::header_text(H::SalePrice, MarketPanelType::Consign),
        "出售价格"
    );
    assert_eq!(
        mk::header_text(H::SalePrice, MarketPanelType::Auction),
        "起始出价"
    );

    println!("  ✓ 寄售/拍卖面板 Title[787] + 680..682/700..708 精灵与锚点对齐 C#");
}

/// TrustMerchant 左列筛选树（C# `TrustMerchantDialog.SetupFilters/DrawFilters`）：
/// 主/子按钮 `Prguse2[920..923]`(100x22)、滚动条 `[197..199]`/`[207..209]`(12x12) 与
/// 拖动手柄 `[205/206]`(12x18) 的精灵存在性、锚点（x=108）、以及不越出 `Title[786]` 面板。
#[test]
fn trust_merchant_filter_tree_aligned() {
    use client_bevy::game::dialogs::market_filter as mf;
    let mut libs = Libs::new();

    // C# `Title[786]` 面板 492x478（筛选树与滚动条都必须落在面板内）
    let (pw, ph) = libs.size(LibraryName::Title, 786);
    assert_eq!(
        (pw, ph),
        (492.0, 478.0),
        "[尺寸] TrustMerchant 面板 = Title[786]"
    );

    // 行按钮精灵实测 100x22（C# 未设 Size → 精灵原始尺寸）
    for idx in [920usize, 921, 922, 923] {
        let (w, h) = libs.size(LibraryName::Prguse2, idx);
        assert_eq!(
            (w, h),
            (mf::FILTER_BTN_W, mf::FILTER_BTN_H),
            "[尺寸] Prguse2[{idx}] 应等于行按钮常量"
        );
    }
    // 滚动条精灵：上/下箭头 12x12、拖动手柄 12x18
    for idx in [197usize, 198, 199, 207, 208, 209] {
        let (w, h) = libs.size(LibraryName::Prguse2, idx);
        assert_eq!(
            (w, h),
            (mf::FILTER_ARROW_W, mf::FILTER_ARROW_H),
            "[尺寸] Prguse2[{idx}] 应等于上下翻箭头常量"
        );
    }
    for idx in [205usize, 206] {
        let (w, h) = libs.size(LibraryName::Prguse2, idx);
        assert_eq!(
            (w, h),
            (mf::FILTER_HANDLE_W, mf::FILTER_HANDLE_H),
            "[尺寸] Prguse2[{idx}] 应等于拖动手柄常量"
        );
    }

    // 锚点：按钮 x=7、首行 y=60、步进 20；滚动条 x=108（C# 字面值）
    assert_eq!((mf::FILTER_BTN_X, mf::FILTER_BTN_Y), (7.0, 60.0));
    assert_eq!(mf::FILTER_MAIN_STEP, 20.0);
    assert_eq!(mf::FILTER_BAR_X, 108.0);
    assert_eq!((mf::FILTER_UP_Y, mf::FILTER_DOWN_Y), (60.0, 429.0));
    // 行按钮与滚动条同列不重叠、且不侵入列表区（x≥130）
    assert!(mf::FILTER_BTN_X + mf::FILTER_BTN_W <= mf::FILTER_BAR_X);
    assert!(mf::FILTER_BAR_X + mf::FILTER_HANDLE_W <= 130.0);

    // 全部行槽 + 滚动条都落在面板内
    let last_row_y = mf::FILTER_BTN_Y + (mf::FILTER_MAX_LINES - 1) as f32 * mf::FILTER_MAIN_STEP;
    assert_in_canvas(
        "筛选树末行",
        mf::FILTER_BTN_X,
        last_row_y,
        mf::FILTER_BTN_W,
        mf::FILTER_BTN_H,
    );
    assert!(
        last_row_y + mf::FILTER_BTN_H <= ph,
        "[包含] 末行 y={last_row_y} 应落在面板高 {ph} 内"
    );
    assert!(mf::FILTER_DOWN_Y + mf::FILTER_ARROW_H <= ph);
    assert!(mf::FILTER_BAR_MAX_Y <= ph);

    // 行 y 递推与 C# `DrawFilters` 一致（展开「衣服」：主 20 步进 → +2 → 子 21 步进）
    let f = mf::setup_filters();
    let rows = mf::visible_rows(&f, 2, -1, 0, mf::FILTER_MAX_LINES);
    assert_eq!((rows[0].y, rows[1].y), (60.0, 80.0));
    assert_eq!(rows[3].y, 122.0, "[递推] 展开子列表首行 = 100 + 20 + 2");
    assert_eq!(rows[4].y, 143.0, "[递推] 子项步进 21");

    println!("  ✓ 筛选树 Prguse2[920..923]/[197..209] 尺寸与 (7,60)/108 锚点对齐 C#");
}

/// 物品租赁四窗（C# `ItemRentDialog`/`ItemRentingDialog`/`GuestItemRentDialog`/
/// `GuestItemRentingDialog`）：每端显示 1 自有窗 + 1 对方窗，四窗同源 `Prguse[238]`(204x109)，
/// 自有/对方同坐标（费用窗 y=163、物品窗 y=287，互补不重叠）；引用的精灵索引全部存在。
#[test]
fn item_rental_guest_windows_aligned() {
    use client_bevy::game::dialogs::item_rental as ir;
    let mut libs = Libs::new();

    // 面板 Prguse[238] 实测 204x109（C# 四窗同源）
    let (pw, ph) = libs.size(LibraryName::Prguse, 238);
    assert_eq!(
        (pw, ph),
        (204.0, 109.0),
        "[尺寸] 租赁面板应为 Prguse[238] 204x109"
    );

    // C# `Location`：(1024-204-102, 109+54)=163 与 (…, 218+54+15)=287
    let fee = ir::RentalWindow::OwnFee.pos();
    let item = ir::RentalWindow::OwnItem.pos();
    assert_eq!(
        fee,
        (718.0, 163.0),
        "[坐标] 费用窗 = (ScreenWidth-W-W/2, H+H/2)"
    );
    assert_eq!(item, (718.0, 287.0), "[坐标] 物品窗 = (…, H*2+H/2+15)");
    assert_eq!(
        ir::RentalWindow::GuestFee.pos(),
        fee,
        "对方费用窗与自有费用窗同坐标"
    );
    assert_eq!(
        ir::RentalWindow::GuestItem.pos(),
        item,
        "对方物品窗与自有物品窗同坐标"
    );
    assert_in_canvas("租赁费用窗", fee.0, fee.1, pw, ph);
    assert_in_canvas("租赁物品窗", item.0, item.1, pw, ph);
    assert!(
        !overlap(fee.0, pw, item.0, pw) || (fee.1 - item.1).abs() >= ph,
        "[重叠] 费用窗与物品窗不得重叠"
    );

    // C# `GameScene.ItemRentalRequest`：renting=false → 物主（物品窗+对方费用窗），true → 租客
    for (role, own, guest, neg) in [
        (
            ir::RentalRole::Owner,
            ir::RentalWindow::OwnItem,
            ir::RentalWindow::GuestFee,
            ir::RentalWindow::OwnFee,
        ),
        (
            ir::RentalRole::Renter,
            ir::RentalWindow::OwnFee,
            ir::RentalWindow::GuestItem,
            ir::RentalWindow::OwnItem,
        ),
    ] {
        assert!(own.shown_for(role), "[分流] {role:?} 应显示自有窗 {own:?}");
        assert!(
            guest.shown_for(role),
            "[分流] {role:?} 应显示对方窗 {guest:?}"
        );
        assert!(!neg.shown_for(role), "[分流] {role:?} 不应显示 {neg:?}");
        let shown = [
            ir::RentalWindow::OwnFee,
            ir::RentalWindow::OwnItem,
            ir::RentalWindow::GuestFee,
            ir::RentalWindow::GuestItem,
        ]
        .into_iter()
        .filter(|w| w.shown_for(role))
        .count();
        assert_eq!(shown, 2, "[分流] {role:?} 应恰好显示 2 个窗口");
    }

    // C# 引用到的精灵索引（关闭 / 价格 / 锁定 250..253 / 限期 / 确认）
    for (lib, idx) in [
        (LibraryName::Prguse2, 360usize),
        (LibraryName::Prguse2, 361),
        (LibraryName::Prguse2, 362),
        (LibraryName::Prguse, 28),
        (LibraryName::Prguse, 250),
        (LibraryName::Prguse, 251),
        (LibraryName::Prguse, 252),
        (LibraryName::Prguse, 253),
        (LibraryName::Prguse3, 7),
        (LibraryName::Prguse3, 8),
        (LibraryName::Prguse3, 9),
        (LibraryName::Prguse3, 10),
        (LibraryName::Prguse3, 11),
        (LibraryName::Prguse3, 12),
    ] {
        let (w, h) = libs.size(lib, idx);
        assert!(w > 0.0 && h > 0.0, "[精灵] {lib:?}[{idx}] 应存在且非空");
    }

    println!("  ✓ 租赁四窗 Prguse[238] 204x109 @163/287、自有/对方分流、锁定帧 250..253 齐全");
}
