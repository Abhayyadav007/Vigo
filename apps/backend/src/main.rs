use anyhow::Context;
use backend::{app, config::Config, services::user_service, state::AppState};
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const USAGE: &str = "usage: backend [serve | promote-admin <+91XXXXXXXXXX>]";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A missing .env is fine; real deployments set env vars directly.
    let _ = dotenvy::dotenv();
    let config = Config::from_env()?;
    init_tracing(&config);

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["serve"] => serve(config).await,
        ["promote-admin", phone] => promote_admin(config, phone).await,
        _ => anyhow::bail!(USAGE),
    }
}

async fn serve(config: Config) -> anyhow::Result<()> {
    let addr = config.addr;
    let state = AppState::connect(config).await?;
    let app = app::build_router(state);

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "backend listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving HTTP")?;
    Ok(())
}

/// Bootstraps the first admin. The user must have signed in once.
async fn promote_admin(config: Config, phone: &str) -> anyhow::Result<()> {
    let state = AppState::connect(config).await?;
    let user = user_service::promote_admin_by_phone(&state, phone)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{} ({}) is now ADMIN", user.phone, user.id);
    Ok(())
}

fn init_tracing(config: &Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,backend=debug,tower_http=info,sqlx=warn"));
    let registry = tracing_subscriber::registry().with(filter);
    if config.is_production() {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = signal::ctrl_c().await {
            tracing::error!(%error, "failed to listen for ctrl-c");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to listen for SIGTERM"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
