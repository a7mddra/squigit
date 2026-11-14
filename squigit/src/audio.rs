use anyhow::{Context, Result};
use rodio::cpal::{
    self,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample,
};
use rodio::{
    buffer::SamplesBuffer,
    dynamic_mixer::{self, DynamicMixer, DynamicMixerController},
    Decoder, Source,
};
use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use crate::storage::AppData;

const TICK: &[u8] = include_bytes!("../assets/tick.wav");
const ALERT: &[u8] = include_bytes!("../assets/alert.mp3");
const VOLUME: f32 = 0.5;
const POLL: Duration = Duration::from_millis(50);
const PREFERENCE_POLL: Duration = Duration::from_millis(250);
const DEVICE_POLL: Duration = Duration::from_secs(1);
const RETRY: Duration = Duration::from_secs(1);
const STALL_TIMEOUT: Duration = Duration::from_secs(2);
const LOG_INTERVAL: Duration = Duration::from_secs(5);

static PREFERENCE: AtomicU64 = AtomicU64::new(0);
static PREFERENCE_UPDATE: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Tick,
    Alert,
}

impl Sound {
    fn lifetime(self) -> Duration {
        match self {
            Self::Tick => Duration::from_millis(150),
            Self::Alert => Duration::from_secs(2),
        }
    }
}

struct Request {
    sound: Sound,
    queued: Instant,
    preference: u64,
}

impl Request {
    fn current(&self) -> bool {
        self.preference == PREFERENCE.load(Ordering::Acquire)
            && self.queued.elapsed() < self.sound.lifetime()
    }
}

#[derive(Default)]
struct Diagnostics {
    last: Option<Instant>,
}

impl Diagnostics {
    fn report(&mut self, message: impl std::fmt::Display) {
        if self.last.is_none_or(|last| last.elapsed() >= LOG_INTERVAL) {
            eprintln!("[squigit] Audio: {message}");
            self.last = Some(Instant::now());
        }
    }
}

#[derive(Clone)]
pub struct Player {
    sender: SyncSender<Request>,
    diagnostics: Arc<Mutex<Diagnostics>>,
}

struct WorkerStop(Arc<AtomicBool>);

impl Drop for WorkerStop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl Player {
    pub fn start(data: AppData) -> Self {
        if let Err(error) = read_preference(&data) {
            eprintln!("[squigit] Could not load the sound preference: {error:#}");
        }
        let (sender, receiver) = mpsc::sync_channel(32);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = WorkerStop(stop.clone());
        match thread::Builder::new()
            .name("squigit-audio".into())
            .spawn(move || {
                let _stop = worker_stop;
                run(receiver);
            }) {
            Ok(_) => {
                if let Err(error) = thread::Builder::new()
                    .name("squigit-sound-preference".into())
                    .spawn(move || watch_preference(data, stop))
                {
                    eprintln!("[squigit] Could not start the sound preference monitor: {error}");
                }
            }
            Err(error) => eprintln!("[squigit] Could not start the audio worker: {error}"),
        }
        Self {
            sender,
            diagnostics: Arc::new(Mutex::new(Diagnostics::default())),
        }
    }

    pub fn play(&self, sound: Sound) {
        let preference = PREFERENCE.load(Ordering::Acquire);
        if preference & 1 == 0 {
            return;
        }
        if let Err(error) = self.sender.try_send(Request {
            sound,
            queued: Instant::now(),
            preference,
        }) {
            let message = match error {
                mpsc::TrySendError::Full(_) => "The sound queue is full; feedback was dropped",
                mpsc::TrySendError::Disconnected(_) => "The audio worker is unavailable",
            };
            self.diagnostics
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .report(message);
        }
    }
}

pub fn set_enabled(enabled: bool) {
    let _update = PREFERENCE_UPDATE
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    update_preference(enabled);
}

fn update_preference(enabled: bool) {
    if (PREFERENCE.load(Ordering::Acquire) & 1 != 0) != enabled {
        PREFERENCE.fetch_add(1, Ordering::AcqRel);
    }
}

fn read_preference(data: &AppData) -> Result<()> {
    let _update = PREFERENCE_UPDATE
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    update_preference(data.load()?.general.play_sounds);
    Ok(())
}

fn watch_preference(data: AppData, stop: Arc<AtomicBool>) {
    let mut revision = None;
    let mut diagnostics = Diagnostics::default();
    while !stop.load(Ordering::Acquire) {
        let result = (|| {
            let metadata = std::fs::metadata(data.config_path())?;
            let current = (metadata.modified()?, metadata.len());
            if revision != Some(current) {
                read_preference(&data)?;
                revision = Some(current);
            }
            Ok::<_, anyhow::Error>(())
        })();
        if let Err(error) = result {
            diagnostics.report(format!("Could not refresh the sound preference: {error:#}"));
        }
        thread::park_timeout(PREFERENCE_POLL);
    }
}

fn decode(bytes: &'static [u8]) -> Result<SamplesBuffer<f32>> {
    let decoder = Decoder::new(Cursor::new(bytes))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples = decoder.convert_samples::<f32>().collect::<Vec<_>>();
    anyhow::ensure!(
        channels > 0 && sample_rate > 0 && !samples.is_empty(),
        "The embedded sound is empty or invalid"
    );
    Ok(SamplesBuffer::new(channels, sample_rate, samples))
}

fn clip(bytes: &'static [u8], name: &str) -> Option<SamplesBuffer<f32>> {
    match decode(bytes) {
        Ok(clip) => Some(clip),
        Err(error) => {
            eprintln!("[squigit] Could not decode the {name} sound: {error:#}");
            None
        }
    }
}

fn run(receiver: Receiver<Request>) {
    let tick = clip(TICK, "tick");
    let alert = clip(ALERT, "alert");
    if tick.is_none() && alert.is_none() {
        return;
    }
    let mut output: Option<Output> = None;
    let mut pending: Option<Request> = None;
    let mut preference = PREFERENCE.load(Ordering::Acquire);
    let mut probe_at = Instant::now();
    let mut retry_at = Instant::now();
    let mut diagnostics = Diagnostics::default();
    loop {
        let request = match receiver.recv_timeout(POLL) {
            Ok(request) => Some(request),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let current = PREFERENCE.load(Ordering::Acquire);
        if preference != current {
            output = None;
            pending = None;
            preference = current;
            retry_at = Instant::now();
        }
        if preference & 1 == 0 {
            continue;
        }
        if let Some(active) = &mut output {
            if let Err(error) = active.check() {
                diagnostics.report(error);
                output = None;
                retry_at = Instant::now() + RETRY;
            }
        }
        if Instant::now() >= probe_at
            || request
                .as_ref()
                .is_some_and(|request| request.sound == Sound::Alert)
        {
            probe_at = Instant::now() + DEVICE_POLL;
            if let Some(active) = &output {
                match default_device_name() {
                    Ok(name) if name != active.device => {
                        output = None;
                        retry_at = Instant::now();
                    }
                    Err(error) => {
                        diagnostics.report(error);
                        output = None;
                        retry_at = Instant::now() + RETRY;
                    }
                    _ => {}
                }
            }
        }
        if let Some(request) = request.filter(Request::current) {
            if pending.is_none() || request.sound == Sound::Alert {
                pending = Some(request);
            }
        }
        if pending.as_ref().is_some_and(|request| !request.current()) {
            pending = None;
        }
        if pending.is_none() {
            continue;
        }
        if output.is_none() && Instant::now() >= retry_at {
            match Output::open() {
                Ok(active) => output = Some(active),
                Err(error) => {
                    diagnostics.report(format!(
                        "Could not open the default sound output: {error:#}"
                    ));
                    retry_at = Instant::now() + RETRY;
                }
            }
        }
        if let Some(active) = &output {
            if let Some(request) = pending.take().filter(Request::current) {
                if let Some(clip) = match request.sound {
                    Sound::Tick => &tick,
                    Sound::Alert => &alert,
                } {
                    active.mixer.add(clip.clone().amplify(VOLUME));
                }
            }
        }
    }
}

fn default_device_name() -> Result<String> {
    cpal::default_host()
        .default_output_device()
        .context("No default sound output is available")?
        .name()
        .context("Could not identify the default sound output")
}

struct Output {
    _stream: cpal::Stream,
    device: String,
    mixer: Arc<DynamicMixerController<f32>>,
    faults: Receiver<cpal::StreamError>,
    heartbeat: Arc<AtomicU64>,
    last_heartbeat: u64,
    last_progress: Instant,
}

impl Output {
    fn open() -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .context("No default sound output is available")?;
        let mut error = match device.default_output_config() {
            Ok(format) => match Self::open_format(&device, format) {
                Ok(output) => return Ok(output),
                Err(error) => error,
            },
            Err(error) => error.into(),
        };
        for format in device.supported_output_configs()? {
            let rate = 48_000u32.clamp(format.min_sample_rate().0, format.max_sample_rate().0);
            match Self::open_format(&device, format.with_sample_rate(cpal::SampleRate(rate))) {
                Ok(output) => return Ok(output),
                Err(failure) => error = failure,
            }
        }
        Err(error)
    }

    fn open_format(device: &cpal::Device, format: cpal::SupportedStreamConfig) -> Result<Self> {
        let name = device.name()?;
        let config = format.config();
        let (mixer, samples) = dynamic_mixer::mixer::<f32>(config.channels, config.sample_rate.0);
        let heartbeat = Arc::new(AtomicU64::new(0));
        let (faults, receiver) = mpsc::sync_channel(1);
        let stream = match format.sample_format() {
            cpal::SampleFormat::I8 => {
                stream::<i8>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::I16 => {
                stream::<i16>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::I32 => {
                stream::<i32>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::I64 => {
                stream::<i64>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::U8 => {
                stream::<u8>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::U16 => {
                stream::<u16>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::U32 => {
                stream::<u32>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::U64 => {
                stream::<u64>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::F32 => {
                stream::<f32>(device, &config, samples, faults, heartbeat.clone())
            }
            cpal::SampleFormat::F64 => {
                stream::<f64>(device, &config, samples, faults, heartbeat.clone())
            }
            format => anyhow::bail!("Unsupported sound output format: {format}"),
        }?;
        stream.play()?;
        Ok(Self {
            _stream: stream,
            device: name,
            mixer,
            faults: receiver,
            heartbeat,
            last_heartbeat: 0,
            last_progress: Instant::now(),
        })
    }

    fn check(&mut self) -> Result<()> {
        if let Ok(error) = self.faults.try_recv() {
            anyhow::bail!("The sound output failed: {error}");
        }
        let heartbeat = self.heartbeat.load(Ordering::Relaxed);
        if heartbeat != self.last_heartbeat {
            self.last_heartbeat = heartbeat;
            self.last_progress = Instant::now();
        }
        anyhow::ensure!(
            self.last_progress.elapsed() < STALL_TIMEOUT,
            "The sound output stopped responding"
        );
        Ok(())
    }
}

fn stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut samples: DynamicMixer<f32>,
    faults: SyncSender<cpal::StreamError>,
    heartbeat: Arc<AtomicU64>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                heartbeat.fetch_add(1, Ordering::Relaxed);
                for sample in data {
                    *sample = T::from_sample(samples.next().unwrap_or(0.0).clamp(-1.0, 1.0));
                }
            },
            move |error| {
                let _ = faults.try_send(error);
            },
            None,
        )
        .context("Could not create the sound output stream")
}
