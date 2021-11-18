use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use spl_associated_token_account::{create_associated_token_account};
use spl_token_metadata::{
    instruction::{
        create_metadata_accounts,        
    },
    
};

use anchor_lang::solana_program::{
    program::{invoke, invoke_signed},
    system_instruction::transfer,     
};

use switchboard_program;
use switchboard_program::{AggregatorState, RoundResult};



declare_id!("D4gZCnjamagp3VRQMZm3s2uhp3FM6UXPnMVk8ARpMoNk");

#[program]
pub mod charmtoken {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, name: String, symbol: String, url: String) -> ProgramResult {
        //Create Metadata
          
        msg!("Making metadata accounts vector...");
        let metadata_infos = vec![
            ctx.accounts.metadata_account.clone(),
            ctx.accounts.metadata_program.clone(),
            ctx.accounts.mint.clone(),
            ctx.accounts.mint_authority.clone(),
            ctx.accounts.payer.clone(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ];
        msg!("Making metadata instruction");

        let instruction = create_metadata_accounts(
            *ctx.accounts.metadata_program.key,
            *ctx.accounts.metadata_account.key,
            *ctx.accounts.mint.key,
            *ctx.accounts.mint_authority.key,
            *ctx.accounts.payer.key,
            *ctx.accounts.payer.key,
            name.to_string(),
            symbol.to_string(),
            url.to_string(),
            None,
            //Default creator royality.. will be changed to client
            0,
            //At the moment defaulting to update authority as signer as well... will be changed to client
            true,
            true,
        );
        msg!("Calling the metadata program to make metadata...");
        invoke(&instruction, metadata_infos.as_slice())?;

        
        msg!("Metadata created");  
       

        Ok(())
    }


    pub fn change_ownership(ctx: Context<ChangeOwnership>) -> ProgramResult{

        msg!("Transffer account authority");

        let change_ownership = spl_token::instruction::set_authority(
            &ctx.accounts.token_program.key,
            &ctx.accounts.token_purse.to_account_info().key,
            Some(&ctx.accounts.future_authority.key),
            spl_token::instruction::AuthorityType::AccountOwner,
            &ctx.accounts.payer.key,
            &[&ctx.accounts.payer.key],
        )?;
        
        invoke(
            &change_ownership,
            &[ctx.accounts.token_purse.to_account_info(), 
                ctx.accounts.payer.clone(), 
                ctx.accounts.token_program.to_account_info(),
                ],
        )?;

        msg!("Account ownership transfered...");

        Ok(())
        
    }

    pub fn create_associated_account(ctx: Context<CreateAssociated>) -> Result<()> {
        
        msg!("Creating Associated account");
        let create_ix = create_associated_token_account(
            &ctx.accounts.signer.key,
            &ctx.accounts.signer.key,
            &ctx.accounts.mint.key,
        );
        let associated_required_accounts = vec![
            ctx.accounts.signer.clone(),
            ctx.accounts.user_account.clone(),
            ctx.accounts.signer.clone(),
            ctx.accounts.mint.clone(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.rent_program.to_account_info(),
            ctx.accounts.associated_program.clone(),
        ];

        msg!("Invoking instruction to create account 1111 trying");
        invoke(&create_ix, &associated_required_accounts)?;
       
 
        Ok(())
    }


    pub fn buy_charm(ctx: Context<BuyCharm>, bump: u8, lamports: u64) -> ProgramResult {
        msg!("Buying charm");
        if lamports <= 0 {
            return Err(ErrorCode::InvalidInstruction.into());
        }
        let lamport_balance = **ctx.accounts.signer.to_account_info().try_borrow_lamports()?;
        if lamport_balance < lamports {
            return Err(ErrorCode::InvalidInstruction.into());
        }

        
        const BASE:u64 = 10;
        //const PRIVATE_SALE_SUPPLY: u64 = 50* BASE.pow(15);
        //const PUBLIC_SALE_SUPPLY: u64 = 115* BASE.pow(15);

        const CHARM_BUY_LIMIT: u64 = 2* BASE.pow(15);
        msg!("Charm Buy limit {:?}", CHARM_BUY_LIMIT);      
        let user_account = &ctx.accounts.user_account;        
        if  user_account.amount > CHARM_BUY_LIMIT {
            msg!("user charm balance {:?}", user_account.amount);
            return Err(ErrorCode::SoldOut.into());
        }



        
        /* let mint_data:Mint = Pack::unpack_from_slice(&ctx.accounts.mint.data.borrow())?;
        msg!("Mint Supply {:?}", mint_data.supply);
        let mint_supply = mint_data.supply; */
        
        
        let aggregator:AggregatorState = switchboard_program::get_aggregator(&ctx.accounts.aggregator_feed_account)?;
        let round_result:RoundResult = switchboard_program::get_aggregator_result(&aggregator)?;

        
        let sol_price = round_result.result.unwrap_or(0.00);
        if sol_price == 0.00{
            return Err(ErrorCode::PriceNotFound.into());
        }

        msg!("Solana price {:?}", sol_price);
        let price_per_token = 1_000_000_000.00/(sol_price*100.0*2.0);
        msg!("Price per Token {:?}", price_per_token);

        let tokens_to_buy:f64 = lamports as f64/price_per_token;
        msg!("Tokens to buy {:?}", tokens_to_buy);
        msg!("Tokens acurrently available {:?}", user_account.amount);
        let tokens_after_purchase = tokens_to_buy as u64 + user_account.amount;
        msg!("Tokens after_purchase {:?}", tokens_after_purchase);

        if tokens_after_purchase > CHARM_BUY_LIMIT{
            msg!("Tokens after_purchase condition executed");       
            return Err(ErrorCode::SoldOut.into());
        }
        msg!("Tokens after_purchase condition not executed");

        //Transffer sols from user to program
        msg!("Making instruction for Deducting Lamports");
        let transfer_ix = transfer(
            &ctx.accounts.signer.key,
            &ctx.accounts.charm_account.key,
            lamports,            
        );
        invoke(&transfer_ix, &[
            ctx.accounts.signer.clone(), 
            ctx.accounts.charm_account.clone(),
            ctx.accounts.system_program.to_account_info(),
            ])?;
        
        msg!("Making instruction for transfer tokens to users account");
        let ix = spl_token::instruction::transfer(
            &ctx.accounts.token_program.key,
            &ctx.accounts.token_purse.to_account_info().key,
            &ctx.accounts.user_account.to_account_info().key,
            &ctx.accounts.pda.key,
            &[&ctx.accounts.pda.key],
            (tokens_to_buy*1_000_000_000.00) as u64,
        )?;
        msg!("Invoking instruction for faucet");
        invoke_signed(
            &ix,
            &[
                ctx.accounts.token_purse.to_account_info(),
                ctx.accounts.user_account.to_account_info(),
                ctx.accounts.pda.clone(),
                ctx.accounts.token_program.to_account_info(),
            ],
            &[&[&b"charmpda"[..], &[bump]]],
        )?;
        

        Ok(())
    }

}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(signer)]
    pub mint_authority: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    #[account(mut)]
    pub metadata_account: AccountInfo<'info>,
    #[account(executable)]
    pub metadata_program: AccountInfo<'info>,
    #[account(executable)]
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent_program: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ChangeOwnership<'info> {
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    #[account(mut)]
    pub token_purse: AccountInfo<'info>,
    #[account(mut)]
    pub future_authority: AccountInfo<'info>,
    #[account(executable)]
    pub token_program: Program<'info, Token>,
    
}

#[derive(Accounts)]
pub struct CreateAssociated<'info> {
    #[account(signer)]
    pub signer: AccountInfo<'info>,
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub user_account: AccountInfo<'info>,
    #[account(executable)]
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent_program: Sysvar<'info, Rent>,
    #[account(executable)]
    pub associated_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BuyCharm<'info> {
    #[account(signer)]
    pub signer: AccountInfo<'info>,
    #[account(mut)]
    pub token_purse: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pda: AccountInfo<'info>,
    pub aggregator_feed_account: AccountInfo<'info>,
    #[account(mut, address = "Hjzu13Y262nDZCwNDtHURukEHryw9CmM5ZDFXqRN6Zxb".parse().unwrap())]
    pub charm_account: AccountInfo<'info>,    
    #[account(executable)]
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    
}





#[error]
pub enum ErrorCode {
    #[msg("Associate account already exists")]
    AssociateAccountAlreadyExists,
    #[msg("Not yet available")]
    NotYetAvailable,
    #[msg("Sold out")]
    SoldOut,
    #[msg("Invalid Instruction")]
    InvalidInstruction,
    #[msg("Price Not Found, Please try again")]
    PriceNotFound,

}
