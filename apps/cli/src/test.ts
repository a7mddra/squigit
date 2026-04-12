import { AuthPayload, parseLastJsonLine, runHarness } from "./harness.js";

async function main(): Promise<void> {
  const stdout = await runHarness(["auth-google"]);
  const payload = parseLastJsonLine<AuthPayload>(stdout);
  console.log(`logged in as ${payload.email}`);
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(message);
  process.exit(1);
});
