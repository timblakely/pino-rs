use core::cell::Cell;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use scopeguard::defer;

const EMPTY: usize = 0;
const LOADING: usize = 1;
const LOADED: usize = 2;
const LOCKED: usize = 3;

/// A mechanism for loaning a reference to an interrupt handler (or another
/// thread).
///
/// An `IRef` is initially empty. An exclusive reference to some data can be
/// *donated* by using the `donate` method; this puts the `IRef` into the
/// *loaded* state, runs a supplied closure, and then returns it to *empty*
/// before returning.
///
/// The contents of the `IRef` can be observed using the `observe` method. If
/// the `IRef` is *loaded*, `observe` switches it to *locked* state and runs a
/// closure on a reference to the contents. When the closure finishes, the
/// `IRef` returns to *loaded*.
///
/// `donate` is intended primarily for non-interrupt code, and can busy-wait.
/// `observe` cannot, and is safer for use by interrupts. See each method's
/// documentation for specifics.
#[cfg(target_os = "none")]
#[derive(Debug)]
pub struct IRef<T> {
    state: AtomicUsize,
    poisoned: AtomicBool,
    contents: Cell<(usize, usize)>,
    _marker: PhantomData<T>,
}

#[cfg(target_os = "none")]
unsafe impl<T> Sync for IRef<T> {}

#[cfg(target_os = "none")]
impl<T> IRef<T> {
    /// Creates an `IRef` in the *empty* state.
    ///
    /// ```ignore
    /// static REF: IRef<MyType> = IRef::new();
    /// ```
    pub const fn new() -> Self {
        IRef {
            state: AtomicUsize::new(EMPTY),
            poisoned: AtomicBool::new(false),
            contents: Cell::new((0, 0)),
            _marker: PhantomData,
        }
    }

    /// Donates an exclusive reference `val` to observers of the `IRef` for the
    /// duration of execution of `scope`.
    ///
    /// When `scope` returns, `donate` will busy-wait until any observer of the
    /// `IRef` is done, and then atomically reset the `IRef` to empty, ensuring
    /// that the caller regains exclusive use of `val`.
    ///
    /// # Panics
    ///
    /// If `self` is not empty. This means `donate` cannot be called recursively
    /// or from multiple threads simultaneously.
    pub fn donate<'env, F, R>(&self, val: &'env mut F, scope: impl FnOnce() -> R) -> R
    where
        F: FnMut(&mut T),
        F: Send + 'env,
    {
        let r = self
            .state
            .compare_exchange(EMPTY, LOADING, Ordering::Acquire, Ordering::Relaxed);
        assert_eq!(r, Ok(EMPTY), "concurrent/reentrant donation to IRef");

        // Construct a FnMut fat pointer to our closure, and then erase its
        // type.
        let val: &mut (dyn FnMut(_) + Send + 'env) = val;
        // Safety: we only reinterpret these bits as the same type used above
        // but with *narrower* lifetime.
        let val: (usize, usize) = unsafe { core::mem::transmute(val) };

        // By placing the cell in LOADING state we now have exclusive control.
        // In particular, it is safe to do this:
        self.contents.set(val);
        self.state.store(LOADED, Ordering::Release);

        defer! {{
            // Busy-wait on the interrupt.
            loop {
                let r = self.state.compare_exchange_weak(
                    LOADED,
                    EMPTY,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                    );
                if let Ok(_) = r { break }
                cortex_m::asm::wfi();
            }

            if self.poisoned.load(Ordering::Acquire) {
                panic!("IRef poisoned by panic in observer")
            }
        }}

        scope()
    }

    /// Locks the `IRef` and observes its contents, if it is not empty or
    /// already locked.
    ///
    /// If this is called concurrently with `supply`, it will execute `body`
    /// with the reference donated by the caller of `supply`. During the
    /// execution of `body`, the `IRef` will be *locked*, preventing both
    /// other concurrent/reentrant calls to `observe` from succeeding, and the
    /// caller of `supply` from returning.
    ///
    /// If the `IRef` is either empty or locked, returns `None` without
    /// executing `body`.
    ///
    /// This operation will never busy-wait (unless, of course, `body` contains
    /// code that will busy-wait).
    pub fn observe<R, F>(&self, body: F) -> Option<R>
    where
        F: FnOnce(
            &mut (dyn FnMut(&mut T) + Send),
        ) -> R,
    {
        self.state
            .compare_exchange(LOADED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            // Having successfully exchanged LOADED for LOCKED, we know no other
            // thread is going to try to muck with the cell. Time to access its
            // contents. This is safe because of the atomic exchange above.
            .map(|_| {
                if self.poisoned.load(Ordering::Acquire) {
                    panic!("IRef poisoned by panic in observer")
                }

                let poisoner =
                    scopeguard::guard((), |_| self.poisoned.store(true, Ordering::Release));

                let result = {
                    let r = self.contents.get();
                    // We do *not* know the correct lifetime for the &mut.  This
                    // is why the `body` closure is (implicitly) `for<'a>`.
                    let r: &mut (dyn FnMut(&mut _)
                                 + Send) =
                        // Safety: we put it in there, we have used locking to
                        // ensure that our reference will be unique, and the
                        // `donate` code will ensure this hasn't gone out of
                        // scope.
                        unsafe { core::mem::transmute(r) };
                    body(r)
                };
                self.state.store(LOADED, Ordering::Release);
                scopeguard::ScopeGuard::into_inner(poisoner);
                result
            })
    }
}
