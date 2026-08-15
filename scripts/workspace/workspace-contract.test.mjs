import { access, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

import { describe, expect, test } from "vitest";

import { classifyOwnedPath, WORKSPACE_AREAS } from "./workspace-map.mjs";
import { runNode, runNodeWithInput, withTemporaryDirectory } from "./test-support.mjs";

const repositoryRoot = resolve(import.meta.dirname, "../..");

async function readRepositoryFile(path) {
  return readFile(resolve(repositoryRoot, path), "utf8");
}

describe("workspace foundation", () => {
  test("declares one pnpm workspace with backend and contract members", async () => {
    const workspace = await readRepositoryFile("pnpm-workspace.yaml");

    expect(workspace).toContain('"apps/*"');
    expect(workspace).toContain('"contracts"');
    expect(workspace).toContain("disallowWorkspaceCycles: true");
  });

  test("promotes all existing Rust packages to one root workspace", async () => {
    const rootManifest = await readRepositoryFile("Cargo.toml");
    const mobileManifest = await readRepositoryFile("src-tauri/Cargo.toml");

    expect(rootManifest).toContain('"src-tauri"');
    expect(rootManifest).toContain('"src-tauri/crates/recorder-core"');
    expect(rootManifest).toContain('"src-tauri/crates/transcription-core"');
    expect(rootManifest).toContain('"src-tauri/plugins/recorder"');
    expect(rootManifest).toContain('resolver = "3"');
    expect(mobileManifest).not.toContain("[workspace]");
    expect(mobileManifest).not.toContain("[patch.crates-io]");
  });

  test("assigns every known path to one documented owner", () => {
    expect(WORKSPACE_AREAS.map(({ id }) => id)).toEqual([
      "mobile",
      "backend",
      "contract",
      "shared",
    ]);
    expect(classifyOwnedPath("src/app/App.tsx")).toBe("mobile");
    expect(classifyOwnedPath("src-tauri/src/lib.rs")).toBe("mobile");
    expect(classifyOwnedPath("apps/backend/README.md")).toBe("backend");
    expect(classifyOwnedPath("contracts/transcription-api/v1/openapi.json")).toBe("contract");
    expect(classifyOwnedPath("scripts/workspace/select-scopes.mjs")).toBe("shared");
    expect(classifyOwnedPath("unexpected/new-root.txt")).toBe("unknown");
  });

  test("keeps the Apple project and uninitialized Android host state at src-tauri", async () => {
    await expect(
      access(resolve(repositoryRoot, "src-tauri/gen/apple/stt-voice-memo-app.xcodeproj")),
    ).resolves.toBeUndefined();
    await expect(
      access(resolve(repositoryRoot, "src-tauri/gen/android/settings.gradle")),
    ).rejects.toThrow();
    await expect(
      access(resolve(repositoryRoot, "src-tauri/gen/android/app/src/main/AndroidManifest.xml")),
    ).rejects.toThrow();

    const tauriConfig = JSON.parse(await readRepositoryFile("src-tauri/tauri.conf.json"));
    expect(tauriConfig.build.frontendDist).toBe("../dist");
    expect(tauriConfig.bundle.iOS.minimumSystemVersion).toBe("15.0");
    expect(tauriConfig.bundle.android.minSdkVersion).toBe(24);
  });
});

describe("user story 1: owned repository commands", () => {
  test("exposes exact scoped and full commands from the root", async () => {
    const packageJson = JSON.parse(await readRepositoryFile("package.json"));
    const requiredScripts = [
      "dev:mobile",
      "build:mobile",
      "test:mobile",
      "lint:mobile",
      "format:mobile",
      "dev:backend",
      "build:backend",
      "test:backend",
      "lint:backend",
      "format:backend",
      "build:contract",
      "test:contract",
      "lint:contract",
      "format:contract",
      "validate:mobile",
      "validate:backend",
      "validate:contract",
      "validate",
      "test:workspace",
    ];

    expect(Object.keys(packageJson.scripts)).toEqual(expect.arrayContaining(requiredScripts));
    expect(packageJson.scripts.tauri).toBe("tauri");
  });

  test("keeps backend development explicitly unavailable", async () => {
    const backendPackage = JSON.parse(await readRepositoryFile("apps/backend/package.json"));

    expect(backendPackage.scripts.dev).toContain("workspace-unavailable.mjs");
    expect(backendPackage.scripts.dev).toContain("backend");
  });

  test("rejects cross-runtime imports and duplicate contract sources", async () => {
    const result = await runNode("scripts/workspace/check-boundaries.mjs", [
      "--root",
      "scripts/workspace/fixtures/boundaries",
    ]);

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("BOUNDARY_MOBILE_TO_BACKEND");
    expect(result.stderr).toContain("BOUNDARY_BACKEND_TO_MOBILE");
    expect(result.stderr).toContain("DUPLICATE_CANONICAL_CONTRACT");
    expect(result.stderr).not.toContain("runtime.js");
  });
});

describe("user story 2: canonical contract and secret boundary", () => {
  test("generates deterministic contract metadata and detects manual drift", async () => {
    await withTemporaryDirectory("stt-contract-", async (directory) => {
      const output = resolve(directory, "contract-manifest.json");
      const commonArguments = [
        "--source",
        "contracts/transcription-api/v1/openapi.json",
        "--output",
        output,
      ];

      const missing = await runNode("scripts/workspace/contract-artifacts.mjs", [
        "--check",
        ...commonArguments,
      ]);
      expect(missing.exitCode).toBe(1);
      expect(missing.stderr).toContain("CONTRACT_ARTIFACT_DRIFT");

      const generated = await runNode("scripts/workspace/contract-artifacts.mjs", [
        "--write",
        ...commonArguments,
      ]);
      expect(generated.exitCode).toBe(0);
      const first = await readFile(output, "utf8");

      const regenerated = await runNode("scripts/workspace/contract-artifacts.mjs", [
        "--write",
        ...commonArguments,
      ]);
      expect(regenerated.exitCode).toBe(0);
      expect(await readFile(output, "utf8")).toBe(first);

      await writeFile(output, `${first}\n`, "utf8");
      const drift = await runNode("scripts/workspace/contract-artifacts.mjs", [
        "--check",
        ...commonArguments,
      ]);
      expect(drift.exitCode).toBe(1);
      expect(drift.stderr).toContain("CONTRACT_ARTIFACT_DRIFT");
    });
  });

  test("accepts safe client output", async () => {
    const result = await runNode("scripts/workspace/check-client-secrets.mjs", [
      "--template",
      "scripts/workspace/fixtures/client-secrets/backend.env.example",
      "--scan-dir",
      "scripts/workspace/fixtures/client-secrets/safe",
    ]);

    expect(result).toMatchObject({ exitCode: 0 });
    expect(result.stdout).toContain("CLIENT_SECRET_SCAN_OK");
  });

  test("rejects backend names and caller canaries without echoing values", async () => {
    const canary = "stt-synthetic-canary-never-secret";
    const result = await runNode("scripts/workspace/check-client-secrets.mjs", [
      "--template",
      "scripts/workspace/fixtures/client-secrets/backend.env.example",
      "--scan-dir",
      "scripts/workspace/fixtures/client-secrets/leaking",
      "--canary",
      canary,
    ]);

    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("CLIENT_SECRET_NAME");
    expect(result.stderr).toContain("CLIENT_SECRET_CANARY");
    expect(result.stderr).not.toContain(canary);
    expect(result.stderr).not.toContain("redacted-placeholder");
  });

  test("rejects encoded, minified, and source-map canary representations", async () => {
    await withTemporaryDirectory("stt-transformed-canary-", async (directory) => {
      const canary = "stt-synthetic-canary-never-secret";
      await writeFile(
        resolve(directory, "encoded.js"),
        `globalThis.value = "${Buffer.from(canary).toString("base64")}";\n`,
        "utf8",
      );
      await writeFile(
        resolve(directory, "minified.js"),
        'globalThis.value="stt-synthetic-canary-"+"never-secret";\n',
        "utf8",
      );
      await writeFile(
        resolve(directory, "app.js.map"),
        JSON.stringify({ sourcesContent: [Buffer.from(canary).toString("hex")] }),
        "utf8",
      );

      const result = await runNode("scripts/workspace/check-client-secrets.mjs", [
        "--template",
        "scripts/workspace/fixtures/client-secrets/backend.env.example",
        "--scan-dir",
        directory,
        "--canary",
        canary,
      ]);

      expect(result.exitCode).toBe(1);
      expect(result.stderr).toContain("CLIENT_SECRET_CANARY count=3");
      expect(result.stderr).not.toContain(canary);
    });
  });

  test("wires a unique canary through an actual client build validation", async () => {
    const packageJson = JSON.parse(await readRepositoryFile("package.json"));
    const viteConfig = await readRepositoryFile("vite.config.ts");
    const validator = await readRepositoryFile(
      "scripts/workspace/verify-client-secret-boundary.mjs",
    );

    expect(packageJson.scripts["validate:mobile"]).toContain("pnpm verify:client-secret-boundary");
    expect(packageJson.scripts.validate).toContain("pnpm verify:client-secret-boundary");
    expect(viteConfig).toContain("STT_SYNTHETIC_CLIENT_CANARY");
    expect(validator).toContain('"--canary"');
    expect(validator).toContain("CLIENT_SECRET_BUILD_CANARY_DETECTED");
  });
});

describe("user story 3: affected validation and mobile preservation", () => {
  test.each([
    ["mobile source", ["src/app/App.tsx"], { backend: false, contract: false, mobile: true }],
    [
      "backend source",
      ["apps/backend/README.md"],
      { backend: true, contract: false, mobile: false },
    ],
    [
      "canonical contract",
      ["contracts/transcription-api/v1/openapi.json"],
      { backend: true, contract: true, mobile: true },
    ],
    ["root manifest", ["package.json"], { backend: true, contract: true, mobile: true }],
    [
      "mixed source",
      ["src/app/App.tsx", "apps/backend/README.md"],
      { backend: true, contract: false, mobile: true },
    ],
    ["unknown path", ["new-area/file.txt"], { backend: true, contract: true, mobile: true }],
    ["empty manual input", [], { backend: true, contract: true, mobile: true }],
  ])("classifies %s", async (_name, paths, expected) => {
    const result = await runNode("scripts/workspace/select-scopes.mjs", paths);

    expect(result.exitCode).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject(expected);
  });

  test("writes stable booleans for GitHub job outputs", async () => {
    await withTemporaryDirectory("stt-scopes-", async (directory) => {
      const output = resolve(directory, "github-output");
      const result = await runNode("scripts/workspace/select-scopes.mjs", [
        "--github-output",
        output,
        "contracts/transcription-api/v1/openapi.json",
      ]);

      expect(result.exitCode).toBe(0);
      expect(await readFile(output, "utf8")).toBe("mobile=true\nbackend=true\ncontract=true\n");
    });
  });

  test("reads null-delimited GitHub input without treating stdin as a filesystem path", async () => {
    const result = await runNodeWithInput(
      "scripts/workspace/select-scopes.mjs",
      ["--stdin0"],
      "src/app/App.tsx\0apps/backend/README.md\0",
    );

    expect(result.exitCode).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject({
      backend: true,
      contract: false,
      mobile: true,
    });
  });

  test("selects both previous and current owners for renamed files", async () => {
    const result = await runNodeWithInput(
      "scripts/workspace/select-scopes.mjs",
      ["--name-status0"],
      "R100\0src/app/Legacy.tsx\0apps/backend/current.ts\0",
    );

    expect(result.exitCode).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject({
      backend: true,
      contract: false,
      mobile: true,
    });
  });

  test("defines conditional scoped jobs, isolated caches, aggregate, and manual full validation", async () => {
    const workflow = await readRepositoryFile(".github/workflows/validate.yml");

    for (const job of ["changes:", "mobile:", "backend:", "contract:", "aggregate:", "full:"]) {
      expect(workflow).toContain(job);
    }
    expect(workflow).toContain("scripts/workspace/select-scopes.mjs");
    expect(workflow).toContain("git diff --name-status -z");
    expect(workflow).toContain("--name-status0");
    expect(workflow).toContain("CHANGES_RESULT: ${{ needs.changes.result }}");
    expect(workflow).toContain('if [[ "$CHANGES_RESULT" != "success" ]]');
    expect(workflow).toContain("needs.changes.outputs.mobile == 'true'");
    expect(workflow).toContain("needs.changes.outputs.backend == 'true'");
    expect(workflow).toContain("needs.changes.outputs.contract == 'true'");
    expect(workflow).toContain("scope-mobile");
    expect(workflow).toContain("scope-backend");
    expect(workflow).toContain("scope-contract");
    expect(workflow).toContain("workflow_dispatch:");
    expect(workflow).toContain("pnpm validate");
    expect(workflow).not.toMatch(/OPENAI_API_KEY|BACKEND_AUTH_SECRET/u);
  });

  test("mobile validation includes real boundary, native-host, and secret checks", async () => {
    const packageJson = JSON.parse(await readRepositoryFile("package.json"));
    const validation = packageJson.scripts["validate:mobile"];

    expect(validation).toContain("pnpm check:boundaries");
    expect(validation).toContain("check-mobile-paths.mjs");
    expect(validation).toContain("check-client-secrets.mjs");
  });

  test("clean-checkout Swift validation generates the ignored Tauri API first", async () => {
    const runner = await readRepositoryFile("scripts/workspace/run-swift-tests.mjs");

    expect(runner).toContain('"tauri-plugin-recorder"');
    expect(runner).toContain('"aarch64-apple-ios"');
    expect(runner).toContain('IPHONEOS_DEPLOYMENT_TARGET: "15.0"');
    expect(runner).toContain("prepareTauriApi");
  });
});
