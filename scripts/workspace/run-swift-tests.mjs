import { execFile, spawn } from "node:child_process";
import { resolve } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const repositoryRoot = resolve(import.meta.dirname, "../..");

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
