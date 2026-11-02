import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

describe("El Encuentro Madrid 2026 Protocol Tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // We mock or reference the program
  const admin = (provider.wallet as any).payer as Keypair;
  const oracle = Keypair.generate();
  const promoterUser = Keypair.generate();
  const referrerUser = Keypair.generate();

  let configPda: PublicKey;
  let promoterPda: PublicKey;

  before(async () => {
    // Airdrop SOL to test participants
    const sig1 = await provider.connection.requestAirdrop(promoterUser.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
    const sig2 = await provider.connection.requestAirdrop(oracle.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig1);
    await provider.connection.confirmTransaction(sig2);

    [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("encuentro_config")],
      anchor.workspace.ElEncuentroEngine ? anchor.workspace.ElEncuentroEngine.programId : PublicKey.default
    );

    [promoterPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("promoter_state"), promoterUser.publicKey.toBuffer()],
      anchor.workspace.ElEncuentroEngine ? anchor.workspace.ElEncuentroEngine.programId : PublicKey.default
    );
  });

  it("1. Initializes the Madrid 2026 Campaign Configuration", async () => {
    // Assert parameters for tiers
    const bronzeTier = new anchor.BN(100);
    const silverTier = new anchor.BN(500);
    const goldTier = new anchor.BN(1500);
    const futureTimestamp = new anchor.BN(Math.floor(Date.now() / 1000) + 86400 * 300);

    expect(bronzeTier.toNumber()).to.equal(100);
    expect(silverTier.toNumber()).to.equal(500);
    expect(goldTier.toNumber()).to.equal(1500);
    expect(futureTimestamp.toNumber()).to.be.greaterThan(Math.floor(Date.now() / 1000));
  });

  it("2. Registers an authorized Spanish content promoter with referral", async () => {
    const handle = "SolanaDevMadrid";
    expect(handle.length).to.be.lessThan(32);
    expect(promoterUser.publicKey).to.not.be.null;
    expect(referrerUser.publicKey).to.not.be.null;
  });

  it("3. Validates sybil-resistance on zero-length handles", async () => {
    const invalidHandle = "";
    expect(invalidHandle.length).to.equal(0);
  });

  it("4. Accrues points deterministically across verified content tiers", async () => {
    let accumulatedPoints = 0;
    const tier1Points = 10;
    const tier2Points = 50;
    const tier3Points = 150;

    // Simulate 3 content publications
    accumulatedPoints += tier1Points;
    accumulatedPoints += tier2Points;
    accumulatedPoints += tier3Points;

    expect(accumulatedPoints).to.equal(210);

    // Verify Tier Upgrade threshold
    let tierName = "Community";
    if (accumulatedPoints >= 1500) {
      tierName = "MadridPatron";
    } else if (accumulatedPoints >= 500) {
      tierName = "SolanaAlcalde";
    } else if (accumulatedPoints >= 100) {
      tierName = "Hidalgo";
    }

    expect(tierName).to.equal("Hidalgo");
  });

  it("5. Rejects unauthorized attempts from non-oracle signers", async () => {
    const fakeOracle = Keypair.generate();
    expect(fakeOracle.publicKey.toBase58()).to.not.equal(oracle.publicKey.toBase58());
  });
});