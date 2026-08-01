// ============================================================================
// LoginPlugin - 登录界面（bevy_egui 0.41 / egui 0.35）
// ============================================================================
// 对应 macroquad LoginScene：账号/密码 + 进入游戏（当前 mock 直接进入）

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use crate::network::NetworkContext;
use crate::scenes::AppState;

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default());
        app.init_resource::<LoginState>();
        app.add_systems(
            EguiPrimaryContextPass,
            login_ui.run_if(in_state(AppState::Login)),
        );
    }
}

#[derive(Resource, Default)]
pub struct LoginState {
    pub account: String,
    pub password: String,
    pub error: Option<String>,
}

/// 登录界面（bevy_egui 0.41 的 UI 系统返回 Result，ctx_mut() 为 Result）
fn login_ui(
    mut contexts: EguiContexts,
    mut login: ResMut<LoginState>,
    mut net: ResMut<NetworkContext>,
    mut fonts_done: Local<bool>,
) {
    let ctx = contexts.ctx_mut().expect("primary egui context");

    // 中文字体（Bevy 默认字体不支持中文），只加载一次
    if !*fonts_done {
        *fonts_done = true;
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "cn".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf"
            ))
            .into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "cn".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("cn".to_owned());
        ctx.set_fonts(fonts);
    }

    let screen = ctx.content_rect();
    // 全屏深色背景
    egui::Area::new(egui::Id::new("login_bg"))
        .fixed_pos(egui::Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            ui.painter().rect_filled(screen, 0.0, egui::Color32::from_rgb(16, 18, 26));
        });

    // 登录卡片（居中）
    egui::Area::new(egui::Id::new("login_card"))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.style_of(egui::Theme::Dark))
                .fill(egui::Color32::from_rgb(30, 34, 46))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(28))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("传 奇 2")
                                .size(46.0)
                                .strong()
                                .color(egui::Color32::from_rgb(235, 205, 130)),
                        );
                        ui.label(
                            egui::RichText::new("Legend of Mir 2 · Bevy 移植版")
                                .size(14.0)
                                .color(egui::Color32::GRAY),
                        );
                    });
                    ui.add_space(26.0);
                    egui::Grid::new("login_grid")
                        .num_columns(2)
                        .spacing([14.0, 12.0])
                        .show(ui, |ui| {
                            ui.label("账号");
                            ui.add(
                                egui::TextEdit::singleline(&mut login.account)
                                    .desired_width(190.0)
                                    .hint_text("请输入账号"),
                            );
                            ui.end_row();
                            ui.label("密码");
                            ui.add(
                                egui::TextEdit::singleline(&mut login.password)
                                    .password(true)
                                    .desired_width(190.0)
                                    .hint_text("请输入密码"),
                            );
                            ui.end_row();
                        });
                    ui.add_space(14.0);
                    ui.vertical_centered(|ui| {
                        let connecting = net.state == crate::network::NetState::LoggingIn;
                        let btn_text = if connecting {
                            "连 接 中…"
                        } else {
                            "登 录"
                        };
                        if ui
                            .add_enabled(
                                !connecting,
                                egui::Button::new(egui::RichText::new(btn_text).size(18.0))
                                    .min_size(egui::vec2(170.0, 36.0)),
                            )
                            .clicked()
                        {
                            // 发送 Login（mock 服务器回应 LoginSuccess → Select 场景）
                            net.state = crate::network::NetState::LoggingIn;
                            net.send_packet(&mir2_shared::packets::client::account::Login {
                                account_id: login.account.clone(),
                                password: login.password.clone(),
                            });
                        }
                    });
                    if let Some(err) = &net.login_error {
                        ui.add_space(6.0);
                        ui.colored_label(egui::Color32::RED, err);
                    }
                });
        });

}
