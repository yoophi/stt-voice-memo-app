import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  collectFiles,
  defaultIgnoredDirectories,
  extension,
  repositoryRelative,
  repositoryRoot,
} from "./repository-files.mjs";

const defaultRoot = repositoryRoot;
const ignoredDirectories = new Set([...defaultIgnoredDirectories, "dist"]);
const textExtensions = new Set([
  ".cjs",
  ".js",
  ".json",
  ".jsx",
  ".mjs",
  ".rs",
  ".toml",
  ".ts",
  ".tsx",
  ".yaml",
  ".yml",
]);

function parseRoot(arguments_) {
  const index = arguments_.indexOf("--root");
  return index === -1 ? defaultRoot : resolve(defaultRoot, arguments_[index + 1]);
}

async function readJson(path) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch {
    return null;
  }
}

export async function checkBoundaries(root = defaultRoot) {
  const fixtures = resolve(defaultRoot, "scripts/workspace/fixtures");
  const files = await collectFiles([root], {
    ignoredDirectories,
    exclude: (path) =>
      path.endsWith("/.wtp.yml") || (root === defaultRoot && path.startsWith(fixtures)),
  });
  const violations = [];
  const openApiSources = [];
  for (const path of files.filter((candidate) => candidate.endsWith("/openapi.json"))) {
    const ownedPath = repositoryRelative(root, path);
    if (ownedPath === "contracts/transcription-api/v1/openapi.json") {
      openApiSources.push(path);
      continue;
    }
    const document = await readJson(path);
    if (
      document?.info?.title === "STT Voice Memo Transcription API" ||
      Object.hasOwn(document?.paths ?? {}, "/v1/transcriptions")
    ) {
      openApiSources.push(path);
    }
  }

  if (openApiSources.length !== 1) {
    violations.push({ code: "DUPLICATE_CANONICAL_CONTRACT", count: openApiSources.length });
  }

  for (const path of files) {
    const ownedPath = repositoryRelative(root, path);
    if (!textExtensions.has(extension(path))) continue;

    const contents = await readFile(path, "utf8");
    if (
      (ownedPath.startsWith("src/") || ownedPath.startsWith("src-tauri/")) &&
      /(apps\/backend|@stt-voice-memo\/backend)/u.test(contents)
    ) {
      violations.push({ code: "BOUNDARY_MOBILE_TO_BACKEND", count: 1 });
    }

    if (
      ownedPath.startsWith("apps/backend/") &&
      /(src-tauri|@stt-voice-memo\/mobile|@tauri-apps|(?:^|["'])\.\.\/.*\/src\/)/mu.test(contents)
    ) {
      violations.push({ code: "BOUNDARY_BACKEND_TO_MOBILE", count: 1 });
    }
  }

  return violations;
}

async function main() {
  const root = parseRoot(process.argv.slice(2));
  const violations = await checkBoundaries(root);
  if (violations.length === 0) {
    process.stdout.write("WORKSPACE_BOUNDARIES_OK\n");
    return;
  }

  const counts = new Map();
  for (const { code, count } of violations) counts.set(code, (counts.get(code) ?? 0) + count);
  for (const [code, count] of [...counts].sort()) {
    process.stderr.write(`${code} count=${count}\n`);
  }
  process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
