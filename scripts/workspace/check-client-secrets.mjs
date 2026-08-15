import { readFile } from "node:fs/promises";
import { isAbsolute, resolve } from "node:path";

import { collectFiles, repositoryRoot } from "./repository-files.mjs";

function valuesFor(arguments_, name) {
  const values = [];
  for (let index = 0; index < arguments_.length; index += 1) {
    if (arguments_[index] === name) values.push(arguments_[index + 1]);
  }
  return values;
}

function absolutePath(path) {
  return isAbsolute(path) ? path : resolve(repositoryRoot, path);
}

function backendNames(template) {
  return template
    .split(/\r?\n/u)
    .map((line) => /^([A-Z][A-Z0-9_]*)=/u.exec(line.trim())?.[1])
    .filter(Boolean);
}

function encodedCanaries(canary) {
  const utf8 = Buffer.from(canary, "utf8");
  const unicodeEscaped = [...canary]
    .map((character) => `\\u${character.codePointAt(0).toString(16).padStart(4, "0")}`)
    .join("");
  return new Set([
    canary,
    utf8.toString("base64"),
    utf8.toString("base64url"),
    utf8.toString("hex"),
    encodeURIComponent(canary),
    unicodeEscaped,
  ]);
}

function containsCanary(contents, canary) {
  const text = contents.toString("utf8");
  if ([...encodedCanaries(canary)].some((value) => text.includes(value))) return true;

  const minified = text.replace(/["'`+\s]/gu, "");
  return minified.includes(canary.replace(/\s/gu, ""));
}

async function main() {
  const arguments_ = process.argv.slice(2);
  const templatePath = absolutePath(
    valuesFor(arguments_, "--template")[0] ?? "apps/backend/.env.example",
  );
  const requestedDirectories = valuesFor(arguments_, "--scan-dir");
  const scanDirectories = (
    requestedDirectories.length > 0
      ? requestedDirectories
      : ["src", "src-tauri/tauri.conf.json", "src-tauri/gen", "dist"]
  ).map(absolutePath);
  const canaries = valuesFor(arguments_, "--canary").filter((value) => value.length > 0);

  const template = await readFile(templatePath, "utf8");
  const names = backendNames(template);
  const files = [];
  for (const path of scanDirectories) {
    const statFiles = await collectFiles([path]);
    if (statFiles.length > 0) files.push(...statFiles);
    else {
      const readable = await readFile(path).then(
        () => true,
        () => false,
      );
      if (readable) files.push(path);
    }
  }

  let nameFindings = 0;
  let canaryFindings = 0;
  for (const path of files) {
    const contents = await readFile(path);
    for (const name of names) if (contents.includes(Buffer.from(name))) nameFindings += 1;
    for (const canary of canaries) if (containsCanary(contents, canary)) canaryFindings += 1;
  }

  if (nameFindings > 0) process.stderr.write(`CLIENT_SECRET_NAME count=${nameFindings}\n`);
  if (canaryFindings > 0) process.stderr.write(`CLIENT_SECRET_CANARY count=${canaryFindings}\n`);
  if (nameFindings > 0 || canaryFindings > 0) {
    process.exitCode = 1;
    return;
  }
  process.stdout.write("CLIENT_SECRET_SCAN_OK\n");
}

await main().catch(() => {
  process.stderr.write("CLIENT_SECRET_SCAN_ERROR\n");
  process.exitCode = 1;
});
