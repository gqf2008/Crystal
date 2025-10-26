# 游戏渲染问题修复记录

## 2024年最新修复

### 1. ✅ 中文乱码问题 (H键帮助面板)

**问题**: 按H键显示的快捷键帮助面板中文显示为乱码

**根本原因**: `HotkeyHelpPanel`使用`Text::new`创建文本,没有指定字体,GGEZ默认不支持中文

**解决方案**:
1. 在`HotkeyHelpPanel`添加`font_name: Option<String>`字段
2. 添加`set_font(&mut self, font_name: String)`方法
3. 所有文本使用`TextFragment::new(text).font(font_name)`设置字体
4. 在`GameScene::new`中调用`hotkey_help.set_font(ui_font_name.clone())`

**修改文件**:
- `src/ui/hotkey_help.rs`: 添加字体支持
- `src/ecs/scenes/game_scene.rs`: 设置字体名称

---

### 2. ✅ Front层火把遮挡问题

**问题**: 人物被Front层的火把遮挡,但人物的Y坐标其实在火把下面

**根本原因**: Front层在所有实体(Monster/NPC/Player)之后绘制,导致Front层永远在最上层

**解决方案**:
1. 将Front层瓦片添加到Y-sorting系统
2. 每个Front瓦片按其Y坐标(`grid_y * CELL_HEIGHT`)参与排序
3. 在`EntityType`枚举添加`FrontTile(Entity)`
4. 在绘制循环中处理FrontTile,调用`RenderSystem::draw_tile_fast`
5. 移除单独绘制Front层的代码

**效果**: 
- Front层瓦片现在按Y坐标正确排序
- Y坐标更下方的人物会显示在Front瓦片前面
- Y坐标更上方的人物会被Front瓦片遮挡

**修改文件**:
- `src/ecs/scenes/game_scene.rs`:
  - 添加`EntityType::FrontTile`
  - 收集Front瓦片参与Y-sorting
  - 在绘制循环中处理FrontTile
  - 移除单独绘制Front层的代码

---

### 3. ⏳ NPC闪烁问题 (调查中)

**问题**: 公告牌NPC时有时无,差不多1秒交替出现一次

**当前调查**:
- 终端日志没有`ObjectRemoved`事件,说明NPC实体没有被删除
- 可能是渲染条件问题,例如纹理加载失败或渲染判断逻辑

**已采取措施**:
1. 将Y-sorting调试日志从`debug`提升到`info`级别
2. 日志显示monster/NPC/front_tiles数量
3. 等待用户测试查看日志输出

**待验证**:
- 检查`RenderSystem::draw_single_npc`的条件
- 检查纹理加载是否成功
- 查看日志确认NPC是否一直在`entities_to_draw`中

---

### 4. ✅ 窗口缩放问题 (无需修复)

**问题**: 窗口最小化恢复后游戏画面被放大,主控面板位置不对

**调查结果**:
- `game_app.rs::resize_event`正确处理了`scale_factor`
- 物理像素转换为逻辑像素: `logical = physical / scale_factor`
- `Camera.zoom`保持不变(初始值1.25)
- 只更新`screen_width`和`screen_height`

**结论**: 代码行为正确,可能是用户误解或其他问题。需要用户提供更多细节。

---

## 调试工具

### 按键绑定
- **F9**: 切换NPC边框绘制
- **F10**: 切换Monster边框绘制
- **F11**: 切换特效边框绘制
- **H**: 显示/隐藏快捷键帮助面板 (现已支持中文)
- **G**: 切换网格显示
- **O**: 切换障碍物显示
- **P**: 切换寻路路径显示

### 日志输出
```rust
// 每秒输出一次Y-sorting信息
tracing::info!("🎯 Y-sorting: {} monsters, {} NPCs, {} front tiles", 
              monster_count, npc_count, front_tile_count);
```

---

## 技术细节

### Y-Sorting渲染顺序
1. **Back层**: 地面、草地等
2. **Middle层**: 墙壁、建筑底部等
3. **Y-Sorting实体** (按Y坐标从小到大):
   - Monster (Y坐标)
   - NPC (Y坐标)
   - Player (Y坐标)
   - **Front层瓦片** (grid_y * CELL_HEIGHT)
4. **地面物品**: 金币、物品等
5. **血条和名称**: 始终在最上层
6. **调试图形**: 网格、障碍物、路径

### Front层Y坐标计算
```rust
// 使用瓦片底部的Y坐标参与排序
let tile_y = tile.grid_y * CELL_HEIGHT; // CELL_HEIGHT = 48
entities_to_draw.push((tile_y, EntityType::FrontTile(entity)));
```

### 字体加载
```rust
// GameScene::load_chinese_font
// 优先加载: C:/Windows/Fonts/msyh.ttc (Microsoft YaHei)
// 备选: C:/Windows/Fonts/simsun.ttc (SimSun)
```

---

## 下一步工作

1. **NPC闪烁**: 等待用户测试,查看日志输出,确定是否是渲染条件问题
2. **窗口缩放**: 等待用户提供更多细节
3. **装备渲染**: 继续实现衣服、装饰物渲染 (武器已完成)
4. **地面特效**: 实现地面光效等特效系统

---

**修复日期**: 2024年
**修复人**: GitHub Copilot
