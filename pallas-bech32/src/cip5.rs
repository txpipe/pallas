const ACCT_SK: &'static str = "acct_sk";
const ACCT_VK: &'static str = "acct_vk";
const ACCT_XSK: &'static str = "acct_xsk";
const ACCT_XVK: &'static str = "acct_xvk";
const ACCT_SHARED_SK: &'static str = "acct_shared_sk";
const ACCT_SHARED_VK: &'static str = "acct_shared_vk";
const ACCT_SHARED_XSK: &'static str = "acct_shared_xsk";
const ACCT_SHARED_XVK: &'static str = "acct_shared_xvk";
const ADDR_SK: &'static str = "addr_sk";
const ADDR_VK: &'static str = "addr_vk";
const ADDR_XSK: &'static str = "addr_xsk";
const ADDR_XVK: &'static str = "addr_xvk";
const ADDR_SHARED_SK: &'static str = "addr_shared_sk";
const ADDR_SHARED_VK: &'static str = "addr_shared_vk";
const ADDR_SHARED_XSK: &'static str = "addr_shared_xsk";
const ADDR_SHARED_XVK: &'static str = "addr_shared_xvk";
const KES_SK: &'static str = "kes_sk";
const KES_VK: &'static str = "kes_vk";
const POLICY_SK: &'static str = "policy_sk";
const POLICY_VK: &'static str = "policy_vk";
const POOL_SK: &'static str = "pool_sk";
const POOL_VK: &'static str = "pool_vk";
const ROOT_SK: &'static str = "root_sk";
const ROOT_VK: &'static str = "root_vk";
const ROOT_XSK: &'static str = "root_xsk";
const ROOT_XVK: &'static str = "root_xvk";
const ROOT_SHARED_SK: &'static str = "root_shared_sk";
const ROOT_SHARED_VK: &'static str = "root_shared_vk";
const ROOT_SHARED_XSK: &'static str = "root_shared_xsk";
const ROOT_SHARED_XVK: &'static str = "root_shared_xvk";
const STAKE_SK: &'static str = "stake_sk";
const STAKE_VK: &'static str = "stake_vk";
const STAKE_XSK: &'static str = "stake_xsk";
const STAKE_XVK: &'static str = "stake_xvk";
const STAKE_SHARED_SK: &'static str = "stake_shared_sk";
const STAKE_SHARED_VK: &'static str = "stake_shared_vk";
const STAKE_SHARED_XSK: &'static str = "stake_shared_xsk";
const STAKE_SHARED_XVK: &'static str = "stake_shared_xvk";
const VRF_SK: &'static str = "vrf_sk";
const VRF_VK: &'static str = "vrf_vk";

pub struct Keys<'a> {
    acct_sk: &'a str,
    acct_vk: &'a str,
    acct_xsk: &'a str,
    acct_xvk: &'a str,
    acct_shared_sk: &'a str,
    acct_shared_vk: &'a str,
    acct_shared_xsk: &'a str,
    acct_shared_xvk: &'a str,
    addr_sk: &'a str,
    addr_vk: &'a str,
    addr_xsk: &'a str,
    addr_xvk: &'a str,
    addr_shared_sk: &'a str,
    addr_shared_vk: &'a str,
    addr_shared_xsk: &'a str,
    addr_shared_xvk: &'a str,
    kes_sk: &'a str,
    kes_vk: &'a str,
    policy_sk: &'a str,
    policy_vk: &'a str,
    pool_sk: &'a str,
    pool_vk: &'a str,
    root_sk: &'a str,
    root_vk: &'a str,
    root_xsk: &'a str,
    root_xvk: &'a str,
    root_shared_sk: &'a str,
    root_shared_vk: &'a str,
    root_shared_xsk: &'a str,
    root_shared_xvk: &'a str,
    stake_sk: &'a str,
    stake_vk: &'a str,
    stake_xsk: &'a str,
    stake_xvk: &'a str,
    stake_shared_sk: &'a str,
    stake_shared_vk: &'a str,
    stake_shared_xsk: &'a str,
    stake_shared_xvk: &'a str,
    vrf_sk: &'a str,
    vrf_vk: &'a str
}

pub const KEYS: Keys<'static> = Keys {
    acct_sk : ACCT_SK,
    acct_vk : ACCT_VK,
    acct_xsk : ACCT_XSK,
    acct_xvk : ACCT_XVK,
    acct_shared_sk : ACCT_SHARED_SK,
    acct_shared_vk : ACCT_SHARED_VK,
    acct_shared_xsk : ACCT_SHARED_XSK,
    acct_shared_xvk : ACCT_SHARED_XVK,
    addr_sk : ADDR_SK,
    addr_vk : ADDR_VK,
    addr_xsk : ADDR_XSK,
    addr_xvk : ADDR_XVK,
    addr_shared_sk : ADDR_SHARED_SK,
    addr_shared_vk : ADDR_SHARED_VK,
    addr_shared_xsk : ADDR_SHARED_XSK,
    addr_shared_xvk : ADDR_SHARED_XVK,
    kes_sk: KES_SK,
    kes_vk: KES_VK,
    policy_sk: POLICY_SK,
    policy_vk: POLICY_VK,
    pool_sk: POOL_SK,
    pool_vk: POOL_VK,
    root_sk: ROOT_SK,
    root_vk: ROOT_VK,
    root_xsk: ROOT_XSK,
    root_xvk: ROOT_XVK,
    root_shared_sk: ROOT_SHARED_SK,
    root_shared_vk: ROOT_SHARED_VK,
    root_shared_xsk: ROOT_SHARED_XSK,
    root_shared_xvk: ROOT_SHARED_XVK,
    stake_sk: STAKE_SK,
    stake_vk: STAKE_VK,
    stake_xsk: STAKE_XSK,
    stake_xvk: STAKE_XVK,
    stake_shared_sk: STAKE_SHARED_SK,
    stake_shared_vk: STAKE_SHARED_VK,
    stake_shared_xsk: STAKE_SHARED_XSK,
    stake_shared_xvk: STAKE_SHARED_XVK,
    vrf_sk: VRF_SK,
    vrf_vk: VRF_VK
};

const ASSET: &'static str = "asset";
const POOL: &'static str = "pool";
const SCRIPT: &'static str = "script";
const ADDR_VKH: &'static str = "addr_vkh";
const ADDR_SHARED_VKH: &'static str = "addr_shared_vkh";
const POLICY_VKH: &'static str = "policy_vkh";
const STAKE_VKH: &'static str = "stake_vkh";
const STAKE_SHARED_VKH: &'static str = "stake_shared_vkh";
const VRF_VKH: &'static str = "vrf_vkh";
pub struct Hashes<'a> {
    asset: &'a str,
    pool: &'a str,
    script: &'a str,
    addr_vkh: &'a str,
    addr_shared_vkh: &'a str,
    policy_vkh: &'a str,
    stake_vkh: &'a str,
    stake_shared_vkh: &'a str,
    vrf_vkh: &'a str
}

pub const HASHES: Hashes<'static> = Hashes {
    asset : ASSET,
    pool : POOL,
    script : SCRIPT,
    addr_vkh : ADDR_VKH,
    addr_shared_vkh : ADDR_SHARED_VKH,
    policy_vkh : POLICY_VKH,
    stake_vkh : STAKE_VKH,
    stake_shared_vkh : STAKE_SHARED_VKH,
    vrf_vkh : VRF_VKH
};


const ADDR: &'static str = "addr";
const ADDR_TEST: &'static str = "addr_test";
const STAKE: &'static str = "stake";
const STAKE_TEST: &'static str = "stake_test";

pub struct Miscellaneous<'a> {
    addr: &'a str,
    addr_test: &'a str,
    stake: &'a str,
    stake_test: &'a str
}


pub const MISCELLANEOUS: Miscellaneous<'static> = Miscellaneous {
    addr: ADDR,
    addr_test: ADDR_TEST,
    stake: STAKE,
    stake_test: STAKE_TEST
};

#[cfg(test)]
mod tests {
    use crate::cip5::*;

    #[test]
    fn hashes_prefix_is_properly_set() {
        assert_eq!(HASHES.asset, "asset");
    }


    #[test]
    fn keys_prefix_is_properly_set() {
        assert_eq!(KEYS.acct_shared_sk, "acct_shared_sk");
    }

    #[test]
    fn asset_prefix_is_properly_set() {
        assert_eq!(MISCELLANEOUS.addr, "addr");
    }
}