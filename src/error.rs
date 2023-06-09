//! Error type definition.

use thiserror::Error;

/// Error type for `viguno`
#[derive(Error, Debug)]
pub enum Error {
    #[error("other error")]
    OtherError,
}
