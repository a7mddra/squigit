// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use image::{imageops, GenericImageView};

struct Lcg(u64);
impl Lcg {
    #[inline]
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    #[inline]
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo + 1 {
            return lo;
        }
        lo + (self.next() as u32 % (hi - lo))
    }
}

pub fn detect_image_tone_from_bytes(bytes: &[u8]) -> Option<String> {
    let img = image::load_from_memory(bytes).ok()?;
    let (width, height) = img.dimensions();

    if width == 0 || height == 0 {
        return Some("d".to_string());
    }

    let max_dim = 256;
    let thumb = img.thumbnail(max_dim, max_dim);
    let blurred = imageops::blur(&thumb, 1.5);

    let (w, h) = blurred.dimensions();
    if w == 0 || h == 0 {
        return Some("d".to_string());
    }

    let srgb_to_linear = |c: u8| -> f32 {
        let f = c as f32 / 255.0;
        if f <= 0.04045 {
            f / 12.92
        } else {
            ((f + 0.055) / 1.055).powf(2.4)
        }
    };

    let get_luminance = |r: u8, g: u8, b: u8| -> f32 {
        0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)
    };

    let mut sum_lum = 0.0;
    let mut count = 0;
    for pixel in blurred.pixels() {
        if pixel[3] > 128 {
            // Ignore mostly transparent pixels
            sum_lum += get_luminance(pixel[0], pixel[1], pixel[2]);
            count += 1;
        }
    }

    if count == 0 {
        return Some("l".to_string());
    }

    let global_mean = sum_lum / count as f32;
    if global_mean <= 0.05 {
        return Some("d".to_string());
    }
    if global_mean >= 0.75 {
        return Some("l".to_string());
    }

    let mut rng = Lcg(0xDEAD_BEEF_CAFE_1337);
    let grid_size = 12;
    let spc = 8;

    let thresh = 0.179;
    let mut dark_score = 0.0;
    let mut light_score = 0.0;

    for gy in 0..grid_size {
        for gx in 0..grid_size {
            let cx0 = gx * w / grid_size;
            let cx1 = ((gx + 1) * w / grid_size).max(cx0 + 1);
            let cy0 = gy * h / grid_size;
            let cy1 = ((gy + 1) * h / grid_size).max(cy0 + 1);

            for _ in 0..spc {
                let x = rng.range(cx0, cx1).min(w.saturating_sub(1));
                let y = rng.range(cy0, cy1).min(h.saturating_sub(1));

                if blurred.get_pixel(x, y)[3] < 128 {
                    continue;
                }

                let mut local_dark = 0;
                let mut local_light = 0;
                let mut valid_neighbors = 0;

                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = (x as i32 + dx) as u32;
                        let ny = (y as i32 + dy) as u32;
                        if nx < w && ny < h {
                            let px = blurred.get_pixel(nx, ny);
                            if px[3] > 128 {
                                let l = get_luminance(px[0], px[1], px[2]);
                                if l < thresh {
                                    local_dark += 1;
                                } else {
                                    local_light += 1;
                                }
                                valid_neighbors += 1;
                            }
                        }
                    }
                }

                if valid_neighbors > 0 {
                    let confidence =
                        (local_dark as f32 - local_light as f32).abs() / valid_neighbors as f32;
                    let is_node_dark = local_dark >= local_light;

                    if is_node_dark {
                        dark_score += 1.0 + confidence;
                    } else {
                        light_score += 1.0 + confidence;
                    }
                }
            }
        }
    }

    if dark_score >= light_score {
        Some("d".to_string())
    } else {
        Some("l".to_string())
    }
}
