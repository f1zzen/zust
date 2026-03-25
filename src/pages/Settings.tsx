import { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { log } from '../Logic'
import { notify } from '../Notifications'
import { Header } from '@/Buttons';

interface SettingItemProps {
    label: string;
    description: string;
    emoji: string;
    enabled: boolean;
    onToggle: () => void;
}

interface ISettings {
    notifications: boolean;
    autoStart: boolean;
    minimizeToTray: boolean;
    animationDisabled: boolean;
    gameFilter: boolean;
    refreshBridges: boolean;
    autoTor: boolean;
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

        if (key === 'autoTor' && newValue === true) {
            newSettings.autoStart = true;
        }

        setSettings(newSettings);

        try {
            await invoke('save_settings', { settings: newSettings });
            window.dispatchEvent(new Event('settings-changed'));

            if (key === 'autoStart' || (key === 'autoTor' && newValue === true)) {
                await invoke('manage_autostart', { enabled: newSettings.autoStart });
                notify(newSettings.autoStart ? "Автозагрузка включена" : "Автозагрузка выключена");
            }

            switch (key) {
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
            log("" + err);
            fetchData();
        }
    };

    if (!settings) return null;

    return (
        <div className="content">
            <Header title="Настройки" />
            <div className="credits-section">
                <h2 className="section-title">Основные</h2>
                <SettingItem
                    label="Авто-запуск"
                    description="Автоматический запуск при включении компьютера."
                    emoji="🚀"
                    enabled={settings.autoStart}
                    onToggle={() => toggle('autoStart')}
                />
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
                <h2 className="section-title" style={{ marginTop: '20px' }}>Zapret</h2>
                <SettingItem
                    label="GameFilter"
                    description='"Переключение режима обхода для игр (и других сервисов, использующих UDP и TCP на портах выше 1023)."'
                    emoji="🕹️"
                    enabled={settings.gameFilter}
                    onToggle={() => toggle('gameFilter')}
                />
                <h2 className="section-title" style={{ marginTop: '20px' }}>TOR</h2>
                <SettingItem
                    label="Обновление мостов"
                    description='Поиск новых мостов для файла torrc при каждом новом подключении к сети TOR.'
                    emoji="🌉"
                    enabled={settings.refreshBridges}
                    onToggle={() => toggle('refreshBridges')}
                />
                <SettingItem
                    label="Автоматическое включение"
                    description="Автоматически запускать TOR при включении компьютера."
                    emoji="🤖"
                    enabled={settings.autoTor}
                    onToggle={() => toggle('autoTor')}
                />
            </div>
        </div>
    );
};