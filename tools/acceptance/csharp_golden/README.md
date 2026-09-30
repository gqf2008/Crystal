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

> **账号 `333` 的口令 = 原版 DB 里的 `333333`**（owner 2026-09-30 确认；`make_sandbox -Force`
> 从原版 `Server.MirADB` 重拷后就是这个值）。下文 §3.2b / §3.2bl 那段
> `setpw 333 abbtest123` **只在显式改过密码之后**成立——照抄 `abbtest123` 会一直停在登录界面
> （§3.2bk ③ 踩过一次）。

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

> **⚠️ 2026-09-28 撤回（见 §3.2ak）**：本节"原版鼠标点不动"这个**前提不成立**——那两次 `Click-Image`
> 都是在**非置顶窗口**下跑的（真实输入落到了压在上面的窗口）。真置顶 + 真实光标后复测：
> `Click-Image` 能关背包（73370 px）、能点 `ITEMS II` 页签（弹出居中确认框 86523 px）。
> 下文的两条"替代路线"仍然有效（源码对表/键盘路径本身没错），但**不是**因为鼠标不能用。

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
#     注意：**不改密码时沙箱就是原版口令 `333` / `333333`**（见 §2.1 的提示框）
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

> **2026-09-28 更新（§3.2f 的「居中窗 ±1px」待办，详见 §3.2ac）**：`golden_ab_diff.py` **默认带**
> `--max-shift 1`，同批帧复跑后 Friends **25.5% → 6.6%（dx=1）**、Help **87.6% → 10.7%（dx=1）**，
> 其余窗 dx=dy=0 ⇒ 只有居中窗在这对帧里偏 1px，是**取帧口径**不是本端排版 bug；
> Friends 的残余里有 3546 px 压在**翻页条那一行**（新线索，留给下一批）。

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

> **2026-09-28 更新（见 §3.2ad）**：已补上（`scan_loop_literal_size` + `TABLE_DECL_LET`），
> 并用它捞出两处真缺陷（Friends 5 颗操作钮写死 24x22 而图头 28x25、Help 关闭钮写死 16x16 而图头 24x21），
> 两处都已改成按图头取尺寸，正/负对照与实机 A/B 都在 §3.2ad。

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

> **⚠️ 2026-09-28 更新（见 §3.2ak）**：本节"真鼠标不可用"的判断**作废**——当轮 `Click-Image` 打在
> **非置顶**窗口上（真实输入落到别的窗口）。真置顶 + 真实光标后，`Click-Image` 能关背包、能点页签。
> ⇒ 这批"只能靠鼠标到达"的窗，**以后直接用真实鼠标做两侧同状态 A/B 即可**。

| 目标（鼠标路径） | 差异占比 | 定性 |
|---|---|---|
| `game_shop` 分类页签 `Previous/Next`（`Prguse2[197..199]/[207..209]@(120,103)/(120,421)`） | **未采集** | 未采集（帧拿不到） |
> **2026-09-28 更新**：这条已在 **§3.2y** 定性——驱动补了 `Msg-Drag`（按下-移动-抬起）并做了阴阳两验；
> `game_shop` 那根 `PositionBar` 是**分类列**滑条（C# 守卫 `CStartIndex + 22 >= CategoryList.Count` 就返回），
> 本沙箱只有 10 类 ⇒ **行程为 0**，记「无可滚行程 / 本数据下不可判定」，不是"点不动"。原文保留。

| `game_shop` 分类列 `PositionBar`（`Prguse2[205/206]@(120,117)` 拖动） | **未采集** | 未采集 |
| `npc` 菜单点行 / `npc_goods` 列表滚轮命中区 | **未采集** | 未采集 |
| `inventory` 页签 ITEMS II / QUEST 切换 | **未采集** | 未采集 |
| §3.2c–§3.2k 里因「原版侧压根没出那扇窗」而未比的其余窗 | **未采集** | 未采集 |

> **2026-09-29 结案：上面这张表已逐行收口**（下面每行都给出后续小节，别再按"未采集"读）：
>
> | §3.2l 的行 | 现在的状态 | 依据 |
> |---|---|---|
> | `game_shop` 分类页签 `Previous/Next`（`Prguse2[197..199]/[207..209] @(120,103)/(120,421)`） | **已对表**：两端 chrome 与翻页条位置一致 | §3.2g |
> | `game_shop` 分类列 `PositionBar` 拖动 | 分类索引**真行程 = 0**（C# 守卫 `CStartIndex + 22 >= CategoryList.Count`，真源只有 9 类）；**滑块本身跟手**（真实光标拖 80px ⇒ 滑块 +80px） | §3.2y §3.2ah §3.2ai |
> | `npc` 菜单点行 / `npc_goods` 列表滚轮命中区 | **已采集**：格点扫描 + 黄字带量链接矩形后能点行、点链接开商店；>8 项商品列表真实滚轮可逆（10451/10451/28） | §3.2r §3.2z §3.2aj |
> | `inventory` 页签 ITEMS II / QUEST 切换 | **已采集且已修**：46 格点 ITEMS II 不再换页（issue #3332）；真实 `Click-Image` 点 ITEMS II 弹居中确认框 **86523 px** | §3.2n §3.2o §3.2ak |
> | §3.2c–§3.2k 里因「原版侧压根没出那扇窗」而未比的其余窗 | **几何 + 面板美术都补上**：`probe_ui_nodes -RectKinds all` 对表 **29 窗一致 / 0 不一致**；13 扇从未入表的窗面板美术 **0.004–0.098** 全命中；另 8 个 kind 逐个定判据 | §3.2ao §3.2ap |
>
> **仍未采集的只剩一类**：需要**原版侧真鼠标点 NPC** 才能拿到的**两端同状态**帧
> （`storage / craft / market / npc_awake`）。2026-09-29 复测工作站**仍锁屏**
> （前台窗口 `class=Windows.UI.Core.CoreWindow`、标题「Windows 默认锁屏界面」），
> 按 owner 的边界**没有硬跑**；解锁后一条命令即可（配方与"起点别与 NPC 同格"的坑见 §3.2ao ⑤）。

> **2026-09-29 再更新（这条也作废了）**：上面那句「仍未采集的只剩一类」**已不成立**——
> ① `storage / craft / market / npc_awake` 四扇**已经用鼠标驱动做完两端同状态 A/B**（§3.2ar，
> 捞出的 `npc_awake` 位置缺口已修）；② 「原版真鼠标点不动」这条前提**本身作废**（§3.2ak：真置顶 + 真实光标
> 后 `Click-Image` 能关背包/点页签）；③ 工作站 2026-09-29 已经**解锁**（前台 `Chrome_WidgetWin_1`）。
> ⇒ **§3.2l 这张表没有任何未采集中**；「只能靠鼠标到达」批的入口与清单另见 §3.2bh 开头。

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

> **⚠️ 2026-09-28 再修正（见 §3.2ak）**：本节表里"真实鼠标 `Click-Image` **点不动**"那一行**也要作废**——
> 当轮的窗口是**非置顶**的（`SetWindowPos(...,[IntPtr]::Zero,…)` 是 HWND_TOP，会把 TOPMOST 降级），
> 真实输入落到压在上面的窗口。真置顶后实测：`Click-Image` 点背包关闭钮 → **关掉**；
> 点 `ITEMS II` 页签 → **弹出扩容确认框**。⇒ 两条路径（真实鼠标 / 注入消息）**都能用**。

1. **窗口被拖到 `768,316,1024,768`**（四个区域只有右下角在屏内）——`Click-Image`（真实鼠标）走的是
   `窗口 origin + 图像坐标`，此时**全部落在屏外**；必须先 `SetWindowPos` 摆回 `(0,0,1024,768)`。
2. **C# 子控件 `Location` 是"面板内相对坐标"**：商店面板原点 `(164,146)`，`UpButton@(120,103)`
   的**屏幕**位置是 `(284,249)`——按相对坐标直接点会全部打空（本轮第一遍就是这样，四次点击窗内只差 0.1–0.2%，
   加上面板原点后立刻变成 2.0–2.2%）。背包窗面板原点 `(0,0)`，相对=绝对，所以它"看起来"一直是对的。

> **2026-09-28 更新**：下面第 2 条（背包**页序-身份对照表**）已在 **§3.2n** 收口——
> 两侧逐格命中，并顺带捞出 1 处真分歧（46 格时点 ITEMS II 的行为）。原文保留。

**仍未采集（如实）**

> **2026-09-28 更新**：这条已在 **§3.2q** 部分收口——原版侧**能**打开 NPC 窗了（格点扫描）、
> 窗内点击/悬停/滚轮三件都做了对照；**行/链接点击仍未驱动**（细节与下一轮前置见 §3.2q）。

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
   > **2026-09-28 更新**：已在 **§3.2v** 定性——残差=6 个槽位**热键数字**的字形（本端每格多压约 18 个边框像素，
   > 6 格 ≈110 px，正好等于 216−95 的差），属**字形级残留**，不是布局/画源缺陷。

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

### 3.2q 原版侧「NPC 窗」终于能驱动了：格点扫描找 NPC + 窗内点击/悬停/滚轮三件对照（2026-09-28）

§3.2l-b 挂着的最后一条是「`npc`/`npc_goods` 的菜单点行与滚轮命中区 **未采集**」，卡点是
**原版侧怎么把 NPC 点开**（我方侧 `npc_call` 有 RPC，原版只能在世界里点它）。

**① 打开 NPC：不去求 NPC 坐标，直接做「点击格点扫描」**（配方可复跑）

```powershell
# 前置：沙箱服务端 + 原版客户端已登录，窗口置顶并摆到 (0,0,1024,768)
. tools\acceptance\csharp_golden\csharp_client_driver.ps1 -SandboxRoot $env:TEMP\golden_sandbox
Msg-Key 0x1B                      # Escape：先关掉可能开着的窗
foreach ($y in 160,260,360,460,560) { foreach ($x in 200,300,400,500,600,700,800) {
    Move-Image $x $y; Msg-Click $x $y 120; Move-Image 512 760; Start-Sleep -Milliseconds 350
    Shot-Cs ("sweep_{0}_{1}" -f $x,$y)
} }
# 判据：每帧与 `Data/Prguse.Lib[995]`（NPC 窗美术 440x224 @ (0,0)）比**不透明像素不符率**
#   art_match.py --rect 0 0 440 224 --candidates 995  → 命中帧的不符率会从 ~0.95 掉到 ~0.02
```

实测：35 击里 **(800,360)** 那一击打开了 NPC 窗（`BorderVillage`，一个 Bounty Board），
此后所有帧都保持 0.023——**窗区判据有 40 倍区分度**（0.023 vs 0.95）。
窗的位置/尺寸与 §3.3 的矩形表完全一致（`NPCDialog` = `Prguse[995]`，`Location (0,0)`、`Size [440,224]`）。

**② 窗内点击有效（判别探针）**：点窗内**关闭钮**（C# `NPCDialogs.cs:136-146`，
框内 `(413,3)`、`Prguse2[360]`、图头 24x21 ⇒ 钮心 `(425,13)`）→ 窗区 **97896 px** 变化（窗关了），
再点 (800,360) 又开 ✔ 可反复。⇒「窗内点击整体有效」，后面链接点不动就**不是**注入问题。

**③ 悬停有效（新加的对照手段）**：`MirButton` 悬停会换图（`HoverIndex`），所以"注入的光标移动
有没有被客户端处理"可以**直接读像素**：把光标悬到关闭钮上 → 那一格 24x21 里 **177 px** 变成 hover 图；
移开 → 变回 177 px。这条以后可以当所有"hover 型"交互的通用阳性对照。

**④ 滚轮**：驱动新增 `Msg-Wheel <x> <y> [delta]`（`WM_MOUSEWHEEL`，`wParam` 高位 = delta，
`lParam` 是**屏幕坐标** ⇒ 先 `SetWindowPos` 到 (0,0)）。
**关键前置：调用前必须先 `Move-Image` 把光标移到目标上**——C# 的派发链是
`CMain_MouseWheel` → `MirScene.OnMouseWheel` → **`MouseControl.OnMouseWheel`**
（`CMain.cs:65/312`、`MirScene.cs:136-144`），`MouseControl` 取的是"光标所在控件"。
实测：同一坐标，先移光标则**大地图 NPC 列表滚动**（`BigMapDialog.cs:368-390`，窗区 408 px 变化）；
不移光标则一动不动 ⇒ 这就是滚轮注入的**阳性对照**。

> **2026-09-28 更新（见 §3.2ab）**：这条**已撤回**——同法复跑（键盘 B 开窗、光标压在列表上、±3 格）
> 窗区逐像素 **0**；真因是那一屏的 NPC 列表 `Count == MaximumRows(18)`，`ScrollDown()` 的守卫
> `ScrollOffset >= Count - MaximumRows` 直接短路，**滚轮本来就该不动**。别再用它当"注入滚轮可达"的证据。

**⑤ NPC 窗滚轮：不动 = C# 守卫的正确行为（不是注入失败）**。该 NPC 的 `#SAY` 只有 4 行，
而 C# `NPCDialog_MouseWheel`（`NPCDialogs.cs:235-251`）第三行守卫是
`if (CurrentLines.Count <= MaximumLines) return;`（`MaximumLines = 8`）⇒ 行数不够就不滚。
要验"滚得动"必须找**超过 8 行**的 NPC 页——本轮没找到，如实记为**未采集**（不是"点不动"）。

> **2026-09-28 更新**：这一条已在 **§3.2r** 收口——根因是**我按公式估错了链接位置**；
> 改成"**链接 = 黄字，用颜色扫描量矩形**"后，悬停转红与点击开商店都实测到了。原文保留。

**⑥ 行/链接点击：本轮未驱动（如实）**。该页第 4 行是
`<Newbie Guild Recruitment/@NR1> {Level1~25./KHAKI}`（前半 `R` 型 = 可点 `NewButton`、
后半 `C` 型 = 仅上色不可点），但：

- 在 `y=78/80`（第 3 行）与 `y=95`（第 4 行）各扫了 6 个 x，**悬停无红字、点击窗区 0 变化、
  聊天区 0 变化**（该分支的可见效果是 `AddToGuild` + `LocalMessage`，所以两者都查了）；
- 同一时刻的关闭钮可点、可悬停 ⇒ 排除"注入失效"。

⇒ 结论只到"**链接标签没被命中**"，具体是标签 `Location = TextLabel[i].Location + (measure(前缀)-10, 0)`
（前缀是那 20 个空格）与视觉文本位置不一致，还是该 build 的行内控件没接上，**未定性**。
下一轮的前置很清楚：先写一个小工具，把 `y=34+18i` 每一行的**整条带**做一遍 hover 扫描并打印
"哪一列开始变红"，用红字范围反推链接矩形，再点它。

### 3.2r 原版侧「NPC 菜单点行 / 点链接」与「npc_goods 点格 / 滚轮」都驱动起来了（2026-09-28）

§3.2q 的"链接没被命中"在这一轮**收口**——问题不在注入，在**我按公式估错了链接的位置**。

**关键方法：链接 = 黄字，用颜色扫描直接量它的矩形。**
`NewButton`（`NPCDialogs.cs:506-520`）给链接标签 `ForeColour = Color.Yellow` 且 `MouseEnter → Red`；
同一行里用 `NewColour`（`{文本/颜色}` 语法）渲染的是**另一段**、不可点。于是只要扫窗区里
`R>200 && G>200 && B<80` 的像素、按 y 分带，就能拿到每一段黄字的 `x/y` 矩形：

```python
# 输入：NPC 窗打开时的一帧（窗在 (0,0) 440x224）
im = Image.open(shot).convert("RGB"); px = im.load()
rows = {y: n for y in range(0, 224)
        if (n := sum(1 for x in range(0, 440)
                     if px[x, y][0] > 200 and px[x, y][1] > 200 and px[x, y][2] < 80))}
# 再按 y 相邻（间隔 >2 断开）分带、每带取 x 范围 ⇒ 每段链接的矩形
```

实测商人菜单：`View` x[12..37] y[92..100]、`Ask` x[12..31] y[110..118]、第三段 x[13..43] y[146..154]。
**上一轮连点 6 处不中的原因**：用的是公式估坐标（行号 + `measure(20 个空格)` 前缀宽），
估错了一整行；颜色扫描是直接读像素，不吃这套。

**链接点击（实测）**

| 步骤 | 读数 |
|---|---|
| 悬停 `View` `(25,96)` | 该段黄字**消失**（转红，窗区 68 px 变化）✓ 与 C# `MouseEnter → Red` 一致 |
| 点击 `(25,96)` | NPC 窗内容变 **4774 px**，且**商品窗区 `(0,224,244,334)` 变 80163 px（≈98%）** ⇒ **"View Store" 真的开出了商店** |

**`npc_goods`（商品窗）**

- 窗 = `Prguse[1000]`、`Location (0,224)`、`Size [244,334]`（`NPCDialogs.cs:1071-1073`）；
  与 `art_match.py --rect 0 224 244 334 --candidates 1000` 实测**不符率 0.082** ✓ 与 §3.3 矩形表一致。
- **点格/点行生效**：点第 1 格 `(100,275)` → 商品窗内 **534 px** 变化，diff 包围盒
  `x[9..243] y[257..299]` —— 与 C# `Cells[0] = (10, 34 + 0*33)`（绝对 y 258..291）**逐格对上**
  （选中高亮）⇒ 行点击在原版侧可驱动。
- **滚轮命中区**：C# 只把 `NPCGoodsPanel_MouseWheel` 挂在 **8 个 `Cells[i]`** 上（`NPCDialogs.cs:1101`），
  **不是整个面板**；实测在格上滚 → 商品窗 19/21 px（≈无变化）⇒ 该商人商品数 ≤ 8，
  符合 `StartIndex` 顶/底守卫（`NPCDialogs.cs:1310-1319`）。

**仍未采集**：>8 项的商品列表、>8 行的 NPC 页——两处"真能滚"的阳性场景本机这两只 NPC 都凑不出
（守卫按行数/条数短路）。判据与配方已就绪，换一只有长列表的 NPC 复跑即可。

**顺带两条操作口径**：① 同一坐标在不同回合点到的**可能是不同 NPC**（角色走动/镜头平移会改变画面，
本轮第一次是 `BorderVillage`、第二次是商人）⇒ 扫描命中后先用**美术/黄字**判据确认是哪只 NPC 再继续；
② 一轮里 `View Store` 只**打开**商店，没有买卖 ⇒ 无持久写入。

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

### 3.4b 全屏定位：`win_locate.py`（**不知道窗口画在哪**时用它；矩形给错时 `art_match` 一律判"没画"）

**为什么需要**：`art_match.py` 只在**给定矩形**里比——矩形给错就直接判"没画"。而 2026-09-29 那批
缺口恰恰是"画在别的坐标上"：`npc_awake` 实际在 **(0,224)**（C# `GameScene.cs:307` 逐实例覆盖，
本端画在 (0,0)）、觉醒 NPC 的「分解」开的是投放面板 `Prguse2[351]`@**(264,224)**、
NPC 窗 Quest 按钮在 C# 开的是 `QuestListDialog` `Prguse[950]`@**(487,0)**。三处当时都是现写
`cv2.matchTemplate` 一行行跑出来的——本工具把它固化，并把当时的口径写成判据。

```powershell
# 整个屏幕找某一帧（不需要知道它该在哪）
py -3.12 tools\acceptance\csharp_golden\win_locate.py --shot <帧.png> --lib Data\Prguse.Lib --index 950
# 顺带判"该在不在 (487,0)"：给了 --expect 就同时判位置（容差 --tol-px，默认 2px）
py -3.12 ... --shot <帧.png> --lib Data\Prguse.Lib --index 950 --expect 487,0 --tol-px 3
# 自检（正/负对照）
py -3.12 ... --lib Data\Prguse.Lib --selftest
```

**判据**：`ratio = 不符像素 / 该帧**不透明**像素数`（`alpha < --alpha` 不参与，单像素 RGB 差之和 > `--tol` 计 1，
与 `art_match.py` 同口径）；退出码 0/1。`--selftest` 两条对照：

* **正**：把库里一帧贴到合成帧的**已知坐标 (137,221)** → 必须报到 **(137,221)** 且不符率 < 0.02（实测 0.0000）；
* **负**：同一帧去搜**没贴它的空帧** → 不符率必须明显更差（相对判据 `r2 > 0.25 且 r2 > r1 + 0.2`）。
  ⚠️ 别用固定阈值：**深色美术贴在深色底上"看起来也还行"**（实测 `Prguse[50]` 只有 0.2076、
  挑亮点占比 ≥25% 的帧也只有 0.4685），所以负对照必须与"贴了它那帧"**相对**比。

**实数据复核（同一批帧）**：`Prguse[950]` 在 C# 那一帧的最佳落点 **(485,0)**（期望 (487,0)±3px 命中，
不符率 0.0891）；⚠️ 在**本端日记帧**上它也能报到 0.1134@(192,60)——因为 `Prguse[950]/[961]`
**两窗美术同尺寸且很像**（§3.3 已记），"是哪一扇"要看**位置**与源码索引，不能只看 ratio 接近。

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
dotnet run --project .\dbtool\dbtool.csproj -c Release -- <沙箱>\Server setgold <accountId> <gold> [credit]
```

**`setgold`（2026-09-30 补）**：改 `Server.MirDatabase.AccountInfo` 的 `Gold` / `Credit`
（两个都是 `UInt32` 公开字段；客户端的 `GameScene.Gold/Credit` 就来自这里），只调 `SaveAccounts()`
写 `Server.MirADB`，**不碰 `SaveDB()`**（与 `setpw`/`setpos` 同一条保存路径，见下 §5 的坑）。
用途是**解锁原版侧"要花钱才到得了"的分支**——例如 `MirGameShopCell.BuyProduct()` 里
`Item.GoldPrice * Quantity <= GameScene.Gold` 不过就只发系统聊天、**不弹确认框**（README §3.2bm）。

```powershell
# 停掉沙箱服务端再改（服务端运行时会把它自己的内存状态写回 MirADB）
Get-CimInstance Win32_Process -Filter "Name='Server.exe'" |
  ? { $_.ExecutablePath -eq "$env:TEMP\golden_sandbox\Server\Server.exe" } | % { Stop-Process -Id $_.ProcessId -Force }
dotnet run --project .\dbtool\dbtool.csproj -c Release -- "$env:TEMP\golden_sandbox\Server" setgold 333 1000000
# 回读：export 里 accounts[].gold（本轮实测 0 → 1000000 → 0 三段都逐值一致）
```

实测（2026-09-30，原版沙箱）：`setgold 333 1000000` → `readback: 333 gold=1000000 credit=0`；
`export` 复读 `"accountId":"333" … "gold":1000000`；再用 `setgold 333 0 0` 还原回 0（`Server.MirDB`
哈希前后不变 = `offline-bak`，即 `ProtectGameDb()` 生效）。

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

### 3.2s NPC 窗两端同状态 A/B（2026-09-28）：**美术与行网格一致，正文不可比（两边 NPC 数据不同源）**

§3.2l-b 的"逐窗同状态占比"对 `npc` 一直没做。本轮补上，并把"为什么只能比一部分"钉成数据。

**两端怎么开到同一只 NPC**

- 原版侧：§3.2q 的**点击格点扫描**（35 击），事后按黄字带图案分类，共 4 种：
  ① 无 NPC（20 帧）；② 商人（含 `View`+`Ask`+`Close`，6 帧）；③ 商人（只有 `View`+`Close`，6 帧）；
  ④ 布告板（一条长黄字 `x37..179`，3 帧）。
- 我方侧：`nearby {radius:2000}` 拿 `object_id` → `npc_call {object_id, key:'[@MAIN]'}`。
- **前置（本轮踩到）**：上一轮的实机巡回把测试角色留在了 `D002`，`nearby` 里只有仓库 NPC ⇒
  先用 `@mapmove 0 288 616`（测试账号有 GM）把它送回出生图；`npc_call` **不需要走动**。
- **对齐 NPC 的通用做法（新增）**：我方 DB `npc_infos.file_name` 就是 C# 脚本的相对路径
  （如 `BichonProvince/BorderVillage/BountyBoard-0`），拿它去沙箱 `Envir/NPCs` 逐个对文件即可判定
  "是不是同一只"（本轮 40/40 命中）。**同名/同位置的 NPC，两边可能取不同脚本文件**——
  这直接决定正文能不能比。

**可比项（实测）**

| 项 | 原版 | 我方 |
|---|---|---|
| 窗矩形 | `(0,0,440,224)` | `(0,0,440,224)`（`dialog_rect npc` 回 `rx/ry/rw/rh`） |
| 窗美术 vs `Prguse[995]` | 不符率 **0.026 / 0.030** | 不符率 **0.030** |
| 行网格 | 黄字带落在 `34 + 18i` 网格上（商人：`y92-100`=第 3 行、`y110-118`=第 4 行、`y146-154`=第 6 行） | `npc_rows` 的链接矩形 `y0 = 70 / 88 / 124` = `34 + 18·{2,3,5}` |
| 关闭钮 | 窗内 `(413,3)`、`Prguse2[360]`、图头 24x21 | `dialog_rect` 回 `cx=425, cy=13.5, w=24, h=21`（与 C# 钮心一致） |

**不可比（**数据差异**，不是客户端渲染）**

| 项 | 原版 | 我方 |
|---|---|---|
| 标题 | `Merchant` / `BorderVillage` | `Merchant_Bull` / `BorderVillage_Board`（`npc_infos.name`） |
| 正文 | 7 行（含 `I see you're holding: No Torch` / `No Amulet`） | 6 行（多一个空行、无 holding 行） |
| 脚本文件 | `BichonProvince/BichonWall/Grocery.txt`（holding 行在 `:15-18`） | `BichonProvince/BichonWall/Grocery-0.txt`（没有 holding 行） |

实测两端整窗差 **8.92%**（正文区 8.93%、标题带 9.99%）——**差异全部来自文案与标题名**，
因为窗美术的 `art_match` 不符率只有 0.03（同一张 `Prguse[995]`）。⇒
`npc` 这一行的"同状态占比"只能给出**美术/几何**结论，**文本像素不具可比性**，如实记录。

**两处如实记录的残留**

1. `BorderVillage_Board`（脚本 `BountyBoard-0.txt`，首行 `#IF CHECKPKPOINT > 100`）在我方**出空页**
   （`npc_rows` 回 `visible:false`），而原版能出页 ⇒ `#IF` 求值或数据不同，**未定性**。
2. 原版标题条右侧有 `Crystal` 水印叠在关闭钮上，我方没有（另一条已知差异，不计入本表）。
3. 商品窗两端 A/B **仍未采集**：本轮只拿到原版侧的商品窗；我方点 `View` 要按 `npc_rows` 给的
   `cx=21, cy=78`（我按原版坐标点了 `(25,96)`，打到的是 `Ask`），只验到"链接可点"，商品窗没开。
### 3.2t 商品窗（`npc_goods`）两端对拍：行文案/数量角标按 C# 修好，残差是 locale 与数据（2026-09-28）

§3.2s 留的"商品窗两端 A/B 未采集"在本轮补上，并据此修掉一处**真客户端偏差**。

**开窗路径（两端都可复跑）**

- 原版侧：格点扫描命中 NPC（§3.2q 的检测判据：与 `Prguse[995]` 的采样像素比对）→ 点**最上面那条黄字链接**
  `View`（§3.2r 的颜色扫描给坐标）→ 商品窗。
- 我方侧：`npc_call Merchant_Bull` → `npc_rows` 给 `View@(21,78)` → `click 21 78`。
  `dialog_rect npc_goods` 回 **`(0,224,244,334)`**（与 §3.3 矩形表一致），`npc_goods_probe` 回 **6 件**
  （Candle/Torch/RandomTeleport/DungeonEscape/Amulet/RepairOil）。

**修前实测（两端都开着商品窗、沙箱英文 locale）**：商品窗区两端差 **23.04%**（逐行 1189–2771 px）。
侧视图看出根因：本端把「名称 x数量 价格 金」塞成**一行**、且**永远显示 `x1`**；
而 C# `MirControls/MirGoodsCell.cs` 是**两行**：

| C# 控件 | 位置（cell 相对） | 内容 |
|---|---|---|
| `NameLabel` | `(44,0)` | 物品名（白） |
| `PriceLabel` | `(44,14)` | 本地化 `PriceGold` = **`价格：{0} 金币`**；珍珠档 `PricePearl` = `价格：{0} 颗珍珠{1}`（`:75`，`{1}` = `price>1 ? "s" : ""`，中文串里也留着这个占位符——原版拼接怪癖，照抄） |
| `CountLabel` | `(23,17)` | 数量（黄），**只有 `Count > 1` 才显示**（`:69`） |

**本次修复**（`Client-Bevy/src/game/dialogs/npc_goods.rs`）：名称/价格拆成两条 `Text`（价格串按 C# 本地化逐字复刻）、
数量角标改成 `count > 1` 才给、名称行去掉 `x{count}` 与"金"。两条都取 `&mut Text` 的查询用 **`ParamSet`**
串行访问——加第二个 `Text` 查询会让系统参数正好越过 Bevy 的 16 上限（实测报
`cannot become an ObserverSystem`，是同一个上限的伪装报错）。

**修后验证**

- 单测：`goods_price_text_matches_csharp_localization`（`价格：10 金币` / `价格：1 颗珍珠` /
  `价格：25 颗珍珠s`）、`goods_count_badge_only_above_one`；`cargo test --lib` **859 passed**。
- 实机（结构判据，与 locale 无关）：逐行量"文字带"（`x∈[50,236]` 处亮像素按 y 分带）——
  原版每行两条 `(4..11) + (18..27)`（= 名称在 `+0`、价格在 `+14` 的 C# 布局）；本端修后也是两条
  `(3..10) + (15..27)`（描边让带更满）；**修前只有一条**。

**仍未采集（如实）**

1. **同中文 locale 的两端复帧**：已把沙箱 `Language.ini` 的 `PriceGold/PricePearl` 换成中文条目
   （仓库本地化文件未动），但这一轮格点扫描**连续两次都没再命中 NPC**（角色位置逐轮漂移，
   扫描本身会把人带走）⇒ 修后只做到"我方中文 vs 原版英文"的结构级对比（22.6%），
   **没有**同串像素数。
2. **商品数据差**：原版那只 7 件（多 `TownTeleport`），本端 6 件——同 §3.2s 的脚本变体差
   （`Grocery.txt` vs `Grocery-0.txt`），属服务端数据。

**门禁**：`cargo test --lib` 859、`cargo test --test b0001_smoke --test ui_alignment`（2+53）、
`rustfmt --edition 2021 --check` 0、实机交互巡回 **44/44 exit=0**
（首跑有 2 项偶发 FAIL：`ranking` / `npc_awake` 的关闭钮，复跑全绿——记为已知偶发，与本改动无关）。

### 3.2u 两侧 NPC 脚本来源对账 + NPC 窗滚轮阳性验证（2026-09-28）

**① 脚本来源对账：解释掉 §3.2s 的两处残留**

做法：把本端 DB `npc_infos.file_name` 指向的脚本（形如 `BichonProvince/BorderVillage/BountyBoard-0`）
与「同目录同名的**默认变体**」（去掉 `-<地图号>` 后缀，即 `BountyBoard`）逐字节比。map1 的 43 只 NPC：

| 类别 | 数量 |
|---|---|
| 默认变体与变体文件**内容相同** | 19 |
| 内容**不同** | 14（如 `BountyBoard-0` vs `BountyBoard`、`Grocery-0` vs `Grocery`、`Blacksmith-0` vs `Blacksmith`…） |
| **没有**默认变体（只有变体文件） | 10 |

⇒ §3.2s 的两处残留都能归因：

- `BorderVillage_Board` 在本端**出空页**：本端取 `BountyBoard-0.txt`（首行 `#IF CHECKPKPOINT > 100`），
  原版取 `BountyBoard.txt`（**没有**这条守卫）——所以同一只"布告板"，一边有页一边没页；
- 商品清单少 `TownTeleport`：`Grocery-0.txt` 与 `Grocery.txt` 的 goods 定义不同。

两者都是**服务端数据差异**（两侧 NPC→脚本映射不同），不是本端渲染或引擎问题。

**② NPC 窗滚轮：把"真能滚"的阳性场景拿到了**

- 选页判据：扫 map1 的脚本，找常用页 `#SAY` 行数 > `MaximumLines(8)` 的 NPC——**15/43** 命中；
  取 `GTMerchant_Jamie`（`@main` 10 行）做阳性用例。
- 实机（本端）：`@mapmove 0 344 270` → `npc_call GTMerchant_Jamie [@MAIN]` →
  页面 **9 行**、滚动列表 `{total:9, visible:8, shown:true}`；
  `wheel {x:220,y:90,delta:+3}` → **offset 0 → 1**；`wheel {…,delta:-3}` → **offset 1 → 0** ✔
  ⇒ 与 C# `NPCDialog_MouseWheel`（`CurrentLines.Count > MaximumLines` 才滚、`_index -= count`）一致，
  也补上了 §3.2q/§3.2s 缺的那条阳性（此前那只 NPC 只有 3–4 行，**不滚才是对的**）。
- **两个读数口径**（本轮各踩一次）：① `wheel` RPC 的 `delta` **正 = 向下滚**
  （`control.rs` 注释），所以在 `offset=0` 上发负值本来就该"不动"；
  ② `npc_rows.lines` 回的是**整页**行、不随 offset 变，判"滚了没有"要看 `scroll` 探针的 `offset`。
- 原版侧同款阳性**未采集**：需要在世界里点到那只 NPC，而沙箱角色位置逐轮漂移
  （智能扫描本轮两次未命中）——留作下一批。
  > **2026-09-28 更新**：已在 **§3.2w** 补做——用新工具 `dbtool setpos` 把角色重置到固定格后，
  > 点到 `MaterialDealer` 的 **11 行**页实测：**滚轮不生效**（5 个光标位置 + 先激活都 0 px），
  > 但**箭头可滚**（11926/11925 px）、**大地图滚轮可滚**（382 px）⇒ 原版侧 NPC 窗滚轮本身不发，
  > 与本端（可用）相反；C# 侧根因未定性。

> **2026-09-28 更新（见 §3.2ab → 已被 §3.2ai 撤回）**：本节"NPC 窗滚轮不发"的结论**作废**。
> 它那几条读数用的是**注入** `WM_MOUSEWHEEL`——而 C# 的 `MPoint` 读真实光标（`CMain.cs:176`），
> 注入滚轮在这条派发链上**永远不生效**（夹具限制）。修正夹具后（真置顶 + 真实光标 + `Real-Wheel`）：
> 11 行页 `-120×3` → 正文区 **11463 px**，反向可还原 ⇒ **原版滚轮可用**，与本端一致。

### 3.2v §3.2m 的腰带残差 4.1pp 定性（2026-09-28）：是 6 个槽位**数字**的字形，不是画错

§3.2m 收口时留了一句"剩余约 4.1pp（原版 3.3% vs 本端 7.4%）尚未逐像素定性"。本轮把它定性完：

**① 失配像素的分布**（`belt_art_check` 的 `--hotspot` 同款口径，rect 恒 `(230,618,240,38)`，只比
`Prguse[1932]` 的**不透明**像素）

- 按列统计：失配集中在 `x ≈ 8–11 / 44–46 / 79–81 / 114–116 / 149–151 / 184–186` —— 正是 6 个槽位
  热键数字的 x（C# `BeltDialog.Key[i] = new MirLabel { Location = (8 + i*35, 2), Size = (26,14) }`）；
- 另一处 `x 222–235` 的固定簇，**原版帧同样存在**（44/35 px）⇒ 那是右上旋转钮盖在 1932 上的共同行为。

**② 为什么数字会造成失配**：`Prguse[1932]` 的格内是**透明**（比对时跳过），但格的**金色边框**是
不透明的——数字画得越靠左/越大，压到的边框像素越多。实测第 1 格数字区（`x 6..36, y 1..17`）：

| 帧 | 该区失配 | bbox |
|---|---|---|
| 原版 | **1 px** | `x[11..11] y[8..8]` |
| 本端 | **19 px** | `x[8..11] y[4..12]` |

⇒ 我们每格数字多压到约 18 个边框像素 × 6 格 ≈ **110 px**，正好等于 216−95 的差。

**③ 顺带纠正一条口径**（本轮中途查错过一次）：C# `MirLabel()` **构造里就把 `_outLine = true`、
`_outLineColour = Color.Black`**（`Client/MirControls/MirLabel.cs:175-183`）⇒ C# 标签**默认带黑描边**，
本端 `spawn_label`（描边）是对的；`spawn_label_plain` 只该用在 C# **显式** `OutLine = false` 的地方。
所以这条残差**不是**"我们多描了一层"，而是**字体度量**：同一个 `(8,2)` 位置，本端 CJK 字体渲染的
数字比原版 `Settings.FontName` 8pt 略宽，多压了几列边框像素。

**结论**：与 §3.2p 的"文字字形差"同类——**字形级残留，不是布局/画源缺陷**；判据仍 PASS
（7.4% ≤ 10% 阈值）。若要再压，需要对齐数字的字体/字号（本轮没有证据说哪个字号对，未改）。

### 3.2w 沙箱"重置按钮" `dbtool setpos` + 原版侧 NPC 窗滚轮实测不生效（2026-09-28）

**① 新工具：`dbtool setpos`（沙箱副本专用）**

原版侧那几条取证（点到哪只 NPC、同 locale 复帧）一直被"**沙箱角色位置逐轮漂移**"卡着——
格点扫描本身会把角色带走，下一次就跑不到同一只 NPC 上了。本轮给 `dbtool` 加了重置按钮：

```powershell
# 把账号下（或指定）角色挪到指定落点；只重写 Server.MirADB
tools\acceptance\csharp_golden\dbtool\bin\Release\net8.0-windows\dbtool.exe ^
    %TEMP%\golden_sandbox\Server setpos <accountId> <mapIndex> <x> <y> [charName]
# 回读：dbtool … export out.json  → characters[].mapIndex / loc
```

- 实现：反射设 `Server.MirDatabase.CharacterInfo.CurrentMapIndex(Int32)` / `CurrentLocation(Point)`
  （`dump Server.MirDatabase.CharacterInfo` 得到），然后**只调 `SaveAccounts()`（写 `Server.MirADB`）**——
  **绝不要**顺手调 `SaveDB()`：离线 `LoadDB()` 时 `MapInfoList/ItemInfoList` 是空的，
  `SaveDB()` 会把 `Server.MirDB` 写坏（dbtool 里 `ProtectGameDb()` 就是为此存在的）。
- 用法前提：**先停掉沙箱 Client/Server**（服务端在内存里持有账号，边跑边改会被它的周期存档覆盖）。
- 实测：`setpos 333 1 296 615` → 输出 `character 女道士: map=1 loc={X=275,Y=607} -> map=1 loc={X=296,Y=615}`
  （同账号两个角色一起改）、`saved Server.MirADB (2 character(s) updated)`；`export` 回读
  `mapIndex=1 loc={X=296,Y=615}` ✓；重启客户端登录后，**从同一格 (296,615) 出发点 (800,160)
  稳定开出 MaterialDealer**（下一节）。

**② 原版侧 NPC 窗滚轮：实测**不生效**（但注入与滚动机制都没问题）**

用上面这次确定的起点打开 `MaterialDealer` 的长页（脚本 `BichonProvince/BichonWall/Materials-0.txt`
的 `[@Main-1]` = **11 行** > `MaximumLines(8)`，右侧箭头可见），分别做了三件事：

| 做什么 | 读数 | 说明 |
|---|---|---|
| 光标移到正文 `(220,120)` 后发 `WM_MOUSEWHEEL`（下 3 格，再上 3 格） | 正文区 **0 px** 变化 | 滚轮没动 |
| 换 4 个位置再试：标题 `(220,15)` / 正文 `(220,120)` / 右缘 `(410,120)` / 底边 `(100,205)` | 全部 **0 px** | 与光标位置无关 |
| 先点窗内空白 `(410,205)`，再悬停正文滚 | 仍 **0 px** | "先激活对话框"也不成立 |
| **箭头**：点下箭头 `(425,182)` ×3 → 再点上箭头 `(425,41)` ×3 | 窗区 **11926 / 11925 px** 变化 | 页面**确实可滚**、滚动机制是好的 |
| **阳性对照**（同一次会话）：大地图（B 键）里滚轮 | 窗区 **382 px** | 注入链路是好的 |

⇒ **在沙箱这一版里，NPC 窗的滚轮不生效**，而"注入可用"（大地图）与"页面可滚"（箭头）都成立。
C# 侧代码是挂着的（`NPCDialog` 构造里 `MouseWheel += NPCDialog_MouseWheel`，`NPCDialogs.cs:64`；
行标签上也挂了 `:502`，但那些标签是 `NotControl = true`），派发链是
`CMain_MouseWheel → MirScene.OnMouseWheel → MouseControl.OnMouseWheel`（`MirScene.cs:136-145`）。
**根因未定性**：最可疑的一处是 `MirControl.Highlight()` 的早退
（`:808-823`：`if (ActiveControl != null && ActiveControl != this) return;`）——沙箱里聊天输入框很可能
占着 `ActiveControl`，于是悬停 NPC 窗不会把 `MouseControl` 切过去，滚轮就发不到对话框上；
但本轮"先点窗内空白"并没有改变结果，所以**没结案**，如实记录。

**③ 与本端的对照（顺带，不冲突）**：本端 NPC 窗滚轮**可用**——§3.2u 实测 9 行页
（`wheel +3` → `offset 0→1`、`-3` → `1→0`），那是我方自己的 `UiScrollList` 命中判据（整窗 + 光标探针）。
两边不是"一方对一方错"就能盖棺的：C# 的挂点/派发已如上，本端的命中区与探针口径也已对齐过
（线程 `scroll-hitrect-npc`），本节的结论只是**"沙箱这一版原版不滚"这一事实**。

### 3.2x 「同中文 locale 的商品窗复帧」本轮仍未拿到：起点已可确定，但**点不到商人**（2026-09-28）

§3.2t 把"同中文 locale 的商品窗复帧"记为未采集，原因是沙箱角色位置漂移。本轮有了 `setpos`（§3.2w）
后重试了三轮，**仍记未采集**，过程如实记下（免得下一轮重走）：

1. **沙箱副本的本地化已就位**：`%TEMP%\golden_sandbox\Client\Language.ini` 里
   `PriceGold=价格：{0} 金币`、`PricePearl=价格：{0} 颗珍珠{1}`（仓库本地化文件未动）——
   这一步 §3.2t 已做，本轮确认仍生效（同一客户端进程重启后仍读它）。
2. **`setpos` 把起点固定**：`setpos 333 1 275 607` → 重启客户端登录 → 位置可复现 ✓。
   但同样的起点 + 同样的点击 `(800,360)` 这次**开不出 NPC**；改用 8 点定向点击后命中的是
   **`Assistant`**（帮助 NPC，页面只有一条 `Close` 链接），不是商人。
3. **按坐标算也不行**：把角色放到 `(288,610)`，按 C# 映射
   （`drawX=(dx+10)*48-10`、`drawY=(dy+12)*32`）算出我们 DB 里 `Merchant_Ruben@(291,610)`
   对应屏幕 `(614,384)`／精灵身体 `(638,416)`，在该点周围 **9 点定向点击全部 0–2/17 命中**（没窗）。
4. **能对上的那只是 Assistant**：命中的 `Assistant@(282,606)` 与我方 `Assistant_Jane@(284,606)`
   **位置基本一致**（差 2 格），说明两侧 NPC 位置大体同源；但**商人在哪一格、叫什么名**本轮没定位到
   （C# 侧 `NPCInfo` 名与我们 DB 的 `Merchant_*` 不同名，§3.2s/§3.2u 已定性为数据差异）。

**下一轮的可复跑路径**：用 `setpos` 固定起点 → 逐格（或已知起点附近的小格点网）扫描 →
对每个命中帧判"**页型**"（商人页有 `View/Ask` 两条黄字带、Assistant 只有 `Close` 一条；
注意把扫描 x 限制在 `8..60`、y 限制在 `34..150`，**别把底部 QUEST 按钮的黄字算进来**——
本轮就是被它骗过一次）→ 命中商人后再点 `View` 抓商品窗。

> **2026-09-28 更新（见 §3.2z）**：这条已收口。配方固化成了 `npc_sweep.ps1`（点商人不再靠盲扫，
> 见 §3.2z ②），**但"同中文 locale"这个前提本身不成立**——沙箱那一版原版二进制没有商品价签的本地化键
> （`PriceGold` 在 `Client.dll`/`Shared.dll` 里一次都不出现），只能出英文 ⇒ 该条结论改为「**不可比**」。

### 3.2y 驱动新增 `Msg-Drag`（按下-移动-抬起）+ `game_shop` 分类滑条的定性（2026-09-28）

§3.2l-b 的 `game_shop` 那条是「分类列 `PositionBar`（`Prguse2[205/206]@(120,117)`）**拖动**」——
而驱动此前只有 `Msg-Click`（按下即抬起），**拖不动任何东西**。本轮补上并做了阴阳两验。

**① `Msg-Drag <x> <y> <dx> <dy> [steps] [holdMs]`**（`csharp_client_driver.ps1`）

注入序列：`WM_MOUSEMOVE` → `WM_LBUTTONDOWN(wParam=1)` → **插值**若干步 `WM_MOUSEMOVE` → `WM_LBUTTONUP`。
坐标系与 `Msg-Click` 一致（**客户区坐标**，窗口摆到 (0,0,1024,768) 时等于屏幕坐标）。
适用对象是 C# 里 `Movable = true` 且挂了 `OnMoving` 的控件——拖动的位移由
`MirScene.OnMouseMove → MouseControl.OnMouseMove`（`MouseControl.Moving`）驱动，**不是** `Click` 事件。

**② 阳性（大地图 NPC 列表的 `ScrollBar`）**：`BigMapDialog.cs:125-147` 的 `ScrollBar`
（`Prguse2[205/206]`、`Movable = true`、`OnMoving` 把 y 映射成 `ScrollOffset`），
面板 `Title[820]` 760x500 居中 ⇒ (132,134)，`ScrollBar` 初位面板内 (739,61) ⇒ 屏幕 **(871,195)**：

| 动作 | 读数 |
|---|---|
| 开窗 → 滑条下拖 130px | 大地图区 **11220 px** 变化（列表滚动 + 滑条位移 408 px）✓ |
| 再拖回 130px | 仅 **110 px**（回到原状态，残差是世界动画） |

⇒ `Msg-Drag` 可用、可复现。

**③ 阴性/定性：`game_shop` 的分类滑条在本数据下**没有行程****

- C# 里那根 `PositionBar`（`GameshopDialog.cs:143-155`，`Index 205/206`、`Movable = true`）是
  **分类列**的滑条：`OnMoving` 改的是 `CStartIndex`，而 `DownButton` 的守卫是
  `if (CStartIndex + 22 >= CategoryList.Count) return;`（`:133-141`）——**分类数 ≤ 22 就滚不动**；
- 本沙箱的商城数据（`GameShop_Guard.txt`）只有 **10** 类 ⇒ 行程为 0；
- 实测：`Y` 开商店 → 在屏幕 `(284,267)`（面板 (164,146) + 面板内 (120,117)）拖 60px →
  商城窗内仅 **164 px**（悬停高亮级）变化、**滑条带 (284,240,20,200) 0 px** ⇒ **滑条没动**；
- 结论：这一条记为「**无可滚行程 / 本数据下不可判定**」，**不是**"点不动"。
  （真要有行程，需要一份分类数 > 22 的商城数据；届时用 `Msg-Drag` 复跑即可。）

> **2026-09-28 更新（见 §3.2ag）**：这条**要重跑**——`PositionBar_OnMoving`（`GameshopDialog.cs:596-616`）
> 里 `PositionBar.Location = (x,y)` 在 `if (CategoryList.Count > 22)` **之外**，10 类时滑块本身也该跟手；
> 它当时没动更像"那一次拖没落到滑块上"。重跑前置：**先做悬停阳性**（§3.2ag ②），本轮因锁屏**未采集**。

> **2026-09-28 收口（见 §3.2ai）**：已复跑——**滑块本身跟手**（真实光标拖 80px ⇒ 滑块 +80px），
> 当时"没动"是夹具问题（`Msg-Drag` 注入移动不改 `MPoint`）；**分类列索引确实不动**（9 类 ≤ 22，
> 守卫短路，§3.2ah 有真源数字）。⇒ 本条两半都定论，不再挂"不可判定"。

### 3.2z 「点哪只 NPC」不再是盲扫：`dbtool npcs` + 格心落点 + 页型数值判据（2026-09-28）

§3.2q–§3.2x 的 NPC 取证一直是**临时配方**（格点盲扫 + 肉眼判页型 + 手算链接坐标），每轮重抄一遍，
而且判页型靠眼睛反复被骗。本轮把它固化成一条命令，并把 §3.2x 卡着的那条**结掉——结论是「不可比」**（见 ④）。

**① 新工具三件套**

| 工具 | 干什么 |
|---|---|
| `dbtool … npcs [mapIndex]`（新增模式） | 从 `Server.MirDB` 的 `NPCInfoList` 导出该地图的 NPC 表：`index/fileName/name/mapIndex/x/y/image/rate`，落盘 `<serverDir>\..\npcs_<map>.json` |
| `npc_page_probe.py`（新） | 单帧数值判据：窗美术 vs `Prguse[995]` ⇒ **开没开 NPC 窗**；窗内扫黄字带 ⇒ **页型 + 每条链接的矩形**；顺手判商品窗（`Prguse[1000]` @ (0,224) 244x334） |
| `npc_sweep.ps1`（新） | 一条命令跑完整链：`-SetPos` 重置起点 →（可选 `-RestartPerPoint`）→ 点击 → **逐点**判页型 → 命中商人页**当场**点最上面那条链接 → 抓商品窗 → 汇总 JSON |

```powershell
# NPC 表（先停 Client/Server：LoadDB 会动 DB，dbtool 自带 Server.MirDB 还原保护）
…\dbtool\bin\Release\net8.0-windows\dbtool.exe %TEMP%\golden_sandbox\Server npcs 1
# 一条命令：固定起点 → 按 NPC 表点名 → 判页型 → 点 View → 抓商品窗
pwsh -NoProfile -File tools\acceptance\csharp_golden\npc_sweep.ps1 -SandboxRoot $env:TEMP\golden_sandbox `
     -SetPos -RestartPerPoint -PosMap 1 -PosX 296 -PosY 615 `
     -NpcsJson $env:TEMP\golden_sandbox\npcs_1.json -PlayerX 296 -PlayerY 615 -MaxDist 20 -Only 'Grocery|Drapery|Blacksmith'
```

**② 落点公式：点击是「按格」判的，不是按精灵**

`GameScene.cs:10323` 的反函数：`MouseCell = MouseLocation / (48,32) - (10,11) + User.CurrentLocation`
⇒ 世界格 `(nx,ny)` 的屏幕盒子是 `[(nx-ux+10)*48, (ny-uy+11)*32)`、48x32 大，取**盒心** `(+24,+16)`。
**别拿精灵的绘制原点**（那是 `(dx+10)*48-10 / (dy+12)*32`，差一个精灵锚点）——§3.2x 就是按那个算的。

实测（起点 `setpos 1 296 615`，三个落点各一击命中）：

| 落点 | DB 里那只 | 结果 |
|---|---|---|
| (504,304) | `BorderVillage/Blacksmith` | 商人页 3 条链接（`View` y74..82 / `Repair` y92..100 / `Close` y128..136），点 `View` → **商品窗 0.0863** |
| (120,144) | `BorderVillage/Grocery` | 商人页 2 条链接，点首条 → **商品窗 0.0970** |
| (936,144) | `BorderVillage/Drapery` | 被小地图面板挡住（见 ③）；`-HideMinimap` 后同一击开出商人页 + **商品窗 0.1095** |

商品窗"开/关"的判据带：开 ≈ **0.086 / 0.097 / 0.110**，关 ≈ **0.91–0.96**（≈9 倍区分度；工具阈值默认 0.10，
Drapery 那次 0.1095 略超阈值，看图确认是开着的——阈值偏紧，按 0.15 判更稳）。

**③ 两个把落点打飞的坑（都实测过）**

- **HUD 遮挡**：原版小地图 `MiniMapDialog` `Location = (ScreenWidth-126, 0)` = **(898,0)**、宽 126
  （`MainDialogs.cs:1780`），面板里的 `BigMapButton`（`:1827-1838`，面板内 `(25,131)`）**点一下就开大地图**。
  Drapery 的格心 (936,144) 正落在这块上 ⇒ 点出来的是**大地图**：大地图区判据 **0.591** vs 基线 **0.865**，
  再 Escape 回 0.865；同一坐标**只 hover 不点**时也是 0.865（⇒ 不是悬停触发的）。
  按 `V`（KeyBinds `Minimap`）收起小地图后，**同一坐标开出 Drapery 的商人页**。
  工具默认把这块列进 `-Avoid` 跳过并在日志里点名，要用就用 `-HideMinimap`。
- **点一次角色就被带走**：点 NPC 会让角色朝它挪格，之后按固定起点算的落点全部错位（实测第一击必中、后续全空）。
  ⇒ 加了 `-RestartPerPoint`：每个落点都 `setpos` + 重启客户端 + 键盘登录重来（代价 ~1 分钟/点）。

**④ 页型判据（黄字带），以及 §3.2x 那条的收口**

窗内扫 `R>200 && G>200 && B<80`（C# 链接 `NewButton` 就是 `ForeColour = Color.Yellow`），按 y 分带：

| 页 | 黄字带（实测） | 判据 |
|---|---|---|
| 商人 Blacksmith | `y74..82 x12..37` / `y92..100 x13..48` / `y128..136 x13..43` | 正文区（`y30..170`）里 **≥2 条左对齐短带** |
| 商人 Grocery / Drapery | 各 2 条 | 同上 |
| Assistant | `y92..100 x13..43`，**只有一条** | 单条 |
| 布告板 | 一条长带（`x37..179` 量级） | 带宽 > `--wide`、x0 偏右 |

**底部 QUEST 按钮**的黄字带在 `y202..205 x204..237`（C# `QuestButton.Location = (172, Size.Height-30)`
= (172,194)）⇒ 判据用 `--link-ymax 170` 排掉——§3.2x 就是被它骗过一次。

**§3.2x 收口：「同中文 locale 的商品窗复帧」= 不可比（原版这一版根本没有这个键）**

§3.2x 把"未采集"归因于"点不到商人"（这一半本轮已解决，见 ②）。更要紧的是**「把原版切成中文」这个前提本身不成立**：

- 仓库源码里 C# 读 `.\Localization\<Language>.json`（`Settings.cs:320-326`，`Language` 取自 `Mir2Config.ini [Game]`）；
  **沙箱这一版二进制不是这个版本**——`Client.dll` 里有 `Language.ini` 字面量、**没有** `Localization`。
- 不管走哪条路，**这一版没有商品价签的本地化键**：`Client.dll` 里是硬编码的 `Price: {0} gold` 与
  `Price: {0} pearl{1}`，而 `PriceGold` / `PricePearl` 在 `Client.dll` / `Shared.dll` 的元数据里**一次都不出现**
  （`ExtraSlots8/4`、`GameLanguage` 反倒都在）。沙箱 `Client\Language.ini` 里 §3.2x 加的那两条
  `PriceGold=价格：{0} 金币` / `PricePearl=…` **没有任何效果**——本轮重启客户端后商品窗仍是 `Price: 50 gold`。
- ⇒ 这一条的结论从"未采集"改成「**不可比**」：原版这一版的商品价签只能出英文，**不存在"两端同中文"的一对帧**。
  §3.2t 那次"我方中文 vs 原版英文"的结构级对比（名称在 `+0`、价格在 `+14`）就是这条线能拿到的最强证据。

**⑤ 一条口径**：`npc_page_probe.py` 的窗美术阈值默认 0.10（`--threshold`）。
实测"开"的帧：NPC 窗 **0.020–0.051**、商品窗 **0.086–0.110**；"关"的帧 **0.808–0.958**。

### 3.2aa 小地图残留三件小事定性 + P 标签按职业显示（2026-09-28）

§3.2i 只把小地图的**缩略图**收口了，owner 还挂着三件小事：① 面板块位 `Prguse[2090]/[2091]` 与 S/A/P 标签；
② 标题/坐标文字；③ 昼夜 fade 是否叠缩略图。本轮把三件都定性完，其中一件（P 标签）是**真缺口**，已修。

**① 面板块位：两侧一致，且"小档"解释了 §3.2z ③ 的 HUD 遮挡**

`Prguse[2090]` = **128x154**、`Prguse[2091]` = **128x45**（`libextract.py` 实测；C# `MiniMapDialog`
`Location = (ScreenWidth-126, 0)` = (898,0)）。大档帧上两侧都是 `2090`：

| 帧 | `art_match --rect 898 0 128 154 --candidates 2090,2091` |
|---|---|
| 原版 `orig_baseline_none.png` | [2090] **0.187** / [2091] 0.737 ⇒ 2090 |
| 本端 `ours_kbd_02_ingame.png` | [2090] **0.211** / [2091] 0.798 ⇒ 2090 |

残差（~0.19/0.21）来自画在美术**不透明像素**上的那些控件（标题文字、底排四钮、光点），不是板块位差。
**顺带解释 §3.2z ③**：原版按 `V` 之后 `Toggle()` 走的是 `SetSmallMode()`（`Index=2091`、`_fade=0`）
⇒ 面板从 154 高缩成 **45 高**，`(936,144)` 自然落回世界，不是"点不动"。

**② 标题 / 坐标文字：两侧都有**

同一带（`(900,2,120,18)` 与 `(944,131,56,18)`）的亮像素计数：

| 帧 | 标题带白像素 | 坐标带白像素 |
|---|---|---|
| 原版 `orig_baseline_none.png` | 187 | 104 |
| 本端 `ours_kbd_02_ingame.png` | 167 | 83 |

⇒ 两侧都画了「地图标题（`MinimapName`）」与「坐标（`{x, y}`）」；数值不同（296,615 vs 285,616）
是两台服务器角色位置不同。

**③ S/A/P 标签：真缺口（已修）——C# 用**职业**覆盖 `ModeView`**

C# 两处口径**不是一回事**：

- 构造里三个标签都 `Visible = Settings.ModeView`（`MainDialogs.cs:359/369/379`；`Settings.ModeView` 默认 **false**）；
- 但 `GameScene.UserInformation`（`GameScene.cs:2222-2226`）在登录/进图时**赋值**覆盖 P：
  `MainDialog.PModeLabel.Visible = User.Class == MirClass.Wizard || User.Class == MirClass.Taoist;`

⇒ `ModeView=false`（默认）时原版**只画 P**（且只对法师/道士），本端三个标签都只看 `ModeView` ⇒ 一个都不画。
本端修法（`Client-Bevy/src/game/hud.rs`）：新增纯函数 `pmode_visible(class)`（`Wizard|Taoist` ⇒ Visible），
P 标签 spawn 一律 `Hidden`，由 `attack_mode_text_system` 每帧按 `ActorAppearance.class` 置位；
S/A 保持 `ModeView` 门控。

**实机 A/B（同一 e2e 服务端、同一账号 `goldenchr`／女道士、只换客户端二进制；判据 =
`(895..1023, 186..199)` 里 `Color.Orange (255,165,0)`（容差 45）的像素数）**

| 帧 | 橙色像素 | bbox |
|---|---|---|
| master 构建（修前） | **0** | — |
| 本 PR 构建（修后） | 55（整带 65） | `x899..983, y188..193` |
| 原版（期望） | 270 | `x899..1004, y188..197` |

左边界（x=899）与首行（y=188）**逐值相同**；残差是**文案**（我方中文「宠物:攻击和跟随」 vs
原版英文「[Pet: Attack and Move]」）+ 字形，与 §3.2e/§3.2t 同类。
**阴性对照**：换成战士角色（账号 `test`／`bevychar`）→ **0** 个橙色像素（证明是按职业，不是"总是画"）。

**④ 昼夜 fade 不叠缩略图（两侧一致）**

C# `Libraries.MiniMap.Draw(map.MiniMap, viewRect, drawLocation, White, _fade)`（`MainDialogs.cs:1933`）
的第 5 个实参 `_fade` 是**大/小模式的 alpha**（`Toggle()`：大档 `_fade = 1F`、收成小档 `_fade = 0`），
**不是昼夜**。本端 `minimap_map_image_system` 只写 `ImageNode.image` / `image.rect`、`Node` 尺寸、
`BackgroundColor` 与 `Visibility`，没有任何 tint/alpha ⇒ 两侧都**不给缩略图叠昼夜 fade**。
昼夜相位只出现在 `LightSetting` 图标上（原版那帧是月亮 `2092`=夜、本端同帧是星 `2093`=昼）——
那是两台服务器时间/地图 `LightSetting` 的**数据差**，不是绘制差。

**门禁**：`cargo test --lib` **861 passed**（含新增 `pmode_visible_follows_class_rule`、
`pmode_label_visibility_tracks_class`（阳性对照：同实体换职业要跟着翻））；
`cargo test --test b0001_smoke --test ui_alignment` **53 passed**；
实机交互巡回 `ui_interact_sweep.ps1 -ManageServer` **44/44 exit=0**。
### 3.2ab §3.2w 那条「原版 NPC 窗滚轮不发」的收口：**不是注入问题，是原版这一版滚轮不达**（2026-09-28）

§3.2w 把"沙箱这一版原版 NPC 窗滚轮不生效"记成**根因未定性**。本轮把它收口，并且**顺带撤回 §3.2q
那条"大地图滚轮"阳性对照**——它当时不是滚轮读数。

**① 真实滚轮也不滚（窗口前台已确认）**

判据：MaterialDealer 的 11 行页（`Materials-0.txt [@Main-1]` = 11 行 > `MaximumLines(8)`，
右侧箭头可见、箭头可滚），窗口 `SetWindowPos(0,0,1024,768)` 置顶，光标在正文 `(220,120)`：

| 路径 | 窗区（0,0,440,224）文本区逐像素差 |
|---|---|
| 注入 `WM_MOUSEWHEEL`（`Msg-Wheel 220 120 -3`） | **0** |
| **真实滚轮**（`SetForegroundWindow` + 真实 `SetCursorPos` + `mouse_event(MOUSEEVENTF_WHEEL, -120)`×3） | **0** |
| 真实滚轮反向（+120×3） | **0**（回到原状，全窗逐像素一致） |

关键前置：实测 `GetForegroundWindow() == csHwnd`（**原版客户端本来就是前台窗口**）⇒ 真实滚轮那条
不是"没送到"。⇒ **在沙箱这一版里，滚轮不达 NPC 窗；这与注入无关。**

**② 撤回 §3.2q 的「大地图滚轮」阳性对照**

同法在键盘 B 打开的大地图上，光标压在 NPC 列表上滚（±3 格）：窗区逐像素 **0**。原因不是"滚轮不达"，
而是**这一屏的列表根本没有行程**：`BigMapDialog.MaximumRows = 18`，而 BorderVillage 一带
`ShowOnBigMap` 的 NPC 正好 **18** 条（截图逐行数得 18 行）；`ScrollDown()` 的守卫是
`if (ScrollOffset >= currentRecord.NPCButtons.Count - MaximumRows) return;`（`BigMapDialog.cs:385-390`）
⇒ `0 >= 18-18` 直接短路，**滚轮本来就该一动不动**；而 §3.2y 拖 `ScrollBar` 仍然能改列表，
是因为拖动路径直接写 `ScrollOffset`、**不走**这条守卫。
⇒ §3.2q 记的那 408 px 要么来自另一个行数 > 18 的地图记录，要么把别的变化读成了滚动；
**在拿到"行数 > 18 且滚轮确实改了 `ScrollOffset`"的复现之前，不要再用它当"注入滚轮可达"的证据。**

**③ 源码层：滚轮与点击走的是两个不同的静态量**

| 事件 | 派发入口（`Client/MirControls/MirScene.cs`） | 谁被调用 |
|---|---|---|
| 点击 | `OnMouseClick:162` —— `if (ActiveControl != null && ActiveControl.IsMouseOver(MPoint) && ActiveControl != this)` | **`ActiveControl`** |
| 滚轮 | `OnMouseWheel:141` —— `if (MouseControl != null && MouseControl != this)` | **`MouseControl`** |

`MouseControl` 全仓**只有一处赋值**：`MirControl.Highlight()`（`MirControl.cs:808-823`）——
它先 `MouseControl.Dehighlight()`（把旧值置 null），然后
`if (ActiveControl != null && ActiveControl != this) return;` **提前返回**（`MouseControl` 停在 null）。
而 `Highlight()` 只在"光标**不在任何子控件**上"时才被这条链调到（`MirControl.cs:921-929`：
先递归到最深的 `IsMouseOver` 子控件，找不到才 `Highlight()` 自己）。
⇒ 页面**箭头是点击语义**（走 `ActiveControl`），所以箭头照样能滚；滚轮走 `MouseControl`，
它一旦被 `ActiveControl` 早退掐掉就没有落点。
**本轮没能把这一环单独钉死**（没能做出"让 `MouseControl` 变成 NPC 窗"的正例——正文区悬停、
先点窗内空白都试过，注入与真实滚轮都不达），所以只记为**最可疑的一环**，不写成定论。

**④ 与本端的对照（刻意背离，不要去"对齐"）**

本端 NPC 窗滚轮**可用**（§3.2u：9 行页 `wheel +3` → `offset 0→1`、`-3` → `1→0`），是**严格超集**；
原版这一版滚轮不达属客户端缺陷。⇒ 结论：**本端保留滚轮**，不把"原版不滚"当基准；
这条差异记「刻意背离」，与 §3.2g 的滚动条实现同类。

> **⚠️ 2026-09-28 撤回（见 §3.2ai）**：本条**整段作废**——那两条"真实滚轮/注入滚轮都 0 px"的读数
> 是在**降级窗口（`[IntPtr]::Zero`）**的夹具下取的，客户端当时被别的窗口压着、真实光标的
> `WM_MOUSEMOVE` 到不了它（`MPoint` 读到的是别处）。修正夹具后**原版 NPC 窗真实滚轮可用**
> （11 行页 `-120×3` → 正文区 **11463 px**，反向可还原）。**不再有"刻意背离"这一说**。

### 3.2ac §3.2f 那条「居中窗 ±1px」待办收口：`--max-shift` 复跑后的真实残差（2026-09-28）

§3.2f 的待办是「给逐窗对拍加『居中窗允许 ±1px 平移』（或把原版取帧改成真实客户区），**之后再看这批窗的真实差异**」。
`golden_ab_diff.py` 现在**默认就带** `--max-shift 1`（默认 1，逐窗报 `平移后=<像素>(<占比>, dx=.. dy=..)`），
本轮用同一批帧（原版 `orig_win_*.png` + 本端 `ours_win_*.png` + `rect_table.json`）复跑，把"之后"这半句做完：

```powershell
py -3.12 tools\acceptance\csharp_golden\golden_ab_diff.py --shots %TEMP%\golden_sandbox\shots --table %TEMP%\rect_table.json --max-shift 1
```

| 窗口 | 原始差异 | ±1px 平移后 | 平移量 | 含义 |
|---|---|---|---|---|
| **Friends** | 25.5%（18331） | **6.6%（4719）** | dx=1 dy=0 | 与 §3.2f 记的 4783 同量级 ⇒ 主导项确实是那 1px |
| **Help** | 87.6%（239050） | **10.7%（29293）** | dx=1 dy=0 | 同上；剩下的是**键位表语言/条目**（§3.2c 已定性） |
| 其余（Inventory/Equipment/Skills/Quests/Options/Group/Relationship/Ranking/GameShop/Bigmap/Keybind） | — | **无位移改善**（dx=dy=0） | — | 它们的差异不是"居中偏 1px"，是真内容/数据差（各自小节已定性） |

⇒ **§3.2f 的待办只剩"口径已加、结论已出"**：居中窗（`Center`）在这对帧里确实整体偏 1px（原版取帧的客户区比 1024 宽 2px），
**不是本端排版 bug**；其余窗的差异与位移无关。

**顺带一条新线索（未收口，留给下一批）**：Friends 平移后的残余 **4870 px 里 3546 px 压在翻页条那一行**
（按 20 行分带：屏幕 `y488..507` = 2488 + `y508..527` = 1058，其余各带只有 55–198 px 的 1px 级残差，
标题带 484 px）。即"翻页条"这块在两边**仍有可见差**（本端是 PR #3320 补的那套：
`PageNumberLabel (87,216) 83x17` + `Prguse2[240..242]@(70,218)` + `[243..245]@(171,218)`，
空好友列表时该显示 `1/1` 且两个箭头为禁用帧）。下一轮的前置很清楚：
**把 `Prguse2[240..245]` 三帧逐张导出来，与本端翻页条那一行逐像素比，判"帧号错"还是"占位/文字错"**。

### 3.2ad 「写死尺寸 ≠ 美术原生尺寸」的第 4 个扫描面：`control_size_audit.py` 补盲点 + 捞出的两处真缺陷（2026-09-28）

§3.2j 末尾记了 `control_size_audit.py` 的**第 4 个盲点**：「帧号来自循环变量」的表驱动形态
（表里只有帧号、**尺寸写在 `spawn` 处当字面量**）此前三个扫描面都看不见。本轮把它补上，
并用它捞出**两处真缺陷**。

**① 补的扫描面（`scan_loop_literal_size`）**

现场（`Client-Bevy/src/game/dialogs/friend.rs` 的 5 颗操作钮）：

```rust
let acts: [(bool, …, usize, f32, &str); 5] = [ (…, 554, 60.0, "添加"), … ];
for (…, idx, x, hint) in acts {
    load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx)   // lib 写死、idx 是循环变量
    spawn_icon_button(p, n, h, pr, x, 241.0, 24.0, 22.0, 10)          // 尺寸是字面量
}
```

三个旧扫描面都看不见它：`TABLE_DECL` 只认 `const … = &[`（这里是 **`let`** 声明，新增 `TABLE_DECL_LET`）、
`DYN_LOAD` 只认 `*lib, *idx` 的**解引用**写法（这里是 `LibraryName::Prguse, idx`）。
新面按「`spawn_*` 的两个尺寸实参是字面量 ⇒ 必须等于**每一行帧号**的美术原生尺寸」逐行比，
帧号从循环体里那次 `load` 的**第 4 个实参**（必须是循环变量）反推它在表行里的列号。

**正/负对照（实测）**

| 场 | 结果 |
|---|---|
| 旧代码副本（把 `bw,bh` 换回 `24.0, 22.0`） | **5 行命中**：`Prguse[554/557/560/563/566]` 美术 **28x25**、写死 **24x22** ⇒ `VERDICT=FAIL`（exit 1） |
| 修后本仓 | `合计 0 处`、`VERDICT=PASS` |

**② 捞出的两处真缺陷（同一类：`MirButton` 不写 `Size` ⇒ 取图头，本端写死了别的尺寸）**

| 位置 | 美术原生 | 修前写死 | 后果 |
|---|---|---|---|
| `friend.rs` 5 颗操作钮（`Prguse[554/557/560/563/566]`） | **28x25** | 24x22 | Bevy 把 28x25 **重采样**成 24x22 ⇒ 整排按钮 3~7 级色差 |
| `help.rs` 3 颗钮里的**关闭钮**（`Prguse2[360..362]`） | **24x21** | 16x16（三颗同值） | 关闭钮被压扁；同表里的翻页 `Prguse2[240..245]` 本来就该是 16x16 |

修法：两处都改成**按图头取尺寸**（`libs.0.get_image(lib, idx)` → `w/h`），不再写死——
与 §3.2j 背包页签的「裁剪 ≠ 缩放」同一口径。

**③ 实机 A/B 复验（同一批原版帧 + 修后重新生成的 `ours_win_*.png`）**

单钮判据（`art_match` 的比对口径，只比美术不透明像素 + ±3px 穷举）：

| 目标 | 修前（本端） | 修后（本端） | 原版 |
|---|---|---|---|
| Friends `Add` 钮 vs `Prguse[554]` | 0.583（379/650） | **0.000（0/650）** | 0.000（dx=1） |
| Help 关闭钮 vs `Prguse2[360]` | （写死 16x16，未单独量） | **0.000（0/373）** | 0.000（dx=1） |

整窗（`golden_ab_diff.py --max-shift 1`）：

| 窗口 | 修前 | 修后 |
|---|---|---|
| **Friends** | 25.5%（平移后 **6.6% / 4719 px**） | 24.4%（平移后 **2.3% / 1678 px**） |
| Help | 87.6%（平移后 10.7%） | 87.6%（平移后 **10.7%**，未动——该项被键位表语言/条目主导，符合预期） |
| Inventory | 2.9% | 3.0%（噪声级） |

⇒ Friends 平移后的残余**掉 64%**（4719 → 1678）；剩下的 1678 px 是标题/页码的**字形差**（§3.2ac 已定位到那一行）。

**顺带把"页码那一行"的字形差钉成数据**（省得下一轮再猜）：文串**两侧一致**（C#
`PageNumberLabel.Text = (Page + 1) + " / " + maxPage`、本端 `format!("{} / {}")`，空列表都是 `1 / 1`），
差的是**字号/字体度量**——C# 那行不设 `Font` ⇒ 取 `MirLabel` 默认 `ScaleFont(new Font(Settings.FontName, 8F))`
（`Client/MirControls/MirLabel.cs:180`），本端是 **12px** CJK 字体；实测文字包围盒
**原版 18x8、本端 27x10**（左/上各差 2~3px）。属 §3.2v「字形级残留」同类——
与"布局/画源"无关，改它要动字体选型，本轮不改。

**门禁**：`cargo test --lib` **861 passed**；`cargo test --test b0001_smoke --test ui_alignment` **53 passed**；
实机交互巡回 `ui_interact_sweep.ps1 -ManageServer` **44/44 exit=0**（含 `help` 关闭钮那条）。

**④ 顺带修好工具自己的「自证」（本轮发现它已经恒失败）**

`control_size_audit.py --selftest` 的**正对照**此前是拿 `group.rs` 的**整行字面量**当锚点
（`let _ = spawn_image_native(p, &mut libs, &mut images, LibraryName::Title, 5, 18.0, 8.0, 9);`）。
`rustfmt` 把这次调用折成多行之后，锚点失配 ⇒ 自证恒报「找不到要改坏的锚点」、**exit 1**，
等于这条门禁失去了"证明自己会红"的能力。改成**空白宽松的正则**匹配后：

```
[负对照] 原样扫描的新增命中 0（期望 0）
[正对照] 改坏一处后命中 1 条：group.rs:245 Title[5] 美术=(55,15) 写死=(57.0,15.0)
[正对照②] 表驱动循环写死 38x19 后命中 13 条：menu.rs:283 Title[633/636]=(32,20)、Prguse[1970]=(32,18) …
VERDICT=PASS（正/负对照）    exit=0
```
### 3.2ae Inventory 窗两个文本口径：`WeightLabel` 是**空格数**、`GoldLabel` 要千分位（2026-09-28）

§3.2j / walgit 线程 `crystal-ab-longtail-windows` 里挂着一句「Inventory 残余 2.9% 里有一小块在
删除钮 `(291,212)` 附近**尚未定性**（原版那格与 `Prguse2[366]` 对不上）」。本轮把它定性——
**不是删除钮的问题**。

**① 定性：那一格是 `WeightLabel`（(268,212)），两侧文案口径不同**

| 帧 | `(268,212)` 那一格 | 右边 `(291,212)` |
|---|---|---|
| 原版 | **`45`**（纯整数） | 删除钮 `Prguse2[366]` 16x15（位置/尺寸本来就对） |
| 本端（修前） | **`0/0`** | 同上 |

C# 原文（`Client/MirScenes/Dialogs/InventoryDialog.cs:386`）：

```csharp
WeightLabel.Text = GameScene.User.Inventory.Count(t => t == null).ToString();   // ← 空格数
//WeightLabel.Text = (MapObject.User.MaxBagWeight - MapObject.User.CurrentBagWeight).ToString();  // 旧口径，已注释掉
```

⇒ **背包窗这个标签显示「空格数」**（原版那只 1 级女道士：46 格 − 1 件起始装备 = **45**）。
"剩余负重 / 空格数"那一对是在**主 HUD** 上（`MainDialogs.cs:464-465` 的 `SpaceLabel`/`WeightLabel`），
本端 `hud_space_weight_system` 早就是那个口径；出错的是**背包窗**这颗——本端写成
`"{weight}/{max_weight}"`（且本端 `Inventory.weight` 只由 `refresh_weight` 更新，A/B 那帧就是 `0/0`）。

**② 同窗第二处：`GoldLabel` 缺千分位**

`InventoryDialog.cs:388`：`GoldLabel.Text = GameScene.Gold.ToString("###,###,##0");`
本端是 `format!("{}", gold)` ⇒ 金币 ≥ 1000 时与原版对不上。改成复用 HUD 的 `format_gold`（同口径）。

**③ 修法**

- 新增纯函数 `inv_weight_text(Option<&Inventory>) -> String`（空格数）——门禁两条：
  「46 格 − 1 件 = 45」「改 `weight` 不影响文案」；
- `GoldLabel` 复用 `crate::game::hud::format_gold`（断言 `0 / 45 / 1,000 / 1,234,567`）。

**④ 实机 A/B 复验**

| 项 | 修前 | 修后 |
|---|---|---|
| 背包窗 `WeightLabel` | `0/0` | **`46`**（空格数；原版同格 `45`——差 1 是**角色背包内容**：本端 `goldenchr` 空背包、原版 1 件起始装备） |
| `golden_ab_diff.py` Inventory 整窗 | 3.0% | **2.9%** |
| 金币千分位 | 无 | `###,###,##0`（本轮 A/B 角色金币 0，**看不出**，靠单测钉住） |

⇒ 那条"删除钮附近未定性"的残留**结案**：删除钮位置/尺寸本来就对
（`Prguse2[366]` 16x15 @(291,212)，见 §3.2j 的尺寸审计），差异来自**左边那颗空格数标签**。

**门禁**：`cargo test --lib` **863 passed**；`cargo test --test b0001_smoke --test ui_alignment` **2 + 53**；
实机交互巡回 `ui_interact_sweep.ps1 -ManageServer` **44/44 exit=0**。

### 3.2af 逐窗 A/B 表**逐行结论索引**（2026-09-28，master `83e5ec6b0` 复跑）

这一节不引入新判据，只是把 §3.2c–§3.2ae 一路攒下来的**20 行结论**收成一张索引表，
方便下一轮"知道哪一行还欠什么"。复跑配方（原版侧沿用 §3.2b 的金标准帧）：

```powershell
pwsh tools\acceptance\csharp_golden\golden_ab_ours.ps1 -SandboxRoot $env:TEMP\golden_sandbox `
     -ClientHome <检出> -User goldenchr -Password 123456      # 我方 20 窗 + ab_windows.json
py -3.12 tools\acceptance\csharp_golden\golden_ab_diff.py `
     --shots %TEMP%\golden_sandbox\shots --table %TEMP%\rect_table.json --max-shift 1
```

| 行 | 本次占比 | ±1px 后 | 结论（类型） | 依据 |
|---|---|---|---|---|
| **Group** | **0.0%** | — | 三处真缺口修完**归零** | §3.2j |
| Inventory | 2.9% | — | 已修 5 处（页签裁剪/ITEMS II 换帧/…）+ 2 处文本口径（空格数/千分位）；残余＝物品内容与透明边 | §3.2j §3.2ae |
| Skills | 1.7% | — | 与 `character` 同窗，可比（§3.2k 定的口径） | §3.2k |
| Equipment | 6.3% | — | **数据**：纸娃娃/装备内容 + 面板底部透出世界 | §3.2j |
| Options | 6.2% | — | **数据**：`Settings` 值（音量 100 vs 0） | §3.2c §3.2j |
| Quests | 6.3% | — | 删掉自造「放弃」钮后，残余＝任务列表文本内容 | §3.2j |
| **Friends** | 24.4% | **2.3%（dx=1）** | 翻页条补全（#3320）+ 5 钮原生尺寸（§3.2ad）后残余＝标题/页码**字形** | §3.2f §3.2ac §3.2ad |
| Relationship | 13.1% | — | 标题/按钮原生尺寸/垂直居中三处修完；残余＝四行**文案语言** | §3.2g |
| **Ranking** | 10.6% | — | **数据**：原版沙箱那份排行榜是空的（本端 20 行） | §3.2g |
| **Guilds** | 66.2% | — | **不可比**（C# `Show()` 守卫：不在公会只弹提示）——判据是「提示框区 1.2% + 按钮逐像素一致」 | §3.2e |
| **Creature / MountWindow / Fishing** | 50.1% / 94.5% / 34.4% | — | 同上（守卫窗）；**别用窗口矩形判** | §3.2e |
| Help | 87.6% | **10.7%（dx=1）** | **语言/条目**（我方中文键位表 vs 原版英文固定清单）+ 关闭钮原生尺寸（§3.2ad） | §3.2c §3.2ad |
| Keybind | 26.6% | — | 同上（语言/条目） | §3.2c |
| **GameShop** | 29.9% | — | chrome 与翻页条位置一致；差＝商品/分类**数据不同源**；分类滑条「无可滚行程」 | §3.2g §3.2y |
| **Bigmap** | 58.6% | — | 视口**画源已对齐**（§3.2h 自证 0.999）；整窗差来自**地图数据不同源**（视口图 + NPC 列表都是各自 DB 的） | §3.2g §3.2h |
| **Minimap** | 91.3% | — | **不可比**：那一帧我方没出窗（本端 `V` 是开关、原版 `V` 是切大/小档）——板块位用 `art_match` 单独比过（2090 0.211 vs 原版 0.187） | §3.2i §3.2aa |
| **Belt / Skillbar** | 91.7% / 91.7% | — | **参考（整帧）**：HUD 行不能用窗口矩形比（露的是世界）；改用「只比美术不透明像素」的判据 — 腰带 7.4%、技能栏位置/档位差 | §3.2k §3.2m §3.2v |

**两类"不是缺陷"的残差（每条都已定性，别再当缺口修）**

1. **字形**：两侧 `Settings.FontName`/字号不同 ⇒ 文本包围盒差 1~2 成（Help/Relationship/Friends 页码、
   腰带数字、商品价格串…）。见 §3.2p §3.2v §3.2ad 的实测。
2. **数据不同源**：两台服务端的**地图数据/DB/角色状态**都不同（§3.2d ② 给了三个地图文件的哈希），
   凡是"露背景/露列表/露排行榜"的行，占比都不可比——只有窗口 chrome 与自绘控件可比。

**唯一还欠凭证的一条**：`game_shop` 分类滑条的**真行程**（C# 守卫 `CStartIndex + 22 >= CategoryList.Count`
就短路，沙箱那份数据只有 10 类）——要拿就必须给沙箱造一份分类数 > 22 的商城数据，属**数据准备**问题，不是判据缺失（§3.2y）。

> **2026-09-28 收口（见 §3.2ah / §3.2ai）**：这一条**结了**——离线量到真源分类数 = **9**（+`Show All` = 10 行，
> 见 §3.2ah），守卫短路⇒**分类列索引本来就不该动**；而**滑块本身**用真实光标一拖就**跟手**（§3.2ai ③）。
> §3.2y 当时记的"滑条没动"是夹具问题。若日后要给沙箱造 >22 类数据，也只需改 `Server.MirDB` 的
> `GameShopList[].Category`（注意 §3.2ah 的 `Edit.ItemInfoList` 坑）。

### 3.2ag 鼠标路径试验的**前置阳性对照**：先证明 `MouseControl` 有落点（2026-09-28，本轮又撞上锁屏）

本轮想复核 §3.2y 那条「`game_shop` 分类滑条没动」，结果连**已证过的阳性**（大地图 `ScrollBar` 拖动）
都复现不出来——最后查明是**工作站又锁屏了**。顺手把这条前置固化成判据，免得下次再把「锁屏」读成「原版不达」。

**① 锁屏判据（比 §3 的 `CopyFromScreen` 更硬的一条）**

```powershell
$fg = [CsUi]::GetForegroundWindow()      # 或 user32 GetForegroundWindow
# 锁屏时 = Windows.UI.Core.CoreWindow，标题「Windows 默认锁屏界面」，覆盖整屏
```

本轮实测：`cs=0xD99094A fg=0x2102CA same=False`，前台窗口 class=`Windows.UI.Core.CoreWindow`、
title=`Windows 默认锁屏界面`、rect=`(0,0)-(2560,1440)` ⇒ **锁屏**。此时 `SetForegroundWindow(client)` 也无效
（Windows 前台锁：后台进程抢不到焦点）。

**② 锁屏时鼠标三条路径**全死（真光标落在锁屏上）：

| 试验 | 结果 |
|---|---|
| 悬停商城 `UpButton`（`Prguse2[197]→[198]`） | 12x12 里 **0 px** 变化（`MouseControl` 没落点） |
| 拖大地图 `ScrollBar`（§3.2y 证过 11220 px 的那一下） | 窗区 **6 px**（世界动画量级）= 没拖到 |
| 注入 `WM_MOUSEWHEEL` / `Msg-Drag`（都是 `SendMessage` 到 hwnd） | 同样 0（派发链要 `MouseControl != null` 才把事件交给控件，而它恒 null） |

⇒ 所以**判据是「悬停换帧」**：动手做任何 hover / 拖动 / 滚轮试验**之前**，先悬停一颗有 `HoverIndex` 的钮
（商城上下页箭头、关闭钮…），**该钮的像素必须变**——不变就说明 `MouseControl` 没落点，
后面所有"不动"都不算证据（§3.2q ③ 那条 177 px 的悬停对照就是本条）。

**③ 对既有结论的影响（如实划界）**

- §3.2ab（原版 NPC 窗滚轮不达）**不受影响**：那轮是**解锁**状态（`GetForegroundWindow()==csHwnd` 实测为真），
  且同会话里**页面箭头点击可用**（点击走 `ActiveControl`，要 MouseControl 有落点才点得动）⇒ 当时派发链是活的。
- §3.2y 的「`game_shop` 分类滑条拖动没动」**要重跑**：`PositionBar_OnMoving`（`GameshopDialog.cs:596-616`）
  里 `PositionBar.Location = (x,y)` 在 `if (CategoryList.Count > 22)` **之外** ⇒ 就算只有 10 类，
  滑块本身也该跟手；它没动更像"那一次拖没落到滑块上"（同会话先前那次大地图拖动可能把 `ActiveControl` 占住了）。
  **重跑前置**：解锁 → 悬停阳性（②）→ 全新会话 `Y` 开商城 → `Msg-Drag 284 267 0 60` → 看滑条带变不变。
  本轮**未采集**（锁屏）。

  > **2026-09-28 收口（见 §3.2ai）**：解锁后复跑——**滑块跟手**（真实光标 `Drag-Image` 拖 80px ⇒ +80px）。
  > 关键不是"那次没落到滑块上"，而是 **`Msg-Drag` 本身拖不动任何东西**：C# 的 `MPoint` 读**真实光标**
  > （`CMain.cs:176`），注入的 `WM_MOUSEMOVE` 改不了它 ⇒ 要拖就用 `Drag-Image`（真实光标）＋真置顶。

**④ 沙箱数据也可能被清空**：本轮还发现沙箱实拷贝文件丢失（`Server\Envir` 全空、`Client` 顶层 32 个文件只剩 3 个，
`Client.exe` 都没了），按原版 `robocopy` 补回后**注意 `make_sandbox.ps1 -Force` 必须重跑**——否则
`Client\Mir2Config.ini` 会被原版那份覆盖成 `Port=7000`（打到共享 Rust 开发服）。口径与命令见
`~/.agents/rules/LESSON_沙箱Envir可能被清空_取证前先核NPC脚本数并按原版恢复.md`。

### 3.2ah 商城分类数**离线量出来**：`dbtool gameshop` ＋ 一个必须知道的绑定坑（2026-09-28）

§3.2y 把「本沙箱商城只有 10 类」记在 **`Drops/GameShop_Guard.txt`** 上——那是**一份 drop 文件**
（10 个以分类命名的段：Pots/Weapons/Armour/...），不是商城数据的真源。商城真源是
`Server.MirDB` 的 `GameShopList[].Category`（服务端 `PlayerObject.GetGameShop()` 逐条发
`S.GameShopInfo`，`Server/MirObjects/PlayerObject.cs:13913-13944`）。本轮把它离线量出来：

```powershell
tools\...\dbtool.exe %TEMP%\golden_sandbox\Server gameshop      # 落 <serverDir>\..\gameshop.json
```

**坑（不修就永远量成 0）**：`BindGameShop(item)` 是拿 **`Envir.Edit.ItemInfoList`**（编辑器那份）
去绑 ItemInfo 的（`Server/MirEnvir/Envir.cs:4550-4561`），而离线 `LoadDB()` 里那份**是空的**
⇒ 每个商品都 `return false` 被丢掉，`GameShopList` 恒 **0**——`dbtool export` 的
`dbCounts.GameShopList = 0` 就是这种**假零**（我第一版 `gameshop` 也踩了同一个坑）。
修法：把 `ItemInfoList` 灌进 `Edit.ItemInfoList` 再跑一遍 `LoadDB()`，这一遍才绑得上
（`gameshop` 与 `export` **都已内置**这遍；修后 `dbCounts.GameShopList` 从 `0` 变成 **105**）。

**实测（原版沙箱 DB，105 件 / 9 类）**

| 分类 | 件数 |
|---|---|
| Potion | 28 |
| Transform | 28 |
| Special | 11 |
| Scroll | 9 |
| Mount | 8 |
| Creature | 6 |
| Torch | 6 |
| Fishing | 5 |
| Package | 4 |

客户端分类列 = 9 类 + `Show All` = **10 行** ⇒ §3.2y 的"10 类"**数字对得上、来源写错了**。

**对 §3.2y 结论的影响（两半分开看）**

- **分类列的滚动/滑条行程**：`CategoryList.Count(9) > 22` = **false** ⇒ 在本数据下
  **确实不可达**（`GameshopDialog.cs:135/583` 两处守卫短路）——这一半结论成立，而且现在有**真源数字**背书。
- **滑块本身跟不跟手**：`PositionBar_OnMoving` 里 `PositionBar.Location = (x,y)` 在守卫**之外**
  ⇒ 与分类数无关，属 §3.2ag ③ 要重跑的那条（**等解锁**）。

**顺带一条口径**：凡是走 `Edit.*` 绑定的列表，离线 `dbCounts` 都可能是**假零**——`GameShopList`
已实证并修好（0 → 105）；同一遍修完仍是 0 的还有 `RecipeInfoList` / `GuildList` / `HeroList` /
`StartItems` / `GTMapList`（**未逐一核它们是真空还是另一条绑定路径**，要量之前先确认它读的是
`Envir.*List` 还是 `Edit.*List`）。

### 3.2ai 夹具重大修正：鼠标交互必须用**真实光标**＋**真置顶**；据此撤回 §3.2ab、修正 §3.2y（2026-09-28）

工作站解锁后按 §3.2ag 的协议复跑，发现**前面两条结论**（§3.2ab 的"原版滚轮不达"、§3.2y 的"滑条没动"）
**都是我自己的夹具造成的假象**。根因两条，都在夹具侧：

**① `MPoint` 读的是真实光标，不是事件坐标**

```csharp
// Client/Forms/CMain.cs:176
MPoint = Program.Form.PointToClient(Cursor.Position);
```

⇒ 注入 `WM_MOUSEMOVE`**不会**改变 `MPoint`（C# 忽略消息里的坐标，回去读真实光标）。
而悬停高亮 / 拖动（`MouseControl.Moving` 的位移计算）/ 滚轮路由（`MirScene.OnMouseWheel` → `MouseControl`）
**全都依赖 `MPoint`** ⇒ **这些交互只能用真实光标驱动**（`SetCursorPos` + `mouse_event`）。
`Msg-Click` 的 down/up 还能触发 Click 语义（§3.2l-b 那条仍成立），但 `Msg-Drag` 的"拖动"实际上只等于
按下+抬起（`MPoint` 没动 ⇒ 控件不会位移）。

**② `SetWindowPos(..., [IntPtr]::Zero, …)` 是 HWND_TOP，会把置顶降下来**

`Init-CsClient` 用的是 `[IntPtr](-1)` = **HWND_TOPMOST** ✔；驱动里若再写一次
`SetWindowPos(h, [IntPtr]::Zero, 0,0,1024,768, 0x40)`（insert-after=0 = **HWND_TOP**），
就会把客户端**降到非置顶** ⇒ 别的窗口（终端）压在上面时，真实光标的 `WM_MOUSEMOVE` 到不了客户端
（`WindowFromPoint(x,y)` 实测返回终端的 hwnd 而不是客户端）。**要置顶就写 `-1`**。

**③ 修正夹具后的实测（同一沙箱、同一会话）**

| 试验 | 夹具（旧） | 夹具（修正后） |
|---|---|---|
| 悬停商城 `UpButton`（`Prguse2[197]→[198]`） | 0 px | **126 px** ✔ |
| 真实光标拖商城分类列 `PositionBar` 80px | （`Msg-Drag`）0 px | **滑块 +80px**（滑条带 bbox `(1,23)-(13,121)`）✔ |
| 真实光标拖大地图 `ScrollBar` 130px | （`Msg-Drag` 本轮 6 px） | **窗区 14396 px**、滑条带 432 px ✔ |
| 原版 NPC 11 行页**真实滚轮** `-120×3` | §3.2ab 记 0 px | **正文区 11463 px**（反向滚轮回原状，净 0）✔ |
| 同位置**注入** `WM_MOUSEWHEEL` | 0 px | **0 px**（口径不变：注入滚轮不生效） |

**④ 撤回 §3.2ab 的结论（重要）**

§3.2ab 由"真实滚轮也不滚"推出「**原版这一版滚轮不达**」——那两条读数都是在
**降级窗口（`[IntPtr]::Zero`）+ 注入移动**的夹具下取的，**不成立**。修正后：
**原版 NPC 窗滚轮可用**（真实滚轮 11463 px，可逆），**注入 `WM_MOUSEWHEEL` 不可用**（夹具限制，
同 §3.2l-b 的"注入点击可用、注入拖动不可用"一类）。⇒ 本端滚轮可用（§3.2u）**与 C# 一致**，
**不再记"刻意背离"**。

**⑤ 修正 §3.2y**

§3.2y 的两半现在都有定论：

- **分类列索引**：`CategoryList.Count = 9 ≤ 22` ⇒ `DownButton` / `PositionBar_OnMoving` 里的守卫短路，
  **不动是对的**（§3.2ah 给了真源数字）；
- **滑块本身**：`PositionBar.Location = (x,y)` 在守卫**之外** ⇒ 真实光标一拖就**跟手**（+80px）。
  §3.2y 记的"滑条没动"是夹具问题（`Msg-Drag` 注入移动不改 `MPoint`）。

**⑥ 驱动补了两个函数（本轮）**

| 函数 | 用途 |
|---|---|
| `Drag-Image x y dx dy [steps] [stepMs]` | **真实光标**按下-步进-抬起（拖 `Movable` 控件/滚动条） |
| `Real-Wheel x y [delta] [n]` | 真实光标就位后发 `MOUSEEVENTF_WHEEL`（滚轮；`delta` 正 = 向上） |

配套口径：**任何鼠标交互试验前先 `SetWindowPos(h,[IntPtr](-1),…)` 置顶 + 用真实光标**，
并按 §3.2ag 做「悬停换帧」阳性对照；`Msg-Click` / `Msg-Wheel` / `Msg-Drag` 只用于**不依赖 `MPoint`**
的场景（点按钮、键盘路径、注入探针）。

> **实测口径补充**：同一个 `Drag-Image` 在会话里的**第一次**调用可能是空放（客户端还没"看见"光标
> 进入窗口）——本轮验证时第一次 0 px、紧接着第二次同参数 **432 px（滑块 +80px）**。所以
> **先跑悬停阳性、再跑正式动作**，并把"第二次复现"写进结论（别拿单次空放当"控件不动"）。

### 3.2aj 用修正后的夹具收掉「>8 项商品列表滚轮」这条未采集项（2026-09-28）

§3.2r / §3.2t 一直记着「**>8 项的商品列表**滚轮命中区未采集」（当时那两只商人的商品数 ≤ 8，
C# 的 `StartIndex` 顶/底守卫短路，滚了也不动）。恢复沙箱 `Envir` 之后，>8 项的商人脚本现成就有
（`BichonWall/Potion-0.txt` 18 项、`Blacksmith-1xxx` 19 项、`BookStore` 10 项…），
用 `setpos` + `npc_sweep` 一键到位：

```powershell
# DB 里 BichonProvince/BichonWall/Potion1（Alchemist_Samuel）@(324,291)
pwsh npc_sweep.ps1 -SandboxRoot $env:TEMP\golden_sandbox -SetPos -PosMap 1 -PosX 324 -PosY 292 -Points 504,336
# → 商人页 → 自动点首条链接（View Store）→ 商品窗（该商人 18 项）
```

判据：修正夹具（**真置顶 + 真实光标 + `Real-Wheel`**），每次滚完把光标移开再取帧（否则物品 tooltip
会盖在列表上、把读数污染成 1.8 万像素）：

| 动作 | 商品窗区 `(0,224,244,558)` 变化 | 说明 |
|---|---|---|
| 真滚轮**下 3 格** | **10451 px** | 列表内容滚动（截图逐字可见 `(HP)DrugLarge…` → `(HP)DrugSmall…`） |
| 真滚轮**上 3 格** | 10451 px | 滚回 |
| 再上 3 格 | **28 px** | 到顶后 `StartIndex <= 0` 守卫短路（该 28 px 是面板边框的动画残差） |
| ①→② 净 | **28 px** | 可逆 ⇒ 确认是"滚动"不是"重画" |

**顺带修掉的驱动 bug**：`CsUi.mouse_event` 的第 4 参声明成 `uint`，而滚轮 delta 是**有符号**的
（下滚 -120）⇒ PowerShell 传负数直接抛「无法转换为 `UInt32`」（第一版 `Real-Wheel` 就踩了，
那次"18367 px"其实是**物品 tooltip**、不是滚动）。改成 `int`（DWORD/int ABI 一致，不影响 `Click-Image`）。

**本端对照**：`npc_goods.rs` 用的是同一个 `UiScrollList`（`StartIndex -= count`，与 C#
`NPCGoodsPanel_MouseWheel` 同口径），命中区有单测 `goods_wheel_rect_is_exact_union_of_cells`；
**实机**侧的滚轮阳性在 NPC 窗那条已采（§3.2u），**商品窗的 RPC 级滚轮阳性本轮未单独采集**（如实）。

### 3.2ak 再修正：真实鼠标**本来就能点动原版**（§3.1 / §3.2l-b 的"点不动"是夹具假象）（2026-09-28）

§3.1（2026-09-26）与 §3.2l-b 都记过「解锁状态下 `Click-Image`（真实鼠标）仍点不动原版
`MirControl.OnMouseClick`」，并据此把"只能靠鼠标到达的那批窗"判成只能用键盘/注入消息绕。
按 §3.2ai 修正夹具（**真置顶 `[IntPtr](-1)` + 真实光标**）后复测，**这条作废**：

| 试验 | 判据区域 | 结果 |
|---|---|---|
| `Click-Image` 点背包**关闭钮** `(301,13)` | 背包区 `(0,0,316,236)` | **开 → 关**：73370/74576 px 变化（截图可见变成世界） ✔ |
| `Click-Image` 点背包 **`ITEMS II` 页签** `(112,19)` | 居中确认框区 `(284,289,740,479)` | **86523 px**（弹出扩容确认框，46 格角色的 C# 行为） ✔ |
| 同页签的**注入** `Msg-Click` | — | 同样有效（§3.2l-b 那条仍成立） |

⇒ **真实鼠标可用**：§3.1/§3.2l-b 那两条读数是在**非置顶窗口**下取的（真实输入落到了压在上面的窗口），
不是 `MirControl.OnMouseClick` 的语义限制。

**两个操作口径（本轮踩到）**

1. `Click-Image` 之前必须 `SetWindowPos(h,[IntPtr](-1),…)`（§3.2ai ②），否则真实输入到不了客户端；
2. **点击后要在全屏（或对的自绘区域）找变化**：本例点 `ITEMS II` 打开的是**居中确认框**
   （在背包矩形**之外**），只看 `(0,0,316,236)` 会误判成"没反应"（第一遍就是这么误判的）。

**对既有批次的影响**

- §3.2l「只能靠鼠标到达」那批窗的**前提**（"原版鼠标点不动"）不成立 ⇒ 以后这类窗**直接用真实鼠标**
  做两侧同状态 A/B 即可，不必再绕注入；
- 已经用**注入点击**做出来的结论（§3.2q/§3.2r/§3.2s/§3.2t/§3.2y/§3.2z 的点击类）**仍然有效** ✔；
- 仍然"注入不行"的只剩两类：**拖动**（§3.2y 修正）、**滚轮**（§3.2ab 撤回后：注入滚轮不行、真实滚轮行）。

### 3.2al 「同状态」两侧对表（第一批）：game_shop 分类行 —— 捞出 4 处口径差并修好（2026-09-28）

§3.2l-b 留的「逐窗同状态占比」一直没做，卡点是本端侧没把状态摆成同页/同分类/同选中。
§3.2ai 修正夹具后**真实鼠标可用**，这项终于能做。本轮先做 **`game_shop`**：
两端都点「分类第 2 行」这**同一逻辑目标**再取帧对表。

```powershell
# C# 侧（沙箱、键盘登录、真置顶 `[IntPtr](-1)`）：Y 开商城 → Click-Image 200 271
# 本端侧（--ui-scale 1）：dialog open game_shop → click 200 271
# 两侧同 1024x768@scale1；分类列几何两侧一致：Filters[i] 90x20 @(15,103+15i)，行距 15
```

**① 交互语义一致**

| 项 | C# | 本端 |
|---|---|---|
| 点第 2 行 | 该行进选中态 + 商品列表切到该分类 | 同 ✔ |
| 再点同一行 | **0 px**（幂等，不重复触发） | 0 px ✔ |

**② 捞出并修好的 4 处渲染口径差**（依据 `GameshopDialog.cs:26-28 / 426-467 / 682-698`）

| 项 | C#（原文） | 本端修前 | 修后 |
|---|---|---|---|
| 第 0 项文案 | **字面量** `"Show All"`（`SectionFilter` 哨兵，不本地化） | 中文「全部」 | `"Show All"` ✔ |
| 选中标记 | **只靠颜色** | 额外加 `▶ ` 前缀 | 去前缀 ✔ |
| 对齐 / 字号 | `Size=(90,20)` + 默认左上；`Font(…, **7F**)` | 居中、12px | 左上、**9.333px**（7pt×4/3）✔ |
| 颜色 | 选中 `(230,200,160)`、悬停 `(160,140,110)`、常态 `Color.Gray` | 单一 `(0.9,0.9,0.9)`、无悬停 | 三态照 C# 赋色（悬停复用点击那条矩形判据）✔ |

复验（同帧同状态）：本端选中行出现 **56 px 精确 `(230,200,160)`**（C# 102 px，差在字形粗细），
灰色像素 2 vs C# 0 ✔。

**③ 残余（同状态下仍不同的部分，都是已知类）**

分类列 `(164,240)-(292,420)` 两侧仍差 **28.8%**，且**逐行均匀**（每 15px 行 500–660 px）：
① **字形**（本端 CJK 字体渲染拉丁字母的度量 vs C# GDI 7pt）；② **面板透明处透出世界**
（两侧地图数据不同源）。都不是"画错/没画"。

**门禁**：`cargo test --lib` **864 passed**（含新增 `shop_category_row_text_and_color_match_csharp`）；
`b0001_smoke` 2 + `ui_alignment` 53；`ui_interact_sweep.ps1 -ManageServer` **44/44 exit=0**。
### 3.2am 「同状态」对表（第二批）：`npc_goods` **选中格边框** —— C# 有、本端没有，已补（2026-09-28）

方法同 §3.2al：两端点**同一格**（`(100,274)` = 商品窗第 1 格 `Cells[0] @ (10,34)` 的中心）再对表。

**① C# 侧（权威证据）**

`dbtool setpos 1 324 292` + `npc_sweep -Points 504,336` 打开
`BichonProvince/BichonWall/Potion1`（Alchemist_Samuel）的商店（18 项），真实鼠标点第 1 格 ⇒
商品窗里出现 **`Color.Lime (0,255,0)` 边框 510 px，bbox 屏幕 `(9,257)-(215,290)`**
= 面板相对 `(9,33)-(215,66)`。

**② 源码（三处拼起来的完整行为）**

| 位置 | 内容 |
|---|---|
| `MirControls/MirGoodsCell.cs:21` | `BorderColour = Color.Lime` |
| `MirControls/MirGoodsCell.cs:97-113` | `BorderInfo` 五段：上/左/下/右 + **`Left+40` 处的竖分隔线**；线框在 `(L-1,T-1)-(R,B)` |
| `MirScenes/Dialogs/NPCDialogs.cs:1349` | `Cells[i].Border = SelectedItem != null && Cells[i].Item == SelectedItem;`（**只有选中格**画边框） |

**③ 本端缺口与修法**

`npc_goods.rs` 一直有 `state.selected`（买/回购用），但**完全不画**选中边框 ⇒ 真缺口。修法：

- `select_border_segments()` **纯函数**给出五段几何（逐条对应 C# `BorderInfo`）；
- 每行 5 条 1px `BackgroundColor(Color::Lime)` 节点（spawn 时 `Visibility::Hidden`）；
- `npc_goods_selection_border_system` 按 `state.selected == Some(i) 且该格有货` 切显隐。

**④ 复验（同状态下两侧逐值相同）**

| 侧 | lime 像素 | bbox（屏幕） |
|---|---|---|
| C# | **510** | `(9,257)-(215,290)` |
| 本端（修后） | **510** | `(9,257)-(215,290)` |

⇒ 连像素数都一致 ✔。单测 `select_border_segments_match_csharp_border_info` 把五段几何与
C# 帧实测的 bbox 钉在一起（`(9,33)-(215,66)`、竖线 `x=50`）。

**门禁**：`cargo test --lib` **864 passed**；`b0001_smoke` 2 + `ui_alignment` 53；
`ui_interact_sweep.ps1 -ManageServer` **44/44 exit=0**。
### 3.2an 「同状态」对表（第三批）：`npc` 窗**行内链接两态色** —— C# 纯黄/纯红，本端两态都偏（2026-09-29）

继续按「同一逻辑目标 + 同一取帧口径」逐项对表。这一批的对象是 **NPC 对话窗里的行内链接**
（脚本标记 `<文字/@键>`）。它**不需要原版鼠标**：两态色是**源码常量**，而 §3.2q/§3.2r 当时
为了"用颜色扫描量链接矩形"已经把**原版两态的原像素**顺手记过一遍，直接复用即可。

**① C# 权威依据（源码）**

`Client/MirScenes/Dialogs/NPCDialogs.cs:506-523`（`NewButton`，即 `R = <((.*?)\/(\@.*?))>`
这一类的渲染器）：

| 事件 | 赋值 |
|---|---|
| 初始 | `ForeColour = Color.Yellow` |
| `MouseEnter` | `Color.Red` |
| `MouseLeave` / `MouseDown` | `Color.Yellow` |
| `MouseUp` | `Color.Red` |

`MirControls/MirLabel.cs:220-233` 把 `ForeColour` 直接交给 `TextRenderer.DrawText`（配 `OutLineColour`
描边），所以它就是文字填充色。同窗另两类不是这个色：`C = {文字/颜色}` 走 `NewColour`
（用脚本给的颜色名、无悬停态），`[MONSTER:/NPC:/ITEM:]` 走 `NewLink`（Cyan → 悬停 Orange）——
本服脚本量到 `MONSTER/NPC/ITEM` 各 **0** 处，故本轮只对 `R` 这一支。

**② 原版帧（复用沙箱 `shots/`：同一只商人 NPC、键盘登录、真置顶 + 真实光标悬停）**

| 帧 | 精确 `(255,255,0)` | 精确 `(255,0,0)` |
|---|---|---|
| `orig_lc2_0_before.png`（未悬停） | **191** | 0 |
| `orig_lc2_1_hover_view.png`（悬停 `View`） | 127 | **64** |
| `orig_lc2_2_after_click_view.png`（点开商店后） | 207 | 0 |

⇒ 悬停把**被悬停那一段**的 64 px 黄字换成纯红，其余黄字不动；离开/点击后回黄。
（这就是 §3.2r「悬停 `View` → 窗区 68 px 变化」那条读数的分解。）

**③ 本端缺口**

`Client-Bevy/src/game/dialogs/npc.rs` 的链接段用的是 `(1.0,0.85,0.3)`（≈`255,217,76`）常态、
`(1.0,0.95,0.4)`（≈`255,242,102`）悬停 ⇒ **常态不是纯黄、悬停根本不是红**。

**④ 修法**

抽出 `npc_link_color(hovered) -> Color`（`Color::Yellow` / `Color::Red`，逐值对应 C#），
链接段改调它；同一行里的 `{文字/颜色}` 段与纯文本段不受影响（仍按各自颜色/白色画）。

**⑤ 复验（同状态同坐标）**

本端夹具：worktree 产物 + `--ui-scale 1` + 真实 TCP 7000；`@mapmove 0 374 296` 后
`nearby` 取 NPC 的 `object_id`，再 `npc_call {object_id, key="[@main]"}` 开窗；
链接矩形直接读 `npc_rows`（View `x[8,34] y[70,86]` / Ask `x[8,27.5] y[88,104]` /
Close `x[8,40.5] y[124,140]`）；悬停用本端 `cursor {x,y}` 注入链接中心
（注入的是**窗内逻辑坐标**，不是屏幕坐标——这也是本端比原版好用的地方：
原版必须真置顶 + 真实光标，本端这条 RPC 能直接指定）。

| 帧 | 链接段常态色 | 悬停段色 |
|---|---|---|
| 本端 修前 `ours_pre_normal.png` | `(255,217,76)` × 20 | — |
| 本端 修前 `ours_pre_hover_view.png` | — | `(255,242,102)` × 8 |
| 本端 修后 `ours_nl_normal_01.png` | **`(255,255,0)` × 20** | 红 **0** |
| 本端 修后 `ours_nl_hover_view_01.png` | `(255,255,0)` × 12 | **`(255,0,0)` × 8** |
| 本端 修后 `ours_nl_hover_ask_01.png` | `(255,255,0)` × 14 | **`(255,0,0)` × 6** |

红字 bbox 也逐段对上：悬停 `View` 红字 `(9,73)-(25,81)`（链接矩形 `8..34 / 70..86`），
悬停 `Ask` 红字 `(10,94)-(26,99)`（矩形 `8..27.5 / 88..104`）——**只有被悬停那一段变红，
另两段保持黄**，与 C# 同构。

**⑥ 残余（同状态下仍不同的部分）**

绝对像素数 20 vs C# 191 **不是色差**，是**字形**：本端 GLyph 光栅化落在链接矩形里的饱和像素
天然比 GDI `TextRenderer` 少（同类残差见 §3.2al/§3.2t）。本项判据是**色值**与
**"哪一段变红"**，两者现已逐值一致。

**⑦ 顺带定性：`[@XXX]` 整行分支在现网不可达（本轮未改）**

`npc.rs` 另有一条「无标记的行内可点行 → 整行橙」分支（`!has_markup && clickable`）。
把沙箱 635 个 NPC 脚本逐行过了一遍：`is_clickable_npc_line` 命中 **11433** 行，其中
**6405** 行走 `<.../@...>`（链接分支），剩下 **5028** 行全是 `[@段头]`
（服务端 `ServerRust/src/actors/world/npc_script.rs:239-250` 把 `[@xxx]` 当段头消费掉、
根本不进 `#SAY` 正文；另有 `GM.rar` 的二进制噪声）。⇒ 这条分支在现网数据上**不可达**，
本轮不动它，只记在这里，免得下次把它误当"没对齐的配色"去改。

**门禁**：`cargo test --lib` **866 passed**（含新增
`npc_link_colour_matches_csharp_newbutton_yellow_and_red`；阳性对照实做：把 `npc_link_color`
的悬停分支改回 `(1.0,0.95,0.4)` ⇒ 立即红，读数 `悬停应为 Color.Red (255,0,0)，实得 (255,242.25,102)`）；
`b0001_smoke` 2 + `ui_alignment` 53；`ui_interact_sweep.ps1 -ManageServer` **44/44 exit=0**。
### 3.2ao 「从没进过表」的 13 扇窗：几何复核 + 面板美术逐像素；顺带查到 `report` 窗**原版美术是空的**（2026-09-29）

§3.2af 那张 20 行 A/B 表是**键位可达**的那批；客户端还有一批窗（NPC 驱动 / 条件可见）从没进过
任何一次对表 —— `storage / craft / socket / market / npc_awake / quest_detail / mail_compose /
hero_inventory / input_box / item_rental / item_rental_browse / guild_territory / mentor / report`。
本轮把它们的**几何**与**面板美术**两层各验一遍（都不需要原版客户端，所以**锁屏也能做**）。

**① 几何：把 §3.3 的对表刷新到当前 master（`826078403`）**

```powershell
pwsh tools\acceptance\csharp_golden\probe_ui_nodes.ps1 -Repo <检出> -ClientHome <检出> `
     -RectKinds all -InventoryOnly -Out %TEMP%\rects_all_20260929.json
py -3.12 tools\acceptance\csharp_golden\window_rect_table.py --src <检出> --data Data `
     --compare %TEMP%\rects_all_20260929.json
# ⇒ 不一致：0；跳过（未取到/表达式）：7
```

**29 窗一致、0 不一致、7 SKIP**。这批里**从没被 §3.2af 那种 A/B 表点过名**的窗，现在都有了
C# ↔ 我端的几何行：`storage (0,0,388,346)`、`craft (-12,236,337,215)`、`socket (117,241,81,62)`、
`market (0,0,492,478)`、`npc_awake (0,0,360,420)`、`quest_detail (532,60,316,466)`、
`mail_compose (100,100,236,300)`、`hero_inventory (0,0,324,266)`、`item_rental (718,287,204,109)`、
`item_rental_browse (312,297,400,174)`、`guild_territory (0,0,568,241)`、`mentor (390,280,244,207)`。
7 个 SKIP 各有据：`fishing/guild/creature/mount` 是 §3.2e 的 **`Show()` 守卫窗**（只弹提示不开窗，
取不到关闭钮，是判据的设计不是缺陷）、`trade` 需要对手方、`hud_belt/hud_skillbar` 那一刻不在屏上。

还有 **8 个 kind 不在这张表里**（工具没有 C# 期望可比，只有我端实测矩形）：
`buff / chat_notice / dura_status / timer`（条件可见，探针那一刻不在屏上）、
`hero_equipment / inspect`（C# 侧按状态换图/由对手方状态驱动）、`input_box`（服务端发起）、
以及 `report`（原因见 ④：C# 面板美术是空条目，`Location = Center` 用 0 尺寸解不出可比矩形）。
它们这轮的我端实测矩形：`hero_equipment (760,0,264,380)`、`input_box (368,306,288,156)`、
`report (332,262,360,244)`、`durability`/`buff`/`timer`/`chat_notice` 探针那一刻未上屏。

**② 面板美术：我端渲染 vs C# 美术逐像素（判据 = 不符像素占比，阈值 0.9 命中）**

```powershell
# 逐窗：开窗 → 截图（关掉背包/小地图，见 ③）→ 与 C# 那帧美术比
py -3.12 tools\acceptance\csharp_golden\art_match.py --shot <帧> --lib Data\Prguse.Lib `
     --rect 0 0 388 346 --candidates 586      # storage 为例
```

| 窗 | C# 面板美术 | 不符率 | 判定 |
|---|---|---|---|
| `hero_inventory` | `Prguse[1422]` | **0.004** | 一致 |
| `storage` | `Prguse[586]` | 0.026 | 一致 |
| `quest_detail` | `Prguse[960]` | 0.031 | 一致 |
| `guild_territory` | `Prguse[680]` | 0.033 | 一致 |
| `mail_compose` | `Title[671]` | 0.046 | 一致 |
| `npc_awake` | `Title[710]` | 0.050 | 一致 |
| `socket` | `Prguse3[20]` | 0.057 | 一致 |
| `item_rental_browse` | `Prguse3[1]` | 0.062 | 一致 |
| `craft`（按屏幕内部分） | `Prguse[1109]` | 0.082 | 一致（见 ③ 负原点） |
| `input_box` | `Prguse[660]` | 0.092 | 一致 |
| `market` | `Title[786]` | 0.098 | 一致 |
| `mentor` | `Prguse[170]` | 0.181 | 面板一致，残差 = **窗内中文文案 + 两个钮**（§3.2af 已定性的"语言/内容"类） |

**③ 三处"看着不符"其实是夹具/工具口径，不是缺陷**

1. **`craft` 的负原点**：C# `CraftDialog.Show()` 里 `Location = (InventoryDialog.X - 12, Y + 236)`
   ⇒ x = **-12**，左边 12 px 在屏外。`art_match` 直接按 `-12` 裁会把这 12 列和黑边比 ⇒ 报 **0.160** 假红；
   按**屏幕内那部分**裁（列 12.. 对列 0..）比是 **0.082**。⇒ 负原点窗要么按内部分裁，要么认这个已知口径。
2. **写邮件窗被背包压住**：首轮 `mail_compose` 报 **0.270**，看图发现是**背包窗（`(0,0,316,236)`）
   压在写邮件窗（`(100,100,236,300)`）上面** —— 本端 `dialog open` 的 RPC **不抬 z**（真机点窗会抬）。
   把背包/小地图关掉再截 ⇒ **0.046**。⇒ 这批窗截图前必须 `dialog {kind:'inventory'|'minimap', action:'close'}`。
3. **`mentor` 0.181**：并排看（美术 vs 帧）面板逐像素一致，差的是窗内我们画的中文标签
   （师父/徒弟/允许拜师）与 `MENTOR`/`SECESSION` 两个钮 —— 同一类残差见 §3.2al/§3.2t。

**④ 捞到一条真东西：`report` 窗的 C# 面板美术**是空的

`Client/MirScenes/Dialogs/ReportDialog.cs:15-16`：`Index = 1633; Library = Libraries.Prguse;`。
直接读 `Data/Prguse.Lib` 的图头（`libextract.py` 同格式）：

```
1631 w 0 h 0 len 0 / 1632 w 0 h 0 len 0 / 1633 w 0 h 0 len 0 / 1634 w 0 h 0 len 0 / 1635 w 0 h 0 len 0
```

⇒ **1631–1635 整块都是空条目**。两条旁证：① 沙箱 `Client\Data\Prguse.Lib` 与仓库 `Data\Prguse.Lib`
**MD5 相同**（`06284454F488891AA24BD72BF70F9A38`）⇒ 原版客户端读到的也是 0×0；
② 在 `Prguse/Prguse2/Prguse3/Title` 四个库里扫 `360x244` 的图，**一张都没有**（不是"索引写错到别处"）。
于是原版这版的行为是：面板**什么都不画**，而 `Location = Center` 用 0 尺寸算成 **(512,384)**，
子控件（关闭钮 `(336,3)`、下拉 `(12,35)`、描述框 `(12,57)`、提交钮 `(260,219)`）就锚在屏幕中心那条线上。
`MLibrary` 的索引是**一一对应**（`_indexList[i] = ReadInt32()`，不跳过空条目），所以这不是映射错觉。

本端 `Client-Bevy/src/game/dialogs/report.rs` 的做法是**有意偏离**：`PANEL_SIZE = (360,244)`
（由子控件外沿反推：关闭钮 336+24、提交钮 219+24）+ 深色兜底面板、按居中摆 (332,262)。
⇒ 记在案：**这不是"没对齐"，是 C# 那版缺资产**；要不要改成"照原版什么都不画"，属于产品取向，
留痕不擅自改（§3.2ao 就是它的凭证）。

**⑤ 两端同状态 A/B：这几扇窗本轮**未采集**（工作站锁屏）**

`storage / craft / market / npc_awake` 是 NPC 驱动的，要**原版侧真鼠标点 NPC** 才能出帧；
本轮准备跑时工作站是**锁屏**状态——判据（§3.2ag）：

```powershell
# 前台窗口 class = Windows.UI.Core.CoreWindow、标题「Windows 默认锁屏界面」
fg=0x2102CA class='Windows.UI.Core.CoreWindow' title='Windows 默认锁屏界面'
```

⇒ 按 owner 的边界**没有硬跑**（硬跑出来的"点不动"是假红）。配方已就绪，解锁后一条命令即可：

```powershell
# 起点选在 BichonWall 仓库/工匠/信托商人附近；落点由 DB 的 NPC 表算出（§3.2z）
pwsh tools\acceptance\csharp_golden\npc_sweep.ps1 -SandboxRoot $env:TEMP\golden_sandbox `
     -SetPos -RestartPerPoint -PosMap 1 -PosX 301 -PosY 259 `
     -NpcsJson $env:TEMP\golden_sandbox\npcs_1.json -PlayerX 301 -PlayerY 259 -MaxDist 40 `
     -Only 'Warehouse1'
# ⚠ 起点不能与 NPC 同格（实测 (301,257) 距离 0 时点出来的是角色自己，页型恒 closed）
```

> **2026-09-29 补**：`npc_sweep.ps1` 现在自带**两道前置守卫**（都在"起服务端/重启客户端/键盘登录"**之前**判，不合格 `exit 2` 且不碰客户端）：
> ① **锁屏守卫**（判据同 §3.2ag：前台窗口 `class=Windows.UI.Core.CoreWindow`）——锁屏下点击到不了 winit，
> 硬跑只会产出 `NPC窗=关` 的假红（本轮 02:03/02:06/02:19 连续三次复测都是锁屏）；确需锁屏下跑用 `-AllowLocked` 显式放行。
> ② **同格守卫 + 空候选守卫**：起点与目标 NPC 同格时直接拦下（同格点出来的是角色自己）；
> `-Only/-PlayerX,-Y/-MaxDist` 一个落点都没算出来时也拦下并给出提示（`-Only` 匹配的是**脚本相对路径**，不是名字；
> 可见范围只有 ±10 格 x / ±11 格 y）。四类走法都实测过：锁屏→exit 2、同格→exit 2、空候选→exit 2、正常配置→继续建点。

**门禁**：本轮只动文档 + `report.rs` 一处注释（无代码逻辑变化）：`cargo test --lib` **866 passed**；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**；
`ui_interact_sweep.ps1 -ManageServer` **44/44 exit=0**。
### 3.2ap 「几何表里没映射」的 8 个 kind：`inspect` 入表、`chat_notice` 带透明度复核、`buff/timer` 不可比（2026-09-29）

§3.2ao ① 末尾列了 8 个 kind 工具没有 C# 期望可比。本轮把能定位的逐个补上判据。

**① `inspect`：几何 + 面板美术都逐值对上（新入表的一扇）**

C# `InspectDialog`（`MainDialogs.cs:2153-2155`）：`Index = 430; Library = Libraries.Prguse;
Location = (536,0);`，图头 **264x408** ⇒ 期望矩形 `(536,0,264,408)`。

| 侧 | 读数 |
|---|---|
| 本端 `dialog_rect {kind:'inspect', fallback:'root'}` | **(536,0,264,408)** —— 与 C# 逐值相同 |
| 本端 `art_match --rect 536 0 264 408 --candidates 430` | **0.064**（命中） |

**② `chat_notice`：带 `Opacity` 的面板别拿"纯美术"比——换成两层复合模型逐值命中 94.4%**

C# `ChatNoticeDialog`（`ChatNoticeDialog.cs:15-19`）：面板 `Prguse[1361]`（660x25）、装饰边子控件
`Prguse[1360]`（660x25 @ 面板内 (0,0)）、`Opacity = 0.7F`；
`Location = (ScreenWidth/2 - Size.Width/2, ScreenHeight/6 - Size.Height/2)`（逐项整数除法）= **(182,116)**。

直接用 `art_match` 比"纯美术"只有 **0.805**（FAIL）——**因为面板是 0.7 透明叠在世界上的**，
那个读数不代表画错。改判据（从基线帧取"世界"层、按 C# 的合成顺序算期望像素）：

```python
# layer1 = 0.7*Prguse[1361] + 0.3*world ；final = alpha(1360)*1360 + (1-alpha(1360))*layer1
```

| 模型 | 逐值命中（容差 60） |
|---|---|
| 只算 `0.7*1361 + 0.3*world` | 0.6369（10493/16475） |
| **两层复合**（再叠 `Prguse[1360]`，按它的 alpha） | **0.9439（15551/16475）** |

⇒ 面板精灵、装饰层、透明度、位置**四样都对上**；残差 5.6% 是文案字形（同类见 §3.2al）。
**口径警告**：带 `Opacity` 的窗（`chat_notice`、`game_shop` 分类列的半透明底…）**不能用 art_match 直接判红**。

**③ `dura_status`：结构差一层容器，绝对位置/尺寸与交互等价（不是缺口）**

C# `DuraStatusDialog`（`MainDialogs.cs:3904-3930`）：容器**没有 Index/Library**（不可见），
`Size = (40,19)`、`Location = (MiniMapDialog.X + 86, MiniMapDialog.Size.Height)` = **(984,154)**；
里面那颗 `Character` 钮 `Prguse[2113]`（hover 2111 / pressed 2112）`Size = (20,19)`、容器内 `(20,0)`
⇒ 绝对 **(1004,154)**，点击切 `CharacterDuraPanel`。

| 侧 | 根矩形 | 钮的绝对矩形 |
|---|---|---|
| C# | (984,154,40,19)（不可见容器） | (1004,154,20,19) |
| 本端 `dialog_rect` | **(1004,154,20,19)** | 同 —— 本端的"根"就是那颗钮 |

⇒ 钮的位置/尺寸**逐值相同**；差的是 C# 多一层不画任何东西、也没有点击处理的 40x19 容器
（左半 20px 是空的）⇒ **视觉与交互等价**，记口径不记缺口。

**④ `buff` / `timer`：当前状态**无内容** ⇒ 不可比；而且"开窗帧 vs 基线帧"会被世界动画骗**

| 窗 | C# 期望 | 本端实测 |
|---|---|---|
| `buff` | `BuffDialog` `Prguse2[20]`（44x34）@ `(ScreenWidth-170,0)` = **(854,0)** | `dialog_rect{fallback:'root'}` **ok=false**（见 ⑤）；`art_match (854,0,44,34)` = **1.000** ⇒ 该处什么都没有 |
| `timer` | `TimerDialog`（`MirControl`，**无面板美术**）`(1024-120,768-230)` = **(904,538)**、`Size (120,100)` | 期望矩形内 **0 px 变化** |

两扇都是**内容驱动**的：测试角色没有 buff、没开计时器 ⇒ 两端都是空条，**不可比（不是缺失）**。
本端常量本身是对的（`timer.rs` 的单测就钉着 `PANEL_ORIGIN == (904,538)`；`buff.rs` 注释记 C# (854,0)）。

**判据教训（本轮实测）**：先用"开窗帧 − 基线帧"找窗口位置，会**把世界里的怪物/动画当成窗口**——
同一状态下**连拍两张基线**，自身就在 `(516,301)-(581,406)` 差 **1376 px**（D002 里有怪在走），
而 `buff` 帧的差是 1485 px、`timer` 帧 853 px，被排除掉那块后**窗内是 0**。
⇒ 判"这扇窗画了没有"要看**期望矩形内**的差，别用全屏差的最大包围盒。

**⑤ 工具覆盖缺口（留给下次）**：`dialog_rect {kind, fallback:'root'}` 对 `buff`/`timer` 返回
`ok=false`（它们是 manifest 的"无关闭钮设计"窗，根节点没登记 kind 标记）⇒ 这两扇**进不了几何对表**，
本轮只能靠像素判据。另注：`chat_notice` 的 `dialog open` 只切 `ChatNoticeState.visible`、
**不进 `DialogManager`** ⇒ `dialogs` 列表里看不到它，别据此判"没开"（本轮踩过）。

**门禁**：本轮只动文档，`cargo test --lib` **866 passed**（无代码变化）。
### 3.2aq §3.2af 的 20 行表在 master `632224d15` 复跑：**没有回归**（2026-09-29）

§3.2al/§3.2am/§3.2an 三笔改的是 `game_shop` 分类行、`npc_goods` 选中格边框、`npc` 行内链接两态色
——都落在 A/B 表能看到的窗里。所以按 §3.2af 的老配方**原样复跑一遍**，看有没有把别处带坏：

```powershell
pwsh tools\acceptance\csharp_golden\golden_ab_ours.ps1 -SandboxRoot $env:TEMP\golden_sandbox `
     -ClientHome <检出> -User goldenchr -Password 123456
py -3.12 tools\acceptance\csharp_golden\golden_ab_diff.py `
     --shots %TEMP%\golden_sandbox\shots --table %TEMP%\rect_table_20260929.json --max-shift 1
```

| 行 | §3.2af（`83e5ec6b0`） | 本轮（`632224d15`） | 判定 |
|---|---|---|---|
| Group | 0.0% | **0.0% OK** | 不变 |
| Inventory | 2.9% | **2.9%** | 不变 |
| Skills | 1.7% | **1.7%** | 不变 |
| Equipment | 6.3% | **6.3%** | 不变 |
| Options | 6.2% | **6.2%** | 不变 |
| Quests | 6.3% | **6.9%** | +0.6pp：任务列表**内容**（数据不同源） |
| Friends | 24.4%（±1px 后 2.3%, dx=1） | **24.4%（2.3%, dx=1）** | 不变 |
| Relationship | 13.1% | **13.1%** | 不变 |
| Ranking | 10.6% | **10.6%** | 不变 |
| Guilds（守卫窗） | 66.2% | **66.2%** | 不变 |
| Help | 87.6%（±1px 后 10.7%） | **87.6%（10.7%, dx=1）** | 不变 |
| Keybind | 26.6% | **26.6%** | 不变 |
| Creature（守卫窗） | 50.1% | **50.1%** | 不变 |
| MountWindow（守卫窗） | 94.5% | **94.4%** | 不变 |
| Fishing（守卫窗） | 34.4% | **34.4%** | 不变 |
| GameShop | 29.9% | **29.9%** | 不变 |
| Bigmap | 58.6% | **58.6%** | 不变 |
| Minimap | 91.3%（我方没出窗 ⇒ 不可比） | **91.3%（不可比）** | 不变 |
| Belt / Skillbar | 91.7% / 91.7%（**整帧参考**，露世界） | **98.2% / 99.1%** | 这两行本来就**不能用窗口矩形判**（§3.2k 改成"只比美术不透明像素"：腰带 7.4%）；整帧数会随"露出来的世界"波动，不是回归 |

⇒ **14 行逐值不变**，无回归；唯一的移动项都是早已定性的「数据不同源 / 露世界」两类。
（`Quests` 的 ±0.6pp 与 Belt/Skillbar 的整帧波动都不指向代码改动。）

**门禁**：本轮只动文档，无代码变化。
### 3.2at 觉醒 NPC 的「分解/降级/重置」在 C# 里开的是**投放面板**，不是觉醒面板（2026-09-29）

沿 §3.2ar/§3.2as 继续点 NPC 时顺手查了觉醒 NPC（`BichonProvince/BichonWall/Awakening`，map 1 @(338,338)）
的三个链接 `<Disassemble/@Disassemble>` / `<Downgrade/@Downgrade>` / `<Reset/@Reset>`。

**① C# 真值（源码）**：`GameScene` 里四条路的落点**不一样**

| 包 | C# 处理（`GameScene.cs`） | 开哪扇窗 |
|---|---|---|
| `S.NPCAwakening` | `:6347-6350` `NPCAwakeDialog.Show()` | **觉醒面板** `Title[710]` @**(0,224)** |
| `S.NPCDisassemble` | `:6352-6356` `NPCDropDialog.PType = Disassemble; Show()` | **投放面板** `Prguse2[351]` @**(264,224)** |
| `S.NPCDowngrade` | `:6358-6362` 同上，`PType = Downgrade` | 同上 |
| `S.NPCReset` | `:6364-6368` 同上，`PType = Reset` | 同上 |

**② 实机取证（原版侧）**：点 `Disassemble` 后，(264,224) 出现投放面板，文案
`Item will be Destroyed`（= `ClientTextKeys.ItemWillBeDestroyed`，只出现在 `NPCDropDialog` 的
`BeforeDraw` 里，`NPCDialogs.cs:1770-1771`）；NPC 窗仍在（`Prguse[995]` 不符率 0.0182）。
（同页的另两档 `Downgrade`/`Reset` 走的是同一分支、同一位置，只换 `InfoLabel` 文案，
**本轮没有逐档再点**，如实记为未采集。）

**③ 本端缺口（已修）**：这三条包原先被接成 `NpcAwakePanel{service:1|2|3}` →
**画觉醒面板**（`Title[710]` @(0,224)），而且那扇面板上还多了 **4 颗自造的"服务模式"页签**
（觉醒/分解/降级/重置 @(30+72i,26)）——C# 的 `NPCAwakeDialog` 里**没有**这种页签
（它只有 `UpgradeButton` `Title[712..714]` @(115,391)）。修法：

- `network/packets/handle_progress.rs`：三条包改发 `ServerEvent::NpcSellPanel{ panel_type: Disassemble/Downgrade/Reset }`
  （与 C# 的 `NPCDropDialog.PType = …` 一一对应）；
- `game/dialogs/sell_panel.rs`：补这三档的 `InfoLabel` 文案与**确认发包**
  （`C.DisassembleItem` / `C.DowngradeAwakening` / `C.ResetAddedItem`，对齐 `NPCDialogs.cs:1586-1597`）；
- `game/dialogs/npc_awake.rs`：删掉那 4 颗自造页签；事件入口加纯函数
  `NpcAwakeService::opens_awake_panel(service)` —— **只有 0 才开觉醒面板**，收到 1/2/3 会 `warn!` 并忽略
  （接线回退时不静默）。

**④ 门禁**：`cargo test --lib` **867 passed**（新增
`only_awakening_service_opens_awake_panel`；阳性对照实做：把 `opens_awake_panel` 改回 `service <= 3`
⇒ 立即红），`sell_panel` 的文案测试补了三档断言。

**⑤ 顺带一条口径**：`NpcAwakePanel` 这个 `ServerEvent` 的 `service` 字段从此**只有 0 会真正生效**
——服务端那边仍然照 C# 发四种包（`ServerRust/.../npc.rs:1884-1916`），客户端按上面分流。
### 3.2ar 「鼠标驱动」那批窗的两端同状态 A/B：`storage / craft / market / npc_awake` —— 捞出 1 处真缺口（npc_awake 的位置）（2026-09-29）

§3.2ao⑤ 留下的「需要原版侧真鼠标点 NPC」那批，owner 解锁后本轮跑完。**解锁判据两条都过**：
前台窗口 `class=CASCADIA_HOSTING_WINDOW_CLASS`（不是 `Windows.UI.Core.CoreWindow`）、桌面级
`CopyFromScreen` 采样非纯色。

**① 夹具：NPC 对话窗的**行内链接**要用「真光标 `Move-Image` + `Msg-Click`」，`Click-Image` 会只 hover 不点**

```powershell
# 打开 NPC 窗（npc_sweep 内部就是这一对）：
npc_sweep.ps1 -SetPos -RestartPerPoint -PosMap 1 -PosX <px> -PosY <py> `
   -NpcsJson %TEMP%\golden_sandbox\npcs_1.json -PlayerX <px> -PlayerY <py> -MaxDist 6 -Only '<npc>'
# 点行内链接（本端 RPC 侧同理：先 Move-Image 把真光标压到链接上，再 Msg-Click）：
Move-Image <lx> <ly>; Msg-Click <lx> <ly> 220
```

- `npc_sweep` 点 NPC 与点链接用的都是 **`Move-Image` + `Msg-Click`**（脚本第 284/305 行）——storage 就是这么开出来的。
- 本轮手动用 **`Click-Image`（SetCursorPos + mouse_event）连试两次都没点动** `CraftsLady` 的 `Crafting` 链接：
  截图里链接变成**红色 hover**、页面一字未变（`npc_page_probe` 因此报 `no-yellow-text` —— **别把这个读数当成"页面变了"**，
  链接被悬停成红色后就扫不到黄字了）。改成 `Move-Image` + `Msg-Click` 立刻开出 craft 窗（0.0393）。
- ⇒ 口径：**判据必须是"目标窗真的出现在期望位置"**（全屏模板匹配），不是"黄字带没了"。

**② 四窗两端读数（C# 用全屏 `cv2.matchTemplate` 找面板美术的真实落点；本端用 `dialog_rect` + `art_match`）**

| 窗 | C# 实测（全屏模板匹配） | 本端 | 结论 |
|---|---|---|---|
| `storage` | `Prguse[586]` **@(0,0)**，不符率 **0.0269** | `(0,0,388,346)`，art 0.026 | 一致 |
| `market` | `Title[786]` **@(0,0)**，不符率 **0.0855** | `(0,0,492,478)`，art 0.098 | 一致 |
| `craft` | `Prguse[1109]` **@(431,236)**，不符率 **0.0393** | `(-12,236,337,215)`（背包在 (0,0) 时） | 锚点公式一致（`inv.X-12, inv.Y+236`）、**绝对值随背包位置**：C# 开 NPC 窗会把背包推到 `(44x,0)`（`NPCDialog.Show()` 里 `InventoryDialog.Location = (Size.Width+5, Y)`），本端不推 ⇒ 这一维不可比 |
| `npc_awake` | `Title[710]` **@(0,224)**，不符率 **0.1125** | 修前 `(0,0,360,420)` | **真缺口**（见 ③） |

**③ 捞到的真缺口：`npc_awake` 贴在 NPC 对话窗**正下方**，不是左上角**

C# `NPCAwakeDialog` 类里写的是 `Location = new Point(0, 0)`（`NPCDialogs.cs:1884`），但 `GameScene`
构造时**逐实例覆盖**：`NPCAwakeDialog = new NPCAwakeDialog { …, Location = new Point(0,
GameScene.Scene.NPCDialog.Size.Height) }`（**`GameScene.cs:307`**）。`NPCDialog` = `Prguse[995]` 440x224
⇒ 真位置 **(0,224)**。原版帧的全屏模板匹配也正好命中 **(0,224)**（0.1125），肉眼可见窗口上沿贴在 NPC 窗底边。

修法（`Client-Bevy/src/game/dialogs/npc_awake.rs`）：新增 `PANEL_ORIGIN = (0.0, 224.0)` 并在 spawn 用上，
注释写明"类里的 (0,0) 会被 `GameScene.cs:307` 覆盖"；单测
`main_item_cell_matches_csharp_mir_item_cell_default` 里加断言（阳性对照实做：把 `PANEL_ORIGIN`
改回 `(0,0)` ⇒ 该测试立即红）。

**复验（同状态）**：本端修后 `dialog_rect {kind:'npc_awake'}` = **(0,224,360,420)**；
`art_match --rect 0 224 360 420 --candidates 710` = **0.050**；本端帧全屏模板匹配最佳落点也是 **(0,224)**（0.0519）
⇒ 与 C# 同锚点。

**④ 顺带记一条工具口径缺口**：`tools/acceptance/csharp_golden/window_rect_table.py` 的期望值读的是
**类里的** `Location`，对 `NPCAwakeDialog` 给成 `(0,0,360,420)`——只要本端也画在 (0,0)，
§3.3 那张几何表就会**判 OK（假绿）**。本轮的真值来自实机模板匹配，别再按那张表判这一行
（`GameScene` 里逐实例覆盖 `Location` 的窗全仓只有这一处，已 grep 过）。

**门禁**：`cargo test --lib` **866 passed**（含新增断言；阳性对照实做）；
`b0001_smoke` 2 + `ui_alignment` 53；`ui_interact_sweep.ps1 -ManageServer` 见 PR 正文。
### 3.2as 继续沿「原版点 NPC」这条路：`NPCDropDialog`（修理/出售面板）位置对表（2026-09-29）

§3.2ar 收工后按 jev 结论继续同一类窗（jev `classify`：next_npc_windows **0.960**、
tool_locator 0.040、other 0.000，置信度 0.940 —— "继续把同法套到其余原版侧可入口的 NPC 驱动窗"
显著优先）。本轮选的第一扇是 **`NPCDropDialog`**（我们的 `sell_panel.rs`），它从没进过任何 A/B 表。

**入口**：`BichonProvince/BichonWall/Blacksmith`（`Blacksmith_Bill`，map 1 @(302,221)）页面上的
`<Special/@SRepair>`。配方同 §3.2ar：`-PosX 302 -PosY 223` + `-NoClickLink` 开窗，再
`Move-Image` + `Msg-Click` 点链接。

**① 面板美术与落点（C# 实机）**：C# `NPCDropDialog` 构造期写 `Index = 392; Library = Prguse`，
但 `BeforeDraw`（`NPCDialogs.cs:1743-1745`）改写为 **`Prguse2[351]`（176x147）**，
`Location = new Point(264, GameScene.Scene.NPCDialog.Size.Height)` ⇒ 期望 **(264,224)**。
全屏模板匹配实测：`Prguse2[351]` 最佳落点 **@(264,224)**（不符率 0.2482 —— 面板上叠着修理表单的
卡槽/按钮/文字，所以比纯面板高；**位置**这一维是逐像素对上的）。

| 侧 | 位置依据 |
|---|---|
| C# | 实机模板匹配 **@(264,224)** |
| 本端 | `sell_panel.rs`：`DIALOG_X = 264.0` + `Location = (264, GameScene.Scene.NPCDialog.Size.Height)`（与 C# `BeforeDraw` 同一公式，`NPCDialog`=`Prguse[995]` 440x224 ⇒ 224） ⇒ **(264,224)** |

⇒ 位置一致。**本端实机这一半未采集**：本端服务器在那一格附近没有可点 NPC（`nearby` radius 8/20/60
连续三次 `count=0`；最近的 `Merchant_Bull` 页面只有 `View/Ask/Close`、没有修理入口），
要采得先在本端世界里找到带 `@Repair`/`@SRepair` 的 NPC —— 如实记为未采集，不推数。

**② 顺带一个夹具坑：点链接前先看清**哪条是哪条**。同一页的两条黄字带
（`x13..51,y92..100` / `x13..43,y128..136`）不是"View + Repair"，而是
**`Special Repair Weapon` + `Close`** —— 本轮先点了第二条，结果是**把 NPC 窗关掉**（后面那帧
`Prguse2[351]` 自然找不到）。判据：点完要**同时**看目标窗有没有出现**和** NPC 窗还在不在；
只剩"没出现"时先怀疑点错链接，而不是判"原版没入口"。

**门禁**：本轮只动文档，无代码变化。
### 3.2au 缺口在案：NPC 窗的 **Quest 按钮**在 C# 开的是 `QuestListDialog`（本端画的是任务日记）（2026-09-29）

继续点 NPC 时对着 NPC 窗底部那颗 Quest 按钮查了一遍——**这是一处尚未实现的窗**（本轮只取证在案，不动产品代码）。

**① 按钮本身两端都有、位置也对**：C# `QuestButton` = `Title[530]`（动画钮，10 帧，Hover 284 / Pressed 286）
`Size = (96,25)`、`Location = (172, Size.Height-30)` = **(172,194)**，且**只在 NPC 有可接任务时才 `Visible`**
（`NPCDialogs.cs:165-181` 构造 + `:1006-1020 CheckQuestButtonDisplay()`）。
实机（`BorderVillage/Jane`，map 1 @(284,606)）：全屏模板匹配 `Title[530]` 命中 **(172,194)**、不符率 **0.0138**
⇒ 这一页确实有可接任务。

**② C# 点击它开的是「NPC 侧任务列表窗」**：`QuestButton.Click += (o,e) => GameScene.Scene.QuestListDialog.Toggle()`
（`NPCDialogs.cs:181`）。该窗 `Index = 950; Library = Prguse;`
`Location = new Point(NPCDialog.Size.Width + 47, 0)` = **(487,0)**（`QuestDialogs.cs:34-36`），316x466，
内含标题 `Title[14]` @(18,9)、上下翻页钮 `Prguse[951..953]` @(291,35) 与 `Prguse[957..959]` @(291,83)、
接受/完成钮（`Title[270..272]` / `[273..275]`）、行列表 + **选中任务的详情区**、底部 CLOSE。

**实机**：点 (220,206)（按钮心）后，全屏模板匹配 `Prguse[950]` 命中 **@(485,0)**（不符率 **0.0944**），
同位置 `Prguse[961]` 只有 0.1371 ⇒ 就是 950 这扇（x 差 2px 是模板含透明边的常见偏移）。

**③ 本端现状（缺口）**：同一个 Quest 按钮走 `mgr.toggle(DialogKind::QuestLog)`
（`game/dialogs/npc.rs:449-451`）⇒ 画的是**任务日记** `Prguse[961]` @(192,60)（C# `QuestDiaryDialog`），
**不是** NPC 侧那扇 950。两扇窗**内容也不同**（同帧并排对比）：

| | C# `QuestListDialog`（950 @487,0） | 本端（961 @192,60） |
|---|---|---|
| 顶部 | `Title[14]` 标题条 + 上下翻页钮 @(291,35)/(291,83) | 美术自带的 `QUEST DIARY` + 「任务: 4/20」 |
| 主体 | **单个选中任务**的行 + 描述 + `Tasks:` + `Exp` + `SELECT ITEM` 区 | 按地图**分组**的日记行（带「跟踪」）+「完成任务 …」段 |
| 底部 | `ACCEPT` + `CLOSE` | 只有 `CLOSE` |

⇒ **缺口在案**：`QuestListDialog`（NPC 侧列表窗）本端**未实现**——本端把「可接任务 + 接受钮」并进了日记窗
（`quest_log.rs` #2535 那批）：功能上能接任务，但**窗的身份/位置/画源/布局**都不同。

**④ 下一步的修法（不在本轮做）**：按 C# 加一扇 NPC 侧列表窗（面板 `Prguse[950]` @(487,0)、标题 `Title[14]` @(18,9)、
上下钮 `Prguse[951..959]` @(291,35)/(291,83)、接受/完成钮复用 `Title[270..275]`、单任务详情区 + CLOSE），
NPC 窗的 Quest 按钮切到它（并跟 `NPCDialog.Hide()` 级联隐藏），热键/HUD 那条路仍开日记；做完再按同状态 A/B 复验两扇窗。

**门禁**：本轮只动文档，无代码变化（本端现有行为保持原样）。

> **2026-09-29 补（查实现依赖时发现的更大一层）**：这扇窗**不只是"加个窗口"**——
> C# 的列表内容是 `NPCObject.GetAvailableQuests()`（`Client/MirObjects/NPCObject.cs:390-424`）：
> ① 已接且 `QuestInfo.FinishNPCIndex == 本 NPC` 的（可交付）；② 本 NPC 自己那份 `Quests` 列表里
> `CanAccept` 且没完成的（可接受）。②那份列表来自 **`NPCInfo.Quests`**——而本端共享包
> `ClientNPCInfo`（`SharedRust/src/data/client_data.rs:759-766`）只有 `object_id/name/location/icon/can_teleport_to`，
> **没有 quests，也没有 NPC 的 info 索引**（`ClientQuestInfo.npc_index/finish_npc_index` 是 infra 索引，
> 与运行期 `object_id` 对不上）。⇒ 要做这扇窗，得先在协议/服务端补「这只 NPC 提供哪些任务」
> （或补 object_id → NPCInfo 索引的映射），再在客户端把它渲染出来——**跨端改动**，记在这里当下一批的入口。

> **2026-09-29 再补（动手前逐条核 C#，上述推断被推翻 2/3）**：
> ① **"客户端吃 `S.ObjectNPC.QuestIDs`" 是错的**——全仓 grep `QuestIDs` 只有两处：`Shared/ServerPackets.cs:2859`
> 的字段定义 + `Server/MirObjects/NPCObject.cs:392` 的**服务端写入**；**客户端从不读它**。
> ② 客户端那份 `Quests` 的真实来源是 `GameScene.QuestInfoList.Where(c => c.NPCIndex == ObjectID)`
> （`Client/MirObjects/NPCObject.cs:60` 的 `Load`），即**客户端自己的任务定义表**按 `NpcIndex == 本 NPC 的 ObjectID` 过滤。
> ③ **"本端 `npc_index/finish_npc_index` 是 infra 索引、对不上 object_id" 也不成立**——那是 #2867 之前的状态；
> 现在 `build_client_quest_info`（`ServerRust/src/actors/world/mod.rs:12311-12323`）走
> `quest::quest_client_npc_ids(quest_npc_object_ids(links, 生成出的 NPC), q.index)`，下发的**就是本会话生成出来的
> NPC object_id**（C# `NpcIndex = LoadedObjectID`），e2e 断言见 `e2e.rs:1431-1453`。
> ⇒ 结论：**这扇窗的客户端数据源已经就绪**（`QuestCatalog.infos` 里 `npc_index == npc.object_id` 的那批），
> 不需要新的协议字段；剩下的纯粹是**客户端渲染那扇窗**。

### 3.2av `ObjectNpc.QuestIDs` 不再硬编码 0（包体与原版服务端保真）＋ §3.2au 补记的两处纠偏（2026-09-29）

本轮起手是按 §3.2au 补记说的「先补协议」去做服务端的 `QuestIDs`；**做到一半逐条核 C#，发现那条推断不成立**
（见上一节的 2026-09-29 再补）。服务端这半截改动**仍然保留**——它的价值不是"客户端前置条件"，而是
**包体内容与原版服务端一致**：C# 服务端本来就发真表（`Server/MirObjects/NPCObject.cs:392`），
本端此前恒定 `count=0` 是一处**静默的分歧**。客户端那扇窗**不依赖它**。

**改动**（`ServerRust/src/actors/world/`）：

- `quest.rs` 新增 `quest_ids_for_npc(&quest_npc_links, npc_db_index) -> Vec<i32>`：把世界启动时由脚本
  `[QUESTS]` 页汇总的 `(quest_index, finish) -> [npc db_index]` **反查**成「本 NPC 登记的任务号」，
  去重 + 升序；可接（正数）与可交（负数）两类都算。
- `mod.rs` 的 `build_object_npc_packet[_full]` 尾部从**硬编码 `quest_ids count=0`** 改成写真表
  （`ObjectNpc.quest_ids`）。4 个调用点都补上：首生 `spawn_npcs_and_monsters`（走 `SpawnContext.quest_npc_links`）、
  进场/换图重放 `send_map_spawns_to_session`（新增形参，3 个调用点：`map_sync.rs` 1 处 + `session.rs` 2 处）、
  征服旗子（传 `&[]`——旗子不是 NPC 脚本，本就没有任务）。

**C# 依据（服务端侧）**：`Server/MirObjects/NPC/NPCScript.cs:685-720 ParseQuests` 把 `[QUESTS]` 页登记进
`NPCObject.Quests`；`Server/MirObjects/NPCObject.cs:383-394 GetInfo` 把它当 `QuestIDs` 下发。
**消费方：没有**——C# 客户端不读这个字段（grep 证实，见上一节再补②）。

**验证**：`ServerRust` 门禁——`cargo test` **858 passed / 0 failed**（含本轮新增的
`quest_ids_for_npc_is_sorted_dedup_and_covers_accept_and_finish`、`test_build_object_npc_packet_carries_quest_ids`，
以及改写成传 `&[]` 的 `test_build_object_npc_packet_full`）；`cargo fmt -- --check` 干净。
（本机 `cargo clippy --lib -- -D warnings` 会红 1 条，但**不在本轮文件里**——`src/gate/actor.rs:2598
clippy::chunks_exact_to_as_chunks`，是本地 rustc/clippy **1.98.1** 的新 lint；CI 钉的是 **1.95.0**，与本改动无关。）

**仍未做（缺口维持 §3.2au 原状，但入口变清晰了）**：

- 客户端 `Client-Bevy/src/network/packets/handle_world.rs:206-217` 的 `ObjectNpc` 分支**仍把 `p.quest_ids` 丢掉**。
  按上面的纠偏，**这条路本来也不必接**——要接的是 `QuestCatalog.infos` 按 `npc_index/finish_npc_index == object_id` 过滤；
- `QuestListDialog`（面板 `Prguse[950]` @(487,0)）本端仍未实现（窗身份/位置/画源见 §3.2au ①②③）。
  数据源就绪（#2867 的 quest info 已带 object_id），**下一批可以直接做窗**，不必再动协议；
  可复用本端 `quest_log.rs` 已有的 `QuestMessage`/`QuestRewards`（坐标 `QUEST_MSG_ORIGIN`/`QUEST_REWARD_ORIGIN`
  本就是从 `QuestListDialog` 抄的），差的是**面板身份（950 @487,0）、行列表来源、接受/完成钮的窗内归属**。

### 3.2aw NPC 侧任务列表窗**开出来**了：新增 `DialogKind::QuestList`（`Prguse[950]` @487,0）——§3.2au 的窗口缺口收口（单元①）（2026-09-29）

§3.2au 记的缺口（NPC 窗 Quest 钮在 C# 开 `QuestListDialog`、本端却开任务日记）本轮**按 C# 补了窗**，
并把它从「日记窗的别名」拆成独立 `DialogKind::QuestList`。本单元收口**窗的身份/位置/行列表来源/入口/级联**；
消息区+奖励区+接受/完成钮留单元②（见文末）。

**逐条对 C# 的布局**（`Client/MirScenes/Dialogs/QuestDialogs.cs:15-250`）：

| 元素 | C# | 本端 |
|---|---|---|
| 面板 | `Prguse[950]` 316x466，`Location = (NPCDialog.Size.Width + 47, 0)` | `quest_list.rs` `LIST_PANEL`/`LIST_SIZE`/`LIST_POS`（用 `npc::PANEL_W`=440 算出 **487**，不写死） |
| 标题 | `Title[14]` @(18,9) | 同 |
| 关闭 | `Prguse2[360..362]` @(289,3) | 同（复用 `quest_log::CLOSE_POS`） |
| 帮助 | `Prguse2[257..259]` @(266,3) → `HelpDialog.DisplayPage` | 同（开 `DialogKind::Help`） |
| 上/下翻页 | `Prguse[951..953]` @(291,35) / `Prguse[957..959]` @(291,83) | 同（含 C# 的「非首/末行则移选中，否则整表翻页」两条分支） |
| 行 | `QuestRow` x5 `Location=(9, 36 + i*19)`、`Size=(200,17)` | 同（`LIST_ROW_*`），未选中无高亮 |
| 选中高亮 | `Prguse[956]` @ 行内 (25,0) | 同（`QuestListRowMark`） |
| 任务计数 | `_availableQuestLabel` @(210,8)，`AvailableQuestList` = 「可接任务列表：{0}」 | 同（`available_quest_label`） |
| 离开 | `Title[276..278]` @(205,436) | 同 |

**行列表来源**（这轮纠偏的直接落点）：`NPCObject.GetAvailableQuests()`（`Client/MirObjects/NPCObject.cs:390-424`）
——① 已接且 `FinishNPCIndex == 本 NPC ObjectID`（可交付）；② 本 NPC 提供（`NPCIndex == ObjectID`）且 `CanAccept`。
本端实现为纯函数 `npc_available_quests(catalog, log, npc_object_id, level, class)`，直接读 `QuestCatalog.infos`
（`npc_index`/`finish_npc_index` 自 #2867 起就是**本会话 NPC object_id**）——**不需要 §3.2au 补记设想的协议改动**。

**入口与级联**（照 C# 逐条）：

- NPC 窗 Quest 钮 `mgr.toggle(DialogKind::QuestList)`（`NPCDialogs.cs:181` `QuestListDialog.Toggle()`）；
  热键/HUD 那条路仍开日记 `QuestLog`（`QuestDiaryDialog` 是另一扇窗）。
- `CLOSEALL_NPC_CASCADE` 里的 `QuestLog` 换成 `QuestList`——C# `NPCDialog.Hide()` 的级联表
  （`NPCDialogs.cs:1026-1038`）只有 `QuestListDialog`，**日记窗不在里面**（此前本端没有这扇窗、用日记顶替，
  注释里写着「QuestListDialog→`QuestLog`」，本轮按真身改正）。
- `CLOSEALL_DIRECT`（ESC 关窗表）加 `QuestList`（C# `GameScene.cs:692` 的 Closeall 里有它）。
- 本窗 Hide 连带 `NPCDialog.Hide()`（`QuestDialogs.cs:242-247`）：本端在关闭/离开钮里同时置 `npc.visible=false`，
  交给既有的 NPC 级联边沿处理。

**配套登记**（漏登记会被门禁拦下）：RPC 名 `quest_list`（`control.rs` 的 `parse_dialog_kind`/`has_rpc_mapping`/
`RPC_KIND_NAMES`）、交互巡回 `interact_gate.rs` 的 `SWEEP_KINDS`、`interact_sweep_manifest.json` 的 `sweep`
（41→42 项，`docs/DELIVERY.md` §5.3 标题同步）。本窗有标准关闭钮 `Prguse2[360..362]`，进「点 X 关」跑法。

**验证**：`Client-Bevy` `cargo test --lib` **874 passed / 0 failed**（新增 7 条：可接表来源三态、去重、
已完成剔除、行版式、`bind` 只在换 NPC 时重置、`StartIndex` 夹取、计数文案）。
**实机 A/B 未采集**（本轮只到离线门禁）——按仓库规矩这里写明：**未采集**，下一步用 `dialog open quest_list`
配 §3.4b 的 `win_locate.py` 实测面板落点，再按 §3.2 系做两端同状态对表。

**单元②（未做，缺口保持开着）**：消息区（`QuestMessage` @(10,135) 280x160 + 上 `Prguse2[197..199]` @(292,136) /
下 `[207..209]` @(292,282) / 位置条 `[205/206]` @(292,149)）、奖励区（`QuestRewards` @(5,307) 313x130）、
接受 `Title[270..272]` @(40,436) / 完成 `Title[273..275]` @(40,436)。这三块本端 `quest_log.rs` 已有同坐标实现
（在**详情窗**上），单元②把它们按「父窗 = QuestListDialog」搬过来。
另：NPC 窗 Quest 钮的**显隐**目前仍是文本启发式（脚本行含「可接受任务」），C# 是
`npc.GetAvailableQuests().Any()`（`NPCDialogs.cs:1006-1020`）——留到单元②一并按目录口径改
（改它要动 `npc_ui_system` 的参数预算，见该函数 16 参注释）。

### 3.2ax NPC 侧任务列表窗**内容齐了**（单元②）：消息区 + 奖励区 + 接受/完成钮；任务钮显隐改按 C# 目录口径（2026-09-29）

§3.2aw 收口了窗的身份/位置/行列表；本轮把 C# 同一扇窗的**下半截内容**补齐，并把 NPC 窗任务钮的
显隐判据从文本启发式换成 C# 的口径。

**① 消息区**（C# `QuestMessage`，`QuestDialogs.cs:180-193` 构造 / `:1030-1290` 行为）：

| 项 | C# | 本端 |
|---|---|---|
| 控件落点/尺寸 | `Location = (10, 135)`、`Size = (280, 160)` | `LIST_MSG_ORIGIN` / `LIST_MSG_W` |
| 行数 | `new QuestMessage(..., 10)` | `LIST_MSG_LINE_COUNT = 10` |
| 上下滚 | `Prguse2[197..199] @(292,136)` / `[207..209] @(292,282)` | 同（复用详情窗口径：显式 `Size` 被 `AutoSize` 顶掉 → 12x12 图头） |
| 位置条 | `Prguse2[205/206] @(292,149)`，`PosMinY=149 / PosMaxY=263` | 同 |
| 行模型 | `UpdateQuest` + `AdjustDescription`（`:1142-1213`） | 复用 `quest_message_lines`（与详情窗同一支纯函数） |
| 标题行 | 首行黄、`{Tasks}/{Progress}/{QuestReturn}/{TimeLimit}` 行加粗缩进 15、行前圆点 `Prguse[919] @(x+5, y+5)`、每标题行 `adjust += 5` | 同口径（缩进/`adjust`/圆点照抄；**不加粗**——本端没有同字体的粗体档，留作已知差异） |
| 滚轮 | `QuestMessage_MouseWheel`（`:1082-1098`，含「末行钳位用 `Count-1`」的原版怪癖） | 复用 `quest_msg_wheel_top_line`，光标须在消息区矩形内 |

⚠️ **位置条的 y 算法不能直接复用** `quest_log::quest_msg_bar_y`：那支内部吃的是**详情窗**的
`PosMinY/MaxY`（46/263 系）。本窗是 149/263，故在 `quest_list.rs` 另写 `quest_list_msg_bar_y`
（同一条 C# 公式、不同常量），并单独写了单测。

**② 奖励区**（C# `QuestRewards`，`:1396-1530`）：原点 `(5,307)` 与详情窗**完全相同**，
故直接复用 `QUEST_REWARD_ORIGIN`/`reward_cell_offset`（`(i*45+15, 24|89)`）与
`quest_reward_offsets`（经验/金币/信用三列左移链）、`quest_reward_visible_for_gender`：

- 顶行：经验 `Prguse[966]@(10,2)` / 金币 `[965]@(100+Δ,2)` / 信用 `[2447]@(190+Δ′,2)` + 数值标签；
  区标题 `Title[17]@(20,66)`。
- 固定排**不做**性别过滤、可选排过滤（C# `:1533-1553`，固定排那行的 `FilterRewards` 被注释掉）；
  可选排点击=多选一，存的是**未过滤下标**（`SelectedItemIndex`），与 `C.FinishQuest` 的
  `selected_item_index` 语义一致。
- 格子底：固定格恒 `Prguse[989]@(x,y-1)`；可选格仅选中时 `Prguse[979]@(x,y-5)`；
  物品图居中偏移 `((40-w)/2,(32-h)/2)`（`QuestCell.DrawControl`，`:1690-1696`）。

**③ 接受/完成钮**（C# `:72-143` 构造、`:402-425` `ReDisplayButtons`）：
`Title[270..272]`（接受）与 `Title[273..275]`（完成）**同落点 (40,436)**，靠显隐二选一；
接受 = `!Taken && CurrentQuests.Count < MaxConcurrentQuests`，完成 = `Completed`。
点击守卫照抄：未接不发，未完成不发；有可选奖励未选时**不发送** `FinishQuest`
（C# 此处弹 `MirMessageBox(YouMustSelectRewardItem)`——本端只拦包 + 记日志，
**提示框未做**，见文末）。

**④ NPC 窗任务钮显隐改按 C# 目录口径**（`NPCDialog.CheckQuestButtonDisplay`，`NPCDialogs.cs:1006-1020`）：
由「脚本行含『可接受任务/可完成任务』」改为 `npc_available_quests(...).Any()` 的等价实现——
即上一节 §3.2aw 那支纯函数。顺带补了 `npc_object_id == 0` 的守卫（C# `MapControl.GetObject(0) == null`
→ 钮不显示；不挡会把「无 NPC 关联、服务端下发 `npc_index=0`」的任务算成本 NPC 的）。
`npc_ui_system` 已到 Bevy 的 16 参上限，故任务目录/日志/玩家/关闭钮查询打包成 `NpcQuestAccess`。

**验证**：`cargo check`（lib+bin）0 error；`cargo test --lib` **879 passed / 0 failed**（本轮 +5 条：
消息行模型 4 态、`ReDisplayButtons` 5 态、奖励格落点、位置条钳位、`npc_object_id=0` 守卫）；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**，其中
`inventory_bigmap_constants` 新增本窗**全套纯几何断言**（面板/行/消息区/钮/奖励格，无 `Data/` 也跑，
避免 `require_assets!` 在 CI 上整段跳过）。

**实机 A/B：未采集**（同 §3.2aw）。`dialog open quest_list` + `win_locate.py` 的面板落点、消息区/奖励区
逐块差异占比，留到下一次解锁窗口时补——本批不推数。

**仍未做（已登记，不留在暗处）**：

1. 消息区**彩色叠加段/怪物/NPC/物品链接**（详情窗在 #2810 单元②已实现）——列表窗暂按纯文本；
2. 「必须先选奖励物品」的 `MirMessageBox` 提示框（本端只拦发包）；
3. 位置条**拖动**（`PositionBar_OnMoving`）——本端只跟随 `TopLine` 移动，未做拖拽反向写回；
4. 标题行**加粗**（无同字体粗体档）。

### 3.2ay NPC 侧任务列表窗的**本端实机取证**（§3.2aw/§3.2ax 的「未采集」收口第一半）（2026-09-29）

§3.2aw/§3.2ax 都写着「实机 A/B 未采集」。本轮用**常驻 control RPC**把本端实跑了一遍，把**本端那一半**
补齐（**两端同状态逐像素对表仍需原版客户端**，见文末）。

**夹具**（本次实跑，可复现）：

```powershell
# 1) 用带 Data 的主检出重建客户端（worktree 没有 Data/，必须在主检出跑）
cd Client-Bevy; (Get-Item build.rs).LastWriteTime = Get-Date; cargo build
# 2) 起客户端：mock 网络 + 自动进游戏 + 逻辑缩放 1（与原版同尺度）
#    注意 PATH 必须带 msys64\ucrt64\bin，否则 0xC0000135（缺 DLL），进程静默退出
$env:PATH='D:\toolchains\msys64\ucrt64\bin;'+$env:PATH
.\target\debug\client_bevy.exe --mock --auto-enter --ui-scale 1 --window-title questlist-verify
# 3) 驱动（JSON-RPC over TCP 127.0.0.1:9000）
pwsh tools/acceptance/rpc.ps1 -Method dialog      -Params '{"kind":"quest_list","action":"open"}'
pwsh tools/acceptance/rpc.ps1 -Method dialog_rect -Params '{"kind":"quest_list","fallback":"root"}'
pwsh tools/acceptance/rpc.ps1 -Method screenshot  -Params '{"path":"<绝对路径>.png"}'
py -3.12 tools/acceptance/csharp_golden/win_locate.py --shot <帧> --lib "Data\Prguse.Lib" --index 950 --expect 487,0 --tol-px 3
```

**实测结果**（master `3e718401c` 构建，`--ui-scale 1`）：

| 判据 | 实测 | C# 期望 | 结论 |
|---|---|---|---|
| 根面板矩形（`dialog_rect` fallback=root） | `(487, 0, 316, 466)` | `Location = (NPCDialog.Width+47, 0)` = (487,0)、316x466 | **一致** |
| 关闭钮中心/尺寸 | `tf=(788.0, 13.5)`、`24x21` | `Prguse2[360..362] @(289,3)` 原生 24x21 ⇒ 中心 (788,13.5) | **一致** |
| 全屏模板匹配 `Prguse[950]` | 最佳落点 **(487,0)**，不符率 **0.0231**（3376/146323 不透明像素） | §3.2au 的原版实测：**(485,0)**、不符率 0.0944 | 本端落点是**公式精确值**；原版那 2px 与更高的不符率来自原版整帧里的其它窗/内容（该节已注明） |
| 点关闭钮是否真关 | `click(788,13)` → 命中 `4421v0 24x21 [root=QuestList]` → 复查 `dialog_rect` 返回 `close button not found`（窗已隐） | `closeButton.Click += Hide()`（`QuestDialogs.cs:243`） | **一致** |
| 运行期健康 | 日志无 `B0001`/`panic`/`ERROR`/`WARN`；仅 `control dialog: QuestList -> open=true` + 置顶 z 记录 | — | 新增的两个系统（行列表/消息+奖励）无查询冲突 |

**仍然未采集的部分（如实留痕，不推数）**：

1. **内容级两端对表**（消息区 10 行文本、奖励区图标/格子、接受/完成钮的显隐态）——本次夹具是 mock 网络，
   其任务定义 `npc_index = 0`（`network/mock/mod.rs:2117-2120`），按 C# 语义（`MapControl.GetObject(0) == null`）
   **本就不该**出现在任何 NPC 的列表里，故本端窗口是空列表、消息区与奖励区按代码隐藏——**没有内容可比**。
   要采这一段，需要一次**带任务数据的真实会话**（连 Rust 服务端 → 走到有任务的 NPC → `npc_call` → `dialog open quest_list`）
   或给 mock 补一份绑定到某 NPC object_id 的任务定义。
2. **原版那一侧的同一状态帧**（`QuestListDialog` 在 (487,0) 的内容级 A/B）——需要 C# 沙箱 + 解锁窗口，
   本批未做。

### 3.2az NPC 侧任务列表窗**内容级取证**：实机挖出「行不动」并修好；mock 夹具 + `quest_list_probe`（2026-09-29）

§3.2ay 把「窗的身份/位置」验到了；剩下的缺口是**内容**——行、消息区、奖励区、接受/完成钮在实机上到底画没画。
本轮把这段补齐，并且**真挖出一个缺陷**。

#### ① 取证夹具（新增，可复现）

难点：这扇窗的内容**按 C# 语义本就该空**——没有 NPC 会话（`GetObject(0) == null`）或任务没绑到该 NPC 时，
列表就是 0 行。所以"截图里没字"分不清是"空得对"还是"根本没画"。为此加了三件东西：

| 件 | 内容 | 为什么这么加 |
|---|---|---|
| mock 任务定义 | `mock_npc_quest_info()`（`network/mock/state.rs`）：`index=2, npc_index=npc_index=finish_npc_index=MOCK_QUEST_NPC_ID(4242)`，9 行描述（含 `{文本/颜色}` 段与 `[ITEM:..]` 链接）、1 件固定 + 2 件可选奖励 | 既有 demo 定义 `npc_index=0` 不属于任何 NPC；没有"绑到某只 NPC 的定义"就永远取不到内容 |
| `[@QUEST]` 路径下发 | `CallNPC` 的 `[@QUEST]` 页里也发一次同一份定义（`Magic` 分支原本就发，两条路径同源） | 取证不必先施法：`npc_call {object_id:4242,key:"[@QUEST]"}` 一步进状态 |
| `quest_list_probe` RPC | 只读返回 `bound_npc / npc_object_id / catalog_infos / catalog_npc_indexes / selected / start / top_line / selected_reward` | 这扇窗的"空"有多种成因，截图分不清；probe 直接给状态真值（同 `quest_probe`/`npc_rows` 的路子） |

#### ② 实机挖出的缺陷：**行标签永远不可见**（已修）

probe 说 `catalog_infos=1 / bound_npc=4242 / selected=2`，但行区**亮像素 0**——`quest_list_ui_system`
只更新了行文本、**从没把行的 `Visibility` 置为 `Visible`**（行槽在 spawn 时是 `Visibility::Hidden`）。
C# 里 `RefreshInterface` 每行是**新建控件**（按 `Quests.Count` 逐个 `new QuestRow`），本端是固定 5 槽位，
所以显隐必须显式写——漏写就是"窗开着、一行字都看不见"。

**修复**：行渲染循环同时写 `*vis`（越界槽位清空并隐藏）。修后同一夹具下：row 0 文本带亮像素 **159**、
相邻空槽 **0**（第二行确实隐藏）。

#### ③ 内容级实测（master `dca87c36b` + 本 PR，`--mock --auto-enter --ui-scale 1`）

先 `npc_call {"object_id":4242,"key":"[@QUEST]"}`（NPC 窗开、mock 下发该 NPC 的任务定义），
再 `dialog open quest_list`，截图后按区域读像素：

| 区域 | 实测（亮像素 >180） | 判据 |
|---|---|---|
| 行 0（任务名 `(9,36)` 起 200x18） | **159** | 行文字已画（修复前 0） |
| 行 1 空槽 | **0** | 越界行确实隐藏 |
| 消息区 10 行 `(497,135)` 280x160 | **1875**（首行 127 / 第 2 行 157） | 首行黄标题 + 正文都画出来 |
| 奖励区 经验图标 `Prguse[966]` @(502,309) / 金币 `[965]` @(592,309) | **35 / 35** | 偏移链按 `reward_exp>0` 走零偏移 |
| 奖励区标题 `Title[17]` @(512,373) | **87** | 区标题已画 |
| 固定格 0 / 可选格 0 物品图 `(505,329)` / `(505,394)` | **20 / 20** | 两排奖励格都有物品图 |
| 接受钮 `(527,436)` 68x25 | **76** | `ReDisplayButtons`：未接+未满员 ⇒ 接受钮可见 |
| 选中高亮 `Prguse[956]` | 与"空列表"那帧逐列 diff：变化区间 **x 496..772**、右段(x≥610，无文字)**982 px** | 高亮确实画了（252 宽，起于 521=面板 487+9+25，与 C# `SelectedImage @(25,0)` 对齐）。注：`win_locate.py` 对**这一格**模板匹配失败（0.36）——它拿 Lib 原始像素比屏幕，而这格是**带 alpha 叠在面板美术上**的，像素值必然被底色改写；判据改用"与空列表帧的差分区间" |

#### ④ 顺带发现 + 一处**防御性**改动（不是本轮空目录的原因，必须说清楚）

- 观察：`--mock --auto-enter`（不施法）时 `catalog_infos=0`；查证发现 mock 的任务定义发送挂在
  **`ClientPacketIds::Magic`** 分支（施法才发），`--quest-data-test` 之所以有数据正是因为它在游戏内施法。
  ⇒ **不是丢包**，是本轮最初"登录串丢包"的推断错了，已在 §3.2az 更正。
- 但**真服务端**把任务定义放在 **StartGame 那一串**里（`ServerRust/src/actors/world/session.rs:1344`
  `send_quest_infos`，注释写着"必须在 NPC 生成之后下发"）。而客户端是：写侧
  （`network_system`，**无状态门**）↔ 读侧 `quest_log_server_events` 原本挂着
  `run_if(in_state(AppState::Game))`——**生产/消费两侧门控不一致**，登录串在 Select 帧被解码时
  `ServerEvent::QuestInfo` 就没人读、2 帧后过期。这是**潜在**时序风险，**未在实机复现**
  （本轮复现的是 mock 不施法不发数据，与门控无关）。
- 处置：摄入侧去掉状态门（与写侧对齐）+ 新增单测
  `quest_info_is_ingested_outside_game_state`（**阳性对照实做**：把门加回去该测试立即 FAIL）。
  该系统的参数只有 `MessageReader<ServerEvent>` + 两个插件期就 init 的资源，任何状态跑都安全；
  渲染/交互那几支仍留在 Game 门内。**如实标注：这是防御性收敛，不是已复现缺陷的修复。**

**门禁**：`cargo check`（lib+bin）0 error；`cargo test --lib` **880 passed / 0 failed**；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。

### 3.2bn §3.2ar ③ 的「本端修理窗实机未采集」**收口**：`Prguse2[351]` @(264,224)＋两颗钮逐像素 0；顺带两处 C# 逐行对表修正（2026-09-30）

§3.2ar ① 把 C# 侧那扇修理/特修面板（`NPCDropDialog`，`Prguse2[351]` 176x147 @(264,224)）实机量过了，
③ 留着「**本端这一半未采集**」：当时本端沙箱那一格附近没有可点 NPC（`nearby` radius 8/20/60 连续 `count=0`）。
本轮换路子收口——**不需要世界里真有修理 NPC**。

**① 为什么可以绕过"世界里找 NPC"**：Rust 服务端的修理/特修**不是** C# 那种独立包，而是
`NPCGoods` 带 `panel_type`（`ServerRust/src/actors/world/npc.rs:227-234` 的 `EngineNpcAction::Repair/SpecialRepair`
→ `send_npc_panel`，实现见 `ServerRust/src/actors/world/mod.rs:4019-4045`：`NPCGoods{ list: [], panel_type, .. }`），
客户端也只认这三种（`handle_npc_items.rs` 的 `Sell | Repair | SpecialRepair` 分支 → `NpcSellPanel`）。
⇒ **mock 回一发同款包就能开窗**。夹具补了两页（`network/mock/mod.rs`，与既有 `[@SELL]` 同款）：

```powershell
client_bevy.exe --mock --auto-enter --ui-scale 1 --control-port 9000
# RPC npc_call 4242 带 key：
#   {"object_id":4242,"key":"[@REPAIR]"}   → PanelType::Repair
#   {"object_id":4242,"key":"[@SREPAIR]"}  → PanelType::SpecialRepair
# 然后 {"method":"screenshot","params":{"path":"…\\ours_panel_srepair.png"}}
py -3.12 tools\acceptance\csharp_golden\win_locate.py --shot <png> --lib Data\Prguse2.Lib --index 351
```

**② 实机结果（本端侧，mock；三帧存 `%TEMP%\golden_sandbox\shots\ours_sellpanel_{sell,repair,srepair}.png`）**

| 目标 | Sell | Repair | SpecialRepair | C# 期望（`NPCDialogs.cs:1740-1760`） |
|---|---|---|---|---|
| 面板 `Prguse2[351]` | **(264,224)** | **(264,224)** | **(264,224)** | `Location = (264, NPCDialog.Size.Height=224)` |
| 确认钮 `Title[290]` | — | — | **(378,286)** 不符率 **0.0000** | `ConfirmButton @(114,62)` ⇒ 面板 +(114,62) |
| 按住钮 `Title[293]` | — | — | **(378,260)** 不符率 **0.0000** | `HoldButton @(114,36)`，这三档 `Visible=true` |

面板本身的不符率 0.153~0.164（同一面板上叠着表单/两颗钮/提示文字，§3.2ar 的 C# 侧同类读数是 0.2482）——
**位置这一维逐值对上，两颗钮逐像素 0**。⇒ §3.2ar ③ 的「本端实机未采集」**结案**。

**③ 顺带两处 C# 逐行对表修正**（都出自把 `NPCDropPanel_BeforeDraw` 逐行核一遍）

1. **`hold_button_visible` 少一档**：C# 有 **6** 处 `HoldButton.Visible = false`
   （`NPCDialogs.cs:1772/1785/1789/1793/1799/**1804**`：分解/降级/重置/精炼/查看精炼/**换婚戒**），
   本端只覆盖前 5 档 ⇒ 补 `ReplaceWedRing` 并加断言。当前 `mode` 取不到这一档
   （`NPCReplaceWedRing` 包在 `handle_progress.rs:1348` 只解码记账），故这是**潜在**不一致；
   但该函数是"C# BeforeDraw 的逐值对表"，少一档会在下次接线时悄悄画错。
2. **`sell_panel_prompt` 把 `SpecialRepair` 并进了 `Repair`**：C# 是**两条**本地化键
   （`Client/Localization/Chinese.json:659-660`：`"Repair": "修理："` / `"SpecialRepair": "特殊修理："`），
   而本端两档同一串 ⇒ **模式区分丢失**（同 §3.2at 对 Disassemble/Downgrade/Reset 的处理：不能落回别档文案）。
   实机可见（`InfoLabel` 在面板内 (30,10)，12px/字）：

   | 对比 | 面板区 diff（修前） | 面板区 diff（修后） | bbox（面板内） |
   |---|---|---|---|
   | Sell ↔ Repair | 191 | 191 | (126,10)-(149,22) |
   | **Repair ↔ SpecialRepair** | **0**（逐像素一样！） | **436** | (126,10)-(173,22) |
   | Sell ↔ SpecialRepair | 191 | 439 | (126,10)-(173,22) |

   bbox 正好是提示文字尾部 2 / 4 个字的位置（"出售"/"修理" → 2 字；"特殊修理" → 4 字），
   即修前 `SpecialRepair` 面板画的就是普通修理的提示。

**④ 门禁**（`Client-Bevy`）：`cargo check --tests` 0 error；`cargo test --lib` **886 passed / 0 failed**
（含 40 窗点 X 关的 `interact_gate::sweep_windows_close_via_standard_close_button`）；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。

### 3.2bo `NPCDropDialog` 的第二处缺口：放了物品后**不报价**（C# 会拼价格）—— 已修（2026-09-30）

§3.2bn 把面板/两颗钮对齐后，把 `NPCDropPanel_BeforeDraw` **再往下读**（`NPCDialogs.cs:1815-1853`）
发现第二处真缺口：**放了物品后 C# 会把「这一单多少钱」拼进 `InfoLabel`，本端从来只画固定提示**。

**① C# 原文（`TargetItem != null` 分支）**：按档把价格拼到 `text` 后面，再拼 `ClientTextKeys.Gold2`
（`Client/Localization/Chinese.json:668` = "金币"）：

| 档 | C# 表达式 | 依据 |
|---|---|---|
| Sell | `TargetItem.Price() / 2`（**uint 整除**） | `:1818-1819` |
| Repair | `TargetItem.RepairPrice() * GameScene.NPCRate` | `:1821-1822` |
| SpecialRepair | `TargetItem.RepairPrice() * 3 * NPCRate` | `:1824-1825` |
| Disassemble | `TargetItem.DisassemblePrice()` | `:1827-1828` |
| Downgrade / Reset | `DowngradePrice()` / `ResetPrice()` | `:1830-1834` |
| Refine / ReplaceWedRing | `Info.RequiredAmount * 10 * NPCRate` | `:1836-1841` |

两个价格函数是 C# `Shared/Data/ItemData.cs:516-563` 的 `Price()` / `RepairPrice()`
（另加 `DisassemblePrice :583` / `DowngradePrice :594` / `ResetPrice :605`）。

**② 本端修法（逐行照抄 C#，含截断语义）**

- `game/dialogs/inventory.rs`：`InvItem` 补四个 C# 报价要用的量——`info_durability`
  （C# `Info.Durability`）、`added_stats_count`（`AddedStats.Count`）、`rental`（`RentalInformation != null`）、
  `awake_level`（`Awake.GetAwakeLevel()`）；一个源头 `network/packets/mod.rs::to_inv_item` 填充。
  新增 `csharp_price/repair_price/disassemble_price/downgrade_price/reset_price`：
  **`(uint)` 向零截断、`Math.Floor` 显式向下、f32 单精度、`uint` 回绕** 全部照抄。
- `game/dialogs/sell_panel.rs`：`InfoLabel` 分两种状态——**空面板**仍是本仓的整句提示；
  **放了物品**换成与 C# 同构的「短标签 + 报价 + 金币」。这不是"另发明一套"：整句提示 + 报价会
  画到面板外面（`InfoLabel` 在面板内 (30,10)、面板只有 176 宽、12px/字 ⇒ 整句就 144px）。
- `network/server_event.rs`：`NpcSellPanel` 补 `rate`（C# `GameScene.NPCRate`，
  `GameScene.cs:264`），`handle_npc_items.rs` 从 `NPCGoods.rate` 填；分解/降级/重置那三条包
  C# 不刷 `NPCRate`（沿用上一次），本端服务端面板包恒发 1.0（`world/mod.rs:4019-4025`）故填 1.0。
  `SellPanelState` 的手写 `Default` 给 `rate = 1.0`（`f32::default()` 是 0，会让报价恒为 0）。

**③ 判据（公式）+ 实机（渲染）**

公式单测用的就是**服务端那两条同源测试的同款夹具**（`ServerRust/src/actors/world/item.rs:6869-6910`）：

| 夹具（price/infoDura/maxDura/curDura/count） | `Price()` | `RepairPrice()` |
|---|---|---|
| 100/50/50/50/3 | 300 | **0**（满耐久，C# 同款测试值） |
| 100/50/50/25/1 | 87 | **13**（C# 同款测试值） |
| 101/50/50/50/1 | **100**（`(uint)(50*1.01)=50` 的截断路径） | — |

实机（本端 mock，`npc_call 4242 [@SREPAIR]` / `[@SELL]` → `inv_select {slot:2}`（木剑，price=10）
→ `click {x:321,y:316}`（面板内投放区中心）→ `ui_nodes_at` 取 `InfoLabel` 节点矩形）：

| 帧 | `InfoLabel` 节点矩形 | 渲染出来的字（放大裁剪目检 + 宽度核对） |
|---|---|---|
| 空面板 | (294,234) **144x15** | `放入物品后点确认特殊修理`（12 字 × 12px = 144） |
| 放入木剑（SpecialRepair） | (294,234) **90x15** | `特殊修理：0金币`（木剑 `info.durability=0` ⇒ C# `RepairPrice()` 恒 0） |
| 放入木剑（Sell） | (294,234) **66x15** | `出售：5金币`（`Price()/2 = 10/2 = 5`） |

放大帧：`%TEMP%\golden_sandbox\shots\ours_sellpanel_price_zoom_{pair,sell}.png`；
整帧：`ours_sellpanel_price_{hint,srepair,sell}.png`。
⇒ 「放了物品就有报价」这一条**实机收口**；**仍未采集**的是**原版侧**同状态帧（要解锁 + 真鼠标，属 §3.2l 那批）。

**④ 门禁**（`Client-Bevy`）：`cargo check --tests` 0 error；`cargo test --lib` **889 passed / 0 failed**
（新增 3 条：公式、报价文案、租用 ×2）；`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**；
本轮改动的 6 个文件 `cargo fmt -- --check` **无差异**（master 其余文件的既有 fmt 漂移 70 处不属本轮，未动）。

### 3.2bp `NPCDropDialog.Confirm` 的**客户端预检**：C# 本地拒绝，本端是**服务端权威**——结论「不照抄」（2026-09-30）

§3.2bo 只补了 `BeforeDraw` 的报价；同一扇窗还有第二处「看起来是缺口」的地方：C# 的
`NPCDropDialog.Confirm()`（`NPCDialogs.cs:1520-1656`）在发包**之前**会本地拦一批条件并直接发系统聊天。
本轮把它逐条对到本端，**结论是：不需要照抄**——本端这些条件都在**服务端**判、而且**都有系统提示**。

**① C# 客户端的本地预检（原文）**

| 档 | C# 本地条件 | 拒绝文案 | 物品去留 |
|---|---|---|---|
| Sell | `Bind.HasFlag(DontSell)` | `CannotSellItem` | **留**（`return`） |
| Sell | `Gold + Price()/2 > uint.MaxValue` | `CannotCarryMoreGold` | 清（`break` → 末尾 `TargetItem = null`） |
| Repair | `DontRepair` | `CannotRepairItem` | 留 |
| Repair | `Gold < RepairPrice()*NPCRate` | `LowGold` | 清 |
| SpecialRepair | `DontRepair \|\| NoSRepair` | `CannotRepairItem` | 留 |
| SpecialRepair | `Gold < RepairPrice()*3*NPCRate` | `LowGold` | 清 |
| Consign | `DontStore \|\| DontSell` | `CannotConsignItem` | 留 |
| Reset | `Info.NeedIdentify == false` 才发（否则静默 return） | — | 留 |
| Refine | 精炼窗有存料 **且** `Gold >= RequiredAmount*10*NPCRate` | `YouDontHaveEnoughGoldToRefine` / `YouHaventDepositedItemsToRefine` | 留 |
| CheckRefine | `RefineAdded != 0` | `ItemHasntBeenRefinedNoChecking` | 留 |
| ReplaceWedRing | `Info.Type == ItemType.Ring` | `ItemIsNotRing` | 留 |

**② 本端对应实现（都在服务端，且都发系统聊天）**

| 条件 | 本端位置 | 实测文案（源码字面量） |
|---|---|---|
| Sell `DontSell` | `ServerRust/src/actors/world/item.rs:4044-4053` | 「该物品无法出售」 |
| Repair `DontRepair` / SRepair `NoSRepair` | `item.rs:4281-4293` | 「该物品无法修理」/「该物品无法特殊修理」 |
| Repair 耐久已满 / 费用 0 | `item.rs:4297-4309` | 「该物品不需要修理」/「该物品无法修理」 |
| Repair 金币不足 | `item.rs:4312-4318` | 「金币不足（需要 N 金币）」 |
| Consign `DontStore/DontSell`（+ 死亡/距离） | `market.rs:1817/1845/1873` | 「绑定的物品无法寄售」等 |
| REPAIRALL 金币不足 | `world/mod.rs:6514-6520` | 「金币不足，修理需要 N 金币」 |

⇒ **差别只有一处：C# 是本地立刻拒（零往返），本端是发出去由服务端拒（一次往返）**——而 C# 自己的服务端
（`Server/MirObjects/PlayerObject.cs` 那侧）同样会校验，所以本端把权威放服务端、客户端不重复实现，
不算口径差；绑定类"物品留在面板里"的行为差异也随之不存在（本端面板在收到服务端拒后仍是原样，因为
客户端已在确认时清了 `target` —— 详见 ③ 的如实留痕）。

**③ 如实留痕（本轮**没有**做、也没推数的部分）**

- `CheckRefine`（`RefineAdded == 0`）与 `ReplaceWedRing`（`Type != Ring`）这两条**客户端预检**在服务端
  的对应提示**没有逐条核到**（refine 那条走 `actors/refine.rs` 的结算路径，本轮只到"定性"）——
  记为**未采集**，留给下一轮（要核就得跑 refine/marriage 的 e2e）。
- **原版侧这些拒绝路径的实机帧未采集**：要在沙箱里点 NPC + 放绑定物，属 §3.2l 那批（需解锁 + 真鼠标）。
- 本端「确认被服务端拒后，面板里的物品是留还是清」这一条**没有单独实机取证**（需要一件绑定物品的 mock
  夹具）；本轮只在源码层确认了 C# 的留/清分支，**不推数**。

**门禁**：本轮只动文档，无产品代码变化。

### 3.2bq 镶嵌面板（`SocketDialog`）第二处缺口：**装备格上的 Ctrl+右键**开不出来、且面板恒贴背包 —— 已修（2026-09-30）

继续按「逐行核 C# 的入口表」往下走，`MirItemCell.OnMouseClick`（`MirItemCell.cs:239-247`）给的是一张
**按 GridType 分派**的表：**Ctrl+右键 → `OpenItem()`**，而 `OpenItem()`（`:363-368`）只放行
`GridType ∈ {Inventory, Equipment}` ⇒ **背包格与装备格都能开镶嵌面板**，且两扇宿主窗不同。

**① C# 真值**

| 项 | C# | 依据 |
|---|---|---|
| 入口 | `OnMouseClick`：`if (CMain.Ctrl) { OpenItem(); break; }`（**不带 Ctrl 的右键**才走 `UseItem()`→装/卸） | `MirItemCell.cs:229-247` |
| 允许的来源格 | `GridType != Equipment && GridType != Inventory` → return | `MirItemCell.cs:363-368` |
| 面板图号 | `Index = 20 + (Slots.Length - 1)`（1..12 孔） | `SocketDialog.cs:95` |
| 关闭钮 | `CloseButton.Location = (Size.Width - 23, 3)` | `SocketDialog.cs:99` |
| 定位（背包来源） | `x = inv.X + (inv.W - w)/2`，`y = inv.Y + inv.H + 5` | `SocketDialog.cs:108-110` |
| 定位（**装备来源**） | `x = char.X + (char.W - w)/2`，**`y = char.Y + char.H + 5`** | `SocketDialog.cs:112-118` |

两扇宿主窗 C# 都 `Movable = true`（背包未设 Location；`CharacterDialog.cs:35`）⇒ 公式读的是**当前**位置。

**② 本端修前（两处缺口）**：只有背包格的 Ctrl+右键（`inv_socket_open_system`），**装备格没有入口**；
而且面板**恒按背包公式**定位 ⇒ 哪怕接上装备格，也会贴到背包下方（y=241）而不是人窗下方（y=385）。

**③ 修法**：`socket.rs` 加 `SocketSource{Inventory, Equipment}` + 纯函数 `socket_origin_for(...)`
（两条公式逐值复刻，`Point` 是 int ⇒ `floor` 整除）；装备来源的原点读**运行期** `Node.left/top`
（`DialogRoot(Character)`）、尺寸读 `Title[504]` 真实值（兜底 264x380）；`character.rs` 的
`char_equip_system` 在右键分支里先判 Ctrl——是则开镶嵌面板（来源=Equipment），否则照旧卸下。

**④ 实机（本端 mock，`--ui-scale 1`）**

```powershell
# 起客户端（mock）→ dialog open inventory / character
# A) 背包来源：Ctrl 按住 + 右键 带孔铁剑（格 6）
key {key:"ctrl",action:"down"}; click {x:249,y:53,button:"right"}; key {key:"ctrl",action:"up"}
dialog_rect {kind:"socket"}      # → (99,241,118x62)
# B) 装备来源：先穿到身上，再对武器格 Ctrl+右键
equip_item {unique_id:9007}; key {key:"ctrl",action:"down"}; click {x:909,y:113,button:"right"}; key {…,"up"}
dialog_rect {kind:"socket"}      # → (833,385,118x62)
```

| 来源 | 实机矩形 | 公式核对 |
|---|---|---|
| 背包（0,0 / 316x236） | **(99,241,118x62)** | `0+floor((316-118)/2)=99`、`0+236+5=241` ✓ |
| **装备**（人窗 760,0 / 264x380） | **(833,385,118x62)** | `760+floor((264-118)/2)=833`、**`0+380+5=385`** ✓ |

日志留痕：`💎 打开镶嵌面板: 带孔铁剑 (2 孔)` 与 `💎 打开镶嵌面板（装备）: 带孔铁剑 (2 孔)`；
不带 Ctrl 的右键仍是 `🛡️ 右键卸下装备 带孔铁剑 (uid=9007)`（没被新分支吃掉）。
帧：`%TEMP%\golden_sandbox\shots\ours_socket_{inv,equip}_source.png`。
（面板 118x62 = `Prguse3[21]`（2 孔），与 C# `Index = 20 + slots - 1` 一致。）

**⑤ 顺带改的三处夹具/口径（都附理由）**

1. **两个入口改读 `resolve_cursor(探针优先, 真光标兜底)`**（`inventory.rs` / `character.rs`）：
   修前读 `window.cursor_position()`，夹具注入的点击**拿不到**（本轮第一遍就是这么失败的）。
   正常游玩无探针 ⇒ 仍读真光标，**行为不变**；与 §3.2bc 那条"改读真实光标"不冲突（那条是拖动系统）。
2. **`key` RPC 补 `{"action":"down"|"up"}`** + 新探针 **`keys_probe`**：Ctrl 这类修饰键要**按住**才谈得上
   Ctrl+右键，而修前 `key` 只能"按下+抬起"、也**无从判断到底按住没有**。`keys_probe` 直接读
   `ButtonInput<KeyCode>`，实测 `false → true → false`。
3. **新夹具 `equip_item {unique_id}`**（与背包双击装备同一条 `C.EquipItem{grid: Inventory, to: 0}` 路径）：
   装备格的取证要"先有穿在身上的物品"，而夹具**拖不动**（按下会起"拖整窗"，实测把背包窗拖走了）、
   **双击也不稳**（第二次点击落到别格）。

> **夹具坑记一笔（这次踩了两个）**：① 本端背包网格是 **8 列**（`GRID_COLS = 8`，与 C#
> `InventoryDialog.Grid = new MirItemCell[8*10]` 同口径）——按"6 列"算会把格 6 算成 (27,86)，
> 实际那是**格 8**；判据用 `ui_nodes_at` / `bag_probe` 给的 cell 号，别靠印象。
> ② `click {drag_to}` 拖**物品**会被 `dialog_drag_system` 当成"拖整窗"（它只认 bevy_ui `Button` 命中，
> 背包格不是 Button）——要挪物品用 `inv_select` + 点击，或本轮的 `equip_item`。

**⑥ 门禁**（`Client-Bevy`）：`cargo check --tests` 0 error；`cargo test --lib` **890 passed / 0 failed**
（新增 `socket_origin_follows_source_grid_like_csharp_show`：两条公式 + 人窗被拖动后仍跟随）；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**；本轮改动文件的 `cargo fmt -- --check` 无差异。

**仍未采集**：原版侧同状态帧（要在沙箱里真鼠标点 NPC 拿带孔物品 + Ctrl+右键），属 §3.2l 那批——需解锁。

### 3.2br 背包「Shift+右键 = 物品链接」按 C# 补上；顺带捞出一条 **P0**：`#2736` 把背包格命中判据的**极性写反**（2026-09-30）

继续核 C# `MirItemCell.OnMouseClick`（`MirItemCell.cs:229-330`）的右键/左键分派表，这一格是
**Shift+右键 = 把物品链接塞进聊天框**（`SetChatText("<名字> ")` + `LinkedItems.Add`），
发送时随 `C.Chat{linked_items}` 带走、由服务端换成 `%名字#uid%`。

**① C# 真值**：`MirItemCell.cs:249-268`
`text = string.Format("<{0}> ", Item.FriendlyName)`；
`ChatTextBox.Text.Length + text.Length > Globals.MaxChatLength(=80, Shared/Globals.cs:17)` ⇒
`ReceiveChat(UnableLinkItemMessageTooLong)` 并 return；否则 `LinkedItems.Add(new ChatItem{UniqueID,Title,Grid})`
+ `SetChatText(text)`（`MainDialogs.cs:703-714`：**追加** + 聚焦）。发送时 `LinkedItems` 整份带走再清空
（`MainDialogs.cs:739-751`）。服务端换标记见 `ServerRust/src/actors/world/mod.rs:10254`（`replace_linked_item_markers`）——
**本端协议/服务端早就支持**，只缺客户端这一半。

**② 本端补法**：`chat.rs` 加 `ChatState.pending_links` + 纯函数 `item_link_text` / `can_link_item`
（`MAX_CHAT_LENGTH = 80`）；`C.Chat` 发送时 `linked_items: std::mem::take(&mut chat.pending_links)`；
`inventory.rs` / `character.rs` 的右键分支按 C# 顺序 **Ctrl（镶嵌）→ Shift（链接）→ UseItem** 接上；
mock 回显按服务端语义把 `<名字>` 换成 `%名字#uid%`（夹具要靠这条断言"链接真发出去了"）。

**③ 实机（本端 mock）**

| 步骤 | 实测 |
|---|---|
| Shift 按住 + 右键 木剑（格 2） | `state`：`chat_input_active=true`、`chat_input_text='<木剑> '`（= C# `SetChatText` 的追加形态） |
| 回车发送 | mock 回显 `[刀客] %木剑#9005%`、日志 `💬 [MOCK] 聊天: %木剑#9005%（链接 1 条）` |
| 右键 金创药（格 4，不带修饰键） | 袋内少一件 + `💊 使用物品 uid=9001` ⇒ **右键使用也恢复了**（见 ④） |

**④ 顺带捞出的 P0：`slot_at` 的锁定判据极性写反（`#2736` 引入）**

核 Shift+右键时先发现"按了没反应"，加 `[diag]` 打印出 `cursor=(101.0,53.0) shift=true slot=None`——
光标明明在**有物品的格 2**上，但 `slot_at` 返回 `None`。根因是这一行（`inventory.rs` 的闭包）：

```rust
// inv_clickable_slot(slot, locked) 的语义是"这一格可点吗"（!is_locked）
inv_slot_at(...).filter(|i| !inv_clickable_slot(*i, locked))   // ✗ 只保留【锁定】格
```

⇒ **所有未锁定格都点不中**：左键选中 / 右键使用 / 双击使用 / 删除模式 / Shift 拆分 / Alt 快速出售
**全部失效**（C# 里这些都在 `MirItemCell.OnMouseClick` 同一入口上）。修法：抽成纯函数
[`clickable_slot_at`]（`filter(|i| inv_clickable_slot(*i, locked))`）+ 单测
`clickable_slot_at_excludes_locked_slots`（未锁→命中格 2；锁住→None；邻格不受影响；窗外 None）。

**阳性对照（落地时实做）**：把 filter 改回 `!inv_clickable_slot(...)` ⇒ 该测试**立即红**
（`assertion left == right failed`）；改回正确极性 ⇒ 892 条全绿。
实机对照：修前右键金创药无 `使用物品`、修后同一步就有（上表第 3 行）。

**为什么此前几轮没发现**：既有夹具走的是 `inv_select`（RPC 直接写 `InvClickState.selected`）与
`ui_nodes_at`（bevy_ui 命中栈），**都绕开了 `slot_at`**；`MirItemCell` 那一格只有真点才走它。

**⑤ 门禁**（`Client-Bevy`）：`cargo check --tests` 0 error；`cargo test --lib` **892 passed / 0 failed**
（+2：链接文案/长度守卫、锁定极性）；`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**；
本轮改动文件 `cargo fmt -- --check` 无差异。

**未采集**：原版侧同状态帧（Shift+右键链接在沙箱里要真鼠标，属 §3.2l 那批）。

### 3.2bs 物品升级的「格特效」：`S.ItemUpgraded` → `Prguse[410..418]` 9 帧 + 音效（2026-09-30）

§3.2br 把 `MirItemCell` 的入口表核完后，顺手核了它的**下游反馈**：C# `GameScene.ItemUpgraded`
（`GameScene.cs:4565-4578`）在替换物品之后会调 `InventoryDialog.DisplayItemGridEffect(item.UniqueID, 0)`
（英雄背包那一路调 `HeroInventoryDialog` 的同名方法）。

**① C# 真值**（`InventoryDialog.cs:445-475`）：`MirAnimatedControl`
`AnimationCount = 9` / `AnimationDelay = 150` / `Index = 410` / `Library = Prguse` /
`Location = cell.Location` / `Loop = false`（播完 `AfterAnimation` → `Dispose`）/
`UseOffSet = true`（绘制点 = 格原点 + 该帧艺术偏移）/ `NotControl = true` / `Blending = true`；
音效 `SoundManager.PlaySound(20000 + (ushort)Spell.MagicShield * 10)`。

**② 本端补法**：`inventory.rs` 加 `ItemUpgradeFxRequest`（消息）+ `ItemUpgradeFxState` + `item_upgrade_fx_system`；
`inventory_events` 的 `ItemUpgraded` 分支在**背包**替换成功时投请求（英雄背包那一路**未接线**，如实记）；
起播时按 uid 去重并推 `ITEM_UPGRADE_FX_SOUND` 到既有音效队列（`ItemUseFeedback.sounds`）。

> ⚠️ **`Spell` 枚举的 +3 坑（本轮踩到并钉住）**：C# `Spell.MagicShield = **43**`，而本端
> `SharedRust` 的 `Spell` 整体是 C# 的 **+3**（`None = 3` vs C# `None = 0`）⇒ `Spell::MagicShield as u8 = 46`。
> 音效 id 必须按 **C# 值**算：`20000 + 43*10 = **20430**` → 文件 `M43-0.wav`（`Sound/` 下确实存在）；
> 若照抄"直接用枚举值"会得到 20460 → `M46-0.wav`——**也是一个存在的文件**，所以错了不会报错、只会悄悄放错声音。
> 单测 `item_upgrade_fx_frames_and_sound_match_csharp` 里用 `assert_ne!` 把这条钉住。

**③ 实机（本端 mock，`--upgrade-test` 触发 `S.ItemUpgraded`，物品 = 木剑 uid 9005 @格 2）**

`ui_nodes_at` 每 ~90ms 打一次探针点 `(107,53)`（9 帧都覆盖的那个点），拿到**整段 9 帧**：

| 观测时刻 | 命中节点矩形（z=61，面板相对） | 对应帧 | 核对（格原点 83,37 + 帧偏移） |
|---|---|---|---|
| t≈0 | (104,50,8x7) | `Prguse[410]` | 83+21=104、37+13=50 ✓ |
| t≈90–180 | (100,46,16x15) | `[411]` | +17/+9 ✓ |
| t≈270 | (96,42,24x22) | `[412]` | +13/+5 ✓ |
| t≈360 | (94,39,28x28) | `[413]` | +11/+2 ✓ |
| t≈450 | (92,38,32x30) | `[414]` | +9/+1 ✓ |
| t≈540–630 | (95,41,24x25) | `[415]` | +12/+4 ✓ |
| t≈720 | (97,43,24x21) | `[416]` | +14/+6 ✓ |
| t≈810 | (100,46,16x15) | `[417]` | +17/+9 ✓ |
| t≈900–990 | (104,50,8x7) | `[418]` | +21/+13 ✓ |
| t≳1080 | **节点消失** | — | 9 帧 × 150ms = 1350ms 播完销毁 ✓（探针滞后约 0.1–0.4s） |

日志：`✨ 物品升级格特效起播 uid=9005`；中段帧截图 `shots\ours_itemupgrade_fx_mid.png`。
**阴性对照**：把 `--upgrade-test` 关掉（不触发 `ItemUpgraded`）时探针点**没有** z=61 节点。

**④ 如实留痕（未做/偏差）**

1. **加色混合没复刻**：C# 该控件 `Blending = true`（加色叠加），本端 UI 图片按普通 alpha 画——
   峰值帧的"泛光"会更淡。属**画法偏差**，不是几何/时序差（帧序、帧位、时长、音效都对上）。
2. **英雄背包那一路未接线**：C# `HeroInventoryDialog.DisplayItemGridEffect` 同样存在，
   本端 `ItemUpgraded` 只判背包（`updated` 为真才投请求）；英雄格特效记为**未实现**。
3. **原版侧同状态帧未采集**（要在沙箱里真鼠标点 NPC 触发升级链），属 §3.2l 那批。

**⑤ 门禁**（`Client-Bevy`）：`cargo check --tests` 0 error；`cargo test --lib` **893 passed / 0 failed**
（+1：帧推进/音效 id/格坐标）；`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**；
本轮改动文件 `cargo fmt -- --check` 无差异。

### 3.2bt 施法音效：`20000 + (ushort)Spell * 10` 全表补齐（原版**施法一直没声音**）（2026-09-30）

§3.2bs 补物品升级音效时发现本端**根本没有施法音效**：`sound.rs` 里只有怪物段与 `SoundList` 常量段，
`combat.rs` 有挥砍/受击/死亡音，但 C# `PlayerObject.cs` 那套 `SoundManager.PlaySound(20000 + (ushort)Spell * 10 [+ k])`
**一条都没接**（施法全程静音）。

**① C# 真值**：施法音效在 `case MirAction.AttackRange2:` + `case MirAction.Spell:` 共用的那个
`switch (Spell)` 里（`PlayerObject.cs:1753-1765` 起），**与 `new Effect(...)` 同一个 case** ⇒
落点就是"施法动作开始的那一刻"。共 **61** 条：多数是 `20000 + (ushort)Spell * 10`，
另有四种变体——`+ 1`、`+ 5`（CounterAttack）、`+ (Gender == MirGender.Male ? 0 : 1)`（**女号 +1**：
BattleCry / CrescentSlash / FlashDash）、固定 `20000 + 139 * 10`（OneWithNature）与
`Spell.GreatFireBall * 10`（FireBounce / MeteorShower 复用大火球的音效）。

**② 补法（走既有生成器，不手抄）**：`Client-Bevy/tools/spell_effects_from_csharp.py` 本来就解析这个文件
（`_object_fx_sound` 已会算 `20000 + (ushort)Spell.X * 10`），本轮加 `parse_spell_cast_sounds()` +
`_spell_cast_sound()`（**不认识的形式直接报错，禁止猜**）+ `render_spell_cast_sounds()`，
生成 `spell_effects.rs` 的 `SPELL_CAST_SOUND`（61 条，`--write` 幂等）。
`sound.rs` 加消息 `PlaySoundRequest` + 消费系统 `play_sound_requests`（给拿不到 `Assets<AudioSource>`
的系统用）；`effects.rs` 的 `SpellCast` 分支按 `cast_sound_id(spell, female)` 投请求。

> ⚠️ **还是那条 `Spell` +3**：音效 id 必须用 **C# 枚举值**算（C# `FireBall = 31` ⇒ 20310 → `M31-0.wav`；
> 本端 `Spell::FireBall as u8 = 34`，照抄会算成 20340 → **`M34-0.wav`（也存在，错得安静）**）。
> 生成器读 `Shared/Enums.cs` 的 C# 值；单测用 `assert_ne!` 钉住。

**③ 只有玩家/英雄施法才放这张表**：C# 该表在 `PlayerObject`，怪物走 `MonsterObject` 的
`BaseSound + n` 那套（本端 `combat.rs` 已有怪物音）⇒ 本端按 `With<Player>` 过滤
（**阳性对照就在同一帧**：mock 同一轮里对象 100=玩家、103=怪物都放 FireBall，
只有玩家那条出音效日志）。

**④ 实机（本端 mock，`--upgrade-test` 施放 FireBall）**

| 事件 | 实测日志 |
|---|---|
| 玩家（object 100）施法 | `🔊 施法音效: FireBall female=false id=20310`（= 20000+31*10 ⇒ `M31-0.wav`） |
| 同一轮怪物（object 103）施法 | **无**该日志（走怪物自己的音效路径，未被玩家表污染） |
| 音效文件 | `Sound/M31-0.wav` 存在（错值 `M34-0.wav` 也存在——所以错了不会报错） |

> 夹具补齐：mock 的 `Magic` 分支以前**只**回怪物那条 `S.ObjectSpell`，本轮让它也回一条
> 施法者自己（object 100）的 `S.ObjectSpell`（真实服务端就是把同图玩家的施法广播回来），
> 否则"玩家施法音效"这条链在 mock 下根本走不到。

**⑤ 门禁**（`Client-Bevy`）：`cargo check --tests` 0 error；`cargo test --lib` **894 passed / 0 failed**
（+1：音效表键/抽样 id/性别位/`assert_ne!` 反例）；`cargo test --test b0001_smoke --test ui_alignment`
**2 + 53 passed**；本轮改动文件 `cargo fmt -- --check` 无差异。

### 3.2bm §3.2bl 的最后留白收口：`MirMessageBox` 的**原版现帧**拿到了（换入口，锁屏也能取）；并钉死 `game_shop` 买钮那条路**取不到帧的两条硬前置**（2026-09-29）

§3.2bl 留的唯一留白是「原版侧**同状态帧**」（沙箱里点商品格买钮后的 `MirMessageBox`）。本轮把它定性收口：
**这一帧在当前沙箱里拿不到**，两条原因都取到了实证；同时用一条**不需要鼠标、也不需要 Alt** 的入口，
把 `MirMessageBox` 的**原版现帧**补上了（面板/按钮几何与"谁弹出来"无关）。

**① 拿不到的真因之一：那个角色买不起（数据，不是点偏）**

`dbtool <沙箱>\Server export` 实测：账号 `333`（角色 女道士 / 范德萨法）**gold = 0、credit = 0**（三个账号全 0）。
按 `Client/MirControls/MirGameShopCell.cs:193-228`：`pType` 只在「勾了对应支付方式 **且** `Item.CanBuyCredit/CanBuyGold`」
时才不是 `-1`；即便 `pType = 1`（金币），还要过 `if (Item.GoldPrice * Quantity <= GameScene.Gold)` ——
金币 0 时对任何**有价**商品恒 false ⇒ 走 `else` **只发系统聊天**（`YouCantAffordSelectedItem`）、**不弹框**。

回扫上一轮全部 12 张买路帧（`orig_buy_confirm / orig_buy_pre / orig_buybox / orig_shop_buy / orig_shop2_buy /
orig_shop_ok / orig_post_buy2 / orig_gold_buy / orig_credit_buy / orig_c3_buy / orig_q0 / orig_q1`）：
`win_locate Prguse[360]` 全屏最优落点**没有一张落在 (284,289)**（最优 0.2799~0.5490）⇒ **与源码判定一致**。
上一轮「点买钮没框」**不是点击没到位**（`Title[778]` 确实从 (359,383) 让位到 (359,543)，说明点击被控件消费）。
⇒ 要拿这条路的帧，得先**给角色金币**（`Server.MirADB`）。dbtool 原来只有 `setpw/setpos`，本轮给它补了
**`setgold`**（用法见 §4；实测 `333` 的 gold `0 → 1000000 → 0` 三段逐值一致），属**数据准备**、
不是判据缺失 —— 解锁工作站后"给钱 + 点买钮"就是一条命令的事。

**② 拿不到的真因之二：抓帧时工作站又锁屏了（环境）**

按 §3.2ag 的硬判据：前台窗口 class = `Windows.UI.Core.CoreWindow`、标题「Windows 默认锁屏界面」、
rect `(0,0)-(2560,1440)`；桌面级 `CopyFromScreen` 恒 `#005495`（§3 的旁证）。
锁屏下**两条鼠标路径都死**（本轮补了带判别力的对照，见 ⑤.2）。

**③ 拿到了什么：`MirMessageBox` 的原版现帧 —— 换入口，锁屏/无鼠标也能取**

`MirMessageBox` 的面板与按钮几何是**构造函数里写死**的（`Client/MirControls/MirMessageBox.cs:23/26/34/42-138`），
与「谁把它弹出来」无关。所以用**登录场景那条自动路径**取帧：`LoginScene.cs:84-89` 的
`_connectBox = new MirMessageBox(AttemptingConnectServer, Cancel)`，`Shown` 里 `Network.Connect()` 后 `Show()`，
**服务端不可达时它常驻**（`:95-96` 每帧刷 "Attempting to connect (n)"；`:163` 只有收到 `ClientVersion` 才 `Dispose()`）。

```powershell
# 沙箱服务端停掉 → 起原版客户端 → 登录场景就会常驻一个 MirMessageBox → F12 取帧
Get-CimInstance Win32_Process -Filter "Name='Server.exe'" |
  ? { $_.ExecutablePath -eq "$env:TEMP\golden_sandbox\Server\Server.exe" } | % { Stop-Process -Id $_.ProcessId -Force }
Start-Process "$env:TEMP\golden_sandbox\Client\Client.exe" -WorkingDirectory "$env:TEMP\golden_sandbox\Client"
# 等 ~25s（登录场景加载）后：
. tools\acceptance\csharp_golden\csharp_client_driver.ps1 -SandboxRoot $env:TEMP\golden_sandbox
Init-CsClient; Shot-Cs connectbox        # → shots\orig_connectbox.png
py -3.12 tools\acceptance\csharp_golden\win_locate.py `
  --shot $env:TEMP\golden_sandbox\shots\orig_connectbox.png --lib Data\Prguse.Lib --index 360
```

实测（`shots\orig_connectbox.png` 与 `orig_connectbox2.png`，两帧数字完全相同）：

| 目标 | 最佳落点 | 不符率 |
|---|---|---|
| 面板 `Prguse[360]`（456x190） | **(284,289)** | **0.0290**（2514/86600） |
| `Title[203]`（Cancel **常态**） | **(644,446)** | **0.0000**（0/1900） |
| 负对照 `Title[200]`（OK 常态）同点 | (644,446) | 0.2558 ⇒ **FAIL**（命中是特异的，不是碰巧） |

**面板残余逐区分解**（同一帧对 `Data\Prguse.Lib[360]` 比：α>32 且 RGB 和差 >60）：86600 个不透明像素里
**不符 2514 = 文案矩形 (panel+35,35,390x110) 1148 + Cancel 钮覆盖区 (panel+360,157,76x25) 1366 +
chrome 其余 0**。⇒ **面板 chrome 逐像素相同**，差的全是「文字」和「压在面板上的那颗钮」。

**④ 这条对表把 §3.2bl 缺的那一格补到什么程度**

- 面板原点：**原版 (284,289) / 本端 (284,289)**（§3.2bl 实机 0.0442）—— 同一个值。
  （456x190 居中 ⇒ `MirMessageBox.cs:26` 的 `((1024-456)/2,(768-190)/2) = (284,289)`；§3.2p 也量过同值。）
- 右下钮槽位：本轮实测 `(644,446) = 面板+(360,157)` **逐像素 0**；而 `MirMessageBox.cs:90-92`（YesNo 的 **No**）
  与 `:134-136`（**Cancel** 变体）是同一个 `Location = new Point(360, 157)` ⇒ 商城里
  `ConfirmPurchaseItemGold` 那个 YesNo 框的**右下钮位置由实测锚定**；左下 Yes 的 `(260,157)`（`:80-82`，
  §3.2p 逐像素 0）与两颗钮的图号也已对过。
- **框内两端对拍在上游已做完**（§3.2p：两侧都切中文 locale、**同一串文案** ⇒ 两钮带 0.0000 / 框体其余 ≈42px /
  整框 8.47% / 文案区 17.00%，残差定性为 **CJK vs GDI 字体度量**）。
  ⇒ §3.2bl 这一格剩下的只是「**商城那条文案**在 C# 侧渲染成什么样」，属**未采集**（受 ①② 双重前置挡住），
  不是几何/画源缺口。

**⑤ 顺带三条修正（下一轮直接用）**

1. **锁屏下「注入 Alt」不成立**：`CMain.Alt` 只由 `e.Alt` 赋（`Client/Forms/CMain.cs:134-139 / 186-191`），
   而注入 `WM_SYSKEYDOWN(VK_MENU)` **不会**让它为真。实测：Alt（sys-down）+`Q` 的行为与**单键 `Q` 完全一致**
   ——两次都开的是 `Quests`（`RequireAlt = 0`，`KeyBindSettings.cs:219`），两次帧里 `Prguse[360]` 的
   全屏最优同为 `(49,339)/0.3785`。⇒ **锁屏时别指望 `Alt+X` / `Alt+Q`（`KeyBindSettings.cs:328/330`）
   这条「非鼠标」的 `MirMessageBox` 入口。**
2. **锁屏下鼠标两条路径都死的判别力对照**：`SetCursorPos` **能**移动光标（实测 `(1200,400) → (1069,329)`，
   返回 True），但**没有一个鼠标消息到达客户端** —— 用「F9 开背包 + 点关闭钮 (301,13)」作对照：
   `Msg-Click` 后帧差 **62 px**、真实 `Click-Image` 后帧差 **129 px**（都 = 没关）。真因是
   `CMain.MPoint` 只在 `CMain_MouseMove` 里由 `Cursor.Position` 更新（`CMain.cs:176`），锁屏下客户端收不到
   鼠标消息 ⇒ `MPoint` 陈旧 ⇒ `MirScene.OnMouseClick` 的 `ActiveControl.IsMouseOver(CMain.MPoint)`
   （`MirScene.cs:147-165`）打不中。
3. **沙箱账号密码**：本次用 **`333` / `333333`** 登录成功（服务端日志 `User logged in` +
   `女道士 has connected`）。`make_sandbox -Force` 会把**原版 DB** 拷回来 ⇒ 密码回到**原版密码 `333333`**；
   §3.2b 那段 `setpw 333 abbtest123` 只在**显式改过之后**成立（上一轮一直停在登录/选角界面，就是拿旧密码
   `abbtest123` 登的）。

**收尾状态**：沙箱的 `Client.exe` / `Server.exe` 本轮**已停**（不留 7100 占用）；上一轮那批 `orig_*` 帧都留在
`%TEMP%\golden_sandbox\shots\`，本轮新帧 = `orig_connectbox{,2}.png` / `orig_q_*.png` / `orig_t*_*.png` /
`orig_lm_*.png` / `orig_k_inv_*.png`。

**门禁**：本轮只动文档，无产品代码变化。

**仍未采集**：**原版那一侧的同状态内容帧**（需 C# 沙箱 + 解锁窗口；§3.2au 那张是 Jane 的列表，
数据与本夹具不同，只能做区域级对照，不能做逐像素 A/B）。

### 3.2ba NPC 侧任务列表窗单元③：消息区的 `{文本/颜色}` 彩色段与链接（C# `NewColour`/`NewLink`）（2026-09-29）

§3.2ax 明说这条留在后面：列表窗的消息区当时**按纯文本画**——于是 `{比奇老兵/LimeGreen}` 这种标记会**连花括号
一起裸露在屏幕上**（`_` 段名和颜色名都当字面量显示）。本轮按详情窗同一套做好。

**逐条对 C#**（`QuestMessage.NewText`，`QuestDialogs.cs:1215-1290`）：

| 件 | C# | 本端 |
|---|---|---|
| 主体文本 | 去掉 `{文本/颜色}` 标记后的显示串 | 复用详情窗的 `quest_line_overlays()`（与 `quest_line_display_text` 同判据） |
| 彩色段 | 每段一个叠加 `MirLabel`，颜色走 `Color.FromName`（`NewColour`，`:1336-1353`） | 固定池 `QuestListPart::Overlay{slot,seg}`（10 行 × 6 段）+ `known_color()`；未知名隐藏（C# 取到透明色） |
| 链接 | 初值 `Color.Cyan`、`MouseEnter` 转橙 + `ShowTooltipForLink`（`NewLink`，`:1355-1382`） | 同：青色 / 悬停橙 + `TooltipState.update(13, …)`（走 `quest_link_tooltip_lines`） |
| 行内落位 | 段在**显示串**里的字符偏移 → 像素（`est_text_width` + 折行） | 复用 `quest_segment_offset(prefix, size, LIST_MSG_W)`（与详情窗同一度量，含折行行高 `字号×1.2`） |
| 标题行字号 | `NewText` 标题行用 10F 粗体、正文 8F | 复用 `QUEST_MSG_TITLE_FONT_PX/QUEST_MSG_FONT_PX`（加粗仍缺，见文末） |

**实机实测**（`--mock --auto-enter --ui-scale 1` + `npc_call 4242 [@QUEST]` + `dialog open quest_list`，
夹具里第 3 行含 `{比奇老兵/LimeGreen}`、第 5 行含 `[ITEM:1001|力量戒指]`）：

| 判据 | 实测 | 结论 |
|---|---|---|
| 第 3 行绿色段 | `g-max(r,b) > 30` 像素 **207** | LimeGreen 叠加层已画 |
| 第 5 行链接（未悬停） | 青色像素 **231** | `Color.Cyan` 初值正确 |
| 第 5 行链接（`cursor` RPC 落点 570,203） | 青色 **0** / 橙色 **31** | `MouseEnter` 转橙生效 |
| 标记不再裸露 | 该行文本右缘 **x=662 → 591**（缩短 71px ≈ 被剥掉的标记字符） | 花括号/颜色名不再当字面量画 |

**门禁**：`cargo check --lib` 0 error；`cargo test --lib` **880 passed / 0 failed**；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。

**仍未做（列表窗）**：① 链接**点击**（C# 里怪物/NPC 链接点击会走 `[@…]` 导航、物品链接开提示——本端只做了
配色/悬停/提示，未接点击）；② 标题行**加粗**（无同字体粗体档）；③ 位置条**拖动**；④ 「必须先选奖励物品」
的 `MirMessageBox` 提示框（本端只拦发包）；⑤ 原版那一侧的同状态内容帧（需 C# 沙箱 + 解锁窗口）。

### 3.2bb NPC 侧任务列表窗单元③b：位置条**拖动**（并修好 `CursorSource` 拖动态读陈旧探针的问题）（2026-09-29）

§3.2ba 的 ③ 收了。`PositionBar_OnMoving`（`QuestDialogs.cs:1100-1118`）在 C# 里是"跟手"：拖到哪，
`TopLine` 就按 `(PosMaxY-PosMinY)/(行数-行高)` 反算到哪，条本身不吸附。

**实现**：条（`QuestListPart::Bar`，本身就是 `spawn_icon_button` 出来的 `Button`，带 `Interaction`）
在按下态读光标 → 钳到本窗的 `PosMinY..PosMaxY`（**149..263**，不是详情窗的 46/261，
故新写 `quest_list_msg_bar_interval`/`quest_list_msg_top_line_at_bar` 两个纯函数，与
`quest_list_msg_bar_y` 互为逆，单测覆盖往返）。

**过程中挖到一处夹具/实现共同踩的坑**：`click` 驱动（`control.rs:2358-2712`）在 phase 0 会把
**光标探针**写到按下点、结尾再清掉；而本端"探针优先"的命中来源（`CursorSource::pos`）于是在拖动期间
**一直读到起点** ⇒ 条不动（实测日志：`BAR inter=Pressed cursor=Some(785,158)` 三帧，而拖动目标是 y=200）。
C# 读的是真实鼠标位置，故给 `CursorSource` 加 `real()`（只看真实窗口光标）并在拖动分支用它——
**只影响拖动**，悬停/命中仍走探针优先（`#2767` 语义不变）。

**实机实测**（`--mock --auto-enter --ui-scale 1` + `npc_call 4242 [@QUEST]` + `dialog open quest_list`；
夹具描述给到 14 行 ⇒ 消息区 21 行 > 一页 10 行，位置条才出现）：

| 判据 | 实测 |
|---|---|
| 位置条出现 | `win_locate Prguse2[205]` 命中 **(779,149)**、不符率 **0.0000** |
| 拖动（`click {x:785,y:158,drag_to:{x:785,y:200}}`） | `quest_list_probe.top_line` **0 → 5**（= (200−149)/10，interval=(263−149)/(21−10)=10） |
| 条跟手 | 探针列 x=779..791 的非背景像素：拖前只在 **y=149/159**，拖后只在 **y=189/199/209**（条离开原位） |
| 内容真的滚了 | 消息区黄色像素（首行任务名）**198 → 0**（首行已滚出一页） |

**门禁**：`cargo check`（lib+bin）0 error；`cargo test --lib` **881 passed / 0 failed**（+1 拖动往返单测）；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。

**同类残留（已登记）**：任务**详情窗**（`quest_detail_ui_system`）的位置条拖动仍读"探针优先"的 `cursor`，
在自动化夹具下会有同样的"读到按下点"现象——它那支的改法与本节同（一行换成 `real()`），留给下一批；
`#3368` 这扇窗已改。

### 3.2bc 两处口径更正 + 详情窗同源修复：链接**没有**点击行为；粗体受资产限制；详情窗拖动改读真实光标（2026-09-29）

**① 更正：任务消息里的链接，C# 原版「点了什么都不做」。** §3.2ba 把"链接点击"记成"仍未做①"是**错的**。
逐行核 `QuestMessage` 全类（`QuestDialogs.cs:1003-1400`）：`Click +=` 只出现 **2 次**，都在滚动钮上
（`:1042` 上滚 / `:1052` 下滚）；`NewLink`（`:1355-1382`）只挂了 `MouseEnter`（转橙 + `ShowTooltipForLink`）、
`MouseLeave`（回青 + 隐提示）与 `MouseWheel`。⇒ 本端"青色 + 悬停橙 + tooltip、不响应点击"**就是**同口径，
这一项**没有缺口**（记录更正，免得后续再按"缺点击"去实现一个原版没有的行为）。

**② 更正：标题行加粗当前**做不了**，原因是资产。** C# 标题行用
`new Font(Settings.FontName, 10F, FontStyle.Bold)`（`QuestDialogs.cs:1244`），而本端整个客户端只带
**一个字体资产** `Client-Bevy/Assets/AlibabaPuHuiTi-3-55-Regular.ttf`（3-55 = Regular 字重，无粗体档）。
⇒ 除非引入粗体字体资产（或对标题做描边/双层偏移模拟），否则这条只能挂着；已按"资产限制"归档，不再是"待实现代码"。

**③ 详情窗同源修复**：§3.2bb 挖出的"拖动期读到按下点"在**任务详情窗**（`quest_detail_ui_system`）同样存在
（它那支的位置条也用探针优先的 `cursor`）。本轮把它改成 `real_cursor`（只看真实窗口光标），与列表窗
`CursorSource::real()` 同一口径；悬停/命中仍走探针优先（#2767 语义不变）。

**验证**：`cargo check`（lib+bin）0 error；`cargo test --lib` **881 passed / 0 failed**；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。

**未采集（如实留痕）**：③ 的**实机**拖动复验。路线上要先让详情窗拿到 `quest_id`（C# 是点任务日记行 →
`QuestDetailDialog.DisplayQuestDetails`），而本端这条链要先开日记再点行、且行高/组头会让落点依赖数据——
本轮只做了代码同源改造（与列表窗那条已被实机验证的改法逐字相同），**没有**跑通"详情窗拖动"的实机取证，
留到能稳定驱动该链路时补。

### 3.2be 几何表工具的**假绿**修掉：`window_rect_table.py` 补「运行时改写 Location」覆盖；并纠正 §3.2ap ⑤ 的 buff/timer 归因（2026-09-29）

本批不开新窗，修的是"**判据本身**"——两处已登记的工具缺陷。

**① `window_rect_table.py` 对 `NPCAwakeDialog` 的期望值错 → 假绿（§3.2ar ④ 记的）**

工具从 C# **类构造器**里抠 `Location`，而觉醒窗的真值是在 `GameScene` 里**逐实例改写**的：
`Client/MirScenes/GameScene.cs:307` `NPCAwakeDialog.Location = new Point(0, NPCDialog.Size.Height)` ⇒ **(0,224)**。
2026-09-29 全仓 grep `GameScene.Scene.*Dialog.Location = ` 确认：**逐实例改 Location 的只有这一处**。

修法：把原本**内联在 `main()` 里的** `RUNTIME_LOC = {"SkillBarDialog": (0,0)}`（§3.2k 技能栏那条）
提升为模块级 `RUNTIME_LOCATION`，并加进 `NPCAwakeDialog: (0,224)`（两条都写了出处与行号）。

**三向对照（同一份工具、只换输入）**：

| 工具版本 | 喂进去的帧 | 判定 |
|---|---|---|
| 修改前 | `npc_awake=(0,0,360,420)`（本端若又错位到左上角） | **OK（假绿）** ← 这就是 §3.2ar ④ 记的风险 |
| 修改后 | 同上 | **DIFF** ✓ |
| 修改后 | `npc_awake=(0,224,360,420)`（真值） | **OK** ✓ |

**真机端到端**：本轮跑着的客户端 `dialog_rect {kind:'npc_awake',fallback:'root'}` = **(0,224,360,420)**，
喂进修后工具 ⇒ `npc_awake OK`（不一致 0）。旧工具对这一**正确**位置反而会报 DIFF——同一处假绿的另一面。

**② 纠正 §3.2ap ⑤ 的归因：buff/timer 取不到矩形**不是"根节点没登记 kind 标记"

那两扇窗的根**都挂了** `DialogRoot(kind)`：`buff.rs:662` `DialogRoot(DialogKind::Buff)`、
`timer.rs:217` `DialogRoot(DialogKind::Timer)`。真正原因是**取矩形的两条路都要求"根可见"**：

- 关闭钮那条路要从可见的 `DialogRoot` 上的 `CloseButton` 上溯；
- `fallback:"root"` 那条走 `pick_root_rect`，同样只认 `Visibility::Visible` 的根。

而 buff 面板的显隐是 `count > 0 && hovered`（把 C# `Process` 的 Opacity 渐隐实现成显隐）、timer 要
`S.SetTimer` 起始的**服务端计时器**——§3.2ap 那轮的测试角色既没悬停也没有计时器 ⇒ 两扇都 Hidden。

**可复现的取证（buff）**：先把 `cursor` 探针放进面板矩形再取，即可拿到
`dialog_rect {kind:'buff',fallback:'root'}` = **(830, 0, 68, 34)** ⇒ 右缘 **898** = C# `PANEL_RIGHT`，
与 §3.2af 的 Buff 行（右缘恒 898）一致。timer 仍**未采集**（mock 场景没有计时器状态，需 `S.SetTimer`）。

**门禁**：本轮只动测试工具与文档，无产品代码变化；离线跑过 `--src` 与 `--compare`（上表三向 + 真机一次），
`cargo test --lib` / 集成未受影响（与上一批同一份代码）。

### 3.2bf §3.2be 剩的那条「timer 矩形未采集」收口：**(904,538,120,100)**；外加一条**取窗口矩形的时序坑**（2026-09-29）

**① timer 矩形采到了**（§3.2be 里如实记为"未采集"的那条）

配方：计时器窗是**服务端驱动**的（`S.SetTimer` → `TimerState.active`，`TimerDialog.cs:112-127` 倒计时归零即隐），
而 mock 的 `SetTimer` 挂在**施法分支**里 ⇒ 用 `--quest-data-test`（自动施法）触发，再轮询取矩形：

```powershell
.\target\debug\client_bevy.exe --mock --auto-enter --quest-data-test --ui-scale 1
#  然后 200~700ms 一次轮询（窗口只活 5 秒）
pwsh tools/acceptance/rpc.ps1 -Method dialog_rect -Params '{"kind":"timer","fallback":"root"}'
```

实测（5 次采样全同）：`{rx:904, ry:538, rw:120, rh:100, source:"root"}` ——与 C#
`TimerDialog.cs:27-31` 的 `Location = (ScreenWidth-120, ScreenHeight-230)` = **(904,538)**、`Size = (120,100)`
**逐值一致**（本端 `timer.rs:203-215` 的常量与那支测试同值）。日志侧对上：
`⏱️ 设置计时器 id=1 秒=5 类型=1` → `⏱️ [TIMER] 启动计时器 key=1 5 秒 类型=1`。

**② 时序坑：刚显形的那一帧，根矩形会读成 `(0,0,0,0)`（别当成"窗画在左上角"）**

第一轮我只取**第一次** `ok:true` 就收工，拿到的是 `{rx:0, ry:0, rw:0, rh:0, source:"root"}` ——
根此刻已经 `Visible`（所以 `pick_root_rect` 命中、`ok:true`），但**布局还没跑完**（`ComputedNode` 仍是 0）。
第二轮改成连续采样，随后每一帧都是正确的 `(904,538,120,100)`。

⇒ 判据写法：**取窗口矩形要连采几帧**（或等 1 帧后再取），首次 `ok:true` 的 `rw/rh == 0` 视为"布局未就绪"丢弃。
这一条对**所有刚开窗**的 `dialog_rect` 都成立（本批只改文档，不改工具；`pick_root_rect` 的既有单测仍按原语义）。

**门禁**：只动文档；`cargo test --lib` / 集成与上一批同一份代码（未受影响）。

### 3.2bg 拿**原版留下的帧**当靶子：NPC 侧任务列表窗的**行内容**补齐（图标 + `Lv N` + 名字 x=60）（2026-09-29）

§3.2ay/§3.2az 里一直记着"原版那一侧的同状态内容帧未采集"。本轮工作站**已解锁**，本可以重跑沙箱，
结果先发现**帧已经在手边**：`%TEMP%\golden_sandbox\shots\orig_questlist.png`（2026-09-29 13:05:57，
§3.2au 那轮点 Jane 的 Quest 按钮时留下的）——直接拿它当靶子比读源码更快，也省一轮沙箱驱动。

**对表方式**：两帧同状态（NPC 侧任务列表窗开着），面板原点 C# 是 (485,0)、本端是 (487,0)
（§3.2au 的 2px 透明边偏移），故先把 C# 的 x 归一到面板相对坐标再比。

**量到的结构差（真缺口）**：行内内容 C# 有**三件**，本端上一批只画了**名字**、起点还贴在面板内 x=9：

| 元件 | C#（`QuestRow`，`QuestDialogs.cs:916-991`） | 本端（补前） | 本端（补后，实测） |
|---|---|---|---|
| 图标 | `IconImage = Prguse[961+Icon+(Icon>3?15:0)] @ 行内(3,0)` | **无** | `win_locate Prguse[964]` 命中 **(499,36) 不符率 0.0000** ⇒ 面板 12（= 行 9 + 3） |
| 等级 | `RequirementLabel @ 行内(20,0)`，`"Lv " + MinLevelNeeded`（`>0` 才显示） | **无** | 文字亮带落在面板 29..51（= 行 20..） |
| 名字 | `NameLabel @ 行内(60,0) Size=(140,17)` | 贴在行内 (9,0) | 落在面板 69..（= 行 60） |

**原版帧的列分布（同一行带 y=36..53）**：紧邻面板左缘有两段面板美术（面板 2..4、7），
随后 `497..511`（面板 12..26）= **图标**、`518..535`（面板 33..50）= **Lv 文字**、`540..705` = 名字/分隔带。
本端补后：`499..515`（面板 12..28）= 图标、`516..538`（面板 29..51）= Lv、`542..707` = 名字带
⇒ **三段的相对位置与原版逐一对应**（同一行动作，两侧数据不同故字形宽度不同）。

**图标编号是纯函数**：C# `ClientQuestProgress.Icon => QuestInfo.GetQuestIcon(Taken, Completed)`
（`Shared/Data/ClientData.cs:476-531`），本端照抄成 `quest_row_icon()`；图号公式
`961 + (int)Icon + ((int)Icon > 3 ? 15 : 0)` 里的判据是**C# 枚举值**——本端枚举是 C# 值 **+3**
（`None=3…QuestionGreen=56`，`SharedRust/src/enums.rs:619-632`），故先减 3 再套公式，否则整体偏 3 张图。
（本轮夹具 = General + 已接 + 已完成 ⇒ `QuestionYellow` ⇒ `961+3 = 964` ✓ 与模板匹配一致。）

**门禁**：`cargo test --lib` **885 passed / 0 failed**（+3 条：`GetQuestIcon` 九态、
图号公式含 +15 档、`Lv` 文案）；`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。
实机截图与读数为本机 mock 夹具（`npc_call 4242 [@QUEST]` + `dialog open quest_list`）。

**顺带记一条真机夹具坑（B0001）**：给行加「Lv」标签后，新查询与既有的 `Text` 查询**未互斥**，
`cargo test --lib` 直接 B0001 红（`quest_list_ui_system` 里三条 `&mut Text` 查询要两两 `Without`）。
已按 `Without<QuestListRowLevel>` / `Without<QuestListAvailableLabel>` 补齐。

### 3.2bh 「只能靠鼠标到达」批第一批：`game_shop` **分类页签 `Previous/Next`** 两端同状态对表（2026-09-29）

owner 直令那批（§3.2l）的前置都已成立：① 工作站当前**解锁**；② 真实鼠标可用（§3.2ak 已验：真置顶 + 真实光标
能关背包、能点页签，§3.1/§3.2l-b 的「点不动」作废）。本轮取清单第一项：`game_shop` 分类列的
上/下翻页钮 `Prguse2[197..199] @(120,103)` / `Prguse2[207..209] @(120,421)`（`GameshopDialog.cs:99-135`）。

**方法（先翻 `shots` 目录，别急着重跑沙箱）**：§3.2b 那批**键位** A/B 两侧都留了同状态帧
——`shots\orig_win_GameShop.png`（原版，`Y` 开商城）与 `shots\ours_win_GameShop.png`（本端，同 1024x768@scale1）。
直接用 `win_locate.py` 全屏模板匹配这两帧即可，比"再驱动一遍沙箱"省一整轮。

**① chrome：两侧逐像素命中，只差 §3.2f 记的取帧 1px**

| 目标 | 原版帧最佳落点 | 本端帧最佳落点 | 不符率 |
|---|---|---|---|
| 上翻 `Prguse2[197]`（12x12） | **(285,249)** | **(284,249)** | **0.0000 / 0.0000** |
| 下翻 `Prguse2[207]`（12x12） | **(285,567)** | **(284,567)** | **0.0000 / 0.0000** |

两边都把面板原点算到 **(164,146)**（本端 `dialog_rect game_shop` 实测 `rx=164, ry=146, 696x476`）：
164+120 = 284 / 146+103 = 249、146+421 = 567 ⇒ **面板相对位置逐值相同**；原版那 +1px 是 §3.2f/§3.2af
记的「原版取帧客户区比 1024 宽 2px」的**取帧口径**，不是排版差。

**② 行为：本数据下两侧都「无可滚行程」（同 §3.2y/§3.2ah 的定性）**

- 原版：`DownButton` 有守卫 `if (CStartIndex + 22 >= CategoryList.Count) return;`（`GameshopDialog.cs:131-135`）、
  `UpButton` 有 `if (CStartIndex <= 0) return;`（`:110-114`）；沙箱那份商城只有 **10 类** ⇒ 两侧都直接早退。
- 本端：同一守卫（`game_shop.rs:1912` 起 + 边界单测 `:2581-2599`）。实机（mock，3 类）用 `shop_probe` 取证：
  点 (284,249) 与 (284,567) **前后完全一致**（`categories:["","药品","武器"]`、`rows` 两条、`page:0/pages:1`）⇒ **0 行程**。
- ⇒ 结论按 §3.2y 的同类口径记：**「本数据下不可判定行程」**，不是"点不动"、也不是排版缺陷。
  要拿到真行程必须给一侧造 **>22 类**的商城数据（属**数据准备**，不是判据缺失）。

**门禁**：本轮只动文档，无产品代码变化（翻页钮此前已实现并有边界单测）。

### 3.2bd NPC 侧任务列表窗单元④：`MirMessageBox(你必须选择一个奖励物品)`（本窗最后一个用户可见缺口）（2026-09-29）

§3.2ax/§3.2ba 一直挂着这条：C# `_finishButton.Click`（`QuestDialogs.cs:121-141`）在**有可选奖励但没选**时
`new MirMessageBox(ClientTextKeys.YouMustSelectRewardItem).Show(); return;`；本端此前**只拦下发包**，
点完成**什么反应都没有**（玩家不知道为什么不交任务）。

**逐条对 C#**：

| 件 | C# | 本端 |
|---|---|---|
| 触发 | `Reward.SelectedItemIndex < 0 && QuestInfo.RewardsSelectItem.Count > 0`（`:130-137`） | `finish_needs_reward_pick(可选奖励数, 已选)`（**用未过滤条数**，与原版同口径）+ 单测 |
| 弹框 | `MirMessageBox(message)`（`MirMessageBox.cs:14-52`）：`Prguse[360]` 456x190 **居中** = (284,289)、文本 `@(35,35)` 390x110、OK `Title[200/201/202] @(360,157)` | 同几何（`LIST_NOTICE_*`），文案取 `ClientTextKeys.YouMustSelectRewardItem` = 「你必须选择一个奖励物品。」（`Chinese.json:681`） |
| OK | `OKButton.Click += Dispose()`（`:52`） | `state.notice = false`（面板随之隐藏） |
| 放行 | 选好奖励后正常发 `C.FinishQuest{QuestIndex, SelectedItemIndex}` | 同（`finish_selected_index` 给出**未过滤**下标；无可选奖励时 -1） |

**实机实测**（`--mock --auto-enter --ui-scale 1` + `npc_call 4242 [@QUEST]` + `dialog open quest_list`；
夹具给该 NPC 的任务补一条 `ChangeQuest(taken=true, completed=true)` ⇒ 完成钮才出现）：

| 步骤 | 实测 |
|---|---|
| 未选奖励点「完成」(561,448) | `quest_list_probe.notice` **false → true**；`win_locate Prguse[360]` 命中 **(284,289)**、不符率 **0.0252**；日志「未选定奖励物品，不发送 FinishQuest（弹提示框）」⇒ **没有**发包 |
| 点 OK (682,458) | 命中 `76x25 [root=QuestList]` ⇒ `notice` **true → false**，同一模板匹配已找不到该面板（框已关） |
| 先点可选奖励格 0 (507,396) | `selected_reward` **0**；日志「🎁 选择奖励：木剑×1（未过滤下标 0）」 |
| 再点「完成」 | `notice` 仍 **false**（不弹框）；日志「📜 交付任务 #2 给比奇老兵送信（选定奖励下标 0）」⇒ 发了 `FinishQuest{selected_item_index: 0}` |

**顺带修了夹具自身的坑**：`mock/state.rs` 的 `quest_reward()` 给的是 `RequiredGender::NONE`（**无位**），
而 C# `FilterRewards` 用 `HasFlag` ⇒ 位掩码 0 任何性别都不通过、可选格一个都不画（看起来像"本端没实现奖励格"）。
新增 `quest_reward_both_genders()` 并让 NPC 任务的可选奖励用它——这是**夹具**问题，不是产品缺陷。

**照抄的原版怪癖（记一笔）**：提示框判据用的是**未过滤**的 `RewardsSelectItem.Count`，所以当可选奖励
全被性别过滤掉时，原版会"弹出提示却没有任何可选的格子"（走不出去）——本端同口径照抄，不"修正"。

**门禁**：`cargo check`（lib+bin）0 error；`cargo test --lib` **882 passed / 0 failed**（+1 守卫单测）；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。

**仍未做**：C# 的 `Modal = true`（弹框期间拦其它输入）——本端只做了显隐与 OK；以及 §3.2bc 记的
详情窗拖动实机复验、原版同状态内容帧。

### 3.2bi `game_shop` 筛选按钮的**选中态**画源修好（`ImageButton.normal` vs 直接写 `ImageNode`）（2026-09-29）

线索来自 §3.2bh 的同一对同状态帧：第 0 档区段页签（`Show All`）在**原版帧**里最像 `Title[771]`（选中帧，
0.288；常态 `770` 是 0.805），而**本端帧**在 (302,214) 是 `Title[770]`（**0.000**）。
先用当前客户端**重取一帧**排除"旧帧过时"：新帧仍是 `770`（0.000）⇒ **真缺陷**。

**根因（一行说清）**：`ui::theme::image_button_system` 每帧按 `Interaction` 覆盖 `ImageNode`
（`None → ImageButton.normal`），而 `game_shop` 的筛选按钮系统**直接写 `ImageNode`** 表示选中 ⇒ 被它盖回去。
C# 的语义本来就不是"叠一张图"，而是**换按钮的基础索引**：`GameshopDialog.cs:625-628`
`if (SectionFilter == "Show All") allItems.Index = 771;`。

**修法**：新增 `ShopFilterArt { normal, selected }` 记两态；选中时把 **`ImageButton.normal`** 换成 selected 帧
（hover/pressed 保持 spawn 值），**不再**直接写 `ImageNode`，交给通用系统按 `Interaction` 渲染。

**实机复验**（mock，`dialog open game_shop`，`art_match` 比对固定格）：

| 状态 | 第 0 档 (302,214) | 第 1 档 (373,214) |
|---|---|---|
| 默认（`section_filter="Show All"`） | **`771`(选中) 0.000**（修前 `770` 0.000 / `771` 0.766） | `776` 0.000 |
| 点第 1 档中心 (409,226) 后（`TopItems`，商品 2→1 条） | **回 `770` 0.000** | **`777`(选中) 0.000** |

**顺带记一条夹具坑（与 §3.2y 的"点了没反应"同类）**：页签精灵 72x24 的**左上角点** (373,214) 会被
上面的面板根吃掉（`hits` 回的是 `696x476 [root=GameShop]`），要点**中心** (409,226) 才落到按钮
（`hits` 回 `72x24 [root=GameShop]`）。凡"按整格边界算落点"的夹具都可能踩这条。

**同模式排查**：按"查询里同时有 `&mut ImageButton` 又直接写 `ImageNode`"扫全仓
`Client-Bevy/src/game/dialogs/`，**只有 game_shop 命中**（mail 那处的 `ImageNode` 是只读）。
下次做同类"选中态"窗时先跑这条 grep。

**门禁**：`cargo check`（lib+bin）0 error；`cargo test --lib` **885 passed / 0 failed**；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。

### 3.2bv 「零对拍」批次①：公告窗 `NoticeDialog` 拿到**同状态原版帧**（键盘路径，锁屏也能取）——捞出三处真缺口并修（2026-09-30）

**为什么能取到**（其他「只能靠鼠标到达」的窗做不到）：公告窗的入口是**服务端下发**
`S.UpdateNotice`，`<Server>\Envir\Notice.txt` 一改、重登就推，全程不用鼠标（§2.1 键盘路径）。

**配方（两端同文本，可复跑）**

```powershell
# ① 原版侧：写沙箱公告（注意路径是 Envir\ 下，不是 Server\ 根！）+ 重启沙箱服务端 + 键盘登录
$sb="$env:TEMP\golden_sandbox"
$lines=@('TITLE=服务器公告') + (1..25 | % { "第 $_ 行公告内容" })
[IO.File]::WriteAllText("$sb\Server\Envir\Notice.txt", (($lines -join "`r`n")+"`r`n"), (New-Object Text.UTF8Encoding($false)))
Start-Process "$sb\Server\Server.exe" -WorkingDirectory "$sb\Server"
pwsh tools\acceptance\csharp_golden\csharp_kbd_login.ps1 -SandboxRoot $sb -Account 333 -Password 333333
#   → shots\orig_kbd_02_ingame.png 就带公告窗（进图后 10s 截的那帧）
# ② 本端侧：同一条文本走新夹具 `notice_set`（与 `S.UpdateNotice` 同一支 `set_notice`）
pwsh tools\acceptance\rpc.ps1 -Method notice_set -Params '{"title":"服务器公告","message":"第 1 行公告内容\r\n…\r\n"}'
pwsh tools\acceptance\rpc.ps1 -Method screenshot -Params '{"path":"<worktree>\ours_notice_long.png"}'
```

**判据与读数**（面板 `Prguse[961]` 316x466；原版实测落在 **(355,100)**、本端 **(354,100)**——
差的这 1px 就是 §3.2c–§3.2f 记的**居中窗恒定取帧偏移**，比对时按 `dx=+1` 对齐，别当缺陷）

| 控件 | C# 出处 | 原版实机落点（26 行公告） | 修复前（本端） | 修复后 |
|---|---|---|---|---|
| Panel `Prguse[961]` | `NoticeDialog.cs:34-38` | (355,100) 316x466 | (354,100) 316x466 ✓ | 同 |
| Ok/CLOSE `Title[193]` | `:63-75`（**未设 Size** → art 68x25） | 面板内 (120,436) | **20x20 压缩**：1700 px 里 1694 不符（0.9965） | **0.0000** |
| Up `Prguse2[470]` | `:70-88`（`Size=(16,14)` 只作命中框） | 面板内 (293,33) art 12x12 | **20x20 拉伸**：0.8194 | **0.0000** |
| Down `Prguse2[473]` | `:89-98` | 面板内 (293,418) art 12x12 | 0.8750 | **0.0000** |
| PositionBar `Prguse2[205]` | `:100-118`（`Movable`，y∈[46,399]） | 面板内 (293,46) art 12x18 | **根本没画**（`NoticeBar` 只声明没生成）；`>19 行`时原版有、本端无 | **0.0000** |

> 上表「不符」口径 = 同尺寸区域逐像素 RGB 差之和 > 12 的像素数 / 区域像素数，**已按 `dx=+1`
> 对齐**（原版帧比本端右 1px）。模板匹配另测：修复后 `Prguse2[205]/[470]/[473]` 在两端
> `win_locate.py` 最优落点都不符率 **0.0000**。

**整窗对表**（`shot_diff.py ours orig 354,100,670,566`）：**未对齐 120029/147256 = 81.5%**；
**按 `dx=+1` 对齐后 23434/147256 = 15.9%**（其中正文区 18.6%、标题行 6.8%、Ok 钮 0.64%）。
⇒ 剩下的差异是**文字**（C# `10F` + `TextRenderer` 渲染 vs 本端 CJK 字体/10px），不是几何：
控件与面板边框都已是 0~7%。

**本轮改了什么**（`Client-Bevy`）

1. `game/dialogs/notice.rs`：Ok/Up/Down 三钮改**art 原生尺寸**（68x25 / 12x12 / 12x12）——
   原先是统一 20x20，把 art 压/拉变形；新增 `ARROW_W/ARROW_H/OK_W/OK_H` 常量 + 单测钉住。
2. 同文件：**补上 PositionBar**（`Prguse2[205/206]` @(293,46)，12x18）+ `>19 行`才显示
   （C# `NewText` :218-243）+ 拖动跟手（`PositionBar_OnMoving` :134-152 的
   `index_from_bar_y`，本就写好且有单测，此前是**死代码**——条根本没生成）。
3. `control.rs`：新增实机夹具 **`notice_set {title, message}`**（→ `notice::set_notice`，
   与 `S.UpdateNotice` 共用一条折行/置态路径）。**没有它就没法把同一份 26 行文本喂给两端**，
   这扇窗只能停在"零对拍"。注：`apply_control_commands` 已是 Bevy 的 16 参数上限，新资源并进
   `ControlQueries` 而不是加系统参数（加了会 `cannot become an ObserverSystem`）。
4. `set_notice()` 抽出来：服务端事件与夹具共用（两处各写一遍折行必然漂移）。

**口径坑（本轮踩到，写给下一轮）**

- **公告文件在 `<Server>\Envir\Notice.txt`**，不是 `<Server>\Notice.txt`（`Server/Settings.cs:32`
  `NoticePath = Path.Combine(EnvirPath, "Notice.txt")`）。写到根目录 → 服务端静默不推公告，
  客户端一切正常、只是没窗——很容易误判成"本端没实现"。
- 推送条件：`Settings.Notice.LastUpdate > Info.LastLogoutDate`（`Server/MirObjects/PlayerObject.cs:1172`）
  ⇒ 改完文件必须**重启沙箱服务端**（`LoadNotice` 只在启动时读）；文件 mtime 新于上次登出即成立。
- **原版 `NoticeDialog.cs` 源码与实机行为不一致**：源码 `NewText` :218-243 写明
  `lines.Count <= 19` 时 `UpButton/DownButton.Visible = false`，但**实机帧里两个箭头一直在**
  （3 行短公告也画，模板匹配 0.0000；`q_closed/t1_closed/select` 那些没有公告的帧则 0.55
  找不到 ⇒ 确实是公告窗画的）。本端**按实机行为对齐**（箭头常显、位置条按行数显隐）——
  这条差异如实记录，别照源码把箭头改没了。
- 公告正则 `{t/colour}`、`(t/http-link)` 的解析在 §3.2ba/§3.2bc 已定案，本轮未动。

**本批另外两行的状态（如实）**

- **Roll（`RollDialog`）**：本端布局/帧表已按 C# `Setup` 对齐（`roll.rs` 有常量单测，骰子
  65x65 @(474,344)、尤茨 180x130 @(422,319)）。**原版侧同状态帧未采集**：入口是 NPC 脚本
  `ROLLDIE/ROLLYUT` 发 `S.Roll`，要先把人物走到指定 NPC 再点行——属 §3.2l 那批（需解锁 + 真鼠标）。
  `window_rect_table.py` 也**收不了它**：`RollDialog` 是纯 `MirControl`（无 `Index/Library`，
  尺寸/位置在 `Setup` 里按 type 现设），几何期望值只能落在 `roll.rs` 常量里（即上面那两行）。
- **Trade / GuestTrade**：本端几何已按 C# 对齐（`trade.rs` 常量：我方 `Prguse[389]` 204x152
  @(298,418)、对方 `[390]` @(522,418)）。**原版侧未采集**：`TradeDialog` 单开不可达
  （§3.3 已把 Trade 列为 excluded）——要取帧得有**第二个客户端**或服务端造出交易态；
  本端 `GuestTrade` 也刻意没有独立 RPC（批M 审查：由交易会话驱动）。判据设计留给下一轮：
  要么给 `--mock` 补一条"造交易态"的入口（本端先能同时开两扇窗），要么原版侧起两个沙箱客户端。

**门禁**：`cargo check --tests` 0 error；新增单测 `notice_buttons_use_native_art_size`
（红检：把三钮改回 20x20 → FAILED）；lib/集成测试结论见 PR。

### 3.2bw 模态层批次（walgit `crystal-modal-layer-batch`）：UI 层遮挡 + 旧提示复活 + 巡回脚本两处工具缺陷（2026-09-30）

**① 统一模态层：把 C# `MirControl.Modal = true` 的「吞掉整个客户区」补全**

原版依据（`MirControl.cs:825-828`）：`IsMouseOver` 在 `Modal` 为真时对**任意点**返回真，
子控件派发自顶向下取首个命中并 `return` ⇒ 弹框期间**点别的对话框也不该有反应**。
本端此前只有世界点击那一半（`player_control::UiLockState`），点下层对话框仍穿透。

做法（`Client-Bevy/src/game/dialogs/modal_layer.rs`）：

* 一枚**全客户区**遮挡节点（1024x768 @(0,0)，z = [`MODAL_BLOCKER_Z`] = 59），
  显隐由**唯一真值** `modal_any_visible(9 个来源)` 驱动（背包选中 / 数量框 / 丢弃确认 /
  快捷键分配 / 通用 `MirMessageBox` / 组队邀请 / 行会邀请 / 商城确认 / 英雄询问）；
  世界点击闸与它共用同一支函数（原先 7 处各自挑 z：60/60/60/45/45/46/47）。
* 模态面板统一挂 `MODAL_PANEL_Z` = 60（高于遮挡层 ⇒ 框自己的按钮照常可点）。
* **踩到的两个坑**（都写进单测红检）：
  1. 遮挡节点只写 `Button` 不够 —— `ui_focus_system` 照样把 `Pressed` 发给下层按钮
     （实测：弹框期间点背包 X 仍然关窗）。必须**显式** `FocusPolicy::Block`。
  2. 没有 `Pickable` 时，bevy_picking 的 UI 后端在 `require_markers` 下**直接跳过**该节点
     （`bevy_ui-0.19.1/src/picking_backend.rs:194`）⇒ HoverMap 里根本看不见遮挡层。
* **更深的根因：动态 z 无界**。`dialog_front_system::bump_dialog_z` 每次 +10 把被点窗抬到最前
  ——逐窗巡回连点 45 扇窗后，被点窗的 z 涨到 400+，**盖过固定 z 的模态层**（实测 `(modal)block`
  第一次就是被它打红的）。现改为**带内重排**：所有对话框按当前 z 排名压回 `[30, 55]`
  （`compact_zs`，带顶 55 > 全部静态非模态 z 的 51、< 遮挡层 59），同 kind 整体平移 ⇒
  窗内相对层级（如写邮件覆盖层）不变。单测 `dialog_z_band_stays_below_modal_layer` 钉住。

**实机判据**（进 `ui_interact_sweep.ps1`，与逐窗巡回同一次运行）：

```
--- 模态遮挡 ---
模态遮挡: 弹框期间点背包 X 无反应 blocked=YES hits=[630v2 1536x1152 []]
模态解除: 同一处点击生效 closed=YES hits=[6379v0 36x31 [root=Inventory]]
```

即：开背包 → `notice_box_show`（新夹具，弹通用 `MirMessageBox`）→ 点背包关闭钮 ⇒ **没关**；
清掉提示框 → 点**同一处** ⇒ **关**（反例同时排除了「这个钮本来就点不动」）。
`notice_box_show {text}` 是本批新增的实机夹具（`control.rs`）。

**② `cleanup_notice_box` 旧提示复活**：退出 `AppState::Game` 时若提示框还开着，
原实现只清 `panel_ready` 不清 `text` ⇒ 下次进图面板一建出来，**上一局的旧提示自己弹回来**。
现一并 `notice.text = None`；单测 `cleanup_clears_text_so_stale_notice_cannot_revive`
（红检：删掉那一行 → FAILED）。

**③ 巡回脚本两处工具缺陷**（都会让读证据的人误判）

- **(a) `pass` 两套口径**：同一次运行控制台打 `pass=45 total=45`，产物 JSON 却记 `pass=41`
  （JSON 只数 `closed=YES`，把 4 条 `open=GUARDED` 漏了）。现统一为
  `closed=YES 或 open=GUARDED`，并在 JSON 里**单列** `guarded`。实测本轮
  `gate.pass=46 total=47 guarded=4` 与控制台一致。
- **(b) 构建戳护栏静默降级**：跨 target 目录跑（worktree + `CARGO_TARGET_DIR`、或显式
  `-ClientExe`）时旧实现「反推不出构建根」只打一行 WARN 就跳过比对——正是「拿昨天的二进制
  跑出绿」那道护栏失效的场景。现 `Assert-ClientBuildStamp -ExpectCommit <sha>` 支持**显式期望提交**
  （不等即 exit 2，不做"Client-Bevy 有无改动"的软化），`ui_interact_sweep.ps1` 默认取
  `-RepoRoot` 的 HEAD 传进去，并把 `build_commit`/`expect_commit` 写进结论 JSON。自证实测：
  `-ExpectCommit 0000…` ⇒ exit 2；`touch src/lib.rs` 让 exe 比源码旧 ⇒ exit 2。
  **顺带记一条真坑**：`cargo build` 不一定重跑 `build.rs`，构建戳会**停在旧提交**
  （本轮实测：提交后 build 仍是上一个 commit 的戳，被新护栏当场拦下）——改完提交要重建时，
  `touch Client-Bevy/build.rs` 再 build（或 `cargo clean -p client_bevy`）。

**门禁**：`cargo check --tests` 0 error；`cargo test --lib` **900 passed / 0 failed**；
`b0001_smoke` 2 + `ui_alignment` 53；**`ui_interact_sweep.ps1 -ManageServer` pass=46 total=47
fail=0 skip=0 exit=0**；改动文件 `cargo fmt -- --check` 与 master 基线逐 hunk 一致。

### 3.2bx 「零对拍」批次①续：**Buff 窗拿到原版帧**（`setadmin` + GM 登录即推 buff），外加锁屏期「聊天/GM 命令」为什么走不通（2026-09-30）

**结论先行**：Buff 这行在 walgit `crystal-zero-ab-windows` 里原记为「判为不可比：测试角色无 buff」。
现在**原版侧帧拿到了**，而且**不需要鼠标**——靠的是「GM 账号登录时服务端自己推 buff」。

**配方（可复跑）**

```powershell
# ① 把沙箱账号改成 GM（只重写沙箱 Server.MirADB；与 setpw/setgold/setpos 同一条路径）
dotnet run --project tools\acceptance\csharp_golden\dbtool\dbtool.csproj -c Release -- `
    "$env:TEMP\golden_sandbox\Server" setadmin 333 1
# ② 重启沙箱服务端 + 键盘登录（§2.1）
Start-Process "$env:TEMP\golden_sandbox\Server\Server.exe" -WorkingDirectory "$env:TEMP\golden_sandbox\Server"
pwsh tools\acceptance\csharp_golden\csharp_kbd_login.ps1 -SandboxRoot $env:TEMP\golden_sandbox -Account 333 -Password 333333
# ③ 进图那一刻的帧（`orig_kbd_02_ingame.png`）右上角就有 buff 图标
py -3.12 tools\acceptance\csharp_golden\shot_diff.py "$env:TEMP\golden_sandbox\shots\orig_kbd_02_ingame.png" ...
```

**判据与读数**

| 证据 | 值 |
|---|---|
| 服务端日志 | `2026-09-30 08:25:09 INFO - 女道士 is now a GM`（`PlayerObject.cs:224-228` 只在 `Account.AdminAccount` 时打） |
| 原版帧 | `shots/orig_kbd_02_ingame.png` 右上 **(≈846..890, 20..38)** 出现**两枚 buff 图标**（一枚 "GM"、一枚蓝白） |
| 依据（源码） | `PlayerObject.cs:1341-1347`：登录收尾 `if (IsGM) UpdateGMBuff();` → `HumanObject.cs:833-844` `AddBuff(BuffType.GameMaster, …, values: options)` |

⚠️ **`dbtool export` 的 `admin` 字段不可靠**：`setadmin` 回读 `admin=True`、服务端日志也证明生效，
但同一份 `Server.MirADB` 用 `export` 读出来仍是 `admin:false`（账号 `333`）。判「改没改成功」请看
**`setadmin` 的回读 + 服务端 `is now a GM` 日志**，别只看 export 的 JSON（工具口径问题，另记）。

**顺带把「锁屏期能不能用 GM 命令」这条路走死了**（省得下一轮再试）：

- GM 命令（`@SUPERMAN`/`@GAMEMASTER`/`@OBSERVER`，`PlayerObject.cs:2438-2462`）走**聊天框**；
- 锁屏下聊天框**能开**（向客户端主窗口发 `WM_CHAR '@'` → `ChatPanel_KeyPress` → `ChatTextBox.SetFocus()`，
  实测子窗口里出现 `WindowsForms10.Edit…` 且 `text='@'`），但**后续字符进不去**：
  再往主窗口或该 Edit 子窗口发 `WM_CHAR` 都不生效（`text` 恒为 `'@'`），`WM_SETTEXT` 也会被
  镜像同步覆盖——`MirTextBox.Text` 直接读 WinForms `TextBox.Text`，而字符流要经**真实消息泵**进焦点控件。
  ⇒ **解锁前别押「打字发命令」**；`setadmin` 的价值是「登录即推的状态」（GM buff）这类**服务端主动**路径。
- 另一条「不靠打字」的路子试过且**未采集**：`[Rested] Period=1`（`Setup.ini`）+ `setpos 0 288 616` +
  原地 80s，`win_locate Prguse2[20]`（Buff 面板 44x34 @(854,0)）仍 0.52 不符 ⇒ 没抓到 buff 窗
  （右上角常被角色窗/HUD 占位，且当时无法读原版客户端的 buff 列表，判不出「没下发」还是「没画出来」）。
  这条留给下一轮：需要先有一个「读原版客户端 buff 列表」的探针，或解锁后用鼠标开窗。

**本端同状态 A/B（本轮补齐，2026-09-30）**

夹具：新增 **`buff_set {buffs:[{tag,remaining_ms},…]}`**（`control.rs`，替换 `BuffState.buffs`；
`--mock` 自带的 `--buff-test` 只回发 Mirroring ×3，与原版这帧不同态）。

```powershell
.\target\debug\client_bevy.exe --mock --auto-enter --ui-scale 1 --control-port 9000
pwsh tools\acceptance\rpc.ps1 -Method buff_set -Params '{"buffs":[{"tag":115,"remaining_ms":600000},{"tag":103,"remaining_ms":600000}]}'
pwsh tools\acceptance\rpc.ps1 -Method cursor   -Params '{"x":870,"y":15}'   # C# BuffDialog 悬停才显形
pwsh tools\acceptance\rpc.ps1 -Method screenshot -Params '{"path":"<worktree>\ours_buff2.png"}'
```

判据（`win_locate.py --lib Data\BuffIcon.Lib`，两端各自最优落点）：

| 图标 | 原版 `orig_kbd_02_ingame.png` | 本端 `ours_buff2.png` | 不符率 |
|---|---|---|---|
| `BuffIcon[173]`（C# `BuffType.GameMaster`） | **(841,6)** | **(842,6)** | 0.0000（两端） |
| `BuffIcon[240]`（C# `BuffType.Rested`） | **(864,6)** | **(865,6)** | 0.0000（两端） |

差的 1px 仍是 §3.2c–§3.2f 的取帧口径；**顺序也对上了**（Rested 在右 = i=0，GM 在左 = i=1，
与原版一致——C# `AddBuff` 是 `_buffList.Insert(0, …)`「最新在 i=0」，本端 `buff_added` 同样是
`buffs.insert(0, …)`）。窗内**背景**不可比（buff 面板背后是世界地图，两端地图视野不同）——
所以这一行只报「图标逐枚 0.0000 + 落点 1px」，不报整块像素占比。

**本轮改了什么**（`Client-Bevy`）

1. `game/dialogs/buff.rs`：`buff_display` 补 **103 `GameMaster`（icon 173）/ 115 `Rested`（icon 240）**
   ——原先落到 `_` 分支拿 icon 0（占位图），原版帧这两枚根本画不出来；名称取自
   `Chinese.json` 的 `Enum.BuffType_*`，图标 index 逐字取自 `BuffDialog.cs:478-501`。
2. 同文件：**图标预载**从 `0..=30` 扩到把 C# `//Special` 段 `103..=118` 一起载入
   （`buff_ui_system` 只从 `assets.icons` 取句柄，漏载的 tag 即使表里有 icon 也只会画占位图）。
3. `control.rs`：`buff_set` 夹具（并进 `ControlQueries`，`apply_control_commands` 已在 16 参数上限）。

**门禁**：`cargo test --lib` **900 passed / 0 failed**；`ui_interact_sweep.ps1 -ManageServer`
**pass=46 total=47 fail=0 skip=0 exit=0**；改动文件 `cargo fmt -- --check` 与 master 基线逐 hunk 一致。

**仍未采集**：Buff 面板在**未展开**（只画 i=0）与**悬停渐隐动画**这两档还没逐帧比
（原版那档要真鼠标悬停/点展开钮；本端是直接显隐，不逐帧复刻 `Opacity` 渐隐）。

### 3.2bu 原版 C# 客户端连的是**原版 C# 服务端**，不是 `ServerRust`（2026-09-30 实测）

**问题**：原版 `Client.exe` 到底连哪个服务端？——**本目录沙箱里连的是原版 `Server\Server.exe`**
（§1 的 7100）。本项目的产品配对是 **`Client-Bevy` ↔ `ServerRust`**，两条线不是一回事。

| 侧 | 监听 | 帧格式 | 谁能连 |
|---|---|---|---|
| 原版 `Server\Server.exe`（沙箱 7100） | 明文游戏端口 | `[u16 LE 长度][i16 opcode][body]`，**无加密** | 原版 `Client.exe`（`Client/MirNetwork/Packet.cs` 读写的就是这个） |
| `ServerRust/.../mir2_server.exe`（gate 7000） | `cfg.network.listen_addr`（`ServerRust/config/server.toml`） | **`[u16 LE 长度] + XOR 0xAA(payload)`**（`ServerRust/src/gate/codec.rs:1-8`，对应原版 **LoginGate** 的约定；`Client-Bevy/src/network/codec.rs:4` 同 key） | `Client-Bevy`（`--real-net`） |

**实测**（把沙箱客户端 `Mir2Config.ini [Network] Port` 临时从 7100 → 7000，指向本机正在跑的
`mir2_server` debug 实例；测完已还原 7100、客户端进程已停）：

1. **TCP 能连上**：`netstat` 见 `127.0.0.1:57947 → 127.0.0.1:7000 ESTABLISHED`，owner pid 的
   `ExecutablePath` 已核为沙箱 `Client.exe`。
2. **服务端 accept 后立刻回 `Connected`**：裸 TCP 探针收到 6 字节 `04 00 ae aa aa aa`
   = `[u16 len=4] + XOR0xAA(00 00 00 00)` ⇒ 解出 `[u16 len=0][i16 opcode=0]`，
   opcode 0 = `ServerPacketIds::Connected`（`SharedRust/src/enums.rs:2333`，与 C# 同值）。
3. **原版客户端不认这帧**：登录框的 WinForms 文本框始终没出现（`Get-CsEdits` 返回 0 条
   `WindowsForms10.Edit`；同一沙箱走 7100 时是 2 条），画面停在 `LoginScene.cs:84` 的
   `_connectBox = new MirMessageBox(AttemptingConnectServer, Cancel)`；25s 与 90s 两帧
   `shot_diff.py` 只差 **3430 px / 106066**，差异 bbox `(322,5,615,367)` 就是那块消息框
   （差别来自 `LoginScene.Process()` 刷新的重试计数）。
4. **客户端在反复重连**：90s 快照里 `127.0.0.1:* → :7000` 有 **≥20 条 `TIME_WAIT`、0 条 `ESTABLISHED`**
   （`Network.MaxAttempts = 20`、`RetryTime = CMain.Time + 5000`）。

**根因（确定性）**：原版客户端直连游戏端口时**不做 XOR**（`Client/MirNetwork/Network.cs:24-63` 只有
`TcpClient` + `Packet` 的小端 `Length/Index` 头），而 `ServerRust` 的 gate 把 payload 整体 XOR 0xAA
——两边「同一份 opcode 表、不同的封装」，所以**原版客户端目前连不上 `ServerRust`**，
也不是「连上了但版本校验不过」。

**据此的使用口径**：需要「原版侧」的帧/行为，一律走 §1 沙箱（原版 `Server.exe` + `Client.exe`，
7100，口令 `333/333333`）；`ServerRust` 只服务 `Client-Bevy`。要验 Rust 服务端，用
`scripts/run_real_e2e.ps1` / `client_bevy.exe --real-net`，别拿原版客户端当探针。

**未采集**：① 中间插一层原版 `LoginGate` 后原版客户端能否连 `ServerRust`（本轮没试）；
② `ServerRust` 那一侧的握手日志（跑着的是 9/28 的 debug 实例，stdout 没留痕，本轮只从
客户端/网络侧取证）。

### 3.2bk §3.2bj 的「详情窗拖动未采集」**收口：能跑，之前是夹具用法问题**（2026-09-29）

§3.2bj 记的"拖动拿不到"本轮复现并推翻——按下面配方一次就成：

```powershell
# 1) 起客户端（需要 --quest-data-test 让目录里有任务定义）
.\target\debug\client_bevy.exe --mock --auto-enter --quest-data-test --ui-scale 1
# 2) 让 mock 下发「14 行描述」的那条任务（quest 1 只有 8 行描述，不够一页）
pwsh tools/acceptance/rpc.ps1 -Method npc_call     -Params '{"object_id":4242,"key":"[@QUEST]"}'
pwsh tools/acceptance/rpc.ps1 -Method quest_detail -Params '{"quest_id":2}'
# 3) 取当前面板原点（该窗 Movable，被拖过就不是 (532,60) 了）
pwsh tools/acceptance/rpc.ps1 -Method dialog_rect  -Params '{"kind":"quest_detail","fallback":"root"}'
# 4) **用 ui_nodes_at 回读条的节点矩形**，拿它的中心当按压点
pwsh tools/acceptance/rpc.ps1 -Method ui_nodes_at   -Params '{"x":825,"y":108}'   # → rect [825,106,12,18] z=12
pwsh tools/acceptance/rpc.ps1 -Method click         -Params '{"x":831,"y":115,"drag_to":{"x":831,"y":200}}'
```

**实测（本轮）**：

| 判据 | 实测 |
|---|---|
| 按压命中 | `hits: ["5168v0 12x18 [root=QuestDetail]"]` ← 就是位置条节点 |
| 状态 | `detail_top_line` **0 → 3**（`interval = (261−46)/(24−16) = 26`） |
| 窗**没被误拖** | `dialog_rect` 前后都是 `(532,60,316,466)` |
| 条**跟手** | 条节点 rect **y: 106 → 184**（= 面板 46 + 3×26 ✓，`ui_nodes_at` 回读） |

⇒ **§3.2bc ③（拖动读真实光标）实机验证通过**；§3.2bj 的"未采集"作废。

**两条教训（正是 §3.2bj 两次失败的原因）**：

1. **消息区必须先 >16 行**：quest 1（8 行描述）根本没有位置条，按在"推定的条位置"上其实按到**消息标签**
   ⇒ 被该窗的窗拖动系统接管，**整扇窗被拖走**（§3.2bj 里"条跑到 (813,88)"就是这么来的——那是窗位移 (-12,-20) 的结果）；
2. **按压点要用 `ui_nodes_at` 回读的条节点矩形中心**，别用"面板原点 + C# 常量"推出来的点——该窗 `Movable = true`，原点随时会变。

（另注：拖动后 `win_locate Prguse2[205]` 找不到条是**正常的**——鼠标正停在条上，通用按钮系统把它切到 hover 帧了；
判条的位置用 `ui_nodes_at` 更稳。）

**门禁**：本轮只动文档，无产品代码变化。

### 3.2bl 「只能靠鼠标到达」批收尾：`game_shop` **买卖路径**（商品格买钮 → `MirMessageBox` 确认 → 发包）（2026-09-29）

清单最后一项。C# 的买路（`Client/MirControls/MirGameShopCell.cs:193-228`）是**三段**：
点格内买钮 → `MirMessageBox(ConfirmPurchaseItemGold / ConfirmBuyItemCredits, YesNo)` →
**Yes** 才 `Network.Enqueue(new C.GameshopBuy{ GIndex, Quantity, PType })`。

**本端已有同构实现**：格内买钮 `Title[778..780] @ 格内(42,122)`（`game_shop.rs:285-287`）、
确认框 `GameShopConfirm`（`Prguse[360]` 456x190 @**(284,289)**、文本 @(35,35)、Yes `Title[206..208]` @(260,157) /
No `Title[210..212]` @(360,157)，文案取同两个 `ClientTextKeys`）——即 `MirMessageBox` 的同一套几何。

**本端实机全链路**（mock，`dialog open game_shop`）：

| 步骤 | 实测 |
|---|---|
| 点格 0 买钮 (392,395) | `hits` 命中 `11428v0 [root=GameShop]`；确认框 `win_locate Prguse[360]` 命中 **(284,289)**、不符率 **0.0442**；日志「🛒 确认购买 #221 木剑 x1 付款方式=金币」 |
| 点 **Yes** (582,458) | `hits` 命中 `11685v0 76x25 [root=GameShop]`（Yes 钮）；日志「🛒 购买商城商品 #221（付款 1）」→ mock 回「🛒 [MOCK] 商城购买 #221 x1（付款 1）**邮件送达**」 |
| 框是否关掉 | 复查 `Prguse[360]` 全屏最优落点已不在 (284,289)（0.2093）⇒ **框已关** |

⇒ 这条清单项**本端实机收口**。**仍未采集的只剩"原版侧同状态帧"**（要在沙箱里点商品格）；
但确认框这个控件（`MirMessageBox`）的**两端几何对照在上游已经做过**——§3.2az 用同一模板在两帧上量过
它落在 (284,289)（本端 0.0252；C# 由 `MirMessageBox.cs:23-26` 的 `((1024-456)/2,(768-190)/2)` 给出同值），
本轮又给同式框量到 0.0442，故只在"原版现帧"这一层留白。

**门禁**：本轮只动文档，无产品代码变化。

### 3.2bj 探针补「详情窗状态」＋ §3.2bc ③（详情窗位置条拖动）复验：**仍未采集，但原因具体化**（2026-09-29）

**① 探针扩展（本轮落地）**：`quest_list_probe` 原来只回 NPC 侧列表窗的状态，详情窗（`QuestDetailDialog`）
没有判据——§3.2bc ③ 那条"改读真实光标"的改动就是想验也没法断言。本轮给它补四项：
`detail_quest_id` / `detail_top_line` / `detail_selected_reward` / `detail_confirm_cancel`
（读的就是 `QuestDetailState`，与 `quest_detail{quest_id[,top_line][,confirm]}` 这个**既有夹具 RPC** 写的是同一份）。

**② 顺带把"条该不该在"这一步确认了**：详情窗消息区要 **>16 行**位置条才出现。
夹具里 quest 1（8 行描述）不够；**quest 2**（14 行描述，走 `npc_call 4242 [@QUEST]`）够 ⇒
`win_locate Prguse2[205]` 命中 **(813,88) 不符率 0.0000**（该坐标 = 当前窗原点 + 面板内 (293,48)；
**注意详情窗 `Movable = true`，面板原点会被拖走**，量条之前先取 `dialog_rect quest_detail`）。

**③ 拖动本身：仍未采集，原因具体化**。夹具用 `click {x,y,drag_to}` 压在条上：

| 尝试 | 结果 |
|---|---|
| 第一次（条在 (825,108) 的**推定**位置，实际窗原点已偏） | 按下点落在窗内 ⇒ **整扇窗被拖走**（随后量到条在 (813,88)，正好是面板原点位移 (-12,-20) 的结果）；`detail_top_line` 仍 0 |
| 第二次（条的真实位置 (819,97) 中心） | `hits` 回 `66v0 ?`、拖后帧里**找不到** `Prguse2[205]`（最优 0.30）；`detail_top_line` 仍 0 |

⇒ 与本端**列表窗**同一驱动方式能改 `top_line`（§3.2bb 实测 0→5）形成对照。差别在哪需要单独查
（候选：两窗根/子按钮在 `dialog_drag_system` 下的命中差异、或详情窗那个 `?` 尺寸的命中实体是谁）。
**本轮不硬猜、也不推数**，按"未采集"记。

**下一次的两条路**（写给下一轮，任选其一即可给结论）：
1. 给详情窗拖动复验加一个"只压条不拖窗"的驱动（例如测试期临时挂 `NotDraggable`，或用键盘/滚轮改 `top_line` 只验条的**显示**跟随）；
2. 先查清两窗差异：`ui_nodes_at` 打在条心，看命中的实体与祖先链，与列表窗同点对照。

**门禁**：`cargo check`（lib+bin）0 error；`cargo test --lib` **885 passed / 0 failed**；
`cargo test --test b0001_smoke --test ui_alignment` **2 + 53 passed**。
