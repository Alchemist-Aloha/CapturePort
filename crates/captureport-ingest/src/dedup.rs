//! Progressive duplicate classification.

use crate::Fingerprint;
use captureport_core::ImportStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateInput {
    pub source_identity: String,
    pub source_path: String,
    pub size: u64,
    pub capture_time: Option<String>,
    pub quick: Option<Fingerprint>,
    pub full: Option<Fingerprint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRecord {
    pub source_identity: String,
    pub source_path: String,
    pub size: u64,
    pub capture_time: Option<String>,
    pub quick: Option<Fingerprint>,
    pub full: Option<Fingerprint>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DuplicateEvidence {
    None,
    StableIdentity,
    QuickFingerprint,
    FullFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateClassification {
    pub status: ImportStatus,
    pub evidence: DuplicateEvidence,
}

/// Classify against import history, from cheap evidence to strong evidence.
/// Only an exact full fingerprint is strong enough to report `Imported`.
pub fn classify_duplicate(
    candidate: &DuplicateInput,
    history: &[HistoryRecord],
) -> DuplicateClassification {
    let mut possible = false;
    for old in history {
        if let (Some(new), Some(previous)) = (&candidate.full, &old.full)
            && new == previous
        {
            return DuplicateClassification {
                status: ImportStatus::Imported,
                evidence: DuplicateEvidence::FullFingerprint,
            };
        }
        if let (Some(new), Some(previous)) = (&candidate.quick, &old.quick)
            && new == previous
        {
            possible = true;
        }
        if candidate.source_identity == old.source_identity
            && candidate.source_path == old.source_path
            && candidate.size == old.size
            && candidate.capture_time == old.capture_time
        {
            possible = true;
        }
    }
    if possible {
        DuplicateClassification {
            status: ImportStatus::PossibleDuplicate,
            evidence: if candidate.quick.is_some() {
                DuplicateEvidence::QuickFingerprint
            } else {
                DuplicateEvidence::StableIdentity
            },
        }
    } else {
        DuplicateClassification {
            status: ImportStatus::New,
            evidence: DuplicateEvidence::None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DuplicateIndex {
    records: Vec<HistoryRecord>,
}

impl DuplicateIndex {
    pub fn new(records: impl IntoIterator<Item = HistoryRecord>) -> Self {
        Self {
            records: records.into_iter().collect(),
        }
    }

    pub fn classify(&self, candidate: &DuplicateInput) -> DuplicateClassification {
        classify_duplicate(candidate, &self.records)
    }

    pub fn records(&self) -> &[HistoryRecord] {
        &self.records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fp(algorithm: &'static str, hex: &str) -> Fingerprint {
        Fingerprint {
            algorithm,
            hex: hex.into(),
        }
    }
    fn input(identity: &str, path: &str, full: Option<Fingerprint>) -> DuplicateInput {
        DuplicateInput {
            source_identity: identity.into(),
            source_path: path.into(),
            size: 42,
            capture_time: Some("2026-01-01T00:00:00Z".into()),
            quick: Some(fp("quick-blake3-v1", "same")),
            full,
        }
    }
    #[test]
    fn full_match_is_imported() {
        let candidate = input("card-a", "DCIM/DSC0001.ARW", Some(fp("blake3-v1", "exact")));
        let old = HistoryRecord {
            source_identity: "card-a".into(),
            source_path: candidate.source_path.clone(),
            size: 42,
            capture_time: candidate.capture_time.clone(),
            quick: candidate.quick.clone(),
            full: Some(fp("blake3-v1", "exact")),
        };
        assert_eq!(
            classify_duplicate(&candidate, &[old]).status,
            ImportStatus::Imported
        );
    }
    #[test]
    fn formatted_card_counter_reset_is_not_imported() {
        let candidate = input("card-new-session", "DCIM/DSC0001.ARW", None);
        let old = HistoryRecord {
            source_identity: "card-old-session".into(),
            source_path: candidate.source_path.clone(),
            size: 42,
            capture_time: candidate.capture_time.clone(),
            quick: candidate.quick.clone(),
            full: None,
        };
        let result = classify_duplicate(&candidate, &[old]);
        assert_eq!(result.status, ImportStatus::PossibleDuplicate);
        assert_ne!(result.status, ImportStatus::Imported);
    }
}
