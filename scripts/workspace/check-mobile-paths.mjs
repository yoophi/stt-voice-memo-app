import { access, readFile } from "node:fs/promises";
import { resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dirname, "../..");

async function main() {
  const requiredPaths = [
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json",
    "src-tauri/gen/apple/stt-voice-memo-app.xcodeproj",
  ];
  for (const path of requiredPaths) await access(resolve(repositoryRoot, path));

  const config = JSON.parse(
    await readFile(resolve(repositoryRoot, "src-tauri/tauri.conf.json"), "utf8"),
  );
  if (
    config.build?.frontendDist !== "../dist" ||
    config.bundle?.iOS?.minimumSystemVersion !== "15.0" ||
    config.bundle?.android?.minSdkVersion !== 24
  ) {
    throw new Error("MOBILE_PATH_CONTRACT_INVALID");
  }

  const serialized = JSON.stringify(config);
  if (/OPENAI_API_KEY|BACKEND_AUTH_SECRET|BACKEND_DATABASE_URL/u.test(serialized)) {
    throw new Error("MOBILE_BACKEND_CONFIGURATION_EXPOSED");
  }

  const androidHostPaths = [
    "src-tauri/gen/android/settings.gradle",
    "src-tauri/gen/android/app/build.gradle.kts",
    "src-tauri/gen/android/app/src/main/AndroidManifest.xml",
  ];
  const androidHostState = await Promise.all(
    androidHostPaths.map((path) =>
      access(resolve(repositoryRoot, path)).then(
        () => true,
        () => false,
      ),
    ),
  );

  if (androidHostState.some(Boolean) && !androidHostState.every(Boolean)) {
    throw new Error("MOBILE_ANDROID_HOST_INCOMPLETE");
  }

  const android = androidHostState.every(Boolean) ? "present-unverified" : "unavailable";
  process.stdout.write(`MOBILE_PATHS_PARTIAL apple=verified android=${android} owner=issue-24\n`);
}

await main().catch((error) => {
  const code = /^MOBILE_/u.test(error.message) ? error.message : "MOBILE_PATH_CHECK_FAILED";
  process.stderr.write(`${code}\n`);
  process.exitCode = 1;
});
