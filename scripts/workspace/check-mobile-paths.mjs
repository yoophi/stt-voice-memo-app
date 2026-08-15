import { access, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { parseArgs } from "node:util";

import { XMLParser } from "fast-xml-parser";

const defaultRepositoryRoot = resolve(import.meta.dirname, "../..");
const {
  values: { root: rootArgument },
} = parseArgs({
  options: { root: { type: "string" } },
});
const repositoryRoot = resolve(rootArgument ?? defaultRepositoryRoot);

const androidHostRoot = "src-tauri/gen/android";
const manifestPath = `${androidHostRoot}/app/src/main/AndroidManifest.xml`;
const activityPath = `${androidHostRoot}/app/src/main/java/com/yoophi/sttvoicememo/MainActivity.kt`;

const requiredAndroidPaths = [
  `${androidHostRoot}/.gitignore`,
  `${androidHostRoot}/app/.gitignore`,
  `${androidHostRoot}/app/build.gradle.kts`,
  `${androidHostRoot}/app/proguard-rules.pro`,
  manifestPath,
  activityPath,
  `${androidHostRoot}/app/src/main/res/values/strings.xml`,
  `${androidHostRoot}/app/src/main/res/values/themes.xml`,
  `${androidHostRoot}/build.gradle.kts`,
  `${androidHostRoot}/buildSrc/build.gradle.kts`,
  `${androidHostRoot}/buildSrc/src/main/java/com/yoophi/sttvoicememo/kotlin/BuildTask.kt`,
  `${androidHostRoot}/buildSrc/src/main/java/com/yoophi/sttvoicememo/kotlin/RustPlugin.kt`,
  `${androidHostRoot}/gradle.properties`,
  `${androidHostRoot}/gradle/wrapper/gradle-wrapper.jar`,
  `${androidHostRoot}/gradle/wrapper/gradle-wrapper.properties`,
  `${androidHostRoot}/gradlew`,
  `${androidHostRoot}/gradlew.bat`,
  `${androidHostRoot}/settings.gradle`,
];

function asArray(value) {
  if (value === undefined) return [];
  return Array.isArray(value) ? value : [value];
}

function fail(code, path) {
  const error = new Error(code);
  error.path = path;
  throw error;
}

async function exists(path) {
  try {
    await access(resolve(repositoryRoot, path));
    return true;
  } catch {
    return false;
  }
}

async function validateSharedMobileConfig() {
  for (const path of [
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json",
    "src-tauri/gen/apple/stt-voice-memo-app.xcodeproj",
  ]) {
    if (!(await exists(path))) fail("MOBILE_PATH_REQUIRED", path);
  }

  const configPath = "src-tauri/tauri.conf.json";
  const config = JSON.parse(await readFile(resolve(repositoryRoot, configPath), "utf8"));
  if (
    config.identifier !== "com.yoophi.sttvoicememo" ||
    config.productName !== "STT Voice Memo" ||
    config.build?.frontendDist !== "../dist" ||
    config.bundle?.iOS?.minimumSystemVersion !== "15.0" ||
    config.bundle?.android?.minSdkVersion !== 24 ||
    config.bundle?.android?.versionCode !== undefined ||
    config.bundle?.android?.versionName !== undefined
  ) {
    fail("MOBILE_CONFIGURATION_INVALID", configPath);
  }

  const serialized = JSON.stringify(config);
  if (/OPENAI_API_KEY|BACKEND_AUTH_SECRET|BACKEND_DATABASE_URL/u.test(serialized)) {
    fail("MOBILE_BACKEND_CONFIGURATION_EXPOSED", configPath);
  }
}

async function classifyHostFiles() {
  const states = await Promise.all(requiredAndroidPaths.map(exists));
  const present = states.filter(Boolean).length;
  if (present === 0) fail("ANDROID_HOST_UNAVAILABLE", androidHostRoot);
  if (present !== states.length) {
    const missing = requiredAndroidPaths[states.findIndex((state) => !state)];
    fail("ANDROID_HOST_INVALID_PARTIAL", missing);
  }
}

async function validateGradle() {
  const path = `${androidHostRoot}/app/build.gradle.kts`;
  const source = await readFile(resolve(repositoryRoot, path), "utf8");
  const required = [
    /compileSdk\s*=\s*36/u,
    /namespace\s*=\s*"com\.yoophi\.sttvoicememo"/u,
    /applicationId\s*=\s*"com\.yoophi\.sttvoicememo"/u,
    /minSdk\s*=\s*24/u,
    /targetSdk\s*=\s*36/u,
    /applicationIdSuffix\s*=\s*"\.debug"/u,
    /rootDirRel\s*=\s*"\.\.\/\.\.\/\.\.\/"/u,
  ];
  if (!required.every((pattern) => pattern.test(source))) {
    fail("ANDROID_HOST_GRADLE_INVALID", path);
  }
}

async function validateActivity() {
  const source = await readFile(resolve(repositoryRoot, activityPath), "utf8");
  const normalized = source.trim().replaceAll(/\r\n/gu, "\n");
  const expected = [
    "package com.yoophi.sttvoicememo",
    "",
    "class MainActivity : TauriActivity()",
  ].join("\n");
  if (normalized !== expected) fail("ANDROID_HOST_ACTIVITY_INVALID", activityPath);
}

async function validateManifest() {
  const xml = await readFile(resolve(repositoryRoot, manifestPath), "utf8");
  const parser = new XMLParser({ ignoreAttributes: false, trimValues: true });
  let document;
  try {
    document = parser.parse(xml);
  } catch {
    fail("ANDROID_HOST_MANIFEST_MALFORMED", manifestPath);
  }
  const manifest = document?.manifest;
  if (!manifest) fail("ANDROID_HOST_MANIFEST_MALFORMED", manifestPath);

  if (asArray(manifest["uses-permission"]).length !== 0) {
    fail("ANDROID_CAPABILITY_PERMISSION", manifestPath);
  }
  const features = asArray(manifest["uses-feature"]);
  if (
    features.length !== 1 ||
    features[0]?.["@_android:name"] !== "android.hardware.touchscreen" ||
    features[0]?.["@_android:required"] !== "true"
  ) {
    fail("ANDROID_CAPABILITY_FEATURE", manifestPath);
  }

  const application = manifest.application;
  if (
    !application ||
    application["@_android:icon"] !== "@mipmap/ic_launcher" ||
    application["@_android:label"] !== "@string/app_name" ||
    application["@_android:theme"] !== "@style/Theme.stt_voice_memo_app" ||
    application["@_android:usesCleartextTraffic"] !== "false"
  ) {
    fail("ANDROID_CAPABILITY_APPLICATION", manifestPath);
  }
  for (const kind of ["provider", "service", "receiver", "activity-alias"]) {
    if (asArray(application[kind]).length !== 0) {
      fail(`ANDROID_CAPABILITY_${kind.toUpperCase().replaceAll("-", "_")}`, manifestPath);
    }
  }

  const activities = asArray(application.activity);
  if (activities.length !== 1) fail("ANDROID_CAPABILITY_ACTIVITY", manifestPath);
  const activity = activities[0];
  if (
    activity?.["@_android:name"] !== ".MainActivity" ||
    activity?.["@_android:exported"] !== "true" ||
    activity?.["@_android:launchMode"] !== "singleTask"
  ) {
    fail("ANDROID_CAPABILITY_ACTIVITY", manifestPath);
  }
  const filters = asArray(activity["intent-filter"]);
  const actions = filters
    .flatMap((filter) => asArray(filter.action))
    .map((entry) => entry["@_android:name"]);
  const categories = filters
    .flatMap((filter) => asArray(filter.category))
    .map((entry) => entry["@_android:name"]);
  if (
    filters.length !== 1 ||
    actions.length !== 1 ||
    actions[0] !== "android.intent.action.MAIN" ||
    categories.length !== 1 ||
    categories[0] !== "android.intent.category.LAUNCHER"
  ) {
    fail("ANDROID_CAPABILITY_LAUNCHER", manifestPath);
  }

  const forbidden =
    /LEANBACK|FileProvider|file_paths|RECORD_AUDIO|FOREGROUND_SERVICE|OPENAI_API_KEY|BACKEND_AUTH_SECRET/u;
  if (forbidden.test(xml)) fail("ANDROID_CAPABILITY_FORBIDDEN", manifestPath);
}

async function main() {
  await validateSharedMobileConfig();
  await classifyHostFiles();
  await validateGradle();
  await validateActivity();
  await validateManifest();
  process.stdout.write("MOBILE_PATHS_OK apple=verified android=verified\n");
}

await main().catch((error) => {
  const code = /^(ANDROID|MOBILE)_/u.test(error.message)
    ? error.message
    : "MOBILE_PATH_CHECK_FAILED";
  const path = typeof error.path === "string" ? ` path=${error.path}` : "";
  process.stderr.write(`${code}${path}\n`);
  process.exitCode = 1;
});
