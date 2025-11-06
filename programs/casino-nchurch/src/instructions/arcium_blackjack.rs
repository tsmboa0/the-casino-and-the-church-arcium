use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;
use crate::errors::*;
use crate::state::casino::*;
use anchor_spl::{
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use arcium_client::idl::arcium::*;
use crate::SignerAccount;

use crate::COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS;
use crate::COMP_DEF_OFFSET_PLAYER_HIT;
use crate::COMP_DEF_OFFSET_PLAYER_DOUBLE_DOWN;
use crate::COMP_DEF_OFFSET_PLAYER_STAND;
use crate::COMP_DEF_OFFSET_DEALER_PLAY;
use crate::COMP_DEF_OFFSET_RESOLVE_GAME;

// --- Init computation definitions ---

pub fn init_shuffle_and_deal_cards_comp_def(
    ctx: Context<InitShuffleAndDealCardsCompDef>,
) -> Result<()> {
    init_comp_def(ctx.accounts,true, 0, None, None)?;
    Ok(())
}

pub fn init_player_hit_comp_def(ctx: Context<InitPlayerHitCompDef>) -> Result<()> {
    init_comp_def(ctx.accounts,true, 0, None, None)?;
    Ok(())
}

pub fn init_player_double_down_comp_def(
    ctx: Context<InitPlayerDoubleDownCompDef>,
) -> Result<()> {
    init_comp_def(ctx.accounts,true, 0, None, None)?;
    Ok(())
}

pub fn init_player_stand_comp_def(ctx: Context<InitPlayerStandCompDef>) -> Result<()> {
    init_comp_def(ctx.accounts,true, 0, None, None)?;
    Ok(())
}

pub fn init_dealer_play_comp_def(ctx: Context<InitDealerPlayCompDef>) -> Result<()> {
    init_comp_def(ctx.accounts,true, 0, None, None)?;
    Ok(())
}

pub fn init_resolve_game_comp_def(ctx: Context<InitResolveGameCompDef>) -> Result<()> {
    init_comp_def(ctx.accounts,true, 0, None, None)?;
    Ok(())
}

// --- Queue entrypoints ---

pub fn initialize_blackjack_game(
    ctx: Context<InitializeBlackjackGame>,
    computation_offset: u64,
    game_id: u64,
    mxe_nonce: u128,
    mxe_again_nonce: u128,
    client_pubkey: [u8; 32],
    client_nonce: u128,
    client_again_nonce: u128,
    bet_amount: u64,
) -> Result<()> {
    let blackjack_game = &mut ctx.accounts.blackjack_game;
    blackjack_game.bump = ctx.bumps.blackjack_game;
    blackjack_game.game_id = game_id;
    blackjack_game.player_pubkey = ctx.accounts.payer.key();
    blackjack_game.player_hand = [0; 32];
    blackjack_game.dealer_hand = [0; 32];
    blackjack_game.deck_nonce = 0;
    blackjack_game.client_nonce = 0;
    blackjack_game.dealer_nonce = 0;
    blackjack_game.player_enc_pubkey = client_pubkey;
    blackjack_game.game_state = BlackjackGameState::Initial;
    blackjack_game.player_hand_size = 0;
    blackjack_game.dealer_hand_size = 0;
    blackjack_game.player_has_stood = false;
    blackjack_game.game_result = 0;
    blackjack_game.bet_amount = bet_amount;

    let args = vec![
        Argument::PlaintextU128(mxe_nonce),
        Argument::PlaintextU128(mxe_again_nonce),
        Argument::ArcisPubkey(client_pubkey),
        Argument::PlaintextU128(client_nonce),
        Argument::ArcisPubkey(client_pubkey),
        Argument::PlaintextU128(client_again_nonce),
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        None,
        vec![ShuffleAndDealCardsCallback::callback_ix(&[CallbackAccount {
            pubkey: ctx.accounts.blackjack_game.key(),
            is_writable: true,
        }])],
    )?;
    Ok(())
}

pub fn player_hit(
    ctx: Context<PlayerHit>,
    computation_offset: u64,
) -> Result<()> {
    require!(ctx.accounts.blackjack_game.game_state == BlackjackGameState::PlayerTurn, CasinoError::InvalidGameState);
    require!(!ctx.accounts.blackjack_game.player_has_stood, CasinoError::InvalidMove);

    let args = vec![
        // Deck
        Argument::PlaintextU128(ctx.accounts.blackjack_game.deck_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8, 32 * 3),
        // Player hand
        Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
        Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
        // Sizes
        Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        None,
        vec![PlayerHitCallback::callback_ix(&[CallbackAccount { pubkey: ctx.accounts.blackjack_game.key(), is_writable: true }])],
    )?;
    Ok(())
}

pub fn player_double_down(
    ctx: Context<PlayerDoubleDown>,
    computation_offset: u64,
) -> Result<()> {
    require!(ctx.accounts.blackjack_game.game_state == BlackjackGameState::PlayerTurn, CasinoError::InvalidGameState);
    require!(!ctx.accounts.blackjack_game.player_has_stood, CasinoError::InvalidMove);

    let args = vec![
        Argument::PlaintextU128(ctx.accounts.blackjack_game.deck_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8, 32 * 3),
        Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
        Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        None,
        vec![PlayerDoubleDownCallback::callback_ix(&[CallbackAccount { pubkey: ctx.accounts.blackjack_game.key(), is_writable: true }])],
    )?;
    Ok(())
}

pub fn player_stand(
    ctx: Context<PlayerStand>,
    computation_offset: u64,
) -> Result<()> {
    require!(ctx.accounts.blackjack_game.game_state == BlackjackGameState::PlayerTurn, CasinoError::InvalidGameState);
    require!(!ctx.accounts.blackjack_game.player_has_stood, CasinoError::InvalidMove);

    let args = vec![
        Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
        Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        None,
        vec![PlayerStandCallback::callback_ix(&[CallbackAccount { pubkey: ctx.accounts.blackjack_game.key(), is_writable: true }])],
    )?;
    Ok(())
}

pub fn dealer_play(
    ctx: Context<DealerPlay>,
    computation_offset: u64,
    client_nonce: u128,
) -> Result<()> {
    require!(ctx.accounts.blackjack_game.game_state == BlackjackGameState::DealerTurn, CasinoError::InvalidGameState);

    let args = vec![
        Argument::PlaintextU128(ctx.accounts.blackjack_game.deck_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8, 32 * 3),
        Argument::PlaintextU128(ctx.accounts.blackjack_game.dealer_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3 + 32, 32),
        Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
        Argument::PlaintextU128(client_nonce),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        None,
        vec![DealerPlayCallback::callback_ix(&[CallbackAccount { pubkey: ctx.accounts.blackjack_game.key(), is_writable: true }])],
    )?;
    Ok(())
}

pub fn resolve_game(
    ctx: Context<ResolveGame>,
    computation_offset: u64,
) -> Result<()> {
    require!(ctx.accounts.blackjack_game.game_state == BlackjackGameState::Resolving, CasinoError::InvalidGameState);

    let args = vec![
        Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
        Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
        Argument::PlaintextU128(ctx.accounts.blackjack_game.dealer_nonce),
        Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3 + 32, 32),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
        Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        None,
        vec![ResolveGameCallback::callback_ix(&[CallbackAccount { pubkey: ctx.accounts.blackjack_game.key(), is_writable: true }])],
    )?;
    Ok(())
}

// --- Callbacks ---

#[event]
pub struct CardsShuffledAndDealtEvent {
    pub player_hand: [u8; 32],
    pub dealer_face_up_card: [u8; 32],
    pub client_nonce: u128,
    pub dealer_client_nonce: u128,
    pub game_id: u64,
}

#[event]
pub struct PlayerHitEvent { pub player_hand: [u8; 32], pub client_nonce: u128, pub game_id: u64 }
#[event]
pub struct PlayerDoubleDownEvent { pub player_hand: [u8; 32], pub client_nonce: u128, pub game_id: u64 }
#[event]
pub struct PlayerStandEvent { pub is_bust: bool, pub game_id: u64 }
#[event]
pub struct PlayerBustEvent { pub client_nonce: u128, pub game_id: u64 }
#[event]
pub struct DealerPlayEvent { pub dealer_hand: [u8; 32], pub dealer_hand_size: u8, pub client_nonce: u128, pub game_id: u64 }
#[event]
pub struct BlackjackResultEvent { pub result_code: u8, pub game_id: u64 }


pub fn shuffle_and_deal_cards_callback(
    ctx: Context<ShuffleAndDealCardsCallback>,
    output: ComputationOutputs<ShuffleAndDealCardsOutput>,
) -> Result<()> {
    let (deck, dealer_hand, player_hand, dealer_face_up_card) = match output {
        ComputationOutputs::Success(ShuffleAndDealCardsOutput { field_0: ShuffleAndDealCardsOutputStruct0 { field_0: deck, field_1: dealer_hand, field_2: player_hand, field_3: dealer_face_up_card } }) => (deck, dealer_hand, player_hand, dealer_face_up_card),
        _ => return Err(ErrorCode::AbortedComputation.into()),
    };

    let deck_nonce = deck.nonce;
    let deck_chunks: [[u8; 32]; 3] = deck.ciphertexts;

    let dealer_nonce = dealer_hand.nonce;
    let dealer_hand_ct = dealer_hand.ciphertexts[0];

    let client_pubkey: [u8; 32] = player_hand.encryption_key;
    let client_nonce = player_hand.nonce;
    let player_hand_ct = player_hand.ciphertexts[0];

    let dealer_client_pubkey: [u8; 32] = dealer_face_up_card.encryption_key;
    let dealer_client_nonce = dealer_face_up_card.nonce;
    let dealer_face_up_ct = dealer_face_up_card.ciphertexts[0];

    let game = &mut ctx.accounts.blackjack_game;
    game.deck = deck_chunks;
    game.deck_nonce = deck_nonce;
    game.client_nonce = client_nonce;
    game.dealer_nonce = dealer_nonce;
    game.player_enc_pubkey = client_pubkey;
    game.game_state = BlackjackGameState::PlayerTurn;
    game.player_hand = player_hand_ct;
    game.dealer_hand = dealer_hand_ct;
    game.player_hand_size = 2;
    game.dealer_hand_size = 2;

    require!(dealer_client_pubkey == game.player_enc_pubkey, CasinoError::InvalidDealerClientPubkey);

    emit!(CardsShuffledAndDealtEvent { client_nonce, dealer_client_nonce, player_hand: player_hand_ct, dealer_face_up_card: dealer_face_up_ct, game_id: game.game_id });
    Ok(())
}

pub fn player_hit_callback(
    ctx: Context<PlayerHitCallback>,
    output: ComputationOutputs<PlayerHitOutput>,
) -> Result<()> {
    let (player_hand, is_bust) = match output {
        ComputationOutputs::Success(PlayerHitOutput { field_0: PlayerHitOutputStruct0 { field_0: player_hand, field_1: is_bust } }) => (player_hand, is_bust),
        _ => return Err(ErrorCode::AbortedComputation.into()),
    };
    let client_nonce = player_hand.nonce;
    let player_hand_ct = player_hand.ciphertexts[0];

    let game = &mut ctx.accounts.blackjack_game;
    game.player_hand = player_hand_ct;
    game.client_nonce = client_nonce;
    if is_bust { game.game_state = BlackjackGameState::DealerTurn; emit!(PlayerBustEvent { client_nonce, game_id: game.game_id }); } else { game.game_state = BlackjackGameState::PlayerTurn; emit!(PlayerHitEvent { player_hand: player_hand_ct, client_nonce, game_id: game.game_id }); game.player_hand_size += 1; }
    Ok(())
}

pub fn player_double_down_callback(
    ctx: Context<PlayerDoubleDownCallback>,
    output: ComputationOutputs<PlayerDoubleDownOutput>,
) -> Result<()> {
    let (player_hand, is_bust) = match output {
        ComputationOutputs::Success(PlayerDoubleDownOutput { field_0: PlayerDoubleDownOutputStruct0 { field_0: player_hand, field_1: is_bust } }) => (player_hand, is_bust),
        _ => return Err(ErrorCode::AbortedComputation.into()),
    };
    let client_nonce = player_hand.nonce;
    let player_hand_ct = player_hand.ciphertexts[0];

    let game = &mut ctx.accounts.blackjack_game;
    game.player_hand = player_hand_ct;
    game.client_nonce = client_nonce;
    game.player_has_stood = true;
    if is_bust { game.game_state = BlackjackGameState::DealerTurn; emit!(PlayerBustEvent { client_nonce, game_id: game.game_id }); } else { game.game_state = BlackjackGameState::DealerTurn; emit!(PlayerDoubleDownEvent { player_hand: player_hand_ct, client_nonce, game_id: game.game_id }); }
    Ok(())
}

pub fn player_stand_callback(
    ctx: Context<PlayerStandCallback>,
    output: ComputationOutputs<PlayerStandOutput>,
) -> Result<()> {
    let is_bust = match output { ComputationOutputs::Success(PlayerStandOutput { field_0 }) => field_0, _ => return Err(ErrorCode::AbortedComputation.into()) };
    let game = &mut ctx.accounts.blackjack_game;
    game.player_has_stood = true;
    if is_bust { game.game_state = BlackjackGameState::PlayerTurn; emit!(PlayerBustEvent { client_nonce: game.client_nonce, game_id: game.game_id }); } else { game.game_state = BlackjackGameState::DealerTurn; emit!(PlayerStandEvent { is_bust, game_id: game.game_id }); }
    Ok(())
}

pub fn dealer_play_callback(
    ctx: Context<DealerPlayCallback>,
    output: ComputationOutputs<DealerPlayOutput>,
) -> Result<()> {
    let (dealer_hand, dealer_client_hand, dealer_hand_size) = match output {
        ComputationOutputs::Success(DealerPlayOutput { field_0: DealerPlayOutputStruct0 { field_0: dealer_hand, field_1: dealer_client_hand, field_2: dealer_hand_size } }) => (dealer_hand, dealer_client_hand, dealer_hand_size),
        _ => return Err(ErrorCode::AbortedComputation.into()),
    };

    let dealer_nonce = dealer_hand.nonce;
    let dealer_hand_ct = dealer_hand.ciphertexts[0];
    let dealer_client_hand_ct = dealer_client_hand.ciphertexts[0];
    let client_nonce = dealer_client_hand.nonce;

    let game = &mut ctx.accounts.blackjack_game;
    game.dealer_hand = dealer_hand_ct;
    game.dealer_nonce = dealer_nonce;
    game.dealer_hand_size = dealer_hand_size;
    game.game_state = BlackjackGameState::Resolving;
    emit!(DealerPlayEvent { dealer_hand: dealer_client_hand_ct, dealer_hand_size, client_nonce, game_id: game.game_id });
    Ok(())
}

pub fn resolve_game_callback(
    ctx: Context<ResolveGameCallback>,
    output: ComputationOutputs<ResolveGameOutput>,
) -> Result<()> {
    let result = match output { ComputationOutputs::Success(ResolveGameOutput { field_0 }) => field_0, _ => return Err(ErrorCode::AbortedComputation.into()) };
    let game = &mut ctx.accounts.blackjack_game;
    game.game_state = BlackjackGameState::Resolved;
    game.game_result = result;

    // Base payout
    let bet_amount = game.bet_amount;
    let base_payout = match result {
        0 => 0u64,
        1 => bet_amount,
        2 => bet_amount,
        3 => 0u64,
        _ => bet_amount,
    };

    // Apply RTP
    let rtp_multiplier = ctx.accounts.casino_state.house_edge_config.blackjack_rtp_bps as f64 / 10000.0;
    let final_payout = (base_payout as f64 * rtp_multiplier) as u64;

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

    emit!(BlackjackResultEvent { result_code: result, game_id: game.game_id });
    Ok(())
}

// --- Accounts ---

#[queue_computation_accounts("shuffle_and_deal_cards", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, game_id: u64)]
pub struct InitializeBlackjackGame<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
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
    /// CHECK: arcium
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())]
    /// CHECK: arcium
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))]
    /// CHECK: arcium
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,

    #[account(
        init,
        payer = payer,
        space = 8 + BlackjackGame::INIT_SPACE,
        seeds = [b"blackjack_game".as_ref(), game_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("shuffle_and_deal_cards")]
#[derive(Accounts)]
pub struct ShuffleAndDealCardsCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: sysvar
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("shuffle_and_deal_cards", payer)]
#[derive(Accounts)]
pub struct InitShuffleAndDealCardsCompDef<'info> {
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

#[queue_computation_accounts("player_hit", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct PlayerHit<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
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
    /// CHECK
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())]
    /// CHECK
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))]
    /// CHECK
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_HIT))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        mut,
        seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = blackjack_game.bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("player_hit")]
#[derive(Accounts)]
pub struct PlayerHitCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_HIT))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("player_hit", payer)]
#[derive(Accounts)]
pub struct InitPlayerHitCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("player_double_down", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct PlayerDoubleDown<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(init_if_needed, space = 9, payer = payer, seeds = [&SIGN_PDA_SEED], bump, address = derive_sign_pda!())]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())] pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!())] /// CHECK
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())] /// CHECK
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))] /// CHECK
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_DOUBLE_DOWN))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)] pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)] pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(mut, seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()], bump = blackjack_game.bump)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("player_double_down")]
#[derive(Accounts)]
pub struct PlayerDoubleDownCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_DOUBLE_DOWN))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)] /// CHECK
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)] pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("player_double_down", payer)]
#[derive(Accounts)]
pub struct InitPlayerDoubleDownCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)] /// CHECK
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("player_stand", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct PlayerStand<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(init_if_needed, space = 9, payer = payer, seeds = [&SIGN_PDA_SEED], bump, address = derive_sign_pda!())]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())] pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!())] /// CHECK
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())] /// CHECK
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))] /// CHECK
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_STAND))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)] pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)] pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(mut, seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()], bump = blackjack_game.bump)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("player_stand")]
#[derive(Accounts)]
pub struct PlayerStandCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_STAND))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)] /// CHECK
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)] pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("player_stand", payer)]
#[derive(Accounts)]
pub struct InitPlayerStandCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)] /// CHECK
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("dealer_play", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct DealerPlay<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(init_if_needed, space = 9, payer = payer, seeds = [&SIGN_PDA_SEED], bump, address = derive_sign_pda!())]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())] pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!())] /// CHECK
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())] /// CHECK
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))] /// CHECK
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEALER_PLAY))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)] pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)] pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(mut, seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()], bump = blackjack_game.bump)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("dealer_play")]
#[derive(Accounts)]
pub struct DealerPlayCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEALER_PLAY))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)] /// CHECK
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)] pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("dealer_play", payer)]
#[derive(Accounts)]
pub struct InitDealerPlayCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)] /// CHECK
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("resolve_game", payer)]
#[derive(Accounts)]
pub struct InitResolveGameCompDef<'info> {
    #[account(mut)] 
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] 
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)] /// CHECK
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("resolve_game", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct ResolveGame<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(init_if_needed, space = 9, payer = payer, seeds = [&SIGN_PDA_SEED], bump, address = derive_sign_pda!())]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(address = derive_mxe_pda!())] 
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!())] /// CHECK
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())] 
    /// CHECK
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))] 
    /// CHECK
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_RESOLVE_GAME))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)] pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)] pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(mut, seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()], bump = blackjack_game.bump)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("resolve_game")]
#[derive(Accounts)]
pub struct ResolveGameCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_RESOLVE_GAME))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)] /// CHECK
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)] pub blackjack_game: Account<'info, BlackjackGame>,
    #[account(
        mut,
        seeds = [b"casino_state"],
        bump = casino_state.casino_state_bump
    )]
    pub casino_state: Account<'info, CasinoState>,
    #[account(mut)]
    pub casino_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    pub usdc_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
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
