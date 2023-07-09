use std::mem::swap;

use crossbeam::atomic::AtomicCell;

static CONTEXT: AtomicCell<Option<mdbg_rs::Debugger>> = AtomicCell::new(None);

pub struct Context {
    debugger: Option<mdbg_rs::Debugger<'static>>,
}

impl Context {
    pub fn store(debugger: mdbg_rs::Debugger<'static>) -> *mut Option<mdbg_rs::Debugger<'static>> {
        CONTEXT.store(Some(debugger));
        CONTEXT.as_ptr()
    }

    pub fn from(ctx: u64) -> Result<Context, ()> {
        if ctx != CONTEXT.as_ptr() as u64 {
            return Err(());
        }

        Ok(Context {
            debugger: CONTEXT.swap(None),
        })
    }

    pub fn with_debugger<T, F: FnMut(&mut mdbg_rs::Debugger) -> Result<T, ()>>(
        &mut self,
        debugger_action: F,
    ) -> Result<T, ()> {
        self.debugger.as_mut().ok_or(()).and_then(debugger_action)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let mut dbg = None;
        swap(&mut self.debugger, &mut dbg);
        CONTEXT.store(dbg);
    }
}
