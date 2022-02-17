#![feature(c_unwind)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod lua;

macro_rules! dbg {
	($fmt:literal, $($arg:expr),+) => {
		#[cfg(debug_assertions)]
		println!(concat!("gm_async_write | DEBUG: ", $fmt), $($arg),+);
	};

	($msg:literal) => {
		#[cfg(debug_assertions)]
		println!(concat!("gm_async_write | DEBUG: ", $msg));
	};
}

mod whitelist;
mod worker;
mod util;
mod fs;

#[no_mangle]
pub unsafe extern "C-unwind" fn gmod13_open(lua: lua::State) -> i32 {
	dbg!("gmod13_open");

	// util::ThreadSingleton needs the main thread id
	#[cfg(debug_assertions)]
	lazy_static::initialize(&worker::CALLBACK_QUEUE);

	lua.get_global(lua_string!("file"));

	lua.push_function(fs::async_write);
	lua.set_field(-2, lua_string!("AsyncWrite"));

	lua.push_function(fs::async_append);
	lua.set_field(-2, lua_string!("AsyncAppend"));

	lua.pop();
	0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn gmod13_close(_lua: lua::State) -> i32 {
	dbg!("gmod13_close");

	if let Some(rt) = worker::WORKER_RT.0.take() {
		println!("Shutting down gm_async_write worker thread...");
		rt.shutdown_timeout(std::time::Duration::from_secs(20));
	}

	dbg!("done");
	0
}