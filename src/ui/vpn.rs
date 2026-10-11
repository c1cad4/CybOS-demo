use crate::CybOs;
use eframe::egui;

impl CybOs {
    pub(crate) fn vpn_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(crate::i18n::tr(ui.ctx(), "Защищённый интернет"));
        ui.colored_label(
            egui::Color32::YELLOW,
            crate::i18n::tr(ui.ctx(), "Статус подключения не проверен"),
        );
        ui.label(crate::i18n::tr(
            ui.ctx(),
            "Нужен ваш VPN-сервер. Подготовьте профиль, затем включите его в WireGuard.",
        ));
        ui.small(crate::i18n::tr(
            ui.ctx(),
            "Экспорт профиля не включает VPN. Проверка ключей не подтверждает защиту трафика.",
        ));
        let id = egui::Id::new("vpn_dns");
        let mut dns = ui
            .ctx()
            .data(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| "9.9.9.9".into());
        ui.horizontal(|ui| {
            ui.label("DNS");
            ui.text_edit_singleline(&mut dns);
        });
        ui.ctx().data_mut(|d| d.insert_temp(id, dns.clone()));
        let ready = self.vpn_setup.profile.validate().is_ok() && self.vpn_setup.server_verified;
        if ui
            .add_enabled(
                ready,
                egui::Button::new(crate::i18n::tr(ui.ctx(), "Экспорт интернет-профиля")),
            )
            .clicked()
        {
            self.vpn_setup.export_internet(&self.store, &dns);
        }
        ui.label(crate::i18n::tr(ui.ctx(), "Проверьте маршрут, DNS и IPv6 в системном VPN-клиенте. Автоматическая блокировка трафика пока не реализована."));
        if ui
            .button(crate::i18n::tr(ui.ctx(), "Проверить соединение"))
            .clicked()
        {
            self.vpn_setup.start_diagnostics();
        }
        ui.label(&self.vpn_setup.diagnostic_status);
        ui.separator();
        ui.collapsing(
            crate::i18n::tr(ui.ctx(), "Настройка профиля"),
            |ui| self.wireguard_setup_panel(ui),
        );
        ui.collapsing("Разработка CybVPN",|ui| {
            ui.label("Следующий этап: отдельный системный сервис, AmneziaWG, проверка DNS/IPv6, аварийная блокировка трафика и подписанные обновления. Приложение не меняет маршруты с правами администратора без работающего сервера.");
            ui.hyperlink_to("Amnezia: исходный код","https://github.com/amnezia-vpn/amnezia-client");
            ui.hyperlink_to("Nabzram: клиент Marzban","https://github.com/Gozargah/Nabzram");
        });
    }
}
