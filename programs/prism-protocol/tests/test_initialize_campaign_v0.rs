#![cfg(feature = "test-sbf")]

use {
    anchor_lang::prelude::AccountDeserialize,
    prism_protocol::state::CampaignV0,
    prism_protocol_testing::TestFixture,
    solana_sdk::signature::{Keypair, Signer},
};

#[test]
fn test_initialize_campaign_v0() {
    let mut fixture = TestFixture::new();

    // Create a test mint
    let mint_keypair = Keypair::new();
    let _mint_account = fixture.create_mint(&mint_keypair, 9);
    let mint = mint_keypair.pubkey();

    let campaign_result = fixture.initialize_campaign(mint);

    // Verify the campaign was created correctly
    let campaign_account = &campaign_result.campaign_account;
    assert_eq!(campaign_account.owner, prism_protocol::ID);

    // Deserialize and verify the campaign state
    let campaign_state = CampaignV0::try_deserialize(&mut campaign_account.data.as_slice())
        .expect("Failed to deserialize campaign state");

    assert_eq!(campaign_state.admin, fixture.admin_address);
    assert_eq!(campaign_state.fingerprint, fixture.test_fingerprint);
    assert_eq!(campaign_state.bump, campaign_result.bump);
    assert!(!campaign_state.is_active); // Should start inactive

    // Verify mint matches
    assert_eq!(
        campaign_state.mint, mint,
        "Campaign mint should match the provided mint"
    );
    assert_eq!(
        mint, campaign_state.mint,
        "Provided mint should match campaign mint"
    );

    println!("✅ Campaign initialized successfully");
    println!("   - Address: {}", campaign_result.address);
    println!("   - Admin: {}", campaign_state.admin);
    println!("   - Mint: {}", campaign_state.mint);
    println!("   - Fingerprint: {:?}", campaign_state.fingerprint);
    println!("   - Is Active: {}", campaign_state.is_active);
}
