use crate::{Store, StoreResult, Stored};
use std::collections::{HashMap, HashSet};
use vl_proto::VaccineItem;

/// In-memory [`Store`] for tests and the simulator. Never fails.
#[derive(Debug, Default, Clone)]
pub struct MemStore {
    envelopes: HashMap<[u8; 16], Stored>,
    seen: HashMap<[u8; 16], u32>,
    vaccines: HashMap<[u8; 16], VaccineItem>,
    my_tags: HashSet<[u8; 16]>,
}

impl MemStore {
    pub fn with_tags(tags: impl IntoIterator<Item = [u8; 16]>) -> Self {
        Self {
            my_tags: tags.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn add_tag(&mut self, tag: [u8; 16]) {
        self.my_tags.insert(tag);
    }
}

impl Store for MemStore {
    fn get(&self, id: &[u8; 16]) -> StoreResult<Option<Stored>> {
        Ok(self.envelopes.get(id).cloned())
    }

    fn all(&self) -> StoreResult<Vec<Stored>> {
        Ok(self.envelopes.values().cloned().collect())
    }

    fn insert(&mut self, stored: Stored) -> StoreResult<()> {
        self.envelopes
            .insert(stored.envelope.header.envelope_id, stored);
        Ok(())
    }

    fn set_budget(&mut self, id: &[u8; 16], budget: u8) -> StoreResult<()> {
        if let Some(s) = self.envelopes.get_mut(id) {
            s.envelope.header.copy_budget = budget;
        }
        Ok(())
    }

    fn set_retried(&mut self, id: &[u8; 16]) -> StoreResult<()> {
        if let Some(s) = self.envelopes.get_mut(id) {
            s.retried = true;
        }
        Ok(())
    }

    fn remove(&mut self, id: &[u8; 16]) -> StoreResult<()> {
        self.envelopes.remove(id);
        Ok(())
    }

    fn carried_bytes(&self) -> StoreResult<usize> {
        Ok(self
            .envelopes
            .values()
            .filter(|s| !s.own)
            .map(|s| s.envelope.body.len() + vl_proto::HEADER_LEN)
            .sum())
    }

    fn carried_for_tag(&self, tag: &[u8; 16]) -> StoreResult<usize> {
        Ok(self
            .envelopes
            .values()
            .filter(|s| !s.own && &s.envelope.header.recipient_tag == tag)
            .count())
    }

    fn seen(&self, id: &[u8; 16]) -> StoreResult<bool> {
        Ok(self.seen.contains_key(id))
    }

    fn mark_seen(&mut self, id: &[u8; 16], expiry_min: u32) -> StoreResult<()> {
        self.seen.insert(*id, expiry_min);
        Ok(())
    }

    fn is_my_tag(&self, tag: &[u8; 16]) -> StoreResult<bool> {
        Ok(self.my_tags.contains(tag))
    }

    fn vaccines(&self, now_min: u32, limit: usize) -> StoreResult<Vec<VaccineItem>> {
        Ok(self
            .vaccines
            .values()
            .filter(|v| v.expiry_min > now_min)
            .take(limit)
            .copied()
            .collect())
    }

    fn has_vaccine(&self, commitment: &[u8; 16]) -> StoreResult<bool> {
        Ok(self.vaccines.contains_key(commitment))
    }

    fn vaccine_holders(&self, commitment: &[u8; 16]) -> StoreResult<Vec<([u8; 16], bool)>> {
        Ok(self
            .envelopes
            .values()
            .filter(|s| {
                s.envelope.header.vaccine_enabled()
                    && &s.envelope.header.ack_commitment == commitment
            })
            .map(|s| (s.envelope.header.envelope_id, s.own))
            .collect())
    }

    fn add_vaccine(&mut self, item: VaccineItem, commitment: [u8; 16]) -> StoreResult<()> {
        self.vaccines.insert(commitment, item);
        self.envelopes.retain(|_, s| {
            s.envelope.header.ack_commitment != commitment || !s.envelope.header.vaccine_enabled()
        });
        Ok(())
    }

    fn purge_expired(&mut self, now_min: u32) -> StoreResult<Vec<Stored>> {
        let expired: Vec<_> = self
            .envelopes
            .values()
            .filter(|s| s.envelope.header.expiry_min <= now_min)
            .cloned()
            .collect();
        for s in &expired {
            self.envelopes.remove(&s.envelope.header.envelope_id);
        }
        self.seen.retain(|_, e| *e > now_min);
        self.vaccines.retain(|_, v| v.expiry_min > now_min);
        Ok(expired)
    }
}
