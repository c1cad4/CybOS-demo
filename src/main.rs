#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod store;
mod network;

mod actions;
mod ai;
mod assets;
mod brain;
mod cell;
mod config;
mod crypto;
mod graph;
mod identity;
mod navigation;
mod runtime;
mod shell;
mod state;
mod theme;
mod ui;

#[cfg(test)]
mod tests;

pub(crate) use navigation::{Icon, Page};
pub(crate) use state::CybOs;

use eframe::egui;

fn main() -> eframe::Result {
    #[cfg(debug_assertions)]
    {
        let mut args = std::env::args().skip(1);
        if let Some(command) = args.next() {
            if command == "--self-test" {
                match args.next().as_deref() {
                    Some("onion") | None => {
                        match network::lan::run_headless_onion_test() {
                            Ok(()) => return Ok(()),
                            Err(error) => {
                                eprintln!("cybOS onion self-test failed: {error}");
                                std::process::exit(2);
                            }
                        }
                    }
                    Some("help") | Some("--help") | Some("-h") => {
                        println!("cybOS self-tests:");
                        println!("  cargo run -- --self-test onion");
                        return Ok(());
                    }
                    Some(name) => {
                        eprintln!("unknown self-test '{name}'. Use '--self-test help'.");
                        std::process::exit(2);
                    }
                }
            }
        }
    }
    #[cfg(debug_assertions)]
    if std::env::var_os("CYBOS_HEADLESS_ONION_TEST").is_some() {
        if let Err(error) = network::lan::run_headless_onion_test() {
            eprintln!("cybOS headless onion test failed: {error}");
            std::process::exit(2);
        }
        return Ok(());
    }

    #[cfg(debug_assertions)]
    if std::env::var_os("CYBOS_HEADLESS_TEST_NODE").is_some() {
        if let Err(error) = network::lan::run_headless_test_node() {
            eprintln!("cybOS headless test node failed: {error}");
            std::process::exit(2);
        }
        return Ok(());
    }

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
