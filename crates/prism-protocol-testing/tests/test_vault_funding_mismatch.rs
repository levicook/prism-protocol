use prism_protocol_testing::{FixtureStage, TestFixture};

#[test]
#[ignore]
fn test_vault_funding_mismatch() {
    let mut test = TestFixture::default();

    // Setup campaign, cohort, and vault
    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    test.jump_to(FixtureStage::CohortInitialized)
        .expect("cohort initialization failed");

    test.jump_to(FixtureStage::VaultInitialized)
        .expect("vault initialization failed");

    todo!();
    // Try to activate vault with wrong expected balance
    // Note: The actual vault funding vs expected balance validation
    // depends on your program's implementation
    // let wrong_expected_balance = 999_999_999; // Different from what's actually funded
    // let result = test.jump_to(FixtureStage::VaultActivated { expected_balance: wrong_expected_balance,
    // });

    // This might pass if your TestFixture doesn't actually fund the vault yet
    // You may need to enhance TestFixture to do real token funding
    // match result {
    //     Ok(_) => {
    //         println!("⚠️  Vault activation succeeded - may need real funding logic in TestFixture")
    //     }
    //     Err(_) => println!("✅ Correctly rejected vault with funding mismatch"),
    // }
}
