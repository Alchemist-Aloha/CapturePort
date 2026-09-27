//! Poll based device discovery. A higher layer may call `discover` on a timer
//! or from a udev event loop; this crate deliberately does not copy anything.

use crate::{CameraDescriptor, GPhotoError, GPhotoSource};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveredSource {
    Ptp(CameraDescriptor),
    MountedFilesystem {
        path: PathBuf,
        stable_id: String,
        volume_uuid: Option<String>,
        label: Option<String>,
    },
}

impl DiscoveredSource {
    pub fn stable_id(&self) -> &str {
        match self {
            Self::Ptp(c) => &c.stable_id,
            Self::MountedFilesystem { stable_id, .. } => stable_id,
        }
    }
}

/// Returns one entry per stable source. PTP devices win over a matching mount
/// path because libgphoto2 can provide the camera identity and media tree.
pub fn discover() -> Result<Vec<DiscoveredSource>, GPhotoError> {
    let cameras = GPhotoSource::autodetect()?;
    let mut result = cameras
        .into_iter()
        .map(DiscoveredSource::Ptp)
        .collect::<Vec<_>>();
    let camera_usb_addresses = result
        .iter()
        .filter_map(|s| match s {
            DiscoveredSource::Ptp(c) => parse_usb_port(&c.port),
            _ => None,
        })
        .collect::<Vec<_>>();
    for mount in mounted_filesystems() {
        // A gphoto USB port is `usb:BUS,DEVICE`.  Match it against the USB
        // ancestor of the mounted block device, rather than comparing the
        // human-readable mount path with a port string.  If sysfs cannot
        // provide the topology, keep the mount: hiding an unrelated volume
        // is worse than showing a possible duplicate.
        if !mount_usb_address(&mount.device)
            .is_some_and(|address| camera_usb_addresses.contains(&address))
        {
            let stable_id = mount
                .volume_uuid
                .as_ref()
                .map(|uuid| format!("volume:uuid:{uuid}"))
                .unwrap_or_else(|| format!("mount:device:{}", mount.device.display()));
            result.push(DiscoveredSource::MountedFilesystem {
                stable_id,
                path: mount.path,
                volume_uuid: mount.volume_uuid,
                label: mount.label,
            });
        }
    }
    let mut unique = HashMap::new();
    result.retain(|source| unique.insert(source.stable_id().to_owned(), ()).is_none());
    Ok(result)
}

#[derive(Debug)]
struct Mount {
    device: PathBuf,
    path: PathBuf,
    volume_uuid: Option<String>,
    label: Option<String>,
}
fn mounted_filesystems() -> Vec<Mount> {
    let Ok(text) = fs::read_to_string("/proc/self/mounts") else {
        return Vec::new();
    };
    parse_mounts(&text)
        .map(enrich_mount)
        .filter(|m| likely_removable_mount(&m.path))
        .collect()
}
fn parse_mounts(text: &str) -> impl Iterator<Item = Mount> + '_ {
    text.lines().filter_map(parse_mount_line)
}
fn parse_mount_line(line: &str) -> Option<Mount> {
    let mut fields = line.split_whitespace();
    let device = unescape_mount(fields.next()?);
    let path = unescape_mount(fields.next()?);
    let filesystem = fields.next()?;
    if filesystem == "tmpfs"
        || filesystem == "proc"
        || filesystem == "sysfs"
        || filesystem == "devtmpfs"
        || filesystem == "cgroup2"
        || filesystem == "overlay"
    {
        return None;
    }
    let label = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    Some(Mount {
        device,
        path,
        volume_uuid: None,
        label,
    })
}

fn enrich_mount(mut mount: Mount) -> Mount {
    mount.volume_uuid = volume_uuid_for_device(&mount.device);
    mount
}

fn volume_uuid_for_device(device: &Path) -> Option<String> {
    let canonical_device = fs::canonicalize(device).ok()?;
    let entries = fs::read_dir("/dev/disk/by-uuid").ok()?;
    entries.filter_map(Result::ok).find_map(|entry| {
        let target = fs::canonicalize(entry.path()).ok()?;
        (target == canonical_device).then(|| entry.file_name().to_string_lossy().into_owned())
    })
}

fn parse_usb_port(port: &str) -> Option<(u32, u32)> {
    let value = port.strip_prefix("usb:")?;
    let (bus, device) = value.split_once(',')?;
    Some((bus.parse().ok()?, device.parse().ok()?))
}

fn mount_usb_address(device: &Path) -> Option<(u32, u32)> {
    mount_usb_address_from_sysfs(device, Path::new("/sys"))
}

fn mount_usb_address_from_sysfs(device: &Path, sysfs_root: &Path) -> Option<(u32, u32)> {
    let name = device.file_name()?.to_str()?;
    let class_device = sysfs_root.join("class/block").join(name).join("device");
    let mut current = fs::canonicalize(&class_device).ok()?;
    for _ in 0..12 {
        let bus = fs::read_to_string(current.join("busnum")).ok();
        let number = fs::read_to_string(current.join("devnum")).ok();
        if let (Some(bus), Some(number)) = (bus, number) {
            return Some((bus.trim().parse().ok()?, number.trim().parse().ok()?));
        }
        if !current.pop() {
            break;
        }
    }
    None
}
fn unescape_mount(value: &str) -> PathBuf {
    PathBuf::from(
        value
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\134", "\\"),
    )
}
fn likely_removable_mount(path: &Path) -> bool {
    let value = path.to_string_lossy();
    value.starts_with("/media/") || value.starts_with("/mnt/") || value.starts_with("/run/media/")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_escaped_mount_path() {
        let m = parse_mount_line("/dev/sdb1 /run/media/user/My\\040Card vfat rw 0 0").unwrap();
        assert_eq!(m.path, PathBuf::from("/run/media/user/My Card"));
        assert_eq!(m.device, PathBuf::from("/dev/sdb1"));
    }
    #[test]
    fn ignores_system_mounts() {
        assert!(parse_mount_line("proc /proc proc rw 0 0").is_none());
    }
    #[test]
    fn source_ids_are_unique() {
        let a = DiscoveredSource::MountedFilesystem {
            path: "/mnt/a".into(),
            stable_id: "mount:/mnt/a".into(),
            volume_uuid: None,
            label: None,
        };
        assert_eq!(a.stable_id(), "mount:/mnt/a");
    }

    #[test]
    fn parses_gphoto_usb_ports_without_path_heuristics() {
        assert_eq!(parse_usb_port("usb:001,002"), Some((1, 2)));
        assert_eq!(parse_usb_port("usb:7,42"), Some((7, 42)));
        assert_eq!(parse_usb_port("serial:/dev/ttyUSB0"), None);
        assert_eq!(parse_usb_port("usb:broken,2"), None);
    }

    #[test]
    fn resolves_usb_address_from_a_synthetic_sysfs_tree() {
        let root = std::env::temp_dir().join(format!(
            "captureport-gphoto-sysfs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let usb = root.join("devices/usb/1-2");
        fs::create_dir_all(&usb).unwrap();
        fs::write(usb.join("busnum"), "001\n").unwrap();
        fs::write(usb.join("devnum"), "002\n").unwrap();
        let block = root.join("class/block/sdb1");
        fs::create_dir_all(&block).unwrap();
        std::os::unix::fs::symlink(&usb, block.join("device")).unwrap();
        assert_eq!(
            mount_usb_address_from_sysfs(Path::new("/dev/sdb1"), &root),
            Some((1, 2))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
