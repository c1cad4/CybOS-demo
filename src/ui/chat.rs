use crate::{
    CybOs,
    conversations::{Book, Group, GroupMessage},
};
use eframe::egui::{self, Color32, RichText};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct ChatView {
    selected: String,
    tab: usize,
    search: String,
    add_friend: bool,
    create_group: bool,
    name: String,
    group_title: String,
    members: Vec<String>,
    error: String,
    draft_for: String,
    drafts: std::collections::BTreeMap<String, String>,
}
#[derive(Clone)]
struct GroupDelivery(Arc<Mutex<Delivery>>);
struct Delivery {
    message: GroupMessage,
    results: Vec<(String, Result<(), String>)>,
    done: bool,
}
fn view_id() -> egui::Id {
    egui::Id::new("cybchat_view_v1")
}
fn delivery_id() -> egui::Id {
    egui::Id::new("cybchat_group_delivery_v1")
}

impl CybOs {
    pub(crate) fn poll_group_chat(&mut self, ctx: &egui::Context) {
        let job = ctx.data(|d| d.get_temp::<GroupDelivery>(delivery_id()));
        let Some(job) = job else {
            return;
        };
        let Ok(job) = job.0.lock() else {
            return;
        };
        if !job.done {
            return;
        }
        let delivered = job.results.iter().filter(|(_, r)| r.is_ok()).count();
        if delivered > 0 && delivered == job.results.len() {
            let conversation = format!("group:{}", job.message.group.id);
            if let Some(mut view) = ctx.data(|d| d.get_temp::<ChatView>(view_id())) {
                if view.draft_for == conversation && self.chat_input.trim() == job.message.text { self.chat_input.clear(); }
                if view.drafts.get(&conversation).is_some_and(|draft| draft.trim() == job.message.text) { view.drafts.remove(&conversation); }
                ctx.data_mut(|d| d.insert_temp(view_id(), view));
            }
        }

        if delivered > 0 {
            if let Ok(wire) = job.message.encode() {
                self.push_chat_message(format!("YOU → GROUP:{}", job.message.group.id), wire, true);
            }
        }
        let summary = job
            .results
            .iter()
            .map(|(peer, r)| {
                format!(
                    "{peer}: {}",
                    match r {
                        Ok(()) => "доставлено".into(),
                        Err(e) => e.clone(),
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.secure_status = format!(
            "Группа: доставлено {delivered}/{}\n{summary}",
            job.results.len()
        );
        self.notify(format!(
            "Группа: доставлено {delivered}/{}",
            job.results.len()
        ));
        drop(job);
        ctx.data_mut(|d| d.remove::<GroupDelivery>(delivery_id()));
    }

    fn send_group_message(
        &mut self,
        ctx: &egui::Context,
        group: &Group,
        text: &str,
    ) -> Result<(), String> {
        if self.secure_startup_error.is_some() {
            return Err("Защищённый обмен недоступен".into());
        }
        if ctx.data(|d| d.get_temp::<GroupDelivery>(delivery_id()).is_some()) {
            return Err("Дождись текущей отправки группе".into());
        }
        let message = GroupMessage {
            id: uuid::Uuid::new_v4().to_string(),
            group: group.clone(),
            text: text.trim().into(),
        };
        let wire = message.encode()?;
        let peers = self.chat_peers();
        let mut routes = Vec::new();
        for member in group.members.iter().filter(|m| *m != &self.node_id) {
            let peer = peers
                .iter()
                .find(|p| &p.node_id == member)
                .ok_or_else(|| format!("Нет адреса участника {member}. Добавь его в друзья"))?;
            let key = crate::network::peer_trust::approved_key(&self.store, member)?
                .ok_or_else(|| format!("Сначала сверь код безопасности участника {member}"))?;
            routes.push((member.clone(), peer.address.clone(), key));
        }
        let job = GroupDelivery(Arc::new(Mutex::new(Delivery {
            message,
            results: Vec::new(),
            done: false,
        })));
        ctx.data_mut(|d| d.insert_temp(delivery_id(), job.clone()));
        self.runtime.set_status("CYBCHAT", "RUNNING");
        let sender = self.node_id.clone();
        let key = self.noise_private_key.clone();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            for (peer, address, approved) in routes {
                let status = crate::network::secure_chat::send(
                    &sender,
                    &peer,
                    &address,
                    &key,
                    Some(&approved),
                    &wire,
                );
                let result = match status {
                    crate::network::secure_chat::SecureSendStatus::Delivered { .. } => Ok(()),
                    crate::network::secure_chat::SecureSendStatus::NeedsTrust { .. } => {
                        Err("ключ изменился, сообщение не отправлено".into())
                    }
                    crate::network::secure_chat::SecureSendStatus::Failed { reason, .. } => {
                        Err(reason)
                    }
                };
                if let Ok(mut state) = job.0.lock() {
                    state.results.push((peer, result));
                }
                repaint.request_repaint();
            }
            if let Ok(mut state) = job.0.lock() {
                state.done = true;
            }
            repaint.request_repaint();
        });
        Ok(())
    }

    pub(crate) fn chat_page(&mut self, ui: &mut egui::Ui) {
        if self.last_scan.is_none_or(|last| last.elapsed().as_secs() >= 10) && self.lan_scan.is_none() {
            self.last_scan = Some(std::time::Instant::now()); self.start_lan_scan();
        }
        let mut view = ui
            .ctx()
            .data(|d| d.get_temp::<ChatView>(view_id()))
            .unwrap_or_default();
        if view.selected.is_empty() {
            view.selected = "ai".into();
        }
        let mut book = match Book::load(&self.store) {
            Ok(book) => book,
            Err(e) => {
                ui.colored_label(Color32::LIGHT_RED, e);
                return;
            }
        };
        self.chat_trust_panel(ui);
        if !view.error.is_empty() {
            ui.colored_label(Color32::LIGHT_RED, &view.error);
        }
        let mut invitations = Vec::new();
        for (who, raw, mine) in &self.chat {
            if *mine {
                continue;
            }
            let Some(sender) = who.strip_prefix("CYB:") else {
                continue;
            };
            if let Some(message) = GroupMessage::decode(raw) {
                if message.invitation(&self.node_id, sender)
                    && !book.groups.iter().any(|g| g.id == message.group.id)
                    && !invitations.iter().any(|g: &Group| g.id == message.group.id)
                {
                    invitations.push(message.group);
                }
            }
        }
        for group in invitations {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Приглашение в «{}» от {}",
                    group.title,
                    book.name(&group.creator)
                ));
                if ui.button("Присоединиться").clicked() {
                    let mut changed = book.clone();
                    changed.groups.push(group.clone());
                    match changed.save(&self.store) {
                        Ok(()) => {
                            book = changed;
                            view.selected = format!("group:{}", group.id);
                        }
                        Err(e) => view.error = e,
                    }
                }
            });
        }
        let width = ui.available_width();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width((width * 0.28).clamp(200.0, 310.0));
                ui.add(egui::TextEdit::singleline(&mut view.search).hint_text("Найти чат или друга…").desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    for (index, label) in ["Чаты", "Друзья", "Группы"].iter().enumerate() {
                        if ui.selectable_label(view.tab == index, *label).clicked() { view.tab = index; }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.small_button("+ Друг").clicked() { view.add_friend = !view.add_friend; }
                    if ui.small_button("+ Группа").clicked() { view.create_group = !view.create_group; }
                });
                ui.separator();
                if view.tab == 0 && ui.selectable_label(view.selected == "ai", "ИИ-помощник").clicked() { view.selected = "ai".into(); }
                let query = view.search.trim().to_lowercase();
                egui::ScrollArea::vertical().id_salt("cybchat-sidebar").max_height(520.0).show(ui, |ui| {
                    if view.tab != 2 {
                        if self.manual_peers.is_empty() { ui.label("Добавь друга по ID и адресу его устройства."); }
                        let mut contact_ids: Vec<_> = self.manual_peers.iter().map(|p| p.node_id.clone()).collect();
                        if view.tab == 0 {
                            for (who, raw, _) in &self.chat {
                                if raw.starts_with(crate::conversations::PREFIX) { continue; }
                                if let Some(node) = who.strip_prefix("CYB:").or_else(|| who.strip_prefix("YOU → ")) {
                                    if !contact_ids.iter().any(|id| id == node) { contact_ids.push(node.to_owned()); }
                                }
                            }
                        }
                        for node in contact_ids {
                            let name = book.name(&node).to_owned();
                            if !format!("{name} {}", node).to_lowercase().contains(&query) { continue; }
                            let id = format!("peer:{}", node);
                            let online = self.lan_peers.iter().any(|p| p.node_id == node && p.last_seen.elapsed().as_secs() <= 20);
                            if ui.selectable_label(view.selected == id, format!("{}  {name}", if online { "●" } else { "○" })).clicked() {
                                view.selected = id; self.lan_target = Some(node);
                            }
                            ui.small(if online { "В сети рядом" } else { "Доступность неизвестна" });
                            ui.add_space(6.0);
                        }
                    }
                    if view.tab != 1 {
                        for group in &book.groups {
                            if !group.title.to_lowercase().contains(&query) { continue; }
                            let id = format!("group:{}", group.id);
                            if ui.selectable_label(view.selected == id, format!("Группа: {}", group.title)).clicked() { view.selected = id; }
                            ui.small(format!("{} участников", group.members.len()));
                        }
                        if view.tab == 2 && book.groups.is_empty() { ui.label("Создай группу и выбери друзей."); }
                    }
                });
                ui.collapsing("Найти рядом", |ui| {
                    if ui.add_enabled(self.lan_scan.is_none(), egui::Button::new("Найти пользователей")).clicked() { self.start_lan_scan(); }
                    for peer in self.lan_peers.clone().into_iter().filter(|p| p.last_seen.elapsed().as_secs() <= 20) {
                        if ui.button(format!("Добавить {}", peer.node_id)).clicked() {
                            self.manual_node_input = peer.node_id; self.manual_address_input = peer.address;
                            view.add_friend = true;
                        }
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width((width - (width * 0.28).clamp(200.0, 310.0) - 30.0).max(240.0));
                let peer = view.selected.strip_prefix("peer:").map(str::to_owned);
                let group = view.selected.strip_prefix("group:").and_then(|id| book.groups.iter().find(|g| g.id == id)).cloned();
                let title = if let Some(group) = &group { group.title.clone() } else if let Some(peer) = &peer { book.name(peer).into() } else { "ИИ-помощник".into() };
                ui.heading(title);
                if let Some(group) = &group {
                    ui.collapsing(format!("Участники · {}", group.members.len()), |ui| {
                        for member in &group.members { ui.label(book.name(member)); }
                        ui.small("Защищённая прямая доставка каждому участнику. Недоступные участники не получают сообщения позже автоматически.");
                    });
                } else if let Some(peer) = &peer {
                    self.lan_target = Some(peer.clone());
                    ui.collapsing("Контакт и безопасность", |ui| {
                        ui.label(format!("ID: {peer}"));
                        let mut name = book.names.get(peer).cloned().unwrap_or_default();
                        let changed = ui.add(egui::TextEdit::singleline(&mut name).hint_text("Имя друга")).changed();
                        if changed && name.chars().count() <= 64 {
                            let mut edited = book.clone(); edited.names.insert(peer.clone(), name);
                            match edited.save(&self.store) { Ok(()) => book = edited, Err(e) => view.error = e }
                        }
                        if ui.add_enabled(self.secure_send_task.is_none(), egui::Button::new("Проверить соединение и ключ")).clicked() { self.check_secure_connection(); }
                        if let Ok(Some(key)) = crate::network::peer_trust::approved_key(&self.store, peer) { ui.label(crate::network::secure_chat::fingerprint(&key)); }
                        ui.label(&self.secure_status);
                        if ui.button("Удалить из друзей").clicked() { self.remove_manual_chat_peer(peer); view.selected = "ai".into(); }
                    });
                } else { ui.small("Локальный помощник использует твою память и граф знаний."); }
                if view.draft_for != view.selected {
                    if !view.draft_for.is_empty() { view.drafts.insert(view.draft_for.clone(), self.chat_input.clone()); }
                    self.chat_input = view.drafts.get(&view.selected).cloned().unwrap_or_default();
                    view.draft_for = view.selected.clone();
                }
                ui.separator();
                let mut seen = std::collections::HashSet::new();
                egui::ScrollArea::vertical().id_salt(("cybchat-messages", &view.selected)).max_height((ui.ctx().content_rect().height() - 440.0).clamp(180.0, 800.0)).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                    let mut count = 0;
                    for (who, raw, mine) in &self.chat {
                        let body;
                        let show = if let Some(group) = &group {
                            if let Some(message) = GroupMessage::decode(raw) {
                                let author = if *mine { self.node_id.as_str() } else { who.strip_prefix("CYB:").unwrap_or("") };
                                let valid = message.accepted(group, &self.node_id, author) && seen.insert(message.id);
                                body = message.text;
                                valid
                            } else { body = raw.clone(); false }
                        } else {
                            body = raw.clone();
                            !raw.starts_with(crate::conversations::PREFIX) && if let Some(peer) = &peer { who == &format!("CYB:{peer}") || who == &format!("YOU → {peer}") } else { who == "YOU" || who == "ROBOTCYB" }
                        };
                        if !show { continue; }
                        count += 1;
                        egui::Frame::NONE.fill(if *mine { Color32::from_rgb(16, 57, 43) } else { Color32::from_rgb(12, 29, 25) }).corner_radius(10).inner_margin(10.0).show(ui, |ui| {
                            ui.label(RichText::new(if *mine { "Ты" } else { book.name(who.strip_prefix("CYB:").unwrap_or(who)) }).small().color(Color32::from_rgb(100, 220, 165)));
                            ui.add(egui::Label::new(body).selectable(true).wrap());
                        });
                        ui.add_space(7.0);
                    }
                    if count == 0 { ui.label("Здесь появится ваша переписка."); }
                });
                ui.add(egui::TextEdit::multiline(&mut self.chat_input).desired_rows(2).desired_width(f32::INFINITY).hint_text("Напиши сообщение…"));
                let group_busy = ui.ctx().data(|d| d.get_temp::<GroupDelivery>(delivery_id()).is_some());
                let busy = (group.is_some() && group_busy) || (peer.is_some() && self.secure_send_task.is_some()) || (peer.is_none() && group.is_none() && self.robot_job.is_some());
                let route_ready = peer.as_ref().is_none_or(|id| self.chat_peers().iter().any(|p| &p.node_id == id));
                let transport_ready = (peer.is_none() && group.is_none()) || self.secure_startup_error.is_none();
                ui.horizontal(|ui| {
                    if ui.add_enabled(!busy && route_ready && transport_ready && !self.chat_input.trim().is_empty(), egui::Button::new(if busy { "Отправляется…" } else { "Отправить" })).clicked() {
                        let text = self.chat_input.trim().to_owned();
                        if let Some(group) = &group {
                            match self.send_group_message(ui.ctx(), group, &text) { Ok(()) => { view.error.clear(); }, Err(e) => view.error = e }
                        } else if peer.is_some() { self.send_secure_chat(&text); }
                        else { self.push_chat_message("YOU", text.clone(), true); let _ = self.start_robot_job(text); self.chat_input.clear(); }
                    }
                    ui.small("История хранится на этом устройстве");
                });
                if group.is_some() { ui.add(egui::Label::new(&self.secure_status).wrap()); }
                if !transport_ready { ui.colored_label(Color32::LIGHT_RED, "Соединение недоступно. Подробности — в Cyb Диспетчере. Проверь, не запущена ли другая копия CybOS."); }
                if !route_ready { ui.label("Добавь адрес этого собеседника через кнопку «+ Друг»."); }
                if peer.is_some() && self.secure_status.contains("DELIVERED") { ui.small("Последнее сообщение доставлено"); }
            });
        });
        if view.add_friend {
            egui::Window::new("Добавить друга").default_width(420.0).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).collapsible(false).show(ui.ctx(), |ui| {
                ui.label("Имя"); ui.text_edit_singleline(&mut view.name);
                ui.label("ID устройства друга"); ui.text_edit_singleline(&mut self.manual_node_input);
                ui.label("IPv4-адрес устройства"); ui.text_edit_singleline(&mut self.manual_address_input);
                ui.small("Имя используется только на твоём устройстве. Перед перепиской сверь код безопасности.");
                if ui.button("Сохранить друга").clicked() {
                    let node = self.manual_node_input.trim().to_owned();
                    self.save_manual_chat_peer();
                    if self.manual_peers.iter().any(|p| p.node_id == node) {
                        let mut edited = book.clone(); edited.names.insert(node.clone(), view.name.trim().chars().take(64).collect());
                        match edited.save(&self.store) { Ok(()) => { view.add_friend = false; view.selected = format!("peer:{node}"); view.name.clear(); }, Err(e) => view.error = e }
                    }
                }
                ui.label(&self.manual_peer_status);
                if ui.button("Закрыть").clicked() { view.add_friend = false; }
            });
        }
        if view.create_group {
            egui::Window::new("Новая группа").default_width(420.0).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).collapsible(false).show(ui.ctx(), |ui| {
                ui.add(egui::TextEdit::singleline(&mut view.group_title).hint_text("Название группы"));
                ui.label("Кого добавить?");
                for peer in &self.manual_peers {
                    let mut chosen = view.members.contains(&peer.node_id);
                    if ui.checkbox(&mut chosen, book.name(&peer.node_id)).changed() {
                        if chosen { view.members.push(peer.node_id.clone()); } else { view.members.retain(|m| m != &peer.node_id); }
                    }
                }
                ui.small("Первое сообщение станет приглашением. Друг должен принять его. Для отправки нужны проверенные ключи и адреса всех участников.");
                if ui.button("Создать группу").clicked() {
                    let mut members = view.members.clone(); members.retain(|m| m != &self.node_id); members.insert(0, self.node_id.clone());
                    let group = Group { id: uuid::Uuid::new_v4().to_string(), title: view.group_title.trim().into(), creator: self.node_id.clone(), members };
                    let mut edited = book.clone(); edited.groups.push(group.clone());
                    match edited.save(&self.store) { Ok(()) => { view.selected = format!("group:{}", group.id); view.create_group = false; view.members.clear(); view.group_title.clear(); view.error.clear(); }, Err(e) => view.error = e }
                }
                if ui.button("Отмена").clicked() { view.create_group = false; }
            });
        }
        ui.ctx().data_mut(|d| d.insert_temp(view_id(), view));
    }

    fn chat_trust_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Мой ID и код безопасности", |ui| {
            ui.add(egui::Label::new(&self.node_id).selectable(true));
            if let Some(fp) = &self.local_secure_fingerprint {
                ui.add(egui::Label::new(fp).selectable(true).wrap());
            }
        });
        for (index, item) in self.pending_secure_trust.clone().iter().enumerate().rev() {
            ui.group(|ui| {
                ui.colored_label(Color32::GOLD, format!("Проверь устройство {}", item.node_id));
                ui.label("Сверь весь код с другом по другому каналу связи. После подтверждения повтори отправку.");
                ui.add(egui::Label::new(&item.fingerprint).selectable(true).wrap());
                ui.horizontal(|ui| {
                    if ui.button("Коды совпадают").clicked() { self.approve_pending_secure_peer(index); }
                    if ui.button("Отклонить").clicked() { self.pending_secure_trust.remove(index); }
                });
            });
        }
    }
}
