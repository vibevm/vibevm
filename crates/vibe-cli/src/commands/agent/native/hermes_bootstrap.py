"""Invocation-owned Hermes carrier; Hermes retains prompt and session assembly.

This module alone does not enable launch. Its supervisor must admit an exact
native authority session before constructing the carrier, and detach on exit.
"""
import types


class CarrierAdmissionError(RuntimeError):
    """The invocation must not continue with an unadmitted prompt."""


class HermesCarrier:
    """Attach only to one owned runner and its exact authority session."""

    def __init__(self, runner, session_id, prefix):
        if not isinstance(session_id, str) or not session_id:
            raise CarrierAdmissionError("Hermes requires an exact native session")
        if not isinstance(prefix, str) or not prefix:
            raise CarrierAdmissionError("Hermes requires an admitted nonempty prefix")
        if "_run_agent_build_turn_context" in runner.__dict__:
            raise CarrierAdmissionError("Hermes runner already has an instance owner")
        live = runner.session_authority.sessions.get(session_id)
        if live is None:
            raise CarrierAdmissionError("Hermes session is absent from native authority")
        self.runner = runner
        self.session_id = session_id
        self.source = live.source
        self.route = live.route
        self.prefix = prefix
        self.agents = {}
        self.original = runner._run_agent_build_turn_context
        self.wrapper = types.MethodType(self._build_turn, runner)
        runner.__dict__["_run_agent_build_turn_context"] = self.wrapper

    def _admit(self):
        if self.runner.__dict__.get("_run_agent_build_turn_context") is not self.wrapper:
            raise CarrierAdmissionError("Hermes runner ownership changed")
        live = self.runner.session_authority.sessions.get(self.session_id)
        if live is None or live.source is not self.source or live.route != self.route:
            raise CarrierAdmissionError("Hermes native session authority changed")

    def _build_turn(self, runner, display, factory, **params):
        self._admit()
        if runner is not self.runner:
            raise CarrierAdmissionError("Hermes runner instance changed")
        session_id = params.get("session_id")
        key = params.get("session_key")
        with runner._agent_cache_lock:
            cached = runner._agent_cache.get(key)
            if cached and id(cached[0]) in self.agents and session_id != self.session_id:
                raise CarrierAdmissionError("Hermes cached session binding changed; re-admit before use")
        if session_id != self.session_id:
            return self.original(display, factory, **params)
        if params.get("source") is not self.source or key != self.route:
            raise CarrierAdmissionError("Hermes turn source or route changed")

        def create_owned(**kwargs):
            self._admit()
            if kwargs.get("session_id") != self.session_id:
                raise CarrierAdmissionError("Hermes constructor session changed")
            agent = factory(**kwargs)
            if "_build_system_prompt" in agent.__dict__:
                raise CarrierAdmissionError("Hermes agent already has a prompt owner")
            original = agent._build_system_prompt

            def assemble(actual, system_message=None):
                self._admit()
                if actual is not agent:
                    raise CarrierAdmissionError("Hermes agent instance changed")
                if system_message is not None and not isinstance(system_message, str):
                    raise CarrierAdmissionError("Hermes personal prompt tail must be text")
                return original(self.prefix + ("\n\n" + system_message if system_message else ""))

            wrapper = types.MethodType(assemble, agent)
            agent.__dict__["_build_system_prompt"] = wrapper
            self.agents[id(agent)] = (agent, wrapper)
            return agent

        return self.original(display, create_owned, **params)

    def detach(self):
        # Validate every owned shadow before restoring anything; foreign owners win.
        if self.runner.__dict__.get("_run_agent_build_turn_context") is not self.wrapper:
            raise CarrierAdmissionError("Hermes runner ownership changed; refusing teardown")
        for agent, wrapper in self.agents.values():
            if agent.__dict__.get("_build_system_prompt") is not wrapper:
                raise CarrierAdmissionError("Hermes prompt ownership changed; refusing teardown")
        for agent, _ in self.agents.values():
            del agent.__dict__["_build_system_prompt"]
        del self.runner.__dict__["_run_agent_build_turn_context"]
        self.agents.clear()
