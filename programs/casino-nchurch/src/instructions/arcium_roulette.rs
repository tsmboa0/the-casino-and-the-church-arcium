 use anchor_lang::prelude::*;
 use arcium_anchor::prelude::*;
 use anchor_spl::{
     associated_token::AssociatedToken,
     token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
 };
 use crate::state::casino::*;
 use crate::utils::{calculate_roulette_payout};

 use arcium_client::idl::arcium::types::CallbackAccount;

 use crate::errors::*;

 use crate::SignerAccount;
 use arcium_client::idl::arcium::*;
 use crate::COMP_DEF_OFFSET_ROLL_ROULETTE;



 pub fn init_roll_roulette_comp_def(ctx: Context<InitRollRouletteCompDef>) -> Result<()> {
     init_comp_def(ctx.accounts,true, 0, None, None)?;
     Ok(())
 }

 pub fn roll_roulette(
     ctx: Context<RollRoulette>,
     computation_offset: u64,
     bet_amount: u64,
     bet_type: RouletteBetType,
     numbers: Vec<u8>,
     nonce: u128,
 ) -> Result<()> {
     // basic bet validations similar to existing flow
     require!(ctx.accounts.casino_state.is_active, CasinoError::CasinoNotActive);

     // Transfer bet to vault
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

     // Store bet info in user_stats as a scratch area if desired (skipped here); rely on callback computing payout based on returned number and cached bet in client or event. For full persistence, a dedicated account is recommended.
     let roulette_game = &mut ctx.accounts.roulette_game;
     roulette_game.user = ctx.accounts.payer.key();
     roulette_game.bet_amount = bet_amount;
     roulette_game.bet_type = bet_type;
     roulette_game.bet_numbers = numbers;
     roulette_game.bump = ctx.bumps.roulette_game;
     roulette_game.nonce = nonce;
     // Update casino metrics
     let casino_state = &mut ctx.accounts.casino_state;
     casino_state.total_games_played += 1;
     casino_state.total_volume += bet_amount;

     // prepare args: encode bet type and length in plaintext for circuit if needed later (here RNG only)
     let args = vec![Argument::PlaintextU128(nonce)];

     ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

     queue_computation(
         ctx.accounts,
         computation_offset,
         args,
         None,
         vec![RollRouletteCallback::callback_ix(&[
            CallbackAccount { pubkey: ctx.accounts.roulette_game.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.user_stats.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.casino_state.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.casino_vault.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.user_token_account.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.usdc_mint.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.token_program.key(), is_writable: true },
        ]),
        ],
     )?;

     Ok(())
 }

 #[queue_computation_accounts("roll_roulette", payer)]
 #[derive(Accounts)]
 #[instruction(computation_offset: u64)]
 pub struct RollRoulette<'info> {
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
     #[account(
        init,
        payer=payer,
        space = RouletteGame::DISCRIMINATOR.len() + RouletteGame::INIT_SPACE,
        seeds = [b"roulette_game", payer.key().as_ref()],
        bump
     )]
     pub roulette_game: Account<'info, RouletteGame>,
     #[account(mut, seeds = [b"user_stats", payer.key().as_ref()], bump = user_stats.bump)]
     pub user_stats: Account<'info, UserStats>,
     #[account(mut)]
     pub user_token_account: InterfaceAccount<'info, TokenAccount>,
     pub usdc_mint: InterfaceAccount<'info, Mint>,

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
     /// CHECK: checked by arcium program
     pub mempool_account: UncheckedAccount<'info>,
     #[account(mut, address = derive_execpool_pda!())]
     /// CHECK: checked by arcium program
     pub executing_pool: UncheckedAccount<'info>,
     #[account(mut, address = derive_comp_pda!(computation_offset))]
     /// CHECK: checked by arcium program
     pub computation_account: UncheckedAccount<'info>,
     #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ROLL_ROULETTE))]
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

 #[callback_accounts("roll_roulette")]
 #[derive(Accounts)]
 pub struct RollRouletteCallback<'info> {
     pub arcium_program: Program<'info, Arcium>,
     #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ROLL_ROULETTE))]
     pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
     #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
     /// CHECK: checked by constraint
     pub instructions_sysvar: AccountInfo<'info>,
     #[account(mut)]
     pub roulette_game: Account<'info, RouletteGame>,
     #[account(mut)]
     pub user_stats: Account<'info, UserStats>,
     #[account(mut)]
     pub casino_state: Account<'info, CasinoState>,
     #[account(mut)]
     pub casino_vault: InterfaceAccount<'info, TokenAccount>,
     #[account(mut)]
     pub user_token_account: InterfaceAccount<'info, TokenAccount>,
     pub usdc_mint: InterfaceAccount<'info, Mint>,
     pub token_program: Interface<'info, TokenInterface>,
 }

#[event]
pub struct RouletteResultEvent {
    pub winning_number: u8,
    pub payout: u64,
}


pub fn roll_roulette_callback(
    ctx: Context<RollRouletteCallback>,
    output: ComputationOutputs<RollRouletteOutput>,
) -> Result<()> {
    let roulette_game = &mut ctx.accounts.roulette_game;
    // Expect (u8, u64) => (winning_number, bet_amount). We only emit number here.
    let winning_number = match output {
        ComputationOutputs::Success(RollRouletteOutput { field_0 }) => field_0,
        _ => return Err(ErrorCode::AbortedComputation.into()),
    };
    let bet_type = roulette_game.bet_type.clone();
    let bet_amount = roulette_game.bet_amount;
    let payout = calculate_roulette_payout(bet_type, bet_amount, winning_number);
    let rtp_multiplier = ctx.accounts.casino_state.house_edge_config.roulette_rtp_bps as f64 / 10000.0;
    let final_payout = (payout as f64 * rtp_multiplier) as u64;
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
    // stats
    ctx.accounts.casino_state.total_payouts += final_payout;
    if final_payout > 0 {
        ctx.accounts.user_stats.total_wins += final_payout;
        ctx.accounts.user_stats.loyalty_points += bet_amount / 100;
    } else {
        ctx.accounts.user_stats.total_losses += bet_amount;
    }
    ctx.accounts.roulette_game.winning_number = winning_number;
    ctx.accounts.roulette_game.payout = final_payout;
    ctx.accounts.roulette_game.is_complete = true;
    emit!(RouletteResultEvent { winning_number, payout: final_payout });
    Ok(())
}

 #[init_computation_definition_accounts("roll_roulette", payer)]
 #[derive(Accounts)]
 pub struct InitRollRouletteCompDef<'info> {
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