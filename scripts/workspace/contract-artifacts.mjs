import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, relative, resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dirname, "../..");
const defaultSource = "contracts/transcription-api/v1/openapi.json";
const defaultOutput = "contracts/transcription-api/v1/generated/contract-manifest.json";

function argumentValue(arguments_, name, fallback) {
  const index = arguments_.indexOf(name);
  return index === -1 ? fallback : arguments_[index + 1];
}

function absolutePath(path) {
  return isAbsolute(path) ? path : resolve(repositoryRoot, path);
}

function displayPath(path) {
  const value = relative(repositoryRoot, path).replaceAll("\\", "/");
  return value.startsWith("../") ? "external-test-output" : value;
}

export async function expectedContractArtifact(sourcePath) {
  const bytes = await readFile(sourcePath);
  const document = JSON.parse(bytes.toString("utf8"));
  const artifact = {
    formatVersion: 1,
    source: displayPath(sourcePath),
    openapi: document.openapi,
    apiVersion: document.info?.version,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };

  if (
    typeof artifact.openapi !== "string" ||
    typeof artifact.apiVersion !== "string" ||
    !/^[a-f0-9]{64}$/u.test(artifact.sha256)
  ) {
    throw new Error("CONTRACT_SOURCE_INVALID");
  }

  return `${JSON.stringify(artifact, null, 2)}\n`;
}

async function main() {
  const arguments_ = process.argv.slice(2);
  const mode = arguments_.includes("--write") ? "write" : "check";
  const source = absolutePath(argumentValue(arguments_, "--source", defaultSource));
  const output = absolutePath(argumentValue(arguments_, "--output", defaultOutput));

  try {
    const expected = await expectedContractArtifact(source);
    if (mode === "write") {
      await mkdir(dirname(output), { recursive: true });
      await writeFile(output, expected, "utf8");
      process.stdout.write("CONTRACT_ARTIFACT_WRITTEN\n");
      return;
    }

    const actual = await readFile(output, "utf8").catch(() => null);
    if (actual !== expected) {
      process.stderr.write("CONTRACT_ARTIFACT_DRIFT\n");
      process.exitCode = 1;
      return;
    }
    process.stdout.write("CONTRACT_ARTIFACT_CURRENT\n");
  } catch (error) {
    const code = error instanceof SyntaxError ? "CONTRACT_SOURCE_INVALID" : error.message;
    process.stderr.write(`${/^CONTRACT_/u.test(code) ? code : "CONTRACT_ARTIFACT_ERROR"}\n`);
    process.exitCode = 1;
  }
}

await main();
