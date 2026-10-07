#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod store;

mod actions;
mod ai;
mod app;
mod assets;
mod brain;
mod cameras;
mod chat;
mod farm;
mod graph;
mod network;
mod robot;
mod shell;
mod state;
mod theme;
mod ui;

use crate::models::TokenMarket;
pub(crate) use app::{CybOs, Icon, Page};

use eframe::egui;
const CICADAFARM_MINT: &str = "9QLCEL7Xo9VTwgBeAYU1PWX7JJ8joKxQCYw3msjUpump";
const ROBOTCYB_MINT: &str = "8WZiguAp8NyFnwm8Z97k6sCbSRCWaW1YYXKeCTpupump";

static TOKEN_MARKET_CACHE: std::sync::OnceLock<
    std::sync::Mutex<(std::time::Instant, Vec<TokenMarket>)>,
> = std::sync::OnceLock::new();

const APP_VERSION: &str = "0.6.0";

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
