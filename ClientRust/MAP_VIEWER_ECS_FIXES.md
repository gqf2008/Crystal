# Map Viewer ECS 编译错误修复指南

## 需要修复的问题:

### 1. TileLayer 排序问题
**错误**: `TileLayer` 没有实现 `Ord` trait
**修复**: 为 `TileLayer` 派生 `PartialOrd` 和 `Ord`

### 2. MLibrary API 不匹配
**错误**: `get_image()` 方法不存在
**修复**: 使用 `get_image_ggez()` 方法

### 3. CellInfo 字段名错误
**错误**: 没有 `can_walk` 和 `back_animation_frame` 字段
**实际字段**:
- `back_animation_frame` 不存在,只有 `front_animation_frame` 和 `middle_animation_frame`
- `can_walk` 不存在,需要检查其他字段

### 4. MapReader API 不匹配
**错误**: `load_from_file()` 方法不存在
**修复**: 使用 `MapReader::new(path)` 替代

### 5. GGEZ KeyInput API 变化
**错误**: `KeyInput` 没有 `keycode` 字段
**修复**: 使用 `input.event` 来获取按键

### 6. KeyCode 枚举值
**错误**: GGEZ 0.10 的 KeyCode 使用不同的命名
**修复**: 使用正确的 KeyCode 值 (例如 `Key::M` 而不是 `KeyCode::M`)

### 7. main() 返回类型
**错误**: `event::run()` 不返回 `GameResult`
**修复**: main() 返回 `()`

## 完整修复版本见下文
