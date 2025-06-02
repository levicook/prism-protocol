use prism_protocol_testing::{FixtureStage, TestFixture};

#[test]
fn test_cohort_initialization_before_campaign() {
    let mut test = TestFixture::default();

    // Try to initialize cohort without initializing campaign first
    let result = test.jump_to(FixtureStage::CohortInitialized);

    // This should now succeed because fixture auto-creates missing campaign
    assert!(
        result.is_ok(),
        "Expected cohort initialization to succeed with auto-advancement"
    );

    // Verify that both campaign and cohort are now initialized
    let final_state = result.unwrap();
    assert!(matches!(
        final_state.stage,
        Some(FixtureStage::CohortInitialized { .. })
    ));
    assert!(
        final_state.campaign_fingerprint.is_some(),
        "Campaign should be auto-created"
    );
    assert!(final_state.mint.is_some(), "Mint should be auto-created");
    assert!(final_state.cohort.is_some(), "Cohort should be created");

    println!("✅ Auto-advancement successfully created campaign before cohort");
}
