import Lean

open Lean

-- This runner-owned program inspects compiled declarations, not text printed
-- by a target theorem's source. Proof inputs are trusted local code, not a
-- sandboxed proof marketplace. Kernel replay is a separate required step.
unsafe def main (args : List String) : IO Unit := do
  let [moduleName, theoremName, output] := args
    | throw <| IO.userError "expected module theorem output"
  initSearchPath (← findSysroot)
  -- Inspection trusts the already-built environment. The Rust runner separately
  -- requires kernel replay of every local module before producing a witness.
  let env ← importModules #[{module := moduleName.toName}] {} 1
  let some (.thmInfo info) := env.find? theoremName.toName
    | throw <| IO.userError "selected declaration is missing or is not a theorem"
  let some moduleIndex := env.getModuleIdxFor? info.name
    | throw <| IO.userError "selected theorem has no imported module identity"
  unless env.header.moduleNames[moduleIndex.toNat]? == some moduleName.toName do
    throw <| IO.userError "selected theorem belongs to a different module"
  let axioms ← (collectAxioms info.name : CoreM (Array Name)).toIO'
    {fileName := "<lean-ai-native>", fileMap := default} {env := env}
  let result := Json.mkObj [
    ("theorem", toJson info.name.toString),
    ("statement", toJson (reprStr info.type)),
    ("axioms", toJson ((axioms.map Name.toString).qsort (· < ·)))]
  IO.FS.writeFile output result.compress
