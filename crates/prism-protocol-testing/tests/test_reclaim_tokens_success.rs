use litesvm::LiteSVM;
use prism_protocol_testing::{FixtureStage, FixtureState, TestFixture};

/// Test successful token reclamation from permanently halted campaign
///
/// Should test:
/// - Set up permanently halted campaign with funded vaults
/// - Reclaim tokens to admin's token account
/// - Verify tokens transferred correctly from vaults to admin
/// - Verify proper authorization and account validation
#[tokio::test]
async fn test_reclaim_tokens_success() {
    let mut test = TestFixture::new(FixtureState::rand().await, LiteSVM::new())
        .await
        .unwrap();

    // 1. Set up active campaign and then permanently halt it
    test.jump_to(FixtureStage::CampaignActivated).await;

    test.try_permanently_halt_campaign()
        .await
        .expect("Should be able to permanently halt campaign");

    // 2. Reclaim tokens to admin's token account (should succeed)
    test.try_reclaim_tokens()
        .await
        .expect("Should be able to reclaim tokens from permanently halted campaign");

    println!("✅ Tokens successfully reclaimed from permanently halted campaign");
}
