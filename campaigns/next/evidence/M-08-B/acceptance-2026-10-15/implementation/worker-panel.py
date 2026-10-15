import subprocess,pathlib,json
r=pathlib.Path(__file__).parent
commands=[]
commands.append(('worker-lifecycle-final',['cargo','test','--offline','--locked','-p','vibe-lifecycle','--lib','mechanism::deploy::m_08_b_1_cases','--','--nocapture']))
for i,name in enumerate(json.loads((r/'worker-check-bindings.json').read_text())['exact_regressions']):
 commands.append((f'worker-preserve-{i}',['cargo','test','--offline','--locked','-p','vibe-lifecycle','--lib','mechanism::deploy::plugin::lifecycle_tests::'+name,'--','--exact']))
commands += [('worker-route',['cargo','test','--offline','--locked','-p','vibe-orchestrator','--lib','machine_operation::adapter_plan::tests']),('worker-mcp',['cargo','test','--offline','--locked','-p','vibe-mcp','--lib','tools::adapter_plan::tests']),('worker-cli-build',['cargo','build','--offline','--locked','-p','vibe-cli','--bin','vibe']),('worker-cli',['cargo','test','--offline','--locked','-p','vibe-cli','--bin','vibe','commands::adapter_plan::tests']),('worker-wire',['cargo','test','--offline','--locked','-p','vibe-wire','--lib','behaviour::adapter_plan::tests'])]
(r/'worker-panel-plan.json').write_text(json.dumps(commands,indent=2))
results=[]
for label,argv in commands:
 code=subprocess.run(['python3',str(r/'worker-run.py'),label,*argv]).returncode
 results.append({'label':label,'exit':code})
 (r/'worker-panel-results.json').write_text(json.dumps(results,indent=2))
 # consumer compile errors need repair before proceeding to build dependent consumers
 if code and label in ['worker-route','worker-mcp','worker-cli-build']: break
raise SystemExit(any(x['exit'] for x in results))
