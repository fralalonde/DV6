use core::cell::OnceCell;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};

pub struct Shared<T: Sized> {
    name: &'static str,
    value: Mutex<ThreadModeRawMutex, OnceCell<T>>
}

impl<T: Sized> Shared<T> {
    /// Create a new mutex with the given value.
    pub const fn uninit(name: &'static str) -> Self {
        Self {
            name,
            value: Mutex::new(OnceCell::new()),
        }
    }

    pub async fn lock(&self) -> MutexGuard<'_, ThreadModeRawMutex, OnceCell<T>> {
        self.value.lock().await
    }
}
