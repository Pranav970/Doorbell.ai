use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
pub use metrics_exporter_prometheus::PrometheusHandle;
use metrics_exporter_prometheus::{BuildError, PrometheusBuilder};

/// Installs the global Prometheus recorder and returns a handle whose
/// `.render()` produces the exposition-format text served at `GET /metrics`.
/// Call exactly once at startup, before any `metrics::counter!`/`gauge!`/
/// `histogram!` call fires elsewhere (they're no-ops without a recorder).
pub fn install_metrics_recorder() -> Result<PrometheusHandle, BuildError> {
    let handle = PrometheusBuilder::new().install_recorder()?;
    metrics::gauge!("process_start_time_seconds").set(unix_timestamp_now());
    Ok(handle)
}

fn unix_timestamp_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Axum middleware recording `http_requests_total{method,path,status}` and
/// `http_request_duration_seconds{method,path}` for every request. Uses
/// `MatchedPath` (the route *template*, e.g. `/api/teams/:team_id`) rather
/// than the literal URI — otherwise every distinct UUID in a path would
/// mint its own metric series. Basic HTTP metrics only, per this epic's
/// scope; per-stage pipeline metrics land once the `Stage` trait exists.
pub async fn track_http_metrics(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());

    let start = Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    metrics::counter!(
        "http_requests_total",
        "method" => method.clone(), "path" => path.clone(), "status" => status
    )
    .increment(1);
    metrics::histogram!("http_request_duration_seconds", "method" => method, "path" => path)
        .record(elapsed);

    response
}
