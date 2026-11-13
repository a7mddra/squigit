use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use rune::{col, el, icon, row, text, El, TextEdit, TextStyle, Tx, Waker, WindowHandle};
use rune_kit::{Kit, RadioItem, TextInput};

use super::{ui, App};
use crate::{
    auth::{self, SecretString},
    image_hosting::{self, Provider},
    storage::{AppData, HostingConfig},
};

enum Outcome {
    Status([bool; 3], HostingConfig),
    Saved(Provider, HostingConfig),
    Deleted(Provider),
    Revealed(Provider, u64, Option<SecretString>),
    Settings(HostingConfig),
}

struct KeyInput {
    edit: TextEdit,
    configured: bool,
    show: bool,
    revealed: bool,
    dirty: bool,
    save_pending: bool,
    was_focused: bool,
    generation: u64,
    pending_reveal: Option<(SecretString, Instant)>,
}

impl KeyInput {
    fn new() -> Self {
        Self {
            edit: TextEdit::new("").secure(),
            configured: false,
            show: false,
            revealed: false,
            dirty: false,
            save_pending: false,
            was_focused: false,
            generation: 0,
            pending_reveal: None,
        }
    }

    fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.edit.replace_text("");
        self.edit.set_revealed(false);
        self.edit.focused = false;
        self.show = false;
        self.revealed = false;
        self.dirty = false;
        self.save_pending = false;
        self.was_focused = false;
        self.pending_reveal = None;
    }

    fn edit_changed(&mut self) {
        if self.configured && !self.show && !self.revealed && !self.dirty {
            self.edit.replace_text("");
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.revealed = false;
        self.dirty = true;
        self.pending_reveal = None;
    }
}

pub(super) struct Hosting {
    keys: [KeyInput; 3],
    config: HostingConfig,
    endpoint: TextEdit,
    endpoint_dirty: bool,
    endpoint_was_focused: bool,
    endpoint_pending: bool,
    pending_provider: Option<Provider>,
    busy: bool,
    error: Option<String>,
    pub(super) focused: bool,
    pub(super) window: Option<WindowHandle>,
    sender: mpsc::Sender<Result<Outcome, String>>,
    receiver: mpsc::Receiver<Result<Outcome, String>>,
    waker: Option<Waker>,
}

impl Hosting {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            keys: std::array::from_fn(|_| KeyInput::new()),
            config: HostingConfig::default(),
            endpoint: TextEdit::new(""),
            endpoint_dirty: false,
            endpoint_was_focused: false,
            endpoint_pending: false,
            pending_provider: None,
            busy: false,
            error: None,
            focused: false,
            window: None,
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
        if !self.endpoint_dirty
            && self
                .keys
                .iter()
                .all(|key| !key.dirty && !key.revealed && key.pending_reveal.is_none())
        {
            let error = self.error.take();
            self.work(|| {
                let data = AppData::open()?;
                Ok(Outcome::Status(auth::status(&data)?, data.load()?.hosting))
            });
            self.error = error;
        }
    }

    pub(super) fn clear(&mut self) {
        for key in &mut self.keys {
            key.clear();
        }
    }

    pub(super) fn blur(&mut self) {
        self.focused = false;
        for provider in Provider::ALL {
            self.save(provider);
        }
        self.save_endpoint();
        for key in &mut self.keys {
            key.edit.focused = false;
            if key.show
                || key.revealed
                || key.dirty
                || !key.edit.text.is_empty()
                || key.pending_reveal.is_some()
            {
                key.clear();
            }
        }
        self.endpoint.focused = false;
    }

    fn apply_config(&mut self, config: HostingConfig) {
        if !self.endpoint_dirty {
            self.endpoint.replace_text(config.chevereto_url.clone());
        }
        if image_hosting::custom_endpoint(&config.chevereto_url).is_ok() {
            let key = &mut self.keys[Provider::Chevereto.index()];
            key.save_pending = key.dirty && !key.edit.focused;
        }
        self.config = config;
    }

    fn select(&mut self, provider: Provider) {
        if self.busy || self.config.provider == provider {
            return;
        }
        for current in Provider::ALL {
            self.save(current);
            if self.busy {
                self.pending_provider = Some(provider);
                return;
            }
        }
        self.save_endpoint();
        if self.busy {
            self.pending_provider = Some(provider);
            return;
        }
        self.clear();
        self.work(move || {
            let data = AppData::open()?;
            data.update(|config| config.hosting.provider = provider)?;
            Ok(Outcome::Settings(data.load()?.hosting))
        });
    }

    fn save(&mut self, provider: Provider) {
        let key = &mut self.keys[provider.index()];
        if !key.dirty || key.revealed {
            key.save_pending = false;
            return;
        }
        if self.busy {
            key.save_pending = true;
            return;
        }
        key.save_pending = false;
        if !provider.valid_key(&key.edit.text) {
            if !key.edit.focused {
                key.clear();
            }
            return;
        }
        if provider == Provider::Chevereto
            && image_hosting::custom_endpoint(&self.endpoint.text).is_err()
        {
            self.error =
                Some("Enter your Chevereto HTTPS instance URL before saving its key.".into());
            return;
        }
        let secret = SecretString::new(key.edit.text.trim().to_owned());
        let endpoint = self.endpoint.text.trim().to_owned();
        key.dirty = false;
        self.work(move || {
            let data = AppData::open()?;
            auth::save(&data, provider, &secret, &endpoint)?;
            data.update(|config| {
                config.hosting.provider = provider;
                if provider == Provider::Chevereto {
                    config.hosting.chevereto_url = endpoint;
                }
            })?;
            Ok(Outcome::Saved(provider, data.load()?.hosting))
        });
    }

    fn save_endpoint(&mut self) {
        if !self.endpoint_dirty {
            self.endpoint_pending = false;
            return;
        }
        if self.busy {
            self.endpoint_pending = true;
            return;
        }
        self.endpoint_pending = false;
        let endpoint = self.endpoint.text.trim().to_owned();
        if !endpoint.is_empty() && image_hosting::custom_endpoint(&endpoint).is_err() {
            self.error =
                Some("Enter an HTTPS instance URL without credentials or query parameters.".into());
            return;
        }
        self.endpoint_dirty = false;
        self.work(move || {
            let data = AppData::open()?;
            let previous = data.load()?.hosting.chevereto_url;
            if image_hosting::custom_endpoint(&previous).ok()
                != image_hosting::custom_endpoint(&endpoint).ok()
            {
                auth::delete(&data, Provider::Chevereto)?;
            }
            data.update(|config| config.hosting.chevereto_url = endpoint)?;
            Ok(Outcome::Status(auth::status(&data)?, data.load()?.hosting))
        });
    }

    fn reveal(&mut self, provider: Provider) {
        if self.busy {
            return;
        }
        let key = &mut self.keys[provider.index()];
        if key.show {
            if key.revealed {
                key.clear();
            } else {
                key.show = false;
                key.edit.set_revealed(false);
            }
        } else if !key.edit.text.is_empty() {
            key.show = true;
            key.edit.set_revealed(true);
        } else if key.configured {
            key.generation = key.generation.wrapping_add(1);
            let generation = key.generation;
            let window = self.window.clone();
            self.work(move || {
                Ok(Outcome::Revealed(
                    provider,
                    generation,
                    auth::reveal(&AppData::open()?, provider, window)?,
                ))
            });
        }
    }

    fn delete(&mut self, provider: Provider) {
        if self.busy {
            return;
        }
        self.keys[provider.index()].clear();
        self.work(move || {
            auth::delete(&AppData::open()?, provider)?;
            Ok(Outcome::Deleted(provider))
        });
    }

    pub(super) fn poll(&mut self, tx: &mut Tx, active: bool) -> bool {
        let mut changed = false;
        while let Ok(outcome) = self.receiver.try_recv() {
            self.busy = false;
            match outcome {
                Ok(Outcome::Status(status, config)) => {
                    for provider in Provider::ALL {
                        self.keys[provider.index()].configured = status[provider.index()];
                    }
                    self.apply_config(config);
                }
                Ok(Outcome::Saved(provider, config)) => {
                    let key = &mut self.keys[provider.index()];
                    key.configured = true;
                    key.clear();
                    self.apply_config(config);
                }
                Ok(Outcome::Deleted(provider)) => {
                    let key = &mut self.keys[provider.index()];
                    key.configured = false;
                    key.clear();
                }
                Ok(Outcome::Revealed(provider, generation, secret)) => {
                    let key = &mut self.keys[provider.index()];
                    if key.generation == generation && active {
                        key.pending_reveal = secret.map(|secret| (secret, Instant::now()));
                    }
                }
                Ok(Outcome::Settings(config)) => self.apply_config(config),
                Err(error) => {
                    self.error = Some(error);
                    self.pending_provider = None;
                    for key in &mut self.keys {
                        if !self.focused {
                            key.clear();
                        } else if !key.revealed && !key.edit.text.is_empty() {
                            key.dirty = true;
                        }
                    }
                }
            }
            changed = true;
        }
        for provider in Provider::ALL {
            let key = &mut self.keys[provider.index()];
            if let Some((_, since)) = &key.pending_reveal {
                if since.elapsed() >= Duration::from_secs(5) || !active {
                    key.pending_reveal = None;
                } else if self.focused {
                    if let Some((secret, _)) = key.pending_reveal.take() {
                        key.edit.replace_text(secret.expose());
                        key.edit.set_revealed(true);
                        key.edit.focused = true;
                        key.show = true;
                        key.revealed = true;
                        tx.focus(provider.field_key(), false);
                    }
                } else {
                    tx.wake_after(Duration::from_millis(50));
                }
                changed = true;
            }
            if active && key.was_focused && !key.edit.focused {
                if key.revealed {
                    key.clear();
                } else {
                    key.show = false;
                    key.edit.set_revealed(false);
                    key.save_pending = key.dirty;
                }
                changed = true;
            }
            key.was_focused = key.edit.focused;
            if active && key.edit.focused {
                tx.wake_after(Duration::from_millis(50));
            }
            if key.save_pending && !self.busy {
                self.save(provider);
                changed = true;
            }
        }
        if active && self.endpoint.focused {
            tx.wake_after(Duration::from_millis(50));
        }
        if active && self.endpoint_was_focused && !self.endpoint.focused {
            self.endpoint_pending = self.endpoint_dirty;
        }
        self.endpoint_was_focused = self.endpoint.focused;
        if self.endpoint_pending && !self.busy {
            self.save_endpoint();
            changed = true;
        }
        if !self.busy {
            if let Some(provider) = self.pending_provider.take() {
                self.select(provider);
                changed = true;
            }
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

fn key_field(s: &App, k: &Kit, provider: Provider) -> El {
    let h = &s.hosting;
    let index = provider.index();
    let key = &h.keys[index];
    let mut actions = row().gap(2.0).center_y().child(action(
        k,
        "reveal",
        if key.show { &ui::EYE } else { &ui::EYE_OFF },
        if h.busy {
            "Please wait"
        } else if key.show {
            "Hide key"
        } else {
            "Show key"
        },
        h.busy,
        move |s, tx| {
            s.hosting.reveal(provider);
            s.hosting.keys[index].edit.focused = true;
            tx.focus(provider.field_key(), false);
        },
    ));
    if key.configured {
        actions = actions.child(action(
            k,
            "delete",
            &ui::TRASH,
            "Delete key",
            h.busy,
            move |s, _| s.hosting.delete(provider),
        ));
    }
    TextInput::new(&key.edit)
        .secure()
        .revealed(key.show)
        .label(format!("{} API key", provider.name()))
        .placeholder(if key.configured {
            "•".repeat(if provider == Provider::Imgbb { 32 } else { 24 })
        } else if provider == Provider::Imagekit {
            "Private API key".into()
        } else {
            "API key".into()
        })
        .width(Some(255.0))
        .large()
        .trailing(actions)
        .disabled(h.busy)
        .invalid(key.dirty && !key.edit.text.is_empty() && !provider.valid_key(&key.edit.text))
        .on_edit(move |s: &mut App| &mut s.hosting.keys[index].edit)
        .on_key(move |s: &mut App, event, _| {
            let key = &s.hosting.keys[index];
            key.configured
                && !key.show
                && !key.revealed
                && !key.dirty
                && matches!(
                    event.key,
                    rune::Key::Text(_)
                        | rune::Key::Backspace
                        | rune::Key::Delete
                        | rune::Key::Command('v' | 'x' | 'z')
                )
        })
        .on_change(move |s: &mut App, tx| {
            s.hosting.keys[index].edit_changed();
            s.hosting.error = None;
            tx.wake_after(Duration::from_millis(50));
        })
        .on_submit(move |s: &mut App, _| s.hosting.save(provider))
        .build(k)
        .key(provider.field_key())
        .shrink()
        .min_w(168.0)
}

fn provider_row(s: &App, k: &Kit, provider: Provider) -> El {
    let h = &s.hosting;
    let info = el()
        .size(20.0, 20.0)
        .center()
        .focusable(true)
        .label(format!("About {}", provider.name()))
        .help(provider.help())
        .child(icon(&ui::INFO, 14.0).color(k.c.icon_muted));
    let mut detail = col().grow(1.0).shrink().gap(6.0).child(
        row()
            .gap(2.0)
            .center_y()
            .child(
                RadioItem::new(provider.name(), h.config.provider == provider)
                    .disabled(h.busy)
                    .on_select(move |s: &mut App| s.hosting.select(provider))
                    .build(k),
            )
            .child(info),
    );
    let link_label = match provider {
        Provider::Imgbb => "Get a free API key",
        Provider::Imagekit => "Get your private key",
        Provider::Chevereto => "API setup guide",
    };
    detail = detail.child(
        row()
            .gap(4.0)
            .center_y()
            .pl(24.0)
            .role("link")
            .label(link_label)
            .focusable(true)
            .help(provider.dashboard())
            .child(text(link_label).font(k.ty.small).color(k.c.text_muted))
            .child(icon(&ui::EXTERNAL, 12.0).color(k.c.icon_muted))
            .on_press(move |s: &mut App| {
                if let Err(error) = opener::open_browser(provider.dashboard()) {
                    s.hosting.error = Some(error.to_string());
                }
            }),
    );
    let mut fields = col()
        .shrink()
        .min_w(168.0)
        .gap(8.0)
        .child(key_field(s, k, provider));
    if provider == Provider::Chevereto {
        fields = fields.child(
            TextInput::new(&h.endpoint)
                .label("Chevereto instance URL")
                .placeholder("https://images.example.com")
                .width(Some(255.0))
                .large()
                .disabled(h.busy)
                .invalid(
                    h.endpoint_dirty
                        && !h.endpoint.text.is_empty()
                        && image_hosting::custom_endpoint(&h.endpoint.text).is_err(),
                )
                .on_edit(|s: &mut App| &mut s.hosting.endpoint)
                .on_change(|s: &mut App, tx| {
                    s.hosting.endpoint_dirty = true;
                    s.hosting.error = None;
                    tx.wake_after(Duration::from_millis(50));
                })
                .on_submit(|s: &mut App, _| s.hosting.save_endpoint())
                .build(k)
                .key("chevereto-url")
                .help("Changing the instance URL clears its saved API key.")
                .shrink()
                .min_w(168.0),
        );
    }
    row()
        .key(provider.id())
        .w_full()
        .min_h(76.0)
        .py(14.0)
        .gap(12.0)
        .center_y()
        .child(detail)
        .child(fields)
}

pub(super) fn page(s: &App, k: &Kit, hair: f32) -> El {
    let mut card = col()
        .w_full()
        .px(16.0)
        .fill(k.c.layer_01)
        .radius(16.0)
        .border(hair, k.c.border_muted)
        .role("radiogroup")
        .label("Image hosting provider");
    for provider in Provider::ALL {
        if provider != Provider::Imgbb {
            card = card.child(el().h(hair).fill(k.c.border_muted).a11y_hidden());
        }
        card = card.child(provider_row(s, k, provider));
    }
    let mut body = col()
        .key("image-hosting")
        .w_full()
        .max_w(660.0)
        .gap(24.0)
        .child(
            text("Image hosting")
                .font(TextStyle::new(28.0, 450.0))
                .color(k.c.text_base),
        )
        .child(
            col()
                .gap(8.0)
                .child(
                    text("Choose a provider and add your API key.")
                        .font(k.ty.small)
                        .color(k.c.text_muted),
                )
                .child(card),
        );
    if let Some(error) = &s.hosting.error {
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
