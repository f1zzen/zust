import { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModalAnimation } from "./useModalAnimation";
import { notify } from "../Notifications";
import { log } from "../Logic";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    state: any;
    data: {
        type: 'strategy' | 'ipset' | 'hostlist' | 'configure_zapret' | 'restore_zapret';
        payload: string;
        name?: string;
    } | null;
}

const TYPE_CONFIG = {
    strategy: { label: 'STRATEGY' },
    ipset: { label: 'IPSET' },
    hostlist: { label: 'LIST-GENERAL' },
    configure_zapret: { label: 'CONFIGURATION' },
    restore_zapret: { label: 'RECOVERY' }
};

const ConfigNode = ({ label, status, title, desc, icon, isError, isDisabled }: any) => (
    <div className={`v3-config-node ${isError ? 'is-danger' : 'is-primary'} ${isDisabled ? 'disabled' : ''}`}>
        <div className="node-meta">
            <span className="node-label">{label}</span>
            <span className={`node-status ${isError ? 'error' : 'active'}`}>{status}</span>
        </div>
        <div className="node-main">
            <div className="node-icon">{icon}</div>
            <div className="node-content">
                <div className="node-title">{title || '—'}</div>
                <div className="node-desc">{desc}</div>
            </div>
        </div>
    </div>
);

export const DeepLinkModal = ({ isOpen, onClose, state, data }: Props) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    const [loading, setLoading] = useState(false);
    const [missingResources, setMissingResources] = useState<string[]>([]);

    const configData = useMemo(() => {
        if (data?.type === 'configure_zapret' && data.payload) {
            try {
                return JSON.parse(data.payload);
            } catch (e) {
                log("Ошибка парсинга payload DeepLink");
                return null;
            }
        }
        return null;
    }, [data]);

    useEffect(() => {
        const checkResources = async () => {
            setMissingResources([]);
            if (data?.type === 'configure_zapret' && configData) {
                try {
                    const missing = await invoke<string[]>("check_resources_exist", {
                        strategy: configData.strategy || null,
                        ipset: configData.ipset_config || null
                    });
                    setMissingResources(missing);
                } catch (e) {
                    log(`Ошибка проверки ресурсов: ${e}`);
                }
            }
        };
        if (isOpen) checkResources();
    }, [isOpen, data, configData]);

    if (!shouldRender || !data) return null;

    const currentConfig = TYPE_CONFIG[data.type] || { label: 'UNKNOWN' };

    const handleConfirm = async () => {
        if (loading || (data.type === 'configure_zapret' && missingResources.length > 0)) return;
        setLoading(true);

        try {
            if (data.type === 'configure_zapret' && configData) {
                const fullStrategyName = configData.strategy.endsWith('.zapret') ? configData.strategy : `${configData.strategy}.zapret`;
                let fullIpsetName = configData.ipset_config;

                if (fullIpsetName && !['any', 'none'].includes(fullIpsetName.toLowerCase()) && !fullIpsetName.endsWith('.txt')) {
                    fullIpsetName = `${fullIpsetName}.txt`;
                }

                const isGameFilter = configData.game_filter === 'true';
                const currentSettings = await invoke<any>('load_settings');

                await invoke('save_settings', { settings: { ...currentSettings, gameFilter: isGameFilter } });
                await invoke('game_filter_toggle', { enabled: isGameFilter });

                localStorage.setItem("selected_strategy", fullStrategyName);
                state.zapret.setSelectedConfig(fullStrategyName);

                if (fullIpsetName) {
                    localStorage.setItem("selected_ipset", fullIpsetName);
                    state.zapret.setSelectedIpset(fullIpsetName);
                }

                const newList = await invoke<string[]>('get_list_strategies');
                state.zapret.setConfigs(newList);
                await state.zapret.startProcess(fullStrategyName, fullIpsetName);

                window.dispatchEvent(new Event('settings-changed'));
                notify("Конфигурация применена и запущена", "success");

            } else if (data.type === 'restore_zapret') {
                if (state.zapret.status === 'running') await state.zapret.stopProcess();
                await invoke('restore_zapret_files');
                localStorage.removeItem("selected_strategy");
                localStorage.removeItem("selected_ipset");
                notify("Система восстановлена", "success");
                setTimeout(() => window.location.reload(), 1000);
            }

            onClose();
        } catch (e) {
            log(`DeepLink Error: ${e}`);
            notify("Ошибка выполнения", "error");
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className={`modal-overlay ${isAnimatingOut ? 'closing' : ''}`} onClick={onClose}>
            <div className="modal-content hosts-modal" onClick={e => e.stopPropagation()}>
                <div className="v2-header">
                    <div className="modal-title-row">
                        <h3>
                            {data.type === 'configure_zapret' ? 'Настройка системы' :
                                data.type === 'restore_zapret' ? 'Восстановление' : 'Импорт объекта'}
                        </h3>
                        <span className="hosts-badge" style={{ background: 'rgba(168, 85, 247, 0.15)', color: '#a855f7' }}>
                            {currentConfig.label}
                        </span>
                    </div>
                </div>

                <div className="modal-body">
                    <div className="v3-info-grid" style={{ padding: '24px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
                        <div style={{ fontSize: '10px', color: '#666', fontWeight: '900', letterSpacing: '1px' }}>
                            {data.type === 'configure_zapret' ? 'ПАРАМЕТРЫ КОНФИГУРАЦИИ' :
                                data.type === 'restore_zapret' ? 'ДЕЙСТВИЕ ВОССТАНОВЛЕНИЯ' : 'ДЕТАЛИ ИМПОРТА'}
                        </div>

                        {data.type === 'configure_zapret' && configData && (
                            <div className={`v3-config-container ${missingResources.length > 0 ? 'has-critical-errors' : ''}`}>
                                <ConfigNode
                                    label="СТРАТЕГИЯ"
                                    status={missingResources.includes('strategy') ? 'ОТСУТСТВУЕТ' : 'ПРИСУТСТВУЕТ'}
                                    isError={missingResources.includes('strategy')}
                                    title={configData.strategy}
                                    desc="Стратегия для обхода блокировок."
                                    icon={<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" /></svg>}
                                />
                                <ConfigNode
                                    label="IPSET КОНФИГ"
                                    status={missingResources.includes('ipset') ? 'ОТСУТСТВУЕТ' : 'ПРИСУТСТВУЕТ'}
                                    isError={missingResources.includes('ipset')}
                                    title={configData.ipset_config}
                                    desc="Список IP для обхода блокировок."
                                    icon={<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M21 5H3M15 12H3M17 19H3" /></svg>}
                                />
                                <ConfigNode
                                    label="ИГРОВОЙ ФИЛЬТР"
                                    status={configData.game_filter === 'true' ? 'ВКЛ.' : 'ВЫКЛ.'}
                                    isDisabled={configData.game_filter !== 'true'}
                                    title="Игровой фильтр"
                                    desc="Специальный фильтр для игр."
                                    icon={<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>}
                                />
                            </div>
                        )}

                        {data.type === 'restore_zapret' && (
                            <div className="v3-restore-wrapper">
                                <div className="v3-restore-content">
                                    <div className="restore-icon-container">
                                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" width="40">
                                            <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" strokeLinecap="round" strokeLinejoin="round" />
                                            <path d="M3 3v5h5" strokeLinecap="round" strokeLinejoin="round" />
                                        </svg>
                                    </div>
                                    <div className="restore-text-block">
                                        <div className="restore-system-label">ОПАСНАЯ_ЗОНА</div>
                                        <h4 className="restore-main-title">Полный сброс параметров</h4>
                                        <p className="restore-description">Удалит стратегии, IPSET конфиги и сбросит систему до заводских настроек.</p>
                                    </div>
                                </div>
                            </div>
                        )}

                        {!['configure_zapret', 'restore_zapret'].includes(data.type) && (
                            <div className="v3-import-container">
                                <div className="v3-object-card">
                                    <div className="object-info">
                                        <div className="object-label">ФАЙЛ ДЛЯ ИЗМЕНЕНИЯ</div>
                                        <div className="object-name">{data.name || 'External_Resource'}</div>
                                    </div>
                                    <div className="object-icon">
                                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" /></svg>
                                    </div>
                                </div>
                                <div className="v3-source-monitor">
                                    <div className="monitor-header">
                                        <span className="monitor-title">ИСТОЧНИК</span>
                                    </div>
                                    <div className="monitor-viewport">
                                        <div className="scan-line"></div>
                                        <code className="source-url" style={{ wordBreak: 'break-all', fontSize: '11px' }}>
                                            {data.payload}
                                        </code>
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>

                    {data.type === 'configure_zapret' && (
                        <div style={{
                            margin: '0 24px 24px', display: 'flex', gap: '12px', padding: '16px',
                            background: missingResources.length > 0 ? 'rgba(239, 68, 68, 0.1)' : 'rgba(168, 85, 247, 0.08)',
                            borderRadius: '12px', border: `1px solid ${missingResources.length > 0 ? 'rgba(239, 68, 68, 0.2)' : 'rgba(168, 85, 247, 0.2)'}`
                        }}>
                            <span style={{ fontSize: '12px', color: '#bbb' }}>
                                {missingResources.length > 0 ? "Файлы не найдены! Вы установили всё необходимое? (￣ ￣|||)" : "Все компоненты найдены. Можно запускать."}
                            </span>
                        </div>
                    )}
                </div>

                <div className="modal-footer">
                    <button className="close-modal-btn" onClick={onClose} disabled={loading}>Отмена</button>
                    <button
                        className={`save-modal-btn ${missingResources.length > 0 ? 'btn-disabled' : ''}`}
                        onClick={handleConfirm}
                        disabled={loading || (data.type === 'configure_zapret' && missingResources.length > 0)}
                        style={{ background: data.type === 'restore_zapret' ? '#ef4444' : undefined }}
                    >
                        {loading ? "..." : data.type === 'restore_zapret' ? "Сбросить всё" : "Выполнить"}
                    </button>
                </div>
            </div>
        </div>
    );
};