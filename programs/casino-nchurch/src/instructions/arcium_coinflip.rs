 use anchor_lang::prelude::*;
 use arcium_anchor::prelude::*;
 use arcium_client::idl::arcium::types::CallbackAccount;
 use anchor_spl::{
     associated_token::AssociatedToken,
     token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
 };
 use crate::state::casino::*;
 use crate::errors::*;

 use arcium_client::idl::arcium::*;
 use crate::SignerAccount;
 use crate::COMP_DEF_OFFSET_FLIP;


 pub fn init_flip_comp_def(ctx: Context<InitFlipCompDef>) -> Result<()> {
     init_comp_def(ctx.accounts,true, 0, None, None)?;
     Ok(())
 }

 pub fn flip(
     ctx: Context<Flip>,
     computation_offset: u64,
     user_choice: [u8; 32],
     pub_key: [u8; 32],
     nonce: u128,
     bet_amount: u64,
 ) -> Result<()> {
     // Validate & take bet
     require!(ctx.accounts.casino_state.is_active, CasinoError::CasinoNotActive);

     let transfer_instruction = TransferChecked {
         from: ctx.accounts.user_token_account.to_account_info(),
         to: ctx.accounts.casino_vault.to_account_info(),
         authority: ctx.accounts.user_token_account.to_account_info(),
         mint: ctx.accounts.usdc_mint.to_account_info(),
     };
     let cpi_ctx = CpiContext::new(
         ctx.accounts.token_program.to_account_info(),
         transfer_instruction,
     );
     transfer_checked(cpi_ctx, bet_amount, ctx.accounts.usdc_mint.decimals)?;

     // Update metrics
     let casino_state = &mut ctx.accounts.casino_state;
     casino_state.total_games_played += 1;
     casino_state.total_volume += bet_amount;

     // Track bet for callback
     let coinflip_game = &mut ctx.accounts.coinflip_game;
     coinflip_game.user = ctx.accounts.payer.key();
     coinflip_game.bet_amount = bet_amount;
     coinflip_game.bump = ctx.bumps.coinflip_game;

     // Prepare args per example
     let args = vec![
         Argument::ArcisPubkey(pub_key),
         Argument::PlaintextU128(nonce),
         Argument::EncryptedU8(user_choice),
     ];

     ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

     queue_computation(
         ctx.accounts,
         computation_offset,
         args,
         None,
         vec![FlipCallback::callback_ix(&[
             CallbackAccount { pubkey: ctx.accounts.coinflip_game.key(), is_writable: true },
             CallbackAccount { pubkey: ctx.accounts.user_stats.key(), is_writable: true },
             CallbackAccount { pubkey: ctx.accounts.casino_state.key(), is_writable: true },
         ])],
     )?;

     Ok(())
 }

 #[event]
 pub struct CoinflipEvent {
     pub win: bool,
     pub payout: u64,
 }

 pub fn flip_callback(
     ctx: Context<FlipCallback>,
     output: ComputationOutputs<FlipOutput>,
 ) -> Result<()> {
     let win = match output {
         ComputationOutputs::Success(FlipOutput { field_0 }) => field_0,
         _ => return Err(ErrorCode::AbortedComputation.into()),
     };

     let bet_amount = ctx.accounts.coinflip_game.bet_amount;
     let gross_payout = if win { bet_amount * 2 } else { 0 };

     // Apply house edge (reuse slots RTP for now)
     let rtp_multiplier = ctx.accounts.casino_state.house_edge_config.slots_rtp_bps as f64 / 10000.0;
     let final_payout = (gross_payout as f64 * rtp_multiplier) as u64;

     if final_payout > 0 {
         let ix = TransferChecked {
             from: ctx.accounts.casino_vault.to_account_info(),
             to: ctx.accounts.user_token_account.to_account_info(),
             authority: ctx.accounts.casino_state.to_account_info(),
             mint: ctx.accounts.usdc_mint.to_account_info(),
         };
         let seeds: &[&[&[u8]]] = &[&[b"casino_state", &[ctx.accounts.casino_state.casino_state_bump]]];
         let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), ix, seeds);
         transfer_checked(cpi_ctx, final_payout, ctx.accounts.usdc_mint.decimals)?;
     }

     // Stats
     ctx.accounts.casino_state.total_payouts += final_payout;
     if final_payout > 0 {
         ctx.accounts.user_stats.total_wins += final_payout;
         ctx.accounts.user_stats.loyalty_points += bet_amount / 100;
     } else {
         ctx.accounts.user_stats.total_losses += bet_amount;
     }

     emit!(CoinflipEvent { win, payout: final_payout });
     Ok(())
 }

 // Accounts
 #[queue_computation_accounts("flip", payer)]
 #[derive(Accounts)]
 #[instruction(computation_offset: u64)]
 pub struct Flip<'info> {
     #[account(mut)]
     pub payer: Signer<'info>,

     #[account(
         mut,
         seeds = [b"casino_state"],
         bump = casino_state.casino_state_bump
     )]
     pub casino_state: Account<'info, CasinoState>,
     #[account(
         mut,
         associated_token::mint = usdc_mint,
         associated_token::authority = casino_state,
         associated_token::token_program = token_program,
     )]
     pub casino_vault: InterfaceAccount<'info, TokenAccount>,

     #[account(mut)]
     pub user_token_account: InterfaceAccount<'info, TokenAccount>,
     pub usdc_mint: InterfaceAccount<'info, Mint>,

     #[account(
         init_if_needed,
         payer = payer,
         space = UserStats::DISCRIMINATOR.len() + UserStats::INIT_SPACE,
         seeds = [b"user_stats", payer.key().as_ref()],
         bump
     )]
     pub user_stats: Account<'info, UserStats>,

     #[account(
         init,
         payer = payer,
         space = CoinflipGame::DISCRIMINATOR.len() + CoinflipGame::INIT_SPACE,
         seeds = [b"coinflip_game", payer.key().as_ref()],
         bump
     )]
     pub coinflip_game: Account<'info, CoinflipGame>,

     // Arcium infra
     #[account(
         init_if_needed,
         space = 9,
         payer = payer,
         seeds = [&SIGN_PDA_SEED],
         bump,
         address = derive_sign_pda!(),
     )]
     pub sign_pda_account: Account<'info, SignerAccount>,
     #[account(address = derive_mxe_pda!())]
     pub mxe_account: Account<'info, MXEAccount>,
     #[account(mut, address = derive_mempool_pda!())]
     /// CHECK: arcium program checks
     pub mempool_account: UncheckedAccount<'info>,
     #[account(mut, address = derive_execpool_pda!())]
     /// CHECK: arcium program checks
     pub executing_pool: UncheckedAccount<'info>,
     #[account(mut, address = derive_comp_pda!(computation_offset))]
     /// CHECK: arcium program checks
     pub computation_account: UncheckedAccount<'info>,
     #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_FLIP))]
     pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
     #[account(mut, address = derive_cluster_pda!(mxe_account))]
     pub cluster_account: Account<'info, Cluster>,
     #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
     pub pool_account: Account<'info, FeePool>,
     #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
     pub clock_account: Account<'info, ClockAccount>,

     pub token_program: Interface<'info, TokenInterface>,
     pub associated_token_program: Program<'info, AssociatedToken>,
     pub system_program: Program<'info, System>,
     pub arcium_program: Program<'info, Arcium>,
 }

 #[callback_accounts("flip")]
 #[derive(Accounts)]
 pub struct FlipCallback<'info> {
     pub arcium_program: Program<'info, Arcium>,
     #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_FLIP))]
     pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
     #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
     /// CHECK: constraint check
     pub instructions_sysvar: AccountInfo<'info>,

     #[account(mut,
         seeds = [b"casino_state"],
         bump = casino_state.casino_state_bump
     )]
     pub casino_state: Account<'info, CasinoState>,
     #[account(mut,
         seeds = [b"coinflip_game", coinflip_game.user.as_ref()],
         bump = coinflip_game.bump
     )]
     pub coinflip_game: Account<'info, CoinflipGame>,
     #[account(mut,
         seeds = [b"user_stats", coinflip_game.user.as_ref()],
         bump = user_stats.bump
     )]
     pub user_stats: Account<'info, UserStats>,
     #[account(mut)]
     pub casino_vault: InterfaceAccount<'info, TokenAccount>,
     #[account(mut)]
     pub user_token_account: InterfaceAccount<'info, TokenAccount>,
     pub usdc_mint: InterfaceAccount<'info, Mint>,
     pub token_program: Interface<'info, TokenInterface>,
 }

 #[init_computation_definition_accounts("flip", payer)]
 #[derive(Accounts)]
 pub struct InitFlipCompDef<'info> {
     #[account(mut)]
     pub payer: Signer<'info>,
     #[account(mut, address = derive_mxe_pda!())]
     pub mxe_account: Box<Account<'info, MXEAccount>>,
     #[account(mut)]
     /// CHECK: not initialized yet
     pub comp_def_account: UncheckedAccount<'info>,
     pub arcium_program: Program<'info, Arcium>,
     pub system_program: Program<'info, System>,
 }

 #[error_code]
 pub enum ErrorCode {
     #[msg("The computation was aborted")]
     AbortedComputation,
     #[msg("Not authorized")]
     NotAuthorized,
     #[msg("Cluster not set")]
     ClusterNotSet,
 }
