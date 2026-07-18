//! PreKey bundle store and one-time prekey management.

use lm_core::{LmError, PreKeyBundle, Result, SignedOneTimePreKeyRecord, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::kademlia::current_unix_timestamp;

#[derive(Debug, Clone, Default)]
pub struct PreKeyStore {
    bundles: HashMap<UserId, PreKeyBundle>,
    signed_one_time_prekey_records: HashMap<UserId, Vec<SignedOneTimePreKeyRecord>>,
    pub(crate) consumed_one_time_prekeys: HashMap<UserId, Vec<u32>>,
}

impl PreKeyStore {
    pub fn publish_verified(&mut self, bundle: PreKeyBundle) -> Result<()> {
        self.publish_verified_with_signed_one_time_prekey_records(bundle, Vec::new())
    }

    pub fn publish_verified_with_signed_one_time_prekey_records(
        &mut self,
        bundle: PreKeyBundle,
        signed_one_time_prekey_records: Vec<SignedOneTimePreKeyRecord>,
    ) -> Result<()> {
        if signed_one_time_prekey_records.len() > lm_core::limits::MAX_ONE_TIME_PREKEYS {
            return Err(LmError::PayloadTooLarge);
        }
        bundle.verify()?;
        for record in &signed_one_time_prekey_records {
            record.verify_for_bundle(&bundle)?;
        }
        let user_id = bundle.user_id.clone();
        let reset_consumed = self
            .bundles
            .get(&user_id)
            .map(|existing| existing.signed_prekey_id != bundle.signed_prekey_id)
            .unwrap_or(true);
        self.bundles.insert(user_id.clone(), bundle);
        if reset_consumed {
            self.consumed_one_time_prekeys.remove(&user_id);
            self.signed_one_time_prekey_records.remove(&user_id);
        }
        self.merge_verified_signed_one_time_prekey_records_for(
            &user_id,
            signed_one_time_prekey_records,
        );
        self.prune_signed_one_time_prekey_records_for(&user_id);
        self.prune_consumed_for(&user_id);
        Ok(())
    }

    pub fn get_for(&mut self, user_id: &UserId) -> Option<PreKeyBundle> {
        self.prune_expired(current_unix_timestamp());
        self.bundles.get(user_id).cloned()
    }

    pub fn get_for_unchecked(&self, user_id: &UserId) -> Option<PreKeyBundle> {
        self.bundles.get(user_id).cloned()
    }

    pub fn take_for(&mut self, user_id: &UserId, consume: bool) -> Option<PreKeyBundle> {
        let (bundle, _, _) =
            self.take_for_with_selected_one_time_prekey_material(user_id, consume)?;
        Some(bundle)
    }

    pub fn take_for_with_selected_one_time_prekey(
        &mut self,
        user_id: &UserId,
        consume: bool,
    ) -> Option<(PreKeyBundle, Option<u32>)> {
        let (bundle, selected_id, _) =
            self.take_for_with_selected_one_time_prekey_material(user_id, consume)?;
        Some((bundle, selected_id))
    }

    pub fn take_for_with_selected_one_time_prekey_record(
        &mut self,
        user_id: &UserId,
        consume: bool,
    ) -> Option<(PreKeyBundle, Option<SignedOneTimePreKeyRecord>)> {
        let (bundle, _, selected_record) =
            self.take_for_with_selected_one_time_prekey_material(user_id, consume)?;
        Some((bundle, selected_record))
    }

    pub fn take_for_with_selected_one_time_prekey_material(
        &mut self,
        user_id: &UserId,
        consume: bool,
    ) -> Option<(PreKeyBundle, Option<u32>, Option<SignedOneTimePreKeyRecord>)> {
        self.prune_expired(current_unix_timestamp());
        let bundle = self.bundles.get(user_id).cloned()?;
        let consumed = self
            .consumed_one_time_prekeys
            .entry(user_id.clone())
            .or_default();
        let selected_record =
            self.signed_one_time_prekey_records
                .get(user_id)
                .and_then(|records| {
                    records
                        .iter()
                        .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                        .find(|record| !consumed.contains(&record.key_id))
                        .cloned()
                });
        let selected_id = selected_record
            .as_ref()
            .map(|record| record.key_id)
            .or_else(|| {
                if self
                    .signed_one_time_prekey_records
                    .get(user_id)
                    .map(|records| !records.is_empty())
                    .unwrap_or(false)
                {
                    None
                } else {
                    bundle
                        .one_time_prekeys
                        .iter()
                        .map(|key| key.key_id)
                        .find(|id| !consumed.contains(id))
                }
            });
        if consume && let Some(id) = selected_id {
            consumed.push(id);
            consumed.sort_unstable();
            consumed.dedup();
        }
        Some((bundle, selected_id, selected_record))
    }

    pub fn consumed_for(&self, user_id: &UserId) -> Vec<u32> {
        self.consumed_one_time_prekeys
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn signed_one_time_prekey_records_for(
        &self,
        user_id: &UserId,
    ) -> Vec<SignedOneTimePreKeyRecord> {
        self.signed_one_time_prekey_records
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn remaining_one_time_prekeys_for(&self, user_id: &UserId) -> Option<usize> {
        let bundle = self.bundles.get(user_id)?;
        let consumed = self.consumed_one_time_prekeys.get(user_id);
        if let Some(records) = self.signed_one_time_prekey_records.get(user_id) {
            let count = records
                .iter()
                .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                .filter(|record| {
                    !consumed
                        .map(|ids| ids.contains(&record.key_id))
                        .unwrap_or(false)
                })
                .count();
            return Some(count);
        }
        Some(
            bundle
                .one_time_prekeys
                .iter()
                .filter(|key| {
                    !consumed
                        .map(|ids| ids.contains(&key.key_id))
                        .unwrap_or(false)
                })
                .count(),
        )
    }

    pub fn published_one_time_prekeys_for(&self, user_id: &UserId) -> Option<usize> {
        let bundle = self.bundles.get(user_id)?;
        if let Some(records) = self.signed_one_time_prekey_records.get(user_id) {
            return Some(
                records
                    .iter()
                    .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                    .count(),
            );
        }
        Some(bundle.one_time_prekeys.len())
    }

    pub fn prune_expired(&mut self, now: u64) -> usize {
        let expired: Vec<_> = self
            .bundles
            .iter()
            .filter(|(_, bundle)| {
                bundle.expires_at <= now || bundle.signed_prekey_expires_at <= now
            })
            .map(|(user_id, _)| user_id.clone())
            .collect();
        let removed = expired.len();
        for user_id in expired {
            self.bundles.remove(&user_id);
            self.signed_one_time_prekey_records.remove(&user_id);
            self.consumed_one_time_prekeys.remove(&user_id);
        }
        let users: Vec<_> = self
            .signed_one_time_prekey_records
            .keys()
            .cloned()
            .collect();
        for user_id in users {
            if let Some(records) = self.signed_one_time_prekey_records.get_mut(&user_id) {
                records.retain(|record| record.expires_at > now);
                if records.is_empty() {
                    self.signed_one_time_prekey_records.remove(&user_id);
                }
            }
            self.prune_consumed_for(&user_id);
        }
        removed
    }

    fn merge_verified_signed_one_time_prekey_records_for(
        &mut self,
        user_id: &UserId,
        records: Vec<SignedOneTimePreKeyRecord>,
    ) -> usize {
        if records.is_empty() {
            return 0;
        }
        let list = self
            .signed_one_time_prekey_records
            .entry(user_id.clone())
            .or_default();
        let mut inserted = 0usize;
        for record in records {
            if let Some(existing) = list.iter_mut().find(|existing| {
                existing.signed_prekey_id == record.signed_prekey_id
                    && existing.key_id == record.key_id
            }) {
                *existing = record;
            } else {
                list.push(record);
                inserted = inserted.saturating_add(1);
            }
        }
        list.sort_by_key(|record| (record.signed_prekey_id, record.key_id));
        inserted
    }

    fn prune_signed_one_time_prekey_records_for(&mut self, user_id: &UserId) {
        let Some(bundle) = self.bundles.get(user_id) else {
            self.signed_one_time_prekey_records.remove(user_id);
            return;
        };
        let Some(records) = self.signed_one_time_prekey_records.get_mut(user_id) else {
            return;
        };
        records.retain(|record| record.verify_for_bundle(bundle).is_ok());
        records.sort_by_key(|record| (record.signed_prekey_id, record.key_id));
        records.dedup_by_key(|record| (record.signed_prekey_id, record.key_id));
        if records.is_empty() {
            self.signed_one_time_prekey_records.remove(user_id);
        }
    }

    fn valid_one_time_key_ids_for(&self, user_id: &UserId) -> Option<Vec<u32>> {
        let bundle = self.bundles.get(user_id)?;
        if let Some(records) = self.signed_one_time_prekey_records.get(user_id) {
            return Some(
                records
                    .iter()
                    .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                    .map(|record| record.key_id)
                    .collect(),
            );
        }
        Some(
            bundle
                .one_time_prekeys
                .iter()
                .map(|key| key.key_id)
                .collect(),
        )
    }

    fn prune_consumed_for(&mut self, user_id: &UserId) {
        let Some(valid_ids) = self.valid_one_time_key_ids_for(user_id) else {
            self.consumed_one_time_prekeys.remove(user_id);
            return;
        };
        if let Some(consumed) = self.consumed_one_time_prekeys.get_mut(user_id) {
            consumed.retain(|id| valid_ids.contains(id));
            consumed.sort_unstable();
            consumed.dedup();
            if consumed.is_empty() {
                self.consumed_one_time_prekeys.remove(user_id);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.bundles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bundles.is_empty()
    }

    pub fn all_bundles(&self) -> Vec<PreKeyBundle> {
        self.bundles.values().cloned().collect()
    }

    pub fn all_signed_one_time_prekey_records(&self) -> Vec<SignedOneTimePreKeyRecord> {
        self.signed_one_time_prekey_records
            .values()
            .flat_map(|records| records.iter().cloned())
            .collect()
    }

    pub(crate) fn restore_bundles(&mut self, bundles: Vec<PreKeyBundle>) {
        self.bundles.clear();
        self.signed_one_time_prekey_records.clear();
        for bundle in bundles {
            if bundle.verify().is_ok() {
                self.bundles.insert(bundle.user_id.clone(), bundle);
            }
        }
        self.prune_expired(current_unix_timestamp());
    }

    pub(crate) fn restore_signed_one_time_prekey_records(
        &mut self,
        records: Vec<SignedOneTimePreKeyRecord>,
    ) {
        self.signed_one_time_prekey_records.clear();
        self.merge_signed_one_time_prekey_records(records);
    }

    pub(crate) fn merge_bundles(&mut self, bundles: Vec<PreKeyBundle>) -> usize {
        let mut inserted = 0;
        for bundle in bundles {
            if bundle.verify().is_err() {
                continue;
            }
            let user_id = bundle.user_id.clone();
            let reset_consumed = self
                .bundles
                .get(&user_id)
                .map(|existing| existing.signed_prekey_id != bundle.signed_prekey_id)
                .unwrap_or(true);
            let is_new = !self.bundles.contains_key(&user_id);
            self.bundles.insert(user_id.clone(), bundle);
            if reset_consumed {
                self.consumed_one_time_prekeys.remove(&user_id);
                self.signed_one_time_prekey_records.remove(&user_id);
            }
            self.prune_signed_one_time_prekey_records_for(&user_id);
            self.prune_consumed_for(&user_id);
            if is_new {
                inserted += 1;
            }
        }
        inserted
    }

    pub(crate) fn merge_signed_one_time_prekey_records(
        &mut self,
        records: Vec<SignedOneTimePreKeyRecord>,
    ) -> usize {
        let mut grouped: HashMap<UserId, Vec<SignedOneTimePreKeyRecord>> = HashMap::new();
        for record in records {
            let Some(bundle) = self.bundles.get(&record.user_id) else {
                continue;
            };
            if record.verify_for_bundle(bundle).is_err() {
                continue;
            }
            grouped
                .entry(record.user_id.clone())
                .or_default()
                .push(record);
        }
        let mut inserted = 0usize;
        for (user_id, records) in grouped {
            inserted = inserted.saturating_add(
                self.merge_verified_signed_one_time_prekey_records_for(&user_id, records),
            );
            self.prune_consumed_for(&user_id);
        }
        inserted
    }

    pub(crate) fn restore_consumed(&mut self, consumed: Vec<ConsumedOneTimePreKey>) {
        self.consumed_one_time_prekeys.clear();
        self.merge_consumed(consumed);
    }

    pub(crate) fn merge_consumed(&mut self, consumed: Vec<ConsumedOneTimePreKey>) {
        for item in consumed {
            let user_id = item.user_id;
            let ids = self
                .consumed_one_time_prekeys
                .entry(user_id.clone())
                .or_default();
            ids.push(item.key_id);
            ids.sort_unstable();
            ids.dedup();
            self.prune_consumed_for(&user_id);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumedOneTimePreKey {
    pub user_id: UserId,
    pub key_id: u32,
}

pub(crate) fn prekey_low_one_time_prekeys(remaining: Option<usize>) -> bool {
    remaining.map(|value| value <= 1).unwrap_or(false)
}

pub(crate) fn prekey_replenishment_required(remaining: Option<usize>) -> bool {
    remaining.map(|value| value <= 1).unwrap_or(true)
}
