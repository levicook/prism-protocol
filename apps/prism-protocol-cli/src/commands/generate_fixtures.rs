use crate::error::{CliError, CliResult};
use csv::Writer;
use solana_sdk::pubkey::Pubkey;
use std::path::PathBuf;

/// Generate deterministic test fixtures for benchmarking
pub fn execute(
    count: u64,
    seed: u64,
    output: PathBuf,
    distribution: String,
    min_entitlements: u64,
    max_entitlements: u64,
) -> CliResult<()> {
    println!("Generating {} fixtures with seed {}", count, seed);
    println!("Distribution: {}", distribution);
    println!("Entitlements range: {} - {}", min_entitlements, max_entitlements);
    
    // Create CSV writer
    let mut writer = Writer::from_path(&output)?;
    
    // Write header
    writer.write_record(&["claimant", "entitlements"])?;
    
    // Generate deterministic random number generator
    let mut rng_state = seed;
    
    for i in 0..count {
        // Generate deterministic pubkey
        let claimant = generate_deterministic_pubkey(seed, i);
        
        // Generate entitlements based on distribution
        let entitlements = match distribution.as_str() {
            "uniform" => {
                // Simple uniform distribution
                let range = max_entitlements - min_entitlements + 1;
                min_entitlements + (simple_rng(&mut rng_state) % range)
            }
            "realistic" => {
                // Weighted towards lower values (more realistic)
                generate_realistic_entitlements(&mut rng_state, min_entitlements, max_entitlements)
            }
            "exponential" => {
                // Exponential decay distribution
                generate_exponential_entitlements(&mut rng_state, min_entitlements, max_entitlements)
            }
            _ => {
                return Err(CliError::InvalidConfig(format!(
                    "Unknown distribution type: {}. Valid options: uniform, realistic, exponential",
                    distribution
                )));
            }
        };
        
        // Write record
        writer.write_record(&[claimant.to_string(), entitlements.to_string()])?;
        
        // Progress indicator for large datasets
        if count > 10_000 && i % 10_000 == 0 {
            println!("Generated {} / {} fixtures", i, count);
        }
    }
    
    writer.flush()?;
    
    println!("✅ Generated {} fixtures to {}", count, output.display());
    println!("📊 Distribution: {}", distribution);
    println!("🎯 Entitlements range: {} - {}", min_entitlements, max_entitlements);
    
    Ok(())
}

/// Generate a deterministic pubkey from seed and index
fn generate_deterministic_pubkey(seed: u64, index: u64) -> Pubkey {
    // Create a deterministic 32-byte array
    let mut bytes = [0u8; 32];
    
    // Mix seed and index to create unique but deterministic bytes
    let combined = seed.wrapping_mul(31).wrapping_add(index);
    
    // Fill bytes with deterministic pattern
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = ((combined.wrapping_mul(i as u64 + 1)) >> (i % 8)) as u8;
    }
    
    // Ensure it's a valid pubkey (not all zeros, not all 255s)
    if bytes.iter().all(|&b| b == 0) {
        bytes[0] = 1;
    }
    if bytes.iter().all(|&b| b == 255) {
        bytes[31] = 254;
    }
    
    Pubkey::new_from_array(bytes)
}

/// Simple deterministic RNG (Linear Congruential Generator)
fn simple_rng(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(1103515245).wrapping_add(12345);
    *state
}

/// Generate realistic entitlements (weighted towards lower values)
fn generate_realistic_entitlements(rng_state: &mut u64, min: u64, max: u64) -> u64 {
    let range = max - min + 1;
    
    // Use inverse exponential to weight towards lower values
    let random_val = simple_rng(rng_state) as f64 / u64::MAX as f64;
    let weighted = 1.0 - (-random_val * 2.0).exp(); // Exponential decay
    
    min + (weighted * range as f64) as u64
}

/// Generate exponential distribution entitlements
fn generate_exponential_entitlements(rng_state: &mut u64, min: u64, max: u64) -> u64 {
    let range = max - min + 1;
    
    // Exponential distribution with lambda = 2
    let random_val = simple_rng(rng_state) as f64 / u64::MAX as f64;
    let exponential = (-random_val.ln() / 2.0).min(1.0); // Cap at 1.0
    
    min + (exponential * range as f64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    
    #[test]
    fn test_deterministic_pubkey_generation() {
        // Same seed and index should produce same pubkey
        let pubkey1 = generate_deterministic_pubkey(42, 0);
        let pubkey2 = generate_deterministic_pubkey(42, 0);
        assert_eq!(pubkey1, pubkey2);
        
        // Different indices should produce different pubkeys
        let pubkey3 = generate_deterministic_pubkey(42, 1);
        assert_ne!(pubkey1, pubkey3);
        
        // Different seeds should produce different pubkeys
        let pubkey4 = generate_deterministic_pubkey(43, 0);
        assert_ne!(pubkey1, pubkey4);
    }
    
    #[test]
    fn test_pubkey_uniqueness() {
        let mut pubkeys = HashSet::new();
        
        // Generate 1000 pubkeys and ensure they're all unique
        for i in 0..1000 {
            let pubkey = generate_deterministic_pubkey(42, i);
            assert!(pubkeys.insert(pubkey), "Duplicate pubkey found at index {}", i);
        }
    }
    
    #[test]
    fn test_entitlements_in_range() {
        let mut rng_state = 42;
        let min = 10;
        let max = 100;
        
        for _ in 0..1000 {
            let entitlements = generate_realistic_entitlements(&mut rng_state, min, max);
            assert!(entitlements >= min && entitlements <= max);
        }
    }
} 