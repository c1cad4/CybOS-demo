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
            ui.label(RichText::new("PLANETARY PULSE").size(18.0).strong().color(neon));
            ui.label(RichText::new("PUBLIC DATA · SOURCE-ATTRIBUTED").size(9.0).color(dim));
        });
        ui.label(RichText::new("A view of the living world and humanity — not a fabricated real-time counter.")
            .size(11.0).color(Color32::LIGHT_GRAY));
        ui.add_space(12.0);

        ui.columns(3, |columns| {
            egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
                .corner_radius(12).inner_margin(egui::Margin::same(14))
                .show(&mut columns[0], |ui| {
                    ui.label(RichText::new("◉ WILDLIFE").size(11.0).strong().color(neon));
                    ui.add_space(8.0);
                    ui.label(RichText::new("−73%").size(34.0).strong().color(Color32::from_rgb(255, 170, 90)));
                    ui.label(RichText::new("average change in monitored wildlife population abundance").size(10.0).color(Color32::WHITE));
                    ui.add_space(8.0);
                    ui.label(RichText::new("1970–2022 · Living Planet Index 2026").size(9.0).color(dim));
                    ui.label(RichText::new("35,803 populations · 5,790 vertebrate species").size(9.0).color(dim));
                    ui.separator();
                    ui.label(RichText::new("Important: this is not a claim that 73% of all animals or species disappeared.").size(9.0).color(Color32::LIGHT_GRAY));
                    if ui.button("OPEN LIVING PLANET DATA ↗").clicked() {
                        self.browser_url = "https://ourworldindata.org/2026-lpi-update".into();
                        self.go(Page::Browser);
                    }
                });
            egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
                .corner_radius(12).inner_margin(egui::Margin::same(14))
                .show(&mut columns[1], |ui| {
                    ui.label(RichText::new("◌ AIR & POLLUTION").size(11.0).strong().color(neon));
                    ui.add_space(8.0);
                    ui.label(RichText::new("NO LIVE FEED").size(22.0).strong().color(Color32::from_rgb(255, 205, 100)));
                    ui.label(RichText::new("No air-quality provider or monitoring location is connected in this build.").size(10.0).color(Color32::WHITE));
                    ui.add_space(8.0);
                    ui.label(RichText::new("OpenAQ: public measurements from a growing set of monitoring networks; global coverage is incomplete.").size(9.0).color(dim));
                    if ui.button("OPEN OPENAQ ↗").clicked() {
                        self.browser_url = "https://docs.openaq.org/about/about".into();
                        self.go(Page::Browser);
                    }
                    if ui.button("OPEN AIRNOW WIDGETS ↗").clicked() {
                        self.browser_url = "https://www.airnow.gov/aqi-widgets/".into();
                        self.go(Page::Browser);
                    }
                });
            egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
                .corner_radius(12).inner_margin(egui::Margin::same(14))
                .show(&mut columns[2], |ui| {
                    ui.label(RichText::new("◎ HUMANITY").size(11.0).strong().color(neon));
                    ui.add_space(8.0);
                    ui.label(RichText::new("BIRTHS & DEATHS").size(19.0).strong().color(neon));
                    ui.label(RichText::new("Annual demographic series are published estimates, not a live count of every birth or death.").size(10.0).color(Color32::WHITE));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Next integration: population, annual births and deaths, with source year and uncertainty shown on each metric.").size(9.0).color(dim));
                    if ui.button("OPEN OWID DEMOGRAPHICS ↗").clicked() {
                        self.browser_url = "https://ourworldindata.org/births-and-deaths".into();
                        self.go(Page::Browser);
                    }
                });
        });

        ui.add_space(12.0);
        egui::Frame::new().fill(Color32::from_rgb(3, 13, 9))
            .stroke(Stroke::new(1.0, edge)).corner_radius(10)
            .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.label(RichText::new("DATA INTEGRITY").size(11.0).strong().color(neon));
                ui.add_space(4.0);
                ui.label(RichText::new("Each metric must show its publisher, reference period, last update and coverage. Estimates and projections must be visibly labelled. Missing data stays missing — cybOS will not invent measurements.").size(10.0).color(Color32::LIGHT_GRAY));
                ui.add_space(6.0);
                ui.label(RichText::new("Current state: source-linked prototype · external data is not yet ingested or refreshed automatically.").size(9.0).color(dim));
            });
    }
}
