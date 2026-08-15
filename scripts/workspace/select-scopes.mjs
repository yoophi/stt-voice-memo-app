import { appendFile } from "node:fs/promises";
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

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(Buffer.from(chunk));
  return Buffer.concat(chunks).toString("utf8");
}

export function parseNameStatus(input) {
  const tokens = input.split("\0").filter(Boolean);
  const paths = [];
  for (let index = 0; index < tokens.length;) {
    const status = tokens[index++];
    const pathCount = /^[RC]/u.test(status) ? 2 : 1;
    for (let pathIndex = 0; pathIndex < pathCount; pathIndex += 1) {
      const path = tokens[index++];
      if (!path) throw new Error("INVALID_NAME_STATUS_INPUT");
      paths.push(path);
    }
  }
  return paths;
}

async function main() {
  const arguments_ = process.argv.slice(2);
  const githubOutputIndex = arguments_.indexOf("--github-output");
  const githubOutput =
    githubOutputIndex === -1 ? null : outputPath(arguments_[githubOutputIndex + 1]);
  if (githubOutputIndex !== -1) arguments_.splice(githubOutputIndex, 2);

  const stdinIndex = arguments_.indexOf("--stdin0");
  const nameStatusIndex = arguments_.indexOf("--name-status0");
  let paths = arguments_.filter((argument) => argument !== "--");
  if (nameStatusIndex !== -1) {
    paths = parseNameStatus(await readStdin());
  } else if (stdinIndex !== -1) {
    paths = (await readStdin()).split("\0").filter(Boolean);
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
