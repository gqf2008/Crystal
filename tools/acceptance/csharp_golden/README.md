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
例外三类，各有据：

1. **无关闭钮的窗**（`menu/minimap/buff/refine/timer/chat_notice`，即 manifest 的
   `no_close_by_design`）取不到——`dialog_rect` 是**从关闭钮反推**窗口矩形的
   （`rx = cx - w/2`），没关闭钮就没有 `cx/cy`，返回 `{ok:false,error:"close button not found"}`。
   想让这 6 扇窗也进对表，得给 `dialog_rect` 加一条"直接从面板 Node 取矩形"的路径（未做）。
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
