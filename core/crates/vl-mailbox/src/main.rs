//! Runs the mailbox. Environment: `DATABASE_URL` (Postgres, internal URL, no
//! TLS), `PORT` (default 8080), `WEB_ORIGIN` (web chat origins, comma-separated, for CORS), and
//! `TRUST_PROXY=1` behind Render's proxy.

use std::net::SocketAddr;
use std::time::Duration;
use vl_mailbox::{Mailbox, app};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL")?;
    let (client, conn) = tokio_postgres::connect(&url, tokio_postgres::NoTls).await?;
    // ponytail: one connection; if it drops the process exits and Render restarts it.
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("database connection lost: {e}");
            std::process::exit(1);
        }
    });
    let m = Mailbox::new(client, std::env::var("TRUST_PROXY").is_ok_and(|v| v == "1")).await?;
    let purger = m.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = purger.purge().await {
                eprintln!("purge failed: {e}");
            }
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });
    // Comma-separated, e.g. "https://chat.example.com,https://example.com".
    let origins = std::env::var("WEB_ORIGIN")
        .unwrap_or_default()
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    eprintln!("mailbox listening on port {port}");
    axum::serve(
        listener,
        app(m, origins).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
