import { copyFile, mkdir } from "node:fs/promises";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const siteRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const repositoryRoot = path.resolve(siteRoot, "..");
const manifest = path.join(siteRoot, "wasm", "Cargo.toml");
const targetDirectory = path.join(repositoryRoot, "target", "web-wasm");

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: siteRoot,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} exited with ${code}`));
    });
  });
}

await run("rustup", ["target", "add", "wasm32-unknown-unknown"]);
await run("cargo", [
  "build",
  "--manifest-path",
  manifest,
  "--target",
  "wasm32-unknown-unknown",
  "--target-dir",
  targetDirectory,
  "--release",
]);

const source = path.join(
  siteRoot,
  "..",
  "target",
  "web-wasm",
  "wasm32-unknown-unknown",
  "release",
  "dotman_web_state.wasm",
);
const destination = path.join(siteRoot, "public", "dotman-web-state.wasm");
await mkdir(path.dirname(destination), { recursive: true });
await copyFile(source, destination);
