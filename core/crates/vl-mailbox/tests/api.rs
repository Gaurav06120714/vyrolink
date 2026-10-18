//! The mailbox against a real Postgres. Needs `MAILBOX_TEST_DB`, for example
//! `host=/tmp dbname=vl_mailbox_test`; the test fails (never skips) without it.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use http_body_util::BodyExt;
use std::net::SocketAddr;
use tower::ServiceExt;
use vl_mailbox::*;
use vl_proto::{Envelope, EnvelopeKind, Header};

async fn mailbox() -> std::sync::Arc<Mailbox> {
    let url = std::env::var("MAILBOX_TEST_DB").expect("set MAILBOX_TEST_DB");
    let (client, conn) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .unwrap();
    tokio::spawn(conn);
    client
        .batch_execute("DROP TABLE IF EXISTS envelopes")
        .await
        .unwrap();
    Mailbox::new(client, false).await.unwrap()
}

fn envelope(id: u8, tag: u8, expiry_min: u32) -> String {
    let env = Envelope {
        header: Header {
            kind: EnvelopeKind::Data,
            flags: 0,
            hop_count: 0,
            hop_limit: 8,
            copy_budget: 4,
            envelope_id: [id; 16],
            recipient_tag: [tag; 16],
            expiry_min,
            ack_commitment: [9; 16],
            body_len: 256,
        },
        body: vec![0xAB; 256],
    };
    B64.encode(env.encode())
}

async fn call(
    m: &std::sync::Arc<Mailbox>,
    path: &str,
    json: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let mut req = Request::post(path)
        .header("content-type", "application/json")
        .body(Body::from(json.to_string()))
        .unwrap();
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(SocketAddr::from((
            [10, 0, 0, 1],
            1,
        ))));
    let res = app(m.clone(), Vec::new()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

fn tag(t: u8) -> String {
    B64.encode([t; 16])
}

// One test so the shared table is not raced by parallel tests.
#[tokio::test]
async fn stores_only_valid_envelopes_and_returns_them_by_tag() {
    let m = mailbox().await;
    let now = now_min();
    let good = envelope(1, 7, now + 60);
    let other = envelope(2, 8, now + 60);
    let (s, r) = call(
        &m,
        "/v1/put",
        serde_json::json!({ "envelopes": [
            good, other, good,
            envelope(3, 7, now.saturating_sub(1)),          // expired
            envelope(4, 7, now + 30 * 24 * 60),              // lives too long
            B64.encode(b"not an envelope"),
            "%%%not base64",
        ]}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(r["accepted"], 3, "two new, one duplicate; the rest refused");

    let (_, r) = call(&m, "/v1/get", serde_json::json!({ "tags": [tag(7)] })).await;
    assert_eq!(
        r["envelopes"],
        serde_json::json!([good]),
        "exact bytes, only that tag, once"
    );
    let (_, r) = call(
        &m,
        "/v1/get",
        serde_json::json!({ "tags": [tag(5), "bad"] }),
    )
    .await;
    assert_eq!(r["envelopes"], serde_json::json!([]));

    // Too many at once is refused whole.
    let many: Vec<String> = (0..=MAX_PUT)
        .map(|i| envelope(100, 1, now + i as u32 + 1))
        .collect();
    let (s, _) = call(&m, "/v1/put", serde_json::json!({ "envelopes": many })).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);

    // Expired envelopes are purged and never served.
    m.purge().await.unwrap();
    let (_, r) = call(
        &m,
        "/v1/get",
        serde_json::json!({ "tags": [tag(7), tag(8)] }),
    )
    .await;
    assert_eq!(r["envelopes"].as_array().unwrap().len(), 2);

    // One address is limited per minute.
    let mut limited = false;
    for _ in 0..RATE_PER_MIN {
        let (s, _) = call(&m, "/v1/get", serde_json::json!({ "tags": [] })).await;
        limited |= s == StatusCode::TOO_MANY_REQUESTS;
    }
    assert!(limited);
}
