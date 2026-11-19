// Thin executable Pi adapter: native modules own runtime, resources and modes.
// Unsupported parser families refuse explicitly before creating a session.
import { readFileSync, realpathSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';

export async function runNativePi({packageDir, carrierPath, prefixPath, cwd, args}) {
  const meta=JSON.parse(readFileSync(join(packageDir,'package.json'),'utf8'));
  if(meta.name!=='@earendil-works/pi-coding-agent'||meta.version!=='1.5.2') throw new Error('Pi runtime is not an admitted 1.5.2 installation');
  // Native extensions use argv[1] to launch plain Pi children. Private Vibe
  // bootstrap arguments must never become the native process identity.
  process.argv=[process.execPath,join(packageDir,'dist','cli.js'),...args];
  const native=relative=>import(pathToFileURL(join(packageDir,'dist',relative)).href);
  // Native main owns these pre-parser command families; they are not chat text.
  if(['auth','config','mcp','install','remove','uninstall','update','list'].includes(args[0])){const {main}=await native('main.js');return main(args);}
  const {parseArgs}=await native('cli/args.js');
  const parsed=parseArgs(args);
  if(parsed.diagnostics.some(d=>d.type==='error')) throw new Error('Native Pi argument validation failed; run plain pi --help for its argument contract');
  if(parsed.help||parsed.version||parsed.listModels||parsed.export){const {main}=await native('main.js');return main(args);}
  // Do not silently reinterpret native flags while their complete bootstrap mapping
  // is still being integrated. Values (notably apiKey) never enter diagnostics.
  if(parsed.systemPrompt!==undefined) throw new Error('Pi --system-prompt replaces the native base and conflicts with Vibe launch delivery; use read delivery for native prompt replacement');
  const appMode=parsed.mode==='rpc'?'rpc':parsed.mode==='json'?'json':parsed.print||!process.stdin.isTTY||!process.stdout.isTTY?'print':'interactive';
  let stdinContent;
  if(appMode!=='rpc'&&!process.stdin.isTTY){
    let input='';for await(const chunk of process.stdin)input+=chunk.toString('utf8');
    stdinContent=input.trim()||undefined;
  }
  const {buildInitialMessage}=await native('cli/initial-message.js');
  const files=parsed.fileArgs.length?await (await native('cli/file-processor.js')).processFileArguments(parsed.fileArgs,{autoResizeImages:false}):{};
  const {initialMessage,initialImages}=buildInitialMessage({parsed,stdinContent,fileText:files.text,fileImages:files.images});
  const {admitPrefix,createSessionCarrier}=await import(pathToFileURL(carrierPath).href);
  const prefix=admitPrefix(prefixPath); // Before runtime/session effects or compaction.
  if(parsed.offline){process.env.PI_OFFLINE='1';process.env.PI_SKIP_VERSION_CHECK='1';}
  const {getAgentDir}=await native('config.js');
  const {ProjectTrustStore,hasTrustRequiringProjectResources}=await native('core/trust-manager.js');
  const agentDir=getAgentDir();
  const trustStore=new ProjectTrustStore(agentDir);
  const {resolveProjectTrusted}=await native('core/project-trust.js');
  const {createProjectTrustContext}=await native('cli/project-trust.js');
  const {builtInExtensions}=await native('extensions/index.js');
  const projectTrustByCwd=new Map();
  const {SettingsManager}=await native('core/settings-manager.js');
  const {createSessionManager}=await native('main.js');
  const settings=SettingsManager.create(cwd,agentDir);
  const manager=await createSessionManager(parsed,cwd,parsed.sessionDir,settings);
  const {pathsEqual}=await native('utils/paths.js');
  if(!pathsEqual(realpathSync(manager.getCwd()),realpathSync(cwd))) throw new Error('Pi selected another project cwd; relaunch Vibe there to re-admit its boot before loading native resources');
  const {createAgentSessionRuntime,createAgentSessionServices,createAgentSessionFromServices}=await native('core/agent-session-runtime.js');
  const {resolveCliModel,resolveModelScope}=await native('core/model-resolver.js');
  const carriers=new Set();
  const createRuntime=async({cwd:nextCwd,agentDir:nextAgentDir,sessionManager,sessionStartEvent,projectTrustContext})=>{
    // Runtime replacement is native; revalidate scope before resource loading.
    if(resolve(nextCwd)!==resolve(cwd)) throw new Error('Pi changed project cwd; relaunch Vibe in the selected project to re-admit its boot');
    const cachedTrust=projectTrustByCwd.get(nextCwd);
    const hasResources=hasTrustRequiringProjectResources(nextCwd);
    const needsTrust=parsed.projectTrustOverride===undefined&&cachedTrust===undefined&&hasResources;
    const projectTrusted=needsTrust?false:(cachedTrust??parsed.projectTrustOverride??trustStore.get(nextCwd)??!hasResources);
    const runtimeSettings=SettingsManager.create(nextCwd,nextAgentDir,{projectTrusted});
    const trustDiagnostics=[];
    const reloadOptions=needsTrust?{resolveProjectTrust:async({extensionsResult})=>{
      const trusted=await resolveProjectTrusted({cwd:nextCwd,trustStore,trustOverride:parsed.projectTrustOverride,defaultProjectTrust:settings.getDefaultProjectTrust(),extensionsResult,projectTrustContext:projectTrustContext??createProjectTrustContext({cwd:nextCwd,mode:appMode,settingsManager:settings,hasUI:sessionStartEvent===undefined&&appMode==='interactive'}),onExtensionError:()=>trustDiagnostics.push({type:'warning',message:'Native Pi project trust extension reported an error'})});
      projectTrustByCwd.set(nextCwd,trusted);return trusted;
    }}:undefined;
    const holder={session:null};
    const carrier=createSessionCarrier({sessionId:sessionManager.getSessionId(),prefix,getSession:()=>holder.session});
    let services;
    let created;
    try {
    services=await createAgentSessionServices({cwd:nextCwd,agentDir:nextAgentDir,settingsManager:runtimeSettings,resourceLoaderReloadOptions:reloadOptions,extensionFlagValues:parsed.unknownFlags,resourceLoaderOptions:{extensionFactories:[...builtInExtensions,carrier.extension],additionalExtensionPaths:parsed.extensions,additionalSkillPaths:parsed.skills,additionalPromptTemplatePaths:parsed.promptTemplates,additionalThemePaths:parsed.themes,noExtensions:parsed.noExtensions,disabledBuiltinExtensions:parsed.noMcp?['mcp']:undefined,noSkills:parsed.noSkills,noPromptTemplates:parsed.noPromptTemplates,noThemes:parsed.noThemes,noContextFiles:parsed.noContextFiles,appendSystemPrompt:parsed.appendSystemPrompt}});
    if(services.diagnostics.some(d=>d.type==='error')||services.resourceLoader.getExtensions().errors.length) throw new Error('Native Pi resource or extension argument admission failed');
    if(parsed.provider&&!parsed.model) throw new Error('Native Pi --provider requires --model');
    const patterns=parsed.models??services.settingsManager.getEnabledModels();
    const scopedModels=patterns?.length?await resolveModelScope(patterns,services.modelRuntime,{signal:AbortSignal.timeout(15000)}):[];
    const selected=resolveCliModel({cliProvider:parsed.provider,cliModel:parsed.model,cliThinking:parsed.thinking,inlineApiKey:parsed.apiKey!==undefined,modelRuntime:services.modelRuntime});
    if(!selected.model&&scopedModels.length&&!sessionManager.buildSessionContext().messages.length){
      const savedProvider=services.settingsManager.getDefaultProvider();
      const savedModel=services.settingsManager.getDefaultModel();
      const first=scopedModels.find(item=>item.model.provider===savedProvider&&item.model.id===savedModel)??scopedModels[0];
      selected.model=first.model;selected.thinkingLevel=first.thinkingLevel;
    }
    if(selected.error) throw new Error('Native Pi model selection failed; check the provider/model arguments');
    if(selected.warning) console.error('Native Pi model selection reported a warning; verify the selected model');
    if(parsed.apiKey){if(!selected.model)throw new Error('Native --api-key requires an admitted model');await services.modelRuntime.setRuntimeApiKey(selected.model.provider,parsed.apiKey);}
    created=await createAgentSessionFromServices({services,sessionManager,sessionStartEvent,model:selected.model,thinkingLevel:parsed.thinking??selected.thinkingLevel,scopedModels,tools:parsed.tools,excludeTools:parsed.excludeTools,noTools:parsed.noTools?'all':parsed.noBuiltinTools?'builtin':undefined});
    holder.session=created.session;carrier.attach();carriers.add(carrier);
    return {...created,services,diagnostics:[...trustDiagnostics,...services.diagnostics]};
    } catch(error) {
      carrier.detach();
      if(created?.session) await created.session.dispose();
      await services?.modelRuntime.settleCredentialWrites();
      throw error;
    }
  };
  const runtime=await createAgentSessionRuntime(createRuntime,{cwd,agentDir,sessionManager:manager});
  try{
    if(parsed.name)runtime.session.sessionManager.appendSessionInfo(parsed.name);
    if(appMode==='rpc'){const {runRpcMode}=await native('modes/rpc/rpc-mode.js');return await runRpcMode(runtime);}
    if(appMode==='print'||appMode==='json'){const {runPrintMode}=await native('modes/print-mode.js');return await runPrintMode(runtime,{mode:appMode==='json'?'json':'text',initialMessage,initialImages,messages:parsed.messages});}
    const {InteractiveMode}=await native('modes/interactive/interactive-mode.js');
    const ui=new InteractiveMode(runtime,{verbose:parsed.verbose,tuiMode:parsed.tuiMode,initialThemeSetting:parsed.useTheme,initialMessage,initialImages,initialMessages:parsed.messages});
    await ui.run();
  }finally{for(const carrier of carriers)carrier.detach();await runtime.dispose();}
}

if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  const [packageDir,carrierPath,prefixPath,cwd,...args]=process.argv.slice(2);
  try{const code=await runNativePi({packageDir,carrierPath,prefixPath,cwd,args});if(typeof code==='number')process.exitCode=code;}
  catch(error){console.error(error instanceof Error?error.message:'Vibe native Pi admission failed');process.exitCode=1;}
}
