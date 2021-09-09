use crate::{
	lua::{self, LuaReference},
	whitelist,
	worker::{self, Job},
};
use path_clean::PathClean;
use std::{fs, io::ErrorKind, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(isize)]
#[allow(non_camel_case_types)]
#[allow(unused)]
pub enum FSASYNC {
	FSASYNC_ERR_NOT_MINE = -8, // Filename not part of the specified file system, try a different one. (Used internally to find the right filesystem)
	FSASYNC_ERR_RETRY_LATER = -7, // Failure for a reason that might be temporary. You might retry, but not immediately. (E.g. Network problems)
	FSASYNC_ERR_ALIGNMENT = -6,   // read parameters invalid for unbuffered IO
	FSASYNC_ERR_FAILURE = -5,     // hard subsystem failure
	FSASYNC_ERR_READING = -4,     // read error on file
	FSASYNC_ERR_NOMEMORY = -3,    // out of memory for file read
	FSASYNC_ERR_UNKNOWNID = -2,   // caller's provided id is not recognized
	FSASYNC_ERR_FILEOPEN = -1,    // filename could not be opened (bad path, not exist, etc)
	FSASYNC_OK = 0,               // operation is successful
	FSASYNC_STATUS_PENDING = 1,   // file is properly queued, waiting for service
	FSASYNC_STATUS_INPROGRESS = 2, // file is being accessed
	FSASYNC_STATUS_ABORTED = 3,   // file was aborted by caller
	FSASYNC_STATUS_UNSERVICED = 4, // file is not yet queued
}
impl From<ErrorKind> for FSASYNC {
	fn from(error: ErrorKind) -> Self {
		dbg!("Error: {:?}", error);
		match error {
			std::io::ErrorKind::OutOfMemory => FSASYNC::FSASYNC_ERR_NOMEMORY,
			_ => FSASYNC::FSASYNC_ERR_FAILURE,
		}
	}
}

pub fn validate_path<S: AsRef<str>>(file_name: S) -> Option<PathBuf> {
	let file_name = file_name.as_ref();

	if file_name.ends_with(|char| char == '/' || char == '\\') {
		dbg!("Invalid (ends with slash)");
		return None;
	}

	let path = PathBuf::from(format!("garrysmod/data/{}", file_name)).clean();
	dbg!("Path: {:?}", path);
	if path.starts_with("garrysmod/data") && !path.is_dir() && whitelist::check(&path) {
		Some(path)
	} else {
		dbg!(
			"Invalid {} {} {}",
			path.starts_with("garrysmod/data"),
			!path.is_dir(),
			whitelist::check(&path)
		);
		None
	}
}

#[inline]
unsafe fn digest_args<'a>(
	lua: &'a lua::State
) -> Option<(String, PathBuf, &'a [u8], Option<LuaReference>, bool)> {
	let file_name = lua.check_string(1);
	let data = lua.check_binary_string(2);
	let sync = lua.to_boolean(4);
	let callback = if lua.get_top() < 3 || lua.is_nil(3) {
		None
	} else {
		lua.check_function(3);
		if sync {
			Some(0)
		} else {
			lua.push_value(3);
			Some(lua.reference())
		}
	};

	if let Some(path) = validate_path(file_name.as_ref()) {
		Some((file_name.into_owned(), path, data, callback, sync))
	} else {
		None
	}
}

pub unsafe extern "C-unwind" fn async_write(lua: lua::State) -> i32 {
	use std::io::Write;

	let (raw_path, path, data, callback, sync) = match digest_args(&lua) {
		Some(args) => args,
		None => {
			lua.push_integer(FSASYNC::FSASYNC_ERR_FILEOPEN as _);
			return 1;
		}
	};

	if sync {
		let result = match fs::File::create(&path)
			.map_err(|_| FSASYNC::FSASYNC_ERR_FILEOPEN)
			.and_then(|mut file| {
				file.write_all(data)
					.map_err(|_| FSASYNC::FSASYNC_ERR_FAILURE)
			}) {
			Ok(_) => FSASYNC::FSASYNC_OK,
			Err(error) => error,
		};

		if let Some(_) = callback {
			lua.push_value(3);
			lua.push_string(&raw_path);
			lua.push_integer(result as _);
			lua.call(2, 0);
		}
	} else {
		worker::job(
			lua,
			Job {
				raw_path,
				path,
				data: data.to_vec(),
				callback,
				append: false,
				result: None
			},
		);
	}

	lua.push_integer(FSASYNC::FSASYNC_OK as _);
	1
}

pub unsafe extern "C-unwind" fn async_append(lua: lua::State) -> i32 {
	use std::io::Write;

	let (raw_path, path, data, callback, sync) = match digest_args(&lua) {
		Some(args) => args,
		None => {
			lua.push_integer(FSASYNC::FSASYNC_ERR_FILEOPEN as _);
			return 1;
		}
	};

	if sync {
		let result = match fs::OpenOptions::new()
			.append(true)
			.create(true)
			.open(&path)
			.map_err(|_| FSASYNC::FSASYNC_ERR_FILEOPEN)
			.and_then(|mut f| f.write_all(data).map_err(|_| FSASYNC::FSASYNC_ERR_FAILURE))
		{
			Ok(_) => FSASYNC::FSASYNC_OK,
			Err(error) => error,
		};

		if let Some(_) = callback {
			lua.push_value(3);
			lua.push_string(&raw_path);
			lua.push_integer(result as _);
			lua.call(2, 0);
		}
	} else {
		worker::job(
			lua,
			Job {
				raw_path,
				path,
				data: data.to_vec(),
				callback,
				append: true,
				result: None
			},
		);
	}

	1
}
