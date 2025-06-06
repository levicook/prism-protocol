use prism_protocol_entities::compiled_vaults;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;

pub trait CompiledVaultExt {
    fn vault_address(&self) -> Pubkey;
    fn cohort_address(&self) -> Pubkey;
    fn vault_index(&self) -> u8;
    fn vault_budget_human(&self) -> Decimal;
    fn vault_budget_token(&self) -> u64;
    fn vault_dust_human(&self) -> Decimal;
    fn vault_dust_token(&self) -> u64;
    fn amount_per_entitlement_human(&self) -> Decimal;
    fn amount_per_entitlement_token(&self) -> u64;
    fn total_entitlements(&self) -> Decimal;
}

impl CompiledVaultExt for compiled_vaults::Model {
    fn vault_address(&self) -> Pubkey {
        let vault_address = self.vault_address.parse::<Pubkey>();
        debug_assert!(
            vault_address.is_ok(),
            "Invalid vault address {}",
            self.vault_address
        );
        vault_address.unwrap()
    }

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

    fn vault_budget_human(&self) -> Decimal {
        let result = self.vault_budget_human.parse::<Decimal>();
        debug_assert!(
            result.is_ok(),
            "Invalid vault budget {}",
            self.vault_budget_human
        );
        result.unwrap()
    }

    fn vault_budget_token(&self) -> u64 {
        let result = self.vault_budget_token.parse::<u64>();
        debug_assert!(
            result.is_ok(),
            "Invalid vault budget {}",
            self.vault_budget_token
        );
        result.unwrap()
    }

    fn vault_dust_human(&self) -> Decimal {
        let result = self.vault_dust_human.parse::<Decimal>();
        debug_assert!(
            result.is_ok(),
            "Invalid vault dust {}",
            self.vault_dust_human
        );
        result.unwrap()
    }

    fn vault_dust_token(&self) -> u64 {
        let result = self.vault_dust_token.parse::<u64>();
        debug_assert!(
            result.is_ok(),
            "Invalid vault dust {}",
            self.vault_dust_token
        );
        result.unwrap()
    }

    fn amount_per_entitlement_human(&self) -> Decimal {
        let result = self.amount_per_entitlement_human.parse::<Decimal>();
        debug_assert!(
            result.is_ok(),
            "Invalid amount per entitlement {}",
            self.amount_per_entitlement_human
        );
        result.unwrap()
    }

    fn amount_per_entitlement_token(&self) -> u64 {
        let result = self.amount_per_entitlement_token.parse::<u64>();
        debug_assert!(
            result.is_ok(),
            "Invalid amount per entitlement {}",
            self.amount_per_entitlement_token
        );
        result.unwrap()
    }

    fn total_entitlements(&self) -> Decimal {
        let total_entitlements = self.total_entitlements.parse::<Decimal>();
        debug_assert!(
            total_entitlements.is_ok(),
            "Invalid total entitlements {}",
            self.total_entitlements
        );
        total_entitlements.unwrap()
    }
}
