#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod brand;
mod i18n;
mod agent_run;
mod conversations;
mod movie_catalog;
mod media_stream;
mod torrent_indexer;
mod models;
mod mining;
mod mining_report;
mod wallet_vault;
mod wallet_gate;
mod store;

mod actions;
mod ai;
mod automation;
mod assets;
mod bio;
mod brain;
mod config;
mod cyblex;
mod cybdex;
mod graph;
mod hardware;
mod navigation;
mod network;
mod oracle;
mod power;
mod runtime;
mod shell;
mod state;
mod stacks;
mod theme;
mod technical_analysis;
mod ui;

#[cfg(test)]
mod tests;

pub(crate) use navigation::{Icon, Page};
pub(crate) use state::CybOs;

use eframe::egui;

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CybOS — личное пространство")
            .with_fullscreen(true)
            .with_icon(brand::window_icon())
            .with_inner_size([1320.0, 840.0])
            .with_min_inner_size([1000.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "cybOS",
        opts,
        Box::new(|cc| { i18n::install_fonts(&cc.egui_ctx); Ok(Box::new(wallet_gate::Gate::new())) }),
    )
}
