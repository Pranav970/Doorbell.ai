use std::future::Future;
use std::time::Duration;

/// Resolves on SIGTERM (or SIGINT, for local `Ctrl-C` during development) —
/// the reusable "detect the signal" half of graceful shutdown. Kept separate
/// from the deadline/watchdog logic below so that logic can be tested with
/// an arbitrary trigger future instead of a real OS signal.
pub async fn wait_for_os_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await
    }
}

/// Given a `trigger` future (in production: [`wait_for_os_signal`]; in
/// tests: anything), waits for it to resolve, logs that shutdown started,
/// then spawns a watchdog that force-exits the process if `deadline` elapses
/// before the caller's own future (typically `axum::serve(...)`) has
/// finished draining in-flight requests on its own. Pass the returned
/// future to `axum::serve(...).with_graceful_shutdown(...)`.
pub async fn with_deadline_watchdog<F: Future<Output = ()>>(trigger: F, deadline: Duration) {
    trigger.await;
    tracing::info!(
        deadline_secs = deadline.as_secs(),
        "shutdown triggered; draining in-flight requests"
    );
    tokio::spawn(async move {
        tokio::time::sleep(deadline).await;
        tracing::warn!(
            deadline_secs = deadline.as_secs(),
            "graceful shutdown deadline exceeded; forcing exit"
        );
        std::process::exit(0);
    });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use axum::routing::get;
    use axum::Router;

    use super::*;

    /// Exercises the exact function `main.rs` wires into
    /// `axum::serve(...).with_graceful_shutdown(...)`, using a fake trigger
    /// (fires after 50ms) instead of a real OS signal — sending a literal
    /// SIGTERM to the test process isn't how anything else in this repo is
    /// tested, and would affect every test running concurrently in the same
    /// binary. A generous 30s deadline means the watchdog never fires
    /// during this test (it gets dropped when the test's runtime shuts
    /// down), so this only proves the drain side, not the force-exit side.
    #[tokio::test]
    async fn graceful_shutdown_drains_in_flight_requests_before_exiting() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let in_flight_clone = in_flight.clone();
        let completed_clone = completed.clone();

        let app = Router::new().route(
            "/slow",
            get(move || {
                let in_flight = in_flight_clone.clone();
                let completed = completed_clone.clone();
                async move {
                    in_flight.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    completed.fetch_add(1, Ordering::SeqCst);
                    "ok"
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let shutdown = with_deadline_watchdog(
            async {
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
            Duration::from_secs(30),
        );

        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown)
                .await
                .unwrap();
        });

        // Give the listener a moment to actually be accepting, then fire a
        // request that will still be sleeping when the 50ms shutdown
        // trigger above resolves.
        tokio::time::sleep(Duration::from_millis(10)).await;
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{addr}/slow"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        assert_eq!(
            completed.load(Ordering::SeqCst),
            1,
            "in-flight request should have completed"
        );
        assert_eq!(in_flight.load(Ordering::SeqCst), 1);

        server.await.unwrap();
    }
}
