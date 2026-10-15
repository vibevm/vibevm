import sys,subprocess,os,json,time,re,hashlib,pathlib
root=pathlib.Path('/fast/zap-managed/2026-10-15-m08b1-review-correction')
label=sys.argv[1];argv=sys.argv[2:];log=root/(label+'.log'); result=root/(label+'.result.json')
assert not log.exists(),label
os.environ['CARGO_TARGET_DIR']='/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target'
os.environ['TMPDIR']=str(root/'tmp')
os.environ['VIBE_M08_B1_TEST_BINARY']=os.environ['CARGO_TARGET_DIR']+'/debug/vibe'
start=time.time()
with log.open('w') as out: status=subprocess.run(argv,stdout=out,stderr=subprocess.STDOUT).returncode
text=log.read_text(errors='replace'); counts=[dict(zip(['passed','failed','ignored','measured','filtered'],map(int,m))) for m in re.findall(r'test result: .*?\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',text)]
record={'argv':argv,'cwd':os.getcwd(),'exit':status,'seconds':time.time()-start,'log':str(log),'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest(),'counts':counts,'nonzero_case_count':sum(c['passed']+c['failed'] for c in counts)}
result.write_text(json.dumps(record,indent=2));print(json.dumps(record));sys.exit(status)
