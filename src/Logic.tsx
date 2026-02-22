import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { notify } from './Notifications';
import { ZapretUtils } from "./ZapretUtils";
import { getVersion } from '@tauri-apps/api/app';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

export function log(text: string) {
    invoke('log', { text: text });
}

export function Logic() {
    const [activePage, setActivePage] = useState('home');
    const [isPinned, setIsPinned] = useState(() => {
        return localStorage.getItem('window_pinned') === 'true';
    });
    const [logs, setLogs] = useState<string[]>([]);
    const [hoverText, setHoverText] = useState<string | null>(null);
    const [lastText, setLastText] = useState('');
    const [isSelectorOpen, setIsSelectorOpen] = useState(false);
    const [isConvertOpen, setIsConvertOpen] = useState(false);
    const [isIpsetModalOpen, setIsIpsetModalOpen] = useState(false);
    const [isHostsModalOpen, setIsHostsModalOpen] = useState(false);
    const [isLegacyOpen, setIsLegacyOpen] = useState(false);
    const [customIpsetFiles, setCustomIpsetFiles] = useState<string[]>([]);
    const [ipsetView, setIpsetView] = useState<'main' | 'custom'>('main');
    const [hoveredDesc, setHoveredDesc] = useState<string | null>(null);
    const [updatableStrats, setUpdatableStrats] = useState<string[]>([]);
    const [isResolverOpen, setIsResolverOpen] = useState(false);
    const [isProxyModalOpen, setIsProxyModalOpen] = useState(false);
    const [isNewsModalOpen, setIsNewsModalOpen] = useState(false);
    const [isDeepLinkModalOpen, setIsDeepLinkModalOpen] = useState(false);
    const [deepLinkData, setDeepLinkData] = useState<{
        type: 'strategy' | 'ipset' | 'hostlist' | 'configure_zapret';
        payload: string;
        name?: string;
    } | null>(null);
    const logStart = useRef<HTMLDivElement>(null);

    const zapret = ZapretUtils();
    const handleHover = (text: string | null) => {
        setHoverText(text);
        if (text) {
            setLastText(text);
        }
    };
    const [updateAvailable, setUpdateAvailable] = useState<string | null>(null);
    const [isUpdateChecked, setIsUpdateChecked] = useState(false);

    const checkUpdate = async () => {
        try {
            const currentVersion = await getVersion();
            const url = "https://raw.githubusercontent.com/f1zzen/zust/main/version";
            const res = await fetch(`${url}?t=${Date.now()}`);
            const latestVersion = res.ok ? (await res.text()).trim() : null;

            if (latestVersion && latestVersion !== currentVersion) {
                setUpdateAvailable(latestVersion);
            }
        } catch (e) {
            log("Ошибка проверки версии: " + e);
        } finally {
            setIsUpdateChecked(true);
            setIsNewsModalOpen(true);
        }
    };

    useEffect(() => {
        let intervalId: number | null = null;

        const refreshSettings = async () => {
            const updated = await invoke<any>('load_settings');
            document.body.classList.toggle('no-animations', updated.animationDisabled);
        };

        const initPin = async () => {
            const savedPin = localStorage.getItem('window_pinned') === 'true';

            if (savedPin) {
                await appWindow.setAlwaysOnTop(true);
                setIsPinned(true);
            } else {
                const actuallyPinned = await appWindow.isAlwaysOnTop();
                setIsPinned(actuallyPinned);
            }
        };

        initPin();

        window.addEventListener('settings-changed', refreshSettings);


        const initialize = async () => {
            await invoke('sync_zapret_files');

            const hasLegacy = await invoke('check_legacy_folder');
            if (hasLegacy) {
                setIsLegacyOpen(true);
                await emit("app_ready");
                return;
            }

            await Promise.all([
                zapret.init(),
                invoke<string[]>('check_strategy_updates')
                    .then(list => setUpdatableStrats(list))
                    .catch(e => log("strategy check failed: " + e))
            ]);
            await emit("app_ready");
            checkUpdate();
            invoke<boolean>("check_winws_update")
                .then(wasUpdated => {
                    if (wasUpdated) log("zapret (winws.exe) обновлен до последней версии!");
                })
                .catch(() => { });
            log("Запуск обновления tls_max_ru");
            try {
                const result = await invoke<string>('update_tls_bin');
                log(result);
            } catch (e) {
                log("Ошибка при обновлении tls " + e);
            }
        };

        initialize();

        const handleConfigureZapret = (params: Record<string, string>) => {
            const { strategy, ipset_config, game_filter, restart } = params;

            setDeepLinkData({
                type: 'configure_zapret',
                payload: JSON.stringify({ strategy, ipset_config, game_filter, restart }),
                name: 'Глубокая настройка'
            });
            setIsDeepLinkModalOpen(true);
        };

        const handleAddAction = (params: Record<string, string>) => {
            const { url, type, name: paramName } = params;
            const validTypes = ['strategy', 'ipset', 'hostlist'];
            const incomingType = type?.toLowerCase();

            if (url && validTypes.includes(incomingType)) {
                let finalName: string;
                if (incomingType === 'hostlist') {
                    finalName = 'list-general.txt';
                } else {
                    const fileName = url.split('/').pop()?.split('.')[0] || 'imported_file';
                    finalName = paramName || fileName;
                }

                setDeepLinkData({
                    type: incomingType as any,
                    payload: url,
                    name: finalName
                });
                setIsDeepLinkModalOpen(true);
            }
        };

        const unlistenDeepLink = listen<{ action: string, params: Record<string, string> }>('zust-deeplink', async (event) => {
            const { action, params } = event.payload;

            switch (action) {
                case 'restore_zapret':
                    setDeepLinkData({
                        type: 'restore_zapret' as any,
                        payload: 'all_files',
                    });
                    setIsDeepLinkModalOpen(true);
                    break;
                case 'flush_dns':
                    try {
                        const result = await invoke<string>('flush_dns');
                        notify("Кэш ДНС успешно очищен", "success");
                        log(result);
                    } catch (e) {
                        notify("Ошибка cache-DNS", "error");
                        log(`cache-DNS err ${e}`);
                    }
                    break;
                case 'add':
                    handleAddAction(params);
                    break;

                case 'configure_zapret':
                    handleConfigureZapret(params);
                    break;

                case 'open':
                    if (params.page) setActivePage(params.page);
                    break;

                default:
                    log(`Команда ${action} не распознана`);
            }
        });

        const unlistenLog = listen<string>('log-event', (event) => {
            setLogs((prev) => [event.payload, ...prev.slice(0, 49)]);
        });

        const handleContextMenu = (e: MouseEvent) => e.preventDefault();
        window.addEventListener('contextmenu', handleContextMenu);

        return () => {
            if (intervalId) clearInterval(intervalId);
            unlistenDeepLink.then((f) => f());
            unlistenLog.then((f) => f());
            window.removeEventListener('contextmenu', handleContextMenu);
            window.removeEventListener('settings-changed', refreshSettings);
        };
    }, []);
    const actions = {
        changeIpsetMode: async (mode: any, fileName: string | null = null) => {
            const val = mode === 'Any' ? 'any' : mode === 'None' ? 'none' : (fileName || 'any');
            zapret.setSelectedIpset(val);
            localStorage.setItem("selected_ipset", val);
            if (zapret.status === 'running') await zapret.startProcess(undefined, val);
        },
        handlePickFiles: async () => {
            const selected = await open({ multiple: true, filters: [{ name: 'Batch', extensions: ['bat'] }] });
            if (selected) {
                try {
                    await invoke('convert_multiple_bats', { paths: Array.isArray(selected) ? selected : [selected] });
                    setIsConvertOpen(false);
                    const list = await invoke<string[]>('get_list_strategies');
                    zapret.setConfigs(list);
                    notify("Конвертация завершена!", "success");
                } catch (e) { notify("Ошибка при конвертации", "error"); }
            }
        },
        addIp: async (ip: string) => {
            const targetFile = (zapret.selectedIpset === 'any' || zapret.selectedIpset === 'none' || !zapret.selectedIpset)
                ? 'ipset-all.txt'
                : zapret.selectedIpset;

            try {
                await invoke('add_ip', { fileName: targetFile, ip });
                const message = `resolver - ip ${ip} -> ${targetFile}`;
                log(message);
                if (zapret.status === 'running') {
                    await zapret.stopProcess();
                    await zapret.startProcess();
                }
                notify(`Добавлено в ${targetFile}`, "success");
            } catch (e) {
                notify("Ошибка при записи IP", "error");
                log("ipset err " + e);
            }
        },
        handleStrategyChange: async (newName: string) => {
            setIsSelectorOpen(false);
            zapret.setSelectedConfig(newName);
            localStorage.setItem("selected_strategy", newName);
            await zapret.startProcess(newName);
        },
        togglePin: async () => {
            const nextValue = !isPinned;
            setIsPinned(nextValue);
            await appWindow.setAlwaysOnTop(nextValue);
            localStorage.setItem('window_pinned', String(nextValue));
        },
        loadIpsetConfigs: async () => {
            const files = await invoke<string[]>('get_custom_configs');
            setCustomIpsetFiles(files);
        }
    };

    return {
        state: {
            activePage, isPinned, logs, hoverText, lastText, isSelectorOpen, isConvertOpen, isIpsetModalOpen, isHostsModalOpen, customIpsetFiles, ipsetView, hoveredDesc, zapret, logStart, isLegacyOpen, updatableStrats, isResolverOpen, isProxyModalOpen, isNewsModalOpen, isDeepLinkModalOpen, updateAvailable, deepLinkData, isUpdateChecked
        },
        prefs: { setActivePage, setHoverText: handleHover, setLastText, setIsSelectorOpen, setIsConvertOpen, setIsIpsetModalOpen, setIsHostsModalOpen, setIpsetView, setHoveredDesc, setUpdatableStrats, setIsResolverOpen, setIsProxyModalOpen, setIsNewsModalOpen, setIsDeepLinkModalOpen, setDeepLinkData },
        actions
    };
}