# Interface copy of the documentation site and the local reader (P.4) {#root}

<status stage="spec" state="work" comment="P.4 of the docs-2026-09 campaign, 2026-09-12; the English strings of the shell; wave B moves them into site/i18n of org.vibevm.doc/web from this file; the Russian adaptation of these strings is wave C"/>

Rules: every string is plain English, no exclamation marks, no emoji, sentence
case, the shortest phrase that names the action. Keys are stable identifiers
for wave B; the value is the text a person sees. Placeholders are in braces.

## Header and navigation {#header}

| Key | Text |
|---|---|
| `nav.docs` | Documentation |
| `nav.packages` | Packages |
| `nav.for-agents` | For agents |
| `nav.github` | GitHub |
| `nav.gitverse` | GitVerse |
| `search.placeholder` | Search the documentation |
| `search.shortcut` | Ctrl K |
| `search.no-results` | Nothing found for “{query}”. |
| `lang.label` | Language |
| `lang.official` | official |
| `lang.community` | community |
| `lang.publisher` | by {group} |
| `theme.label` | Theme |
| `theme.light` | Light |
| `theme.dark` | Dark |
| `theme.system` | System |

## Catalogue and shelves {#catalogue}

| Key | Text |
|---|---|
| `catalogue.title` | Documentation of VibeVM packages |
| `catalogue.lead` | Every published package, rendered from its own files, and the manuals written for it. |
| `shelf.primary` | Primary documentation |
| `shelf.official` | Official documentation |
| `shelf.community` | Community documentation |
| `shelf.empty` | No documentation has been published for this package yet. |
| `card.official-star.tooltip` | Named official by the package’s publisher |
| `card.translation-star.tooltip` | Named official by the author of this documentation, not by the package |
| `card.publisher` | Published by {group} |
| `card.language` | Language: {language} |
| `card.version` | Version {version} |
| `card.latest` | latest |
| `card.updated` | Published {date} |
| `card.audiences` | For: {audiences} |
| `audience.user` | users |
| `audience.author` | package authors |
| `audience.dev` | maintainers |
| `audience.agent` | agents |

## Package page, level 0 {#package-page}

| Key | Text |
|---|---|
| `package.tab.overview` | Overview |
| `package.tab.readme` | README |
| `package.tab.specs` | Specifications |
| `package.tab.docs` | Documentation |
| `package.tab.versions` | Versions |
| `package.tab.agents` | For agents |
| `package.overview.kind` | Kind |
| `package.overview.license` | Licence |
| `package.overview.keywords` | Keywords |
| `package.overview.requires` | Requires |
| `package.overview.provides` | Provides |
| `package.overview.dependants` | Used by |
| `package.overview.explained-in` | Explained in |
| `package.overview.translated-into` | Translated into |
| `package.boot-snippet.note` | Read by the agent at every session start. |
| `package.skills` | Skills |
| `package.binaries` | Tools |
| `package.servers` | MCP servers |
| `package.version.render-failed` | This version could not be rendered: {reason}. Other versions and languages are listed on the left. |

## Documentation page and the reader {#reader}

| Key | Text |
|---|---|
| `page.meta.publisher` | Published by {group} |
| `page.meta.version` | Version {version} |
| `page.meta.rendered` | Rendered {date} |
| `page.meta.last-read` | Last read aloud {date} |
| `page.meta.adapts` | An adaptation of {title} by {group} |
| `page.meta.reading-time` | {minutes} min read |
| `page.links.markdown` | Markdown |
| `page.links.xml` | XML |
| `page.links.agent` | For agent |
| `toc.title` | On this page |
| `toc.rules` | Rules this page cites |
| `anchors.toggle` | Block numbers |
| `anchors.copied` | Link copied |
| `settings.title` | Reading |
| `settings.font-smaller` | Smaller text |
| `settings.font-larger` | Larger text |
| `settings.width-narrower` | Narrower column |
| `settings.width-wider` | Wider column |
| `settings.reset` | Reset |
| `reading.return` | Return to where you were |
| `reading.contents` | Contents |
| `contents.view` | Contents view |
| `contents.view.path` | In order |
| `contents.view.sections` | By section |
| `contents.chapter` | Chapter |
| `path.label` | The learning path |
| `path.previous` | Previous |
| `path.next` | Next |
| `pages.order.layer` | In the order the layer law gives them: text that stands still before text that moves with the product. |
| `pages.order.path` | In the order of the learning path |
| `pages.start` | Start here |
| `fallback.notice` | This page is not yet available in {language}. You are reading the {source-language} original. |
| `fallback.ok` | OK |
| `example.expected` | Expected output |
| `example.stderr` | Expected errors |
| `example.copy` | Copy |
| `rule.source` | From the specification, in its own words |
| `rule.open` | Open the rule |
| `derived.generated-from` | Generated from {source} |
| `prompt.title` | Ask your agent |
| `prompt.needs` | The agent needs |
| `prompt.outcome` | When it worked |
| `prompt.asserts` | Check with |
| `prompt.copy` | Copy the request |
| `prompt.send` | Hand to agent |
| `figure.open` | Open the image |
| `table.expand` | Expand the table |
| `footnotes.title` | Notes |
| `footnotes.back` | Back to the text |
| `print.hint` | Printed from {url} on {date} |

## The button for agents {#fab}

| Key | Text |
|---|---|
| `fab.label` | For agent |
| `fab.address` | Address of this place |
| `fab.copy-address` | Copy the address |
| `fab.markdown` | This page as Markdown |
| `fab.xml` | This page as XML |
| `fab.llms` | The index for agents |
| `fab.hint` | Give the address to your agent; the skill knows what to do with it. |

## Local reader {#local}

| Key | Text |
|---|---|
| `local.source.store` | From the machine store |
| `local.source.lock` | From this project’s lock file |
| `local.source.registry` | From the registry |
| `local.source.checkout` | From the checkout |
| `local.shell.fallback` | The full reader interface is not installed. Run `vibe doc shell install` to download it for this version, or keep reading with the plain interface. |
| `local.offline` | No network is used. Nothing leaves this machine. |
| `local.open-file` | Open in the editor |

## Errors and empty states {#errors}

| Key | Text |
|---|---|
| `404.title` | There is no page at this address |
| `404.lead` | The address may be old, or the page may not exist in this version or language. |
| `404.search` | Search the documentation |
| `404.home` | Back to the documentation |
| `error.render.title` | This page could not be rendered |
| `empty.section` | This section has no pages yet. |

## Footer {#footer}

| Key | Text |
|---|---|
| `footer.copyright` | © {year} VibeVM |
| `footer.source` | Source on GitHub and GitVerse |
| `footer.machine-files` | For agents: llms.txt, llms-full.txt, manifest.json |
| `footer.licence` | Documentation licence: UPL-1.0 |
