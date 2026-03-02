// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use tauri::State;

use crate::services::audio::{UiSoundEffect, UiSoundPlayer};

#[tauri::command]
pub fn play_ui_sound(
    effect: Option<String>,
    sound_player: State<'_, UiSoundPlayer>,
) -> Result<(), String> {
    let parsed_effect = UiSoundEffect::from_input(effect.as_deref())?;
    sound_player.play(parsed_effect)
}
