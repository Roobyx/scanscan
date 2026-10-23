export const site = {
  name: "scanscan",
  version: "1.15.6",
  tagline: "Where did the disk space go?",
  description:
    "Scan a filesystem once, then explore it as an interactive treemap in your browser. Read-only, Docker-aware, JSON-first, and built for servers.",
  github: "https://github.com/Roobyx/scanscan",
  issues: "https://github.com/Roobyx/scanscan/issues",
  zip: "https://github.com/Roobyx/scanscan/archive/refs/heads/main.zip",
  license: "GPL-3.0-or-later",
  docs: "/overview/",
};

export const nav = [
  { label: "Why", href: "#why" },
  { label: "How", href: "#how" },
  { label: "Views", href: "#views" },
  { label: "Compare", href: "#compare" },
  { label: "Docs", href: "/overview/" },
];

export const heroFacts = [
  { value: "O(1)", label: "folder sizes", note: "one column read" },
  { value: "O(depth)", label: "scan memory", note: "streams to disk" },
  { value: "0", label: "telemetry calls", note: "offline by default" },
];

export const steps = [
  {
    n: "01",
    title: "Scan",
    copy: "The scanner walks the tree in pre-order and streams node records to disk. Memory tracks depth, not the number of files. Hardlinked files are counted once.",
    artifact: "$ scanscan scan /var --json\n\npre-order nodes\nhardlink dedup\nsparse + mount flags\nO(depth) memory",
  },
  {
    n: "02",
    title: "Index",
    copy: "A snapshot is four flat files: a manifest, fixed-size node records, concatenated names, and a subtree-size column. Folder totals are one read, not a recursive query.",
    artifact: "snapshots/8f2c/\n  manifest.json\n  nodes.bin\n  names.bin\n  subtree.bin",
  },
  {
    n: "03",
    title: "Explore",
    copy: "The daemon serves depth-limited tiles over a Unix socket. The browser receives the rectangles it can draw, never fifty million rows. Every command and endpoint is JSON.",
    artifact: "browser\n  → REST / SSE\n  → daemon (UDS)\n  → mmap\n\nJSON in, JSON out",
  },
];

export const views = [
  {
    id: "treemap",
    name: "Treemap",
    blurb: "Area is allocated bytes. Click a block to drill in; breadcrumbs walk back out.",
  },
  {
    id: "sunburst",
    name: "Sunburst",
    blurb: "The same hierarchy on a radial axis. Depth reads at a glance; exact sizes do not.",
  },
  {
    id: "icicle",
    name: "Icicle",
    blurb: "One row per depth level, so a path reads as a straight line from root to file.",
  },
  {
    id: "bubble",
    name: "Bubble",
    blurb: "Packed circles for the biggest entries. A fast read of what dominates the tree.",
  },
  {
    id: "bars",
    name: "Bars",
    blurb: "The top 25 entries, ranked. Switch between allocated and apparent size.",
  },
  {
    id: "docker",
    name: "Docker",
    blurb: "Container, image and volume sizes, with live CPU and memory from a GET-only socket.",
  },
];

export const containers = [
  {
    name: "postgres",
    image: "postgres:16",
    size: "42.6 GB",
    pct: 100,
    mounts: "pgdata → /var/lib/postgresql/data",
    cpu: "2.4%",
    mem: "412 MB",
    memPct: 34,
  },
  {
    name: "api",
    image: "app:1.4",
    size: "18.2 GB",
    pct: 43,
    mounts: "uploads → /app/uploads",
    cpu: "8.1%",
    mem: "1.2 GB",
    memPct: 78,
  },
  {
    name: "worker",
    image: "app:1.4",
    size: "9.4 GB",
    pct: 22,
    mounts: "none",
    cpu: "1.2%",
    mem: "640 MB",
    memPct: 46,
  },
  {
    name: "redis",
    image: "redis:7",
    size: "3.1 GB",
    pct: 7,
    mounts: "redisdata → /data",
    cpu: "0.4%",
    mem: "96 MB",
    memPct: 12,
  },
];

export const trust = [
  {
    title: "Read-only",
    copy: "No delete, move or edit. The only writes go to its own data directory.",
  },
  {
    title: "No telemetry",
    copy: "No external calls at runtime. It works with the network unplugged.",
  },
  {
    title: "GET-only Docker",
    copy: "The app talks to a socket proxy that allows GET. It never gets write access.",
  },
  {
    title: "Validated paths",
    copy: "API paths are checked against the indexed roots. Traversal outside them is rejected.",
  },
];

export const faq = [
  {
    q: "Does scanscan change anything on the disk?",
    a: "No. It never deletes, moves or edits scanned files, and v1 exposes no destructive operation. Snapshots are written to its own data directory and nowhere else.",
  },
  {
    q: "How much memory does a scan use?",
    a: "The scanner streams nodes to disk with state proportional to depth, not file count. The design budget is under 512 MB at 50 million files, and under 1 GB for the full stack. Measure your own tree before a 50M-file run.",
  },
  {
    q: "Do I need root?",
    a: "For a full filesystem scan, yes, or CAP_DAC_READ_SEARCH. Mounting a narrower path read-only works without broad access.",
  },
  {
    q: "What happens with rootless Docker or Podman?",
    a: "The Docker views report unavailable. The scanner, the index and every other view keep working. Docker is optional.",
  },
  {
    q: "Is there authentication?",
    a: "Not in v1. Bind it to localhost or put it behind a reverse proxy with TLS. Do not expose port 8080 directly to the internet.",
  },
  {
    q: "Which platforms are supported?",
    a: "Linux is the primary target and has the fast path. The portable scanner builds on macOS and Windows; the Linux path is what is tuned.",
  },
  {
    q: "How do I update?",
    a: "Run git pull, then docker compose up -d --build. Snapshots stay in the scanscan-data volume.",
  },
  {
    q: "Can I query it from scripts?",
    a: "Yes. Every CLI command emits JSON with --json, and the daemon exposes a versioned JSON-RPC socket. The REST API mirrors it for the web client.",
  },
];

export const install = {
  commands: [
    "git clone https://github.com/Roobyx/scanscan",
    "cd scanscan",
    "docker compose up -d --build",
  ],
  requirements: [
    { label: "OS", value: "Linux. The portable scanner builds on macOS and Windows." },
    { label: "Runtime", value: "Docker Engine with Compose v2." },
    { label: "Access", value: "Root or CAP_DAC_READ_SEARCH for a full filesystem scan." },
  ],
};
