import type { FedimintTotals, FederationSummary } from '../types/api';

const BASE_URL = import.meta.env.VITE_API_SERVER || 'http://127.0.0.1:3000';

export const api = {
  async getTotals(): Promise<FedimintTotals> {
    const response = await fetch(`${BASE_URL}/totals`);
    if (!response.ok) {
      throw new Error('Failed to fetch totals');
    }
    return response.json();
  },

  async getFederations(): Promise<FederationSummary[]> {
    const response = await fetch(`${BASE_URL}/federations`);
    if (!response.ok) {
      throw new Error('Failed to fetch federations');
    }
    return response.json();
  },

  async getFederation(id: string): Promise<FederationSummary> {
    const response = await fetch(`${BASE_URL}/federations/${id}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch federation ${id}`);
    }
    return response.json();
  },

  async getNostrFederations(): Promise<FederationSummary[]> {
    const response = await fetch(`${BASE_URL}/nostr/federations`);
    if (!response.ok) {
      throw new Error('Failed to fetch nostr federations');
    }
    return response.json();
  },
};
