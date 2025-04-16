use git2::{FetchOptions, RemoteCallbacks};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;

/// Prepares the limine bootloader
pub fn prepare_bootloader(limine_branch: &str, file_dir: &Path) {
    let limine_dir = file_dir.join("limine");
    // Stores the old version, so that the crate re-clones if the branch has changed
    let meta_path = limine_dir.join("meta.old");
    let old_branch = std::fs::read_to_string(&meta_path).unwrap_or_default();
    if old_branch == limine_branch {
        // Nothing to do
        return;
    }

    // We first remove the old version, so that we can re-clone
    std::fs::remove_dir_all(&limine_dir).ok();

    let multi = MultiProgress::new();
    let pb = multi.add(ProgressBar::new(100));
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
        .unwrap()
        .progress_chars("#>-"));

    pb.set_message("Cloning limine...");
    let start_time = std::time::Instant::now();

    let mut callbacks = RemoteCallbacks::new();
    callbacks.transfer_progress(|stats| {
        // Rough calculations, we just do integer division
        let progress = stats.received_objects() * 100 / stats.total_objects();
        pb.set_position(progress as u64);
        pb.set_message(format!(
            "Objects: {}/{}, Deltas: {}/{}",
            stats.received_objects(),
            stats.total_objects(),
            stats.indexed_deltas(),
            stats.total_deltas()
        ));
        true
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    fetch_options.depth(1);
    fetch_options.download_tags(git2::AutotagOption::None);
    fetch_options.update_fetchhead(false);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);
    builder.branch(limine_branch);

    const LIMINE_GIT: &str = "https://github.com/limine-bootloader/limine";
    let repo = builder.clone(LIMINE_GIT, &limine_dir).unwrap();

    let duration = std::time::Instant::now()
        .duration_since(start_time)
        .as_secs_f32();
    pb.finish_with_message(format!("Clone completed in {:.2}s", duration));

    let checkout_pb = multi.add(ProgressBar::new_spinner());
    checkout_pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    checkout_pb.set_message(format!("Checking out branch {}", limine_branch));

    let obj = repo
        .revparse_single(&format!("origin/{}", limine_branch))
        .unwrap();
    repo.checkout_tree(&obj, None).unwrap();
    repo.set_head(&format!("refs/heads/{}", limine_branch))
        .unwrap();

    let duration = std::time::Instant::now()
        .duration_since(start_time)
        .as_secs_f32();
    println!();
    checkout_pb.finish_with_message(format!(
        "Branch {} checked out in {:.2}s",
        limine_branch, duration
    ));

    std::fs::write(&meta_path, limine_branch).expect("failed to write to target/limine/meta");
}
