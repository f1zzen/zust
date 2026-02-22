import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from './Notifications';

export type Status = 'stopped' | 'running' | 'loading';
const DEFAULT_IPSET = "none";

export function ZapretUtils() {
    const [status, setStatus] = useState<Status>('loading');
    const [stratName, setStratName] = useState('Проверяем...');
    const [configs, setConfigs] = useState<string[]>([]);
    const [selectedConfig, setSelectedConfig] = useState("");
    const [selectedIpset, setSelectedIpset] = useState<string | null>(null);

    useEffect(() => {
        if (status === 'loading') return;

        const interval = setInterval(async () => {
            const isActive = await invoke<boolean>('is_active');

            if (!isActive && status === 'running') {
                setStatus('stopped');
                setStratName("Отсутствует");
            }
            else if (isActive && status === 'stopped') {
                const name = await invoke<string>('get_strategy');
                setStratName(name);
                setStatus('running');
            }
        }, 3000);

        return () => clearInterval(interval);
    }, [status]);

    const checkZapret = async () => {
        const name = await invoke<string>('get_strategy');
        setStratName(name);
        setStatus(name !== "Отсутствует" ? 'running' : 'stopped');
    };

    const stopProcess = async () => {
        setStatus('loading');
        try {
            await invoke('stop_service');
            await checkZapret();
        } catch {
            notify("Ошибка при остановке", "error");
            setStatus('stopped');
        }
    };

    const startProcess = async (forcedConfig?: string, forcedIpset?: string) => {
        setStatus('loading');
        const configName = forcedConfig || selectedConfig || localStorage.getItem("selected_strategy") || "";
        const ipsetConfig = forcedIpset || selectedIpset || localStorage.getItem("selected_ipset") || DEFAULT_IPSET;

        let currentConfigs = configs;
        if (configName && !currentConfigs.includes(configName)) {
            currentConfigs = await invoke<string[]>('get_list_strategies');
            setConfigs(currentConfigs);
        }

        const idx = currentConfigs.indexOf(configName) + 1;
        if (idx <= 0) {
            console.error("Config not found:", configName);
            setStatus('stopped');
            return;
        }

        try {
            await invoke('start_service', { args: { index: idx, ipset_config: ipsetConfig } });
            setTimeout(checkZapret, 500);
        } catch (e) {
            setStatus('stopped');
        }
    };

    const init = async () => {
        const list = await invoke<string[]>('get_list_strategies');
        setConfigs(list);
        const currentRunning = await invoke<string>('get_strategy');
        const isActive = await invoke<boolean>('is_active');
        const savedIpset = localStorage.getItem("selected_ipset") || DEFAULT_IPSET;
        setSelectedIpset(savedIpset);
        if (isActive && currentRunning !== "Отсутствует") {
            setStratName(currentRunning);
            setSelectedConfig(currentRunning);
            setStatus('running');
        } else {
            const savedConfig = localStorage.getItem("selected_strategy");
            if (savedConfig && list.includes(savedConfig)) {
                setSelectedConfig(savedConfig);
            }
            setStratName("Отсутствует");
            setStatus('stopped');
        }

        return { list, currentRunning, isActive };
    };

    return {
        status, stratName, configs, selectedConfig, selectedIpset,
        setConfigs, setSelectedConfig, setSelectedIpset,
        startProcess, stopProcess, checkZapret, init
    };
}