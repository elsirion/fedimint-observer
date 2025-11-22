
import React, { Suspense } from 'react';
import { Link } from 'react-router-dom';
import type { FederationHealth, FederationRating } from '../types/api';
import { Badge } from './Badge';
import { Copyable } from './Copyable';
import { Rating } from './Rating';
const CombinedMiniChart = React.lazy(() => import('./MiniChart').then((m) => ({ default: m.CombinedMiniChart })));
import { asBitcoin } from '../utils/format';

interface ActivityData {
  num_transactions: number;
  amount_transferred: number;
}

interface FederationRowProps {
  id: string;
  name: string;
  rating: FederationRating;
  invite: string;
  totalAssets: number;
  health: FederationHealth;
  activityData: ActivityData[];
  maxTransaction?: number;  // global max for consistent chart scale
  maxVolume?: number;        // global max for consistent chart scale
}

export function FederationRow({
  id,
  name,
  rating,
  invite,
  totalAssets,
  health,
  activityData,
  maxTransaction,
  maxVolume,
}: FederationRowProps) {
  // Extract data for mini charts
  const transactionData = activityData.map(d => d.num_transactions);
  const volumeData = activityData.map(d => d.amount_transferred / 100000000000); // millisats to BTC
  
  // Generate date labels (most recent data last, so we count backwards)
  const dates = activityData.map((_, index) => {
    const daysAgo = activityData.length - 1 - index;
    if (daysAgo === 0) return 'Today';
    if (daysAgo === 1) return 'Yesterday';
    return `${daysAgo} days ago`;
  });

  return (
    <div className="bg-white dark:bg-gray-800 hover:bg-gray-50 dark:hover:bg-gray-700 px-3 sm:px-6 py-4 grid grid-cols-1 md:grid-cols-5 gap-3 md:gap-3 text-xs sm:text-sm">
      {/* Name */}
      <div className="font-medium text-gray-900 dark:text-white">
        <span className="text-[10px] md:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Name</span>
        <Link
          to={`/federations/${id}`}
          className="font-medium text-blue-600 dark:text-blue-500 hover:underline break-words"
        >
          {name}
        </Link>
      </div>

      {/* Recommendations */}
      <div>
        <span className="text-[10px] md:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">
          <a
            href="https://github.com/nostr-protocol/nips/pull/1110"
            className="underline hover:no-underline"
          >
            Recommendations
          </a>
        </span>
        <Rating count={rating.count} rating={rating.avg} />
      </div>

      {/* Invite Code / Status */}
      <div>
        <span className="text-[10px] md:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Invite Code</span>
        {health === 'online' ? (
          <Copyable text={invite} />
        ) : health === 'degraded' ? (
          <Badge level="warning">Degraded</Badge>
        ) : (
          <Badge level="error">Offline</Badge>
        )}
      </div>

      {/* Total Assets */}
      <div>
        <span className="text-[10px] md:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Total Assets</span>
        <span className="text-gray-900 dark:text-white">{asBitcoin(totalAssets, 6)}</span>
      </div>

      {/* Activity Charts (7d) */}
      <div>
        <span className="text-[10px] md:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Activity Charts (7d)</span>
        <Suspense fallback={<div className="w-full h-12 bg-transparent" /> }>
          <CombinedMiniChart
            transactionData={transactionData}
            volumeData={volumeData}
            dates={dates}
            formatTransaction={(val) => Math.round(val).toString()}
            formatVolume={(val) => `${val.toFixed(8)} BTC`}
            maxTransaction={maxTransaction}
            maxVolume={maxVolume}
          />
        </Suspense>
      </div>
    </div>
  );
}
