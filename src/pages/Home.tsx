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
            <button className="wide-btn" onClick={() => setIsHostsModalOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">📋</span>
                    <div className="btn-text">
                        <span className="btn-title">Файлы hosts</span>
                        <span className="btn-subtitle">Доступ к Gemini, СhatGPT, Spotify и т.п!</span>
                    </div>
                </div>
                <span className="btn-arrow">→</span>
            </button>
            <div className="separator">
                <span>Zapret</span>
            </div>
            <button className="wide-btn" onClick={() => setIsIpsetModalOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">🌐</span>
                    <div className="btn-text">
                        <span className="btn-title">IPSet конфиги</span>
                        <span className="btn-subtitle">Any / Loaded / None</span>
                    </div>
                </div>
                <span className="btn-arrow">→</span>
            </button>

            <button className="wide-btn" onClick={() => setIsConvertOpen(true)}>
                <div className="btn-content">
                    <span className="btn-icon">🛠️</span>
                    <div className="btn-text">
                        <span className="btn-title">Конвертатор</span>
                        <span className="btn-subtitle">Из .bat в .zapret</span>
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