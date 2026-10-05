use std::os::raw::{c_char, c_int, c_ulong};

#[repr(C)]
pub struct Camera {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CameraList {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CameraFile {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CameraAbilitiesList {
    _private: [u8; 0],
}
#[repr(C)]
pub struct GPPortInfoList {
    _private: [u8; 0],
}
#[repr(C)]
pub struct GPContext {
    _private: [u8; 0],
}
#[repr(C)]
pub struct GPPortInfo {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CameraAbilities {
    pub model: [c_char; 128],
    pub status: c_int,
    pub port: c_int,
    pub speed: [c_int; 64],
    pub operations: c_int,
    pub file_operations: c_int,
    pub folder_operations: c_int,
    pub usb_vendor: c_int,
    pub usb_product: c_int,
    pub usb_class: c_int,
    pub usb_subclass: c_int,
    pub usb_protocol: c_int,
    pub library: [c_char; 1024],
    pub id: [c_char; 1024],
    pub device_type: c_int,
    pub reserved: [c_int; 8],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CameraFileInfoFile {
    pub fields: c_int,
    pub status: c_int,
    pub size: u64,
    pub mime_type: [c_char; 64],
    pub width: u32,
    pub height: u32,
    pub permissions: c_int,
    pub mtime: i64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CameraFileInfoPreview {
    pub fields: c_int,
    pub status: c_int,
    pub size: u64,
    pub mime_type: [c_char; 64],
    pub width: u32,
    pub height: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CameraFileInfoAudio {
    pub fields: c_int,
    pub status: c_int,
    pub size: u64,
    pub mime_type: [c_char; 64],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CameraFileInfo {
    pub preview: CameraFileInfoPreview,
    pub file: CameraFileInfoFile,
    pub audio: CameraFileInfoAudio,
}

pub const FILE_INFO_MTIME: c_int = 1 << 7;

pub const FILE_PREVIEW: c_int = 0;
pub const FILE_NORMAL: c_int = 1;

extern "C" {
    pub fn gp_context_new() -> *mut GPContext;
    pub fn gp_context_unref(context: *mut GPContext);
    pub fn gp_camera_new(camera: *mut *mut Camera) -> c_int;
    pub fn gp_camera_free(camera: *mut Camera) -> c_int;
    pub fn gp_camera_set_abilities(camera: *mut Camera, abilities: CameraAbilities) -> c_int;
    pub fn gp_camera_set_port_info(camera: *mut Camera, info: *mut GPPortInfo) -> c_int;
    pub fn gp_camera_init(camera: *mut Camera, context: *mut GPContext) -> c_int;
    pub fn gp_camera_exit(camera: *mut Camera, context: *mut GPContext) -> c_int;
    pub fn gp_camera_autodetect(list: *mut CameraList, context: *mut GPContext) -> c_int;
    pub fn gp_camera_folder_list_files(
        camera: *mut Camera,
        folder: *const c_char,
        list: *mut CameraList,
        context: *mut GPContext,
    ) -> c_int;
    pub fn gp_camera_folder_list_folders(
        camera: *mut Camera,
        folder: *const c_char,
        list: *mut CameraList,
        context: *mut GPContext,
    ) -> c_int;
    pub fn gp_camera_file_get_info(
        camera: *mut Camera,
        folder: *const c_char,
        file: *const c_char,
        info: *mut CameraFileInfo,
        context: *mut GPContext,
    ) -> c_int;
    pub fn gp_camera_file_get(
        camera: *mut Camera,
        folder: *const c_char,
        file: *const c_char,
        kind: c_int,
        output: *mut CameraFile,
        context: *mut GPContext,
    ) -> c_int;
    pub fn gp_camera_file_read(
        camera: *mut Camera,
        folder: *const c_char,
        file: *const c_char,
        kind: c_int,
        offset: u64,
        buffer: *mut c_char,
        size: *mut u64,
        context: *mut GPContext,
    ) -> c_int;
    pub fn gp_file_new(file: *mut *mut CameraFile) -> c_int;
    pub fn gp_file_new_from_fd(file: *mut *mut CameraFile, fd: c_int) -> c_int;
    pub fn gp_file_free(file: *mut CameraFile) -> c_int;
    pub fn gp_file_get_data_and_size(
        file: *mut CameraFile,
        data: *mut *const c_char,
        size: *mut c_ulong,
    ) -> c_int;
    pub fn gp_list_new(list: *mut *mut CameraList) -> c_int;
    pub fn gp_list_free(list: *mut CameraList) -> c_int;
    pub fn gp_list_count(list: *mut CameraList) -> c_int;
    pub fn gp_list_get_name(list: *mut CameraList, index: c_int, name: *mut *const c_char)
        -> c_int;
    pub fn gp_list_get_value(
        list: *mut CameraList,
        index: c_int,
        value: *mut *const c_char,
    ) -> c_int;
    pub fn gp_abilities_list_new(list: *mut *mut CameraAbilitiesList) -> c_int;
    pub fn gp_abilities_list_free(list: *mut CameraAbilitiesList) -> c_int;
    pub fn gp_abilities_list_load(list: *mut CameraAbilitiesList, context: *mut GPContext)
        -> c_int;
    pub fn gp_abilities_list_lookup_model(
        list: *mut CameraAbilitiesList,
        model: *const c_char,
    ) -> c_int;
    pub fn gp_abilities_list_get_abilities(
        list: *mut CameraAbilitiesList,
        index: c_int,
        abilities: *mut CameraAbilities,
    ) -> c_int;
    pub fn gp_port_info_list_new(list: *mut *mut GPPortInfoList) -> c_int;
    pub fn gp_port_info_list_free(list: *mut GPPortInfoList) -> c_int;
    pub fn gp_port_info_list_load(list: *mut GPPortInfoList) -> c_int;
    pub fn gp_port_info_list_lookup_path(list: *mut GPPortInfoList, path: *const c_char) -> c_int;
    pub fn gp_port_info_list_get_info(
        list: *mut GPPortInfoList,
        index: c_int,
        info: *mut *mut GPPortInfo,
    ) -> c_int;
}

pub fn text(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}
