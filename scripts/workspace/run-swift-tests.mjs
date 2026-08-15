import { execFile, spawn } from "node:child_process";
import { access, cp, rm } from "node:fs/promises";
import { dirname, relative, resolve, sep } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const repositoryRoot = resolve(import.meta.dirname, "../..");

async function prepareTauriApi() {
  const pluginTauriDirectory = resolve(repositoryRoot, "src-tauri/plugins/recorder/.tauri");
  const packageManifest = resolve(pluginTauriDirectory, "tauri-api/Package.swift");
  try {
    await access(packageManifest);
  } catch {
    const { stdout } = await execFileAsync(
      "cargo",
      ["metadata", "--format-version", "1", "--locked"],
      { cwd: repositoryRoot, maxBuffer: 16 * 1024 * 1024 },
    );
    const metadata = JSON.parse(stdout);
    const tauriPackage = metadata.packages.find((package_) => package_.name === "tauri");
    if (!tauriPackage) throw new Error("tauri package metadata missing");

    const source = resolve(dirname(tauriPackage.manifest_path), "mobile/ios-api");
    const destination = resolve(pluginTauriDirectory, "tauri-api");
    const excludedRoots = new Set([".build", "Package.resolved", "Tests"]);

    await rm(destination, { recursive: true, force: true });
    await cp(source, destination, {
      recursive: true,
      filter: (path) => {
        const firstComponent = relative(source, path).split(sep)[0];
        return firstComponent === "" || !excludedRoots.has(firstComponent);
      },
    });
    await access(packageManifest);
  }
}

async function selectSimulator() {
  const { stdout } = await execFileAsync("xcrun", [
    "simctl",
    "list",
    "devices",
    "available",
    "--json",
  ]);
  const payload = JSON.parse(stdout);
  const devices = Object.values(payload.devices).flat();
  const iPhones = devices.filter(
    (device) => device.isAvailable !== false && device.name.includes("iPhone"),
  );
  return iPhones.find((device) => device.state === "Booted") ?? iPhones[0];
}

async function main() {
  if (process.platform !== "darwin") {
    process.stderr.write("SWIFT_IOS_TESTS_UNAVAILABLE reason=macos-required\n");
    process.exitCode = 2;
    return;
  }

  await prepareTauriApi();

  const simulator = await selectSimulator();
  if (!simulator) {
    process.stderr.write("SWIFT_IOS_TESTS_UNAVAILABLE reason=iphone-simulator-required\n");
    process.exitCode = 2;
    return;
  }

  const child = spawn(
    "xcodebuild",
    [
      "-scheme",
      "tauri-plugin-recorder",
      "-destination",
      `platform=iOS Simulator,id=${simulator.udid}`,
      "test",
      "CODE_SIGNING_ALLOWED=NO",
    ],
    {
      cwd: resolve(repositoryRoot, "src-tauri/plugins/recorder/ios"),
      stdio: "inherit",
    },
  );

  child.on("error", () => {
    process.stderr.write("SWIFT_IOS_TESTS_FAILED reason=launch\n");
    process.exitCode = 1;
  });
  child.on("exit", (code) => {
    process.exitCode = code ?? 1;
  });
}

await main().catch(() => {
  process.stderr.write("SWIFT_IOS_TESTS_FAILED reason=discovery\n");
  process.exitCode = 1;
});
