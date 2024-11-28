use std::collections::VecDeque;
use std::sync::Arc;

use config::Config;
use progress_bars::ProgressBars;
use worker::{Job, SharedData, Worker};

mod config;
mod progress_bars;
mod worker;

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::parse_and_validate()?;

    let mut job_queue = VecDeque::new();

    for entry in std::fs::read_dir(&config.source_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }

        let source_file_name = match entry.file_name().to_str() {
            Some(file_name_str) => file_name_str.to_string(),
            None => continue,
        };

        let frame_name = match source_file_name.strip_suffix(&config.source_suffix) {
            Some(id) => id,
            None => continue,
        };

        let dest_file_name = format!("{}{}", frame_name, config.dest_suffix);
        let dest_path = config.dest_dir.join(&dest_file_name);

        let job = Job::new(entry.path(), source_file_name, dest_path);

        job_queue.push_back(job);
    }

    let progress = ProgressBars::new(job_queue.len() as u64);
    let shared_data = Arc::new(SharedData::new(config.clone(), job_queue, progress));

    let mut handles = Vec::new();
    for worker_id in 0..config.num_workers {
        handles.push(Worker::spawn(worker_id, shared_data.clone()));
    }
    for handle in handles {
        handle.join().map_err(|err| anyhow::anyhow!(format!("{:?}", err)))?;
    }

    shared_data.progress().finish();

    Ok(())
}
