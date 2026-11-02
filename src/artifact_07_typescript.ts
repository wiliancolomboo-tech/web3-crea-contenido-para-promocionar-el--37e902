import { Command } from 'commander';
import { Connection, Keypair, PublicKey, clusterApiUrl } from '@solana/web3.js';
import * as anchor from '@coral-xyz/anchor';
import * as crypto from 'crypto';
import * as fs from 'fs';
import * as dotenv from 'dotenv';
import { MadridNotificationService } from './service/notifier';

dotenv.config();

const programId = new PublicKey("MDRD26x9eQJqF8sWbK3qY2PjT4rZ8uV1mN5cE7wA3vL");
const program = new Command();
const notifier = new MadridNotificationService();

function loadKeypair(path: string): Keypair {
  const secretKey = JSON.parse(fs.readFileSync(path, 'utf8'));
  return Keypair.fromSecretKey(new Uint8Array(secretKey));
}

program
  .name('madrid26-cli')
  .description('Madrid 2026 Autonomous Viral Promotion CLI Engine')
  .version('1.0.0');

program
  .command('simulate-purchase')
  .description('Simulates an on-chain ticket acquisition with advocate reward distribution')
  .requiredOption('-k, --keypair <path>', 'Signer keypair path')
  .requiredOption('-r, --referral <string>', 'Referral code utilized (e.g. MADRID_VIP)')
  .requiredOption('-e, --email <email>', 'Attendee notification email')
  .action(async (options) => {
    console.log('🚀 Executing purchase on Solana for El Encuentro Madrid 2026...');
    const buyerKeypair = loadKeypair(options.keypair);
    const emailHash = crypto.createHash('sha256').update(options.email).digest();

    console.log(`Attendee Wallet: ${buyerKeypair.publicKey.toBase58()}`);
    console.log(`Referral Code: ${options.referral}`);
    console.log(`Hashed Identity: ${emailHash.toString('hex')}`);

    // Off-chain notification triggering demo (simulating parsed Anchor event)
    const simulatedPayload = {
      recipientEmail: options.email,
      buyerPubkey: buyerKeypair.publicKey.toBase58(),
      advocatePubkey: 'AdvoCatE1111111111111111111111111111111111',
      referralCode: options.referral,
      ticketNumber: 42,
      ticketPriceSol: 0.5,
      advocateEarnedSol: 0.05,
      txSignature: '5KnB...simulatedTxSignature...9xYZ',
    };

    console.log('📡 Triggering Enterprise Communication Matrix...');
    await notifier.sendAttendeeTicketEmail(simulatedPayload);
    await notifier.broadcastSocialAlert(simulatedPayload);

    console.log('✅ Flow successfully completed.');
  });

program.parse(process.argv);