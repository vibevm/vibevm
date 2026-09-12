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
  DocsNav,
  type DocsNavItem,
  type DocsNavProps,
} from "./components/docs-nav/index.tsx";
export { DocCard, type DocCardProps } from "./components/doc-card/index.tsx";
export {
  SearchBox,
  type SearchBoxProps,
} from "./components/search-box/index.tsx";
export { Tag, type TagProps } from "./components/tag/index.tsx";
export {
  Badge,
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
