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
    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
      <th
        scope="row"
        className="px-6 py-4 font-medium text-gray-900 whitespace-nowrap dark:text-white"
      >
        <Link
          to={`/federations/${id}`}
          className="font-medium text-blue-600 dark:text-blue-500 hover:underline"
        >
          {name}
        </Link>
      </th>
      <td>
        <Rating count={rating.count} rating={rating.avg} />
      </td>
      <td className="px-6 py-4">
        {health === 'online' ? (
          <Copyable text={invite} />
        ) : health === 'degraded' ? (
          <Badge level="warning">Degraded</Badge>
        ) : (
          <Badge level="error">Offline</Badge>
        )}
      </td>
      <td className="px-6 py-4">{asBitcoin(totalAssets, 6)}</td>
      <td className="px-6 py-4">
        <ul>
          <li>#tx: {avgTxs.toFixed(1)}</li>
          <li>volume: {asBitcoin(avgVolume, 6)}</li>
        </ul>
      </td>
    </tr>
  );
}
