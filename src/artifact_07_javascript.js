#!/usr/bin/env node
import { Command } from 'commander';
import { EncuentroPromoAgent } from './agent.js';
import fs from 'fs';
import path from 'path';

const program = new Command();
const idl = JSON.parse(fs.readFileSync(path.resolve('./target/idl/el_encuentro_promo.json'), 'utf8'));

program
  .name('encuentro-cli')
  .description('Madrid 2026 Solana Content Promotion Automation Suite')
  .version('1.0.0');

program
  .command('attest')
  .requiredOption('-c, --campaign <name>', 'Campaign name', 'Madrid2026')
  .requiredOption('-u, --url <url>', 'Promotional content URL')
  .requiredOption('-t, --tag <tag>', 'Category tag (e.g. "x-thread", "video", "article")', 'x-thread')
  .option('-k, --keypair <path>', 'Path to Oracle keypair JSON', './oracle-keypair.json')
  .option('-e, --email <email>', 'Recipient notification email')
  .action(async (opts) => {
    try {
      console.log(`Initializing Oracle Agent for El Encuentro 2026 Madrid...`);
      const agent = new EncuentroPromoAgent(opts.keypair, idl);
      const res = await agent.processSubmission(opts.campaign, opts.url, opts.tag, opts.email);
      console.log(`✅ Success! Submission Verified:`, res);
    } catch (e) {
      console.error(`❌ Execution Failed:`, e);
      process.exit(1);
    }
  });

program.parse(process.argv);