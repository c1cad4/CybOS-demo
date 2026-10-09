#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod store;
mod agent_economy;
mod agent_runtime;

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
mod language;
mod network;
mod oracle;
mod power;
mod runtime;
mod shell;
mod state;
mod stacks;
mod theme;
mod ui;

#[cfg(test)]
mod tests;

pub(crate) use navigation::{Icon, Page};
pub(crate) use state::CybOs;

use eframe::egui;

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("cybOS — CicadaFarm + RobotCYB")
            .with_inner_size([1320.0, 840.0])
            .with_min_inner_size([1000.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "cybOS",
        opts,
        Box::new(|_cc| Ok(Box::new(CybOs::default()))),
    )
}
