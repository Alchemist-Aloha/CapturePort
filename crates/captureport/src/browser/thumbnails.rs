use crate::*;

impl Browser {
    pub(crate) fn request_thumbnail(&mut self, id: MediaId) {
        if self.cache_clearing {
            return;
        }
        if self.requested_thumbnails.contains(&id) {
            return;
        }
        let (Some(source_identity), Some(item)) = (&self.state.source, self.state.item(id)) else {
            return;
        };
        // A filesystem source reads bytes from a path the thumbnail pipeline can
        // open directly. Every other source exposes its preview through the
        // `MediaSource`, so it must take the source-preview route instead.
        if source_identity.source_type != SourceType::Filesystem {
            let Some(source) = &self.source else {
                return;
            };
            let request = ThumbnailRequest {
                source_id: format!(
                    "{}:{}",
                    source_identity
                        .stable_id
                        .clone()
                        .unwrap_or_else(|| source_identity
                            .display_name
                            .clone()
                            .unwrap_or_default()),
                    self.camera_preview_scan_token
                ),
                relative_path: item.source_path.clone(),
                size: item.size,
                modified_unix: 0,
                path: PathBuf::new(),
                media_type: item.media_type,
                priority: 10,
            };
            let key = ThumbnailPipeline::cache_key(&request);
            let job = CameraPreviewJob {
                generation: self.generation,
                id,
                source: source.clone(),
                locator: item.locator.clone(),
                cache_path: self.cache_dir.join("thumbnails").join(format!("{key}.jpg")),
                cancellation: self.cancellation.clone().unwrap_or_default(),
            };
            if self.camera_preview_sender.try_send(job).is_ok() {
                self.requested_thumbnails.insert(id);
            }
            return;
        }
        let Some(root) = &self.filesystem_root else {
            return;
        };
        let Some(modified_unix) = self.thumbnail_modified.get(&id).copied() else {
            // Resolve unusable cache metadata instead of blocking every later
            // candidate behind this item on each queue refill.
            self.failed_thumbnails.insert(id);
            self.requested_thumbnails.insert(id);
            return;
        };
        let request = ThumbnailRequest {
            source_id: format!(
                "{}:{}",
                source_identity.stable_id.as_deref().unwrap_or(""),
                root.display()
            ),
            relative_path: item.source_path.clone(),
            size: item.size,
            modified_unix,
            path: root.join(&item.source_path),
            media_type: item.media_type,
            priority: 10,
        };
        let key = ThumbnailPipeline::cache_key(&request);
        if self.thumbnails.submit(request).is_ok() {
            self.requested_thumbnails.insert(id);
            self.thumbnail_keys.insert(key, id);
        }
    }
}
