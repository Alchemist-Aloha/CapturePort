use crate::CatalogError;
use crate::{database::CatalogPath, migrations, repository, repository::*};
use rusqlite::{Connection, OptionalExtension};
use std::{
    sync::{mpsc, Arc},
    thread,
};

#[derive(Clone, Debug)]
pub struct ImportInput {
    pub media_id: i64,
    pub destination_path: String,
    pub destination_size: Option<u64>,
    pub copy_completed_at: Option<String>,
    pub verification_method: Option<String>,
    pub verified_at: Option<String>,
    pub status: ImportStatus,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReconciledImportInput {
    pub session_id: i64,
    pub media: MediaIdentity,
    pub media_type: MediaType,
    pub observed_at: String,
    pub quick_fingerprint: Option<String>,
    pub content_hash: Option<String>,
    pub import: ImportInput,
}
#[derive(Clone, Debug)]
pub enum CatalogCommand {
    UpsertSource {
        identity: SourceIdentity,
        observed_at: String,
    },
    LookupSourceByStableId {
        stable_id: String,
    },
    LookupSourceById {
        source_id: i64,
    },
    SetSourceAlias {
        source_id: i64,
        alias: Option<String>,
    },
    UpsertMedia {
        media: MediaIdentity,
        media_type: MediaType,
        observed_at: String,
        quick_fingerprint: Option<String>,
        content_hash: Option<String>,
    },
    LookupMedia {
        media: MediaIdentity,
    },
    LookupImported {
        media: MediaIdentity,
    },
    /// As `LookupImported`, but names the prior import rather than answering yes/no.
    LookupImportedSessionByIdentity {
        media: MediaIdentity,
    },
    /// Look up a previously imported file by its content identity.  This is
    /// used after a source scan when path based matching is unavailable.
    LookupImportedFingerprint {
        source_size: u64,
        quick_fingerprint: String,
        content_hash: Option<String>,
    },
    /// As above, but also names the prior import so the browser can show which
    /// session a possible duplicate matched.
    LookupImportedSession {
        source_size: u64,
        quick_fingerprint: String,
        content_hash: Option<String>,
    },
    UpdateMediaFingerprints {
        media_id: i64,
        quick_fingerprint: Option<String>,
        content_hash: Option<String>,
    },
    BeginSession {
        source_id: i64,
        preset_id: Option<i64>,
        started_at: String,
    },
    RecordImport {
        session_id: i64,
        import: ImportInput,
    },
    /// Persist a file discovered while reconciling an existing destination.
    /// The operation upserts its media cache row and records a verified import
    /// in the supplied reconciliation session atomically on the catalog
    /// worker.
    RecordReconciledImport {
        session_id: i64,
        media: MediaIdentity,
        media_type: MediaType,
        observed_at: String,
        quick_fingerprint: Option<String>,
        content_hash: Option<String>,
        import: ImportInput,
    },
    CompleteSession {
        session_id: i64,
        completed_at: Option<String>,
        status: SessionStatus,
        error: Option<String>,
    },
    ListSessions {
        limit: u32,
    },
    SessionDetail {
        session_id: i64,
    },
    IncompleteSessions,
    UpsertPreset {
        id: Option<i64>,
        name: String,
        configuration_json: String,
        updated_at: String,
    },
}
#[derive(Clone, Debug)]
pub enum CatalogResponse {
    Source(SourceRecord),
    SourceMatch(Option<SourceRecord>),
    SourceById(Option<SourceRecord>),
    Media(MediaRecord),
    MediaMatch(Option<MediaRecord>),
    Imported(bool),
    ImportedSession(Option<ImportedMatch>),
    Session(SessionRecord),
    Import(ImportRecord),
    Sessions(Vec<SessionRecord>),
    Detail(SessionDetail),
    Incomplete(Vec<IncompleteSession>),
    Preset(PresetRecord),
    Unit,
}
struct Envelope {
    command: CatalogCommand,
    reply: mpsc::Sender<Result<CatalogResponse, CatalogError>>,
}
#[derive(Clone)]
pub struct CatalogHandle {
    sender: Arc<mpsc::Sender<Envelope>>,
}

impl CatalogHandle {
    pub fn open(path: impl Into<CatalogPath>) -> Result<Self, CatalogError> {
        let path = path.into();
        let (tx, rx) = mpsc::channel::<Envelope>();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("captureport-catalog".into())
            .spawn(move || {
                let result = match &path {
                    CatalogPath::Memory => Connection::open_in_memory().map_err(CatalogError::from),
                    CatalogPath::File(p) => Connection::open(p).map_err(CatalogError::from),
                };
                let connection = match result {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let file_path = match &path {
                    CatalogPath::File(p) => Some(p.as_path()),
                    CatalogPath::Memory => None,
                };
                if let Err(e) = migrations::migrate(&connection, file_path) {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
                let _ = ready_tx.send(Ok(()));
                while let Ok(envelope) = rx.recv() {
                    let result = execute(&connection, envelope.command);
                    let _ = envelope.reply.send(result);
                }
            })
            .map_err(|_| CatalogError::WorkerUnavailable)?;
        ready_rx
            .recv()
            .map_err(|_| CatalogError::WorkerUnavailable)??;
        Ok(Self {
            sender: Arc::new(tx),
        })
    }
    pub fn execute(&self, command: CatalogCommand) -> Result<CatalogResponse, CatalogError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.sender
            .send(Envelope {
                command,
                reply: reply_tx,
            })
            .map_err(|_| CatalogError::WorkerUnavailable)?;
        reply_rx
            .recv()
            .map_err(|_| CatalogError::WorkerUnavailable)?
    }
    pub fn upsert_source(
        &self,
        identity: SourceIdentity,
        observed_at: impl Into<String>,
    ) -> Result<SourceRecord, CatalogError> {
        match self.execute(CatalogCommand::UpsertSource {
            identity,
            observed_at: observed_at.into(),
        })? {
            CatalogResponse::Source(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn source_by_stable_id(
        &self,
        stable_id: impl Into<String>,
    ) -> Result<Option<SourceRecord>, CatalogError> {
        match self.execute(CatalogCommand::LookupSourceByStableId {
            stable_id: stable_id.into(),
        })? {
            CatalogResponse::SourceMatch(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn source_by_id(&self, source_id: i64) -> Result<Option<SourceRecord>, CatalogError> {
        match self.execute(CatalogCommand::LookupSourceById { source_id })? {
            CatalogResponse::SourceById(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn set_source_alias(
        &self,
        source_id: i64,
        alias: Option<String>,
    ) -> Result<SourceRecord, CatalogError> {
        match self.execute(CatalogCommand::SetSourceAlias { source_id, alias })? {
            CatalogResponse::Source(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn upsert_media(
        &self,
        media: MediaIdentity,
        media_type: MediaType,
        observed_at: impl Into<String>,
    ) -> Result<MediaRecord, CatalogError> {
        match self.execute(CatalogCommand::UpsertMedia {
            media,
            media_type,
            observed_at: observed_at.into(),
            quick_fingerprint: None,
            content_hash: None,
        })? {
            CatalogResponse::Media(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn lookup_media(&self, media: MediaIdentity) -> Result<Option<MediaRecord>, CatalogError> {
        match self.execute(CatalogCommand::LookupMedia { media })? {
            CatalogResponse::MediaMatch(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    /// Returns true only when the exact source media identity has a completed
    /// or verified import row. A catalog media row by itself is not enough.
    pub fn lookup_imported(&self, media: MediaIdentity) -> Result<bool, CatalogError> {
        match self.execute(CatalogCommand::LookupImported { media })? {
            CatalogResponse::Imported(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn lookup_imported_fingerprint(
        &self,
        source_size: u64,
        quick_fingerprint: impl Into<String>,
        content_hash: Option<String>,
    ) -> Result<bool, CatalogError> {
        match self.execute(CatalogCommand::LookupImportedFingerprint {
            source_size,
            quick_fingerprint: quick_fingerprint.into(),
            content_hash,
        })? {
            CatalogResponse::Imported(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn lookup_imported_session(
        &self,
        source_size: u64,
        quick_fingerprint: impl Into<String>,
        content_hash: Option<String>,
    ) -> Result<Option<ImportedMatch>, CatalogError> {
        match self.execute(CatalogCommand::LookupImportedSession {
            source_size,
            quick_fingerprint: quick_fingerprint.into(),
            content_hash,
        })? {
            CatalogResponse::ImportedSession(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn lookup_imported_session_by_identity(
        &self,
        media: MediaIdentity,
    ) -> Result<Option<ImportedMatch>, CatalogError> {
        match self.execute(CatalogCommand::LookupImportedSessionByIdentity { media })? {
            CatalogResponse::ImportedSession(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    /// Update fingerprints on an existing media row without changing its
    /// source identity or creating a second catalog row.
    pub fn update_media_fingerprints(
        &self,
        media_id: i64,
        quick_fingerprint: Option<String>,
        content_hash: Option<String>,
    ) -> Result<MediaRecord, CatalogError> {
        match self.execute(CatalogCommand::UpdateMediaFingerprints {
            media_id,
            quick_fingerprint,
            content_hash,
        })? {
            CatalogResponse::Media(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn begin_session(
        &self,
        source_id: i64,
        preset_id: Option<i64>,
        started_at: impl Into<String>,
    ) -> Result<SessionRecord, CatalogError> {
        match self.execute(CatalogCommand::BeginSession {
            source_id,
            preset_id,
            started_at: started_at.into(),
        })? {
            CatalogResponse::Session(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn record_import(
        &self,
        session_id: i64,
        import: ImportInput,
    ) -> Result<ImportRecord, CatalogError> {
        match self.execute(CatalogCommand::RecordImport { session_id, import })? {
            CatalogResponse::Import(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn record_reconciled_import(
        &self,
        input: ReconciledImportInput,
    ) -> Result<ImportRecord, CatalogError> {
        match self.execute(CatalogCommand::RecordReconciledImport {
            session_id: input.session_id,
            media: input.media,
            media_type: input.media_type,
            observed_at: input.observed_at,
            quick_fingerprint: input.quick_fingerprint,
            content_hash: input.content_hash,
            import: input.import,
        })? {
            CatalogResponse::Import(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn complete_session(
        &self,
        session_id: i64,
        completed_at: Option<String>,
        status: SessionStatus,
        error: Option<String>,
    ) -> Result<SessionRecord, CatalogError> {
        match self.execute(CatalogCommand::CompleteSession {
            session_id,
            completed_at,
            status,
            error,
        })? {
            CatalogResponse::Session(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn list_sessions(&self, limit: u32) -> Result<Vec<SessionRecord>, CatalogError> {
        match self.execute(CatalogCommand::ListSessions { limit })? {
            CatalogResponse::Sessions(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn session_detail(&self, id: i64) -> Result<SessionDetail, CatalogError> {
        match self.execute(CatalogCommand::SessionDetail { session_id: id })? {
            CatalogResponse::Detail(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn incomplete_sessions(&self) -> Result<Vec<IncompleteSession>, CatalogError> {
        match self.execute(CatalogCommand::IncompleteSessions)? {
            CatalogResponse::Incomplete(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    pub fn upsert_preset(
        &self,
        id: Option<i64>,
        name: impl Into<String>,
        configuration_json: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Result<PresetRecord, CatalogError> {
        match self.execute(CatalogCommand::UpsertPreset {
            id,
            name: name.into(),
            configuration_json: configuration_json.into(),
            updated_at: updated_at.into(),
        })? {
            CatalogResponse::Preset(v) => Ok(v),
            _ => unreachable!(),
        }
    }
}

fn execute(c: &Connection, cmd: CatalogCommand) -> Result<CatalogResponse, CatalogError> {
    match cmd {
        CatalogCommand::UpsertSource {
            identity,
            observed_at,
        } => Ok(CatalogResponse::Source(upsert_source(
            c,
            &identity,
            &observed_at,
        )?)),
        CatalogCommand::LookupSourceByStableId { stable_id } => Ok(CatalogResponse::SourceMatch(
            source_by_stable_id(c, &stable_id)?,
        )),
        CatalogCommand::LookupSourceById { source_id } => Ok(CatalogResponse::SourceById(
            repository::source_by_id_optional(c, source_id)?,
        )),
        CatalogCommand::SetSourceAlias { source_id, alias } => {
            c.execute(
                "UPDATE sources SET alias=?1 WHERE id=?2",
                rusqlite::params![alias, source_id],
            )?;
            Ok(CatalogResponse::Source(repository::source_by_id(
                c, source_id,
            )?))
        }
        CatalogCommand::UpsertMedia {
            media,
            media_type,
            observed_at,
            quick_fingerprint,
            content_hash,
        } => Ok(CatalogResponse::Media(upsert_media(
            c,
            &media,
            &media_type,
            &observed_at,
            quick_fingerprint.as_deref(),
            content_hash.as_deref(),
        )?)),
        CatalogCommand::LookupMedia { media } => {
            Ok(CatalogResponse::MediaMatch(media_matches(c, &media)?))
        }
        CatalogCommand::LookupImported { media } => {
            Ok(CatalogResponse::Imported(imported_match(c, &media)?))
        }
        CatalogCommand::LookupImportedSessionByIdentity { media } => Ok(
            CatalogResponse::ImportedSession(imported_session(c, &media)?),
        ),
        CatalogCommand::LookupImportedFingerprint {
            source_size,
            quick_fingerprint,
            content_hash,
        } => Ok(CatalogResponse::Imported(imported_fingerprint_match(
            c,
            source_size,
            &quick_fingerprint,
            content_hash.as_deref(),
        )?)),
        CatalogCommand::LookupImportedSession {
            source_size,
            quick_fingerprint,
            content_hash,
        } => Ok(CatalogResponse::ImportedSession(
            imported_fingerprint_session(
                c,
                source_size,
                &quick_fingerprint,
                content_hash.as_deref(),
            )?,
        )),
        CatalogCommand::UpdateMediaFingerprints {
            media_id,
            quick_fingerprint,
            content_hash,
        } => Ok(CatalogResponse::Media(
            repository::update_media_fingerprints(
                c,
                media_id,
                quick_fingerprint.as_deref(),
                content_hash.as_deref(),
            )?,
        )),
        CatalogCommand::BeginSession {
            source_id,
            preset_id,
            started_at,
        } => {
            c.execute("INSERT INTO import_sessions(source_id,preset_id,started_at,status) VALUES(?1,?2,?3,'in_progress')",rusqlite::params![source_id,preset_id,started_at])?;
            Ok(CatalogResponse::Session(session(c, c.last_insert_rowid())?))
        }
        CatalogCommand::RecordImport { session_id, import } => {
            c.execute("INSERT INTO imports(session_id,media_id,destination_path,destination_size,copy_completed_at,verification_method,verified_at,status,error) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",rusqlite::params![session_id,import.media_id,import.destination_path,import.destination_size.map(|v|v as i64),import.copy_completed_at,import.verification_method,import.verified_at,import.status.as_str(),import.error])?;
            let id = c.last_insert_rowid();
            let d = detail(c, session_id)?;
            Ok(CatalogResponse::Import(
                d.imports.into_iter().find(|v| v.id == id).unwrap(),
            ))
        }
        CatalogCommand::RecordReconciledImport {
            session_id,
            media,
            media_type,
            observed_at,
            quick_fingerprint,
            content_hash,
            import,
        } => {
            let transaction = c.unchecked_transaction()?;
            let media = repository::upsert_media(
                &transaction,
                &media,
                &media_type,
                &observed_at,
                quick_fingerprint.as_deref(),
                content_hash.as_deref(),
            )?;
            if import.media_id != 0 && import.media_id != media.id {
                return Err(CatalogError::Request(
                    "reconciled import media_id does not match media identity".into(),
                ));
            }
            transaction.execute("INSERT INTO imports(session_id,media_id,destination_path,destination_size,copy_completed_at,verification_method,verified_at,status,error) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",rusqlite::params![session_id,media.id,import.destination_path,import.destination_size.map(|v|v as i64),import.copy_completed_at,import.verification_method,import.verified_at,import.status.as_str(),import.error])?;
            let id = transaction.last_insert_rowid();
            transaction.commit()?;
            let d = detail(c, session_id)?;
            Ok(CatalogResponse::Import(
                d.imports.into_iter().find(|v| v.id == id).unwrap(),
            ))
        }
        CatalogCommand::CompleteSession {
            session_id,
            completed_at,
            status,
            error,
        } => {
            c.execute(
                "UPDATE import_sessions SET completed_at=?1,status=?2,error=?3 WHERE id=?4",
                rusqlite::params![completed_at, status.as_str(), error, session_id],
            )?;
            Ok(CatalogResponse::Session(session(c, session_id)?))
        }
        CatalogCommand::ListSessions { limit } => {
            let mut s = c.prepare(
                "SELECT id FROM import_sessions ORDER BY started_at DESC,id DESC LIMIT ?1",
            )?;
            let ids = s
                .query_map([limit], |r| r.get(0))?
                .collect::<Result<Vec<i64>, _>>()?;
            Ok(CatalogResponse::Sessions(
                ids.into_iter()
                    .map(|id| session(c, id))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        CatalogCommand::SessionDetail { session_id } => {
            Ok(CatalogResponse::Detail(detail(c, session_id)?))
        }
        CatalogCommand::IncompleteSessions => {
            let mut s=c.prepare("SELECT id FROM import_sessions WHERE status='in_progress' OR status='partial' ORDER BY started_at DESC")?;
            let ids = s
                .query_map([], |r| r.get(0))?
                .collect::<Result<Vec<i64>, _>>()?;
            Ok(CatalogResponse::Incomplete(
                ids.into_iter()
                    .map(|id| detail(c, id))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        CatalogCommand::UpsertPreset {
            id,
            name,
            configuration_json,
            updated_at,
        } => {
            let id = if let Some(id) = id {
                c.execute(
                    "UPDATE presets SET name=?1,configuration_json=?2,updated_at=?3 WHERE id=?4",
                    rusqlite::params![name, configuration_json, updated_at, id],
                )?;
                id
            } else {
                let existing: Option<i64> = c
                    .query_row("SELECT id FROM presets WHERE name=?1", [&name], |r| {
                        r.get(0)
                    })
                    .optional()?;
                if let Some(id) = existing {
                    c.execute(
                        "UPDATE presets SET configuration_json=?1,updated_at=?2 WHERE id=?3",
                        rusqlite::params![configuration_json, updated_at, id],
                    )?;
                    id
                } else {
                    c.execute("INSERT INTO presets(name,configuration_json,created_at,updated_at) VALUES(?1,?2,?3,?3)",rusqlite::params![name,configuration_json,updated_at])?;
                    c.last_insert_rowid()
                }
            };
            let p = c.query_row(
                "SELECT id,name,configuration_json,created_at,updated_at FROM presets WHERE id=?1",
                [id],
                |r| {
                    Ok(PresetRecord {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        configuration_json: r.get(2)?,
                        created_at: r.get(3)?,
                        updated_at: r.get(4)?,
                    })
                },
            )?;
            Ok(CatalogResponse::Preset(p))
        }
    }
}
