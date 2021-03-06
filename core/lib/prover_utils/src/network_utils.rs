use super::{SETUP_MAX_POW2, SETUP_MIN_POW2};
use crate::fs_utils;
use anyhow::format_err;
use backoff::Operation;
use reqwest::blocking::Response;
use std::time::Duration;

/// Downloads universal setup in the monomial form of the given power of two (range: SETUP_MIN_POW2..=SETUP_MAX_POW2)
pub fn download_universal_setup_monomial_form(power_of_two: u32) -> Result<(), anyhow::Error> {
    anyhow::ensure!(
        (SETUP_MIN_POW2..=SETUP_MAX_POW2).contains(&power_of_two),
        "setup power of two is not in the correct range"
    );

    let mut retry_op = move || try_to_download_setup(power_of_two);

    let mut response = retry_op
        .retry_notify(&mut get_backoff(), |err, next_after: Duration| {
            let duration_secs = next_after.as_millis() as f32 / 1000.0f32;

            vlog::warn!(
                "Failed to download setup err: <{}>, retrying after: {:.1}s",
                err,
                duration_secs,
            )
        })
        .map_err(|e| {
            format_err!(
                "Can't download setup, max elapsed time of the backoff reached: {}",
                e
            )
        })?;

    fs_utils::save_universal_setup_monomial_file(power_of_two, &mut response)?;
    Ok(())
}

fn try_to_download_setup(power_of_two: u32) -> Result<Response, backoff::Error<anyhow::Error>> {
    let setup_network_dir = std::env::var("MISC_PROVER_SETUP_NETWORK_DIR")
        .map_err(|e| backoff::Error::Permanent(e.into()))?;

    let setup_dl_path = format!("{}/setup_2%5E{}.key", setup_network_dir, power_of_two);

    vlog::info!("Downloading universal setup from {}", &setup_dl_path);

    reqwest::blocking::get(&setup_dl_path).map_err(|e| backoff::Error::Transient(e.into()))
}

fn get_backoff() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoff {
        current_interval: Duration::from_secs(5),
        initial_interval: Duration::from_secs(5),
        multiplier: 1.2f64,
        max_interval: Duration::from_secs(80),
        max_elapsed_time: Some(Duration::from_secs(10 * 60)),
        ..Default::default()
    }
}
