import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import { basename, resolve } from "node:path";
import { parseArgs, promisify } from "node:util";

import { XMLParser } from "fast-xml-parser";

const execFileAsync = promisify(execFile);
const {
  values: { apk, manifest: manifestArgument, variant = "debug" },
} = parseArgs({
  options: {
    apk: { type: "string" },
    manifest: { type: "string" },
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

async function loadInput() {
  if (manifestArgument) {
    return {
      manifest: await readFile(resolve(manifestArgument), "utf8"),
      name: basename(manifestArgument),
    };
  }
  if (!apk) fail("ANDROID_APK_INPUT_REQUIRED");
  const applicationId = await analyze(["manifest", "application-id", resolve(apk)]);
  const minSdk = await analyze(["manifest", "min-sdk", resolve(apk)]);
  if (
    applicationId !==
      (variant === "debug" ? "com.yoophi.sttvoicememo.debug" : "com.yoophi.sttvoicememo") ||
    minSdk !== "24"
  ) {
    fail("ANDROID_APK_IDENTITY_INVALID");
  }
  return {
    manifest: await analyze(["manifest", "print", resolve(apk)]),
    name: basename(apk),
  };
}

async function main() {
  const { manifest, name } = await loadInput();
  const document = new XMLParser({ ignoreAttributes: false }).parse(manifest);
  const root = document?.manifest;
  if (!root) fail("ANDROID_APK_MANIFEST_INVALID");
  const expectedDynamicReceiverPermission =
    `${variant === "debug" ? "com.yoophi.sttvoicememo.debug" : "com.yoophi.sttvoicememo"}` +
    ".DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION";
  const permissions = asArray(root["uses-permission"]).map((entry) => entry["@_android:name"]);
  const declaredPermissions = asArray(root.permission);
  if (
    permissions.length !== 1 ||
    permissions[0] !== expectedDynamicReceiverPermission ||
    declaredPermissions.length !== 1 ||
    declaredPermissions[0]?.["@_android:name"] !== expectedDynamicReceiverPermission ||
    declaredPermissions[0]?.["@_android:protectionLevel"] !== "0x2"
  ) {
    fail("ANDROID_APK_PERMISSION_INVALID");
  }

  const application = root.application;
  const activities = asArray(application?.activity);
  const launcher = activities.find(
    (activity) => activity["@_android:name"] === "com.yoophi.sttvoicememo.MainActivity",
  );
  if (!launcher || launcher["@_android:exported"] !== "true") {
    fail("ANDROID_APK_LAUNCHER_INVALID");
  }

  const serialized = JSON.stringify(root);
  if (
    /LEANBACK|FileProvider|RECORD_AUDIO|FOREGROUND_SERVICE|OPENAI_API_KEY|BACKEND_AUTH_SECRET/u.test(
      serialized,
    )
  ) {
    fail("ANDROID_APK_CAPABILITY_INVALID");
  }

  const providers = asArray(application.provider).map((entry) => entry["@_android:name"]);
  const receivers = asArray(application.receiver).map((entry) => entry["@_android:name"]);
  const services = asArray(application.service).map((entry) => entry["@_android:name"]);
  const allowedProviders = new Set(["androidx.startup.InitializationProvider"]);
  const allowedReceivers = new Set(["androidx.profileinstaller.ProfileInstallReceiver"]);
  if (providers.some((entry) => !allowedProviders.has(entry))) fail("ANDROID_APK_PROVIDER_INVALID");
  if (receivers.some((entry) => !allowedReceivers.has(entry))) fail("ANDROID_APK_RECEIVER_INVALID");
  if (services.length !== 0) fail("ANDROID_APK_SERVICE_INVALID");

  process.stdout.write(
    `ANDROID_APK_OK variant=${variant} artifact=${name} sensitivePermissions=0 launcher=verified\n`,
  );
}

await main().catch((error) => {
  const code = /^ANDROID_APK_/u.test(error.message) ? error.message : "ANDROID_APK_CHECK_FAILED";
  process.stderr.write(`${code}\n`);
  process.exitCode = 1;
});
