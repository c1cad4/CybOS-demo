use crate::CybOs;
use eframe::egui::{self, RichText};

impl CybOs {
    pub(crate) fn robot(&mut self, ui: &mut egui::Ui) {
        if let Some(answer) = self.poll_robot_job() {
            self.robot_output = answer.clone();
            self.push_chat_message("ROBOTCYB", answer, false);
            self.add_event("ROBOT", "RobotCYB worker completed the request");
        }
        ui.label(RichText::new("Личный помощник").size(24.0).strong());
        ui.label("Объяснит сложное, поможет сформулировать идею или разобрать текст. Для ответов нужна работающая локальная модель.");
        ui.add_space(10.0);
        ui.collapsing("Модель и состояние", |ui| {
            ui.label(format!("Qwen: {}", self.qwen_status));
            ui.label(&self.robot_status);
            if ui.button("Настройка локального ИИ").clicked() {
                self.go(crate::Page::Brain);
            }
        });
        ui.add_space(16.0);
        ui.label("Что нужно сделать?");
        ui.add(
            egui::TextEdit::multiline(&mut self.robot_input)
                .desired_rows(5)
                .desired_width(f32::INFINITY)
                .hint_text("Например: объясни этот текст простыми словами…"),
        );
        ui.add_space(8.0);
        let busy = self.robot_job.is_some();
        let ready = !busy && !self.robot_input.trim().is_empty();
        if ui
            .add_enabled(
                ready,
                egui::Button::new(if busy {
                    "Обрабатываю…"
                } else {
                    "Отправить задачу"
                }),
            )
            .clicked()
        {
            let q = self.robot_input.trim().to_string();
            self.push_chat_message("YOU", q.clone(), true);
            self.robot_input.clear();
            self.robot_output = "Задача принята. Ожидаю ответ локальной модели…".into();
            let _ = self.start_robot_job(q);
        }
        ui.label(RichText::new("Enter — новая строка. Задача отправляется кнопкой.").small());
        ui.add_space(20.0);
        ui.label(RichText::new("Ответ").size(18.0).strong());
        ui.add_space(8.0);
        ui.add(egui::Label::new(&self.robot_output).wrap().selectable(true));
        ui.add_space(20.0);
        ui.collapsing("Следующий этап: специализированные агенты", |ui| {
            ui.label("Исследователь, хранитель знаний и разработчик пока не реализованы как самостоятельные агенты. Сейчас доступен один локальный помощник.");
        });
    }
}
