use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle, Source};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

const TICK: &[u8] = include_bytes!("../assets/tick.wav");
const HEADER_LENGTH: usize = 44;
const SAMPLE_RATE: u32 = 44_100;
const CHANNELS: u16 = 1;
const VOLUME: f32 = 0.5;

#[derive(Clone)]
pub struct Player {
    sender: SyncSender<()>,
}

impl Player {
    pub fn start() -> Self {
        let samples = TICK[HEADER_LENGTH..]
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32_768.0)
            .collect::<Vec<_>>();
        let (sender, receiver) = sync_channel(64);
        let _ = thread::Builder::new()
            .name("squigit-tick".to_string())
            .spawn(move || {
                let mut output = OutputStream::try_default().ok();
                while receiver.recv().is_ok() {
                    play(&mut output, &samples);
                }
            });
        Self { sender }
    }

    pub fn play(&self) {
        let _ = self.sender.try_send(());
    }
}

fn play(output: &mut Option<(OutputStream, OutputStreamHandle)>, samples: &[f32]) {
    if output.is_none() {
        *output = OutputStream::try_default().ok();
    }
    if !play_now(output, samples) {
        *output = OutputStream::try_default().ok();
        play_now(output, samples);
    }
}

fn play_now(output: &Option<(OutputStream, OutputStreamHandle)>, samples: &[f32]) -> bool {
    output.as_ref().is_some_and(|(_, handle)| {
        handle
            .play_raw(SamplesBuffer::new(CHANNELS, SAMPLE_RATE, samples.to_vec()).amplify(VOLUME))
            .is_ok()
    })
}
