use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{CloseAccount, Mint, Token, TokenAccount, Transfer}};


declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod staking {
    use super::*;
    use anchor_spl::token::Transfer;

    pub fn initialize(ctx: Context<Initialize>, application_idx: u64, state_bump: u8, _wallet_bump: u8, amount: u64) -> Result<()> {

        let state = &mut ctx.accounts.application_state;
        state.idx = application_idx;
        state.user_sending = ctx.accounts.user_sending.key().clone();
        state.mint_of_token_being_sent = ctx.accounts.mint_of_token_being_sent.key().clone();
        state.escrow_wallet = ctx.accounts.escrow_wallet_state.key().clone();
        state.amount_tokens = amount;

        msg!("Initialized new Safe Transfer instance for {}", amount);

        // CPI time! we now need to call into the Token program to transfer our funds to the 
        // Escrow wallet. Our state account account is a PDA, which means that no private key
        // exists for the corresponding public key and therefore this key was not signed in the original 
        // transaction. Our program is the only entity that can programmatically sign for the PDA
        // and we can do this by specifying the PDA "derivation hash key" and using `CpiContext::new_with_signer()`.

        // This specific step is very different compared to Ethereum. In Ethereum, accounts need to first set allowances towards 
        // a specific contract (like ZeroEx, Uniswap, Curve..) before the contract is able to withdraw funds. In this other case,
        // the SafePay program can use Bob's signature to "authenticate" the `transfer()` instruction sent to the token contract.
        let bump_vector = state_bump.to_le_bytes();
        let mint_of_token_being_sent_pk = ctx.accounts.mint_of_token_being_sent.key().clone();
        let application_idx_bytes = application_idx.to_le_bytes();
        let inner = vec![
            b"state".as_ref(),
            ctx.accounts.user_sending.key.as_ref(),
            mint_of_token_being_sent_pk.as_ref(), 
            application_idx_bytes.as_ref(),
            bump_vector.as_ref(),
        ];
        let outer = vec![inner.as_slice()];

        // Below is the actual instruction that we are going to send to the Token program.
        let transfer_instruction = Transfer{
            from: ctx.accounts.wallet_to_withdraw_from.to_account_info(),
            to: ctx.accounts.escrow_wallet_state.to_account_info(),
            authority: ctx.accounts.user_sending.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            outer.as_slice(),
        );

        // The `?` at the end will cause the function to return early in case of an error.
        // This pattern is common in Rust.
        anchor_spl::token::transfer(cpi_ctx, state.amount_tokens)?;

        Ok(())
    }

    pub fn withdraw_funds(ctx: Context<Withdraw>, application_idx: u64, state_bump: u8, _wallet_bump: u8, amount: u64, time_period: u64) -> Result<()> {

        let state = &mut ctx.accounts.application_state;
        // state.idx = application_idx;
        // state.user_sending = ctx.accounts.user_sending.key().clone();
        // state.mint_of_token_being_sent = ctx.accounts.mint_of_token_being_sent.key().clone();
        // state.escrow_wallet = ctx.accounts.escrow_wallet_state.key().clone();
        // state.amount_tokens = amount;

        if time_period < 2 {
            return Err(error!(ErrorCode::Hello))
        };

        let total_amount = amount + (amount * time_period * 10)/100;

        msg!("Initialized new Safe Refund instance for {}", total_amount);

        // CPI time! we now need to call into the Token program to transfer our funds to the 
        // Escrow wallet. Our state account account is a PDA, which means that no private key
        // exists for the corresponding public key and therefore this key was not signed in the original 
        // transaction. Our program is the only entity that can programmatically sign for the PDA
        // and we can do this by specifying the PDA "derivation hash key" and using `CpiContext::new_with_signer()`.

        // This specific step is very different compared to Ethereum. In Ethereum, accounts need to first set allowances towards 
        // a specific contract (like ZeroEx, Uniswap, Curve..) before the contract is able to withdraw funds. In this other case,
        // the SafePay program can use Bob's signature to "authenticate" the `transfer()` instruction sent to the token contract.
        let bump_vector = state_bump.to_le_bytes();
        let mint_of_token_being_sent_pk = ctx.accounts.mint_of_token_being_sent.key().clone();
        let application_idx_bytes = application_idx.to_le_bytes();
        let inner = vec![
            b"state".as_ref(),
            ctx.accounts.user_sending.key.as_ref(),
            mint_of_token_being_sent_pk.as_ref(), 
            application_idx_bytes.as_ref(),
            bump_vector.as_ref(),
        ];
        let outer = vec![inner.as_slice()];


        // Below is the actual instruction that we are going to send to the Token program.
        let transfer_instruction = Transfer {
            from: ctx.accounts.escrow_wallet_state.to_account_info(),
            to: ctx.accounts.refund_wallet.to_account_info(),
            authority: ctx.accounts.application_state.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            outer.as_slice(),
        );

        // The `?` at the end will cause the function to return early in case of an error.
        // This pattern is common in Rust.
        anchor_spl::token::transfer(cpi_ctx, total_amount)?;

        Ok(())
    }
 


}

#[derive(Accounts)]
#[instruction(application_idx: u64)]
pub struct Initialize<'info> {
    #[account(init, payer = user_sending,
        seeds = [b"state".as_ref(),
                 user_sending.key().as_ref(), 
                 mint_of_token_being_sent.key().as_ref(), 
                 application_idx.to_le_bytes().as_ref()],
        bump,space = 800
    )]
    application_state: Account<'info, State>,
    #[account(init, payer = user_sending,
        seeds = [b"wallet".as_ref(),
                mint_of_token_being_sent.key().as_ref(), 
                application_idx.to_le_bytes().as_ref()],
        bump,
        token::mint=mint_of_token_being_sent,
        token::authority=application_state,
    )]
    escrow_wallet_state: Account<'info, TokenAccount>,
    #[account(mut)]
    user_sending: Signer<'info>,
    mint_of_token_being_sent: Account<'info, Mint>,
    #[account(
        mut,
        constraint=wallet_to_withdraw_from.owner == user_sending.key(),
        constraint=wallet_to_withdraw_from.mint == mint_of_token_being_sent.key()
    )]
    wallet_to_withdraw_from: Account<'info, TokenAccount>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,

}

#[derive(Accounts)]
#[instruction(application_idx: u64, state_bump: u8, wallet_bump: u8)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"state".as_ref(),
                 user_sending.key().as_ref(), 
                 mint_of_token_being_sent.key().as_ref(), 
                 application_idx.to_le_bytes().as_ref()],
        bump = state_bump
    )]
    application_state: Account<'info, State>,
    #[account(
        mut,
        seeds = [b"wallet".as_ref(),
                mint_of_token_being_sent.key().as_ref(), 
                application_idx.to_le_bytes().as_ref()],
        bump = wallet_bump,
        token::mint=mint_of_token_being_sent,
        token::authority=application_state,
    )]
    escrow_wallet_state: Account<'info, TokenAccount>,
    #[account(mut)]
    user_sending: Signer<'info>,
    mint_of_token_being_sent: Account<'info, Mint>,
    #[account(
        mut,
        // constraint=refund_wallet.owner == user_sending.key(),
        // constraint=refund_wallet.mint == mint_of_token_being_sent.key()
    )]
    refund_wallet: Account<'info, TokenAccount>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,

}



#[account]
pub struct State {
    idx: u64,
    user_sending: Pubkey,
    mint_of_token_being_sent: Pubkey,
    escrow_wallet: Pubkey,
    amount_tokens: u64,
    stage: u8
}

#[error_code]
pub enum ErrorCode {
    #[msg("This is an error message clients will automatically display")]
    Hello,
}
