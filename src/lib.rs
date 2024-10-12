use std::ffi::c_void;

use advert_manager::{advert_manager_initialize_hook, zone_postload_hook};
use log::debug;
use windows::{
    core::PCSTR,
    Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    Win32::{Foundation::HMODULE, System::LibraryLoader::GetModuleHandleA},
};

mod advert_manager;
mod dx_tools;
mod file_utils;

pub static EXE_BASE_ADDR: i32 = 0x00400000;

fn init_logs() {
    use log::LevelFilter;
    use simplelog::{
        ColorChoice, CombinedLogger, Config, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
    };
    let cfg = ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .build();
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Trace,
            cfg,
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Trace,
            Config::default(),
            std::fs::File::create(".\\ads.log").expect("Couldn't create log file: .\\ads.log"),
        ),
    ])
    .unwrap();
    log_panics::init();
}

#[no_mangle]
#[allow(non_snake_case)]
extern "system" fn DllMain(
    dll_module: windows::Win32::Foundation::HMODULE,
    call_reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> i32 {
    match call_reason {
        DLL_PROCESS_ATTACH => init(dll_module),
        DLL_PROCESS_DETACH => free(dll_module),
        _ => (),
    }
    true.into()
}

pub fn init(module: HMODULE) {
    init_logs();
    debug!("amax_ads base: {module:X?}");
    unsafe { zone_postload_hook() };
    unsafe { advert_manager_initialize_hook() };

    let _ptr_base: *mut c_void = unsafe { GetModuleHandleA(PCSTR::null()) }.unwrap().0 as _;
}

pub fn free(module: HMODULE) {
    debug!("amax_ads exiting: {module:X?}");
}
