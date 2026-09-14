/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-WORKSPACE */

/**
 * The seam of the design system: everything the application is allowed
 * to import, and nothing else.
 *
 * The package exports source, not a build. The Qwik optimizer reads
 * these `.tsx` files through the workspace link and splits them into
 * chunks beside the application's own code, so there is no library build
 * step between a change here and the page that shows it — and no second
 * copy of the framework to keep in step.
 */

export {
  DocsHeader,
  type DocsHeaderProps,
} from "./components/docs-header/index.tsx";
export {
  Contents,
  type ContentsItem,
  type ContentsProps,
  type ContentsSection,
} from "./components/contents/index.tsx";
export { DocCard, type DocCardProps } from "./components/doc-card/index.tsx";
export {
  SearchBox,
  type SearchBoxProps,
  type SearchHit,
} from "./components/search-box/index.tsx";
export { Tag, type TagProps } from "./components/tag/index.tsx";
export {
  AuthorshipBadge,
  Badge,
  GeneratedBadge,
  type AuthorshipBadgeProps,
  type AuthorshipMark,
  type BadgeProps,
  type DocStatus,
} from "./components/badge/index.tsx";
export { Prose, type ProseProps } from "./components/prose/index.tsx";
export {
  Footnotes,
  type FootnotesProps,
} from "./components/footnotes/index.tsx";
export {
  TableScroll,
  type TableScrollProps,
} from "./components/table-scroll/index.tsx";
export { Fab, type FabProps } from "./components/fab/index.tsx";
export { Footer, type FooterProps } from "./components/footer/index.tsx";
export {
  SectionHead,
  type SectionHeadProps,
} from "./components/section-head/index.tsx";
export {
  TabPills,
  type TabPill,
  type TabPillsProps,
} from "./components/tab-pills/index.tsx";

/* The landing's own three. They live in the design system rather than
   beside the route because the landing and the documentation are one
   site on one set of components (D-28): a hero written inside the route
   tree would be a second design system with one user. */
export {
  Hero,
  type HeroAction,
  type HeroProps,
} from "./components/hero/index.tsx";
export {
  InstallBlock,
  type InstallBlockProps,
  type InstallCommand,
} from "./components/hero/install-block.tsx";
export { DepGraph } from "./components/dep-graph/index.tsx";
export {
  CapabilityCard,
  type CapabilityCardProps,
  CapabilityRow,
  type CapabilityRowProps,
} from "./components/capability-card/index.tsx";
export {
  LanguageSelector,
  type LanguageChoice,
  type LanguageSelectorProps,
} from "./components/language-selector/index.tsx";
export {
  AuthorshipFilter,
  type AuthorshipChoice,
  type AuthorshipFilterProps,
} from "./components/authorship-filter/index.tsx";
export {
  SiteLanguageSwitch,
  type SiteLanguageChoice,
  type SiteLanguageSwitchProps,
} from "./components/site-language/index.tsx";
export {
  VersionSwitch,
  type VersionChoice,
  type VersionSwitchProps,
} from "./components/version-switch/index.tsx";
export { Card, type CardProps } from "./components/card/index.tsx";
export {
  BridgeSignatures,
  type BridgeAuthorship,
  type BridgeSignaturesProps,
} from "./components/bridge-signatures/index.tsx";
export { Shelf, type ShelfProps } from "./components/shelf/index.tsx";
export {
  PackageHeader,
  type PackageHeaderProps,
} from "./components/package-header/index.tsx";
export {
  PageMeta,
  type MetaLink,
  type PageMetaProps,
} from "./components/page-meta/index.tsx";
export {
  RulePanel,
  type RulePanelProps,
} from "./components/rule-panel/index.tsx";
export {
  ForAgent,
  type AgentLink,
  type ForAgentProps,
} from "./components/for-agent/index.tsx";
export {
  SettingsPanel,
  type SettingsPanelProps,
} from "./components/settings-panel/index.tsx";
export {
  ThemeSwitch,
  type ThemeSwitchProps,
} from "./components/theme-switch/index.tsx";
export {
  ReturnToPlace,
  type ReturnToPlaceProps,
} from "./components/return-to-place/index.tsx";
export { Toc, type TocProps } from "./components/toc/index.tsx";
export {
  CitedRules,
  type CitedRulesProps,
} from "./components/cited-rules/index.tsx";
export { Lightbox, type LightboxProps } from "./components/lightbox/index.tsx";
export { CodeChrome } from "./components/code-block/index.tsx";
export {
  FallbackNotice,
  type FallbackNoticeProps,
} from "./components/fallback-notice/index.tsx";
