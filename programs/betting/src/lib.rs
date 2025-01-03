use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("6W1ReqRjnLTNyT4jAbexbAr1s2Qq2Ewa57pvPX6mhncm");

#[program]
pub mod betting {
    use super::*;

    pub fn create_bet(
        ctx: Context<CreateBet>,
        title: String,
        bet_amount: u64,
        end_time: i64,
    ) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        bet.creator = ctx.accounts.signer.key();
        bet.title = title;
        bet.bet_amount = bet_amount;
        bet.total_yes_amount = 0;
        bet.total_no_amount = 0;
        bet.yes_bettors = 0;
        bet.no_bettors = 0;
        bet.end_time = end_time;
        bet.resolved = false;
        bet.outcome = false;
        bet.token_mint = ctx.accounts.token_mint.key();
        bet.vault = ctx.accounts.vault_token_account.key();
        bet.bump = ctx.bumps.bet;
        bet.bump_vault_authority = ctx.bumps.vault_authority;
        bet.bump_vault_ta = ctx.bumps.vault_token_account;
        Ok(())
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        bet_direction: bool, // true for YES, false for NO
    ) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        require!(!bet.resolved, BettingError::BetAlreadyResolved);
        // require!(
        //     Clock::get()?.unix_timestamp < bet.end_time,
        //     BettingError::BetEndTimeExceeded
        // );

        // Verify the correct vault token account is being used
        require!(
            ctx.accounts.vault_token_account.key() == bet.vault,
            BettingError::InvalidVault
        );

        // Transfer tokens to vault
        let transfer_instruction = Transfer {
            from: ctx.accounts.bettor_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.bettor.to_account_info(),
        };
        
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
            ),
            bet.bet_amount,
        )?;

        // Update bet state
        if bet_direction {
            bet.yes_bettors += 1;
            bet.total_yes_amount += bet.bet_amount;
        } else {
            bet.no_bettors += 1;
            bet.total_no_amount += bet.bet_amount;
        }

        // Record user's bet
        //AFTER BET IS PLACED, UPDATE THE USER_BET ACCOUNT STATE FOR GIVEN USER
        let user_bet = &mut ctx.accounts.user_bet;
        user_bet.user = ctx.accounts.bettor.key();
        user_bet.bet = bet.key();
        user_bet.amount = bet.bet_amount;
        user_bet.direction = bet_direction;
        user_bet.claimed = false;

        Ok(())
    }

    //needs to be modified to use an oracle for checking win outcome
    pub fn resolve_bet(ctx: Context<ResolveBet>, outcome: bool) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        require!(!bet.resolved, BettingError::BetAlreadyResolved);
        require!(
            Clock::get()?.unix_timestamp >= bet.end_time,
            BettingError::BetNotEndedYet
        );
         
        bet.resolved = true;
        bet.outcome = outcome;
        Ok(())
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let bet = &ctx.accounts.bet;
        let user_bet = &mut ctx.accounts.user_bet;
        
        require!(bet.resolved, BettingError::BetNotResolved);
        require!(!user_bet.claimed, BettingError::AlreadyClaimed);
        require!(
            user_bet.direction == bet.outcome,
            BettingError::NotAWinner
        );
        require!(
            ctx.accounts.vault_token_account.key() == bet.vault,
            BettingError::InvalidVault
        );

        // Calculate winnings
        let total_winning_amount = if bet.outcome {
            bet.total_yes_amount + bet.total_no_amount
        } else {
            bet.total_no_amount + bet.total_yes_amount
        };

        let winners_count = if bet.outcome {
            bet.yes_bettors
        } else {
            bet.no_bettors
        };

        let winning_amount = total_winning_amount / winners_count;

        // Transfer winnings
        let bet_account_key = bet.key();
        let vault_seeds = &[b"vault", bet_account_key.as_ref(), &[ctx.bumps.vault_authority]];
        let signer = &[&vault_seeds[..]];

        let transfer_instruction = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
                signer,
            ),
            winning_amount,
        )?;

        user_bet.claimed = true;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(title: String)]
pub struct CreateBet<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        space = 8 + Bet::INIT_SPACE,
        payer = signer,
        seeds = [title.as_ref()],
        bump,
    )]
    pub bet: Account<'info, Bet>,
    /// CHECK: PDA for vault authority
    #[account(
        seeds = [b"vault", bet.key().as_ref()],
        bump,
    )]
    pub vault_authority: AccountInfo<'info>,

    // CREATE AN ESCROW VAULT TOKEN ACCOUNT FOR A GIVEN BET
    #[account(
        init,
        token::mint = token_mint,
        token::authority = vault_authority,
        payer = signer,
        seeds = [b"vault_token_account", bet.key().as_ref()],
        bump,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub bettor: Signer<'info>,

    #[account(mut)]
    pub bet: Account<'info, Bet>,

    #[account(
        init,
        payer = bettor,
        space = 8 + UserBet::INIT_SPACE,
        seeds = [b"user_bet", bet.key().as_ref(), bettor.key().as_ref()],
        bump
    )]
    pub user_bet: Account<'info, UserBet>,

    #[account(
        mut,
        constraint = bettor_token_account.mint == bet.token_mint
    )]
    pub bettor_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.key() == bet.vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveBet<'info> {
    #[account(
        mut,
        constraint = bet.creator == creator.key()
    )]
    pub bet: Account<'info, Bet>,
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub bet: Account<'info, Bet>,
    #[account(
        mut,
        seeds = [b"user_bet", bet.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub user_bet: Account<'info, UserBet>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: PDA for vault authority
    #[account(
        seeds = [b"vault", bet.key().as_ref()],
        bump,
    )]
    pub vault_authority: AccountInfo<'info>,
    #[account(
        mut,
        constraint = vault_token_account.key() == bet.vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_token_account.mint == bet.token_mint
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Bet {
    pub creator: Pubkey,
    #[max_len(100)]
    pub title: String,
    pub bet_amount: u64,
    pub total_yes_amount: u64,
    pub total_no_amount: u64,
    pub yes_bettors: u64,
    pub no_bettors: u64,
    pub end_time: i64,
    pub resolved: bool,
    pub outcome: bool,
    pub token_mint: Pubkey,    // Added to track the token being used
    pub vault: Pubkey,         // Added to track the vault token account
    pub bump: u8,
    pub bump_vault_authority: u8,
    pub bump_vault_ta: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserBet {
    pub user: Pubkey,
    pub bet: Pubkey,
    pub amount: u64,
    pub direction: bool,
    pub claimed: bool,
    pub bump: u8
}

#[error_code]
pub enum BettingError {
    #[msg("Bet has already been resolved")]
    BetAlreadyResolved,
    #[msg("Bet end time has been exceeded")]
    BetEndTimeExceeded,
    #[msg("Bet has not ended yet")]
    BetNotEndedYet,
    #[msg("Bet has not been resolved yet")]
    BetNotResolved,
    #[msg("Winnings have already been claimed")]
    AlreadyClaimed,
    #[msg("User did not win this bet")]
    NotAWinner,
    #[msg("Invalid vault token account")]
    InvalidVault,
}