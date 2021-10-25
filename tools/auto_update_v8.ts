const V8_TRACKING_BRANCH = "9.6-lkgr-denoland";
const AUTOROLL_BRANCH = "autoroll";

function extractVersion() {
  const MAJOR_PREFIX = "#define V8_MAJOR_VERSION ";
  const MINOR_PREFIX = "#define V8_MINOR_VERSION ";
  const BUILD_PREFIX = "#define V8_BUILD_NUMBER ";
  const PATCH_PREFIX = "#define V8_PATCH_LEVEL ";

  const versionDotH = Deno.readTextFileSync("./v8/include/v8-version.h");
  const lines = versionDotH.split("\n");
  const major = parseInt(lines.find((s) => s.startsWith(MAJOR_PREFIX))!
    .substring(MAJOR_PREFIX.length));
  const minor = parseInt(lines.find((s) => s.startsWith(MINOR_PREFIX))!
    .substring(MINOR_PREFIX.length));
  const build = parseInt(lines.find((s) => s.startsWith(BUILD_PREFIX))!
    .substring(BUILD_PREFIX.length));
  const patch = parseInt(lines.find((s) => s.startsWith(PATCH_PREFIX))!
    .substring(PATCH_PREFIX.length));

  return `${major}.${minor}.${build}.${patch}`;
}

await run(["git", "checkout", "origin/main"]);
await run(["git", "submodule", "update", "--init", "--recursive", "v8"]);

const currentVersion = extractVersion();
console.log(`Starting auto update. Currently on ${currentVersion}`);

async function run(cmd: string[], cwd?: string) {
  console.log("$", ...cmd);
  const proc = Deno.run({ cmd, cwd });
  const status = await proc.status();
  if (!status.success) {
    console.error(`Failed to run ${cmd.join(" ")}`);
    Deno.exit(1);
  }
}

// Update v8 submodule
await run(["git", "fetch", `origin`, V8_TRACKING_BRANCH], "./v8");
await run(["git", "checkout", `origin/${V8_TRACKING_BRANCH}`], "./v8");

const newVersion = extractVersion();
if (currentVersion == newVersion) {
  console.log(`No new version available. Staying on ${newVersion}`);
  Deno.exit(0);
}

console.log(`Updated to version ${newVersion}`);

// Update version in readme
let readme = Deno.readTextFileSync("README.md");
readme = readme.replace(
  `V8 Version: ${currentVersion}`,
  `V8 Version: ${newVersion}`,
);
Deno.writeTextFileSync("README.md", readme);

// Stage the changes
await run(["git", "add", "v8", "README.md"]);

// Commit the changes
await run(["git", "commit", "-m", `Rolling to V8 ${newVersion}`]);

// Push to the `denoland/rusty_v8#autoroll`
await run(["git", "push", "origin", `+HEAD:refs/heads/${AUTOROLL_BRANCH}`]);

// Fetch the remote branch so `gh` cli can find it
await run(["git", "fetch", "origin", AUTOROLL_BRANCH]);

const proc = Deno.run({
  cmd: ["gh", "pr", "view", AUTOROLL_BRANCH, "--json", "state"],
  stdout: "piped",
});
const status = await proc.status();
const isPrOpen = status.success
  ? JSON.parse(new TextDecoder().decode(await proc.output())).state === "OPEN"
  : false;

if (isPrOpen) {
  console.log("Already open PR. Editing existing PR.");
  await run([
    "gh",
    "pr",
    "edit",
    AUTOROLL_BRANCH,
    "--title",
    `Rolling to V8 ${newVersion}`,
  ]);
} else {
  console.log("No PR open. Creating a new PR.");
  await run([
    "gh",
    "pr",
    "create",
    "--title",
    `Rolling to V8 ${newVersion}`,
    "--body",
    "",
    "--head",
    `denoland:${AUTOROLL_BRANCH}`,
  ]);
}
