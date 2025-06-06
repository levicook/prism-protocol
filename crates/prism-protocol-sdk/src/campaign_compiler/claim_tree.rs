use prism_protocol::{ClaimLeaf, ClaimProof};
use prism_protocol_merkle::{create_claim_tree_v0, create_claim_tree_v1, ClaimTreeV0, ClaimTreeV1};
use solana_sdk::pubkey::Pubkey;

// TODO move this into the merkle crate:
pub struct ClaimTree {
    claim_tree_type: ClaimTreeType,
    v0_base: Option<ClaimTreeV0>,
    v1_base: Option<ClaimTreeV1>,
}

impl ClaimTree {
    pub fn root(&self) -> Option<[u8; 32]> {
        match self.claim_tree_type {
            ClaimTreeType::V0 => self.v0_base.as_ref().unwrap().root(),
            ClaimTreeType::V1 => self.v1_base.as_ref().unwrap().root(),
        }
    }

    pub fn claimant_leaf(
        &self,
        claimant: &Pubkey,
    ) -> Result<&ClaimLeaf, anchor_lang::prelude::Error> {
        match self.claim_tree_type {
            ClaimTreeType::V0 => self.v0_base.as_ref().unwrap().leaf_for_claimant(claimant),
            ClaimTreeType::V1 => self.v1_base.as_ref().unwrap().leaf_for_claimant(claimant),
        }
    }

    pub fn claimant_proof(
        &self,
        claimant: &Pubkey,
    ) -> Result<ClaimProof, anchor_lang::prelude::Error> {
        match self.claim_tree_type {
            ClaimTreeType::V0 => {
                let base = self
                    .v0_base
                    .as_ref()
                    .unwrap()
                    .proof_for_claimant(claimant)?;
                Ok(ClaimProof::from_binary(base))
            }
            ClaimTreeType::V1 => {
                let base = self
                    .v1_base
                    .as_ref()
                    .unwrap()
                    .proof_for_claimant(claimant)?;
                Ok(ClaimProof::from_wide(base))
            }
        }
    }
}

#[derive(Clone)]
pub enum ClaimTreeType {
    V0,
    V1,
}

impl ClaimTreeType {
    pub fn new_tree(
        &self,
        campaign: Pubkey,
        claimant_entitlements: &[(Pubkey, u64)],
        vault_count: u8,
    ) -> Result<ClaimTree, anchor_lang::prelude::Error> {
        match self {
            ClaimTreeType::V0 => {
                let base = create_claim_tree_v0(campaign, claimant_entitlements, vault_count)?;
                Ok(ClaimTree {
                    claim_tree_type: self.clone(),
                    v0_base: Some(base),
                    v1_base: None,
                })
            }
            ClaimTreeType::V1 => {
                let base = create_claim_tree_v1(campaign, claimant_entitlements, vault_count)?;
                Ok(ClaimTree {
                    claim_tree_type: self.clone(),
                    v0_base: None,
                    v1_base: Some(base),
                })
            }
        }
    }
}

impl std::fmt::Display for ClaimTreeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ClaimTreeType::*;
        let s = match self {
            V0 => "v0",
            V1 => "v1",
        };

        write!(f, "{}", s)
    }
}

impl std::str::FromStr for ClaimTreeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "v0" => Ok(ClaimTreeType::V0),
            "v1" => Ok(ClaimTreeType::V1),
            _ => Err(format!("Invalid claim tree type: {s}")),
        }
    }
}
