import axios from "axios";
import { Resend } from "resend";
import { CONFIG } from "./config";

const resend = new Resend(CONFIG.RESEND_API_KEY);

export async function sendDiscordAlert(data: {
  creator: string;
  tier: string;
  rewardUSDC: number;
  txHash: string;
  platform: string;
}) {
  if (!CONFIG.DISCORD_WEBHOOK_URL) return;

  const embed = {
    title: "🎉 ¡Nueva Recompensa Entregada! - El Encuentro Madrid 2026",
    description: `Un creador de la comunidad ha sido verificado y recompensado por amplificar la voz de Solana en español.`,
    color: 0x9945ff, // Solana Purple
    fields: [
      { name: "Creador", value: `\`${data.creator}\``, inline: true },
      { name: "Plataforma", value: data.platform, inline: true },
      { name: "Nivel Otorgado", value: `🏆 ${data.tier}`, inline: true },
      { name: "Recompensa", value: `${data.rewardUSDC} USDC`, inline: true },
      { name: "Solana Explorer", value: `[Ver Transacción](https://explorer.solana.com/tx/${data.txHash}?cluster=devnet)`, inline: false }
    ],
    footer: { text: "El Encuentro 2026 Madrid • Protocolo Oficial de Promoción" },
    timestamp: new Date().toISOString()
  };

  try {
    await axios.post(CONFIG.DISCORD_WEBHOOK_URL, { embeds: [embed] });
  } catch (err: any) {
    console.error("Failed to dispatch Discord webhook:", err.message);
  }
}

export async function sendCreatorEmailReceipt(recipientEmail: string, data: {
  creator: string;
  tier: string;
  rewardUSDC: number;
  txHash: string;
}) {
  if (!CONFIG.RESEND_API_KEY) return;

  try {
    await resend.emails.send({
      from: "El Encuentro 2026 <tickets@elencuentro2026.madrid>",
      to: recipientEmail,
      subject: "🎟️ Tu Pase & Recompensa Verificada: El Encuentro 2026 en Madrid",
      html: `
        <div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 24px; border: 1px solid #eaeaea; border-radius: 8px;">
          <h2 style="color: #14f195; text-align: center;">¡Felicidades por Difundir El Encuentro Madrid 2026!</h2>
          <p>Hola <b>${data.creator}</b>,</p>
          <p>Tu contenido promocional para el congreso <b>El Encuentro 2026 en Madrid</b> ha sido validado exitosamente en la blockchain de Solana.</p>
          <div style="background-color: #f7f9fa; padding: 16px; border-radius: 6px; margin: 16px 0;">
            <p><strong>Nivel Alcanzado:</strong> ${data.tier}</p>
            <p><strong>Monto Acreditado:</strong> ${data.rewardUSDC} USDC</p>
            <p><strong>Identificador de Transacción:</strong> <code style="word-break: break-all;">${data.txHash}</code></p>
          </div>
          <p>Tu credencial digital VIP ha sido reservada para el ingreso al recinto ferial de Madrid.</p>
          <hr style="border: none; border-top: 1px solid #eaeaea; margin: 20px 0;" />
          <p style="font-size: 12px; color: #888; text-align: center;">El Encuentro 2026 • Impulsando el futuro de Solana en Iberoamérica</p>
        </div>
      `
    });
  } catch (err: any) {
    console.error("Failed to send transactional email via Resend:", err.message);
  }
}