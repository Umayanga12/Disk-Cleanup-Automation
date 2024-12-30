use std::env;
use std::fs;
use std::io::Write;
use std::time::{Duration, SystemTime};
use sysinfo::{DiskExt, SystemExt};

fn calculate_percentage(total: u64, available: u64) -> f64 {
    (available as f64 / total as f64) * 100.0
}

fn check_storage(base_dir: &str) -> Option<f64> {
    let sys = sysinfo::System::new_all();
    let disk_name = sys
        .disks()
        .iter()
        .find(|disk| base_dir.starts_with(&*disk.mount_point().to_string_lossy()))
        .map(|disk| disk.name().to_str())
        .flatten();

    if let Some(disk_name) = disk_name {
        for disk in sys.disks() {
            if disk.name().to_str() == Some(disk_name) {
                let total_space = disk.total_space();
                let available_space = disk.available_space();
                return Some(calculate_percentage(total_space, available_space));
            }
        }
    }
    None
}

fn get_oldest_folder(dir_path: &str) -> std::io::Result<Option<String>> {
    let mut oldest_folder: Option<String> = None;
    let mut oldest_time: Option<SystemTime> = None;

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let metadata = fs::metadata(&path)?;
            let folder_time = metadata.created()?;

            if oldest_time.is_none() || folder_time < oldest_time.unwrap() {
                oldest_time = Some(folder_time);
                oldest_folder = Some(path.to_string_lossy().to_string());
            }
        }
    }
    Ok(oldest_folder)
}

fn delete_folder(folder_path: &str) -> std::io::Result<()> {
    fs::remove_dir_all(folder_path)?;
    Ok(())
}

fn log_message(log_path: &str, message: &str) -> std::io::Result<()> {
    let log_file_path = format!("{}/cleanup.log", log_path);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;
    writeln!(file, "{}", message)?;
    Ok(())
}

fn clean_log(log_path: &str) -> std::io::Result<()> {
    let log_file_path = format!("{}/cleanup.log", log_path);
    if let Ok(metadata) = fs::metadata(&log_file_path) {
        if let Ok(modified) = metadata.modified() {
            if modified.elapsed().unwrap_or(Duration::ZERO) > Duration::from_secs(7 * 24 * 60 * 60)
            {
                fs::remove_file(&log_file_path)?;
            }
        }
    }
    Ok(())
}

fn clean_disk(base_dir: &str, log_path: &str) -> std::io::Result<()> {
    loop {
        let free_space_percentage = check_storage(base_dir).unwrap_or(100.0);
        if free_space_percentage > 25.0 {
            log_message(log_path, "Free space is above 25%. Exiting cleanup.")?;
            break;
        }

        log_message(log_path, "Free space is below 25%. Cleaning up...")?;

        for folder in fs::read_dir(base_dir)? {
            let folder = folder?;
            let folder_path = folder.path();

            if folder_path.is_dir() {
                for subfolder in fs::read_dir(&folder_path)? {
                    let subfolder = subfolder?;
                    let subfolder_path = subfolder.path();

                    if subfolder_path.is_dir() {
                        if let Some(oldest_folder) =
                            get_oldest_folder(&subfolder_path.to_string_lossy())?
                        {
                            log_message(log_path, &format!("Deleting folder: {}", oldest_folder))?;
                            delete_folder(&oldest_folder)?;
                        } else {
                            log_message(
                                log_path,
                                &format!(
                                    "No subfolders found in: {}",
                                    subfolder_path.to_string_lossy()
                                ),
                            )?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let env_file = ".env";
    dotenv::from_path(env_file).expect("Failed to read .env file");
    let base_dir = env::var("DIRPATH").expect("DIRPATH not set in .env");
    let log_path = env::var("LOGPATH").expect("LOGPATH not set in .env");

    clean_log(&log_path)?;

    if let Some(free_space_percentage) = check_storage(&base_dir) {
        log_message(
            &log_path,
            &format!("Current free space: {:.2}%", free_space_percentage),
        )?;

        if free_space_percentage < 20.0 {
            log_message(&log_path, "Free space below threshold. Starting cleanup...")?;
            clean_disk(&base_dir, &log_path)?;
        } else {
            log_message(&log_path, "Sufficient free space. No cleanup needed.")?;
        }
    } else {
        log_message(&log_path, "Disk not found for the base directory.")?;
    }

    Ok(())
}
