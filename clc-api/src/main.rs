use camino::Utf8PathBuf;
use clap::Parser;
use tokio::net::TcpListener;
use tisket::Repo;

use clc_api::AppState;

#[derive(Parser)]
#[command(name = "clc-api")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Start the API server
    Serve {
        /// Port to listen on (0 = OS-assigned)
        #[arg(long, default_value = "3000")]
        port: u16,

        /// Root directory of the tisket repository
        #[arg(long, default_value = ".")]
        root: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Serve { port, root } => {
            let root = Utf8PathBuf::from(&root);

            // Fail fast if tisket.yml is missing
            if let Err(e) = Repo::open(&root) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }

            let app = clc_api::router(AppState { root });

            let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
                .await
                .unwrap_or_else(|e| {
                    eprintln!("error: failed to bind: {e}");
                    std::process::exit(1);
                });

            let actual_port = listener.local_addr().unwrap().port();
            eprintln!("listening on :{actual_port}");

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .unwrap_or_else(|e| {
                    eprintln!("error: server failed: {e}");
                    std::process::exit(1);
                });
        }
    }
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
