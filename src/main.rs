#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod api;
mod config;
mod collection;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing::Level;
use config::LogLevel;

fn init_logger(level: Level) {
    fn configure(level: Level) {
        match tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_max_level(level)
            .with_span_events(FmtSpan::CLOSE)
            .with_ansi(true)
            .try_init()
        {
            Err(err) => panic!("configure logger error"),
            _ => ()
        }
    }
    configure(level);
    tracing::info!("logging level {}", &level);
}


#[tokio::main]
async fn main() {
    let cli_args = config::cli_args();
    init_logger(cli_args.log_level.clone().into());
    let _ = api::run_server(&cli_args).await.expect("failed to run server");
}
