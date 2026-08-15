import { spawn } from "node:child_process";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dirname, "../..");
const androidHome = process.env.ANDROID_HOME;
if (!androidHome) {
  process.stderr.write("ANDROID_TOOLCHAIN_UNAVAILABLE component=ANDROID_HOME\n");
  process.exitCode = 1;
} else {
  const ndkHome = resolve(androidHome, "ndk/28.2.13676358");
  let javaHome = process.env.JAVA_HOME;
  if (!javaHome && process.platform === "darwin") {
    try {
      javaHome = execFileSync("/usr/libexec/java_home", ["-v", "17"], {
        encoding: "utf8",
      }).trim();
    } catch {
      process.stderr.write("ANDROID_TOOLCHAIN_UNAVAILABLE component=JAVA_HOME_17\n");
      process.exit(1);
    }
  }
  const child = spawn(
    "pnpm",
    ["tauri", "android", "build", "--debug", "--apk", "--target", "aarch64", "--ci"],
    {
      cwd: repositoryRoot,
      env: { ...process.env, ANDROID_NDK: ndkHome, JAVA_HOME: javaHome, NDK_HOME: ndkHome },
      stdio: "inherit",
    },
  );
  child.on("error", () => {
    process.stderr.write("ANDROID_BUILD_FAILED\n");
    process.exitCode = 1;
  });
  child.on("exit", (code) => {
    process.exitCode = code ?? 1;
  });
}
