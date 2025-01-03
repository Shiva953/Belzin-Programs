import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Betting } from "../target/types/betting";
import { Keypair, PublicKey,LAMPORTS_PER_SOL } from "@solana/web3.js";
import {createMint, getAssociatedTokenAddress, getOrCreateAssociatedTokenAccount, mintTo} from "@solana/spl-token"
import { BN } from "@coral-xyz/anchor"
import {getKeypairFromFile} from "@solana-developers/helpers"

describe("Betting Contract Tests", () => {
  // Configure the client to use the local cluster.
  let tokenMint: PublicKey;
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Betting as Program<Betting>;


  const mint = Keypair.generate();

  before(async () => {
    const tokenCreator =  await getKeypairFromFile("~/.config/solana/id.json");
    tokenMint = await createMint(
      provider.connection,
      tokenCreator,
      provider.publicKey,
      null,
      6,  // 6 decimals like USDC
      mint
    );
    console.log("Token created: ", tokenMint)

    const amountToMint = 1000000 * Math.pow(10, 6); // Adjust for decimals
    const providerTokenAccount = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        tokenCreator,
        tokenMint,
        provider.publicKey
    );

    await mintTo(
        provider.connection,
        tokenCreator,
        tokenMint,
        providerTokenAccount.address,
        tokenCreator.publicKey,
        amountToMint
    );

    console.log(`Minted ${amountToMint} tokens to ${provider.publicKey.toString()}`);
  });


  it("Is bet created", async () => {
    const payerWallet = provider.wallet;
    console.log("Provider Wallet: ",payerWallet.publicKey.toBase58())

    const tx = await program.methods.createBet("SOL 500$ before December 2025?", new BN(100), new BN(100000))
    .accounts({
      signer: payerWallet.publicKey,
      tokenMint: tokenMint,
    })
    .rpc({ commitment: "confirmed"});
    console.log("Successfully created bet", tx);
  });

  it("Should place a bet", async() => {

    //here you're getting the bet account pubkey from the bet title, but in the blink you should have it before hand
    let [betAccountKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("SOL 500$ before December 2025?")],
      program.programId
    );
    const betAccount = await program.account.bet.fetch(betAccountKey);
    console.log("Bet Account Details: ", betAccount)
    console.log("Number of Yes Bettors: ", betAccount.yesBettors.toNumber())
    console.log("Number of No Bettors: ", betAccount.noBettors.toNumber())
    console.log("Total Yes Amount: ", betAccount.totalYesAmount.toNumber())
    console.log("Total No Amount: ", betAccount.totalNoAmount.toNumber())

    let [vaultTokenAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_token_account"), betAccountKey.toBuffer()],
      program.programId
    );

    const bettorTokenAccount = await getAssociatedTokenAddress(
      tokenMint,
      provider.publicKey,
      true
  );
  console.log("Bettor token Account: ",bettorTokenAccount)

    const vault_token_balance_0 = await provider.connection.getTokenAccountBalance(vaultTokenAccount)
    console.log("Vault Account Balance: ",vault_token_balance_0.value.amount)

    const tx = await program.methods
    .placeBet(true)
    .accounts({
      bettor: provider.wallet.publicKey,
      bet: betAccountKey,
      bettorTokenAccount: bettorTokenAccount,
      vaultTokenAccount: vaultTokenAccount,
    })
    .rpc({ commitment: "confirmed"});

    console.log("Successfully placed bet: ", tx)

    const updatedBetAccountState = await program.account.bet.fetch(betAccountKey);
    console.log("Bet Account Details(Updated): ", updatedBetAccountState)
    console.log("Number of Yes Bettors(Updated): ", updatedBetAccountState.yesBettors.toNumber())
    console.log("Number of No Bettors(Updated): ", updatedBetAccountState.noBettors.toNumber())
    console.log("Total Yes Amount(Updated): ", updatedBetAccountState.totalYesAmount.toNumber())
    console.log("Total No Amount(Updated): ", updatedBetAccountState.totalNoAmount.toNumber())

    const vault_token_balance = await provider.connection.getTokenAccountBalance(vaultTokenAccount)
    console.log("Vault Account Balance: ",vault_token_balance.value.amount)
  })

  //figure out the resolution part properly, use perplexity as resolution source
  it("Should Resolve the Bet", async() => {
    
  })

  it("Should Claim the Bet Amount on Winning", async() => {

  })

});
