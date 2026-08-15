import { readFile } from "node:fs/promises";

export function backendConfigurationNames(template) {
  return template
    .split(/\r?\n/u)
    .map((line) => /^([A-Z][A-Z0-9_]*)=/u.exec(line.trim())?.[1])
    .filter(Boolean);
}

export function encodedCanaryRepresentations(canary) {
  const bytes = Buffer.from(canary, "utf8");
  const unicodeEscaped = [...canary]
    .map((character) => `\\u${character.codePointAt(0).toString(16).padStart(4, "0")}`)
    .join("");
  return new Set([
    canary,
    bytes.toString("base64"),
    bytes.toString("base64url"),
    bytes.toString("hex"),
    encodeURIComponent(canary),
    unicodeEscaped,
  ]);
}

export function containsCanary(contents, canary) {
  const text = contents.toString("utf8");
  if ([...encodedCanaryRepresentations(canary)].some((value) => text.includes(value))) {
    return true;
  }
  const minified = text.replace(/["'`+\s]/gu, "");
  return minified.includes(canary.replace(/\s/gu, ""));
}

export async function scanContentSafeFiles(files, { canaries = [], names = [] } = {}) {
  let nameFindings = 0;
  let canaryFindings = 0;
  const encodedNames = names.map((name) => Buffer.from(name, "utf8"));
  for (const path of files) {
    const contents = await readFile(path);
    for (const name of encodedNames) if (contents.includes(name)) nameFindings += 1;
    for (const canary of canaries) if (containsCanary(contents, canary)) canaryFindings += 1;
  }
  return { canaryFindings, nameFindings };
}
