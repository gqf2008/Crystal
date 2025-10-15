# 🐛 装备库路径格式错误修复

## 问题诊断

### 错误现象
```
2025-10-15T07:38:49.601707Z ERROR ❌ 装备库 CArmours(0) 未加载
2025-10-15T07:38:49.613814Z ERROR ❌ 装备库 CArmours(0) 未加载
```

每帧都尝试加载装备库但一直失败,角色无法绘制。

### 根本原因

**路径格式错误**: Rust 代码使用了错误的文件名格式

| 代码 | 生成路径 | 实际文件 | 结果 |
|------|---------|---------|------|
| ❌ Rust (错误) | `CArmour/0000.Lib` | `CArmour/00.Lib` | ❌ 文件不存在 |
| ✅ C# (正确) | `CArmour/00.Lib` | `CArmour/00.Lib` | ✅ 文件存在 |

**原因分析**:

1. **Rust 原代码** (`libraries.rs`):
   ```rust
   LibraryName::CArmours(idx) => format!("CArmour/{:04}", idx),
   // 0 -> "CArmour/0000" (4位数字)
   ```

2. **C# 原版** (`MLibrary.cs` line 277):
   ```csharp
   library[i] = new MLibrary(path + i.ToString("00") + suffix);
   // 0 -> "CArmour/00" (2位数字)
   ```

3. **实际文件结构**:
   ```
   Data/CArmour/
     ├─ 00.Lib  ✅ 存在
     ├─ 01.Lib  ✅ 存在
     ├─ 02.Lib  ✅ 存在
     └─ ...
   
   Data/CArmour/
     ├─ 0000.Lib  ❌ 不存在 (Rust 错误查找的文件)
   ```

## C# 原版分析

### InitLibrary 方法

**文件**: `Client/MirGraphics/MLibrary.cs` line 261-278

```csharp
static void InitLibrary(ref MLibrary[] library, string path, string toStringValue, string suffix = "")
{
    if (!Directory.Exists(path))
    {
        Directory.CreateDirectory(path);
    }

    // 查找目录中所有符合后缀的文件
    var allFiles = Directory.GetFiles(path, "*" + suffix + MLibrary.Extention, SearchOption.TopDirectoryOnly)
        .OrderBy(x => int.Parse(Regex.Match(x, @"\d+").Value));

    // 获取最后一个文件的编号
    var lastFile = allFiles.Count() > 0 ? Path.GetFileName(allFiles.Last()) : "0";
    var count = int.Parse(Regex.Match(lastFile, @"\d+").Value) + 1;

    library = new MLibrary[count];

    // 创建 MLibrary 数组
    for (int i = 0; i < count; i++)
    {
        // 关键: i.ToString(toStringValue)
        // toStringValue = "00" -> 生成 "00", "01", "02", ...
        library[i] = new MLibrary(path + i.ToString(toStringValue) + suffix);
    }
}
```

### 调用示例

**文件**: `Client/MirGraphics/MLibrary.cs` line 153

```csharp
static Libraries()
{
    //Wiz/War/Tao
    InitLibrary(ref CArmours, Settings.CArmourPath, "00");
    //                                               ^^^^
    //                                               使用 "00" 格式
    
    InitLibrary(ref CHair, Settings.CHairPath, "00");
    InitLibrary(ref CWeapons, Settings.CWeaponPath, "00");
    
    //Assassin
    InitLibrary(ref AArmours, Settings.AArmourPath, "00");
    
    //Archer
    InitLibrary(ref ARArmours, Settings.ARArmourPath, "00");
    
    //Other
    InitLibrary(ref Monsters, Settings.MonsterPath, "000");
    //                                               ^^^^^
    //                                               怪物使用 "000" (3位)
}
```

**格式规则**:
- 角色装备/武器/发型: `"00"` → 2位数 (00-99)
- 怪物: `"000"` → 3位数 (000-999)

## 解决方案

### 修复路径格式化

**文件**: `src/graphics/libraries.rs`

**修复前** (错误):
```rust
LibraryName::CArmours(idx) => format!("CArmour/{:04}", idx),
// 0 -> "CArmour/0000"
// 1 -> "CArmour/0001"
```

**修复后** (正确):
```rust
// 使用 {:02} 格式,对应 C# ToString("00")
LibraryName::CArmours(idx) => format!("CArmour/{:02}", idx),
// 0 -> "CArmour/00"
// 1 -> "CArmour/01"
```

### 完整修复代码

```rust
// 角色装备库 (使用 {:02} 格式,对应 C# ToString("00"))
// C# 代码: library[i] = new MLibrary(path + i.ToString("00") + suffix);
// 生成文件名: 00.Lib, 01.Lib, 02.Lib, ... 99.Lib
LibraryName::CArmours(idx) => format!("CArmour/{:02}", idx),
LibraryName::AArmours(idx) => format!("AArmour/{:02}", idx),
LibraryName::ARArmours(idx) => format!("ARArmour/{:02}", idx),
LibraryName::CHair(idx) => format!("CHair/{:02}", idx),
LibraryName::AHair(idx) => format!("AHair/{:02}", idx),
LibraryName::ARHair(idx) => format!("ARHair/{:02}", idx),
LibraryName::CWeapons(idx) => format!("CWeapon/{:02}", idx),
LibraryName::ARWeapons(idx) => format!("ARWeapon/{:02}", idx),
LibraryName::CHumEffect(idx) => format!("CHumEffect/{:02}", idx),
```

## 格式化对比表

| 类型 | C# 格式 | Rust (修复前) | Rust (修复后) | 示例文件 |
|------|---------|--------------|--------------|---------|
| 角色装备 | `"00"` | `{:04}` ❌ | `{:02}` ✅ | `00.Lib` |
| 角色武器 | `"00"` | `{:04}` ❌ | `{:02}` ✅ | `00.Lib` |
| 角色发型 | `"00"` | `{:04}` ❌ | `{:02}` ✅ | `00.Lib` |
| 怪物 | `"000"` | `"000"` ✅ | `"000"` ✅ | `000.Lib` |

### Rust 格式化语法

```rust
// {:02} = 2位数字,不足补0
format!("{:02}", 0)  // "00"
format!("{:02}", 1)  // "01"
format!("{:02}", 99) // "99"

// {:04} = 4位数字,不足补0 (错误)
format!("{:04}", 0)  // "0000"
format!("{:04}", 1)  // "0001"
```

## 验证测试

### 测试步骤

1. **编译代码**:
   ```powershell
   cd D:\Users\gxh\Documents\GitHub\Crystal\ClientRust
   cargo build
   ```

2. **运行游戏**:
   ```powershell
   cargo run
   ```

3. **预期日志**:
   ```
   INFO  🔄 懒加载库: CArmours(0)
   INFO  加载库: CArmours(0) (Data/CArmour/00)
   INFO  ✓ 成功加载 CArmours(0) (1616 张图像)
   TRACE ✅ CArmours(0)[16] 纹理绘制成功
   ```

4. **预期效果**:
   - ✅ 角色正常显示
   - ✅ 站立动画循环
   - ✅ 无加载错误日志

### 文件验证命令

```powershell
# 检查文件是否存在
Test-Path "Data/CArmour/00.Lib"  # True ✅
Test-Path "Data/CArmour/0000.Lib"  # False ❌

# 列出所有装备库文件
Get-ChildItem "Data/CArmour" | Select-Object -First 10 Name
```

**输出**:
```
Name
----
00.Lib
01.Lib
02.Lib
03.Lib
04.Lib
...
```

## 技术细节

### C# String.ToString() 方法

```csharp
int num = 5;
num.ToString("00");   // "05" (2位)
num.ToString("000");  // "005" (3位)
num.ToString("0000"); // "0005" (4位)

// "00" 是标准格式字符串
// 0 = 零占位符,必须出现一个数字
```

### Rust format! 宏

```rust
let num = 5;
format!("{:02}", num);  // "05" (2位,对应 C# "00")
format!("{:03}", num);  // "005" (3位,对应 C# "000")
format!("{:04}", num);  // "0005" (4位,对应 C# "0000")

// {:02} 语法
// : = 格式化开始
// 0 = 用 0 填充
// 2 = 最小宽度 2 位
```

### 为什么之前没发现?

1. **编译时无错误**: 路径格式化是运行时操作
2. **懒加载延迟**: 只有在绘制角色时才尝试加载
3. **错误日志不明显**: 只显示"未加载",没有显示尝试的路径
4. **文件名相似**: `0000.Lib` vs `00.Lib` 容易忽略

## 相关文件

### 修改的文件
- `src/graphics/libraries.rs` - 修复 9 个装备库路径格式

### 参考文件
- `Client/MirGraphics/MLibrary.cs` - C# 原版实现
  - Line 153: CArmours 初始化
  - Line 261-278: InitLibrary() 方法
  - Line 277: `i.ToString(toStringValue)` 关键代码

### 相关文档
- `BUGFIX_角色装备ID负值问题.md` - 装备ID负值修复
- `FEATURE_角色动画系统实现.md` - 角色动画系统
- `角色绘制系统移植指南.md` - C# 源码分析

## 经验教训

### 1. 文件路径验证
- ✅ **应该做**: 在代码中添加文件存在性检查
- ❌ **不应该做**: 假设路径格式,应该验证实际文件

### 2. 错误日志改进
```rust
// ❌ 不够详细
tracing::error!("❌ 装备库 CArmours(0) 未加载");

// ✅ 更好的日志
tracing::error!("❌ 装备库加载失败: CArmours(0) -> 路径: {} -> 错误: {}", 
    path, error);
```

### 3. 单元测试
```rust
#[test]
fn test_library_path_format() {
    assert_eq!(LibraryName::CArmours(0).default_path(), "CArmour/00");
    assert_eq!(LibraryName::CArmours(5).default_path(), "CArmour/05");
    assert_eq!(LibraryName::CArmours(99).default_path(), "CArmour/99");
}
```

## 提交信息

```
fix: 修复装备库路径格式错误 (4位→2位)

问题:
- Rust 使用 {:04} 生成 0000.Lib (4位数字)
- C# 使用 ToString("00") 生成 00.Lib (2位数字)
- 实际文件是 00.Lib, 导致 Rust 找不到文件

修复:
- 改为 {:02} 格式化 (对应 C# "00")
- 适用于 9 个装备库类型
- CArmours, AArmours, ARArmours, CHair 等

效果:
- 懒加载现在能正确找到文件
- 角色可以正常显示
- 无加载错误

参考: Client/MirGraphics/MLibrary.cs line 277
Issues: #装备库路径错误
```

## 后续优化

### 短期
- [ ] 添加路径验证测试
- [ ] 改进错误日志 (显示完整路径)
- [ ] 添加文件存在性预检查

### 中期
- [ ] 统一路径管理 (避免硬编码格式)
- [ ] 自动检测文件命名规则
- [ ] 支持自定义路径配置

### 长期
- [ ] 热重载支持
- [ ] 路径映射配置文件
- [ ] 资源包系统
