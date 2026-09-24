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

## 3. 已知限制（2026-09-24 实测）

**会话被锁屏（`LockScreenBackstopFrame` 覆盖桌面）时，原版客户端的菜单/对话框点不动**：

- 真实输入（`mouse_event`/`SetCursorPos`）落到锁屏，不落客户端；桌面级 `CopyFromScreen` 只得到纯色；
- 注入窗口消息可以到达客户端：`WM_MOUSEMOVE` 能让按钮变 hover、`WM_KEYDOWN/UP` 能触发 `CMain_KeyUp`（截图就是这么触发的）；
- 但 **WinForms 不会为注入的 `WM_LBUTTONDOWN/UP` 触发 `MouseClick`**（用最小 WinForms 探针程序验证：MouseDown/MouseUp 触发、MouseClick 不触发；加 `AttachThreadInput+SetCapture`、或同时按住真实左键都无效）；
- 原版镜像 UI 的按钮 `Click` 只在 `MirScene.OnMouseClick` → `MirControl.OnMouseClick` 里触发
  （`Client/MirControls/MirScene.cs`、`MirControl.cs:849`），**MouseDown/Up 不产生 Click** →
  锁屏状态下无法自动点击菜单/对话框。

因此：**要做逐窗 A/B（商城/背包/行会等），需要先解锁工作站或重连该 console 会话**，再跑本驱动脚本。
锁屏期间仍可用的路径：键盘驱动的登录（`LoginDialog.TextBox_KeyPress`：账户/密码框里回车 =
`OKButton.InvokeMouseClick`，见 `Client/MirScenes/LoginScene.cs:481`）。

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
