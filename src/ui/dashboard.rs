use crate::{CybOs, Page};
use eframe::egui::{self, Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn dashboard(&mut self, ui: &mut egui::Ui) {
        let accent = Self::neon();
        ui.add_space(12.0);
        ui.label(
            RichText::new("Общайтесь. Исследуйте. Создавайте.")
                .size(30.0)
                .strong(),
        );
        ui.add_space(8.0);
        ui.label("CybOS объединяет ваши переписки, личные знания и локального ИИ-помощника.");
        ui.add_space(24.0);
        let available = ui.available_width();
        let card_width = if available >= 700.0 {
            (available - 52.0) / 2.0
        } else {
            available - 32.0
        };
        ui.horizontal_wrapped(|ui| {
        for (page, title, description, action) in [
            (Page::Chat, "01  CybChat", "Сообщения между вашими устройствами. Начните с добавления собеседника.", "Открыть чаты"),
            (Page::Brain, "02  Личный мозг", "Сохраняйте заметки локально и изучайте граф знаний.", "Открыть знания"),
            (Page::Browser, "03  Браузер", "Открывайте веб-страницы и читайте материалы внутри приложения.", "Открыть браузер"),
            (Page::CybLex, "04  CybLex", "Загружайте музыку, фильмы и другие файлы по torrent-ссылке. Раздавайте свои файлы.", "Открыть библиотеку"),
            (Page::Robot, "05  ИИ-агенты", "Задайте вопрос локальному помощнику. Команды агентов — следующий этап.", "Открыть помощника"),
            (Page::Network, "06  Устройства", "Находите устройства и готовьте профили защищённой сети.", "Настроить устройства"),
        ] {
            egui::Frame::new()
                .fill(Color32::from_rgb(7, 20, 17))
                .stroke(Stroke::new(1.0, Color32::from_rgb(28, 55, 46)))
                .corner_radius(12)
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                    ui.set_width(card_width);
                    ui.set_min_height(112.0);
                    ui.label(RichText::new(title).size(18.0).strong().color(accent));
                    ui.add_space(5.0);
                    ui.add(egui::Label::new(description).wrap());
                    ui.add_space(10.0);
                    if ui.add_sized([180.0, 30.0], egui::Button::new(action)).clicked() { self.go(page); }
                    });
                });
            ui.add_space(10.0);
        }
        });
        ui.add_space(10.0);
        ui.collapsing("Что уже доступно и что готовится", |ui| {
            ui.label("Доступно: локальная память, граф, чтение веб-страниц, запросы к настроенной локальной модели и защищённый обмен сообщениями между настроенными узлами.");
            ui.label("Готовится: глобальный мозг, команды ИИ-агентов, звонки и полноценный веб-браузер.");
            ui.label("WireGuard: можно подготовить профиль. Работа туннеля требует отдельной настройки и проверки сервера.");
        });
        ui.collapsing("Состояние приложения", |ui| {
            ui.label(format!("Помощник: {}", self.qwen_status));
            ui.label(format!(
                "Устройства: {} сохранено · {} найдено",
                self.manual_peers.len(),
                self.lan_peers.len()
            ));
            ui.label(format!("CybChat: {}", self.secure_status));
            ui.label(format!("База данных: {}", self.database_integrity));
        });
    }
}
