use anyhow::{Context, Result};
use std::path::PathBuf;
use url::Url;

use crate::{audio::Player, auth, image_hosting, notifications, settings, storage::AppData};

pub enum Target {
    Lens,
    Ai(String),
}

pub fn check_configuration(
    data: AppData,
    settings: rune::AppHandle<settings::App>,
    sounds: Player,
) {
    std::thread::spawn(move || {
        match data
            .load()
            .and_then(|config| image_hosting::configured(&data, &config.hosting))
        {
            Ok(true) => {}
            Ok(false) => notifications::missing_image_hosting_key(settings, sounds),
            Err(error) => notifications::image_search_error(
                format!("Could not check image hosting: {error:#}"),
                false,
                settings,
                sounds,
            ),
        }
    });
}

pub fn submit(
    data: AppData,
    image: PathBuf,
    target: Target,
    settings: rune::AppHandle<settings::App>,
    sounds: Player,
) {
    std::thread::spawn(move || {
        let result = (|| {
            let lease = data.blob_lock()?;
            fs2::FileExt::lock_shared(&lease)?;
            let config = data.load()?.hosting;
            if !image_hosting::configured(&data, &config)? {
                notifications::missing_image_hosting_key(settings.clone(), sounds.clone());
                return Ok(());
            }
            let Some(key) = auth::load(&data, config.provider, &config.chevereto_url)? else {
                notifications::missing_image_hosting_key(settings.clone(), sounds.clone());
                return Ok(());
            };
            let hosted = image_hosting::upload(&image, &key, &config)?;
            let url = google_url(&hosted, target)?;
            opener::open_browser(url.as_str())
                .context("Could not open image search in your browser")?;
            Ok::<_, anyhow::Error>(())
        })();
        if let Err(error) = result {
            notifications::image_search_error(format!("{error:#}"), false, settings, sounds);
        }
    });
}

fn google_url(image: &Url, target: Target) -> Result<Url> {
    let mut url = Url::parse("https://lens.google.com/uploadbyurl")?;
    url.query_pairs_mut()
        .append_pair("url", image.as_str())
        .append_pair("ep", "subb")
        .append_pair("re", "df")
        .append_pair("s", "4");
    if let Target::Ai(prompt) = target {
        anyhow::ensure!(!prompt.trim().is_empty(), "Enter a prompt for Ask Google");
        url.query_pairs_mut()
            .append_pair("udm", "50")
            .append_pair("q", prompt.trim());
    }
    Ok(url)
}
