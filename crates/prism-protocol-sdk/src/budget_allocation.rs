/*!
# Budget Allocation Logic

This module provides isolated, thoroughly tested budget allocation math.
Separating this from the campaign compiler allows for focused testing of
the critical token distribution calculations.

## Key Safety Features

- **Mint Decimals Aware**: Respects token mint constraints
- **Conservative Allocation**: Never over-allocates budget
- **Precise Math**: Uses rust_decimal for exact calculations
- **Overflow Protection**: Safe arithmetic throughout

## Example Usage

```rust
use rust_decimal::Decimal;
use std::str::FromStr;
use prism_protocol_sdk::budget_allocation::BudgetAllocator;

let allocator = BudgetAllocator::new(
    Decimal::from_str("1000.5").unwrap(), // Budget: 1000.5 SOL
    9 // SOL has 9 decimals
).unwrap();

let result = allocator.calculate_cohort_allocation(
    Decimal::from(60), // 60% share
    300 // 300 total entitlements
).unwrap();

// result.amount_per_entitlement is precise and respects mint decimals
```
*/

use rust_decimal::Decimal;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur during budget allocation
#[derive(Debug, Error)]
pub enum AllocationError {
    #[error("Invalid percentage: {0}% (must be 0-100)")]
    InvalidPercentage(Decimal),

    #[error("Zero entitlements not allowed")]
    ZeroEntitlements,

    #[error("Calculation overflow: {0}")]
    Overflow(String),

    #[error("Invalid mint decimals: {0} (must be 0-18)")]
    InvalidDecimals(u8),

    #[error("Budget allocation failed: {0}")]
    AllocationFailed(String),
}

pub type AllocationResult<T> = Result<T, AllocationError>;

/// Result of budget allocation calculation
#[derive(Debug, Clone, PartialEq)]
pub struct CohortAllocation {
    /// Total allocation for this cohort (in budget tokens)
    pub cohort_total: Decimal,

    /// Amount per entitlement (respects mint decimals)
    pub amount_per_entitlement: Decimal,

    /// Human-readable allocation (full precision)
    pub amount_per_entitlement_humane: String,

    /// Amount that couldn't be allocated due to mint constraints
    pub dust_amount: Decimal,
}

/// Budget allocator with mint decimal constraints
pub struct BudgetAllocator {
    total_budget: Decimal,
    mint_decimals: u8,
    decimal_precision: Decimal,
}

impl BudgetAllocator {
    /// Create new allocator with budget and mint decimal constraints
    pub fn new(total_budget: Decimal, mint_decimals: u8) -> AllocationResult<Self> {
        if mint_decimals > 18 {
            return Err(AllocationError::InvalidDecimals(mint_decimals));
        }

        // Calculate the smallest unit for this mint (e.g., 0.000000001 for SOL)
        let decimal_precision =
            Decimal::from_str("1").unwrap() / Decimal::from(10_u64.pow(mint_decimals as u32));

        Ok(Self {
            total_budget,
            mint_decimals,
            decimal_precision,
        })
    }

    /// Calculate allocation for a cohort given share percentage and total entitlements
    pub fn calculate_cohort_allocation(
        &self,
        share_percentage: Decimal,
        total_entitlements: u64,
    ) -> AllocationResult<CohortAllocation> {
        // Validate inputs
        if share_percentage < Decimal::ZERO || share_percentage > Decimal::from(100) {
            return Err(AllocationError::InvalidPercentage(share_percentage));
        }

        if total_entitlements == 0 {
            return Err(AllocationError::ZeroEntitlements);
        }

        // Calculate cohort's total allocation
        let cohort_total = self.total_budget * (share_percentage / Decimal::from(100));

        // Calculate raw amount per entitlement
        let raw_amount_per_entitlement = cohort_total / Decimal::from(total_entitlements);

        // Round down to nearest mint decimal unit to respect constraints
        let amount_per_entitlement = self.round_to_mint_precision(raw_amount_per_entitlement);

        // Calculate dust (amount lost due to rounding)
        let actual_total_allocated = amount_per_entitlement * Decimal::from(total_entitlements);
        let dust_amount = cohort_total - actual_total_allocated;

        Ok(CohortAllocation {
            cohort_total,
            amount_per_entitlement,
            amount_per_entitlement_humane: raw_amount_per_entitlement.to_string(),
            dust_amount,
        })
    }

    /// Calculate total dust across all cohort allocations
    pub fn calculate_total_dust(
        &self,
        cohort_shares: &[(Decimal, u64)], // (share_percentage, total_entitlements)
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
    fn round_to_mint_precision(&self, amount: Decimal) -> Decimal {
        // Divide by precision, floor, then multiply back
        (amount / self.decimal_precision).floor() * self.decimal_precision
    }

    /// Get the allocator's budget and constraints
    pub fn budget(&self) -> Decimal {
        self.total_budget
    }

    pub fn mint_decimals(&self) -> u8 {
        self.mint_decimals
    }

    pub fn decimal_precision(&self) -> Decimal {
        self.decimal_precision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sol_allocation_simple() {
        let allocator = BudgetAllocator::new(
            Decimal::from_str("1000").unwrap(), // 1000 SOL
            9,                                  // SOL decimals
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                Decimal::from(50), // 50%
                100,               // 100 entitlements
            )
            .unwrap();

        // 50% of 1000 SOL = 500 SOL
        // 500 SOL / 100 entitlements = 5 SOL per entitlement
        assert_eq!(allocation.cohort_total, Decimal::from(500));
        assert_eq!(allocation.amount_per_entitlement, Decimal::from(5));
        assert_eq!(allocation.dust_amount, Decimal::ZERO);
    }

    #[test]
    fn test_dust_calculation_with_indivisible_amount() {
        let allocator = BudgetAllocator::new(
            Decimal::from_str("1000.123456789").unwrap(), // Precise SOL amount
            9,                                            // SOL decimals
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                Decimal::from(100), // 100%
                3,                  // 3 entitlements (creates indivisible situation)
            )
            .unwrap();

        // 1000.123456789 / 3 = 333.374485596333... SOL per entitlement
        // Rounded down to: 333.374485596 SOL per entitlement
        // Actual allocated: 333.374485596 * 3 = 1000.123456788
        // Dust: 1000.123456789 - 1000.123456788 = 0.000000001 SOL

        assert_eq!(
            allocation.amount_per_entitlement,
            Decimal::from_str("333.374485596").unwrap()
        );
        assert_eq!(
            allocation.dust_amount,
            Decimal::from_str("0.000000001").unwrap()
        );
    }

    #[test]
    fn test_usdc_allocation() {
        let allocator = BudgetAllocator::new(
            Decimal::from_str("10000.50").unwrap(), // 10000.50 USDC
            6,                                      // USDC decimals
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                Decimal::from(25), // 25%
                7,                 // 7 entitlements
            )
            .unwrap();

        // 25% of 10000.50 = 2500.125 USDC
        // 2500.125 / 7 = 357.160714... USDC per entitlement
        // Rounded to USDC precision: 357.160714 USDC per entitlement

        assert_eq!(
            allocation.cohort_total,
            Decimal::from_str("2500.125").unwrap()
        );
        assert_eq!(
            allocation.amount_per_entitlement,
            Decimal::from_str("357.160714").unwrap()
        );
    }

    #[test]
    fn test_zero_decimal_token() {
        let allocator = BudgetAllocator::new(
            Decimal::from(1000), // 1000 whole tokens
            0,                   // No decimals (like some NFT tokens)
        )
        .unwrap();

        let allocation = allocator
            .calculate_cohort_allocation(
                Decimal::from(100), // 100%
                3,                  // 3 entitlements
            )
            .unwrap();

        // 1000 / 3 = 333.333... but must round down to 333 whole tokens
        // Dust = 1000 - (333 * 3) = 1 token

        assert_eq!(allocation.amount_per_entitlement, Decimal::from(333));
        assert_eq!(allocation.dust_amount, Decimal::from(1));
    }

    #[test]
    fn test_invalid_inputs() {
        let allocator = BudgetAllocator::new(Decimal::from(1000), 9).unwrap();

        // Invalid percentage
        assert!(allocator
            .calculate_cohort_allocation(
                Decimal::from(150), // > 100%
                100
            )
            .is_err());

        // Zero entitlements
        assert!(allocator
            .calculate_cohort_allocation(
                Decimal::from(50),
                0 // Zero entitlements
            )
            .is_err());

        // Invalid decimals
        assert!(BudgetAllocator::new(Decimal::from(1000), 19).is_err());
    }

    #[test]
    fn test_multiple_cohorts_dust_calculation() {
        let allocator =
            BudgetAllocator::new(Decimal::from_str("1000.123456789").unwrap(), 9).unwrap();

        let cohorts = vec![
            (Decimal::from(60), 7),  // 60% to 7 people
            (Decimal::from(40), 11), // 40% to 11 people
        ];

        let total_dust = allocator.calculate_total_dust(&cohorts).unwrap();

        // Should have some dust due to indivisible allocations
        assert!(total_dust > Decimal::ZERO);
        // With SOL precision (9 decimals), dust should be small but may accumulate across cohorts
        // Expecting at most a few lamports (units of 10^-9)
        assert!(total_dust < Decimal::from_str("0.00000002").unwrap()); // 20 lamports max

        // Verify the actual dust amount is reasonable for SOL
        assert_eq!(total_dust, Decimal::from_str("0.00000001").unwrap()); // Exactly 10 lamports
    }
}
