use gix::Repository;
use std::error::Error;
use std::path::Path;
use std::time::Duration;

fn main() {
    let dir = Path::new("/repo");
    let url = String::from("https://github.com/poske57/portfolio.git");
    let repo = clone_if_not_exist(dir, url);
    println!("{:?}", repo);
    loop {
        println!("reconciliation!");
        std::thread::sleep(Duration::from_secs(15 * 60));
    }
}

fn clone_if_not_exist(dir: &Path, url: String) -> Result<Repository, Box<dyn Error>> {
    if dir.exists() {
        let repo = gix::open(dir)?;
        return Ok(repo);
    }
    let (repo, _) = gix::prepare_clone(url, dir)?
        .fetch_only(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    return Ok(repo);
}
