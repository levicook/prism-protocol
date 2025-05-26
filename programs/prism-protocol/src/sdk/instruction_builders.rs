use anchor_lang::solana_program::{
    instruction::Instruction, system_program::ID as SYSTEM_PROGRAM_ID,
};
use anchor_lang::{prelude::*, InstructionData as _};

use crate::ID as PRISM_PROGRAM_ID;

pub fn build_initialize_campaign_ix(
    admin: Pubkey,
    campaign: Pubkey,
    campaign_fingerprint: [u8; 32],
    mint: Pubkey,
) -> Result<(
    Instruction,
    crate::accounts::InitializeCampaignV0,
    crate::instruction::InitializeCampaignV0,
)> {
    let ix_accounts = crate::accounts::InitializeCampaignV0 {
        admin,
        campaign,
        system_program: SYSTEM_PROGRAM_ID,
    };

    let ix_data = crate::instruction::InitializeCampaignV0 {
        campaign_fingerprint,
        mint,
    };

    let ix = Instruction {
        program_id: PRISM_PROGRAM_ID,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}
