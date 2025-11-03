import { createContext, useContext, useState, type ReactNode } from 'react';

interface TabsContextType {
  activeTab: string;
  setActiveTab: (tab: string) => void;
}

const TabsContext = createContext<TabsContextType | undefined>(undefined);

interface TabsProps {
  defaultTab: string;
  children: ReactNode;
}

export function Tabs({ defaultTab, children }: TabsProps) {
  const [activeTab, setActiveTab] = useState(defaultTab);

  return (
    <TabsContext.Provider value={{ activeTab, setActiveTab }}>
      <div>{children}</div>
    </TabsContext.Provider>
  );
}

interface TabListProps {
  tabs: string[];
}

export function TabList({ tabs }: TabListProps) {
  const context = useContext(TabsContext);
  if (!context) {
    throw new Error('TabList must be used within Tabs');
  }

  const { activeTab, setActiveTab } = context;

  const inactiveClasses =
    'inline-block p-4 border-b-2 border-transparent rounded-t-lg hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300';
  const activeClasses =
    'inline-block p-4 text-blue-600 border-b-2 border-blue-600 rounded-t-lg active dark:text-blue-500 dark:border-blue-500';

  return (
    <div className="text-sm font-medium text-center text-gray-500 border-b border-gray-200 dark:text-gray-400 dark:border-gray-700">
      <ul className="flex flex-wrap -mb-px">
        {tabs.map((tab) => (
          <li key={tab} className="me-2">
            <a
              href="#"
              className={activeTab === tab ? activeClasses : inactiveClasses}
              onClick={(e) => {
                e.preventDefault();
                setActiveTab(tab);
              }}
            >
              {tab}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}

interface TabPanelProps {
  name: string;
  children: ReactNode;
}

export function TabPanel({ name, children }: TabPanelProps) {
  const context = useContext(TabsContext);
  if (!context) {
    throw new Error('TabPanel must be used within Tabs');
  }

  const { activeTab } = context;

  if (activeTab !== name) {
    return null;
  }

  return <div>{children}</div>;
}
