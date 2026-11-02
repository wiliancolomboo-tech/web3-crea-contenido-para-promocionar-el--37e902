import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { EncuentroPromo } from "../target/types/encuentro_promo";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { assert, expect } from "chai";
import * as crypto from "crypto";

describe("encuentro-promo protocol tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.EncuentroPromo as Program<EncuentroPromo>;

  let authority: Keypair;
  let oracle: Keypair;
  let creator: Keypair;
  let rewardMint: PublicKey;
  let creatorTokenAccount: PublicKey;
  let campaignPda: PublicKey;
  let escrowVaultPda: PublicKey;

  const INITIAL_ESCROW_AMOUNT = new anchor.BN(10_000_000_000); // 10,000 Tokens (6 decimals)
  const TIER_GOLD_REWARD = new anchor.BN(250_000_000); // 250 Tokens

  before(async () => {
    authority = Keypair.generate();
    oracle = Keypair.generate();
    creator = Keypair.generate();

    // Airdrop SOL to test actors
    for (const actor of [authority, oracle, creator]) {
      const sig = await provider.connection.requestAirdrop(actor.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);
    }

    // Create reward Mint (USDC simulation)
    rewardMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      6
    );

    // Derive PDAs
    [campaignPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("encuentro-campaign"), authority.publicKey.toBuffer(), rewardMint.toBuffer()],
      program.programId
    );

    [escrowVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow-vault"), campaignPda.toBuffer()],
      program.programId
    );

    // Initialize Creator ATA
    creatorTokenAccount = await createAccount(
      provider.connection,
      creator,
      rewardMint,
      creator.publicKey
    );
  });

  it("1. Successfully initializes the Madrid 2026 Campaign and Escrow Vault", async () => {
    await program.methods
      .initializeCampaign(INITIAL_ESCROW_AMOUNT)
      .accounts({
        authority: authority.publicKey,
        oracleAuthority: oracle.publicKey,
        rewardMint: rewardMint,
        campaign: campaignPda,
        escrowVault: escrowVaultPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authority])
      .rpc();

    // Fund the escrow vault with minted tokens
    await mintTo(
      provider.connection,
      authority,
      rewardMint,
      escrowVaultPda,
      authority,
      BigInt(INITIAL_ESCROW_AMOUNT.toString())
    );

    const campaignState = await program.account.campaign.fetch(campaignPda);
    assert.isTrue(campaignState.authority.equals(authority.publicKey));
    assert.isTrue(campaignState.oracleAuthority.equals(oracle.publicKey));
    assert.equal(campaignState.totalAllocated.toString(), INITIAL_ESCROW_AMOUNT.toString());
    assert.equal(campaignState.totalDistributed.toString(), "0");
    assert.isTrue(campaignState.isActive);

    const vaultAccount = await getAccount(provider.connection, escrowVaultPda);
    assert.equal(vaultAccount.amount.toString(), INITIAL_ESCROW_AMOUNT.toString());
  });

  it("2. Registers a verified promotional content submission by Creator", async () => {
    const rawContentUrl = "https://x.com/es_solana/status/1792837192837?s=20";
    const contentHash = crypto.createHash("sha256").update(rawContentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    await program.methods
      .submitContent(Array.from(contentHash), 0) // Platform 0: X
      .accounts({
        creator: creator.publicKey,
        campaign: campaignPda,
        submission: submissionPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    const submissionState = await program.account.submission.fetch(submissionPda);
    assert.isTrue(submissionState.creator.equals(creator.publicKey));
    assert.isFalse(submissionState.isVerified);
    assert.equal(submissionState.tierAwarded, 0);

    const campaignState = await program.account.campaign.fetch(campaignPda);
    assert.equal(campaignState.totalSubmissions.toNumber(), 1);
  });

  it("3. Oracle verifies content reach and disburses on-chain reward", async () => {
    const rawContentUrl = "https://x.com/es_solana/status/1792837192837?s=20";
    const contentHash = crypto.createHash("sha256").update(rawContentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    await program.methods
      .verifyAndReward(3, TIER_GOLD_REWARD) // Tier 3 = Gold
      .accounts({
        oracle: oracle.publicKey,
        campaign: campaignPda,
        submission: submissionPda,
        escrowVault: escrowVaultPda,
        creatorTokenAccount: creatorTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([oracle])
      .rpc();

    const submissionState = await program.account.submission.fetch(submissionPda);
    assert.isTrue(submissionState.isVerified);
    assert.equal(submissionState.tierAwarded, 3);
    assert.equal(submissionState.rewardAmount.toString(), TIER_GOLD_REWARD.toString());

    // Verify creator received SPL Tokens
    const creatorAta = await getAccount(provider.connection, creatorTokenAccount);
    assert.equal(creatorAta.amount.toString(), TIER_GOLD_REWARD.toString());
  });

  it("4. Rejects duplicate payout on already processed submission (Replay Protection)", async () => {
    const rawContentUrl = "https://x.com/es_solana/status/1792837192837?s=20";
    const contentHash = crypto.createHash("sha256").update(rawContentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    try {
      await program.methods
        .verifyAndReward(3, TIER_GOLD_REWARD)
        .accounts({
          oracle: oracle.publicKey,
          campaign: campaignPda,
          submission: submissionPda,
          escrowVault: escrowVaultPda,
          creatorTokenAccount: creatorTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([oracle])
        .rpc();
      assert.fail("Should have thrown error on already processed submission.");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("SubmissionAlreadyProcessed");
    }
  });

  it("5. Rejects unauthorized callers attempting to trigger oracle reward settlement", async () => {
    const maliciousActor = Keypair.generate();
    const sig = await provider.connection.requestAirdrop(maliciousActor.publicKey, anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig);

    const rawContentUrl = "https://youtube.com/watch?v=encuentro_madrid";
    const contentHash = crypto.createHash("sha256").update(rawContentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    // Register submission first
    await program.methods
      .submitContent(Array.from(contentHash), 1)
      .accounts({
        creator: creator.publicKey,
        campaign: campaignPda,
        submission: submissionPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    try {
      await program.methods
        .verifyAndReward(1, new anchor.BN(25_000_000))
        .accounts({
          oracle: maliciousActor.publicKey, // Invalid signer
          campaign: campaignPda,
          submission: submissionPda,
          escrowVault: escrowVaultPda,
          creatorTokenAccount: creatorTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([maliciousActor])
        .rpc();
      assert.fail("Should have failed with UnauthorizedSigner");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("UnauthorizedSigner");
    }
  });
});