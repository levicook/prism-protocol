use prism_protocol_entities::compiled_cohorts;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;

pub trait CompiledCohortExt {
    fn address(&self) -> Pubkey;
    fn merkle_root(&self) -> [u8; 32];
    fn vault_count(&self) -> u8;
    // fn total_entitlements(&self) -> Decimal;
    fn cohort_budget_human(&self) -> Decimal;
    fn cohort_budget_token(&self) -> u64;
    fn amount_per_entitlement_human(&self) -> Decimal;
    fn amount_per_entitlement_token(&self) -> u64;
    fn dust_amount_human(&self) -> Decimal;
    fn dust_amount_token(&self) -> u64;
}

impl CompiledCohortExt for compiled_cohorts::Model {
    fn address(&self) -> Pubkey {
        let address = self.address.parse::<Pubkey>();
        debug_assert!(address.is_ok(), "Invalid cohort address {}", self.address);
        address.unwrap()
    }

    fn merkle_root(&self) -> [u8; 32] {
        let merkle_root = hex::decode(self.merkle_root.as_bytes()).unwrap();
        debug_assert!(
            merkle_root.len() == 32,
            "Invalid merkle root {}",
            self.merkle_root
        );
        merkle_root.try_into().unwrap()
    }

    fn vault_count(&self) -> u8 {
        let vault_count = self.vault_count.parse::<u8>();
        debug_assert!(
            vault_count.is_ok(),
            "Invalid vault count {}",
            self.vault_count
        );
        vault_count.unwrap()
    }

    // fn total_entitlements(&self) -> Decimal {
    //     let total_entitlements = self.total_entitlements.parse::<Decimal>();
    //     debug_assert!(
    //         total_entitlements.is_ok(),
    //         "Invalid total entitlements {}",
    //         self.total_entitlements
    //     );
    //     total_entitlements.unwrap()
    // }

    fn cohort_budget_human(&self) -> Decimal {
        let result = self.cohort_budget_human.parse::<Decimal>();
        debug_assert!(
            result.is_ok(),
            "Invalid cohort budget {}",
            self.cohort_budget_human
        );
        result.unwrap()
    }

    fn cohort_budget_token(&self) -> u64 {
        let result = self.cohort_budget_token.parse::<u64>();
        debug_assert!(
            result.is_ok(),
            "Invalid cohort budget {}",
            self.cohort_budget_token
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

    fn dust_amount_human(&self) -> Decimal {
        let result = self.dust_amount_human.parse::<Decimal>();
        debug_assert!(
            result.is_ok(),
            "Invalid dust amount {}",
            self.dust_amount_human
        );
        result.unwrap()
    }

    fn dust_amount_token(&self) -> u64 {
        let result = self.dust_amount_token.parse::<u64>();
        debug_assert!(
            result.is_ok(),
            "Invalid dust amount {}",
            self.dust_amount_token
        );
        result.unwrap()
    }
}
