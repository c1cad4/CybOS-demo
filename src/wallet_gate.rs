//! Boot gate: no CybOS services are initialized before profile authentication.
use crate::{CybOs, wallet_vault::Vault};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
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
        }
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
        if let Some(app) = &mut self.inner {
            eframe::App::ui(app, ui, frame);
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
