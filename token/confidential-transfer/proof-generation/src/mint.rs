use {
    crate::{encryption::MintAmountCiphertext, errors::TokenProofGenerationError, try_split_u64},
    solana_zk_sdk::{
        encryption::{elgamal::ElGamalPubkey, pedersen::Pedersen},
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU64Data,
        },
    },
};

const MINT_AMOUNT_LO_BIT_LENGTH: usize = 16;
const MINT_AMOUNT_HI_BIT_LENGTH: usize = 32;
/// The padding bit length in range proofs to make the bit-length power-of-2
const RANGE_PROOF_PADDING_BIT_LENGTH: usize = 16;

/// The proof data required for a confidential mint instruction
pub struct MintProofData {
    pub ciphertext_validity_proof_data: BatchedGroupedCiphertext3HandlesValidityProofData,
    pub range_proof_data: BatchedRangeProofU64Data,
}

pub fn mint_split_proof_data(
    mint_amount: u64,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: &ElGamalPubkey,
    supply_elgamal_pubkey: &ElGamalPubkey,
) -> Result<MintProofData, TokenProofGenerationError> {
    // split the mint amount into low and high bits
    let (mint_amount_lo, mint_amount_hi) = try_split_u64(mint_amount, MINT_AMOUNT_LO_BIT_LENGTH)
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // encrypt the mint amount under the destination and auditor's ElGamal public
    // keys
    let (mint_amount_grouped_ciphertext_lo, mint_amount_opening_lo) = MintAmountCiphertext::new(
        mint_amount_lo,
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        supply_elgamal_pubkey,
    );

    let (mint_amount_grouped_ciphertext_hi, mint_amount_opening_hi) = MintAmountCiphertext::new(
        mint_amount_hi,
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        supply_elgamal_pubkey,
    );

    // generate ciphertext validity proof data
    let ciphertext_validity_proof_data = BatchedGroupedCiphertext3HandlesValidityProofData::new(
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        supply_elgamal_pubkey,
        &mint_amount_grouped_ciphertext_lo.0,
        &mint_amount_grouped_ciphertext_hi.0,
        mint_amount_lo,
        mint_amount_hi,
        &mint_amount_opening_lo,
        &mint_amount_opening_hi,
    )
    .map_err(TokenProofGenerationError::from)?;

    // generate range proof data
    let (padding_commitment, padding_opening) = Pedersen::new(0_u64);
    let range_proof_data = BatchedRangeProofU64Data::new(
        vec![
            mint_amount_grouped_ciphertext_lo.get_commitment(),
            mint_amount_grouped_ciphertext_hi.get_commitment(),
            &padding_commitment,
        ],
        vec![mint_amount_lo, mint_amount_hi, 0],
        vec![
            MINT_AMOUNT_LO_BIT_LENGTH,
            MINT_AMOUNT_HI_BIT_LENGTH,
            RANGE_PROOF_PADDING_BIT_LENGTH,
        ],
        vec![
            &mint_amount_opening_lo,
            &mint_amount_opening_hi,
            &padding_opening,
        ],
    )
    .map_err(TokenProofGenerationError::from)?;

    Ok(MintProofData {
        ciphertext_validity_proof_data,
        range_proof_data,
    })
}
