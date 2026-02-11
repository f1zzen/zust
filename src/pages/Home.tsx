export const HomePage = ({
    status,
    stratName,
    handleToggle,
    logs,
    logStart,
    setIsSelectorOpen,
    setIsConvertOpen,
    setIsIpsetModalOpen,
    setIsHostsModalOpen,
    setIsResolverOpen,
    setIsProxyModalOpen
}: any) => (
    <div className="content">
        <div className="strategy-header">
            <span className="strat-label">STRATEGY</span>
            <div className="strat-title-row">
                <div className="strat-value">{stratName.replace('.zapret', '')}</div>
                <button className="select-strat-btn" onClick={() => setIsSelectorOpen(true)}>Сменить</button>
            </div>
        </div>

        <button className={`hero-card ${status}`} onClick={handleToggle} disabled={status === 'loading'}>
            <div className="inner-glow"></div>
            <div className="visual-box">
                <div className={`icon-wrapper stopped ${status === 'stopped' ? 'visible' : ''}`}>X</div>
                <div className={`icon-wrapper loading ${status === 'loading' ? 'visible' : ''}`}>↺</div>
                <div className={`icon-wrapper running ${status === 'running' ? 'visible' : ''}`}>✦</div>
            </div>
        </button>

        <div className={`status-indicator ${status}`}>
            {status === 'stopped' && 'Выключен'}
            {status === 'loading' && 'Загрузка...'}
            {status === 'running' && 'Работает в фоновом режиме'}
        </div>

        <div className="action-buttons">
            <div className="separator"><span>Обходы</span></div>
            <button className="wide-btn hosts-variant" onClick={() => setIsHostsModalOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><polyline points="13 2 13 9 20 9" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /><polyline points="10 9 9 9 8 9" /></svg>
                    </span>
                    <div className="btn-text">
                        <span className="btn-title">Файлы hosts</span>
                        <span className="btn-subtitle">Доступ к Gemini, СhatGPT, Spotify и т.п.</span>
                    </div>
                </div>
                <span className="btn-arrow">→</span>
            </button>
            <button className="wide-btn proxy-variant" onClick={() => setIsProxyModalOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">
                        <svg width="24px" height="24px" viewBox="0 0 192 192" xmlns="http://www.w3.org/2000/svg" fill="none"><path stroke="#3b82f6" stroke-width="12" d="M23.073 88.132s65.458-26.782 88.16-36.212c8.702-3.772 38.215-15.843 38.215-15.843s13.621-5.28 12.486 7.544c-.379 5.281-3.406 23.764-6.433 43.756-4.54 28.291-9.459 59.221-9.459 59.221s-.756 8.676-7.188 10.185c-6.433 1.509-17.027-5.281-18.919-6.79-1.513-1.132-28.377-18.106-38.214-26.404-2.649-2.263-5.676-6.79.378-12.071 13.621-12.447 29.891-27.913 39.728-37.72 4.54-4.527 9.081-15.089-9.837-2.264-26.864 18.483-53.35 35.835-53.35 35.835s-6.053 3.772-17.404.377c-11.351-3.395-24.594-7.921-24.594-7.921s-9.08-5.659 6.433-11.693Z" /></svg>
                    </span>
                    <div className="btn-text">
                        <span className="btn-title">Прокси для TG</span>
                        <span className="btn-subtitle">Прокси для общения в телеграмме.</span>
                    </div>
                </div>
                <span className="btn-arrow">→</span>
            </button>

            <div className="separator"><span>запрет</span></div>
            <button className="wide-btn ipset-variant" onClick={() => setIsIpsetModalOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#a855f7" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 7v14" /><path d="M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z" /></svg>
                    </span>
                    <div className="btn-text">
                        <span className="btn-title">IPSet конфиги</span>
                        <span className="btn-subtitle">Использование айпи-шников для обхода блокировок.</span>
                    </div>
                </div>
                <span className="btn-arrow">→</span>
            </button>
            <button className="wide-btn convert-variant" onClick={() => setIsConvertOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M20 12V8H6a2 2 0 0 1-2-2c0-1.1.9-2 2-2h12v4" /><path d="M4 6v12c0 1.1.9 2 2 2h14v-4" /><path d="M18 12a2 2 0 0 0-2 2c0 1.1.9 2 2 2h4v-4h-4z" /></svg>
                    </span>
                    <div className="btn-text">
                        <span className="btn-title">Конвертатор</span>
                        <span className="btn-subtitle">Конвертация стратегий из файлов типа .bat в .zapret</span>
                    </div>
                </div>
                <span className="btn-arrow">→</span>
            </button>
            <button className="wide-btn resolver-variant" onClick={() => setIsResolverOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#22c55e" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 7V6.7l-8.9-3.3a1 1 0 0 0-.8 0L3 6.7V7c0 .3.1.7.4.9l8.1 4.1c.3.2.7.2 1 0l8.1-4.1c.3-.2.4-.6.4-.9Z" /><path d="m3 13 8.1 4.1c.3.2.7.2 1 0L21 13" /><path d="m3 18 8.1 4.1c.3.2.7.2 1 0L21 18" /></svg>
                    </span>
                    <div className="btn-text">
                        <span className="btn-title">Резолвер | Minecraft</span>
                        <span className="btn-subtitle">Быстрое добавление серверов по буквенному айпи.</span>
                    </div>
                </div>
                <span className="btn-arrow">→</span>
            </button>
        </div>

        <div className={`log-container ${logs.length > 0 ? 'visible' : 'hidden'}`}>
            {[...logs].reverse().map((log: string, i: number) => (
                <div key={i} className="log-line">{log}</div>
            ))}
            <div ref={logStart} />
        </div>
    </div>
);