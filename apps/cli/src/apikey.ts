import fs from "node:fs/promises";
import path from "node:path";
import { parseArgs } from "node:util";
import { getStoreBaseDir, runHarness } from "./harness.js";

type ProviderConfig = {
  displayName: string;
  storageName: "google ai studio" | "imgbb";
  validate: (key: string) => boolean;
  usageName: string;
};

function usage(): string {
  return [
    "Usage:",
    "  npm --prefix apps/cli run apikey:test -- <email> <provider> <key>",
    "  npm --prefix apps/cli run apikey:test -- <email> --provider <provider> --key <key>",
    "",
    "Providers:",
    "  google-ai-studio",
    "  imgbb",
    "",
    "Examples:",
    "  npm --prefix apps/cli run apikey:test -- you@example.com google-ai-studio AIzaSXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "  npm --prefix apps/cli run apikey:test -- you@example.com --provider imgbb --key 12345678901234567890123456789012",
  ].join("\n");
}

function resolveProvider(rawProvider: string): ProviderConfig {
  switch (rawProvider.trim().toLowerCase()) {
    case "google ai studio":
    case "google-ai-studio":
    case "google_ai_studio":
    case "gemini":
      return {
        displayName: "Google AI Studio",
        storageName: "google ai studio",
        usageName: "google-ai-studio",
        validate: (key) => !key || (key.startsWith("AIzaS") && key.length === 39),
      };
    case "imgbb":
      return {
        displayName: "ImgBB",
        storageName: "imgbb",
        usageName: "imgbb",
        validate: (key) => !key || key.length === 32,
      };
    default:
      throw new Error(
        `Unsupported provider '${rawProvider}'. Use 'google-ai-studio' or 'imgbb'.`,
      );
  }
}

function assertValidKey(provider: ProviderConfig, key: string): void {
  if (provider.validate(key)) {
    return;
  }

  if (provider.storageName === "google ai studio") {
    throw new Error(
      "Invalid Google AI Studio key. Expected a key starting with 'AIzaS' and exactly 39 characters.",
    );
  }

  throw new Error("Invalid ImgBB key. Expected exactly 32 characters.");
}

async function syncDesktopActiveAccount(profileId: string): Promise<void> {
  const storeBaseDir = await getStoreBaseDir();
  const preferencesPath = path.resolve(storeBaseDir, "..", "preferences.json");

  let preferences: Record<string, unknown> = {};
  try {
    const existing = await fs.readFile(preferencesPath, "utf8");
    preferences = JSON.parse(existing) as Record<string, unknown>;
  } catch (error: unknown) {
    const nodeError = error as NodeJS.ErrnoException;
    if (nodeError.code !== "ENOENT") {
      throw error;
    }
  }

  preferences.activeAccount = profileId;
  await fs.mkdir(path.dirname(preferencesPath), { recursive: true });
  await fs.writeFile(preferencesPath, `${JSON.stringify(preferences, null, 2)}\n`);
}

async function main(): Promise<void> {
  const { positionals, values } = parseArgs({
    args: process.argv.slice(2),
    allowPositionals: true,
    options: {
      email: { type: "string" },
      provider: { type: "string" },
      key: { type: "string" },
    },
  });

  const email = (values.email ?? positionals[0] ?? "").trim();
  const rawProvider = (values.provider ?? positionals[1] ?? "").trim();
  const key = (values.key ?? positionals[2] ?? "").trim();

  if (!email || !rawProvider || values.key === undefined && positionals[2] === undefined) {
    throw new Error(usage());
  }

  const provider = resolveProvider(rawProvider);
  assertValidKey(provider, key);

  const profileId = (await runHarness(["profile-id-for-email", email])).trim();
  if (!profileId) {
    throw new Error(`No Squigit profile found for ${email}. Sign in first.`);
  }

  await runHarness(["save-key", profileId, provider.storageName, key]);

  const savedKey = (await runHarness(["get-key", profileId, provider.storageName])).trim();
  if (savedKey !== key) {
    throw new Error(`Saved ${provider.displayName} key could not be verified.`);
  }

  await runHarness(["set-active-profile", profileId]);
  await syncDesktopActiveAccount(profileId);

  if (key) {
    console.log(`saved ${provider.displayName} key for ${email}`);
  } else {
    console.log(`cleared ${provider.displayName} key for ${email}`);
  }
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(message);
  process.exit(1);
});
