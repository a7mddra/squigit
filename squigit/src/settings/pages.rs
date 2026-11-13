use std::time::Instant;

use rune::{col, el, row, text, Drag, El, Phase, ScrollSpec, TextStyle};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Loader, ProgressCircle};
use squigit_ocr::models::{OcrModelDefinition, DEFAULT_OCR_MODEL_ID, OCR_MODELS};

use super::{ui, App, DeleteGesture, DELETE_ARM, DELETE_HOLD, SPINNER_GRACE};

pub fn active(status: &str) -> bool {
    status.starts_with("checking_")
        || matches!(
            status,
            "internet_available"
                | "model_not_installed"
                | "model_available"
                | "downloading"
                | "paused"
                | "extracting"
                | "cancelling"
        )
}

pub fn page(s: &App, k: &Kit, hair: f32) -> El {
    let mut rows = Vec::new();
    for model in OCR_MODELS
        .iter()
        .filter(|model| model.id != DEFAULT_OCR_MODEL_ID)
    {
        rows.push(model_row(s, k, model));
    }
    let mut inner = col()
        .key("models")
        .w_full()
        .max_w(660.0)
        .gap(40.0)
        .child(
            row().key("header").w_full().gap(16.0).center_y().child(
                text("OCR Models")
                    .key("title")
                    .font(TextStyle::new(28.0, 450.0))
                    .color(k.c.text_base)
                    .grow(1.0),
            ),
        )
        .child(
            ui::section(k, hair, "Get more models", rows)
                .key("model-downloads")
                .focusable(true),
        );
    if let Some(error) = &s.error {
        inner = inner.child(
            col()
                .key("error")
                .role("alert")
                .w_full()
                .gap(8.0)
                .child(text(error.clone()).font(k.ty.small).color(k.c.danger.fg))
                .child(
                    Button::new("Dismiss")
                        .on_press(|s: &mut App| s.error = None)
                        .build(k),
                ),
        );
    }
    col()
        .key("page/models")
        .grow(1.0)
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .px(28.0)
        .pt(24.0)
        .pb(24.0)
        .child(inner)
}

fn model_row(s: &App, k: &Kit, model: &OcrModelDefinition) -> El {
    let id = model.id;
    let job = s.job(id);
    let status = job.map_or("idle", |job| job.status.as_str());
    let busy = active(status);
    let installed = s.installed.contains(id) && !busy;
    let pending = s.pending.contains(id);
    let mut actions = row().key("actions").w(126.0).gap(6.0).center_y();
    if busy {
        let cancelable = !matches!(status, "extracting" | "cancelling");
        if cancelable {
            actions = actions.child(
                Button::new("Cancel")
                    .control_size(ControlSize::Large)
                    .on_press(move |s: &mut App| s.cancel(id))
                    .build(k)
                    .key("cancel")
                    .grow(1.0)
                    .label(format!("Cancel {} download", model.name)),
            );
        } else {
            actions = actions.child(el().grow(1.0));
        }
        let progress = job.map_or(0, |job| job.progress);
        let grace = s
            .started
            .get(id)
            .is_none_or(|started| started.elapsed() < SPINNER_GRACE);
        let known = job.is_some_and(|job| job.total > 0);
        let indicator = if status == "extracting" || (status == "downloading" && !grace && known) {
            ProgressCircle::new(if status == "extracting" {
                1.0
            } else {
                s.visual.get(id).copied().unwrap_or(0.0) / 100.0
            })
            .size(16.0)
            .label(format!("Downloading {}, {progress}%", model.name))
            .build(k)
        } else {
            Loader::new().size(16.0).build(k)
        };
        let tooltip = match job {
            Some(job) if job.total > 0 => {
                format!("{progress}% · {} / {} MB", mb(job.loaded), mb(job.total))
            }
            Some(job) if job.loaded > 0 => format!("{} MB downloaded", mb(job.loaded)),
            _ if status == "extracting" => format!("Installing {}", model.name),
            _ if status == "cancelling" => "Cancelling download".to_string(),
            _ => format!("Preparing {} download", model.name),
        };
        actions = actions.child(indicator.key("progress").help(tooltip));
    } else if installed {
        actions = actions.child(delete_button(s, k, id, model.name).disabled(pending));
    } else {
        actions = actions.child(
            Button::new(if pending { "Please wait" } else { "Download" })
                .control_size(ControlSize::Large)
                .disabled(pending || s.models.is_none() || s.runtime.is_none())
                .on_press(move |s: &mut App| s.download(id))
                .build(k)
                .key("download")
                .w_full()
                .label(format!("Download {}", model.name)),
        );
    }
    row()
        .key(id)
        .w_full()
        .min_h(44.0)
        .py(7.0)
        .gap(16.0)
        .center_y()
        .child(
            col()
                .key("meta")
                .gap(4.0)
                .grow(1.0)
                .shrink()
                .child(
                    text(model.name)
                        .font(TextStyle::new(13.0, 450.0))
                        .color(k.c.text_base),
                )
                .child(
                    text(model.size)
                        .font(TextStyle::new(12.0, 400.0))
                        .color(k.c.text_muted),
                ),
        )
        .child(actions)
}

fn mb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_000_000.0)
}

fn delete_button(s: &App, k: &Kit, id: &'static str, name: &str) -> El {
    let gesture = s.delete.as_ref().filter(|gesture| gesture.id == id);
    let held = gesture
        .and_then(|gesture| gesture.holding)
        .map_or(0.0, |started| {
            (started.elapsed().as_secs_f32() / DELETE_HOLD.as_secs_f32()).min(1.0)
        });
    Button::new(if gesture.is_some() {
        "Hold to Delete"
    } else {
        "Delete"
    })
    .variant(ButtonVariant::Danger)
    .control_size(ControlSize::Large)
    .help("Click, then hold for 2 seconds to delete")
    .build(k)
    .key("delete")
    .w_full()
    .label(format!("Delete {name}"))
    .on_press_tx(move |s: &mut App, tx| {
        if s.delete.as_ref().is_none_or(|gesture| gesture.id != id) {
            s.delete = Some(DeleteGesture {
                id: id.to_string(),
                armed: Instant::now(),
                holding: None,
            });
            tx.wake_after(DELETE_ARM);
        }
    })
    .on_drag(move |s: &mut App, drag: &mut Drag, tx| match drag.phase {
        Phase::Down => {
            if let Some(gesture) = s
                .delete
                .as_mut()
                .filter(|gesture| gesture.id == id && gesture.armed.elapsed() < DELETE_ARM)
            {
                gesture.holding = Some(Instant::now());
                tx.wake_after(std::time::Duration::from_millis(16));
            }
        }
        Phase::Move => {
            if drag.x < 0.0 || drag.x > 126.0 || drag.y < 0.0 || drag.y > 28.0 {
                s.delete = None;
            }
        }
        Phase::Up => {
            if drag.cancelled() {
                s.delete = None;
            } else if let Some(gesture) = s.delete.as_mut().filter(|gesture| gesture.id == id) {
                gesture.holding = None;
                gesture.armed = Instant::now();
            }
        }
    })
    .child(
        el().key("hold-fill")
            .absolute(0.0, 0.0)
            .h_full()
            .w_frac(held)
            .fill(k.c.danger.fg)
            .opacity(0.12)
            .a11y_hidden(),
    )
}
