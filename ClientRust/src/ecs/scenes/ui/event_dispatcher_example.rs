//! 使用UIEventDispatcher重构LoginScene的示例
//! 
//! 这个文件展示了如何使用事件分发器来简化事件处理逻辑

/*
使用事件分发器后的LoginScene结构：

pub struct LoginScene {
    // 事件分发器
    event_dispatcher: UIEventDispatcher,
    
    // UI组件（不需要手动管理优先级）
    login_dialog: LoginDialog,
    new_account_dialog: Option<NewAccountDialog>,
    change_password_dialog: Option<ChangePasswordDialog>,
    virtual_keyboard: Option<VirtualKeyboard>,
    message_box: Option<MessageBox>,
    
    // ... 其他字段
}

impl LoginScene {
    pub fn new() -> Self {
        let mut dispatcher = UIEventDispatcher::new();
        
        // 定义UI层级（Z-order越大越靠前）
        dispatcher.add_layer(UILayer::new("background", 0));
        dispatcher.add_layer(UILayer::new("login_dialog", 10));
        // new_account_dialog, change_password_dialog, virtual_keyboard会在显示时动态添加
        
        Self {
            event_dispatcher: dispatcher,
            // ... 初始化其他字段
        }
    }
    
    // 显示新建账号对话框
    fn show_new_account_dialog(&mut self) {
        let dialog = NewAccountDialog::new();
        self.new_account_dialog = Some(dialog);
        
        // 添加到事件分发器（模态层）
        self.event_dispatcher.add_layer(
            UILayer::new("new_account_dialog", 20).modal()
        );
    }
    
    // 关闭新建账号对话框
    fn close_new_account_dialog(&mut self) {
        self.new_account_dialog = None;
        self.event_dispatcher.remove_layer("new_account_dialog");
    }
    
    // 显示虚拟键盘
    fn show_virtual_keyboard(&mut self, focused: FocusedInput) {
        let mut keyboard = VirtualKeyboard::new();
        keyboard.show(focused);
        self.virtual_keyboard = Some(keyboard);
        
        // 虚拟键盘不是模态的，允许底层显示悬停效果
        self.event_dispatcher.add_layer(
            UILayer::new("virtual_keyboard", 30)
        );
    }
}

impl Scene for LoginScene {
    fn on_mouse_move(&mut self, ctx: &mut Context, world: &mut World, x: f32, y: f32) -> GameResult {
        let (vx, vy) = self.screen_to_virtual(x, y);
        
        // 使用事件分发器自动处理优先级和传播
        self.event_dispatcher.dispatch_mouse_move(vx, vy, |layer_name| {
            match layer_name {
                "virtual_keyboard" => {
                    if let Some(keyboard) = &mut self.virtual_keyboard {
                        keyboard.on_mouse_move(vx, vy);
                        // 返回HandledContinue允许底层UI显示悬停效果
                        EventResult::HandledContinue
                    } else {
                        EventResult::Unhandled
                    }
                }
                "message_box" => {
                    if let Some(msg_box) = &mut self.message_box {
                        msg_box.on_mouse_move(vx, vy);
                        EventResult::Handled
                    } else {
                        EventResult::Unhandled
                    }
                }
                "change_password_dialog" => {
                    if let Some(dialog) = &mut self.change_password_dialog {
                        dialog.on_mouse_move(vx, vy);
                        EventResult::Handled
                    } else {
                        EventResult::Unhandled
                    }
                }
                "new_account_dialog" => {
                    if let Some(dialog) = &mut self.new_account_dialog {
                        dialog.on_mouse_move(vx, vy);
                        EventResult::Handled
                    } else {
                        EventResult::Unhandled
                    }
                }
                "login_dialog" => {
                    self.login_dialog.on_mouse_move(vx, vy);
                    EventResult::Handled
                }
                _ => EventResult::Unhandled
            }
        });
        
        Ok(())
    }
    
    fn on_mouse_down(&mut self, ctx: &mut Context, world: &mut World, button: MouseButton, x: f32, y: f32, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult {
        let (vx, vy) = self.screen_to_virtual(x, y);
        
        // 事件分发器自动处理模态对话框的阻塞
        self.event_dispatcher.dispatch_mouse_down(vx, vy, |layer_name| {
            match layer_name {
                "virtual_keyboard" => {
                    if let Some(keyboard) = &mut self.virtual_keyboard {
                        let action = keyboard.on_mouse_down(vx, vy);
                        self.handle_virtual_keyboard_action(action);
                        EventResult::Handled
                    } else {
                        EventResult::Unhandled
                    }
                }
                "message_box" => {
                    if let Some(msg_box) = &mut self.message_box {
                        if msg_box.on_mouse_down(vx, vy) {
                            self.close_message_box();
                        }
                        EventResult::Handled
                    } else {
                        EventResult::Unhandled
                    }
                }
                "change_password_dialog" => {
                    if let Some(dialog) = &mut self.change_password_dialog {
                        let action = dialog.on_mouse_down(vx, vy);
                        self.handle_change_password_action(action, network_tx);
                        EventResult::Handled
                    } else {
                        EventResult::Unhandled
                    }
                }
                "new_account_dialog" => {
                    if let Some(dialog) = &mut self.new_account_dialog {
                        let action = dialog.on_mouse_down(vx, vy);
                        self.handle_new_account_action(action, network_tx);
                        EventResult::Handled
                    } else {
                        EventResult::Unhandled
                    }
                }
                "login_dialog" => {
                    let action = self.login_dialog.on_mouse_down(vx, vy);
                    self.handle_login_action(action, network_tx);
                    EventResult::Handled
                }
                _ => EventResult::Unhandled
            }
        });
        
        Ok(())
    }
}

// ===== 优势总结 =====

优势1: 自动管理UI层级
- 不需要手动编写if-else判断优先级
- Z-order自动排序，新增UI层不会影响现有代码

优势2: 清晰的事件传播控制
- EventResult::Handled - 拦截事件
- EventResult::HandledContinue - 处理但允许传播（悬停效果）
- EventResult::Unhandled - 跳过该层

优势3: 模态对话框自动处理
- UILayer::modal() 标记为模态
- 自动阻止底层接收点击事件
- 不需要手动return

优势4: 焦点管理
- 点击自动设置焦点
- 键盘事件只发送给有焦点的层
- 支持Tab切换焦点（可扩展）

优势5: 易于测试和调试
- 每个层的事件处理独立
- 可以单独测试各层逻辑
- 添加日志追踪事件流

优势6: 代码复用
- 通用的事件分发逻辑
- 所有Scene都可以使用同一套系统
- 减少重复代码

// ===== 进一步优化建议 =====

1. 让UI组件实现UIComponent trait
   这样可以进一步简化代码：
   
   self.event_dispatcher.dispatch_mouse_move(vx, vy, |layer_name| {
       match layer_name {
           "virtual_keyboard" => self.virtual_keyboard.as_mut()
               .map(|k| k.on_mouse_move(vx, vy))
               .unwrap_or(EventResult::Unhandled),
           // ... 其他层
       }
   });

2. 使用宏简化层管理
   
   ui_layer!(self.event_dispatcher, "dialog", 10, modal: true);

3. 支持拖拽事件
   
   trait UIComponent {
       fn on_drag_start(&mut self, x: f32, y: f32) -> EventResult { ... }
       fn on_drag_move(&mut self, x: f32, y: f32) -> EventResult { ... }
       fn on_drag_end(&mut self, x: f32, y: f32) -> EventResult { ... }
   }

4. 支持触摸事件（移动端）
   
   fn dispatch_touch_start(&mut self, touches: &[Touch]) { ... }

5. 热区调试工具
   
   impl UIEventDispatcher {
       pub fn debug_draw(&self, canvas: &mut Canvas) {
           // 绘制每个UI层的边界和Z-order
       }
   }
*/
