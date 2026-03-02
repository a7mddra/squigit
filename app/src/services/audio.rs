// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::{
    io::Cursor,
    sync::mpsc::{self, Receiver, Sender},
};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

const DIALOG_WARNING_SOUND: &[u8] =
    include_bytes!("../../../ui/src/assets/sounds/dialog-warning.mp3");

#[derive(Debug, Clone, Copy)]
pub enum UiSoundEffect {
    DialogWarning,
}

impl UiSoundEffect {
    pub fn from_input(input: Option<&str>) -> Result<Self, String> {
        match input
            .unwrap_or("dialog-warning")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "dialog-warning" | "dialog_warning" | "warning" => Ok(Self::DialogWarning),
            other => Err(format!("Unsupported ui sound effect: {}", other)),
        }
    }
}

pub struct UiSoundPlayer {
    tx: Sender<UiSoundEffect>,
}

impl UiSoundPlayer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<UiSoundEffect>();
        std::thread::Builder::new()
            .name("ui-sound-worker".to_string())
            .spawn(move || run_audio_worker(rx))
            .expect("Failed to spawn ui sound worker");
        Self { tx }
    }

    pub fn play(&self, effect: UiSoundEffect) -> Result<(), String> {
        self.tx
            .send(effect)
            .map_err(|error| format!("Failed to queue ui sound playback: {error}"))
    }
}

impl Default for UiSoundPlayer {
    fn default() -> Self {
        Self::new()
    }
}

fn run_audio_worker(rx: Receiver<UiSoundEffect>) {
    let mut backend = AudioBackend::new();
    while let Ok(effect) = rx.recv() {
        backend.play(effect);
    }
}

struct AudioBackend {
    stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sinks: Vec<Sink>,
}

impl AudioBackend {
    fn new() -> Self {
        let mut backend = Self {
            stream: None,
            stream_handle: None,
            sinks: Vec::new(),
        };
        backend.try_init_output();
        backend
    }

    fn try_init_output(&mut self) {
        if self.stream.is_some() && self.stream_handle.is_some() {
            return;
        }
        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                self.stream = Some(stream);
                self.stream_handle = Some(handle);
            }
            Err(error) => {
                self.stream = None;
                self.stream_handle = None;
                log::warn!(
                    "UI sound output device is unavailable. Dialog sounds are disabled: {}",
                    error
                );
            }
        }
    }

    fn play(&mut self, effect: UiSoundEffect) {
        self.sinks.retain(|sink| !sink.empty());
        self.try_init_output();

        let Some(handle) = self.stream_handle.as_ref() else {
            return;
        };

        let source_bytes = match effect {
            UiSoundEffect::DialogWarning => DIALOG_WARNING_SOUND,
        };

        let cursor = Cursor::new(source_bytes);
        let Ok(decoder) = Decoder::new(cursor) else {
            log::warn!("Failed to decode ui sound payload");
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            log::warn!("Failed to initialize ui sound sink");
            return;
        };
        sink.append(decoder);
        self.sinks.push(sink);
    }
}
