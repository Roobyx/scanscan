import { gsap } from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { SplitText } from "gsap/SplitText";
import Lenis from "lenis";

import { KIND_META, SAMPLE_ROOT, findByPath, type SampleNode } from "../data/sample.js";
import { formatBytes, formatCount } from "../lib/format.js";
import { squarify } from "../lib/treemap.js";

gsap.registerPlugin(ScrollTrigger, SplitText);

const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const fine = window.matchMedia("(hover: hover) and (pointer: fine)").matches;

let lenis: Lenis | null = null;
let headerEl: HTMLElement | null = null;
let closeMenu: (() => void) | null = null;

const escapeHtml = (value: string) =>
  value.replace(
    /[&<>"']/g,
    (character) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character] ??
      character,
  );

const blockLabel = (width: number, height: number) => {
  if (width >= 84 && height >= 48) return "full";
  if (width >= 56 && height >= 30) return "name";
  return "none";
};

function initScroll() {
  if (reduce || !fine) return;
  lenis = new Lenis({ lerp: 0.1, wheelMultiplier: 1, touchMultiplier: 1 });
  lenis.on("scroll", ScrollTrigger.update);
  gsap.ticker.add((time) => lenis?.raf(time * 1000));
  gsap.ticker.lagSmoothing(0);
}

function initHeader() {
  headerEl = document.querySelector<HTMLElement>("[data-header]");
  if (!headerEl) return;

  ScrollTrigger.create({
    trigger: document.body,
    start: "top top",
    end: "max",
    onUpdate: (self) => headerEl?.classList.toggle("is-scrolled", self.scroll() > 16),
  });

  const button = document.querySelector<HTMLButtonElement>("[data-menu-btn]");
  const menu = document.querySelector<HTMLElement>("[data-menu]");
  if (!button || !menu) return;

  const links = [...menu.querySelectorAll<HTMLElement>(".menu__link")];
  const focusables = [
    ...menu.querySelectorAll<HTMLElement>('a[href], button:not([disabled])'),
  ];
  let open = false;
  let timeline: gsap.core.Timeline | null = null;

  const setOpen = (next: boolean, instant = false) => {
    if (open === next) return;
    open = next;
    button.setAttribute("aria-expanded", String(next));
    headerEl?.classList.toggle("is-open", next);
    document.documentElement.classList.toggle("menu-open", next);
    timeline?.kill();

    if (next) {
      menu.hidden = false;
      lenis?.stop();
      if (!reduce && !instant) {
        timeline = gsap
          .timeline()
          .fromTo(
            menu,
            { clipPath: "inset(0 0 100% 0)" },
            { clipPath: "inset(0 0 0% 0)", duration: 0.5, ease: "power3.out" },
          )
          .fromTo(
            links,
            { yPercent: 110, autoAlpha: 0 },
            {
              yPercent: 0,
              autoAlpha: 1,
              duration: 0.6,
              stagger: 0.05,
              ease: "power3.out",
            },
            0.08,
          );
      }
      focusables[0]?.focus();
      return;
    }

    lenis?.start();
    if (!reduce && !instant) {
      timeline = gsap
        .timeline({
          onComplete: () => {
            menu.hidden = true;
          },
        })
        .to(links, {
          yPercent: -40,
          autoAlpha: 0,
          duration: 0.22,
          stagger: 0.02,
          ease: "power2.in",
        })
        .to(menu, { clipPath: "inset(0 0 100% 0)", duration: 0.32, ease: "power3.in" }, 0.04);
    } else {
      menu.hidden = true;
    }
    button.focus();
  };

  button.addEventListener("click", () => setOpen(!open));

  document.addEventListener("keydown", (event) => {
    if (!open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
      return;
    }
    if (event.key !== "Tab" || focusables.length === 0) return;
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    }
  });

  const desktop = window.matchMedia("(min-width: 1081px)");
  desktop.addEventListener("change", (event) => {
    if (event.matches) setOpen(false, true);
  });

  closeMenu = () => setOpen(false, true);
}

function initAnchors() {
  document.addEventListener("click", (event) => {
    const anchor = (event.target as HTMLElement).closest<HTMLAnchorElement>("a[href]");
    if (!anchor || anchor.classList.contains("skip-link")) return;
    const url = new URL(anchor.href);
    if (url.pathname !== location.pathname || !url.hash || url.hash === "#") return;
    const target = document.querySelector<HTMLElement>(url.hash);
    if (!target) return;

    event.preventDefault();
    closeMenu?.();

    const offset = (headerEl?.offsetHeight ?? 64) + 16;
    const top = target.getBoundingClientRect().top + window.scrollY - offset;
    if (lenis) {
      lenis.scrollTo(top, {
        duration: 1.2,
        easing: (t) => 1 - (1 - t) ** 4,
      });
    } else {
      window.scrollTo({ top, behavior: reduce ? "auto" : "smooth" });
    }
    history.replaceState(null, "", url.hash);
  });
}

function initPageTransition() {
  if (reduce) return;
  const curtain = document.querySelector<HTMLElement>("[data-curtain]");
  if (!curtain) return;
  const blocks = [...curtain.querySelectorAll<HTMLElement>(".curtain__block")];
  if (blocks.length === 0) return;
  gsap.set(blocks, { scaleY: 0, transformOrigin: "bottom" });

  let leaving = false;
  document.addEventListener("click", (event) => {
    if (leaving || event.defaultPrevented) return;
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)
      return;
    const anchor = (event.target as HTMLElement).closest<HTMLAnchorElement>("a[href]");
    if (!anchor) return;
    if (anchor.target === "_blank" || anchor.hasAttribute("download")) return;
    if (anchor.classList.contains("skip-link")) return;
    const url = new URL(anchor.href);
    if (url.origin !== location.origin) return;
    if (url.hash && url.pathname === location.pathname) return;

    event.preventDefault();
    leaving = true;
    curtain.style.visibility = "visible";
    gsap.to(blocks, {
      scaleY: 1,
      duration: 0.55,
      stagger: 0.05,
      ease: "power3.inOut",
      onComplete: () => {
        location.href = url.href;
      },
    });
  });
}

function initButtons() {
  if (reduce || !fine) return;

  document.querySelectorAll<HTMLElement>("[data-fill]").forEach((button) => {
    const fill = button.querySelector<HTMLElement>(".btn__fill");
    if (!fill) return;

    let closed = true;
    const point = (event: PointerEvent) => {
      const rect = button.getBoundingClientRect();
      return `${event.clientX - rect.left}px ${event.clientY - rect.top}px`;
    };
    const open = (at: string) => {
      if (closed) gsap.set(fill, { clipPath: `circle(0% at ${at})` });
      closed = false;
      button.classList.add("is-filled");
      gsap.to(fill, {
        clipPath: `circle(150% at ${at})`,
        duration: 0.55,
        ease: "power3.out",
        overwrite: true,
      });
    };
    const close = (at: string) => {
      button.classList.remove("is-filled");
      gsap.to(fill, {
        clipPath: `circle(0% at ${at})`,
        duration: 0.4,
        ease: "power3.out",
        overwrite: true,
        onComplete: () => {
          closed = true;
        },
      });
    };

    button.addEventListener("pointerenter", (event) => open(point(event)));
    button.addEventListener("pointerleave", (event) => close(point(event)));
    button.addEventListener("focus", () => {
      if (button.matches(":focus-visible")) open("50% 50%");
    });
    button.addEventListener("blur", () => close("50% 50%"));
  });
}

function initMap() {
  const map = document.querySelector<HTMLElement>("[data-map]");
  if (!map) return;

  const canvas = map.querySelector<HTMLElement>("[data-map-canvas]");
  const crumbs = map.querySelector<HTMLElement>("[data-map-crumbs]");
  const legend = map.querySelector<HTMLElement>("[data-map-legend]");
  const pathEl = map.querySelector<HTMLElement>("[data-map-path]");
  const sizeEl = map.querySelector<HTMLElement>("[data-map-size]");
  const itemsEl = map.querySelector<HTMLElement>("[data-map-items]");
  const back = map.querySelector<HTMLButtonElement>("[data-map-back]");
  const status = document.querySelector<HTMLElement>("[data-map-status]");
  if (!canvas || !crumbs || !legend || !pathEl || !sizeEl || !itemsEl || !back) return;

  const scan = map.querySelector<HTMLElement>(".map__scan");
  if (scan && !reduce) {
    const sweep = gsap.fromTo(
      scan,
      { yPercent: -100 },
      { yPercent: 100, duration: 11, ease: "none", repeat: -1, paused: true },
    );
    const inView = () => {
      const rect = map.getBoundingClientRect();
      return rect.top < window.innerHeight && rect.bottom > 0;
    };
    ScrollTrigger.create({
      trigger: map,
      start: "top bottom",
      end: "bottom top",
      onToggle: (self) => {
        if (self.isActive) sweep.play();
        else sweep.pause();
      },
    });
    if (inView()) sweep.play();
  }

  const canvasEl = canvas;
  const backEl = back;

  let currentPath = "/";
  let current: SampleNode = SAMPLE_ROOT;
  let resizeCall: gsap.core.Tween | null = null;

  const announce = (message: string) => {
    if (status) status.textContent = message;
  };

  const showReadout = (node: SampleNode, path: string) => {
    pathEl.textContent = path;
    sizeEl.textContent = formatBytes(node.size);
    itemsEl.textContent = formatCount(node.items);
  };

  const renderCrumbs = (path: string) => {
    const parts = path.split("/").filter(Boolean);
    const fragment = document.createDocumentFragment();

    if (parts.length === 0) {
    const rootCurrent = document.createElement("span");
    rootCurrent.className = "map__crumb";
    rootCurrent.setAttribute("aria-current", "location");
    rootCurrent.textContent = "snapshot";
    fragment.appendChild(rootCurrent);
  } else {
    const rootButton = document.createElement("button");
    rootButton.type = "button";
    rootButton.className = "map__crumb";
    rootButton.textContent = "snapshot";
    rootButton.addEventListener("click", () => render("/", { animate: !reduce }));
    fragment.appendChild(rootButton);
  }

    parts.forEach((part, index) => {
      const separator = document.createElement("span");
      separator.className = "map__sep";
      separator.setAttribute("aria-hidden", "true");
      separator.textContent = "/";
      fragment.appendChild(separator);

      const prefix = `/${parts.slice(0, index + 1).join("/")}`;
      if (index === parts.length - 1) {
        const currentCrumb = document.createElement("span");
        currentCrumb.className = "map__crumb";
        currentCrumb.setAttribute("aria-current", "location");
        currentCrumb.textContent = part;
        fragment.appendChild(currentCrumb);
        return;
      }
      const crumb = document.createElement("button");
      crumb.type = "button";
      crumb.className = "map__crumb";
      crumb.textContent = part;
      crumb.addEventListener("click", () => render(prefix, { animate: !reduce }));
      fragment.appendChild(crumb);
    });

    crumbs.replaceChildren(fragment);
  };

  const renderLegend = (node: SampleNode) => {
    const kinds = [...new Set((node.children ?? []).map((child) => child.kind))];
    const fragment = document.createDocumentFragment();
    for (const kind of kinds) {
      const item = document.createElement("li");
      const dot = document.createElement("span");
      dot.className = "map__dot";
      dot.style.setProperty("--c", KIND_META[kind].color);
      item.appendChild(dot);
      item.appendChild(document.createTextNode(KIND_META[kind].label));
      fragment.appendChild(item);
    }
    legend.replaceChildren(fragment);
  };

  function render(
    path: string,
    options: { animate?: boolean; from?: { x: number; y: number; w: number; h: number }; focus?: boolean } = {},
  ) {
    const node = findByPath(path) ?? SAMPLE_ROOT;
    current = node;
    currentPath = path;

    const width = canvasEl.clientWidth;
    const height = canvasEl.clientHeight;
    const children = node.children ?? [];
    const tiles = squarify(
      children.map((child) => ({ value: child.size, data: child })),
      { x: 0, y: 0, w: width, h: height },
    );

    const oldBlocks = [...canvasEl.querySelectorAll<HTMLElement>(".map__block")];
    const fragment = document.createDocumentFragment();
    const newBlocks: HTMLButtonElement[] = [];

    for (const tile of tiles) {
      const child = tile.data;
      const childPath = path === "/" ? `/${child.name}` : `${path}/${child.name}`;
      const button = document.createElement("button");
      button.type = "button";
      button.className = "map__block";
      button.style.setProperty("--c", KIND_META[child.kind].color);
      button.style.width = `${Math.max(0, tile.w)}px`;
      button.style.height = `${Math.max(0, tile.h)}px`;
      button.dataset.label = blockLabel(tile.w, tile.h);
      button.setAttribute(
        "aria-label",
        `${child.name} ${formatBytes(child.size)}, ${formatCount(child.items)} items. ${child.children?.length ? "Open" : "Select"} ${childPath}.`,
      );
      button.innerHTML = `<span class="map__inner"><span class="map__name">${escapeHtml(child.name)}</span> <span class="map__size">${formatBytes(child.size)}</span>${child.children?.length ? '<span class="map__arrow" aria-hidden="true">↗</span>' : ""}</span>`;
      gsap.set(button, { x: tile.x, y: tile.y });

      button.addEventListener("pointerenter", () => showReadout(child, childPath));
      button.addEventListener("pointerleave", () => showReadout(current, currentPath));
      button.addEventListener("focus", () => showReadout(child, childPath));
      button.addEventListener("blur", () => showReadout(current, currentPath));
      button.addEventListener("click", () => {
        if (child.children?.length) {
          render(childPath, {
            animate: !reduce,
            from: { x: tile.x, y: tile.y, w: tile.w, h: tile.h },
          });
        } else {
          showReadout(child, childPath);
          announce(`Selected ${childPath}, ${formatBytes(child.size)}. No deeper breakdown in this sample.`);
        }
      });

      fragment.appendChild(button);
      newBlocks.push(button);
    }

    canvasEl.appendChild(fragment);
    if (options.animate && oldBlocks.length) {
      gsap.to(oldBlocks, {
        autoAlpha: 0,
        duration: 0.2,
        ease: "power2.out",
        onComplete: () => {
          for (const block of oldBlocks) block.remove();
        },
      });
    } else {
      for (const block of oldBlocks) block.remove();
    }

    if (options.animate && !reduce) {
      const from = options.from;
      if (from) {
        newBlocks.forEach((block, index) => {
          const tile = tiles[index];
          if (!tile) return;
          gsap.fromTo(
            block,
            {
              x: from.x,
              y: from.y,
              scaleX: from.w / Math.max(1, tile.w),
              scaleY: from.h / Math.max(1, tile.h),
              transformOrigin: "0 0",
            },
            {
              x: tile.x,
              y: tile.y,
              scaleX: 1,
              scaleY: 1,
              duration: 0.55,
              delay: index * 0.015,
              ease: "power3.out",
            },
          );
        });
        const inners = newBlocks
          .map((block) => block.querySelector<HTMLElement>(".map__inner"))
          .filter((inner): inner is HTMLElement => inner !== null);
        gsap.fromTo(
          inners,
          { autoAlpha: 0 },
          { autoAlpha: 1, duration: 0.3, delay: 0.24, stagger: 0.015, ease: "power2.out" },
        );
      } else {
        gsap.from(newBlocks, {
          autoAlpha: 0,
          scale: 0.985,
          duration: 0.5,
          stagger: 0.025,
          ease: "power3.out",
        });
      }
    }

    renderCrumbs(path);
    renderLegend(node);
    showReadout(node, path);
    backEl.hidden = path === "/";
    const desired = path === "/" ? "" : `#map=${encodeURIComponent(path)}`;
    if (location.hash !== desired) {
      history.replaceState(null, "", desired || `${location.pathname}${location.search}`);
    }

    if (options.focus !== false) {
      if (path !== "/") backEl.focus({ preventScroll: true });
      else canvasEl.focus({ preventScroll: true });
      announce(`Opened ${path}. ${children.length} entries.`);
    }
  }

  backEl.addEventListener("click", () => {
    if (currentPath === "/") return;
    const parent = `/${currentPath.split("/").filter(Boolean).slice(0, -1).join("/")}`;
    render(parent === "//" ? "/" : parent, { animate: !reduce });
  });

  let firstResize = true;
  const observer = new ResizeObserver(() => {
    if (firstResize) {
      firstResize = false;
      return;
    }
    resizeCall?.kill();
    resizeCall = gsap.delayedCall(0.08, () => render(currentPath, { animate: false, focus: false }));
  });
  observer.observe(canvasEl);

  const hash = new URLSearchParams(location.hash.replace(/^#/, "")).get("map");
  const start = hash && findByPath(hash) ? hash : "/";
  render(start, { animate: true, focus: false });
}

function initViews() {
  const tablist = document.querySelector<HTMLElement>('[role="tablist"]');
  if (!tablist) return;
  const tabs = [...tablist.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  const panels = [...document.querySelectorAll<HTMLElement>('[role="tabpanel"]')];
  if (tabs.length === 0) return;

  const select = (index: number, focus = false) => {
    const tab = tabs[index];
    if (!tab) return;
    for (const [i, item] of tabs.entries()) {
      const selected = i === index;
      item.setAttribute("aria-selected", String(selected));
      item.tabIndex = selected ? 0 : -1;
      const panel = panels.find((candidate) => candidate.id === item.getAttribute("aria-controls"));
      if (panel) panel.hidden = !selected;
    }
    if (focus) tab.focus();
    const panel = panels.find((candidate) => candidate.id === tab.getAttribute("aria-controls"));
    if (panel && !reduce) {
      gsap.fromTo(
        panel,
        { autoAlpha: 0, y: 8 },
        { autoAlpha: 1, y: 0, duration: 0.35, ease: "power3.out" },
      );
    }
  };

  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => select(index));
    tab.addEventListener("keydown", (event) => {
      let next = -1;
      if (event.key === "ArrowDown" || event.key === "ArrowRight") next = (index + 1) % tabs.length;
      else if (event.key === "ArrowUp" || event.key === "ArrowLeft")
        next = (index - 1 + tabs.length) % tabs.length;
      else if (event.key === "Home") next = 0;
      else if (event.key === "End") next = tabs.length - 1;
      if (next >= 0) {
        event.preventDefault();
        select(next, true);
      }
    });
  });
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const area = document.createElement("textarea");
      area.value = text;
      area.setAttribute("readonly", "");
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      const copied = (document as Document & { execCommand: (command: string) => boolean }).execCommand("copy");
      area.remove();
      return copied;
    } catch {
      return false;
    }
  }
}

function initCopy() {
  const status = document.querySelector<HTMLElement>("[data-copy-status]");
  document.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((button) => {
    button.addEventListener("click", async () => {
      const text = button.dataset.copy ?? "";
      const label = button.querySelector<HTMLElement>("[data-copy-text]");
      const copied = await copyText(text);
      if (!copied) {
        if (label) label.textContent = "Select and copy";
        if (status) status.textContent = "Copy failed. Select the command and copy it manually.";
        return;
      }
      button.classList.add("is-copied");
      if (label) label.textContent = "Copied";
      if (status) status.textContent = "Copied to clipboard.";
      window.setTimeout(() => {
        button.classList.remove("is-copied");
        if (label) label.textContent = "Copy";
      }, 1600);
    });
  });
}

function initGlyph() {
  const glyph = document.querySelector<SVGSVGElement>("[data-glyph]");
  if (!glyph || reduce) return;
  const rects = [...glyph.querySelectorAll("rect")];
  const alt = JSON.parse(glyph.dataset.glyphAlt ?? "[]") as {
    x: number;
    y: number;
    w: number;
    h: number;
  }[];
  if (rects.length !== alt.length || rects.length === 0) return;

  const timeline = gsap.timeline({
    repeat: -1,
    yoyo: true,
    defaults: { duration: 4.5, ease: "sine.inOut" },
  });
  rects.forEach((rect, index) => {
    const target = alt[index];
    if (!target) return;
    timeline.to(
      rect,
      { attr: { x: target.x, y: target.y, width: target.w, height: target.h } },
      0,
    );
  });

  const brand = glyph.closest("a");
  if (!brand) return;
  brand.addEventListener("pointerenter", () => timeline.timeScale(2.5));
  brand.addEventListener("pointerleave", () => timeline.timeScale(1));
  brand.addEventListener("focus", () => timeline.timeScale(2.5));
  brand.addEventListener("blur", () => timeline.timeScale(1));
}

function initReveals() {
  if (reduce) return;

  document.querySelectorAll<HTMLElement>("[data-split]").forEach((element) => {
    SplitText.create(element, {
      type: "lines",
      mask: "lines",
      linesClass: "split-line",
      autoSplit: true,
      onSplit(self) {
        gsap.set(element, { visibility: "visible" });
        return gsap.from(self.lines, {
          yPercent: 110,
          rotate: 2,
          duration: 1.1,
          stagger: 0.08,
          ease: "power3.out",
          scrollTrigger: { trigger: element, start: "top 88%", once: true },
        });
      },
    });
  });

  document.querySelectorAll<HTMLElement>("[data-reveal]").forEach((element) => {
    gsap.set(element, { visibility: "visible" });
    gsap.from(element, {
      y: 28,
      autoAlpha: 0,
      duration: 1,
      ease: "power3.out",
      scrollTrigger: { trigger: element, start: "top 90%", once: true },
    });
  });
}

export function initLanding() {
  initScroll();
  initHeader();
  initAnchors();
  initPageTransition();
  initButtons();
  initMap();
  initViews();
  initCopy();
  initGlyph();
  initReveals();
  document.documentElement.classList.add("is-motion-ready");
  document.fonts?.ready.then(() => ScrollTrigger.refresh());
  window.addEventListener("load", () => ScrollTrigger.refresh(), { once: true });
}
