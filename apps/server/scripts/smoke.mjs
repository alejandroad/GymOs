// Smoke test for the built server (CI runs it on Windows, macOS and Linux
// after `pnpm build`). It drives dist/server.mjs the way a user does:
//   1. start it on a free port with an empty temporary data folder;
//   2. load the UI and add a note through the RPC endpoint;
//   3. kill it hard, start it again, and check the note and the integrity check;
//   4. check that a second server refuses the same database;
//   5. check that a foreign Origin or Host is refused with 403.
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtempSync, rmSync } from "node:fs";
import { request } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const serverPath = fileURLToPath(new URL("../dist/server.mjs", import.meta.url));
const dataDir = mkdtempSync(join(tmpdir(), "gymos-smoke-"));
const env = { ...process.env, GYMOS_PORT: "0", GYMOS_DATA_DIR: dataDir };
const children = new Set();

function check(condition, message) {
  if (!condition) throw new Error(`Smoke test failed: ${message}`);
  console.log(`ok - ${message}`);
}

/** Starts the server and resolves with its port once it is listening. */
function start() {
  const child = spawn(process.execPath, [serverPath], { env, stdio: ["ignore", "pipe", "pipe"] });
  children.add(child);
  let output = "";
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`server didn't start:\n${output}`)), 15000);
    const onData = (chunk) => {
      output += chunk;
      const match = /running at http:\/\/127\.0\.0\.1:(\d+)/.exec(output);
      if (match) {
        clearTimeout(timer);
        resolve({ child, port: Number(match[1]), output });
      }
    };
    child.stdout.on("data", onData);
    child.stderr.on("data", onData);
    child.on("exit", (code) => {
      clearTimeout(timer);
      reject(Object.assign(new Error(`server exited with ${code}:\n${output}`), { code, output }));
    });
  });
}

function kill(child) {
  const exited = new Promise((resolve) => child.once("exit", resolve));
  child.kill("SIGKILL");
  return exited.then(() => children.delete(child));
}

function http(port, { method = "GET", path = "/", headers = {}, body } = {}) {
  return new Promise((resolve, reject) => {
    const req = request(
      { host: "127.0.0.1", port, method, path, headers: { host: `127.0.0.1:${port}`, ...headers } },
      (res) => {
        let text = "";
        res.on("data", (chunk) => (text += chunk));
        res.on("end", () => resolve({ status: res.statusCode, text }));
      },
    );
    req.on("error", reject);
    req.end(body);
  });
}

async function rpc(port, method, ...args) {
  const res = await http(port, {
    method: "POST",
    path: "/api/rpc",
    headers: { origin: `http://127.0.0.1:${port}`, "content-type": "application/json" },
    body: JSON.stringify({ method, args }),
  });
  const result = JSON.parse(res.text);
  if (!result.ok) throw new Error(`${method} failed: ${result.error.message}`);
  return result.result;
}

try {
  const first = await start();
  check(first.output.includes(join(dataDir, "gymos.sqlite3")), "startup prints the database path");

  const page = await http(first.port, { path: "/spike" });
  check(page.status === 200 && page.text.includes("<html"), "the UI is served");

  const note = { id: randomUUID(), body: "smoke", createdAt: Date.now() };
  await rpc(first.port, "addNote", note);
  check((await rpc(first.port, "countNotes")) === 1, "a note is written");

  const second = await start().then(
    () => null,
    (error) => error,
  );
  check(
    second?.code === 1 && second.output.includes("Is GymOs already running?"),
    "a second server refuses the same database",
  );

  await kill(first.child);
  const restarted = await start();
  check((await rpc(restarted.port, "countNotes")) === 1, "the note survives a hard kill and restart");
  check((await rpc(restarted.port, "listNotes", 10))[0]?.body === "smoke", "the note reads back intact");
  check((await rpc(restarted.port, "integrityCheck")) === "ok", "integrity_check is ok");

  const foreignOrigin = await http(restarted.port, {
    method: "POST",
    path: "/api/rpc",
    headers: { origin: "https://attacker.example", "content-type": "application/json" },
    body: JSON.stringify({ method: "countNotes", args: [] }),
  });
  check(foreignOrigin.status === 403, "a foreign Origin gets 403");

  const foreignHost = await http(restarted.port, { headers: { host: "attacker.example" } });
  check(foreignHost.status === 403, "a foreign Host gets 403");

  await kill(restarted.child);
  console.log("Smoke test passed.");
} finally {
  for (const child of children) child.kill("SIGKILL");
  // Give Windows a moment to release the file before deleting it.
  await new Promise((resolve) => setTimeout(resolve, 300));
  rmSync(dataDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
}
