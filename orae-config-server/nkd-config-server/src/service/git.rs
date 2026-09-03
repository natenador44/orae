use std::path::Path;

use anyhow::{Result, anyhow};
use git2::Repository;

pub fn read_file_from_branch(repo: &Repository, branch: &str, file_path: &str) -> Result<String> {
    let ref_name = format!("refs/remotes/origin/{}", branch);
    let reference = repo.find_reference(&ref_name)?;
    let commit = reference.peel_to_commit()?;
    let tree = commit.tree()?;

    let entry = tree.get_path(Path::new(file_path))?;
    let object = entry.to_object(repo)?;

    let blob = object
        .into_blob()
        .map_err(|_| anyhow!("Entry is not a file"))?;

    Ok(String::from_utf8(blob.content().to_vec())?)
}
