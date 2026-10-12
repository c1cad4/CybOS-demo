use crate::secure_files::Entry;
use eframe::egui;
use std::sync::{Arc, Mutex};
type Job = Arc<Mutex<Option<Result<Vec<Entry>, String>>>>;
#[derive(Clone, Default)]
struct View {
    source: String,
    destination: String,
    selected: String,
    files: Vec<Entry>,
    status: String,
    job: Option<Job>,
    loaded: bool,
}
enum Action {
    List,
    Import(String),
    Export(String, String),
}
impl View {
    fn start(&mut self, ctx: &egui::Context, action: Action) {
        let job: Job = Arc::new(Mutex::new(None));
        self.job = Some(job.clone());
        self.status = "Обработка…".into();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let mut conn = crate::secure_store::open_session(&crate::store::dirs_fallback())?;
                match action {
                    Action::List => {}
                    Action::Import(path) => {
                        crate::secure_files::import(&mut conn, std::path::Path::new(path.trim()))?
                    }
                    Action::Export(id, path) => {
                        crate::secure_files::export(&conn, &id, std::path::Path::new(path.trim()))?
                    }
                }
                crate::secure_files::list(&conn)
            })();
            if let Ok(mut slot) = job.lock() {
                *slot = Some(result);
            }
            repaint.request_repaint();
        });
    }
}
impl crate::CybOs {
    pub(crate) fn protected_files_page(&mut self, ui: &mut egui::Ui) {
        let id = egui::Id::new("cybos-protected-files");
        let mut view = ui
            .ctx()
            .data(|d| d.get_temp::<View>(id))
            .unwrap_or_default();
        if !view.loaded {
            view.loaded = true;
            view.start(ui.ctx(), Action::List);
        }
        let result = view.job.as_ref().and_then(|job| job.lock().ok()?.take());
        if let Some(result) = result {
            view.job = None;
            match result {
                Ok(files) => {
                    view.files = files;
                    view.status = "Готово".into();
                }
                Err(e) => view.status = e,
            }
        }
        ui.separator();
        ui.heading("Защищённые файлы");
        ui.label("Документы и вложения до 32 МБ · всего до 256 МБ");
        ui.label("Копии хранятся в зашифрованной базе. Исходные файлы и загрузки CybLex остаются на своих местах.");
        ui.add_enabled_ui(view.job.is_none(), |ui| {
            ui.label("Полный путь исходного файла");
            ui.text_edit_singleline(&mut view.source);
            if ui.button("Сохранить защищённую копию").clicked() {
                view.start(ui.ctx(), Action::Import(view.source.clone()));
            }
            if ui.button("Обновить список").clicked() && view.job.is_none() {
                view.start(ui.ctx(), Action::List);
            }
            egui::ScrollArea::vertical()
                .max_height(160.)
                .show(ui, |ui| {
                    for file in &view.files {
                        ui.selectable_value(
                            &mut view.selected,
                            file.id.clone(),
                            format!("{} · {} КБ", file.name, file.bytes.div_ceil(1024)),
                        );
                    }
                });
            ui.label("Полный путь для выгрузки нового файла");
            ui.text_edit_singleline(&mut view.destination);
            ui.label("Выгруженная копия будет незашифрованной. Существующие файлы не заменяются.");
            if ui
                .add_enabled(
                    !view.selected.is_empty(),
                    egui::Button::new("Выгрузить выбранный файл"),
                )
                .clicked()
                && view.job.is_none()
            {
                view.start(
                    ui.ctx(),
                    Action::Export(view.selected.clone(), view.destination.clone()),
                );
            }
        });
        ui.label(&view.status);
        if view.job.is_some() {
            ui.spinner();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        ui.ctx().data_mut(|d| d.insert_temp(id, view));
    }
}
