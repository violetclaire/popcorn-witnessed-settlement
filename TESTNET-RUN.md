# Testnet run record

Recorded by the builder, November 12, 2026. All of this is NEAR testnet.
No mainnet account was touched and no real funds moved. The demo-1 run
below bought no POPCORN stamp; the demo-2 run further down bought
exactly one, for $0.001, with the owner's authorization.

## Accounts

- Throwaway builder account (created for this demo through the testnet
  faucet, funded with 10 testnet NEAR): witness7672676.testnet
  - Creation transaction: FuqA1gpAEvSEPrRFbWqDGw8mc7YLezeo2Cfh1ANcBQXq
  - This is not Violet's account. Its keys live only in the builder's
    sandbox keychain and are worthless outside testnet.
- Contract account: witness7672676.testnet (the contract is deployed on
  the same account). Deployed with the init call `new`. A second deploy
  attempt failed with "The contract has already been initialized"
  (transaction BtcVVWc9HSYxropQ5YXvNpnkFJjpNs7RojGTDPnMrGwS), which is
  the on-chain proof the first deploy and init succeeded.

## Contract build

- Source: contract/src/lib.rs (Rust, near-sdk 5.30, built with
  cargo-near 0.23.0)
- Wasm: contract/target/near/popcorn_witnessed_settlement.wasm
  - 140,358 bytes
  - SHA-256: 155de2ce6192d0ab4aa95eefe78b3d9911ecff0632286ffa3d688d66a54662b4
- Unit tests: cargo test, 6 passed, 0 failed (pair_digest vector,
  open state, release success, digest mismatch refusal, window refusal,
  double release refusal).

## Settlement run (settlement id "demo-1")

Terms (demo text): "Agent B will deliver the market summary. Payment
released on a witnessed receipt."

- payload_digest: E9WRWJ2vK-MKR1hmk-I6Sck5ZkZGzCydtkg5YNvU7PE
- payment_identifier: near-testnet-demo-0001
- pair_digest computed locally:
  OUedVz6cHab9NTlQLCE2um65TCrr_06qu77fXmMY_a8
- open_settlement transaction:
  7263eV3CNYYe5DbgyU1UH8qvK57MkGWHp7D1zU4LkMtF
  - Attached 0.1 testnet NEAR. The contract returned the pair_digest
    OUedVz6cHab9NTlQLCE2um65TCrr_06qu77fXmMY_a8, identical to the
    local computation.
- Negative pass: release presented with a wrong stamped digest failed
  on chain; the settlement stayed Open with the funds locked.
- Release transaction: 78MgDDUVzQ42HtLsBL3wmNt7nbn3pjPRQM7KR5E2KDr1
  - Receipt fields presented: receipt_id "pwr_SIMULATED_testnet_demo",
    stamped digest equal to the pair_digest, witnessed interval
    1794465943 .. 1794469563 covering the approval window
    1794465953 .. 1794469553.
  - The receipt id says SIMULATED because it is: the contract checks
    the fields it is presented, and no real POPCORN stamp was bought
    for this run. Verifying a real receipt is the checker script's
    job, against the published JWKS.
- Final on-chain state: status Released, receipt id and interval
  recorded on the settlement.

## Settlement run with a real POPCORN stamp (settlement id "demo-2")

Run November 12, 2026, in one pass, in this order: terms fixed,
pair_digest computed, stamp bought, settlement opened with the approval
window set to the receipt's witnessed interval, release presented with
the real receipt, checker run against the published keys.

Authorization: the owner approved exactly one $0.001 stamp for this
rerun ("Yes spend", November 11, 2026). Exactly one stamp was bought.
The payer was the dedicated payer wallet used by the November 10, 2026
paid handoff test (0xe6bb9Fad3A52e16Da0543cB22E2361dD890E598B), paying
through the same x402 client in ~/workspace/popcorn-payer
(near-demo2-witness-x402.mjs).

- Terms (the demo page's default terms, exact bytes, 197 bytes):
  "Agent B will deliver the market summary for November 2026.
  Payment: 1 testnet NEAR, released on a witnessed receipt matching
  these terms.
  Approved stamp moment (unix seconds): see settlement panel."
- payload_digest: 4hKE0hNJafQdhZVcnyi1pp36h1xaxi3_SfTnoM3zC6U
- payment_identifier: near-testnet-demo-0002
- pair_digest computed locally:
  Y2Dfq7SFvOFKH9yf4LaLnadXOzjrxfcEkI0NV94d7jg
- The stamp: POST https://767-2676.com/v1/receipt, paid $0.001 USDC
  (1000 atomic, Base, x402 v2). Payment settled in Base transaction
  0xf767dcd364b47e57ee49a08b9c39fb884212f8e57dca25bf9f513e0df31f976a.
  Request nonce: 0fbVoga9lCPzz4UPTnRVFrO7M6qt_wcFOqp4lKav18k.
  - receipt_id: pwr_3354865ba13a734d32536e80f9f61056
  - protocol_id: POPCORN-WITNESS/2.0, node_id 767-2676.com
  - stamped digest: Y2Dfq7SFvOFKH9yf4LaLnadXOzjrxfcEkI0NV94d7jg
    (the pair_digest, as sent)
  - witnessed interval: 2026-11-12T07:00:04.203Z ..
    2026-11-12T07:00:24.203Z (unix 1794466804 .. 1794466824), read out
    of the signed payload, identical in the signed payload and the
    response body.
  - Full receipt saved at fixtures/receipt-demo2.near.json.
- open_settlement transaction:
  HisyZJnAJujbBJSDTihtyWHy8B3kCuFHR8acKPEiPQgM
  - Attached 1 testnet NEAR. Approval window set to the receipt's
    interval: opens 1794466804, closes 1794466824. The contract
    returned the pair_digest
    Y2Dfq7SFvOFKH9yf4LaLnadXOzjrxfcEkI0NV94d7jg, identical to the
    local computation.
- Release transaction: DfBeH3v7Fx1XLN2fsR42byJKQwdXa5htTumSx9y57LBm
  - Presented the real receipt id, its stamped digest, and its
    interval. Final on-chain state, read back with get_settlement:
    status Released, receipt_id pwr_3354865ba13a734d32536e80f9f61056,
    receipt_stamped_digest equal to the pair_digest, interval
    1794466804 .. 1794466824 recorded.
- Checker, run against the live published JWKS:
  node checker/check-receipt.mjs --receipt
  fixtures/receipt-demo2.near.json --expect-digest
  Y2Dfq7SFvOFKH9yf4LaLnadXOzjrxfcEkI0NV94d7jg --window 1794466804
  1794466824 printed "Signature: valid" (key popcorn-witness-2026-01),
  the digest match, and "witnessed interval covers the approval
  window". Exit code 0.
- Negative controls:
  - Checker against the same receipt with a different expected digest:
    "FAIL: stamped digest does not equal the expected pair_digest",
    exit code 3.
  - On chain: a second release call on demo-2 after it released
    failed, transaction
    CEitG8qSdpFiN1zEpgZfhKiCnQaCVCq4JUDSohwzqKBC, contract panic
    "settlement is not open". The funds moved exactly once.

What is real here and what is not: the POPCORN stamp is real (a live,
paid, signed receipt anyone can verify against the published keys).
The money is testnet NEAR, worth nothing. The settlement is a
demonstration on a throwaway account, not a production payment.
