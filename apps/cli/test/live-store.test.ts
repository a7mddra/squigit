import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const liveStoreEnabled = process.env.SQUIGIT_LIVE_STORE_TESTS === "1";

type HarnessResult = {
  stdout: string;
  stderr: string;
};

async function runHarness(
  args: string[],
  timeoutMs = 10 * 60 * 1000,
): Promise<HarnessResult> {
  const child = spawn(
    "cargo",
    ["run", "-p", "ops-profile-store", "--example", "live_store_harness", "--", ...args],
    {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      env: process.env,
    },
  );

  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (chunk) => {
    stdout += chunk.toString();
  });
  child.stderr.on("data", (chunk) => {
    stderr += chunk.toString();
  });

  const timeout = setTimeout(() => {
    child.kill("SIGTERM");
  }, timeoutMs);

  const [code] = (await once(child, "close")) as [number | null];
  clearTimeout(timeout);

  if (code !== 0) {
    throw new Error(
      `Harness failed for ${args.join(" ")} with exit code ${code}\nSTDOUT:\n${stdout}\nSTDERR:\n${stderr}`,
    );
  }

  return { stdout, stderr };
}

const liveTest = liveStoreEnabled ? test : test.skip;

liveTest("browser-based Google auth writes to the live shared store", async () => {
  const baseDir = (await runHarness(["store-base-dir"])).stdout.trim();
  assert.ok(baseDir, "Expected the shared store base dir to be available.");

  const authRun = await runHarness(["auth-google"]);
  const jsonLine = authRun.stdout
    .trim()
    .split("\n")
    .filter(Boolean)
    .at(-1);

  assert.ok(jsonLine, "Expected auth harness to print a JSON payload.");

  const payload = JSON.parse(jsonLine) as {
    id: string;
    email: string;
    avatar: string;
  };

  assert.ok(payload.id);
  assert.ok(payload.email);

  const profilePath = path.join(baseDir, payload.id, "profile.json");
  const profile = JSON.parse(await fs.readFile(profilePath, "utf8")) as {
    email: string;
    avatar?: string | null;
  };

  assert.equal(profile.email, payload.email);
  if (payload.avatar) {
    assert.equal(profile.avatar, payload.avatar);
  }

  const activeProfileId = (await runHarness(["active-profile-id"])).stdout.trim();
  assert.equal(activeProfileId, payload.id);
});

liveTest("per-profile API keys are encrypted on disk and readable through the shared crate", async () => {
  const baseDir = (await runHarness(["store-base-dir"])).stdout.trim();
  const profileId = (await runHarness(["active-profile-id"])).stdout.trim();
  assert.ok(
    profileId,
    "No active profile found. Run the auth live test first or sign in through desktop.",
  );

  const expectedKey = "live-store-google-key";
  const savedPath = (
    await runHarness(["save-key", profileId, "google ai studio", expectedKey])
  ).stdout.trim();

  assert.ok(savedPath.startsWith(baseDir));

  const encryptedPayload = JSON.parse(await fs.readFile(savedPath, "utf8")) as {
    ciphertext: string;
    algo: string;
  };
  assert.equal(encryptedPayload.algo, "aes-256-gcm");
  assert.notEqual(encryptedPayload.ciphertext, expectedKey);

  const decryptedKey = (
    await runHarness(["get-key", profileId, "google ai studio"])
  ).stdout.trim();
  assert.equal(decryptedKey, expectedKey);
});
