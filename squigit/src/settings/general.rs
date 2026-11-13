use std::sync::mpsc;

use rune::{col, icon, row, text, El, ScrollSpec, TextStyle, Waker};
use rune_kit::{Button, ControlSize, Kit, Menu, MenuItem, Switch};
use squigit_ocr::models::OCR_MODELS;

use super::{ui, App, Page};
use crate::{
    cache, startup,
    storage::{AppData, GeneralConfig},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Task {
    Refresh,
    Sounds,
    Startup,
    Clear,
}

enum Outcome {
    Refreshed(GeneralConfig, Result<bool, String>, Result<u64, String>),
    Sounds(bool),
    Startup(bool),
    Cleared(Option<u64>, Option<String>),
}

pub(super) struct General {
    config: GeneralConfig,
    startup: bool,
    bytes: Option<u64>,
    busy: Option<Task>,
    refresh_pending: bool,
    error: Option<String>,
    sender: mpsc::Sender<Result<Outcome, String>>,
    receiver: mpsc::Receiver<Result<Outcome, String>>,
    waker: Option<Waker>,
}

impl General {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            config: GeneralConfig::default(),
            startup: false,
            bytes: None,
            busy: None,
            refresh_pending: false,
            error: None,
            sender,
            receiver,
            waker: None,
        }
    }

    pub(super) fn setup(&mut self, waker: Waker) {
        self.waker = Some(waker);
        self.refresh();
    }

    fn work(
        &mut self,
        task: Task,
        work: impl FnOnce() -> anyhow::Result<Outcome> + Send + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        let Some(waker) = self.waker.clone() else {
            return;
        };
        self.busy = Some(task);
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(work().map_err(|error| format!("{error:#}")));
            waker();
        });
    }

    pub(super) fn refresh(&mut self) {
        if self.busy.is_some() {
            self.refresh_pending = true;
            return;
        }
        self.refresh_pending = false;
        self.work(Task::Refresh, || {
            let data = AppData::open()?;
            Ok(Outcome::Refreshed(
                data.load()?.general,
                startup::enabled().map_err(|error| format!("{error:#}")),
                cache::storage_size(&data).map_err(|error| format!("{error:#}")),
            ))
        });
    }

    fn toggle_sounds(&mut self) {
        let enabled = !self.config.play_sounds;
        self.work(Task::Sounds, move || {
            AppData::open()?.update(|config| config.general.play_sounds = enabled)?;
            Ok(Outcome::Sounds(enabled))
        });
    }

    fn toggle_startup(&mut self) {
        let enabled = !self.startup;
        self.work(Task::Startup, move || {
            startup::set_enabled(enabled)?;
            Ok(Outcome::Startup(startup::enabled()?))
        });
    }

    fn clear(&mut self) {
        self.work(Task::Clear, || {
            let data = AppData::open()?;
            let error = cache::clear(&data).err().map(|error| format!("{error:#}"));
            Ok(Outcome::Cleared(cache::storage_size(&data).ok(), error))
        });
    }

    pub(super) fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(outcome) = self.receiver.try_recv() {
            let task = self.busy.take();
            match outcome {
                Ok(Outcome::Refreshed(config, startup, bytes)) => {
                    self.config = config;
                    let mut errors = Vec::new();
                    match startup {
                        Ok(enabled) => self.startup = enabled,
                        Err(error) => errors.push(error),
                    }
                    match bytes {
                        Ok(bytes) => self.bytes = Some(bytes),
                        Err(error) => {
                            self.bytes = None;
                            errors.push(error);
                        }
                    }
                    self.error = (!errors.is_empty()).then(|| errors.join("\n"));
                }
                Ok(Outcome::Sounds(enabled)) => {
                    self.config.play_sounds = enabled;
                    self.error = None;
                }
                Ok(Outcome::Startup(enabled)) => {
                    self.startup = enabled;
                    self.error = None;
                }
                Ok(Outcome::Cleared(bytes, error)) => {
                    self.bytes = bytes;
                    self.error = error;
                }
                Err(error) => {
                    if task == Some(Task::Refresh) {
                        self.bytes = None;
                    }
                    self.error = Some(error);
                }
            }
            changed = true;
        }
        if self.refresh_pending && self.busy.is_none() {
            self.refresh();
            changed = true;
        }
        changed
    }
}

fn setting(k: &Kit, key: &str, label: &str, description: &str, control: El) -> El {
    row()
        .key(key)
        .w_full()
        .min_h(76.0)
        .py(14.0)
        .gap(16.0)
        .center_y()
        .child(
            col()
                .grow(1.0)
                .shrink()
                .gap(4.0)
                .child(text(label).font(k.ty.ui_medium).color(k.c.text_base))
                .child(text(description).font(k.ty.small).color(k.c.text_muted)),
        )
        .child(control)
}

fn storage_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub(super) fn page(s: &App, k: &Kit, layer: &mut Vec<El>, hair: f32) -> El {
    let g = &s.general;
    let busy = g.busy.is_some();
    let sounds = Switch::new(g.config.play_sounds)
        .label("Play sounds")
        .hide_label()
        .disabled(busy)
        .on_toggle(|s: &mut App| s.general.toggle_sounds())
        .build(k);
    let startup = Switch::new(g.startup)
        .label("Open on startup")
        .hide_label()
        .disabled(busy)
        .on_toggle(|s: &mut App| s.general.toggle_startup())
        .build(k);
    let clear_label = match (g.busy, g.bytes) {
        (Some(Task::Clear), _) => "Clearing…".to_string(),
        (Some(Task::Refresh), None) => "Calculating…".to_string(),
        (_, Some(bytes)) => format!("Clear {}", storage_size(bytes)),
        _ => "Calculate size".to_string(),
    };
    let clear = Button::new(clear_label)
        .control_size(ControlSize::Large)
        .disabled(busy || g.bytes == Some(0))
        .on_press(|s: &mut App| {
            if s.general.bytes.is_some() {
                s.general.clear();
            } else {
                s.general.refresh();
            }
        })
        .build(k);
    let rows = vec![
        setting(
            k,
            "sounds",
            "Play sounds",
            "Menu ticks and notification alerts",
            sounds,
        ),
        setting(
            k,
            "ocr-model",
            "Selected OCR model",
            "Used for new text scans",
            model_switcher(s, k, layer),
        ),
        setting(
            k,
            "startup",
            "Open on startup",
            "Show Squigit in the tray when you log in",
            startup,
        ),
        setting(
            k,
            "storage",
            "Clear storage",
            "Clear caches and temporary images",
            clear,
        ),
    ];
    let mut body = col()
        .key("general")
        .w_full()
        .max_w(660.0)
        .gap(24.0)
        .child(
            text("General")
                .font(TextStyle::new(28.0, 450.0))
                .color(k.c.text_base),
        )
        .child(ui::section(k, hair, "Preferences", rows))
        .child(
            text("Saved screenshots, downloaded models, settings, and API keys are kept.")
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    for error in [&g.error, &s.error].into_iter().flatten() {
        body = body.child(
            text(error.clone())
                .role("alert")
                .font(k.ty.small)
                .color(k.c.danger.fg),
        );
    }
    col()
        .key("page/general")
        .grow(1.0)
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .px(28.0)
        .py(24.0)
        .child(body)
}

fn model_switcher(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let current = OCR_MODELS
        .iter()
        .find(|model| model.id == s.selected)
        .unwrap_or(&OCR_MODELS[0]);
    let trigger = row()
        .key("model-trigger-label")
        .h(28.0)
        .px(8.0)
        .gap(6.0)
        .center_y()
        .role("button")
        .label("Select OCR model")
        .a11y_value(current.name)
        .focusable(true)
        .disabled(!s.engine_ready || s.pending.contains("selection"))
        .radius(6.0)
        .hover(k.c.hover)
        .on_press(|s: &mut App| s.open = !s.open)
        .child(text(current.name).font(k.ty.ui_medium).color(k.c.text_base))
        .child(
            icon(
                if s.open {
                    k.glyph.chevron_up
                } else {
                    k.glyph.chevron_down
                },
                14.0,
            )
            .color(k.c.icon_muted),
        );
    let mut menu = Menu::new("ocr-model")
        .trigger(trigger)
        .open(s.open)
        .min_width(220.0)
        .gutter(18.0)
        .on_close(|s: &mut App| s.open = false)
        .group_label("OCR Model");
    for model in OCR_MODELS
        .iter()
        .filter(|model| s.installed.contains(model.id))
    {
        let id = model.id;
        menu = menu.item(
            MenuItem::new(model.name)
                .checked(id == s.selected)
                .disabled(s.pending.contains("selection"))
                .on_select(move |s: &mut App| s.select(id)),
        );
    }
    menu.separator()
        .item(
            MenuItem::new("Get more models")
                .icon(&ui::PACKAGE_PLUS)
                .on_select_tx(|s: &mut App, tx| {
                    s.show_page(Page::Models);
                    tx.focus("model-downloads", true);
                    tx.scroll_into_view("model-downloads");
                }),
        )
        .build(k, layer)
}
