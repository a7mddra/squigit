/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";
import { Profile, UserData } from "./tauri.types";
export * from "./tauri.types";

export const commands = {
  // Image Processing
  cacheAvatar: (url: string, profileId?: string) =>
    invoke<string>("cache_avatar", { url, profileId }),
  processImagePath: (path: string) =>
    invoke<{ hash: string; path: string }>("process_image_path", { path }),
  getInitialImage: () =>
    invoke<{ hash: string; path: string } | null>("get_initial_image"),

  // Auth & Keys (profile-aware)
  getApiKey: (provider: "google ai studio" | "imgbb", profileId: string) =>
    invoke<string>("get_api_key", { provider, profileId }),
  setApiKey: (
    provider: "google ai studio" | "imgbb",
    key: string,
    profileId: string,
  ) =>
    invoke("encrypt_and_save", {
      provider,
      plaintext: key,
      profileId,
    }),
  startGoogleAuth: () => invoke("start_google_auth"),
  cancelGoogleAuth: () => invoke("cancel_google_auth"),
  logout: () => invoke("logout"),
  getUserData: () => invoke<UserData>("get_user_data"),

  // Profile Management
  getActiveProfile: () => invoke<Profile | null>("get_active_profile"),
  getActiveProfileId: () => invoke<string | null>("get_active_profile_id"),
  listProfiles: () => invoke<Profile[]>("list_profiles"),
  setActiveProfile: (profileId: string) =>
    invoke("set_active_profile", { profileId }),
  deleteProfile: (profileId: string) => invoke("delete_profile", { profileId }),
  hasProfiles: () => invoke<boolean>("has_profiles"),
  getProfileCount: () => invoke<number>("get_profile_count"),

  // Window Mgmt
  setBackgroundColor: (color: string) =>
    invoke("set_background_color", { color }),

  // OCR Model Management
  cancelDownloadOcrModel: (modelId: string) =>
    invoke("cancel_download_ocr_model", { modelId }),

  // AI Runtime Control
  quickAnswerProviderRequest: (channelId: string) =>
    invoke("quick_answer_gemini_request", { channelId }),

  // UI audio
  playUiSound: (effect: "dialog-warning" = "dialog-warning") =>
    invoke("play_ui_sound", { effect }),

  // Utils
  openExternalUrl: (url: string) => invoke("open_external_url", { url }),
  getAppConstants: () =>
    invoke<import("./tauri.types").AppConstants>("get_app_constants"),
};
