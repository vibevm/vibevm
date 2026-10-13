import pathlib,hashlib,json,sys
r=pathlib.Path('/fast/git/v/vibevm-next');roots=['crates/vibe-wire/src/generated','crates/progress-core/src/generated','vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/generated']
f={str(p.relative_to(r)):hashlib.sha256(p.read_bytes()).hexdigest() for name in roots for p in (r/name).rglob('*') if p.is_file()}
p=r/'vibevm/vibepacks/org.vibevm.doc/web/v1.1.0/site/src/generated/doc-manifest.ts';f[str(p.relative_to(r))]=hashlib.sha256(p.read_bytes()).hexdigest();pathlib.Path(sys.argv[1]).write_text(json.dumps(f,indent=2,sort_keys=True)+'\n');print(len(f))
