use std::cmp::Ordering;

use pallas_primitives::alonzo;

use crate::MultiEraWithdrawals;

/// The ledger's order of two raw reward accounts: network, then script credentials before key credentials, then hash.
pub fn cmp_reward_accounts(a: &[u8], b: &[u8]) -> Ordering {
    fn key(raw: &[u8]) -> (Option<u8>, Option<u8>, Option<&[u8]>) {
        (
            raw.first().map(|x| x & 15),
            raw.first().map(|x| 15 - (x >> 4)),
            raw.get(1..),
        )
    }

    key(a).cmp(&key(b))
}

impl MultiEraWithdrawals<'_> {
    pub fn as_alonzo(&self) -> Option<&alonzo::Withdrawals> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MultiEraWithdrawals::AlonzoCompatible(x) => x.is_empty(),
            _ => true,
        }
    }

    pub fn collect<'a, T>(&'a self) -> T
    where
        T: FromIterator<(&'a [u8], u64)>,
    {
        match self {
            MultiEraWithdrawals::NotApplicable => std::iter::empty().collect(),
            MultiEraWithdrawals::Empty => std::iter::empty().collect(),
            MultiEraWithdrawals::AlonzoCompatible(x) => {
                x.iter().map(|(k, v)| (k.as_slice(), *v)).collect()
            }
            MultiEraWithdrawals::Conway(x) => x.iter().map(|(k, v)| (k.as_slice(), *v)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Era, MultiEraTx};

    const KEY: u8 = 0xe0;
    const SCRIPT: u8 = 0xf0;

    fn account(header: u8, fill: u8) -> Vec<u8> {
        let mut account = vec![fill; 29];
        account[0] = header;
        account
    }

    /// A Conway transaction whose withdrawal map holds a script account before a key account.
    fn tx_with_withdrawals() -> Vec<u8> {
        hex::decode(format!(
            "84a400d9010281825820{}000180021903e805a2581d{}0a581d{}0aa0f5f6",
            "11".repeat(32),
            hex::encode(account(SCRIPT, 2)),
            hex::encode(account(KEY, 1)),
        ))
        .unwrap()
    }

    #[test]
    fn the_sorted_set_orders_withdrawals_by_reward_account_bytes() {
        let cbor = tx_with_withdrawals();
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).unwrap();
        let accounts: Vec<_> = tx
            .withdrawals_sorted_set()
            .into_iter()
            .map(|(account, _)| account.to_vec())
            .collect();

        assert_eq!(accounts, vec![account(KEY, 1), account(SCRIPT, 2)]);
    }

    #[test]
    fn the_ledger_orders_reward_accounts_by_network_then_script_first() {
        assert_eq!(
            cmp_reward_accounts(&account(SCRIPT, 2), &account(KEY, 1)),
            Ordering::Less
        );
        assert_eq!(
            cmp_reward_accounts(&account(SCRIPT | 1, 0), &account(KEY, 1)),
            Ordering::Greater
        );
    }
}
