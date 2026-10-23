const GB = 1024 ** 3;

export type SampleKind =
  | "containers"
  | "volumes"
  | "images"
  | "cache"
  | "logs"
  | "system"
  | "home"
  | "data"
  | "other";

export type SampleNode = {
  name: string;
  kind: SampleKind;
  size: number;
  items: number;
  children?: SampleNode[];
};

type Input = {
  name: string;
  kind: SampleKind;
  size?: number;
  items?: number;
  children?: Input[];
};

export const KIND_META: Record<SampleKind, { label: string; color: string }> = {
  containers: { label: "Containers", color: "#3ddc97" },
  volumes: { label: "Volumes", color: "#ffb454" },
  images: { label: "Images", color: "#b98cff" },
  cache: { label: "Cache", color: "#f78fb3" },
  logs: { label: "Logs", color: "#ff6b6b" },
  system: { label: "System", color: "#4f8cff" },
  home: { label: "Home", color: "#5ad1e6" },
  data: { label: "Data", color: "#9ccc65" },
  other: { label: "Other", color: "#8d99ae" },
};

const leaf = (name: string, kind: SampleKind, gb: number, items: number): Input => ({
  name,
  kind,
  size: gb * GB,
  items,
});

const input: Input[] = [
  {
    name: "var",
    kind: "system",
    children: [
      {
        name: "lib",
        kind: "system",
        children: [
          {
            name: "docker",
            kind: "containers",
            children: [
              leaf("overlay2", "containers", 148.6, 2_184_204),
              leaf("volumes", "volumes", 72.4, 412_806),
              leaf("containers", "containers", 24.1, 88_204),
              leaf("images", "images", 12.8, 1_204),
              leaf("buildkit", "cache", 4.9, 42_118),
            ],
          },
          leaf("postgres", "data", 9.4, 12_408),
          leaf("apt", "cache", 3.2, 8_204),
          leaf("systemd", "system", 1.8, 3_118),
          leaf("other", "other", 1.7, 6_204),
        ],
      },
      {
        name: "log",
        kind: "logs",
        children: [
          leaf("journal", "logs", 9.2, 184),
          leaf("nginx", "logs", 4.6, 1_204_806),
          leaf("docker", "logs", 3.1, 88_204),
          leaf("other", "logs", 1.5, 12_408),
        ],
      },
      leaf("cache", "cache", 9.8, 48_204),
      leaf("tmp", "other", 3.1, 8_204),
      leaf("spool", "system", 2.2, 1_204),
    ],
  },
  {
    name: "home",
    kind: "home",
    children: [
      {
        name: "ana",
        kind: "home",
        children: [
          leaf("projects", "home", 54.6, 1_204_806),
          leaf("downloads", "home", 18.4, 8_204),
          leaf("cache", "cache", 9.2, 412_806),
          leaf("other", "other", 6.0, 1_204),
        ],
      },
      {
        name: "deploy",
        kind: "data",
        children: [leaf("releases", "data", 41.2, 184_204), leaf("shared", "data", 12.9, 42_806)],
      },
      leaf("backups", "data", 46.1, 18_204),
    ],
  },
  {
    name: "srv",
    kind: "data",
    children: [
      leaf("media", "data", 38.4, 84_204),
      leaf("apps", "data", 14.2, 412_806),
      leaf("backups", "data", 8.6, 8_204),
    ],
  },
  {
    name: "usr",
    kind: "system",
    children: [
      leaf("lib", "system", 26.4, 184_204),
      leaf("share", "system", 11.2, 412_806),
      leaf("bin", "system", 4.6, 18_204),
      leaf("local", "system", 2.6, 42_806),
    ],
  },
  {
    name: "opt",
    kind: "data",
    children: [
      leaf("analytics", "data", 18.2, 88_204),
      leaf("tools", "data", 9.4, 42_118),
      leaf("other", "other", 5.0, 18_204),
    ],
  },
  {
    name: "data",
    kind: "data",
    children: [
      leaf("postgres", "data", 12.6, 42_806),
      leaf("redis", "data", 3.4, 8_204),
      leaf("other", "other", 2.4, 4_118),
    ],
  },
  leaf("other", "other", 12.8, 22_322),
];

function finalize(node: Input, parentPath: string): SampleNode {
  const path = parentPath === "/" ? `/${node.name}` : `${parentPath}/${node.name}`;
  if (node.children) {
    const children = node.children.map((child) => finalize(child, path));
    return {
      name: node.name,
      kind: node.kind,
      size: children.reduce((sum, child) => sum + child.size, 0),
      items: children.reduce((sum, child) => sum + child.items, 0),
      children,
    };
  }
  return {
    name: node.name,
    kind: node.kind,
    size: node.size ?? 0,
    items: node.items ?? 0,
  };
}

const children = input.map((node) => finalize(node, "/"));

export const SAMPLE_ROOT: SampleNode = {
  name: "/",
  kind: "system",
  size: children.reduce((sum, child) => sum + child.size, 0),
  items: children.reduce((sum, child) => sum + child.items, 0),
  children,
};

export function nodePath(node: SampleNode, root: SampleNode = SAMPLE_ROOT): string {
  if (node === root) return "/";
  const parts: string[] = [];
  let found = false;
  const walk = (current: SampleNode, trail: string[]): boolean => {
    if (current === node) {
      parts.push(...trail);
      found = true;
      return true;
    }
    for (const child of current.children ?? []) {
      if (walk(child, [...trail, child.name])) return true;
    }
    return false;
  };
  walk(root, []);
  return found ? `/${parts.join("/")}` : "/";
}

export function findByPath(path: string, root: SampleNode = SAMPLE_ROOT): SampleNode | null {
  if (path === "/") return root;
  const parts = path.split("/").filter(Boolean);
  let node = root;
  for (const part of parts) {
    const next = node.children?.find((child) => child.name === part);
    if (!next) return null;
    node = next;
  }
  return node;
}
