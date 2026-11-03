import { Modal, CloseButton } from "./Modal";
import { useEffect, useState, useMemo } from "react";
import { notify } from "../Notifications";
import { api, log } from "@/lib/api";
import type { Zapret } from "@/ZapretUtils";
import {
    ModalHeader,
    ModalContent,
    ModalFooter,
    SelectButton,
    InfoHeader,
    WarningHeader
} from "@/Buttons";
import { cn } from "@/lib/utils";

export interface DeepLinkData {
    type: 'strategy' | 'ipset' | 'hostlist' | 'configure_zapret' | 'restore_zapret';
    payload: string;
    name?: string;
}

interface Props {
    isOpen: boolean;
    onClose: () => void;
    zapret: Zapret;
    data: DeepLinkData | null;
}

const TYPE_CONFIG = {
    strategy: { label: 'STRATEGY', icon: '🎯' },
    ipset: { label: 'IPSET', icon: '📋' },
    hostlist: { label: 'LIST-GENERAL', icon: '🌐' },
    configure_zapret: { label: 'CONFIGURATION', icon: '⚙️' },
    restore_zapret: { label: 'RECOVERY', icon: '⚠️' }
};

const ConfigNode = ({ label, status, title, desc, icon, isError, isDisabled }: any) => (
    <div className={cn(
        "flex items-center gap-4 rounded-xl border p-3.5 transition-all bg-white/[0.03]",
        isError ? "border-red-500/30 bg-red-500/5" : "border-white/5",
        isDisabled && "opacity-50"
    )}>
        <div className={cn(
            "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-white/5 text-purple-400",
            isError && "text-red-400"
        )}>
            {icon}
        </div>
        <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between mb-0.5">
                <span className="text-[10px] font-black uppercase tracking-widest text-white/30">{label}</span>
                <span className={cn("text-[10px] font-bold px-1.5 rounded bg-white/5", isError ? "text-red-400" : "text-green-400")}>
                    {status}
                </span>
            </div>
            <div className="truncate text-[14px] font-medium text-white/90">{title || '—'}</div>
            <div className="text-[11px] text-white/40 leading-tight">{desc}</div>
        </div>
    </div>
);

export const DeepLinkModal = ({ isOpen, onClose, zapret, data }: Props) => {
    const [loading, setLoading] = useState(false);
    const [missingResources, setMissingResources] = useState<string[]>([]);

    const configData = useMemo(() => {
        if (data?.type !== 'configure_zapret' || !data.payload) return null;
        try {
            return JSON.parse(data.payload);
        } catch {
            log("Ошибка разбора параметров deeplink");
            return null;
        }
    }, [data]);

    useEffect(() => {
        setMissingResources([]);
        if (!isOpen || !configData) return;
        api.missingResources(configData.strategy || null, configData.ipset_config || null)
            .then(setMissingResources)
            .catch((e) => log(`Ошибка проверки ресурсов: ${e}`));
    }, [isOpen, configData]);

    const applyConfiguration = async () => {
        const strategy = configData.strategy.endsWith('.zapret') ? configData.strategy : `${configData.strategy}.zapret`;
        let ipset: string | undefined = configData.ipset_config;
        if (ipset && !['any', 'none'].includes(ipset.toLowerCase()) && !ipset.endsWith('.txt')) {
            ipset = `${ipset}.txt`;
        }
        const settings = await api.loadSettings();
        await api.saveSettings({ ...settings, gameFilter: configData.game_filter === 'true' });
        window.dispatchEvent(new Event('settings-changed'));

        if (ipset) zapret.setIpset(ipset);
        zapret.setStrategy(strategy);
        await zapret.refreshList();
        await zapret.start(strategy, ipset);
        notify("Конфигурация применена", "success");
    };

    const importFile = async () => {
        if (!data) return;
        const message = await api.addFromLink(data.type as 'strategy' | 'ipset' | 'hostlist', data.payload, data.name);
        await zapret.refreshList();
        notify(message, "success");
        // новый список доменов начинает работать только после перезапуска zapret
        if (data.type === 'hostlist' && zapret.status === 'running') await zapret.start();
    };

    const handleConfirm = async () => {
        if (!data || loading || (data.type === 'configure_zapret' && missingResources.length > 0)) return;
        setLoading(true);
        try {
            if (data.type === 'configure_zapret' && configData) {
                await applyConfiguration();
            } else if (data.type === 'restore_zapret') {
                await api.restoreZapretFiles();
                localStorage.removeItem("selected_strategy");
                localStorage.removeItem("selected_ipset");
                notify("Файлы zapret восстановлены", "success");
                setTimeout(() => window.location.reload(), 1000);
            } else {
                await importFile();
            }
            onClose();
        } catch (e) {
            log(`Ошибка deeplink: ${e}`);
            notify(`Ошибка: ${e}`, "error");
        } finally {
            setLoading(false);
        }
    };

    if (!data) return null;
    const currentConfig = TYPE_CONFIG[data.type] || { label: 'UNKNOWN', icon: '❓' };

    return (
        <Modal isOpen={isOpen} onClose={onClose} locked={loading}>
                <ModalHeader
                    title={data.type === 'configure_zapret' ? 'Настройка системы' :
                        data.type === 'restore_zapret' ? 'Восстановление' : 'Импорт объекта'}
                    status={currentConfig.label}
                    icon={currentConfig.icon}
                    description="Обработка входящего запроса конфигурации"
                />

                <ModalContent>
                    <div className="flex flex-col gap-3">
                        {data.type === 'configure_zapret' && configData && (
                            <>
                                <ConfigNode
                                    label="СТРАТЕГИЯ"
                                    status={missingResources.includes('strategy') ? 'ОТСУТСТВУЕТ' : 'ПРИСУТСТВУЕТ'}
                                    isError={missingResources.includes('strategy')}
                                    title={configData.strategy}
                                    desc="Основной набор правил для обхода."
                                    icon={<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" /></svg>}
                                />
                                <ConfigNode
                                    label="IPSET КОНФИГ"
                                    status={missingResources.includes('ipset') ? 'ОТСУТСТВУЕТ' : 'ПРИСУТСТВУЕТ'}
                                    isError={missingResources.includes('ipset')}
                                    title={configData.ipset_config}
                                    desc="Список адресов для фильтрации."
                                    icon={<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M21 5H3M15 12H3M17 19H3" /></svg>}
                                />
                                <div className={cn(
                                    "mt-2 p-3 rounded-xl border text-[12px] transition-colors",
                                    missingResources.length > 0
                                        ? "bg-red-500/10 border-red-500/20 text-red-400"
                                        : "bg-purple-500/5 border-purple-500/20 text-purple-300/80"
                                )}>
                                    {missingResources.length > 0
                                        ? "Некоторые файлы отсутствуют в системе. (￣ ￣|||)"
                                        : "Все компоненты найдены и готовы к работе."}
                                </div>
                            </>
                        )}

                        {data.type === 'restore_zapret' && (
                            <div className="flex flex-col gap-4">
                                <WarningHeader title="Полный сброс параметров" description="Это действие удалит все ваши стратегии и вернет настройки к начальным." />
                                <InfoHeader
                                    description="Если вас попросили использовать это для восстановления работы запрета, убедитесь что кастомные стратегии/конфиги (если есть) сохранены, и перенесены в безопасное место."
                                    more="Если нет, то смело нажимайте на кнопку ниже."
                                />
                            </div>
                        )}

                        {!['configure_zapret', 'restore_zapret'].includes(data.type) && (
                            <div className="space-y-3">
                                <div className="p-3 rounded-xl bg-white/[0.03] border border-white/5">
                                    <div className="text-[10px] text-white/30 font-bold uppercase mb-1">ФАЙЛ</div>
                                    <div className="text-white font-mono text-sm">{data.name || 'External_Resource'}</div>
                                </div>
                                <div className="p-3 rounded-xl bg-black/20 border border-white/5 font-mono text-[11px] text-purple-300/70 break-all max-h-32 overflow-y-auto">
                                    {data.payload}
                                </div>
                            </div>
                        )}
                    </div>
                </ModalContent>

                <ModalFooter>
                    <CloseButton onClick={onClose}>Отмена</CloseButton>

                    <SelectButton
                        onClick={handleConfirm}
                        className={cn(
                            (loading || (data.type === 'configure_zapret' && missingResources.length > 0)) && "pointer-events-none opacity-50"
                        )}
                        front={
                            <div className="flex items-center gap-2">
                                {loading ? "Выполнение..." : data.type === 'restore_zapret' ? "Сбросить всё" : "Применить"}
                            </div>
                        }
                        back="Подтвердить"
                    />
                </ModalFooter>
            </Modal>
    );
};