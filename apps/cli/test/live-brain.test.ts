import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { parseLastJsonLine, runBrainHarness, runHarness } from "../src/harness.js";

const liveStoreEnabled = process.env.SQUIGIT_LIVE_STORE_TESTS === "1";
const liveTest = liveStoreEnabled ? test : test.skip;

let latestChatId: string | null = null;

type AnalyzePayload = {
  chat_id: string;
  title: string;
  assistant_message: string;
  image_path: string;
};

type PromptPayload = {
  chat_id: string;
  assistant_message: string;
};

async function createFixtureImage(): Promise<string> {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "squigit-cli-"));
  const imagePath = path.join(dir, "fixture.png");

  // 1x1 transparent PNG
  const pngBase64 =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO3Z4J0AAAAASUVORK5CYII=";
  await fs.writeFile(imagePath, Buffer.from(pngBase64, "base64"));

  return imagePath;
}

liveTest("analyze command creates a chat in the active profile store", async () => {
  const profileId = (await runHarness(["active-profile-id"])).trim();
  assert.ok(profileId, "No active profile found. Sign in first.");

  const imagePath = await createFixtureImage();
  const stdout = await runBrainHarness(["analyze", imagePath]);
  const payload = parseLastJsonLine<AnalyzePayload>(stdout);

  assert.ok(payload.chat_id);
  assert.ok(payload.assistant_message.trim().length > 0);
  assert.notEqual(payload.title.trim().toLowerCase(), "untitled");
  latestChatId = payload.chat_id;

  const baseDir = (await runHarness(["store-base-dir"])).trim();
  const chatDir = path.join(baseDir, profileId, "chats", payload.chat_id);

  await fs.access(path.join(chatDir, "messages.json"));
  await fs.access(path.join(chatDir, "messages.md"));
});

liveTest("prompt command appends to same chat and normalizes @/absolute/path", async () => {
  const profileId = (await runHarness(["active-profile-id"])).trim();
  assert.ok(profileId, "No active profile found. Sign in first.");

  if (!latestChatId) {
    const imagePath = await createFixtureImage();
    const stdout = await runBrainHarness(["analyze", imagePath]);
    latestChatId = parseLastJsonLine<AnalyzePayload>(stdout).chat_id;
  }

  assert.ok(latestChatId);

  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "squigit-file-"));
  const attachmentPath = path.resolve(tmpDir, "note.txt");
  await fs.writeFile(attachmentPath, "hello from cli attachment\n");

  const promptText = `please inspect @${attachmentPath}`;
  const stdout = await runBrainHarness(["prompt", latestChatId, promptText]);
  const payload = parseLastJsonLine<PromptPayload>(stdout);

  assert.equal(payload.chat_id, latestChatId);
  assert.ok(payload.assistant_message.trim().length > 0);

  const baseDir = (await runHarness(["store-base-dir"])).trim();
  const chatDir = path.join(baseDir, profileId, "chats", latestChatId);
  const messagesJson = await fs.readFile(path.join(chatDir, "messages.json"), "utf8");
  const messagesMd = await fs.readFile(path.join(chatDir, "messages.md"), "utf8");

  assert.match(messagesJson, /"role"\s*:\s*"user"/);
  assert.match(messagesJson, /\[note\.txt\]\(<.*objects\//);
  assert.match(messagesMd, /note\.txt/);
});
