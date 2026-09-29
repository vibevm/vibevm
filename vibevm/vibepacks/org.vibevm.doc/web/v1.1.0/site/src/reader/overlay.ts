/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

/**
 * Pictures and wide tables, opened full-screen and closed again.
 *
 * One overlay serves both because a reader gets out of them the same
 * way; only what goes inside differs. The scroll of the page behind it
 * is locked by fixing the body rather than by `overflow: hidden`, which
 * is the one part of this worth explaining: on iOS Safari `overflow:
 * hidden` on the body does not stop the page scrolling, and the usual
 * repair — a document-level `touchmove` handler — blocks the single
 * finger scroll INSIDE the overlay, which is exactly what a reader needs
 * for a table. Fixing the body stops the page and leaves every gesture
 * in the overlay working.
 *
 * A wide table also gets a toolbar with an expand control, and the table
 * that opens is a CLONE. The original stays in the document where the
 * block number, the anchors and the reader's place all still point at
 * it: moving it would break three things to save one copy.
 */

import { all, island, one, targetOf } from "./dom.ts";

/** What a reader is told the control does, in the shell's language. */
const EXPAND = "expand";

export function startOverlay(): () => void {
  const region = island();
  const overlay = one("[data-lightbox]");
  const slotImage = one("[data-lightbox-image]");
  const slotTable = one("[data-lightbox-table]");
  if (
    region === null ||
    overlay === null ||
    slotImage === null ||
    slotTable === null
  ) {
    return () => undefined;
  }

  let savedScroll = 0;

  const lock = (): void => {
    savedScroll = window.scrollY;
    document.body.style.position = "fixed";
    document.body.style.top = `-${savedScroll}px`;
    document.body.style.left = "0";
    document.body.style.right = "0";
  };

  const unlock = (): void => {
    document.body.style.position = "";
    document.body.style.top = "";
    document.body.style.left = "";
    document.body.style.right = "";
    window.scrollTo(0, savedScroll);
  };

  const close = (): void => {
    if (overlay.hidden) return;
    overlay.hidden = true;
    slotImage.hidden = true;
    slotTable.hidden = true;
    slotTable.replaceChildren();
    unlock();
  };

  const openImage = (image: HTMLImageElement): void => {
    if (slotImage instanceof HTMLImageElement) {
      slotImage.src =
        image.currentSrc.length > 0 ? image.currentSrc : image.src;
      slotImage.alt = image.alt;
    }
    slotImage.hidden = false;
    slotTable.hidden = true;
    overlay.hidden = false;
    lock();
  };

  const openTable = (table: Element): void => {
    slotTable.replaceChildren(table.cloneNode(true));
    slotTable.hidden = false;
    slotImage.hidden = true;
    overlay.hidden = false;
    lock();
  };

  /**
   * Every table is put into a scrolling region with the control that
   * opens it above.
   *
   * The region is what keeps a wide table from making the whole PAGE
   * scroll sideways, and it carries `tabindex` because a region that
   * scrolls has to be reachable from a keyboard — without it the columns
   * past the fold exist only for a mouse. The toolbar sits outside the
   * region rather than in it, so the control does not scroll away from
   * the reader who is looking for it.
   */
  const wrappers: HTMLElement[] = [];
  for (const table of all("table", region)) {
    const parent = table.parentNode;
    if (parent === null) continue;

    const block = document.createElement("div");
    block.className = "table-block";

    const toolbar = document.createElement("div");
    toolbar.className = "table-toolbar";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "table-toolbar__expand";
    button.textContent = EXPAND;
    button.setAttribute("data-expand-table", "");
    toolbar.appendChild(button);

    const scroller = document.createElement("div");
    scroller.className = "table-scroll";
    scroller.setAttribute("role", "region");
    const caption = table.querySelector("caption")?.textContent ?? "";
    scroller.setAttribute(
      "aria-label",
      caption.trim().length === 0 ? "Table" : caption.trim(),
    );
    scroller.tabIndex = 0;

    parent.insertBefore(block, table);
    block.appendChild(toolbar);
    block.appendChild(scroller);
    scroller.appendChild(table);
    wrappers.push(block);
  }

  const onClick = (event: Event): void => {
    const target = targetOf(event);
    if (target === null) return;

    if (target.closest("[data-lightbox-close]") !== null) {
      close();
      return;
    }

    const expand = target.closest("[data-expand-table]");
    if (expand !== null) {
      const table = expand.closest(".table-block")?.querySelector("table");
      if (table !== null && table !== undefined) openTable(table);
      return;
    }

    if (!region.contains(target)) return;
    const image = target.closest("img");
    if (image instanceof HTMLImageElement) {
      event.preventDefault();
      openImage(image);
    }
  };

  const onKey = (event: KeyboardEvent): void => {
    if (event.key === "Escape") close();
  };

  document.addEventListener("click", onClick);
  document.addEventListener("keydown", onKey);

  return () => {
    document.removeEventListener("click", onClick);
    document.removeEventListener("keydown", onKey);
    // Put every table back where the pipeline left it, so a second
    // start does not wrap it twice.
    for (const block of wrappers) {
      const table = block.querySelector("table");
      if (table !== null) block.parentNode?.insertBefore(table, block);
      block.remove();
    }
    close();
  };
}
