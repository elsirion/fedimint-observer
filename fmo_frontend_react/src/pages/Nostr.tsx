import { useEffect, useState } from 'react';
import { api } from '../services/api';
import { Copyable } from '../components/Copyable';

interface NostrFederation {
  id: string;
  name: string | null;
  invite: string;
}

interface FederationInfo {
  name?: string;
  guardians?: number;
  modules?: string[];
  network?: string;
  config?: any; // Store the full config for announce
}

// Extend window type for Nostr extension
declare global {
  interface Window {
    nostr?: {
      getPublicKey(): Promise<string>;
      signEvent(event: any): Promise<any>;
    };
  }
}

export function Nostr() {
  const [federations, setFederations] = useState<NostrFederation[]>([]);
  const [loading, setLoading] = useState(true);
  const [inviteCode, setInviteCode] = useState('');
  const [checking, setChecking] = useState(false);
  const [federationInfo, setFederationInfo] = useState<FederationInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [announcing, setAnnouncing] = useState(false);
  const [announceSuccess, setAnnounceSuccess] = useState(false);

  useEffect(() => {
    const fetchFederationsWithNames = async () => {
      try {
        const [nostrData, mainFederations] = await Promise.all([
          api.getNostrFederations(),
          api.getFederations()
        ]);

        // Create a map of federation IDs to names from the main list
        const nameMap = new Map(
          mainFederations.map(fed => [fed.id, fed.name])
        );

        // Convert the nostr object to an array
        const nostrFeds = Object.entries(nostrData).map(([id, invite]) => ({
          id,
          name: nameMap.get(id) || null,
          invite,
        }));

        setFederations(nostrFeds);
        setLoading(false); // Stop loading immediately - show table with IDs

        // For federations without names, fetch from /config/{invite}/meta in background
        const federationsWithoutNames = nostrFeds.filter(fed => !fed.name);

        if (federationsWithoutNames.length > 0) {
          const BASE_URL = import.meta.env.VITE_FMO_API_BASE_URL || 'http://127.0.0.1:3000';

          // Fetch names for federations without them (in background, non-blocking)
          federationsWithoutNames.forEach(async (fed) => {
            try {
              const metaResponse = await fetch(`${BASE_URL}/config/${fed.invite}/meta`, {
                signal: AbortSignal.timeout(5000), // 5 second timeout
              });
              if (metaResponse.ok) {
                const meta = await metaResponse.json();
                const fetchedName = meta.federation_name || null;

                // Update this specific federation's name
                if (fetchedName) {
                  setFederations(prevFeds =>
                    prevFeds.map(f =>
                      f.id === fed.id ? { ...f, name: fetchedName } : f
                    )
                  );
                }
              }
            } catch (err) {
              console.error(`Failed to fetch name for ${fed.id}:`, err);
            }
          });
        }
      } catch (err) {
        console.error('Failed to fetch nostr federations:', err);
        setLoading(false);
      }
    };

    fetchFederationsWithNames();
  }, []);

  const handleCheckFederation = async (e: React.FormEvent) => {
    e.preventDefault();

    // Clear previous data and errors
    setFederationInfo(null);
    setError(null);
    setAnnounceSuccess(false);

    // Validate invite code is not empty
    if (!inviteCode.trim()) {
      setError('Invite code not present. Please enter a valid invite code.');
      return;
    }

    setChecking(true);

    try {
      // Fetch federation config
      const configResponse = await fetch(
        `${import.meta.env.VITE_FMO_API_BASE_URL || 'http://127.0.0.1:3000'}/config/${inviteCode}`
      );

      if (!configResponse.ok) {
        throw new Error('Failed to fetch federation config');
      }

      const config = await configResponse.json();

      // Fetch federation metadata
      const metaResponse = await fetch(
        `${import.meta.env.VITE_FMO_API_BASE_URL || 'http://127.0.0.1:3000'}/config/${inviteCode}/meta`
      );

      let name = 'Unknown';
      if (metaResponse.ok) {
        const meta = await metaResponse.json();
        name = meta.federation_name || 'Unknown';
      }

      // Extract info from config
      const guardians = config.global?.api_endpoints
        ? Object.keys(config.global.api_endpoints).length
        : 0;

      const modules = config.modules
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        ? Object.values(config.modules).map((mod: any) => mod.kind || 'unknown')
        : [];

      const walletModule = config.modules
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        ? Object.values(config.modules).find((mod: any) => mod.kind === 'wallet')
        : null;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const network = walletModule ? (walletModule as any).network : undefined;

      setFederationInfo({
        name,
        guardians,
        modules,
        network,
        config, // Store config for announce
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch federation info');
    } finally {
      setChecking(false);
    }
  };

  const handleAnnounceFederation = async () => {
    if (!federationInfo?.config) return;

    setAnnouncing(true);
    setError(null);
    setAnnounceSuccess(false);

    try {
      // Check if Nostr extension is available
      if (!window.nostr) {
        throw new Error('Nostr extension not found. Please install a Nostr browser extension like nos2x or Alby.');
      }

      const config = federationInfo.config;

      // Calculate federation ID
      const federationId = config.global.federation_id;

  // Build invite code from config
  const inviteCode = await buildInviteCode(config);

      // Get network and modules
      const network = federationInfo.network || 'unknown';
      const modules = federationInfo.modules?.join(',') || '';

      // Get public key from Nostr extension
      const pubkey = await window.nostr.getPublicKey();

      // Create unsigned event
      const unsignedEvent = {
        kind: 38173,
        created_at: Math.floor(Date.now() / 1000),
        tags: [
          ['d', federationId],
          ['u', inviteCode],
          ['n', network],
          ['modules', modules],
        ],
        content: JSON.stringify(config.global.meta || {}),
        pubkey,
      };

      // Sign event with Nostr extension
      const signedEvent = await window.nostr.signEvent(unsignedEvent);

      // Publish to backend
      const response = await fetch(
        `${import.meta.env.VITE_FMO_API_BASE_URL || 'http://127.0.0.1:3000'}/nostr/federations`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(signedEvent),
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to announce federation: ${response.status}`);
      }

      setAnnounceSuccess(true);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to announce federation');
      setAnnounceSuccess(false);
    } finally {
      setAnnouncing(false);
    }
  };

  // Helper function to build invite code from config
  const buildInviteCode = async (_config: any): Promise<string> => {
    // Use the stored invite code or build from config
    if (inviteCode) {
      return inviteCode;
    }

    // If we don't have the invite code, we can't proceed
    throw new Error('Invite code not available');
  };

  return (
    <div>
      <div className="relative overflow-x-auto shadow-md sm:rounded-lg mt-8">
        <h1 className="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
          Inspect Federation
          <p className="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
            Fetch federation info by invite code
          </p>
        </h1>

        <div className="p-5 pt-0 dark:text-white dark:bg-gray-800">
          <form className="flex gap-2 items-center" onSubmit={handleCheckFederation}>
            <div className="relative flex-1">
              <input
                type="text"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                disabled={checking || announcing}
                placeholder=" "
                className="block px-2.5 h-11 w-full text-sm text-gray-900 bg-transparent rounded-lg border-gray-300 appearance-none dark:text-white dark:border-gray-600 dark:focus:border-blue-500 focus:outline-none focus:ring-0 focus:border-blue-600 peer border disabled:opacity-50 disabled:cursor-not-allowed"
              />
              <label className="absolute text-sm text-gray-500 dark:text-gray-400 duration-300 transform -translate-y-4 scale-75 top-2 z-10 origin-[0] bg-white dark:bg-gray-800 px-2 peer-focus:px-2 peer-focus:text-blue-600 peer-focus:dark:text-blue-500 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1">
                Invite Code
              </label>
            </div>
            <button
              type="submit"
              disabled={checking}
              className="h-11 whitespace-nowrap font-medium rounded-lg text-sm px-5 py-2.5 text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800 disabled:bg-blue-400 disabled:cursor-not-allowed"
            >
              {checking ? 'Checking...' : 'Check Federation'}
            </button>
            <button
              type="button"
              onClick={handleAnnounceFederation}
              disabled={!federationInfo || announcing || announceSuccess || checking}
              className="h-11 whitespace-nowrap font-medium rounded-lg text-sm px-5 py-2.5 text-white bg-green-700 hover:bg-green-800 focus:ring-4 focus:ring-green-300 dark:bg-green-600 dark:hover:bg-green-700 focus:outline-none disabled:bg-green-400 disabled:cursor-not-allowed"
            >
              {announcing ? 'Announcing...' : announceSuccess ? 'Announced!' : 'Announce Federation'}
            </button>
          </form>

          {(checking || federationInfo) && (
            <div className="flow-root mt-4">
              <div className="relative overflow-x-auto">
                <table className="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                  <tbody>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Name
                      </th>
                      <td className="px-6 py-4">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-48"></div>
                        ) : (
                          federationInfo?.name || 'Unknown'
                        )}
                      </td>
                    </tr>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Guardians
                      </th>
                      <td className="px-6 py-4 whitespace-normal">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-48"></div>
                        ) : (
                          federationInfo?.guardians || 0
                        )}
                      </td>
                    </tr>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Modules
                      </th>
                      <td className="px-6 py-4 whitespace-normal">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-48"></div>
                        ) : (
                          federationInfo?.modules?.map((mod) => (
                            <span
                              key={mod}
                              className="bg-blue-100 text-blue-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-blue-900 dark:text-blue-300"
                            >
                              {mod}
                            </span>
                          ))
                        )}
                      </td>
                    </tr>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Network
                      </th>
                      <td className="px-6 py-4 whitespace-normal">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-48"></div>
                        ) : (
                          federationInfo?.network || 'Unknown'
                        )}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {error && (
            <div className="p-4 mb-4 mt-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400">
              <span className="font-bold">Error: </span>
              {error}
            </div>
          )}

          {announceSuccess && (
            <div className="p-4 mb-4 mt-4 text-sm text-green-800 rounded-lg bg-green-50 dark:bg-gray-800 dark:text-green-400">
              <span className="font-bold">Success! </span>
              Federation announced successfully! Reload the page to see it listed.
            </div>
          )}
        </div>
      </div>

      <div className="relative overflow-x-auto shadow-md sm:rounded-lg mt-8">
        <table className="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
          <caption className="p-5 text-lg font-semibold text-left rtl:text-right text-gray-900 bg-white dark:text-white dark:bg-gray-800">
            Nostr Federations
            <p className="mt-1 text-sm font-normal text-gray-500 dark:text-gray-400">
              Other federations announced via Nostr
            </p>
          </caption>
          <thead className="text-xs text-gray-700 uppercase bg-gray-50 dark:bg-gray-700 dark:text-gray-400">
            <tr>
              <th scope="col" className="px-6 py-3">
                Name
              </th>
              <th scope="col" className="px-6 py-3">
                Invite Code
              </th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr>
                <td colSpan={2} className="px-6 py-4 text-center">
                  Loading...
                </td>
              </tr>
            ) : federations.length === 0 ? (
              <tr>
                <td colSpan={2} className="px-6 py-4 text-center">
                  No Nostr federations found
                </td>
              </tr>
            ) : (
              federations.map((fed) => (
                <tr
                  key={fed.id}
                  className="bg-white border-b dark:bg-gray-800 dark:border-gray-700"
                >
                  <th
                    scope="row"
                    className="px-6 py-4 font-medium text-gray-900 whitespace-nowrap dark:text-white"
                  >
                    {fed.name || fed.id}
                  </th>
                  <td className="px-6 py-4">
                    <Copyable text={fed.invite} />
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
