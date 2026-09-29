/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * "The Big Vision" — the English edition.
 *
 * An adaptation of the owner's Russian essay, not a word-for-word
 * translation: the same argument, paragraph for paragraph, written the
 * way an English essay is written. The metaphors are the author's — the
 * datacenter plugged into a star, the fairy tales about OOP, the
 * assembly of the coming world — and so are the claims, including the
 * deliberately sharp ones. The disclaimer is the author's too, and it
 * ships with the essay in both languages.
 */

import type { VisionStrings } from "./strings.ts";

export const COPY: VisionStrings = {
  eyebrow: "Vision · an essay",
  title: "The Big Vision",
  heroAlt:
    "Two sources of intention: a human circle and a machine square, joined by traceable edges across a field of nodes.",
  heroCaption: "fig. 01 · intention, now from two sources",

  lede: "Under the previous technological order, all meaning came from humans — and only from humans. No surprise there: we were the only intelligent beings around, give or take the apes and the dolphins. Humans were the source of every intention to change the world (for better or for worse), humans produced the sources of universal truth (sacred texts, say, or next year's plans), and in the end it was humans who carried personal responsibility for what they did.",

  beingsH: "New intelligent beings",
  beingsP1:
    "The new order changes everything: alongside us, a new kind of intelligent being is emerging on Earth — the thing we currently call artificial intelligence. AI is free of our human limitations (the need to sleep, say, or the size of a working memory), and it may, in the end, transform fully and for the better if not all life on Earth, then at least the landscape of the modern economy.",
  beingsP2:
    "Unfortunately, artificial intelligence comes with a set of obvious technical problems: it struggles to understand the Real World (embodied AI is working on that), it cannot reliably tell the important from the unimportant, and it cannot hold a long-term plan. VibeVM is a partial answer to the important-versus-unimportant question, and long-horizon agents such as Zap run processes that last days, weeks, months — or even years.",

  intentH: "A second source of intention",
  intentP1:
    "Everyone knows that AI-assisted development is very expensive. A single two-hundred-dollar Claude or Codex subscription can go up in smoke in a matter of hours.",
  intentP2:
    "There are plenty of hypotheses about what exactly to optimize. Within AI-Native languages, here is the one we find interesting. The deepest change of the new order is that intention no longer comes from humans alone: AI is now a source of intention too. AI has its own view of the world, its own mode of perception, its own way of thinking — and that way of thinking resembles ours only remotely.",
  intentP3:
    "Which means that if we make the development process legible to machines first, rather than to people, we can reach results that are faster, better and cheaper. People are welcome to join in — as soon as direct human-machine interfaces are invented. A lot of hope rides on Neuralink here.",

  sddH: "Spec-Driven Development",
  sddP1:
    "The general approach — keep a well-structured, dynamic context and watch it evolve over time — is called Spec-Driven Development. It has plenty of problems.",
  sddP2:
    "First, contrary to every expectation, SDD puts the inhumanity of AI-assisted development on full display. The volume of specifications quickly passes all reasonable bounds and becomes impossible to hold in a human head. Specifications start to consist of strange words and terms that an AI understands perfectly and a human can reach only through long, laborious translation. The good news: that translation is possible. Merely inconvenient.",
  sddP3:
    "Second — and this one is serious — true Spec-Driven Development, where every change regenerates all of the code from the specification, is an unreachable dream: nobody has that kind of money. To afford absolute SDD we would have to plug our datacenter straight into the Sun, or into some other star. Possibly more than one!",
  starAlt:
    "A datacenter plugged straight into a star: a small black block, a long diagonal cord, a plug, and a huge golden circle.",
  starCaption:
    "fig. 02 · the price of absolute SDD: a plug, straight into the Sun",

  splitH: "Specification and code",
  splitP1:
    "Our coding discipline is built in a rather unusual way. We write code to be understood by machines first. Do we lose something? Yes and no. People do not want to write code anyway — and they do not want to write specifications either.",
  splitP2:
    "Ideally, all of these artifacts should be generated automatically, spending as little LLM inference as possible (a reminder: LLM generation is very expensive). The human becomes part of the system, and the human's value is to bring new meaning and desires of their own — definitely not to write and read millions of lines of specifications.",
  splitP3:
    "Our coding style is somewhat reminiscent of the classic split between interfaces and classes in Java, or between headers and sources in C++. There is a dichotomy between the LLM specification and the algorithmic implementation in code.",
  splitP4:
    "The specification sets the contract. The code is its technical implementation, dense with small details.",
  dichotomyLabel: "The specification/code dichotomy, in two columns",
  specHead: "Specification",
  specPoints: [
    "sets the contract",
    "checked and executed only through an LLM",
    "analysis comes back as a sample: blurry and unstable",
    "very expensive",
  ],
  codeHead: "Code",
  codePoints: [
    "the contract's technical implementation",
    "checked by crisp, cheap commands",
    "uses the hardware you actually have",
    "cheap",
  ],
  splitP5:
    "Yes, you can describe every low-level implementation detail right in the specification. But it quickly turns out that for low-level detail there is no better contract than the code itself. Yes — that same good old code we were used to before AI was invented. The best high-level description of a processor, after all, is assembly. However long you describe an x86_64 or Arm64 algorithm in English, what you get is invariably a poorer copy of the processor.",
  splitP6:
    "And the specification is expensive: the only way to check or execute it is an LLM, and the result of that analysis is a sample — blurry and unstable. Code, by contrast, is checked by crisp, cheap commands that squeeze everything out of the hardware we have today.",
  layersAlt:
    "Two layers: a blurred golden field of sampled dots above, a crisp grid of ink blocks below, thin traceable edges between them; one block hangs unattached, marked with a cross.",
  layersCaption:
    "fig. 03 · the expensive probabilistic layer above the cheap deterministic one; between them, traceable edges",

  lawK: "The most important rule",
  lawLead: "Hence the most important rule:",
  lawQuote:
    "Everything that can be done without an LLM must be done without an LLM.",
  lawBody:
    "No generation — and certainly no regeneration. We touch the LLM only when we cross into the territory of high-level meaning, the part that has to be written down abstractly, as a specification.",

  edgesH: "Traceable edges",
  edgesP1:
    "All AI-Native languages in VibeVM are connected by traceable, checkable edges. For every piece of code we can say which requirement or principle that piece of code fulfils. Or we can say that it has no specification attached — and then there is a range of possibilities: either the code is its own specification (something low-level), or we are prototyping and have not generalized the requirements yet, or it is a piece of old legacy code, or — and this is the most frequent case — it is simply an error to be fixed. We can build graphs of these edges and of how they evolve, and reason over them.",
  edgesP2:
    "This lets us take on tasks that were unimaginable in the recent past and finish them in the shortest time — and right now it does not require drinking the energy of stars or travelling to parallel quantum worlds.",
  edgesP3:
    "One pattern deserves a mention of its own: specification inheritance, in exactly the style people wrote object-oriented code in for decades.",
  oopQuote:
    "This may be the first time in history when all the fairy tales about OOP suddenly turn out to be the harsh reality.",
  ladderAlt:
    "Specification inheritance: a column of squares, each carrying a smaller copy of the one above, joined by edges.",
  ladderCaption:
    "fig. 04 · specification inheritance — OOP, but for real this time",

  togetherH: "A foundation for collaboration",
  togetherP1:
    "Better still, all of this information is not merely computable inside VibeVM — it is open to any long-horizon agent and to any other tool. The VibeVM distribution ships Zap as one example of such a tool, but you can easily write your own. We would genuinely love you to. VibeVM is a foundation for the developers of the new technological order to build on together, not an occasion for yet another needless competition.",

  startH: "The starting point",
  startP1:
    "In an ideal world — the one where we all become cyborgs, augmented with powerful AI instruments — VibeVM is a low-level system, perceived roughly the way assembly is perceived today. It is our starting point into a new world where everything is different. A world where we live far better, more beautifully and more happily than we do now.",

  disclaimerK: "Disclaimer",
  disclaimerHtml:
    "This research and this vision are a <em>temporary</em> point of view: it will change as the project evolves and as our knowledge of AI-assisted development grows. The article will be edited and extended.",

  tieK: "Where this becomes practice",
  tieHead: "AI-Native Language",
  tieBody:
    "The discipline this essay argues for: code whose every line is machine-traceable to a requirement, held by deterministic gates beside your ordinary toolchain.",
  tieLink: "Why AI-Native",
};
