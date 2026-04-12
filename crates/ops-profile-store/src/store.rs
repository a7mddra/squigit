// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Profile storage manager.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::error::{ProfileError, Result};
use crate::types::{Profile, ProfileIndex};

const APP_DIR_NAME: &str = "squigit";

/// Storage directory name under the app config.
const STORAGE_DIR: &str = "Local Storage";

/// Profile index filename.
const INDEX_FILE: &str = "index.json";

/// Individual profile metadata filename.
const PROFILE_FILE: &str = "profile.json";

/// Manager for profile storage operations.
///
/// Handles CRUD operations for profiles, maintaining an index
/// of all profiles and tracking the active profile.
pub struct ProfileStore {
    /// Base directory: `{config_dir}/squigit/Local Storage/`
    base_dir: PathBuf,
    /// Path to the index file.
    index_path: PathBuf,
}

impl ProfileStore {
    /// Create a new profile store.
    ///
    /// Uses the OS-appropriate config directory:
    /// - Linux: `~/.config/squigit/Local Storage/`
    /// - macOS: `~/Library/Application Support/squigit/Local Storage/`
    /// - Windows: `%APPDATA%/squigit/Local Storage/`
    pub fn new() -> Result<Self> {
        let base_dir = dirs::config_dir()
            .ok_or(ProfileError::NoConfigDir)?
            .join(APP_DIR_NAME)
            .join(STORAGE_DIR);

        Self::with_base_dir(base_dir)
    }

    /// Create a profile store using an explicit base directory.
    ///
    /// This is primarily intended for tests and future CLI integration.
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        let index_path = base_dir.join(INDEX_FILE);

        fs::create_dir_all(&base_dir)?;

        Ok(Self {
            base_dir,
            index_path,
        })
    }

    /// Get the base storage directory path.
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Get the directory path for a specific profile.
    ///
    /// Returns `{base_dir}/{profile_id}/`
    pub fn get_profile_dir(&self, profile_id: &str) -> PathBuf {
        self.base_dir.join(profile_id)
    }

    /// Get the chats directory for a specific profile.
    ///
    /// Returns `{base_dir}/{profile_id}/chats/`
    pub fn get_chats_dir(&self, profile_id: &str) -> PathBuf {
        self.get_profile_dir(profile_id).join("chats")
    }

    /// Get the provider key file path for a profile.
    pub fn get_provider_key_path(&self, profile_id: &str, provider: &str) -> PathBuf {
        self.get_profile_dir(profile_id)
            .join(format!("{}_key.json", provider))
    }

    fn temp_path_for(&self, path: &Path) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("temp");
        path.with_file_name(format!(".{}.tmp-{}-{}", file_name, std::process::id(), suffix))
    }

    pub(crate) fn write_json_atomic<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        let json = serde_json::to_vec_pretty(value)?;
        self.write_bytes_atomic(path, &json)
    }

    pub(crate) fn write_bytes_atomic(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        let parent = path.parent().ok_or_else(|| {
            ProfileError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Path has no parent: {}", path.display()),
            ))
        })?;
        fs::create_dir_all(parent)?;

        let temp_path = self.temp_path_for(path);
        {
            let mut temp_file = File::create(&temp_path)?;
            temp_file.write_all(bytes)?;
            temp_file.sync_all()?;
        }

        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(path)?;
        }

        fs::rename(&temp_path, path)?;
        Ok(())
    }

    // =========================================================================
    // Index Operations
    // =========================================================================

    /// Load the profile index from disk.
    fn load_index(&self) -> Result<ProfileIndex> {
        if !self.index_path.exists() {
            return Ok(ProfileIndex::default());
        }

        let content = fs::read_to_string(&self.index_path)?;
        let index: ProfileIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    /// Save the profile index to disk.
    fn save_index(&self, index: &ProfileIndex) -> Result<()> {
        self.write_json_atomic(&self.index_path, index)
    }

    /// Get the ID of the currently active profile.
    pub fn get_active_profile_id(&self) -> Result<Option<String>> {
        let index = self.load_index()?;
        Ok(index.active_profile_id)
    }

    /// Set the active profile by ID.
    ///
    /// Returns an error if the profile doesn't exist.
    pub fn set_active_profile_id(&self, profile_id: &str) -> Result<()> {
        let mut index = self.load_index()?;

        if !index.contains(profile_id) {
            return Err(ProfileError::ProfileNotFound(profile_id.to_string()));
        }

        index.active_profile_id = Some(profile_id.to_string());
        self.save_index(&index)?;
        self.touch_profile(profile_id)?;
        Ok(())
    }

    /// Clear the active profile (for Guest mode logout).
    pub fn clear_active_profile_id(&self) -> Result<()> {
        let mut index = self.load_index()?;
        index.active_profile_id = None;
        self.save_index(&index)?;
        Ok(())
    }

    // =========================================================================
    // Profile CRUD
    // =========================================================================

    /// Create or update a profile.
    ///
    /// If the profile already exists, it will be updated with the new data.
    /// The profile is automatically added to the index.
    pub fn upsert_profile(&self, profile: &Profile) -> Result<()> {
        let profile_dir = self.get_profile_dir(&profile.id);
        fs::create_dir_all(&profile_dir)?;

        let profile_path = profile_dir.join(PROFILE_FILE);
        let existing = self.get_profile(&profile.id)?;
        let mut stored_profile = profile.clone();
        if let Some(existing_profile) = existing {
            stored_profile.created_at = existing_profile.created_at;
            if stored_profile.avatar.is_none() {
                stored_profile.avatar = existing_profile.avatar;
            }
            if stored_profile.original_avatar.is_none() {
                stored_profile.original_avatar = existing_profile.original_avatar;
            }
        }

        self.write_json_atomic(&profile_path, &stored_profile)?;

        let mut index = self.load_index()?;
        index.add(stored_profile.id.clone());

        if index.active_profile_id.is_none() {
            index.active_profile_id = Some(stored_profile.id.clone());
        }

        self.save_index(&index)?;
        Ok(())
    }

    /// Get a profile by ID.
    pub fn get_profile(&self, profile_id: &str) -> Result<Option<Profile>> {
        let profile_path = self.get_profile_dir(profile_id).join(PROFILE_FILE);

        if !profile_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&profile_path)?;
        let profile: Profile = serde_json::from_str(&content)?;
        Ok(Some(profile))
    }

    /// Find a profile by email address.
    ///
    /// Profile IDs are derived from normalized email addresses, so this
    /// performs a deterministic lookup and verifies the profile exists.
    pub fn find_profile_by_email(&self, email: &str) -> Result<Option<Profile>> {
        let normalized = email.trim();
        if normalized.is_empty() {
            return Ok(None);
        }

        let profile_id = Profile::id_from_email(normalized);
        self.get_profile(&profile_id)
    }

    /// Get the currently active profile.
    pub fn get_active_profile(&self) -> Result<Option<Profile>> {
        match self.get_active_profile_id()? {
            Some(id) => self.get_profile(&id),
            None => Ok(None),
        }
    }

    /// List all profiles.
    pub fn list_profiles(&self) -> Result<Vec<Profile>> {
        let index = self.load_index()?;
        let mut profiles = Vec::new();

        for id in &index.profile_ids {
            if let Some(profile) = self.get_profile(id)? {
                profiles.push(profile);
            }
        }

        // Sort by last used (most recent first)
        profiles.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));

        Ok(profiles)
    }

    /// Delete a profile and all its data.
    ///
    /// Returns an error if trying to delete the last profile.
    pub fn delete_profile(&self, profile_id: &str) -> Result<()> {
        let mut index = self.load_index()?;

        if index.profile_ids.len() <= 1 && index.contains(profile_id) {
            return Err(ProfileError::CannotDeleteLastProfile);
        }

        if !index.contains(profile_id) {
            return Err(ProfileError::ProfileNotFound(profile_id.to_string()));
        }

        // Remove profile directory
        let profile_dir = self.get_profile_dir(profile_id);
        if profile_dir.exists() {
            fs::remove_dir_all(&profile_dir)?;
        }

        index.remove(profile_id);
        self.save_index(&index)?;

        Ok(())
    }

    /// Check if any profiles exist.
    pub fn has_profiles(&self) -> Result<bool> {
        let index = self.load_index()?;
        Ok(!index.profile_ids.is_empty())
    }

    /// Get the count of profiles.
    pub fn profile_count(&self) -> Result<usize> {
        let index = self.load_index()?;
        Ok(index.profile_ids.len())
    }

    fn touch_profile(&self, profile_id: &str) -> Result<()> {
        let Some(mut profile) = self.get_profile(profile_id)? else {
            return Ok(());
        };
        profile.touch();
        let profile_path = self.get_profile_dir(profile_id).join(PROFILE_FILE);
        self.write_json_atomic(&profile_path, &profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn temp_store() -> ProfileStore {
        let temp_dir = tempdir().unwrap();
        let root = temp_dir.path().to_path_buf();
        std::mem::forget(temp_dir);
        ProfileStore::with_base_dir(root.join(STORAGE_DIR)).unwrap()
    }

    #[test]
    fn test_profile_crud() {
        let store = temp_store();

        // Create profile
        let profile = Profile::new("test@gmail.com", "Test User", None, None);
        store.upsert_profile(&profile).unwrap();

        // Verify it exists
        let loaded = store.get_profile(&profile.id).unwrap().unwrap();
        assert_eq!(loaded.email, "test@gmail.com");
        assert_eq!(loaded.name, "Test User");

        // Should be active (first profile)
        assert_eq!(
            store.get_active_profile_id().unwrap(),
            Some(profile.id.clone())
        );
    }

    #[test]
    fn test_provider_key_path() {
        let store = temp_store();
        let path = store.get_provider_key_path("profile1", "imgbb");
        assert!(path.ends_with("profile1/imgbb_key.json"));
    }
}
