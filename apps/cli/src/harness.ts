import { spawn } from "node:child_process";
import { once } from "node:events";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");

export type AuthPayload = {
  id: string;
  email: string;
  name: string;
  avatar: string;
  original_picture: string;
};

export async function runHarness(args: string[]): Promise<string> {
  const child = spawn(
    "cargo",
    ["run", "-q", "-p", "ops-profile-store", "--example", "live_store_harness", "--", ...args],
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

  const [code] = (await once(child, "close")) as [number | null];

  if (code !== 0) {
    throw new Error(
      stderr.trim() || stdout.trim() || `Harness exited with code ${code ?? "unknown"}`,
    );
  }

  return stdout;
}

export function parseLastJsonLine<T>(stdout: string): T {
  const jsonLine = stdout
    .trim()
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .at(-1);

  if (!jsonLine) {
    throw new Error("Command completed without a JSON payload.");
  }

  return JSON.parse(jsonLine) as T;
}

export async function getStoreBaseDir(): Promise<string> {
  return (await runHarness(["store-base-dir"])).trim();
}
