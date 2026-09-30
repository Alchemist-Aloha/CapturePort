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
pub(crate) type DiscoveryResult = Result<
    (
        Vec<captureport_gphoto::discovery::DiscoveredSource>,
        HashMap<String, String>,
    ),
    String,
>;
