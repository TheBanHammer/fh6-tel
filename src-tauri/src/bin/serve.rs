use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};

use clap::Parser;
use fh6_tel_lib::{
    db, event::ServerEvent, server, session::SessionManager, settings, AppState, Shared,
};

#[derive(Parser, Debug)]
#[command(name = "fh6-tel-serve", about = "Headless FH6 Telemetry server + web host")]
struct Cli {
    /// Bind address for the HTTP server.
    #[arg(long, default_value = "127.0.0.1")]
    ip: IpAddr,
    /// HTTP port.
    #[arg(long, default_value_t = 8080)]
    port: u16,
    /// Optional access token. When set, browsers must log in.
    #[arg(long)]
    auth_token: Option<String>,
    /// Override the UDP telemetry port (defaults to settings.json).
    #[arg(long)]
    udp_port: Option<u16>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let loaded = settings::load();
    let udp_port = cli.udp_port.unwrap_or(loaded.port);
    let auto_record = loaded.auto_record;

    let conn = match db::open() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[serve] fatal: cannot open database: {e}");
            std::process::exit(1);
        }
    };
    let app: Shared = Arc::new(AppState {
        db: Mutex::new(conn),
        session_manager: Mutex::new(SessionManager::new(auto_record)),
        settings: Mutex::new(loaded),
    });

    let (tx, _rx) = tokio::sync::broadcast::channel::<ServerEvent>(256);

    // Ingest loop.
    {
        let app = app.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            fh6_tel_lib::udp::run(app, udp_port, tx).await;
        });
    }

    let auth = server::auth::AuthState::new(cli.auth_token.clone());
    if !cli.ip.is_loopback() && !auth.enabled() {
        eprintln!(
            "[serve] WARNING: bound to {} with no --auth-token; the dashboard is open to anyone on the network.",
            cli.ip
        );
    }

    let state = server::ServerState { app, tx, auth };
    let router = server::router(state);

    let addr = SocketAddr::new(cli.ip, cli.port);
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[serve] fatal: cannot bind {addr}: {e}");
            std::process::exit(1);
        }
    };
    println!("[serve] dashboard at http://{addr}  (UDP telemetry on :{udp_port})");

    if let Err(e) = axum::serve(listener, router).await {
        eprintln!("[serve] server error: {e}");
        std::process::exit(1);
    }
}
