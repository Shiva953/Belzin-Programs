use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("GziZBjauok6LcPAoXuBPCXhfpg38WzUT5Gj3h9UmgnT3");

#[program]
pub mod betting_contracts {
    use super::*;

    pub fn initialize_bet(
        ctx: Context<InitializeBet>,
        title: String,
        bet_amount: u64,
    ) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        bet.creator = ctx.accounts.creator.key();
        bet.title = title;
        bet.bet_amount = bet_amount;
        bet.total_yes_amount = 0;
        bet.total_no_amount = 0;
        bet.yes_bettors = Vec::new();
        bet.no_bettors = Vec::new();
        bet.resolved = false;
        Ok(())
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        side: bool,
    ) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        require!(!bet.resolved, ErrorCode::BetAlreadyResolved);

        // Transfer tokens to escrow
        let transfer_instruction = Transfer {
            from: ctx.accounts.bettor_token_account.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.bettor.to_account_info(),
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
            ),
            bet.bet_amount,
        )?;

        if side {
            bet.yes_bettors.push(ctx.accounts.bettor.key());
            bet.total_yes_amount += bet.bet_amount;
        } else {
            bet.no_bettors.push(ctx.accounts.bettor.key());
            bet.total_no_amount += bet.bet_amount;
        }

        Ok(())
    }

    pub fn resolve_bet(
        ctx: Context<ResolveBet>,
        winning_side: bool,
    ) -> Result<()> {
        let bet = &mut ctx.accounts.bet;
        require!(!bet.resolved, ErrorCode::BetAlreadyResolved);
        require!(
            ctx.accounts.creator.key() == bet.creator,
            ErrorCode::UnauthorizedResolver
        );

        let winners = if winning_side {
            &bet.yes_bettors
        } else {
            &bet.no_bettors
        };

        let total_pot = bet.total_yes_amount + bet.total_no_amount;
        let winners_count = winners.len() as u64;
        let profit_per_winner = if winners_count > 0 {
            total_pot / winners_count
        } else {
            0
        };

        for winner in winners {
            let winner_account = next_account_info(ctx.accounts.remaining_accounts.iter())?;
            let winner_token_account = next_account_info(ctx.accounts.remaining_accounts.iter())?;

            require!(winner_account.key() == winner, ErrorCode::InvalidWinner);

            // Transfer winnings
            let transfer_instruction = Transfer {
                from: ctx.accounts.escrow_token_account.to_account_info(),
                to: winner_token_account.to_account_info(),
                authority: ctx.accounts.bet.to_account_info(),
            };

            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    transfer_instruction,
                    &[&[
                        b"bet",
                        bet.creator.as_ref(),
                        &[ctx.bumps.bet],
                    ]],
                ),
                profit_per_winner,
            )?;
        }

        bet.resolved = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeBet<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + 32 + 200 + 8 + 8 + 8 + 1000 + 1000 + 1,
        seeds = [b"bet", creator.key().as_ref()],
        bump
    )]
    pub bet: Account<'info, Bet>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub bet: Account<'info, Bet>,
    #[account(mut)]
    pub bettor: Signer<'info>,
    #[account(mut)]
    pub bettor_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ResolveBet<'info> {
    #[account(mut)]
    pub bet: Account<'info, Bet>,
    pub creator: Signer<'info>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Bet {
    pub creator: Pubkey,
    pub title: String,
    pub bet_amount: u64,
    pub total_yes_amount: u64,
    pub total_no_amount: u64,
    pub yes_bettors: Vec<Pubkey>,
    pub no_bettors: Vec<Pubkey>,
    pub resolved: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Bet has already been resolved")]
    BetAlreadyResolved,
    #[msg("Only the creator can resolve the bet")]
    UnauthorizedResolver,
    #[msg("Invalid winner account provided")]
    InvalidWinner,
}
