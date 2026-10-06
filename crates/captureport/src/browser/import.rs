use crate::*;

impl Browser {
    pub(crate) fn import_selected(
        &mut self,
        _: &ImportSelected,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.importing || self.planning || self.marking_imported {
            return;
        }
        if self.scanning {
            self.message = Some("Wait for the source scan to finish before planning".into());
            cx.notify();
            return;
        }
        if self.plan.is_none() || self.last_import_result.is_some() {
            self.start_plan(cx);
            return;
        }
        if self.page != Page::Preview {
            self.page = Page::Preview;
            cx.notify();
            return;
        }
        let Some(source) = self.source.clone() else {
            return;
        };
        let plan = self.plan.clone().expect("plan checked above");
        if plan.items.iter().any(|i| blocks_import(i.status)) {
            self.message = Some("Import preview contains blocked destinations".into());
            cx.notify();
            return;
        }
        self.plan = Some(plan.clone());
        self.last_import_result = None;
        self.deletion_armed = false;
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        self.import_cancellation = Some(cancel.clone());
        self.importing = true;
        self.progress = None;
        self.message = None;
        let cat = self.catalog.clone();
        let sid = self.catalog_source_id;
        let media_ids = self.catalog_media_ids.clone();
        let generation = self.state.generation;
        thread::spawn(move || {
            run_import(source, plan, cancel, cat, sid, media_ids, generation, tx)
        });
        cx.notify();
    }
    pub(crate) fn start_plan(&mut self, cx: &mut Context<Self>) {
        if self.importing || self.marking_imported {
            return;
        }
        let Some(source) = self.source.clone() else {
            self.message = Some("Open a source first".into());
            cx.notify();
            return;
        };
        let inputs: Vec<_> = self
            .state
            .items()
            .filter(|item| self.state.is_selected(item.id))
            .filter(|item| {
                self.preset
                    .media_rules
                    .classify(&item.source_path)
                    .is_some()
            })
            .cloned()
            .map(|item| PlanInput {
                capture_time: capture_time(&item),
                session_name: self.gallery_item_names.get(&item.id).cloned(),
                item,
            })
            .collect();
        if inputs.is_empty() {
            self.message = Some("Select at least one item".into());
            cx.notify();
            return;
        }
        let preset = self.preset.clone();
        let explicit_members = self.explicit_bundle_selection.clone();
        self.last_import_result = None;
        self.plan_revision = self.plan_revision.wrapping_add(1);
        let revision = self.plan_revision;
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        self.planning = true;
        self.message = Some("Building import preview…".into());
        thread::spawn(move || {
            let plan = ImportPlanner::build_with_explicit_members(
                source.as_ref(),
                inputs,
                &preset,
                &explicit_members,
            );
            let _ = tx.send(WorkMessage::Plan(revision, plan));
        });
        cx.notify();
    }
    pub(crate) fn cancel_import(
        &mut self,
        _: &CancelImport,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(t) = &self.import_cancellation {
            t.cancel()
        }
        self.message = Some("Cancelling import…".into());
        cx.notify()
    }
    pub(crate) fn history(&mut self, _: &ShowHistory, _: &mut Window, cx: &mut Context<Self>) {
        let cat = self.catalog.clone();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let result = cat.list_sessions(50).and_then(|sessions| {
                sessions
                    .into_iter()
                    .map(|session| {
                        let detail = cat.session_detail(session.id)?;
                        let source = cat.source_by_id(session.source_id)?;
                        let source_name = source
                            .and_then(|source| source.identity.alias.or(source.identity.model))
                            .unwrap_or_else(|| format!("Source {}", session.source_id));
                        Ok(HistoryEntry {
                            detail,
                            source_name,
                        })
                    })
                    .collect::<Result<Vec<_>, captureport_catalog::CatalogError>>()
            });
            match result {
                Ok(entries) => {
                    let _ = tx.send(WorkMessage::History(entries));
                }
                Err(error) => {
                    let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
                }
            }
        });
        cx.notify()
    }
    pub(crate) fn inspect_session(&mut self, id: i64, cx: &mut Context<Self>) {
        self.session_detail = self
            .history
            .iter()
            .find(|entry| entry.detail.session.id == id)
            .map(|entry| entry.detail.clone());
        cx.notify();
    }
    pub(crate) fn show_recovery(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Recovery;
        cx.notify();
    }
    pub(crate) fn resume_from_current_source(&mut self, cx: &mut Context<Self>) {
        let matching = self.catalog_source_id.is_some_and(|id| {
            self.incomplete_sessions
                .iter()
                .any(|session| session.session.source_id == id)
        });
        if !matching || self.scanning {
            self.message = Some(
                "Reconnect the source and finish scanning before building a recovery preview"
                    .into(),
            );
            cx.notify();
            return;
        }
        let ids = self.state.items().map(|item| item.id).collect::<Vec<_>>();
        for id in ids {
            self.state.select(id, false);
        }
        let new_ids = self
            .state
            .items()
            .filter(|item| item.import_status == captureport_core::ImportStatus::New)
            .map(|item| item.id)
            .collect::<Vec<_>>();
        for id in new_ids {
            self.state.select(id, true);
        }
        self.invalidate_plan();
        self.start_plan(cx);
    }
    /// The sources a confirmed deletion would actually remove: every required
    /// copy reached its exact planned destination. Shared by the confirmation
    /// text and the importer so the stated scope cannot drift from the scope.
    pub(crate) fn arm_delete_sources(&mut self, cx: &mut Context<Self>) {
        self.deletion_armed = true;
        self.message = Some(match self.deletion_scope() {
            Some(scope) => format!(
                "{scope}. Deleting originals cannot be undone; only files whose required copies verified are removed"
            ),
            None => "Review the verified copies, then confirm source deletion separately".into(),
        });
        cx.notify();
    }
    pub(crate) fn cancel_delete_sources(&mut self, cx: &mut Context<Self>) {
        self.deletion_armed = false;
        self.message = Some("Original deletion cancelled".into());
        cx.notify();
    }
    /// "Delete 47 originals (2.1 GB) from Card · X", or `None` when the scope is
    /// not yet knowable. Never claims a scope it cannot justify.
    pub(crate) fn deletion_scope(&self) -> Option<String> {
        let plan = self.plan.as_ref()?;
        let result = self.last_import_result.as_ref()?;
        let deletable = deletable_sources(plan, result, true);
        if deletable.is_empty() {
            return None;
        }
        let bytes: u64 = deletable.iter().map(|item| item.expected_size).sum();
        let source = self
            .source_alias
            .clone()
            .or_else(|| {
                self.state
                    .source
                    .as_ref()
                    .and_then(|source| source.display_name.clone())
            })
            .unwrap_or_else(|| "this source".into());
        Some(format!(
            "Delete {} original(s) ({}) from {source}",
            deletable.len(),
            format_size(bytes)
        ))
    }
    pub(crate) fn delete_sources(&mut self, cx: &mut Context<Self>) {
        if !self.deletion_armed {
            self.arm_delete_sources(cx);
            return;
        }
        let (Some(root), Some(plan), Some(result)) = (
            self.filesystem_root.clone(),
            self.plan.clone(),
            self.last_import_result.clone(),
        ) else {
            self.message =
                Some("A verified filesystem import is required before deleting originals".into());
            cx.notify();
            return;
        };
        self.deletion_armed = false;
        self.last_import_result = None;
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let mut remover = VerifiedFilesystemRemover::new(root, &plan);
            let report = captureport_ingest::delete_verified_sources(
                &mut remover,
                &plan.items,
                &result,
                captureport_ingest::DeletionOptions {
                    confirmed: true,
                    require_all_destinations: true,
                },
            );
            let _ = tx.send(WorkMessage::Done(Ok(format!(
                "Deleted {} verified original(s); {} refused; {} failed",
                report.deleted.len(),
                report.refused.len(),
                report.failed.len()
            ))));
        });
        cx.notify();
    }
    pub(crate) fn cancel_clean_partial(&mut self, cx: &mut Context<Self>) {
        self.armed_partial = None;
        self.message = Some("Incomplete file kept".into());
        cx.notify();
    }
    pub(crate) fn clean_partial(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(partial) = self.partial_files.get(index).cloned() else {
            return;
        };
        if self.armed_partial != Some(index) {
            // Deleting an incomplete file is permanent, so the first click only
            // says which file is about to go.
            self.armed_partial = Some(index);
            self.message = Some(format!(
                "Confirm deleting {} ({}) — this cannot be undone",
                partial.path.display(),
                format_size(partial.size)
            ));
            cx.notify();
            return;
        }
        self.armed_partial = None;
        let roots = [
            self.preset.photo.root.clone(),
            self.preset.video.root.clone(),
        ];
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let result = roots
                .iter()
                .find(|root| partial.path.starts_with(root))
                .map(|root| captureport_ingest::cleanup_partial(root, &partial.path));
            match result {
                Some(Ok(())) => {
                    let _ = tx.send(WorkMessage::PartialCleaned(partial.path));
                }
                Some(Err(error)) => {
                    let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
                }
                None => {
                    let _ = tx.send(WorkMessage::Done(Err(
                        "Partial is outside configured destinations".into(),
                    )));
                }
            }
        });
        cx.notify();
    }
    pub(crate) fn reconcile(
        &mut self,
        _: &ReconcileLibrary,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.importing || self.reconcile_cancellation.is_some() {
            self.message = Some("Finish the active import or reconciliation first".into());
            cx.notify();
            return;
        }
        let roots = vec![
            self.preset.photo.root.clone(),
            self.preset.video.root.clone(),
        ];
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        let catalog = self.catalog.clone();
        let cancel = CancellationToken::new();
        self.reconcile_cancellation = Some(cancel.clone());
        thread::spawn(move || {
            let result = run_reconcile(&roots, &catalog, &cancel);
            let _ = tx.send(WorkMessage::ReconcileDone(result));
        });
        cx.notify()
    }
    pub(crate) fn cancel_reconcile(
        &mut self,
        _: &CancelReconcile,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(cancel) = &self.reconcile_cancellation {
            cancel.cancel();
            self.message = Some("Stopping library scan…".into());
            cx.notify();
        }
    }
    pub(crate) fn clock(&mut self, _: &AdjustClock, _: &mut Window, cx: &mut Context<Self>) {
        self.page = Page::Settings;
        cx.notify()
    }
}
