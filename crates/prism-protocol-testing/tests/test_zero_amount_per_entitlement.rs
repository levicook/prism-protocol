use prism_protocol_testing::{FixtureStage, TestFixture};

#[test]
#[ignore]
fn test_zero_amount_per_entitlement() {
    let mut test = TestFixture::default();
    
    test.jump_to(FixtureStage::CampaignInitialized)
    .expect("campaign initialization failed");
    
    // Try to initialize cohort with zero amount per entitlement
    let result = test.jump_to(FixtureStage::CohortInitialized);
    
    assert!(result.is_err(), "Expected cohort initialization to fail with zero amount");
    println!("✅ Correctly rejected zero amount per entitlement");
} 