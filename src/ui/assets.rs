use crate::config::{CICADAFARM_MINT, ROBOTCYB_MINT};
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn assets(&mut self, ui: &mut egui::Ui) {
        let id=egui::Id::new("assets_mining_tab");
        let mut mining=ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        ui.horizontal(|ui| {ui.selectable_value(&mut mining,false,crate::i18n::tr(ui.ctx(),"Токены")); ui.selectable_value(&mut mining,true,"CybHero · CPU lab");});
        ui.ctx().data_mut(|d| d.insert_temp(id,mining));
        ui.separator();
        if mining { self.mining_panel(ui); return; }
        ui.label("Токены проекта и публичные адреса. Кошелёк пока не подключён: показанные демо-балансы не являются твоими средствами.");
        if ui.button("Открыть CybDEX · найти торговую пару").clicked() { self.go(crate::Page::CybDex); }
        ui.collapsing("Демо экономики проекта", |ui| { self.token_matrix(ui); });

        ui.collapsing("CybHero · разработка собственной сети", |ui| {
            ui.label("CYBH: сеть ещё не запущена, монеты и награды не выпускаются. Основа протокола — Monero; CPU-майнинг RandomX — первый кандидат. GPU и постквантовая приватность требуют отдельного исследования.");
            ui.label("Награды будут зависеть от принятой сетью работы, а не от владения токеном или заявленной мощности. Майнинг не запускается автоматически.");
            ui.hyperlink_to("Репозиторий CybHero", "https://github.com/c1cad4/cybhero");
        });
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading(
            RichText::new(crate::i18n::tr(ui.ctx(), "Адреса токенов"))
                .strong()
                .color(Self::green()),
        );

        self.asset(ui, "$CICADAFARM", CICADAFARM_MINT, "CicadaFarm");
        self.asset(ui, "$ROBOTCYB", ROBOTCYB_MINT, "RobotCYB");
    }

    pub(crate) fn asset(&mut self, ui: &mut egui::Ui, name: &str, mint: &str, desc: &str) {
        egui::Frame::new()
            .fill(Color32::from_rgb(7, 25, 16))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 74, 48)))
            .corner_radius(9)
            .inner_margin(egui::Margin::same(13))
            .show(ui, |ui| {
                ui.label(RichText::new(name).size(20.0).strong());
                ui.label(desc);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(mint).small().color(Color32::GRAY));
                    if ui.button(crate::i18n::tr(ui.ctx(), "Копировать адрес")).clicked() {
                        ui.ctx().copy_text(mint.into());
                        self.notify(format!("{} MINT COPIED", name));
                    }
                });
            });
        ui.add_space(8.0);
    }
}
