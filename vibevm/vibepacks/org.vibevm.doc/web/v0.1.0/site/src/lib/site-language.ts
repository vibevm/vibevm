/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The language of the SITE — the furniture a reader reads the manual
 * through — which is not the language of the manual.
 *
 * They were one control until now, and a reader who wanted the interface
 * in their own language had to move the documentation into it as well.
 * The two questions are different and have different answers: which
 * edition of a text you are reading is a fact about the text, and lives
 * in the address (D-06); which words the buttons around it carry is a
 * preference of the person, and lives with the theme.
 *
 * So this file owns one question only: what the chrome says. The
 * documentation's language is `lib/library.ts`'s and the address map's,
 * and nothing here touches either — in particular `<html lang>`, which
 * is written by `entry.ssr.tsx` from the ADDRESS and names the language
 * of the document, stays exactly as it was. A page whose text is English
 * is an English document however its buttons are labelled.
 *
 * The table is keyed by the English string because the English string is
 * what the page already carries: the routes are prerendered once, in the
 * language they are written in, and a reader's choice arrives afterwards
 * in their own browser. A key beside every label would be the same table
 * with a second name for each row to keep in step.
 */

export const SITE_LANGUAGES = ["en", "ru"] as const;

/** Which language the furniture speaks. Two, because the site has two. */
export type SiteLanguage = (typeof SITE_LANGUAGES)[number];

/** Whether a value is one of the two — asked, never asserted. */
export function isSiteLanguage(value: unknown): value is SiteLanguage {
  return SITE_LANGUAGES.some((one) => one === value);
}

/** What a language is called in itself, for the switch that offers it. */
export const SITE_LANGUAGE_LABEL: Readonly<Record<SiteLanguage, string>> = {
  en: "EN",
  ru: "RU",
};

/**
 * The chrome, in the one language it is not written in.
 *
 * Every row is a string some route or component passes as a label, and
 * the whole of it is furniture: headings of shelves, names of controls,
 * the sentences that say what a shelf's order means, the words a panel
 * puts on its own buttons. Nothing a package wrote is in here and
 * nothing ever will be — a title, an abstract and a page's text belong
 * to the documentation and are shown in the language the documentation
 * is in.
 *
 * It is deliberately one table rather than a module per surface. It is
 * small because the chrome is small, and a reader comparing the two
 * columns can see the whole interface at once.
 */
export const RUSSIAN_CHROME: Readonly<Record<string, string>> = {
  /* The site's header and footer. */
  "Search the documentation": "Искать в документации",
  Search: "Поиск",
  "Nothing here carries that word.": "Здесь нет ничего с этим словом.",
  "Site language": "Язык сайта",
  "© 2026 Oleg Chirukhin": "© 2026 Олег Чирухин",

  /* The catalogue behind the door, and its three shelves. */
  Documentation: "Документация",
  "the catalogue for an agent": "каталог для агента",
  Catalogue: "Каталог",
  Featured: "Избранное",
  "Where this site asks a new reader to start. Everything else it carries is on the two shelves beside this one.":
    "С чего этот сайт предлагает начать новому читателю. Всё остальное, что он несёт, — на двух полках рядом.",
  "This build carries none of the documentations named as featured.":
    "В этой сборке нет ни одной из документаций, названных избранными.",
  Documents: "Документы",
  "Every documentation somebody wrote, the featured ones included. A star marks an adaptation the author of the documentation named; each documentation's source is listed first, because it is what officiality is measured against.":
    "Все документации, которые кто-то написал, включая избранные. Звёздочка отмечает адаптацию, которую назвал автор документации; источник каждой документации стоит первым — именно с ним сверяется официальность.",
  "This build carries no documentation.": "Эта сборка не несёт документации.",
  Projections: "Проекции",
  "Printed by the pipeline out of a package's own bytes, so that every package has something to read. Nobody wrote these pages, and every card here says so.":
    "Напечатано конвейером из собственных байтов пакета, чтобы у каждого пакета было что читать. Эти страницы никто не писал, и каждая карточка здесь об этом говорит.",
  "This build carries no generated documentation.":
    "Эта сборка не несёт сгенерированной документации.",
  "what it covers": "что покрывает",

  /* The three shelves of a package's own page. */
  "A star marks documentation the subject itself points at. The order says the same thing: primary, then official, then community.":
    "Звёздочка отмечает документацию, на которую указывает сам предмет. Порядок говорит о том же: основная, затем официальная, затем от сообщества.",
  "Nothing documents this subject yet.":
    "Этот предмет пока никто не документирует.",
  Adaptations: "Адаптации",
  "A star marks an adaptation the author of this documentation named. It means «named by them», not «approved by the subject».":
    "Звёздочка отмечает адаптацию, которую назвал автор этой документации. Это значит «названа им», а не «одобрена предметом».",
  "No adaptation of this documentation has been published.":
    "Ни одной адаптации этой документации не опубликовано.",
  Pages: "Страницы",
  "In the order the layer law gives them: text that stands still before text that moves with the product.":
    "В порядке закона слоёв: сначала текст, который стоит на месте, затем текст, который движется вместе с продуктом.",
  "This documentation has no pages yet.":
    "У этой документации пока нет страниц.",

  /* The furniture of one page. The two lists on either side of the text
     are named for what each one is a list OF: the manual, and the page
     open in it. */
  Contents: "Оглавление",
  "On this page": "На этой странице",
  "Rules this page cites": "Правила, которые цитирует эта страница",
  "Documentation language": "Язык документации",
  Everything: "Все языки",
  Version: "Версия",
  Platform: "Платформа",
  "For an agent": "Для агента",
  "This page has a machine mirror. The citation carries the version rather than latest, so what an agent quotes does not move under it.":
    "У этой страницы есть машинное зеркало. В цитате стоит версия, а не latest, поэтому то, что процитирует агент, не уедет под ним.",
  copy: "скопировать",
  "The rule this page quotes": "Правило, которое цитирует страница",
  "copy address": "скопировать адрес",
  "open the rule": "открыть правило",
  close: "закрыть",
  "Reading settings": "Настройки чтения",
  "back to where you were": "вернуться туда, где вы были",
  "The picture or table you opened": "Картинка или таблица, которую вы открыли",
  "For an agent: the address of this place": "Для агента: адрес этого места",
  ok: "ок",

  /* The meta row and the reading panel. */
  Publisher: "Издатель",
  Adapts: "Адаптирует",
  Audiences: "Аудитории",
  "Reading time": "Время чтения",
  Rendered: "Собрано",
  "Read aloud": "Прочитано вслух",
  never: "никогда",
  Theme: "Тема",
  Text: "Текст",
  Column: "Ширина колонки",
  "Block numbers": "Номера блоков",
  "Smaller text": "Мельче",
  "Larger text": "Крупнее",
  Light: "Светлая",
  Dark: "Тёмная",
  System: "Системная",
};

/** The same table read the other way, so a switch back is not a reload. */
const ENGLISH_CHROME: Readonly<Record<string, string>> = Object.fromEntries(
  Object.entries(RUSSIAN_CHROME).map(([english, russian]) => [
    russian,
    english,
  ]),
);

/**
 * One string of the chrome in the language asked for, or `null` when
 * this is not a string of the chrome at all.
 *
 * `null` and not the input, because the caller has to tell «leave this
 * alone» from «this is already right»: a text node the table does not
 * know must not be written back to, or every rewrite would dirty the
 * whole document for nothing.
 */
export function chromeIn(text: string, language: SiteLanguage): string | null {
  const table = language === "ru" ? RUSSIAN_CHROME : ENGLISH_CHROME;
  return table[text] ?? null;
}

/**
 * The language to start in for a reader who has never chosen: the
 * browser's own preference list, in its own order.
 *
 * Only the primary subtag is compared — `ru-RU` and `ru` are the same
 * request — and anything the site does not carry is passed over rather
 * than approximated. Nothing matching means English, which is the
 * language the interface is written in.
 */
export function preferredSiteLanguage(
  preferences: readonly string[],
): SiteLanguage {
  for (const preference of preferences) {
    const primary = (preference.split("-")[0] ?? preference).toLowerCase();
    const known = SITE_LANGUAGES.find((one) => one === primary);
    if (known !== undefined) return known;
  }
  return "en";
}
