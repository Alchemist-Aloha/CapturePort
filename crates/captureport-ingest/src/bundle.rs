//! Deterministic grouping of source files into logical media bundles.
//!
//! Bundling is deliberately a pure operation.  It never removes, renames, or
//! otherwise changes a source file; callers apply `BundlePolicy` when making
//! an import plan.

use crate::{BundlePolicy, PlanInput};
use captureport_core::{BundleId, MediaId, MediaItem, MediaType};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleType {
    RawJpeg,
    VideoSidecar,
    Single,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaBundle {
    pub id: BundleId,
    pub primary: MediaId,
    pub members: Vec<MediaId>,
    pub bundle_type: BundleType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleResult {
    pub items: Vec<MediaItem>,
    pub bundles: Vec<MediaBundle>,
}

/// Group files by parent directory and filename stem.
pub fn group_media(items: &[MediaItem]) -> BundleResult {
    let mut groups: BTreeMap<(String, String), Vec<&MediaItem>> = BTreeMap::new();
    for item in items {
        let path = std::path::Path::new(&item.source_path);
        let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        groups
            .entry((parent.to_owned(), stem.to_ascii_lowercase()))
            .or_default()
            .push(item);
    }

    let mut member_groups: Vec<(BundleType, Vec<&MediaItem>)> = Vec::new();
    for (_, members) in groups {
        let mut raw = Vec::new();
        let mut jpeg = Vec::new();
        let mut video = Vec::new();
        let mut sidecar = Vec::new();
        let mut other = Vec::new();
        for member in members {
            match member.media_type {
                MediaType::Raw => raw.push(member),
                MediaType::Jpeg => jpeg.push(member),
                MediaType::Video => video.push(member),
                MediaType::Sidecar => sidecar.push(member),
                _ => other.push(member),
            }
        }
        while !raw.is_empty() && !jpeg.is_empty() {
            member_groups.push((
                BundleType::RawJpeg,
                vec![
                    raw.pop().expect("checked non-empty"),
                    jpeg.pop().expect("checked non-empty"),
                ],
            ));
        }
        while !video.is_empty() && !sidecar.is_empty() {
            member_groups.push((
                BundleType::VideoSidecar,
                vec![
                    video.pop().expect("checked non-empty"),
                    sidecar.pop().expect("checked non-empty"),
                ],
            ));
        }
        for member in raw
            .into_iter()
            .chain(jpeg)
            .chain(video)
            .chain(sidecar)
            .chain(other)
        {
            member_groups.push((BundleType::Single, vec![member]));
        }
    }
    member_groups.sort_by_key(|(_, members)| members[0].source_path.to_ascii_lowercase());

    let mut bundles = Vec::new();
    let mut ids = BTreeMap::new();
    for (index, (bundle_type, members)) in member_groups.into_iter().enumerate() {
        let bundle_id = BundleId(index as u64 + 1);
        let member_ids: Vec<_> = members.iter().map(|item| item.id).collect();
        for id in &member_ids {
            ids.insert(*id, bundle_id);
        }
        bundles.push(MediaBundle {
            id: bundle_id,
            primary: member_ids[0],
            members: member_ids,
            bundle_type,
        });
    }
    let items = items
        .iter()
        .cloned()
        .map(|mut item| {
            item.bundle_id = ids.get(&item.id).copied();
            item
        })
        .collect();
    BundleResult { items, bundles }
}

pub fn bundle_media(items: &[MediaItem]) -> BundleResult {
    group_media(items)
}

/// Apply a preset to paired captures while retaining standalone files.
pub fn apply_bundle_policy(inputs: Vec<PlanInput>, policy: BundlePolicy) -> Vec<PlanInput> {
    apply_bundle_policy_with_overrides(inputs, policy, &HashSet::new())
}

/// Explicitly selected members take precedence over the preset's bundle policy.
pub fn apply_bundle_policy_with_overrides(
    inputs: Vec<PlanInput>,
    policy: BundlePolicy,
    explicit_members: &HashSet<MediaId>,
) -> Vec<PlanInput> {
    if policy == BundlePolicy::KeepAll {
        return inputs;
    }
    let media: Vec<_> = inputs.iter().map(|entry| entry.item.clone()).collect();
    let grouped = group_media(&media);
    let excluded: std::collections::HashSet<_> = grouped
        .bundles
        .iter()
        .filter(|bundle| bundle.bundle_type == BundleType::RawJpeg)
        .flat_map(|bundle| bundle.members.iter().copied())
        .filter(|id| {
            if explicit_members.contains(id) {
                return false;
            }
            media
                .iter()
                .find(|item| item.id == *id)
                .is_some_and(|item| match policy {
                    BundlePolicy::RawOnly => item.media_type == MediaType::Jpeg,
                    BundlePolicy::JpegOnly => item.media_type == MediaType::Raw,
                    BundlePolicy::KeepAll => false,
                })
        })
        .collect();
    inputs
        .into_iter()
        .filter(|entry| !excluded.contains(&entry.item.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use captureport_core::SourceId;

    fn item(id: u64, path: &str) -> MediaItem {
        MediaItem::new(MediaId(id), SourceId(1), path, 1)
    }

    #[test]
    fn pairs_only_same_parent_and_stem() {
        let result = group_media(&[
            item(1, "DCIM/a/DSC1234.ARW"),
            item(2, "DCIM/a/DSC1234.JPG"),
            item(3, "DCIM/b/DSC1234.JPG"),
        ]);
        assert_eq!(result.bundles.len(), 2);
        assert_eq!(result.bundles[0].bundle_type, BundleType::RawJpeg);
        assert_eq!(result.bundles[0].members, vec![MediaId(1), MediaId(2)]);
        assert_eq!(
            result
                .items
                .iter()
                .find(|i| i.id == MediaId(1))
                .unwrap()
                .bundle_id,
            Some(BundleId(1))
        );
    }

    #[test]
    fn video_sidecar_is_one_selectable_bundle() {
        let result = group_media(&[item(1, "C0001.MP4"), item(2, "C0001.XML")]);
        assert_eq!(result.bundles[0].bundle_type, BundleType::VideoSidecar);
        assert_eq!(result.bundles[0].primary, MediaId(1));
    }

    #[test]
    fn raw_only_keeps_single_jpeg_but_drops_paired_jpeg() {
        use chrono::TimeZone;
        let time = chrono::FixedOffset::east_opt(0)
            .unwrap()
            .timestamp_opt(0, 0)
            .unwrap();
        let inputs = [
            item(1, "pair.ARW"),
            item(2, "pair.JPG"),
            item(3, "single.JPG"),
        ]
        .into_iter()
        .map(|item| PlanInput {
            item,
            capture_time: time,
            session_name: None,
        })
        .collect();
        let result = apply_bundle_policy(inputs, BundlePolicy::RawOnly);
        assert_eq!(
            result.iter().map(|entry| entry.item.id).collect::<Vec<_>>(),
            vec![MediaId(1), MediaId(3)]
        );
    }

    #[test]
    fn explicit_group_selection_overrides_policy_only_for_chosen_member() {
        use chrono::TimeZone;
        let time = chrono::FixedOffset::east_opt(0)
            .unwrap()
            .timestamp_opt(0, 0)
            .unwrap();
        let inputs = [
            item(1, "first.ARW"),
            item(2, "first.JPG"),
            item(3, "second.ARW"),
            item(4, "second.JPG"),
        ]
        .into_iter()
        .map(|item| PlanInput {
            item,
            capture_time: time,
            session_name: None,
        })
        .collect();
        let result = apply_bundle_policy_with_overrides(
            inputs,
            BundlePolicy::RawOnly,
            &HashSet::from([MediaId(2)]),
        );
        assert_eq!(
            result.iter().map(|entry| entry.item.id).collect::<Vec<_>>(),
            vec![MediaId(1), MediaId(2), MediaId(3)]
        );
    }
}
