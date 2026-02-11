import { useEffect, useState, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { notify } from './Notifications';
import { ZapretUtils } from "./ZapretUtils";
import { getVersion } from '@tauri-apps/api/app';

const appWindow = getCurrentWindow();

export function log(text: string) {
    invoke('log', { text: text });
}

export function Logic() {
    const [activePage, setActivePage] = useState('home');
    const [isPinned, setIsPinned] = useState(false);
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
    const logStart = useRef<HTMLDivElement>(null);

    const zapret = ZapretUtils();
    const handleHover = (text: string | null) => {
        setHoverText(text);
        if (text) {
            setLastText(text);
        }
    };
    const [updateAvailable, setUpdateAvailable] = useState<string | null>(null);

    const checkUpdate = async () => {
        try {
            const currentVersion = await getVersion();
            const url = "https://raw.githubusercontent.com/f1zzen/zust/main/version";
            const res = await fetch(`${url}?t=${Date.now()}`);
            if (!res.ok) return;
            const latestVersion = (await res.text()).trim();
            log(`Сравнение версий: Локальная [${currentVersion}] vs GitHub [${latestVersion}]`);

            if (latestVersion !== currentVersion) {
                log("Доступна новая версия: " + latestVersion);
                setUpdateAvailable(latestVersion);
            }
        } catch (e) {
            log("Ошибка авто-апдейта: " + e);
        }
    };
    useEffect(() => {
        setTimeout(() => { appWindow.show() }, 200);

        let intervalId: number | null = null;

        const initialize = async () => {
            await invoke('sync_zapret_files');
            checkUpdate();
            const hasLegacy = await invoke('check_legacy_folder');
            if (hasLegacy) {
                setIsLegacyOpen(true);
                return;
            }
            invoke<string[]>('check_strategy_updates')
                .then(list => setUpdatableStrats(list))
                .catch(e => log("update check failed " + e));

            await zapret.init();

            intervalId = window.setInterval(() => {
                zapret.checkZapret();
            }, 5000);
        };

        initialize();

        const unlistenLog = listen<string>('log-event', (event) => {
            setLogs((prev) => [event.payload, ...prev.slice(0, 49)]);
        });

        const handleContextMenu = (e: MouseEvent) => e.preventDefault();
        window.addEventListener('contextmenu', handleContextMenu);

        return () => {
            if (intervalId) clearInterval(intervalId);
            unlistenLog.then((f) => f());
            window.removeEventListener('contextmenu', handleContextMenu);
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
            setIsPinned(!isPinned);
            await appWindow.setAlwaysOnTop(!isPinned);
        },
        loadIpsetConfigs: async () => {
            const files = await invoke<string[]>('get_custom_configs');
            setCustomIpsetFiles(files);
        }
    };

    return {
        state: { activePage, isPinned, logs, hoverText, lastText, isSelectorOpen, isConvertOpen, isIpsetModalOpen, isHostsModalOpen, customIpsetFiles, ipsetView, hoveredDesc, zapret, logStart, isLegacyOpen, updatableStrats, isResolverOpen, isProxyModalOpen, isNewsModalOpen, updateAvailable },
        prefs: { setActivePage, setHoverText: handleHover, setLastText, setIsSelectorOpen, setIsConvertOpen, setIsIpsetModalOpen, setIsHostsModalOpen, setIpsetView, setHoveredDesc, setUpdatableStrats, setIsResolverOpen, setIsProxyModalOpen, setIsNewsModalOpen },
        actions
    };
}