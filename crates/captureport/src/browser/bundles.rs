use crate::*;

impl Browser {
    pub(crate) fn invalidate_plan(&mut self) {
        // Selection/settings edits are for the next import, not the running
        // operation. Source reset clears state.source first and still cancels it.
        if self.importing && self.state.source.is_some() {
            return;
        }
        self.plan_revision = self.plan_revision.wrapping_add(1);
        self.planning = false;
        self.plan = None;
        self.last_import_result = None;
        self.deletion_armed = false;
    }
    pub(crate) fn rebuild_bundles(&mut self) {
        self.bundles.clear();
        self.bundle_members.clear();
        self.bundle_owner.clear();
        self.bundle_preview.clear();
        let items = self.state.items().cloned().collect::<Vec<_>>();
        let grouped = captureport_ingest::group_media(&items);
        let mut pairs = Vec::new();
        let merge_raw_jpeg = self.ui.merge_raw_jpeg;
        for bundle in grouped
            .bundles
            .into_iter()
            .filter(|bundle| shows_as_bundle(bundle, merge_raw_jpeg))
        {
            for member in &bundle.members {
                self.bundle_owner.insert(*member, bundle.primary);
            }
            self.bundle_members.extend(
                bundle
                    .members
                    .iter()
                    .copied()
                    .filter(|id| *id != bundle.primary),
            );
            // Render a bundle with the first non-RAW member, so a RAW+JPEG pair
            // shows the JPEG and never decodes the RAW's own preview.
            let preview = bundle
                .members
                .iter()
                .copied()
                .find(|id| {
                    self.state
                        .item(*id)
                        .is_some_and(|item| item.media_type != MediaType::Raw)
                })
                .unwrap_or(bundle.primary);
            if preview != bundle.primary {
                self.bundle_preview.insert(bundle.primary, preview);
                pairs.push((bundle.primary, preview));
            }
            self.bundles.insert(bundle.primary, bundle.members);
        }
        self.pair_index.replace(pairs);
        if self
            .expanded_bundle
            .is_some_and(|id| !self.bundles.contains_key(&id))
        {
            self.expanded_bundle = None;
        }
    }
    /// The id whose thumbnail represents a display item. For a RAW+JPEG bundle
    /// this is the JPEG, so the pair renders without a RAW decode.
    pub(crate) fn preview_id(&self, id: MediaId) -> MediaId {
        self.bundle_preview.get(&id).copied().unwrap_or(id)
    }
    pub(crate) fn toggle_bundle(&mut self, id: MediaId, cx: &mut Context<Self>) {
        if self.bundles.contains_key(&id) {
            self.expanded_bundle = (self.expanded_bundle != Some(id)).then_some(id);
            cx.notify();
        } else {
            self.toggle_bundle_member(id, cx);
        }
    }
    /// Clicking a group name selects every member, or clears them when all are selected.
    pub(crate) fn toggle_group_selection(&mut self, id: MediaId, cx: &mut Context<Self>) {
        if self.bundles.contains_key(&id) {
            let all_selected = self.bundles.get(&id).is_some_and(|members| {
                members.iter().all(|member| self.state.is_selected(*member))
            });
            self.select_bundle_members(id, !all_selected, cx);
        } else {
            self.toggle_bundle_member(id, cx);
        }
    }
    pub(crate) fn toggle_bundle_member(&mut self, id: MediaId, cx: &mut Context<Self>) {
        let selected = !self.state.is_selected(id);
        self.state.select(id, selected);
        if self.bundle_owner.contains_key(&id) {
            if selected {
                self.explicit_bundle_selection.insert(id);
            } else {
                self.explicit_bundle_selection.remove(&id);
            }
        }
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify();
    }
    pub(crate) fn select_bundle_members(
        &mut self,
        primary: MediaId,
        selected: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(members) = self.bundles.get(&primary) else {
            return;
        };
        for &member in members {
            self.state.select(member, selected);
            if selected {
                self.explicit_bundle_selection.insert(member);
            } else {
                self.explicit_bundle_selection.remove(&member);
            }
        }
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify();
    }
}
