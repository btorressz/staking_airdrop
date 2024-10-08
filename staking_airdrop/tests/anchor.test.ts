import * as anchor from '@coral-xyz/anchor';
import * as spl from '@solana/spl-token';

describe("Staking Airdrop", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.StakingAirdrop;

  const poolKp = anchor.web3.Keypair.generate();
  const stakerKp = anchor.web3.Keypair.generate();
  const mintAuthorityKp = anchor.web3.Keypair.generate();
  let mint = null;
  let userTokenAccount = null;
  let poolTokenAccount = null;

  const INITIAL_REWARD = new anchor.BN(1000000); // Example reward amount

  it("Initialize the reward pool", async () => {
    // Create an SPL token mint for testing
    mint = await spl.createMint(
      provider.connection,
      mintAuthorityKp,
      mintAuthorityKp.publicKey,
      null,
      6 // Decimals
    );

    // Create token accounts
    poolTokenAccount = await spl.getOrCreateAssociatedTokenAccount(
      provider.connection,
      mintAuthorityKp,
      mint,
      poolKp.publicKey
    );

    userTokenAccount = await spl.getOrCreateAssociatedTokenAccount(
      provider.connection,
      mintAuthorityKp,
      mint,
      provider.wallet.publicKey
    );

    // Mint tokens to user account
    await spl.mintTo(
      provider.connection,
      mintAuthorityKp,
      mint,
      userTokenAccount.address,
      mintAuthorityKp.publicKey,
      1000 * 10 ** 6 // Mint 1000 tokens
    );

    // Find the pool PDA and calculate bump
    const [poolPda, bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("pool")],
      program.programId
    );

    // Initialize the pool
    await program.methods
      .initializePool(INITIAL_REWARD, bump)
      .accounts({
        pool: poolPda,
        user: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([poolKp])
      .rpc();

    const poolAccount = await program.account.airdropPool.fetch(poolPda);
    console.log("Pool initialized:", poolAccount);
    assert.ok(poolAccount.totalReward.eq(INITIAL_REWARD));
  });

  it("Stake tokens", async () => {
    const STAKE_AMOUNT = new anchor.BN(100);
    const STAKE_PERIOD = new anchor.BN(30 * 24 * 60 * 60); // 30 days

    const tx = await program.methods
      .stakeTokens(STAKE_AMOUNT, STAKE_PERIOD)
      .accounts({
        stakerAccount: stakerKp.publicKey,
        pool: poolKp.publicKey,
        userTokenAccount: userTokenAccount.address,
        poolTokenAccount: poolTokenAccount.address,
        user: provider.wallet.publicKey,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([stakerKp])
      .rpc();
    console.log(`Staked tokens with tx: ${tx}`);

    const stakerAccount = await program.account.stakerAccount.fetch(stakerKp.publicKey);
    assert.ok(stakerAccount.amountStaked.eq(STAKE_AMOUNT));
  });

  it("Unstake and claim rewards", async () => {
    const tx = await program.methods
      .unstakeAndClaim()
      .accounts({
        stakerAccount: stakerKp.publicKey,
        pool: poolKp.publicKey,
        userTokenAccount: userTokenAccount.address,
        poolTokenAccount: poolTokenAccount.address,
        user: provider.wallet.publicKey,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log(`Unstaked and claimed rewards with tx: ${tx}`);

    const stakerAccount = await program.account.stakerAccount.fetch(stakerKp.publicKey);
    assert.ok(stakerAccount.amountStaked.eq(new anchor.BN(0)));
  });
});
