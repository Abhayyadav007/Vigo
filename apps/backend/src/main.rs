use anyhow::Context;
use backend::{
    app,
    config::Config,
    services::{dispatch_service, order_service, user_service},
    state::AppState,
};
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const USAGE: &str = "usage: backend [serve | promote-admin <+91XXXXXXXXXX> | seed-demo]";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A missing .env is fine; real deployments set env vars directly.
    let env_file = dotenvy::dotenv().ok();
    let config = Config::from_env(env_file.as_deref().and_then(std::path::Path::parent))?;
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
        ["seed-demo"] => seed_demo(config).await,
        _ => anyhow::bail!(USAGE),
    }
}

async fn serve(config: Config) -> anyhow::Result<()> {
    let addr = config.addr;
    let state = AppState::connect(config).await?;
    let sweeper = tokio::spawn(sweep_reservations(state.clone()));
    let dispatcher = tokio::spawn(dispatch_loop(state.clone()));
    let app = app::build_router(state);

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "backend listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving HTTP")?;
    sweeper.abort();
    dispatcher.abort();
    Ok(())
}

/// Re-offers packed orders nobody has accepted yet. Safe on every instance:
/// an open wave or a winner in Redis makes the others skip the order.
async fn dispatch_loop(state: AppState) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        if let Err(error) = dispatch_service::tick(&state).await {
            tracing::warn!(%error, "dispatch tick failed");
        }
    }
}

/// Releases stock held by abandoned checkouts. Idempotent, so every instance
/// can run it.
async fn sweep_reservations(state: AppState) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(15));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        match order_service::sweep_expired_reservations(&state).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(released = n, "expired reservations released"),
            Err(error) => tracing::warn!(%error, "reservation sweep failed"),
        }
    }
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

/// Loads demo stores, catalog and stock (idempotent). Refused in production.
async fn seed_demo(config: Config) -> anyhow::Result<()> {
    anyhow::ensure!(!config.is_production(), "seed-demo is for development only");
    let state = AppState::connect(config).await?;
    sqlx::raw_sql(include_str!("../seeds/demo.sql"))
        .execute(&state.db)
        .await
        .context("running seeds/demo.sql")?;
    println!("demo data loaded: 2 Bengaluru stores, 6 categories, 24 products");
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
