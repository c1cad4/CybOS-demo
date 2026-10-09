//! Planetary Pulse: source-attributed public indicators, with explicit freshness and coverage caveats.
use crate::{CybOs, Page};
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn planetary_pulse(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(120, 175, 145);
        let panel = Color32::from_rgb(4, 18, 12);
        let edge = Color32::from_rgb(24, 90, 56);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(crate::language::tr(self.language, "planetary_pulse")).size(18.0).strong().color(neon));
            ui.label(RichText::new(crate::language::tr(self.language, "pulse_subtitle")).size(9.0).color(dim));
        });
        ui.label(RichText::new(crate::language::tr(self.language, "pulse_intro"))
            .size(11.0).color(Color32::LIGHT_GRAY));
        ui.add_space(12.0);

        ui.columns(3, |columns| {
            egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
                .corner_radius(12).inner_margin(egui::Margin::same(14))
                .show(&mut columns[0], |ui| {
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_wildlife")).size(11.0).strong().color(neon));
                    ui.add_space(8.0);
                    ui.label(RichText::new("−73%").size(34.0).strong().color(Color32::from_rgb(255, 170, 90)));
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_avg_change")).size(10.0).color(Color32::WHITE));
                    ui.add_space(8.0);
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_wildlife_period")).size(9.0).color(dim));
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_wildlife_coverage")).size(9.0).color(dim));
                    ui.separator();
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_wildlife_caveat")).size(9.0).color(Color32::LIGHT_GRAY));
                    if ui.button(crate::language::tr(self.language, "pulse_open_lpi")).clicked() {
                        self.browser_url = "https://ourworldindata.org/2026-lpi-update".into();
                        self.go(Page::Browser);
                    }
                });
            egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
                .corner_radius(12).inner_margin(egui::Margin::same(14))
                .show(&mut columns[1], |ui| {
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_air")).size(11.0).strong().color(neon));
                    ui.add_space(8.0);
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_no_live")).size(22.0).strong().color(Color32::from_rgb(255, 205, 100)));
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_air_unavailable")).size(10.0).color(Color32::WHITE));
                    ui.add_space(8.0);
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_openaq_note")).size(9.0).color(dim));
                    if ui.button(crate::language::tr(self.language, "pulse_open_openaq")).clicked() {
                        self.browser_url = "https://docs.openaq.org/about/about".into();
                        self.go(Page::Browser);
                    }
                    if ui.button(crate::language::tr(self.language, "pulse_open_airnow")).clicked() {
                        self.browser_url = "https://www.airnow.gov/aqi-widgets/".into();
                        self.go(Page::Browser);
                    }
                });
            egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
                .corner_radius(12).inner_margin(egui::Margin::same(14))
                .show(&mut columns[2], |ui| {
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_humanity")).size(11.0).strong().color(neon));
                    ui.add_space(8.0);
                    ui.label(RichText::new("8.2B").size(34.0).strong().color(neon));
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_population")).size(9.0).color(dim));
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("132M").size(18.0).strong().color(Color32::from_rgb(100, 225, 150)));
                        ui.label(RichText::new(crate::language::tr(self.language, "pulse_births")).size(9.0).color(Color32::WHITE));
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("63M").size(18.0).strong().color(Color32::from_rgb(255, 170, 120)));
                        ui.label(RichText::new(crate::language::tr(self.language, "pulse_deaths")).size(9.0).color(Color32::WHITE));
                    });
                    ui.add_space(5.0);
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_demographic_period")).size(9.0).color(dim));
                    ui.label(RichText::new(crate::language::tr(self.language, "pulse_demographic_note")).size(9.0).color(Color32::LIGHT_GRAY));
                    if ui.button(crate::language::tr(self.language, "pulse_open_demographics")).clicked() {
                        self.browser_url = "https://ourworldindata.org/births-and-deaths".into();
                        self.go(Page::Browser);
                    }
                });
        });

        ui.add_space(12.0);
        egui::Frame::new().fill(Color32::from_rgb(3, 13, 9))
            .stroke(Stroke::new(1.0, edge)).corner_radius(10)
            .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.label(RichText::new(crate::language::tr(self.language, "pulse_integrity")).size(11.0).strong().color(neon));
                ui.add_space(4.0);
                ui.label(RichText::new(crate::language::tr(self.language, "pulse_integrity_note")).size(10.0).color(Color32::LIGHT_GRAY));
                ui.add_space(6.0);
                ui.label(RichText::new(crate::language::tr(self.language, "pulse_current_state")).size(9.0).color(dim));
            });
    }
}
