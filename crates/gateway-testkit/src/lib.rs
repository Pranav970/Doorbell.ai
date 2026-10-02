//! Shared test fixtures and helpers for the gateway workspace — see
//! `CLAUDE.md`'s crate map. Every crate used to hand-roll its own
//! `setup_team`/`store_credential`/`test_cipher` helpers; this crate is
//! where those live now.

mod fixtures;
mod golden;
mod mock;

pub use fixtures::{
    create_org, create_provider_credential, create_team, create_user, create_virtual_key,
    setup_team, test_cipher, test_master_key_b64, TeamFixture, VirtualKeyFixture,
};
pub use golden::golden_settings;
pub use mock::{MockProviderBuilder, MockServer};
