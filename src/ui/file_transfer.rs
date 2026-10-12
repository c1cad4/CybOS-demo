use eframe::egui;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Clone)]
struct Job {
    pause: Arc<AtomicBool>,
    result: Arc<Mutex<Option<Result<(), String>>>>,
}
#[derive(Clone, Default)]
struct View {
    selected: String,
    job: Option<Job>,
    status: String,
}
impl View {
    fn start(
        &mut self,
        app: &crate::CybOs,
        peer: &str,
        file: Option<String>,
        resume: Option<String>,
        ctx: &egui::Context,
    ) {
        if self.job.is_some() {
            return;
        }
        let Some(route) = app.chat_peers().into_iter().find(|p| p.node_id == peer) else {
            self.status = "Друг недоступен".into();
            return;
        };
        let approved = match crate::network::peer_trust::approved_key(&app.store, peer) {
            Ok(Some(key)) => key,
            _ => {
                self.status = "Сначала подтвердите ключ друга".into();
                return;
            }
        };
        let job = Job {
            pause: Arc::new(AtomicBool::new(false)),
            result: Arc::new(Mutex::new(None)),
        };
        self.job = Some(job.clone());
        self.status = "Передача…".into();
        let sender = app.node_id.clone();
        let key = zeroize::Zeroizing::new(app.noise_private_key.clone());
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let c = crate::secure_store::open_session(&crate::store::dirs_fallback())?;
                let id = match resume {
                    Some(id) => id,
                    None => crate::file_transfer::create(
                        &c,
                        &route.node_id,
                        &file.ok_or("Выберите файл")?,
                    )?,
                };
                crate::file_transfer::send(
                    &c,
                    &id,
                    &route.node_id,
                    &route.address,
                    &sender,
                    &key,
                    &approved,
                    job.pause.clone(),
                )
            })();
            if let Ok(mut slot) = job.result.lock() {
                *slot = Some(result)
            }
            repaint.request_repaint();
        });
    }
}
impl crate::CybOs {
    pub(crate) fn transfer_panel(&mut self, ui: &mut egui::Ui, peer: &str) {
        let key = egui::Id::new("cybchat-large-file-job");
        let mut view = ui
            .ctx()
            .data(|d| d.get_temp::<View>(key))
            .unwrap_or_default();
        let result = view.job.as_ref().and_then(|j| j.result.lock().ok()?.take());
        if let Some(result) = result {
            view.job = None;
            view.status = match result {
                Ok(()) => "Файл доставлен в защищённое хранилище друга".into(),
                Err(e) => e,
            };
        }
        ui.collapsing("📎 Передача файла · до 32 МиБ",|ui|{
   ui.small("Части по 8 КиБ. Передача продолжится после блокировки экрана. Пауза сработает после текущей части.");
   match self.store.protected_files(){
    Ok(files)=>{egui::ComboBox::from_id_salt("large-file-picker").selected_text(files.iter().find(|f|f.id==view.selected).map(|f|f.name.as_str()).unwrap_or("Выберите защищённый файл")).show_ui(ui,|ui|{for file in files{ui.selectable_value(&mut view.selected,file.id,&file.name);}});
     if ui.add_enabled(view.job.is_none()&&!view.selected.is_empty(),egui::Button::new("Передать файл")).clicked(){let file=view.selected.clone();view.start(self,peer,Some(file),None,ui.ctx());}
    },Err(e)=>{ui.label(e);}
   }
   if let Some(job)=&view.job{if ui.button("Приостановить").clicked(){job.pause.store(true,Ordering::Relaxed);}}
   ui.label(&view.status);
   if let Ok(transfers)=self.store.file_transfers(peer){
    for transfer in transfers{
     ui.push_id(&transfer.id,|ui|{
      let phase=match transfer.phase.as_str(){"sending"=>"Отправляется","receiving"=>"Получение","paused"=>"Приостановлено","complete"=>"Готово","cancelled"=>"Отменено",_=>"Ожидание"};
      ui.label(format!("{} · {} · {}",if transfer.outgoing{"Отправка"}else{"Приём"},transfer.name,phase));
      let progress=if transfer.phase=="complete"||transfer.size==0{1.0}else{transfer.done as f32/transfer.size as f32};ui.add(egui::ProgressBar::new(progress).show_percentage());
      if transfer.outgoing && transfer.phase!="complete" && ui.add_enabled(view.job.is_none(),egui::Button::new("Продолжить")).clicked(){view.start(self,peer,None,Some(transfer.id.clone()),ui.ctx());}
      if !transfer.outgoing && transfer.phase=="receiving" && ui.button("Отменить приём и удалить части").clicked(){view.status=match self.store.discard_transfer(&transfer.id,peer){Ok(())=>"Приём отменён; части удалены из базы".into(),Err(e)=>e};}
     });
    }
   }
  });
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
        ui.ctx().data_mut(|d| d.insert_temp(key, view));
    }
}
