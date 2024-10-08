use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("7jnHKJjgLdHu2uNnx26fw2BoEFm2caH3bbgq9DGe9MN9");

#[program]
mod staking_airdrop {
    use super::*;

    // Initialize the reward pool for airdrop distribution
    pub fn initialize_pool(ctx: Context<InitializePool>, total_reward: u64, bump: u8) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        pool.total_reward = total_reward;
        pool.available_reward = total_reward;
        pool.bump = bump; // Assign the bump directly from input
        Ok(())
    }

    // Stake tokens into the pool
    pub fn stake_tokens(ctx: Context<StakeTokens>, amount: u64, staking_period: u64) -> Result<()> {
        let staker = &mut ctx.accounts.staker_account;

        // Ensure the user isn't already staking
        require!(staker.amount_staked == 0, CustomError::AlreadyStaked);

        // Update staker account
        staker.amount_staked = amount;
        staker.staking_period = staking_period;
        staker.start_time = Clock::get()?.unix_timestamp as u64;
        staker.has_premium_access = amount >= PREMIUM_THRESHOLD; // Premium access based on staking

        // Transfer staked tokens from user to pool's token account
        token::transfer(
            ctx.accounts
                .into_transfer_to_pool_context(),
            amount,
        )?;

        // Emit an event to log the staking action
        emit!(StakeEvent {
            user: ctx.accounts.user.key(),
            amount,
            staking_period
        });

        Ok(())
    }

    // Unstake tokens and claim reward after staking period
pub fn unstake_and_claim(ctx: Context<UnstakeAndClaim>) -> Result<()> {
    let staker = &ctx.accounts.staker_account;
    let pool = &ctx.accounts.pool;

    // Calculate reward based on staking period and amount
    let current_time = Clock::get()?.unix_timestamp as u64;
    require!(current_time >= staker.start_time + staker.staking_period, CustomError::StakingPeriodNotComplete);

    let reward = calculate_reward(staker.amount_staked, current_time - staker.start_time, pool.total_reward)?;

    // Ensure pool has enough rewards left
    require!(pool.available_reward >= reward, CustomError::InsufficientRewardPool);

    // Perform the token transfer to the user
    token::transfer(
        ctx.accounts.transfer_to_user_context(),
        reward,
    )?;

    // Now mutate the accounts
    let staker = &mut ctx.accounts.staker_account;
    let pool = &mut ctx.accounts.pool;

    staker.amount_staked = 0;
    staker.has_premium_access = false; // Revoke premium access

    // Reduce available reward in the pool
    pool.available_reward = pool.available_reward.checked_sub(reward).unwrap();

    // Emit an event to log the unstaking action
    emit!(UnstakeEvent {
        user: ctx.accounts.user.key(),
        amount: staker.amount_staked,
        reward
    });

    Ok(())
}
}

// Helper function to calculate rewards based on staking amount and time
fn calculate_reward(amount_staked: u64, elapsed_time: u64, total_reward: u64) -> Result<u64> {
    // Linear reward calculation as an example
    let reward = (amount_staked * elapsed_time) / (30 * 24 * 60 * 60); // 30 days = 1 month staking
    Ok(reward.min(total_reward)) // Ensure reward doesn't exceed pool
}

// Account storing the total reward pool for staking
#[account]
pub struct AirdropPool {
    pub total_reward: u64,      // Total amount of tokens for reward distribution
    pub available_reward: u64,  // Remaining reward available in the pool
    pub bump: u8,               // PDA bump seed for the pool account
}

// Account for tracking individual stakers
#[account]
pub struct StakerAccount {
    pub amount_staked: u64,     // Amount of tokens staked
    pub staking_period: u64,    // Time (in seconds) for which tokens are staked
    pub start_time: u64,        // Timestamp when staking started
    pub has_premium_access: bool, // Premium data access flag
}

const PREMIUM_THRESHOLD: u64 = 1000; // Tokens needed to grant premium access

// Context for initializing the reward pool
#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(init, payer = user, space = 8 + 48, seeds = [b"pool".as_ref()], bump)]
    pub pool: Account<'info, AirdropPool>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Context for staking tokens
#[derive(Accounts)]
pub struct StakeTokens<'info> {
    #[account(mut)]
    pub staker_account: Account<'info, StakerAccount>,
    #[account(mut)]
    pub pool: Account<'info, AirdropPool>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> StakeTokens<'info> {
    // Context for transferring tokens from user to pool
    fn into_transfer_to_pool_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account.to_account_info(),
                to: self.pool_token_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }
}

// Context for unstaking and claiming rewards
#[derive(Accounts)]
pub struct UnstakeAndClaim<'info> {
    #[account(mut)]
    pub staker_account: Account<'info, StakerAccount>,
    #[account(mut)]
    pub pool: Account<'info, AirdropPool>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> UnstakeAndClaim<'info> {
    fn transfer_to_user_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.pool_token_account.to_account_info(),
                to: self.user_token_account.to_account_info(),
                authority: self.pool.to_account_info(),
            },
        )
    }
}

// Events for tracking user actions
#[event]
pub struct StakeEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub staking_period: u64,
}

#[event]
pub struct UnstakeEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub reward: u64,
}

// Custom error handling
#[error_code]
pub enum CustomError {
    #[msg("Staking period is not yet complete.")]
    StakingPeriodNotComplete,
    #[msg("Insufficient reward pool available.")]
    InsufficientRewardPool,
    #[msg("User already has tokens staked.")]
    AlreadyStaked,
}