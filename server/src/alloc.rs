//! Heap accounting for one solve, after the counting allocator in synta-fuzz.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use serde::Serialize;

/// The system allocator plus process-wide counters.
struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOC: Counting = Counting;

fn grow(by: u64) {
    PEAK.fetch_max(LIVE.fetch_add(by, Relaxed) + by, Relaxed);
}

// SAFETY: every method delegates to `System` with the caller's arguments unchanged;
// the counters only observe sizes and never touch the memory.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size() as u64, Relaxed);
            grow(layout.size() as u64);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds `GlobalAlloc::dealloc`'s contract.
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size() as u64, Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::realloc`'s contract.
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            let (old, size) = (layout.size() as u64, new_size as u64);
            if size > old {
                BYTES.fetch_add(size - old, Relaxed);
                grow(size - old);
            } else {
                LIVE.fetch_sub(old - size, Relaxed);
            }
        }
        new
    }
}

/// Heap use while one closure ran.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Usage {
    /// Calls to `alloc`.
    pub allocs: u64,
    /// Bytes requested by `alloc` plus growth by `realloc`.
    pub bytes: u64,
    /// Most bytes live at once, above the level when the closure started.
    pub peak: u64,
}

/// Runs `f` and reports its heap use.
///
/// The counters are process-wide, so a lock keeps two measurements from overlapping.
/// shortcut: allocations by other requests running at the same time still count,
/// move to per-thread counters if the numbers must be exact under load.
pub fn measure<T>(f: impl FnOnce() -> T) -> (T, Usage) {
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = LIVE.load(Relaxed);
    PEAK.store(base, Relaxed);
    let (allocs, bytes) = (ALLOCS.load(Relaxed), BYTES.load(Relaxed));
    let out = f();
    let usage = Usage {
        allocs: ALLOCS.load(Relaxed) - allocs,
        bytes: BYTES.load(Relaxed) - bytes,
        peak: PEAK.load(Relaxed).saturating_sub(base),
    };
    (out, usage)
}

#[cfg(test)]
mod tests {
    use super::measure;

    #[test]
    fn counts_a_vector() {
        let (len, usage) = measure(|| std::hint::black_box(vec![0u8; 1 << 20]).len());
        assert_eq!(len, 1 << 20);
        assert!(usage.allocs >= 1);
        assert!(usage.bytes >= 1 << 20);
        assert!(usage.peak >= 1 << 20);
    }
}
