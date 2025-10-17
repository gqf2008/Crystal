# Bevy 开发快速参考

## 🚀 常用命令

```bash
# 编译 Bevy 版本
cargo build --bin mir2_bevy

# 运行 Bevy 版本
cargo run --bin mir2_bevy

# 编译 ggez 版本 (对比)
cargo build --bin mir2_client

# 检查代码 (快速)
cargo check --bin mir2_bevy

# 清理构建
cargo clean
```

## 📂 关键文件

| 文件 | 用途 |
|------|------|
| `src/bin/main_bevy.rs` | Bevy 程序入口 |
| `src/bevy/components.rs` | ECS 组件定义 |
| `src/bevy/systems/*.rs` | 系统实现 |
| `src/bevy/assets.rs` | MLibrary 加载 |
| `Cargo.toml` | 依赖配置 |

## 🧩 添加新组件

```rust
// src/bevy/components.rs
#[derive(Component)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}
```

## ⚙️ 添加新系统

```rust
// src/bevy/systems/health.rs
pub fn health_system(
    mut query: Query<&mut Health>,
) {
    for mut health in query.iter_mut() {
        // 处理生命值逻辑
    }
}

// 在 main_bevy.rs 中注册
.add_systems(Update, health_system)
```

## 🎮 生成实体

```rust
commands.spawn((
    Player,
    GridPosition::new(0, 0),
    Movement::new(),
    Transform::default(),
));
```

## 📊 查询实体

```rust
// 查询单个
fn system(query: Query<&Transform, With<Player>>) {
    if let Ok(transform) = query.get_single() {
        // ...
    }
}

// 查询多个
fn system(query: Query<(&Transform, &Health)>) {
    for (transform, health) in query.iter() {
        // ...
    }
}

// 可变查询
fn system(mut query: Query<&mut Health>) {
    for mut health in query.iter_mut() {
        health.current -= 1;
    }
}
```

## 🌍 访问资源

```rust
fn system(
    config: Res<GameConfig>,
    mut mlibrary: ResMut<MLibraryResource>,
) {
    println!("Cell size: {}", config.cell_width);
}
```

## 🎯 状态切换

```rust
// 在系统中切换状态
fn system(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Login);
}

// 条件运行系统
.add_systems(Update, system.run_if(in_state(GameState::Game)))

// 状态进入/退出钩子
.add_systems(OnEnter(GameState::Game), setup_game)
.add_systems(OnExit(GameState::Game), cleanup_game)
```

## 🎨 加载精灵

```rust
fn spawn_sprite(
    mut commands: Commands,
    mlibrary: Res<MLibraryResource>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Some(texture) = mlibrary.loader.load_sprite(0, 0, &mut images) {
        commands.spawn((
            Sprite { image: texture, ..default() },
            Transform::default(),
        ));
    }
}
```

## ⏱️ 使用时间

```rust
fn system(time: Res<Time>) {
    let delta = time.delta_secs();
    let elapsed = time.elapsed_secs();
}
```

## 🐛 调试技巧

```rust
// 打印组件
fn debug_system(query: Query<(Entity, &GridPosition)>) {
    for (entity, pos) in query.iter() {
        println!("Entity {:?} at ({}, {})", entity, pos.x, pos.y);
    }
}

// 条件运行 (每N秒)
if time.elapsed_secs() as u32 % 5 == 0 {
    println!("Debug info");
}
```

## 📝 最佳实践

### ✅ 做
- 使用组件存储数据
- 使用系统处理逻辑
- 查询要明确 (With/Without)
- 系统保持小而专注

### ❌ 不做
- 在组件中存储逻辑
- 在系统中存储状态
- 过度使用全局资源
- 创建大而全的系统

## 🔧 性能优化

```rust
// 使用批量操作
fn batch_update(mut query: Query<&mut Transform>) {
    query.par_iter_mut().for_each(|mut transform| {
        // 并行处理
    });
}

// 使用 Changed 过滤器
fn system(query: Query<&Transform, Changed<Transform>>) {
    // 只处理改变的组件
}
```

## 📚 更多资源

- [Bevy Cheat Book](https://bevy-cheatbook.github.io/)
- [Bevy API Docs](https://docs.rs/bevy/)
- [Bevy Assets](https://bevyengine.org/assets/)
