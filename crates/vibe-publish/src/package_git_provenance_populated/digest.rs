//! Pure, run-independent populated-Gitlink semantic plan digest.
use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::package_git_gitlink::{GitIndexCensus, GitlinkCandidate};
use crate::package_git_operation::journal::{GitlinkPending, PopulatedIndexLink};
use crate::package_git_submodule::{MetadataBlob, SubmoduleInspection};

const DOMAIN: &[u8] = b"vibevm/package-git-populated-provenance/e1\0";

fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    })
}
fn oid(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn normalized(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn frame(hash: &mut Sha256, label: &[u8], value: &[u8]) {
    hash.update((label.len() as u64).to_be_bytes());
    hash.update(label);
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

pub(crate) fn semantic_digest(
    index: &GitIndexCensus,
    proofs: &[SubmoduleInspection],
) -> Result<String, String> {
    let GitIndexCensus::PopulatedCandidate {
        repository_root_token,
        package_prefix,
        index_sha256,
        links,
    } = index
    else {
        return Err("complete populated index witness absent".into());
    };
    if links.is_empty()
        || !digest(index_sha256)
        || repository_root_token.is_empty()
        || proofs.len() != links.len()
        || links.windows(2).any(|pair| pair[0].path >= pair[1].path)
    {
        return Err("populated index is empty, duplicated, unordered, or incomplete".into());
    }
    let object_len = links[0].indexed_oid.len();
    if !matches!(object_len, 40 | 64) {
        return Err("Gitlink OID algorithm unsupported".into());
    }
    let mut by_path = BTreeMap::new();
    for proof in proofs {
        let SubmoduleInspection::ExactCandidate { path, .. } = proof else {
            return Err("non-exact Gitlink comparison".into());
        };
        if by_path.insert(path.as_str(), proof).is_some() {
            return Err("duplicate Gitlink comparison path".into());
        }
    }
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    frame(
        &mut hash,
        b"repository-root",
        repository_root_token.as_bytes(),
    );
    frame(&mut hash, b"package-prefix", package_prefix.as_bytes());
    frame(&mut hash, b"index-sha256", index_sha256.as_bytes());
    frame(
        &mut hash,
        b"link-count",
        &(links.len() as u64).to_be_bytes(),
    );
    for link in links {
        if !normalized(&link.path)
            || !oid(&link.indexed_oid, object_len)
            || link.checkout_head_oid != link.indexed_oid
        {
            return Err("indexed Gitlink path/OID/HEAD malformed".into());
        }
        let Some(SubmoduleInspection::ExactCandidate {
            path,
            indexed_commit_oid,
            index_sha256: proof_index_sha,
            index_gitlink_count,
            index_repository_root_token,
            index_package_prefix,
            commit_tree_oid,
            commit_sha256,
            vault_subtree_sha256,
            files,
            directories,
            total_raw_bytes,
            omitted_gitmodules,
            ..
        }) = by_path.remove(link.path.as_str())
        else {
            return Err("indexed Gitlink lacks exact comparison".into());
        };
        if path != &link.path
            || indexed_commit_oid != &link.indexed_oid
            || proof_index_sha != index_sha256
            || *index_gitlink_count != links.len()
            || index_repository_root_token != repository_root_token
            || index_package_prefix != package_prefix
            || !oid(commit_tree_oid, object_len)
            || !digest(commit_sha256)
            || !digest(vault_subtree_sha256)
        {
            return Err("Gitlink raw comparison differs from complete selected index".into());
        }
        frame(&mut hash, b"path", path.as_bytes());
        frame(&mut hash, b"indexed-oid", indexed_commit_oid.as_bytes());
        frame(
            &mut hash,
            b"checkout-head",
            link.checkout_head_oid.as_bytes(),
        );
        frame(&mut hash, b"commit-tree", commit_tree_oid.as_bytes());
        frame(&mut hash, b"commit-sha256", commit_sha256.as_bytes());
        frame(&mut hash, b"vault-subtree", vault_subtree_sha256.as_bytes());
        frame(&mut hash, b"files", &(*files as u64).to_be_bytes());
        frame(
            &mut hash,
            b"directories",
            &(*directories as u64).to_be_bytes(),
        );
        frame(&mut hash, b"raw-bytes", &total_raw_bytes.to_be_bytes());
        if omitted_gitmodules
            .windows(2)
            .any(|pair| pair[0].path >= pair[1].path)
        {
            return Err("omitted Git metadata rows are not strictly ordered".into());
        }
        frame(
            &mut hash,
            b"metadata-count",
            &(omitted_gitmodules.len() as u64).to_be_bytes(),
        );
        for row in omitted_gitmodules {
            if !oid(&row.git_oid, object_len)
                || !digest(&row.sha256)
                || !normalized(&row.path)
                || row.path.split('/').next_back() != Some(".gitmodules")
            {
                return Err("omitted Git metadata digest malformed".into());
            }
            frame(&mut hash, b"metadata-path", row.path.as_bytes());
            frame(&mut hash, b"metadata-oid", row.git_oid.as_bytes());
            frame(&mut hash, b"metadata-sha256", row.sha256.as_bytes());
            frame(&mut hash, b"metadata-size", &row.size_bytes.to_be_bytes());
        }
    }
    if !by_path.is_empty() {
        return Err("extra Gitlink comparison outside complete index".into());
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

pub(super) fn proof_from_pending(row: &GitlinkPending) -> SubmoduleInspection {
    SubmoduleInspection::ExactCandidate {
        path: row.path.clone(),
        indexed_commit_oid: row.indexed_commit_oid.clone(),
        index_sha256: row.index_sha256.clone(),
        index_gitlink_count: row.index_gitlink_count,
        index_repository_root_token: row.index_repository_root_token.clone(),
        index_package_prefix: row.index_package_prefix.clone(),
        commit_tree_oid: row.commit_tree_oid.clone(),
        commit_sha256: row.commit_sha256.clone(),
        git_dir_root_token: row.git_dir_root_token.clone(),
        vault_subtree_sha256: row.vault_subtree_sha256.clone(),
        files: row.files,
        directories: row.directories,
        total_raw_bytes: row.total_raw_bytes,
        omitted_gitmodules: row
            .omitted_gitmodules
            .iter()
            .map(|meta| MetadataBlob {
                path: meta.path.clone(),
                git_oid: meta.git_oid.clone(),
                sha256: meta.sha256.clone(),
                size_bytes: meta.size_bytes,
            })
            .collect(),
    }
}

pub(super) fn index_links(rows: &[PopulatedIndexLink]) -> Vec<GitlinkCandidate> {
    rows.iter()
        .map(|row| GitlinkCandidate {
            path: row.path.clone(),
            indexed_oid: row.indexed_oid.clone(),
            checkout_head_oid: row.checkout_head_oid.clone(),
        })
        .collect()
}
