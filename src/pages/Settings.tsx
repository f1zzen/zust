import { useState, useEffect } from 'react';
import { api, log, type Settings } from '@/lib/api';
import { notify } from '../Notifications'
import { Header } from '@/Buttons';
import type { Zapret } from '@/ZapretUtils';

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

const SECTIONS: { title: string; items: { key: keyof Settings; label: string; description: string; emoji: string }[] }[] = [
    {
        title: 'Основные',
        items: [
            { key: 'autoStart', label: 'Автозапуск', description: 'Запускать Zust (свёрнутым в трей) при входе в систему.', emoji: '🚀' },
            { key: 'notifications', label: 'Уведомления', description: 'Показывать уведомления об ошибках и успехах.', emoji: '🔔' },
            { key: 'minimizeToTray', label: 'Сворачивать в трей', description: 'Крестик прячет окно в трей. Если выключено — Zust закрывается (zapret продолжит работать).', emoji: '📥' },
        ],
    },
    {
        title: 'Zapret',
        items: [
            { key: 'gameFilter', label: 'GameFilter', description: 'Обход для игр и других сервисов на портах выше 1023 (TCP и UDP).', emoji: '🕹️' },
        ],
    },
    {
        title: 'TOR',
        items: [
            { key: 'refreshBridges', label: 'Обновлять мосты', description: 'Запрашивать новые мосты при каждом подключении к TOR.', emoji: '🌉' },
            { key: 'autoTor', label: 'TOR при запуске', description: 'Подключаться к TOR сразу после запуска Zust (включает автозапуск).', emoji: '🤖' },
        ],
    },
];

export const SettingsPage = ({ zapret }: { zapret: Zapret }) => {
    const [settings, setSettings] = useState<Settings | null>(null);

    const fetchData = async () => {
        try {
            setSettings(await api.loadSettings());
        } catch (e) {
            log("Ошибка загрузки настроек: " + e);
        }
    };

    useEffect(() => {
        fetchData();
        window.addEventListener('settings-changed', fetchData);
        return () => window.removeEventListener('settings-changed', fetchData);
    }, []);

    const toggle = async (key: keyof Settings) => {
        if (!settings) return;
        const value = !settings[key];
        const next: Settings = { ...settings, [key]: value };
        if (key === 'autoTor' && value) next.autoStart = true;
        setSettings(next);

        try {
            // автозапуск бэкенд применяет сам при сохранении
            await api.saveSettings(next);
            window.dispatchEvent(new Event('settings-changed'));

            if (next.autoStart !== settings.autoStart) {
                notify(next.autoStart ? "Автозапуск включён" : "Автозапуск выключен");
            }
            // GameFilter меняет аргументы zapret — перезапускаем, если он работает
            if (key === 'gameFilter' && zapret.status === 'running') {
                await zapret.start();
                notify("zapret перезапущен с новым GameFilter", "success");
            }
        } catch (err) {
            notify(`Ошибка сохранения: ${err}`, "error");
            log("Ошибка сохранения настроек: " + err);
            fetchData();
        }
    };

    if (!settings) return null;

    return (
        <div className="content">
            <Header title="Настройки" />
            <div className="credits-section">
                {SECTIONS.map((section, i) => (
                    <div key={section.title} className="contents">
                        <h2 className="section-title" style={i ? { marginTop: '20px' } : undefined}>{section.title}</h2>
                        {section.items.map((item) => (
                            <SettingItem
                                key={item.key}
                                label={item.label}
                                description={item.description}
                                emoji={item.emoji}
                                enabled={settings[item.key]}
                                onToggle={() => toggle(item.key)}
                            />
                        ))}
                    </div>
                ))}
            </div>
        </div>
    );
};
