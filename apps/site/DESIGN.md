# scanscan landing page: design record

## Brief

**Subject:** scanscan, a self-hosted, read-only disk-space analyzer and indexer.
**Audience:** Linux operators with large disks and Docker hosts; second, developers comparing disk tools.
**Job:** get a qualified visitor to run the compose stack or open the docs. The "download" is the
repository and the container build, because no binary release is published.
**Assets:** the product's real palette, favicon, index format, CLI, views and Docker integration.
No screenshots exist, so the hero map is a working demo of the product's rule, clearly labelled as a
sample snapshot.
**Language:** English. **Constraints:** static Astro + Starlight, offline-capable, no fake metrics.

Assumptions:

- No production domain is configured. `SITE_URL` sets canonical, OG and sitemap URLs; it falls back
  to `http://localhost:3300`.
- The sample filesystem and the container rows are illustrative and labelled "sample". They are not
  presented as a benchmark or a real customer.
- The install path is Docker compose from a clone. No published image tag or binary is promised.
- The docs index moves from `/` to `/overview/` so the landing page can own `/`.

## Concept

Three candidates:

| | A. The map of the disk | B. The ledger of blocks | C. The sealed index |
|---|---|---|---|
| Territory | The product's material: the treemap | The index format: pre-order columns | The security posture |
| Idea | The page is the map the product makes: chrome stays monochrome, colour is reserved for data | Every claim is an entry in a block ledger that resolves in O(1) | The site is a read-only vault; nothing can write |
| Signature | A live treemap of a sample snapshot that drills into the clicked block | A folder size resolving from one column read | A write path that visibly refuses |
| Scores O/F/Fe/P | 4 / 5 / 4 / 2 | 4 / 4 / 3 / 3 | 3 / 4 / 3 / 3 |

Pick A. It uses the product's own object, gives the page its layout, colour rule, motion verb and
signature, and is the cheapest to execute well. Steal from B for the snapshot-format diagram and from
C for the trust strip. C is too narrow to carry a whole page.

### Concept card

```
Idea:        The page is the map scanscan makes: a filesystem drawn as area, with
             monochrome chrome and colour reserved for data.
Metaphor:    A disk allocation map.
Signature:   The hero treemap is live. Click a block and the map re-lays out from the
             block you clicked; breadcrumbs walk back. Squarified layout + FLIP transforms.
Emotion:     Recognition ("that is the folder eating the disk"), then control.
Motion verb: Allocate. Blocks settle into their area, split and recombine. Nothing bounces.
Screenshot:  The hero at rest: the question in 100 px type on the left, a lit treemap of
             the sample snapshot on the right, /var/lib/docker as the biggest block,
             and the readout at 262.8 GB.
```

## Art direction

**Palette.** The product's palette, so the site and the app feel like one thing.

| Role | Value | Use |
|---|---|---|
| ground | `#0B0E14` | page |
| panel | `#121722` | cards, map chrome |
| panel-2 | `#1A2130` | raised rows |
| ink | `#E6EBF2` | text |
| muted | `#8A97AB` | secondary text |
| line | `rgb(230 235 242 / 0.12)` | rules |
| accent | `#4F8CFF` | CTAs, focus, active state |
| paper | `#E9EDF2` | the install band only |
| paper-ink | `#0B0E14` | text on paper |

Data colours (the product's ten categorical colours) appear **only** inside the treemap, the view
previews and the container bars. They never style chrome. That is the whole colour rule: colour is
data. Near-black plus blue is a common dev-tool palette, but here it is the product's actual UI
palette and the contrast between monochrome chrome and lit data is the point.

**Type.** Supreme (display, text) and Azeret Mono (data, labels, code), both self-hosted from
Fontshare. Display tracking `-0.045em`, leading 0.92 to 0.96. Body 17 px / 1.55, measure 60ch.
Mono 12 to 13 px with tabular figures for sizes and counts. Labels are sentence case; no tracked
uppercase eyebrows.

**Grid.** 12 columns, `minmax(0, 1fr)`, fluid gutter `clamp(1rem, 0.5rem + 2.2vw, 2.5rem)`, max
1440 px, outer margin = gutter. Section rhythm varies: hero airy, statement full-bleed poster, how
normal, views airy, compare tight, install airy paper band, trust tight, FAQ normal, footer full.

**Shape.** One radius: 3 px. Treemap blocks are square with 1 px gaps. Pills are 3 px rectangles, not
capsules. One-pixel rules carry structure.

**Motion.** One easing family: `power3.out` for entrances and UI, `expo.inOut` for the rare large
state change. 0.3 to 0.6 s for UI, 0.9 to 1.2 s for line reveals, 0.55 s for the treemap drill,
20 to 40 ms stagger. All scroll and interaction motion runs on `gsap.ticker`; Lenis is driven by the
same ticker on fine pointers. The hero's one-shot entrance is CSS keyframes so the H1 is never
hidden from the first paint; it is an entrance, not a clock.

**Idle.** Two motions carry the concept at rest: the wordmark glyph is a tiny treemap that
re-squarifies on a slow 4.5 s loop, and a low-opacity scan band sweeps the hero map every 11 s.
The sweep is the product's own action, paused when the map is off screen and removed under reduced
motion. It is ambient, not a claim that the sample data is live; the panel still says "Sample
snapshot".

## Composition plan

```
Header      [wordmark]        [Why How Views Compare Docs]   [GitHub] [Get started]
Hero        H1 + lede + CTAs + trust        |  live treemap panel
            (5 cols)                        |  (7 cols)
Statement   full-bleed: "Area is bytes. Nothing else."  (7 cols) + body (4 cols offset)
How         01 Scan   | copy            | artifact: scan output
            02 Index  | copy            | artifact: snapshot directory
            03 Explore| copy            | artifact: browser -> daemon -> mmap
Views       tab list (4 cols)           | preview panel (8 cols) + blurb
Compare     heading + lede (5 cols)     | table (7 cols, scanscan column lit)
Docker      sample container card (7)   | heading + copy (4, offset right)
Install     paper band: copy + requirements (6) | terminal card + download links (6)
Trust       four facts on a rule, then the no-auth note
FAQ         heading (4)                 | eight details rows (8)
Footer      "Point it at a disk." + command + link columns + colophon
404         "This path is not in the index." + a map with one empty block
```

Genericness test: a generic model would centre the hero over a gradient, add three feature cards and
a "Get started" button. This page instead splits the hero around a working treemap, makes colour a
data-only signal, replaces feature cards with a views tab panel and a comparison table, and puts the
conversion moment on a paper band with a copyable command. The filesystem-format diagram and the
honest security notes are specific to this product.

## Content rules

- No em or en dashes in visible copy. No banned filler.
- Every number is either from the repository or explicitly a design budget / sample.
- The sample snapshot and container rows are labelled as samples.
- One action, one name: "Get started" in the header, "Run it with Docker" in the hero, "Copy" on the
  command. The install section is the single conversion target.

## QA decisions

- The hero map is the signature and must work with keyboard, touch and reduced motion (blocks are
  buttons; no hover-only information).
- Reduced motion: no Lenis, no idle glyph, no scroll reveals, no FLIP; the map renders in place and
  still drills on click.
- The page is static; docs navigation is a normal page load. There is no client router between the
  landing page and Starlight, so no page-transition curtain is faked.
