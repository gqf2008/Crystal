# 🎒 物品堆叠和分离功能指南

## 📋 功能概述

物品堆叠和分离系统为传奇2背包添加了完整的物品管理功能，支持相同物品的自动合并、手动分离，以及灵活的数量控制。

## 🎯 核心特性

### 1. 智能堆叠系统
- **自动合并**: 相同物品拖拽时自动合并
- **容量限制**: 每种物品类型有不同的最大堆叠数量
- **溢出处理**: 超过最大堆叠数量时自动分离多余部分

### 2. 分离操作
- **Shift+点击**: 快速分离一半数量
- **Ctrl+点击**: 自定义分离数量（弹出对话框）
- **空格子检查**: 自动寻找空格子进行分离

### 3. 堆叠限制设置
```rust
// 不同物品类型的最大堆叠数量
武器 (0-49):     max_stack: 1    // 不可堆叠
防具 (50-99):    max_stack: 1    // 不可堆叠
药水 (100-199):  max_stack: 250  // 可堆叠250个
任务物品 (300-349): max_stack: 100  // 可堆叠100个
特殊物品 (其他):   max_stack: 50   // 可堆叠50个
```

## 🎮 操作指南

### 基础操作
1. **普通点击**: 选择物品或移动到目标位置
2. **拖拽合并**: 将相同物品拖到已有物品上自动合并
3. **右键菜单**: 显示简洁的使用菜单

### 分离操作
1. **快速分离 (Shift+点击)**:
   - 按住Shift键点击可堆叠物品
   - 自动分离一半数量到空格子
   - 例：10个药水 → 5个 + 5个

2. **自定义分离 (Ctrl+点击)**:
   - 按住Ctrl键点击可堆叠物品
   - 弹出数量选择对话框
   - 输入要分离的数量 (1 到 最大数量-1)
   - 点击"确认"完成分离

### 合并规则
- **完全合并**: 总数量 ≤ 最大堆叠数 → 全部合并到目标格子
- **部分合并**: 总数量 > 最大堆叠数 → 目标格子填满，源格子保留剩余

## 🔧 技术实现

### 核心方法
```rust
// 堆叠检查和处理
fn try_stack_items(&mut self, selected: SelectedItem, target_container: ItemContainer, 
                   target_index: usize, target_slot: ItemSlot)

// 完全合并物品
fn merge_items_completely(&mut self, selected: SelectedItem, target_container: ItemContainer, 
                         target_index: usize, total_count: u32)

// 部分合并物品  
fn merge_items_partially(&mut self, selected: SelectedItem, target_container: ItemContainer,
                        target_index: usize, max_stack: u32, remaining: u32)

// Shift+点击分离一半
fn handle_item_split_half(&mut self, container: ItemContainer, index: usize)

// Ctrl+点击自定义分离
fn handle_item_split_custom(&mut self, container: ItemContainer, index: usize)

// 查找空格子
fn find_empty_slot(&self, container: ItemContainer) -> Option<usize>
```

### 数据结构
```rust
pub struct InventoryDialog {
    // ... 其他字段 ...
    
    /// 数量选择对话框状态
    quantity_dialog_visible: bool,
    quantity_dialog_item: Option<SelectedItem>,
    quantity_input: String,
    quantity_max: u32,
}

struct ItemInfo {
    // ... 其他字段 ...
    
    /// 最大堆叠数量
    max_stack: u32,
}
```

## 🎨 用户界面

### 数量选择对话框
- **标题**: "选择分离数量"
- **信息显示**: 显示最多可分离的数量
- **输入框**: 允许输入1到最大数量-1的值
- **按钮**: "确认" (绿色) 和 "取消" (红色)
- **样式**: 传奇2风格的金色边框和深色背景

### 视觉反馈
- **合并成功**: 控制台输出 "🔄 物品完全合并: 格子X -> 格子Y，数量Z"
- **部分合并**: 控制台输出 "🔄 物品部分合并: 格子X剩余Y, 格子Z填满W"
- **分离成功**: 控制台输出 "✂️ Shift+点击分离: 格子X 剩余Y，格子Z 分离W"
- **错误提示**: 显示相应的警告信息

## 🧪 测试用例

### 堆叠测试
1. **药水合并**: 将5个药水拖到3个药水上 → 合并为8个药水
2. **溢出处理**: 将200个药水拖到100个药水上 → 目标格子250个，源格子50个
3. **武器堆叠**: 尝试合并武器 → 进行交换而非合并

### 分离测试
1. **Shift分离**: Shift+点击10个药水 → 分离为5个+5个
2. **Ctrl分离**: Ctrl+点击20个药水，输入8 → 分离为12个+8个
3. **空格子不足**: 在背包满的情况下尝试分离 → 显示"没有空格子"提示

## 🔮 未来扩展

1. **智能分离**: 自动选择最合适的空格子位置
2. **批量操作**: 支持选择多个物品进行批量分离
3. **快捷键绑定**: 允许用户自定义分离快捷键
4. **动画效果**: 为分离和合并添加平滑的视觉动画

## 📝 更新日志

### v1.0.0 (2024-11-17)
- ✅ 实现基础堆叠合并功能
- ✅ 添加Shift+点击分离一半功能
- ✅ 添加Ctrl+点击自定义分离功能
- ✅ 实现数量选择对话框
- ✅ 支持不同物品类型的堆叠限制
- ✅ 完善错误处理和用户反馈

---
*传奇2背包系统 - 现代化实现，经典体验*