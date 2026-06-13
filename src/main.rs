//! simple-proxy: a minimalist Layer-4 TCP forwarder.
//!
//! Two subcommands:
//!   * `serve`   — config-driven daemon (dynamic/static target, allow-list, restart).
//!   * `forward` — one-shot single port pair.
//!
//! Protocol-agnostic: SSH, HTTP, HTTPS and anything else pass through as raw bytes.

use anyhow::Result;
use simple_proxy::logln;
use simple_proxy::{cli, config, proxy, shutdown};

const APP_NAME: &str = concat!("Simple Proxy v", env!("CARGO_PKG_VERSION"));

#[tokio::main]
async fn main() -> Result<()> {
    let command = match cli::parse(std::env::args()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    // Register signal handling exactly once, then fan out via the shutdown channel.
    // (The JS version registered SIGTERM/SIGINT inside each per-port forwarder, so N
    // ports installed N duplicate handlers — fixed here.)
    let (controller, shutdown) = shutdown::channel();
    tokio::spawn(async move {
        wait_for_signal().await;
        logln!("Shutdown signal received");
        controller.trigger();
    });

    logln!("{APP_NAME}");
    match command {
        cli::Command::Serve { config } => {
            logln!("Loading config from {}", config.display());
            let cfg = config::Config::load(&config)?;
            proxy::run_serve(cfg, shutdown).await?;
        }
        cli::Command::Forward { source, target } => {
            proxy::run_forward(source, target, shutdown).await?;
        }
    }
    Ok(())
}

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
