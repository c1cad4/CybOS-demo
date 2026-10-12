use crate::{
    CybOs,
    mining::{Operation, thread_limit},
};
use eframe::egui;

impl CybOs {
    pub(crate) fn mining_panel(&mut self, ui: &mut egui::Ui) {
        self.mining.poll();
        let ctx = ui.ctx().clone();
        ui.heading("CybHero · CPU lab");
        ui.colored_label(
            egui::Color32::YELLOW,
            crate::i18n::tr(&ctx, "Тестовая сеть · без реальных выплат"),
        );
        ui.label("Управление уже запущенным локальным узлом. CYBH ещё не выпущен.");
        ui.add_enabled_ui(!self.mining.busy(), |ui| {
            ui.horizontal(|ui| {
                ui.label("127.0.0.1 · RPC порт");
                if ui
                    .add(egui::DragValue::new(&mut self.mining.port).range(1..=65535))
                    .changed()
                {
                    self.mining.invalidate();
                }
                if ui
                    .button(crate::i18n::tr(&ctx, "Проверить соединение"))
                    .clicked()
                {
                    self.mining.request(Operation::Check, &ctx);
                }
            });
            ui.label(crate::i18n::tr(&ctx, "Публичный тестовый адрес"));
            ui.text_edit_singleline(&mut self.mining.address);
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr(&ctx, "CPU-потоки"));
                ui.add(egui::Slider::new(
                    &mut self.mining.threads,
                    1..=thread_limit(),
                ));
                ui.small("GPU: пока не поддерживается");
            });
            ui.checkbox(
                &mut self.mining.consent,
                crate::i18n::tr(&ctx, "Согласен на нагрузку CPU"),
            );
            ui.horizontal(|ui| {
                let ready = self.mining.fresh()
                    && self.mining.snapshot.as_ref().is_some_and(|s| !s.active)
                    && self.mining.consent
                    && !self.mining.address.is_empty();
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(crate::i18n::tr(&ctx, "Начать тест")),
                    )
                    .clicked()
                {
                    self.mining.request(
                        Operation::Start {
                            address: self.mining.address.trim().into(),
                            threads: self.mining.threads,
                            consent: self.mining.consent,
                        },
                        &ctx,
                    );
                }
                // Stop can be retried even after an ambiguous RPC timeout; identity is rechecked.
                if ui
                    .button(crate::i18n::tr(&ctx, "Остановить майнинг"))
                    .clicked()
                {
                    self.mining.request(Operation::Stop, &ctx);
                }
            });
        });
        ui.label(&self.mining.status);
        if self.mining.busy() {
            ui.spinner();
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if let Some(snapshot) = &self.mining.snapshot {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Высота: {}", snapshot.height));
                ui.label(format!(
                    "{}: {}",
                    crate::i18n::tr(&ctx, "Хешрейт"),
                    snapshot
                        .speed
                        .map(|v| format!("{v} H/s"))
                        .unwrap_or_else(|| "—".into())
                ));
                if let Some(threads) = snapshot.threads {
                    ui.label(format!("CPU: {threads}"));
                }
            });
            if let Some(time) = self.mining.checked {
                ui.small(format!(
                    "Последнее измерение: {} с назад",
                    time.elapsed().as_secs()
                ));
            }
        }
        ui.separator();
        ui.collapsing("Журнал тестовых наград", |ui| {
            ui.label("Импорт снимка учёта CybHero. Это исторический отчёт, не баланс кошелька и не подтверждение текущего состояния цепочки.");
            ui.label("Путь к JSON-отчёту");
            if ui.text_edit_singleline(&mut self.mining.report_path).changed() {
                self.mining.report = None;
                self.mining.report_error.clear();
            }
            if ui.button("Прочитать отчёт").clicked() {
                self.mining.report = None;
                match crate::mining_report::Report::load(&self.mining.report_path) {
                    Ok(report) => { self.mining.report=Some(report); self.mining.report_error.clear(); },
                    Err(error) => self.mining.report_error=error,
                }
            }
            ui.label(&self.mining.report_error);
            if let Some(report)=&self.mining.report {
                ui.label(format!("Проверено экспортёром: {} · высота {}",report.observed_at,report.chain_height));
                for state in ["pending","matured","orphaned"] {
                    let label=match state {"pending"=>"Ожидают созревания","matured"=>"Созрели на момент отчёта",_=>"Исключены из цепочки"};
                    ui.label(format!("{}: {}",label,report.states.values().filter(|s| s.as_str()==state).count()));
                }
                ui.label("Распределения в атомарных единицах лаборатории · выплат нет");
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for (participant, amount) in &report.allocations_atomic {
                        ui.label(format!("{participant}: {amount}"));
                    }
                });
                if report.allocations_atomic.is_empty() { ui.label("Подтверждённых распределений в отчёте нет"); }
            }
        });
        ui.small(crate::i18n::tr(
            &ctx,
            "Закрытие CybOS не останавливает отдельный узел. Остановите майнер кнопкой выше.",
        ));
        ui.collapsing("Как подготовить тест", |ui| {
            ui.label("Нужен собранный offline lab daemon CybHero и публичный адрес временного кошелька. Этот экран проверяет параметры узла, но не подтверждает подпись его бинарника.");
            ui.hyperlink_to("Инструкция лаборатории", "https://github.com/c1cad4/cybhero/blob/feature/protocol-mining-foundations/docs/OFFLINE_LAB_RU.md");
        });
    }
}
