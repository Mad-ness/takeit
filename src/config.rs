use clap::{Parser, ArgAction};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::path::{PathBuf, Path};
use std::env;
use std::convert::Into;
use tracing::Level;

#[derive(clap::ValueEnum, Default, Debug, Clone)]
pub enum LogLevel {
    #[default]
    Info,
    Warn,
    Debug,
    Trace,
    Error,
}

impl Into<String> for LogLevel {
    fn into(self) -> String {
        let level = match self {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
            LogLevel::Error => "error",
        };
        level.into()
    }
}

impl Into<Level> for LogLevel {
    fn into(self) -> Level {
        match self {
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Error => Level::ERROR,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Root directory with documents
    #[arg(short, long)] // , default_value_t = default_storedir())]
    pub collection_dir: PathBuf,
    /// Address:port to run the server on
    #[arg(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8009))]
    pub bind: SocketAddr,
    /// Log level
    #[arg(value_enum, short, long, default_value_t = LogLevel::default())]
    pub log_level: LogLevel,
    /// Enable web UI
    #[arg(short, long, default_value_t = false)]
    pub enable_ui: bool,
    /// Ignore bad documents. If true it will fail if any document incorrect
    #[arg(short, long, default_value_t = false)]
    pub ignore_bad_documents: bool,
}

impl CliArgs {
    pub fn log_level_as_str(&self) -> String {
        self.log_level.clone().into()
    }
}



///
/// Get default store dir
///
// fn default_storedir() -> String {
fn default_storedir() -> PathBuf {
    // env::current_dir().unwrap_or(PathBuf::from(r".")).to_str().unwrap().to_string()
    env::current_dir().unwrap_or(PathBuf::from(r"."))
}

pub fn cli_args() -> CliArgs {
    CliArgs::parse()
}
