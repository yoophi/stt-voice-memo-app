import { execFile } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { promisify } from "node:util";
import { randomBytes } from "node:crypto";

import { repositoryRoot } from "./repository-files.mjs";

const execute = promisify(execFile);

async function main() {
  const outputDirectory = await mkdtemp(resolve(tmpdir(), "stt-client-canary-build-"));
  const canary = `stt-synthetic-${randomBytes(24).toString("hex")}`;

  try {
    await execute("pnpm", ["exec", "vite", "build", "--outDir", outputDirectory], {
      cwd: repositoryRoot,
      env: { ...process.env, STT_SYNTHETIC_CLIENT_CANARY: canary },
    });

    const scan = await execute(
      process.execPath,
      [
        "scripts/workspace/check-client-secrets.mjs",
        "--scan-dir",
        outputDirectory,
        "--canary",
        canary,
      ],
      { cwd: repositoryRoot },
    ).then(
      () => ({ exitCode: 0, stderr: "" }),
      (error) => ({ exitCode: error.code, stderr: String(error.stderr ?? "") }),
    );

    if (
      scan.exitCode === 0 ||
      !scan.stderr.includes("CLIENT_SECRET_CANARY") ||
      scan.stderr.includes(canary)
    ) {
      throw new Error("CLIENT_SECRET_BUILD_CANARY_NOT_DETECTED");
    }

    process.stdout.write("CLIENT_SECRET_BUILD_CANARY_DETECTED\n");
  } finally {
    await rm(outputDirectory, { recursive: true, force: true });
  }
}

await main().catch(() => {
  process.stderr.write("CLIENT_SECRET_BUILD_VALIDATION_FAILED\n");
  process.exitCode = 1;
});
