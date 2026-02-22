import { getCurrentWindow } from "@tauri-apps/api/window";
import { ExtraPage } from './pages/Extra';
import { HomePage } from './pages/Home';
import { SettingsPage } from "./pages/Settings";
import { EditorPage } from "./pages/Editor";
import { NotificationProvider } from './Notifications';
import { StrategyModal } from "./modals/StrategyModal"
import { IpsetModal } from "./modals/IpsetModal"
import { ConvertModal } from "./modals/ConvertModal"
import { HostsModal } from "./modals/HostsModal"
import { LegacyModal } from "./modals/LegacyModal";
import { Logic } from "./Logic";
import { ResolverModal } from "./modals/ResolverModal";
import { ProxyModal } from "./modals/ProxyModal";
import { NewsModal } from "./modals/NewsModal";
import { memo, useEffect, useRef, useState } from "react";
import Particles from "@tsparticles/react";
import { ProxyPage } from "./pages/Proxy";
import { DeepLinkModal } from "./modals/DeepLinkModal";

const STRASHILKI_BUGAGA = 11;

const particlesOptions: any = {
  fpsLimit: 120,
  background: { color: "transparent" },
  particles: {
    number: {
      value: 100,
      limit: 150,
      density: { enable: true, area: 800 }
    },
    color: {
      value: ["#a855f7", "#6366f1", "#ffffff"],
    },
    shape: {
      type: "circle",
    },
    opacity: {
      value: { min: 0.1, max: 0.5 },
      animation: { enable: true, speed: 1, sync: false }
    },
    size: {
      value: { min: 1, max: 3 },
      animation: { enable: true, speed: 2, sync: false }
    },
    links: {
      enable: true,
      distance: 120,
      color: "#6366f1",
      opacity: 0.2,
      width: 1,
      consent: false,
      revealOffset: 10
    },
    move: {
      enable: true,
      speed: { min: 0.1, max: 0.8 },
      direction: "none",
      random: true,
      straight: false,
      outModes: { default: "out" },
      attract: {
        enable: true,
        rotateX: 2000,
        rotateY: 2000
      }
    }
  },
  interactivity: {
    events: {
      onHover: {
        enable: true,
        mode: ["bubble", "connect"],
        parallax: { enable: true, force: 100, smooth: 15 }
      },
      onClick: { enable: true, mode: "repulse" }
    },
    modes: {
      bubble: {
        distance: 200,
        size: 6,
        duration: 0.3,
        opacity: 0.8,
        color: "#ffffff"
      },
      connect: {
        distance: 150,
        links: { opacity: 0.4 },
        radius: 150
      },
      repulse: {
        distance: 250,
        duration: 0.4
      }
    }
  },
  detectRetina: true
};

const ParticlesBackground = memo(() => {
  return (
    <Particles
      id="particles"
      key="constant-particles-root"
      options={particlesOptions}
    />
  );
});

const PAGE_INDEX: Record<string, number> = { home: 0, settings: 1, proxy: 2, editor: 3, credits: 4 };

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
  credits: {
    label: 'Дополнительно',
    path: <path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" />
  }
};

const NavItem = ({ id, isActive, onClick, onHover }: any) => {
  const icon = NAV_ICONS[id as keyof typeof NAV_ICONS];
  return (
    <button className={`nav-item ${isActive ? 'active' : ''}`} onClick={onClick} onMouseEnter={() => onHover(icon.label)} onMouseLeave={() => onHover(null)}>
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">{icon.path}</svg>
    </button>
  );
};

function App() {
  const { state, prefs, actions } = Logic();
  const { zapret } = state;

  const [bugagaConfig, setBugagaConfig] = useState<{ path: string, id: number } | null>(null);
  const jumpscareTimer = useRef<NodeJS.Timeout | null>(null);
  const lastClickTime = useRef<number>(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const creditsClickCount = useRef(0);

  useEffect(() => {
    return () => {
      if (jumpscareTimer.current) clearTimeout(jumpscareTimer.current);
    };
  }, []);

  const handleNavClick = (id: string) => {
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

    prefs.setActivePage(id);
  };

  return (
    <div className="main-container">
      <ParticlesBackground />
      <header className="titlebar" data-tauri-drag-region>
        <div className="app-identity" data-tauri-drag-region>
          <span className="star-icon">✦</span>
          <span className="app-name">Zust</span>
        </div>
        <div className="window-controls">
          <button className={`win-btn pin ${state.isPinned ? 'active' : ''}`} onClick={actions.togglePin} title="Always on top">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 17v5" /><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" /></svg>
          </button>
          <button className="win-btn" onClick={() => getCurrentWindow().minimize()}>−</button>
          <button className="win-btn" onClick={() => getCurrentWindow().toggleMaximize()}>☐</button>
          <button className="win-btn close" onClick={() => getCurrentWindow().close()}>×</button>
        </div>
      </header>
      <NotificationProvider>
        <main className="scroll-area">
          <div className="pages-slider" style={{ transform: `translateX(-${PAGE_INDEX[state.activePage] * 20}%)` }}>
            <div className={`page ${state.activePage === 'home' ? 'active' : ''}`}>
              <HomePage
                {...zapret}
                onSelectConfig={actions.handleStrategyChange}
                isSelectorOpen={state.isSelectorOpen}
                setIsSelectorOpen={prefs.setIsSelectorOpen}
                loadConfigs={actions.loadIpsetConfigs}
                setIsConvertOpen={prefs.setIsConvertOpen}
                setIsIpsetModalOpen={prefs.setIsIpsetModalOpen}
                setIsHostsModalOpen={prefs.setIsHostsModalOpen}
                setIsResolverOpen={prefs.setIsResolverOpen}
                setIsProxyModalOpen={prefs.setIsProxyModalOpen}
                setIsNewsModalOpen={prefs.setIsNewsModalOpen}
                handleToggle={() => zapret.status === 'stopped' ? zapret.startProcess() : zapret.stopProcess()}
              />
            </div>

            <div className={`page ${state.activePage === 'settings' ? 'active' : ''}`}><SettingsPage /></div>
            <div className={`page ${state.activePage === 'proxy' ? 'active' : ''}`}><ProxyPage /></div>
            <div className={`page ${state.activePage === 'editor' ? 'active' : ''}`}><EditorPage /></div>
            <div className={`page ${state.activePage === 'credits' ? 'active' : ''}`}><ExtraPage
              {...zapret}
              logs={state.logs}
              logStart={state.logStart} /></div>
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
        {Object.keys(NAV_ICONS).map((id) => (
          <NavItem key={id} id={id} isActive={state.activePage === id} onClick={() => handleNavClick(id)} onHover={prefs.setHoverText} />
        ))}
        <div className={`nav-tooltip ${state.hoverText ? 'visible' : ''}`}>{state.hoverText || state.lastText}</div>
      </footer>
      <StrategyModal isOpen={state.isSelectorOpen} onClose={() => prefs.setIsSelectorOpen(false)} configs={zapret.configs} stratName={zapret.stratName} onSelect={actions.handleStrategyChange} updatableStrats={state.updatableStrats} setUpdatableStrats={prefs.setUpdatableStrats} />
      <ConvertModal isOpen={state.isConvertOpen} onClose={() => prefs.setIsConvertOpen(false)} onPick={actions.handlePickFiles} />
      <IpsetModal
        isOpen={state.isIpsetModalOpen}
        onClose={() => { prefs.setIsIpsetModalOpen(false); prefs.setIpsetView('main'); }}
        selectedIpset={zapret.selectedIpset}
        ipsetView={state.ipsetView}
        setIpsetView={prefs.setIpsetView}
        customIpsetFiles={state.customIpsetFiles}
        onModeChange={actions.changeIpsetMode}
        loadCustom={actions.loadIpsetConfigs}
        hoveredDesc={state.hoveredDesc}
        setHoveredDesc={prefs.setHoveredDesc}
      />
      <DeepLinkModal
        isOpen={state.isDeepLinkModalOpen}
        onClose={() => prefs.setIsDeepLinkModalOpen(false)}
        state={state}
        data={state.deepLinkData}
      />
      <HostsModal isOpen={state.isHostsModalOpen} onClose={() => prefs.setIsHostsModalOpen(false)} />
      <LegacyModal isOpen={state.isLegacyOpen} />
      <ResolverModal
        isOpen={state.isResolverOpen}
        onClose={() => prefs.setIsResolverOpen(false)}
        selectedIpset={zapret.selectedIpset}
        onAdd={actions.addIp}
      />
      <ProxyModal isOpen={state.isProxyModalOpen} onClose={() => prefs.setIsProxyModalOpen(false)} />
      {state.isNewsModalOpen && state.isUpdateChecked && (
        <NewsModal
          updateAvailable={state.updateAvailable}
          onClose={() => prefs.setIsNewsModalOpen(false)}
        />
      )}
    </div>
  );
}

export default App;