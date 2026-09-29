/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The language-independent artifacts `/why/ai-native` prints: tool
 * names, commands, and four pieces of real code.
 *
 * They are shared verbatim between the two editions of the page, which
 * is why they are not in the string table beside the prose: a command is
 * typed rather than read, and a code sample that differed between two
 * translations of the same page would be two different claims about the
 * same repository.
 *
 * The two error-enum samples are modelled as LINES rather than as a
 * string of markup. The source built them by escaping each line and
 * wrapping some of them in a `<span>` before writing the result in with
 * `set:html`; here the same distinction is a flag on a line, the page
 * renders it as an element, and no markup is written into the document
 * from a string. The escaping disappears with it — a line is text, and
 * text in JSX is text.
 */

/** One printed line of a code sample, and whether the page marks it. */
export type CodeLine = {
  readonly text: string;
  /** Marked lines are what the discipline added; they carry the accent. */
  readonly marked?: true;
};

/** The floor of one language: the tools it already had, and the three gates. */
export type Floor = {
  readonly key: "rust" | "ts" | "go";
  readonly name: string;
  readonly yours: readonly string[];
  readonly added: readonly string[];
};

export const FLOORS: readonly Floor[] = [
  {
    key: "rust",
    name: "Rust",
    yours: ["fmt", "test", "clippy"],
    added: ["conform", "specmap", "test-gate"],
  },
  {
    key: "ts",
    name: "TypeScript",
    yours: ["prettier", "tsc", "tests", "eslint"],
    added: ["conform", "specmap", "test-gate"],
  },
  {
    key: "go",
    name: "Go",
    yours: ["gofmt", "vet", "tests", "staticcheck + exhaustive"],
    added: ["conform", "specmap", "test-gate"],
  },
];

/** The error enum as the pilot's own `vibe-core` carried it before. */
export const CODE_BEFORE: readonly CodeLine[] = [
  { text: "#[derive(Debug, Error)]" },
  { text: "pub enum Error {" },
  { text: '    #[error("invalid package reference `{input}`")]' },
  { text: "    InvalidRef { input: String }," },
  { text: "}" },
];

/** The same enum under the discipline: it names the contract it broke. */
export const CODE_AFTER: readonly CodeLine[] = [
  { text: "#[derive(Debug, Error)]" },
  { text: '#[spec(implements = "spec://…#package-identity")]', marked: true },
  { text: "pub enum Error {" },
  { text: "    #[error(" },
  { text: '        "invalid package reference `{input}`: \\' },
  { text: "         {reason} \\" },
  { text: "         (violates spec://…#pkgref; \\", marked: true },
  { text: '          fix: `[kind:][group/]name[@version]`)"', marked: true },
  { text: "    )]" },
  { text: "    InvalidRef { input: String, reason: String }," },
  { text: "}" },
];

/** The same traceability edge, spelled in each of the three languages. */
export const TAG_ROWS: readonly { mark: Floor["key"]; code: string }[] = [
  { mark: "rust", code: '#[spec(implements = "spec://…#anchor")]' },
  { mark: "ts", code: "/** @implements spec://…#anchor */" },
  { mark: "go", code: "//spec:implements spec://…#anchor r=1" },
];

/** vibevm's own ratchet baseline today, in full. */
export const BASELINE_JSON = [
  "{",
  '  "findings": [],',
  '  "schema": 1',
  "}",
].join("\n");

/** A real deviation shape from the pilot: the reason travels with the code. */
export const DEVIATION_CODE =
  '#[spec(deviates, reason = "platform ctor runs single-threaded, pre-main")]';

/** The type oracle, asked about an edit that does not exist yet. */
export const ORACLE_CMD =
  "vibe bin exec rust-ai-native-tcg -- validate src/cells/edit.rs --content-from -";

/** The prompt every command on this page is printed behind. */
export const PROMPT = "$";
