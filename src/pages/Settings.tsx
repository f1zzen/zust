import { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { log } from '../Logic'
import { notify } from '../Notifications'

interface SettingItemProps {
    label: string;
    description: string;
    emoji: string;
    enabled: boolean;
    onToggle: () => void;
}

interface ISettings {
    notifications: boolean;
    minimizeToTray: boolean;
    animationDisabled: boolean;
    gameFilter: boolean;
    refreshBridges: boolean;
    [key: string]: any;
}

const SettingItem = ({ label, description, emoji, enabled, onToggle }: SettingItemProps) => (
    <div className={`settings-card ${enabled ? 'active' : ''}`} onClick={onToggle}>
        <div className="settings-info">
            <span className="settings-emoji">{emoji}</span>
            <div className="settings-text">
                <span className="settings-label">{label}</span>
                <span className="settings-desc">{description}</span>
            </div>
        </div>
        <div className={`settings-toggle ${enabled ? 'on' : ''}`}>
            <div className="toggle-handle"></div>
        </div>
    </div>
);

export const SettingsPage = () => {
    const [settings, setSettings] = useState<ISettings | null>(null);

    const fetchData = async () => {
        try {
            const res = await invoke<ISettings>('load_settings');
            setSettings(res);
        } catch (e) {
            log("Ошибка загрузки настроек: " + e);
        }
    };

    useEffect(() => {
        fetchData();
        window.addEventListener('settings-changed', fetchData);
        return () => window.removeEventListener('settings-changed', fetchData);
    }, []);

    const toggle = async (key: string) => {
        if (!settings) return;

        const newValue = !settings[key];
        const newSettings = { ...settings, [key]: newValue };
        setSettings(newSettings);

        try {
            await invoke('save_settings', { settings: newSettings });
            window.dispatchEvent(new Event('settings-changed'));
            switch (key) {
                case 'animationDisabled':
                    notify(newValue ? "Анимации выключены" : "Анимации включены", "success");
                    break;

                case 'gameFilter':
                    await invoke('game_filter_toggle', { enabled: newValue });
                    notify("Обновление конфигурации...");

                    const currentStrat = await invoke<string>('get_strategy');
                    if (currentStrat !== "None") {
                        await invoke('stop_service');
                        const list = await invoke<string[]>('get_list_strategies');
                        const idx = list.indexOf(currentStrat) + 1;
                        if (idx > 0) {
                            await invoke('start_service', {
                                args: {
                                    index: idx,
                                    ipsetConfig: localStorage.getItem('selected_ipset') || "ipset-all.txt"
                                }
                            });
                        }
                    }
                    notify("Сборка перезапущена!", "success");
                    break;
            }
        } catch (err) {
            notify("Ошибка сохранения", "error");
            fetchData();
        }
    };

    if (!settings) return null;

    return (
        <div className="content">
            <div className="strategy-header">
                <span className="strat-label">CONFIGURATION</span>
                <div className="strat-title-row">
                    <span className="strat-value">Настройки Zust</span>
                </div>
            </div>

            <div className="credits-section">
                <h2 className="section-title">Основные</h2>
                <SettingItem
                    label="Уведомления"
                    description="Уведомлять о ошибках и успехах."
                    emoji="🔔"
                    enabled={settings.notifications}
                    onToggle={() => toggle('notifications')}
                />

                <h2 className="section-title" style={{ marginTop: '20px' }}>Интерфейс</h2>
                <SettingItem
                    label="Сворачивать в трей"
                    description="Закрытие окна не выключает службу"
                    emoji="📥"
                    enabled={settings.minimizeToTray}
                    onToggle={() => toggle('minimizeToTray')}
                />
                <SettingItem
                    label="Отключение анимаций"
                    description="Используйте если ZUST нагружает ваш компьютер интерфейсом."
                    emoji="⚡"
                    enabled={settings.animationDisabled}
                    onToggle={() => toggle('animationDisabled')}
                />

                <h2 className="section-title" style={{ marginTop: '20px' }}>Zapret</h2>
                <SettingItem
                    label="GameFilter"
                    description='"Переключение режима обхода для игр (и других сервисов, использующих UDP и TCP на портах выше 1023)."'
                    emoji="🕹️"
                    enabled={settings.gameFilter}
                    onToggle={() => toggle('gameFilter')}
                />
                <h2 className="section-title" style={{ marginTop: '20px' }}>VPN*</h2>
                <SettingItem
                    label="Обновление мостов"
                    description='Поиск новых мостов для файла torrc при каждом новом подключении к сети TOR.'
                    emoji="🌉"
                    enabled={settings.refreshBridges}
                    onToggle={() => toggle('refreshBridges')}
                />
            </div>
        </div>
    );
};