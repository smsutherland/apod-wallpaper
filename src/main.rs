use std::num::NonZeroU32;

use chrono::prelude::*;
use regex::Regex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = match download_apod_today() {
        Ok(path) => path,
        // Try to use the most recent day.
        Err(_) => match try_get_prev_path() {
            Ok(path) => path,
            // Default to restoring.
            Err(_) => {
                std::process::Command::new("nitrogen")
                    .arg("--restore")
                    .output()?;
                return Ok(());
            }
        },
    };
    std::process::Command::new("nitrogen")
        .arg("--set-zoom-fill")
        .arg(path)
        .output()?;
    Ok(())
}

fn download_apod_today() -> Result<String, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME")?;
    let parent = format!("{home}/apod");
    let today = Local::now().date_naive();
    let path = format!("{}/{}.png", parent, today.format("%Y%m%d"));

    if !std::path::Path::new(&path).exists() {
        let re = Regex::new("<IMG SRC=\"(.+)\"").unwrap();

        let body = try_n_times_download("https://apod.nasa.gov/apod/", unsafe {
            NonZeroU32::new_unchecked(3)
        })?
        .into_string()?;

        let img = re
            .captures(&body)
            .ok_or("failed to match image source")?
            .get(1)
            .ok_or("Failed to find image source in match.")?;

        std::fs::create_dir_all(&parent)?;

        let mut reader = try_n_times_download(
            &format!("https://apod.nasa.gov/apod/{}", img.as_str()),
            unsafe { NonZeroU32::new_unchecked(3) },
        )?
        .into_reader();
        let mut writer = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)?;
        std::io::copy(&mut reader, &mut writer)?;
    }

    Ok(path)
}

fn try_n_times_download(url: &str, times: NonZeroU32) -> Result<ureq::Response, ureq::Error> {
    let mut times = times.get();
    loop {
        match ureq::get(url).call() {
            response @ Ok(_) => return response,
            err @ Err(_) => {
                if times == 1 {
                    return err;
                }
            }
        }
        times -= 1;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn try_get_prev_path() -> Result<String, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME")?;
    let parent = format!("{home}/apod");
    let entries = std::fs::read_dir(parent)?;
    let last = entries.last().ok_or("")??;
    Ok(last.file_name().into_string().map_err(|_| "")?)
}
