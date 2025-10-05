export interface FedimintTotals {
  federations: number;
  tx_volume: number;
  tx_count: number;
}

export interface FederationSummary {
  id: string;
  name: string | null;
  last_7d_activity: FederationActivity[];
  deposits: number;
  invite: string;
  nostr_votes: FederationRating;
  health: FederationHealth;
}

export interface FederationRating {
  count: number;
  avg: number | null;
}

export interface FederationActivity {
  num_transactions: number;
  amount_transferred: number;
}

export interface FederationUtxo {
  address: string;
  out_point: string;
  amount: number;
}

export interface GuardianHealth {
  avg_uptime: number;
  avg_latency: number;
  latest: GuardianHealthLatest | null;
}

export interface GuardianHealthLatest {
  block_height: number;
  block_outdated: boolean;
  session_count: number;
  session_outdated: boolean;
}

export type FederationHealth = 'online' | 'degraded' | 'offline';

export interface NavItem {
  name: string;
  href: string;
  active: boolean;
}
