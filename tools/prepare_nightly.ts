// Prepares a nightly release.
//
// Nightlies reuse the regular release pipeline: everything in ci.yml that
// builds the matrix, uploads prebuilt binaries and runs `cargo publish` is
// already triggered by any pushed tag. So all this script has to do is produce
// a correctly versioned tag.
//
// The version bump is committed on a detached commit that is *not* pushed to
// main -- only the tag is pushed. That keeps main's Cargo.toml on the last
// stable version while the tagged tree carries the nightly version, which is
// what build.rs uses to locate prebuilt binaries (it downloads from
// `releases/download/v{CARGO_PKG_VERSION}/`).
//
// Usage:
//   deno run -A ./tools/prepare_nightly.ts [--dry-run]

const CARGO_TOML = "./Cargo.toml";
const CARGO_LOCK = "./Cargo.lock";

const dryRun = Deno.args.includes("--dry-run");

function git(...args: string[]): string {
  const result = tryGit(...args);
  if (result === null) {
    throw new Error(`git ${args.join(" ")} failed`);
  }
  return result;
}

/** Like `git`, but returns null instead of throwing when the command fails. */
function tryGit(...args: string[]): string | null {
  const output = new Deno.Command("git", { args, stderr: "inherit" })
    .outputSync();
  if (!output.success) {
    return null;
  }
  return new TextDecoder().decode(output.stdout).trim();
}

/** Returns the `version` of the `[package]` section, e.g. "152.0.0". */
export function packageVersion(cargoToml: string): string {
  const section = cargoToml.split(/^\[/m).find((s) => s.startsWith("package]"));
  if (section === undefined) {
    throw new Error("no [package] section in Cargo.toml");
  }
  const match = section.match(/^version = "([^"]+)"$/m);
  if (match === null) {
    throw new Error("no version in the [package] section of Cargo.toml");
  }
  return match[1];
}

/**
 * Computes the version for today's nightly.
 *
 * A prerelease sorts *below* its base version, so a nightly cut after 152.0.0
 * was published must be based on the next unreleased version -- otherwise
 * `152.0.0-nightly.20260804` would rank as older than the 152.0.0 it was built
 * from. We bump the minor, matching the default of the regular release
 * workflow.
 *
 * `existingTags` disambiguates repeat runs on the same day: the second nightly
 * of the day becomes `...-nightly.20260804.1`. Numeric prerelease identifiers
 * compare numerically, so the ordering stays correct.
 */
export function nightlyVersion(
  baseVersion: string,
  date: Date,
  existingTags: string[],
): string {
  const match = baseVersion.match(/^(\d+)\.(\d+)\.(\d+)(-nightly\.|$|-|\+)/);
  if (match === null) {
    throw new Error(`unexpected version in Cargo.toml: ${baseVersion}`);
  }
  const [, major, minor, patch, suffix] = match;
  // Idempotent: a version that is already a nightly has had the minor bumped
  // once, so bumping again would drift a minor per run.
  const next = suffix === "-nightly."
    ? `${major}.${minor}.${patch}`
    : `${major}.${Number(minor) + 1}.0`;

  const stamp = [
    date.getUTCFullYear(),
    String(date.getUTCMonth() + 1).padStart(2, "0"),
    String(date.getUTCDate()).padStart(2, "0"),
  ].join("");

  const candidate = `${next}-nightly.${stamp}`;
  if (!existingTags.includes(`v${candidate}`)) {
    return candidate;
  }
  for (let n = 1;; n++) {
    if (!existingTags.includes(`v${candidate}.${n}`)) {
      return `${candidate}.${n}`;
    }
  }
}

function replacePackageVersion(cargoToml: string, version: string): string {
  let seen = false;
  return cargoToml.replace(/^version = "[^"]+"$/m, (line) => {
    // Only the first line-anchored `version = ` belongs to [package]; keys in
    // dependency tables are inline (`foo = { version = "..." }`).
    if (seen) return line;
    seen = true;
    return `version = "${version}"`;
  });
}

function replaceLockVersion(cargoLock: string, version: string): string {
  // `cargo publish --locked` fails unless the lockfile agrees with Cargo.toml.
  const re = /(\[\[package\]\]\nname = "v8"\nversion = ")[^"]+(")/;
  if (!re.test(cargoLock)) {
    throw new Error("could not find the v8 package entry in Cargo.lock");
  }
  return cargoLock.replace(re, `$1${version}$2`);
}

/** Tags of past nightlies, oldest first. */
function nightlyTags(): string[] {
  const tags = git("tag", "--list", "v*-nightly.*", "--sort=creatordate");
  return tags.length === 0 ? [] : tags.split("\n");
}

function setOutput(key: string, value: string) {
  console.log(`${key}=${value}`);
  const path = Deno.env.get("GITHUB_OUTPUT");
  if (path !== undefined) {
    Deno.writeTextFileSync(path, `${key}=${value}\n`, { append: true });
  }
}

function main() {
  const head = git("rev-parse", "HEAD");
  const tags = nightlyTags();
  const previous = tags.at(-1);

  // Every nightly tag points at a bump commit whose parent is the main commit
  // it was cut from, so the parent tells us whether anything has landed since.
  if (previous !== undefined) {
    const previousBase = git("rev-parse", `${previous}^`);
    if (previousBase === head) {
      console.log(
        `No commits since ${previous} (${head.slice(0, 9)}), skipping.`,
      );
      setOutput("skipped", "true");
      return;
    }
  }

  const cargoToml = Deno.readTextFileSync(CARGO_TOML);
  const version = nightlyVersion(
    packageVersion(cargoToml),
    new Date(),
    tags,
  );
  const tag = `v${version}`;
  const cargoLock = replaceLockVersion(
    Deno.readTextFileSync(CARGO_LOCK),
    version,
  );

  if (dryRun) {
    console.log(`Would tag ${tag} at ${head.slice(0, 9)}.`);
    setOutput("skipped", "false");
    setOutput("version", version);
    setOutput("tag", tag);
    return;
  }

  // Commit detached so the branch never moves: the bump belongs to the tag
  // only, and main keeps the last stable version. Detaching also keeps repeat
  // runs in one working tree behaving like the fresh checkouts CI does -- the
  // branch is restored below, so the next run sees an unbumped tree again.
  const original = tryGit("symbolic-ref", "--quiet", "--short", "HEAD") ?? head;
  git("checkout", "--detach", "--quiet", "HEAD");

  Deno.writeTextFileSync(CARGO_TOML, replacePackageVersion(cargoToml, version));
  Deno.writeTextFileSync(CARGO_LOCK, cargoLock);

  git("add", CARGO_TOML, CARGO_LOCK);
  git("commit", "--quiet", "-m", `${version}`);
  git("tag", "-a", tag, "-m", `Nightly release ${version} from ${head}`);
  git("checkout", "--quiet", original);

  console.log(`Tagged ${tag} at ${head.slice(0, 9)}.`);
  setOutput("skipped", "false");
  setOutput("version", version);
  setOutput("tag", tag);
}

if (import.meta.main) {
  main();
}
