use crate::*;

impl Browser {
    fn saved_presets_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        use saved_presets::PresetAction;
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let selected = self
            .saved_presets
            .iter()
            .find(|preset| Some(preset.id) == self.selected_preset);
        let mut controls = div().flex().flex_col().gap(px(spacing::CONTROL_GAP))
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Saved presets include every setting below: import rules, browsing, appearance, and the source-alias field.")));
        if self.saved_presets.is_empty() && !self.preset_busy {
            controls = controls.child(div().text_sm().text_color(p.muted).child(i18n::tr(
                "No saved presets yet. Enter a name and create one from your current settings.",
            )));
        } else {
            controls = controls.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .children(self.saved_presets.iter().map(|preset| {
                        let id = preset.id;
                        div().id(("saved-preset", id as u64)).child(chip(
                            preset.name.clone(),
                            self.selected_preset == Some(id),
                            p,
                            cx.listener(move |t, _, _, c| t.load_saved_preset(id, c)),
                        ))
                    })),
            );
        }
        controls = controls.child(settings_field(
            "Preset name",
            self.settings.preset_name.clone(),
            p,
            cx,
        ));
        if !self.preset_busy {
            let mut actions = div()
                .flex()
                .flex_wrap()
                .gap(px(spacing::CONTROL_GAP))
                .child(button(
                    Icon::AddToList,
                    "Create preset",
                    p,
                    cx.listener(|t, _, w, c| t.manage_preset(PresetAction::Create, w, c)),
                ));
            if selected.is_some() {
                actions = actions
                    .child(button(
                        Icon::Rename,
                        "Rename",
                        p,
                        cx.listener(|t, _, w, c| t.manage_preset(PresetAction::Rename, w, c)),
                    ))
                    .child(button(
                        Icon::Confirm,
                        "Overwrite",
                        p,
                        cx.listener(|t, _, w, c| t.manage_preset(PresetAction::Overwrite, w, c)),
                    ))
                    .child(button(
                        Icon::Delete,
                        "Delete preset",
                        p,
                        cx.listener(|t, _, w, c| t.manage_preset(PresetAction::Delete, w, c)),
                    ));
            }
            controls = controls.child(actions);
        }
        if let (Some(action), Some(selected)) = (self.preset_confirmation, selected) {
            let deleting = action == PresetAction::Delete;
            controls = controls
                .child(div().text_sm().child(if deleting {
                    i18n::format("Delete ‘{0}’? This removes only the saved preset, not your current settings or imported files.", std::slice::from_ref(&selected.name))
                } else {
                    i18n::format("Overwrite ‘{0}’ with all current settings? Its previous saved settings will be replaced.", std::slice::from_ref(&selected.name))
                }))
                .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                    .child(danger_button(if deleting { Icon::Delete } else { Icon::Confirm },
                        if deleting { "Confirm delete" } else { "Confirm overwrite" }, p,
                        cx.listener(move |t, _, w, c| t.manage_preset(action, w, c))))
                    .child(button(Icon::Cancel, "Cancel", p, cx.listener(|t, _, _, c| {
                        t.preset_confirmation = None;
                        c.notify();
                    }))));
        }
        if let Some(message) = &self.preset_message {
            controls = controls.child(div().text_sm().text_color(p.muted).child(message.clone()));
        }
        if self.preset_busy {
            controls = controls.child(
                div()
                    .text_sm()
                    .text_color(p.muted)
                    .child(i18n::tr("Loading or saving presets…")),
            );
        }
        controls.into_any_element()
    }

    pub(crate) fn settings_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let tab = self.settings_tab;
        let timezone = parse_utc_offset(&self.settings.timezone_seconds.read(cx).value())
            .ok()
            .flatten();
        let clock = parse_clock_offset(&self.settings.clock_seconds.read(cx).value()).ok();
        let panel = div()
            .id("settings-panel")
            .w_full().max_w(px(820.))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p(px(spacing::CONTENT))
            .flex()
            .flex_col()
            .gap(px(spacing::CONTENT))
            .overflow_y_scroll()
            .track_scroll(&self.scrollbars.settings.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
            .child(
                div()
                    .font_family(DISPLAY_FONT)
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(i18n::tr("Settings")),
            )
            .child(div().text_sm().text_color(p.muted)
                .child(i18n::tr("Set destinations and naming, then review the safety and media rules. Appearance and source alias save separately.")))
            .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP)).pb(px(spacing::TIGHT))
                .border_b_1().border_color(p.border)
                .children(SettingsTab::ALL.map(|(choice, label)| div()
                    .id(label)
                    .child(chip(i18n::tr(label), tab == choice, p,
                        cx.listener(move |t, _, _, c| {
                            t.settings_tab = choice;
                            t.timezone_menu_open = false;
                            c.notify()
                        }))))))
            .when(tab == SettingsTab::Presets, |d| d
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Built-in starting points replace import rules. Choose a saved preset to restore all settings.")))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(chip(i18n::tr("Everyday"),
                        self.selected_preset.is_none() && self.preset.name == "Everyday", p,
                        cx.listener(|t, _, _, c| t.choose_preset(false, c)),
                    ))
                    .child(chip(i18n::tr("Organized"),
                        self.selected_preset.is_none() && self.preset.name == "Organized", p,
                        cx.listener(|t, _, _, c| t.choose_preset(true, c)),
                    )),
            )
            .child(self.saved_presets_controls(cx)))
            .when(tab == SettingsTab::Destinations, |d| d
            .child(settings_path_field(
                "Photo destination",
                self.settings.photo_root.clone(),
                p,
                cx,
            ))
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Destination roots are absolute paths, for example /home/you/Pictures. Folder templates create subfolders inside these roots.")))
            .child(settings_template_field(
                "Photo folder template",
                self.settings.photo_folder.clone(),
                false,
                false,
                self.template_segment_target.as_ref(),
                p,
                cx,
            ))
            .child(settings_path_field(
                "Video destination",
                self.settings.video_root.clone(),
                p,
                cx,
            ))
            .child(settings_template_field(
                "Video folder template",
                self.settings.video_folder.clone(),
                false,
                true,
                self.template_segment_target.as_ref(),
                p,
                cx,
            ))
            .child(settings_section("File naming", p))
            .child(settings_template_field(
                "Filename template",
                self.settings.filename.clone(),
                true,
                false,
                self.template_segment_target.as_ref(),
                p,
                cx,
            )))
            .when(tab == SettingsTab::Safety, |d| d
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Verification checks copies; collision controls what happens when a destination file already exists.")))
            .child(settings_choices("Verification",
                [VerificationMode::Fast, VerificationMode::Standard, VerificationMode::Strict].map(|mode| div()
                    .id(("verification", mode as u64))
                    .child(chip(i18n::tr(&format!("{mode:?}")), self.preset.verification == mode, p,
                        cx.listener(move |t, _, _, c| t.set_import_rule(|preset| preset.verification = mode, c))))
                    .into_any_element()), Some(match self.preset.verification {
                    VerificationMode::Fast => "Checks only the file size. Quickest, least certain.",
                    VerificationMode::Standard => "Checks size and a sampled fingerprint of each copy. Recommended.",
                    VerificationMode::Strict => "Hashes every byte of source and copy. Slowest; use before deleting source media.",
                }), p))
            .child(settings_choices("Collision",
                [CollisionPolicy::Stop, CollisionPolicy::Skip, CollisionPolicy::UniqueSuffix, CollisionPolicy::VerifyIdentical].map(|policy| div()
                    .id(("collision", policy as u64))
                    .child(chip(i18n::tr(&format!("{policy:?}")), self.preset.collision == policy, p,
                        cx.listener(move |t, _, _, c| t.set_import_rule(|preset| preset.collision = policy, c))))
                    .into_any_element()), Some(match self.preset.collision {
                    CollisionPolicy::Stop => "A different file already at the destination blocks the import until you resolve it.",
                    CollisionPolicy::Skip => "A different file already at the destination is left alone and that item is skipped.",
                    CollisionPolicy::UniqueSuffix => "A different file already at the destination is kept; the new copy gets a numbered suffix.",
                    CollisionPolicy::VerifyIdentical => "An identical existing file counts as imported; a different one blocks the import.",
                }), p))
            .child(settings_choices("Bundles",
                [BundlePolicy::KeepAll, BundlePolicy::RawOnly, BundlePolicy::JpegOnly].map(|policy| div()
                    .id(("bundle-policy", policy as u64))
                    .child(chip(i18n::tr(&format!("{policy:?}")), self.preset.bundle_policy == policy, p,
                        cx.listener(move |t, _, _, c| t.set_import_rule(|preset| preset.bundle_policy = policy, c))))
                    .into_any_element()), Some(match self.preset.bundle_policy {
                    BundlePolicy::KeepAll => "Imports every file of each RAW+JPEG pair.",
                    BundlePolicy::RawOnly => "Imports only the RAW file of each RAW+JPEG pair.",
                    BundlePolicy::JpegOnly => "Imports only the JPEG of each RAW+JPEG pair.",
                }), p))
            .child({
                // Choosing Time gap uses the threshold typed under Capture time.
                let gap = self.settings.gap_minutes.read(cx).value().trim().parse().unwrap_or(30);
                let choices = [Grouping::None, Grouping::Day, Grouping::Week, Grouping::Month, Grouping::Year,
                    Grouping::TimeGap { threshold_minutes: gap }];
                settings_choices("Grouping", choices.into_iter().enumerate().map(|(index, grouping)| {
                    let label = match grouping {
                        Grouping::TimeGap { threshold_minutes } => i18n::format("Time gap: {0} min", &[threshold_minutes.to_string()]),
                        _ => i18n::tr(&format!("{grouping:?}")),
                    };
                    let active = std::mem::discriminant(&self.preset.grouping) == std::mem::discriminant(&grouping);
                    div().id(("grouping", index as u64))
                        .child(chip(label, active, p, cx.listener(move |t, _, _, c| {
                            let grouping = grouping.clone();
                            t.set_import_rule(|preset| preset.grouping = grouping, c)
                        })))
                        .into_any_element()
                }), None, p)
            }))
            .when(tab == SettingsTab::Discovery, |d| d
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("These rules determine which files appear when you scan a source.")))
            .child(settings_field(
                "Additional photo extensions (comma separated)",
                self.settings.include_photo.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Additional video extensions (comma separated)",
                self.settings.include_video.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Excluded extensions (comma separated)",
                self.settings.exclude_extensions.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Excluded folders (comma separated)",
                self.settings.exclude_folders.clone(),
                p,
                cx,
            ))
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Use folder names or path fragments, for example Screenshots, DCIM/Private. Matches are case-sensitive and include subfolders. Rescan to update the browser.")))
            .child(settings_choices("Ignored media types",
                [("raw", "RAW"), ("photo", "Photos"), ("video", "Videos"), ("sidecar", "Sidecars"), ("unknown", "Unknown files")]
                    .map(|(kind, label)| {
                        let ignored = self.preset.media_rules.ignore.iter()
                            .any(|entry| entry.trim().eq_ignore_ascii_case(kind));
                        div().id(kind).child(chip(i18n::tr(label), ignored, p,
                            cx.listener(move |t, _, _, c| t.set_import_rule(|preset| {
                                let ignore = &mut preset.media_rules.ignore;
                                if ignored {
                                    ignore.retain(|entry| !entry.trim().eq_ignore_ascii_case(kind));
                                } else {
                                    ignore.push(kind.into());
                                }
                            }, c))))
                            .into_any_element()
                    }), None, p))
            .child(settings_section("Browsing", p))
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Choose whether a RAW and its JPEG sidecar appear as one item or as separate files.")))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(chip(i18n::tr("Merge RAW+JPEG"),
                        self.ui.merge_raw_jpeg, p,
                        cx.listener(|t, _, _, c| t.set_merge_raw_jpeg(true, c)),
                    ))
                    .child(chip(i18n::tr("Separate files"),
                        !self.ui.merge_raw_jpeg, p,
                        cx.listener(|t, _, _, c| t.set_merge_raw_jpeg(false, c)),
                    )),
            )
            .child(settings_section("Capture time", p))
            .child(settings_field(
                "Clock correction (+H:MM:SS added to camera time; negative if the camera runs fast)",
                self.settings.clock_seconds.clone(),
                p,
                cx,
            ))
            .children(clock.map(|seconds| {
                let example = chrono::NaiveTime::from_hms_opt(12, 0, 0).expect("noon")
                    + chrono::Duration::seconds(seconds);
                div().text_xs().text_color(p.muted).child(i18n::format(
                    "Example: camera time 12:00:00 is imported as {0}",
                    &[example.format("%H:%M:%S").to_string()],
                ))
            }))
            .child(settings_field(
                "Time-gap session threshold (minutes)",
                self.settings.gap_minutes.clone(),
                p,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::TIGHT))
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.muted)
                            .child(i18n::tr("Timezone applied to capture times")),
                    )
                    .child(button_with_id(
                        "timezone-picker",
                        Icon::Time,
                        i18n::format("Timezone: {0}", &[timezone_label(timezone)]), p,
                        cx.listener(|t, _, _, c| {
                            t.timezone_menu_open = !t.timezone_menu_open;
                            c.notify()
                        }),
                    ))
                    .child(if self.timezone_menu_open {
                        div().relative().flex_shrink_0().max_h(px(200.)).flex().flex_col()
                            .child(div()
                            .id("timezone-menu")
                            .max_h(px(200.))
                            .overflow_y_scroll()
                            .track_scroll(&self.scrollbars.timezone.handle)
                            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
                            .flex()
                            .flex_col()
                            .gap(px(spacing::TIGHT))
                            .child(timezone_choice(
                                i18n::tr("Capture metadata"),
                                self.settings.timezone_seconds.read(cx).value().trim().is_empty(),
                                u64::MAX,
                                p,
                                cx.listener(|t, _, _, c| t.set_timezone_offset(None, c)),
                            ))
                            .children(TIMEZONE_CHOICES.iter().map(|(label, seconds)| {
                                timezone_choice(
                                    (*label).to_string(),
                                    timezone
                                        == Some(*seconds),
                                    (*seconds + 50_400) as u64,
                                    p,
                                    cx.listener(move |t, _, _, c| {
                                        t.set_timezone_offset(Some(*seconds), c)
                                    }),
                                )
                            })))
                            .child(self.scrollbars.timezone.element(p.border, p.muted, p.accent))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    })
                    .child(settings_field(
                        "Custom offset (for example UTC+08:00; blank keeps metadata timezone)",
                        self.settings.timezone_seconds.clone(),
                        p,
                        cx,
                    )),
            ))
            .when(tab == SettingsTab::Safety, |d| d
            .child(settings_section("Backup copies", p))
            .child(settings_path_field(
                "Backup photo root (blank disables backup)",
                self.settings.backup_photo_root.clone(),
                p,
                cx,
            ))
            .child(settings_path_field(
                "Backup video root (blank disables backup)",
                self.settings.backup_video_root.clone(),
                p,
                cx,
            ))
            .child(settings_choices("Backup required", [(false, "No"), (true, "Yes")].map(|(required, label)| div()
                .id(("backup-required", required as u64))
                .child(chip(i18n::tr(label), self.backup_required_choice == required, p,
                    cx.listener(move |t, _, _, c| t.set_backup_required(required, c))))
                .into_any_element()), None, p)))
            .when(tab == SettingsTab::Discovery, |d| d
            .child(settings_section("Source", p))
            .child(settings_field(
                "Current source alias (blank uses device name)",
                self.settings.source_alias.clone(), p, cx,
            ))
            .child(button(Icon::Confirm, "Save source alias", p,
                cx.listener(|t, _, _, c| t.save_source_alias(c)))))
            .when(tab == SettingsTab::Appearance, |d| d
            .child(settings_section("Language", p))
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Language changes save immediately.")))
            .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                .child(chip(i18n::tr("System language"), self.ui.language.is_none(), p,
                    cx.listener(|t, _, _, c| t.set_language(None, c))))
                .children(i18n::Language::ALL.map(|language| chip(
                    language.native_name(), self.ui.language == Some(language), p,
                    cx.listener(move |t, _, _, c| t.set_language(Some(language), c)),
                ))))
            .child(settings_section("Appearance", p))
            .child(div().text_xs().text_color(p.muted)
                .child(i18n::tr("Color scheme and light or dark mode save immediately.")))
            .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                .children(ColorScheme::ALL.map(|scheme| chip(
                    i18n::tr(scheme.label()), self.ui.scheme == scheme, p,
                    cx.listener(move |t, _, _, c| t.set_scheme(scheme, c)),
                ))))
            .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                .child(chip(i18n::tr("Light"), !self.ui.dark_mode, p,
                    cx.listener(|t, _, _, c| { if t.ui.dark_mode { t.toggle_dark_mode(c); } })))
                .child(chip(i18n::tr("Dark"), self.ui.dark_mode, p,
                    cx.listener(|t, _, _, c| { if !t.ui.dark_mode { t.toggle_dark_mode(c); } })))))
            .when(tab == SettingsTab::Maintenance, |d| d
            .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                .child(if self.reconcile_cancellation.is_some() {
                    button_with_id("maintenance-reconcile", Icon::Cancel, "Cancel library scan", p,
                        cx.listener(|t, _, w, c| t.cancel_reconcile(&CancelReconcile, w, c)))
                        .into_any_element()
                } else {
                    button_with_id("maintenance-reconcile", Icon::Catalog, "Reconcile library", p,
                        cx.listener(|t, _, w, c| t.reconcile(&ReconcileLibrary, w, c)))
                        .into_any_element()
                })
                .child(button_with_id("clear-thumbnails", Icon::Delete,
                    if self.cache_clearing { "Clearing thumbnails…" } else { "Clear thumbnails" }, p,
                    cx.listener(|t, _, _, c| t.clear_thumbnail_cache(c))))))
            .into_any_element();
        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(panel)
                    .child(
                        self.scrollbars
                            .settings
                            .element(p.border, p.muted, p.accent),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(p.border)
                    .px(px(spacing::CONTENT))
                    .py(px(spacing::CONTROL_GAP))
                    .text_sm()
                    .map(|footer| match &self.settings_error {
                        Some(error) => footer
                            .bg(p.danger)
                            .text_color(p.text)
                            .child(i18n::format("Settings: {0}", std::slice::from_ref(error))),
                        None => footer
                            .text_color(p.muted)
                            .child(i18n::tr("Changes save when you leave a field.")),
                    }),
            )
            .into_any_element()
    }
}
