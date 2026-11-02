import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Madrid26Core } from "../target/types/madrid26_core";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import * as crypto from "crypto";

describe("madrid26_core unit & integration test suite", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Madrid26Core as Program<Madrid26Core>;

  const organizer = Keypair.generate();
  const treasury = Keypair.generate();
  const advocate = Keypair.generate();
  const buyer = Keypair.generate();

  const referralCode = "MADRID2026VIP";
  const ticketPrice = new anchor.BN(1 * LAMPORTS_PER_SOL); // 1 SOL
  const advocateFeeBps = 1500; // 15%

  let campaignPda: PublicKey;
  let campaignBump: number;
  let advocatePda: PublicKey;
  let advocateBump: number;

  before(async () => {
    // Airdrop balances to test accounts
    for (const kp of [organizer, treasury, advocate, buyer]) {
      const sig = await provider.connection.requestAirdrop(kp.publicKey, 10 * LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);
    }

    [campaignPda, campaignBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("madrid2026_campaign"), organizer.publicKey.toBuffer()],
      program.programId
    );

    [advocatePda, advocateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("madrid2026_advocate"), campaignPda.toBuffer(), Buffer.from(referralCode)],
      program.programId
    );
  });

  it("1. Successfully initializes the El Encuentro Madrid 2026 campaign", async () => {
    await program.methods
      .initializeCampaign(ticketPrice, advocateFeeBps)
      .accounts({
        campaign: campaignPda,
        treasury: treasury.publicKey,
        organizer: organizer.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([organizer])
      .rpc();

    const campaignAccount = await program.account.campaignState.fetch(campaignPda);
    expect(campaignAccount.organizer.toBase58()).to.equal(organizer.publicKey.toBase58());
    expect(campaignAccount.treasury.toBase58()).to.equal(treasury.publicKey.toBase58());
    expect(campaignAccount.ticketPrice.eq(ticketPrice)).to.be.true;
    expect(campaignAccount.advocateFeeBps).to.equal(advocateFeeBps);
    expect(campaignAccount.totalTicketsSold.toNumber()).to.equal(0);
  });

  it("2. Rejects invalid campaign initialization if fee exceeds 50%", async () => {
    const bogusOrganizer = Keypair.generate();
    const sig = await provider.connection.requestAirdrop(bogusOrganizer.publicKey, 2 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig);

    const [bogusCampaignPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("madrid2026_campaign"), bogusOrganizer.publicKey.toBuffer()],
      program.programId
    );

    try {
      await program.methods
        .initializeCampaign(ticketPrice, 5001) // 50.01%
        .accounts({
          campaign: bogusCampaignPda,
          treasury: treasury.publicKey,
          organizer: bogusOrganizer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([bogusOrganizer])
        .rpc();
      expect.fail("Should have thrown FeeTooHigh");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("FeeTooHigh");
    }
  });

  it("3. Registers an advocate and generates referral PDA", async () => {
    await program.methods
      .registerAdvocate(referralCode)
      .accounts({
        advocateRecord: advocatePda,
        campaign: campaignPda,
        advocate: advocate.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([advocate])
      .rpc();

    const record = await program.account.advocateRecord.fetch(advocatePda);
    expect(record.advocate.toBase58()).to.equal(advocate.publicKey.toBase58());
    expect(record.referralCode).to.equal(referralCode);
    expect(record.ticketsReferred.toNumber()).to.equal(0);
    expect(record.totalEarned.toNumber()).to.equal(0);
  });

  it("4. Executes a ticket purchase with trustless on-chain fee splitting", async () => {
    const advocateBalBefore = await provider.connection.getBalance(advocate.publicKey);
    const treasuryBalBefore = await provider.connection.getBalance(treasury.publicKey);

    const emailHash = Array.from(crypto.createHash("sha256").update("attendee@madrid2026.es").digest());

    await program.methods
      .purchasePromotedTicket(emailHash)
      .accounts({
        campaign: campaignPda,
        advocateRecord: advocatePda,
        advocateWallet: advocate.publicKey,
        treasury: treasury.publicKey,
        buyer: buyer.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const advocateBalAfter = await provider.connection.getBalance(advocate.publicKey);
    const treasuryBalAfter = await provider.connection.getBalance(treasury.publicKey);

    // Expected splits:
    // 1 SOL * 15% = 0.15 SOL to advocate
    // 1 SOL - 0.15 SOL = 0.85 SOL to treasury
    const expectedAdvocateDelta = (1 * LAMPORTS_PER_SOL * 15) / 100;
    const expectedTreasuryDelta = (1 * LAMPORTS_PER_SOL * 85) / 100;

    expect(advocateBalAfter - advocateBalBefore).to.equal(expectedAdvocateDelta);
    expect(treasuryBalAfter - treasuryBalBefore).to.equal(expectedTreasuryDelta);

    // Verify State Updates
    const campaignAccount = await program.account.campaignState.fetch(campaignPda);
    const advocateAccount = await program.account.advocateRecord.fetch(advocatePda);

    expect(campaignAccount.totalTicketsSold.toNumber()).to.equal(1);
    expect(advocateAccount.ticketsReferred.toNumber()).to.equal(1);
    expect(advocateAccount.totalEarned.toNumber()).to.equal(expectedAdvocateDelta);
  });

  it("5. Rejects purchase if attacker attempts to redirect advocate commission to a bogus wallet", async () => {
    const bogusWallet = Keypair.generate();
    const emailHash = Array.from(crypto.createHash("sha256").update("hacker@exploit.com").digest());

    try {
      await program.methods
        .purchasePromotedTicket(emailHash)
        .accounts({
          campaign: campaignPda,
          advocateRecord: advocatePda,
          advocateWallet: bogusWallet.publicKey, // Invalid!
          treasury: treasury.publicKey,
          buyer: buyer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([buyer])
        .rpc();
      expect.fail("Should have failed constraint check on advocateWallet");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("InvalidAdvocateWallet");
    }
  });
});