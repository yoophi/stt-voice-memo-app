import { appendFile, readFile } from "node:fs/promises";
import { isAbsolute, resolve } from "node:path";

import { classifyOwnedPath, normalizeChangedPath } from "./workspace-map.mjs";

const repositoryRoot = resolve(import.meta.dirname, "../..");

function outputPath(path) {
  return isAbsolute(path) ? path : resolve(repositoryRoot, path);
}

export function selectScopes(paths) {
  const selected = { backend: false, contract: false, mobile: false };
  const reasons = new Set();

  if (paths.length === 0) {
    selected.backend = true;
    selected.contract = true;
    selected.mobile = true;
    reasons.add("empty-input-full");
  }

  for (const rawPath of paths) {
    const path = normalizeChangedPath(rawPath);
    const owner = classifyOwnedPath(path);
    reasons.add(`${owner}-change`);

    if (owner === "mobile") selected.mobile = true;
    else if (owner === "backend") selected.backend = true;
    else {
      selected.backend = true;
      selected.contract = true;
      selected.mobile = true;
    }
  }

  return { ...selected, reasons: [...reasons].sort() };
}

async function main() {
  const arguments_ = process.argv.slice(2);
  const githubOutputIndex = arguments_.indexOf("--github-output");
  const githubOutput =
    githubOutputIndex === -1 ? null : outputPath(arguments_[githubOutputIndex + 1]);
  if (githubOutputIndex !== -1) arguments_.splice(githubOutputIndex, 2);

  const stdinIndex = arguments_.indexOf("--stdin0");
  let paths = arguments_.filter((argument) => argument !== "--");
  if (stdinIndex !== -1) {
    paths = (await readFile(0)).toString("utf8").split("\0").filter(Boolean);
  }

  const selection = selectScopes(paths);
  if (githubOutput) {
    await appendFile(
      githubOutput,
      `mobile=${selection.mobile}\nbackend=${selection.backend}\ncontract=${selection.contract}\n`,
      "utf8",
    );
  }
  process.stdout.write(`${JSON.stringify(selection)}\n`);
}

await main();
