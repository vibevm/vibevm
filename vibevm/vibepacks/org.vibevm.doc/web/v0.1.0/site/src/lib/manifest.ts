/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER */

/**
 * The erasure boundary between the page manifest as bytes and the page
 * manifest as a type.
 *
 * The type is generated from the same JTD schema the Rust pipeline
 * writes the manifest with, so both halves of the contract move
 * together. What a generated type cannot do is make the bytes true: a
 * manifest arrives as JSON — from a fetch, from an import, from a file a
 * reader's own `vibe` produced — and the compiler has never seen it. An
 * `as DocManifest` here would be the whole contract reduced to a claim,
 * which the discipline forbids in domain code for exactly this reason.
 *
 * So this file is the one place that turns `unknown` into `DocManifest`,
 * by looking. Failure is a value with the path that failed in it, not an
 * exception and not a silent `undefined`: a shelf that renders half a
 * manifest is worse than a shelf that says which field was missing.
 */

import {
  Audience,
  DocumentationStatus,
  PageGenre,
  TranslationStatus,
  type AdaptedSource,
  type CardMedia,
  type DocManifest,
  type DocPackage,
  type DocPage,
  type DocumentedSubject,
} from "../generated/doc-manifest.ts";

/** Where the manifest stopped making sense, and why. */
export type ManifestError = {
  /** A dotted path into the document, e.g. `pages[0].genre`. */
  readonly path: string;
  readonly reason: string;
};

export type Parsed<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: ManifestError };

function fail<T>(path: string, reason: string): Parsed<T> {
  return { ok: false, error: { path, reason } };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function field(source: Record<string, unknown>, key: string): unknown {
  return Object.prototype.hasOwnProperty.call(source, key)
    ? source[key]
    : undefined;
}

function str(
  source: Record<string, unknown>,
  key: string,
  at: string,
): Parsed<string> {
  const value = field(source, key);
  return typeof value === "string"
    ? { ok: true, value }
    : fail(`${at}.${key}`, "expected a string");
}

function num(
  source: Record<string, unknown>,
  key: string,
  at: string,
): Parsed<number> {
  const value = field(source, key);
  return typeof value === "number" && Number.isFinite(value)
    ? { ok: true, value }
    : fail(`${at}.${key}`, "expected a number");
}

/** A value out of a closed vocabulary, checked against the vocabulary itself. */
function member<T extends string>(
  source: Record<string, unknown>,
  key: string,
  at: string,
  vocabulary: Readonly<Record<string, T>>,
): Parsed<T> {
  const value = field(source, key);
  const allowed = Object.values(vocabulary);
  const found = allowed.find((candidate) => candidate === value);
  return found === undefined
    ? fail(`${at}.${key}`, `expected one of ${allowed.join(", ")}`)
    : { ok: true, value: found };
}

function list(
  source: Record<string, unknown>,
  key: string,
  at: string,
): Parsed<unknown[]> {
  const value = field(source, key);
  return Array.isArray(value)
    ? { ok: true, value }
    : fail(`${at}.${key}`, "expected an array");
}

function strings(
  source: Record<string, unknown>,
  key: string,
  at: string,
): Parsed<string[]> {
  const items = list(source, key, at);
  if (!items.ok) return items;
  const out: string[] = [];
  for (const [index, item] of items.value.entries()) {
    if (typeof item !== "string")
      return fail(`${at}.${key}[${index}]`, "expected a string");
    out.push(item);
  }
  return { ok: true, value: out };
}

function audiences(
  source: Record<string, unknown>,
  at: string,
): Parsed<Audience[]> {
  const items = list(source, "audiences", at);
  if (!items.ok) return items;
  const allowed = Object.values(Audience);
  const out: Audience[] = [];
  for (const [index, item] of items.value.entries()) {
    const found = allowed.find((candidate) => candidate === item);
    if (found === undefined) {
      return fail(
        `${at}.audiences[${index}]`,
        `expected one of ${allowed.join(", ")}`,
      );
    }
    out.push(found);
  }
  return { ok: true, value: out };
}

function subject(value: unknown, at: string): Parsed<DocumentedSubject> {
  if (!isRecord(value)) return fail(at, "expected an object");
  const pkg = str(value, "package", at);
  if (!pkg.ok) return pkg;
  const version = str(value, "version", at);
  if (!version.ok) return version;
  const status = member(value, "status", at, DocumentationStatus);
  if (!status.ok) return status;
  return {
    ok: true,
    value: { package: pkg.value, version: version.value, status: status.value },
  };
}

function translation(value: unknown, at: string): Parsed<AdaptedSource> {
  if (!isRecord(value)) return fail(at, "expected an object");
  const pkg = str(value, "package", at);
  if (!pkg.ok) return pkg;
  const version = str(value, "version", at);
  if (!version.ok) return version;
  const status = member(value, "status", at, TranslationStatus);
  if (!status.ok) return status;
  return {
    ok: true,
    value: { package: pkg.value, version: version.value, status: status.value },
  };
}

/**
 * Where the card's three pictures are published.
 *
 * All three or none. A build of this pipeline writes every role — a role
 * that declares nothing gets a generated placeholder
 * (`##CARD-PLACEHOLDERS-GENERATED`) — so a `media` with two addresses in
 * it is a manifest somebody assembled by hand, and reading it as «this
 * package has no banner» would show a drawn placeholder beside two real
 * pictures and call that the card.
 *
 * The addresses are relative to the base the documentation is served
 * under, which is why nothing here joins them to anything: the site and
 * the local reader mount at different places, and the one that knows
 * where it is standing is the caller.
 */
function cardMedia(value: unknown, at: string): Parsed<CardMedia> {
  if (!isRecord(value)) return fail(at, "expected an object");
  const icon = str(value, "icon", at);
  if (!icon.ok) return icon;
  const banner = str(value, "banner", at);
  if (!banner.ok) return banner;
  const preview = str(value, "preview", at);
  if (!preview.ok) return preview;
  return {
    ok: true,
    value: { icon: icon.value, banner: banner.value, preview: preview.value },
  };
}

function page(value: unknown, at: string): Parsed<DocPage> {
  if (!isRecord(value)) return fail(at, "expected an object");
  const path = str(value, "path", at);
  if (!path.ok) return path;
  const title = str(value, "title", at);
  if (!title.ok) return title;
  const genre = member(value, "genre", at, PageGenre);
  if (!genre.ok) return genre;
  const who = audiences(value, at);
  if (!who.ok) return who;
  const anchors = strings(value, "anchors", at);
  if (!anchors.ok) return anchors;
  const summary = str(value, "summary", at);
  if (!summary.ok) return summary;
  const minutes = num(value, "reading_time_min", at);
  if (!minutes.ok) return minutes;
  const reviewed = field(value, "reviewed_at");
  if (reviewed !== undefined && typeof reviewed !== "string") {
    return fail(`${at}.reviewed_at`, "expected a string when present");
  }
  return {
    ok: true,
    value: {
      path: path.value,
      title: title.value,
      genre: genre.value,
      audiences: who.value,
      anchors: anchors.value,
      summary: summary.value,
      reading_time_min: minutes.value,
      ...(reviewed === undefined ? {} : { reviewed_at: reviewed }),
    },
  };
}

function card(value: unknown, at: string): Parsed<DocPackage> {
  if (!isRecord(value)) return fail(at, "expected an object");
  const required = [
    "group",
    "name",
    "version",
    "publisher",
    "title",
    "abstract",
    "lang",
    "rendered_at",
  ] as const;
  const read: Record<string, string> = {};
  for (const key of required) {
    const parsed = str(value, key, at);
    if (!parsed.ok) return parsed;
    read[key] = parsed.value;
  }
  const status = member(value, "status", at, DocumentationStatus);
  if (!status.ok) return status;
  const who = audiences(value, at);
  if (!who.ok) return who;
  const subjects = list(value, "subjects", at);
  if (!subjects.ok) return subjects;
  const parsedSubjects: DocumentedSubject[] = [];
  for (const [index, item] of subjects.value.entries()) {
    const parsed = subject(item, `${at}.subjects[${index}]`);
    if (!parsed.ok) return parsed;
    parsedSubjects.push(parsed.value);
  }

  const description = field(value, "description");
  if (description !== undefined && typeof description !== "string") {
    return fail(`${at}.description`, "expected a string when present");
  }
  const published = field(value, "published_at");
  if (published !== undefined && typeof published !== "string") {
    return fail(`${at}.published_at`, "expected a string when present");
  }
  const adapted = field(value, "translation");
  let parsedTranslation: AdaptedSource | undefined = undefined;
  if (adapted !== undefined) {
    const parsed = translation(adapted, `${at}.translation`);
    if (!parsed.ok) return parsed;
    parsedTranslation = parsed.value;
  }
  /* Optional because the registry rules this format permissive: a
     manifest written before the field existed is still a manifest, and a
     reader that refused one would refuse every document already
     published. Absent means «this document predates the field», never
     «this package has no picture». */
  const pictures = field(value, "media");
  let parsedMedia: CardMedia | undefined = undefined;
  if (pictures !== undefined) {
    const parsed = cardMedia(pictures, `${at}.media`);
    if (!parsed.ok) return parsed;
    parsedMedia = parsed.value;
  }

  // Read one by one rather than spread from `read`: an index signature
  // would make every field `string | undefined` again, which is the
  // exact uncertainty this function exists to remove.
  const group = read["group"];
  const name = read["name"];
  const version = read["version"];
  const publisher = read["publisher"];
  const title = read["title"];
  const summary = read["abstract"];
  const lang = read["lang"];
  const renderedAt = read["rendered_at"];
  if (
    group === undefined ||
    name === undefined ||
    version === undefined ||
    publisher === undefined ||
    title === undefined ||
    summary === undefined ||
    lang === undefined ||
    renderedAt === undefined
  ) {
    return fail(
      at,
      "a required field of the card went missing between the check and the read",
    );
  }

  return {
    ok: true,
    value: {
      group,
      name,
      version,
      publisher,
      title,
      abstract: summary,
      lang,
      status: status.value,
      subjects: parsedSubjects,
      audiences: who.value,
      rendered_at: renderedAt,
      ...(parsedMedia === undefined ? {} : { media: parsedMedia }),
      ...(description === undefined ? {} : { description }),
      ...(published === undefined ? {} : { published_at: published }),
      ...(parsedTranslation === undefined
        ? {}
        : { translation: parsedTranslation }),
    },
  };
}

/** Turn a parsed JSON document into a manifest, or say where it failed. */
export function parseDocManifest(input: unknown): Parsed<DocManifest> {
  if (!isRecord(input)) return fail("$", "expected an object");
  const schemaVersion = num(input, "schema_version", "$");
  if (!schemaVersion.ok) return schemaVersion;
  const pkg = card(field(input, "package"), "$.package");
  if (!pkg.ok) return pkg;
  const pages = list(input, "pages", "$");
  if (!pages.ok) return pages;
  const parsedPages: DocPage[] = [];
  for (const [index, item] of pages.value.entries()) {
    const parsed = page(item, `$.pages[${index}]`);
    if (!parsed.ok) return parsed;
    parsedPages.push(parsed.value);
  }
  return {
    ok: true,
    value: {
      schema_version: schemaVersion.value,
      package: pkg.value,
      pages: parsedPages,
    },
  };
}
