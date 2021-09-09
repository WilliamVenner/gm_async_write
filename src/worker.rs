use crate::{
	fs::FSASYNC,
	lua::{self, LuaReference, LUA_REGISTRYINDEX},
	util,
};
use std::{path::PathBuf, sync::atomic::AtomicUsize};

pub(super) static WORKER_RT: util::ThreadSingleton<Option<tokio::runtime::Runtime>> = util::ThreadSingleton::new(None);

lazy_static! {
	static ref JOB_QUEUE: tokio::sync::mpsc::UnboundedSender<Job> = {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		spawn(rx);
		tx
	};

	pub(super) static ref CALLBACK_QUEUE: util::UnsafeSync<(std::sync::mpsc::Sender<Job>, std::sync::mpsc::Receiver<Job>)> =util::UnsafeSync(std::sync::mpsc::channel());
}
static CALLBACKS_PENDING: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
#[must_use]
pub struct Job {
	pub raw_path: String,
	pub path: PathBuf,
	pub data: Vec<u8>,
	pub callback: Option<LuaReference>,
	pub append: bool,
	pub result: Option<FSASYNC>,
}

pub fn job(lua: lua::State, job: Job) {
	dbg!("spawning job: {:#?}", job);
	if job.callback.is_some() {
		listen(lua);
		CALLBACKS_PENDING.fetch_add(1, std::sync::atomic::Ordering::Release);
	}
	JOB_QUEUE.send(job).unwrap();
}

async fn process_job(mut job: Job) {
	use tokio::io::AsyncWriteExt;

	dbg!("processing job: {:#?}", job);

	let f = tokio::fs::OpenOptions::new()
		.append(job.append)
		.truncate(!job.append)
		.write(!job.append)
		.create(true)
		.open(&job.path)
		.await;

	if job.callback.is_none() {

		if let Ok(mut f) = f {
			f.write_all(&std::mem::take(&mut job.data)).await.ok();
		}
		dbg!("processed job [no callback]");

	} else {

		match f {
			Ok(mut f) => match f.write_all(&std::mem::take(&mut job.data)).await {
				Ok(_) => job.result = Some(FSASYNC::FSASYNC_OK),
				Err(_error) => {
					dbg!("Error: {:#?}", _error);
					job.result = Some(FSASYNC::FSASYNC_ERR_FAILURE);
				}
			},
			Err(_error) => {
				dbg!("Error: {:#?}", _error);
				job.result = Some(FSASYNC::FSASYNC_ERR_FILEOPEN);
			}
		}

		dbg!("processed job [calling back] {:?}", job.result);
		(&**CALLBACK_QUEUE).0.send(job).ok();

	}
}

fn spawn(mut rx: tokio::sync::mpsc::UnboundedReceiver<Job>) {
	dbg!("spawning worker thread");

	let rt = tokio::runtime::Builder::new_multi_thread()
		.thread_name("gm_async_write")
		.build()
		.expect("Failed to create Tokio runtime");

	rt.spawn(async move {
		dbg!("worker thread started");
		while let Some(job) = rx.recv().await {
			tokio::task::spawn(process_job(job));
		}
		dbg!("worker thread shutdown");
	});

	*WORKER_RT.0.borrow_mut() = Some(rt);
}

fn listen(lua: lua::State) {
	dbg!("listen");
	unsafe {
		lua.get_global(lua_string!("hook"));
		lua.get_field(-1, lua_string!("Add"));
		lua.push_string("Think");
		lua.push_string("gm_async_write");
		lua.push_function(poll);
		lua.call(3, 0);
		lua.pop();
	}
}
fn deafen(lua: lua::State) {
	dbg!("deafen");
	unsafe {
		lua.get_global(lua_string!("hook"));
		lua.get_field(-1, lua_string!("Remove"));
		lua.push_string("Think");
		lua.push_string("gm_async_write");
		lua.call(2, 0);
		lua.pop();
	}
}
unsafe extern "C-unwind" fn poll(lua: lua::State) -> i32 {
	let mut recv = 0;
	match CALLBACK_QUEUE.1.try_recv() {
		Ok(job) => {
			recv += 1;

			let callback = job.callback.unwrap_unchecked();
			lua.raw_geti(LUA_REGISTRYINDEX, callback);
			lua.dereference(callback);
			lua.push_string(&job.raw_path);
			lua.push_integer(job.result.unwrap_unchecked() as _);
			lua.call(2, 0);
		}
		Err(std::sync::mpsc::TryRecvError::Empty) => {}
		Err(std::sync::mpsc::TryRecvError::Disconnected) => {
			deafen(lua);
			return 0;
		}
	}
	if recv > 0 && CALLBACKS_PENDING.fetch_sub(recv, std::sync::atomic::Ordering::Release) - recv == 0 {
		deafen(lua);
	}
	0
}
