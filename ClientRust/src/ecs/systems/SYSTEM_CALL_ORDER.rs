// ============================================================================
// 五层架构系统调用顺序参考
// ============================================================================
// 
// 这个文件展示了在游戏主循环中应该如何按顺序调用各层系统
// 确保数据流向正确：Layer 1 → Layer 2 → Layer 3 → Layer 4 → Layer 5
// 
// ============================================================================

/*
在 GameApp 的 update() 方法中，应该按以下顺序调用系统：

pub fn update(&mut self, dt: f32) {
    // ========================================================================
    // Layer 1: 输入与网络层
    // ========================================================================
    // 职责：捕获原始输入和网络数据，转换为组件
    
    // 1. 输入收集系统（每帧）
    InputCollectingSystem::update(
        &mut self.world,
        &self.ctx,           // ggez Context
        &self.mouse_state,   // 鼠标状态
        &self.key_state,     // 键盘状态
    );
    
    // 2. 客户端网络系统（处理接收到的网络事件）
    if let Some(game_client) = &mut self.game_client {
        while let Some(event) = game_client.poll_event() {
            self.client_network_system.process_event(&mut self.world, &event);
        }
    }
    
    // ========================================================================
    // Layer 2: 核心逻辑层
    // ========================================================================
    // 职责：游戏核心逻辑，包括预测、物理、校正、插值
    
    // 3. 本地预测系统（只对本地玩家）
    LocalPredictionSystem::update(
        &mut self.world,
        &self.map_data,      // 地图数据（用于寻路）
        dt,
    );
    
    // 4. 纯物理运动系统（所有实体）
    MovementSystemV2::update(&mut self.world, dt);
    
    // 5. 服务器校正系统（本地玩家）
    ReconciliationSystem::update(&mut self.world, dt);
    
    // 6. 插值系统（其他玩家/怪物/NPC）
    InterpolationSystem::update(&mut self.world, dt);
    
    // 7. 其他逻辑系统
    MonsterSystem::update(&mut self.world, dt);
    CombatSystem::update(&mut self.world, dt);
    MagicCastSystem::update(&mut self.world, dt);
    // ... 其他游戏逻辑系统
    
    // ========================================================================
    // Layer 3: 表现状态层
    // ========================================================================
    // 职责：根据游戏逻辑状态决定表现效果
    
    // 8. 动画状态系统（决定玩家应该播放什么动画）
    AnimationStateSystem::update(&mut self.world, dt);
    
    // 9. 怪物动画状态系统（决定怪物应该播放什么动画）✨ 新增
    MonsterAnimationStateSystem::update(&mut self.world);
    
    // 10. NPC动作决策系统（决定NPC应该播放什么动作）
    NPCActionSystem::update(&mut self.world, delta_ms);
    
    // 11. 音效触发系统（决定应该播放什么音效）
    SoundTriggerSystem::process_events(&mut self.world, &mut cmd, &events);
    
    // ========================================================================
    // Layer 4: 渲染层
    // ========================================================================
    // 职责：纯粹的渲染与动画播放，不处理游戏逻辑
    
    // 12. 相机系统（更新相机位置）
    CameraSystem::update(&mut self.world);
    
    // 13. 地图瓦片动画系统（更新地图动画帧）
    TileAnimationSystem::update(&mut self.world, animation_count);
    
    // 14. 动画播放系统（更新实体动画帧）
    AnimationPlaybackSystem::update(&mut self.world, delta_ms);
    
    // 15. 移动插值系统（计算移动时的屏幕偏移）
    MovementInterpolationSystem::update(&mut self.world);
    
    // 16. 音效播放系统（实际播放音效）✨ 新增
    SoundPlaybackSystem::update(&mut self, ctx, &mut self.world, &mut cmd)?;
    
    // 17. 渲染系统（实际绘制）
    // 在 draw() 方法中调用，不在 update() 中
    
    // ========================================================================
    // Layer 5: UI层
    // ========================================================================
    // 职责：UI更新和交互
    
    // 18. UI系统
    UISystem::update(&mut self.world, &self.ctx, dt);
    
    // 19. 物品系统 - 未来实现
    // ItemSystem::update(&mut self.world);
    
    // 20. 任务系统 - 未来实现（需要拆分为Layer 2进度跟踪 + Layer 5 UI交互）
    // QuestSystem::update(&mut self.world);
    
    // ========================================================================
    // 网络发送（在所有逻辑处理完之后）
    // ========================================================================
    
    // 21. 发送网络命令（基于 PlayerInputComponent）
    if let Some(game_client) = &mut self.game_client {
        self.client_network_system.send_commands(&mut self.world, game_client);
    }
}

pub fn draw(&mut self, ctx: &mut Context) -> GameResult {
    // ========================================================================
    // 渲染阶段（Layer 4）
    // ========================================================================
    
    graphics::clear(ctx, Color::BLACK);
    
    // 1. 渲染地图
    self.render_system.render_map(ctx, &self.map_data, &self.camera);
    
    // 2. 渲染实体（玩家、怪物、NPC、物品）
    self.render_system.render_entities(ctx, &self.world, &self.camera);
    
    // 3. 渲染UI（Layer 5）
    UISystem::render(ctx, &self.world);
    
    graphics::present(ctx)?;
    Ok(())
}
*/

// ============================================================================
// 关键设计原则
// ============================================================================

/*
1. **单向数据流**
   - Layer 1 写入原始数据 → Layer 2 读取并写入逻辑状态 → Layer 3 读取并写入表现状态 → Layer 4 只读取

2. **系统职责分离**
   - InputCollectingSystem: 只收集输入，不处理游戏逻辑
   - LocalPredictionSystem: 只预测，不发送网络
   - ClientNetworkSystem: 只收发网络，不处理游戏逻辑
   - AnimationStateSystem: 只决定动画状态，不播放动画
   - AnimationPlaybackSystem: 只播放动画（更新frame_index），不决定状态
   - TileAnimationSystem: 只更新地图瓦片动画
   - MovementInterpolationSystem: 只计算移动插值
   - NPCActionSystem: 只决定NPC动作切换

3. **服务调用时机**
   - PathfindingService: 在 Layer 2 的 LocalPredictionSystem 中调用
   - CollisionService: 在 Layer 2 的各种逻辑系统中调用
   - 服务是无状态的，可以在任何地方安全调用

4. **组件读写规则**
   - Layer 1 系统：写入 PlayerInputComponent, ServerStateComponent
   - Layer 2 系统：读取 Layer 1 组件，写入 VelocityComponent, Position, PredictionComponent, AIState
   - Layer 3 系统：读取 Layer 2 组件，写入 AnimationStateComponent, Animation.action, SoundTrigger
   - Layer 4 系统：读取所有组件，写入 Animation.frame_index, MapTile.image_index, MovementAnimation.offset_move, Camera.position

5. **系统职责检查清单**
   - ✅ MonsterSystem: 只更新 AIState, Position, Velocity（Layer 2）
   - ✅ PlayerSystem: 只更新 Position, Velocity（Layer 2）⚠️ 需移除相机和动画插值代码
   - ✅ AnimationStateSystem: 只更新 Animation.action（Layer 3）
   - ✅ MonsterAnimationStateSystem: 只更新 Animation.action（Layer 3）
   - ✅ AnimationPlaybackSystem: 只更新 Animation.frame_index（Layer 4）
   - ✅ CameraSystem: 只更新 Camera.position（Layer 4）
   - ✅ MovementInterpolationSystem: 只更新 MovementAnimation.offset_move（Layer 4）

6. **网络发送时机**
   - 在所有逻辑处理完之后
   - 基于 PlayerInputComponent 发送命令
   - 不在系统内部直接发送网络包

7. **错误处理**
   - 寻路失败：不移动，保持当前状态
   - 网络断开：显示UI提示，游戏暂停
   - 服务器校正：平滑插值，避免瞬移
*/

// ============================================================================
// 性能优化建议
// ============================================================================

/*
1. **系统执行频率**
   - InputCollectingSystem: 每帧（60 FPS）
   - LocalPredictionSystem: 每帧（60 FPS）
   - ClientNetworkSystem: 事件驱动（网络到达时）
   - MovementSystemV2: 每帧（60 FPS）
   - ReconciliationSystem: 每帧（60 FPS）
   - InterpolationSystem: 每帧（60 FPS）
   - AnimationStateSystem: 每帧（60 FPS）
   - NPCActionSystem: 每帧（60 FPS）
   - TileAnimationSystem: 每帧（60 FPS）
   - AnimationPlaybackSystem: 每帧（60 FPS）
   - MovementInterpolationSystem: 每帧（60 FPS）

2. **网络发送频率**
   - 移动命令: 最多每 50ms 发送一次（避免网络拥塞）
   - 技能命令: 立即发送
   - 聊天消息: 立即发送

3. **ECS查询优化**
   - 使用 .with<>() 过滤器减少迭代
   - 本地玩家查询：.with::<LocalPlayer>()
   - 其他玩家查询：.without::<LocalPlayer>()
   - 避免在循环中重复查询

4. **组件大小优化**
   - Position: 8 bytes (f32 x 2)
   - VelocityComponent: 12 bytes (f32 x 2 + max_speed)
   - 避免在组件中存储大数据（如路径点数组）
*/

// ============================================================================
// 调试建议
// ============================================================================

/*
1. **日志级别**
   - 输入系统: DEBUG（点击、移动目标）
   - 预测系统: INFO（寻路成功/失败）
   - 网络系统: INFO（命令发送、事件接收）
   - 校正系统: WARN（误差过大）

2. **性能监控**
   - 使用 tracing::span! 记录系统执行时间
   - 监控 ECS 查询耗时
   - 监控网络延迟和丢包率

3. **可视化调试**
   - 绘制预测路径（绿色线）
   - 绘制服务器权威位置（红点）
   - 绘制误差范围（圆圈）
   - 显示网络延迟（UI文字）
*/
