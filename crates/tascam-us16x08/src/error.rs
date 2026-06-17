//! Error and result types for the crate.

use thiserror::Error;

/// Convenience alias for results returned across this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Everything that can go wrong while talking to a US-16x08.
///
/// The variants deliberately avoid exposing backend-specific error types
/// (e.g. `alsa::Error`) so the public surface stays backend-agnostic
/// (rust-coding-rules RS-63); the backend folds its own errors into
/// [`Error::Backend`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// No card whose ALSA id is `US16x08` was found on the system.
    #[error("no Tascam US-16x08 sound card found")]
    CardNotFound,

    /// A control element addressed by `name`/`index` does not exist on the card.
    #[error("unknown control element {name:?} (index {index})")]
    UnknownControl {
        /// The ALSA control name that was looked up.
        name: String,
        /// The element index that was looked up.
        index: u32,
    },

    /// A channel/output index lies outside the control's scope.
    #[error("index {index} out of range (expected 0..{count})")]
    IndexOutOfRange {
        /// The offending index.
        index: u32,
        /// The number of valid indices for the control's scope.
        count: u32,
    },

    /// An integer/enum value lies outside the control's permitted range.
    #[error("value {value} out of range for {control} (expected {min}..={max})")]
    ValueOutOfRange {
        /// Human-readable control name.
        control: &'static str,
        /// The offending value.
        value: i32,
        /// Inclusive minimum.
        min: i32,
        /// Inclusive maximum.
        max: i32,
    },

    /// The supplied [`crate::Value`] kind does not match the control's kind.
    #[error("type mismatch for {control}: expected {expected}")]
    TypeMismatch {
        /// Human-readable control name.
        control: &'static str,
        /// The value kind the control expects.
        expected: &'static str,
    },

    /// The underlying hardware backend failed; the string carries its message.
    #[error("device backend error: {0}")]
    Backend(String),
}
