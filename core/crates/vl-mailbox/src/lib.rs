//! Internet mailbox for sealed envelopes (ADR-039).
//!
//! Phones and the web app upload the same sealed envelopes that phones carry
//! for each other over Bluetooth, and fetch the ones addressed to their current
//! recipient tags. The mailbox cannot read them: it sees only the outer header
//! (rotating tag, expiry, size bucket), like any relay. Nothing identifies a
//! person, there are no accounts, and envelopes are deleted when they expire.
//! It never logs tags, addresses or envelope bytes.

use axum::extract::{ConnectInfo, DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio_postgres::Client;
use tower_http::cors::CorsLayer;
use vl_proto::{Admission, Envelope};

/// Envelopes per upload, tags per fetch, envelopes per fetch.
pub const MAX_PUT: usize = 64;
pub const MAX_TAGS: usize = 256;
pub const MAX_GET: i64 = 500;
/// Envelopes kept for one tag (stops one tag being flooded) and in total.
pub const MAX_PER_TAG: i64 = 500;
pub const MAX_TOTAL: i64 = 500_000;
/// Same lifetime limits as phones (vl-routing `Policy::default_spray`).
const MAX_LIFETIME_MIN: u32 = 7 * 24 * 60;
const SKEW_MIN: u32 = 120;
/// Requests per address per minute.
pub const RATE_PER_MIN: u32 = 120;

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS envelopes (
        envelope_id BYTEA PRIMARY KEY,
        tag         BYTEA NOT NULL,
        expiry_min  BIGINT NOT NULL,
        wire        BYTEA NOT NULL,
        stored_at   TIMESTAMPTZ NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS envelopes_tag ON envelopes(tag, stored_at);
    CREATE INDEX IF NOT EXISTS envelopes_expiry ON envelopes(expiry_min);
";

pub struct Mailbox {
    db: Client,
    full: AtomicBool,
    rate: Mutex<HashMap<IpAddr, (Instant, u32)>>,
    /// Behind Render's proxy the client address is the last X-Forwarded-For entry.
    trust_proxy: bool,
}

impl std::fmt::Debug for Mailbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Mailbox")
    }
}

pub fn now_min() -> u32 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    u32::try_from(secs / 60).unwrap_or(u32::MAX)
}

/// An uploaded envelope that passed the checks.
#[derive(Debug, PartialEq, Eq)]
pub struct Admitted {
    pub id: [u8; 16],
    pub tag: [u8; 16],
    pub expiry_min: u32,
    pub wire: Vec<u8>,
}

/// Structural check only (the mailbox has no keys): a valid VYRO LINK
/// envelope, not expired, not living longer than phones allow. Copy budget and
/// hop limit do not matter here: the mailbox hands an envelope only to whoever
/// asks for its tag, like a final hop.
pub fn admit(b64: &str, now_min: u32) -> Option<Admitted> {
    let wire = B64.decode(b64).ok()?;
    let env = Envelope::decode(&wire).ok()?;
    let h = &env.header;
    match h.admission(now_min, MAX_LIFETIME_MIN, SKEW_MIN) {
        Admission::Expired | Admission::LifetimeTooLong => None,
        _ => Some(Admitted {
            id: h.envelope_id,
            tag: h.recipient_tag,
            expiry_min: h.expiry_min,
            wire,
        }),
    }
}

impl Mailbox {
    pub async fn new(db: Client, trust_proxy: bool) -> Result<Arc<Self>, tokio_postgres::Error> {
        db.batch_execute(SCHEMA).await?;
        Ok(Arc::new(Self {
            db,
            full: AtomicBool::new(false),
            rate: Mutex::new(HashMap::new()),
            trust_proxy,
        }))
    }

    /// Deletes expired envelopes and re-checks the total. Run every few minutes.
    pub async fn purge(&self) -> Result<(), tokio_postgres::Error> {
        self.db
            .execute(
                "DELETE FROM envelopes WHERE expiry_min <= $1",
                &[&i64::from(now_min())],
            )
            .await?;
        let total: i64 = self
            .db
            .query_one("SELECT count(*) FROM envelopes", &[])
            .await?
            .get(0);
        self.full.store(total >= MAX_TOTAL, Ordering::Relaxed);
        if let Ok(mut r) = self.rate.lock() {
            r.retain(|_, (start, _)| start.elapsed() < Duration::from_secs(60));
        }
        Ok(())
    }

    fn allowed(&self, ip: IpAddr) -> bool {
        let Ok(mut r) = self.rate.lock() else {
            return false;
        };
        let e = r.entry(ip).or_insert((Instant::now(), 0));
        if e.0.elapsed() >= Duration::from_secs(60) {
            *e = (Instant::now(), 0);
        }
        e.1 += 1;
        e.1 <= RATE_PER_MIN
    }

    fn client_ip(&self, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
        if self.trust_proxy
            && let Some(ip) = headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit(',').next())
                .and_then(|v| v.trim().parse().ok())
        {
            return ip;
        }
        peer.ip()
    }
}

#[derive(Debug, Deserialize)]
pub struct PutRequest {
    pub envelopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutResponse {
    /// Envelopes now held (new or already present).
    pub accepted: u32,
}

#[derive(Debug, Deserialize)]
pub struct GetRequest {
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetResponse {
    pub envelopes: Vec<String>,
}

type Shared = State<Arc<Mailbox>>;
type Peer = ConnectInfo<SocketAddr>;

async fn put(
    State(m): Shared,
    ConnectInfo(peer): Peer,
    headers: HeaderMap,
    Json(req): Json<PutRequest>,
) -> Result<Json<PutResponse>, StatusCode> {
    if !m.allowed(m.client_ip(&headers, peer)) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    if req.envelopes.len() > MAX_PUT {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    if m.full.load(Ordering::Relaxed) {
        return Err(StatusCode::INSUFFICIENT_STORAGE);
    }
    let now = now_min();
    let mut accepted = 0;
    for a in req.envelopes.iter().filter_map(|e| admit(e, now)) {
        let (id, tag) = (a.id.to_vec(), a.tag.to_vec());
        // Insert unless this tag already holds its share; duplicates count as held.
        let n =
            m.db.execute(
                "INSERT INTO envelopes (envelope_id, tag, expiry_min, wire)
                 SELECT $1, $2, $3, $4
                 WHERE (SELECT count(*) FROM envelopes WHERE tag = $2) < $5
                    OR EXISTS (SELECT 1 FROM envelopes WHERE envelope_id = $1)
                 ON CONFLICT (envelope_id) DO NOTHING",
                &[&id, &tag, &i64::from(a.expiry_min), &a.wire, &MAX_PER_TAG],
            )
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let held = n == 1
            || m.db
                .query_opt("SELECT 1 FROM envelopes WHERE envelope_id = $1", &[&id])
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .is_some();
        accepted += u32::from(held);
    }
    Ok(Json(PutResponse { accepted }))
}

async fn fetch(
    State(m): Shared,
    ConnectInfo(peer): Peer,
    headers: HeaderMap,
    Json(req): Json<GetRequest>,
) -> Result<Json<GetResponse>, StatusCode> {
    if !m.allowed(m.client_ip(&headers, peer)) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    if req.tags.len() > MAX_TAGS {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let tags: Vec<Vec<u8>> = req
        .tags
        .iter()
        .filter_map(|t| B64.decode(t).ok())
        .filter(|t| t.len() == 16)
        .collect();
    let rows =
        m.db.query(
            "SELECT wire FROM envelopes WHERE tag = ANY($1) AND expiry_min > $2
             ORDER BY stored_at LIMIT $3",
            &[&tags, &i64::from(now_min()), &MAX_GET],
        )
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(GetResponse {
        envelopes: rows
            .iter()
            .map(|r| B64.encode(r.get::<_, Vec<u8>>(0)))
            .collect(),
    }))
}

/// The HTTP API. `web_origins` are the web chat's origins, allowed by CORS.
pub fn app(m: Arc<Mailbox>, web_origins: Vec<HeaderValue>) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .allow_origin(web_origins);
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/put", post(put))
        .route("/v1/get", post(fetch))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(cors)
        .with_state(m)
}
