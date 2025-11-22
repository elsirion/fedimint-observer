import { useEffect, useState } from 'react';
import { api } from '../services/api';
import type { FederationSummary } from '../types/api';
import { Totals } from '../components/Totals';
import { FederationRow } from '../components/FederationRow';
import { ratingIndex } from '../utils/format';

export function Home() {
  const [federations, setFederations] = useState<FederationSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [collapseOffline, setCollapseOffline] = useState(true);

  useEffect(() => {
    api
      .getFederations()
      .then((data) => {
        const federationsWithStats = data
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

  // Compute global maxes across all federations for consistent chart scale
  const globalMaxTransaction = federations.reduce((max, fed) => {
    const fedMax = Math.max(...fed.last_7d_activity.map(d => d.num_transactions));
    return Math.max(max, fedMax);
  }, 0);

  const globalMaxVolume = federations.reduce((max, fed) => {
    const fedMax = Math.max(...fed.last_7d_activity.map(d => d.amount_transferred / 100000000000)); // convert to BTC
    return Math.max(max, fedMax);
  }, 0);

  return (
    <div className="pb-4">
      <div className="my-8 sm:my-16">
        <Totals />
      </div>
  <div className="relative overflow-x-auto bg-white shadow-md rounded-lg border border-gray-200 dark:bg-gray-800 dark:border-gray-700">
    <div className="p-4 sm:p-5 text-base sm:text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
          Observed Federations
          <p className="mt-1 text-xs sm:text-sm font-normal text-gray-500 dark:text-gray-400">
            List of all active federations this instance is collecting statistics on
          </p>
        </div>
        <div className="hidden md:grid bg-gray-50 dark:bg-gray-700 px-3 sm:px-6 py-3 text-xs text-gray-700 dark:text-gray-400 uppercase font-semibold grid-cols-5 gap-4 border-y border-gray-200 dark:border-gray-600">
          <div>Name</div>
          <div>
            <a
              href="https://github.com/nostr-protocol/nips/pull/1110"
              className="underline hover:no-underline"
            >
              Recommendations
            </a>
          </div>
          <div>Invite Code</div>
          <div>Total Assets</div>
          <div>Activity Charts (7d)</div>
        </div>
  <div className="divide-y divide-gray-200 dark:divide-gray-700">
          {loading ? (
            <div className="px-3 sm:px-6 py-4 text-center text-xs sm:text-sm text-gray-500 dark:text-gray-400 bg-white dark:bg-gray-800">
              Loading...
            </div>
          ) : activeFederations.length === 0 ? (
            <div className="px-3 sm:px-6 py-4 text-center text-xs sm:text-sm text-gray-500 dark:text-gray-400 bg-white dark:bg-gray-800">
              No active federations found
            </div>
          ) : (
            activeFederations.map((fed) => (
              <FederationRow
                key={fed.id}
                id={fed.id}
                name={fed.name || 'Unnamed'}
                rating={fed.nostr_votes}
                invite={fed.invite}
                totalAssets={fed.deposits}
                health={fed.health}
                activityData={fed.last_7d_activity}
                maxTransaction={globalMaxTransaction}
                maxVolume={globalMaxVolume}
              />
            ))
          )}
        </div>
      </div>

  <div className="relative overflow-x-auto bg-white shadow-md rounded-lg border border-gray-200 dark:bg-gray-800 dark:border-gray-700 mt-6">
        <div
          className="p-4 sm:p-5 text-base sm:text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800 cursor-pointer"
          onClick={() => setCollapseOffline(!collapseOffline)}
        >
          <svg
            className={`w-3 h-3 shrink-0 inline mr-2 transition-transform ${collapseOffline ? 'rotate-180' : ''}`}
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
          <p className="mt-1 text-xs sm:text-sm font-normal text-gray-500 dark:text-gray-400">
            List of federations that have ceased operations but were observed in the past
          </p>
        </div>
        {!collapseOffline && (
          <>
            <div className="hidden md:grid bg-gray-50 dark:bg-gray-700 px-3 sm:px-6 py-3 text-xs text-gray-700 dark:text-gray-400 uppercase font-semibold grid-cols-5 gap-4 border-y border-gray-200 dark:border-gray-600">
              <div>Name</div>
              <div>
                <a
                  href="https://github.com/nostr-protocol/nips/pull/1110"
                  className="underline hover:no-underline"
                >
                  Recommendations
                </a>
              </div>
              <div>Invite Code</div>
              <div>Total Assets</div>
              <div>Activity Charts (7d)</div>
            </div>
            <div className="divide-y divide-gray-200 dark:divide-gray-700">
              {loading ? (
                <div className="px-3 sm:px-6 py-4 text-center text-xs sm:text-sm text-gray-500 dark:text-gray-400 bg-white dark:bg-gray-800">
                  Loading...
                </div>
              ) : offlineFederations.length === 0 ? (
                <div className="px-3 sm:px-6 py-4 text-center text-xs sm:text-sm text-gray-500 dark:text-gray-400 bg-white dark:bg-gray-800">
                  No offline federations
                </div>
              ) : (
                offlineFederations.map((fed) => (
                  <FederationRow
                    key={fed.id}
                    id={fed.id}
                    name={fed.name || 'Unnamed'}
                    rating={fed.nostr_votes}
                    invite={fed.invite}
                    totalAssets={fed.deposits}
                    health={fed.health}
                    activityData={fed.last_7d_activity}
                    maxTransaction={globalMaxTransaction}
                    maxVolume={globalMaxVolume}
                  />
                ))
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
