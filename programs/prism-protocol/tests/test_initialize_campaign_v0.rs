#![cfg(all(feature = "testing"))]

use {
    anchor_lang::{prelude::AccountDeserialize as _, Space as _},
    prism_protocol::{
        self,
        state::CampaignV0,
        test_utils::TestFixture,
        ID as PRISM_PROGRAM_ID,
    },
};

#[test]
fn test_initialize_campaign_success() {
    // 1. Setup test fixture and initialize campaign
    let mut fixture = TestFixture::new();
    let campaign_result = fixture.initialize_campaign();

    // 2. Validate campaign account properties
    let campaign_account = &campaign_result.campaign_account;
    
    assert_eq!(
        campaign_account.owner,
        PRISM_PROGRAM_ID,
        "owner mismatch: expected: {:?}, actual: {:?}",
        PRISM_PROGRAM_ID,
        campaign_account.owner
    );

    assert_eq!(
        campaign_account.data.len(),
        CampaignV0::INIT_SPACE + 8,
        "account size mismatch: expected: {}, actual: {}",
        CampaignV0::INIT_SPACE + 8,
        campaign_account.data.len()
    );

    // 3. Validate campaign state
    let campaign_state = CampaignV0::try_deserialize(&mut campaign_account.data.as_slice())
        .expect("Failed to deserialize Campaign state");

    assert_eq!(
        campaign_state.admin, fixture.admin_address,
        "admin mismatch: expected: {}, actual: {}",
        fixture.admin_address, campaign_state.admin
    );

    assert_eq!(
        campaign_state.fingerprint, fixture.test_fingerprint,
        "fingerprint mismatch: expected: {:?}, actual: {:?}",
        fixture.test_fingerprint, campaign_state.fingerprint
    );

    assert_eq!(
        campaign_state.mint, fixture.mint,
        "mint mismatch: expected: {:?}, actual: {:?}",
        fixture.mint, campaign_state.mint
    );

    assert_eq!(
        campaign_state.is_active, true,
        "is_active mismatch: expected: {}, actual: {}",
        true, campaign_state.is_active
    );

    assert_eq!(
        campaign_state.bump, campaign_result.bump,
        "bump mismatch: expected: {}, actual: {}",
        campaign_result.bump, campaign_state.bump
    );

    println!("✅ Campaign state validation passed");
}
