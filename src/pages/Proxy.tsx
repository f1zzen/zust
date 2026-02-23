import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { log } from '../Logic';

export const ProxyPage = () => {
    const [status, setStatus] = useState<'stopped' | 'loading' | 'running'>('stopped');
    const [fillLevel, setFillLevel] = useState(0);
    const [lastLog, setLastLog] = useState('Система готова');
    const [mode, setMode] = useState<'vpn' | 'direct'>('direct');
    const [isPending, setIsPending] = useState(false);

    useEffect(() => {
        const checkInitialStatus = async () => {
            try {
                const isRunning = await invoke<boolean>('check_tor_status');
                if (isRunning) {
                    setStatus('running');
                    setFillLevel(100);
                }
            } catch (err) {
                log("tor status failed w err " + err);
            }
        };

        const unlistenProgress = listen<number>('tor-progress', (event) => {
            const progress = event.payload;
            setFillLevel(progress);
            if (progress === 100) setStatus('running');
        });

        const unlistenLog = listen<string>('log-event', (event) => {
            const rawLog = event.payload;
            if (rawLog.includes('[TOR]')) {
                let cleanLog = rawLog.replace('[TOR]', '').trim();
                if (cleanLog.includes("general SOCKS server failure")) return;
                const msg = cleanLog.replace(/^[A-Z][a-z]{2}\s+\d+\s+\d{2}:\d{2}:\d{2}\.\d{3}\s+\[\w+\]\s+/, '');
                setLastLog(msg);
            }
        });

        checkInitialStatus();

        return () => {
            unlistenProgress.then(f => f());
            unlistenLog.then(f => f());
        };
    }, []);

    const handleModeChange = async (newMode: 'vpn' | 'direct') => {
        setMode(newMode);
        if (status === 'running') {
            if (newMode === 'vpn') {
                await invoke('enable_system_proxy');
                setLastLog('Прокси работает в режиме ВПН*');
            } else {
                await invoke('disable_system_proxy');
                setLastLog('Системный прокси отключен');
            }
        }
    };

    const handleToggle = async () => {
        if (isPending) return;

        setIsPending(true);

        if (status === 'stopped') {
            setStatus('loading');
            setFillLevel(0);
            setLastLog('Инициализация...');

            try {
                await invoke('start_tor');
                if (mode === 'vpn') {
                    await invoke('enable_system_proxy');
                }
            } catch (err) {
                if (err !== 'interrupted') {
                    setLastLog(`Ошибка: ${err}`);
                    setStatus('stopped');
                    await invoke('stop_tor');
                }
            } finally {
                setIsPending(false);
            }
        } else {
            setStatus('stopped');
            setFillLevel(0);
            setLastLog('Остановка...');

            try {
                await invoke('stop_tor');
                setLastLog('Готов к работе');
            } finally {
                setTimeout(() => setIsPending(false), 1000);
            }
        }
    };

    return (
        <div className="content">
            <div className="strategy-header">
                <span className="strat-label">CONNECTION</span>
                <div className="strat-title-row">
                    <div className="strat-value">Туннель TOR</div>
                </div>
            </div>

            <div className="mode-toggle-container">
                <div className={`mode-slider ${mode}`}></div>
                <div className="mode-btn-wrapper">
                    <button
                        className={`mode-btn ${mode === 'vpn' ? 'active' : ''}`}
                        onClick={() => !isPending && handleModeChange('vpn')}
                    >
                        VPN*
                    </button>
                    <div className="mode-info-tooltip">
                        Помните, что ВПН* - это тот же прокси, но нацеленный на всю вашу систему.
                    </div>
                </div>
                <button
                    className={`mode-btn ${mode === 'direct' ? 'active' : ''}`}
                    onClick={() => handleModeChange('direct')}
                >
                    Прокси
                </button>
            </div>
            <div className="hero-container">
                <button
                    className={`hero-reactor ${status}`}
                    onClick={handleToggle}
                >
                    <div className="reactor-aura"></div>
                    <div className="reactor-core">
                        <div className="core-inner">
                            <div className={`status-icon stopped ${status === 'stopped' ? 'visible' : ''}`}>
                                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                            </div>

                            <div className={`status-icon loading ${status === 'loading' ? 'visible' : ''}`}>
                                <div className="spinner-arc"></div>
                            </div>

                            <div className={`status-icon running ${status === 'running' ? 'visible' : ''}`}>
                                <span className="sparkle-diamond">✦</span>
                            </div>
                        </div>
                    </div>

                    <div className="reactor-ring"></div>
                    <div className="reactor-ring ring-2"></div>
                </button>
            </div>

            <div className={`status-indicator ${status}`}>
                {status === 'stopped' && 'Выключен'}
                {status === 'loading' && `Подключение: ${fillLevel}%`}
                {status === 'running' && 'Подключение обеспечено.'}
            </div>

            <div className="action-buttons">
                <div className="separator"><span>Параметры</span></div>
                <div className="menu-button-mock hosts">
                    <div className="menu-button-content">
                        <div className="menu-button-icon">
                            <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#a855f7" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m10.586 5.414-5.172 5.172" /><path d="m18.586 13.414-5.172 5.172" /><path d="M6 12h12" /><circle cx="12" cy="20" r="2" /><circle cx="12" cy="4" r="2" /><circle cx="20" cy="12" r="2" /><circle cx="4" cy="12" r="2" /></svg>
                        </div>
                        <div className="menu-button-text">
                            <div className="menu-button-title">Процесс</div>
                            <div className="menu-button-subtitle">Состояние подключения к TOR</div>
                        </div>
                        <div className="menu-button-value" style={{ color: '#a855f7', fontSize: '18px', fontWeight: '500' }}>
                            {fillLevel}%
                        </div>
                    </div>
                </div>
                <div className="menu-button-mock proxy">
                    <div className="menu-button-content">
                        <div className="menu-button-icon">
                            <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" /></svg>
                        </div>
                        <div className="menu-button-text">
                            <div className="menu-button-title">Состояние подключения</div>
                            <div className="menu-button-subtitle" style={{
                                maxWidth: '220px',
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap'
                            }}>
                                {lastLog
                                    // [19.02 21:27:56]
                                    .replace(/^\[\d{2}\.\d{2}\s\d{2}:\d{2}:\d{2}\]\s*/, '')
                                    // Feb 19 21:29:02.000
                                    .replace(/^[A-Z][a-z]{2}\s+\d+\s+\d{2}:\d{2}:\d{2}\.\d{3}\s*/, '')
                                    // [notice], [warn]
                                    .replace(/^\[\w+\]\s*/, '')
                                    .trim()}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};