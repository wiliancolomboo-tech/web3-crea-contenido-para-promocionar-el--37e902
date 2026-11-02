import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { EncuentroPromoter } from "../target/types/encuentro_promoter";
import { PublicKey, Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import * as crypto from "crypto";

describe("encuentro_promoter protocol tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.EncuentroPromoter as Program<EncuentroPromoter>;

  const campaignId = "madrid-2026-v1";
  const rewardPerSubmission = new anchor.BN(0.25 * LAMPORTS_PER_SOL);
  const totalSlots = 10;

  const oracleAuthority = Keypair.generate();
  const creatorKeypair = Keypair.generate();
  const impostorKeypair = Keypair.generate();

  let campaignPda: PublicKey;
  let vaultPda: PublicKey;
  let creatorPda: PublicKey;

  before(async () => {
    // Airdrop SOL to test participants
    const airdropSig1 = await provider.connection.requestAirdrop(oracleAuthority.publicKey, 2 * LAMPORTS_PER_SOL);
    const airdropSig2 = await provider.connection.requestAirdrop(creatorKeypair.publicKey, 2 * LAMPORTS_PER_SOL);
    const airdropSig3 = await provider.connection.requestAirdrop(impostorKeypair.publicKey, 2 * LAMPORTS_PER_SOL);
    
    await provider.connection.confirmTransaction(airdropSig1);
    await provider.connection.confirmTransaction(airdropSig2);
    await provider.connection.confirmTransaction(airdropSig3);

    [campaignPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("campaign"), Buffer.from(campaignId)],
      program.programId
    );

    [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), campaignPda.toBuffer()],
      program.programId
    );

    [creatorPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("creator"), creatorKeypair.publicKey.toBuffer()],
      program.programId
    );
  });

  it("1. Successfully initializes the Madrid 2026 promotional campaign", async () => {
    await program.methods
      .initializeCampaign(campaignId, rewardPerSubmission, totalSlots)
      .accounts({
        campaign: campaignPda,
        vault: vaultPda,
        oracleAuthority: oracleAuthority.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Fund the campaign vault with SOL for payouts
    const fundTx = new anchor.web3.Transaction().add(
      SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: vaultPda,
        lamports: 3 * LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(fundTx);

    const campaignAccount = await program.account.campaign.fetch(campaignPda);
    expect(campaignAccount.campaignId).to.equal(campaignId);
    expect(campaignAccount.totalSlots).to.equal(totalSlots);
    expect(campaignAccount.claimedSlots).to.equal(0);
    expect(campaignAccount.rewardPerSubmission.toNumber()).to.equal(0.25 * LAMPORTS_PER_SOL);
    expect(campaignAccount.isActive).to.be.true;
  });

  it("2. Registers a new Spanish Web3 content creator profile", async () => {
    const handle = "@solana_madrid_dev";

    await program.methods
      .registerCreator(handle)
      .accounts({
        creator: creatorPda,
        wallet: creatorKeypair.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([creatorKeypair])
      .rpc();

    const creatorAccount = await program.account.creator.fetch(creatorPda);
    expect(creatorAccount.handle).to.equal(handle);
    expect(creatorAccount.wallet.toBase58()).to.equal(creatorKeypair.publicKey.toBase58());
    expect(creatorAccount.totalRewardsEarned.toNumber()).to.equal(0);
    expect(creatorAccount.verifiedSubmissions).to.equal(0);
  });

  it("3. Creator submits valid promotional thread for El Encuentro 2026", async () => {
    const contentUri = "https://x.com/solana_madrid_dev/status/1888999999999999999";
    const contentHash = crypto.createHash("sha256").update(contentUri).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    await program.methods
      .submitContent(contentUri, Array.from(contentHash))
      .accounts({
        campaign: campaignPda,
        creator: creatorPda,
        submission: submissionPda,
        creatorWallet: creatorKeypair.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([creatorKeypair])
      .rpc();

    const submission = await program.account.submission.fetch(submissionPda);
    expect(submission.contentUri).to.equal(contentUri);
    expect(JSON.stringify(submission.status)).to.equal(JSON.stringify({ pending: {} }));
  });

  it("4. Prevents unauthorized oracle from approving and draining funds", async () => {
    const contentUri = "https://x.com/solana_madrid_dev/status/1888999999999999999";
    const contentHash = crypto.createHash("sha256").update(contentUri).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    try {
      await program.methods
        .verifyAndReward(95)
        .accounts({
          campaign: campaignPda,
          vault: vaultPda,
          creator: creatorPda,
          submission: submissionPda,
          creatorWallet: creatorKeypair.publicKey,
          oracleAuthority: impostorKeypair.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([impostorKeypair])
        .rpc();
      expect.fail("Unauthorized oracle should have been rejected");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("UnauthorizedOracle");
    }
  });

  it("5. Oracle successfully verifies engagement and settles SOL reward to creator", async () => {
    const contentUri = "https://x.com/solana_madrid_dev/status/1888999999999999999";
    const contentHash = crypto.createHash("sha256").update(contentUri).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    const initialBalance = await provider.connection.getBalance(creatorKeypair.publicKey);

    await program.methods
      .verifyAndReward(92)
      .accounts({
        campaign: campaignPda,
        vault: vaultPda,
        creator: creatorPda,
        submission: submissionPda,
        creatorWallet: creatorKeypair.publicKey,
        oracleAuthority: oracleAuthority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([oracleAuthority])
      .rpc();

    const finalBalance = await provider.connection.getBalance(creatorKeypair.publicKey);
    expect(finalBalance - initialBalance).to.equal(rewardPerSubmission.toNumber());

    const submission = await program.account.submission.fetch(submissionPda);
    expect(JSON.stringify(submission.status)).to.equal(JSON.stringify({ approved: {} }));
    expect(submission.metricScore).to.equal(92);

    const creatorAccount = await program.account.creator.fetch(creatorPda);
    expect(creatorAccount.totalRewardsEarned.toNumber()).to.equal(rewardPerSubmission.toNumber());
    expect(creatorAccount.verifiedSubmissions).to.equal(1);
  });

  it("6. Prevents double-claiming the same approved submission", async () => {
    const contentUri = "https://x.com/solana_madrid_dev/status/1888999999999999999";
    const contentHash = crypto.createHash("sha256").update(contentUri).digest();

    const [submissionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), campaignPda.toBuffer(), contentHash],
      program.programId
    );

    try {
      await program.methods
        .verifyAndReward(90)
        .accounts({
          campaign: campaignPda,
          vault: vaultPda,
          creator: creatorPda,
          submission: submissionPda,
          creatorWallet: creatorKeypair.publicKey,
          oracleAuthority: oracleAuthority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([oracleAuthority])
        .rpc();
      expect.fail("Should not allow re-verification of already approved content");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("SubmissionAlreadyProcessed");
    }
  });
});