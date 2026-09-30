use crate::*;

impl Browser {
    pub(crate) fn gallery_member_ids(&self, key: &str) -> Vec<MediaId> {
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        if let Some(group) = self.gallery_groups.iter().find(|group| group.key == key) {
            for &id in &group.ids {
                if let Some(members) = self.bundles.get(&id) {
                    for &member in members {
                        if seen.insert(member) {
                            ids.push(member);
                        }
                    }
                } else if seen.insert(id) {
                    ids.push(id);
                }
            }
        }
        ids
    }
    /// Clicking a session name selects every visible item in it, or clears them.
    pub(crate) fn toggle_gallery_selection(&mut self, key: &str, cx: &mut Context<Self>) {
        let ids = self.gallery_member_ids(key);
        if ids.is_empty() {
            return;
        }
        let all_selected = ids.iter().all(|id| self.state.is_selected(*id));
        for &id in &ids {
            self.state.select(id, !all_selected);
            if !self.bundle_owner.contains_key(&id) {
                continue;
            }
            if all_selected {
                self.explicit_bundle_selection.remove(&id);
            } else {
                self.explicit_bundle_selection.insert(id);
            }
        }
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify();
    }
    pub(crate) fn edit_gallery(
        &mut self,
        key: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self
            .gallery_groups
            .iter()
            .find(|group| group.key == key)
            .map(|group| group.title.clone())
            .unwrap_or_default();
        self.gallery_edit_input
            .update(cx, |input, cx| input.set_value(title, cx));
        self.gallery_edit_key = Some(key);
        window.focus(&self.gallery_edit_input.read(cx).focus_handle(cx));
        cx.notify();
    }
    pub(crate) fn save_gallery_name(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.gallery_edit_key.clone() else {
            return;
        };
        let name = self.gallery_edit_input.read(cx).value().trim().to_string();
        let mut names = self.gallery_names.clone();
        if name.is_empty() {
            names.remove(&key);
        } else {
            names.insert(key, name);
        }
        let result = serde_json::to_vec_pretty(&names)
            .map_err(|e| e.to_string())
            .and_then(|data| {
                let temporary = self.gallery_names_path.with_extension("json.tmp");
                std::fs::write(&temporary, data).map_err(|e| e.to_string())?;
                std::fs::rename(temporary, &self.gallery_names_path).map_err(|e| e.to_string())
            });
        if let Err(error) = result {
            self.message = Some(format!("Could not save gallery name: {error}"));
            cx.notify();
            return;
        }
        self.gallery_names = names;
        self.gallery_edit_key = None;
        self.rebuild_gallery_groups();
        cx.notify();
    }
}
