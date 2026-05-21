//! Microsoft OAuth Device Flow + token store. Plan 08-02.

pub mod device_flow_microsoft;
pub mod error;
pub mod token_store;

pub use device_flow_microsoft::{DeviceFlowStart, MicrosoftAuth, MicrosoftTokens};
pub use error::MicrosoftAuthError;
pub use token_store::MicrosoftTokenStore;
