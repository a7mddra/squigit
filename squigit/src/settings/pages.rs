use rune::tok::{motion, space};
use rune::{col, row, text, El, ScrollSpec};
use rune_kit::glyphs::COPY;
use rune_kit::{
    Button, ButtonVariant, Checkbox, IconButton, Kit, RadioGroup, RadioItem, SegmentedControl,
    Select, Switch, TextArea, TextInput,
};

use super::config::CaptureMode;
use super::ui;
use crate::settings::{App, Page};

pub fn page(s: &App, k: &Kit, layer: &mut Vec<El>, hair: f32) -> El {
    let (title, desc, sections) = match s.page {
        Page::General => (
            "General",
            "Screenshot defaults and demonstration controls.",
            general(s, k, layer, hair),
        ),
        Page::Appearance => (
            "Appearance",
            "Colours, spacing and motion.",
            appearance(s, k, layer, hair),
        ),
        Page::Editor => ("Editor", "How text behaves.", editor(s, k, layer, hair)),
        Page::Advanced => (
            "Advanced",
            "Connections and diagnostics.",
            advanced(s, k, layer, hair),
        ),
        Page::ConfigFile => (
            "Config file",
            "Everything above, as it is saved.",
            file(s, k, layer, hair),
        ),
    };
    col()
        .key(format!("page/{}", s.page.slug()))
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .px(space::S6)
        .pt(20.0)
        .pb(space::S6)
        .child(
            col()
                .key("inner")
                .max_w(560.0)
                .gap(space::S6)
                .child(
                    col()
                        .key("header")
                        .gap(2.0)
                        .child(
                            text(title)
                                .key("title")
                                .font(k.ty.body_medium)
                                .color(k.c.text_base),
                        )
                        .child(
                            text(desc)
                                .key("desc")
                                .font(k.ty.small.lh(16.0))
                                .color(k.c.text_muted),
                        ),
                )
                .children(sections),
        )
}

fn general(s: &App, k: &Kit, layer: &mut Vec<El>, hair: f32) -> Vec<El> {
    vec![
        ui::section(
            k,
            hair,
            "capture",
            "Screenshot",
            vec![
                select_row(
                    s,
                    k,
                    layer,
                    "capture-mode",
                    "Default capture type",
                    Some("Used when taking a screenshot from the tray."),
                    &["Squiggle", "Traditional"],
                    s.cfg.general.capture_mode.label(),
                    |s: &mut App, value: &str| {
                        s.cfg.general.capture_mode = match value {
                            "Traditional" => CaptureMode::Traditional,
                            _ => CaptureMode::Squiggle,
                        };
                    },
                ),
                switch_row(
                    k,
                    "auto-ocr",
                    "Auto OCR",
                    Some("Start recognition when you begin drawing a selection."),
                    s.cfg.general.auto_ocr,
                    |s: &mut App| s.cfg.general.auto_ocr = !s.cfg.general.auto_ocr,
                ),
            ],
        ),
        ui::section(
            k,
            hair,
            "profile",
            "Profile",
            vec![ui::setting_row(
                k,
                "nickname",
                "Nickname",
                Some("Shown in greetings."),
                TextInput::new(&s.edits.nickname)
                    .placeholder("Your name")
                    .width(Some(200.0))
                    .on_edit(|s: &mut App| &mut s.edits.nickname)
                    .on_change(|s: &mut App, _| {
                        s.cfg.general.nickname = s.edits.nickname.text.clone();
                        s.touch();
                    })
                    .build(k),
            )],
        ),
        ui::section(
            k,
            hair,
            "startup",
            "Startup",
            vec![
                switch_row(
                    k,
                    "launch-at-login",
                    "Launch at login",
                    Some("Start quietly when you sign in."),
                    s.cfg.general.launch_at_login,
                    |s: &mut App| s.cfg.general.launch_at_login = !s.cfg.general.launch_at_login,
                ),
                switch_row(
                    k,
                    "check-updates",
                    "Check for updates",
                    Some("Look for new versions once a week."),
                    s.cfg.general.check_updates,
                    |s: &mut App| s.cfg.general.check_updates = !s.cfg.general.check_updates,
                ),
            ],
        ),
        ui::section(
            k,
            hair,
            "navigation",
            "Navigation",
            vec![select_row(
                s,
                k,
                layer,
                "default-page",
                "Default page",
                Some("Where the app opens."),
                &["Home", "Editor", "Settings"],
                &s.cfg.general.default_page,
                |s: &mut App, v: &str| s.cfg.general.default_page = v.to_owned(),
            )],
        ),
    ]
}

fn appearance(s: &App, k: &Kit, layer: &mut Vec<El>, hair: f32) -> Vec<El> {
    let theme_idx = match s.cfg.appearance.theme.as_str() {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    };
    vec![
        ui::section(
            k,
            hair,
            "theme",
            "Theme",
            vec![
                ui::setting_row(
                    k,
                    "theme",
                    "Theme",
                    Some("Follow the system or pick a fixed look."),
                    SegmentedControl::new(&["System", "Light", "Dark"], Some(theme_idx))
                        .label("Theme")
                        .width(Some(220.0))
                        .on_pick(|s: &mut App, i, tx| {
                            s.cfg.appearance.theme =
                                ["system", "light", "dark"][i.min(2)].to_owned();
                            s.touch();
                            tx.color_motion(motion::CALM);
                        })
                        .build(k),
                ),
                ui::setting_row(
                    k,
                    "accent",
                    "Accent",
                    Some("A grey for tracks, knobs and highlights."),
                    ui::swatches(k, hair, s.cfg.appearance.accent_index),
                ),
            ],
        ),
        ui::section(
            k,
            hair,
            "layout",
            "Layout",
            vec![select_row(
                s,
                k,
                layer,
                "density",
                "Density",
                Some("How much air between controls."),
                &["Compact", "Comfortable", "Spacious"],
                &s.cfg.appearance.density,
                |s: &mut App, v: &str| s.cfg.appearance.density = v.to_owned(),
            )],
        ),
        ui::section(
            k,
            hair,
            "motion",
            "Motion",
            vec![
                switch_row(
                    k,
                    "reduce-motion",
                    "Reduce motion",
                    Some("Springs settle instantly."),
                    s.cfg.appearance.reduce_motion,
                    |s: &mut App| s.cfg.appearance.reduce_motion = !s.cfg.appearance.reduce_motion,
                ),
                ui::setting_row(
                    k,
                    "animation-speed",
                    "Animation speed",
                    Some("How fast springs play, 0.25× to 2×."),
                    ui::slider(
                        k,
                        hair,
                        "animation-speed",
                        "Animation speed",
                        s.cfg.appearance.animation_speed,
                        0.0,
                        1.0,
                        0.05,
                        get_anim,
                        set_anim,
                        fmt_speed,
                        180.0,
                    ),
                ),
            ],
        ),
    ]
}

fn editor(s: &App, k: &Kit, _layer: &mut Vec<El>, hair: f32) -> Vec<El> {
    let tab_idx = [2usize, 4, 8]
        .iter()
        .position(|&t| t == s.cfg.editor.tab_width)
        .unwrap_or(1);
    vec![
        ui::section(
            k,
            hair,
            "text",
            "Text",
            vec![
                ui::setting_row(
                    k,
                    "font-size",
                    "Font size",
                    Some("Points, 10 to 20."),
                    ui::slider(
                        k,
                        hair,
                        "font-size",
                        "Font size",
                        s.cfg.editor.font_size,
                        10.0,
                        20.0,
                        1.0,
                        get_font_size,
                        set_font_size,
                        fmt_pt,
                        180.0,
                    ),
                ),
                ui::setting_row(
                    k,
                    "tab-width",
                    "Tab width",
                    Some("Columns a tab stops at."),
                    SegmentedControl::new(&["2", "4", "8"], Some(tab_idx))
                        .label("Tab width")
                        .width(Some(150.0))
                        .on_pick(|s: &mut App, i, _| {
                            s.cfg.editor.tab_width = [2, 4, 8][i.min(2)];
                            s.touch();
                        })
                        .build(k),
                ),
            ],
        ),
        ui::section(
            k,
            hair,
            "indentation",
            "Indentation",
            vec![row()
                .key("indent")
                .w_full()
                .min_h(44.0)
                .px(space::S3)
                .py(10.0)
                .center_y()
                .child(
                    RadioGroup::new()
                        .label("Indent with")
                        .item(
                            RadioItem::new("Spaces", s.cfg.editor.indent_with_spaces).on_select(
                                |s: &mut App| {
                                    s.cfg.editor.indent_with_spaces = true;
                                    s.touch();
                                },
                            ),
                        )
                        .item(
                            RadioItem::new("Tabs", !s.cfg.editor.indent_with_spaces).on_select(
                                |s: &mut App| {
                                    s.cfg.editor.indent_with_spaces = false;
                                    s.touch();
                                },
                            ),
                        )
                        .build(k),
                )],
        ),
        ui::section(
            k,
            hair,
            "behavior",
            "Behavior",
            vec![
                checkbox_row(
                    k,
                    "word-wrap",
                    "Word wrap",
                    s.cfg.editor.word_wrap,
                    |s: &mut App| {
                        s.cfg.editor.word_wrap = !s.cfg.editor.word_wrap;
                    },
                ),
                checkbox_row(
                    k,
                    "line-numbers",
                    "Line numbers",
                    s.cfg.editor.line_numbers,
                    |s: &mut App| {
                        s.cfg.editor.line_numbers = !s.cfg.editor.line_numbers;
                    },
                ),
                checkbox_row(
                    k,
                    "spellcheck",
                    "Spellcheck",
                    s.cfg.editor.spellcheck,
                    |s: &mut App| {
                        s.cfg.editor.spellcheck = !s.cfg.editor.spellcheck;
                    },
                ),
            ],
        ),
        ui::section(
            k,
            hair,
            "prompt",
            "Prompt",
            vec![ui::setting_col(
                k,
                "prompt-template",
                "Prompt template",
                Some("Sent as the first message of every chat."),
                TextArea::new(&s.edits.prompt)
                    .placeholder("System prompt")
                    .label("Prompt template")
                    .width(None)
                    .rows(3)
                    .auto_grow(3, 8)
                    .on_edit(|s: &mut App| &mut s.edits.prompt)
                    .on_change(|s: &mut App, _| {
                        s.cfg.editor.prompt_template = s.edits.prompt.text.clone();
                        s.touch();
                    })
                    .build(k),
            )],
        ),
    ]
}

fn advanced(s: &App, k: &Kit, layer: &mut Vec<El>, hair: f32) -> Vec<El> {
    vec![
        ui::section(
            k,
            hair,
            "connection",
            "Connection",
            vec![
                ui::setting_row(
                    k,
                    "endpoint",
                    "Endpoint",
                    Some("Where requests go."),
                    TextInput::new(&s.edits.endpoint)
                        .placeholder("https://…")
                        .width(Some(240.0))
                        .on_edit(|s: &mut App| &mut s.edits.endpoint)
                        .on_change(|s: &mut App, _| {
                            s.cfg.advanced.endpoint = s.edits.endpoint.text.clone();
                            s.touch();
                        })
                        .build(k),
                ),
                ui::setting_row(
                    k,
                    "timeout",
                    "Timeout",
                    Some("Give up after this long, 5 to 120 seconds."),
                    ui::slider(
                        k,
                        hair,
                        "timeout",
                        "Timeout",
                        s.cfg.advanced.timeout_secs,
                        5.0,
                        120.0,
                        5.0,
                        get_timeout,
                        set_timeout,
                        fmt_secs,
                        180.0,
                    ),
                ),
                switch_row(
                    k,
                    "verify-tls",
                    "Verify TLS",
                    Some("Reject self-signed certificates."),
                    s.cfg.advanced.verify_tls,
                    |s: &mut App| s.cfg.advanced.verify_tls = !s.cfg.advanced.verify_tls,
                ),
            ],
        ),
        ui::section(
            k,
            hair,
            "diagnostics",
            "Diagnostics",
            vec![select_row(
                s,
                k,
                layer,
                "log-level",
                "Log level",
                Some("What lands in the log."),
                &["trace", "debug", "info", "warn", "error"],
                &s.cfg.advanced.log_level,
                |s: &mut App, v: &str| s.cfg.advanced.log_level = v.to_owned(),
            )],
        ),
        ui::section(
            k,
            hair,
            "experiments",
            "Experiments",
            vec![switch_row(
                k,
                "experimental",
                "Experimental features",
                Some("Try things that may change or break."),
                s.cfg.advanced.experimental,
                |s: &mut App| s.cfg.advanced.experimental = !s.cfg.advanced.experimental,
            )],
        ),
    ]
}

fn file(s: &App, k: &Kit, _layer: &mut Vec<El>, hair: f32) -> Vec<El> {
    let toml = s
        .cfg
        .to_toml_string()
        .unwrap_or_else(|e| format!("# couldn't serialize: {e}"));
    vec![
        ui::section(
            k,
            hair,
            "location",
            "Location",
            vec![row()
                .key("path-row")
                .w_full()
                .min_h(44.0)
                .px(space::S3)
                .py(10.0)
                .gap(space::S2)
                .center_y()
                .child(
                    col()
                        .key("words")
                        .gap(2.0)
                        .grow(1.0)
                        .shrink()
                        .child(text("Path").key("label").font(k.ty.ui).color(k.c.text_base))
                        .child(
                            text(s.path.display().to_string())
                                .key("path")
                                .font(k.ty.small.mono())
                                .color(k.c.text_muted)
                                .truncate_head(),
                        ),
                )
                .child(
                    IconButton::new(&COPY)
                        .label("Copy path")
                        .help("Copy the config file's path")
                        .on_press_tx(|s: &mut App, tx| tx.copy(s.path.display().to_string()))
                        .build(k),
                )],
        ),
        ui::section(
            k,
            hair,
            "actions",
            "Actions",
            vec![row()
                .key("buttons")
                .w_full()
                .min_h(44.0)
                .px(space::S3)
                .py(10.0)
                .gap(space::S2)
                .center_y()
                .child(
                    Button::new("Save now")
                        .variant(ButtonVariant::Submit)
                        .on_press_tx(|s: &mut App, _| s.save())
                        .build(k),
                )
                .child(
                    Button::new("Reload")
                        .variant(ButtonVariant::Outline)
                        .on_press_tx(|s: &mut App, _| s.reload())
                        .build(k),
                )
                .child(
                    Button::new("Reset to defaults")
                        .variant(ButtonVariant::GhostMuted)
                        .on_press_tx(|s: &mut App, _| s.reset())
                        .build(k),
                )],
        ),
        ui::section(
            k,
            hair,
            "contents",
            "Contents",
            vec![col().key("preview").w_full().px(space::S3).py(10.0).child(
                text(toml)
                    .key("toml")
                    .font(k.ty.small.mono())
                    .color(k.c.text_muted)
                    .selectable(true)
                    .wrap_anywhere(),
            )],
        ),
    ]
}

fn switch_row(
    k: &Kit,
    key: &str,
    label: &str,
    desc: Option<&str>,
    on: bool,
    flip: fn(&mut App),
) -> El {
    ui::setting_row(
        k,
        key,
        label,
        desc,
        Switch::new(on)
            .label(label)
            .hide_label()
            .on_toggle(move |s: &mut App| {
                flip(s);
                s.touch();
            })
            .build(k),
    )
}

fn checkbox_row(k: &Kit, key: &str, label: &str, on: bool, flip: fn(&mut App)) -> El {
    row()
        .key(key.to_owned())
        .w_full()
        .min_h(44.0)
        .px(space::S3)
        .py(10.0)
        .center_y()
        .child(
            Checkbox::new(on)
                .label(label)
                .on_toggle(move |s: &mut App| {
                    flip(s);
                    s.touch();
                })
                .build(k),
        )
}

fn select_row(
    s: &App,
    k: &Kit,
    layer: &mut Vec<El>,
    key: &'static str,
    label: &str,
    desc: Option<&str>,
    options: &'static [&'static str],
    current: &str,
    choose: fn(&mut App, &str),
) -> El {
    ui::setting_row(
        k,
        key,
        label,
        desc,
        Select::new(format!("{key}-select"), options)
            .width(200.0)
            .appearance(rune_kit::SelectAppearance::Field)
            .current(Some(current))
            .open(s.open == Some(key))
            .on_toggle(toggle(key))
            .on_close(close)
            .on_choose(move |s: &mut App, v: &str, _| {
                choose(s, v);
                s.open = None;
                s.touch();
            })
            .build(k, layer),
    )
}

fn toggle(key: &'static str) -> impl Fn(&mut App) + Clone {
    move |s: &mut App| s.open = if s.open == Some(key) { None } else { Some(key) }
}

fn close(s: &mut App) {
    s.open = None;
}

fn get_anim(s: &App) -> f32 {
    s.cfg.appearance.animation_speed
}
fn set_anim(s: &mut App, v: f32) {
    s.cfg.appearance.animation_speed = v;
}
fn get_font_size(s: &App) -> f32 {
    s.cfg.editor.font_size
}
fn set_font_size(s: &mut App, v: f32) {
    s.cfg.editor.font_size = v;
}
fn get_timeout(s: &App) -> f32 {
    s.cfg.advanced.timeout_secs
}
fn set_timeout(s: &mut App, v: f32) {
    s.cfg.advanced.timeout_secs = v;
}

fn fmt_speed(v: f32) -> String {
    format!("{:.2}\u{00d7}", ui::speed_of(v))
}
fn fmt_pt(v: f32) -> String {
    format!("{v:.0} pt")
}
fn fmt_secs(v: f32) -> String {
    format!("{v:.0} s")
}
