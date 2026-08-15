import { readdir, stat } from "node:fs/promises";
import { relative, resolve, sep } from "node:path";

export const repositoryRoot = resolve(import.meta.dirname, "../..");

export const defaultIgnoredDirectories = Object.freeze(
  new Set([".git", "node_modules", "target", "coverage"]),
);

export function repositoryRelative(root, path) {
  return relative(root, path).split(sep).join("/");
}

export function extension(path) {
  const match = /\.[^./]+$/u.exec(path);
  return match?.[0] ?? "";
}

async function collectPath(path, options, files) {
  const metadata = await stat(path).catch(() => null);
  if (!metadata) return;
  if (!metadata.isDirectory()) {
    if (!options.exclude?.(path)) files.push(path);
    return;
  }

  const entries = await readdir(path, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isDirectory() && options.ignoredDirectories.has(entry.name)) continue;
    const child = resolve(path, entry.name);
    if (options.exclude?.(child)) continue;
    if (entry.isDirectory()) await collectPath(child, options, files);
    else files.push(child);
  }
}

export async function collectFiles(paths, options = {}) {
  const files = [];
  const resolvedOptions = {
    exclude: options.exclude,
    ignoredDirectories: options.ignoredDirectories ?? defaultIgnoredDirectories,
  };
  for (const path of paths) await collectPath(path, resolvedOptions, files);
  return files;
}
