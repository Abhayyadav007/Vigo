use axum::{
    Router,
    http::{HeaderName, HeaderValue, Method, Request, StatusCode, header},
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{routes, state::AppState};

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub fn build_router(state: AppState) -> Router {
    let layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<_>| {
                    let request_id = req
                        .headers()
                        .get(&REQUEST_ID)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or_default();
                    tracing::info_span!(
                        "http",
                        method = %req.method(),
                        uri = %req.uri(),
                        request_id,
                    )
                })
                // One access-log line per request (the default level is DEBUG).
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(cors(&state.config.cors_allowed_origins))
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            state.config.request_timeout,
        ));

    routes::router().layer(layers).with_state(state)
}

fn cors(allowed: &[String]) -> CorsLayer {
    let origins = if allowed.is_empty() {
        // Unset in local dev: allow any origin. Auth uses bearer tokens, not cookies.
        AllowOrigin::any()
    } else {
        AllowOrigin::list(allowed.iter().filter_map(|o| HeaderValue::from_str(o).ok()))
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, REQUEST_ID])
        .expose_headers([REQUEST_ID])
        .max_age(std::time::Duration::from_secs(3600))
}
