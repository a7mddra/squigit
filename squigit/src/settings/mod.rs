mod config;
mod pages;
pub(crate) mod ui;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rune::{AppSpec, Cx, El, Key, KeyEvent, Latest, TextEdit, Tx, Waker};

static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn capture_arguments(path: &Path) -> Result<Vec<String>, String> {
    let toml = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let general = config::Config::from_toml_string(&toml)?.general;
    let mut arguments = vec![general.capture_mode.argument().into()];
    if general.auto_ocr {
        arguments.push("--auto-ocr".into());
    }
    Ok(arguments)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    General,
    Appearance,
    Editor,
    Advanced,
    ConfigFile,
}

impl Page {
    pub fn slug(self) -> &'static str {
        match self {
            Page::General => "general",
            Page::Appearance => "appearance",
            Page::Editor => "editor",
            Page::Advanced => "advanced",
            Page::ConfigFile => "config-file",
        }
    }
}

pub struct Edits {
    pub nickname: TextEdit,
    pub endpoint: TextEdit,
    pub prompt: TextEdit,
}

pub struct App {
    pub cfg: config::Config,
    pub path: PathBuf,
    pub edits: Edits,
    pub page: Page,
    pub open: Option<&'static str>,
    pub saved_at: Option<String>,
    pub error: Option<String>,
    pub(crate) dirty: bool,
    saves: Latest<Result<(), String>>,
}

impl App {
    pub fn touch(&mut self) {
        self.dirty = true;
    }

    pub fn save(&mut self) {
        self.dirty = true;
        self.flush();
    }

    fn flush(&mut self) {
        match self.cfg.to_toml_string() {
            Ok(toml) => {
                let path = self.path.clone();
                self.saves.request((path, toml), |(path, toml)| {
                    std::fs::write(path, toml).map_err(|e| e.to_string())
                });
                self.dirty = false;
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn reload(&mut self) {
        match std::fs::read_to_string(&self.path)
            .map_err(|e| e.to_string())
            .and_then(|toml| config::Config::from_toml_string(&toml))
        {
            Ok(cfg) => {
                self.cfg = cfg;
                self.sync_edits();
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn reset(&mut self) {
        self.cfg = config::Config::default();
        self.sync_edits();
        self.touch();
    }

    fn sync_edits(&mut self) {
        self.edits
            .nickname
            .replace_text(self.cfg.general.nickname.clone());
        self.edits
            .endpoint
            .replace_text(self.cfg.advanced.endpoint.clone());
        self.edits
            .prompt
            .replace_text(self.cfg.editor.prompt_template.clone());
    }
}

fn now_utc_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02} UTC")
}

fn init() -> App {
    let path = CONFIG_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("config.toml"));
    let (cfg, error) = match std::fs::read_to_string(&path) {
        Ok(toml) => config::Config::from_toml_string(&toml)
            .map_or_else(|e| (config::Config::default(), Some(e)), |cfg| (cfg, None)),
        Err(_) => {
            let cfg = config::Config::default();
            if let Ok(toml) = cfg.to_toml_string() {
                let _ = std::fs::write(&path, toml);
            }
            (cfg, None)
        }
    };
    App {
        edits: Edits {
            nickname: TextEdit::new(cfg.general.nickname.clone()),
            endpoint: TextEdit::new(cfg.advanced.endpoint.clone()),
            prompt: TextEdit::new(cfg.editor.prompt_template.clone()),
        },
        cfg,
        path,
        page: Page::General,
        open: None,
        saved_at: None,
        error,
        dirty: false,
        saves: Latest::default().debounce(Duration::from_millis(40)),
    }
}

fn setup(s: &mut App, waker: Waker) {
    s.saves.set_waker(waker);
}

fn view(s: &App, cx: &mut Cx<App>) -> El {
    let dark = match s.cfg.appearance.theme.as_str() {
        "light" => false,
        "dark" => true,
        _ => cx.system_dark(),
    };
    cx.reduced_motion(s.cfg.appearance.reduce_motion);
    cx.time_scale(ui::speed_of(s.cfg.appearance.animation_speed));
    let k = ui::kit(dark, s.cfg.appearance.accent_index);
    let hair = 1.0 / cx.env().scale;
    let mut layer = Vec::new();
    let body = pages::page(s, &k, &mut layer, hair);
    rune_kit::root(ui::chrome(s, &k, body), layer)
}

fn poll(s: &mut App, _now: f64, _tx: &mut Tx) -> bool {
    let mut changed = false;
    if s.dirty {
        s.flush();
        changed = true;
    }
    if let Some(result) = s.saves.take() {
        match result {
            Ok(()) => {
                s.saved_at = Some(now_utc_hms());
                s.error = None;
            }
            Err(e) => s.error = Some(e),
        }
        changed = true;
    }
    changed
}

fn on_key(s: &mut App, e: &KeyEvent, _tx: &mut Tx) -> bool {
    if e.key == Key::Command('s') {
        s.save();
        return true;
    }
    false
}

pub fn app(path: PathBuf) -> rune::App<App> {
    let _ = CONFIG_PATH.set(path);
    rune::App::new(AppSpec::<App> {
        min_size: Some((640.0, 480.0)),
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("Squigit Configuration", (760.0, 560.0), init, view)
    })
    .on_close(|s, _| s.save())
}
