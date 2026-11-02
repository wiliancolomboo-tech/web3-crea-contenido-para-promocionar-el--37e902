import { Resend } from 'resend';
import axios from 'axios';

export interface TicketNotificationPayload {
  recipientEmail: string;
  buyerPubkey: string;
  advocatePubkey: string;
  referralCode: string;
  ticketNumber: number;
  ticketPriceSol: number;
  advocateEarnedSol: number;
  txSignature: string;
}

export class MadridNotificationService {
  private resend: Resend | null = null;
  private discordWebhookUrl: string | null = null;

  constructor() {
    if (process.env.RESEND_API_KEY) {
      this.resend = new Resend(process.env.RESEND_API_KEY);
    }
    this.discordWebhookUrl = process.env.DISCORD_WEBHOOK_URL || null;
  }

  /**
   * Dispatches branded bilingual Spanish/English Madrid 2026 confirmation email
   */
  async sendAttendeeTicketEmail(payload: TicketNotificationPayload): Promise<boolean> {
    if (!this.resend) {
      console.warn('[Notifier] Resend API key missing. Skipping email dispatch.');
      return false;
    }

    try {
      const emailHtml = `
        <div style="font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif; background-color: #0b0e14; color: #ffffff; padding: 40px; border-radius: 12px; max-width: 600px; margin: auto;">
          <div style="text-align: center; margin-bottom: 30px;">
            <h1 style="color: #9945FF; font-size: 28px; margin-bottom: 8px;">¡Nos Vemos en Madrid! 🇪🇸</h1>
            <p style="color: #14F195; font-size: 16px; font-weight: bold; letter-spacing: 1px;">EL ENCUENTRO 2026 | SOLANA SUMMIT</p>
          </div>
          
          <div style="background-color: #181d28; border: 1px solid #2d3748; border-radius: 8px; padding: 24px; margin-bottom: 24px;">
            <h3 style="margin-top: 0; color: #e2e8f0;">Pase Oficial de Asistencia # ${payload.ticketNumber}</h3>
            <p style="font-size: 14px; color: #a0aec0;"><strong>Comprador:</strong> <span style="font-family: monospace; color: #ffffff;">${payload.buyerPubkey}</span></p>
            <p style="font-size: 14px; color: #a0aec0;"><strong>Código Promotor:</strong> <span style="color: #14F195;">${payload.referralCode}</span></p>
            <p style="font-size: 14px; color: #a0aec0;"><strong>Tx Hash:</strong> <a href="https://explorer.solana.com/tx/${payload.txSignature}?cluster=devnet" style="color: #38bdf8;">Ver en Solana Explorer</a></p>
          </div>

          <div style="text-align: center; padding: 20px; background: linear-gradient(90deg, #9945FF22, #14F19522); border-radius: 8px;">
            <p style="margin: 0; font-size: 14px; color: #cbd5e1;">Presenta este comprobante o tu wallet conectada en el check-in oficial en Madrid.</p>
          </div>

          <p style="font-size: 12px; color: #64748b; text-align: center; margin-top: 30px;">
            Impulsado por el ecosistema de Solana España • El Encuentro 2026 Madrid
          </p>
        </div>
      `;

      await this.resend.emails.send({
        from: process.env.EMAIL_FROM || 'El Encuentro 2026 <entradas@solana-madrid.es>',
        to: payload.recipientEmail,
        subject: `🎟️ Tu Entrada para El Encuentro Madrid 2026 [Ticket #${payload.ticketNumber}]`,
        html: emailHtml,
      });

      console.log(`[Notifier] Confirmation email sent successfully to ${payload.recipientEmail}`);
      return true;
    } catch (error) {
      console.error('[Notifier] Error sending Resend email:', error);
      return false;
    }
  }

  /**
   * Broadcasts referral alerts to Discord/Slack for ambassador leaderboard tracking
   */
  async broadcastSocialAlert(payload: TicketNotificationPayload): Promise<boolean> {
    if (!this.discordWebhookUrl) {
      console.warn('[Notifier] Discord Webhook URL not configured.');
      return false;
    }

    try {
      await axios.post(this.discordWebhookUrl, {
        embeds: [
          {
            title: '🔥 ¡Nueva Entrada Vendida para El Encuentro Madrid 2026!',
            color: 0x14f195,
            fields: [
              { name: '🎟️ Ticket #', value: `${payload.ticketNumber}`, inline: true },
              { name: '🤝 Promotor', value: `${payload.referralCode}`, inline: true },
              { name: '💰 Comisión Generada', value: `${payload.advocateEarnedSol} SOL`, inline: true },
              { name: '🔑 Comprador', value: `\`${payload.buyerPubkey.slice(0, 4)}...${payload.buyerPubkey.slice(-4)}\``, inline: false },
            ],
            footer: {
              text: 'Solana Madrid 2026 • EMAVE Engine',
            },
            timestamp: new Date().toISOString(),
          },
        ],
      });
      console.log(`[Notifier] Discord alert dispatched for ticket #${payload.ticketNumber}`);
      return true;
    } catch (error) {
      console.error('[Notifier] Failed to post to Discord webhook:', error);
      return false;
    }
  }
}