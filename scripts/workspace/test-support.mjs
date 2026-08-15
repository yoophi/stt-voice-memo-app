import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

export const repositoryRoot = resolve(import.meta.dirname, "../..");

export function repositoryPath(path) {
  return resolve(repositoryRoot, path);
}

export async function readRepositoryFile(path) {
  return readFile(repositoryPath(path), "utf8");
}

export async function withTemporaryDirectory(prefix, callback) {
  const directory = await mkdtemp(resolve(tmpdir(), prefix));
  try {
    return await callback(directory);
  } finally {
    await rm(directory, { force: true, recursive: true });
  }
}

export async function writeUtf8(path, contents) {
  await writeFile(path, contents, "utf8");
}

export async function runNode(script, arguments_ = [], options = {}) {
  try {
    const result = await execFileAsync(process.execPath, [repositoryPath(script), ...arguments_], {
      cwd: repositoryRoot,
      encoding: "utf8",
      ...options,
    });
    return { exitCode: 0, stderr: result.stderr, stdout: result.stdout };
  } catch (error) {
    return {
      exitCode: typeof error.code === "number" ? error.code : 1,
      stderr: error.stderr ?? "",
      stdout: error.stdout ?? "",
    };
  }
}
