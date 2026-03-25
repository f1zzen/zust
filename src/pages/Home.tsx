import { ActionButton, MenuButton, SelectButton } from "../Buttons"

const ICONS = {
    hosts: <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><polyline points="13 2 13 9 20 9" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /><polyline points="10 9 9 9 8 9" /></svg>,
    proxy: <svg width="24px" height="24px" viewBox="0 0 192 192" xmlns="http://www.w3.org/2000/svg" fill="none"><path stroke="#3b82f6" strokeWidth="12" d="M23.073 88.132s65.458-26.782 88.16-36.212c8.702-3.772 38.215-15.843 38.215-15.843s13.621-5.28 12.486 7.544c-.379 5.281-3.406 23.764-6.433 43.756-4.54 28.291-9.459 59.221-9.459 59.221s-.756 8.676-7.188 10.185c-6.433 1.509-17.027-5.281-18.919-6.79-1.513-1.132-28.377-18.106-38.214-26.404-2.649-2.263-5.676-6.79.378-12.071 13.621-12.447 29.891-27.913 39.728-37.72 4.54-4.527 9.081-15.089-9.837-2.264-26.864 18.483-53.35 35.835-53.35 35.835s-6.053 3.772-17.404.377c-11.351-3.395-24.594-7.921-24.594-7.921s-9.08-5.659 6.433-11.693Z" /></svg>,
    ipset: <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 7v14" /><path d="M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z" /></svg>,
    convert: <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M20 12V8H6a2 2 0 0 1-2-2c0-1.1.9-2 2-2h12v4" /><path d="M4 6v12c0 1.1.9 2 2 2h14v-4" /><path d="M18 12a2 2 0 0 0-2 2c0 1.1.9 2 2 2h4v-4h-4z" /></svg>,
    resolver: <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#22c55e" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 7V6.7l-8.9-3.3a1 1 0 0 0-.8 0L3 6.7V7c0 .3.1.7.4.9l8.1 4.1c.3.2.7.2 1 0l8.1-4.1c.3-.2.4-.6.4-.9Z" /><path d="m3 13 8.1 4.1c.3.2.7.2 1 0L21 13" /><path d="m3 18 8.1 4.1c.3.2.7.2 1 0L21 18" /></svg>
};

const STATUS = {
    stopped: 'Выключен',
    loading: 'Загрузка...',
    running: 'Работает в фоновом режиме'
};

export const HomePage = ({
    status,
    stratName,
    handleToggle,
    setIsSelectorOpen,
    setIsConvertOpen,
    setIsIpsetModalOpen,
    setIsHostsModalOpen,
    setIsProxyModalOpen
}: any) => {
    const actionButtons = [
        { type: 'separator', label: 'Обходы' },
        {
            title: 'DNS Override',
            subtitle: 'Доступ к Gemini, СhatGPT, Spotify и т.п.',
            variant: 'hosts',
            icon: ICONS.hosts,
            onClick: () => setIsHostsModalOpen(true)
        },
        {
            title: 'Прокси для TG',
            subtitle: 'Прокси для общения в телеграмме.',
            variant: 'proxy',
            icon: ICONS.proxy,
            onClick: () => setIsProxyModalOpen(true)
        },
        { type: 'separator', label: 'запрет' },
        {
            title: 'IPSet конфиги',
            subtitle: 'Использование айпи-шников для обхода блокировок.',
            variant: 'ipset',
            icon: ICONS.ipset,
            onClick: () => setIsIpsetModalOpen(true)
        },
        {
            title: 'Конвертатор',
            subtitle: 'Конвертация стратегий из файлов типа .bat в .zapret',
            variant: 'convert',
            icon: ICONS.convert,
            onClick: () => setIsConvertOpen(true)
        }
    ];

    return (
        <div className="content">
            <div className="strategy-header">
                <span className="strat-label">Стратегия</span>
                <div className="strat-title-row">
                    <div className="strat-value">{stratName.replace('.zapret', '')}</div>
                    <SelectButton onClick={() => setIsSelectorOpen(true)} front="Сменить" back="Стратегию" />
                </div>
            </div>
            <ActionButton STATUS={STATUS} handleToggle={handleToggle} status={status} />

            <div className="action-buttons">
                {actionButtons.map((item, idx) => (
                    item.type === 'separator' ? (
                        <div key={`sep-${idx}`} className="separator"><span>{item.label}</span></div>
                    ) : (
                        <MenuButton
                            key={item.title}
                            title={item.title}
                            subtitle={item.subtitle}
                            icon={item.icon}
                            variant={item.variant}
                            onClick={item.onClick}
                        />
                    )
                ))}
            </div>
        </div>
    );
};