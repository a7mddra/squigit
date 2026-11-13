mod general;
mod hosting;
mod pages;
pub(crate) mod ui;

use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use rune::{AppSpec, Cx, El, Tx, Waker};
use squigit_ocr::models::{ModelDownloadJobSnapshot, ModelManager, DEFAULT_OCR_MODEL_ID};

const SPINNER_GRACE: Duration = Duration::from_secs(3);
const CATCH_UP: Duration = Duration::from_millis(650);
const DELETE_HOLD: Duration = Duration::from_secs(2);
const DELETE_ARM: Duration = Duration::from_secs(3);

type JobUpdates = Arc<Mutex<Option<Vec<ModelDownloadJobSnapshot>>>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    General,
    Models,
    ImageHosting,
}

enum Action {
    Engine,
    Models,
}

struct Finished {
    key: String,
    result: Result<Action, String>,
}

struct DeleteGesture {
    id: String,
    armed: Instant,
    holding: Option<Instant>,
}

pub struct App {
    page: Page,
    general: general::General,
    hosting: hosting::Hosting,
    models: Option<ModelManager>,
    runtime: Option<tokio::runtime::Runtime>,
    jobs: Vec<ModelDownloadJobSnapshot>,
    installed: HashSet<String>,
    selected: String,
    open: bool,
    error: Option<String>,
    engine_ready: bool,
    pending: HashSet<String>,
    started: HashMap<String, Instant>,
    visual: HashMap<String, f32>,
    delete: Option<DeleteGesture>,
    updates: JobUpdates,
    finished_tx: mpsc::Sender<Finished>,
    finished_rx: mpsc::Receiver<Finished>,
    waker: Option<Waker>,
}

impl App {
    fn show_page(&mut self, page: Page) {
        if self.page != page {
            let focused = self.hosting.focused;
            self.hosting.blur();
            self.hosting.clear();
            self.hosting.focused = focused;
            self.page = page;
        }
        self.open = false;
        self.delete = None;
        if page == Page::ImageHosting {
            self.hosting.refresh();
        } else if page == Page::General {
            self.general.refresh();
        }
    }

    pub fn open_image_hosting(&mut self) {
        self.show_page(Page::ImageHosting);
    }

    fn refresh(&mut self) {
        let Some(models) = &self.models else { return };
        match models
            .list_downloaded_models()
            .and_then(|installed| Ok((installed, models.selected_model()?)))
        {
            Ok((installed, selected)) => {
                self.installed = installed.into_iter().collect();
                self.installed.insert(DEFAULT_OCR_MODEL_ID.to_string());
                self.selected = selected;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn work(
        &mut self,
        key: String,
        work: impl FnOnce() -> Result<Action, String> + Send + 'static,
    ) {
        if !self.pending.insert(key.clone()) {
            return;
        }
        let (Some(runtime), Some(waker)) = (&self.runtime, self.waker.clone()) else {
            self.pending.remove(&key);
            return;
        };
        let sender = self.finished_tx.clone();
        runtime.spawn_blocking(move || {
            let result = work();
            let _ = sender.send(Finished { key, result });
            waker();
        });
    }

    fn download(&mut self, id: &str) {
        let (Some(runtime), Some(models)) = (&self.runtime, &self.models) else {
            return;
        };
        let _context = runtime.enter();
        match models.start_download(id) {
            Ok(()) => {
                self.started.insert(id.to_string(), Instant::now());
                self.visual.insert(id.to_string(), -1.0);
                self.jobs = models.download_jobs_snapshot();
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn cancel(&mut self, id: &str) {
        if let Some(models) = &self.models {
            models.cancel_download(id);
            self.jobs = models.download_jobs_snapshot();
        }
    }

    fn select(&mut self, id: &str) {
        self.open = false;
        let Some(models) = self.models.clone() else {
            return;
        };
        let id = id.to_string();
        self.work("selection".to_string(), move || {
            models
                .select_model(&id)
                .map_err(|error| error.to_string())?;
            Ok(Action::Models)
        });
    }

    fn remove(&mut self, id: &str) {
        self.delete = None;
        let Some(models) = self.models.clone() else {
            return;
        };
        let id = id.to_string();
        self.work(id.clone(), move || {
            models
                .trash_downloaded_model(&id)
                .map_err(|error| error.to_string())?;
            Ok(Action::Models)
        });
    }

    fn job(&self, id: &str) -> Option<&ModelDownloadJobSnapshot> {
        self.jobs.iter().find(|job| job.id == id)
    }
}

fn init() -> App {
    let (finished_tx, finished_rx) = mpsc::channel();
    let mut s = App {
        page: Page::General,
        general: general::General::new(),
        hosting: hosting::Hosting::new(),
        models: None,
        runtime: None,
        jobs: Vec::new(),
        installed: HashSet::new(),
        selected: DEFAULT_OCR_MODEL_ID.to_string(),
        open: false,
        error: None,
        engine_ready: false,
        pending: HashSet::new(),
        started: HashMap::new(),
        visual: HashMap::new(),
        delete: None,
        updates: Arc::new(Mutex::new(None)),
        finished_tx,
        finished_rx,
        waker: None,
    };
    match crate::storage::AppData::open().map(ModelManager::new) {
        Ok(models) => s.models = Some(models),
        Err(error) => s.error = Some(error.to_string()),
    }
    match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(runtime) => s.runtime = Some(runtime),
        Err(error) => s.error = Some(error.to_string()),
    }
    s.refresh();
    s
}

fn setup(s: &mut App, waker: Waker) {
    s.waker = Some(waker.clone());
    s.general.setup(waker.clone());
    s.hosting.setup(waker.clone());
    if let (Some(models), Some(runtime)) = (&s.models, &s.runtime) {
        let mut changes = models.subscribe_downloads();
        let slot = s.updates.clone();
        runtime.spawn(async move {
            while changes.changed().await.is_ok() {
                let snapshot = changes.borrow_and_update().clone();
                if let Ok(mut slot) = slot.lock() {
                    *slot = Some(snapshot);
                }
                waker();
            }
        });
    }
    s.work("engine".to_string(), || {
        squigit_ocr::sidecar::resolve_sidecar_path()
            .map(|_| Action::Engine)
            .map_err(|error| format!("{error:#}"))
    });
}

fn view(s: &App, cx: &mut Cx<App>) -> El {
    let k = ui::kit(true);
    let mut layer = Vec::new();
    let hair = 1.0 / cx.env().scale;
    let body = match s.page {
        Page::General => general::page(s, &k, &mut layer, hair),
        Page::Models => pages::page(s, &k, hair),
        Page::ImageHosting => hosting::page(s, &k, hair),
    };
    rune_kit::root(ui::chrome(s, &k, body), layer)
}

fn poll(s: &mut App, _now: f64, tx: &mut Tx) -> bool {
    let mut changed = false;
    let snapshot = s.updates.lock().ok().and_then(|mut slot| slot.take());
    if let Some(jobs) = snapshot {
        for job in &jobs {
            if job.status == "failed"
                && s.job(&job.id)
                    .is_none_or(|previous| previous.error != job.error)
            {
                s.error = job.error.clone();
            }
        }
        s.jobs = jobs;
        changed = true;
    }
    while let Ok(finished) = s.finished_rx.try_recv() {
        s.pending.remove(&finished.key);
        match finished.result {
            Ok(Action::Engine) => {
                s.engine_ready = true;
                s.general.refresh();
            }
            Ok(Action::Models) => s.error = None,
            Err(error) => s.error = Some(error),
        }
        changed = true;
    }
    if changed {
        s.refresh();
    }
    for job in &s.jobs {
        if !pages::active(&job.status) {
            continue;
        }
        let started = s.started.entry(job.id.clone()).or_insert_with(Instant::now);
        let elapsed = started.elapsed();
        if elapsed < SPINNER_GRACE {
            tx.wake_after(SPINNER_GRACE - elapsed);
        } else {
            let fraction =
                ((elapsed - SPINNER_GRACE).as_secs_f32() / CATCH_UP.as_secs_f32()).min(1.0);
            let visible = f32::from(job.progress) * fraction;
            let previous = s.visual.insert(job.id.clone(), visible);
            changed |= previous != Some(visible);
            if fraction < 1.0 {
                tx.wake_after(Duration::from_millis(16));
            }
        }
    }
    if let Some(gesture) = &s.delete {
        if let Some(started) = gesture.holding {
            if started.elapsed() >= DELETE_HOLD {
                let id = gesture.id.clone();
                s.remove(&id);
                changed = true;
            } else {
                tx.wake_after(Duration::from_millis(16));
                changed = true;
            }
        } else if gesture.armed.elapsed() >= DELETE_ARM {
            s.delete = None;
            changed = true;
        } else {
            tx.wake_after(DELETE_ARM - gesture.armed.elapsed());
        }
    }
    changed | s.general.poll() | s.hosting.poll(tx, s.page == Page::ImageHosting)
}

pub fn app() -> rune::App<App> {
    let app = rune::App::new(AppSpec::<App> {
        min_size: Some((640.0, 480.0)),
        setup: Some(setup),
        poll: Some(poll),
        ..AppSpec::new("Squigit Configuration", (760.0, 560.0), init, view)
    })
    .on_focus(|s, focused, tx| {
        s.hosting.focused = focused;
        if focused {
            s.refresh();
            if s.page == Page::General {
                s.general.refresh();
            }
            s.hosting.refresh();
            tx.wake_after(Duration::ZERO);
        } else {
            s.delete = None;
            s.hosting.blur();
        }
    })
    .on_close(|s, _| {
        s.open = false;
        s.delete = None;
        s.hosting.blur();
        s.hosting.clear();
        s.hosting.window = None;
    });
    let handle = app.handle();
    app.on_window(move |window| {
        let window = window.clone();
        handle.update(move |s, _| s.hosting.window = Some(window));
    })
}
