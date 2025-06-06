use prism_protocol_entities::compiled_campaigns;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;

use crate::ClaimTreeType;

pub trait CompiledCampaignExt {
    fn address(&self) -> Pubkey;
    fn campaign_budget_human(&self) -> Decimal;
    fn campaign_budget_token(&self) -> u64;
    fn claim_tree_type(&self) -> ClaimTreeType;
}

impl CompiledCampaignExt for compiled_campaigns::Model {
    fn address(&self) -> Pubkey {
        let address = self.address.parse::<Pubkey>();
        debug_assert!(address.is_ok(), "Invalid campaign address {}", self.address);
        address.unwrap()
    }

    fn campaign_budget_human(&self) -> Decimal {
        let result = self.campaign_budget_human.parse::<Decimal>();
        debug_assert!(
            result.is_ok(),
            "Invalid campaign budget {}",
            self.campaign_budget_human
        );
        result.unwrap()
    }

    fn campaign_budget_token(&self) -> u64 {
        let result = self.campaign_budget_token.parse::<u64>();
        debug_assert!(
            result.is_ok(),
            "Invalid campaign budget {}",
            self.campaign_budget_token
        );
        result.unwrap()
    }

    fn claim_tree_type(&self) -> ClaimTreeType {
        let claim_tree_type = self.claim_tree_type.parse::<ClaimTreeType>();
        debug_assert!(
            claim_tree_type.is_ok(),
            "Invalid claim tree type {}",
            self.claim_tree_type
        );
        claim_tree_type.unwrap()
    }
}
