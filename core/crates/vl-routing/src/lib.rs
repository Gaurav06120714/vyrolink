//! Store-and-forward routing for VYRO LINK (Doc 18 §7.3).
//!
//! [`LinkSession`] is a transport-independent state machine run once per
//! neighbour connection. It speaks the link messages of `vl-proto`, decides
//! what to deliver and what to replicate, and keeps replication accounting.
//! Persistence sits behind [`Store`]; [`MemStore`] serves tests and the simulator.

mod mem;
mod session;

pub use mem::MemStore;
pub use session::{Event, LinkSession, maintain, split_budget};

use vl_proto::{Envelope, MAX_COPY_BUDGET, RelayClass, VaccineItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// Only the sender's own copy, handed straight to the recipient.
    Direct,
    /// Every willing neighbour gets a full copy (simulation baseline only).
    Epidemic,
    /// Binary spray-and-wait with `initial` copies (ADR-003).
    SprayAndWait { initial: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub algorithm: Algorithm,
    pub relay_class: RelayClass,
    pub relay_willing: bool,
    pub default_lifetime_min: u32,
    pub max_lifetime_min: u32,
    pub skew_min: u32,
    pub carry_cap_bytes: usize,
    pub per_tag_cap: usize,
    pub session_accept_envelopes: usize,
    pub session_accept_bytes: usize,
    /// Sender re-sprays once after this share of the lifetime passes without delivery.
    pub retry_after_percent: u8,
    /// A mid-session refresh offers only summary entries and vaccines not yet
    /// sent in this session (research/algorithms/03, experiment E4).
    pub delta_refresh: bool,
}

impl Policy {
    /// Doc 18 §7.2 values; initial copy budget 8 per the simulation study
    /// (ADR-021, `research/22-routing-simulation-results.md`). Battery cost of
    /// the extra transfers is unmeasured until Experiment C.
    pub const fn default_spray() -> Self {
        Self {
            algorithm: Algorithm::SprayAndWait { initial: 8 },
            relay_class: RelayClass::BestEffort,
            relay_willing: true,
            default_lifetime_min: 72 * 60,
            max_lifetime_min: 7 * 24 * 60,
            skew_min: 120,
            carry_cap_bytes: 10 * 1024 * 1024,
            per_tag_cap: 8,
            session_accept_envelopes: 32,
            session_accept_bytes: 64 * 1024,
            retry_after_percent: 25,
            delta_refresh: true,
        }
    }

    pub fn initial_budget(&self) -> u8 {
        match self.algorithm {
            Algorithm::Direct => 1,
            Algorithm::Epidemic => MAX_COPY_BUDGET,
            Algorithm::SprayAndWait { initial } => initial.clamp(1, MAX_COPY_BUDGET),
        }
    }
}

/// An envelope held locally, either our own outgoing copy or one carried for others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub envelope: Envelope,
    pub own: bool,
    pub stored_at_min: u32,
    pub retried: bool,
}

/// Storage failure. The engine fails closed: the session ends and nothing is half-applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "store: {}", self.0)
    }
}

impl std::error::Error for StoreError {}

pub type StoreResult<T> = Result<T, StoreError>;

/// Persistence required by the routing engine. Each call must be atomic; the
/// engine never relies on multi-call transactions.
pub trait Store {
    fn get(&self, id: &[u8; 16]) -> StoreResult<Option<Stored>>;
    fn all(&self) -> StoreResult<Vec<Stored>>;
    fn insert(&mut self, stored: Stored) -> StoreResult<()>;
    fn set_budget(&mut self, id: &[u8; 16], budget: u8) -> StoreResult<()>;
    fn set_retried(&mut self, id: &[u8; 16]) -> StoreResult<()>;
    fn remove(&mut self, id: &[u8; 16]) -> StoreResult<()>;
    fn carried_bytes(&self) -> StoreResult<usize>;
    fn carried_for_tag(&self, tag: &[u8; 16]) -> StoreResult<usize>;
    fn seen(&self, id: &[u8; 16]) -> StoreResult<bool>;
    fn mark_seen(&mut self, id: &[u8; 16], expiry_min: u32) -> StoreResult<()>;
    /// Is `tag` one of this device's current inbound recipient tags?
    fn is_my_tag(&self, tag: &[u8; 16]) -> StoreResult<bool>;
    fn vaccines(&self, now_min: u32, limit: usize) -> StoreResult<Vec<VaccineItem>>;
    fn has_vaccine(&self, commitment: &[u8; 16]) -> StoreResult<bool>;
    /// Held vaccine-enabled envelopes with this commitment: `(envelope_id, own)`.
    /// Must be an indexed lookup in persistent stores (it runs once per received vaccine).
    fn vaccine_holders(&self, commitment: &[u8; 16]) -> StoreResult<Vec<([u8; 16], bool)>>;
    /// Stores the vaccine and removes every vaccine-enabled envelope with that commitment.
    fn add_vaccine(&mut self, item: VaccineItem, commitment: [u8; 16]) -> StoreResult<()>;
    /// Removes every record whose expiry is at or before `now_min`; returns removed envelopes.
    fn purge_expired(&mut self, now_min: u32) -> StoreResult<Vec<Stored>>;
}
