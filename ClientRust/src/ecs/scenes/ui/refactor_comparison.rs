//! LoginScene重构前后对比示例
//! 
//! 这个文件展示了使用UIEventDispatcher前后的代码差异

// ============================================================
// 重构前：复杂的手动事件管理
// ============================================================

#[allow(dead_code)]
mod before {
    use ggez::{Context, GameResult};
    
    pub struct LoginScene {
        login_dialog: LoginDialog,
        new_account_dialog: Option<NewAccountDialog>,
        change_password_dialog: Option<ChangePasswordDialog>,
        virtual_keyboard: Option<VirtualKeyboard>,
        message_box: Option<MessageBox>,
    }
    
    impl LoginScene {
        // ❌ 问题1: 每个事件方法都要重复优先级判断
        fn on_mouse_move_old(&mut self, x: f32, y: f32) -> GameResult {
            let (vx, vy) = self.screen_to_virtual(x, y);
            
            // 虚拟键盘优先级最高，但也更新背后的界面
            if let Some(keyboard) = &mut self.virtual_keyboard {
                keyboard.on_mouse_move(vx, vy);
                // ❌ 问题2: 不清楚为什么要继续更新login_dialog
                self.login_dialog.on_mouse_move(vx, vy);
                return Ok(());
            }
            
            // ❌ 问题3: 大量重复的if-else
            if let Some(msg_box) = &mut self.message_box {
                msg_box.on_mouse_move(vx, vy);
                return Ok(());  // ❌ 问题4: 手动return来阻止传播
            }
            
            if let Some(dialog) = &mut self.change_password_dialog {
                dialog.on_mouse_move(vx, vy);
                return Ok(());
            }
            
            if let Some(dialog) = &mut self.new_account_dialog {
                dialog.on_mouse_move(vx, vy);
                return Ok(());
            }
            
            self.login_dialog.on_mouse_move(vx, vy);
            Ok(())
        }
        
        // ❌ 问题5: on_mouse_down也要重复同样的判断逻辑
        fn on_mouse_down_old(&mut self, x: f32, y: f32) -> GameResult {
            let (vx, vy) = self.screen_to_virtual(x, y);
            
            // 完全相同的优先级判断顺序
            if let Some(keyboard) = &mut self.virtual_keyboard {
                let action = keyboard.on_mouse_down(vx, vy);
                // ❌ 问题6: 业务逻辑和事件分发混在一起
                match action {
                    VirtualKeyboardAction::Close => {
                        self.virtual_keyboard = None;
                    }
                    VirtualKeyboardAction::Delete => {
                        // 大量业务逻辑...
                    }
                    VirtualKeyboardAction::Input(ch) => {
                        // 大量业务逻辑...
                    }
                    _ => {}
                }
                return Ok(());
            }
            
            if let Some(msg_box) = &mut self.message_box {
                if msg_box.on_mouse_down(vx, vy) {
                    self.message_box = None;
                }
                return Ok(());
            }
            
            // ... 更多重复代码
            
            Ok(())
        }
        
        // ❌ 问题7: 添加新UI需要修改多个地方
        // 如果要添加一个tooltip UI:
        // 1. 在on_mouse_move中添加判断
        // 2. 在on_mouse_down中添加判断  
        // 3. 在on_key_down中添加判断
        // 4. 确保优先级顺序正确（容易出错）
    }
}

// ============================================================
// 重构后：清晰的事件分发
// ============================================================

#[allow(dead_code)]
mod after {
    use ggez::{Context, GameResult};
    use crate::ecs::scenes::ui::{UIEventDispatcher, UILayer, EventResult};
    
    pub struct LoginScene {
        // ✅ 新增：统一的事件管理
        event_dispatcher: UIEventDispatcher,
        
        login_dialog: LoginDialog,
        new_account_dialog: Option<NewAccountDialog>,
        change_password_dialog: Option<ChangePasswordDialog>,
        virtual_keyboard: Option<VirtualKeyboard>,
        message_box: Option<MessageBox>,
    }
    
    impl LoginScene {
        pub fn new() -> Self {
            let mut dispatcher = UIEventDispatcher::new();
            
            // ✅ 优势1: 一次性定义所有UI层级
            dispatcher.add_layer(UILayer::new("background", 0));
            dispatcher.add_layer(UILayer::new("login_dialog", 10));
            
            Self {
                event_dispatcher: dispatcher,
                // ... 其他字段
            }
        }
        
        // ✅ 优势2: 事件处理逻辑清晰明了
        fn on_mouse_move_new(&mut self, x: f32, y: f32) -> GameResult {
            let (vx, vy) = self.screen_to_virtual(x, y);
            
            // 统一的事件分发，自动处理优先级
            self.event_dispatcher.dispatch_mouse_move(vx, vy, |layer_name| {
                match layer_name {
                    "virtual_keyboard" => {
                        if let Some(keyboard) = &mut self.virtual_keyboard {
                            keyboard.on_mouse_move(vx, vy);
                            // ✅ 优势3: 明确表示"处理但继续传播"
                            EventResult::HandledContinue
                        } else {
                            EventResult::Unhandled
                        }
                    }
                    "message_box" => {
                        if let Some(msg_box) = &mut self.message_box {
                            msg_box.on_mouse_move(vx, vy);
                            // ✅ 优势4: 明确表示"处理并停止传播"
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
        
        // ✅ 优势5: 业务逻辑分离，代码更清晰
        fn on_mouse_down_new(&mut self, x: f32, y: f32) -> GameResult {
            let (vx, vy) = self.screen_to_virtual(x, y);
            
            self.event_dispatcher.dispatch_mouse_down(vx, vy, |layer_name| {
                match layer_name {
                    "virtual_keyboard" => {
                        if let Some(keyboard) = &mut self.virtual_keyboard {
                            let action = keyboard.on_mouse_down(vx, vy);
                            // ✅ 优势6: 业务逻辑提取到独立方法
                            self.handle_virtual_keyboard_action(action);
                            EventResult::Handled
                        } else {
                            EventResult::Unhandled
                        }
                    }
                    "message_box" => {
                        if let Some(_) = &self.message_box {
                            self.close_message_box();  // 使用辅助方法
                            EventResult::Handled
                        } else {
                            EventResult::Unhandled
                        }
                    }
                    // ... 其他层
                    _ => EventResult::Unhandled
                }
            });
            
            Ok(())
        }
        
        // ✅ 优势7: 统一的UI层管理
        fn show_new_account_dialog(&mut self) {
            let dialog = NewAccountDialog::new();
            self.new_account_dialog = Some(dialog);
            
            // 动态添加UI层，自动处理优先级
            self.event_dispatcher.add_layer(
                UILayer::new("new_account_dialog", 20).modal()
            );
        }
        
        fn close_new_account_dialog(&mut self) {
            self.new_account_dialog = None;
            self.event_dispatcher.remove_layer("new_account_dialog");
        }
        
        // ✅ 优势8: 添加新UI非常简单
        // 只需要：
        // 1. 在show方法中添加layer
        // 2. 在事件分发的match中添加一个分支
        // 不需要修改其他任何代码！
    }
}

// ============================================================
// 代码量对比
// ============================================================

/*
重构前 on_mouse_move:
- 5个if-else嵌套
- 5个return语句
- 约35行代码
- 优先级逻辑分散

重构后 on_mouse_move:
- 1个match表达式
- 自动优先级处理
- 约30行代码（更清晰）
- 优先级逻辑集中在dispatcher中
*/

// ============================================================
// 维护性对比
// ============================================================

/*
场景：添加一个Tooltip UI

重构前需要：
1. 在on_mouse_move中添加if let判断（找到正确的优先级位置）
2. 在on_mouse_down中添加if let判断（保持相同优先级）
3. 在on_key_down中添加if let判断
4. 确保所有方法中的优先级顺序一致
风险：优先级顺序不一致，导致事件处理错误

重构后需要：
1. 添加show_tooltip/close_tooltip方法
2. 在事件分发match中添加"tooltip"分支
3. dispatcher自动处理优先级
风险：极低，优先级由Z-order自动管理
*/

// ============================================================
// 性能对比
// ============================================================

/*
重构前：
- 每次事件都要遍历所有if-else
- 运行时判断优先级
- O(n)复杂度（n为UI层数）

重构后：
- layers在初始化时已按Z-order排序
- 遍历预排序的列表
- O(n)复杂度，但常数更小
- 可以提前终止（遇到modal层）

性能差异：微小，但重构后更优
*/

// ============================================================
// 可测试性对比
// ============================================================

/*
重构前：
- 事件处理逻辑分散在多个if-else中
- 难以单独测试某个UI层
- 优先级逻辑难以验证

重构后：
- 每个UI层的事件处理独立
- 可以单独测试每个层
- dispatcher可以单元测试
- 优先级测试简单（测试Z-order）
*/

// ============================================================
// 扩展性对比
// ============================================================

/*
未来功能扩展：

1. 拖拽支持
   重构前：需要在每个事件方法中添加拖拽逻辑
   重构后：在dispatcher中添加dispatch_drag方法

2. 触摸支持
   重构前：需要为每个UI添加触摸事件处理
   重构后：在dispatcher中添加dispatch_touch方法

3. 焦点管理
   重构前：需要手动跟踪焦点状态
   重构后：dispatcher自动管理焦点

4. 调试工具
   重构前：难以可视化UI层级
   重构后：dispatcher.debug_draw()即可
*/

// ============================================================
// 总结
// ============================================================

/*
重构优势：
✅ 代码更清晰、更易读
✅ 优先级管理自动化
✅ 事件传播逻辑明确
✅ 业务逻辑分离
✅ 易于测试和维护
✅ 易于扩展新功能
✅ 降低出错风险

重构成本：
- 初期需要理解事件分发器概念
- 需要修改现有事件处理方法
- 需要添加辅助方法

建议：
- 渐进式重构，先试点一个方法
- 充分测试后再全面替换
- 保持良好的文档记录
*/

// 虚拟类型定义（仅用于示例编译）
struct LoginDialog;
struct NewAccountDialog;
struct ChangePasswordDialog;
struct VirtualKeyboard;
struct MessageBox;
enum VirtualKeyboardAction { Close, Delete, Input(char) }

impl LoginScene {
    fn screen_to_virtual(&self, _x: f32, _y: f32) -> (f32, f32) { (0.0, 0.0) }
    fn handle_virtual_keyboard_action(&mut self, _action: VirtualKeyboardAction) {}
    fn close_message_box(&mut self) {}
}

// 这些trait方法的实现会在实际重构时添加
trait MouseMove { fn on_mouse_move(&mut self, _x: f32, _y: f32) {} }
trait MouseDown { fn on_mouse_down(&mut self, _x: f32, _y: f32) -> bool { false } }
impl MouseMove for LoginDialog {}
impl MouseMove for NewAccountDialog {}
impl MouseMove for ChangePasswordDialog {}
impl MouseMove for VirtualKeyboard {}
impl MouseMove for MessageBox {}
impl MouseDown for MessageBox {}
impl MouseDown for VirtualKeyboard { fn on_mouse_down(&mut self, _x: f32, _y: f32) -> VirtualKeyboardAction { VirtualKeyboardAction::Close } }
