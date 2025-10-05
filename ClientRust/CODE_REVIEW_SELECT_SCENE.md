# select_scene.rs 代码审查报告

## 审查日期
2025年10月5日

## 审查发现的问题

### ❌ 问题 1: 创建了不存在的 `SelectCharacter` 结构体

**严重性**: 🔴 高 - 违反审查标准 #2

**问题描述**:
代码创建了一个新的 `SelectCharacter` 结构体，但这个结构体在 C# 中不存在。C# 使用 `SelectInfo` 类（定义在 `Shared/Data/SharedData.cs`），并且 SharedRust 中已经有对应的实现。

**C# 原版**:
```csharp
// Shared/Data/SharedData.cs line 3
public class SelectInfo
{
    public int Index;
    public string Name = string.Empty;
    public ushort Level;
    public MirClass Class;
    public MirGender Gender;
    public DateTime LastAccess;
}

// Client/MirScenes/SelectScene.cs line 20
public List<SelectInfo> Characters = new List<SelectInfo>();
```

**SharedRust 中已存在**:
```rust
// SharedRust/src/data/client_data.rs line 15
pub struct SelectInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub last_access: DateTime<Utc>,
}
```

**错误的 Rust 代码**:
```rust
/// ❌ 不应该存在！SharedRust 中已有 SelectInfo
#[derive(Debug, Clone)]
pub struct SelectCharacter {
    pub index: u32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub exists: bool,  // ❌ C# 中没有这个字段
}
```

**修复**:
```rust
// ✅ 使用 SharedRust 中已定义的 SelectInfo
use mir2_shared::SelectInfo;
```

---

### ❌ 问题 2: 错误的数据结构设计

**严重性**: 🔴 高 - 违反审查标准 #1

**问题描述**:
`SelectScene` 的字段设计与 C# 原版不一致。

**C# 原版**:
```csharp
public class SelectScene : MirScene
{
    public List<SelectInfo> Characters = new List<SelectInfo>();
    private int _selected;
    private NewCharacterDialog _character;
    
    public SelectScene(List<SelectInfo> characters)
    {
        Characters = characters;
        SortList();
        // ... UI initialization
    }
}
```

**错误的 Rust 代码**:
```rust
pub struct SelectScene {
    // ❌ 错误：使用 Option 包装，C# 中是 List
    pub characters: Vec<Option<SelectCharacter>>,
    
    // ❌ 错误：类型应该是 i32，不是 usize
    pub selected_index: usize,
    
    // ❌ C# 中不存在这些字段
    pub creating_character: bool,
    pub deleting_character: bool,
    pub new_char_name: String,
    pub new_char_class: MirClass,
    pub new_char_gender: MirGender,
    
    // ❌ 不应该总是存在，C# 中是按需创建
    pub character_creation_dialog: CharacterCreationDialog,
    pub character_deletion_dialog: CharacterDeletionDialog,
    
    // ❌ 使用了不存在的 egui 依赖
    pub character_preview_textures: HashMap<usize, egui::TextureHandle>,
}
```

**修复后**:
```rust
pub struct SelectScene {
    /// Mirrors C# `public List<SelectInfo> Characters`
    pub characters: Vec<SelectInfo>,
    
    /// Mirrors C# `private int _selected`
    pub selected_index: i32,
    
    /// Mirrors C# `private NewCharacterDialog _character`
    pub character_creation_dialog: Option<CharacterCreationDialog>,
    pub character_deletion_dialog: Option<CharacterDeletionDialog>,
    
    // TODO Phase 3: Add UI controls (see C# code)
}
```

---

### ❌ 问题 3: 构造函数签名不匹配

**严重性**: 🟡 中 - 违反审查标准 #1

**C# 原版**:
```csharp
public SelectScene(List<SelectInfo> characters)
{
    Characters = characters;
    SortList();
    // ...
}
```

**错误的 Rust 代码**:
```rust
pub fn new() -> Self {
    Self {
        characters: vec![None; 4],  // ❌ 硬编码 4 个空槽位
        // ...
    }
}
```

**修复后**:
```rust
pub fn new(characters: Vec<SelectInfo>) -> Self {
    let mut scene = Self {
        characters,
        selected_index: 0,
        character_creation_dialog: None,
        character_deletion_dialog: None,
    };
    scene.sort_list();
    scene
}
```

---

### ❌ 问题 4: 缺少 `SortList()` 方法

**严重性**: 🟡 中 - 违反审查标准 #1

**C# 原版**:
```csharp
public void SortList()
{
    if (Characters != null)
        Characters.Sort((c1, c2) => c2.LastAccess.CompareTo(c1.LastAccess));
}
```

**原代码**: 没有实现

**修复后**:
```rust
fn sort_list(&mut self) {
    self.characters.sort_by(|a, b| b.last_access.cmp(&a.last_access));
}
```

---

### ❌ 问题 5: 方法签名与 C# 不一致

**严重性**: 🟡 中

**问题**: 方法参数类型不匹配

**C# 原版**:
```csharp
private int _selected;  // int 类型

_selected = 0;
_selected = 1;
```

**错误的 Rust 代码**:
```rust
pub fn select_character(&mut self, index: usize) {  // ❌ 应该是 i32
    // ...
}
```

**修复后**:
```rust
pub fn select_character(&mut self, index: i32) {
    if index >= 0 && (index as usize) < self.characters.len() {
        self.selected_index = index;
        // ...
    }
}
```

---

### ❌ 问题 6: 添加了不存在的依赖

**严重性**: 🔴 高 - 导致编译失败

**问题**: 使用了未声明的 `egui` 依赖

```rust
pub character_preview_textures: HashMap<usize, egui::TextureHandle>,
//                                              ^^^^ 未定义
```

**修复**: 完全移除这个字段，C# 中也不存在。

---

## 审查结果统计

### 违反审查标准的问题

| 标准 | 问题数 | 严重性 |
|------|--------|--------|
| 标准 #1: 与 C# 逻辑一致 | 4 | 🟡 中-高 |
| 标准 #2: 禁止创建不存在的结构 | 1 | 🔴 高 |
| 标准 #3: 禁止过度抽象 | 0 | - |
| 标准 #4: 禁止提前重构 | 1 | 🟡 中 |

### 修复后的对应关系

| C# 代码 | 原 Rust 代码 | 修复后 Rust 代码 |
|---------|-------------|-----------------|
| `SelectInfo` | `SelectCharacter` ❌ | `SelectInfo` (from mir2_shared) ✅ |
| `List<SelectInfo> Characters` | `Vec<Option<SelectCharacter>>` ❌ | `Vec<SelectInfo>` ✅ |
| `int _selected` | `usize selected_index` ❌ | `i32 selected_index` ✅ |
| `NewCharacterDialog _character` | 总是存在的实例 ❌ | `Option<CharacterCreationDialog>` ✅ |
| 构造函数接受参数 | 无参数 ❌ | `new(characters: Vec<SelectInfo>)` ✅ |
| `SortList()` | 不存在 ❌ | `sort_list()` ✅ |

---

## 修复验证

### 编译状态
✅ `select_scene.rs` 修复后可以编译（其他模块有问题但与此无关）

### 测试状态
✅ 添加了 3 个测试：
1. `test_select_scene_creation` - 验证基本创建
2. `test_character_selection` - 验证选择逻辑
3. `test_sort_characters_by_last_access` - 验证排序功能

---

## 教训总结

### ❌ 不要做的事

1. **不要创建 SharedRust 中已存在的结构体** - 总是先检查 `mir2_shared`
2. **不要假设数据结构** - 仔细对照 C# 原版
3. **不要添加未声明的依赖** - 检查 Cargo.toml
4. **不要改变核心类型** - `int` → `i32`，不是 `usize`
5. **不要添加 C# 中不存在的字段** - `exists`, `creating_character` 等

### ✅ 应该做的事

1. **使用 grep_search 查找原版定义** - 找到 C# 代码的准确位置
2. **检查 SharedRust** - 看是否已有对应的数据结构
3. **保持构造函数签名一致** - 包括参数和初始化逻辑
4. **实现所有核心方法** - 如 `SortList()`
5. **添加明确的注释** - 标注对应的 C# 代码

---

## 修复摘要

**修复的问题**: 6 个
**修复的文件**: 1 个 (`select_scene.rs`)
**删除的代码**: ~40 行（错误的结构体和字段）
**添加的代码**: ~80 行（正确的实现和测试）
**代码行数变化**: 从 235 行 → 约 275 行

**结论**: ✅ 修复后的代码现在与 C# 原版对齐
