use anchor_lang::prelude::*;
// use arcium_anchor::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::state::casino::*;

// Initialize casino
#[derive(Accounts)]
pub struct InitializeCasino<'info> {
    #[account(
        init,
        payer = authority,
        space = CasinoState::DISCRIMINATOR.len() + CasinoState::INIT_SPACE,
        seeds = [b"casino_state"],
        bump
    )]
    pub casino_state: Account<'info, CasinoState>,
    
    #[account(
        init,
        payer = authority,
        associated_token::mint = usdc_mint,
        associated_token::authority = casino_state,
        associated_token::token_program = token_program,
    )]
    pub casino_vault: InterfaceAccount<'info, TokenAccount>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub usdc_mint: InterfaceAccount<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl <'info> InitializeCasino<'info> {
    pub fn initialize_casino(&mut self, bumps: &InitializeCasinoBumps) -> Result<()> {
        self.casino_state.set_inner(CasinoState {
            authority: self.authority.key(),
            vault: self.casino_vault.key(),
            total_games_played: 0,
            total_volume: 0,
            total_payouts: 0,
            house_edge_config: HouseEdgeConfig {
                slots_rtp_bps: 9500,      // 95% RTP
                roulette_rtp_bps: 9730,   // 97.3% RTP
                aviator_rtp_bps: 9600,    // 96% RTP
                blackjack_rtp_bps: 9950,   // 99.5% RTP
                platform_fee_bps: 200,    // 2% platform fee
            },
            is_active: true,
            casino_state_bump: bumps.casino_state
        });
        
        msg!("Casino initialized successfully");
        Ok(())
    }
}
