/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The roadmap — the English edition, and the source the Russian one is
 * adapted from.
 *
 * The six stages and their order are the owner's (2026-09-26), and so is
 * what each one is: a more flexible and extensible core; Zap, the
 * announced agent workspace, described here only as its own page
 * describes it; the first packages from the community; IDE plugins for
 * IntelliJ IDEA and VS Code; a library and marketplace in the spirit of
 * Steam; a Linux whose package manager is vibevm. The page adds a
 * sentence of what each stage changes for the reader, and nothing else —
 * no dates, no version numbers beyond the two previews, no feature
 * claimed as existing. The one month named anywhere in the family is
 * Zap's, and it stays on Zap's page, where the link leads.
 */

import type { RoadmapStrings } from "./strings.ts";

export const COPY: RoadmapStrings = {
  eyebrow: "Roadmap · six stages, in order",
  title: "From a preview to an operating system.",
  lead: "VibeVM is in Developer Preview 1. This page is the order of what comes after it: six stages, each standing on the one before. It carries no dates. It is a sequence, not a schedule.",
  heroAlt:
    "A road as one long ink diagonal rising to the right. At its foot a terracotta circle marks where the project stands; along the road six stations grow from a single square, through a small orbital map, into a whole system built of the same squares, blocks, plates and shelves.",
  heroCaption:
    "fig. 01 · the road, from Developer Preview 1 at the foot to Spec-Driven Linux at the top. Every station is built of the shapes of the one before it.",

  hereK: "You are here",
  hereName: "Developer Preview 1",
  hereBody:
    "What can be installed today: the vibe command, the package format, the registries it resolves packages from, and the manual on this site. The front page calls it a preview, and it is one: early, working, and not yet the whole idea.",
  hereAlt:
    "A terracotta circle with a thin halo, and beside it one small ink square: the project as it stands, and the core it stands on.",

  stagesK: "The six stages",
  stagesH: "What comes next, in this order",
  routeLabel: "The six stages of the roadmap, in order",
  stages: [
    {
      id: "dp2",
      name: "Developer Preview 2",
      sub: "A more flexible and extensible core.",
      body: "The second preview rebuilds the core so that more of it can be replaced or extended without touching the rest. Everything after this stage needs something from the core: a workspace that talks to it, plugins that talk to it, a marketplace that reads it, a distribution that stands on it. This is where the core learns to be built on.",
      alt: "A large ink square open on one side; three smaller squares are drawn out of the opening on thin edges, and one of them, filled terracotta, has docked.",
    },
    {
      id: "zap",
      name: "Zap",
      sub: "A local workspace for running coding agents in parallel, announced and in development.",
      body: "Zap is to put projects, agent conversations, questions, managed work and Git worktrees on one map on your own machine, with the agents you already have accounts for. It is an announcement, not a release: its own page says what it will be. On this road it is the first product to arrive after the second preview.",
      alt: "An orbital map in Zap's green: a goal core with dashed rings around it, work nodes on the rings, one terracotta node for the project, and a trajectory leaving toward the edge.",
      link: { lead: "Its own page:", text: "Why Zap" },
    },
    {
      id: "community",
      name: "The first community packages",
      sub: "The registries fill with packages other people wrote.",
      body: "A package manager with one author is a build tool. With many, it is a commons. Once this stage holds, the spec you need may already exist, written by someone who met your problem before you did, and installing it is one line.",
      alt: "A grid of small squares, most of them ink, a few gold, a few still empty outlines; more squares arrive from outside along thin diagonals to fill the gaps.",
    },
    {
      id: "ide",
      name: "IDE plugins",
      sub: "For IntelliJ IDEA and Visual Studio Code.",
      body: "Until this stage vibe is a terminal. After it, vibe is also a panel beside the code, in two editors a great many developers already have open. What the terminal answers today, the editor will answer in place.",
      alt: "Two frames with corner brackets, side by side. A cobalt plate slides into the left frame and an ink plate into the right; under both, one small terracotta square is joined to each by a thin edge.",
    },
    {
      id: "library",
      name: "A library and marketplace",
      sub: "A place of our own for browsing packages, in the spirit of Steam.",
      body: "A registry answers a name you already know. A store front shows you what you did not know to look for. This stage gives packages the second kind of place: covers on shelves rather than names in a list, and room to browse.",
      alt: "Three shelf lines; on them stand covers of unequal height and width in ink, gold, cobalt and terracotta, and one cover is drawn forward and tilted, as if picked up.",
    },
    {
      id: "linux",
      name: "Spec-Driven Linux",
      sub: "A Linux distribution whose package manager is vibevm.",
      body: "The last stage turns the argument around. Today a spec stack is installed into a system. Here the system is what gets installed: packaged, pinned and described the way a stack is, by the same tool. The discipline stops at no boundary.",
      alt: "A large ink plane standing on a floor line, and inside it the earlier shapes assembled into one construction: the opened core square, the grid of packages, a plate in its frame, a shelf of covers; a terracotta circle at its base and a gold diamond where they meet.",
      link: { lead: "Its future home:", text: "specdrivenlinux.org" },
    },
  ],

  bandK: "How to read this page",
  bandQuote: "Six stages, in this order. No dates.",
  bandBody:
    "Each stage stands on the one before it, and a date written here would be a guess dressed as a promise. When a stage lands, the news channel says so, and the marker on this page moves.",

  tiesK: "From here",
  tieNewsHead: "News & support",
  tieNewsBody:
    "Where a stage landing is announced first: the Telegram channel, and the chat for questions.",
  tieNewsLink: "Follow the news",
  tieVisionHead: "The Big Vision",
  tieVisionBody:
    "Why the road goes where it goes: the essay behind every stage on this page.",
  tieVisionLink: "Read the essay",
  tieDocsHead: "Documentation",
  tieDocsBody:
    "Start where the marker is: the manual for what can be installed today.",
  tieDocsLink: "Open the manual",
};
