import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
  Ed25519Program,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { Program, AnchorProvider, Wallet, BN } from "@coral-xyz/anchor";
import * as crypto from "crypto";
import axios from "axios";
import dotenv from "dotenv";

dotenv.config();

// Configuration Interfaces
export interface MadridEncuentroConfig {
  rpcUrl: string;
  programId: string;
  oraclePrivateKey: string;
  organizerPrivateKey: string;
  resendApiKey?: string;
  slackWebhookUrl?: string;
}

export interface PromotionSubmission {
  creatorPubkey: string;
  creatorEmail: string;
  campaignId: number;
  contentUri: string; // e.g., https://x.com/solana_es/status/123456789
  engagementMetric: number;
}

export class MadridEncuentroClient {
  private connection: Connection;
  private oracleKeypair: Keypair;
  private organizerKeypair: Keypair;
  private programId: PublicKey;
  private resendApiKey?: string;
  private slackWebhookUrl?: string;

  constructor(config: MadridEncuentroConfig) {
    this.connection = new Connection(config.rpcUrl, "confirmed");
    this.programId = new PublicKey(config.programId);
    this.oracleKeypair = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(config.oraclePrivateKey))
    );
    this.organizerKeypair = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(config.organizerPrivateKey))
    );
    this.resendApiKey = config.resendApiKey;
    this.slackWebhookUrl = config.slackWebhookUrl;
  }

  /**
   * Derives PDA addresses for the campaign
   */
  public getCampaignPda(campaignId: number): [PublicKey, number] {
    const campaignIdBuffer = new BN(campaignId).toArrayLike(Buffer, "le", 8);
    return PublicKey.findProgramAddressSync(
      [Buffer.from("campaign"), campaignIdBuffer],
      this.programId
    );
  }

  public getVaultPda(campaignId: number): [PublicKey, number] {
    const campaignIdBuffer = new BN(campaignId).toArrayLike(Buffer, "le", 8);
    return PublicKey.findProgramAddressSync(
      [Buffer.from("campaign_vault"), campaignIdBuffer],
      this.programId
    );
  }

  public getClaimRecordPda(campaignId: number, contentHash: Buffer): [PublicKey, number] {
    const campaignIdBuffer = new BN(campaignId).toArrayLike(Buffer, "le", 8);
    return PublicKey.findProgramAddressSync(
      [Buffer.from("claim"), campaignIdBuffer, contentHash],
      this.programId
    );
  }

  /**
   * Off-chain oracle content verification engine
   */
  public verifyContentCriteria(submission: PromotionSubmission): boolean {
    const isMadridPromotion =
      submission.contentUri.includes("x.com") ||
      submission.contentUri.includes("twitter.com") ||
      submission.contentUri.includes("github.com");
    const hasSufficientEngagement = submission.engagementMetric >= 50;

    return isMadridPromotion && hasSufficientEngagement;
  }

  /**
   * Generates cryptographic attestation payload and signs with oracle
   */
  public generateAttestation(
    creatorPubkey: PublicKey,
    campaignId: number,
    contentHash: Buffer
  ): { payload: Buffer; signature: Buffer } {
    const campaignIdBuffer = new BN(campaignId).toArrayLike(Buffer, "le", 8);
    const payload = Buffer.concat([
      creatorPubkey.toBuffer(),
      campaignIdBuffer,
      contentHash,
    ]);

    const signature = Buffer.from(
      crypto.sign(null, payload, {
        key: this.oracleKeypair.secretKey,
        format: "der",
        type: "ed25519",
      })
    );

    // Ensure raw 64-byte Ed25519 signature
    const rawSignature = this.oracleKeypair.secretKey.slice(0, 64);
    const nacl = require("tweetnacl");
    const ed25519Sig = Buffer.from(
      nacl.sign.detached(payload, this.oracleKeypair.secretKey)
    );

    return { payload, signature: ed25519Sig };
  }

  /**
   * Enterprise Notification: Sends localized VIP Welcome Kit via Resend API
   */
  public async dispatchAttendeeWelcomeKit(
    email: string,
    creatorPubkey: string,
    txSignature: string
  ): Promise<void> {
    if (!this.resendApiKey) {
      console.log(`[Resend Mock] Skipping email delivery for ${email}`);
      return;
    }

    const payload = {
      from: "El Encuentro Madrid 2026 <tickets@elencuentro2026.madrid>",
      to: [email],
      subject: "¡Confirmación de Pase VIP! - El Encuentro 2026 en Madrid",
      html: `
        <div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 24px; border: 1px solid #e2e8f0; border-radius: 8px;">
          <h1 style="color: #9945FF; margin-bottom: 8px;">¡Nos vemos en Madrid! 🇪🇸</h1>
          <h2 style="color: #14F195; margin-top: 0;">El Encuentro 2026 - Solana Summit</h2>
          <p>Tu contribución de contenido promocional ha sido verificada en la blockchain de Solana.</p>
          <div style="background-color: #f7fafc; padding: 16px; border-radius: 6px; margin: 20px 0;">
            <p><strong>Wallet Beneficiaria:</strong> <code>${creatorPubkey}</code></p>
            <p><strong>ID de Recompensa (Tx):</strong> <code>${txSignature}</code></p>
            <p><strong>Ubicación:</strong> Palacio Municipal IFEMA, Madrid, España</p>
            <p><strong>Fecha Estimada:</strong> Primavera 2026</p>
          </div>
          <p>Has asegurado acceso preferencial a los tracks técnicos, networking exclusivo y sesiones de hackathon.</p>
          <hr style="border: 0; border-top: 1px solid #edf2f7; margin: 24px 0;" />
          <p style="font-size: 12px; color: #718096;">Comunidad Oficial Solana Hispanoamérica & España</p>
        </div>
      `,
    };

    try {
      await axios.post("https://api.resend.com/emails", payload, {
        headers: {
          Authorization: `Bearer ${this.resendApiKey}`,
          "Content-Type": "application/json",
        },
      });
      console.log(`[Resend] Successfully dispatched VIP pass to ${email}`);
    } catch (err: any) {
      console.error("[Resend Error]", err.response?.data || err.message);
    }
  }

  /**
   * Enterprise Notification: Sends real-time alert to Slack / Discord
   */
  public async postMarketingSlackAlert(
    creatorPubkey: string,
    contentUri: string,
    rewardAmountLamports: number,
    txSignature: string
  ): Promise<void> {
    if (!this.slackWebhookUrl) return;

    const solAmount = rewardAmountLamports / LAMPORTS_PER_SOL;
    const body = {
      text: `🚀 *Nuevo Creador Verificado: El Encuentro Madrid 2026*\n• *Creador:* \`${creatorPubkey}\`\n• *Contenido:* ${contentUri}\n• *Recompensa:* ${solAmount} SOL + VIP Pass NFT\n• *Explorador:* <https://explorer.solana.com/tx/${txSignature}?cluster=custom>`,
    };

    try {
      await axios.post(this.slackWebhookUrl, body);
      console.log("[Slack] Broadcast sent to marketing channel.");
    } catch (err: any) {
      console.error("[Slack Error]", err.message);
    }
  }

  /**
   * End-to-end automated orchestration handler
   */
  public async processClaim(
    submission: PromotionSubmission,
    creatorKeypair: Keypair
  ): Promise<string> {
    console.log(`[Verification] Checking content: ${submission.contentUri}...`);

    if (!this.verifyContentCriteria(submission)) {
      throw new Error("Content does not meet the Madrid 2026 verification threshold.");
    }

    // 1. Calculate content hash
    const contentHash = crypto
      .createHash("sha256")
      .update(submission.contentUri)
      .digest();

    const creatorPubkey = creatorKeypair.publicKey;
    const [campaignPda] = this.getCampaignPda(submission.campaignId);
    const [vaultPda] = this.getVaultPda(submission.campaignId);
    const [claimRecordPda] = this.getClaimRecordPda(submission.campaignId, contentHash);

    // 2. Generate Oracle Attestation
    const { payload, signature } = this.generateAttestation(
      creatorPubkey,
      submission.campaignId,
      contentHash
    );

    // 3. Build Ed25519 Pre-instruction
    const ed25519Ix = Ed25519Program.createInstructionWithPublicKey({
      publicKey: this.oracleKeypair.publicKey.toBytes(),
      message: payload,
      signature: signature,
    });

    // 4. Construct Anchor Claim Instruction
    // Anchor discriminator for 'claim_reward' = 4e3d93549646b9ec
    const discriminator = Buffer.from("4e3d93549646b9ec", "hex");
    const uriBuffer = Buffer.from(submission.contentUri, "utf8");
    const uriLength = Buffer.alloc(4);
    uriLength.writeUInt32LE(uriBuffer.length, 0);

    const instructionData = Buffer.concat([
      discriminator,
      contentHash,
      uriLength,
      uriBuffer,
      signature,
    ]);

    const claimIx = {
      programId: this.programId,
      keys: [
        { pubkey: campaignPda, isSigner: false, isWritable: true },
        { pubkey: vaultPda, isSigner: false, isWritable: true },
        { pubkey: claimRecordPda, isSigner: false, isWritable: true },
        { pubkey: creatorPubkey, isSigner: true, isWritable: true },
        { pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: instructionData,
    };

    const transaction = new Transaction().add(ed25519Ix).add(claimIx);
    transaction.feePayer = creatorPubkey;
    const { blockhash } = await this.connection.getLatestBlockhash("confirmed");
    transaction.recentBlockhash = blockhash;

    // Creator signs the claim transaction
    transaction.sign(creatorKeypair);

    console.log("[Solana] Submitting attestation and reward claim...");
    const txId = await this.connection.sendRawTransaction(transaction.serialize());
    await this.connection.confirmTransaction(txId, "confirmed");

    console.log(`[Success] Claim confirmed. TxId: ${txId}`);

    // Trigger enterprise notifications
    await this.dispatchAttendeeWelcomeKit(
      submission.creatorEmail,
      creatorPubkey.toBase58(),
      txId
    );

    await this.postMarketingSlackAlert(
      creatorPubkey.toBase58(),
      submission.contentUri,
      0.25 * LAMPORTS_PER_SOL,
      txId
    );

    return txId;
  }
}