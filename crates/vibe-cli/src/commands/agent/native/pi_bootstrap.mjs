// Agent-native Pi carrier. Rust owns invocation files and runtime admission;
// the installed Pi runtime owns sessions, UI, resources, auth and model turns.
import { readFileSync } from 'node:fs';
import { randomUUID } from 'node:crypto';

export class CarrierAdmissionError extends Error {
  constructor(message, cause) {
    super(message, { cause });
    this.name = 'CarrierAdmissionError';
  }
}

/** Admit immutable project bytes before startup or possible pre-compaction. */
export function admitPrefix(prefixPath) {
  if (typeof prefixPath !== 'string' || prefixPath.length === 0) {
    throw new CarrierAdmissionError('Vibe prefix path is missing; restart with a prepared launch');
  }
  const prefix = new TextDecoder('utf-8', { fatal: true }).decode(readFileSync(prefixPath));
  if (!prefix) throw new CarrierAdmissionError('Vibe prefix is empty; regenerate boot and restart');
  return prefix;
}

/** One native session owns this extension and its additive error supervisor. */
export function createSessionCarrier({ sessionId, prefix, getSession }) {
  if (typeof sessionId !== 'string' || !sessionId || typeof prefix !== 'string' || !prefix || typeof getSession !== 'function') {
    throw new CarrierAdmissionError('Vibe carrier requires an admitted prefix and exact native session');
  }
  // Invocation-local nonsecret identity avoids clashing with authored inline names.
  const extensionName = `vibe-native-prefix-${randomUUID()}`;
  const extensionPath = `<inline:${extensionName}>`;
  const owned = new WeakMap();
  let boundRunner;
  let unsubscribe;
  const currentSession = () => {
    const session = getSession();
    if (!session || session.sessionManager.getSessionId() !== sessionId) {
      throw new CarrierAdmissionError('Vibe native session binding changed; re-admit the session');
    }
    return session;
  };
  const detach = () => {
    unsubscribe?.();
    unsubscribe = undefined;
    boundRunner = undefined;
  };
  const attach = () => {
    const session = currentSession();
    const matches = session.resourceLoader.getExtensions().extensions.filter(extension => extension.path === extensionPath);
    if (matches.length !== 1) {
      throw new CarrierAdmissionError('Vibe native extension identity collided or was replaced; restart with an unambiguous carrier');
    }
    const runner = session.extensionRunner;
    if (runner === boundRunner) return;
    detach();
    boundRunner = runner;
    unsubscribe = runner.onError(error => {
      if (error.extensionPath === extensionPath && (error.event === 'before_agent_start' || error.event === 'session_before_compact')) {
        currentSession();
        throw new CarrierAdmissionError('Vibe carrier rejected this turn; no bootless submission is permitted', error);
      }
    });
  };
  const factory = api => {
    api.on('session_start', attach);
    api.on('session_shutdown', detach);
    // Native compaction can precede before_agent_start on restored history.
    // Admit the same exact session/extension before its summarization boundary;
    // keep Pi's own compaction prompt and summary generator unchanged.
    api.on('session_before_compact', () => { attach(); });
    const beforeStart = (event, context) => {
      if (context.sessionManager.getSessionId() !== sessionId) return;
      currentSession();
      const options = event?.systemPromptOptions;
      if (!options || typeof options !== 'object' || Array.isArray(options)) {
        throw new CarrierAdmissionError('Vibe system prompt options are unavailable');
      }
      const previous = owned.get(options);
      if (previous) {
        if (options.appendSystemPrompt !== previous.written) {
          throw new CarrierAdmissionError('Vibe-owned prompt was changed by another extension; restart the session');
        }
        return;
      }
      const tail = options.appendSystemPrompt;
      if (tail != null && typeof tail !== 'string') {
        throw new CarrierAdmissionError('Native personal prompt tail must be a string');
      }
      const original = tail ?? '';
      const written = prefix + (original ? '\n\n' + original : '');
      options.appendSystemPrompt = written;
      owned.set(options, { original, written });
    };
    api.on('before_agent_start', beforeStart);
  };
  return { extension: { name: extensionName, factory }, attach, detach, assertAdmitted: attach };
}
