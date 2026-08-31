mod auth;
mod config;
mod db;
mod error;
mod format;
mod models;
mod routes;

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use axum::http::Request;
use clap::Parser;
use tokio::signal;
use tower_http::trace::TraceLayer;

use crate::auth::AuthState;
use crate::config::{Cli, Config};
use crate::db::FilmDatabase;

#[derive(Clone)]
pub struct AppState {
    pub db: FilmDatabase,
    pub auth: AuthState,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    if cli.healthcheck {
        probe_health(cli.port)?;
        return Ok(());
    }

    let config = Config::from_cli(cli)?;
    create_parent_directory(&config.db_path)?;

    let db = FilmDatabase::connect(&config.db_path).await?;
    let state = AppState {
        db,
        auth: AuthState::new(&config.token),
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let app = routes::build_router(state).layer(TraceLayer::new_for_http().make_span_with(
        |request: &Request<_>| {
            // Log only the path, never the query string where legacy clients
            // may place `token`.
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                path = %request.uri().path()
            )
        },
    ));
    let address: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(address = %address, "FilmRecordLite Rust server started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn probe_health(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let timeout = Duration::from_secs(2);
    let address = ("127.0.0.1", port)
        .to_socket_addrs()?
        .next()
        .ok_or("healthcheck address could not be resolved")?;
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;

    let mut response = [0_u8; 64];
    let bytes_read = stream.read(&mut response)?;
    let status_line = std::str::from_utf8(&response[..bytes_read])?
        .lines()
        .next()
        .unwrap_or_default();
    if status_line.contains(" 200 ") {
        Ok(())
    } else {
        Err(format!("healthcheck failed: {status_line}").into())
    }
}

fn create_parent_directory(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };
        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = terminate => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = signal::ctrl_c().await;
    }
}
