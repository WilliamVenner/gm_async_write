use std::cell::RefCell;

#[cfg(debug_assertions)]
pub struct ThreadSingleton<T>(pub RefCell<T>, RefCell<Option<std::thread::ThreadId>>);

#[cfg(not(debug_assertions))]
pub struct ThreadSingleton<T>(pub RefCell<T>);

impl<T> ThreadSingleton<T> {
	#[cfg(debug_assertions)]
	pub const fn new(val: T) -> ThreadSingleton<T> {
		ThreadSingleton(RefCell::new(val), RefCell::new(None))
	}

	#[cfg(not(debug_assertions))]
	pub const fn new(val: T) -> ThreadSingleton<T> {
		Self(RefCell::new(val))
	}
}

unsafe impl<T> Sync for ThreadSingleton<T> {}
impl<T> std::ops::Deref for ThreadSingleton<T> {
	type Target = RefCell<T>;

	#[cfg(not(debug_assertions))]
	fn deref(&self) -> &Self::Target {
		&self.0
	}

	#[cfg(debug_assertions)]
	fn deref(&self) -> &Self::Target {
		assert_eq!(std::thread::current().id(), *self.1.borrow_mut().get_or_insert_with(|| std::thread::current().id()), "Potential data race of global singleton!");
		&self.0
	}
}

pub struct UnsafeSync<T>(pub T);
unsafe impl<T> Sync for UnsafeSync<T> {}
impl<T> std::ops::Deref for UnsafeSync<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}