import { useEffect, useState, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { notify } from './Notifications';
import { ZapretUtils } from "./ZapretUtils";

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
    const [customIpsetFiles, setCustomIpsetFiles] = useState<string[]>([]);
    const [ipsetView, setIpsetView] = useState<'main' | 'custom'>('main');
    const [hoveredDesc, setHoveredDesc] = useState<string | null>(null);
    const logStart = useRef<HTMLDivElement>(null);

    const zapret = ZapretUtils();
    const handleHover = (text: string | null) => {
        setHoverText(text);
        if (text) {
            setLastText(text);
        }
    };
    useEffect(() => {
        setTimeout(() => { appWindow.show() }, 200);
        const initialize = async () => {
            await zapret.init();
            const interval = setInterval(() => {
                zapret.checkZapret();
            }, 5000);

            return interval;
        };
        const promise = initialize();
        const unlistenLog = listen<string>('log-event', (event) => {
            setLogs((prev) => [event.payload, ...prev.slice(0, 49)]);
        });

        const handleContextMenu = (e: MouseEvent) => {
            e.preventDefault();
        };
        window.addEventListener('contextmenu', handleContextMenu);

        return () => {
            promise.then(id => clearInterval(id))
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
        state: { activePage, isPinned, logs, hoverText, lastText, isSelectorOpen, isConvertOpen, isIpsetModalOpen, customIpsetFiles, ipsetView, hoveredDesc, zapret, logStart },
        prefs: { setActivePage, setHoverText: handleHover, setLastText, setIsSelectorOpen, setIsConvertOpen, setIsIpsetModalOpen, setIpsetView, setHoveredDesc },
        actions
    };
}