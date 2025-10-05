import { useEffect, useState } from 'react';
import { api } from '../services/api';
import type { FederationSummary } from '../types/api';
import { Copyable } from '../components/Copyable';
import { Button } from '../components/Button';

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
}

export function Nostr() {
  const [federations, setFederations] = useState<NostrFederation[]>([]);
  const [loading, setLoading] = useState(true);
  const [inviteCode, setInviteCode] = useState('');
  const [checking, setChecking] = useState(false);
  const [federationInfo, setFederationInfo] = useState<FederationInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .getNostrFederations()
      .then((data: FederationSummary[]) => {
        const nostrFeds = data.map((fed) => ({
          id: fed.id,
          name: fed.name,
          invite: fed.invite,
        }));
        setFederations(nostrFeds);
        setLoading(false);
      })
      .catch((err) => {
        console.error('Failed to fetch nostr federations:', err);
        setLoading(false);
      });
  }, []);

  const handleCheckFederation = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inviteCode.trim()) return;

    setChecking(true);
    setError(null);
    setFederationInfo(null);

    try {
      // Fetch federation config
      const configResponse = await fetch(
        `${import.meta.env.FMO_API_SERVER || 'http://127.0.0.1:3000'}/config/${inviteCode}`
      );
      
      if (!configResponse.ok) {
        throw new Error('Failed to fetch federation config');
      }

      const config = await configResponse.json();

      // Fetch federation metadata
      const metaResponse = await fetch(
        `${import.meta.env.FMO_API_SERVER || 'http://127.0.0.1:3000'}/config/${inviteCode}/meta`
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
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch federation info');
    } finally {
      setChecking(false);
    }
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
                placeholder=" "
                className="block px-2.5 h-11 w-full text-sm text-gray-900 bg-transparent rounded-lg border-gray-300 appearance-none dark:text-white dark:border-gray-600 dark:focus:border-blue-500 focus:outline-none focus:ring-0 focus:border-blue-600 peer border"
              />
              <label className="absolute text-sm text-gray-500 dark:text-gray-400 duration-300 transform -translate-y-4 scale-75 top-2 z-10 origin-[0] bg-white dark:bg-gray-800 px-2 peer-focus:px-2 peer-focus:text-blue-600 peer-focus:dark:text-blue-500 peer-placeholder-shown:scale-100 peer-placeholder-shown:-translate-y-1/2 peer-placeholder-shown:top-1/2 peer-focus:top-2 peer-focus:scale-75 peer-focus:-translate-y-4 rtl:peer-focus:translate-x-1/4 rtl:peer-focus:left-auto start-1">
                Invite Code
              </label>
            </div>
            <Button onClick={() => {}} disabled={checking} className="h-11">
              {checking ? 'Checking...' : 'Check Federation'}
            </Button>
            <Button
              colorScheme="success"
              onClick={() => alert('Announce Federation requires Nostr signer integration')}
              disabled={!federationInfo}
              className="h-11"
            >
              Announce Federation
            </Button>
          </form>

          {error && (
            <div className="p-4 mb-4 mt-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400">
              <span className="font-bold">Error: </span>
              {error}
            </div>
          )}

          {federationInfo && (
            <div className="flow-root mt-4">
              <div className="relative overflow-x-auto">
                <table className="w-full text-sm text-left rtl:text-right text-gray-500 dark:text-gray-400">
                  <tbody>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Name
                      </th>
                      <td className="px-6 py-4">{federationInfo.name || 'Loading...'}</td>
                    </tr>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Guardians
                      </th>
                      <td className="px-6 py-4 whitespace-normal">
                        {federationInfo.guardians || 'Loading...'}
                      </td>
                    </tr>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Modules
                      </th>
                      <td className="px-6 py-4 whitespace-normal">
                        {federationInfo.modules?.map((mod) => (
                          <span
                            key={mod}
                            className="bg-blue-100 text-blue-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-blue-900 dark:text-blue-300"
                          >
                            {mod}
                          </span>
                        ))}
                      </td>
                    </tr>
                    <tr className="bg-white border-b dark:bg-gray-800 dark:border-gray-700">
                      <th scope="row" className="px-6 py-4 font-medium text-gray-900 dark:text-white">
                        Network
                      </th>
                      <td className="px-6 py-4 whitespace-normal">
                        {federationInfo.network || 'Loading...'}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
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
