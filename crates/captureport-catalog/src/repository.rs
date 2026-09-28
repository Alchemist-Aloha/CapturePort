use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::CatalogError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceIdentity {
    pub stable_id: Option<String>,
    pub source_type: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub volume_uuid: Option<String>,
    pub alias: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRecord {
    pub id: i64,
    pub identity: SourceIdentity,
    pub first_seen_at: String,
    pub last_seen_at: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MediaType {
    Photo,
    Video,
    Raw,
    Sidecar,
    Unknown,
}
impl MediaType {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Video => "video",
            Self::Raw => "raw",
            Self::Sidecar => "sidecar",
            Self::Unknown => "unknown",
        }
    }
}
impl From<String> for MediaType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "photo" => Self::Photo,
            "video" => Self::Video,
            "raw" => Self::Raw,
            "sidecar" => Self::Sidecar,
            _ => Self::Unknown,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaIdentity {
    pub source_id: i64,
    pub source_path: String,
    pub source_filename: String,
    pub source_size: u64,
    pub capture_time: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaRecord {
    pub id: i64,
    pub identity: MediaIdentity,
    pub media_type: MediaType,
    pub quick_fingerprint: Option<String>,
    pub content_hash: Option<String>,
    pub first_seen_at: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportStatus {
    Planned,
    Copying,
    Completed,
    Verified,
    Failed,
    Cancelled,
    Partial,
}
impl ImportStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Copying => "copying",
            Self::Completed => "completed",
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Partial => "partial",
        }
    }
}
impl From<String> for ImportStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "planned" => Self::Planned,
            "copying" => Self::Copying,
            "completed" => Self::Completed,
            "verified" => Self::Verified,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Partial,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportRecord {
    pub id: i64,
    pub session_id: i64,
    pub media_id: i64,
    pub destination_path: String,
    pub destination_size: Option<u64>,
    pub copy_completed_at: Option<String>,
    pub verification_method: Option<String>,
    pub verified_at: Option<String>,
    pub status: ImportStatus,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionStatus {
    InProgress,
    Completed,
    Partial,
    Failed,
    Cancelled,
}
impl SessionStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}
impl From<String> for SessionStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "completed" => Self::Completed,
            "partial" => Self::Partial,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::InProgress,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub id: i64,
    pub source_id: i64,
    pub preset_id: Option<i64>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: SessionStatus,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionDetail {
    pub session: SessionRecord,
    pub imports: Vec<ImportRecord>,
}
pub type IncompleteSession = SessionDetail;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresetRecord {
    pub id: i64,
    pub name: String,
    pub configuration_json: String,
    pub created_at: String,
    pub updated_at: String,
}

pub(crate) fn upsert_source(
    c: &Connection,
    identity: &SourceIdentity,
    now: &str,
) -> Result<SourceRecord, CatalogError> {
    let id: Option<i64> = if let Some(stable) = &identity.stable_id {
        c.query_row("SELECT id FROM sources WHERE stable_id=?1", [stable], |r| {
            r.get(0)
        })
        .optional()?
    } else {
        None
    };
    let id = id.or_else(|| {
        if identity.serial.is_none() && identity.volume_uuid.is_none() {
            None
        } else {
            c.query_row("SELECT id FROM sources WHERE source_type=?1 AND ifnull(serial,'')=ifnull(?2,'') AND ifnull(volume_uuid,'')=ifnull(?3,'')",params![identity.source_type,identity.serial,identity.volume_uuid],|r|r.get(0)).optional().ok().flatten()
        }
    });
    let id = if let Some(id) = id {
        // Discovery may not know the user's friendly alias. Preserve it on
        // reconnect; explicit edits use `set_source_alias` below.
        c.execute("UPDATE sources SET source_type=?1,manufacturer=?2,model=?3,serial=?4,alias=coalesce(?5,alias),volume_uuid=?6,last_seen_at=?7 WHERE id=?8",params![identity.source_type,identity.manufacturer,identity.model,identity.serial,identity.alias,identity.volume_uuid,now,id])?;
        id
    } else {
        c.execute("INSERT INTO sources(stable_id,source_type,manufacturer,model,serial,alias,volume_uuid,first_seen_at,last_seen_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8)",params![identity.stable_id,identity.source_type,identity.manufacturer,identity.model,identity.serial,identity.alias,identity.volume_uuid,now])?;
        c.last_insert_rowid()
    };
    source_by_id(c, id)
}
pub(crate) fn source_by_id(c: &Connection, id: i64) -> Result<SourceRecord, CatalogError> {
    c.query_row("SELECT id,stable_id,source_type,manufacturer,model,serial,alias,volume_uuid,first_seen_at,last_seen_at FROM sources WHERE id=?1",[id],|r| Ok(SourceRecord{id:r.get(0)?,identity:SourceIdentity{stable_id:r.get(1)?,source_type:r.get(2)?,manufacturer:r.get(3)?,model:r.get(4)?,serial:r.get(5)?,alias:r.get(6)?,volume_uuid:r.get(7)?},first_seen_at:r.get(8)?,last_seen_at:r.get(9)?})).map_err(Into::into)
}

pub(crate) fn source_by_id_optional(
    c: &Connection,
    id: i64,
) -> Result<Option<SourceRecord>, CatalogError> {
    let found = c
        .query_row("SELECT id FROM sources WHERE id=?1", [id], |r| r.get(0))
        .optional()?;
    found.map(|id| source_by_id(c, id)).transpose()
}

pub(crate) fn source_by_stable_id(
    c: &Connection,
    stable_id: &str,
) -> Result<Option<SourceRecord>, CatalogError> {
    let id = c
        .query_row(
            "SELECT id FROM sources WHERE stable_id=?1",
            [stable_id],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|id| source_by_id(c, id)).transpose()
}
pub(crate) fn upsert_media(
    c: &Connection,
    m: &MediaIdentity,
    media_type: &MediaType,
    now: &str,
    quick: Option<&str>,
    hash: Option<&str>,
) -> Result<MediaRecord, CatalogError> {
    let existing:Option<i64>=c.query_row("SELECT id FROM media WHERE source_id=?1 AND source_path=?2 AND source_filename=?3 AND source_size=?4 AND (capture_time=?5 OR (capture_time IS NULL AND ?5 IS NULL))",params![m.source_id,m.source_path,m.source_filename,m.source_size as i64,m.capture_time],|r|r.get(0)).optional()?;
    let id = if let Some(id) = existing {
        c.execute("UPDATE media SET quick_fingerprint=coalesce(?1,quick_fingerprint),content_hash=coalesce(?2,content_hash) WHERE id=?3",params![quick,hash,id])?;
        id
    } else {
        c.execute("INSERT INTO media(source_id,source_path,source_filename,source_size,capture_time,media_type,quick_fingerprint,content_hash,first_seen_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![m.source_id,m.source_path,m.source_filename,m.source_size as i64,m.capture_time,media_type.as_str(),quick,hash,now])?;
        c.last_insert_rowid()
    };
    media_by_id(c, id)
}
pub(crate) fn media_by_id(c: &Connection, id: i64) -> Result<MediaRecord, CatalogError> {
    c.query_row("SELECT id,source_id,source_path,source_filename,source_size,capture_time,media_type,quick_fingerprint,content_hash,first_seen_at FROM media WHERE id=?1",[id],media_from_row).map_err(Into::into)
}

pub(crate) fn update_media_fingerprints(
    c: &Connection,
    media_id: i64,
    quick_fingerprint: Option<&str>,
    content_hash: Option<&str>,
) -> Result<MediaRecord, CatalogError> {
    let changed = c.execute(
        "UPDATE media SET quick_fingerprint=coalesce(?1,quick_fingerprint), content_hash=coalesce(?2,content_hash) WHERE id=?3",
        params![quick_fingerprint, content_hash, media_id],
    )?;
    if changed == 0 {
        return Err(CatalogError::Request(format!(
            "media row {media_id} does not exist"
        )));
    }
    media_by_id(c, media_id)
}
fn media_from_row(r: &Row<'_>) -> rusqlite::Result<MediaRecord> {
    Ok(MediaRecord {
        id: r.get(0)?,
        identity: MediaIdentity {
            source_id: r.get(1)?,
            source_path: r.get(2)?,
            source_filename: r.get(3)?,
            source_size: r.get::<_, i64>(4)? as u64,
            capture_time: r.get(5)?,
        },
        media_type: MediaType::from(r.get::<_, String>(6)?),
        quick_fingerprint: r.get(7)?,
        content_hash: r.get(8)?,
        first_seen_at: r.get(9)?,
    })
}
pub(crate) fn media_matches(
    c: &Connection,
    m: &MediaIdentity,
) -> Result<Option<MediaRecord>, CatalogError> {
    c.query_row("SELECT id,source_id,source_path,source_filename,source_size,capture_time,media_type,quick_fingerprint,content_hash,first_seen_at FROM media WHERE source_id=?1 AND source_path=?2 AND source_filename=?3 AND source_size=?4 AND (capture_time=?5 OR (capture_time IS NULL AND ?5 IS NULL))",params![m.source_id,m.source_path,m.source_filename,m.source_size as i64,m.capture_time],media_from_row).optional().map_err(Into::into)
}
pub(crate) fn imported_match(c: &Connection, m: &MediaIdentity) -> Result<bool, CatalogError> {
    let imported: Option<i64> = c
        .query_row(
            "SELECT 1 FROM media m JOIN imports i ON i.media_id=m.id WHERE m.source_id=?1 AND m.source_path=?2 AND m.source_filename=?3 AND m.source_size=?4 AND (m.capture_time=?5 OR (m.capture_time IS NULL AND ?5 IS NULL)) AND i.status IN ('completed','verified') LIMIT 1",
            params![m.source_id, m.source_path, m.source_filename, m.source_size as i64, m.capture_time],
            |r| r.get(0),
        )
        .optional()?;
    Ok(imported.is_some())
}

pub(crate) fn mark_manually_imported(
    c: &Connection,
    media_ids: &[i64],
    marked_at: &str,
) -> Result<(), CatalogError> {
    let tx = c.unchecked_transaction()?;
    for media_id in media_ids {
        let changed = tx.execute(
            "INSERT INTO manual_import_marks(media_id,marked_at,quick_fingerprint) SELECT id,?2,quick_fingerprint FROM media WHERE id=?1 ON CONFLICT(media_id) DO UPDATE SET marked_at=excluded.marked_at,quick_fingerprint=excluded.quick_fingerprint",
            params![media_id, marked_at],
        )?;
        if changed == 0 {
            return Err(CatalogError::Request(format!(
                "media row {media_id} does not exist"
            )));
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn manually_imported_match(c: &Connection, media_id: i64) -> Result<bool, CatalogError> {
    c.query_row(
        "SELECT EXISTS(SELECT 1 FROM manual_import_marks mark JOIN media m ON m.id=mark.media_id WHERE m.id=?1 AND mark.quick_fingerprint IS m.quick_fingerprint)",
        [media_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

/// Names the prior import matched by source identity, for the case where the
/// identity is strong evidence but not strong enough to call the file imported.
pub(crate) fn imported_session(
    c: &Connection,
    m: &MediaIdentity,
) -> Result<Option<ImportedMatch>, CatalogError> {
    c.query_row(
        "SELECT i.session_id, s.started_at, i.destination_path FROM media m JOIN imports i ON i.media_id=m.id JOIN import_sessions s ON s.id=i.session_id WHERE m.source_id=?1 AND m.source_path=?2 AND m.source_filename=?3 AND m.source_size=?4 AND (m.capture_time=?5 OR (m.capture_time IS NULL AND ?5 IS NULL)) AND i.status IN ('completed','verified') ORDER BY s.started_at DESC LIMIT 1",
        params![
            m.source_id,
            m.source_path,
            m.source_filename,
            m.source_size as i64,
            m.capture_time
        ],
        |row| {
            Ok(ImportedMatch {
                session_id: row.get(0)?,
                started_at: row.get(1)?,
                destination_path: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedMatch {
    pub session_id: i64,
    pub started_at: String,
    pub destination_path: String,
}

pub(crate) fn imported_fingerprint_match(
    c: &Connection,
    source_size: u64,
    quick_fingerprint: &str,
    content_hash: Option<&str>,
) -> Result<bool, CatalogError> {
    // A quick fingerprint is only useful when its algorithm/version marker
    // is stored with the value.  Requiring the size as a separate predicate
    // keeps this query useful if a future fingerprint implementation changes
    // what it includes in the digest.
    let imported: Option<i64> = if let Some(content_hash) = content_hash {
        c.query_row(
            "SELECT 1 FROM media m JOIN imports i ON i.media_id=m.id WHERE m.source_size=?1 AND m.quick_fingerprint=?2 AND m.content_hash=?3 AND i.status IN ('completed','verified') LIMIT 1",
            params![source_size as i64, quick_fingerprint, content_hash],
            |r| r.get(0),
        )
        .optional()?
    } else {
        c.query_row(
            "SELECT 1 FROM media m JOIN imports i ON i.media_id=m.id WHERE m.source_size=?1 AND m.quick_fingerprint=?2 AND i.status IN ('completed','verified') LIMIT 1",
            params![source_size as i64, quick_fingerprint],
            |r| r.get(0),
        )
        .optional()?
    };
    Ok(imported.is_some())
}

/// The same evidence rules as [`imported_fingerprint_match`], but it also names
/// the prior import so the browser can explain *why* a file looks imported
/// instead of only asserting that it does.
pub(crate) fn imported_fingerprint_session(
    c: &Connection,
    source_size: u64,
    quick_fingerprint: &str,
    content_hash: Option<&str>,
) -> Result<Option<ImportedMatch>, CatalogError> {
    let map = |row: &Row| -> rusqlite::Result<ImportedMatch> {
        Ok(ImportedMatch {
            session_id: row.get(0)?,
            started_at: row.get(1)?,
            destination_path: row.get(2)?,
        })
    };
    let matched = if let Some(content_hash) = content_hash {
        c.query_row(
            "SELECT i.session_id, s.started_at, i.destination_path FROM media m JOIN imports i ON i.media_id=m.id JOIN import_sessions s ON s.id=i.session_id WHERE m.source_size=?1 AND m.quick_fingerprint=?2 AND m.content_hash=?3 AND i.status IN ('completed','verified') ORDER BY s.started_at DESC LIMIT 1",
            params![source_size as i64, quick_fingerprint, content_hash],
            map,
        )
        .optional()?
    } else {
        c.query_row(
            "SELECT i.session_id, s.started_at, i.destination_path FROM media m JOIN imports i ON i.media_id=m.id JOIN import_sessions s ON s.id=i.session_id WHERE m.source_size=?1 AND m.quick_fingerprint=?2 AND i.status IN ('completed','verified') ORDER BY s.started_at DESC LIMIT 1",
            params![source_size as i64, quick_fingerprint],
            map,
        )
        .optional()?
    };
    Ok(matched)
}
pub(crate) fn session(c: &Connection, id: i64) -> Result<SessionRecord, CatalogError> {
    c.query_row("SELECT id,source_id,preset_id,started_at,completed_at,status,error FROM import_sessions WHERE id=?1",[id],|r|Ok(SessionRecord{id:r.get(0)?,source_id:r.get(1)?,preset_id:r.get(2)?,started_at:r.get(3)?,completed_at:r.get(4)?,status:SessionStatus::from(r.get::<_,String>(5)?),error:r.get(6)?})).map_err(Into::into)
}
pub(crate) fn imports(c: &Connection, id: i64) -> Result<Vec<ImportRecord>, CatalogError> {
    let mut st=c.prepare("SELECT id,session_id,media_id,destination_path,destination_size,copy_completed_at,verification_method,verified_at,status,error FROM imports WHERE session_id=?1 ORDER BY id")?;
    let rows = st
        .query_map([id], |r| {
            Ok(ImportRecord {
                id: r.get(0)?,
                session_id: r.get(1)?,
                media_id: r.get(2)?,
                destination_path: r.get(3)?,
                destination_size: r.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                copy_completed_at: r.get(5)?,
                verification_method: r.get(6)?,
                verified_at: r.get(7)?,
                status: ImportStatus::from(r.get::<_, String>(8)?),
                error: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
pub(crate) fn detail(c: &Connection, id: i64) -> Result<SessionDetail, CatalogError> {
    Ok(SessionDetail {
        session: session(c, id)?,
        imports: imports(c, id)?,
    })
}
