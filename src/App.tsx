import { getCurrentWindow } from "@tauri-apps/api/window";
import { CreditsPage } from './pages/Credits';
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
import { Initializer } from "./main";

const PAGE_INDEX: Record<string, number> = { home: 0, settings: 1, editor: 2, credits: 3 };

const NAV_ICONS = {
  home: {
    label: 'Меню',
    path: <><path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8" /><path d="M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></>
  },
  settings: {
    label: 'Настройки',
    path: <><path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" /><circle cx="12" cy="12" r="3" /></>
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

  return (
    <div className="main-container">
      <div className="space-background">
        <div className="nebula"></div>
        <div className="stars"></div>
        <div className="stars2"></div>
        <div className="shooting-stars"></div>
      </div>

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

      <Initializer />
      <NotificationProvider>
        <main className="scroll-area">
          <div className="pages-slider" style={{ transform: `translateX(-${PAGE_INDEX[state.activePage] * 25}%)` }}>
            <div className={`page ${state.activePage === 'home' ? 'active' : ''}`}>
              <HomePage
                {...zapret}
                logs={state.logs}
                logStart={state.logStart}
                onSelectConfig={actions.handleStrategyChange}
                isSelectorOpen={state.isSelectorOpen}
                setIsSelectorOpen={prefs.setIsSelectorOpen}
                loadConfigs={actions.loadIpsetConfigs}
                setIsConvertOpen={prefs.setIsConvertOpen}
                setIsIpsetModalOpen={prefs.setIsIpsetModalOpen}
                setIsHostsModalOpen={prefs.setIsHostsModalOpen}
                handleToggle={() => zapret.status === 'stopped' ? zapret.startProcess() : zapret.stopProcess()}
              />
            </div>

            <div className={`page ${state.activePage === 'settings' ? 'active' : ''}`}><SettingsPage /></div>
            <div className={`page ${state.activePage === 'editor' ? 'active' : ''}`}><EditorPage /></div>
            <div className={`page ${state.activePage === 'credits' ? 'active' : ''}`}><CreditsPage /></div>
          </div>
        </main>
      </NotificationProvider>

      <footer className="bottom-nav">
        {Object.keys(NAV_ICONS).map((id) => (
          <NavItem key={id} id={id} isActive={state.activePage === id} onClick={() => prefs.setActivePage(id)} onHover={prefs.setHoverText} />
        ))}
        <div className={`nav-tooltip ${state.hoverText ? 'visible' : ''}`}>{state.hoverText || state.lastText}</div>
      </footer>

      <StrategyModal isOpen={state.isSelectorOpen} onClose={() => prefs.setIsSelectorOpen(false)} configs={zapret.configs} stratName={zapret.stratName} onSelect={actions.handleStrategyChange} />
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
      <HostsModal isOpen={state.isHostsModalOpen} onClose={() => prefs.setIsHostsModalOpen(false)} />
      <LegacyModal isOpen={state.isLegacyOpen} />
    </div>
  );
}

export default App;