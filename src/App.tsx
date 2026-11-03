import { getCurrentWindow } from "@tauri-apps/api/window";
import { ExtraPage } from './pages/Extra';
import { HomePage } from './pages/Home';
import { SettingsPage } from "./pages/Settings";
import { EditorPage } from "./pages/Editor";
import { LogsPage } from "./pages/Logs";
import { ProxyPage } from "./pages/Proxy";
import { NotificationProvider } from './Notifications';
import { StrategyModal } from "./modals/StrategyModal"
import { IpsetModal } from "./modals/IpsetModal"
import { ConvertModal } from "./modals/ConvertModal"
import { HostsModal } from "./modals/HostsModal"
import { ProxyModal } from "./modals/ProxyModal";
import { NewsModal } from "./modals/NewsModal";
import { DeepLinkModal } from "./modals/DeepLinkModal";
import { HotspotModal } from "./modals/HotSpotModal";
import { SponsorsModal } from "./modals/SponsorsModal";
import { CommunityModal } from "./modals/CommunityModal";
import { RepairZapret4TorModal } from "./modals/RepairZapret4Tor";
import { useApp, type PageId } from "./Logic";
import { useEffect, useRef, useState } from "react";
import { StarsBackground } from "./components/animate-ui/components/backgrounds/stars";
import { cn } from "./lib/utils";
import { Dock, DockIcon } from "./components/ui/dock";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./components/ui/tooltip";
import { Separator } from "./components/ui/separator";
import { motion } from "motion/react";
import { FreezeProvider } from "./Freeze";

const STRASHILKI_BUGAGA = 12;

const PAGES: PageId[] = ['home', 'settings', 'proxy', 'editor', 'logs', 'credits'];

const NAV_ICONS = {
  home: {
    label: 'Меню',
    path: <><path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8" /><path d="M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></>
  },
  settings: {
    label: 'Настройки',
    path: <><path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" /><circle cx="12" cy="12" r="3" /></>
  },
  proxy: {
    label: 'Прокси',
    path: (
      <>
        <path d="M18 11c-1.5 0-2.5.5-3 2" /><path d="M4 6a2 2 0 0 0-2 2v4a5 5 0 0 0 5 5 8 8 0 0 1 5 2 8 8 0 0 1 5-2 5 5 0 0 0 5-5V8a2 2 0 0 0-2-2h-3a8 8 0 0 0-5 2 8 8 0 0 0-5-2z" /><path d="M6 11c1.5 0 2.5.5 3 2" />
      </>
    )
  },
  editor: {
    label: 'Редактор',
    path: <><path d="M13 21h8" /><path d="m15 5 4 4" /><path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" /></>
  },
  logs: {
    label: 'Логи',
    path: <><path d="M15 12h-5" /><path d="M15 8h-5" /><path d="M19 17V5a2 2 0 0 0-2-2H4" /><path d="M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3" /></>
  },
  credits: {
    label: 'Дополнительно',
    path: <path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" />
  }
};

const NavItem = (props: any) => {
  const { id, isActive, onClick, ...dockProps } = props;
  const icon = NAV_ICONS[id as keyof typeof NAV_ICONS];

  return (
    <DockIcon {...dockProps}>
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            onClick={onClick}
            className={cn(
              "nav-item relative flex h-full w-full items-center justify-center rounded-full outline-none",
              isActive && "active"
            )}
          >
            <motion.div
              animate={isActive ? {
                y: [0, -12, 0],
                scale: [1, 1.3, 1.15],
                rotate: [0, 15, -10, 0],
              } : {
                y: 0,
                scale: 1,
                rotate: 0,
              }}
              transition={{
                type: "spring",
                stiffness: 400,
                damping: 10,
                duration: 0.5
              }}
              whileTap={{ scale: 0.9, y: 2 }}
              className="flex items-center justify-center"
            >
              <svg
                width="22"
                height="22"
                viewBox="0 0 24 24"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="icon-svg pointer-events-none"
              >
                {icon.path}
              </svg>
            </motion.div>
          </button>
        </TooltipTrigger>
        <TooltipContent className="border-white/10 bg-black/80 text-white backdrop-blur-md">
          <p>{icon.label}</p>
        </TooltipContent>
      </Tooltip>
    </DockIcon>
  );
};

function App() {
  const app = useApp();
  const { zapret } = app;

  const [bugagaConfig, setBugagaConfig] = useState<{ path: string, id: number } | null>(null);
  const jumpscareTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastClickTime = useRef<number>(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const creditsClickCount = useRef(0);

  // страница рендерится при первом посещении и дальше остаётся в памяти
  const [renderedPages, setRenderedPages] = useState<PageId[]>([app.activePage]);
  useEffect(() => {
    setRenderedPages((prev) => prev.includes(app.activePage) ? prev : [...prev, app.activePage]);
  }, [app.activePage]);

  useEffect(() => () => {
    if (jumpscareTimer.current) clearTimeout(jumpscareTimer.current);
  }, []);

  const handleNavClick = (id: PageId) => {
    const currentTime = Date.now();

    if (id === 'credits') {
      if (currentTime - lastClickTime.current > 5000) {
        creditsClickCount.current = 0;
      }

      lastClickTime.current = currentTime;
      creditsClickCount.current += 1;

      if (creditsClickCount.current >= 20) {
        if (audioRef.current) {
          audioRef.current.pause();
          audioRef.current.currentTime = 0;
        }
        const audio = new Audio('/jumpscare.mp3');
        audioRef.current = audio;
        audio.play().catch(() => { });

        const random = Math.floor(Math.random() * STRASHILKI_BUGAGA) + 1;
        const path = `/jumpscare_${random}.jpg`;

        setBugagaConfig(null);

        setTimeout(() => {
          setBugagaConfig({ path: path, id: Date.now() });
          if (jumpscareTimer.current) clearTimeout(jumpscareTimer.current);
          jumpscareTimer.current = setTimeout(() => {
            setBugagaConfig(null);
          }, 5000);
        }, 5);
      }
    } else {
      creditsClickCount.current = 0;
    }

    app.setActivePage(id);
  };

  const renderPage = (pageId: PageId) => {
    if (!renderedPages.includes(pageId)) return null;

    switch (pageId) {
      case 'home': return <HomePage zapret={zapret} openModal={app.openModal} />;
      case 'settings': return <SettingsPage zapret={zapret} />;
      case 'proxy': return <ProxyPage
        hotspotPort={app.hotspotPort}
        setHotspotPort={app.setHotspotPort}
        openHotspot={() => app.openModal('hotspot')}
      />;
      case 'editor': return <EditorPage />;
      case 'logs': return <LogsPage logs={app.logs} torLogs={app.torLogs} />;
      case 'credits': return <ExtraPage appInfo={app.appInfo} updateAvailable={app.updateAvailable} openModal={app.openModal} />;
    }
  };

  const isOpen = (id: Parameters<typeof app.openModal>[0]) => app.modal === id;
  const pageIndex = PAGES.indexOf(app.activePage);

  return (
    <FreezeProvider>
      <div className="relative w-full h-screen bg-[#05010a] overflow-hidden">
        <StarsBackground speed={20} className={cn(
          'absolute inset-0 flex items-center justify-center rounded-xl brightness-50',
          'bg-[radial-gradient(ellipse_at_bottom,#180a30_0%,#000_100%)]'
        )} />
        <header className="titlebar" style={{ userSelect: 'none' }} data-tauri-drag-region>
          <div className="app-identity" data-tauri-drag-region>
            <span className="star-icon">✦</span>
            <span className="app-name">Zust</span>
            {app.appInfo && (
              <span className="text-[11px] text-white/30" title={`${app.appInfo.commit} · ${app.appInfo.buildDate}`}>
                v{app.appInfo.version}
              </span>
            )}
            {app.updateAvailable && (
              <button
                className="rounded-full bg-blue-500/15 px-2 text-[10px] font-bold text-blue-300 hover:bg-blue-500/25"
                onClick={() => app.openModal('news')}
              >
                обновление
              </button>
            )}
          </div>
          <div className="window-controls">
            <button className={`win-btn pin ${app.isPinned ? 'active' : ''}`} onClick={app.actions.togglePin} title="Always on top">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 17v5" /><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" /></svg>
            </button>
            <button className="win-btn" onClick={() => getCurrentWindow().minimize()}>−</button>
            <button className="win-btn" onClick={() => getCurrentWindow().toggleMaximize()}>☐</button>
            <button className="win-btn close" onClick={() => getCurrentWindow().close()}>×</button>
          </div>
        </header>
        <NotificationProvider>
          <main className="scroll-area">
            <div
              className="pages-slider"
              style={{ width: `${PAGES.length * 100}%`, transform: `translateX(-${(pageIndex * 100) / PAGES.length}%)` }}
            >
              {PAGES.map((pageId) => (
                <div key={pageId} className={`page ${app.activePage === pageId ? 'active' : ''}`} style={{ width: `${100 / PAGES.length}%` }}>
                  {renderPage(pageId)}
                </div>
              ))}
            </div>
          </main>
        </NotificationProvider>
        {bugagaConfig && (
          <div
            key={bugagaConfig.id}
            style={{
              position: 'fixed',
              top: 0,
              left: 0,
              width: '100vw',
              height: '100vh',
              zIndex: 999999,
              pointerEvents: 'none',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'transparent'
            }}
          >
            <div className="jpeg-ruin" style={{
              width: '100%',
              height: '100%',
              background: 'transparent'
            }}>
              <img
                src={bugagaConfig.path}
                style={{
                  width: '100%',
                  height: '100%',
                  objectFit: 'fill',
                  animation: 'jumpscare-simple 5s ease-out forwards',
                  display: 'block',
                  background: 'transparent'
                }}
              />
            </div>
          </div>
        )}

        <style>
          {`
  @keyframes jumpscare-simple {
    0% { opacity: 1; }
    15% { opacity: 1; }
    100% { opacity: 0; }
  }

  .jpeg-ruin {
    position: relative;
    filter: contrast(1.5) brightness(1.2) saturate(1.8) blur(0.4px);
    image-rendering: pixelated;
  }

  .jpeg-ruin::after {
    content: "";
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    opacity: 0.25;
    mix-blend-mode: overlay;
    pointer-events: none;
    mask-image: linear-gradient(#000, #000);
  }
  `}
        </style>

        <footer className="bottom-nav">
          <TooltipProvider>
            <Dock
              direction="middle"
              iconSize={50}
              iconMagnification={80}
              iconDistance={60}
              className="bg-transparent border-white/10 border p-2 rounded-2xl"
            >
              <NavItem
                id="home"
                isActive={app.activePage === 'home'}
                onClick={() => handleNavClick('home')}
              />
              <Separator orientation="vertical" className="h-8 bg-white/10 mx-2" />
              {(['settings', 'proxy', 'editor', 'logs'] as const).map((id) => (
                <NavItem
                  key={id}
                  id={id}
                  isActive={app.activePage === id}
                  onClick={() => handleNavClick(id)}
                />
              ))}

              <Separator orientation="vertical" className="h-8 bg-white/10 mx-2" />

              <NavItem
                id="credits"
                isActive={app.activePage === 'credits'}
                onClick={() => handleNavClick('credits')}
              />
            </Dock>
          </TooltipProvider>
        </footer>
        <StrategyModal
          isOpen={isOpen('strategy')}
          onClose={app.closeModal}
          configs={zapret.configs}
          stratName={zapret.running}
          onSelect={app.actions.selectStrategy}
          updatableStrats={app.updatableStrats}
          setUpdatableStrats={app.setUpdatableStrats}
        />
        <ConvertModal isOpen={isOpen('convert')} onClose={app.closeModal} onPick={app.actions.pickBatFiles} />
        <IpsetModal
          isOpen={isOpen('ipset')}
          onClose={() => { app.closeModal(); app.setIpsetView('main'); }}
          selectedIpset={zapret.ipset}
          ipsetView={app.ipsetView}
          setIpsetView={app.setIpsetView}
          customIpsetFiles={app.customIpsetFiles}
          onModeChange={app.actions.changeIpsetMode}
          loadCustom={app.actions.loadIpsetConfigs}
          hoveredDesc={app.hoveredDesc}
          setHoveredDesc={app.setHoveredDesc}
        />
        <DeepLinkModal isOpen={isOpen('deeplink')} onClose={app.closeModal} zapret={zapret} data={app.deepLinkData} />
        <HostsModal isOpen={isOpen('hosts')} onClose={app.closeModal} />
        <ProxyModal isOpen={isOpen('proxy')} onClose={app.closeModal} />
        <HotspotModal isOpen={isOpen('hotspot')} onClose={app.closeModal} onLaunched={app.setHotspotPort} />
        <NewsModal isOpen={isOpen('news')} onClose={app.closeModal} updateAvailable={app.updateAvailable} />
        <SponsorsModal isOpen={isOpen('sponsors')} onClose={app.closeModal} />
        <CommunityModal isOpen={isOpen('community')} onClose={app.closeModal} />
        <RepairZapret4TorModal stratName={zapret.strategy} />
      </div>
    </FreezeProvider>
  );
}

export default App;