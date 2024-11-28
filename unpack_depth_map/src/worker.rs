use std::thread::JoinHandle;
use std::sync::Mutex;
use std::sync::Arc;
use std::path::PathBuf;
use std::io::BufWriter;
use std::fs::File;
use std::collections::VecDeque;

use indicatif::ProgressBar;

use crate::config::Config;
use crate::progress_bars::ProgressBars;

pub struct Worker {
    shared_data: Arc<SharedData>,
    job_progress: ProgressBar,
    source_buffer: Vec<u8>,
    dest_buffer: Vec<u8>,
}

pub struct SharedData {
    config: Config,
    job_queue: Mutex<VecDeque<Job>>,
    progress_bars: ProgressBars,
}

pub struct Job {
    source_path: PathBuf,
    source_file_name: String,
    dest_path: PathBuf,
}

impl Worker {
    pub fn spawn(id: usize, data: Arc<SharedData>) -> JoinHandle<()> {
        let job_progress = data.progress_bars.new_job_progress_bar(id);
        std::thread::spawn(move || Worker::new(job_progress, data).run())
    }
}

impl SharedData {
    pub fn new(config: Config, job_queue: VecDeque<Job>, progress: ProgressBars) -> Self {
        Self {
            config,
            job_queue: Mutex::new(job_queue),
            progress_bars: progress,
        }
    }

    pub fn progress(&self) -> &ProgressBars {
        &self.progress_bars
    }
}

impl Job {
    pub fn new(source_path: PathBuf, source_file_name: String, dest_path: PathBuf) -> Self {
        Self {
            source_path,
            source_file_name,
            dest_path,
        }
    }
}

impl Worker {
    fn new(job_progress: ProgressBar, data: Arc<SharedData>) -> Self {
        Worker {
            shared_data: data,
            job_progress,
            source_buffer: Vec::new(),
            dest_buffer: Vec::new(),
        }
    }

    fn run(&mut self) {
        while let Some(job) = self.get_job() {
            self.run_job(&job).ok();
        }
        self.shared_data.progress_bars.remove_job_progress_bar(&self.job_progress);
    }

    fn get_job(&self) -> Option<Job> {
        self.shared_data.job_queue.lock().ok()?.pop_front()
    }

    fn run_job(&mut self, job: &Job) -> anyhow::Result<()> {
        self.job_progress.set_message(format!("unpacking {}", job.source_file_name));

        let source_file = File::open(&job.source_path)?;
        let (width, height) = Self::decode(source_file, &mut self.source_buffer)?;

        self.unpack(width, height);

        let dest_file = File::create(&job.dest_path)?;
        Self::encode(dest_file, width, height, &self.dest_buffer)?;

        self.shared_data.progress_bars.inc_total();

        Ok(())
    }

    fn unpack(&mut self, width: u32, height: u32) {
        let num_pixels = (width * height) as usize;

        let near = self.shared_data.config.near;
        let far = self.shared_data.config.far;

        self.dest_buffer.clear();
        self.dest_buffer.reserve(num_pixels);

        for source_pixel in self.source_buffer.chunks_exact(4) {
            let r = source_pixel[0];
            let g = source_pixel[1];
            let b = source_pixel[2];
            let a = source_pixel[3];

            let bits
                = (a as u32) << 8 * 3
                | (r as u32) << 8 * 2
                | (g as u32) << 8 * 1
                | (b as u32) << 8 * 0;

            let depth = f32::from_bits(bits);
            let intensity = ((depth - far) / (near - far)).clamp(0.0, 1.0);
            let dest_pixel = (255.0 * intensity) as u8;

            self.dest_buffer.push(dest_pixel);
        }
        self.job_progress.finish();
    }

    fn decode(source_file: File, source_buffer: &mut Vec<u8>) -> anyhow::Result<(u32, u32)> {
        let decoder = png::Decoder::new(source_file);
        let mut reader = decoder.read_info()?;

        source_buffer.clear();
        source_buffer.extend(std::iter::repeat(0).take(reader.output_buffer_size()));

        let info = reader.next_frame(source_buffer)?;

        if info.color_type != png::ColorType::Rgba {
            return Err(anyhow::anyhow!("Unexpected color type"));
        }

        if info.bit_depth != png::BitDepth::Eight {
            return Err(anyhow::anyhow!("Unexpected bit depth"));
        }

        source_buffer.truncate(info.buffer_size());

        Ok((info.width, info.height))
    }

    fn encode(dest_file: File, width: u32, height: u32, dest_buffer: &[u8]) -> anyhow::Result<()> {
        let ref mut w = BufWriter::new(dest_file);
        let mut encoder = png::Encoder::new(w, width, height);

        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(dest_buffer)?;

        Ok(())
    }
}
