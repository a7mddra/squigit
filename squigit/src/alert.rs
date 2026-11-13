use rodio::{Decoder, OutputStream, OutputStreamHandle, Source};
use std::io::Cursor;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

use crate::storage::AppData;

const ALERT: &[u8] = include_bytes!("../assets/alert.mp3");
const VOLUME: f32 = 0.5;

#[derive(Clone)]
pub struct Player {
    sender: SyncSender<()>,
}

impl Player {
    pub fn start(data: AppData) -> Self {
        let (sender, receiver) = sync_channel(64);
        let _ = thread::Builder::new()
            .name("squigit-alert".to_string())
            .spawn(move || {
                let mut output = None;
                while receiver.recv().is_ok() {
                    if data.load().is_ok_and(|config| config.general.play_sounds) {
                        play(&mut output);
                    }
                }
            });
        Self { sender }
    }

    pub fn play(&self) {
        let _ = self.sender.try_send(());
    }
}

fn play(output: &mut Option<(OutputStream, OutputStreamHandle)>) {
    if output.is_none() {
        *output = OutputStream::try_default().ok();
    }
    if !play_now(output) {
        *output = OutputStream::try_default().ok();
        play_now(output);
    }
}

fn play_now(output: &Option<(OutputStream, OutputStreamHandle)>) -> bool {
    output.as_ref().is_some_and(|(_, handle)| {
        Decoder::new_mp3(Cursor::new(ALERT)).is_ok_and(|source| {
            handle
                .play_raw(source.convert_samples::<f32>().amplify(VOLUME))
                .is_ok()
        })
    })
}
