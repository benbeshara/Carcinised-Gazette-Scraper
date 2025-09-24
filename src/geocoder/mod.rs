pub mod azure;
pub mod core;
pub mod google;
#[cfg(test)]
pub(crate) mod mock;

use core::GeocoderProvider;

pub use core::GeocoderRequest;
