# XML embed provenance implementation decisions — candidate

Root reviewed the source design and actual second/third installed artifacts. The preexisting XML projection loses generated embed identity; a reader-only unescape cannot repair dropped comments and would reinterpret authored examples. Implement the complete existing-owner path, not a new parser, compiler or provenance store.

- Mint typed nested/repeated occurrence records during actual recursive expansion; retain the public string API as a compatibility adapter. Carry body association through existing Closure, Link and Lane owners and their digests/validators.
- Map builtin line-preserving edits explicitly; normal once coverage returns a retention map. Distinguish fully suppressed occurrences from intrinsically empty embeds. A custom transform cannot keep stale annotations or silently erase them from a provenance-bearing reply.
- Preserve ordinary semantic paragraph text exactly. Prefer canonical c1 XML comments at mapped semantic positions without introducing semantic whitespace; actual generated comments may be inside a paragraph. Only bounded generated-comment observation is extended. Escaped prose/CDATA and authored literal examples remain non-controls. Legacy raw marker framing/fence rules remain unchanged.
- Markdown generated framing and authored reserved-looking literals must remain distinct through an explicit target rendering policy. No authenticity claim is made over editable legacy artifacts.
- Independently validate emitted semantic content and ordered provenance against the pre-emission witness. Byte accounting uses the same actual rendering.
- This is real compiler-IR wire evolution. The implementer must propose exact epoch/strict-reader/transform compatibility before canonical schema or generated writes. A private-field-only change or old reply silently losing annotations is unacceptable. No epoch patch is approved merely by this decision record.

The actual install/repeat/unit-reuse/tree/reinstall oracle, both paragraph-adjacent and standalone layouts, must pass together with literal/fence/suppression/roundtrip controls. Implementation is active; no acceptance or completion is claimed. Root owns canonical spec/format notes; the existing implementation worker owns the coherent code path and main Cargo slot.
