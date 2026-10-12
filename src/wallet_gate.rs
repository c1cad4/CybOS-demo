//! Boot gate: no CybOS services are initialized before profile authentication.
use crate::{CybOs, wallet_vault::Vault};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};

pub(crate) struct Gate {
    path: PathBuf,
    vault: Option<Vault>,
    damaged: bool,
    error: String,
    phrase: Zeroizing<String>,
    password: Zeroizing<String>,
    repeat: Zeroizing<String>,
    backup: [Zeroizing<String>; 3],
    saved: bool,
    restore: bool,
    task: Option<Receiver<Result<(Vault, Zeroizing<[u8; 32]>), String>>>,
    inner: Option<CybOs>,
    idle: IdleLock,
    unlock_task: Option<Receiver<Result<(), String>>>,
}
impl Gate {
    pub fn new() -> Self {
        let path = crate::store::dirs_fallback().join("profile-vault.json");
        let (vault, damaged, error) = if path.try_exists().unwrap_or(true) {
            match Vault::load(&path) {
                Ok(v) => (Some(v), false, String::new()),
                Err(e) => (None, true, e),
            }
        } else {
            (None, false, String::new())
        };
        Self {
            path,
            vault,
            damaged,
            error,
            phrase: Zeroizing::new(String::new()),
            password: Zeroizing::new(String::new()),
            repeat: Zeroizing::new(String::new()),
            backup: std::array::from_fn(|_| Zeroizing::new(String::new())),
            saved: false,
            restore: false,
            task: None,
            inner: None,
            idle: IdleLock::new(Instant::now()),
            unlock_task: None,
        }
    }
    fn lock_screen(&mut self, ui: &mut egui::Ui) {
        if let Some(result) = self.unlock_task.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Проверка пароля прервалась".into())),
        }) {
            self.unlock_task = None;
            self.password.zeroize();
            match result {
                Ok(()) => { self.idle.unlocked(Instant::now()); self.error.clear(); }
                Err(error) => self.error = error,
            }
        }
        // Render no application content on the frame that completes authentication.
        ui.vertical_centered(|ui| {
            ui.add_space(60.);
            ui.heading("CybOS заблокирован");
            ui.label("Введите пароль профиля, чтобы продолжить");
            ui.label("Фоновые задачи продолжаются. Для завершения сессии закройте приложение.");
            let mut submit = false;
            ui.add_enabled_ui(self.unlock_task.is_none(), |ui| {
                ui.add(egui::TextEdit::singleline(&mut *self.password).password(true));
                submit = ui.button("Разблокировать").clicked();
            });
            if submit {
                let path = self.path.clone();
                let password = Zeroizing::new(self.password.to_string());
                let expected_id = self.vault.as_ref().map(|vault| vault.id.clone());
                self.password.zeroize();
                let (tx, rx) = mpsc::channel();
                self.unlock_task = Some(rx);
                self.error.clear();
                std::thread::spawn(move || {
                    let result = Vault::load(&path).and_then(|vault| {
                        if Some(&vault.id) != expected_id.as_ref() {
                            return Err("Профиль изменился. Закройте приложение и войдите заново".into());
                        }
                        vault.unlock(&password)
                    });
                    let _ = tx.send(result);
                });
            }
            if self.unlock_task.is_some() { ui.spinner(); }
            ui.colored_label(egui::Color32::RED, &self.error);
            ui.label("Забыли пароль? Закройте CybOS и восстановите доступ по 24 словам при запуске.");
        });
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    fn request(&mut self) {
        if self.task.is_some() {
            return;
        }
        if self.vault.is_none() && !self.restore {
            let words: Vec<_> = self.phrase.split_whitespace().collect();
            if !self.saved
                || words.len() != 24
                || [2, 11, 20]
                    .iter()
                    .enumerate()
                    .any(|(i, index)| self.backup[i].trim() != words[*index])
            {
                self.error = "Сохраните фразу и подтвердите слова №3, №12 и №21".into();
                return;
            }
        }
        let creating = self.vault.is_none();
        let recovery = self.restore;
        if (creating || recovery) && *self.password != *self.repeat {
            self.error = "Пароли не совпадают".into();
            return;
        }
        let path = self.path.clone();
        let phrase = Zeroizing::new(self.phrase.to_string());
        let password = Zeroizing::new(self.password.to_string());
        let (tx, rx) = mpsc::channel();
        self.task = Some(rx);
        self.error.clear();
        std::thread::spawn(move || {
            let result = (|| {
                let vault = if creating {
                    let vault = Vault::seal(&phrase, &password)?;
                    vault.save_new(&path)?;
                    vault
                } else {
                    let vault = Vault::load(&path)?;
                    if recovery {
                        vault.recover(&path, &phrase, &password)?
                    } else {
                        vault
                    }
                };
                let data_key = vault.data_key(&password)?;
                crate::secure_store::prepare(
                    path.parent().ok_or("Папка профиля недоступна")?,
                    &data_key,
                )?;
                Ok((vault, data_key))
            })();
            let _ = tx.send(result);
        });
    }
    fn poll(&mut self) {
        let result = self.task.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Вход прервался".into())),
        });
        if let Some(result) = result {
            self.task = None;
            match result {
                Ok((vault, data_key)) => {
                    if let Err(error) = crate::secure_store::set_session_key(data_key) {
                        self.error = error;
                        return;
                    }
                    self.phrase.zeroize();
                    self.password.zeroize();
                    self.repeat.zeroize();
                    for word in &mut self.backup {
                        word.zeroize();
                    }
                    let mut app = CybOs::default();
                    app.wallet_profile_id = vault.id.clone();
                    self.vault = Some(vault);
                    self.inner = Some(app);
                    self.idle.unlocked(Instant::now());
                }
                Err(error) => {
                    self.password.zeroize();
                    self.repeat.zeroize();
                    self.error = error;
                    // Profile creation may have succeeded before database preparation failed.
                    if let Ok(vault) = Vault::load(&self.path) {
                        self.vault = Some(vault);
                        self.phrase.zeroize();
                        for word in &mut self.backup {
                            word.zeroize();
                        }
                    }
                }
            }
        }
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.inner.take();
        crate::secure_store::clear_session_key();
    }
}
impl eframe::App for Gate {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.poll();
        if self.inner.is_some() {
            let now = Instant::now();
            // Check expiry before accepting fresh input (including resume from sleep).
            self.idle.check(now);
            if self.idle.locked {
                if let Some(app)=&mut self.inner {app.poll_secure_events();app.poll_secure_send();}
                self.lock_screen(ui); return;
            }
            let activity = ui.input(|input| input.focused && !input.events.is_empty());
            if activity { self.idle.activity(now); }
            let lock = ui.horizontal(|ui| {
                ui.label("Автоблокировка · 5 минут");
                ui.button("🔒 Заблокировать").clicked()
            }).inner;
            if lock {
                self.idle.locked = true;
                self.password.zeroize();
                self.error.clear();
                self.lock_screen(ui);
                return;
            }
            ui.ctx().request_repaint_after(Duration::from_secs(1));
            if let Some(app) = &mut self.inner { eframe::App::ui(app, ui, frame); }
            return;
        }
        ui.ctx().set_visuals(egui::Visuals::dark());
        ui.vertical_centered(|ui| {
            ui.add_space(30.);ui.heading(egui::RichText::new("CybWallet").size(32.).color(egui::Color32::from_rgb(100,255,170)));ui.label("Ваш ключ доступа к CybOS");
            ui.label("24 слова BIP39 восстанавливают ключ профиля. Это не seed кошелька Monero/CybHero.");
            ui.label("Ключ и активная база будут зашифрованы. Старый cybos.db сохранится как открытая резервная копия; остальные файлы пока без шифрования.");
            ui.label("Модули CybOS запустятся после входа.");ui.add_space(16.);
            if self.damaged {ui.colored_label(egui::Color32::RED,&self.error);ui.label("Повреждённый файл сохранён. Автоматический сброс запрещён.");return;}
            ui.add_enabled_ui(self.task.is_none(),|ui| {
                if let Some(vault)=&self.vault {ui.label(format!("Профиль {}",&vault.id[..12]));ui.checkbox(&mut self.restore,"Восстановить доступ по 24 словам");}
                else {ui.checkbox(&mut self.restore,"У меня уже есть 24 слова профиля CybOS");}
                if self.vault.is_none() && !self.restore {
                    if self.phrase.is_empty() {
                        if ui.button("Создать фразу из 24 слов").clicked() {match Vault::generate(){Ok(p)=>self.phrase=p,Err(e)=>self.error=e}}
                    } else {
                        ui.label("Запишите все слова по порядку и сохраните вне компьютера. Не отправляйте их в чат.");
                        let words:Vec<_>=self.phrase.split_whitespace().collect();
                        egui::Grid::new("backup_words").num_columns(4).show(ui,|ui| {for (i,word) in words.iter().enumerate(){ui.label(format!("{}. {}",i+1,word));if i%4==3 {ui.end_row();}}});
                        ui.checkbox(&mut self.saved,"Я сохранил фразу в безопасном месте");
                        for (i,number) in [3,12,21].iter().enumerate(){ui.horizontal(|ui|{ui.label(format!("Слово №{number}"));ui.add(egui::TextEdit::singleline(&mut *self.backup[i]).password(true));});}
                    }
                } else if self.restore {ui.label("24 слова на английском · ввод скрыт");ui.add(egui::TextEdit::singleline(&mut *self.phrase).password(true).desired_width(500.));}
                ui.label(if self.restore || self.vault.is_none(){"Новый пароль · минимум 12 символов"}else{"Пароль профиля"});
                ui.add(egui::TextEdit::singleline(&mut *self.password).password(true));
                if self.restore || self.vault.is_none(){ui.label("Повторите пароль");ui.add(egui::TextEdit::singleline(&mut *self.repeat).password(true));}
                if ui.button("Войти в CybOS").clicked(){self.request();}
            });
            if self.task.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
            ui.colored_label(egui::Color32::RED,&self.error);
        });
    }
}

/// Only explicit UI input refreshes the timer; rendering/background work never does.
struct IdleLock { last_activity: Instant, locked: bool }
impl IdleLock {
    fn new(now: Instant) -> Self { Self { last_activity: now, locked: false } }
    fn check(&mut self, now: Instant) {
        if now.saturating_duration_since(self.last_activity) >= Duration::from_secs(300) {
            self.locked = true;
        }
    }
    fn activity(&mut self, now: Instant) { if !self.locked { self.last_activity = now; } }
    fn unlocked(&mut self, now: Instant) { self.last_activity = now; self.locked = false; }
}
#[cfg(test)]
mod lock_tests {
    use super::*;
    #[test]
    fn idle_expiry_cannot_be_bypassed_by_resume_input() {
        let start = Instant::now();
        let mut timer = IdleLock::new(start);
        timer.check(start + Duration::from_secs(299));
        assert!(!timer.locked);
        timer.check(start + Duration::from_secs(300));
        timer.activity(start + Duration::from_secs(300));
        assert!(timer.locked);
        timer.unlocked(start + Duration::from_secs(301));
        assert!(!timer.locked);
        timer.activity(start + Duration::from_secs(500));
        timer.check(start + Duration::from_secs(799));
        assert!(!timer.locked);
        timer.check(start + Duration::from_secs(800));
        assert!(timer.locked);
    }
}
