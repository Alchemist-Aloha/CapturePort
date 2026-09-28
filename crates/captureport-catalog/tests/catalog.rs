use captureport_catalog::*;
use rusqlite::Connection;
use tempfile::tempdir;

fn identity(stable: &str) -> SourceIdentity {
    SourceIdentity {
        stable_id: Some(stable.into()),
        source_type: "camera".into(),
        manufacturer: Some("Sony".into()),
        model: Some("A7C II".into()),
        serial: Some(stable.into()),
        volume_uuid: None,
        alias: None,
    }
}

#[test]
fn reconnect_matches_strong_media_identity_after_reopen() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("catalog.sqlite");
    let catalog = CatalogHandle::open(db.clone()).unwrap();
    let source = catalog
        .upsert_source(identity("camera-1"), "2026-09-27T10:00:00Z")
        .unwrap();
    let media = MediaIdentity {
        source_id: source.id,
        source_path: "DCIM/100/DSC00001.JPG".into(),
        source_filename: "DSC00001.JPG".into(),
        source_size: 42,
        capture_time: Some("2026-09-27T09:00:00Z".into()),
    };
    catalog
        .upsert_media(media.clone(), MediaType::Photo, "2026-09-27T10:00:00Z")
        .unwrap();
    drop(catalog);
    let catalog = CatalogHandle::open(db).unwrap();
    let source = catalog
        .upsert_source(identity("camera-1"), "2026-09-28T10:00:00Z")
        .unwrap();
    let mut media = media;
    media.source_id = source.id;
    assert!(catalog.lookup_media(media).unwrap().is_some());
}

#[test]
fn source_alias_is_explicit_and_survives_reconnect() {
    let catalog = CatalogHandle::open(CatalogPath::Memory).unwrap();
    let source = catalog
        .upsert_source(identity("alias-camera"), "now")
        .unwrap();
    let named = catalog
        .set_source_alias(source.id, Some("Travel camera".into()))
        .unwrap();
    assert_eq!(named.identity.alias.as_deref(), Some("Travel camera"));

    // A later device observation has no alias and must not erase the user's
    // label. An explicit clear remains available through set_source_alias.
    let observed = catalog
        .upsert_source(identity("alias-camera"), "later")
        .unwrap();
    assert_eq!(observed.identity.alias.as_deref(), Some("Travel camera"));
    assert_eq!(
        catalog
            .source_by_stable_id("alias-camera")
            .unwrap()
            .unwrap()
            .identity
            .alias
            .as_deref(),
        Some("Travel camera")
    );
    assert_eq!(
        catalog
            .set_source_alias(source.id, None)
            .unwrap()
            .identity
            .alias,
        None
    );
}

#[test]
fn source_by_id_is_read_only_and_returns_none_for_unknown_ids() {
    let catalog = CatalogHandle::open(CatalogPath::Memory).unwrap();
    let source = catalog
        .upsert_source(identity("source-by-id"), "now")
        .unwrap();
    assert_eq!(catalog.source_by_id(source.id).unwrap(), Some(source));
    assert_eq!(catalog.source_by_id(-1).unwrap(), None);
}

#[test]
fn repeated_names_are_distinguished_by_source_and_path() {
    let catalog = CatalogHandle::open(CatalogPath::Memory).unwrap();
    let a = catalog.upsert_source(identity("a"), "now").unwrap();
    let b = catalog.upsert_source(identity("b"), "now").unwrap();
    let make = |source_id, path: &str| MediaIdentity {
        source_id,
        source_path: path.into(),
        source_filename: "DSC00001.JPG".into(),
        source_size: 8,
        capture_time: None,
    };
    catalog
        .upsert_media(make(a.id, "DCIM/1/DSC00001.JPG"), MediaType::Photo, "now")
        .unwrap();
    assert!(catalog
        .lookup_media(make(b.id, "DCIM/1/DSC00001.JPG"))
        .unwrap()
        .is_none());
}

#[test]
fn partial_session_is_recoverable_and_history_contains_imports() {
    let catalog = CatalogHandle::open(CatalogPath::Memory).unwrap();
    let source = catalog.upsert_source(identity("camera"), "now").unwrap();
    let media = catalog
        .upsert_media(
            MediaIdentity {
                source_id: source.id,
                source_path: "a.jpg".into(),
                source_filename: "a.jpg".into(),
                source_size: 10,
                capture_time: None,
            },
            MediaType::Photo,
            "now",
        )
        .unwrap();
    let session = catalog.begin_session(source.id, None, "now").unwrap();
    catalog
        .record_import(
            session.id,
            ImportInput {
                media_id: media.id,
                destination_path: "Pictures/a.jpg".into(),
                destination_size: Some(10),
                copy_completed_at: None,
                verification_method: None,
                verified_at: None,
                status: ImportStatus::Partial,
                error: Some("interrupted".into()),
            },
        )
        .unwrap();
    let incomplete = catalog.incomplete_sessions().unwrap();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].imports.len(), 1);
    catalog
        .complete_session(
            session.id,
            Some("later".into()),
            SessionStatus::Partial,
            Some("stopped".into()),
        )
        .unwrap();
    assert_eq!(catalog.list_sessions(10).unwrap().len(), 1);
}

fn imported_fixture(
    catalog: &CatalogHandle,
    status: ImportStatus,
) -> (SourceRecord, MediaIdentity, i64) {
    let source = catalog
        .upsert_source(identity("import-camera"), "now")
        .unwrap();
    let media = MediaIdentity {
        source_id: source.id,
        source_path: "DCIM/100/DSC00001.JPG".into(),
        source_filename: "DSC00001.JPG".into(),
        source_size: 123,
        capture_time: Some("2026-09-27T12:34:56Z".into()),
    };
    let record = catalog
        .upsert_media(media.clone(), MediaType::Photo, "now")
        .unwrap();
    let session = catalog.begin_session(source.id, None, "now").unwrap();
    catalog
        .record_import(
            session.id,
            ImportInput {
                media_id: record.id,
                destination_path: "Pictures/DSC00001.JPG".into(),
                destination_size: Some(123),
                copy_completed_at: Some("later".into()),
                verification_method: Some("size".into()),
                verified_at: None,
                status,
                error: None,
            },
        )
        .unwrap();
    (source, media, session.id)
}

#[test]
fn imported_lookup_requires_an_exact_completed_or_verified_row() {
    let catalog = CatalogHandle::open(CatalogPath::Memory).unwrap();
    let source = catalog
        .upsert_source(identity("unimported"), "now")
        .unwrap();
    let media = MediaIdentity {
        source_id: source.id,
        source_path: "DCIM/100/DSC00001.JPG".into(),
        source_filename: "DSC00001.JPG".into(),
        source_size: 123,
        capture_time: Some("2026-09-27T12:34:56Z".into()),
    };
    catalog
        .upsert_media(media.clone(), MediaType::Photo, "now")
        .unwrap();
    assert!(!catalog.lookup_imported(media.clone()).unwrap());

    let (_, failed_media, _) = imported_fixture(&catalog, ImportStatus::Failed);
    assert!(!catalog.lookup_imported(failed_media.clone()).unwrap());
    let (_, partial_media, _) = imported_fixture(&catalog, ImportStatus::Partial);
    assert!(!catalog.lookup_imported(partial_media.clone()).unwrap());

    let (_, verified_media, _) = imported_fixture(&catalog, ImportStatus::Verified);
    assert!(catalog.lookup_imported(verified_media.clone()).unwrap());
    // A partial or failed import must not be named as the prior import either.
    assert!(catalog
        .lookup_imported_session_by_identity(verified_media.clone())
        .unwrap()
        .is_some());
    let mut different_size = verified_media.clone();
    different_size.source_size += 1;
    assert!(!catalog.lookup_imported(different_size).unwrap());
    let mut different_time = verified_media;
    different_time.capture_time = Some("2026-09-27T12:34:57Z".into());
    assert!(!catalog.lookup_imported(different_time).unwrap());
}

#[test]
fn imported_lookup_by_quick_and_full_fingerprint_requires_successful_import() {
    let catalog = CatalogHandle::open(CatalogPath::Memory).unwrap();
    let source = catalog
        .upsert_source(identity("fingerprint-camera"), "now")
        .unwrap();
    let media = MediaIdentity {
        source_id: source.id,
        source_path: "DCIM/100/A.JPG".into(),
        source_filename: "A.JPG".into(),
        source_size: 256,
        capture_time: None,
    };
    let record = catalog
        .execute(CatalogCommand::UpsertMedia {
            media: media.clone(),
            media_type: MediaType::Photo,
            observed_at: "now".into(),
            quick_fingerprint: Some("quick-blake3-v1:abc".into()),
            content_hash: Some("blake3:def".into()),
        })
        .unwrap();
    let media_id = match record {
        CatalogResponse::Media(media) => media.id,
        _ => unreachable!(),
    };
    assert!(!catalog
        .lookup_imported_fingerprint(256, "quick-blake3-v1:abc", None)
        .unwrap());
    let session = catalog.begin_session(source.id, None, "now").unwrap();
    catalog
        .record_import(
            session.id,
            ImportInput {
                media_id,
                destination_path: "Pictures/A.JPG".into(),
                destination_size: Some(256),
                copy_completed_at: Some("later".into()),
                verification_method: Some("standard".into()),
                verified_at: Some("later".into()),
                status: ImportStatus::Verified,
                error: None,
            },
        )
        .unwrap();
    assert!(catalog
        .lookup_imported_fingerprint(256, "quick-blake3-v1:abc", None)
        .unwrap());
    assert!(catalog
        .lookup_imported_fingerprint(256, "quick-blake3-v1:abc", Some("blake3:def".into()))
        .unwrap());
    assert!(!catalog
        .lookup_imported_fingerprint(256, "quick-blake3-v1:abc", Some("blake3:other".into()))
        .unwrap());
    assert!(!catalog
        .lookup_imported_fingerprint(255, "quick-blake3-v1:abc", None)
        .unwrap());

    // The session variants name the prior import instead of answering yes/no,
    // because a "possible duplicate" the user cannot trace is an unsafe label.
    let found = catalog
        .lookup_imported_session(256, "quick-blake3-v1:abc", None)
        .unwrap()
        .expect("verified import should be named");
    assert_eq!(found.session_id, session.id);
    assert_eq!(found.destination_path, "Pictures/A.JPG");
    assert!(!found.started_at.is_empty());
    assert!(catalog
        .lookup_imported_session(255, "quick-blake3-v1:abc", None)
        .unwrap()
        .is_none());
    assert!(catalog
        .lookup_imported_session(256, "quick-blake3-v1:abc", Some("blake3:other".into()))
        .unwrap()
        .is_none());
    let by_identity = catalog
        .lookup_imported_session_by_identity(media.clone())
        .unwrap()
        .expect("identity match should name the prior import");
    assert_eq!(by_identity.session_id, session.id);
    assert_eq!(by_identity.destination_path, "Pictures/A.JPG");
}

#[test]
fn fingerprint_update_preserves_the_existing_media_identity() {
    let catalog = CatalogHandle::open(CatalogPath::Memory).unwrap();
    let source = catalog
        .upsert_source(identity("fingerprint-update"), "now")
        .unwrap();
    let original = MediaIdentity {
        source_id: source.id,
        source_path: "DCIM/A.JPG".into(),
        source_filename: "A.JPG".into(),
        source_size: 99,
        capture_time: Some("2026-09-27T12:00:00Z".into()),
    };
    let media = catalog
        .upsert_media(original.clone(), MediaType::Photo, "now")
        .unwrap();
    let updated = catalog
        .update_media_fingerprints(
            media.id,
            Some("quick-blake3-v1:updated".into()),
            Some("blake3:updated".into()),
        )
        .unwrap();
    assert_eq!(updated.id, media.id);
    assert_eq!(updated.identity, original);
    assert_eq!(
        updated.quick_fingerprint.as_deref(),
        Some("quick-blake3-v1:updated")
    );
    assert_eq!(updated.content_hash.as_deref(), Some("blake3:updated"));
    assert!(catalog
        .update_media_fingerprints(999_999, Some("x".into()), None)
        .is_err());
}

#[test]
fn imported_lookup_survives_reopening_database() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("catalog.sqlite");
    let catalog = CatalogHandle::open(db.clone()).unwrap();
    let (_, media, _) = imported_fixture(&catalog, ImportStatus::Completed);
    assert!(catalog.lookup_imported(media.clone()).unwrap());
    drop(catalog);
    let reopened = CatalogHandle::open(db).unwrap();
    assert!(reopened.lookup_imported(media).unwrap());
}

#[test]
fn v1_database_migrates_and_allows_primary_and_backup_destinations() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("catalog.sqlite");
    let connection = Connection::open(&db).unwrap();
    connection.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        CREATE TABLE sources (id INTEGER PRIMARY KEY, stable_id TEXT, source_type TEXT NOT NULL, manufacturer TEXT, model TEXT, serial TEXT, alias TEXT, volume_uuid TEXT, first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL);
        CREATE TABLE media (id INTEGER PRIMARY KEY, source_id INTEGER NOT NULL REFERENCES sources(id), source_path TEXT NOT NULL, source_filename TEXT NOT NULL, source_size INTEGER NOT NULL, capture_time TEXT, media_type TEXT NOT NULL, quick_fingerprint TEXT, content_hash TEXT, first_seen_at TEXT NOT NULL);
        CREATE TABLE import_sessions (id INTEGER PRIMARY KEY, source_id INTEGER NOT NULL REFERENCES sources(id), preset_id INTEGER, started_at TEXT NOT NULL, completed_at TEXT, status TEXT NOT NULL, error TEXT);
        CREATE TABLE imports (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES import_sessions(id), media_id INTEGER NOT NULL REFERENCES media(id), destination_path TEXT NOT NULL, destination_size INTEGER, copy_completed_at TEXT, verification_method TEXT, verified_at TEXT, status TEXT NOT NULL, error TEXT, UNIQUE(session_id, media_id));
        CREATE TABLE presets (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, configuration_json TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
        CREATE TABLE schema_version (version INTEGER NOT NULL);
        INSERT INTO schema_version VALUES (1);
        PRAGMA user_version = 1;
        "#,
    ).unwrap();
    drop(connection);

    let catalog = CatalogHandle::open(db.clone()).unwrap();
    let source = catalog
        .upsert_source(identity("migration-camera"), "now")
        .unwrap();
    let media = catalog
        .upsert_media(
            MediaIdentity {
                source_id: source.id,
                source_path: "DCIM/100/DSC00001.JPG".into(),
                source_filename: "DSC00001.JPG".into(),
                source_size: 99,
                capture_time: Some("2026-09-27T12:00:00Z".into()),
            },
            MediaType::Photo,
            "now",
        )
        .unwrap();
    let session = catalog.begin_session(source.id, None, "now").unwrap();
    for destination in ["Pictures/DSC00001.JPG", "Backup/DSC00001.JPG"] {
        catalog
            .record_import(
                session.id,
                ImportInput {
                    media_id: media.id,
                    destination_path: destination.into(),
                    destination_size: Some(99),
                    copy_completed_at: Some("now".into()),
                    verification_method: Some("size".into()),
                    verified_at: Some("now".into()),
                    status: ImportStatus::Verified,
                    error: None,
                },
            )
            .unwrap();
    }
    assert_eq!(catalog.session_detail(session.id).unwrap().imports.len(), 2);
    assert!(catalog.lookup_imported(media.identity.clone()).unwrap());
    assert!(db.with_extension("sqlite.pre-v2.bak").exists());
}
