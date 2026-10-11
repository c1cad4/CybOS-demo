use crate::CybOs;
use eframe::egui::{self, RichText};

impl CybOs {
    fn launch_cybbrowser(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let result = std::env::current_exe().map_err(|e| e.to_string()).and_then(|exe| {
                    let dir = exe.parent().ok_or("Не найден каталог приложения")?;
                    let packaged = dir.parent().ok_or("Не найден каталог пакета")?.join("Resources/CybBrowser.app");
                    let development = dir.join("CybBrowser.app");
                    let bundle = if packaged.is_dir() { packaged } else { development };
                    if !bundle.is_dir() { return Err("CybBrowser не собран. Запустите scripts/build_browser_macos.sh или используйте полный пакет CybOS".into()); }
                    std::process::Command::new("/usr/bin/open")
                        .args(["-n", "-a"]).arg(bundle).arg("--args")
                        .arg(self.browser_url.trim()).spawn().map(|_| ()).map_err(|e| e.to_string())
                });
            if let Err(error) = result {
                self.notify(format!("Не удалось открыть CybBrowser: {error}"));
            }
        }
        #[cfg(not(target_os = "macos"))]
        self.notify("Браузерный движок пока доступен на macOS");
    }
    pub(crate) fn browser(&mut self, ui: &mut egui::Ui) {
        self.poll_browser();
        ui.set_max_width(1100.0);
        ui.add_space(22.0);
        ui.label(
            RichText::new("Куда отправимся сегодня?")
                .size(30.0)
                .strong()
                .color(Self::neon()),
        );
        ui.label("Введи вопрос или адрес сайта — CybBrowser откроет результаты.");
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            let input = ui.add_sized(
                [(ui.available_width() - 120.0).max(120.0), 42.0],
                egui::TextEdit::singleline(&mut self.browser_url)
                    .hint_text("Поиск или адрес сайта…"),
            );
            if ui
                .add_sized([110.0, 42.0], egui::Button::new("Найти"))
                .clicked()
                || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.launch_cybbrowser();
            }
        });
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            for (label, url) in [
                ("Поиск", "https://duckduckgo.com"),
                ("Википедия", "https://ru.wikipedia.org"),
                ("Видео", "https://www.youtube.com"),
            ] {
                if ui.button(label).clicked() {
                    self.browser_url = url.into();
                    self.launch_cybbrowser();
                }
            }
        });
        ui.add_space(22.0);
        ui.collapsing("Режим чтения внутри CybOS", |ui| {
            ui.label("Только текст и ссылки. Для интерактивных сайтов используй кнопку «Найти».");
            if ui.button("Прочитать страницу").clicked() {
                self.navigate_browser();
            }
            ui.label(&self.browser_status);
            if !self.browser_text.is_empty() {
                ui.heading(&self.browser_title);
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.label(&self.browser_text);
                    });
                ui.collapsing("Ссылки на странице", |ui| {
                    for link in self.browser_links.clone() {
                        if ui.button(&link.label).clicked() {
                            self.browser_url = link.url;
                            self.navigate_browser();
                        }
                    }
                });
            }
        });
        ui.add_space(18.0);
        ui.small("Сеанс CybBrowser не сохраняет cookies между запусками. Сеть и система могут видеть активность.");
    }
}
