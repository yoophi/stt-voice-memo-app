import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, isAbsolute, resolve } from "node:path";
import { parseArgs, promisify } from "node:util";

import { XMLParser } from "fast-xml-parser";

import { collectFiles, repositoryRoot } from "./repository-files.mjs";

const execFileAsync = promisify(execFile);
const {
  values: {
    apk,
    canary = [],
    manifest: manifestArgument,
    payloadDir,
    template = "apps/backend/.env.example",
    variant = "debug",
  },
} = parseArgs({
  options: {
    apk: { type: "string" },
    canary: { multiple: true, type: "string" },
    manifest: { type: "string" },
    payloadDir: { type: "string" },
    template: { type: "string" },
    variant: { default: "debug", type: "string" },
  },
});

function asArray(value) {
  if (value === undefined) return [];
  return Array.isArray(value) ? value : [value];
}

function fail(code) {
  throw new Error(code);
}

function absolutePath(path) {
  return isAbsolute(path) ? path : resolve(repositoryRoot, path);
}

function exactNames(entries) {
  return asArray(entries).map((entry) => entry?.["@_android:name"]);
}

function assertExactAttributes(entry, expected, code) {
  const actual = Object.fromEntries(
    Object.entries(entry ?? {}).filter(([key]) => key.startsWith("@_")),
  );
  if (JSON.stringify(actual) !== JSON.stringify(expected)) fail(code);
}

function assertExactSet(actual, expected, code) {
  if (
    actual.length !== expected.length ||
    [...actual].sort().some((value, index) => value !== [...expected].sort()[index])
  ) {
    fail(code);
  }
}

function assertExactIntentFilter(filter, expectedActions, expectedCategories, code) {
  if (!filter) fail(code);
  assertExactSet(exactNames(filter.action), expectedActions, code);
  assertExactSet(exactNames(filter.category), expectedCategories, code);
}

async function analyze(arguments_) {
  try {
    const androidHome = process.env.ANDROID_HOME;
    const options = androidHome
      ? `-Dcom.android.sdklib.toolsdir=${resolve(androidHome, "cmdline-tools/latest")}`
      : undefined;
    return (
      await execFileAsync("apkanalyzer", arguments_, {
        encoding: "utf8",
        env: { ...process.env, APKANALYZER_OPTS: options },
      })
    ).stdout.trim();
  } catch {
    fail("ANDROID_APK_ANALYZER_UNAVAILABLE");
  }
}

async function loadInput(applicationId) {
  if (manifestArgument) {
    return {
      manifest: await readFile(resolve(manifestArgument), "utf8"),
      name: basename(manifestArgument),
    };
  }
  if (!apk) fail("ANDROID_APK_INPUT_REQUIRED");
  const analyzedApplicationId = await analyze(["manifest", "application-id", resolve(apk)]);
  const minSdk = await analyze(["manifest", "min-sdk", resolve(apk)]);
  if (analyzedApplicationId !== applicationId || minSdk !== "24") {
    fail("ANDROID_APK_IDENTITY_INVALID");
  }
  return {
    manifest: await analyze(["manifest", "print", resolve(apk)]),
    name: basename(apk),
  };
}

function validatePermissions(root, applicationId) {
  const expected = `${applicationId}.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`;
  const permissions = asArray(root["uses-permission"]);
  const declaredPermissions = asArray(root.permission);
  if (
    permissions.length !== 1 ||
    permissions[0]?.["@_android:name"] !== expected ||
    declaredPermissions.length !== 1 ||
    declaredPermissions[0]?.["@_android:name"] !== expected ||
    declaredPermissions[0]?.["@_android:protectionLevel"] !== "0x2"
  ) {
    fail("ANDROID_APK_PERMISSION_INVALID");
  }
  assertExactAttributes(
    permissions[0],
    { "@_android:name": expected },
    "ANDROID_APK_PERMISSION_INVALID",
  );
  assertExactAttributes(
    declaredPermissions[0],
    { "@_android:name": expected, "@_android:protectionLevel": "0x2" },
    "ANDROID_APK_PERMISSION_INVALID",
  );
}

function validateFeatures(root) {
  const features = asArray(root["uses-feature"]);
  if (
    features.length !== 1 ||
    features[0]?.["@_android:name"] !== "android.hardware.touchscreen" ||
    features[0]?.["@_android:required"] !== "true"
  ) {
    fail("ANDROID_APK_FEATURE_INVALID");
  }
  assertExactAttributes(
    features[0],
    {
      "@_android:name": "android.hardware.touchscreen",
      "@_android:required": "true",
    },
    "ANDROID_APK_FEATURE_INVALID",
  );
}

function validateLauncher(application) {
  const activities = asArray(application.activity);
  if (activities.length !== 1 || asArray(application["activity-alias"]).length !== 0) {
    fail("ANDROID_APK_ACTIVITY_INVALID");
  }
  const launcher = activities[0];
  if (
    launcher?.["@_android:name"] !== "com.yoophi.sttvoicememo.MainActivity" ||
    launcher?.["@_android:exported"] !== "true" ||
    launcher?.["@_android:launchMode"] !== "2"
  ) {
    fail("ANDROID_APK_LAUNCHER_INVALID");
  }
  assertExactAttributes(
    launcher,
    {
      "@_android:name": "com.yoophi.sttvoicememo.MainActivity",
      "@_android:exported": "true",
      "@_android:launchMode": "2",
      "@_android:configChanges": "0xfb4",
    },
    "ANDROID_APK_LAUNCHER_INVALID",
  );
  const filters = asArray(launcher["intent-filter"]);
  if (filters.length !== 1) fail("ANDROID_APK_LAUNCHER_INVALID");
  assertExactIntentFilter(
    filters[0],
    ["android.intent.action.MAIN"],
    ["android.intent.category.LAUNCHER"],
    "ANDROID_APK_LAUNCHER_INVALID",
  );
  assertExactAttributes(
    asArray(filters[0].action)[0],
    { "@_android:name": "android.intent.action.MAIN" },
    "ANDROID_APK_LAUNCHER_INVALID",
  );
  assertExactAttributes(
    asArray(filters[0].category)[0],
    { "@_android:name": "android.intent.category.LAUNCHER" },
    "ANDROID_APK_LAUNCHER_INVALID",
  );
}

function validateProvider(application, applicationId) {
  const providers = asArray(application.provider);
  if (providers.length !== 1) fail("ANDROID_APK_PROVIDER_INVALID");
  const provider = providers[0];
  if (
    provider?.["@_android:name"] !== "androidx.startup.InitializationProvider" ||
    provider?.["@_android:exported"] !== "false" ||
    provider?.["@_android:authorities"] !== `${applicationId}.androidx-startup`
  ) {
    fail("ANDROID_APK_PROVIDER_INVALID");
  }
  assertExactAttributes(
    provider,
    {
      "@_android:name": "androidx.startup.InitializationProvider",
      "@_android:exported": "false",
      "@_android:authorities": `${applicationId}.androidx-startup`,
    },
    "ANDROID_APK_PROVIDER_INVALID",
  );
  const metadata = asArray(provider["meta-data"]);
  const expectedNames = [
    "androidx.emoji2.text.EmojiCompatInitializer",
    "androidx.lifecycle.ProcessLifecycleInitializer",
    "androidx.profileinstaller.ProfileInstallerInitializer",
  ];
  assertExactSet(exactNames(metadata), expectedNames, "ANDROID_APK_PROVIDER_INVALID");
  if (metadata.some((entry) => entry?.["@_android:value"] !== "androidx.startup")) {
    fail("ANDROID_APK_PROVIDER_INVALID");
  }
  for (const entry of metadata) {
    assertExactAttributes(
      entry,
      {
        "@_android:name": entry["@_android:name"],
        "@_android:value": "androidx.startup",
      },
      "ANDROID_APK_PROVIDER_INVALID",
    );
  }
}

function validateReceiver(application) {
  const receivers = asArray(application.receiver);
  if (receivers.length !== 1) fail("ANDROID_APK_RECEIVER_INVALID");
  const receiver = receivers[0];
  if (
    receiver?.["@_android:name"] !== "androidx.profileinstaller.ProfileInstallReceiver" ||
    receiver?.["@_android:permission"] !== "android.permission.DUMP" ||
    receiver?.["@_android:enabled"] !== "true" ||
    receiver?.["@_android:exported"] !== "true" ||
    receiver?.["@_android:directBootAware"] !== "false"
  ) {
    fail("ANDROID_APK_RECEIVER_INVALID");
  }
  assertExactAttributes(
    receiver,
    {
      "@_android:name": "androidx.profileinstaller.ProfileInstallReceiver",
      "@_android:permission": "android.permission.DUMP",
      "@_android:enabled": "true",
      "@_android:exported": "true",
      "@_android:directBootAware": "false",
    },
    "ANDROID_APK_RECEIVER_INVALID",
  );
  const filters = asArray(receiver["intent-filter"]);
  const actions = filters.map((filter) => {
    assertExactIntentFilter(filter, exactNames(filter.action), [], "ANDROID_APK_RECEIVER_INVALID");
    const names = exactNames(filter.action);
    if (names.length !== 1) fail("ANDROID_APK_RECEIVER_INVALID");
    assertExactAttributes(
      asArray(filter.action)[0],
      { "@_android:name": names[0] },
      "ANDROID_APK_RECEIVER_INVALID",
    );
    return names[0];
  });
  assertExactSet(
    actions,
    [
      "androidx.profileinstaller.action.BENCHMARK_OPERATION",
      "androidx.profileinstaller.action.INSTALL_PROFILE",
      "androidx.profileinstaller.action.SAVE_PROFILE",
      "androidx.profileinstaller.action.SKIP_FILE",
    ],
    "ANDROID_APK_RECEIVER_INVALID",
  );
}

function backendNames(contents) {
  return contents
    .split(/\r?\n/u)
    .map((line) => /^([A-Z][A-Z0-9_]*)=/u.exec(line.trim())?.[1])
    .filter(Boolean);
}

function encodedValues(value) {
  const bytes = Buffer.from(value, "utf8");
  const unicodeEscaped = [...value]
    .map((character) => `\\u${character.codePointAt(0).toString(16).padStart(4, "0")}`)
    .join("");
  return [
    value,
    bytes.toString("base64"),
    bytes.toString("base64url"),
    bytes.toString("hex"),
    encodeURIComponent(value),
    unicodeEscaped,
  ];
}

async function scanPayload() {
  if (!apk && !payloadDir) return;
  let extractedDirectory;
  let scanDirectory = payloadDir && resolve(payloadDir);
  try {
    if (apk) {
      extractedDirectory = await mkdtemp(resolve(tmpdir(), "stt-android-apk-"));
      try {
        await execFileAsync("unzip", ["-qq", resolve(apk), "-d", extractedDirectory]);
      } catch {
        fail("ANDROID_APK_PAYLOAD_UNAVAILABLE");
      }
      scanDirectory = extractedDirectory;
    }
    const names = backendNames(await readFile(absolutePath(template), "utf8"));
    const forbidden = [...names, ...canary.flatMap(encodedValues)].map((value) =>
      Buffer.from(value, "utf8"),
    );
    const files = await collectFiles([scanDirectory]);
    if (files.length === 0) fail("ANDROID_APK_PAYLOAD_UNAVAILABLE");
    for (const path of files) {
      const contents = await readFile(path);
      const text = contents.toString("utf8");
      const minified = text.replace(/["'`+\s]/gu, "");
      if (
        forbidden.some((value) => contents.includes(value)) ||
        canary.some((value) => minified.includes(value.replace(/\s/gu, "")))
      ) {
        fail("ANDROID_APK_SECRET_BOUNDARY_INVALID");
      }
    }
  } finally {
    if (extractedDirectory) await rm(extractedDirectory, { force: true, recursive: true });
  }
}

async function main() {
  if (!new Set(["debug", "release"]).has(variant)) fail("ANDROID_APK_VARIANT_INVALID");
  const applicationId =
    variant === "debug" ? "com.yoophi.sttvoicememo.debug" : "com.yoophi.sttvoicememo";
  const { manifest, name } = await loadInput(applicationId);
  const document = new XMLParser({ ignoreAttributes: false }).parse(manifest);
  const root = document?.manifest;
  if (!root) fail("ANDROID_APK_MANIFEST_INVALID");

  validatePermissions(root, applicationId);
  validateFeatures(root);
  if (asArray(root.instrumentation).length !== 0) fail("ANDROID_APK_COMPONENT_INVALID");

  const application = root.application;
  if (
    !application ||
    application["@_android:usesCleartextTraffic"] !== "false" ||
    asArray(application.service).length !== 0
  ) {
    fail("ANDROID_APK_APPLICATION_INVALID");
  }
  validateLauncher(application);
  validateProvider(application, applicationId);
  validateReceiver(application);
  await scanPayload();

  process.stdout.write(
    `ANDROID_APK_OK variant=${variant} artifact=${name} sensitivePermissions=0 launcher=verified payload=verified\n`,
  );
}

await main().catch((error) => {
  const code = /^ANDROID_APK_/u.test(error.message) ? error.message : "ANDROID_APK_CHECK_FAILED";
  process.stderr.write(`${code}\n`);
  process.exitCode = 1;
});
