//! Deterministic import planning and safe execution, independent of GPUI.

mod bundle;
mod dedup;
mod fingerprint;
mod importer;
mod planner;
mod preset;
mod reconcile;
mod recovery;
mod source_delete;
mod template;

pub use bundle::{
    BundleResult, BundleType, MediaBundle, apply_bundle_policy, bundle_media, group_media,
};
pub use dedup::{
    DuplicateClassification, DuplicateEvidence, DuplicateIndex, DuplicateInput, HistoryRecord,
    classify_duplicate,
};
pub use fingerprint::{
    CopyDigest, FULL_ALGORITHM, Fingerprint, QUICK_ALGORITHM, digest_reader, quick_path,
    quick_source, verify_copy,
};
pub use importer::{
    CopyResult, ImportEngine, ImportError, ImportItemState, ImportProgress, ImportRecorder,
    ImportResult, ItemResult, NoopRecorder,
};
pub use planner::{
    ImportPlan, ImportPlanner, PlanInput, PlanStatus, PlannedCopy, PlannedImport, PlannerError,
    effective_time, session_numbers,
};
pub use preset::{
    BackupRule, BundlePolicy, CollisionPolicy, DestinationRule, Grouping, ImportPreset, MediaRules,
    TimeCorrection, VerificationMode,
};
pub use reconcile::{
    FakeFailureScenario, LibraryRecord, ReconcileOptions, ReconcileReport, reconcile_library,
    scan_existing_library,
};
pub use recovery::{PartialFile, RecoveryReport, cleanup_partial, inspect_partials};
pub use source_delete::{
    DeletionFailure, DeletionOptions, DeletionRefusal, DeletionReport, SourceRemover,
    delete_verified_sources, deletion_refusal,
};
pub use template::{TEMPLATE_TOKENS, Template, TemplateContext, TemplateError, TemplateTokenHelp};
