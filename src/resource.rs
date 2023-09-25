use core::mem::MaybeUninit;

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};

pub struct Shared<T: Sized> {
    name: &'static str,
    init: AtomicBool,
    value: MaybeUninit<Mutex<ThreadModeRawMutex, T>>,
}

unsafe impl<T: Sized + Send> Send for Shared<T> {}

unsafe impl<T: Sized + Send> Sync for Shared<T> {}

impl<T: Sized> Shared<T> {
    /// Create a new mutex with the given value.
    pub const fn uninit(name: &'static str) -> Self {
        Self {
            name,
            value: MaybeUninit::uninit(),
            init: AtomicBool::new(false),
        }
    }
    pub fn init_static(&self, value: T) {
        if self.init.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            panic!("Shared resource {} init twice", self.name)
        }
        let p = self.value.as_ptr() as *mut Mutex<ThreadModeRawMutex, T>;
        unsafe { *p = Mutex::new(value) }
        self.init.store(true, Ordering::Relaxed)
    }

    pub async fn lock(&self) -> MutexGuard<'_, ThreadModeRawMutex, T> {
        unsafe { self.value.assume_init_ref() }.lock().await
    }
}
