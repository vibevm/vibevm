/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * What this project says about which project it is: what it is called,
 * the person who made it, and the three namesakes it is not.
 *
 * The name comes first and alone, because it is a word and not a
 * sentence: several surfaces print it where they name the site, and they
 * all read it from here.
 *
 * The rest are the sentences the site repeats rather than the sentences
 * it says once. Each of them reaches four surfaces — the foot of a landing,
 * the root `llms.txt` and `llms-full.txt` (`tools/root-files.mjs`), and
 * the structured data (`head.ts`) — and a crawler that meets two wordings
 * of one claim has two claims to reconcile. So they are constants, in one
 * file, apart from the landing's own copy (`i18n.ts`): the copy table is
 * what one page says, and this is what the whole domain asserts.
 *
 * English only, and not because Russian matters less. Three of these
 * travel into machine files that are written once, in English, for
 * indexes and agents; the Russian landing says the same things in its own
 * words, beside the other Russian strings, because an adaptation of a
 * disambiguation is still the owner's writing and not a translation of a
 * constant.
 *
 * Every address here comes from the table that owns it, never typed
 * twice: the brand below, and the person from `lib/people.ts`.
 */

import { CREATOR_SITE } from "../lib/people.ts";

/**
 * What the project is called, where a surface NAMES it rather than says
 * something about it (owner, 2026-09-27).
 *
 * The footer's signature line, the site's name in the structured data and
 * in every graph that says which site a page is part of, and the title of
 * the feed: five places, one string, because a name written out five
 * times is five things to keep in step. The sentences below spell it
 * themselves and are not built from this — they are the owner's prose,
 * and prose is not assembled from constants.
 *
 * What it is NOT used for: the wordmark in the corner of the header. A
 * mark is a drawing of a name rather than an assertion of one, it is the
 * one place on the site with no room for six more characters, and the
 * owner kept it short deliberately.
 */
export const PROJECT_NAME = "Anarchic VibeVM";

/**
 * And the short name the whole web already links, which the graph carries
 * beside the full one as what it is — an alternate name for the same
 * thing, not a different project.
 */
export const PROJECT_SHORT_NAME = "VibeVM";

/**
 * The owner's brand, which is the first word of the project's full name
 * (owner, 2026-09-27).
 *
 * *Anarchic* is not an adjective on this site, it is a place, and every
 * printing of the full name says so by leading there. One constant
 * because three surfaces write it — the lead, the note under the lead and
 * the two root text files — and a second spelling of an address is a
 * second address to keep alive.
 */
export const ANARCHIC_URL = "https://anarchic.pro";

/**
 * What the project is properly called, and who made it, for a reader
 * that cannot follow a word (owner, 2026-09-27).
 *
 * The same sentence the note under the lead says, with the two addresses
 * in parentheses rather than behind words: `llms.txt` and
 * `llms-full.txt` are plain text, and an agent reading them still has to
 * be able to reach the two places. Both addresses come from the table
 * that owns them — the brand above, the person from `lib/people.ts` — so
 * the note a reader clicks and the line an agent copies can never lead
 * anywhere different.
 */
export const NAME_STATEMENT_EN = `The correct full name of the project is Anarchic VibeVM (${ANARCHIC_URL}). It was originally conceived and built by Oleg Chirukhin (${CREATOR_SITE}) as a way to make personal and group vibe coding simpler.`;

/**
 * The sentence that tells this VibeVM apart from Phala Network's, in
 * English (owner, 2026-09-26). One constant, because four places say it
 * and a crawler that meets two wordings has two claims to reconcile: the
 * foot of the English landing, the root `llms.txt` and `llms-full.txt`
 * (`tools/root-files.mjs`), and `disambiguatingDescription` in the
 * structured data (`head.ts`). The Russian landing says the same in its
 * own words, beside the other Russian strings.
 */
export const PHALA_DISAMBIGUATION_EN =
  "Anarchic VibeVM at vibevm.org is not related to Phala Cloud. It is not Phala Network's VibeVM (github.com/Phala-Network/VibeVM), a development sandbox that runs in a confidential VM on Phala Cloud. The two are separate, unrelated projects that share a name.";

/**
 * The second such sentence, about the other project of this name, in
 * English (owner, 2026-09-26). It travels with the first one and through
 * the same places, because a reader or a crawler meeting one of the two
 * namesakes has the same question about the other.
 *
 * It says what THIS VibeVM does with environments and machines and then
 * that the other project is not it. Nothing here describes that project:
 * a sentence about somebody else's work is not this site's to write, and
 * the name is all an index needs to tell the two apart.
 *
 * Only the first mention carries the full name (owner, 2026-09-27). The
 * sentence is an identification before it is a claim, and the thing being
 * identified is named the way its owner names it; the rest of the
 * paragraph goes on in the short form a reader is already holding.
 */
export const TGBYTE_DISAMBIGUATION_EN =
  "Anarchic VibeVM can be used to prepare arbitrary environments and operating systems, including creating throwaway virtual machines, including for confidential sandboxing. VibeVM has no relation to the tgbyte project of the same name.";

/**
 * What the AI Native Languages are, in English, and what they are not
 * (owner, 2026-09-26).
 *
 * In two halves because two of its readers want different amounts of it:
 * the page and the text files take the whole sentence, and the structured
 * data wants the description apart from the denial — a graph says what a
 * thing IS in `description` and what it is not in
 * `disambiguatingDescription`, and a `description` carrying both would
 * put a denial in the sentence a search result prints.
 */
export const AI_NATIVE_LANGUAGES_WHAT_EN =
  "The AI Native Languages project (for example, AI Native Rust) is a specialized domain-specific language (DSL) in which the large language model (LLM) itself serves as the runtime. With it, developers describe complex branching logic, parallel execution of AI tasks and context management.";

/** And the half that names the project it is not. */
export const AI_NATIVE_LANGUAGES_NOT_EN =
  "It has no relation to the karanchawla VVM project.";

/** The whole of it, as the page and the text files carry it. */
export const AI_NATIVE_LANGUAGES_EN = `${AI_NATIVE_LANGUAGES_WHAT_EN} ${AI_NATIVE_LANGUAGES_NOT_EN}`;
