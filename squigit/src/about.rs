use rune::{col, el, text, App, AppSpec, Cx, El, Pixels, WindowOptions};
use rune_kit::Button;

struct About {
    icon: Pixels,
}

fn init() -> About {
    let image = image::load_from_memory(crate::ICON)
        .expect("embedded Squigit icon")
        .into_rgba8();
    let icon = Pixels::new();
    icon.set(image.width(), image.height(), image.into_raw());
    About { icon }
}

fn view(state: &About, cx: &mut Cx<About>) -> El {
    let k = crate::settings::ui::kit(cx.system_dark());
    col()
        .fill(k.c.page)
        .padding(24.0)
        .gap(10.0)
        .items_center()
        .child(el().size(64.0, 64.0).pixels(&state.icon))
        .child(text("Squigit").font(k.ty.body_medium).color(k.c.text_base))
        .child(
            text(format!("Version {}", env!("CARGO_PKG_VERSION")))
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(text("© Squigit").font(k.ty.small).color(k.c.text_faint))
        .child(
            Button::new("OK")
                .on_press_tx(|_: &mut About, tx| tx.close_window())
                .build(&k),
        )
}

pub fn app() -> App<impl Sized> {
    App::new(AppSpec::new("About Squigit", (340.0, 264.0), init, view)).window_options(
        WindowOptions {
            min_size: Some((340.0, 264.0)),
            resizable: false,
            ..Default::default()
        },
    )
}
