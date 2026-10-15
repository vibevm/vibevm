// Operator-only: existing public projector, no scan/reindex/journal append.
use std::{collections::BTreeSet, env, error::Error, fs, path::{Path, PathBuf}};
use vibe_index::{index::{memory::WriteCtx, sha256_of_bytes, WRITER_DIRS, WRITER_FILES}, journal};
const CORPUS: &str = "/fast/git/v/vibevm-next/formats/corpora/index/e1";
const ROOT: &str = "/fast/git/v/research/2026-10-15-m08c1-index-projection";
fn files(root: &Path) -> Result<BTreeSet<PathBuf>, Box<dyn Error>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeSet<PathBuf>) -> Result<(), Box<dyn Error>> {
        for entry in fs::read_dir(at)? {
            let entry = entry?; let p = entry.path(); let ty = entry.file_type()?;
            if ty.is_symlink() { return Err(format!("link refused: {}", p.display()).into()); }
            if ty.is_dir() { walk(root, &p, out)?; }
            else if ty.is_file() { out.insert(p.strip_prefix(root)?.to_path_buf()); }
            else { return Err(format!("nonregular entry: {}",p.display()).into()); }
        }
        Ok(())
    }
    let mut out=BTreeSet::new(); walk(root,root,&mut out)?; Ok(out)
}
fn snapshot(root: &Path) -> Result<Vec<(PathBuf,String)>,Box<dyn Error>> {
    files(root)?.into_iter().map(|p| Ok((p.clone(),sha256_of_bytes(&fs::read(root.join(p))?)))).collect()
}
fn main() -> Result<(), Box<dyn Error>> {
    let args:Vec<_>=env::args_os().collect();
    if args.len()!=3 { return Err("usage: projector <exact corpus> <new scratch>".into()); }
    let corpus=Path::new(&args[1]).canonicalize()?;
    if corpus!=Path::new(CORPUS).canonicalize()? { return Err("wrong corpus".into()); }
    let scratch=PathBuf::from(&args[2]);
    if scratch!=Path::new(ROOT).join("projection") || scratch.exists() { return Err("scratch must be exact new projection path".into()); }
    if scratch.parent().ok_or("scratch parent")?.canonicalize()? != Path::new(ROOT).canonicalize()? { return Err("scratch parent mismatch".into()); }
    let before=snapshot(&corpus)?;
    let records=journal::replay(&journal::default_dir(&corpus))?;
    println!("journal_records={}",records.len());
    let index=journal::project(records)?;
    let at=index.generated_at;
    if at.to_rfc3339()!="2026-08-09T10:00:00+00:00" { return Err("unexpected journal generated_at".into()); }
    if index.generator!="vibe-index 1.1.0" { return Err(format!("wrong linked generator {}",index.generator).into()); }
    println!("generated_at={at};generator={}", index.generator);
    fs::create_dir(&scratch)?;
    index.write_to(&scratch,&WriteCtx{at})?;
    if before!=snapshot(&corpus)? { return Err("READONLY CORPUS CHANGED".into()); }
    let produced=files(&scratch)?; let mut committed=BTreeSet::new();
    for name in WRITER_FILES { if corpus.join(name).is_file() { committed.insert(PathBuf::from(name)); } }
    for name in WRITER_DIRS { for p in files(&corpus.join(name))? { committed.insert(Path::new(name).join(p)); } }
    if produced!=committed { return Err("writer file-set mismatch".into()); }
    let mut changed=Vec::new();
    for rel in &produced {
        let a=fs::read(corpus.join(rel))?;let b=fs::read(scratch.join(rel))?;
        println!("file={} old={} new={} bytes={}",rel.display(),sha256_of_bytes(&a),sha256_of_bytes(&b),b.len());
        if a!=b { changed.push(rel.clone()); }
    }
    if changed!=vec![PathBuf::from("repomd.json")] { return Err(format!("unexpected changed files {changed:?}").into()); }
    let old=fs::read_to_string(corpus.join("repomd.json"))?;
    let expected="\"generator\": \"vibe-index 1.0.0\"";
    if old.matches(expected).count()!=1 { return Err("old generator occurrence mismatch".into()); }
    // Comparison oracle only; generated scratch is never patched/normalized.
    let expected_bytes=old.replace(expected,"\"generator\": \"vibe-index 1.1.0\"").into_bytes();
    if fs::read(scratch.join("repomd.json"))?!=expected_bytes { return Err("delta is not sole generator stamp".into()); }
    println!("PASS sole generated generator stamp; writer_files={}; all input files/journals unchanged",produced.len());
    Ok(())
}
