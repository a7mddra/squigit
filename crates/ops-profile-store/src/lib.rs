// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Profile storage and management for Squigit.
//!
//! This crate provides multi-account profile management where each profile
//! is tied to a Google account. Profiles store user identity, BYOK keys,
//! and chat data in isolated directories.
//!
//! # Directory Structure
//!
//! ```text
//! {config_dir}/squigit/
//! ├── preferences.json              # GLOBAL (shared across profiles)
//! └── Local Storage/
//!     ├── index.json                # Profile index + active profile
//!     └── {profile_id}/
//!         ├── profile.json          # Google profile data
//!         ├── {provider}_key.json   # Per-profile BYOK
//!         ├── imgbb_key.json        # Per-profile BYOK
//!         └── chats/                # Per-profile chat storage
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ops_profile_store::{ProfileStore, Profile};
//!
//! let store = ProfileStore::new().unwrap();
//!
//! // Create profile from Google auth data
//! let profile = Profile::new(
//!     "user@gmail.com",
//!     "John Doe",
//!     Some("/path/to/avatar.png".to_string()),
//!     Some("https://google.com/avatar.jpg".to_string()),
//! );
//!
//! store.upsert_profile(&profile).unwrap();
//! store.set_active_profile_id(&profile.id).unwrap();
//! ```

pub mod auth;
pub mod error;
pub mod security;
pub mod store;
pub mod types;

pub use error::{ProfileError, Result};
pub use store::ProfileStore;
pub use types::{Profile, ProfileIndex};
pub use auth::{AuthFlowSettings, AuthSuccessData, BrowserOpener, CredentialsSource};
pub use security::{ApiKeyProvider, validate_api_key};
