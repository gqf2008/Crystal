# DrawableMapObject 对象系统实现完成

## 完成的工作

### 1. 创建DrawableMapObject Trait
- 文件: `src/objects/drawable.rs`
- 定义了所有可绘制地图对象必须实现的接口:
  - `draw()` - 绘制对象
  - `object_id()` - 获取对象ID
  - `is_dead()` - 检查是否死亡
  - `is_hidden()` - 检查是否隐藏
  - `draw_priority()` - 获取绘制优先级 (Items=0, Spells=1, Other=2)

### 2. 所有对象类型实现DrawableMapObject Trait

已实现的对象类型:
- ✅ **ItemObject** (地面物品)
  - draw_priority = 0 (最先绘制)
  - 不会死亡
  
- ✅ **SpellObject** (飞行法术)
  - draw_priority = 1 (在物品之后)
  - expired 作为死亡标记
  
- ✅ **PlayerObject** (玩家基类)
  - draw_priority = 2 (在物品和法术之后)
  - 调用现有draw()方法
  
- ✅ **UserObject** (当前玩家)
  - 委托给PlayerObject的draw()实现
  
- ✅ **HeroObject** (英雄伙伴)
  - 委托给PlayerObject的draw()实现
  - spawn_state检查死亡和隐藏
  
- ✅ **MonsterObject** (怪物)
  - draw_priority = 2
  - 支持骷髅/尸体渲染
  
- ✅ **NPCObject** (NPC)
  - draw_priority = 2
  - NPC不会死亡

### 3. CellInfo绘制方法实现

文件: `src/objects/map_code.rs`

实现了两个关键方法:

```rust
/// 绘制所有存活的对象
pub fn draw_objects(
    &self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>,
    draw_location: Point,
) -> GameResult

/// 绘制所有死亡的对象(尸体等)
pub fn draw_dead_objects(
    &self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>,
    draw_location: Point,
) -> GameResult
```

对应C# `Client/MirObjects/MapCode.cs`:
- `DrawObjects()` lines 55-82
- `DrawDeadObjects()` lines 85-113

### 4. 模块导出更新

文件: `src/objects/mod.rs`
- 导出 `DrawableMapObject` trait
- 导出 `CellInfo` 和 `MapReader`

## 架构说明

### 正确的渲染流程

```
GameScene::draw_map()
  ├─> Layer 1-3: 绘制地砖 (BackImage, MiddleImage, FrontImage)
  └─> 遍历每个单元格:
        ├─> cell.draw_dead_objects() - 绘制尸体
        ├─> 绘制瓷砖动画
        └─> cell.draw_objects() - 绘制活动对象
              └─> 对每个对象调用 object.draw()
```

### 与C#对应关系

**C# 架构**:
```csharp
GameScene.DrawObjects() {
    // 瓷砖动画
    TileAnimationImage
    MiddleImage (animated)
    FrontImage (large objects)
    
    // 对象渲染
    for (y...) {
        for (x...) {
            M2CellInfo[x,y].DrawObjects() {
                foreach (obj in CellObjects) {
                    obj.Draw()  // 多态调用
                }
            }
        }
    }
}
```

**Rust 架构** (现在实现):
```rust
GameScene::draw_map() {
    // 瓷砖层 1-3
    
    // 对象渲染
    for y in start_y..=end_y {
        for x in start_x..=end_x {
            cell.draw_objects(ctx, canvas, &objects_map, draw_location)?;
        }
    }
}
```

## 下一步工作

### 1. GameScene集成 (高优先级)
需要在 `game_scene.rs` 中:
- 创建 `objects_map: HashMap<u32, Box<dyn DrawableMapObject>>`
- 移除Layer 4-6的直接瓷砖绘制
- 添加cell.draw_objects()调用

### 2. 对象管理器
需要实现对象的创建、添加、移除:
- 从服务器包创建对象 (ObjectPlayer, ObjectMonster, ObjectNpc等)
- 将对象添加到objects_map和cell.cell_objects
- 移除对象时从两处删除

### 3. 完整绘制实现
当前所有draw()方法都是TODO占位符,需要实现:
- 获取纹理库
- 计算帧索引
- 应用特效和混合模式
- 正确的位置和层级

### 4. 对象排序
CellInfo::sort()需要完整实现C#逻辑:
- Items优先级最高
- Spells次之
- 死亡对象排后面
- 按ObjectID排序

### 5. 瓷砖动画
从Layer 4-6移到独立方法:
- TileAnimationImage (Library 190)
- MiddleImage animated
- 在draw_objects()之前调用

## 编译状态

✅ **编译成功** - 无错误,仅有unused变量警告

所有对象类型已正确实现DrawableMapObject trait,代码结构与C#架构一致。

## 文件修改列表

新建文件:
- `src/objects/drawable.rs`

修改文件:
- `src/objects/item_object.rs`
- `src/objects/spell_object.rs`
- `src/objects/player_object.rs`
- `src/objects/user_object.rs`
- `src/objects/hero_object.rs`
- `src/objects/monster_object.rs`
- `src/objects/npc_object.rs`
- `src/objects/map_code.rs`
- `src/objects/mod.rs`

## 技术细节

### Trait对象使用
使用 `Box<dyn DrawableMapObject>` 实现多态:
- 允许不同类型对象存储在同一集合
- 运行时动态调用正确的draw()方法
- 与C#的多态继承效果相同

### 方法签名统一
所有draw()方法签名一致:
```rust
fn draw(&self, ctx: &mut Context, canvas: &mut Canvas, draw_location: Point) -> GameResult
```

### 错误处理
使用ggez的GameResult进行错误传播,允许在绘制失败时正确返回错误。
