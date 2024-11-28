use indicatif::ProgressStyle;
use indicatif::ProgressBar;
use indicatif::MultiProgress;

pub struct ProgressBars {
    multi_progress_bar: MultiProgress,
    total_progress_bar: ProgressBar,
    job_progress_bar_style: ProgressStyle,
}

impl ProgressBars {
    pub fn new(num_jobs: u64) -> Self {
        let multi_progress_bar = MultiProgress::new();

        let total_progress_bar_style = ProgressStyle::with_template("[{elapsed_precise}] [{bar:50.green/blue}] {pos:>9}/{len:9} eta: {eta_precise}")
            .expect("main progress style template should be valid")
            .progress_chars("=>-");

        let total_progress_bar = multi_progress_bar.add(ProgressBar::new(num_jobs));
        total_progress_bar.set_style(total_progress_bar_style);
        total_progress_bar.tick();

        let job_progress_bar_style = ProgressStyle::with_template("[{prefix}] {msg}")
            .expect("worker progress style template should be valid")
            .progress_chars("=>-");

        Self {
            multi_progress_bar,
            total_progress_bar,
            job_progress_bar_style,
        }
    }

    pub fn inc_total(&self) {
        self.total_progress_bar.inc(1);
    }

    pub fn new_job_progress_bar(&self, worker_id: usize) -> ProgressBar {
        let job_progress_bar = self.multi_progress_bar.add(ProgressBar::no_length());
        job_progress_bar.set_style(self.job_progress_bar_style.clone());
        job_progress_bar.set_prefix(format!("worker {}", worker_id));
        job_progress_bar.tick();
        return job_progress_bar;
    }

    pub fn remove_job_progress_bar(&self, job_progress_bar: &ProgressBar) {
        self.multi_progress_bar.remove(job_progress_bar);
    }

    pub fn finish(&self) {
        self.total_progress_bar.finish();
    }
}
