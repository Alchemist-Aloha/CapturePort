use crate::{
    AppEvent, ImportStatus, MediaId, MediaItem, MediaType, ScanGeneration, SourceIdentity,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MediaFilter {
    #[default]
    All,
    Photos,
    Videos,
    New,
    Imported,
    PossibleDuplicates,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MediaSort {
    Name,
    #[default]
    CaptureTime,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectionSummary {
    pub count: usize,
    pub bytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub generation: ScanGeneration,
    pub source: Option<SourceIdentity>,
    items: HashMap<MediaId, MediaItem>,
    order: Vec<MediaId>,
    selected: HashSet<MediaId>,
    thumbnails: HashSet<MediaId>,
    pub filter: MediaFilter,
    pub sort: MediaSort,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn apply_event(&mut self, event: AppEvent) -> bool {
        let generation = event.generation();
        match event {
            AppEvent::SourceDetected { source, .. } => {
                if generation < self.generation {
                    return false;
                }
                if generation > self.generation {
                    self.reset(generation);
                }
                self.source = Some(source);
            }
            AppEvent::ScanStarted { source_id, .. } => {
                if generation < self.generation {
                    return false;
                }
                if generation > self.generation {
                    self.reset(generation);
                }
                if self.source.as_ref().is_some_and(|s| s.id != source_id) {
                    self.source = None;
                }
            }
            _ if generation != self.generation => return false,
            AppEvent::SourceRemoved { source_id, .. } => {
                if self.source.as_ref().is_some_and(|s| s.id == source_id) {
                    self.source = None;
                    self.items.clear();
                    self.order.clear();
                    self.selected.clear();
                    self.thumbnails.clear();
                }
            }
            AppEvent::MediaDiscovered { item, .. } => {
                if !self.source.as_ref().is_some_and(|s| s.id == item.source_id) {
                    return false;
                }
                let is_new = !self.items.contains_key(&item.id);
                if is_new {
                    self.order.push(item.id);
                }
                let id = item.id;
                let status = item.import_status.clone();
                self.items.insert(id, item);
                if is_new {
                    if status == ImportStatus::New {
                        self.selected.insert(id);
                    } else {
                        self.selected.remove(&id);
                    }
                }
            }
            AppEvent::MetadataReady {
                media_id, metadata, ..
            } => {
                if let Some(item) = self.items.get_mut(&media_id) {
                    item.metadata = crate::MetadataState::Ready(metadata);
                }
            }
            AppEvent::MetadataFailed {
                media_id, error, ..
            } => {
                if let Some(item) = self.items.get_mut(&media_id) {
                    item.metadata = crate::MetadataState::Failed(error);
                }
            }
            AppEvent::ThumbnailReady { media_id, .. } => {
                if self.items.contains_key(&media_id) {
                    self.thumbnails.insert(media_id);
                }
            }
            AppEvent::ImportStatusChanged {
                media_id,
                status,
                prior_import,
                ..
            } => {
                if let Some(item) = self.items.get_mut(&media_id) {
                    item.import_status = status.clone();
                    item.prior_import = prior_import.clone();
                    if status != ImportStatus::New {
                        self.selected.remove(&media_id);
                    }
                }
            }
            AppEvent::ImportProgress { .. }
            | AppEvent::ImportCompleted { .. }
            | AppEvent::ImportFailed { .. } => {}
        }
        true
    }
    fn reset(&mut self, generation: ScanGeneration) {
        self.generation = generation;
        self.source = None;
        self.items.clear();
        self.order.clear();
        self.selected.clear();
        self.thumbnails.clear();
    }
    pub fn items(&self) -> impl Iterator<Item = &MediaItem> {
        self.order.iter().filter_map(|id| self.items.get(id))
    }
    pub fn item(&self, id: MediaId) -> Option<&MediaItem> {
        self.items.get(&id)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn has_thumbnail(&self, id: MediaId) -> bool {
        self.thumbnails.contains(&id)
    }
    pub fn select(&mut self, id: MediaId, selected: bool) {
        if self.items.contains_key(&id) {
            if selected {
                self.selected.insert(id);
            } else {
                self.selected.remove(&id);
            }
        }
    }
    pub fn is_selected(&self, id: MediaId) -> bool {
        self.selected.contains(&id)
    }
    pub fn select_all(&mut self, selected: bool) {
        if selected {
            self.selected.extend(self.visible_ids());
        } else {
            for id in self.visible_ids() {
                self.selected.remove(&id);
            }
        }
    }
    pub fn select_all_new(&mut self) {
        for item in self.items.values() {
            if item.import_status == ImportStatus::New && self.matches_filter(item) {
                self.selected.insert(item.id);
            }
        }
    }
    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }
    pub fn selection_summary(&self) -> SelectionSummary {
        SelectionSummary {
            count: self.selected.len(),
            bytes: self
                .selected
                .iter()
                .filter_map(|id| self.items.get(id))
                .map(|item| item.size)
                .fold(0u64, u64::saturating_add),
        }
    }
    pub fn visible_items(&self) -> Vec<&MediaItem> {
        let mut items: Vec<_> = self
            .items()
            .filter(|item| self.matches_filter(item))
            .collect();
        match self.sort {
            MediaSort::Name => items.sort_by(|a, b| {
                a.source_name
                    .to_lowercase()
                    .cmp(&b.source_name.to_lowercase())
                    .then(a.id.cmp(&b.id))
            }),
            MediaSort::CaptureTime => items.sort_by(|a, b| {
                capture_sort_key(a)
                    .cmp(capture_sort_key(b))
                    .then(a.source_name.cmp(&b.source_name))
            }),
        }
        items
    }
    fn visible_ids(&self) -> Vec<MediaId> {
        self.items()
            .filter(|item| self.matches_filter(item))
            .map(|item| item.id)
            .collect()
    }
    fn matches_filter(&self, item: &MediaItem) -> bool {
        match self.filter {
            MediaFilter::All => true,
            MediaFilter::Photos => matches!(
                item.media_type,
                MediaType::Raw
                    | MediaType::Jpeg
                    | MediaType::Heif
                    | MediaType::Png
                    | MediaType::Tiff
            ),
            MediaFilter::Videos => item.media_type == MediaType::Video,
            MediaFilter::New => item.import_status == ImportStatus::New,
            MediaFilter::Imported => item.import_status == ImportStatus::Imported,
            MediaFilter::PossibleDuplicates => {
                item.import_status == ImportStatus::PossibleDuplicate
            }
        }
    }
}
fn capture_sort_key(item: &MediaItem) -> &str {
    match &item.metadata {
        crate::MetadataState::Ready(m) => m.capture_time.as_deref().unwrap_or(""),
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MediaId, MediaItem, ScanGeneration, SourceId, SourceIdentity, SourceType};
    fn source() -> SourceIdentity {
        SourceIdentity {
            id: SourceId(1),
            source_type: SourceType::Fake,
            stable_id: None,
            serial: None,
            manufacturer: None,
            model: None,
            volume_uuid: None,
            display_name: Some("fake".into()),
        }
    }
    fn discovered(id: u64, generation: u64) -> AppEvent {
        AppEvent::MediaDiscovered {
            generation: ScanGeneration(generation),
            item: MediaItem::new(MediaId(id), SourceId(1), format!("DSC{id:05}.JPG"), id),
        }
    }
    #[test]
    fn incremental_sets_handle_expected_scales_and_selection() {
        for count in [1, 100, 10_000] {
            let mut state = AppState::new();
            state.apply_event(AppEvent::SourceDetected {
                generation: ScanGeneration(1),
                source: source(),
            });
            for id in 0..count {
                assert!(state.apply_event(discovered(id as u64, 1)));
            }
            assert_eq!(state.len(), count);
            assert_eq!(state.selected_count(), count);
            assert_eq!(state.selection_summary().bytes, (0..count as u64).sum());
            assert_eq!(state.visible_items().len(), count);
        }
    }
    #[test]
    fn duplicate_events_are_idempotent_and_metadata_failures_are_visible() {
        let mut state = AppState::new();
        state.apply_event(AppEvent::SourceDetected {
            generation: ScanGeneration(1),
            source: source(),
        });
        state.apply_event(discovered(3, 1));
        state.apply_event(discovered(3, 1));
        state.apply_event(AppEvent::MetadataFailed {
            generation: ScanGeneration(1),
            media_id: MediaId(3),
            error: "bad metadata".into(),
        });
        assert_eq!(state.len(), 1);
        assert_eq!(state.selected_count(), 1);
        assert_eq!(
            state.item(MediaId(3)).unwrap().metadata,
            crate::MetadataState::Failed("bad metadata".into())
        );
    }
    #[test]
    fn removal_clears_mid_scan_and_old_generation_is_ignored() {
        let mut state = AppState::new();
        state.apply_event(AppEvent::SourceDetected {
            generation: ScanGeneration(4),
            source: source(),
        });
        state.apply_event(discovered(1, 4));
        assert!(state.apply_event(AppEvent::SourceRemoved {
            generation: ScanGeneration(4),
            source_id: SourceId(1)
        }));
        assert_eq!(state.len(), 0);
        assert!(state.source.is_none());
        assert!(!state.apply_event(discovered(2, 4)));
        assert!(!state.apply_event(discovered(2, 3)));
        assert_eq!(state.len(), 0);
    }
}
