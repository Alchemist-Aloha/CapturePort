use crate::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UnmountedCard {
    pub(crate) path: PathBuf,
    pub(crate) identity: String,
    pub(crate) label: String,
}
/// Removable volumes found by `lsblk`: the ones that still need mounting, plus
/// the device model of those already mounted, keyed by mount point.
#[derive(Default)]
pub(crate) struct RemovableVolumes {
    pub(crate) unmounted: Vec<UnmountedCard>,
    pub(crate) mounted_models: HashMap<PathBuf, String>,
}
#[derive(Deserialize)]
pub(crate) struct BlockListing {
    pub(crate) blockdevices: Vec<BlockDevice>,
}
#[derive(Deserialize)]
pub(crate) struct BlockDevice {
    pub(crate) path: PathBuf,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) rm: bool,
    pub(crate) tran: Option<String>,
    pub(crate) fstype: Option<String>,
    pub(crate) mountpoint: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) serial: Option<String>,
}
pub(crate) fn removable_volumes() -> Result<RemovableVolumes, String> {
    let output = Command::new("lsblk")
        .args([
            "-J",
            "-o",
            "PATH,TYPE,RM,TRAN,FSTYPE,MOUNTPOINT,LABEL,MODEL,SERIAL",
        ])
        .output()
        .map_err(|e| format!("lsblk: {e}"))?;
    if !output.status.success() {
        return Err("lsblk failed".into());
    }
    let listing: BlockListing =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    Ok(cards_from_blocks(&listing.blockdevices))
}
pub(crate) fn cards_from_blocks(blocks: &[BlockDevice]) -> RemovableVolumes {
    let mut volumes = RemovableVolumes::default();
    let mut parent: Option<&BlockDevice> = None;
    for block in blocks {
        if block.kind == "disk" {
            parent = (block.rm || block.tran.as_deref() == Some("usb")).then_some(block);
        }
        let drive = (block.kind == "disk" || block.kind == "part")
            .then_some(parent)
            .flatten();
        if let Some(drive) = drive
            && block
                .fstype
                .as_deref()
                .is_some_and(|fs| !fs.is_empty() && fs != "crypto_LUKS" && fs != "swap")
        {
            if let Some(mountpoint) = &block.mountpoint {
                volumes.mounted_models.insert(
                    PathBuf::from(mountpoint),
                    drive
                        .model
                        .clone()
                        .unwrap_or_else(|| drive.path.display().to_string()),
                );
            } else {
                volumes.unmounted.push(UnmountedCard {
                    path: block.path.clone(),
                    identity: format!(
                        "{}:{}:{}",
                        drive.path.display(),
                        drive.serial.as_deref().unwrap_or(""),
                        block.path.display()
                    ),
                    label: block
                        .label
                        .clone()
                        .or_else(|| drive.model.clone())
                        .unwrap_or_else(|| block.path.display().to_string()),
                });
            }
        }
    }
    volumes
}
/// Physical presence is sampled independently of libgphoto2: a detection error
/// (or a camera busy importing) is not evidence that a device was unplugged.
pub(crate) struct DevicePresence {
    pub(crate) sampled_at: std::time::Instant,
    pub(crate) usb_ports: Option<HashSet<String>>,
}
impl DevicePresence {
    pub(crate) fn sample() -> Self {
        let sampled_at = std::time::Instant::now();
        let usb_ports = (|| -> std::io::Result<HashSet<String>> {
            let mut ports = HashSet::new();
            for bus in std::fs::read_dir("/dev/bus/usb")? {
                let bus = bus?;
                for device in std::fs::read_dir(bus.path())? {
                    ports.insert(format!(
                        "usb:{},{}",
                        bus.file_name().to_string_lossy(),
                        device?.file_name().to_string_lossy()
                    ));
                }
            }
            Ok(ports)
        })()
        .ok();
        Self {
            sampled_at,
            usb_ports,
        }
    }
}

pub(crate) struct SourceWatch {
    pub(crate) selected_at: std::time::Instant,
    pub(crate) connection: DeviceConnection,
}
pub(crate) enum DeviceConnection {
    Usb(String),
    Mount(PathBuf),
}
impl SourceWatch {
    pub(crate) fn new(
        request: &SourceRequest,
        mounted_models: &HashMap<PathBuf, String>,
    ) -> Option<Self> {
        let connection = match request {
            SourceRequest::Camera(camera) if camera.port.starts_with("usb:") => {
                DeviceConnection::Usb(camera.port.clone())
            }
            SourceRequest::Filesystem(root) => {
                let mount = mounted_models
                    .keys()
                    .filter(|mount| root.starts_with(mount))
                    .max_by_key(|mount| mount.components().count())?;
                DeviceConnection::Mount(mount.clone())
            }
            _ => return None,
        };
        Some(Self {
            selected_at: std::time::Instant::now(),
            connection,
        })
    }
    pub(crate) fn disconnected(
        &self,
        presence: &DevicePresence,
        volumes: Option<&RemovableVolumes>,
    ) -> bool {
        // A poll already in flight when the user selected a source cannot
        // invalidate that newer selection.
        if presence.sampled_at < self.selected_at {
            return false;
        }
        match &self.connection {
            DeviceConnection::Usb(port) => presence
                .usb_ports
                .as_ref()
                .is_some_and(|ports| !ports.contains(port)),
            DeviceConnection::Mount(path) => {
                volumes.is_some_and(|volumes| !volumes.mounted_models.contains_key(path))
            }
        }
    }
}

pub(crate) type DiscoveryResult = Result<
    (
        Vec<captureport_gphoto::discovery::DiscoveredSource>,
        HashMap<String, String>,
    ),
    String,
>;

#[cfg(test)]
mod presence_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn usb_disconnect_ignores_old_polls_and_unknown_presence() {
        let selected_at = Instant::now();
        let watch = SourceWatch {
            selected_at,
            connection: DeviceConnection::Usb("usb:001,057".into()),
        };
        let mut presence = DevicePresence {
            sampled_at: selected_at - Duration::from_secs(1),
            usb_ports: Some(HashSet::new()),
        };
        assert!(!watch.disconnected(&presence, None));
        presence.sampled_at = selected_at + Duration::from_secs(1);
        presence.usb_ports = None;
        assert!(!watch.disconnected(&presence, None));
        presence.usb_ports = Some(HashSet::from(["usb:001,057".into()]));
        assert!(!watch.disconnected(&presence, None));
        presence.usb_ports = Some(HashSet::from(["usb:001,058".into()]));
        assert!(watch.disconnected(&presence, None));
        // A reconnect is a new selection, not a revival from an old poll.
        let reconnected = SourceWatch {
            selected_at: presence.sampled_at + Duration::from_secs(1),
            connection: DeviceConnection::Usb("usb:001,058".into()),
        };
        assert!(!reconnected.disconnected(&presence, None));
    }

    #[test]
    fn mounted_subfolder_is_watched_but_local_folders_are_not() {
        let root = PathBuf::from("/run/media/user/Card");
        let mut volumes = RemovableVolumes::default();
        volumes.mounted_models.insert(root.clone(), "Camera".into());
        let watch = SourceWatch::new(
            &SourceRequest::Filesystem(root.join("DCIM")),
            &volumes.mounted_models,
        )
        .unwrap();
        let presence = DevicePresence {
            sampled_at: watch.selected_at + Duration::from_secs(1),
            usb_ports: None,
        };
        assert!(!watch.disconnected(&presence, Some(&volumes)));
        assert!(!watch.disconnected(&presence, None));
        volumes.mounted_models.clear();
        assert!(watch.disconnected(&presence, Some(&volumes)));
        for local in ["/home/user/Pictures", "/run/media/user/Card-other"] {
            assert!(
                SourceWatch::new(
                    &SourceRequest::Filesystem(local.into()),
                    &HashMap::from([(root.clone(), "Camera".into())]),
                )
                .is_none()
            );
        }
    }
}
