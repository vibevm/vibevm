"""Ordinary scratch Supersedes fixture; no product proof or shipped engine."""
import hashlib,json,os,pathlib,subprocess
OUT=pathlib.Path('/tmp/vibevm-next-supersedes-proof')
ROOT=OUT/'repo';ROOT.mkdir(exist_ok=False)
RELEASE=ROOT/'release';RELEASE.mkdir()
BINARY=pathlib.Path('/home/olegchir/git/v/vibevm-next/target/debug/preview-transition-check')
ENV={'PATH':os.environ['PATH'],'LC_ALL':'C','LANG':'C','GIT_NO_LAZY_FETCH':'1','GIT_NO_REPLACE_OBJECTS':'1','GIT_OPTIONAL_LOCKS':'0','GIT_TERMINAL_PROMPT':'0','GIT_CONFIG_NOSYSTEM':'1','GIT_CONFIG_GLOBAL':'/dev/null','GIT_CONFIG_SYSTEM':'/dev/null','GIT_GRAFT_FILE':'/dev/null','GIT_AUTHOR_DATE':'2026-09-29T00:00:00Z','GIT_COMMITTER_DATE':'2026-09-29T00:00:00Z'}
def sha(data):return hashlib.sha256(data).hexdigest()
def git(*args):
 p=subprocess.run(['git','--no-replace-objects','--literal-pathspecs','-c','protocol.allow=never','-C',str(ROOT),*args],env=ENV,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 if p.returncode:raise RuntimeError('scratch Git fixture construction failed')
 return p.stdout.decode('utf-8').removesuffix('\n')
def write(path,value):
 b=(json.dumps(value,sort_keys=True,indent=2)+'\n').encode();path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b);return sha(b)
def commit(message):
 git('add','cli.rs','pkg/vibe.toml','vibe.toml')
 git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','-c','commit.gpgsign=false','commit','--quiet','-m',message)
 return git('rev-parse','HEAD')
git('init','--quiet')
(ROOT/'pkg').mkdir();(ROOT/'pkg/vibe.toml').write_bytes(b"name = 'example'\n")
(ROOT/'vibe.toml').write_text('[project]\nname="supersedes-fixture"\ngroup="org.example"\nversion="1.0.0"\nspec_format="xml"\n')
raw=b'{"ordinary_fixture":true,"product_proof":false}\n';(RELEASE/'raw.json').write_bytes(raw)
def snapshot(identity):
 code=(ROOT/'cli.rs').read_bytes();commit_sha=git('rev-parse','HEAD');tree=git('rev-parse','HEAD^{tree}')
 capture={'schema':1,'capture_id':identity,'repository_commit_sha1':commit_sha,'repository_tree_sha1':tree,'source_record':'raw.json','source_record_sha256':sha(raw),'scope':'tracked-commit','packages':[{'coordinate':'example@1.0.0','kind':'flow','slot':'pkg','git_tree_sha1':git('rev-parse','HEAD:pkg'),'manifest_sha256':sha((ROOT/'pkg/vibe.toml').read_bytes())}],'files':[{'path':'cli.rs','sha256':sha(code),'git_blob_sha1':git('rev-parse','HEAD:cli.rs')}],'surfaces':[{'kind':'cli','source':'cli.rs','source_sha256':sha(code),'evidence_level':'source-declared','identifiers':['FixtureDeclaration']}],'excluded_from_product_delta':[],'toolchains':[],'artifact_state':'unavailable','documentation_input_state':'pending-owner-binding'}
 digest=write(RELEASE/(identity+'.json'),capture)
 return {'capture_id':identity,'source':identity+'.json','capture_file_sha256':digest,'commit_sha1':commit_sha,'git_tree_sha1':tree}
(ROOT/'cli.rs').write_bytes(b'enum FixtureDeclaration { Baseline }\n');baseline=commit('ordinary fixture baseline');before=snapshot('before')
(ROOT/'cli.rs').write_bytes(b'enum FixtureDeclaration { Original }\n');original=commit('ordinary original contribution');middle=snapshot('middle')
(ROOT/'cli.rs').write_bytes(b'enum FixtureDeclaration { Replacement }\n');replacement=commit('ordinary superseding contribution');after=snapshot('after')
proof=b'ordinary fixture supplied passing marker; not a real product result\n';(ROOT/'proof').mkdir();(ROOT/'proof/fixture.txt').write_bytes(proof)
def record(identity,subject,left,right,relations):
 return {'schema':1,'change_id':identity,'before_capture_id':left,'after_capture_id':right,'implementation_subjects':[{'kind':'commit','identity':subject}],'affected_paths':['cli.rs'],'effects':[],'user_impact':{'state':'pending','reason':'Toy fixture; central meaning intentionally unproved.','migration_action':'none'},'documentation_impact':{'state':'pending','reason':'Toy fixture; documentation meaning intentionally unproved.','documentation_action':'none'},'evidence':[{'claim_id':'ordinary-fixture-marker','subject_id':subject,'source':'proof/fixture.txt','verdict':'passed','sha256':sha(proof)}],'relations':relations,'acceptance':{'state':'candidate','review_subject':'','review_evidence':''}}
records=[record('fixture-original',original,'before','middle',[]),record('fixture-replacement',replacement,'middle','after',[{'kind':'supersedes','change_id':'fixture-original'}])]
bindings=[]
for r in records:
 digest=write(ROOT/'vibevm/vibespecs/changes'/r['change_id']/'release-impact.json',r)
 bindings.append({'change_id':r['change_id'],'sha256':digest,'required_claim_ids':['ordinary-fixture-marker']})
transition={'schema':1,'record_kind':'preview-transition','schema_status':'ordinary supersedes fixture; no central acceptance','dp1':before,'dp2':{'state':'candidate',**after},'captures':[middle],'change_records':[r['change_id'] for r in records],'contribution_scope':{'before_capture_id':'before','after_capture_id':'after','record_digests':bindings},'documentation':{'state':'pending-owner-binding'},'migration_guide':{'state':'pending'},'proof':{'capture_validated':False,'contributions_verified':False,'documentation_verified':False,'final_product_panel_passed':False,'migration_rehearsed':False}}
# DP2 uses the same reference fields plus state; that is the existing generated shape.
transition_path=RELEASE/'transition.json'
results=[]
for name,selected in [('selected-supersedes',bindings),('missing-related-selection',bindings[1:])]:
 transition['change_records']=[r['change_id'] for r in selected];transition['contribution_scope']['record_digests']=selected
 transition_sha=write(transition_path,transition);write(OUT/(name+'-transition.json'),transition)
 request={'schema':1,'transition':str(transition_path),'mode':'contribution','format':'json'};write(OUT/(name+'-request.json'),request)
 p=subprocess.run([str(BINARY)],input=json.dumps(request).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=ENV,check=False)
 (OUT/(name+'-report.json')).write_bytes(p.stdout);(OUT/(name+'-stderr.txt')).write_bytes(p.stderr)
 report=json.loads(p.stdout);codes={f['code'] for f in report['findings']};claims=report['checked_claims']
 if name=='selected-supersedes':
  assert p.returncode in [0,2] and report['verdict'] in ['passed','pending'],report
  assert 'selected-local-git-coverage' in claims,report
  assert 'record-bytes:fixture-original' in claims and 'record-bytes:fixture-replacement' in claims,report
  assert 'git-commit-paths:'+original in claims and 'git-commit-paths:'+replacement in claims,report
  assert all(c in codes for c in ['semantic-review-later','native-linkage-later','interface-linkage-later']),report
  assert all(f['state']=='pending' for f in report['findings'] if f['code'] in ['semantic-review-later','native-linkage-later','interface-linkage-later']),report
 else:
  assert p.returncode==1 and report['verdict']=='failed' and 'record-relation' in codes,report
 results.append({'case':name,'exit_code':p.returncode,'verdict':report['verdict'],'finding_codes':sorted(codes),'checked_claim_count':len(claims),'transition_sha256':transition_sha,'report_sha256':sha(p.stdout)})
h=hashlib.sha256()
with BINARY.open('rb') as f:
 for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
summary={'ordinary_fixture_only':True,'product_acceptance':False,'binary':str(BINARY),'binary_sha256':h.hexdigest(),'scratch_repository':str(ROOT),'commits':{'baseline':baseline,'original':original,'replacement':replacement},'relation':records[1]['relations'][0],'fixture_evidence_is_mock_data':True,'results':results,'fixture_files':[{'path':str(p.relative_to(OUT)),'sha256':sha(p.read_bytes())} for p in sorted(OUT.rglob('*')) if p.is_file() and '.git' not in p.parts]}
write(OUT/'summary.json',summary)
print(json.dumps({k:summary[k] for k in ['binary_sha256','commits','relation','results']},indent=2))
