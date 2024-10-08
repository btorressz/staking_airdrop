import * as anchor from '@coral-xyz/anchor';  
import { web3, Program } from '@coral-xyz/anchor'; 
import * as spl from '@solana/spl-token'; 
import { LAMPORTS_PER_SOL } from '@solana/web3.js'; 

// Initialize Provider
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = pg.program; 

console.log("My address:", provider.wallet.publicKey.toString());

// Wallet balance
(async () => {
  // Fetch and display the wallet balance
  const balance = await provider.connection.getBalance(provider.wallet.publicKey);
  console.log(`My balance: ${balance / LAMPORTS_PER_SOL} SOL`);

  // Example: Generate keypair for staking pool
  const poolKp = new web3.Keypair(); // Create a new keypair for the pool
  const mintAuthorityKp = new web3.Keypair(); // Mint authority keypair

  const INITIAL_REWARD = new anchor.BN(1000000); // 1,000,000 tokens as reward pool

  // Create and mint SPL token
  console.log("Creating a new token mint...");
  const mint = await spl.createMint(
    provider.connection, // Solana connection object
    mintAuthorityKp, // Mint authority
    mintAuthorityKp.publicKey, // Mint authority's public key
    null, // Freeze authority (optional)
    6 // Number of decimals (for SPL tokens)
  );

  // Create token accounts for user and pool
  console.log("Creating token accounts...");
  const poolTokenAccount = await spl.getOrCreateAssociatedTokenAccount(
    provider.connection, mintAuthorityKp, mint, poolKp.publicKey
  );

  const userTokenAccount = await spl.getOrCreateAssociatedTokenAccount(
    provider.connection, mintAuthorityKp, mint, provider.wallet.publicKey
  );

  // Mint some tokens to the user's token account
  console.log("Minting tokens to the user...");
  await spl.mintTo(
    provider.connection,
    mint,
    userTokenAccount.address, // Mint to user's token account
    mintAuthorityKp.publicKey, // Mint authority
    [mintAuthorityKp], // Signer
    1000 * 10 ** 6 // Mint 1000 tokens (6 decimals)
  );

  // Find PDA for pool
  const [poolPda, bump] = await web3.PublicKey.findProgramAddress(
    [Buffer.from("pool")],
    program.programId
  );

  // Initialize the staking pool
  console.log("Initializing the staking pool...");
  const tx = await program.methods.initializePool(INITIAL_REWARD, bump).accounts({
    pool: poolPda,
    user: provider.wallet.publicKey,
    systemProgram: web3.SystemProgram.programId,
  }).signers([poolKp]).rpc();

  console.log(`Staking pool initialized with transaction: ${tx}`);
})();
