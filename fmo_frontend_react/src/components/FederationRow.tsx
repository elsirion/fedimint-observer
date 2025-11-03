import { Link } from 'react-router-dom';
import type { FederationHealth, FederationRating } from '../types/api';
import { Badge } from './Badge';
import { Copyable } from './Copyable';
import { Rating } from './Rating';
import { asBitcoin } from '../utils/format';

interface FederationRowProps {
  id: string;
  name: string;
  rating: FederationRating;
  invite: string;
  totalAssets: number;
  avgTxs: number;
  avgVolume: number;
  health: FederationHealth;
}

export function FederationRow({
  id,
  name,
  rating,
  invite,
  totalAssets,
  avgTxs,
  avgVolume,
  health,
}: FederationRowProps) {
  return (
    <div className="bg-blue-100 dark:bg-gray-800 px-3 sm:px-6 py-4 grid grid-cols-1 lg:grid-cols-5 gap-3 lg:gap-4 text-xs sm:text-sm">
      {/* Name */}
      <div className="font-medium text-gray-900 dark:text-white">
        <span className="text-[10px] lg:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Name</span>
        <Link
          to={`/federations/${id}`}
          className="font-medium text-blue-600 dark:text-blue-500 hover:underline break-words"
        >
          {name}
        </Link>
      </div>

      {/* Recommendations */}
      <div>
        <span className="text-[10px] lg:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">
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
        <span className="text-[10px] lg:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Invite Code</span>
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
        <span className="text-[10px] lg:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Total Assets</span>
        <span className="text-gray-900 dark:text-white">{asBitcoin(totalAssets, 6)}</span>
      </div>

      {/* Average Activity */}
      <div>
        <span className="text-[10px] lg:hidden uppercase text-gray-600 dark:text-gray-400 block mb-1">Average Activity (7d)</span>
        <div className="text-gray-900 dark:text-white">
          <div>#tx: {avgTxs.toFixed(1)}</div>
          <div>volume: {asBitcoin(avgVolume, 6)}</div>
        </div>
      </div>
    </div>
  );
}
