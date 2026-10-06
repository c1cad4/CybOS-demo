use crate::{CybOs, CICADAFARM_MINT};
use eframe::egui;
use egui::{Color32, RichText, Vec2};

impl CybOs {
        pub(crate) fn farm(&mut self, ui: &mut egui::Ui) {
            if self.balance_refresh.elapsed() >= std::time::Duration::from_secs(30) {
                self.refresh_cicada_balances();
                self.balance_refresh = std::time::Instant::now();
            }
    
            let neon = Color32::from_rgb(0, 255, 150);
            let dim = Color32::from_rgb(55, 145, 105);
            let panel = Color32::from_rgb(5, 18, 13);
            let soft = Color32::from_rgb(170, 225, 195);
    
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("⌬  CICADAFARM")
                        .size(24.0)
                        .strong()
                        .color(neon),
                );
    
                ui.label(
                    RichText::new(
                        "LIVING FARM · DIRECT ACCESS · $CICADAFARM COMMERCE"
                    )
                    .size(11.0)
                    .color(dim),
                );
    
                ui.add_space(14.0);
    
                // FARM STATUS
                ui.horizontal(|ui| {
                    for (symbol, title, value) in [
                        ("∿", "CHICKENS", "60+"),
                        ("∆", "GOATS", "2"),
                        ("∞", "HIVES", "4"),
                        ("◉", "LAKES", "3"),
                    ] {
                        egui::Frame::NONE
                            .fill(panel)
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.set_min_width(115.0);
    
                                ui.label(
                                    RichText::new(symbol)
                                        .size(21.0)
                                        .color(neon),
                                );
    
                                ui.label(
                                    RichText::new(title)
                                        .size(8.0)
                                        .strong()
                                        .color(dim),
                                );
    
                                ui.label(
                                    RichText::new(value)
                                        .size(17.0)
                                        .strong()
                                        .color(soft),
                                );
                            });
                    }
                });
    
                ui.add_space(14.0);
    
                // ------------------------------------------------
                // WALLET
                // ------------------------------------------------
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("◈  CICADAFARM PAYMENT WALLET")
                                    .size(12.0)
                                    .strong()
                                    .color(neon),
                            );
    
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("SOLANA")
                                            .size(9.0)
                                            .strong()
                                            .color(dim),
                                    );
                                },
                            );
                        });
    
                        ui.add_space(8.0);
    
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&self.cicada_wallet)
                                    .size(11.0)
                                    .monospace()
                                    .color(soft),
                            );
    
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("⧉")
                                            .size(17.0)
                                            .color(neon),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("Copy wallet")
                                .clicked()
                            {
                                ui.ctx().copy_text(self.cicada_wallet.clone());
                                self.payment_status =
                                    "WALLET COPIED".into();
                            }
                        });
    
                        ui.add_space(7.0);
    
                        ui.horizontal(|ui| {
                            let balance = match self.cicada_balance {
                                Some(v) => format!("{:.4}", v),
                                None => "CHECKING…".into(),
                            };
    
                            ui.label(
                                RichText::new(format!(
                                    "$CICADAFARM BALANCE  {}",
                                    balance
                                ))
                                .size(10.0)
                                .strong()
                                .color(neon),
                            );
    
                            ui.add_space(20.0);
    
                            let sol = match self.sol_balance {
                                Some(v) => format!("{:.6} SOL", v),
                                None => "SOL BALANCE · CHECKING…".into(),
                            };
    
                            ui.label(
                                RichText::new(sol)
                                    .size(10.0)
                                    .color(dim),
                            );
                        });
                    });
    
                ui.add_space(12.0);
    
                // ------------------------------------------------
                // PRODUCTS
                // ------------------------------------------------
                ui.horizontal(|ui| {
                    // HONEY
                    egui::Frame::NONE
                        .fill(panel)
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.set_min_width(310.0);
    
                            ui.label(
                                RichText::new("⊙  FARM HONEY")
                                    .size(18.0)
                                    .strong()
                                    .color(neon),
                            );
    
                            ui.label(
                                RichText::new("1 KG")
                                    .size(12.0)
                                    .strong()
                                    .color(soft),
                            );
    
                            ui.add_space(8.0);
    
                            ui.label(
                                RichText::new("40,000,000 $CICADAFARM")
                                    .size(17.0)
                                    .strong()
                                    .color(neon),
                            );
    
                            ui.add_space(5.0);
    
                            ui.label(
                                RichText::new("CURRENTLY UNAVAILABLE")
                                    .size(10.0)
                                    .strong()
                                    .color(Color32::from_rgb(230, 150, 80)),
                            );
    
                            ui.add_space(9.0);
    
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    RichText::new("⊙  BUY FOR $CICADAFARM")
                                        .size(11.0)
                                        .strong(),
                                )
                                .min_size(Vec2::new(250.0, 36.0)),
                            );
    
                            ui.label(
                                RichText::new("Payment disabled while stock = 0")
                                    .size(9.0)
                                    .color(dim),
                            );
                        });
    
                    ui.add_space(12.0);
    
                    // EGGS
                    egui::Frame::NONE
                        .fill(panel)
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.set_min_width(310.0);
    
                            ui.label(
                                RichText::new("○  FARM EGGS")
                                    .size(18.0)
                                    .strong()
                                    .color(neon),
                            );
    
                            ui.label(
                                RichText::new("50 EGGS")
                                    .size(12.0)
                                    .strong()
                                    .color(soft),
                            );
    
                            ui.add_space(8.0);
    
                            ui.label(
                                RichText::new("10,000,000 $CICADAFARM")
                                    .size(17.0)
                                    .strong()
                                    .color(neon),
                            );
    
                            ui.add_space(5.0);
    
                            ui.label(
                                RichText::new("PICKUP ONLY · CICADAFARM")
                                    .size(10.0)
                                    .strong()
                                    .color(soft),
                            );
    
                            ui.label(
                                RichText::new("NO DELIVERY")
                                    .size(9.0)
                                    .color(dim),
                            );
    
                            ui.add_space(9.0);
    
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("○  BUY FOR $CICADAFARM")
                                            .size(11.0)
                                            .strong()
                                            .color(neon),
                                    )
                                    .min_size(Vec2::new(250.0, 36.0)),
                                )
                                .clicked()
                            {
                                self.payment_uri = format!(
                                    "solana:{}?amount=10000000&spl-token={}",
                                    self.cicada_wallet,
                                    CICADAFARM_MINT
                                );
    
                                self.payment_status =
                                    "EGGS PAYMENT REQUEST READY · PICKUP ONLY".into();
    
                                self.add_event(
                                    "FARM",
                                    "Eggs payment request created: 50 eggs",
                                );
                                self.notify("EGGS PAYMENT REQUEST READY");
                            }
                        });
                });
    
                ui.add_space(12.0);
    
                // PAYMENT REQUEST
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("↗  PAYMENT REQUEST")
                                    .size(11.0)
                                    .strong()
                                    .color(neon),
                            );
    
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(&self.payment_status)
                                            .size(9.0)
                                            .color(dim),
                                    );
                                },
                            );
                        });
    
                        ui.add_space(7.0);
    
                        if self.payment_uri.is_empty() {
                            ui.label(
                                RichText::new(
                                    "Select a product to generate a Solana payment request."
                                )
                                .size(10.0)
                                .color(dim),
                            );
                        } else {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut self.payment_uri,
                                    )
                                    .desired_width(f32::INFINITY),
                                );
    
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("⧉")
                                                .size(17.0)
                                                .color(neon),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text("Copy payment request")
                                    .clicked()
                                {
                                    ui.ctx().copy_text(
                                        self.payment_uri.clone(),
                                    );
    
                                    self.payment_status =
                                        "PAYMENT REQUEST COPIED".into();
                                }
                            });
                        }
    
                        ui.add_space(7.0);
    
                        ui.label(
                            RichText::new(
                                "Payment is a request only. cybOS does not claim payment confirmation until the transaction is verified."
                            )
                            .size(9.0)
                            .color(dim),
                        );
                    });
            });
        }
}
