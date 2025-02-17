use blur_plugins_core::{BlurAPI, BlurPlugin};
use std::ffi::c_void;

mod advert_manager;
mod dx_tools;
mod file_utils;

mod hooks;
mod cache;

pub struct MyPlugin {}

static mut G_API: Option<&dyn BlurAPI> = None;

impl MyPlugin {
	fn new(api: &dyn BlurAPI) -> Self {
		let ptr_base = api.get_exe_base_ptr();
		hooks::install(ptr_base);
		Self {}
	}

	pub fn get_api() -> &'static dyn BlurAPI {
		unsafe { G_API.unwrap() }
	}

	pub fn get_exe_base_ptr() -> *mut c_void {
		Self::get_api().get_exe_base_ptr()
	}
}

impl BlurPlugin for MyPlugin {
	fn name(&self) -> &'static str {
		"AMAX_ADS"
	}

	fn on_event(&self, _event: &blur_plugins_core::BlurEvent) {}

	fn free(&self) {}
}

#[no_mangle]
fn plugin_init(api: &'static mut dyn BlurAPI) -> Box<dyn BlurPlugin> {
	init_logs();
	unsafe {
		G_API = Some(api);
	}
	Box::new(MyPlugin::new(api))
}

fn init_logs() {
	use simplelog::{
		ColorChoice, CombinedLogger, ConfigBuilder, LevelFilter, TermLogger, TerminalMode,
		WriteLogger,
	};
	let cfg = ConfigBuilder::new()
		.set_time_offset_to_local()
		.unwrap()
		//.add_filter_allow_str("amax_ads")
		.build();

	let log_file = blur_plugins_core::create_log_file("amax_ads.log").unwrap();

	CombinedLogger::init(vec![
		TermLogger::new(
			LevelFilter::Trace,
			cfg.clone(),
			TerminalMode::Mixed,
			ColorChoice::Auto,
		),
		WriteLogger::new(LevelFilter::Trace, cfg, log_file),
	])
	.unwrap();
	log_panics::init();
}
