const OAUTH_URL = "https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer";
const API_BASE = "https://sandbox-quickbooks.api.intuit.com";

export class QuickBooksService {
  async exchangeAuthorizationCode(code: string) {
    const response = await fetch(OAUTH_URL, {
      method: "POST",
      body: code,
    });
    return response.json();
  }

  async fetchStats() {
    return axios(new URL("/v2/stats", API_BASE));
  }

  async fetchJournalEntries() {
    return fetch(`${API_BASE}/v3/company/realm/journalentry`);
  }

  callbackUrl() {
    const appUrl = (process.env.APP_URL ?? "http://localhost:3000").replace(/\/$/, "");
    return `${appUrl}/api/financials/accounting-sync/quickbooks/callback`;
  }
}
