//! Git bundle creation and extraction using gix.
//!
//! Creates git bundle v2 format files for transferring repos between
//! workspaces without requiring a git CLI or server.

use std::io::Write;
use std::path::Path;

use crate::error::Error;

/// Create a git bundle containing all refs and objects from the repo.
/// The bundle is the standard v2 git bundle format: header + pack data.
///
/// For now, we use a simpler approach: export the repo as a tar of the
/// .git directory's essential files. This avoids implementing pack
/// generation from scratch while still using gix for all git operations.
///
/// TODO: Replace with proper pack-based bundle once gix exposes the
/// necessary APIs for creating packs from a set of objects.
pub fn create_bundle(project_dir: &Path, output_path: &Path) -> Result<(), Error> {
    let repo = gix::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("open repo: {e}")))?;

    // Get all refs to include in the bundle header.
    let mut refs: Vec<(String, String)> = Vec::new();

    let ref_store = repo.references()
        .map_err(|e| Error::NonBlocking(format!("refs: {e}")))?;

    for reference in ref_store.all()
        .map_err(|e| Error::NonBlocking(format!("iterate refs: {e}")))?
    {
        if let Ok(r) = reference {
            let name = r.name().as_bstr().to_string();
            if let Some(id) = r.target().try_id() {
                refs.push((id.to_string(), name));
            }
        }
    }

    if refs.is_empty() {
        return Err(Error::NonBlocking("no refs to bundle".to_string()));
    }

    // Write bundle v2 format.
    let mut file = std::fs::File::create(output_path)
        .map_err(|e| Error::NonBlocking(format!("create bundle: {e}")))?;

    // Header.
    writeln!(file, "# v2 git bundle")
        .map_err(|e| Error::NonBlocking(format!("write header: {e}")))?;

    // Refs.
    for (oid, name) in &refs {
        writeln!(file, "{oid} {name}")
            .map_err(|e| Error::NonBlocking(format!("write ref: {e}")))?;
    }

    // Empty line separates header from pack data.
    writeln!(file)
        .map_err(|e| Error::NonBlocking(format!("write separator: {e}")))?;

    // Close the bundle file — we'll rewrite it as a tar of the .git dir.
    // A proper pack-based bundle would use gix_pack to generate pack data,
    // but the API for creating packs from scratch isn't easily accessible yet.
    drop(file);
    drop(refs);

    // Use tar to bundle the .git directory — all objects, refs, config.
    let git_dir = if project_dir.join(".git").is_file() {
        // Worktree — read the gitdir path.
        let content = std::fs::read_to_string(project_dir.join(".git"))
            .map_err(|e| Error::NonBlocking(format!("read .git: {e}")))?;
        let gitdir = content
            .strip_prefix("gitdir: ")
            .unwrap_or(&content)
            .trim();
        std::path::PathBuf::from(gitdir)
    } else {
        project_dir.join(".git")
    };

    let tar_file = std::fs::File::create(output_path)
        .map_err(|e| Error::NonBlocking(format!("create tar: {e}")))?;
    let mut tar = tar::Builder::new(tar_file);
    tar.append_dir_all(".git", &git_dir)
        .map_err(|e| Error::NonBlocking(format!("tar .git: {e}")))?;
    tar.finish()
        .map_err(|e| Error::NonBlocking(format!("finish tar: {e}")))?;

    Ok(())
}

/// Extract a bundle into a project directory, setting up the repo
/// and checking out the specified branch.
pub fn extract_bundle(
    bundle_path: &Path,
    project_dir: &Path,
    branch: &str,
) -> Result<(), Error> {
    std::fs::create_dir_all(project_dir)
        .map_err(|e| Error::NonBlocking(format!("create dir: {e}")))?;

    // Extract the tar archive.
    let tar_file = std::fs::File::open(bundle_path)
        .map_err(|e| Error::NonBlocking(format!("open bundle: {e}")))?;
    let mut tar = tar::Archive::new(tar_file);
    tar.unpack(project_dir)
        .map_err(|e| Error::NonBlocking(format!("unpack: {e}")))?;

    // Use gix to checkout the branch.
    let repo = gix::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("open unpacked repo: {e}")))?;

    // Set HEAD to the branch and checkout.
    let branch_ref = format!("refs/heads/{branch}");
    let reference = repo
        .find_reference(&branch_ref)
        .map_err(|e| Error::NonBlocking(format!("find branch {branch}: {e}")))?;

    let commit_id = reference
        .target()
        .try_id()
        .ok_or_else(|| Error::NonBlocking(format!("branch {branch} is not a direct ref")))?
        .to_owned();

    // Write HEAD.
    std::fs::write(
        project_dir.join(".git").join("HEAD"),
        format!("ref: {branch_ref}\n"),
    )
    .map_err(|e| Error::NonBlocking(format!("write HEAD: {e}")))?;

    // Checkout the tree using gix.
    let commit = repo
        .find_object(commit_id)
        .map_err(|e| Error::NonBlocking(format!("find commit: {e}")))?
        .into_commit();
    let tree = commit
        .tree()
        .map_err(|e| Error::NonBlocking(format!("commit tree: {e}")))?;

    // Walk the tree and write files to the working directory.
    checkout_tree(&repo, &tree, project_dir)?;

    Ok(())
}

/// Recursively checkout a tree to the working directory.
fn checkout_tree(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    base_dir: &Path,
) -> Result<(), Error> {
    for entry in tree.iter() {
        let entry = entry.map_err(|e| Error::NonBlocking(format!("tree entry: {e}")))?;
        let name = entry.filename().to_string();
        let path = base_dir.join(&name);

        match entry.mode().kind() {
            gix::objs::tree::EntryKind::Blob | gix::objs::tree::EntryKind::BlobExecutable => {
                let obj = repo
                    .find_object(entry.oid())
                    .map_err(|e| Error::NonBlocking(format!("find blob: {e}")))?;
                std::fs::write(&path, &*obj.data)
                    .map_err(|e| Error::NonBlocking(format!("write file {name}: {e}")))?;

                // Set executable bit if needed.
                if entry.mode().kind() == gix::objs::tree::EntryKind::BlobExecutable {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let perms = std::fs::Permissions::from_mode(0o755);
                        let _ = std::fs::set_permissions(&path, perms);
                    }
                }
            }
            gix::objs::tree::EntryKind::Tree => {
                std::fs::create_dir_all(&path)
                    .map_err(|e| Error::NonBlocking(format!("mkdir {name}: {e}")))?;
                let subtree = repo
                    .find_object(entry.oid())
                    .map_err(|e| Error::NonBlocking(format!("find tree: {e}")))?
                    .into_tree();
                checkout_tree(repo, &subtree, &path)?;
            }
            _ => {} // Skip symlinks etc for now.
        }
    }

    Ok(())
}
