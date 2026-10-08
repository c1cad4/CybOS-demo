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

use base64::Engine as _;
use eframe::egui;

fn print_identity() -> eframe::Result {
    let store = store::Store::open();
    let identity = identity::NodeIdentity::load_or_create(&store);
    println!("NODE_ID {}", identity.node_id());
    println!("FINGERPRINT {}", crypto::fingerprint(identity.public_key()));
    println!("PUBLIC_KEY {}", base64::engine::general_purpose::STANDARD.encode(identity.public_key()));
    Ok(())
}

fn provision_peer(node_id: &str, fingerprint: &str) -> eframe::Result {
    let normalized = fingerprint.to_ascii_lowercase();
    let valid = normalized.len() == 64
        && normalized
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit());
    if !valid {
        eprintln!("invalid fingerprint: expected 64 hexadecimal SHA-256 characters");
        std::process::exit(2);
    }
    if node_id.trim().is_empty() || node_id.len() > 128 {
        eprintln!("invalid peer node id");
        std::process::exit(2);
    }

    let store = store::Store::open();
    store.provision_peer_fingerprint(node_id, &normalized);
    println!(
        "OOB_PROVISIONED node_id={} fingerprint={}",
        node_id, normalized
    );
    Ok(())
}

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    if let Some(command) = args.next() {
        match command.as_str() {
            "--identity" => return print_identity(),
            "--provision-peer" => {
                let Some(node_id) = args.next() else {
                    eprintln!("usage: cybOS --provision-peer NODE_ID FINGERPRINT");
                    std::process::exit(2);
                };
                let Some(fingerprint) = args.next() else {
                    eprintln!("usage: cybOS --provision-peer NODE_ID FINGERPRINT");
                    std::process::exit(2);
                };
                return provision_peer(&node_id, &fingerprint);
            }
            "--help" | "-h" => {
                println!("cybOS commands:");
                println!("  cybOS --identity");
                println!("      Print local node ID, SHA-256 fingerprint and public key.");
                println!("  cybOS --provision-peer NODE_ID FINGERPRINT");
                println!("      Pre-provision a peer fingerprint for automatic LAN trust.");
                println!("  cybOS --self-test onion");
                println!("      Run the live two-relay routed transport self-test.");
                println!("  cybOS --self-test onion-process");
                println!("      Run the process-isolated relay fault-recovery self-test.");
                return Ok(());
            }
            #[cfg(any(debug_assertions, feature = "qa"))]
            "--self-test" => {
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
                    Some("onion-process") => {
                        match network::lan::run_process_isolated_onion_test() {
                            Ok(()) => return Ok(()),
                            Err(error) => {
                                eprintln!("cybOS process-isolated onion self-test failed: {error}");
                                std::process::exit(2);
                            }
                        }
                    }
                    Some("onion-bind-process") => {
                        match network::lan::run_process_isolated_onion_bind_crash_test() {
                            Ok(()) => return Ok(()),
                            Err(error) => {
                                eprintln!("cybOS onion-bind crash self-test failed: {error}");
                                std::process::exit(2);
                            }
                        }
                    }
                    Some("all") => {
                        if let Err(error) = network::lan::run_headless_onion_test() {
                            eprintln!("cybOS onion self-test failed: {error}");
                            std::process::exit(2);
                        }
                        if let Err(error) = network::lan::run_process_isolated_onion_test() {
                            eprintln!("cybOS process-isolated onion self-test failed: {error}");
                            std::process::exit(2);
                        }
                        if let Err(error) = network::lan::run_process_isolated_onion_bind_crash_test() {
                            eprintln!("cybOS onion-bind crash self-test failed: {error}");
                            std::process::exit(2);
                        }
                        return Ok(());
                    }
                    Some("help") | Some("--help") | Some("-h") => {
                        println!("cybOS self-tests:");
                        println!("  cybOS --self-test onion");
                        println!("  cybOS --self-test onion-process");
                        println!("  cybOS --self-test onion-bind-process");
                        println!("  cybOS --self-test all");
                        return Ok(());
                    }
                    Some(name) => {
                        eprintln!("unknown self-test '{name}'. Use '--self-test help'.");
                        std::process::exit(2);
                    }
                }
            }
            #[cfg(not(any(debug_assertions, feature = "qa")))]
            "--self-test" => {
                eprintln!("self-tests are not included in this product build; use cybOS-QA.app");
                std::process::exit(2);
            }
            _ => {}
        }
    }

    #[cfg(any(debug_assertions, feature = "qa"))]
    if std::env::var_os("CYBOS_HEADLESS_TEST_NODE").is_some() {
        if let Err(error) = network::lan::run_headless_test_node() {
            eprintln!("cybOS headless test node failed: {error}");
            std::process::exit(2);
        }
        return Ok(());
    }

    #[cfg(any(debug_assertions, feature = "qa"))]
    if std::env::var_os("CYBOS_HEADLESS_ONION_TEST").is_some() {
        if let Err(error) = network::lan::run_headless_onion_test() {
            eprintln!("cybOS headless onion test failed: {error}");
            std::process::exit(2);
        }
        return Ok(());
    }

    #[cfg(any(debug_assertions, feature = "qa"))]
    if std::env::var_os("CYBOS_HEADLESS_ONION_PROCESS_TEST").is_some() {
        if let Err(error) = network::lan::run_process_isolated_onion_test() {
            eprintln!("cybOS process-isolated onion test failed: {error}");
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
