use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;
use anchor_spl::token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked};
use crate::state::casino::*;
use crate::errors::*;
use crate::utils::calculate_dice_payout;

use arcium_client::idl::arcium::*;
use crate::SignerAccount;

use crate::COMP_DEF_OFFSET_ROLL_DICE;



pub fn init_roll_dice_comp_def(ctx: Context<InitRollDiceCompDef>) -> Result<()> {
    init_comp_def(ctx.accounts,true, 0, None, None)?;
    Ok(())
}

pub fn roll_dice(
    ctx: Context<RollDice>,
    computation_offset: u64,
    bet_amount: u64,
    bet_type: DiceBetType,
    param: Option<u8>,
    nonce: u128,
) -> Result<()> {
    require!(ctx.accounts.casino_state.is_active, CasinoError::CasinoNotActive);

    // take bet
    let ix = TransferChecked {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.casino_vault.to_account_info(),
        authority: ctx.accounts.user_token_account.to_account_info(),
        mint: ctx.accounts.usdc_mint.to_account_info(),
    };
    let cpi = CpiContext::new(ctx.accounts.token_program.to_account_info(), ix);
    transfer_checked(cpi, bet_amount, ctx.accounts.usdc_mint.decimals)?;

    // Update metrics
    ctx.accounts.casino_state.total_games_played += 1;
    ctx.accounts.casino_state.total_volume += bet_amount;

    ctx.accounts.bet_meta.bet_type = bet_type;
    ctx.accounts.bet_meta.param = param;
    ctx.accounts.bet_meta.bet_amount = bet_amount;
    ctx.accounts.bet_meta.bump = ctx.bumps.bet_meta;

    // Persist bet params transiently in user_stats if needed (skipped); pass plaintext type+param
    let args = vec![
        Argument::PlaintextU128(nonce),
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        None,
        vec![RollDiceCallback::callback_ix(&[
            CallbackAccount { pubkey: ctx.accounts.user_stats.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.bet_meta.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.casino_state.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.casino_vault.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.user_token_account.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.usdc_mint.key(), is_writable: true },
            CallbackAccount { pubkey: ctx.accounts.token_program.key(), is_writable: true },
        ])],
    )?;

    Ok(())
}

#[event]
pub struct DiceResultEvent { pub roll: u8, pub payout: u64 }

pub fn roll_dice_callback(
    ctx: Context<RollDiceCallback>,
    output: ComputationOutputs<RollDiceOutput>,
) -> Result<()> {
    let bet_meta = &mut ctx.accounts.bet_meta;
    let roll = match output {
        ComputationOutputs::Success(RollDiceOutput { field_0}) => field_0,
        _ => return Err(ErrorCode::AbortedComputation.into()),
    };

    // Derive payout from bet info supplied in context
    let bet_type = bet_meta.bet_type.clone();
    let param = bet_meta.param;
    let bet_amount = bet_meta.bet_amount;
    let base = calculate_dice_payout(bet_type, param, roll, bet_amount);
    let rtp_multiplier = ctx.accounts.casino_state.house_edge_config.roulette_rtp_bps as f64 / 10000.0;
    let final_payout = (base as f64 * rtp_multiplier) as u64;

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

    emit!(DiceResultEvent { roll, payout: final_payout });
    Ok(())
}

#[queue_computation_accounts("roll_dice", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, bet_amount: u64, bet_type: DiceBetType, param: Option<u8>)]
pub struct RollDice<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, seeds = [b"casino_state"], bump = casino_state.casino_state_bump)]
    pub casino_state: Account<'info, CasinoState>,
    #[account(
        mut, 
        associated_token::mint = usdc_mint, 
        associated_token::authority = casino_state, 
        associated_token::token_program = token_program,
    )]
    pub casino_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)] pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    pub usdc_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed, 
        payer = payer, 
        space = UserStats::DISCRIMINATOR.len() + UserStats::INIT_SPACE, 
        seeds = [b"user_stats", payer.key().as_ref()], 
        bump
    )]
    pub user_stats: Account<'info, UserStats>,

    #[account(init, payer = payer, space = DiceBetMeta::DISCRIMINATOR.len() + DiceBetMeta::INIT_SPACE, seeds = [b"dice_bet", payer.key().as_ref()], bump)]
    pub bet_meta: Account<'info, DiceBetMeta>,

    #[account(init_if_needed, space = 9, payer = payer, seeds = [&SIGN_PDA_SEED], bump, address = derive_sign_pda!())]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())] pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!())] /// CHECK
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())] /// CHECK
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))] /// CHECK
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ROLL_DICE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)] pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)] pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,

    pub token_program: Interface<'info, TokenInterface>,
}

#[callback_accounts("roll_dice")]
#[derive(Accounts)]
pub struct RollDiceCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ROLL_DICE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)] /// CHECK
    pub instructions_sysvar: AccountInfo<'info>,

    #[account(mut, seeds = [b"casino_state"], bump = casino_state.casino_state_bump)]
    pub casino_state: Account<'info, CasinoState>,
    #[account(mut)] pub casino_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)] pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    pub usdc_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub user_stats: Account<'info, UserStats>,
    #[account(mut)]
    pub bet_meta: Account<'info, DiceBetMeta>,
}

#[init_computation_definition_accounts("roll_dice", payer)]
#[derive(Accounts)]
pub struct InitRollDiceCompDef<'info>{
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)] /// CHECK
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


