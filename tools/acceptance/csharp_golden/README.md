# 原版 C#「金标准」对照工具（Crystal）

目的：把**原版 C# 客户端/服务端**作为功能对齐的金标准，替代「只读 C# 源码推断」的做法：

1. 本机跑起原版服务端 + 原版客户端；
2. 逐窗截图 A/B 对拍（截图差分），用于判定我方 Bevy 客户端哪里不一致；
3. 用原版 `Server.Library.dll` 导出真实存档（账号/角色/物品），作为 C#→Rust 迁移演练夹具。

> 本目录只放工具，不放任何真实存档数据（导出物请留在本机工作目录）。

## 1. 非侵入式沙箱

原版安装目录（示例）`E:\Users\gxh\Desktop\游戏相关\Crystal\`：`Client\` 约 8.6 GB、`Server\` 约 1.1 GB，
**不要直接改原目录**（原版 `Server.MirDB/MirADB` 是真实存档；服务端运行会覆写）。
建一个沙箱（约 300 MB），大目录用 junction 指回原目录：

```powershell
$src='E:\Users\gxh\Desktop\游戏相关\Crystal'; $dst='<沙箱目录>'
New-Item -ItemType Directory -Force "$dst\Server","$dst\Client" | Out-Null
robocopy "$src\Server" "$dst\Server" /E /XD Maps /NFL /NDL /NJH /NJS /NP | Out-Null
robocopy "$src\Client" "$dst\Client" /E /XD Data Map Sound DirectX runtimes /NFL /NDL /NJH /NJS /NP | Out-Null
New-Item -ItemType Junction -Path "$dst\Server\Maps" -Target "$src\Server\Maps" | Out-Null
foreach($d in @('Data','Map','Sound','DirectX','runtimes')){ New-Item -ItemType Junction -Path "$dst\Client\$d" -Target "$src\Client\$d" | Out-Null }
```

端口与启动器（避免与本机 Rust e2e 服务端的 7000 冲突、避免启动器联网 patch）：

- `$dst\Server\Configs\Setup.ini`：`[Network] Port=7100`
- `$dst\Client\Mir2Config.ini`：`[Network] Port=7100`；`[Launcher] Enabled=False`（跳过 Crystal Mir2 Patcher，否则无外网时报 "Could not get Patch Information"）

启动（服务端窗口标题形如 `Total: N, Real: M`）：

```powershell
Start-Process "$dst\Server\Server.exe" -WorkingDirectory "$dst\Server"
Start-Process "$dst\Client\Client.exe" -WorkingDirectory "$dst\Client"
```

服务端启动成功的证据：`$dst\Server\Logs\Server\Server (<date>).log` 出现 `463 Maps Loaded` / `Envir Started` / `Network Started`。

## 2. 截图对拍

原版客户端是 SlimDX/Direct3D 渲染：`PrintWindow`/`CopyFromScreen` 都拿不到画面，
但客户端自带截图（`Screenshots\Image N.png`，直接取 D3D backbuffer）——
按 `PrintScreen`（KeyBinds.ini `[Screenshot]`）或按本目录驱动脚本的 `Shot-Cs`。

```powershell
. .\csharp_client_driver.ps1 -SandboxRoot <沙箱目录>
Init-CsClient          # 找到游戏窗口并置顶
Shot-Cs 'scene01'      # 触发截图并复制到 <沙箱>\shots\orig_scene01.png
Move-Image 460 448     # 光标移到「游戏内图像坐标」(1024x768 逻辑坐标)
Click-Image 460 448    # 真实鼠标点击（见下方限制）
Get-CsEdits            # 列出镜像 UI 的 WinForms 文本框（定位输入框用）
```

差分：

```powershell
py -3.12 .\shot_diff.py a.png b.png [x0,y0,x1,y1]
```

### 2.1 键盘路径：锁屏也能进游戏（2026-09-24 实测跑通）

`csharp_kbd_login.ps1` 全程只用键盘消息（无需鼠标），已实测走通
**登录 → 角色选择 → 进入游戏 → F9 背包 / F10 装备 / F11 技能**：

```powershell
pwsh -NoProfile -File .\csharp_kbd_login.ps1 -SandboxRoot <沙箱目录> -Account 333 -Password <pw>
```

依据（原版 C# 源码）：

- `Client/MirScenes/LoginScene.cs:481` `LoginDialog.TextBox_KeyPress`：账户/密码框回车 → `OKButton.InvokeMouseClick(null)`；
- `Client/MirScenes/SelectScene.cs:219` `SelectScene_KeyPress`：回车且 START 可用 → `StartGame()`，用 `Characters[_selected]`（`_selected` 默认 0）→ **有角色时无需点选**；
- 键位来自 `Mir2Config`/`KeyBinds.ini`：`F9` 背包、`F10` 装备、`F11` 技能（另有 `Ctrl+I/C/S` 等第二键位）。

账号准备（**只对沙箱副本**）：`dbtool setpw <accountId> <newPassword>` 用原版
`AccountInfo.Password` setter 改密码后 `SaveAccounts()`。注意 `setpw` 会重写 `Server.MirADB`，
且离线 `LoadDB()` 会把 `Server.MirDB` 写坏（见下）——工具已内置 `Server.MirDB` 备份/还原保护。

实测证据：服务端日志出现 `User logging in` → `User logged in` → `<角色名> has connected`；
截图 `kbd_01_select`（SELECT 界面，两角色）、`kbd_02_ingame`（BichonProvince，坐标 277,609）、
`ingame_F9_inventory`、`ingame_F10_equipment`、`ingame_F11_skills`。

## 3. 已知限制（2026-09-24 实测）

**会话被锁屏（`LockScreenBackstopFrame` 覆盖桌面）时，鼠标点不动——但键盘路径可用（见 §2.1）**：

- 真实输入（`mouse_event`/`SetCursorPos`）落到锁屏，不落客户端；桌面级 `CopyFromScreen` 只得到纯色；
- 注入窗口消息可以到达客户端：`WM_MOUSEMOVE` 能让按钮变 hover、`WM_KEYDOWN/UP` 能触发 `CMain_KeyUp`（截图就是这么触发的）；
- 但 **WinForms 不会为注入的 `WM_LBUTTONDOWN/UP` 触发 `MouseClick`**（用最小 WinForms 探针程序验证：MouseDown/MouseUp 触发、MouseClick 不触发；加 `AttachThreadInput+SetCapture`、或同时按住真实左键都无效）；
- 原版镜像 UI 的按钮 `Click` 只在 `MirScene.OnMouseClick` → `MirControl.OnMouseClick` 里触发
  （`Client/MirControls/MirScene.cs`、`MirControl.cs:849`），**MouseDown/Up 不产生 Click** →
  锁屏状态下无法自动点击菜单/对话框。

结论：**锁屏期间用键盘路径（§2.1）做 A/B；需要纯鼠标操作的窗口（商城买卖、NPC 菜单点行等）
仍要解锁工作站/重连 console 会话**后再跑 `Click-Image`。

### 3.1 补充实测（2026-09-26，**工作站未锁屏**）：原版客户端仍点不动背包页签/关闭钮

解锁状态下重试两条注入路径，都没能驱动原版的点击语义：

- `Msg-Click`（注入 `WM_LBUTTONDOWN/UP`）：连按 ITEMS II / QUEST 页签，三张帧的页签高亮与网格
  完全没变 ⇒ 又一次证实「WinForms 不为注入消息触发 `MouseClick`」（§3 原有结论）；
- `Click-Image`（真实 `SetCursorPos` + 左键 down/up）点背包关闭钮 (301,13)：背包**没有**关闭；
  点 QUEST 页签 (182,18)：页签**没有**切换。对照可见窗口被拖动了约 12px（背包 `Movable=true`，
  说明真实输入**确实到达了**客户端，只是没有产生 `MirControl.OnMouseClick`）。

推断：原版 `MirScene.OnMouseClick` 需要真正的 `MouseClick` 事件（按下与抬起之间**没有位移**、
且窗口处于可接收输入状态）；`SetCursorPos` 之后紧跟的 down/up 在这条路径上不满足，于是被当成
标题栏拖动。**结论（选型建议）**：需要「点页签/点行/点关闭钮」才能到达的窗口，现阶段不要押在
驱动原版客户端上——改用两条更稳的替代：

1. **源码表对表 + 我方实机几何探针**：C# 的常量表（如 `CharacterDialog.cs:229-340` 的 14 个装备格）
   逐项对 `Client-Bevy` 的常量，再用 `probe_ui_nodes.ps1 -Points` 在我方客户端上验证「同一份几何
   真的被渲染」（2026-09-26 已用这条做完角色窗装备格：14/14 位置与枚举序全部一致）。
2. **键盘可达的状态**（§2.1 的 F9/F10，以及任何有键位的开关）继续做像素 A/B。

### 3.2 金标准基准帧要挑「已渲染」的那张

同一台机、同一客户端，早先那张基准帧（`Image 9.png`，03:32 落盘）里**角色窗的纸娃娃区是黑的**
（纸娃娃未渲染），拿它做 CHAR 窗基准会把「基准自己没画」算成我方差异：CHAR 窗 (762,8)-(1020,372)
对它是 39140/93912（41.7%），换成进场后重拍的 `shots/orig_ingame_F10_equipment.png`
（纸娃娃正常，背包+角色窗同开）后是 26553/93912（28.3%）。
**基准帧落盘后先看一眼窗口内容是否都渲染出来**，别默认它是对的。

### 3.2b 键盘逐窗像素 A/B（2026-09-27 打通：原版鼠标点不动，但**键位**能开窗）

§3.1 已经把「驱动原版点页签/点行」这条路堵死，但**键位是活的**——沙箱那份 `KeyBinds.ini` 把主要窗口
都绑了键（`Inventory=F9`、`Equipment=F10`、`Skills=F11`、`Quests=Q`、`Options=F12`、`Group=P`、
`Friends=F`、`Relationship=L`、`Guilds=G`、`Ranking=K`、`Help=H`、`Keybind=U`、`Creature=E`、
`MountWindow=J`、`Fishing=N`、`GameShop=Y`、`Bigmap=B`、`Minimap=V`、`Belt=Z`、`Skillbar=R`），
而键盘消息**实测能驱动**原版（§2.1）。于是逐窗 A/B 有了可复跑的两侧配方：

```powershell
# 0) 沙箱（按段改端口 + 回读校验）+ 原版服务端 + 键盘登录
#    （账号密码写进**沙箱副本**：dbtool <沙箱>\Server setpw 333 abbtest123；重跑 make_sandbox -Force 会把原版 DB 拷回来，密码要重设）
pwsh tools/acceptance/csharp_golden/make_sandbox.ps1 -Port 7100 -Force
Start-Process "$env:TEMP\golden_sandbox\Server\Server.exe" -WorkingDirectory "$env:TEMP\golden_sandbox\Server"
pwsh tools/acceptance/csharp_golden/csharp_kbd_login.ps1 -SandboxRoot $env:TEMP\golden_sandbox -Account 333 -Password abbtest123
# 1) 原版侧：按 KeyBinds.ini 逐扇开窗 + 截图（自带「这一键有没有让画面变」的自检）
pwsh tools/acceptance/csharp_golden/golden_kbd_windows.ps1 -SandboxRoot $env:TEMP\golden_sandbox
# 2) 我方侧：同一份窗口清单，`dialog open <kind>` 开窗 + `screenshot` 落盘（默认 --ui-scale 1）
pwsh tools/acceptance/csharp_golden/golden_ab_ours.ps1 -SandboxRoot $env:TEMP\golden_sandbox
# 3) 逐窗比：按 C# 期望矩形裁区域算差异（期望表由 window_rect_table.py --out 产出）
py -3.12 tools/acceptance/csharp_golden/golden_ab_diff.py --shots $env:TEMP\golden_sandbox\shots --table %TEMP%\rect_table.json
```

**四个必须踩对的点**（实测踩出来的，别再重踩）：

1. **我方要 `--ui-scale 1`**（2026-09-27 新增的开关）：原版恒 1024x768@scale1，而本端跟系统 DPI
   （本机 150% ⇒ 截图 1536x1152）。不统一尺度，逐窗差异的主项就是**重采样噪声**（实测窗口区域
   30%~99% 的"差异"几乎全是它）；把原版放大、把我方缩小都救不了，必须两端同尺度渲染。
2. **注入键要带 `WM_CHAR`**：C# 的 `MirMessageBox.OnKeyPress` 才处理 Escape（关模态框），
   只发 `WM_KEYDOWN/UP` 关不掉 —— 模态框会一直吞掉后面的键（实测基线帧里那句
   "You are not in a guild." 不退，后续 20 张全等于基线）。
3. **基线帧要干净**：`csharp_kbd_login.ps1` 的 `orig_kbd_02_ingame.png` 是 F9/F10 之后拍的、
   自带两扇窗；逐窗比对要用 `golden_kbd_windows.ps1` 开头 Escape 后现拍的 `orig_baseline_none.png`。
4. **我方 `screenshot` 是下一帧才落盘**：`dialog open` 之后 0.9s 就截会拿到"还没开窗"的帧
   （第一版 20 张全中招）。两侧脚本都内置「与基线比出可见差异，否则重试」。

边界（如实）：C# 侧有些键在**当前角色状态下不开窗**（没行会按 G 弹的是 `MirMessageBox`、
没拿钓竿按 N 什么都不出），另一些键是 HUD 开关（Z 腰带 / R 技能栏）。那几张的像素差再大也与
"窗内绘制"无关，**只有两侧都真的出窗的那几扇可比**——出没出窗以帧为准（脚本会打印每扇窗的
"与基线差"，人工过一眼）。

### 3.2c 首次完整跑通（2026-09-27）与**结论：差异主项是"角色状态"，不是绘制**

链一次跑通（原版沙箱服务端 7100 + 原版客户端键盘登录 `女道士 has connected` → 我方 e2e 服务端 7000
（debug，`b53ddb4c1`）+ 我方客户端 `--ui-scale 1`）：

- 原版侧 20 扇全部"画面有变化"（`shots\orig_win_*.png`，0 扇 no_effect）；
- 我方侧 20 扇全部 `open_ok=True`（`shots\ours_win_*.png`）；
- `golden_ab_diff.py` 逐窗给出了占比表（20 行）。

**逐张目检后的定性（这才是这一轮的产出）**：

| 类型 | 例子 | 判定 |
|---|---|---|
| 窗口矩形/控件与内容**都对得上**，差在内容 | `Options` 6.2%、`Group` 5.2%、`Quests` 8.5%、`Equipment` 7.7%、`Ranking` 10.5% | 形状一致；`Options` 的差主要来自**音量滑块**（原版 100 / 我方 0） |
| 同一扇窗，**两侧内容语言/键位表不同** | `Help` 87.6%、`Keybind` 26.6% | 我方是中文键位表（Tab 拾取、C+F1 等），原版是英文表；窗口 chrome 与位置一致 |
| 原版侧**根本没出那扇窗**（弹 `MirMessageBox`） | `Creature` 98.8%（"You do not own any creatures."）、`Guilds` 98.3%（"You are not in a guild."） | 不可比：**状态差**（我方角色有 5 只宠物 / 已在测试行会） |
| 整帧/窗口内大面积不同 | `Skills`/`Belt`/`Skillbar` 91%（整帧参考）、`Bigmap` 62%、`MountWindow` 97%、`Fishing` 95% | 同上：状态 + **背景地图也不同**（见下） |

**两个必须先消掉的混淆项（不是缺陷）**：

1. **角色状态**：原版登录的是**全新 1 级女道士**（背包 1 件、金币 45、无宠物/无行会/无坐骑/无技能），
   我方当时用的是高等级角色（背包 51/51、金币 1,003,244、5 只宠物、已在测试行会、有技能栏）。
   ⇒ `Creature`/`Guilds`/`MountWindow`/`Fishing` 这几扇在原版侧**压根不是同一扇窗**。
   `golden_ab_ours.ps1` 已加 `-User/-Password`，用来指向**与金标准同职业同等级同空背包**的角色。
2. **背景地图不同**：同在 `(278,609)`，原版画面是 BorderVillage 一带（`Merchant Ruben`/`Assistant Jane`/
   `TravellingMerchant Damian`…），我方画面是城墙/坡道一带 ⇒ 两台服务端的 `BichonProvince` 地图数据
   （或 NPC 落位）**不同源**。窗口区域里凡是半透明/露出背景的部分都会带上这层差。

**下一步（待办，按顺序）**：① 把我方测试角色对齐到金标准状态（1 级女道士、空背包、无宠物/行会/坐骑/技能、
音量等设置也对齐）后重跑同一条链，此时的差异才是可判的窗内绘制差；② 要彻底去掉背景干扰，需要两台服务端
用**同源地图数据**（否则整帧比对无意义，只能比窗口 chrome）；③ 再按 §3.3 的几何表逐窗收口。

### 3.2d 状态对齐后重跑（2026-09-27，同轮）：**差异收敛到窗内，并捞出 4 处真缺口**

**① 角色状态已对齐**（本 PR 给 `l5ac_newchar_create.ps1` 加了 `-CreateClass/-CreateGender`）：

```powershell
# 干净账号（服务端对未知账号自动建号、无角色）→ 真机建角路径上先点职业/性别、再填名字
pwsh tools\acceptance\l5ac_newchar_create.ps1 -User goldenchr -Name 女道士 `
     -CreateClass Taoist -CreateGender Female
# ⇒ probe: {"class":"Taoist","gender":"Female","name":"女道士","name_valid":true} / VERDICT create=PASS
pwsh tools\acceptance\csharp_golden\golden_ab_ours.ps1 -SandboxRoot $env:TEMP\golden_sandbox `
     -ClientHome E:\...\Crystal-wt-blend -User goldenchr -Password 123456
```

两个**必须踩对的坑**（都已写进脚本注释）：

1. **点职业/性别会把焦点从名字框移走**（`probe.name_focused: true → false`）⇒ 必须再点一次名字框，
   否则 `type_text` 落空（`probe.name` 仍为 `''`，本轮实测）。
2. **非 GM 角色用不了 `@mapmove`**：新角色没有 GM 权限，`@mapmove 0 278 609` 被服务端拒绝，
   角色停在出生点 `(288,616)`，而老脚本照样打印"对齐后" ⇒ 假对齐。`golden_ab_ours.ps1` 已改成
   「`@mapmove` 落点不对就退回 `walk_to`（玩家验收能力，真实寻路）」并打印 `aligned=<bool>`。
   实测 `walk_to` 走到 `(285,616)` 就停了 ⇒ **(278,609) 在我们这份数据里不可达/不可走**——这也是
   §3.2d ② 的必然结果：两边的 `BichonProvince` 根本不是同一份地图。

**② 背景地形不同源：已定性为「数据版本差异」，不是绘制缺陷**（三个文件，SHA256 前 20 位）：

| 文件 | 大小 | 格式 | 谁在用 |
|---|---|---|---|
| `ServerRust/Daneo1989/Maps/0.map` | 7,350,054 | `Map 2010 Ver 1.0`（0x10 头） | **我方客户端**（`map_reader.rs` 的解析顺序命中它）+ 我方服务端地图（`loader.rs` 的 `{data_dir}/Maps`） | 
| 沙箱 `Server\Maps\0.map` / `Client\Map\0.map` | 12,740,008 | 老格式（`Legend of mir`） | **原版**（`MapInfo.FileName = "0"` 时） |
| 沙箱 `Server\Maps\n0.map` / `Client\Map\n0.map` | 17,640,052 | 老格式 | 原版（`FileName = "n0"` 时）；**我方 `ServerRust/Data/Map/n0.map` 与它逐字节相同**（hash `79C159D7F5F2338A86A9`），但在我们这套栈里没被用到 |

⇒ 我方跑的是 **Daneo1989 那一版（Map 2010）世界**，原版沙箱跑的是**老版世界**；同一个 tile 的地形/NPC
自然不同。**整帧比对无意义**，A/B 只能在窗口区域内比（窗口 chrome/内容不受背景影响）。
要把背景也做成同源，需要把两台服务端/客户端都换成同一套地图数据（属数据迁移，不在工具链范围）。

**③ 逐窗差异（对齐状态前后对比，同一张 `rect_table.json`）**：

| 窗口 | 对齐前 | 对齐后 | 判定 |
|---|---|---|---|
| Inventory | 37.5% | **7.1%** | 对齐生效（新角色 1 件起始装备 + 金币 46 vs 原版 45） |
| Equipment | 7.7% | 6.3% | 同上 |
| Quests / Options / Group | 8.5% / 6.2% / 5.2% | 7.9% / 6.2% / 5.2% | chrome 与控件一致，差在内容 |
| Ranking / Friends / Relationship | 10.5% / 26.2% / 22.2% | 10.6% / 25.6% / 22.2% | 待逐条定性（列表内容/空表绘制） |
| Help / Keybind | 87.6% / 26.6% | 87.6% / 26.6% | 键位表**语言与条目**不同（我方中文表 vs 原版英文表），窗口位置/尺寸一致 |
| GameShop / Bigmap | 29.9% / 62.0% | 29.9% / 63.4% | 待逐条定性（商品数据/大地图绘制） |
| **Creature / Guilds / MountWindow / Fishing** | 98.8% / 98.3% / 97.0% / 95.2% | 98.6% / 98.3% / 97.0% / 95.2% | **真缺口（见 ④）** |

**④ 本轮捞出的 4 处真缺口（原版弹 `MirMessageBox`，我方照开空窗）**——每一条都有 C# 原文可依：

| 窗口（键） | 原版行为（C# `Show()` 前置守卫） | 我方现状 |
|---|---|---|
| 宠物（E） | `IntelligentCreatureDialogs.cs:832-841`：`!User.IntelligentCreatures.Any()` → `MirMessageBox(NoCreatures)` | 直接开 PET STATUS 空窗（0 只、空槽、有名/召唤/放生等控件） |
| 行会（G） | `GuildDialog.cs:2156-2166`：`MapControl.User.GuildName == ""` → `MirMessageBox(NotInGuild)` | 直接开 GUILD 空窗（NOTICE/MEMBERS/STORAGE/RANKS 四个空黑面板） |
| 坐骑（M/J） | `MountDialog.cs:240-251`：`User.MountType < 0` → `MirMessageBox(NoMount)` | 直接开空坐骑窗（0 槽 + 大片黑底） |
| 钓鱼（N） | `FishingDialog.cs:135-147`：`!User.HasFishingRod` → `MirMessageBox(NoFishingRod)` | 直接开空钓鱼窗（5 个空格子 + 黑底） |

这 4 条**不是**背景/状态差异：守卫在 C# 里写在每个对话框自己的 `Show()` 里，键盘路径和程序化开窗
（我方 `dialog open <kind>` 仪器同理）都必须走它；而我方这 4 个窗没有该守卫。已作为下一批队列项
（`crystal-dialog-show-guards`）落账，修完用同一条链复跑即可看到这 4 行差异归零。

### 3.2e 守卫修复后的复跑（2026-09-28，`crystal-dialog-show-guards` 收口）

修复（`Client-Bevy/src/game/dialogs/notice_box.rs` + 键盘/RPC 两条开窗路径）：把「提示框」与「守卫判定」
收敛成**唯一入口**，4 扇窗在状态不具备时**不开窗、只弹 `MirMessageBox(OK)`**——
面板 `Prguse[360]` 456x190 @(284,289)、文本 (35,35)、OKAY `Title[200/201/202]` @(360,157) **76x25**
（`libextract.py Title.Lib 200` 实测；第一版按截图目测写成 69x25，按钮被横向压扁 8%，逐窗按钮区差异 81%）。

复跑判据（客户端由该提交构建，戳 `dirty=0`；原版侧沿用同一批金标准帧）：

| 窗口 | 机器可读判据（`ab_windows.json`） | 提示框区域（284,289,740,479）| 按钮区（644,446,720,471）|
|---|---|---|---|
| 宠物 E | `我方窗开=False`、`提示=你没有任何宠物。` | 1315 / 86,640 = **1.5%** | **逐像素一致（bbox=None）** |
| 行会 G | `我方窗开=False`、`提示=你不在任何公会中。` | 1041 = **1.2%** | **一致** |
| 坐骑 M/J | `我方窗开=False`、`提示=你没有坐骑。` | 1019 = **1.2%** | **一致** |
| 钓鱼 N | `我方窗开=False`、`提示=你没有拿着鱼竿。` | 1388 = **1.6%** | **一致** |

残余的那 1.2%~1.6% 全部落在**文本行**：原版英文（"You do not own any creatures."）vs 我方中文
（"你没有任何宠物。"，逐字取原版 `Client/Localization/Chinese.json`）。

⚠️ **不要用「窗口矩形」那一列判这 4 条**：`golden_ab_diff.py` 的窗口矩形表里这 4 行仍是 34%~94%，
因为窗口矩形比提示框大得多，多出来的部分是**两套世界数据的背景地图**（见 §3.2d ②）；
按「窗口矩形」判会得出"修了没效果"的错误结论。**判据是上面这张表**：① `window_open=False` +
② `notice=<原版文案>` + ③ 提示框区域≈文本差。

仪器补强（同轮）：新增只读 RPC `notice_probe`（`{"action":"close"}` 可顺手清场），
`golden_ab_ours.ps1` 每扇窗记 `window_open`（`dialog_rect(fallback=root)` 有根才算开；回复是**扁平**
`rx/ry/rw/rh`，第一版按 `$rect.rect` 判空导致 20 扇全部误报"窗开=False"）与 `notice` 两个字段；
`golden_ab_diff.py` 把这两个事实一并打印。**每扇窗开窗前必须先 `notice_probe {action:close}` 清场**——
否则上一扇的提示框会留在后续截图里（本轮实测：Guilds 之后的 Ranking/Help/…/Skillbar 全部带着
"你不在任何公会中。"，凭空多出 17%~60% 的假差异）。

### 3.2f 仍待定性的窗（逐张目检的中间结论，2026-09-28）

§3.2d 里还剩几行没定性（`Friends 25.6% / Relationship 22.2% / Ranking 10.6% / GameShop 29.6% / Bigmap 63.3%`）。
本轮把 **Friends** 定性完，其余留给后续（判据与配方同 §3.2c/§3.2d）：

| 窗口 | 差异 | 定性 |
|---|---|---|
| **Friends** | 25.6% | **两件事叠加（2026-09-28 修其一）**：① **真缺口——本端好友窗缺翻页条**（原版 `FriendDialog.cs:70-118`：`PageNumberLabel` (87,216) 83x17 居中、上一页 `Prguse2[240/241/242]` @(70,218) 16x16、下一页 `Prguse2[243/244/245]` @(171,218) 16x16），本端 `friend.rs` 全文搜 `page/arrow/翻页` 零命中；**已修**（PR #3320：补翻页条 + 列表改回 C# 的 `FriendRow[12]` 两列格子 `((i%2)*115+16, 55+(i/2)*22)`、行尺寸 (115,17)，并补两条门禁 + 阳性对照）。② **残余不是产品缺陷**：修后那 25.6% 几乎不变 ⇒ 主导项是**整体 +1px 的横向偏移**（把原版帧按 dx=1 采样，差异从 18383 掉到 4783 像素），而**我方窗口矩形实测 `dialog_rect kind=friend → rx=380 ry=248 rw=264 rh=272`，与 C# `Center`（(1024-264)/2, (768-272)/2）逐值相同**；逐像素看原版面板左边框在 x=381、我方在 x=380，且两边的边框色序完全一致（同图 +1px）。对照：显式坐标窗口（Inventory/Equipment/Options/Group/Quests/Ranking）的位移扫描都是 dx=0 ⇒ 只有**居中窗**在这一对帧里偏 1px，最可能是**原版侧取帧的客户区宽度比 1024 宽 2px**（C# `Center` 于是算出 381），属**A/B 取帧口径**问题，不是本端排版 bug。**待办**：给逐窗对拍加「居中窗允许 ±1px 平移」或把原版取帧改成真实客户区，之后再看这批窗的真实差异。 |
| Help / Keybind | 87.6% / 26.6% | 非缺陷：键位表**语言与条目**不同（我方中文动态生成 vs 原版英文固定清单），窗口矩形/控件位置一致（§3.2c 已记）。 |
| Relationship / Ranking / GameShop / Bigmap | 22.2% / 10.6% / 29.6% / 63.3% | **已定性，见 §3.2g**（注意 `RankingDialog.cs` 与 `GameshopDialog.cs` 也都有 `PageNumberLabel` 一类的翻页控件，`TrustMerchantDialog.cs`/`HelpDialog.cs`/`MailDialogs.cs`/`IntelligentCreatureDialogs.cs`/`CharacterDialog.cs` 同理——**排查时先按这条线核"本端有没有翻页条"**，Friends 就是这么捞出来的）。 |

### 3.2g 「翻页条」线索套到 Relationship / Ranking / GameShop / Bigmap（2026-09-28）

判据/配方同 §3.2c–§3.2f（原版侧 `golden_kbd_windows.ps1` 键位开窗；我方侧
`golden_ab_ours.ps1 -User goldenchr --ui-scale 1`；`golden_ab_diff.py` 逐窗占比 + 位移扫描）。
**本轮为了少看整图，把「差异块图」做成数值化输出**：把窗口矩形按 8x8 分块、逐块数差异像素，
打成五档 ASCII（`#` >32、`+` >16、`:` >4、`.` >0、空格 0）；定位到可疑带之后再上
**子区差异计数**（视口内/外）、**逐行亮像素直方图**（>90 灰度的像素数）与**单像素色值采样**。
这三样足够区分「几何/尺寸差」「画源差」与「数据差」，全程不必打开整幅 PNG。

**一、对表：C# 有没有翻页条 vs 本端有没有**

| 窗口 | C# 侧翻页/滚动控件（`Client/MirScenes/Dialogs`） | 本端 | 线索结论 |
|---|---|---|---|
| Relationship | **无翻页条**（`RelationshipDialog.cs` 全文无 `PageNumberLabel/PreviousButton/NextButton`；只有 5 颗操作钮） | 无 | 不适用 |
| Ranking | **无** `PageNumberLabel`；是 `PrevButton Prguse2[197..199]@(299,100)` + `NextButton [207..209]@(299,386)` + `ScrollBar [205/206]@(299,113)`（`:132-168`） | 三件套齐（`ranking.rs:439-495`） | 无缺口 |
| GameShop | `PageNumberLabel` 83x17@(597,446) + `PreviousButton Prguse2[240..242]@(600,448)` + `NextButton [243..245]@(660,448)`（`:379-424`）；分类列另有一对 `[197..199]@(120,103)` / `[207..209]@(120,421)` + `PositionBar [205/206]@(120,117)`（`:99-155`） | **全有**（`game_shop.rs:283-293/1160-1381`，`PositionBar` 位置也是 (120,117)） | 无缺口 |
| Bigmap | **无翻页条**；`ScrollUp [197..199]@(W-21,48)` + `ScrollDown [207..209]@(W-21,417)` + `ScrollBar [205/206]@(W-21,61)`（`:101-147`） | **全有**，尺寸按图头 12x12 | 不适用 |

⇒ 这条线索**只对 Friends 有效**（`FriendDialog.cs` 有翻页三件套、本端零命中）。四扇窗里
`Ranking/GameShop` 本来就有（GameShop 连 `PositionBar` 位置都对），`Relationship/Bigmap`
C# 就没有翻页条。**不能照抄 Friends 的修法**——顺着 A/B 的数值化输出，这四扇窗真正的差异是下面三类。

**二、真缺口（已修，本 PR）**

| 窗口 | 缺口 | 数值证据 |
|---|---|---|
| Relationship | 缺 C# `TitleLabel` = `Title[52]` @(18,8) 109x15（`RelationshipDialog.cs:30-36`） | 该带（面板内 y 8..23）原版帧 **439** 个亮像素 / 本端 **13**（≈没画） |
| Relationship | 5 颗操作钮写死 `24x22`，美术原生是 `Prguse[610/600/616/437/566]` = **28x25**（C# 不写 `Size` ⇒ 取图头） | 单颗钮在 A/B 里是 **32x25** 的差异块；逐像素看本端是插值色（精灵被拉伸） |
| Relationship | 四行信息缺**垂直居中**：C# 是 `Size(200,30)` + `DrawFormat.VerticalCenter`（文本中心 = `y + 15`），本端左上锚点 | 逐行亮像素直方图：本端文本行 41..51 / 66..75，C# 51..60 / 76..85（偏上 ~9.5px） |
| Bigmap | 视口画源：原版 `BigMapViewPort.OnBeforeDraw` 画 **`Data/mmap.Lib` 里的 `MapInfo.BigMap` 那张大图**，缩放进 `min(568,W) x min(380,H)` 居中铺满（`BigMapDialog.cs:642-676`）；本端画的是**按瓦片采样自造的地形纹理** | 视口矩形内差异 **207271/215840（96%）**；把原版帧视口与 `mmap.Lib[101]`（沙箱 DB 的 `BigMap`，1052x700 → 568x380 bilinear）逐像素比，**一致率 98.3%**（NEAREST 97.0%）⇒ 原版画的就是这张图 |

**三、数据/状态差（不是绘制缺陷，别去改代码）**

- **Ranking**：行区 20 行的亮像素数 **原版 27 / 本端 7567** —— 原版那份沙箱 DB 的排行榜是**空的**
  （本端有 20 行）。四列（rank/name/class/level）全差、连 rank 列也一样，是内容差不是排版差。
- **GameShop**：两侧 `GameShopList`/分类表不同源（各自 DB），商品格与分类行本来就该不同。
  窗口 chrome 与翻页条的位置/帧号一致（表一已核）。

**四、刻意偏离（**别当缺陷修**）**

- **滚动条轨道/滑块**：本端 `spawn_scroll_bar_ui` 画的是**半透明黑轨道 + 浅色滑块**，而 C# 对应控件
  只是一个 `Prguse2[205]` 手柄精灵（Ranking `:158-168`、GameShop `PositionBar :143-155`）。
  这是 2026-09「好多窗口滚动条好像都没实现」之后落地的实现，有实机验收记录
  （`tools/acceptance/UI_VERIFICATION_REPORT.md` 项 5 / #2968 #2978：滑块高按行数比例、拖动与滚轮
  都改 `offset`）。**它不是 Friends 那类"自造交互"**——删掉会把已验证的滚动能力一起删掉。
- **婚姻窗目标名输入框**：本端协议 `MarriageRequestWire` 带 `target_name`（C# `C.MarriageRequest`
  是**空包**、由服务端选目标），输入框是协议扩展的必然产物，不动。
- **居中窗 ±1px**：同 §3.2f（`--max-shift` 口径，不是本端排版 bug）。

**五、残留（未修，留作队列）**

- Bigmap 的 `BigMap <= 0` 守卫：C# `Show()` 直接返回（**连窗都不开**，`BigMapDialog.cs:288-289`），
  本端无该守卫；本端在 `BigMap == 0` 或 `mmap.Lib` 缺该索引时**回落地形渲染**（＝本端旧行为，
  不是 C# 行为）。收口前要先定"没大图的地图按 B 该弹什么提示"。
- Relationship 四行**文案**仍是本端自造中文，C# 走 `ClientTextKeys.LoverName/MarriageDate/…`
  模板（语言/条目差一类，同 §3.2c 的 Help/Keybind）。

### 3.2h 大地图视口闭环（2026-09-28）：不用原版也能验，外加 C# 的静默守卫

§3.2g 把大地图视口改成 C# 路线（画 `Data/mmap.Lib[MapInfo.BigMap]`，缩放进
`min(568,W) x min(380,H)` 居中）。**取证缺口**是当时只用"原版帧 vs `mmap.Lib[101]`"证明了
*原版*画的是这张图，本端改完没实机验过。现在补上，而且做得**不依赖原版客户端**：

```powershell
# 本端起客户端 → dialog open big_map → screenshot（`--ui-scale 1`）
py -3.12 tools/acceptance/csharp_golden/bigmap_viewport_check.py `
    --shot %TEMP%\bigmap_ours.png --mmap Data/mmap.Lib --index 135
```

`bigmap_viewport_check.py` 自己算 C# 画幅（`(14+(568-w)/2, 52+(380-h)/2)`，`w=min(568,W)`）、
把 `mmap[index]` 缩放到画幅，与本端截图同区域逐像素比（跳过近黑＝原版当透明的像素）。
**为什么它能替代原版帧**：两侧画的是同一份美术、同一套布局，所以本端 vs 这张图既然是 99.9%，
就等价于"本端 vs 原版"（原版那侧 §3.2g 已证 98.3%）。它**不受两侧地图数据不同源影响**——
索引来自本端自己的 DB，两张图不同只说明数据不同源，不代表画错。

实测（master `02c5a0d83` + 本线程改动，客户端 1024x768@scale1，`map_infos.big_map=135`）：

| 场景 | 一致率 | 判定 |
|---|---|---|
| BichonProvince（`big_map=135`）开着大地图 | **0.999** | PASS（画源+布局 = C#） |
| 负对照：拿 `mmap.Lib[136]`（452x300）去比同一张帧 | 0.275 | FAIL（判据能区分"另一张图"） |
| 窗开着从 Bichon 换到 `D002`（`big_map=0`） | **0.000** | 视口已清空（旧实现会留着上一张图 ⇒ 仍 ~99%） |

**顺带补的 C# 守卫**（`BigMapDialog.cs:288-289`）：

```csharp
public override void Show() { var map = GameScene.Scene.MapControl;
    if (map.BigMap <= 0) return;          // ← 静默：不弹 MirMessageBox、连窗都不开
    ...
}
```

本端把它接进既有的 `ShowGuardParams`（键盘热键与 `dialog open` RPC 两条路径共用），
与宠物/行会/坐骑/钓鱼那四扇的区别是**不弹提示框**：`show_guard` 现在返回枚举
`ShowGuard::{Allow, Block(文案), BlockSilent}`。同时把 §3.2g 记的"回落地形渲染"残留**删掉**
——`build_terrain_texture`/`tile_avg_color` 与视口那层自造深色底一并删除，
`OnBeforeDraw:644-645`（`index <= 0` 就 return，连对象点都不画）成了唯一路径；
自造的深色视口底也去掉（C# `BigMapViewPort` 无背景，画幅之外露的是 `Title[820]` 面板美术）。

**这条守卫会波及门禁**（改行为必须同步审计依赖旧行为的门禁，见
`LESSON_改行为必须同步审计依赖旧行为的门禁否则恒红被无视`）：`ui_interact_sweep.ps1`
逐窗段跑在"角色当前停留在哪张图"上，而**本脚本自己的 NPC 段会把角色留在 `D002`**
（`big_map=0`）⇒ 下一次巡回的 big_map 必假红。修法两步：① 逐窗前 best-effort
`@mapmove 0 288 616` 锚到 BichonProvince（地图文件 `0`，`big_map=135`），把结果打进输出
（`锚图 map=0 tile=(…) big_map_ready=True`）；② 锚不上（非 GM 账号等）时 big_map 记
**SKIP**（`-FailOnSkip` 下仍红），锚上了才按严格判据（该开就得开）。

实机判据（`D002`，`big_map=0`）：`dialog open big_map` 后 `dialogs=[Minimap]`、
`notice_probe` 的 `text=null` ⇒ 窗口没开、也没弹提示，与 C# 的静默 `return` 一致。

### 3.2i 小地图缩略图「根本没画」——一条查询空转了三天的根因（2026-09-28）

§3.2c–§3.2h 的 A/B 表里，`Minimap` 是最后一扇没定性的窗（91.3%）。它不是"背景地图不同源"
——数值化拆分后是**本端什么都没画**：

- 同一屏「小地图开 vs 关」在同一区域比：**上带 2747/2772、下带 2905/3024 不同**（面板在），
  但 **图区（120x108）只差 31/12960** ⇒ 那块是**透的**，能看见世界；
- 本端图区只有 98 个颜色、均值 (121.7,104.7,78.1)（= 世界地形），原版同区 1893 个颜色。

**根因（不是错值，是查询空转）**：`minimap_map_image_system` 的查询是
`(&mut ImageNode, &mut Node, &mut BackgroundColor, &mut Visibility) With<MiniMapMapArea>`，
而 spawn 处只插了 `MiniMapMapArea + BackgroundColor + Visibility::Hidden` —— **没有 `ImageNode`**
⇒ 查询一条都匹配不到、循环体**永不执行**、图区永远停在 `Hidden`。自 #7cc68c7af
（"小地图补上 MMap 缩略图"，2026-09-25）起就没生效过：那次提交把缩略图拆成独立系统时
加了 `&mut ImageNode` 要求，却没给实体补这个组件。**没有任何告警**——这正是它活了三天没人发现的原因。

定位靠**先加只读探针把绘制侧真值暴露出来**，而不是继续看图猜：

```
minimap_probe → {"index":101,"art_wh":[1052,700],"map_wh":[700,700],"mode_big":true,"open":true,
                 "area":{"error":"NoEntities(...MiniMapMapArea...)"}}      ← 修复前：实体查不到
                 "area":{"rect":[372,562,492,670],"visible":"Visible","node":{...}}  ← 修复后
```

（当时那条探针自己也带 `&ImageNode`，所以同样查不到它；`ui_nodes_at(960,76)` 命中了图区节点
`vis=Hidden`，两者一对就锁定了"实体在但没组件/没显隐"。）

**判据（不依赖原版，`--view mini`）**：

```powershell
py -3.12 tools/acceptance/csharp_golden/bigmap_viewport_check.py --view mini `
    --shot %TEMP%\minimap_probe_shot.png --mmap Data/mmap.Lib `
    --index 101 --tile 288 616 --map 700 700
```

`--view mini` 按 C# `MiniMapDialog` 的口径算裁剪窗（`scale = mmap尺寸/地图瓦片数`、窗口 120x108
以玩家为中心、先贴右/下再钳 0、**1:1 裁剪不缩放**），画在面板 `Prguse[2090]` 内 (3,22)。
实测（BichonProvince 700x700、玩家 (288,616)、`mmap[101]` 1052x700）：

| 场景 | 一致率 | 判定 |
|---|---|---|
| 修复后本端帧 | **0.998** | PASS（裁剪窗 = C# 期望 (372,562,120,108)） |
| 负对照：拿 `mmap[135]` 比同一帧 | 0.255 | FAIL |

**离线门禁（防再犯）**：`Client-Bevy/src/game/dialogs/interact_gate.rs::minimap_map_area_is_paintable`
——用**与画图系统同一套过滤器**（`With<MiniMapMapArea> + ImageNode + Node + BackgroundColor + Visibility`）
去匹配合成资产下的图区实体，匹配不到即红。**阳性对照实做**：把 spawn 里的
`ImageNode::new(white.clone())` 去掉 ⇒ 立刻红（`left: 0, right: 1`）。

### 3.2j A/B 表剩余低差异窗逐窗收口（2026-09-28）：Group 归零、Inventory/Quests 显著下降

§3.2c 起一直挂着「Inventory 7.1% / Equipment 6.3% / Quests 7.9% / Options 6.2% / Group 5.2%」
这五行，只写了"chrome 与控件一致，差在内容"。本轮把它们逐个拆开——手法还是**数值化**：
8x8 差异块图 → 对差异块做 **1px 掩码** → 再拿候选精灵（`Title/Prguse/Prguse2` 的相关帧）
去**反查"这一块到底是哪张图"**（哪一侧画了它、画的是哪一帧）。后一步是这轮的关键：
它能把"内容不同"（两边都画了、只是数据不同）与"画错了/没画"直接分开。

| 窗口 | 改前 | 改后 | 驱动 |
|---|---|---|---|
| **Group** | 5.2% | **0.0%** | **三处真缺口**（见下）——修完全窗 0 差异 |
| **Inventory** | 7.1% | **2.9%** | 页签**裁剪口径** + ITEMS II 换帧条件（见下）；残余＝物品内容/金币文本值/面板透明边 |
| **Quests** | 7.9% | **6.6%** | 删掉一枚**自造**的「放弃」钮（见下）；残余＝任务列表文本内容 |
| Equipment | 6.3% | 6.3% | 非缺陷：纸娃娃/装备格内容（两侧角色装备不同）+ 面板底部透明行透出世界 |
| Options | 6.2% | 6.2% | 非缺陷：`Settings` 值不同（音量为 100 vs 0，§3.2c 已记） |

**三处真缺口（本轮修复）**

1. **Group：空组时 Add/Del 被整块藏起来**。C# `GroupPanel_BeforeDraw:128-137` 是
   `if (GroupList.Count > 0 && GroupList[0] != User.Name) { 两者 false } else { 两者 true }`
   ——**空组可见**；本端旧写法 `members.first().map(|m| m.name == self).unwrap_or(false)`
   把空组判成"非队长" ⇒ 两钮全 `Hidden`。A/B 里那两块 **60x25 的实心差异**就是"我们没画按钮"
   （反查：原版那两块分别是 `Title[130]`、`Title[136]`，**逐像素 0 差异**；本端那块是面板底）。
2. **Group：AddButton 换帧**。C# 空组 `130/131/132`、非空 `133/134/135`；本端恒 `133..135`。
3. **Group：SwitchButton 换帧**。C# `AllowGroup` → `117/118/119`，否则 `114..116`；本端恒 `114..116`。
   2/3 由新系统 `group_button_art_system` 逐帧改写 `ImageButton` 三帧（同 `dura_status` 的写法）。
4. **Inventory：页签是"裁剪"不是"缩放"**。C# `ItemButton` 声明 `Size=(72,23)` 而图头是 **72x24**
   ⇒ `MirImageControl.Draw` 传的是**源矩形** `(0,0,72,23)`（裁掉最后一行）；本端按节点 72x23 让
   Bevy 把 24 行**线性重采样**成 23 行。实测：改前本端页签与美术差 **26~29**（原版 8~10），
   改后本端 **7.4/8.3/7.8**，与原版**逐值相同**。修法＝给页签 `ImageNode.rect` 钉 `(0,0,72,23)`。
5. **Inventory：ITEMS II 的 `169` 帧永不出现**。C# 判据是 `User.Inventory.Length == 46`；
   本端 `INV_BASE_BAG_SLOTS` 按"本端只存背包（40 格）"的假设写成 `8*5=40` ⇒ 判据永不成立。
   实测（本端自己的 `bag_probe`，全新 1 级角色）：`{"total":46,"quest_total":40}`——`items` 的
   长度**就是**服务端那 46，与 C# 同口径 ⇒ 常量改为 46，页签随原版换成灰掉的 `169`。
6. **Quests：删掉自造的「放弃」钮**。本端在日记窗 `(200,285) 76x25` 常显一枚
   `Title[206..208]` 的钮，而 C# `QuestDiaryDialog` 构造里**没有**这个控件（只有标题 `Title[15]`、
   底部 `_closeButton Title[193..195]@(200,436)`、关闭 `Prguse2[360..362]@(289,3)`）。
   原版的「放弃任务」走**任务详情窗** `_cancelButton`（`QuestDialogs.cs:581-601`：`Title[203..205]@(200,436)`
   → YesNo 询问框 → `C.AbandonQuest`），本端该路径已实现（`confirm_cancel`）⇒ 删除不减能力。
   （顺带把 `Title[206..208]` 还给了它真正的归属：婚姻邀请框的 Yes 钮。）

**顺带记：`control_size_audit.py` 的第 4 个盲点**——它在 Inventory 页签这条**没报**：
`for (idx, &(inactive, active)) in INV_TAB_ART.iter().enumerate() { … load(…, initial) … spawn(…, 72.0, 23.0) }`
里帧号是**变量**（表里只有帧号、尺寸是 spawn 处的字面量），它既不是"常量表带尺寸列"那一型，
也不是"load 与 spawn 相邻且帧号字面量"那一型 ⇒ 静默漏掉（实测 `--data` 跑仍是 0 命中）。
待办：把扫描面扩到"帧号来自循环变量"的表驱动形态（或至少在表驱动分支里按**尺寸列缺失**告警）。

### 3.2k A/B 表最后三行「噪声项」的可比口径（2026-09-28）

`Skills / Belt / Skillbar` 三行长期是**整帧参考**（两边的"差异"其实是世界与角色不同，判不出窗内绘制）。
本轮给它们定口径，方法三件：

1. **我方侧的开窗路径**（它们不是 DialogKind，`dialog open <kind>` 必然失败）：
   - `character_skill_page` → `char_page {page:3}`（C# F11 = `CharacterDialog.Show()+ShowSkillPage()`）；
   - `hud_belt` → `hud_toggle belt`（C# Z = `BeltDialog.Show/Hide`）；
   - `hud_skillbar` → `hud_toggle skillbar`（C# R = `Settings.SkillBar` 开关）。
   `hud_toggle`（新增，`Client-Bevy/src/control.rs`）翻转/置位的**就是热键用的那个状态位**
   （`PotionBeltVisible` / `OptionState.skill_bar`），`on` 省略即翻转。`golden_ab_ours.ps1`
   里这张 kind→opener 映射表就是"我方侧怎么把这三行摆到屏上"的单一出处。
   **踩坑记录**：HUD 两行是**翻转**语义，第一版用"与基线差 < 0.5 就再翻一次"的重试启发式，
   结果第 2 次重试把腰带又翻回来 ⇒ A/B 帧与基线同态、腰地区域 0 变化，看上去像"本端没画腰带"。
   现在 HUD 两行**只翻一次**（`$isHudRow`），判据交给下面的**状态/美术**两把尺子。
2. **期望矩形**（`window_rect_table.py`）：补 `BeltDialog → hud_belt`、`SkillBarDialog → hud_skillbar`，
   并补上 `GameScene.Scene.MainDialog.Location.X` 的求值（`MainDialog` = `Prguse[1]` 1024x152 居中 ⇒ X=0）
   ——不然 `BeltDialog` 的 `MainDialog.X + 230` 解不出，工具会诚实地 SKIP 掉它。
   两行的最终矩形：`hud_belt (230,618,240,38)`、`hud_skillbar (0,0,216,28)`
   （后者取 **默认** `Settings.SkillbarLocation[0]`；原版每帧由 `GameScene.DialogProcess:1327-1333` 改写）。
3. **判据**：`character_skill_page` 走原来的整窗像素比（与 `character` 同一扇窗）——
   **实测 1.7%**（从"整帧参考 89%"变成真实可比项）。HUD 两行**不能**用整窗比：那一帧两侧都把 HUD
   关掉了，区域里露的是**世界**（两边地图数据不同源）⇒ 比值恒 ≈95%。改用**美术对齐**判据：
   只比 `Prguse[1932/2190]` 里**不透明**的像素（同 `bigmap_viewport_check.py` 的思路）。

**实测（美术对齐判据，只比不透明像素）**

| 帧 | 腰带 `Prguse[1932]@(230,618)` 不符 | 技能栏 `Prguse[2190]@(0,0)` 不符 |
|---|---|---|
| 原版基线 | **3.3%**（腰带确实画着） | 81.9%（原版那格不是这张图 ⇒ 位置/档位不同） |
| 原版 Z/R 帧 | 76.3%（Z 之后腰带消失 ✓） | 82.1% |
| 本端基线 | **43.5%** | 5.8%（本端确实画着这张图） |
| 本端 Z/R 帧 | 43.5%（**没变** ⇒ 见下） | 5.8% |

> **2026-09-28 更新**：下面这两条后续线索已在 **§3.2m** 收口——第 1 条（43.5%）根因是
> 本端多画了一层 `Prguse[1933]` 叠层（删掉后 7.4%）；第 2 条（HUD 行"翻了没变化"）是
> `golden_ab_ours.ps1` 把翻转语义的 opener 调了两次 + HUD 状态跨行泄漏，两条都已修。原文保留。

**两个后续线索（本轮未收口，如实记录）**

1. **本端腰带的"画"与原版不一致**：位置正确（±6px 内最优就是 (0,0)），但 **43.5%** 的不透明像素与
   `Prguse[1932]` 不符（原版 3.3%）。已排除两种解释：① 不是位移（±6px 穷举最优 (0,0)）；
   ② 不是 `Prguse[1933]` 0.5 alpha 叠加（按叠加算反而 48.9%）。
2. **A/B 里本端 HUD 两行"翻了没变化"**：`ui_nodes_at(350,637)` 实测翻转会 4→0 节点、
   前后截图在该区域差 9072/9120（99.5%）——RPC 本身是好的；但**同一脚本跑出来的 A/B 帧
   在该区域与基线 0 差异**。即"手工探针有效、A/B 序列里无效"，最可能是脚本里 HUD 行的状态在
   基线取帧前就被别的步骤改过（基线帧本身可能就没有腰带）——下一轮先用 `ui_nodes_at` 把
   **基线帧那一刻**的节点数钉下来再谈像素。

### 3.2l 「只能靠鼠标到达」那批窗的 A/B：**本轮未采集**（2026-09-28，owner 解锁后重试）

§3.2b 的键位逐窗 A/B 覆盖 20 扇；§3.1 把「鼠标点开某个窗/点某一行」这条路堵掉之后，
剩下这些窗只有鼠标路径可达：`game_shop`（分类页签 `Previous/Next`、`PositionBar`）、
`npc` / `npc_goods`（NPC 菜单点行、滚轮命中区）、`inventory` 页签（ITEMS II / QUEST 切换）。
owner 2026-09-28 同意解锁工作站后，本节记录重试的过程与结论。

**结论分两段**：第一轮（真鼠标）没拿到成对帧 ⇒ 未采集；**第二轮找到可用路径后已经跑出数据**（见下"第二轮：注入消息路径可用"）。
真鼠标那一段仍按「未采集」处理，不给任何估算数字。

| 目标（鼠标路径） | 差异占比 | 定性 |
|---|---|---|
| `game_shop` 分类页签 `Previous/Next`（`Prguse2[197..199]/[207..209]@(120,103)/(120,421)`） | **未采集** | 未采集（帧拿不到） |
| `game_shop` 分类列 `PositionBar`（`Prguse2[205/206]@(120,117)` 拖动） | **未采集** | 未采集 |
| `npc` 菜单点行 / `npc_goods` 列表滚轮命中区 | **未采集** | 未采集 |
| `inventory` 页签 ITEMS II / QUEST 切换 | **未采集** | 未采集 |
| §3.2c–§3.2k 里因「原版侧压根没出那扇窗」而未比的其余窗 | **未采集** | 未采集 |

**本轮实际做了什么（可复跑的命令）**

```powershell
# 1) 解锁判据（README §3 口径）：桌面级 CopyFromScreen 是否仍是纯色
#    见下「两次读数矛盾」，这条判据本身在本次不够用
# 2) 起沙箱（7100）+ 原版客户端，键盘登录（键位路径不受锁屏影响）
Start-Process "$env:TEMP\golden_sandbox\Server\Server.exe" -WorkingDirectory "$env:TEMP\golden_sandbox\Server"
Start-Process "$env:TEMP\golden_sandbox\Client\Client.exe" -WorkingDirectory "$env:TEMP\golden_sandbox\Client"
pwsh -File tools\acceptance\csharp_golden\csharp_kbd_login.ps1 -SandboxRoot "$env:TEMP\golden_sandbox" -Account 333 -Password abbtest123
# 3) 摆窗 + 正对照（. csharp_client_driver.ps1 之后）
[CsUi]::SetWindowPos($global:csHwnd,[IntPtr]::Zero,0,0,1024,768,0x40)
Key-Cs 120                      # F9 开背包
Shot-Cs 'before'; Move-Image 301 13; Click-Image 301 13; Shot-Cs 'after'
```

**三个把这一轮卡住的实测事实（都值得下一轮先排掉）**

1. **桌面判据两次读数互相矛盾**：同一台机、相隔几分钟，`CopyFromScreen`（8px 网格 12288 点）
   先读到 **1 种颜色**（`#005495`，§3 的锁屏特征），后读到 **33 种颜色**（可读）。
   ⇒ 解锁状态在这几分钟里发生过切换，**单次采样不足以判定"现在能不能跑"**。
2. **原版客户端窗口会被拖走**：本轮实测窗口 rect = **`768,316,1024,768`** —— 屏幕右下角，
   只有四分之一在屏内。`Click-Image` 是「窗口 origin + 图像坐标」，此时**所有点击都落在屏外**，
   等于没点（`SetWindowPos` 摆回 `(0,0,1024,768)` 才谈得上测鼠标）。
   ⇒ 做鼠标 A/B 前**必须先核对窗口 rect**，否则会把"窗口在屏外"误判成"原版点不动"。
3. **注入式取帧在那一刻不出图**：`Shot-Cs`（向窗口 `SendMessage` `VK_SNAPSHOT`，等客户端
   自己的 D3D 截图落盘）连续报 `FAIL: 4s 内未出现新截图`（4 次里 3 次失败），
   另一次整段脚本在取首帧前就挂住（客户端 `Responding=True`，不是客户端死）。
   ⇒ 拿不到"点前/点后"两张帧，任何占比都是编的。

**下一轮的顺序（建议照抄）**

1. 先测三条前提：① 桌面 `CopyFromScreen` 连续两次都 >5 色；② 客户端窗口 rect 已 `(0,0,1024,768)`
   且 `GetForegroundWindow()` 就是它；③ 连续两次 `Shot-Cs` 都能出新图。
2. 前提全过 → 跑**判据自身的阳性对照**：F9 开背包 → `Shot-Cs` → **键盘再按 F9 关背包** → `Shot-Cs`，
   两帧在 `(0,0,316,236)` 的差异占比必须很大（证明"这个区域能反映窗开关"）。
3. 再跑**鼠标正对照**：同上但第二步换成 `Click-Image 301 13`。
   - 差异很大 ⇒ 鼠标路径通了，按上表逐窗铺开（每窗前后帧 + 占比 + 定性）；
   - 差异 ≈0 ⇒ **终点**：如实记「已解锁但注入鼠标仍驱动不了原版 `MirControl.OnMouseClick`」，
     不再重试（§3.1 已有同类记录），也不改产品代码。

### 3.2l-b 第二轮（2026-09-28，解锁成立）：**注入消息路径可用，真实鼠标仍不可用**

前置三条都过了：桌面 `CopyFromScreen` 12288 点采样 = **3988 种颜色**（可读；顺带查明第一轮那个
"1 色"其实是 `CopyFromScreen` 抛 **`句柄无效`** 的失败读数，**不是**锁屏特征——§3 那条判据要连异常一起看），
窗口 `SetWindowPos` 摆回 `(0,0,1024,768)`，`Shot-Cs` 连续两次成功。

**判据自证（先证明"这块区域能反映窗开关"）**：F9 开背包 → 取帧；键盘再按 F9 关 → 取帧。
背包区 `(0,0,316,236)` 与"已知关闭态"的距离：**开 = 87.1% / 关后 = 2.8%** ⇒ 判据有效。

**两条路径的结果（同一位置、同一判据）**

| 路径 | 操作 | 结果 | 判定 |
|---|---|---|---|
| 真实鼠标 `Click-Image`（`SetCursorPos`+`mouse_event`） | 点背包关闭钮 (301,13) | 开 87.1% → **点后 87.1%**（没变） | **点不动**（复现 §3.1 的结论，这次带判据自证） |
| 注入消息 `Msg-Click`（`SendMessage` `WM_LBUTTONDOWN/UP` 到窗口 hwnd） | 点背包关闭钮 (301,13) | 开 83.3% → **点后 4.3%**（回到关闭态） | **点得动** ✓ |
| 注入消息 | 点背包页签 ITEMS II / QUEST（(112,19)/(182,19)） | 页签带变化 **95.5% / 85.9%**，网格区 80.9% / 86% | **切页生效** ✓ |
| 注入消息 | 点商店分类上下页、商品上下页（(290,255)/(290,573)/(772,602)/(832,602)） | 每次点击**窗内变化 2.0–2.2%** | **点击生效** ✓ |

⇒ **修正 §3 / §3.1 的结论**：原版 `MirControl.OnMouseClick` **确实**响应注入的
`WM_LBUTTONDOWN/UP`（`Msg-Click` 能关背包、能切页签、能翻商店页）。§3.1 那次"注入点页签没反应"
的真因是**坐标系**，不是 OnMouseClick：

1. **窗口被拖到 `768,316,1024,768`**（四个区域只有右下角在屏内）——`Click-Image`（真实鼠标）走的是
   `窗口 origin + 图像坐标`，此时**全部落在屏外**；必须先 `SetWindowPos` 摆回 `(0,0,1024,768)`。
2. **C# 子控件 `Location` 是"面板内相对坐标"**：商店面板原点 `(164,146)`，`UpButton@(120,103)`
   的**屏幕**位置是 `(284,249)`——按相对坐标直接点会全部打空（本轮第一遍就是这样，四次点击窗内只差 0.1–0.2%，
   加上面板原点后立刻变成 2.0–2.2%）。背包窗面板原点 `(0,0)`，相对=绝对，所以它"看起来"一直是对的。

> **2026-09-28 更新**：下面第 2 条（背包**页序-身份对照表**）已在 **§3.2n** 收口——
> 两侧逐格命中，并顺带捞出 1 处真分歧（46 格时点 ITEMS II 的行为）。原文保留。

**仍未采集（如实）**

- `npc` / `npc_goods` 的菜单点行与滚轮命中区：本轮没跑（要用 `Msg-Click` 先点 NPC 开对话，再点行）。
- 背包**页序-身份对照表**：只证明"切得动"，还没把每一页与 C# 期望（`INV_TAB_ART` 的选中帧、
  46 格时 ITEMS II = 灰帧 169）逐页对上；另外"从 QUEST 点回 ITEMS"那次没复现（`t1` 与 `t4` 差 98.5%），
  要按上面的坐标系规则重测一遍。
- 逐窗「原版 vs 本端」占比：本端侧还需要把状态摆成**同页/同分类/同选中**才能比——本轮只做了
  "原版侧点得动"这一段。

### 3.2m 腰带 HUD 暗化收口（2026-09-28）＋ HUD 行的两个夹具坑

§3.2k 只证明「本端腰带区与 `Prguse[1932]` 有 43.5% 不符、且不是位移、不是 1933 叠层」。
本轮把根因钉死并修掉，顺带修掉 §3.2k 留的第二个线索（HUD 行"翻了没变化"）。

**1. 真根因：多画了一层 `Prguse[1933]`「叠层」**

- 本端早期把它当成**画在面板之上**的半透明叠层（`ImageNode::new(h).with_color(srgba(1,1,1,0.5))`）；
- C# 只在 `BeltDialog.BeltPanel_BeforeDraw` 里 `Libraries.Prguse.Draw(Index + 1, …, 0.5F)`
  ——`BeforeDraw` 跑在**控件自身那张图之前**，1932 面板随后把它盖住；且面板的**透明像素**处
  原版露的是**世界**（实测 (20,4)rel=(54,55,55)），不是这层的近黑（1933 实测 `(8,0,0,255)`、
  240x38 近乎全黑）⇒ **这层在可见画面里根本不出现**；
- 而 Bevy 在**线性空间**混合，0.5 alpha 的近黑把整块面板压暗：每个不透明像素 ≈ ×0.73
  （`(224,208,184)→(164,152,134)`、`(88,48,0)→(63,33,0)`）。

修复＝删掉该叠层（连同它的 marker、查询字段与逐帧贴图分支），不动其它任何东西。

**2. 复验：`belt_art_check.py`（新工具，只看 1932 的**不透明**像素）**

```powershell
py -3.12 tools/acceptance/csharp_golden/belt_art_check.py --shot <帧.png> `
    --lib Data/Prguse.Lib --index 1932 --rect 230 618 240 38 [--hotspot] [--offset-search 6]
# 退出码 0 = PASS（≥ 阈值）/ 1 = FAIL / 2 = 前置不满足；--hotspot 打印不符像素包围盒与粗网格
```

| 帧（同一把尺子） | 不符率 |
|---|---|
| 原版基线（金标准） | **3.3%** |
| 本端基线（**修复前**） | **43.5%** |
| 本端基线（**修复后**） | **7.4%** |
| 本端 Belt 行（按 Z 后） | **92.5%**＝腰带消失 ✓（原版同帧 76.3%，同判） |
| 本端 Skillbar 行（按 R 后） | **7.4%**＝腰带未受影响 ✓ |

口径与两条**不是缺陷**的坑：

1. **只比不透明像素**：面板透明像素处两侧露的是各自的世界（地图数据不同源）⇒ 参与比对的只有
   2918 个不透明像素。
2. **居中窗的 +1px 取帧口径**：把 rect 写成 `y=617` 时不符率 **53.9%**，`--offset-search 6`
   的最优偏移是 `(0,1)`（回到 3.3%）——即"差一行"是取帧/居中取整口径，**不是缺陷**，
   本轮没有为它改任何产品代码。
3. 残差 **7.4%** 的定性（`--hotspot`）：不符像素散布在面板内部（包围盒 `x[8..235] y[4..31]`，
   30x9.5px 粗网格每格 3–14 个），**不是位移**（±6px 最优仍是 `(0,0)`）、**不在边缘带**
   （上/下/左/右边界各 0）；其中右边缘一列（x210–239）的 44/35 个不符**原版帧同样有**
   ⇒ 那是另一个控件盖在 1932 这一块的共同行为。剩余约 4.1pp（原版 3.3% vs 本端 7.4%）
   **尚未逐像素定性，如实记为待查**（不推数）。

**3. 顺手修掉 §3.2k 的第二个线索：HUD 行"翻了没变化"**（`golden_ab_ours.ps1`，两个夹具坑）

- **翻转被调两次**：HUD 行的 opener 是**翻转**语义，脚本却在**循环前**与 `try=1` 各调一次
  ⇒ 互相抵消。实测 `ui_nodes_at(350,637)`：`3 → 0 → 3`，最终帧与基线同态，A/B 里看起来
  就是"这块没变化"（§3.2k 记的现象）。修：HUD 行跳过循环前那次调用（普通窗 `dialog open`
  幂等，重复无害，不动）。
- **HUD 状态跨行泄漏**：Belt 行关掉腰带后，后面 Skillbar 行取到的帧里腰带**还是关的**。
  修：每行归零时显式 `hud_toggle {on:true}` 把两行 HUD 摆回基线态（基线帧同样显式置位，
  不再依赖默认值）。

修后整表 A/B 里这两行是 **97.9% / 97.4%**——正是 §3.2k 预期的"两侧都关掉 ⇒ 区域里露世界"
的值，**这两行仍只认 `belt_art_check.py` 这把尺子**（拿 `golden_ab_diff.py` 比整块恒 ≈95%，
不代表画错）。

### 3.2n 「背包页序-身份」对表（2026-09-28）：两侧逐格命中，并捞出 1 处真分歧

§3.2l-b 留的第一条未采集项：「只证明页签**切得动**，没把每一页与 C# 期望的**选帧**对上」。
本轮把它变成**可复跑的数值表**——不靠看图，靠"这一格到底是哪张美术"。

**新工具：`art_match.py`（同一格的候选美术里，报哪一张最像）**

```powershell
py -3.12 tools/acceptance/csharp_golden/art_match.py --shot <帧.png> --lib Data/Title.Lib `
    --rect 6 7 72 23   --candidates 737,197 `
    --rect 76 7 72 23  --candidates 738,168,169 `
    --rect 146 7 72 23 --candidates 739,198 --threshold 0.85
# 退出码 0 = 每格都有候选命中（不符率 ≤ 1-阈值）/ 1 = 有格子没命中 / 2 = 前置不满足
```

口径与 `belt_art_check.py` 同一套（**只比美术的不透明像素**；矩形是 1024x768 逻辑坐标），
两个脚本共用 `art_match.py` 里的 `load_lib/extract/crop_logical/compare`（单一来源，避免漂移）。
候选表就是 C# `InventoryDialog.cs:45/56/67/254-256/341-343/363-365` 的三条换图规则：
`ItemButton(ITEMS I)` 197↔737、`ItemButton2(ITEMS II)` 168↔738（46 格时灰帧 **169**）、
`QuestButton(QUEST)` 198↔739。

**实测（两侧各取 4 帧；括号里是不符率，越低越像）**

| 操作（46 格背包） | ITEMS I | ITEMS II | QUEST |
|---|---|---|---|
| 打开背包（第 1 页）· **原版** | 197 (0.119) | **169** (0.096) | 739 (0.085) |
| 打开背包（第 1 页）· **本端** | 197 (0.119) | **169** (0.096) | 739 (0.085) |
| 点 QUEST · **原版** | 737 (0.091) | 169 (0.096) | 198 (0.114) |
| 点 QUEST · **本端** | 737 (0.091) | 169 (0.096) | 198 (0.114) |
| 点回 ITEMS I · **原版** | 197 (0.119) | 169 (0.096) | 739 (0.085) |
| 点回 ITEMS I · **本端** | 197 (0.119) | 169 (0.096) | 739 (0.085) |
| 点 ITEMS II · **原版** | 197 (0.119) | 169 (0.096) | 739 (0.085) |
| 点 ITEMS II · **本端** | **737** (0.091) | **168** (0.138) | 739 (0.085) |

⇒ 前六格**逐位相同**（连不符率都一样，因为两侧画的是同一份美术、同一套布局）；
**最后一行是真分歧**（下一节）。「46 格时 ITEMS II = 灰帧 169」这条判据在两侧都实测成立。

**真分歧：46 格时点 ITEMS II，原版弹"扩容"框、本端直接切页**

- **原版**：页签**不变**（仍是 197/169/739），改弹一个**居中大窗**（实测新出现矩形
  `x192..895 / y256..511`）——就是 `InventoryDialog.cs:230-239` 那条
  `if (GameScene.User.Inventory.Length == 46 && sender == ItemButton2) → MirMessageBox(ExtraSlots8, OKCancel)`，
  OK 按钮发 `C.Chat { Message = "@ADDINVENTORY" }`。也就是说**未扩容的背包上 ITEMS II 页不可达**。
- **本端**：`inventory.rs:1141-1145` 的页签边沿处理是"无条件 `inv_ui.page = t.0`"⇒ 直接切到第 2 页，
  且 `inv_tab_art_index` 把该页签画成**选中帧 168**（金亮），原版那格始终是灰帧 169。
  本端的扩容入口是**第 2 页上的 BUY 按钮**（`inventory.rs:2066-2108`，已与 C# `RefreshInventory2`
  的 `AddButton.Visible` 对齐）——第 2 页在原版不可达、在本端可达，两条路由此分叉。
- 定性：**产品行为分歧（未收口）**，不是绘制缺陷。修它要先定"扩容入口放哪"（照 C# 走
  `@ADDINVENTORY` 提示框，还是保留本端 BUY 路径），属设计取舍 ⇒ 已开 issue 记账，
  本轮**不改产品代码**。

**本轮踩到并写进配方的四个坑（都在这台机器上实测）**

1. **窗口必须置顶、且点击前先把真实光标移到位**：`SetWindowPos(hwnd, HWND_TOPMOST(-1), 0,0,1024,768)`
   ＋`Move-Image x y` 之后 `Msg-Click x y` 才落地；不置顶时同一套坐标点了没反应
   （第一版拿到四张"完全一样"的帧就是这么来的）。
2. **按键要用消息、不要用 `keybd_event`**：新增驱动函数 `Msg-Key <vk>`（`SendMessage`
   `WM_KEYDOWN/WM_KEYUP`，不依赖前台焦点）。`Key-Cs` 发给**前台窗口**——脚本一退出焦点就回终端，
   实测同一脚本里 F9"有时生效有时不生效"，就是这个。
3. **F9 是开关不是"开"**：面板开着再按就关掉。要确定性取"打开态"，先点关闭钮置零再按 F9
   （或先重启客户端）。
4. **点击后把光标移出被测格再取帧**：客户端会画自己的光标精灵，停在页签上会把那一格读数带偏
   （实测把 tab0 的 737/197 判反）。

**仍未采集（如实，附下一轮的前置）**

- `npc` / `npc_goods` 的**菜单点行**与**列表滚轮命中区**：原版侧要"在世界里点到 NPC"得有它的
  **屏幕坐标**（本端有 `nearby`，原版没有等价探针）；本端侧 `npc_call`/`npc_rows`/`wheel`/`scroll`
  都是现成的。下一轮先解决"原版侧怎么把 NPC 点开"（或改用本端自证：`npc_rows` 给的行矩形 +
  `wheel` 注入 + 行文本/`scroll` 读数对照）。

### 3.2o §3.2n 捞出的分歧已修：46 格点 ITEMS II 不再换页（2026-09-28，issue #3332）

**决策**（`jev-decisions classify`，规则 `RULE_通用决策先自行用jev出结论不向owner求证`）：
候选 `align_prompt`（46 格不换页 + 复用本端已有扩容确认框，保住扩容入口）
**0.880 / 置信度 0.840**，`keep_current` 0.070、`other` 0.050 ⇒ 采用 `align_prompt`。

**改了什么**

1. `Client-Bevy/src/game/dialogs/inventory.rs` 新增纯函数 `inv_tab_target_page(tab, slots)`：
   `(ITEMS II, 46 格)` → `None`（**不切页**），其余原样返回目标页。页签边沿处命中 `None` 时
   调 `request_expand_confirm`（等价 C# `InventoryDialog.cs:230-239` 那个
   `MirMessageBox(ExtraSlots8)`；确认走 `inv_confirm_system` 的 mode 2 → `@ADDINVENTORY`）。
2. 扩容确认抽成 `request_expand_confirm(confirm, len)`，**BUY 按钮与页签两条入口共用**。
3. **顺带修掉同一处的数值缺口**：确认文案里的费用原先按 `(len - 40) / 4` 算（`GRID_COLS*GRID_ROWS`
   = 40），而 C# `InventoryDialog.cs:90-91` 与**本端服务端实扣**
   （`ServerRust/src/actors/world/session.rs:7613-7617`）都是 `(len - 46) / 4`
   ⇒ 修复前**每一档都多报 1M**（46 格显示 2,000,000、实扣 1,000,000；54 格显示 4M、实扣 3M）。

**实机证据（本端，`Crystal-wt-npc` 构建）**

| 步骤 | 读数 |
|---|---|
| 打开背包（46 格） | 页签 **197/169/739**（`art_match.py`，与 §3.2n 原版逐位相同） |
| `click 112 18`（ITEMS II） | 命中 `72x23 [root=Inventory]`；页签**仍是 197/169/739**（**没有**切页） |
| 同上帧的确认框区 `(284,289,456,190)` | 与点前差 **86372/86640 = 99.7%** ⇒ 确认框弹出 |
| 客户端日志 | `背包未扩容（46 格）：ITEMS II 不切页，改为扩容确认` → `📦 请求背包扩容` |
| `click 582,458`（确认钮 Yes：面板 `(284,289)` + 钮 `(260,157)` 76x25） | 命中 `76x25 [root=Inventory]`；`bag_probe` **total 46 → 54**（服务端真的扩容了） |

**探针的持久状态已复原**（`LESSON_验收夹具须对持久游戏态幂等`）：扩容改了测试账号 `bevychar` 的
`characters.backpack_size`（46→54）并扣了 1,000,000 金币；停服后把 `backpack_size` 改回 46、
金币补回 1,000,000，重启登录复核 **`bag_probe total=46`**（DB 回读 `backpack_size=46 / gold=2003244`）。

**门禁**：`cargo test --lib` **856 passed**（新增 2 条：`second_bag_page_needs_expanded_bag`、
`expand_confirm_matches_csharp_and_server_cost`）+ `cargo test --test b0001_smoke --test ui_alignment`
（2 + 53）+ `rustfmt --edition 2021 --check` 0 + 实机交互巡回 **44/44 exit=0**。

> **2026-09-28 更新**：这条已在 **§3.2p** 收口——框体美术与两颗钮**逐像素相同**，
> 并据此修掉了按钮组（`200/203` vs `206/210`）与文案两处口径差；残差只剩**文字字形**，如实记录。

### 3.2p 确认框（`MirMessageBox`）框内对拍：按钮组 + 文案两处口径差已修（2026-09-28）

§3.2o 只证了"框弹出来且能确认"，框内像素记为未采集。本轮把它做完，靠的是**几何由美术图头决定**
（不用在整幅图上找框）：`Prguse[360]` 图头 **456x190** ⇒ C# `MirMessageBox.cs:26` 按自身尺寸居中
⇒ 框恒在 `(284,289)`；两颗钮的 C# 位置是框内 `(260,157)`/`(360,157)`、图头 76x25
⇒ 屏幕 `(544,446)`/`(644,446)`。原版侧配方仍是 §3.2n 那套（置顶 + `Msg-Click`，46 格点 ITEMS II
弹 `ExtraSlots8` 框，取"框开"帧；**别点左钮**，那会真扩容）。

**逐格身份对拍**（`art_match.py`，同一格候选 `200,203,206,210`）

| 帧 | 左钮 `(544,446)` | 右钮 `(644,446)` |
|---|---|---|
| 原版 | **`Title[200]`**（不符率 0.000） | **`Title[203]`**（0.000） |
| 本端（修前） | `Title[206]`（0.000） | `Title[210]`（0.000） |
| 本端（修后） | **`Title[200]`（0.000）** | **`Title[203]`（0.000）** |

C# `MirMessageBox` 有两套按钮组（`MirControls/MirMessageBox.cs`）：
`OKCancel` = OK `200/201/202` + Cancel `203/204/205`（`:54-75`）、
`YesNo` = Yes `206/207/208` + No `210/211/212`（`:76-97`）。
扩容**两条**路径 C# 都用 `OKCancel`（`InventoryDialog.cs:88-98` 的 BUY 与 `:230-239` 的 ITEMS II），
而本端过去**一律**画 YesNo ⇒ 新增 `confirm_button_art(mode)`（mode 2/4 → OKCancel，其余 → YesNo），
由 `inv_confirm_system` 每帧按 mode 改写两颗钮的 `ImageButton` 三态句柄。

**文案**：C# 用的是本地化表（`Client/Localization/Chinese.json`）里的两条 ——
`ExtraSlots8`（`:131`，ITEMS II 路径，文案里**写死** 1,000,000）与
`ExtraSlots4`（`:132`，BUY 路径，`{0:###,###}` 千位分组）。
本端过去是自造文案「花费 N 金币扩展背包格？」⇒ 逐字复刻这两条（新增 `group_thousands` 实现
`###,###`），并区分 mode：**2 = 页签路径**、**4 = BUY 路径**（两者动作相同，都发 `@ADDINVENTORY`）。
配套把确认标签改成**定宽 390**（C# `Label Size=(390,110)` 定宽折行）——自适应宽会把长文案拉成一行、
溢出框外（改长文案后的实机截图里首行被裁）。

**实机残差**（两侧都切中文 locale，同一串文案）

| 区域 | 不同像素 |
|---|---|
| 两钮带 `(544,446)`–`(720,471)` | **0 / 4400 = 0.0000** |
| 框体其余（除文案与两钮） | ≈ 42 px |
| 整框 `(284,289,456,190)` | 8.47% |
| 文案区 `(319,324,390,110)` | 17.00% |

⇒ **面板美术与两颗钮逐像素相同**；唯一残差是**文字字形**：同一串、同一区域，本端 CJK 字体与
C# GDI 字体的度量差导致折行点差一个字符（原版 `…你可以 / 解锁…`，本端 `…你可 / 以解锁…`）。
这条如实记为**字体渲染差**，不是口径差（字符串已逐字相同、有单测钉住）。

**沙箱踩坑（记一笔）**：沙箱那份原版默认跑**英文 locale**（`Client/Language.ini` 是英文），
第一遍对拍出来的"文案不同"其实是**语言不同**；把沙箱**副本**的 `ExtraSlots8` 换成中文条目后，
才是同串对拍（仓库里的本地化文件没动）。

## 3.3 逐窗几何对表（不需要原版交互，2026-09-26 起）

§3.1 把"驱动原版点开某个窗口"这条路堵掉之后，**窗口几何**仍可验：原版每扇窗的矩形是纯常量
（`Index`+`Library` 定图 → 图头给尺寸；`Location` 给位置，或 `Location = Center;` 居中），
把这三样从 C# 读出来就是期望矩形，与我方 `dialog_rect` 实机值比即可。

```powershell
# 1) 我方：一次起客户端，按单一真源 manifest 逐窗「开→取矩形→关」
pwsh tools/acceptance/csharp_golden/probe_ui_nodes.ps1 -Repo <wt> -ClientHome <wt> `
     -RectKinds all -InventoryOnly -Out %TEMP%\rects_all.json
# 2) 对表（期望取自原版 C# 常量 + 同一套 .Lib 的图头）
py -3.12 tools/acceptance/csharp_golden/window_rect_table.py `
     --src <原版仓库> --data <含 *.Lib 的 Data> --compare %TEMP%\rects_all.json
```

**结果（2026-09-26，master f6a43efdc 上跑）：31 个可比窗口全部一致，0 处几何差异。**

**2026-09-27 复核（master `c02f90d38`）**：`dialog_rect` 加了 `fallback:"root"` 之后，无标准关闭钮的 `menu`/`minimap` 也进了对表（其余 4 扇 `buff/refine/timer/chat_notice` 不在本表的 C# 映射里）；顺带修掉解析器把**解不出的符号名当 0** 的假 DIFF（`MenuDialog` 的 y 原被算成 15，真值 349 = `(768 - Prguse[1]高152) - 282 + 15`）——现为 `不一致：0`（可比 33 窗：另把 `refine` 按 C# `Prguse[1002]` 164x207@(0,225) 纳入，实测 OK；`buff`/`chat_notice`/`timer` 是**条件可见**窗，探针那一刻不在屏上，取不到矩形，理由写在 `window_rect_table.py` 的 `CLASS_TO_KIND` 下方注释里）。
例外三类，各有据：

1. **无关闭钮的窗**（`menu/minimap/buff/refine/timer/chat_notice`，即 manifest 的
   `no_close_by_design`）：`dialog_rect` 默认仍**从关闭钮反推**窗口矩形，没钮就返回
   `{ok:false,error:"close button not found"}`（交互巡回靠这个 `ok:false` 判别「无钮设计窗」，
   所以默认语义不能改）。**2026-09-27 补**：加了一条可选路径 `dialog_rect {kind, fallback:"root"}`
   —— 没钮时直接取该 kind 的**可见根面板**矩形（`source:"root"`，`cx/cy` 给矩形中心），
   探针在逐窗分支已默认带上，这 6 扇窗从此**也进对表**。
2. `trade` 未取到：它在 manifest 的 `excluded` 里（单开不可达，需要对手方/服务端状态）。
3. **按状态换图的窗**：`MountDialog` 面板随坐骑孔数在 `Prguse[167]`(324x377)/`Prguse[160]`(272x378)
   之间切（C# `SwitchType`），拿哪一张取决于角色状态——工具里登记为"任一命中即 OK"，
   与背包 ITEMS II 的 `738/169` 同一类，别当缺陷。

写这个解析器时踩到并修掉的三个坑（都已写进 `window_rect_table.py` 的注释，避免后人重踩）：

- **注释行**：`RankingDialog` 里 `//Size = new Size(288, 324);` 是注释掉的历史值，真值是背景图
  原生 324x441。不剥注释就会拿注释值判出一条假 DIFF。
- **居中不是"没有位置"**：原版大量窗写 `Location = Center;`
  （`MirControl.cs:643` = `((ScreenWidth-Size.Width)/2, (ScreenHeight-Size.Height)/2)`），
  `FriendDialog/GroupDialog/GuildDialog/OptionDialog/MentorDialog/RankingDialog/RelationshipDialog`
  都是这一档；不认它会把它们全判成"期望 (0,0)"。
- **位置是表达式或由调用点动态锚定**：`CharacterDialog` 是 `ScreenWidth - 264`；
  `MailListDialog` 是 `(ScreenWidth - Size.Width) - 150`；`CraftDialog` 在
  `Show()` 里 `= (InventoryDialog.X - 12, Y + 236)`（`NPCDialogs.cs:2450-2452`）；
  `SocketDialog` 在 `SocketDialog.cs:107-110` 里按背包算
  （`bag.X + (bag.W - w)/2, bag.Y + bag.H + 5`，且是**整数除法** ⇒ 117 不是 118）。
  工具按"背包在 (0,0)/316x236"求值——探针正是先开背包再逐窗开，这个前置是确定的。

另外两处**映射口径**（会影响"该跟谁比"，不是差异）：我方 `quest_log` 对应原版
`QuestDiaryDialog`（`(ScreenWidth/2-300-20, 60)` = (192,60)），**不是** `QuestListDialog`
（那扇是贴在 NPC 窗右边的列表变体，`(NPCDialog.Width+47, 0)`）；`item_rental`（204x109）对应
`ItemRentingDialog`，`item_rental_browse`（400x174）才对应 `ItemRentalDialog`。

> 边界：这是**窗口级**几何。窗内控件（行高、列距、按钮位置）要靠 `ui_nodes_at` 逐点或像素 A/B
> —— 例如角色窗 14 个装备格已用前者验过（14/14 与 C# 表一致）。

**窗内控件抽点的前置：先开窗（`-OpenKinds`，2026-09-26 补）**

`-RectKinds` 的**逗号表**分支只取 `dialog_rect`、不负责开窗；`all/sweep` 分支虽然逐窗开，
但结尾会把 `no_close_by_design` 的那几个（menu/minimap/buff/refine/timer/chat_notice）也关掉，
而抽点在那之后 ⇒ 想对某个窗的窗内控件抽点，此前没有任何开关能让它留在屏上。
现在加 `-OpenKinds 'menu'`（逗号表）：抽点前逐个 RPC `dialog open` 并**留在屏上**，结果记 `opened_<kind>`。

实测（menu，master `d96dd26bd` 的 e2e 构建位）：

```powershell
pwsh tools/acceptance/csharp_golden/probe_ui_nodes.ps1 -Repo <wt> -ClientHome <wt> `
     -OpenKinds menu -RectKinds '' -Points '1007,371;1025,371;1007,618;1025,618' -Out %TEMP%\menu.json
```

- `(1007,371)` → 顶层节点 rect = **[991.33, 361.33, 32.0, 20.0]** = C# `ExitButton` @(3,12)+面板(988,349)，
  尺寸 = `Title[633]` 图头 32x20；
- `(1007,618)` → **[991.33, 608.67, 32.0, 18.0]** = `GuildButton` @(3,259)，尺寸 = `Prguse[1994]` 图头 32x18；
- `(1025,·)` → **0 个节点**（面板只到 x=1024；修复前按钮写死 38 宽会盖到 1029）。

顺带一条判据：`dialog_rect kind=menu` 返回 `{ok:false, error:"close button not found"}` 是**预期**
——`dialog_rect` 靠关闭钮反推窗口矩形，而菜单窗在 C# 里就没有关闭钮（`no_close_by_design`）；
窗内控件抽点不需要 `dialog_rect`。

**点完再看变化：`-ClickPoints`（2026-09-26 补）**

有些判据是「点一下某控件之后屏上该出现/不该出现什么」——例如 #3258「仓库 `ProtectButton` 点下去
必须弹 `MirInputBox`（`Prguse[660]` 288x156 @(368,306)），而不是本端旧的自造密码面板」。
`-ClickPoints 'x,y;x,y'` 在 `-OpenKinds` 之后、抽点之前逐个 `click`，就能直接抓这个变化：

```powershell
pwsh tools/acceptance/csharp_golden/probe_ui_nodes.ps1 -Repo <wt> -ClientHome <wt> `
     -RectKinds '' -OpenKinds storage -ClickPoints '352,46' `
     -Points '512,384;61,459;512,330' -Out %TEMP%\storage_pwd.json
```

实测（PR #3258 的本端构建位）：点仓库保护钮后 `(512,384)` 命中 **`[368,306,288,156]`**（= `MirInputBox`），
旧自造面板的按钮位置 `(61,459)` 命中 **0 个节点**，`(512,330)` 命中输入框与它的输入区容器。

### 3.4 窗内控件：`control_size_audit.py`（写死尺寸 ≠ 美术原生尺寸 = 0）

原版大量控件只写 `Index`/`Library`/`Location`，**尺寸就是美术尺寸**；本端若在这些地方写死一个数字，
很容易抄成另一张图的尺寸（实测：技能页翻页钮抄成 40x22、三处标题图抄成 `Title[15]` 的 103x17）。
这类"拉伸精灵"逐个窗口肉眼找太慢，用机械扫描一次扫全仓：

```powershell
py -3.12 tools/acceptance/csharp_golden/control_size_audit.py `
     --repo <Rust 仓库根> --data <含 *.Lib 的 Data>
```

判据（**启发式，输出要人工过一遍**）：把 `load_lib_image(…, LibraryName::X, N)` 的句柄变量与
其后 ≤8 行、且中间没有别的 `load` 的 `spawn_icon_button/spawn_image` 里那个**同名句柄**配对，
再比"写死的 (w,h)"与"X[N] 图头尺寸"。配对**必须靠句柄变量名**：只按"最近一次 load"配会把面板背景
配到小按钮上（实测产出 48 条、大半是假阳性）。

**当前结果：0 命中**（2026-09-26，master 0f9da9ae4 上跑）。作为门禁跑的意义是"新增控件时别再抄尺寸"；
真要"按比例裁宽"的精灵（进度条/经验条，C# 用 `Draw(Index, section, …)` 自绘）会落进结果里，
人工确认后加白名单，别直接当缺陷改。

**门禁语义与自证**（2026-09-26 起）：

- 命中 >0 → `VERDICT=FAIL` 且 **exit 1**；0 命中 → `VERDICT=PASS` 且 exit 0 ⇒ 可直接当门禁；
- `--selftest` 跑正/负对照并给出 `VERDICT`：负对照（原样扫必须 0）+ 正对照（**把一处改回修复前的
  写法**——`if let Some(h) = load_lib_image(…); spawn_image(p, h, …, 57.0, 15.0, …)`——必须报出来）。
  正对照是这条门禁"真的会红"的证据，不是自证式断言；
- **扫描面**（正对照的形态也界定了它）：只认「`load` 的句柄变量名 ↔ 后续 spawn 传同一个变量」。
  把 `load_lib_image(...)` **内联**当参数传给 spawn 的写法扫不到（第一版正对照就是这么写的，
  结果扫不出来；改成真实历史形态才成立）。所以它**不是**"所有尺寸问题都能抓"，是"这一类最常见的形态能抓"；
- 已登记进 `docs/DELIVERY.md` 的"离线对账/卫生门禁"清单（第 6 条）。

**2026-09-26 补：修掉一处**严重漏报**，并引入"已知待核表"**

- **漏报根因**：判据原先按「`load` 的句柄变量名 ↔ spawn 同名变量」配对，而仓库里**最主流的写法**
  是 `if let (Some(n), Some(h), Some(pr)) = (load…, load…, load…) { spawn_icon_button(p, n, h, pr, …) }`
  —— 元组里**没有 `let x =` 绑定**，配对直接跳过整组 ⇒ 工具长期报"0 命中"却是**假绿灯**
  （实测：`big_map` 滚屏箭头把 `Prguse2[197]`(12x12) 写成 16x14 就是这样漏掉的）。
  现在补上元组解析，并**优先取 normal 帧**（控件尺寸按 C# `MirImageControl.Size` = `GetTrueSize(Index)`，
  取的是当前 `Index` 即 normal 帧；拿 hover 帧比会造假 FAIL）。
- **由此浮现 30 处**（跨 game_shop/group/inventory/keyboard_layout/mentor/npc_goods/potion_belt/
  quest_log/storage/big_map 等），已登记进 `control_size_audit_known.txt` 作为**待办队列**
  （不是"白名单通过"）：清单内只提示，**新增命中才 FAIL**，`--selftest` 的负对照也按"新增 0"判。

**2026-09-26 再补：第二种扫描面（常量表 + `for` 循环），队列清零**

- **又一处漏报**：`const TBL: &[(…, LibraryName::X, n, h, pr, y, w, h)]` +
  `for (…, bw, bh) in TBL { load(…, *lib, *n) … spawn_icon_button(…, *bw, *bh, …) }` 这种**表驱动**
  写法里 lib/idx 是**变量**，只认字面量的配对逻辑整组看不见 ⇒ 实测 `menu.rs` 13 颗菜单钮统一写死
  38x19（图头 `Title[633/636]`=32x20、其余 `Prguse/Prguse2`=32x18）**报 0 命中**。
  现在新增一路扫描：解析常量表行 + 循环变量到列的映射，**逐表行**比"表行尺寸列 vs 该行 normal 帧图头"。
  陷阱记录：不能写成 `Some\((\w+)\)\s*=\s*load` —— 元组绑定是 `(Some(a), Some(b), Some(c)) = (load…, …)`，
  `=` 后面还有一个 `(`，这么写会一条都匹配不到（第一版就是这样，靠"正对照②"当场抓出来）。
- `--selftest` 现在有**三条对照**：负对照（原样 0 新增）+ 正对照①（字面量路：改 `group.rs` 标题图尺寸）+
  **正对照②**（表驱动路：把 `menu.rs` 的 `*bw, *bh` 换回 38x19 → 必须逐行报 13 条）。
- **首轮 30 条待核队列已全部核完并清零**（`control_size_audit_known.txt` 现为空队列，只剩判定口径注释）：
  17 处按图头改尺寸（PR #3255）+ 菜单 13 颗钮（PR #3256）。

### 3.5 窗口触发：`dialog_trigger_audit.py`（「窗口存在 ≠ 功能存在」）

**为什么需要**：2026-09-27 逐窗核长尾窗时发现 `chat_notice`（顶部公告横幅）——窗、面板、
`dialog chat_notice open` 的 RPC、自检夹具**都齐**，但 `ChatNoticeState.visible` 全仓
**没有任何 `= true` 写入方**（只有计时归零写 `false`）⇒ 实机上横幅永远不出现。
尺寸审计、几何对表、交互门禁都看不见这类缺陷——它们只回答"窗能开吗、控件几何对不对"，
不回答"**谁把它打开**"。

```powershell
py -3.12 tools/acceptance/csharp_golden/dialog_trigger_audit.py --repo <Rust 仓库根>
py -3.12 tools/acceptance/csharp_golden/dialog_trigger_audit.py --repo . --selftest   # 正/负对照
py -3.12 tools/acceptance/csharp_golden/dialog_trigger_audit.py --repo . --openers    # 附：打开方报表
```

**判据（启发式，输出人工过一眼）**：找 `pub struct <X>State` 里 `visible/open/managing/shown/composing`
这些**开语义的 bool 字段**，要求至少有一个 `.<field> = true` 写入方，来源三选一：
① `impl <X>State` 内的 `self.<field> = true`（状态自带 `show()/open()`）；
② 取到该 state 可变引用（`ResMut<…XState>` / `&mut XState`）的函数体；
③ 提到该 state 类型的文件（兜住类型推断的局部变量，如 `npc.visible = true`）。

**关键口径：`control.rs`（RPC 探针）与 `auto/*`（自检夹具）不算触发**——"只有探针能开"
正是这条门禁要抓的反模式；把探针算进去会让它永远绿。

**已知待核表** `dialog_trigger_audit_known.txt`（`State<TAB>field`）：清单内只提示、新增才 FAIL。
处置二选一：补真实触发，或确认是死字段并删掉（首个实例 `RankingState.visible` 就是无人写、
读取处还 `|| mgr.is_open` 的冗余位，已按"删死字段"收口）。

**门禁语义与自证**：命中 >0 且不在表内 → `VERDICT=FAIL` 且 exit 1；`--selftest` 跑负对照
（原样 0 新增）+ 正对照（**把 `ChatNoticeState::show` 里的 `self.visible = true;` 摘掉** ⇒
必须报出 `ChatNoticeState.visible`）；已登记进 `docs/DELIVERY.md` 的离线门禁清单（第 8 条）。

**边界**：`--openers`（每个 `DialogKind` 的玩法侧 `mgr.open/toggle` 入口）是**报表不是门禁**
——状态驱动窗（Storage/InputBox/HeroManage/ChatNotice…）不走 `mgr.open`，只看它会全部误判。
另外它只查"有没有入口调用"，判"该不该有入口"仍要读 C#（例如 `report` 的入口
`ReportButton` 在 C# 里构造即 `Visible=false` 且全树无处置 true ⇒ 本端不做入口才是对的）。

## 4. 存档导出（dbtool）

`dbtool` 用原版 `Server.Library.dll` 读 `Server.MirDB`（游戏数据）与 `Server.MirADB`（账号/角色），
输出 JSON：

```powershell
dotnet run --project .\dbtool\dbtool.csproj -c Release -- <沙箱>\Server export <沙箱>\db_export.json
dotnet run --project .\dbtool\dbtool.csproj -c Release -- <沙箱>\Server dump Server.MirDatabase.CharacterInfo
```

实测（原版 `MirADB` 2025-12-22 / `Server.Library.dll` 2025-10-05）：

- `Envir.LoadDB()` → `True`；
- `Envir.LoadAccounts()` 直接调用会抛 `NotImplementedException: GameMaster has not been implemented.`
  （`Buff..ctor` → `Envir.GetBuffInfo`）；**把 `BuffType` 的 60 个枚举值预注册进 `BuffInfoList` 后即可加载**，
  导出 3 个账号 / 5 个角色；
- 离线单独调用时 `MapInfoList`/`ItemInfoList`/`MonsterInfoList`/`NPCInfoList`/`QuestInfoList`/`GameShopList`
  计数为 0（这些由服务端自身初始化路径装载，不在 `LoadDB()` 里）→ 导出里的物品名解析为空；
  需要靠服务端进程内的状态导出物品名称。

## 5. 两个会把沙箱文件改坏的坑（实测）

1. **原版启动器会删文件**：`Client/Forms/AMain.cs:137 CleanUp()` 按「补丁清单」删掉目录里
   不在清单内的文件（`NeedFile()` 对空清单恒为 false）。补丁站不可达时清单为空 → 会删掉
   `Client.runtimeconfig.json`/`Client.deps.json`/`KeyBinds.ini`/`Language.ini`/各 DLL 等，
   之后客户端报 .NET apphost 启动失败。**因此沙箱里必须 `[Launcher] Enabled=False`；
   也不要在真实安装目录里跑这个启动器**。沙箱大目录用 junction 时更要小心：
   `CleanUp()` 用 `SearchOption.AllDirectories` 会沿 junction 走进真实美术目录。
   （本次实测：真实安装目录 `Client\Data` 1449 文件 / 7277.9 MB、`Map` 1624 / 833.3 MB、
   `Sound` 1608 / 436.2 MB，事后复核**未被删除**。）
2. **离线 `LoadDB()` 会重写 `Server.MirDB`**：一次 `export`/`setpw` 就把它从 540,512 字节
   压到 240 字节，服务端随后启动即报 `0 Maps Loaded` /
   `Cannot start server without atleast 1 Map and StartPoint`。`dbtool` 现已内置
   `Server.MirDB.offline-bak` 备份/还原（每次运行都会还原成原版字节）。
   另外 `SaveAccounts()` 重写的 `Server.MirADB` 比原文件小（20455 → 12685 字节），
   做真实数据演练前请先复制一份存档。
