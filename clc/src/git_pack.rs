//! Git repo transfer between supervisor and workspaces.
//!
//! Transfers the .git directory as a tar archive. The host's .git/ has
//! all objects in gix-native format (pack files, loose objects, indices).
//! No custom pack parsing — gix reads the transferred data natively.

use std::path::Path;

use crate::error::Error;

/// Tar archive of .git/ plus ref metadata.
pub struct PackData {
    /// Tar archive of the .git directory (gzipped).
    pub pack: Vec<u8>,
    /// Refs: (oid_hex, refname). Kept for API compatibility.
    pub refs: Vec<(String, String)>,
}

/// Create a tar archive of the .git directory for the given branch.
pub fn create_pack(project_dir: &Path, branch: &str) -> Result<PackData, Error> {
    let git_dir = project_dir.join(".git");
    if !git_dir.is_dir() {
        return Err(Error::NonBlocking("no .git directory".into()));
    }

    // Resolve the branch ref for the refs metadata.
    let repo = gix::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("open repo: {e}")))?;
    let branch_ref = format!("refs/heads/{branch}");
    let reference = repo
        .find_reference(&branch_ref)
        .map_err(|e| Error::NonBlocking(format!("find ref {branch}: {e}")))?;
    let tip_id = reference
        .target()
        .try_id()
        .ok_or_else(|| Error::NonBlocking(format!("{branch} is not a direct ref")))?
        .to_owned();

    // Tar the .git directory with gzip compression.
    let buf = Vec::new();
    let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);

    tar.append_dir_all(".git", &git_dir)
        .map_err(|e| Error::NonBlocking(format!("tar .git: {e}")))?;

    let encoder = tar.into_inner()
        .map_err(|e| Error::NonBlocking(format!("tar finish: {e}")))?;
    let compressed = encoder.finish()
        .map_err(|e| Error::NonBlocking(format!("gzip finish: {e}")))?;

    Ok(PackData {
        pack: compressed,
        refs: vec![(tip_id.to_string(), branch_ref)],
    })
}

/// Receive a tar archive and set up a working repo at `project_dir`.
/// Extracts .git/, then checks out the tree and writes the index.
pub fn receive_pack(
    pack_data: &[u8],
    _refs: &[(String, String)],
    project_dir: &Path,
    branch: &str,
) -> Result<(), Error> {
    std::fs::create_dir_all(project_dir)
        .map_err(|e| Error::NonBlocking(format!("create project dir: {e}")))?;

    // Extract the tar archive.
    let decoder = flate2::read::GzDecoder::new(pack_data);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(project_dir)
        .map_err(|e| Error::NonBlocking(format!("untar .git: {e}")))?;

    // Open with gix — it reads the native .git/ directly.
    let repo = gix::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("open repo after untar: {e}")))?;

    // Ensure HEAD points to the requested branch.
    let branch_ref = format!("refs/heads/{branch}");
    let head_path = project_dir.join(".git").join("HEAD");
    std::fs::write(&head_path, format!("ref: {branch_ref}\n"))
        .map_err(|e| Error::NonBlocking(format!("write HEAD: {e}")))?;

    // Resolve the branch tip.
    let tip_ref = repo
        .find_reference(&branch_ref)
        .map_err(|e| Error::NonBlocking(format!("find ref {branch}: {e}")))?;
    let commit = tip_ref
        .id()
        .object()
        .map_err(|e| Error::NonBlocking(format!("find commit: {e}")))?
        .into_commit();
    let tree = commit
        .tree()
        .map_err(|e| Error::NonBlocking(format!("tree: {e}")))?;

    // Checkout files and write index.
    checkout_tree(&repo, &tree, project_dir)?;
    write_index_from_tree(&repo, &tree, project_dir)?;

    Ok(())
}

/// Import objects from a workspace back into the host repo for landing.
/// Extracts the tar into a temp dir, then copies new objects and refs
/// into the host repo.
pub fn import_pack(
    pack_data: &[u8],
    refs: &[(String, String)],
    project_dir: &Path,
) -> Result<(), Error> {
    // Extract to a temp dir so we can selectively copy objects.
    let tmp = tempfile::tempdir()
        .map_err(|e| Error::NonBlocking(format!("tempdir: {e}")))?;

    let decoder = flate2::read::GzDecoder::new(pack_data);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(tmp.path())
        .map_err(|e| Error::NonBlocking(format!("untar worker .git: {e}")))?;

    let src_git = tmp.path().join(".git");
    let dst_git = project_dir.join(".git");

    // Copy loose objects that don't exist in the host.
    let src_objects = src_git.join("objects");
    if src_objects.is_dir() {
        for entry in walkdir::WalkDir::new(&src_objects)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let rel = entry.path().strip_prefix(&src_objects).unwrap_or(entry.path());
            // Skip pack files — we only want loose objects and new refs.
            if rel.starts_with("pack") {
                continue;
            }
            let dst = dst_git.join("objects").join(rel);
            if !dst.exists() {
                if let Some(parent) = dst.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::copy(entry.path(), &dst);
            }
        }
    }

    // Update refs.
    for (oid, refname) in refs {
        let ref_path = dst_git.join(refname);
        if let Some(parent) = ref_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&ref_path, format!("{oid}\n"))
            .map_err(|e| Error::NonBlocking(format!("write ref {refname}: {e}")))?;
    }

    Ok(())
}

/// Write a git index from the tree so `git status` reports a clean tree.
fn write_index_from_tree(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    project_dir: &Path,
) -> Result<(), Error> {
    let tree_id = tree.id;
    let index_state = gix::index::State::from_tree(
        &tree_id,
        &repo.objects,
        Default::default(),
    )
    .map_err(|e| Error::NonBlocking(format!("build index from tree: {e}")))?;

    let index_path = project_dir.join(".git").join("index");
    let mut index = gix::index::File::from_state(index_state, index_path);
    index
        .write(gix::index::write::Options::default())
        .map_err(|e| Error::NonBlocking(format!("write index: {e}")))?;

    Ok(())
}

/// Recursively checkout a tree to the working directory.
fn checkout_tree(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    base_dir: &Path,
) -> Result<(), Error> {
    for entry in tree.iter() {
        let entry = entry.map_err(|e| Error::NonBlocking(format!("entry: {e}")))?;
        let name = entry.filename().to_string();
        let path = base_dir.join(&name);

        match entry.mode().kind() {
            gix::objs::tree::EntryKind::Blob | gix::objs::tree::EntryKind::BlobExecutable => {
                let obj = repo
                    .find_object(entry.oid())
                    .map_err(|e| Error::NonBlocking(format!("find blob: {e}")))?;
                std::fs::write(&path, &*obj.data)
                    .map_err(|e| Error::NonBlocking(format!("write {name}: {e}")))?;

                if entry.mode().kind() == gix::objs::tree::EntryKind::BlobExecutable {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &path,
                            std::fs::Permissions::from_mode(0o755),
                        );
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
            gix::objs::tree::EntryKind::Link => {
                let obj = repo
                    .find_object(entry.oid())
                    .map_err(|e| Error::NonBlocking(format!("find link: {e}")))?;
                let target = String::from_utf8_lossy(&obj.data);
                #[cfg(unix)]
                {
                    let _ = std::fs::remove_file(&path);
                    std::os::unix::fs::symlink(target.as_ref(), &path)
                        .map_err(|e| Error::NonBlocking(format!("symlink {name}: {e}")))?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
