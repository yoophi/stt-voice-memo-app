import { posix } from "node:path";

export const WORKSPACE_AREAS = Object.freeze([
  Object.freeze({
    id: "mobile",
    roots: Object.freeze([
      "src/",
      "src-tauri/",
      "index.html",
      "components.json",
      "vite.config.ts",
      "vitest.config.ts",
      "tsconfig.json",
      "tsconfig.app.json",
      "tsconfig.node.json",
      "eslint.config.js",
      "tests/device/backend-monorepo-workspace.md",
    ]),
  }),
  Object.freeze({ id: "backend", roots: Object.freeze(["apps/backend/"]) }),
  Object.freeze({ id: "contract", roots: Object.freeze(["contracts/"]) }),
  Object.freeze({
    id: "shared",
    roots: Object.freeze([
      "scripts/workspace/",
      ".github/workflows/",
      "package.json",
      "pnpm-workspace.yaml",
      "pnpm-lock.yaml",
      "Cargo.toml",
      "Cargo.lock",
      "AGENTS.md",
      ".gitignore",
      ".prettierignore",
      ".prettierrc.json",
    ]),
  }),
]);

function normalizeRepositoryPath(path) {
  return posix.normalize(String(path).replaceAll("\\", "/")).replace(/^\.\//, "");
}

function matchesRoot(path, root) {
  return root.endsWith("/") ? path.startsWith(root) : path === root;
}

export function classifyOwnedPath(path) {
  const normalized = normalizeRepositoryPath(path);
  const owner = WORKSPACE_AREAS.find(({ roots }) =>
    roots.some((root) => matchesRoot(normalized, root)),
  );

  return owner?.id ?? "unknown";
}

export function normalizeChangedPath(path) {
  return normalizeRepositoryPath(path);
}
