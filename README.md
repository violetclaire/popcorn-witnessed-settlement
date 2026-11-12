# 767-2676.com: witnessed settlement between agents

A settlement between two agents on NEAR testnet that releases funds only
when a witnessed receipt matches the terms the agents agreed. The witness
is POPCORN, a live service at https://767-2676.com that stamps the
fingerprint of a record as received within a witnessed time interval,
signed with published keys anyone can check alone, for free.

This is a testnet demonstration. No real funds move anywhere in this
repository. The settlement run demo-2 recorded in TESTNET-RUN.md used
one real POPCORN stamp, bought for $0.001 with the owner's
authorization; the earlier rehearsal run (demo-1) used none.

## The problem

An agent that pays another agent has to trust the other side's word: that
the terms were the terms agreed, and that the work or the payment happened
inside the window everyone accepted. Memory and logs are kept by somebody
and ask to be trusted. This project replaces that trust with a check
anyone can run: the terms are fingerprinted before anything moves, the
fingerprint is bound to the payment, and the money moves only when an
independent witness's receipt matches that binding.

## How it works

1. The two agents agree terms. The terms are frozen as canonical bytes and
   hashed. That hash is the payload_digest.
2. The payer opens a settlement in the NEAR testnet contract, recording
   the payload_digest and a payment identifier. The contract computes the
   pair_digest: SHA-256(payment_identifier : payload_digest), a
   deterministic commitment binding the payment to the approved terms.
   Anyone can recompute it offline. It holds no custody beyond the
   settlement itself.
3. The payer obtains a POPCORN receipt whose stamped digest is the
   pair_digest, witnessed inside the approval window recorded at opening.
   (In production the payer presents the pair_digest to POPCORN as the
   payload commitment to be stamped.)
4. The contract releases the testnet funds only when a receipt is
   presented whose stamped digest equals the pair_digest and whose
   witnessed interval covers the approval window. The contract stores the
   receipt identifier and the pair_digest as the settlement record.
5. If the receipt does not match, or no receipt is presented, the funds
   do not move. The payer can refund an unreleased settlement.

### pair_digest byte form (read this before recomputing)

The published construction is printed as SHA-256(payment_identifier :
payload_digest). The byte form is settled, not a demo choice: SHA-256
over the two values as text joined by a single colon, no spaces. It is
used identically by the contract, the demo page, and every test vector
here:

    SHA-256( payment_identifier bytes || ":" || payload_digest bytes )

That is: the UTF-8 bytes of the payment identifier string, one colon
byte, then the bytes of the payload_digest string as recorded (the
base64url form POPCORN stamps). The result is written in base64url with
no padding. Cross-check against the worked example in issue #37 of the
POPCORN repository: SHA-256 over the text
0xb44292df23f1e5c257aa85ceae84b0cc15613f9cb6c9757151709e60258b22d2,
one colon, then
0xcef0c1853c3deca9f5ed06fb3d8c7e157d430b1cb65c0c8435a2d27445ccf893
is 4c0d3374f85ce2babcb6d4a61ce6b4ddaf7007baf2684fd108a4f54a39556b35
(the same rule, spelled in hex). Worked vector in this demo's own
spelling, also a unit test in the contract:

    payment_identifier  eip3009_demo_0001
    payload_digest      UKwdgKFq4m5a1Mm0qMSuhIJYnJ8jdmNd01uFdHaAE-M
    pair_digest         iemMXR_sqXqRQPWhVSK7GNPgxWzUe_Ru7aJmp3D1akM

### The trust seam, stated plainly

The contract checks the receipt fields it is presented (stamped digest,
witnessed interval) against the settlement. It cannot fetch POPCORN, and
in this demo it does not verify the receipt's ES256 signature on chain.
The independent check that a receipt is real, that its signature verifies
against the witness keys published at
https://767-2676.com/.well-known/popcorn-keys.json, and that the stamped
digest and interval are what the node actually signed, is done off chain
by the checker script in checker/. That separation is the design: the
witness is evidence, never permission, and anyone can run the check
alone, with no account and no keys.

## What is in this repository

- contract/ : the NEAR testnet settlement contract (Rust, near-sdk).
- checker/check-receipt.mjs : the offline receipt checker (Node, no
  dependencies, no account, no keys).
- demo/index.html : a single static demo page, no build step.
- fixtures/ : a real production sample receipt from the POPCORN
  repository (receipt-001.production.json), the JWKS it verifies against
  (jwks.production.json), a cached copy of the live JWKS
  (jwks.live.json), and the real paid receipt that released settlement
  demo-2 on testnet (receipt-demo2.near.json). Key material in the cached live copy and the fixture
  copy is identical; the checker prefers the live fetch and falls back
  to the cache.
- TESTNET-RUN.md : the recorded testnet deployment and settlement run.

## Run the demo page (do X, see Y)

1. Open demo/index.html in a browser. See two agent panels, a terms box,
   and a settlement in the state "not funded." The page is labeled as a
   testnet demonstration that runs the contract's rules in the browser.
2. Click "Agree terms: compute the fingerprint." See the payload_digest
   appear. It is computed in your browser, with WebCrypto, from the exact
   terms bytes in the box.
3. Click "Open settlement." See the pair_digest, the approval window,
   and the locked balance. The approval window is a one second window at
   an agreed moment 20 seconds ahead, because a POPCORN receipt's
   witnessed interval is the witnessed moment plus and minus the node's
   published clock accuracy radius of 10 seconds, and the interval must
   cover the window.
4. Click "Get witness receipt (stand-in)." See a receipt in the live
   node's format, stamped at the agreed moment with the pair_digest as
   its stamped digest. It is labeled SIMULATED in red: it is produced by
   a stand-in in the page, not by POPCORN. A real stamp is a paid call
   ($0.001) and is never made without the owner's authorization for that
   stamp.
5. Click "Release." See the settlement release to Agent B, because the
   receipt's stamped digest equals the pair_digest and its interval
   covers the approval window.
6. Change one character in the terms box, then click the Step 5 button.
   See a different payload_digest and a different pair_digest, and see
   the Step 3 receipt refused: "receipt stamped digest does not equal
   the pair_digest." The funds do not move.
7. Click "Verify the real receipt's signature" in the last panel. See
   the production sample receipt's ES256 signature verified in your
   browser against the published witness key, with its real receipt id,
   stamped digest, and witnessed interval.

## Run the checker (do X, see Y)

Needs Node 18 or newer. No account, no keys, no install.

Verify the bundled production receipt against its own stamped digest
(this is the pass case; it also proves the signature):

    node checker/check-receipt.mjs \
      --receipt fixtures/receipt-001.production.json \
      --expect-digest UKwdgKFq4m5a1Mm0qMSuhIJYnJ8jdmNd01uFdHaAE-M \
      --jwks fixtures/jwks.production.json

See "Signature: valid", the receipt id pwr_24a140e0651c67375cdfb34bc1c38b47,
and exit code 0.

The same receipt against a different expected digest (the refusal case):

    node checker/check-receipt.mjs \
      --receipt fixtures/receipt-001.production.json \
      --expect-digest iemMXR_sqXqRQPWhVSK7GNPgxWzUe_Ru7aJmp3D1akM \
      --jwks fixtures/jwks.production.json

See "FAIL: stamped digest does not equal the expected pair_digest" and
exit code 3.

Window coverage: add --window 1788152530 1788152540 to the first command
and see the window check pass (the receipt's witnessed interval is unix
1788152526 to 1788152546). Widen the window past the interval, for
example --window 1788152500 1788152540, and see exit code 4. A tampered
signature exits 2. Without --jwks, the checker fetches the live JWKS
from https://767-2676.com/.well-known/popcorn-keys.json and falls back
to the cached copy if the fetch fails, printing which source it used.

To check a settlement: pass the settlement's pair_digest (from
get_settlement, or the demo page) as --expect-digest and the receipt the
payer obtained as --receipt. Exit 0 means the receipt is real and its
stamped digest is the settlement's pair_digest.

## Build and test the contract

Needs a Rust toolchain and cargo-near
(https://github.com/near/cargo-near).

    cd contract
    cargo test            # 6 unit tests: digest vector, open, release,
                          # digest refusal, window refusal, double release
    cargo near build non-reproducible-wasm

The wasm lands at contract/target/near/popcorn_witnessed_settlement.wasm
with its ABI beside it. As built on November 12, 2026: 140,358 bytes,
SHA-256 155de2ce6192d0ab4aa95eefe78b3d9911ecff0632286ffa3d688d66a54662b4,
all 6 unit tests passing.

Contract methods: new (init), open_settlement (payable; records the
digests, the window, and the locked funds; returns the pair_digest),
release (presents a receipt's fields; refuses unless the stamped digest
equals the pair_digest and the interval covers the window), refund
(payer only, while open), get_settlement (view; returns the full record
including the releasing receipt's identifier).

## Testnet deployment

The contract is deployed on NEAR testnet at witness7672676.testnet, a
throwaway builder account created through the testnet faucet for this
demo (not the owner's account; its keys never leave the builder's
sandbox and are worthless off testnet). The full run, open, refused
release, matching release, with transaction ids, is recorded in
TESTNET-RUN.md. Read the settlement yourself:

    near contract call-function as-read-only witness7672676.testnet \
      get_settlement json-args '{"settlement_id":"demo-1"}' \
      network-config testnet now

See status "Released", the pair_digest, and the recorded receipt id
"pwr_SIMULATED_testnet_demo". The id says SIMULATED because the on-chain
release presented simulated receipt fields: the contract checks the
fields it is given, and no real POPCORN stamp was bought for that
rehearsal run. Verifying a real receipt is the checker's job, above.

Settlement demo-2 on the same contract is the real-stamp run, done in
one pass with the owner's authorization for exactly one $0.001 stamp:
the terms were fixed, the pair_digest computed, the stamp bought from
the live node, the settlement opened with its approval window set to
the receipt's witnessed interval, and the release presented the real
receipt. Read it the same way, with settlement_id "demo-2": see status
"Released" and the real receipt id "pwr_3354865ba13a734d32536e80f9f61056"
recorded on the settlement. Check that receipt yourself:

    node checker/check-receipt.mjs \
      --receipt fixtures/receipt-demo2.near.json \
      --expect-digest Y2Dfq7SFvOFKH9yf4LaLnadXOzjrxfcEkI0NV94d7jg \
      --window 1794466804 1794466824

See "Signature: valid" against the live published keys, the digest
match, the window check, and exit code 0. The full record, with the
Base settlement transaction for the stamp payment and both NEAR
transaction ids, is in TESTNET-RUN.md.

## What a receipt proves, and what it does not

A POPCORN receipt witnesses only that the node received a fingerprint
within the stated interval, under a published key. It does not prove
agreement timing, authorship, competence, completion, identity,
permission, delivery, or execution. Verification depends on accepting
the node's key and clock. A receipt is evidence, never permission. The
witness never books anyone, never handles the parties' payments beyond
the witnessed stamp, and holds no keys from anyone. Agents bring their
own keys and check the receipt themselves.

## Baseline disclosure (required by the hackathon rules)

POPCORN pre-exists this hackathon. It is live on Base (the x402 payment
rail for its paid stamps) and its source is at
https://github.com/violetclaire/popcorn-temporal-anchor. Under the
hackathon's rules it is baseline. The work built in the hackathon window
(October 19 to November 14, 2026) and judged here is only what is in
this repository: the NEAR testnet settlement contract, the offline
checker script, and this demo. The contract, the checker, and the demo
were built new for this entry.

## Tracks

security, agents.

## License

POPCORN Read-and-Use Terms (see LICENSE in this repo). The code is open
to read. It is not open to take.
