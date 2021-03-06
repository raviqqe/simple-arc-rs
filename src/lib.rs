use std::{
    alloc::{alloc, dealloc, Layout},
    ops::Deref,
    ptr::null,
    sync::atomic::{fence, Ordering},
};

const INITIAL_COUNT: usize = 0;

#[derive(Debug)]
pub struct Arc<T> {
    pointer: *const T,
}

struct ArcInner<T> {
    count: std::sync::atomic::AtomicUsize,
    payload: T,
}

impl<T> Arc<T> {
    pub fn new(payload: T) -> Self {
        if Self::is_zero_sized() {
            Self { pointer: null() }
        } else {
            let pointer = unsafe { &mut *(alloc(Self::block_layout()) as *mut ArcInner<T>) };

            *pointer = ArcInner::<T> {
                count: std::sync::atomic::AtomicUsize::new(INITIAL_COUNT),
                payload,
            };

            Self {
                pointer: &pointer.payload,
            }
        }
    }

    fn block_pointer(&self) -> &ArcInner<T> {
        unsafe { &*((self.pointer as *const usize).offset(-1) as *const ArcInner<T>) }
    }

    fn block_layout() -> Layout {
        Layout::new::<ArcInner<T>>()
    }

    fn is_zero_sized() -> bool {
        Layout::new::<T>().size() == 0
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.block_pointer().payload
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if !Self::is_zero_sized() {
            // TODO Is this correct ordering?
            self.block_pointer().count.fetch_add(1, Ordering::Relaxed);
        }

        Self {
            pointer: self.pointer,
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if Self::is_zero_sized() {
            return;
        }

        // TODO Is this correct ordering?
        if self.block_pointer().count.fetch_sub(1, Ordering::Release) == INITIAL_COUNT {
            fence(Ordering::Acquire);

            unsafe {
                dealloc(
                    self.block_pointer() as *const ArcInner<T> as *mut u8,
                    Self::block_layout(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop<T>(_: T) {}

    #[test]
    fn create() {
        Arc::new(0);
    }

    #[test]
    fn clone() {
        let rc = Arc::new(0);
        drop(rc.clone());
        drop(rc);
    }

    #[test]
    fn load_payload() {
        assert_eq!(*Arc::new(42), 42);
    }

    mod zero_sized {
        use super::*;

        #[test]
        fn create() {
            Arc::new(());
        }

        #[test]
        fn clone() {
            let rc = Arc::new(());
            drop(rc.clone());
            drop(rc);
        }

        #[test]
        #[allow(clippy::unit_cmp)]
        fn load_payload() {
            assert_eq!(*Arc::new(()), ());
        }
    }
}
