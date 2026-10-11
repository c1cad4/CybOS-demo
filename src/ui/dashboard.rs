use crate::{CybOs, Page};
use eframe::egui::{self, Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn dashboard(&mut self, ui: &mut egui::Ui) {
        let apps = [
            (
                Page::Chat,
                "CybChat",
                "Общение",
                "Переписки и пользователи рядом",
            ),
            (
                Page::Browser,
                "CybBrowser",
                "Исследование",
                "Поиск и сайты в браузерном окне",
            ),
            (
                Page::CybLex,
                "CybLex",
                "Файлы",
                "Загрузка и раздача через BitTorrent",
            ),
            (
                Page::Brain,
                "Мозг",
                "Знания",
                "Личные заметки и граф связей",
            ),
            (
                Page::Robot,
                "Агенты",
                "Помощник",
                "Задачи для локального ИИ",
            ),
            (
                Page::Network,
                "Устройства",
                "Соединение",
                "Поиск устройств и профили WireGuard",
            ),
        ];
        ui.add_space(8.0);
        ui.label(
            RichText::new("Ваше цифровое пространство")
                .size(26.0)
                .strong(),
        );
        ui.add_space(6.0);
        ui.add(egui::Label::new("Общайтесь, собирайте знания и поручайте задачи помощнику. Выберите приложение, чтобы начать.").wrap());
        ui.add_space(20.0);
        let count = if ui.available_width() >= 1050.0 { 3 } else { 2 };
        for row in apps.chunks(count) {
            ui.columns(count, |columns| {
                for (column, (page, title, category, description)) in columns.iter_mut().zip(row) {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(7, 20, 17))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(28, 55, 46)))
                        .corner_radius(12)
                        .inner_margin(16)
                        .show(column, |ui| {
                            ui.set_width(ui.available_width());
                            ui.set_min_height(138.0);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(28.0, 28.0),
                                    egui::Sense::hover(),
                                );
                                Self::paint_icon(
                                    ui.painter(),
                                    rect.center(),
                                    page.icon(),
                                    Self::neon(),
                                    18.0,
                                );
                                ui.label(RichText::new(*title).size(20.0).strong());
                            });
                            ui.label(RichText::new(*category).small().color(Self::neon()));
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(*description).wrap());
                            ui.add_space(12.0);
                            if ui
                                .add_sized(
                                    [ui.available_width(), 32.0],
                                    egui::Button::new("Открыть"),
                                )
                                .clicked()
                            {
                                self.go(*page);
                            }
                        });
                }
            });
            ui.add_space(12.0);
        }
        ui.add_space(8.0);
        ui.collapsing("Первый запуск: с чего начать", |ui| {
            ui.label("1. В Мозге сохраните первую заметку или откройте граф знаний.");
            ui.label("2. В Браузере введите запрос или адрес сайта.");
            ui.label("3. Для общения добавьте устройство в CybChat и проверьте его ключ.");
            ui.label("4. В Агентах задайте вопрос. Если модель недоступна, откройте её настройку.");
        });
        ui.collapsing("Возможности и ограничения", |ui| {
            ui.label("Работают: локальная память и граф, BitTorrent, браузерное окно на macOS, запросы к настроенному локальному ИИ и сообщения между настроенными узлами.");
            ui.label("Готовятся: глобальный мозг, команды агентов и звонки. Работа WireGuard требует настройки и проверки сервера.");
            ui.label("USB-хранилище и шифрование переносимого профиля ещё не реализованы.");
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
