/// Domain separation constants for merkle tree hashing
/// These must match the constants in the merkle crate to ensure compatibility
///
/// Domain separation prefix for leaf nodes
pub const LEAF_PREFIX: u8 = 0x00;

/// Domain separation prefix for internal nodes
pub const INTERNAL_PREFIX: u8 = 0x01;
