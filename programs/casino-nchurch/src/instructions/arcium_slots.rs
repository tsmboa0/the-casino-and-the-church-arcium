 use anchor_lang::prelude::*;
 use arcium_anchor::prelude::*;
 use arcium_client::idl::arcium::types::CallbackAccount;
 use anchor_spl::{
     associated_token::AssociatedToken,
     token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
 };
 use crate::state::casino::*;
 use crate::utils::{calculate_slots_payout};

 use crate::errors::*;

 use crate::SignerAccount;
 use arcium_client::idl::arcium::*;
 use crate::COMP_DEF_OFFSET_SPIN_SLOTS;

 // Initialize computation definition for slots spin
 pub fn init_spin_slots_comp_def(ctx: Context<InitSpinSlotsCompDef>) -> Result<()> {
     init_comp_def(ctx.accounts,true, 0, None, None)?;
     Ok(())
 }

 // Queue a slots spin computation
 pub fn spin_slots(
     ctx: Context<SpinSlots>,
     computation_offset: u64,
     bet_amount: u64,
     nonce: u128,
 ) -> Result<()> {
     // validate casino and transfer bet
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

     // track stats
     let casino_state = &mut ctx.accounts.casino_state;
     casino_state.total_games_played += 1;
     casino_state.total_volume += bet_amount;

     let slots_game = &mut ctx.accounts.slots_game;
     slots_game.user = ctx.accounts.payer.key();
     slots_game.bet_amount = bet_amount;
     slots_game.bump = ctx.bumps.slots_game;
     slots_game.nonce = nonce;
     // prepare args (optionally pass bet amount)
     let args = vec![Argument::PlaintextU128(nonce)];

     ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

     // queue computation with callback
     queue_computation(
         ctx.accounts,
         computation_offset,
         args,
         None,
         vec![SpinSlotsCallback::callback_ix(&[
            CallbackAccount {
             pubkey: ctx.accounts.user_stats.key(),
             is_writable: true,
            },
            CallbackAccount {
                pubkey: ctx.accounts.slots_game.key(),
                is_writable: true,
            },
            CallbackAccount {
                pubkey: ctx.accounts.casino_state.key(),
                is_writable: true,
            },
            CallbackAccount {
                pubkey: ctx.accounts.casino_vault.key(),
                is_writable: true,
            },
            CallbackAccount {
                pubkey: ctx.accounts.user_token_account.key(),
                is_writable: true,
            },
            CallbackAccount {
                pubkey: ctx.accounts.usdc_mint.key(),
                is_writable: true,
            },
            CallbackAccount {
                pubkey: ctx.accounts.token_program.key(),
                is_writable: true,
            },
        ]),
        ],
     )?;

     Ok(())
 }

 // Callback will receive (u8,u8,u8). We implement it in a later step after macro switch
// This implements transfer and stats update.

#[event]
pub struct SlotsSpinEvent {
    pub reels: [u8; 3],
    pub payout: u64,
}

pub fn spin_slots_callback(
    ctx: Context<SpinSlotsCallback>,
    output: ComputationOutputs<SpinSlotsOutput>,
) -> Result<()> {
    // Expect tuple (u8,u8,u8)
    let (r0, r1, r2) = match output {
        ComputationOutputs::Success(SpinSlotsOutput { field_0 }) => match field_0 {
            SpinSlotsOutputStruct0 { field_0, field_1, field_2 } => (field_0, field_1, field_2),
        },
        _ => return Err(ErrorCode::AbortedComputation.into()),
    };

    let bet_amount = ctx.accounts.slots_game.bet_amount;
    let reels = [r0, r1, r2];
    let payout = calculate_slots_payout(reels, bet_amount);

    // apply house edge
    let rtp_multiplier = ctx.accounts.casino_state.house_edge_config.slots_rtp_bps as f64 / 10000.0;
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

    // update stats
    ctx.accounts.casino_state.total_payouts += final_payout;
    if final_payout > 0 {
        ctx.accounts.user_stats.total_wins += final_payout;
        ctx.accounts.user_stats.loyalty_points += bet_amount / 100;
    } else {
        ctx.accounts.user_stats.total_losses += bet_amount;
    }

    ctx.accounts.slots_game.reels = reels;
    ctx.accounts.slots_game.payout = final_payout;
    ctx.accounts.slots_game.is_complete = true;

    emit!(SlotsSpinEvent { reels, payout: final_payout });
    Ok(())
}

 // Accounts

 #[queue_computation_accounts("spin_slots", payer)]
 #[derive(Accounts)]
 #[instruction(computation_offset: u64)]
 pub struct SpinSlots<'info> {
     #[account(mut)]
     pub payer: Signer<'info>,

     // casino state and vault
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

     // user stats (init if needed)
     #[account(
         init_if_needed,
         payer = payer,
         space = UserStats::DISCRIMINATOR.len() + UserStats::INIT_SPACE,
         seeds = [b"user_stats", payer.key().as_ref()],
         bump
     )]
     pub user_stats: Account<'info, UserStats>,

     // user slot game account
     #[account(
        init,
        payer=payer,
        space = SlotsGame::DISCRIMINATOR.len() + SlotsGame::INIT_SPACE,
        seeds= [b"slots_game", payer.key().as_ref()],
        bump
     )]
     pub slots_game: Account<'info, SlotsGame>,

     // arcium infra
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
     #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SPIN_SLOTS))]
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

 #[callback_accounts("spin_slots")]
 #[derive(Accounts)]
 pub struct SpinSlotsCallback<'info> {
     pub arcium_program: Program<'info, Arcium>,
     #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SPIN_SLOTS))]
     pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
     #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
     /// CHECK: checked by constraint
     pub instructions_sysvar: AccountInfo<'info>,
     #[account(mut)]
     pub casino_state: Account<'info, CasinoState>,
     #[account(mut)]
     pub user_stats: Account<'info, UserStats>,
     #[account(mut)]
     pub casino_vault: InterfaceAccount<'info, TokenAccount>,
     #[account(mut)]
     pub slots_game: Account<'info, SlotsGame>,
     #[account(mut)]
     pub user_token_account: InterfaceAccount<'info, TokenAccount>,
     pub usdc_mint: InterfaceAccount<'info, Mint>,
     pub token_program: Interface<'info, TokenInterface>,
 }

 #[init_computation_definition_accounts("spin_slots", payer)]
 #[derive(Accounts)]
 pub struct InitSpinSlotsCompDef<'info> {
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


