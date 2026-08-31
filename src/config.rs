use std::env;
use std::path::PathBuf;

use clap::Parser;

/// Runtime configuration for the compatible HTTP service.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "film-record-lite",
    version,
    about = "FilmRecordLite Rust server"
)]
pub struct Cli {
    /// Probe the local HTTP health endpoint and exit.
    #[arg(long, hide = true)]
    pub healthcheck: bool,

    /// Address to bind (the Python service defaults to 0.0.0.0).
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// TCP port to bind.
    #[arg(long, default_value_t = 8000)]
    pub port: u16,

    /// Authentication token. Takes precedence over FILM_RECORD_TOKEN.
    #[arg(long)]
    pub token: Option<String>,

    /// SQLite database file. Defaults to data/films.db.
    #[arg(long)]
    pub db: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub token: String,
    pub db_path: PathBuf,
}

impl Config {
    pub fn from_cli(cli: Cli) -> Result<Self, String> {
        // Python uses `args.token or os.environ.get(...)`, so an explicitly
        // supplied empty CLI value falls back to the environment variable.
        let token = match cli.token {
            Some(value) if !value.is_empty() => value,
            _ => env::var("FILM_RECORD_TOKEN").map_err(|_| {
                "Authentication token required. Set --token or FILM_RECORD_TOKEN.".to_string()
            })?,
        };

        // Python's len() counts Unicode code points, so chars().count() is the
        // compatible minimum-strength check rather than byte length.
        if token.chars().count() < 8 {
            return Err("Token too weak. Minimum 8 characters required.".to_string());
        }

        Ok(Self {
            host: cli.host,
            port: cli.port,
            token,
            db_path: cli.db.unwrap_or_else(|| PathBuf::from("data/films.db")),
        })
    }
}
