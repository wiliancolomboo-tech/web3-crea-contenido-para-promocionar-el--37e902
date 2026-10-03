import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { expect } from "chai";
import { createHash } from "crypto";
import { EncuentroBoost } from "../target/types/encuentro_boost";

describe("encuentro_boost program integration tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.EncuentroBoost as Program<EncuentroBoost>;

  const CAMPAIGN_SEED = Buffer.from("encuentro_campaign");
  const SUBMISSION_SEED = Buffer.from("content_submission");
  const VAULT_SEED = Buffer.from("vault_escrow");

  const campaignName = "madrid_2026_promo";
  const rewardAmount = new anchor.BN(0.5 * LAMPORTS_PER_SOL);
  const maxSubmissions = 50;
  const initialFunding = new anchor.BN(5 * LAMPORTS_PER_SOL);

  const oracleKeypair = Keypair.generate();
  const creatorKeypair = Keypair.generate();
  const unauthorizedUser = Keypair.generate();

  let campaignPda: PublicKey;
  let vaultPda: PublicKey;

  before(async () => {
    // Airdrop SOL to test actors
    const txAirdropCreator = await provider.connection.requestAirdrop(
      creatorKeypair.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(txAirdropCreator);

    const txAirdropOracle = await provider.connection.requestAirdrop(
      oracleKeypair.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(txAirdropOracle);

    [campaignPda] = PublicKey.findProgramAddressSync(
      [CAMPAIGN_SEED, Buffer.from(campaignName)],
      program.programId
    );

    [vaultPda] = PublicKey.findProgramAddressSync(
      [VAULT_SEED, campaignPda.toBuffer()],
      program.programId
    );
  });

  it("1. Successfully initializes the El Encuentro 2026 campaign & deposits escrow", async () => {
    await program.methods
      .initializeCampaign(
        campaignName,
        rewardAmount,
        maxSubmissions,
        initialFunding
      )
      .accounts({
        campaign: campaignPda,
        vault: vaultPda,
        oracleSigner: oracleKeypair.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const campaignAccount = await program.account.campaignState.fetch(campaignPda);
    expect(campaignAccount.campaignName).to.equal(campaignName);
    expect(campaignAccount.rewardPerSubmission.toString()).to.equal(rewardAmount.toString());
    expect(campaignAccount.maxSubmissions).to.equal(maxSubmissions);
    expect(campaignAccount.totalSubmissions).to.equal(0);
    expect(campaignAccount.isActive).to.be.true;

    const vaultBalance = await provider.connection.getBalance(vaultPda);
    expect(vaultBalance).to.be.gte(initialFunding.toNumber());
  });

  it("2. Registers technical promotional content from a community creator", async () => {
    const contentUrl = "https://x.com/solana/status/encuentro-madrid-2026-launch";
    const contentHash = createHash("sha256").update(contentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [SUBMISSION_SEED, campaignPda.toBuffer(), contentHash],
      program.programId
    );

    await program.methods
      .submitContent(Array.from(contentHash), contentUrl)
      .accounts({
        campaign: campaignPda,
        submission: submissionPda,
        creator: creatorKeypair.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([creatorKeypair])
      .rpc();

    const submissionAccount = await program.account.contentSubmissionState.fetch(submissionPda);
    expect(submissionAccount.creator.toBase58()).to.equal(creatorKeypair.publicKey.toBase58());
    expect(submissionAccount.contentUri).to.equal(contentUrl);
    expect(submissionAccount.verified).to.be.false;
    expect(submissionAccount.rewarded).to.be.false;

    const campaignAccount = await program.account.campaignState.fetch(campaignPda);
    expect(campaignAccount.totalSubmissions).to.equal(1);
  });

  it("3. Prevents duplicate content hash registration (Sybil resistance)", async () => {
    const contentUrl = "https://x.com/solana/status/encuentro-madrid-2026-launch";
    const contentHash = createHash("sha256").update(contentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [SUBMISSION_SEED, campaignPda.toBuffer(), contentHash],
      program.programId
    );

    try {
      await program.methods
        .submitContent(Array.from(contentHash), contentUrl)
        .accounts({
          campaign: campaignPda,
          submission: submissionPda,
          creator: creatorKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([creatorKeypair])
        .rpc();
      expect.fail("Should have rejected duplicate content hash");
    } catch (err: any) {
      expect(err.toString()).to.include("already in use");
    }
  });

  it("4. Oracle verifies promotional content and unlocks reward distribution", async () => {
    const contentUrl = "https://x.com/solana/status/encuentro-madrid-2026-launch";
    const contentHash = createHash("sha256").update(contentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [SUBMISSION_SEED, campaignPda.toBuffer(), contentHash],
      program.programId
    );

    const initialCreatorBal = await provider.connection.getBalance(creatorKeypair.publicKey);

    await program.methods
      .verifyAndReward()
      .accounts({
        campaign: campaignPda,
        submission: submissionPda,
        creator: creatorKeypair.publicKey,
        vault: vaultPda,
        oracleSigner: oracleKeypair.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([oracleKeypair])
      .rpc();

    const updatedCreatorBal = await provider.connection.getBalance(creatorKeypair.publicKey);
    expect(updatedCreatorBal - initialCreatorBal).to.equal(rewardAmount.toNumber());

    const submissionAccount = await program.account.contentSubmissionState.fetch(submissionPda);
    expect(submissionAccount.verified).to.be.true;
    expect(submissionAccount.rewarded).to.be.true;
  });

  it("5. Rejects unauthorized attempts to verify and reward submissions", async () => {
    const contentUrl = "https://dev.to/solana/building-solana-apps-madrid-2026";
    const contentHash = createHash("sha256").update(contentUrl).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [SUBMISSION_SEED, campaignPda.toBuffer(), contentHash],
      program.programId
    );

    await program.methods
      .submitContent(Array.from(contentHash), contentUrl)
      .accounts({
        campaign: campaignPda,
        submission: submissionPda,
        creator: creatorKeypair.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([creatorKeypair])
      .rpc();

    try {
      await program.methods
        .verifyAndReward()
        .accounts({
          campaign: campaignPda,
          submission: submissionPda,
          creator: creatorKeypair.publicKey,
          vault: vaultPda,
          oracleSigner: unauthorizedUser.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([unauthorizedUser])
        .rpc();
      expect.fail("Unauthorized oracle was permitted to release rewards");
    } catch (err: any) {
      expect(err.error?.errorCode?.code || err.toString()).to.satisfy(
        (val: string) => val.includes("UnauthorizedOracle") || val.includes("unknown signer")
      );
    }
  });
});