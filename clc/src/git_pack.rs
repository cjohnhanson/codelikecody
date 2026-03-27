//! Git pack creation and extraction using gix.
//!
//! Creates git pack files for transferring repos between the supervisor
//! and workspaces. The supervisor creates a pack of all objects reachable
//! from a branch, serves it via the API. The workspace receives it and
//! unpacks into a ready-to-use repo with the branch checked out.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use crate::error::Error;

/// Response from creating a pack: the pack data and the refs.
pub struct PackData {
    /// The pack file bytes (git pack format v2).
    pub pack: Vec<u8>,
    /// Refs to set up: (oid_hex, refname).
    pub refs: Vec<(String, String)>,
}

/// Create a pack containing all objects reachable from `branch` in the
/// repo at `project_dir`. Returns the raw pack bytes and ref list.
pub fn create_pack(project_dir: &Path, branch: &str) -> Result<PackData, Error> {
    let repo = gix::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("open repo: {e}")))?;

    // Find the branch tip.
    let branch_ref = format!("refs/heads/{branch}");
    let reference = repo
        .find_reference(&branch_ref)
        .map_err(|e| Error::NonBlocking(format!("find ref {branch}: {e}")))?;
    let tip_id = reference
        .target()
        .try_id()
        .ok_or_else(|| Error::NonBlocking(format!("{branch} is not a direct ref")))?
        .to_owned();

    // Collect all reachable object IDs: commits, trees, blobs.
    let mut object_ids: Vec<gix::ObjectId> = Vec::new();
    let mut seen = HashSet::new();

    // Walk commits.
    let walk = repo
        .rev_walk([tip_id])
        .all()
        .map_err(|e| Error::NonBlocking(format!("rev walk: {e}")))?;

    for info in walk {
        let info = info.map_err(|e| Error::NonBlocking(format!("walk: {e}")))?;
        let commit_id = info.id;

        if !seen.insert(commit_id) {
            continue;
        }
        object_ids.push(commit_id);

        // Get the commit's tree and walk it.
        let commit = repo
            .find_object(commit_id)
            .map_err(|e| Error::NonBlocking(format!("find commit: {e}")))?;
        let tree_id = commit
            .into_commit()
            .tree_id()
            .map_err(|e| Error::NonBlocking(format!("tree id: {e}")))?
            .detach();

        collect_tree_objects(&repo, tree_id, &mut object_ids, &mut seen)?;
    }

    // Write pack format v2.
    let pack = write_pack(&repo, &object_ids)?;

    // Refs.
    let refs = vec![(tip_id.to_string(), branch_ref)];

    Ok(PackData { pack, refs })
}

/// Recursively collect all tree and blob object IDs.
fn collect_tree_objects(
    repo: &gix::Repository,
    tree_id: gix::ObjectId,
    ids: &mut Vec<gix::ObjectId>,
    seen: &mut HashSet<gix::ObjectId>,
) -> Result<(), Error> {
    if !seen.insert(tree_id) {
        return Ok(());
    }
    ids.push(tree_id);

    let tree = repo
        .find_object(tree_id)
        .map_err(|e| Error::NonBlocking(format!("find tree: {e}")))?
        .into_tree();

    for entry in tree.iter() {
        let entry = entry.map_err(|e| Error::NonBlocking(format!("tree entry: {e}")))?;
        let oid = entry.oid().to_owned();

        match entry.mode().kind() {
            gix::objs::tree::EntryKind::Blob | gix::objs::tree::EntryKind::BlobExecutable => {
                if seen.insert(oid) {
                    ids.push(oid);
                }
            }
            gix::objs::tree::EntryKind::Tree => {
                collect_tree_objects(repo, oid, ids, seen)?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Write a git pack v2 from a list of object IDs.
///
/// Pack format:
///   4 bytes: "PACK"
///   4 bytes: version (2)
///   4 bytes: object count (big-endian)
///   for each object:
///     varint: type + size header
///     deflated object data
///   20 bytes: SHA1 checksum of everything above
fn write_pack(repo: &gix::Repository, ids: &[gix::ObjectId]) -> Result<Vec<u8>, Error> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    let mut buf = Vec::new();

    // Header.
    buf.extend_from_slice(b"PACK");
    buf.extend_from_slice(&2u32.to_be_bytes()); // version
    buf.extend_from_slice(&(ids.len() as u32).to_be_bytes()); // object count

    for &oid in ids {
        let obj = repo
            .find_object(oid)
            .map_err(|e| Error::NonBlocking(format!("find object {oid}: {e}")))?;

        let obj_type: u8 = match obj.kind {
            gix::object::Kind::Commit => 1,
            gix::object::Kind::Tree => 2,
            gix::object::Kind::Blob => 3,
            gix::object::Kind::Tag => 4,
        };

        let data: &[u8] = &obj.data;
        let size = data.len();

        // Write type+size header as a varint.
        // First byte: MSB=continuation, bits 6-4=type, bits 3-0=size[3:0]
        let mut header_size = size;
        let first_byte = ((obj_type & 0x7) << 4) | (header_size & 0xF) as u8;
        header_size >>= 4;

        if header_size > 0 {
            buf.push(first_byte | 0x80); // continuation bit
            // Remaining bytes: MSB=continuation, bits 6-0=size
            while header_size > 0 {
                let byte = (header_size & 0x7F) as u8;
                header_size >>= 7;
                if header_size > 0 {
                    buf.push(byte | 0x80);
                } else {
                    buf.push(byte);
                }
            }
        } else {
            buf.push(first_byte);
        }

        // Deflate the object data.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(data)
            .map_err(|e| Error::NonBlocking(format!("deflate: {e}")))?;
        let compressed = encoder
            .finish()
            .map_err(|e| Error::NonBlocking(format!("deflate finish: {e}")))?;
        buf.extend_from_slice(&compressed);
    }

    // SHA1 checksum of everything.
    use sha1::Digest;
    let hash = sha1::Sha1::digest(&buf);
    buf.extend_from_slice(&hash);

    Ok(buf)
}

/// Receive a pack + refs and set up a working repo at `project_dir`.
/// Creates the object store, writes pack, sets refs, checks out the tree.
pub fn receive_pack(
    pack_data: &[u8],
    refs: &[(String, String)],
    project_dir: &Path,
    branch: &str,
) -> Result<(), Error> {
    // Create the git directory structure.
    let git_dir = project_dir.join(".git");
    std::fs::create_dir_all(git_dir.join("objects").join("pack"))
        .map_err(|e| Error::NonBlocking(format!("create objects dir: {e}")))?;
    std::fs::create_dir_all(git_dir.join("refs").join("heads"))
        .map_err(|e| Error::NonBlocking(format!("create refs dir: {e}")))?;

    // Unpack objects from the pack into loose object store.
    // This avoids needing a pack index — each object is written
    // individually to .git/objects/xx/yyyy.
    unpack_to_loose(pack_data, &git_dir)?;

    // Write HEAD and refs.
    let branch_ref = format!("refs/heads/{branch}");
    std::fs::write(git_dir.join("HEAD"), format!("ref: {branch_ref}\n"))
        .map_err(|e| Error::NonBlocking(format!("write HEAD: {e}")))?;

    for (oid, refname) in refs {
        let ref_path = git_dir.join(refname);
        if let Some(parent) = ref_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&ref_path, format!("{oid}\n"))
            .map_err(|e| Error::NonBlocking(format!("write ref {refname}: {e}")))?;
    }

    // Write minimal config.
    std::fs::write(
        git_dir.join("config"),
        "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n",
    )
    .map_err(|e| Error::NonBlocking(format!("write config: {e}")))?;

    // Checkout the tree using gix.
    let repo = gix::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("open repo: {e}")))?;

    let branch_oid: gix::ObjectId = refs
        .iter()
        .find(|(_, name)| name == &branch_ref)
        .map(|(oid, _)| oid.parse())
        .ok_or_else(|| Error::NonBlocking(format!("branch {branch} not in refs")))?
        .map_err(|e| Error::NonBlocking(format!("parse oid: {e}")))?;

    let commit = repo
        .find_object(branch_oid)
        .map_err(|e| Error::NonBlocking(format!("find commit: {e}")))?
        .into_commit();
    let tree = commit
        .tree()
        .map_err(|e| Error::NonBlocking(format!("tree: {e}")))?;

    checkout_tree(&repo, &tree, project_dir)?;

    // Write the git index so `git status` reports a clean tree.
    // Without this, the index is empty and git sees all files as
    // untracked or deleted.
    write_index_from_tree(&repo, &tree, project_dir)?;

    Ok(())
}

/// Unpack a pack file into loose objects in .git/objects/.
fn unpack_to_loose(pack_data: &[u8], git_dir: &Path) -> Result<(), Error> {
    use flate2::read::ZlibDecoder;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use sha1::Digest;
    use std::io::Read;

    let objects_dir = git_dir.join("objects");

    // Skip header: "PACK" (4) + version (4) + count (4) = 12 bytes.
    if pack_data.len() < 12 || &pack_data[..4] != b"PACK" {
        return Err(Error::NonBlocking("invalid pack header".to_string()));
    }

    let count = u32::from_be_bytes([pack_data[8], pack_data[9], pack_data[10], pack_data[11]]) as usize;
    let mut offset = 12;

    for _ in 0..count {
        if offset >= pack_data.len() - 20 {
            break; // Reached checksum.
        }

        // Parse type+size header.
        let first = pack_data[offset];
        offset += 1;
        let obj_type = (first >> 4) & 0x7;
        let mut size = (first & 0xF) as usize;
        let mut shift = 4;

        if first & 0x80 != 0 {
            loop {
                if offset >= pack_data.len() {
                    break;
                }
                let byte = pack_data[offset];
                offset += 1;
                size |= ((byte & 0x7F) as usize) << shift;
                shift += 7;
                if byte & 0x80 == 0 {
                    break;
                }
            }
        }

        let type_str = match obj_type {
            1 => "commit",
            2 => "tree",
            3 => "blob",
            4 => "tag",
            _ => {
                return Err(Error::NonBlocking(format!("unsupported pack object type {obj_type}")));
            }
        };

        // Decompress the object data.
        // Use DecompressError tracking to get exact bytes consumed.
        let mut decoder = ZlibDecoder::new(&pack_data[offset..]);
        let mut data = Vec::with_capacity(size);
        decoder
            .read_to_end(&mut data)
            .map_err(|e| Error::NonBlocking(format!("decompress object at offset {offset}: {e}")))?;

        if data.len() != size {
            return Err(Error::NonBlocking(format!(
                "size mismatch: header says {size}, got {}",
                data.len()
            )));
        }

        // Advance offset past the compressed data.
        offset += decoder.total_in() as usize;

        // Compute SHA1 of the loose object format: "type size\0data".
        let header = format!("{type_str} {size}\0");
        let mut hasher = sha1::Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(&data);
        let hash = hasher.finalize();
        let hex = format!("{hash:x}");

        // Write as loose object: .git/objects/xx/yyyyyy (zlib-compressed).
        let obj_dir = objects_dir.join(&hex[..2]);
        std::fs::create_dir_all(&obj_dir)
            .map_err(|e| Error::NonBlocking(format!("mkdir {}: {e}", &hex[..2])))?;

        let obj_path = obj_dir.join(&hex[2..]);
        if !obj_path.exists() {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(header.as_bytes())
                .map_err(|e| Error::NonBlocking(format!("compress header: {e}")))?;
            encoder
                .write_all(&data)
                .map_err(|e| Error::NonBlocking(format!("compress data: {e}")))?;
            let compressed = encoder
                .finish()
                .map_err(|e| Error::NonBlocking(format!("finish compress: {e}")))?;
            std::fs::write(&obj_path, compressed)
                .map_err(|e| Error::NonBlocking(format!("write object: {e}")))?;
        }
    }

    Ok(())
}

/// Import a pack from a workspace into the host repo.
/// Unpacks objects and updates refs. Used for landing — pulling
/// worker commits back to the host for merging.
pub fn import_pack(
    pack_data: &[u8],
    refs: &[(String, String)],
    project_dir: &Path,
) -> Result<(), Error> {
    let git_dir = project_dir.join(".git");

    // Unpack objects into the existing repo's object store.
    unpack_to_loose(pack_data, &git_dir)?;

    // Update refs.
    for (oid, refname) in refs {
        let ref_path = git_dir.join(refname);
        if let Some(parent) = ref_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&ref_path, format!("{oid}\n"))
            .map_err(|e| Error::NonBlocking(format!("write ref {refname}: {e}")))?;
    }

    Ok(())
}

/// Recursively checkout a tree to the working directory.
/// Write a git index file from the tree so `git status` sees a clean state.
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
                        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
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
