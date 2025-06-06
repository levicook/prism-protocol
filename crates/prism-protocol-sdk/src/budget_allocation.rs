/*!
# Budget Allocation Logic

This module provides isolated, thoroughly tested budget allocation math.
Separating this from the campaign compiler allows for focused testing of
the critical token distribution calculations.

## Allocation Hierarchy

The allocator supports two levels of allocation:
1. **Campaign → Cohort**: Allocate campaign budget to cohorts by percentage shares
2. **Cohort → Vault**: Allocate cohort budget to vaults by proportional entitlements

## Key Safety Features

- **Mint Decimals Aware**: Respects token mint constraints (0-28 decimals max)
- **Conservative Allocation**: Never over-allocates budget
- **Precise Math**: Uses rust_decimal with "go big before going small" for numerical stability
- **Overflow Protection**: Safe arithmetic throughout
- **Dust Tracking**: Accurately tracks unallocatable amounts due to precision constraints

## Example Usage

```rust
use prism_protocol_sdk::BudgetAllocator;
use rust_decimal::prelude::*;

// Create allocator for SOL (9 decimals)
let allocator = BudgetAllocator::new(
    dec!(1000.123456789), // Campaign budget
    9 // SOL decimals
).expect("Failed to create allocator");

// Step 1: Campaign → Cohort allocation
let cohort_allocation = allocator.calculate_cohort_allocation(
    dec!(33.333333333), // 1/3 of campaign
    dec!(7) // 7 total entitlements in cohort
).expect("Failed to calculate cohort allocation");

// Step 2: Cohort → Vault allocation
let vault_allocation = allocator.calculate_vault_allocation(
    cohort_allocation.cohort_budget, // Use cohort budget
    dec!(3), // 3 entitlements in this vault
    dec!(7) // 7 total entitlements in cohort
).expect("Failed to calculate vault allocation");

// Precision is maintained through both levels
assert!(vault_allocation.dust_amount >= dec!(0)); // Dust is tracked
```
*/

use rust_decimal::Decimal;
use thiserror::Error;

/// Errors that can occur during budget allocation
#[derive(Debug, Error)]
pub enum AllocationError {
    #[error("Budget allocation failed: {0}")]
    AllocationFailed(String),

    #[error("Max decimals exceeded: {0} (max 28 decimals supported)")]
    MaxDecimalsExceeded(u8),

    #[error("Invalid percentage: {0}% (must be 0-100)")]
    InvalidPercentage(Decimal),

    #[error("Calculation overflow: {0}")]
    Overflow(String),

    #[error("Zero entitlements not allowed")]
    ZeroEntitlements,

    #[error("Invalid entitlements: {0} (must be a positive whole number)")]
    InvalidEntitlements(Decimal),
}

pub type AllocationResult<T> = Result<T, AllocationError>;

/// Result of vault budget allocation calculation
#[derive(Debug, Clone, PartialEq)]
pub struct VaultAllocation {
    /// Total allocation for this vault (in budget tokens)
    pub vault_budget: Decimal,

    /// Amount per entitlement (respects mint decimals)
    pub amount_per_entitlement: Decimal,

    /// Amount that couldn't be allocated due to mint constraints
    pub dust_amount: Decimal,
}

/// Result of budget allocation calculation
#[derive(Debug, Clone, PartialEq)]
pub struct CohortAllocation {
    /// Total allocation for this cohort (in budget tokens)
    pub cohort_budget: Decimal,

    /// Amount per entitlement (respects mint decimals)
    pub amount_per_entitlement: Decimal,

    /// Amount that couldn't be allocated due to mint constraints
    pub dust_amount: Decimal,
}

/// Budget allocator with mint decimal constraints
///
/// Supports two-level allocation hierarchy:
/// 1. Campaign budget → Cohort budgets (by percentage shares)
/// 2. Cohort budget → Vault budgets (by proportional entitlements)
#[derive(Debug)]
pub struct BudgetAllocator {
    campaign_budget: Decimal,
    decimal_precision: Decimal,
}

impl BudgetAllocator {
    /// Create new allocator with budget and mint decimal constraints
    pub fn new(campaign_budget: Decimal, mint_decimals: u8) -> AllocationResult<Self> {
        // rust_decimal has a maximum precision of 28 decimal places
        if mint_decimals as u32 > Decimal::MAX_SCALE {
            return Err(AllocationError::MaxDecimalsExceeded(mint_decimals));
        }

        // Calculate the smallest unit for this mint (e.g., 0.000000001 for SOL)
        let decimal_precision = Decimal::new(1, mint_decimals as u32);

        Ok(Self {
            campaign_budget,
            decimal_precision,
        })
    }

    /// Calculate allocation for a cohort given share percentage and total entitlements
    pub fn calculate_cohort_allocation(
        &self,
        share_percentage: Decimal,
        total_entitlements: Decimal,
    ) -> AllocationResult<CohortAllocation> {
        // Validate inputs
        if share_percentage < Decimal::ZERO || share_percentage > Decimal::from(100) {
            return Err(AllocationError::InvalidPercentage(share_percentage));
        }

        if total_entitlements <= Decimal::ZERO {
            return Err(AllocationError::ZeroEntitlements);
        }

        // Entitlements must be a whole number (no fractional people/claims)
        if total_entitlements.fract() != Decimal::ZERO {
            return Err(AllocationError::InvalidEntitlements(total_entitlements));
        }

        // Calculate cohort's total allocation
        let cohort_budget = self.campaign_budget * (share_percentage / Decimal::ONE_HUNDRED);

        // Calculate raw amount per entitlement
        let raw_amount_per_entitlement = cohort_budget / total_entitlements;

        // Round down to nearest mint decimal unit to respect constraints
        let amount_per_entitlement = self.round_to_mint_precision(raw_amount_per_entitlement);

        // Calculate dust (amount lost due to rounding)
        let actual_total_allocated = amount_per_entitlement * total_entitlements;
        let dust_amount = cohort_budget - actual_total_allocated;

        Ok(CohortAllocation {
            cohort_budget,
            amount_per_entitlement,
            dust_amount,
        })
    }

    /// Calculate vault allocation from cohort budget based on proportional entitlements
    ///
    /// This allocates a portion of a cohort's budget to a specific vault based on how many
    /// entitlements that vault contains relative to the total entitlements in the cohort.
    ///
    /// Uses the "go big before going small" principle: `(cohort_budget * vault_entitlements) / total_entitlements`
    /// for maximum numerical precision.
    ///
    /// # Arguments
    ///
    /// * `cohort_budget` - Total budget available for the cohort
    /// * `vault_entitlements` - Number of entitlements in this vault (must be > 0)
    /// * `total_cohort_entitlements` - Total entitlements across all vaults in the cohort
    ///
    /// # Returns
    ///
    /// * `VaultAllocation` with budget, amount per entitlement, and any dust
    ///
    /// # Errors
    ///
    /// * `AllocationFailed` - If vault has zero entitlements (indicates logic error)
    /// * `InvalidEntitlements` - If entitlements are negative or fractional
    /// * `ZeroEntitlements` - If total cohort entitlements is zero
    pub fn calculate_vault_allocation(
        &self,
        cohort_budget: Decimal,
        vault_entitlements: Decimal,
        total_cohort_entitlements: Decimal,
    ) -> AllocationResult<VaultAllocation> {
        // Validate inputs
        if vault_entitlements < Decimal::ZERO {
            return Err(AllocationError::InvalidEntitlements(vault_entitlements));
        }

        if total_cohort_entitlements <= Decimal::ZERO {
            return Err(AllocationError::ZeroEntitlements);
        }

        // Entitlements must be whole numbers
        if vault_entitlements.fract() != Decimal::ZERO {
            return Err(AllocationError::InvalidEntitlements(vault_entitlements));
        }

        if total_cohort_entitlements.fract() != Decimal::ZERO {
            return Err(AllocationError::InvalidEntitlements(
                total_cohort_entitlements,
            ));
        }

        // Vault cannot have zero entitlements - would indicate logic error
        if vault_entitlements == Decimal::ZERO {
            return Err(AllocationError::AllocationFailed(
                "Vault with zero entitlements should not be created".to_string(),
            ));
        }

        // Calculate vault's proportional share of cohort budget
        // Use (budget * vault) / total to preserve precision (avoids intermediate fraction)
        let vault_budget = (cohort_budget * vault_entitlements) / total_cohort_entitlements;

        // Calculate raw amount per entitlement
        let raw_amount_per_entitlement = vault_budget / vault_entitlements;

        // Round down to nearest mint decimal unit to respect constraints
        let amount_per_entitlement = self.round_to_mint_precision(raw_amount_per_entitlement);

        // Calculate dust (amount lost due to rounding)
        let actual_total_allocated = amount_per_entitlement * vault_entitlements;
        let dust_amount = vault_budget - actual_total_allocated;

        Ok(VaultAllocation {
            vault_budget,
            amount_per_entitlement,
            dust_amount,
        })
    }

    /// Calculate total dust across all cohort allocations
    pub fn calculate_total_dust(
        &self,
        cohort_shares: &[(Decimal, Decimal)], // (share_percentage, total_entitlements)
    ) -> AllocationResult<Decimal> {
        let mut total_dust = Decimal::ZERO;

        for (share_percentage, total_entitlements) in cohort_shares {
            let allocation =
                self.calculate_cohort_allocation(*share_percentage, *total_entitlements)?;
            total_dust += allocation.dust_amount;
        }

        Ok(total_dust)
    }

    /// Round amount to mint's precision (conservative - rounds down)
    pub fn round_to_mint_precision(&self, amount: Decimal) -> Decimal {
        // Divide by precision, floor, then multiply back
        (amount / self.decimal_precision).floor() * self.decimal_precision
    }

    /// Get the allocator's budget and constraints
    pub fn budget(&self) -> Decimal {
        self.campaign_budget
    }

    pub fn decimal_precision(&self) -> Decimal {
        self.decimal_precision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn test_sol_allocation_simple() {
        let allocator = BudgetAllocator::new(
            dec!(1000), // 1000 SOL
            9,          // SOL decimals
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                dec!(50),  // 50%
                dec!(100), // 100 entitlements
            )
            .unwrap();

        // 50% of 1000 SOL = 500 SOL
        // 500 SOL / 100 entitlements = 5 SOL per entitlement
        assert_eq!(allocation.cohort_budget, dec!(500));
        assert_eq!(allocation.amount_per_entitlement, dec!(5));
        assert_eq!(allocation.dust_amount, dec!(0));
    }

    #[test]
    fn test_dust_calculation_with_indivisible_amount() {
        let allocator = BudgetAllocator::new(
            dec!(1000.123456789), // Precise SOL amount
            9,                    // SOL decimals
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                dec!(100), // 100%
                dec!(3),   // 3 entitlements (creates indivisible situation)
            )
            .unwrap();

        // 1000.123456789 / 3 = 333.374485596333... SOL per entitlement
        // Rounded down to: 333.374485596 SOL per entitlement
        // Actual allocated: 333.374485596 * 3 = 1000.123456788
        // Dust: 1000.123456789 - 1000.123456788 = 0.000000001 SOL

        assert_eq!(allocation.amount_per_entitlement, dec!(333.374485596));
        assert_eq!(allocation.dust_amount, dec!(0.000000001));
    }

    #[test]
    fn test_usdc_allocation() {
        let allocator = BudgetAllocator::new(
            dec!(10000.50), // 10000.50 USDC
            6,              // USDC decimals
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                dec!(25), // 25%
                dec!(7),  // 7 entitlements
            )
            .unwrap();

        // 25% of 10000.50 = 2500.125 USDC
        // 2500.125 / 7 = 357.160714... USDC per entitlement
        // Rounded to USDC precision: 357.160714 USDC per entitlement

        assert_eq!(allocation.cohort_budget, dec!(2500.125));
        assert_eq!(allocation.amount_per_entitlement, dec!(357.160714));
    }

    #[test]
    fn test_zero_decimal_token() {
        let allocator = BudgetAllocator::new(
            dec!(1000), // 1000 whole tokens
            0,          // No decimals (like some NFT tokens)
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                dec!(100), // 100%
                dec!(3),   // 3 entitlements
            )
            .unwrap();

        // 1000 / 3 = 333.333... but must round down to 333 whole tokens
        // Dust = 1000 - (333 * 3) = 1 token

        assert_eq!(allocation.amount_per_entitlement, dec!(333));
        assert_eq!(allocation.dust_amount, dec!(1));
    }

    #[test]
    fn test_invalid_inputs() {
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap();

        // Invalid percentage
        assert!(allocator
            .calculate_cohort_allocation(
                dec!(150), // > 100%
                dec!(100)
            )
            .is_err());

        // Zero entitlements
        assert!(allocator
            .calculate_cohort_allocation(
                dec!(50),
                dec!(0) // Zero entitlements
            )
            .is_err());

        // Fractional entitlements
        assert!(allocator
            .calculate_cohort_allocation(
                dec!(50),
                dec!(2.5) // Fractional entitlements - invalid
            )
            .is_err());

        // Invalid decimals (>28 due to rust_decimal limitation)
        assert!(BudgetAllocator::new(dec!(1000), 30).is_err());
    }

    #[test]
    fn test_high_decimal_precision() {
        // Test high precision (20 decimals) - safely below rust_decimal's 28 limit
        let allocator = BudgetAllocator::new(dec!(1000), 20).unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                dec!(100), // 100%
                dec!(2),   // 2 entitlements
            )
            .unwrap();

        // Should handle 20 decimal places without issues
        assert_eq!(allocation.cohort_budget, dec!(1000));
        assert_eq!(allocation.amount_per_entitlement, dec!(500));
        assert_eq!(allocation.dust_amount, dec!(0));
    }

    #[test]
    fn test_excessive_decimal_precision() {
        // Test that excessive precision (>28) fails gracefully with proper error
        let result = BudgetAllocator::new(dec!(1), 30);

        // Should return proper error, not panic
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::MaxDecimalsExceeded(30) => {
                // Expected error
            }
            _ => panic!("Expected InvalidDecimals error"),
        }
    }

    #[test]
    fn test_multiple_cohorts_dust_calculation() {
        let allocator = BudgetAllocator::new(dec!(1000.123456789), 9).unwrap();

        let cohorts = vec![
            (dec!(60), dec!(7)),  // 60% to 7 entitlements
            (dec!(40), dec!(11)), // 40% to 11 entitlements
        ];

        let total_dust = allocator.calculate_total_dust(&cohorts).unwrap();

        // Should have some dust due to indivisible allocations
        assert!(total_dust > Decimal::ZERO);
        // With SOL precision (9 decimals), dust should be small but may accumulate across cohorts
        // Expecting at most a few lamports (units of 10^-9)
        assert!(total_dust < dec!(0.00000002)); // 20 lamports max

        // Verify the actual dust amount is reasonable for SOL
        assert_eq!(total_dust, dec!(0.00000001)); // Exactly 10 lamports
    }

    #[test]
    fn test_fractional_entitlements_rejected() {
        // Test that fractional entitlements are properly rejected
        let allocator = BudgetAllocator::new(dec!(1000), 6).unwrap();

        let result = allocator.calculate_cohort_allocation(
            dec!(100), // 100%
            dec!(7.5), // 7.5 entitlements - should be invalid
        );

        // Should return InvalidEntitlements error
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::InvalidEntitlements(entitlements) => {
                assert_eq!(entitlements, dec!(7.5));
            }
            _ => panic!("Expected InvalidEntitlements error"),
        }
    }

    #[test]
    fn test_very_large_entitlements() {
        // Test numbers that would overflow u64 (18+ quintillion)
        // Use a larger budget to ensure we don't round to zero
        let allocator = BudgetAllocator::new(dec!(999999999999999999), 0).unwrap(); // Whole tokens, huge budget

        let very_large_entitlements = dec!(999999999999999); // Large but not so large we get zero

        let allocation = allocator
            .calculate_cohort_allocation(
                dec!(100), // 100%
                very_large_entitlements,
            )
            .unwrap();

        assert_eq!(allocation.cohort_budget, dec!(999999999999999999));
        // Should handle the calculation without overflow or panic
        assert!(allocation.amount_per_entitlement >= Decimal::ZERO);
        // Even if rounded to zero, the calculation should work
    }

    #[test]
    fn test_negative_entitlements_rejected() {
        // Test that negative entitlements are properly rejected
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap();

        let result = allocator.calculate_cohort_allocation(
            dec!(50), // 50%
            dec!(-5), // Negative entitlements - should be invalid
        );

        // Should return ZeroEntitlements error (we treat <=0 as zero)
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::ZeroEntitlements => {
                // Expected - negative is treated same as zero
            }
            _ => panic!("Expected ZeroEntitlements error"),
        }
    }

    #[test]
    fn test_vault_allocation_simple_case() {
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap();

        let cohort_budget = dec!(100); // 100 SOL for cohort
        let total_entitlements = dec!(10); // 10 total entitlements

        // Vault gets 5 entitlements (50%)
        let vault_allocation = allocator
            .calculate_vault_allocation(
                cohort_budget,
                dec!(5), // vault entitlements
                total_entitlements,
            )
            .unwrap();

        // Should get 50 SOL and 10 SOL per entitlement
        assert_eq!(vault_allocation.vault_budget, dec!(50));
        assert_eq!(vault_allocation.amount_per_entitlement, dec!(10));
        assert_eq!(vault_allocation.dust_amount, dec!(0));
    }

    #[test]
    fn test_vault_allocation_uneven_distribution() {
        let allocator = BudgetAllocator::new(dec!(1000), 6).unwrap(); // USDC precision

        let cohort_budget = dec!(1000.50); // 1000.50 USDC
        let total_entitlements = dec!(100); // 100 total entitlements

        // Vault with 30% of entitlements
        let vault_allocation = allocator
            .calculate_vault_allocation(
                cohort_budget,
                dec!(30), // vault entitlements
                total_entitlements,
            )
            .unwrap();

        // Vault gets 30% of budget = 300.15 USDC
        // 300.15 / 30 = 10.005 USDC per entitlement
        assert_eq!(vault_allocation.vault_budget, dec!(300.15));
        assert_eq!(vault_allocation.amount_per_entitlement, dec!(10.005));
        assert_eq!(vault_allocation.dust_amount, dec!(0));
    }

    #[test]
    fn test_vault_allocation_with_dust() {
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap(); // SOL precision

        let cohort_budget = dec!(100.123456789); // Precise SOL amount
        let total_entitlements = dec!(10);

        // Vault gets 3 out of 10 entitlements
        let vault_allocation = allocator
            .calculate_vault_allocation(
                cohort_budget,
                dec!(3), // vault entitlements
                total_entitlements,
            )
            .unwrap();

        // Calculate expected values
        let expected_budget = cohort_budget * dec!(3) / total_entitlements;
        let expected_amount_per_entitlement =
            allocator.round_to_mint_precision(expected_budget / dec!(3));
        let expected_dust = expected_budget - (expected_amount_per_entitlement * dec!(3));

        assert_eq!(vault_allocation.vault_budget, expected_budget);
        assert_eq!(
            vault_allocation.amount_per_entitlement,
            expected_amount_per_entitlement
        );
        assert_eq!(vault_allocation.dust_amount, expected_dust);
    }

    #[test]
    fn test_vault_allocation_empty_vault_errors() {
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap();

        let cohort_budget = dec!(500);
        let total_entitlements = dec!(100);

        // Empty vault (0 entitlements) should error
        let result = allocator.calculate_vault_allocation(
            cohort_budget,
            dec!(0), // No entitlements
            total_entitlements,
        );

        // Should return an error since empty vaults shouldn't be created
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_allocation_whole_token_dust() {
        let allocator = BudgetAllocator::new(dec!(1000), 0).unwrap(); // Whole tokens only

        let cohort_budget = dec!(10); // 10 whole tokens
        let total_entitlements = dec!(3);

        // Vault with 1 entitlement
        let vault_allocation = allocator
            .calculate_vault_allocation(cohort_budget, dec!(1), total_entitlements)
            .unwrap();

        // Basic checks: vault should get some budget and create dust
        assert!(vault_allocation.vault_budget > dec!(3));
        assert!(vault_allocation.vault_budget < dec!(4));
        assert_eq!(vault_allocation.amount_per_entitlement, dec!(3)); // Rounded down
        assert!(vault_allocation.dust_amount > dec!(0)); // Should have dust
    }

    #[test]
    fn test_vault_allocation_precision_fix() {
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap(); // SOL precision

        // Test case that was problematic before precision fix
        let cohort_budget = dec!(300); // 300 SOL for cohort
        let total_entitlements = dec!(30); // 30 total entitlements

        // Vault gets exactly 1/3 of entitlements
        let vault_allocation = allocator
            .calculate_vault_allocation(
                cohort_budget,
                dec!(10), // vault entitlements
                total_entitlements,
            )
            .unwrap();

        // Should be exactly 100 SOL now with precision fix
        // (300 * 10) / 30 = 3000 / 30 = 100 exactly
        assert_eq!(vault_allocation.vault_budget, dec!(100));
        assert_eq!(vault_allocation.amount_per_entitlement, dec!(10));
        assert_eq!(vault_allocation.dust_amount, dec!(0));
    }

    #[test]
    fn test_compound_precision_campaign_to_vault() {
        // Test end-to-end precision from campaign -> cohort -> vault
        let allocator = BudgetAllocator::new(dec!(1000.123456789), 9).unwrap(); // SOL precision

        // Step 1: Campaign -> Cohort allocation
        let cohort_allocation = allocator
            .calculate_cohort_allocation(
                dec!(33.333333333), // 1/3 of campaign (creates precision challenge)
                dec!(7),            // 7 entitlements
            )
            .unwrap();

        // Step 2: Cohort -> Vault allocation
        let vault_allocation = allocator
            .calculate_vault_allocation(
                cohort_allocation.cohort_budget,
                dec!(3), // 3 of 7 entitlements
                dec!(7),
            )
            .unwrap();

        // Should maintain precision through both levels
        assert!(vault_allocation.vault_budget > dec!(140)); // Rough check
        assert!(vault_allocation.vault_budget < dec!(150)); // Rough check
        assert!(vault_allocation.amount_per_entitlement > dec!(0)); // Should not round to zero

        // Most importantly: calculations should complete without error
    }

    #[test]
    fn test_extreme_precision_ratios() {
        let allocator = BudgetAllocator::new(dec!(1000000), 9).unwrap(); // SOL precision

        // Test extremely uneven distribution (1 out of 1000000 entitlements)
        let vault_allocation = allocator
            .calculate_vault_allocation(
                dec!(1000000), // 1M SOL cohort budget
                dec!(1),       // Just 1 entitlement
                dec!(1000000), // Out of 1M total
            )
            .unwrap();

        // Should get exactly 1 SOL: (1000000 * 1) / 1000000 = 1
        assert_eq!(vault_allocation.vault_budget, dec!(1));
        assert_eq!(vault_allocation.amount_per_entitlement, dec!(1));
        assert_eq!(vault_allocation.dust_amount, dec!(0));
    }

    #[test]
    fn test_tiny_amounts_dont_round_to_zero() {
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap(); // SOL precision

        // Very small cohort budget
        let tiny_budget = dec!(0.000000001); // 1 lamport

        let vault_allocation = allocator
            .calculate_vault_allocation(
                tiny_budget,
                dec!(1), // 1 entitlement
                dec!(1), // Out of 1 total
            )
            .unwrap();

        // Should preserve the tiny amount exactly
        assert_eq!(vault_allocation.vault_budget, dec!(0.000000001));
        assert_eq!(vault_allocation.amount_per_entitlement, dec!(0.000000001));
        assert_eq!(vault_allocation.dust_amount, dec!(0));
    }

    #[test]
    fn test_precision_with_different_mint_decimals() {
        // Test various common token decimals to ensure consistent behavior
        let test_cases = vec![
            (0, "whole tokens (NFTs)"),
            (6, "USDC"),
            (8, "Bitcoin"),
            (9, "SOL"),
            (18, "ETH/ERC20"),
        ];

        for (decimals, description) in test_cases {
            let allocator = BudgetAllocator::new(dec!(1000), decimals).unwrap();

            // Test a tricky division case
            let vault_allocation = allocator
                .calculate_vault_allocation(
                    dec!(1000), // Budget
                    dec!(3),    // 3 entitlements
                    dec!(7),    // Out of 7 total
                )
                .unwrap();

            // Should not panic and should respect decimal constraints
            assert!(vault_allocation.vault_budget > dec!(0));
            assert!(vault_allocation.amount_per_entitlement >= dec!(0));
            assert!(vault_allocation.dust_amount >= dec!(0));

            // Verify precision is respected (dust + allocated = budget)
            let total_allocated = vault_allocation.amount_per_entitlement * dec!(3);
            let reconstructed_budget = total_allocated + vault_allocation.dust_amount;
            assert_eq!(
                reconstructed_budget, vault_allocation.vault_budget,
                "Precision test failed for {}",
                description
            );
        }
    }

    #[test]
    fn test_dust_accumulation_bounds() {
        let allocator = BudgetAllocator::new(dec!(1000), 9).unwrap(); // SOL precision

        // Create a scenario with maximum possible dust per vault
        let cohort_budget = dec!(1000);
        let total_entitlements = dec!(3); // Prime number to maximize dust

        let mut total_dust = dec!(0);
        let mut total_allocated = dec!(0);

        // Allocate to 3 vaults with 1 entitlement each
        for vault_entitlements in [1, 1, 1] {
            let vault_allocation = allocator
                .calculate_vault_allocation(
                    cohort_budget,
                    Decimal::from(vault_entitlements),
                    total_entitlements,
                )
                .unwrap();

            total_dust += vault_allocation.dust_amount;
            total_allocated +=
                vault_allocation.amount_per_entitlement * Decimal::from(vault_entitlements);
        }

        // Total dust should be bounded (less than 1 lamport per vault typically)
        assert!(total_dust < dec!(0.000000003)); // 3 lamports max

        // Verify we don't over-allocate
        assert!(total_allocated + total_dust <= cohort_budget);
    }
}
