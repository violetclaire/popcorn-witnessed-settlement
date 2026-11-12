#!/usr/bin/env node
// POPCORN receipt checker. Offline, no account, no keys.
//
// Verifies one POPCORN witness receipt the way any stranger can:
//   1. Reads the receipt (the JSON form the node issues, or a bare compact JWS).
//   2. Takes the signing key from a JWKS (the live published set, or a
//      cached copy), matched by the receipt's key id.
//   3. Verifies the ES256 signature over the JWS.
//   4. Reads the stamped digest and the witnessed interval out of the
//      SIGNED payload (never out of unsigned wrapper fields).
//   5. Compares the stamped digest to an expected pair_digest you pass in,
//      and, if you pass an approval window, checks the witnessed interval
//      covers it.
//
// Usage:
//   node check-receipt.mjs --receipt <file> --expect-digest <pair_digest>
//                          [--jwks <file>] [--window <opens_unix> <closes_unix>]
//   node check-receipt.mjs --jws <compact.jws.string> --expect-digest <pair_digest>
//
// The receipt file may be the full evidence JSON (with
// witness_attestation.compact_jws), a JSON object with a compact_jws
// field, or a file whose whole content is the compact JWS string.
//
// Exit codes:
//   0  signature valid AND stamped digest matches (AND interval covers
//      the approval window, if one was given)
//   2  signature invalid, or key id not in the JWKS
//   3  stamped digest does not match the expected pair_digest
//   4  witnessed interval does not cover the approval window
//   1  input or usage error

import { createPublicKey, verify } from "node:crypto";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const LIVE_JWKS_URL = "https://767-2676.com/.well-known/popcorn-keys.json";
const CACHED_JWKS = join(HERE, "..", "fixtures", "jwks.live.json");

function fail(code, message) {
  console.log(`FAIL: ${message}`);
  process.exit(code);
}

function parseArgs(argv) {
  const args = { window: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--receipt") args.receipt = argv[++i];
    else if (a === "--jws") args.jws = argv[++i];
    else if (a === "--expect-digest") args.expect = argv[++i];
    else if (a === "--jwks") args.jwks = argv[++i];
    else if (a === "--window") {
      args.window = [Number(argv[++i]), Number(argv[++i])];
    } else fail(1, `unknown argument: ${a}`);
  }
  if (!args.receipt && !args.jws) fail(1, "pass --receipt <file> or --jws <string>");
  if (!args.expect) fail(1, "pass --expect-digest <pair_digest>");
  if (args.window && args.window.some((n) => !Number.isFinite(n))) {
    fail(1, "--window needs two unix-second numbers");
  }
  return args;
}

function looksLikeJws(s) {
  return typeof s === "string" && /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/.test(s.trim());
}

async function extractJws(args) {
  if (args.jws) return args.jws.trim();
  const raw = await readFile(args.receipt, "utf8");
  const trimmed = raw.trim();
  if (looksLikeJws(trimmed)) return trimmed;
  let json;
  try {
    json = JSON.parse(trimmed);
  } catch {
    fail(1, "receipt file is neither a compact JWS nor JSON");
  }
  const jws =
    json?.witness_attestation?.compact_jws ??
    json?.compact_jws ??
    json?.paid_evidence?.witness_attestation?.compact_jws;
  if (!jws) fail(1, "no compact_jws found in the receipt JSON");
  return jws;
}

async function loadJwks(args) {
  if (args.jwks) {
    return { jwks: JSON.parse(await readFile(args.jwks, "utf8")), source: `file ${args.jwks}` };
  }
  try {
    const res = await fetch(LIVE_JWKS_URL);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return { jwks: await res.json(), source: `live ${LIVE_JWKS_URL}` };
  } catch (e) {
    const cached = JSON.parse(await readFile(CACHED_JWKS, "utf8"));
    return { jwks: cached, source: `cached copy fixtures/jwks.live.json (live fetch failed: ${e.message})` };
  }
}

function b64urlJson(part, label) {
  try {
    return JSON.parse(Buffer.from(part, "base64url").toString("utf8"));
  } catch {
    fail(1, `${label} is not base64url JSON`);
  }
}

// A digest may be written as base64url (POPCORN's form) or as hex.
// Compare on raw bytes so either spelling of the expected value works.
function digestBytes(value) {
  if (/^[0-9a-fA-F]{64}$/.test(value)) return Buffer.from(value, "hex");
  return Buffer.from(value, "base64url");
}

const args = parseArgs(process.argv.slice(2));
const compact = await extractJws(args);
const parts = compact.split(".");
if (parts.length !== 3) fail(1, "compact JWS must have three parts");

const header = b64urlJson(parts[0], "JWS header");
if (header.alg !== "ES256") fail(2, `unexpected JWS alg: ${header.alg}`);
const payload = b64urlJson(parts[1], "JWS payload");
const signature = Buffer.from(parts[2], "base64url");

const { jwks, source } = await loadJwks(args);
console.log(`JWKS source: ${source}`);
const jwk = (jwks.keys ?? []).find((k) => k.kid === header.kid);
if (!jwk) fail(2, `key id ${header.kid} is not in the JWKS`);
console.log(`Signing key: ${jwk.kid} (${jwk.kty} ${jwk.crv})`);

const publicKey = createPublicKey({ key: jwk, format: "jwk" });
const ok = verify(
  "sha256",
  Buffer.from(`${parts[0]}.${parts[1]}`),
  { key: publicKey, dsaEncoding: "ieee-p1363" },
  signature,
);
if (!ok) fail(2, "signature does not verify against the published key");
console.log("Signature: valid. The signed payload below is what the node actually signed.");

// Everything from here is read out of the signed payload only.
const receiptId = payload.receipt_id ?? "(none)";
const stamped = payload?.commitment?.payload_digest;
if (!stamped?.value) fail(1, "signed payload has no commitment.payload_digest.value");
const win = payload?.witness_window_utc;
if (!win?.earliest || !win?.latest) fail(1, "signed payload has no witness_window_utc");
const earliest = Math.floor(Date.parse(win.earliest) / 1000);
const latest = Math.floor(Date.parse(win.latest) / 1000);

console.log(`Receipt id: ${receiptId}`);
console.log(`Payment identifier in receipt: ${payload.payment_identifier ?? "(none)"}`);
console.log(`Stamped digest (${stamped.algorithm}): ${stamped.value}`);
console.log(`Witnessed interval: ${win.earliest} .. ${win.latest} (unix ${earliest} .. ${latest})`);
console.log(`Expected pair_digest: ${args.expect}`);

if (!digestBytes(stamped.value).equals(digestBytes(args.expect))) {
  fail(3, "stamped digest does not equal the expected pair_digest");
}
console.log("Digest check: stamped digest equals the expected pair_digest.");

if (args.window) {
  const [opens, closes] = args.window;
  console.log(`Approval window to cover: unix ${opens} .. ${closes}`);
  if (!(earliest <= opens && latest >= closes)) {
    fail(4, "witnessed interval does not cover the approval window");
  }
  console.log("Window check: witnessed interval covers the approval window.");
}

console.log("RESULT: receipt verified, digest matches.");
process.exit(0);
