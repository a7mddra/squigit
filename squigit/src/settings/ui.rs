use rune::color::hex;
use rune::{col, icon, row, text, Color, El};
use rune_kit::{Kit, PaletteFocus, PaletteSeed};

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

pub fn chrome(k: &Kit, content: El) -> El {
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
                    row()
                        .key("nav/models")
                        .role("tab")
                        .label("OCR Models")
                        .checked(true)
                        .h(32.0)
                        .px(8.0)
                        .gap(8.0)
                        .center_y()
                        .radius(4.0)
                        .fill(k.c.layer_02)
                        .child(icon(&PACKAGE, 15.0).color(k.c.icon_base))
                        .child(text("OCR Models").font(k.ty.ui_medium).color(k.c.text_base)),
                ),
        )
        .child(rune_kit::Divider::vertical().build(k))
        .child(content)
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
