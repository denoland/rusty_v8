const UPSTREAM_REPOSITORY = "denoland/rusty_v8";
const UPSTREAM_BRANCH = "main";
const V8_TRACKING_BRANCH = "15.2-lkgr-denoland";
const AUTOROLL_BRANCH = "autoroll";
const PUSH_REMOTE = "origin";
const UPDATER_PATH = "./tools/auto_update_v8.ts";

const decoder = new TextDecoder();
const updaterSource = Deno.readTextFileSync(UPDATER_PATH);

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

function parseGitHubRepository(remoteUrl: string): string | undefined {
  const normalized = remoteUrl.trim().replace(/\.git$/, "");
  return normalized.match(/github\.com[/:]([^/]+\/[^/]+)$/)?.[1];
}

async function getRemoteRepository(
  remote: string,
): Promise<string | undefined> {
  const remoteUrl = decoder.decode(
    await run("git", ["remote", "get-url", remote]),
  );
  return parseGitHubRepository(remoteUrl);
}

async function findUpstreamRemote(): Promise<string> {
  const remoteNames = decoder.decode(await run("git", ["remote"])).trim()
    .split("\n").filter((remote) => remote.length > 0);
  const candidates = [...new Set(["origin", "upstream", ...remoteNames])]
    .filter((remote) => remoteNames.includes(remote));

  for (const remote of candidates) {
    const repository = await getRemoteRepository(remote);
    if (repository?.toLowerCase() === UPSTREAM_REPOSITORY) {
      return remote;
    }
  }

  console.error(
    `No remote points to ${UPSTREAM_REPOSITORY}. ` +
      "Add the upstream repository as a remote and try again.",
  );
  Deno.exit(1);
}

const upstreamRemote = await findUpstreamRemote();
const pushRepository = await getRemoteRepository(PUSH_REMOTE);
if (pushRepository === undefined) {
  console.error(
    `Remote ${PUSH_REMOTE} does not point to a GitHub repository.`,
  );
  Deno.exit(1);
}
const pushOwner = pushRepository.split("/")[0];
const prHead = `${pushOwner}:${AUTOROLL_BRANCH}`;

await run("git", ["fetch", upstreamRemote, UPSTREAM_BRANCH]);
await run("git", ["checkout", `${upstreamRemote}/${UPSTREAM_BRANCH}`]);
const checkedOutUpdaterSource = Deno.readTextFileSync(UPDATER_PATH);
if (updaterSource !== checkedOutUpdaterSource) {
  console.warn(
    `Warning: ${UPDATER_PATH} differs from ` +
      `${upstreamRemote}/${UPSTREAM_BRANCH}; restoring the pre-checkout copy. ` +
      "Verify that it should replace the checked-out updater before pushing.",
  );
}
Deno.writeTextFileSync(UPDATER_PATH, updaterSource);
await run("git", ["submodule", "update", "--init", "--recursive", "v8"]);

const currentVersion = extractVersion();
console.log(`Starting auto update. Currently on ${currentVersion}`);

async function run(
  cmd: string,
  args: string[],
  cwd?: string,
): Promise<Uint8Array> {
  console.log("$", cmd, ...args);
  const proc = new Deno.Command(cmd, { args, cwd });
  const output = await proc.output();
  if (!output.success) {
    const stderr = decoder.decode(output.stderr).trim();
    if (stderr.length > 0) {
      console.error(stderr);
    }
    console.error(`Failed to run ${cmd} ${args.join(" ")}`);
    Deno.exit(1);
  }
  return output.stdout;
}

// Update v8 submodule
await run("git", ["fetch", `origin`, V8_TRACKING_BRANCH], "./v8");
await run("git", ["checkout", `origin/${V8_TRACKING_BRANCH}`], "./v8");

const newVersion = extractVersion();
if (currentVersion == newVersion) {
  console.log(`No new version available. Staying on ${newVersion}`);
  Deno.exit(0);
}

console.log(`Updated to version ${newVersion}`);

// Update V8 dependencies
const depsOutput = await run("python", ["tools/update_deps.py"]);
const depNames = new TextDecoder().decode(depsOutput).split("\n").filter((x) =>
  x.length > 0
).at(-1)!.split(
  ",",
);

// Update version in readme
let readme = Deno.readTextFileSync("README.md");
readme = readme.replace(
  /V8 Version: \S+/,
  `V8 Version: ${newVersion}`,
);
Deno.writeTextFileSync("README.md", readme);

// Stage the changes
await run("git", ["add", "v8", "README.md", UPDATER_PATH, ...depNames]);

// Commit the changes
await run("git", ["commit", "-m", `Rolling to V8 ${newVersion}`]);

// Push to the repository that will own the PR head.
await run("git", [
  "push",
  PUSH_REMOTE,
  `+HEAD:refs/heads/${AUTOROLL_BRANCH}`,
]);

// Refresh the remote-tracking branch after the push.
await run("git", ["fetch", PUSH_REMOTE, AUTOROLL_BRANCH]);

const openPrs = (JSON.parse(
  decoder.decode(
    await run("gh", [
      "pr",
      "list",
      "--repo",
      UPSTREAM_REPOSITORY,
      "--state",
      "open",
      "--head",
      AUTOROLL_BRANCH,
      "--json",
      "number,headRepositoryOwner",
    ]),
  ),
) as {
  number: number;
  headRepositoryOwner: { login: string };
}[]).filter((pr) => pr.headRepositoryOwner.login === pushOwner);

if (openPrs.length > 0) {
  console.log("Already open PR. Editing existing PR.");
  await run("gh", [
    "pr",
    "edit",
    openPrs[0].number.toString(),
    "--repo",
    UPSTREAM_REPOSITORY,
    "--title",
    `Rolling to V8 ${newVersion}`,
  ]);
} else {
  console.log("No PR open. Creating a new PR.");
  await run("gh", [
    "pr",
    "create",
    "--repo",
    UPSTREAM_REPOSITORY,
    "--title",
    `Rolling to V8 ${newVersion}`,
    "--body",
    "",
    "--base",
    UPSTREAM_BRANCH,
    "--head",
    prHead,
  ]);
}
