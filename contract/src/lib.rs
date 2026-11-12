//! Witnessed settlement between two agents on NEAR testnet.
//!
//! A payer locks testnet funds into a settlement that names the digest of
//! the agreed terms and a payment identifier. The contract binds the two
//! into a pair_digest. Funds are released to the payee only when a POPCORN
//! witness receipt is presented whose stamped digest equals the pair_digest
//! and whose witnessed interval covers the approval window recorded when
//! the settlement was opened.
//!
//! TRUST SEAM (read this before trusting anything here):
//! This contract cannot fetch POPCORN, and in this demo it does not verify
//! the receipt's ES256 signature on chain. The `release` call carries the
//! receipt fields (receipt id, stamped digest, witnessed interval) as
//! arguments, and the contract checks those fields against the settlement:
//! digest equality and window coverage. The independent check that the
//! receipt is real, that its signature verifies against the witness keys
//! published at https://767-2676.com/.well-known/popcorn-keys.json, and
//! that its stamped digest is what the node actually signed, is done off
//! chain by the checker script in `../checker/`. That separation is the
//! design: the witness is evidence, never permission, and anyone can run
//! the check alone with no account and no keys.
//!
//! pair_digest byte form (the demo's canonical form):
//!   pair_digest = SHA-256(payment_identifier bytes || ":" || payload_digest bytes)
//! where payload_digest is the string form the payer recorded (the
//! base64url SHA-256 of the agreed terms, the form POPCORN stamps).
//! The result is stored and compared in base64url (no padding), the form
//! POPCORN uses for stamped digests. This matches the owner's published
//! construction, printed as SHA-256(payment_identifier : payload_digest).

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::serde::Serialize;
use near_sdk::{env, near, AccountId, NearToken, PanicOnDefault, Promise};
use sha2::{Digest, Sha256};

/// Encode bytes as base64url without padding (RFC 4648 section 5),
/// the encoding POPCORN uses for digest values.
pub fn base64url_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

/// The pair_digest construction, as a pure function so the exact byte form
/// can be tested off chain: SHA-256 over the payment identifier bytes, one
/// ":" byte, then the payload digest bytes. Returned in base64url.
pub fn compute_pair_digest(payment_identifier: &str, payload_digest: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payment_identifier.as_bytes());
    hasher.update(b":");
    hasher.update(payload_digest.as_bytes());
    base64url_encode(&hasher.finalize())
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Clone, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum SettlementStatus {
    Open,
    Released,
    Refunded,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Settlement {
    pub payer: AccountId,
    pub payee: AccountId,
    /// Digest of the agreed terms, in the string form the payer recorded
    /// (base64url SHA-256, the form POPCORN stamps).
    pub payload_digest: String,
    /// The payment identifier bound to the terms (for an EIP-3009 payment,
    /// its nonce; the identifier from the payment record).
    pub payment_identifier: String,
    /// SHA-256(payment_identifier : payload_digest), base64url. See the
    /// byte-form note at the top of this file.
    pub pair_digest: String,
    /// Approval window, unix seconds. A releasing receipt's witnessed
    /// interval must cover [approval_opens_at, approval_closes_at].
    pub approval_opens_at: u64,
    pub approval_closes_at: u64,
    /// Locked funds, yoctoNEAR.
    pub amount: U128,
    pub status: SettlementStatus,
    /// Set when a receipt releases the settlement.
    pub receipt_id: Option<String>,
    pub receipt_stamped_digest: Option<String>,
    pub receipt_interval_earliest: Option<u64>,
    pub receipt_interval_latest: Option<u64>,
}

/// JSON view of a settlement, returned by `get_settlement`.
#[derive(Serialize, schemars::JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct SettlementView {
    pub payer: String,
    pub payee: String,
    pub payload_digest: String,
    pub payment_identifier: String,
    pub pair_digest: String,
    pub approval_opens_at: u64,
    pub approval_closes_at: u64,
    pub amount_yocto: String,
    pub status: String,
    pub receipt_id: Option<String>,
    pub receipt_stamped_digest: Option<String>,
    pub receipt_interval_earliest: Option<u64>,
    pub receipt_interval_latest: Option<u64>,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    settlements: UnorderedMap<String, Settlement>,
}

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            settlements: UnorderedMap::new(b"s".to_vec()),
        }
    }

    /// Open a settlement. The caller is the payer and attaches the funds to
    /// lock (testnet NEAR in this demo). Records the terms digest and the
    /// payment identifier, computes the pair_digest, and fixes the approval
    /// window the witness receipt must cover.
    #[payable]
    pub fn open_settlement(
        &mut self,
        settlement_id: String,
        payee: AccountId,
        payload_digest: String,
        payment_identifier: String,
        approval_opens_at: u64,
        approval_closes_at: u64,
    ) -> String {
        assert!(
            self.settlements.get(&settlement_id).is_none(),
            "settlement id already exists"
        );
        let amount = env::attached_deposit().as_yoctonear();
        assert!(amount > 0, "attach funds to lock in the settlement");
        assert!(
            approval_closes_at > approval_opens_at,
            "approval window must close after it opens"
        );
        assert!(!payload_digest.is_empty(), "payload_digest must not be empty");
        assert!(
            !payment_identifier.is_empty(),
            "payment_identifier must not be empty"
        );
        let pair_digest = compute_pair_digest(&payment_identifier, &payload_digest);
        let settlement = Settlement {
            payer: env::predecessor_account_id(),
            payee,
            payload_digest,
            payment_identifier,
            pair_digest: pair_digest.clone(),
            approval_opens_at,
            approval_closes_at,
            amount: U128(amount),
            status: SettlementStatus::Open,
            receipt_id: None,
            receipt_stamped_digest: None,
            receipt_interval_earliest: None,
            receipt_interval_latest: None,
        };
        self.settlements.insert(settlement_id.clone(), settlement);
        env::log_str(&format!(
            "settlement {settlement_id} opened, pair_digest {pair_digest}"
        ));
        pair_digest
    }

    /// Release the locked funds to the payee by presenting a POPCORN
    /// witness receipt's fields. Refuses (panics, funds stay locked) unless
    /// the stamped digest equals the settlement's pair_digest AND the
    /// receipt's witnessed interval covers the recorded approval window.
    ///
    /// The contract checks the fields it is given. It does not verify the
    /// receipt's signature; that check belongs to the offline checker
    /// against the published JWKS. See the trust seam note at the top.
    pub fn release(
        &mut self,
        settlement_id: String,
        receipt_id: String,
        stamped_digest: String,
        interval_earliest: u64,
        interval_latest: u64,
    ) -> String {
        let mut settlement = self
            .settlements
            .get(&settlement_id)
            .cloned()
            .unwrap_or_else(|| env::panic_str("unknown settlement id"));
        assert!(
            settlement.status == SettlementStatus::Open,
            "settlement is not open"
        );
        assert!(
            stamped_digest == settlement.pair_digest,
            "refused: receipt stamped digest does not equal the pair_digest"
        );
        assert!(
            interval_earliest <= settlement.approval_opens_at
                && interval_latest >= settlement.approval_closes_at,
            "refused: receipt witnessed interval does not cover the approval window"
        );
        settlement.status = SettlementStatus::Released;
        settlement.receipt_id = Some(receipt_id.clone());
        settlement.receipt_stamped_digest = Some(stamped_digest);
        settlement.receipt_interval_earliest = Some(interval_earliest);
        settlement.receipt_interval_latest = Some(interval_latest);
        let amount = settlement.amount.0;
        let payee = settlement.payee.clone();
        self.settlements.insert(settlement_id.clone(), settlement);
        env::log_str(&format!(
            "settlement {settlement_id} released to {payee} on receipt {receipt_id}"
        ));
        Promise::new(payee).transfer(NearToken::from_yoctonear(amount));
        receipt_id
    }

    /// The payer may reclaim locked funds from a settlement that has not
    /// been released. Once a matching receipt releases it, refund is over.
    pub fn refund(&mut self, settlement_id: String) {
        let mut settlement = self
            .settlements
            .get(&settlement_id)
            .cloned()
            .unwrap_or_else(|| env::panic_str("unknown settlement id"));
        assert!(
            settlement.status == SettlementStatus::Open,
            "settlement is not open"
        );
        assert!(
            env::predecessor_account_id() == settlement.payer,
            "only the payer can refund"
        );
        settlement.status = SettlementStatus::Refunded;
        let amount = settlement.amount.0;
        let payer = settlement.payer.clone();
        self.settlements.insert(settlement_id, settlement);
        Promise::new(payer).transfer(NearToken::from_yoctonear(amount));
    }

    /// Full settlement state, including the pair_digest and, after release,
    /// the receipt identifier that released it.
    pub fn get_settlement(&self, settlement_id: String) -> Option<SettlementView> {
        self.settlements.get(&settlement_id).map(|s| SettlementView {
            payer: s.payer.to_string(),
            payee: s.payee.to_string(),
            payload_digest: s.payload_digest.clone(),
            payment_identifier: s.payment_identifier.clone(),
            pair_digest: s.pair_digest.clone(),
            approval_opens_at: s.approval_opens_at,
            approval_closes_at: s.approval_closes_at,
            amount_yocto: s.amount.0.to_string(),
            status: match s.status {
                SettlementStatus::Open => "Open".to_string(),
                SettlementStatus::Released => "Released".to_string(),
                SettlementStatus::Refunded => "Refunded".to_string(),
            },
            receipt_id: s.receipt_id.clone(),
            receipt_stamped_digest: s.receipt_stamped_digest.clone(),
            receipt_interval_earliest: s.receipt_interval_earliest,
            receipt_interval_latest: s.receipt_interval_latest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    // Vector computed independently (Python hashlib) for
    // sha256(b"eip3009_demo_0001" + b":" + b"UKwdgKFq4m5a1Mm0qMSuhIJYnJ8jdmNd01uFdHaAE-M"),
    // base64url. Filled in by the test-vector check in ../checker/tests.
    const VECTOR_PAIR_DIGEST: &str = "iemMXR_sqXqRQPWhVSK7GNPgxWzUe_Ru7aJmp3D1akM";

    fn context(predecessor: AccountId, deposit: NearToken) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(predecessor)
            .attached_deposit(deposit);
        builder
    }

    fn open_one(contract: &mut Contract) -> String {
        testing_env!(context(accounts(0), NearToken::from_near(1)).build());
        contract.open_settlement(
            "s1".to_string(),
            accounts(1),
            "UKwdgKFq4m5a1Mm0qMSuhIJYnJ8jdmNd01uFdHaAE-M".to_string(),
            "eip3009_demo_0001".to_string(),
            1_000,
            2_000,
        )
    }

    #[test]
    fn pair_digest_matches_independent_vector() {
        let got = compute_pair_digest(
            "eip3009_demo_0001",
            "UKwdgKFq4m5a1Mm0qMSuhIJYnJ8jdmNd01uFdHaAE-M",
        );
        assert_eq!(got, VECTOR_PAIR_DIGEST);
    }

    #[test]
    fn open_records_pair_digest_and_window() {
        let mut contract = Contract::new();
        let pair = open_one(&mut contract);
        let s = contract.get_settlement("s1".to_string()).unwrap();
        assert_eq!(s.pair_digest, pair);
        assert_eq!(s.approval_opens_at, 1_000);
        assert_eq!(s.approval_closes_at, 2_000);
        assert_eq!(s.status, "Open");
        assert_eq!(s.amount_yocto, NearToken::from_near(1).as_yoctonear().to_string());
    }

    #[test]
    fn release_succeeds_on_matching_digest_and_covering_interval() {
        let mut contract = Contract::new();
        let pair = open_one(&mut contract);
        testing_env!(context(accounts(2), NearToken::from_near(0)).build());
        let rid = contract.release("s1".to_string(), "pwr_demo".to_string(), pair, 900, 2_100);
        assert_eq!(rid, "pwr_demo");
        let s = contract.get_settlement("s1".to_string()).unwrap();
        assert_eq!(s.status, "Released");
        assert_eq!(s.receipt_id.as_deref(), Some("pwr_demo"));
    }

    #[test]
    #[should_panic(expected = "stamped digest does not equal")]
    fn release_refuses_on_digest_mismatch() {
        let mut contract = Contract::new();
        open_one(&mut contract);
        testing_env!(context(accounts(2), NearToken::from_near(0)).build());
        contract.release(
            "s1".to_string(),
            "pwr_demo".to_string(),
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            900,
            2_100,
        );
    }

    #[test]
    #[should_panic(expected = "does not cover the approval window")]
    fn release_refuses_when_interval_does_not_cover_window() {
        let mut contract = Contract::new();
        let pair = open_one(&mut contract);
        testing_env!(context(accounts(2), NearToken::from_near(0)).build());
        contract.release("s1".to_string(), "pwr_demo".to_string(), pair, 1_500, 2_100);
    }

    #[test]
    #[should_panic(expected = "settlement is not open")]
    fn release_refuses_twice() {
        let mut contract = Contract::new();
        let pair = open_one(&mut contract);
        testing_env!(context(accounts(2), NearToken::from_near(0)).build());
        contract.release("s1".to_string(), "pwr_demo".to_string(), pair.clone(), 900, 2_100);
        contract.release("s1".to_string(), "pwr_demo".to_string(), pair, 900, 2_100);
    }
}
