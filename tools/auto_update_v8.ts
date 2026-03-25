// V8 version to track. Update this when bumping to a new major V8 version.
const V8_VERSION = "14.7";

const V8_UPSTREAM = "https://chromium.googlesource.com/v8/v8.git";
const V8_FORK = "https://github.com/denoland/v8.git";
const UPSTREAM_LKGR = `${V8_VERSION}-lkgr`;
const DENOLAND_LKGR = `${V8_VERSION}-lkgr-denoland`;
const AUTOROLL_BRANCH = "autoroll";

async function run(
  cmd: string,
  args: string[],
  cwd?: string,
): Promise<Uint8Array> {
  console.log("$", cmd, ...args);
  const proc = new Deno.Command(cmd, { args, cwd, stdout: "piped" });
  const output = await proc.output();
  if (!output.success) {
    console.error(`Failed to run ${cmd} ${args.join(" ")}`);
    Deno.exit(1);
  }
  return output.stdout;
}

function extractVersion(path = "./v8/include/v8-version.h"): string {
  const MAJOR_PREFIX = "#define V8_MAJOR_VERSION ";
  const MINOR_PREFIX = "#define V8_MINOR_VERSION ";
  const BUILD_PREFIX = "#define V8_BUILD_NUMBER ";
  const PATCH_PREFIX = "#define V8_PATCH_LEVEL ";

  const versionDotH = Deno.readTextFileSync(path);
  const lines = versionDotH.split("\n");
  const major = parseInt(
    lines.find((s) => s.startsWith(MAJOR_PREFIX))!.substring(
      MAJOR_PREFIX.length,
    ),
  );
  const minor = parseInt(
    lines.find((s) => s.startsWith(MINOR_PREFIX))!.substring(
      MINOR_PREFIX.length,
    ),
  );
  const build = parseInt(
    lines.find((s) => s.startsWith(BUILD_PREFIX))!.substring(
      BUILD_PREFIX.length,
    ),
  );
  const patch = parseInt(
    lines.find((s) => s.startsWith(PATCH_PREFIX))!.substring(
      PATCH_PREFIX.length,
    ),
  );

  return `${major}.${minor}.${build}.${patch}`;
}

// Start from origin/main
await run("git", ["checkout", "origin/main"]);
await run("git", ["submodule", "update", "--init", "--recursive", "v8"]);

const currentVersion = extractVersion();
console.log(`Starting auto update. Currently on ${currentVersion}`);

// -- Step 1: Update the denoland/v8 fork --
// Ensure upstream remote exists in the v8 submodule
const remotes = new TextDecoder().decode(
  await run("git", ["remote"], "./v8"),
);
if (!remotes.split("\n").includes("upstream")) {
  await run("git", ["remote", "add", "upstream", V8_UPSTREAM], "./v8");
}
if (!remotes.split("\n").includes("denoland")) {
  await run("git", ["remote", "add", "denoland", V8_FORK], "./v8");
}

// Fetch upstream lkgr branch
await run("git", ["fetch", "upstream", UPSTREAM_LKGR], "./v8");

// Create the denoland branch from upstream
await run(
  "git",
  ["checkout", "-B", DENOLAND_LKGR, `upstream/${UPSTREAM_LKGR}`],
  "./v8",
);

// Apply patches
const patches = [...Deno.readDirSync("./patches")]
  .filter((e) => e.name.endsWith(".patch"))
  .map((e) => e.name)
  .sort();

for (const patch of patches) {
  const patchPath = `${Deno.cwd()}/patches/${patch}`;
  console.log(`Applying patch ${patch}`);
  await run("git", ["am", "-3", patchPath], "./v8");
}

// Check if version changed
const newVersion = extractVersion();
if (currentVersion === newVersion) {
  console.log(`No new version available. Staying on ${newVersion}`);
  Deno.exit(0);
}

console.log(`Updated to version ${newVersion}`);

// Push the patched branch to the denoland/v8 fork
console.log("Pushing patched V8 branch to denoland/v8 fork.");
await run(
  "git",
  ["push", "--force", "denoland", DENOLAND_LKGR],
  "./v8",
);

// Create and push a tag
const commit = new TextDecoder().decode(
  await run("git", ["rev-parse", "HEAD"], "./v8"),
).trim();
const tag = `${newVersion}-denoland-${commit.slice(0, 20)}`;
console.log(`Creating tag ${tag}`);
await run("git", ["tag", tag], "./v8");
await run("git", ["push", "denoland", tag], "./v8");

// -- Step 2: Update rusty_v8 --

// Update V8 dependencies
const depsOutput = await run("python", ["tools/update_deps.py"]);
const depNames = new TextDecoder().decode(depsOutput).split("\n").filter((
  x,
) => x.length > 0).at(-1)!.split(",");

// Update version in readme
let readme = Deno.readTextFileSync("README.md");
readme = readme.replace(
  /V8 Version: \S+/,
  `V8 Version: ${newVersion}`,
);
Deno.writeTextFileSync("README.md", readme);

// Stage the changes
await run("git", ["add", "v8", "README.md", ...depNames]);

// Commit the changes
await run("git", ["commit", "-m", `Rolling to V8 ${newVersion}`]);

// Push to the `denoland/rusty_v8#autoroll`
await run("git", ["push", "origin", `+HEAD:refs/heads/${AUTOROLL_BRANCH}`]);

// Fetch the remote branch so `gh` cli can find it
await run("git", ["fetch", "origin", AUTOROLL_BRANCH]);

const proc = new Deno.Command("gh", {
  args: ["pr", "view", AUTOROLL_BRANCH, "--json", "state"],
  stdout: "piped",
});
const output = await proc.output();
const isPrOpen = output.success
  ? JSON.parse(new TextDecoder().decode(output.stdout)).state === "OPEN"
  : false;

if (isPrOpen) {
  console.log("Already open PR. Editing existing PR.");
  await run("gh", [
    "pr",
    "edit",
    AUTOROLL_BRANCH,
    "--title",
    `Rolling to V8 ${newVersion}`,
  ]);
} else {
  console.log("No PR open. Creating a new PR.");
  await run("gh", [
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
