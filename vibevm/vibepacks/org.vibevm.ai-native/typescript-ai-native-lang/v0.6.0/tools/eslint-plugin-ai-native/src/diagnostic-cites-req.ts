/**
 * diagnostic-cites-req.ts — the third structural-diagnostics channel
 * Scaffold F promises for TypeScript (card scaffold-f-structured-
 * diagnostics, Band 3). A diagnostic the project raises by its own hand
 * must speak the Class-F grammar —
 *
 *     violates REQ <uri>: <why>; fix surface: <where>
 *
 * — never bare free text, because tool output is the agent's percept and
 * free text is wasted conditioning (R3-011).
 *
 * WHAT IT DETECTS. A string-literal message handed to a freshly
 * constructed error — the first argument of a `new <ErrorClass>(...)`
 * whose callee's final identifier ends in `Error` or `Exception`
 * (covers `new Error("…")`, `new TypeError("…")`, `new PlanError("…")`,
 * `new errors.Boom("…")`) — and the `message:` string of an object
 * literal thrown directly (`throw { message: "…" }`). Each such literal
 * is checked against the Class-F acceptor; a miss is a finding.
 *
 * HONEST LIMITS, recorded not claimed (the `ts-seam-error-cites-req`
 * precedent — never a silent claim):
 *  - **String literals only.** A message built by concatenation
 *    (`"a" + b`), an interpolated template literal (`` `a ${b}` ``), a
 *    variable, or a constant imported from elsewhere is NOT seen — there
 *    is no value tracking. Such messages pass silently, correctly or
 *    not. (A zero-interpolation template literal is also not matched.)
 *  - **Name-shape, not lineage.** "Error subclass" is recognised by the
 *    callee's name ending `Error`/`Exception`, not by walking the class
 *    hierarchy. A class extending `Error` but named otherwise (e.g.
 *    `class Diagnostics extends Error`) is MISSED; a class named
 *    `FooError` that does NOT extend `Error` is matched. The grammar
 *    burden is identical either way, so the asymmetry is tolerable and
 *    recorded here.
 *  - **Object-literal message, only when thrown directly.** A `message:`
 *    field on an object literal that is returned, nested, or assigned is
 *    not resolved — deciding it is an error would need type tracking this
 *    purely syntactic rule does not do.
 *  - **Unseen shapes.** `Error.captureStackTrace`, `AggregateError`'s
 *    second argument, `throw <non-string>`, and custom assertion helpers
 *    that wrap a message are all outside this syntactic net.
 *
 * It is a syntactic, single-file detector: it does not judge
 * reachability, test-vs-prod, or whether the literal is the project's
 * true diagnostic. File scoping is the eslint config's job.
 *
 * The rule's OWN message is rendered by the one grammar helper
 * {@link reqMessage} (the anti-drift point — the grammar is spelled
 * once, in `req-message.ts`, never reproduced here).
 */

import { ESLintUtils, type TSESTree } from "@typescript-eslint/utils";
import { matchesReqGrammar, reqMessage } from "./req-message.ts";

/**
 * The REQ this rule enforces, cited in its own message — the TypeScript
 * twin of the URI the Rust conform rules cite
 * (`discipline://rust-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops`).
 */
const REQ_URI =
  "discipline://typescript-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops";

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `discipline://typescript-ai-native-lang/rules/${name}`,
);

/** True when `name` looks like an Error-class callee — its final
 *  identifier component ends in `Error` or `Exception`. */
function isErrorCalleeName(name: string): boolean {
  return /(?:Error|Exception)$/.test(name);
}

/** The final identifier of a callee — `Error` for `new Error()`,
 *  `Boom` for `new errors.Boom()`; null when it is not a simple name. */
function calleeName(
  callee: TSESTree.LeftHandSideExpression,
): string | null {
  if (callee.type === "Identifier") {
    return callee.name;
  }
  if (callee.type === "MemberExpression") {
    const property = callee.property;
    return property.type === "Identifier" ? property.name : null;
  }
  return null;
}

/** The literal string handed to a `new <ErrorClass>("…")`, if `node` is
 *  exactly that; otherwise null. */
function errorMessageLiteral(node: TSESTree.Node): TSESTree.Literal | null {
  if (node.type !== "NewExpression") {
    return null;
  }
  const name = calleeName(node.callee);
  if (name === null || !isErrorCalleeName(name)) {
    return null;
  }
  const first = node.arguments[0];
  if (first === undefined) {
    return null;
  }
  if (first.type === "Literal" && typeof first.value === "string") {
    return first;
  }
  return null;
}

/** The literal string of a thrown `{ message: "…" }`, if `node` is
 *  exactly that; otherwise null. */
function thrownObjectMessageLiteral(
  node: TSESTree.ThrowStatement,
): TSESTree.Literal | null {
  const argument = node.argument;
  if (argument === null || argument.type !== "ObjectExpression") {
    return null;
  }
  for (const property of argument.properties) {
    if (
      property.type === "Property" &&
      !property.computed &&
      property.key.type === "Identifier" &&
      property.key.name === "message" &&
      property.value.type === "Literal" &&
      typeof property.value.value === "string"
    ) {
      return property.value;
    }
  }
  return null;
}

/** A one-line, length-capped snippet of an offending literal, for the
 *  rule's own `why` field. */
function snippet(value: string): string {
  const oneLine = value.replace(/\s+/g, " ").trim();
  return oneLine.length > 60 ? `${oneLine.slice(0, 57)}...` : oneLine;
}

export const diagnosticCitesReq = createRule({
  name: "diagnostic-cites-req",
  meta: {
    type: "problem",
    docs: {
      description:
        "A project-raised diagnostic must cite the violated spec:// REQ and a one-line fix surface (Class-F grammar), never bare free text.",
      // The rule's docs URL is the REQ it enforces — the same card the
      // engine's Class-F rules point at, in the TS discipline namespace.
      url: "discipline://typescript-ai-native-lang/cards/scaffold-f-structured-diagnostics",
    },
    schema: [],
    // No static messageId: the emitted text is built by reqMessage (the
    // single grammar helper). Reproducing the grammar here as a
    // messageId template would be the second spelling this channel
    // exists to prevent. ESLint permits reporting with a literal
    // `message`, which we route through reqMessage.
    messages: {},
  },
  defaultOptions: [],
  create(context) {
    /** Report `literal` as a free-text diagnostic, unless it already
     *  speaks the Class-F grammar. */
    function check(context, what, literal) {
      if (literal === null || matchesReqGrammar(literal.value)) {
        return;
      }
      context.report({
        node: literal,
        message: reqMessage(
          REQ_URI,
          `${what} is free text: ${snippet(literal.value)}`,
          "render it with reqMessage(<REQ URI>, why, fix) so it cites the violated REQ and a one-line fix surface",
        ),
      });
    }

    return {
      NewExpression(node) {
        check(context, "custom diagnostic", errorMessageLiteral(node));
      },
      ThrowStatement(node) {
        check(
          context,
          "thrown-object diagnostic",
          thrownObjectMessageLiteral(node),
        );
      },
    };
  },
});
