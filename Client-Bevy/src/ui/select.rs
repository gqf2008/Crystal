// ============================================================================
// SelectPlugin - 角色选择界面（bevy_egui）
// ============================================================================
// 对应 macroquad SelectScene：展示 LoginSuccess 返回的角色列表，选择后发 StartGame

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::network::NetworkContext;
use crate::scenes::AppState;

pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy_egui::EguiPrimaryContextPass,
            select_ui.run_if(in_state(AppState::Select)),
        );
    }
}

fn select_ui(
    mut contexts: EguiContexts,
    mut net: ResMut<NetworkContext>,
    mut fonts_done: Local<bool>,
) {
    let ctx = contexts.ctx_mut().expect("primary egui context");

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
        ctx.set_fonts(fonts);
    }

    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("select_bg"))
        .fixed_pos(egui::Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            ui.painter().rect_filled(screen, 0.0, egui::Color32::from_rgb(16, 18, 26));
        });

    egui::Area::new(egui::Id::new("select_card"))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.style_of(egui::Theme::Dark))
                .fill(egui::Color32::from_rgb(30, 34, 46))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(24))
                .show(ui, |ui| {
                    ui.set_width(340.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("选 择 角 色")
                                .size(26.0)
                                .strong()
                                .color(egui::Color32::from_rgb(235, 205, 130)),
                        );
                    });
                    ui.add_space(16.0);

                    if net.characters.is_empty() {
                        ui.label("暂无角色");
                    } else {
                        for c in net.characters.clone() {
                            let class_name = match c.class {
                                mir2_shared::MirClass::Warrior => "战士",
                                mir2_shared::MirClass::Wizard => "法师",
                                mir2_shared::MirClass::Taoist => "道士",
                                mir2_shared::MirClass::Assassin => "刺客",
                                mir2_shared::MirClass::Archer => "弓箭手",
                            };
                            let gender_name = match c.gender {
                                mir2_shared::MirGender::Male => "男",
                                _ => "女",
                            };
                            let selected = net.selected_index == Some(c.index);
                            if ui
                                .selectable_label(
                                    selected,
                                    format!(
                                        "{}  Lv.{}  {}{}",
                                        c.name, c.level, class_name, gender_name
                                    ),
                                )
                                .clicked()
                            {
                                net.selected_index = Some(c.index);
                            }
                        }
                    }

                    ui.add_space(16.0);
                    ui.vertical_centered(|ui| {
                        let ready = net.selected_index.is_some();
                        if ui
                            .add_enabled(
                                ready,
                                egui::Button::new(
                                    egui::RichText::new("进 入 游 戏").size(18.0),
                                )
                                .min_size(egui::vec2(170.0, 36.0)),
                            )
                            .clicked()
                        {
                            if let Some(idx) = net.selected_index {
                                net.send_packet(&mir2_shared::packets::client::account::StartGame {
                                    character_index: idx,
                                });
                            }
                        }
                    });
                });
        });
}
