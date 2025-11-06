use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;


pub mod state;
pub mod instructions;
pub mod utils;
pub mod errors;

use state::*;
use instructions::*;
use errors::*;

const COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS: u32 = comp_def_offset("shuffle_and_deal_cards");
const COMP_DEF_OFFSET_PLAYER_HIT: u32 = comp_def_offset("player_hit");
const COMP_DEF_OFFSET_PLAYER_DOUBLE_DOWN: u32 = comp_def_offset("player_double_down");
const COMP_DEF_OFFSET_PLAYER_STAND: u32 = comp_def_offset("player_stand");
const COMP_DEF_OFFSET_DEALER_PLAY: u32 = comp_def_offset("dealer_play");
const COMP_DEF_OFFSET_RESOLVE_GAME: u32 = comp_def_offset("resolve_game");

const COMP_DEF_OFFSET_FLIP: u32 = comp_def_offset("flip");

const COMP_DEF_OFFSET_ROLL_DICE: u32 = comp_def_offset("roll_dice");

const COMP_DEF_OFFSET_ROLL_ROULETTE: u32 = comp_def_offset("roll_roulette");

const COMP_DEF_OFFSET_SPIN_SLOTS: u32 = comp_def_offset("spin_slots");

declare_id!("6gPur28ubFVGDiRx1qYLVsP9jUwu6nhr98yv3p5Rocsy");

#[arcium_program]
pub mod casino_nchurch {
    use super::*;

    

    // Arcium Slots (queue encrypted RNG spin)
    pub fn init_spin_slots_comp_def(ctx: Context<InitSpinSlotsCompDef>) -> Result<()> {
        instructions::arcium_slots::init_spin_slots_comp_def(ctx)
    }

    // Arcium Roulette
    pub fn init_roll_roulette_comp_def(ctx: Context<InitRollRouletteCompDef>) -> Result<()> {
        instructions::arcium_roulette::init_roll_roulette_comp_def(ctx)
    }

    pub fn roll_roulette(
        ctx: Context<RollRoulette>,
        computation_offset: u64,
        bet_amount: u64,
        bet_type: RouletteBetType,
        numbers: Vec<u8>,
        nonce: u128,
    ) -> Result<()> {
        instructions::arcium_roulette::roll_roulette(
            ctx,
            computation_offset,
            bet_amount,
            bet_type,
            numbers,
            nonce,
        )
    }

    // Arcium Coinflip
    pub fn init_flip_comp_def(ctx: Context<InitFlipCompDef>) -> Result<()> {
        instructions::arcium_coinflip::init_flip_comp_def(ctx)
    }

    pub fn flip(
        ctx: Context<Flip>,
        computation_offset: u64,
        user_choice: [u8; 32],
        pub_key: [u8; 32],
        nonce: u128,
        bet_amount: u64,
    ) -> Result<()> {
        instructions::arcium_coinflip::flip(ctx, computation_offset, user_choice, pub_key, nonce, bet_amount)
    }

    // Arcium Blackjack
    pub fn init_shuffle_and_deal_cards_comp_def(
        ctx: Context<InitShuffleAndDealCardsCompDef>,
    ) -> Result<()> {
        instructions::arcium_blackjack::init_shuffle_and_deal_cards_comp_def(ctx)
    }

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
        instructions::arcium_blackjack::initialize_blackjack_game(
            ctx,
            computation_offset,
            game_id,
            mxe_nonce,
            mxe_again_nonce,
            client_pubkey,
            client_nonce,
            client_again_nonce,
            bet_amount,
        )
    }

    pub fn init_player_hit_comp_def(ctx: Context<InitPlayerHitCompDef>) -> Result<()> {
        instructions::arcium_blackjack::init_player_hit_comp_def(ctx)
    }
    pub fn player_hit(
        ctx: Context<PlayerHit>,
        computation_offset: u64,
    ) -> Result<()> {
        instructions::arcium_blackjack::player_hit(ctx, computation_offset)
    }

    pub fn init_player_double_down_comp_def(
        ctx: Context<InitPlayerDoubleDownCompDef>,
    ) -> Result<()> {
        instructions::arcium_blackjack::init_player_double_down_comp_def(ctx)
    }
    pub fn player_double_down(
        ctx: Context<PlayerDoubleDown>,
        computation_offset: u64,
    ) -> Result<()> {
        instructions::arcium_blackjack::player_double_down(ctx, computation_offset)
    }

    pub fn init_player_stand_comp_def(ctx: Context<InitPlayerStandCompDef>) -> Result<()> {
        instructions::arcium_blackjack::init_player_stand_comp_def(ctx)
    }
    pub fn player_stand(
        ctx: Context<PlayerStand>,
        computation_offset: u64,
    ) -> Result<()> {
        instructions::arcium_blackjack::player_stand(ctx, computation_offset)
    }

    pub fn init_dealer_play_comp_def(ctx: Context<InitDealerPlayCompDef>) -> Result<()> {
        instructions::arcium_blackjack::init_dealer_play_comp_def(ctx)
    }
    pub fn dealer_play(
        ctx: Context<DealerPlay>,
        computation_offset: u64,
        client_nonce: u128,
    ) -> Result<()> {
        instructions::arcium_blackjack::dealer_play(ctx, computation_offset, client_nonce)
    }

    pub fn init_resolve_game_comp_def(ctx: Context<InitResolveGameCompDef>) -> Result<()> {
        instructions::arcium_blackjack::init_resolve_game_comp_def(ctx)
    }
    pub fn resolve_game(
        ctx: Context<ResolveGame>,
        computation_offset: u64,
    ) -> Result<()> {
        instructions::arcium_blackjack::resolve_game(ctx, computation_offset)
    }

    // Arcium Dice
    pub fn init_roll_dice_comp_def(ctx: Context<InitRollDiceCompDef>) -> Result<()> {
        instructions::arcium_dice::init_roll_dice_comp_def(ctx)
    }

    pub fn roll_dice(
        ctx: Context<RollDice>,
        computation_offset: u64,
        bet_amount: u64,
        bet_type: DiceBetType,
        param: Option<u8>,
        nonce: u128,
    ) -> Result<()> {
        instructions::arcium_dice::roll_dice(ctx, computation_offset, bet_amount, bet_type, param, nonce)
    }

    pub fn spin_slots(ctx: Context<SpinSlots>, computation_offset: u64, bet_amount: u64, nonce: u128) -> Result<()> {
        instructions::arcium_slots::spin_slots(ctx, computation_offset, bet_amount, nonce)
    }

    #[arcium_callback(encrypted_ix = "spin_slots")]
    pub fn spin_slots_callback(ctx: Context<SpinSlotsCallback>, output: ComputationOutputs<SpinSlotsOutput>) -> Result<()> {
        instructions::arcium_slots::spin_slots_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "shuffle_and_deal_cards")]
    pub fn shuffle_and_deal_cards_callback(ctx: Context<ShuffleAndDealCardsCallback>, output: ComputationOutputs<ShuffleAndDealCardsOutput>) -> Result<()> {
        instructions::arcium_blackjack::shuffle_and_deal_cards_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "roll_roulette")]
    pub fn roll_roulette_callback(ctx: Context<RollRouletteCallback>, output: ComputationOutputs<RollRouletteOutput>) -> Result<()> {
        instructions::arcium_roulette::roll_roulette_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "flip")]
    pub fn flip_callback(ctx: Context<FlipCallback>, output: ComputationOutputs<FlipOutput>) -> Result<()> {
        instructions::arcium_coinflip::flip_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "player_hit")]
    pub fn player_hit_callback(ctx: Context<PlayerHitCallback>, output: ComputationOutputs<PlayerHitOutput>) -> Result<()> {
        instructions::arcium_blackjack::player_hit_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "player_double_down")]
    pub fn player_double_down_callback(ctx: Context<PlayerDoubleDownCallback>, output: ComputationOutputs<PlayerDoubleDownOutput>) -> Result<()> {
        instructions::arcium_blackjack::player_double_down_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "player_stand")]
    pub fn player_stand_callback(ctx: Context<PlayerStandCallback>, output: ComputationOutputs<PlayerStandOutput>) -> Result<()> {
        instructions::arcium_blackjack::player_stand_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "dealer_play")]
    pub fn dealer_play_callback(ctx: Context<DealerPlayCallback>, output: ComputationOutputs<DealerPlayOutput>) -> Result<()> {
        instructions::arcium_blackjack::dealer_play_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "resolve_game")]
    pub fn resolve_game_callback(ctx: Context<ResolveGameCallback>, output: ComputationOutputs<ResolveGameOutput>) -> Result<()> {
        instructions::arcium_blackjack::resolve_game_callback(ctx, output)
    }

    #[arcium_callback(encrypted_ix = "roll_dice")]
    pub fn roll_dice_callback(ctx: Context<RollDiceCallback>, output: ComputationOutputs<RollDiceOutput>) -> Result<()> {
        instructions::arcium_dice::roll_dice_callback(ctx, output)
    }




    // Quest Instructions
    pub fn create_quest_campaign(ctx: Context<CreateQuestCampaign>, 
                                title: String, 
                                description: String, 
                                reward_pool: u64, 
                                max_participants: u32,
                                quest_type: QuestType,
                                campaign_counter: u64) -> Result<()> {
        ctx.accounts.create_quest_campaign(title, description, reward_pool, max_participants, quest_type, campaign_counter, &ctx.bumps)
    }

    

    pub fn participate_in_quest(ctx: Context<ParticipateInQuest>, campaign_counter: u64) -> Result<()> {
        ctx.accounts.participate_in_quest(campaign_counter, &ctx.bumps)
    }

    pub fn complete_quest(ctx: Context<CompleteQuest>, campaign_counter: u64) -> Result<()> {
        ctx.accounts.complete_quest(campaign_counter, &ctx.bumps)
    }

    pub fn distribute_quest_rewards(ctx: Context<DistributeQuestRewards>, campaign_counter: u64) -> Result<()> {
        ctx.accounts.distribute_quest_rewards(campaign_counter, &ctx.bumps)
    }

    // Liquidity Pool Instructions
    pub fn initialize_liquidity_pool(ctx: Context<InitializeLiquidityPool>) -> Result<()> {
        ctx.accounts.initialize_liquidity_pool(&ctx.bumps)
    }

    // Casino Instructions
    pub fn initialize_casino(ctx: Context<InitializeCasino>) -> Result<()> {
        ctx.accounts.initialize_casino(&ctx.bumps)
    }

    pub fn deposit_liquidity(ctx: Context<DepositLiquidity>, amount: u64) -> Result<()> {
        ctx.accounts.deposit_liquidity(amount, &ctx.bumps)
    }

    pub fn stake_lp_tokens(ctx: Context<StakeLPTokens>, amount: u64, staking_counter: u64) -> Result<()> {
        ctx.accounts.stake_lp_tokens(amount, staking_counter, &ctx.bumps)
    }

    pub fn claim_lp_rewards(ctx: Context<ClaimLPRewards>, staking_counter: u64) -> Result<()> {
        ctx.accounts.claim_lp_rewards(staking_counter, &ctx.bumps)
    }

    pub fn distribute_platform_fees(ctx: Context<DistributePlatformFees>, epoch: u64) -> Result<()> {
        ctx.accounts.distribute_platform_fees(epoch, &ctx.bumps)
    }
}
