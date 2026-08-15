import { execFile } from "node:child_process";
import { access } from "node:fs/promises";
import { resolve } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const requiredNdk = "28.2.13676358";
const requiredRustTargets = [
  "aarch64-linux-android",
  "armv7-linux-androideabi",
  "i686-linux-android",
  "x86_64-linux-android",
];

function unavailable(component) {
  const error = new Error("ANDROID_TOOLCHAIN_UNAVAILABLE");
  error.component = component;
  throw error;
}

async function command(file, arguments_) {
  try {
    return await execFileAsync(file, arguments_, { encoding: "utf8" });
  } catch {
    unavailable(file);
  }
}

async function requirePath(path, component) {
  try {
    await access(path);
  } catch {
    unavailable(component);
  }
}

async function main() {
  const java = await command("java", ["-version"]);
  const javaOutput = `${java.stdout}${java.stderr}`;
  if (!/version\s+"17(?:\.|")/u.test(javaOutput)) unavailable("java-17");

  const androidHome = process.env.ANDROID_HOME;
  if (!androidHome) unavailable("ANDROID_HOME");
  await requirePath(resolve(androidHome, "platforms/android-36/android.jar"), "platform-36");
  await requirePath(resolve(androidHome, "build-tools/35.0.0/aapt2"), "build-tools-35");
  await requirePath(resolve(androidHome, "platform-tools/adb"), "platform-tools");
  await requirePath(
    resolve(androidHome, `ndk/${requiredNdk}/source.properties`),
    `ndk-${requiredNdk}`,
  );

  const rustup = await command("rustup", ["target", "list", "--installed"]);
  const installed = new Set(rustup.stdout.trim().split(/\s+/u));
  const missingTarget = requiredRustTargets.find((target) => !installed.has(target));
  if (missingTarget) unavailable(`rust-${missingTarget}`);

  process.stdout.write(
    `ANDROID_TOOLCHAIN_OK java=17 sdk=36 buildTools=35.0.0 ndk=${requiredNdk} rustTargets=4\n`,
  );
}

await main().catch((error) => {
  const code =
    error.message === "ANDROID_TOOLCHAIN_UNAVAILABLE" ? error.message : "ANDROID_TOOLCHAIN_INVALID";
  const component = typeof error.component === "string" ? ` component=${error.component}` : "";
  process.stderr.write(`${code}${component}\n`);
  process.exitCode = 1;
});
