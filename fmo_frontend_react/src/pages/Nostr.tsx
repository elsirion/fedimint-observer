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
  config?: Record<string, unknown>; // Store the full config for announce
}

interface NostrEvent {
  kind: number;
  created_at: number;
  tags: string[][];
  content: string;
  pubkey: string;
}

interface SignedNostrEvent extends NostrEvent {
  id: string;
  sig: string;
}

// Extend window type for Nostr extension
declare global {
  interface Window {
    nostr?: {
      getPublicKey(): Promise<string>;
      signEvent(event: NostrEvent): Promise<SignedNostrEvent>;
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
          const BASE_URL = import.meta.env.VITE_FMO_API_BASE_URL || 'https://observer.fedimint.org/api';

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
        `${import.meta.env.VITE_FMO_API_BASE_URL || 'https://observer.fedimint.org/api'}/config/${inviteCode}`
      );

      if (!configResponse.ok) {
        throw new Error('Failed to fetch federation config');
      }

      const config = await configResponse.json();

      // Fetch federation metadata
      const metaResponse = await fetch(
        `${import.meta.env.VITE_FMO_API_BASE_URL || 'https://observer.fedimint.org/api'}/config/${inviteCode}/meta`
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
      const network = walletModule ? ((walletModule as any).value?.network || (walletModule as any).config?.network) : undefined;

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

      // Calculate federation ID from config
      const federationId = await calculateFederationId();

      // Use the stored invite code (the one user entered)
      if (!inviteCode.trim()) {
        throw new Error('Invite code is required');
      }

      // Get network and modules
      const network = federationInfo.network || 'unknown';
      const modules = federationInfo.modules?.join(',') || '';

      // Get public key from Nostr extension
      const pubkey = await window.nostr.getPublicKey();

      // Create unsigned event
      const unsignedEvent: NostrEvent = {
        kind: 38173,
        created_at: Math.floor(Date.now() / 1000),
        tags: [
          ['d', federationId],
          ['u', inviteCode],
          ['n', network],
          ['modules', modules],
        ],
        content: JSON.stringify((config as { global?: { meta?: Record<string, unknown> } }).global?.meta || {}),
        pubkey,
      };

      // Sign event with Nostr extension
      const signedEvent = await window.nostr.signEvent(unsignedEvent);

      // Publish to backend
      const response = await fetch(
        `${import.meta.env.VITE_FMO_API_BASE_URL || 'https://observer.fedimint.org/api'}/nostr/federations`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(signedEvent),
        }
      );

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Failed to announce federation (${response.status}): ${errorText}`);
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

  // Helper function to calculate federation ID from config
  // This matches the Rust implementation's calculate_federation_id() function
  const calculateFederationId = async (): Promise<string> => {
    try {
      // Use the API endpoint to calculate the federation ID from the invite code
      // since calculating it client-side would require crypto libraries
      const response = await fetch(
        `${import.meta.env.VITE_FMO_API_BASE_URL || 'https://observer.fedimint.org/api'}/config/${inviteCode}/id`
      );

      if (!response.ok) {
        throw new Error('Failed to fetch federation ID');
      }

      // The endpoint returns the FederationId directly as a JSON string
      const federationId = await response.json();
      return typeof federationId === 'string' ? federationId : String(federationId);
    } catch (err) {
      throw new Error(`Failed to calculate federation ID: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  return (
    <div className="pb-4">
  <div className="relative shadow-md rounded-lg overflow-hidden mt-8">
  <h1 className="p-4 sm:p-5 rounded-t-lg text-base sm:text-lg font-semibold text-left rtl:text-right text-gray-900 bg-blue-100 dark:text-white dark:bg-gray-800">
          Inspect Federation
          <p className="mt-1 text-xs sm:text-sm font-normal text-gray-500 dark:text-gray-400">
            Fetch federation info by invite code
          </p>
        </h1>

        <div className="p-4 sm:p-5 rounded-b-lg pt-0 bg-blue-100 dark:text-white dark:bg-gray-800">
          <form className="flex flex-col sm:flex-row gap-2 sm:items-center" onSubmit={handleCheckFederation}>
            <div className="relative flex-1 w-full">
              <input
                type="text"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                disabled={checking || announcing}
                placeholder=" "
                className="block px-2.5 h-11 w-full text-sm text-gray-900 bg-blue-50 rounded-lg border-gray-300 appearance-none dark:text-white dark:bg-gray-700 dark:border-gray-600 dark:focus:border-blue-500 focus:outline-none focus:ring-0 focus:border-blue-600 peer border disabled:opacity-50 disabled:cursor-not-allowed"
              />
              <label className="absolute text-sm text-gray-600 dark:text-gray-400 duration-300 transform -translate-y-4 scale-75 top-2 z-10 origin-[0] bg-blue-50 dark:bg-gray-700 px-2 peer-focus:px-2 peer-focus:text-blue-600 peer-focus:dark:text-blue-500 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1">
                Invite Code
              </label>
            </div>
            <div className="flex flex-col sm:flex-row gap-2 w-full sm:w-auto">
              <button
                type="submit"
                disabled={checking}
                className="h-11 w-full sm:w-auto whitespace-nowrap font-medium rounded-lg text-sm px-5 py-2.5 text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800 disabled:bg-blue-400 disabled:cursor-not-allowed"
              >
                {checking ? 'Checking...' : 'Check Federation'}
              </button>
              <button
                type="button"
                onClick={handleAnnounceFederation}
                disabled={!federationInfo || announcing || announceSuccess || checking}
                className="h-11 w-full sm:w-auto whitespace-nowrap font-medium rounded-lg text-sm px-5 py-2.5 text-white bg-green-700 hover:bg-green-800 focus:ring-4 focus:ring-green-300 dark:bg-green-600 dark:hover:bg-green-700 focus:outline-none disabled:bg-green-400 disabled:cursor-not-allowed"
              >
                {announcing ? 'Announcing...' : announceSuccess ? 'Announced!' : 'Announce Federation'}
              </button>
            </div>
          </form>

          {(checking || federationInfo) && (
            <div className="flow-root mt-4">
              <div className="relative">
                <table className="w-full text-xs sm:text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                  <tbody>
                    <tr className="bg-blue-100 border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-3 sm:px-6 py-3 sm:py-4 font-medium text-gray-900 dark:text-white w-24 sm:w-auto">
                        Name
                      </th>
                      <td className="px-3 sm:px-6 py-3 sm:py-4 break-words">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-32 sm:w-48"></div>
                        ) : (
                          federationInfo?.name || 'Unknown'
                        )}
                      </td>
                    </tr>
                    <tr className="bg-blue-100 border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-3 sm:px-6 py-3 sm:py-4 font-medium text-gray-900 dark:text-white w-24 sm:w-auto">
                        Guardians
                      </th>
                      <td className="px-3 sm:px-6 py-3 sm:py-4 break-words">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-32 sm:w-48"></div>
                        ) : (
                          federationInfo?.guardians || 0
                        )}
                      </td>
                    </tr>
                    <tr className="bg-blue-100 border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-3 sm:px-6 py-3 sm:py-4 font-medium text-gray-900 dark:text-white w-24 sm:w-auto align-top">
                        Modules
                      </th>
                      <td className="px-3 sm:px-6 py-3 sm:py-4 break-words">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-32 sm:w-48"></div>
                        ) : (
                          <div className="flex flex-wrap gap-1">
                            {federationInfo?.modules?.map((mod) => (
                              <span
                                key={mod}
                                className="bg-blue-600 text-white text-xs font-medium px-2.5 py-0.5 rounded  dark:bg-indigo-500"
                              >
                                {mod}
                              </span>
                            ))}
                          </div>
                        )}
                      </td>
                    </tr>
                    <tr className="bg-blue-100 border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-3 sm:px-6 py-3 sm:py-4 font-medium text-gray-900 dark:text-white w-24 sm:w-auto">
                        Network
                      </th>
                      <td className="px-3 sm:px-6 py-3 sm:py-4 break-words">
                        {checking ? (
                          <div className="h-2.5 bg-gray-200 rounded-full dark:bg-gray-700 w-32 sm:w-48"></div>
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
            <div className="p-3 sm:p-4 mb-4 mt-4 text-xs sm:text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400 break-words">
              <span className="font-bold">Error: </span>
              {error}
            </div>
          )}

          {announceSuccess && (
            <div className="p-3 sm:p-4 mb-4 mt-4 text-xs sm:text-sm text-green-800 rounded-lg bg-green-50 dark:bg-gray-800 dark:text-green-400">
              <span className="font-bold">Success! </span>
              Federation announced successfully! Reload the page to see it listed.
            </div>
          )}
        </div>
      </div>

  <div className="relative shadow-md rounded-lg overflow-hidden mt-8">
  <div className="p-4 sm:p-5 rounded-t-lg text-base sm:text-lg font-semibold text-left rtl:text-right text-gray-900 bg-blue-100 dark:text-white dark:bg-gray-800">
          Nostr Federations
          <p className="mt-1 text-xs sm:text-sm font-normal text-gray-500 dark:text-gray-400">
            Other federations announced via Nostr
          </p>
        </div>
        <div className="hidden sm:grid bg-gray-100 dark:bg-gray-700 px-3 sm:px-6 py-3 text-xs text-gray-700 dark:text-gray-400 uppercase font-semibold grid-cols-2 gap-2">
          <div>Name</div>
          <div>Invite Code</div>
        </div>
  <div className="divide-y divide-gray-200 dark:divide-gray-700 rounded-lg">
          {loading ? (
            <div className="px-3 sm:px-6 py-4 text-center text-xs sm:text-sm text-gray-500 dark:text-gray-400 bg-blue-100 dark:bg-gray-800">
              Loading...
            </div>
          ) : federations.length === 0 ? (
            <div className="px-3 sm:px-6 py-4 text-center text-xs sm:text-sm text-gray-500 dark:text-gray-400 bg-blue-100 dark:bg-gray-800">
              No Nostr federations found
            </div>
          ) : (
            federations.map((fed) => (
              <div
                key={fed.id}
                className="bg-blue-100 dark:bg-gray-800 px-3 sm:px-6 py-3 sm:py-4 grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-2 text-xs sm:text-sm"
              >
                <div className="font-medium text-gray-900 dark:text-white break-all">
                  <span className="text-[10px] sm:hidden uppercase text-gray-500 dark:text-gray-400 block mb-1">Name</span>
                  {fed.name || fed.id}
                </div>
                <div className='bg-transparent'>
                  <span className="text-[10px]  sm:hidden uppercase text-gray-500 dark:text-gray-400 block mb-1">Invite Code</span>
                  <Copyable text={fed.invite} />
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}