use std::env;
use std::error::Error;
use std::fmt;
use std::time::Duration;

use gix::{ObjectId, Repository};
use tracing::{error, info};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let settings = match Settings::load() {
        Ok(s) => s,
        Err(e) => {
            error!(err = %e, "Failed to load settings");
            std::process::exit(1);
        }
    };

    loop {
        info!("Start periodic reconciliation.");
        if let Err(e) = reconciliation(&settings) {
            error!(err = %e, "Reconciliation failed.");
        }
        std::thread::sleep(Duration::from_secs(
            settings.reconciliation_cycle * 60,
        ));
    }
}

fn reconciliation(s: &Settings) -> Result<(), Box<dyn Error>> {
    let repo = clone_if_not_exist(s)?;

    if !is_repository_updated(&repo)? {
        info!("There are no updates to the repository.");
        return Ok(());
    }

    // IPFS
    Ok(())
}

fn is_repository_updated(repo: &Repository) -> Result<bool, Box<dyn Error>> {
    let remote = repo.find_remote("origin")?;

    // TODO: Progress をinfoに表示
    let should_interrupt = std::sync::atomic::AtomicBool::new(false);
    let connection = remote.connect(gix::remote::Direction::Fetch)?;
    connection
        .prepare_fetch(gix::progress::Discard, Default::default())?
        .receive(gix::progress::Discard, &should_interrupt)?;

    let mut fetch_head = repo.find_reference("refs/remotes/origin/main")?;
    let fetch_commit = fetch_head.peel_to_id()?.detach();
    let our_commit = repo.head_id()?.detach();

    if our_commit == fetch_commit {
        return Ok(false);
    }

    // `merge_base(our, fetch)` returning `our` means `fetch` is a descendant
    // of `our` (fast-forwardable). Returning `fetch` means already up-to-date.
    match repo.merge_base(our_commit, fetch_commit) {
        Ok(base) if base.detach() == our_commit => {
            do_fast_forward(repo, fetch_commit)?;
            Ok(true)
        }
        Ok(_) => {
            do_merge(repo, fetch_commit)?;
            Ok(true)
        }
        // No common ancestry, treat as merge.
        Err(_) => {
            do_merge(repo, fetch_commit)?;
            Ok(true)
        }
    }
}

fn do_fast_forward(repo: &Repository, fetch_commit: ObjectId) -> Result<(), Box<dyn Error>> {
    let refname = "refs/heads/main";
    let mut reference = match repo.find_reference(refname) {
        Ok(r) => r,
        Err(_) => repo.reference(
            refname,
            fetch_commit,
            gix::refs::transaction::PreviousValue::Any,
            "Fast-forward to fetched main",
        )?,
    };

    reference.set_target_id(fetch_commit, "Fast-forward to fetched main")?;

    checkout_head(repo)?;
    Ok(())
}

fn do_merge(repo: &Repository, fetch_commit: ObjectId) -> Result<(), Box<dyn Error>> {
    let our_commit = repo.head_id()?.detach();

    let labels = gix::merge::blob::builtin_driver::text::Labels {
        ancestor: Some("ancestor".as_bytes().into()),
        current: Some("HEAD".as_bytes().into()),
        other: Some("origin/main".as_bytes().into()),
    };

    let mut outcome = repo.merge_commits(
        our_commit,
        fetch_commit,
        labels,
        repo.tree_merge_options()?.into(),
    )?;

    if !outcome.tree_merge.conflicts.is_empty() {
        return Err("merge conflicts occurred; manual resolution required".into());
    }

    let tree_id = outcome.tree_merge.tree.write()?;

    repo.commit(
        "HEAD",
        "Merge remote branch 'origin/main'",
        tree_id,
        [our_commit, fetch_commit],
    )?;

    checkout_head(repo)?;
    Ok(())
}

fn checkout_head(repo: &Repository) -> Result<(), Box<dyn Error>> {
    let tree_id = repo.head_tree_id()?.detach();
    let mut index = repo.index_from_tree(&tree_id)?;

    let workdir = repo
        .workdir()
        .ok_or("cannot checkout: repository has no work tree")?;

    let opts = repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)?;

    let should_interrupt = std::sync::atomic::AtomicBool::new(false);
    gix::worktree::state::checkout(
        &mut index,
        workdir,
        repo.objects.clone().into_arc()?,
        &gix::progress::Discard,
        &gix::progress::Discard,
        &should_interrupt,
        opts,
    )?;

    index.write(Default::default())?;
    Ok(())
}

fn clone_if_not_exist(s: &Settings) -> Result<Repository, Box<dyn Error>> {
    match gix::open(&s.repository_path) {
        Ok(repo) => Ok(repo),
        Err(_) => {
            let should_interrupt = std::sync::atomic::AtomicBool::new(false);
            let mut prepare_fetch = gix::prepare_clone(
                s.repository_url.clone(),
                &s.repository_path,
            )?;
            let (mut prepare_checkout, _outcome) =
                prepare_fetch.fetch_then_checkout(gix::progress::Discard, &should_interrupt)?;
            let (repo, _outcome) =
                prepare_checkout.main_worktree(gix::progress::Discard, &should_interrupt)?;
            Ok(repo)
        }
    }
}

struct Settings {
    repository_path: String,
    repository_url: String,
    reconciliation_cycle: u64,
}

impl Settings {
    fn load() -> Result<Self, Box<dyn Error>> {
        let repository_path = env::var("OIPFS_REPOSITORY_PATH")
            .map_err(|_| EnvironmentVariableNotFound("OIPFS_REPOSITORY_PATH"))?;

        let repository_url = env::var("OIPFS_REPOSITORY_URL")
            .map_err(|_| EnvironmentVariableNotFound("OIPFS_REPOSITORY_URL"))?;

        let reconciliation_cycle = env::var("OIPFS_RECONCILIATION_CYCLE")
            .map_err(|_| EnvironmentVariableNotFound("OIPFS_RECONCILIATION_CYCLE"))?
            .parse::<u64>()
            .map_err(|e| format!("invalid OIPFS_RECONCILIATION_CYCLE: {e}"))?;

        Ok(Settings {
            repository_path,
            repository_url,
            reconciliation_cycle,
        })
    }
}

#[derive(Debug)]
struct EnvironmentVariableNotFound(&'static str);

impl fmt::Display for EnvironmentVariableNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "environment variable not found: {}", self.0)
    }
}

impl Error for EnvironmentVariableNotFound {}
