/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { google } from "@/lib";
import { invoke } from "@tauri-apps/api/core";

/**
 * Uploads an image file path to ImgBB and returns the hosted URL.
 * Uses the native Tauri command for multipart file upload (no base64 conversion).
 * @param imagePath - Absolute local image path
 * @param apiKey - The ImgBB API key
 * @returns The public URL of the uploaded image
 */
export async function uploadToImgBB(
  imagePath: string,
  apiKey: string,
): Promise<string> {
  return invoke<string>("upload_image_to_imgbb", { imagePath, apiKey });
}

/**
 * Generates a Google Lens search URL for the given image URL.
 * @param imageUrl - The public URL of the image
 * @returns The full Google Lens URL
 */
export function generateLensUrl(imageUrl: string): string {
  const params = new URLSearchParams();
  params.append("url", imageUrl);
  params.append("ep", "subb");
  params.append("re", "df");
  params.append("s", "4");
  params.append("hl", "en");
  params.append("gl", "US");

  return `${google.lens}/uploadbyurl?${params.toString()}`;
}
