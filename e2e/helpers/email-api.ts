const MAILPIT_URL = process.env.MAILPIT_URL ?? "http://127.0.0.1:8025";

interface MailpitAddress {
  Name: string;
  Address: string;
}

interface MailpitMessageSummary {
  ID: string;
  From: MailpitAddress;
  To: MailpitAddress[];
  Subject: string;
  Created: string;
}

export interface CapturedEmail {
  id: string;
  to: string;
  from: string;
  subject: string;
}

function toCaptured(m: MailpitMessageSummary): CapturedEmail {
  return {
    id: m.ID,
    to: m.To?.[0]?.Address ?? "",
    from: m.From?.Address ?? "",
    subject: m.Subject,
  };
}

export async function getCapturedEmails(): Promise<CapturedEmail[]> {
  const resp = await fetch(`${MAILPIT_URL}/api/v1/messages`);
  if (!resp.ok) {
    throw new Error(`Mailpit list failed: ${resp.status} ${await resp.text()}`);
  }
  const data = await resp.json() as { messages: MailpitMessageSummary[] };
  return (data.messages ?? []).map(toCaptured);
}

export async function getCapturedEmailsForRecipient(
  email: string,
): Promise<CapturedEmail[]> {
  const resp = await fetch(
    `${MAILPIT_URL}/api/v1/search?query=${encodeURIComponent(`to:${email}`)}`,
  );
  if (!resp.ok) {
    throw new Error(`Mailpit search failed: ${resp.status} ${await resp.text()}`);
  }
  const data = await resp.json() as { messages: MailpitMessageSummary[] };
  return (data.messages ?? []).map(toCaptured);
}

export async function clearCapturedEmails(): Promise<void> {
  const resp = await fetch(`${MAILPIT_URL}/api/v1/messages`, {
    method: "DELETE",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ IDs: [] }),
  });
  if (!resp.ok) {
    throw new Error(`Mailpit delete failed: ${resp.status} ${await resp.text()}`);
  }
}

/**
 * Deletes only the messages addressed to a specific recipient. Safe for
 * parallel test workers — each worker clears its own user's mailbox
 * without touching messages destined for sibling workers.
 */
export async function clearCapturedEmailsForRecipient(
  email: string,
): Promise<void> {
  const messages = await getCapturedEmailsForRecipient(email);
  if (messages.length === 0) return;
  const resp = await fetch(`${MAILPIT_URL}/api/v1/messages`, {
    method: "DELETE",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ IDs: messages.map((m) => m.id) }),
  });
  if (!resp.ok) {
    throw new Error(`Mailpit delete failed: ${resp.status} ${await resp.text()}`);
  }
}
