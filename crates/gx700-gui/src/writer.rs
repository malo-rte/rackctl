//! Background batch writer: stores a set of patches to the unit slot-by-slot off
//! the UI thread (each write verified by read-back), reporting progress as it goes
//! so a whole scene (up to 100 patches) writes behind a progress bar instead of
//! freezing the window.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rackctl_gx700::RawPatch;

use crate::device::{SharedDevice, lock};

/// Gap between writes, to avoid overrunning the USB-MIDI interface.
const WRITE_PACE: Duration = Duration::from_millis(20);

/// A result from the background writer.
pub(crate) enum Written {
    /// A slot was stored and verified.
    Ok(u16),
    /// A slot failed to store.
    Failed(u16),
    /// The batch finished (or was cancelled).
    Done,
}

/// A running batch write. Dropping it cancels and joins the thread.
pub(crate) struct Writer {
    cancel: Arc<AtomicBool>,
    rx: Receiver<Written>,
    handle: Option<JoinHandle<()>>,
    /// Total patches in the batch (for the progress bar).
    total: usize,
}

impl Writer {
    /// Spawn a write of every `(slot, patch)` to the device, verifying each. Locks
    /// the device per slot so the write stays cooperative.
    pub(crate) fn spawn(device: SharedDevice, patches: Vec<(u16, RawPatch)>) -> Self {
        let total = patches.len();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let handle = {
            let cancel = Arc::clone(&cancel);
            thread::spawn(move || run(&device, &cancel, &tx, &patches))
        };
        Self {
            cancel,
            rx,
            handle: Some(handle),
            total,
        }
    }

    /// How many patches the batch will write.
    pub(crate) fn total(&self) -> usize {
        self.total
    }

    /// Take every result produced since the last call.
    pub(crate) fn drain(&self) -> Vec<Written> {
        self.rx.try_iter().collect()
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(
    device: &SharedDevice,
    cancel: &AtomicBool,
    tx: &Sender<Written>,
    patches: &[(u16, RawPatch)],
) {
    for (slot, raw) in patches {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let result = match write_one(device, *slot, raw) {
            Ok(()) => Written::Ok(*slot),
            Err(()) => Written::Failed(*slot),
        };
        if tx.send(result).is_err() {
            return; // UI gone
        }
        thread::sleep(WRITE_PACE);
    }
    let _ = tx.send(Written::Done);
}

/// Write one patch and verify it stuck by reading it back. `Err` if the write
/// failed or the read-back didn't match (usually: not in BULK LOAD mode).
fn write_one(device: &SharedDevice, slot: u16, raw: &RawPatch) -> Result<(), ()> {
    lock(device).write_patch(slot, raw).map_err(|_| ())?;
    let got = lock(device).read_patch(slot).map_err(|_| ())?;
    if got.blocks == raw.blocks {
        Ok(())
    } else {
        Err(())
    }
}
