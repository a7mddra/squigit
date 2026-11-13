use rune::color::hex;
use rune::{col, icon, row, text, Color, El};
use rune_kit::{Kit, PaletteFocus, PaletteSeed};

use super::{App, Page};

pub static IMAGE: rune::Icon = rune::Icon {
    name: "image-hosting",
    view: 24.0,
    body: r#"<rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="8.5" cy="8.5" r="1.5" fill="none" stroke="currentColor" stroke-width="2"/><path d="m21 15-5-5L5 21" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static EYE: rune::Icon = rune::Icon {
    name: "show-api-key",
    view: 24.0,
    body: r#"<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="2"/>"#,
};

pub static EYE_OFF: rune::Icon = rune::Icon {
    name: "hide-api-key",
    view: 24.0,
    body: r#"<path d="m3 3 18 18M10.6 10.6a2 2 0 0 0 2.8 2.8M9.9 5.2A11 11 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3 3.8M6.2 6.2A20 20 0 0 0 2 12s3.5 7 10 7a12 12 0 0 0 5.8-1.8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static TRASH: rune::Icon = rune::Icon {
    name: "delete-api-key",
    view: 24.0,
    body: r#"<path d="M3 6h18M5 6l1 14h12l1-14M9 6V3h6v3m-5 4v6m4-6v6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static INFO: rune::Icon = rune::Icon {
    name: "image-hosting-info",
    view: 24.0,
    body: r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 11v5m0-9v.01" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>"#,
};

pub static EXTERNAL: rune::Icon = rune::Icon {
    name: "open-imgbb-dashboard",
    view: 24.0,
    body: r#"<path d="M14 3h7v7m0-7L10 14m-1-9H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static PACKAGE: rune::Icon = rune::Icon {
    name: "ocr-model-package",
    view: 24.0,
    body: r#"<path d="M21 8l-9 5-9-5m9 5v9M3 7l9-5 9 5v10l-9 5-9-5V7zM7.5 4.5l9 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static PACKAGE_PLUS: rune::Icon = rune::Icon {
    name: "ocr-model-package-plus",
    view: 24.0,
    body: r#"<path d="M21 8l-9 5-9-5m9 5v9M3 7l9-5 9 5v5M12 22l-9-5V7m4.5-2.5l9 5M16 19h6m-3-3v6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub fn kit(dark: bool) -> Kit {
    let (bg, fg) = if dark {
        (hx("#111212"), hx("#e9e9e7"))
    } else {
        (hx("#f6f6f5"), hx("#1b1b1b"))
    };
    Kit::from_palette_focus(
        PaletteSeed {
            bg,
            fg,
            accent: hx("#c9c9c9"),
        },
        PaletteFocus::Ink,
    )
}

fn hx(value: &str) -> Color {
    hex(value).unwrap()
}

pub(super) fn chrome(s: &App, k: &Kit, content: El) -> El {
    row()
        .key("settings")
        .fill(k.c.page)
        .child(
            col()
                .key("sidebar")
                .w(208.0)
                .h_full()
                .fill(k.c.layer_01)
                .pt(12.0)
                .padding(8.0)
                .gap(16.0)
                .child(
                    text("Settings")
                        .key("title")
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base)
                        .px(8.0),
                )
                .child(
                    col()
                        .gap(4.0)
                        .child(nav(s, k, Page::Models, "models", "OCR Models", &PACKAGE))
                        .child(nav(
                            s,
                            k,
                            Page::ImageHosting,
                            "image-hosting",
                            "Image hosting",
                            &IMAGE,
                        )),
                ),
        )
        .child(rune_kit::Divider::vertical().build(k))
        .child(content)
}

fn nav(s: &App, k: &Kit, page: Page, key: &str, label: &str, glyph: &'static rune::Icon) -> El {
    let item = row()
        .key(format!("nav/{key}"))
        .role("tab")
        .label(label)
        .checked(s.page == page)
        .focusable(true)
        .h(32.0)
        .px(8.0)
        .gap(8.0)
        .center_y()
        .radius(4.0)
        .hover(k.c.hover)
        .child(icon(glyph, 15.0).color(k.c.icon_base))
        .child(text(label).font(k.ty.ui_medium).color(k.c.text_base))
        .on_press(move |s: &mut App| s.show_page(page));
    if s.page == page {
        item.fill(k.c.layer_02)
    } else {
        item
    }
}

pub fn section(k: &Kit, hair: f32, title: &str, rows: Vec<El>) -> El {
    let count = rows.len();
    let mut card = col()
        .key("card")
        .w_full()
        .px(16.0)
        .fill(k.c.layer_01)
        .radius(16.0)
        .border(hair, k.c.border_muted);
    for (index, item) in rows.into_iter().enumerate() {
        if index > 0 {
            card = card.child(
                rune::el()
                    .key(format!("divider/{index}"))
                    .h(hair)
                    .fill(k.c.border_muted)
                    .a11y_hidden(),
            );
        }
        card = card.child(item);
    }
    col()
        .key(format!("section/{title}"))
        .w_full()
        .gap(16.0)
        .child(
            text(title)
                .key("label")
                .font(rune::TextStyle::new(14.0, 450.0))
                .color(k.c.text_base),
        )
        .child(card.label(format!("{title}, {count} models")))
}
