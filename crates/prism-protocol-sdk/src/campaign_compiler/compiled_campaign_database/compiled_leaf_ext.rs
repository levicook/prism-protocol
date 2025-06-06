use prism_protocol_entities::compiled_leaves;
use solana_sdk::pubkey::Pubkey;

pub trait CompiledLeafExt {
    fn cohort_address(&self) -> Pubkey;
    fn vault_index(&self) -> u8;
    fn entitlements(&self) -> u64;
}

impl CompiledLeafExt for compiled_leaves::Model {
    fn cohort_address(&self) -> Pubkey {
        let cohort_address = self.cohort_address.parse::<Pubkey>();
        debug_assert!(
            cohort_address.is_ok(),
            "Invalid cohort address {}",
            self.cohort_address
        );
        cohort_address.unwrap()
    }

    fn vault_index(&self) -> u8 {
        let vault_index = self.vault_index.try_into();
        debug_assert!(
            vault_index.is_ok(),
            "Invalid vault index {}",
            self.vault_index
        );
        vault_index.unwrap()
    }

    fn entitlements(&self) -> u64 {
        let entitlements = self.entitlements.parse::<u64>();
        debug_assert!(
            entitlements.is_ok(),
            "Invalid entitlements {}",
            self.entitlements
        );
        entitlements.unwrap()
    }
}
