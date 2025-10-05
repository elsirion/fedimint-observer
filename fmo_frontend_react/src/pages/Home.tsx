import { useEffect, useState } from 'react';
import { api } from '../services/api';
import type { FederationSummary } from '../types/api';
import { Totals } from '../components/Totals';
import { FederationRow } from '../components/FederationRow';
import { ratingIndex } from '../utils/format';

interface FederationWithStats extends FederationSummary {
  avgTxs: number;
  avgVolume: number;
}

export function Home() {
  const [federations, setFederations] = useState<FederationWithStats[]>([]);
  const [loading, setLoading] = useState(true);
  const [collapseOffline, setCollapseOffline] = useState(true);

  useEffect(() => {
    api
      .getFederations()
      .then((data) => {
        const federationsWithStats = data
          .map((fed) => {
            const avgTxs =
              fed.last_7d_activity.reduce((sum, act) => sum + act.num_transactions, 0) /
              (fed.last_7d_activity.length || 1);
            const avgVolume =
              fed.last_7d_activity.reduce((sum, act) => sum + act.amount_transferred, 0) /
              (fed.last_7d_activity.length || 1);
            return {
              ...fed,
              avgTxs,
              avgVolume,
            };
          })
          .sort((a, b) => {
            const aIndex = ratingIndex(a.nostr_votes.count, a.nostr_votes.avg);
            const bIndex = ratingIndex(b.nostr_votes.count, b.nostr_votes.avg);
            return bIndex - aIndex;
          });
        setFederations(federationsWithStats);
        setLoading(false);
      })
      .catch((err) => {
        console.error('Failed to fetch federations:', err);
        setLoading(false);
      });
  }, []);

  const activeFederations = federations.filter((fed) => fed.health !== 'offline');
  const offlineFederations = federations.filter((fed) => fed.health === 'offline');

  return (
    <div>
      <div className="my-16">
        <Totals />
      </div>
      <div className="relative overflow-x-auto shadow-md sm:rounded-lg">
        <table className="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
          <caption className="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
            Observed Federations
            <p className="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
              List of all active federations this instance is collecting statistics on
            </p>
          </caption>
          <thead className="text-xs text-gray-700 uppercase bg-gray-50 dark:bg-gray-700 dark:text-gray-400">
            <tr>
              <th scope="col" className="px-6 py-3">
                Name
              </th>
              <th scope="col" className="px-6 py-3">
                <a
                  href="https://github.com/nostr-protocol/nips/pull/1110"
                  className="underline hover:no-underline"
                >
                  Recommendations
                </a>
              </th>
              <th scope="col" className="px-6 py-3">
                Invite Code
              </th>
              <th scope="col" className="px-6 py-3">
                Total Assets
              </th>
              <th scope="col" className="px-6 py-3">
                Average Activity (7d)
              </th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr>
                <td colSpan={5} className="px-6 py-4 text-center">
                  Loading...
                </td>
              </tr>
            ) : activeFederations.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-6 py-4 text-center">
                  No active federations found
                </td>
              </tr>
            ) : (
              activeFederations.map((fed) => (
                <FederationRow
                  key={fed.id}
                  id={fed.id}
                  name={fed.name || 'Unnamed'}
                  rating={fed.nostr_votes}
                  invite={fed.invite}
                  totalAssets={fed.deposits}
                  avgTxs={fed.avgTxs}
                  avgVolume={fed.avgVolume}
                  health={fed.health}
                />
              ))
            )}
          </tbody>
        </table>
      </div>

      <div className="relative overflow-x-auto shadow-md sm:rounded-lg mt-6">
        <table className="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
          <caption
            className="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800 cursor-pointer"
            onClick={() => setCollapseOffline(!collapseOffline)}
          >
            <svg
              className={`w-3 h-3 shrink-0 inline mr-2 ${collapseOffline ? 'rotate-180' : ''}`}
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 10 6"
            >
              <path
                stroke="currentColor"
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth="2"
                d="M9 5 5 1 1 5"
              />
            </svg>
            <span>Shut Down Federations</span>
            <p className="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
              List of federations that have ceased operations but were observed in the past
            </p>
          </caption>
          <thead
            className={
              collapseOffline
                ? 'hidden'
                : 'text-xs text-gray-700 uppercase bg-gray-50 dark:bg-gray-700 dark:text-gray-400'
            }
          >
            <tr>
              <th scope="col" className="px-6 py-3">
                Name
              </th>
              <th scope="col" className="px-6 py-3">
                <a
                  href="https://github.com/nostr-protocol/nips/pull/1110"
                  className="underline hover:no-underline"
                >
                  Recommendations
                </a>
              </th>
              <th scope="col" className="px-6 py-3">
                Invite Code
              </th>
              <th scope="col" className="px-6 py-3">
                Total Assets
              </th>
              <th scope="col" className="px-6 py-3">
                Average Activity (7d)
              </th>
            </tr>
          </thead>
          <tbody className={collapseOffline ? 'hidden' : ''}>
            {offlineFederations.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-6 py-4 text-center">
                  No offline federations
                </td>
              </tr>
            ) : (
              offlineFederations.map((fed) => (
                <FederationRow
                  key={fed.id}
                  id={fed.id}
                  name={fed.name || 'Unnamed'}
                  rating={fed.nostr_votes}
                  invite={fed.invite}
                  totalAssets={fed.deposits}
                  avgTxs={fed.avgTxs}
                  avgVolume={fed.avgVolume}
                  health={fed.health}
                />
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
