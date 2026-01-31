import { useState, useEffect, useRef } from 'react';
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
    const [settings, setSettings] = useState({
        notifications: true,
        minimizeToTray: true,
        animationDisabled: false,
        devTools: false,
        gameFilter: false
    });

    const cooldown = useRef<ReturnType<typeof setTimeout> | null>(null);

    useEffect(() => {
        invoke('load_settings')
            .then((res: any) => setSettings(res))
            .catch(console.error);
    }, []);


    const toggle = async (key: keyof typeof settings) => {
        const newValue = !settings[key];
        const newSettings = { ...settings, [key]: newValue };
        setSettings(newSettings);
        if (cooldown.current) {
            clearTimeout(cooldown.current)
        }
        try {
            await invoke('save_settings', { settings: newSettings });
            switch (key) {
                case 'animationDisabled':
                    document.body.classList.toggle('no-animations', newValue);
                    if (newValue) {
                        notify("Анимации включены!", "success")
                    } else {
                        notify("Анимации выключены!", "success")
                    }
                    break;
                case 'gameFilter':
                    await invoke('game_filter_toggle', { enabled: newValue });
                    notify("Перезапускаю сборку..");
                    log(`gameFilter ${newValue ? 'выключен' : 'выключен'}`);
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
                case 'notifications':
                    window.dispatchEvent(new Event('settings-updated'));
                    break;
            }
        } catch (err) {
            notify("Случилась непредвиденная ошибка.", "error")
            log("" + err)
        }
    };
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
                    description='"Переключение режима обхода для игр (и других сервисов, использующих UDP и TCP на портах выше 1023)." - flowseal'
                    emoji="🕹️"
                    enabled={settings.gameFilter}
                    onToggle={() => toggle('gameFilter')}
                />
            </div>
        </div>
    );
};