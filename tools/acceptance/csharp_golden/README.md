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
