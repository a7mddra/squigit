use rune::color::hex;
use rune::tok::{motion, radius, space};
use rune::{col, el, icon, row, text, Align, Color, Drag, El, Icon, Phase};
use rune_kit::{Divider, Kit, PaletteFocus, PaletteSeed};

use crate::settings::{App, Page};

pub const ACCENTS: [&str; 6] = [
    "#f5f5f5", "#c9c9c9", "#9e9e9e", "#737373", "#454545", "#262626",
];

const SIDEBAR_W: f32 = 208.0;

pub fn hx(s: &str) -> Color {
    hex(s).unwrap()
}

pub fn speed_of(v: f32) -> f32 {
    2f32.powf(v * 2.0 - 1.0)
}

pub fn kit(dark: bool, accent_index: usize) -> Kit {
    let (bg, fg) = if dark {
        (hx("#111212"), hx("#e9e9e7"))
    } else {
        (hx("#f6f6f5"), hx("#1b1b1b"))
    };
    let accent = hx(ACCENTS[accent_index.min(ACCENTS.len() - 1)]);
    let mut k = Kit::from_palette_focus(PaletteSeed { bg, fg, accent }, PaletteFocus::Ink);
    k.c.knob_on = if accent.luminance() > bg.luminance() {
        bg
    } else {
        fg
    };
    k
}

pub fn chrome(s: &App, k: &Kit, content: El) -> El {
    col().key("window").fill(k.c.page).child(
        row()
            .key("body")
            .grow(1.0)
            .min_h(0.0)
            .child(sidebar(s, k))
            .child(Divider::vertical().build(k))
            .child(
                col()
                    .key("content")
                    .grow(1.0)
                    .min_h(0.0)
                    .h_full()
                    .child(content)
                    .child(Divider::horizontal().build(k))
                    .child(footer(s, k)),
            ),
    )
}

const PAGES: [(Page, &str, &'static Icon); 5] = [
    (Page::General, "General", &rune_kit::glyphs::SPARKLE),
    (Page::Appearance, "Appearance", &rune_kit::glyphs::STAR),
    (Page::Editor, "Editor", &rune_kit::glyphs::CURSOR_TEXT),
    (Page::Advanced, "Advanced", &rune_kit::glyphs::LOCK),
    (
        Page::ConfigFile,
        "Config file",
        &rune_kit::glyphs::FILE_CODE,
    ),
];

fn sidebar(s: &App, k: &Kit) -> El {
    let mut nav = col().key("nav").gap(2.0).padding(space::S2);
    for (page, label, glyph) in PAGES {
        let on = s.page == page;
        nav = nav.child(
            row()
                .key(format!("nav/{}", page.slug()))
                .role("tab")
                .label(label)
                .focusable(true)
                .h(32.0)
                .px(space::S2)
                .gap(space::S2)
                .center_y()
                .radius(radius::SM)
                .fill(if on { k.c.layer_02 } else { Color::TRANSPARENT })
                .hover(if on { Color::TRANSPARENT } else { k.c.hover })
                .on_press(move |s: &mut App| s.page = page)
                .child(icon(glyph, 15.0).color(if on { k.c.icon_base } else { k.c.icon_muted }))
                .child(text(label).font(k.ty.ui_medium).color(if on {
                    k.c.text_base
                } else {
                    k.c.text_muted
                })),
        );
    }
    col()
        .key("sidebar")
        .w(SIDEBAR_W)
        .h_full()
        .fill(k.c.layer_01)
        .pt(space::S3)
        .pb(space::S2)
        .child(
            row().key("title-row").px(16.0).pb(space::S3).child(
                text("Settings")
                    .key("app-title")
                    .font(k.ty.ui_medium)
                    .color(k.c.text_base),
            ),
        )
        .child(nav)
}

fn footer(s: &App, k: &Kit) -> El {
    let status = if let Some(e) = &s.error {
        text(format!("Save failed: {e}"))
            .key("status")
            .font(k.ty.small)
            .color(k.c.danger.fg)
            .shrink()
            .truncate()
    } else if let Some(at) = &s.saved_at {
        text(format!("Saved {at}"))
            .key("status")
            .font(k.ty.small)
            .color(k.c.text_faint)
    } else {
        text("Not saved yet")
            .key("status")
            .font(k.ty.small)
            .color(k.c.text_faint)
    };
    row()
        .key("footer")
        .h(34.0)
        .px(space::S3)
        .gap(space::S3)
        .center_y()
        .fill(k.c.layer_01)
        .child(
            text(s.path.display().to_string())
                .key("path")
                .font(k.ty.small.mono())
                .color(k.c.text_muted)
                .grow(1.0)
                .shrink()
                .truncate_head(),
        )
        .child(status)
}

pub fn section(k: &Kit, hair: f32, key: &str, title: &str, rows: Vec<El>) -> El {
    let n = rows.len();
    let mut card = col()
        .key("card")
        .w_full()
        .padding(space::S1)
        .fill(k.c.layer_01)
        .radius(radius::LG)
        .border(hair, k.c.border_muted);
    for (i, r) in rows.into_iter().enumerate() {
        card = card.child(r);
        if i + 1 < n {
            card = card.child(
                el().key(format!("sep-{i}"))
                    .h(hair)
                    .ml(space::S3)
                    .mr(space::S3)
                    .fill(k.c.border_muted)
                    .a11y_hidden(),
            );
        }
    }
    col()
        .key(key.to_owned())
        .w_full()
        .gap(space::S2)
        .child(
            text(title)
                .key("header")
                .font(k.ty.small_medium)
                .color(k.c.text_muted)
                .ml(space::S2),
        )
        .child(card)
}

pub fn setting_row(k: &Kit, key: &str, label: &str, desc: Option<&str>, control: El) -> El {
    row()
        .key(key.to_owned())
        .w_full()
        .min_h(44.0)
        .px(space::S3)
        .py(10.0)
        .gap(space::S3)
        .center_y()
        .child(words(k, label, desc).grow(1.0).shrink())
        .child(control)
}

pub fn setting_col(k: &Kit, key: &str, label: &str, desc: Option<&str>, control: El) -> El {
    col()
        .key(key.to_owned())
        .w_full()
        .px(space::S3)
        .py(10.0)
        .gap(space::S2)
        .child(words(k, label, desc))
        .child(control)
}

fn words(k: &Kit, label: &str, desc: Option<&str>) -> El {
    let mut w = col()
        .key("words")
        .gap(2.0)
        .child(text(label).key("label").font(k.ty.ui).color(k.c.text_base));
    if let Some(d) = desc {
        w = w.child(
            text(d)
                .key("desc")
                .font(k.ty.small.lh(16.0))
                .color(k.c.text_muted),
        );
    }
    w
}

pub fn swatches(k: &Kit, hair: f32, current: usize) -> El {
    let (d, gap, ring_w) = (18.0, 12.0, 2.0);
    let ring = d + 2.0 * (ring_w + 2.0);
    let current = current.min(ACCENTS.len() - 1);
    let mut r = row()
        .key("swatches")
        .role("radiogroup")
        .label("Accent")
        .gap(gap)
        .motion(motion::SNAPPY);
    r = r.child(
        el().key("ring")
            .size(ring, ring)
            .absolute(
                current as f32 * (d + gap) - (ring - d) / 2.0,
                -(ring - d) / 2.0,
            )
            .radius(ring / 2.0)
            .border(ring_w, k.c.border_focus)
            .lead(motion::LEAD, motion::DEFAULT)
            .a11y_hidden(),
    );
    for (i, grey) in ACCENTS.into_iter().enumerate() {
        r = r.child(
            el().key(format!("accent-{i}"))
                .role("radio")
                .label(format!("Accent {}", i + 1))
                .checked(i == current)
                .focusable(true)
                .size(d, d)
                .radius(d / 2.0)
                .fill(hx(grey))
                .border(hair, k.c.border_base)
                .on_press(move |s: &mut App| {
                    s.cfg.appearance.accent_index = i;
                    s.touch();
                }),
        );
    }
    r
}

#[allow(clippy::too_many_arguments)]
pub fn slider(
    k: &Kit,
    hair: f32,
    key: &str,
    label: &str,
    v: f32,
    min: f32,
    max: f32,
    step: f32,
    get: fn(&App) -> f32,
    set: fn(&mut App, f32),
    fmt: fn(f32) -> String,
    w: f32,
) -> El {
    let (h, kw, bar) = (22.0, 16.0, 4.0);
    let travel = w - kw;
    let frac = move |value: f32| ((value - min) / (max - min)).clamp(0.0, 1.0);
    let to_value = move |x: f32| min + ((x - kw / 2.0) / travel).clamp(0.0, 1.0) * (max - min);
    let f = frac(v);
    row()
        .key(key.to_owned())
        .gap(space::S3)
        .center_y()
        .child(
            el().key(format!("{key}/control"))
                .role("slider")
                .label(label)
                .range_value(f64::from(v), f64::from(min), f64::from(max))
                .range_step(f64::from(step))
                .a11y_value(fmt(v))
                .size(w, h)
                .motion(motion::SNAPPY)
                .on_drag(move |s: &mut App, d: &mut Drag, tx| {
                    let value = get(s);
                    match d.phase {
                        Phase::Down => {
                            d.data = value;
                            let knob = kw / 2.0 + frac(value) * travel;
                            if (d.x - knob).abs() > kw / 2.0 + 2.0 {
                                set(s, to_value(d.x));
                                s.touch();
                            }
                        }
                        Phase::Move => {
                            let knob = kw / 2.0 + frac(d.data) * travel;
                            let grab = if (d.start_x - knob).abs() <= kw / 2.0 + 2.0 {
                                d.start_x - knob
                            } else {
                                0.0
                            };
                            set(s, to_value(d.x - grab));
                            tx.instant();
                        }
                        Phase::Up => {
                            if d.cancelled() {
                                set(s, d.data);
                            }
                            s.touch();
                        }
                    }
                })
                .on_step(move |s: &mut App, steps, _| {
                    set(s, (get(s) + steps as f32 * step).clamp(min, max));
                    s.touch();
                })
                .child(
                    el().key("track")
                        .size(w, bar)
                        .absolute(0.0, (h - bar) / 2.0)
                        .radius(bar / 2.0)
                        .fill(k.c.layer_03)
                        .a11y_hidden(),
                )
                .child(
                    el().key("fill")
                        .size(kw / 2.0 + f * travel, bar)
                        .absolute(0.0, (h - bar) / 2.0)
                        .radius(bar / 2.0)
                        .fill(k.c.bg_accent)
                        .a11y_hidden(),
                )
                .child(
                    el().key("knob")
                        .size(kw, kw)
                        .absolute(f * travel, (h - kw) / 2.0)
                        .radius(kw / 2.0)
                        .fill(k.c.knob)
                        .border(hair, k.c.border_strong)
                        .a11y_hidden(),
                ),
        )
        .child(
            text(fmt(v))
                .key("value")
                .w(56.0)
                .font(k.ty.small.mono())
                .color(k.c.text_muted)
                .align(Align::End)
                .a11y_hidden(),
        )
}
