import { access, cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

import { describe, expect, test } from "vitest";

import { classifyOwnedPath, WORKSPACE_AREAS } from "./workspace-map.mjs";
import { runNode, runNodeWithInput, withTemporaryDirectory } from "./test-support.mjs";

const repositoryRoot = resolve(import.meta.dirname, "../..");

async function readRepositoryFile(path) {
  return readFile(resolve(repositoryRoot, path), "utf8");
}

async function createMobileFixture(directory) {
  await mkdir(resolve(directory, "src-tauri/gen"), { recursive: true });
  await cp(
    resolve(repositoryRoot, "src-tauri/Cargo.toml"),
    resolve(directory, "src-tauri/Cargo.toml"),
  );
  await cp(
    resolve(repositoryRoot, "src-tauri/tauri.conf.json"),
    resolve(directory, "src-tauri/tauri.conf.json"),
  );
  await cp(
    resolve(repositoryRoot, "src-tauri/gen/android"),
    resolve(directory, "src-tauri/gen/android"),
    {
      recursive: true,
    },
  );
  await mkdir(resolve(directory, "src-tauri/gen/apple/stt-voice-memo-app.xcodeproj"), {
    recursive: true,
  });
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

  test("reports the Apple and complete Android hosts as verified", async () => {
    await expect(
      access(resolve(repositoryRoot, "src-tauri/gen/apple/stt-voice-memo-app.xcodeproj")),
    ).resolves.toBeUndefined();
    const tauriConfig = JSON.parse(await readRepositoryFile("src-tauri/tauri.conf.json"));
    expect(tauriConfig.build.frontendDist).toBe("../dist");
    expect(tauriConfig.bundle.iOS.minimumSystemVersion).toBe("15.0");
    expect(tauriConfig.bundle.android.minSdkVersion).toBe(24);

    const result = await runNode("scripts/workspace/check-mobile-paths.mjs");
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("apple=verified");
    expect(result.stdout).toContain("android=verified");
  });

  test("fails closed for a partial Android host", async () => {
    await withTemporaryDirectory("stt-mobile-partial-", async (directory) => {
      await createMobileFixture(directory);
      await rm(resolve(directory, "src-tauri/gen/android/gradlew"));

      const result = await runNode("scripts/workspace/check-mobile-paths.mjs", [
        "--root",
        directory,
      ]);

      expect(result.exitCode).toBe(1);
      expect(result.stderr).toContain("ANDROID_HOST_INVALID_PARTIAL");
      expect(result.stderr).toContain("src-tauri/gen/android/gradlew");
    });
  });

  test.each([
    [
      "permission",
      '<uses-permission android:name="android.permission.RECORD_AUDIO" />',
      "ANDROID_CAPABILITY_PERMISSION",
    ],
    [
      "leanback feature",
      '<uses-feature android:name="android.software.leanback" android:required="false" />',
      "ANDROID_CAPABILITY_FEATURE",
    ],
    [
      "provider",
      '<provider android:name="androidx.core.content.FileProvider" />',
      "ANDROID_CAPABILITY_PROVIDER",
    ],
    ["service", '<service android:name=".UnexpectedService" />', "ANDROID_CAPABILITY_SERVICE"],
    ["receiver", '<receiver android:name=".UnexpectedReceiver" />', "ANDROID_CAPABILITY_RECEIVER"],
  ])("rejects an unowned Android %s", async (_name, addition, expectedCode) => {
    await withTemporaryDirectory("stt-mobile-capability-", async (directory) => {
      await createMobileFixture(directory);
      const manifestPath = resolve(
        directory,
        "src-tauri/gen/android/app/src/main/AndroidManifest.xml",
      );
      const manifest = await readFile(manifestPath, "utf8");
      const insertion = addition.startsWith("<uses-")
        ? manifest.replace("<application", `${addition}\n    <application`)
        : manifest.replace("</application>", `    ${addition}\n    </application>`);
      await writeFile(manifestPath, insertion, "utf8");

      const result = await runNode("scripts/workspace/check-mobile-paths.mjs", [
        "--root",
        directory,
      ]);

      expect(result.exitCode).toBe(1);
      expect(result.stderr).toContain(expectedCode);
      expect(result.stderr).not.toContain("android.permission.RECORD_AUDIO");
    });
  });

  test("rejects native activity lifecycle expansion", async () => {
    await withTemporaryDirectory("stt-mobile-activity-", async (directory) => {
      await createMobileFixture(directory);
      const activityPath = resolve(
        directory,
        "src-tauri/gen/android/app/src/main/java/com/yoophi/sttvoicememo/MainActivity.kt",
      );
      await writeFile(
        activityPath,
        "package com.yoophi.sttvoicememo\n\nclass MainActivity : TauriActivity() { fun record() = Unit }\n",
        "utf8",
      );

      const result = await runNode("scripts/workspace/check-mobile-paths.mjs", [
        "--root",
        directory,
      ]);

      expect(result.exitCode).toBe(1);
      expect(result.stderr).toContain("ANDROID_HOST_ACTIVITY_INVALID");
    });
  });

  test("reports an unavailable Android toolchain without echoing machine paths", async () => {
    const result = await runNode("scripts/workspace/check-android-toolchain.mjs", [], {
      env: { ...process.env, ANDROID_HOME: resolve(repositoryRoot, "missing-android-sdk") },
    });

    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("ANDROID_TOOLCHAIN_UNAVAILABLE");
    expect(result.stderr).toContain("component=platform-36");
    expect(result.stderr).not.toContain(repositoryRoot);
  });

  test("accepts only the reviewed merged Android runtime manifest", async () => {
    await withTemporaryDirectory("stt-merged-manifest-", async (directory) => {
      const manifestPath = resolve(directory, "AndroidManifest.xml");
      const payloadDirectory = resolve(directory, "payload");
      await mkdir(payloadDirectory);
      const dynamicPermission =
        "com.yoophi.sttvoicememo.debug.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION";
      const manifest = `<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
  <uses-feature android:name="android.hardware.touchscreen" android:required="true" />
  <permission android:name="${dynamicPermission}" android:protectionLevel="0x2" />
  <uses-permission android:name="${dynamicPermission}" />
  <application android:usesCleartextTraffic="false">
    <activity android:name="com.yoophi.sttvoicememo.MainActivity" android:exported="true" android:launchMode="2" android:configChanges="0xfb4">
      <intent-filter>
        <action android:name="android.intent.action.MAIN" />
        <category android:name="android.intent.category.LAUNCHER" />
      </intent-filter>
    </activity>
    <provider android:name="androidx.startup.InitializationProvider" android:exported="false" android:authorities="com.yoophi.sttvoicememo.debug.androidx-startup">
      <meta-data android:name="androidx.emoji2.text.EmojiCompatInitializer" android:value="androidx.startup" />
      <meta-data android:name="androidx.lifecycle.ProcessLifecycleInitializer" android:value="androidx.startup" />
      <meta-data android:name="androidx.profileinstaller.ProfileInstallerInitializer" android:value="androidx.startup" />
    </provider>
    <receiver android:name="androidx.profileinstaller.ProfileInstallReceiver" android:permission="android.permission.DUMP" android:enabled="true" android:exported="true" android:directBootAware="false">
      <intent-filter><action android:name="androidx.profileinstaller.action.INSTALL_PROFILE" /></intent-filter>
      <intent-filter><action android:name="androidx.profileinstaller.action.SKIP_FILE" /></intent-filter>
      <intent-filter><action android:name="androidx.profileinstaller.action.SAVE_PROFILE" /></intent-filter>
      <intent-filter><action android:name="androidx.profileinstaller.action.BENCHMARK_OPERATION" /></intent-filter>
    </receiver>
  </application>
</manifest>
`;
      await writeFile(manifestPath, manifest, "utf8");
      await writeFile(resolve(payloadDirectory, "safe.bin"), "foundation-shell", "utf8");

      const missingPayload = await runNode("scripts/workspace/check-android-apk.mjs", [
        "--manifest",
        manifestPath,
        "--variant",
        "debug",
      ]);
      expect(missingPayload.exitCode).toBe(1);
      expect(missingPayload.stderr).toContain("ANDROID_APK_PAYLOAD_REQUIRED");

      const accepted = await runNode("scripts/workspace/check-android-apk.mjs", [
        "--manifest",
        manifestPath,
        "--payloadDir",
        payloadDirectory,
        "--variant",
        "debug",
      ]);
      expect(accepted).toMatchObject({ exitCode: 0 });
      expect(accepted.stdout).toContain("sensitivePermissions=0");
      expect(accepted.stdout).toContain("payload=verified");

      const mutations = [
        [
          "permission",
          (value) =>
            value.replace(
              "<application",
              '<uses-permission android:name="android.permission.RECORD_AUDIO" />\n  <application',
            ),
          "ANDROID_APK_PERMISSION_INVALID",
        ],
        [
          "feature",
          (value) =>
            value.replace(
              "<application",
              '<uses-feature android:name="android.software.leanback" android:required="false" />\n  <application',
            ),
          "ANDROID_APK_FEATURE_INVALID",
        ],
        [
          "activity alias",
          (value) =>
            value.replace(
              "</application>",
              '<activity-alias android:name=".Alias" />\n  </application>',
            ),
          "ANDROID_APK_ACTIVITY_INVALID",
        ],
        [
          "launcher category",
          (value) =>
            value.replace(
              "android.intent.category.LAUNCHER",
              "android.intent.category.LEANBACK_LAUNCHER",
            ),
          "ANDROID_APK_LAUNCHER_INVALID",
        ],
        [
          "provider export",
          (value) =>
            value.replace(
              'android:name="androidx.startup.InitializationProvider" android:exported="false"',
              'android:name="androidx.startup.InitializationProvider" android:exported="true"',
            ),
          "ANDROID_APK_PROVIDER_INVALID",
        ],
        [
          "receiver permission",
          (value) => value.replace("android.permission.DUMP", "android.permission.RECORD_AUDIO"),
          "ANDROID_APK_RECEIVER_INVALID",
        ],
      ];
      for (const [, mutate, expectedCode] of mutations) {
        await writeFile(manifestPath, mutate(manifest), "utf8");
        const rejected = await runNode("scripts/workspace/check-android-apk.mjs", [
          "--manifest",
          manifestPath,
          "--payloadDir",
          payloadDirectory,
          "--variant",
          "debug",
        ]);
        expect(rejected.exitCode).toBe(1);
        expect(rejected.stderr).toContain(expectedCode);
        expect(rejected.stderr).not.toContain("RECORD_AUDIO");
      }

      await writeFile(manifestPath, manifest, "utf8");
      const canary = "stt-apk-payload-canary-never-secret";
      await mkdir(resolve(payloadDirectory, "node_modules"));
      await writeFile(resolve(payloadDirectory, "node_modules/native.so"), canary, "utf8");
      const leaked = await runNode("scripts/workspace/check-android-apk.mjs", [
        "--manifest",
        manifestPath,
        "--payloadDir",
        payloadDirectory,
        "--canary",
        canary,
        "--variant",
        "debug",
      ]);
      expect(leaked.exitCode).toBe(1);
      expect(leaked.stderr).toContain("ANDROID_APK_SECRET_BOUNDARY_INVALID");
      expect(leaked.stderr).not.toContain(canary);
    });
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
    expect(packageJson.scripts["build:android"]).toBe(
      "node scripts/workspace/run-android-build.mjs",
    );
    expect(packageJson.scripts["validate:android-host"]).toBe(
      "node scripts/workspace/check-android-toolchain.mjs && node scripts/workspace/check-mobile-paths.mjs",
    );
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

    expect(runner).toContain('"metadata", "--format-version", "1", "--locked"');
    expect(runner).toContain('"mobile/ios-api"');
    expect(runner).not.toContain('"check", "--package", "tauri-plugin-recorder"');
    expect(runner).toContain("prepareTauriApi");
  });
});
