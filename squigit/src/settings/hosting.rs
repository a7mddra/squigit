use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use rune::{col, el, icon, row, text, El, TextEdit, TextStyle, Tx, Waker, WindowHandle};
use rune_kit::{Kit, TextInput};

use super::{ui, App};
use crate::auth::{self, SecretString};
use crate::storage::AppData;

enum Outcome {
    Status(bool),
    Saved,
    Deleted,
    Revealed(u64, Option<SecretString>),
}

pub(super) struct Hosting {
    edit: TextEdit,
    configured: bool,
    show: bool,
    revealed: bool,
    dirty: bool,
    was_focused: bool,
    busy: bool,
    error: Option<String>,
    generation: u64,
    pub(super) focused: bool,
    pub(super) window: Option<WindowHandle>,
    pending_reveal: Option<(SecretString, Instant)>,
    sender: mpsc::Sender<Result<Outcome, String>>,
    receiver: mpsc::Receiver<Result<Outcome, String>>,
    waker: Option<Waker>,
}

impl Hosting {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            edit: TextEdit::new("").secure(),
            configured: false,
            show: false,
            revealed: false,
            dirty: false,
            was_focused: false,
            busy: false,
            error: None,
            generation: 0,
            focused: false,
            window: None,
            pending_reveal: None,
            sender,
            receiver,
            waker: None,
        }
    }

    pub(super) fn setup(&mut self, waker: Waker) {
        self.waker = Some(waker);
        self.refresh();
    }

    fn work(&mut self, work: impl FnOnce() -> anyhow::Result<Outcome> + Send + 'static) {
        if self.busy {
            return;
        }
        let Some(waker) = self.waker.clone() else {
            return;
        };
        self.busy = true;
        self.error = None;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(work().map_err(|error| format!("{error:#}")));
            waker();
        });
    }

    pub(super) fn refresh(&mut self) {
        if !self.dirty && !self.revealed && self.pending_reveal.is_none() {
            let error = self.error.take();
            self.work(|| Ok(Outcome::Status(auth::configured(&AppData::open()?)?)));
            self.error = error;
        }
    }

    pub(super) fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.edit.replace_text("");
        self.edit.set_revealed(false);
        self.edit.focused = false;
        self.show = false;
        self.revealed = false;
        self.dirty = false;
        self.was_focused = false;
        self.pending_reveal = None;
    }

    pub(super) fn blur(&mut self) {
        self.focused = false;
        self.save();
        self.edit.focused = false;
        if self.show
            || self.revealed
            || self.dirty
            || !self.edit.text.is_empty()
            || self.pending_reveal.is_some()
        {
            self.clear();
        }
    }

    fn edit_changed(&mut self) {
        if self.configured && !self.show && !self.revealed && !self.dirty {
            self.edit.replace_text("");
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.revealed = false;
        self.dirty = true;
        self.error = None;
        self.pending_reveal = None;
    }

    fn save(&mut self) {
        if self.busy || !self.dirty || self.revealed {
            return;
        }
        if !auth::valid_key(&self.edit.text) {
            if !self.edit.focused {
                self.clear();
            }
            return;
        }
        let secret = SecretString::new(self.edit.text.trim().to_owned());
        self.dirty = false;
        self.work(move || {
            auth::save(&AppData::open()?, &secret)?;
            Ok(Outcome::Saved)
        });
    }

    fn reveal(&mut self) {
        if self.busy {
            return;
        }
        if self.show {
            if self.revealed {
                self.clear();
            } else {
                self.show = false;
                self.edit.set_revealed(false);
            }
            return;
        }
        if !self.edit.text.is_empty() {
            self.show = true;
            self.edit.set_revealed(true);
            return;
        }
        if self.configured {
            self.generation = self.generation.wrapping_add(1);
            let generation = self.generation;
            let window = self.window.clone();
            self.work(move || {
                Ok(Outcome::Revealed(
                    generation,
                    auth::reveal(&AppData::open()?, window)?,
                ))
            });
        }
    }

    fn delete(&mut self) {
        if self.busy {
            return;
        }
        self.clear();
        self.work(|| {
            auth::delete(&AppData::open()?)?;
            Ok(Outcome::Deleted)
        });
    }

    pub(super) fn poll(&mut self, tx: &mut Tx, active: bool) -> bool {
        let mut changed = false;
        while let Ok(outcome) = self.receiver.try_recv() {
            self.busy = false;
            match outcome {
                Ok(Outcome::Status(configured)) => self.configured = configured,
                Ok(Outcome::Saved) => {
                    self.configured = true;
                    self.clear();
                }
                Ok(Outcome::Deleted) => {
                    self.configured = false;
                    self.clear();
                }
                Ok(Outcome::Revealed(generation, secret))
                    if generation == self.generation && active =>
                {
                    self.pending_reveal = secret.map(|secret| (secret, Instant::now()));
                }
                Ok(Outcome::Revealed(_, _)) => {}
                Err(error) => {
                    self.error = Some(error);
                    if !self.focused {
                        self.clear();
                    } else if !self.revealed && !self.edit.text.is_empty() {
                        self.dirty = true;
                    }
                }
            }
            changed = true;
        }
        if let Some((_, since)) = &self.pending_reveal {
            if since.elapsed() >= Duration::from_secs(5) || !active {
                self.pending_reveal = None;
            } else if self.focused {
                if let Some((secret, _)) = self.pending_reveal.take() {
                    self.edit.replace_text(secret.expose());
                    self.edit.set_revealed(true);
                    self.edit.focused = true;
                    self.show = true;
                    self.revealed = true;
                    tx.focus("input", false);
                }
            } else {
                tx.wake_after(Duration::from_millis(50));
            }
            changed = true;
        }
        if active && self.was_focused && !self.edit.focused {
            if self.revealed {
                self.clear();
            } else {
                self.save();
            }
            changed = true;
        }
        self.was_focused = self.edit.focused;
        if active && self.edit.focused {
            tx.wake_after(Duration::from_millis(50));
        }
        changed
    }
}

fn action(
    k: &Kit,
    key: &str,
    glyph: &'static rune::Icon,
    label: &str,
    disabled: bool,
    press: impl Fn(&mut App, &mut Tx) + 'static,
) -> El {
    el().key(key)
        .w(26.0)
        .h(26.0)
        .center()
        .radius(4.0)
        .hover(k.c.hover)
        .role("button")
        .label(label)
        .help(label)
        .focusable(true)
        .disabled(disabled)
        .child(icon(glyph, 15.0).color(k.c.icon_muted))
        .on_press_tx(press)
}

pub(super) fn page(s: &App, k: &Kit, hair: f32) -> El {
    let h = &s.hosting;
    let mut actions = row().gap(2.0).center_y().child(action(
        k,
        "reveal",
        if h.show { &ui::EYE } else { &ui::EYE_OFF },
        if h.busy {
            "Please wait"
        } else if h.show {
            "Hide key"
        } else {
            "Show key"
        },
        h.busy,
        |s, tx| {
            s.hosting.reveal();
            s.hosting.edit.focused = true;
            tx.focus("input", false);
        },
    ));
    if h.configured {
        actions = actions.child(action(
            k,
            "delete",
            &ui::TRASH,
            "Delete key",
            h.busy,
            |s, _| s.hosting.delete(),
        ));
    }
    let field = TextInput::new(&h.edit)
        .secure()
        .revealed(h.show)
        .label("ImgBB API key")
        .placeholder(if h.configured {
            "•".repeat(32)
        } else {
            "Enter your API key".into()
        })
        .width(Some(255.0))
        .large()
        .trailing(actions)
        .disabled(h.busy)
        .invalid(h.dirty && !h.edit.text.is_empty() && !auth::valid_key(&h.edit.text))
        .on_edit(|s: &mut App| &mut s.hosting.edit)
        .on_key(|s: &mut App, event, _| {
            s.hosting.configured
                && !s.hosting.show
                && !s.hosting.revealed
                && !s.hosting.dirty
                && matches!(
                    event.key,
                    rune::Key::Text(_)
                        | rune::Key::Backspace
                        | rune::Key::Delete
                        | rune::Key::Command('v' | 'x' | 'z')
                )
        })
        .on_change(|s: &mut App, tx| {
            s.hosting.edit_changed();
            tx.wake_after(Duration::from_millis(50));
        })
        .on_submit(|s: &mut App, _| s.hosting.save())
        .build(k)
        .key("imgbb-key");
    let provider = row().w_full().min_h(76.0).gap(16.0).center_y()
        .child(col().grow(1.0).shrink().gap(6.0)
            .child(row().gap(6.0).center_y().child(text("ImgBB").font(k.ty.ui_medium).color(k.c.text_base))
                .child(icon(&ui::INFO, 14.0).color(k.c.icon_muted).help("ImgBB creates public image URLs for Google Lens and AI Mode. Avoid uploading sensitive images.")))
            .child(text("Use your ImgBB key to get a free public image link.").font(k.ty.small).color(k.c.text_muted)))
        .child(field);
    let mut body = col()
        .key("image-hosting")
        .w_full()
        .max_w(660.0)
        .gap(32.0)
        .child(
            text("Image hosting")
                .font(TextStyle::new(28.0, 450.0))
                .color(k.c.text_base),
        )
        .child(
            col()
                .gap(8.0)
                .child(text("API key").font(k.ty.ui_medium).color(k.c.text_base))
                .child(
                    text("Bring your own key for image hosting.")
                        .font(k.ty.small)
                        .color(k.c.text_muted),
                )
                .child(
                    el().px(16.0)
                        .fill(k.c.layer_01)
                        .radius(16.0)
                        .border(hair, k.c.border_muted)
                        .child(provider),
                ),
        )
        .child(
            row()
                .role("link")
                .label("Get a free ImgBB API key")
                .focusable(true)
                .help("https://api.imgbb.com/")
                .gap(6.0)
                .center_y()
                .child(
                    text("Get a free ImgBB API key")
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(icon(&ui::EXTERNAL, 14.0).color(k.c.icon_muted))
                .on_press(|s: &mut App| {
                    if let Err(error) = opener::open_browser("https://api.imgbb.com/") {
                        s.hosting.error = Some(error.to_string());
                    }
                }),
        );
    if let Some(error) = &h.error {
        body = body.child(
            text(error.clone())
                .role("alert")
                .font(k.ty.small)
                .color(k.c.danger.fg),
        );
    }
    col()
        .key("page/image-hosting")
        .grow(1.0)
        .h_full()
        .min_h(0.0)
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        })
        .px(28.0)
        .py(24.0)
        .child(body)
}
