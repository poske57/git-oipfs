use std::env;
use std::error::Error;
use std::fmt;
use std::time::Duration;

use git2::{ErrorCode, Repository};
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

fn is_repository_updated(repo: &Repository) -> Result<bool, git2::Error> {
    let mut remote = repo.find_remote("origin")?;

    // TODO: Progress をinfoに表示
    remote.fetch(&["main"], None, None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _preference) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_up_to_date() {
        return Ok(false);
    }

    if analysis.is_fast_forward() {
        do_fast_forward(repo, &fetch_commit)?;
        return Ok(true);
    }

    if analysis.is_normal() {
        do_merge(repo, &fetch_commit)?;
        return Ok(true);
    }

    Err(git2::Error::from_str(
        "unable to update repository: analysis does not permit pull",
    ))
}

fn do_fast_forward(repo: &Repository, fetch_commit: &git2::AnnotatedCommit) -> Result<(), git2::Error> {
    let refname = "refs/heads/main";
    let mut reference = match repo.find_reference(refname) {
        Ok(r) => r,
        Err(_) => repo.reference(refname, fetch_commit.id(), true, "Fast-forward to fetched main")?,
    };

    reference.set_target(fetch_commit.id(), "Fast-forward to fetched main")?;
    repo.set_head(refname)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(())
}

fn do_merge(repo: &Repository, fetch_commit: &git2::AnnotatedCommit) -> Result<(), git2::Error> {
    let our_commit = repo.head()?.peel_to_commit()?;
    let our_tree = our_commit.tree()?;

    let their_commit = repo.find_commit(fetch_commit.id())?;
    let their_tree = their_commit.tree()?;

    let ancestor_oid = repo.merge_base(our_commit.id(), their_commit.id())?;
    let ancestor_tree = repo.find_commit(ancestor_oid)?.tree()?;

    let mut index = repo.merge_trees(&ancestor_tree, &our_tree, &their_tree, None)?;

    if index.has_conflicts() {
        return Err(git2::Error::from_str(
            "merge conflicts occurred; manual resolution required",
        ));
    }

    let tree_id = index.write_tree_to(repo)?;
    let tree = repo.find_tree(tree_id)?;

    let sig = repo.signature()?;

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Merge remote branch 'origin/main'",
        &tree,
        &[&our_commit, &their_commit],
    )?;

    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(())
}

fn clone_if_not_exist(s: &Settings) -> Result<Repository, Box<dyn Error>> {
    match Repository::open(&s.repository_path) {
        Ok(repo) => Ok(repo),
        Err(e) if e.code() == ErrorCode::NotFound => {
            let repo = Repository::clone(&s.repository_url, &s.repository_path)?;
            Ok(repo)
        }
        Err(e) => Err(e.into()),
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
