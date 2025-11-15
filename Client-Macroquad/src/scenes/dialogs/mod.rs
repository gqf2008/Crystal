pub mod message_box;

pub use message_box::{MessageBox, MessageBoxButtons, MessageBoxResult};

use egui_macroquad::egui;

/// 对话框 trait，为所有对话框类 UI 提供统一抽象
/// 
/// # 设计理念
/// - 对话框的显示/隐藏状态由实现者内部维护
/// - `show()` 方法通过 `open` 参数告知是否需要显示
/// - 实现者在 `show()` 中处理所有 UI 绘制和交互逻辑
/// 
/// # 示例
/// ```ignore
/// impl Dialog for MyDialog {
///     fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
///         if !*open { return; }
///         
///         egui::Window::new("My Dialog")
///             .open(open)  // 绑定关闭按钮
///             .show(ctx, |ui| {
///                 ui.label("Hello");
///                 if ui.button("OK").clicked() {
///                     *open = false;  // 关闭对话框
///                 }
///             });
///     }
/// }
/// ```
pub trait Dialog {
    /// 显示对话框
    /// 
    /// # 参数
    /// - `ctx`: egui 上下文
    /// - `open`: 对话框是否打开的状态标志
    ///   - 传入 `true` 时应显示对话框
    ///   - 实现者可通过修改为 `false` 来关闭对话框
    /// 
    /// # 注意
    /// - 此方法每帧都会被调用
    /// - 实现者应根据 `open` 参数决定是否绘制
    /// - 所有 UI 绘制和交互逻辑都在此方法中完成
    fn show(&mut self, ctx: &egui::Context, open: &mut bool);
}