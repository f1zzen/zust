import { useEffect, useState, useMemo, memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "../Notifications";
import { log } from "../Logic";
import { useModalAnimation } from "./useModalAnimation";
import { ModalBlock, ModalContent, ModalFooter, ModalHeader, SelectButton } from "@/Buttons";
import { cn } from "@/lib/utils";
import React from "react";

const LIMIT_DOMAINS = 50;

const DomainList = memo(({ lines, isExpanded }: { lines: string[], isExpanded: boolean }) => (
    <div className="flex flex-col gap-2 p-6 pt-0">
        {lines.slice(0, LIMIT_DOMAINS).map((line, i) => {
            const [ip, ...domainParts] = line.trim().split(/\s+/);
            return (
                <div
                    key={i}
                    className={cn(
                        "grid grid-cols-[10px_105px_1fr] items-center gap-2 opacity-0",
                        isExpanded && "animate-[slideInHost_0.25s_ease_forwards]"
                    )}
                    style={{ animationDelay: `${i * 12}ms` }}
                >
                    <div className="h-1 w-1 rounded-full bg-purple-500" />
                    <span className="font-['JetBrains_Mono'] text-[11px] text-purple-400/90">{ip}</span>
                    <span className="truncate text-[12px] text-white/50">{domainParts.join(" ")}</span>
                </div>
            );
        })}
        {lines.length > LIMIT_DOMAINS && (
            <div className="mt-2 flex items-center justify-between rounded-lg border border-dashed border-purple-500/20 bg-purple-500/5 px-3 py-1.5 text-[11px] text-white/40 mr-4">
                <span>Показано {LIMIT_DOMAINS} из {lines.length}</span>
                <span className="text-[9px] font-bold uppercase tracking-wider text-purple-400">Лимит</span>
            </div>
        )}
    </div>
));

const CategoryItem = ({
    name,
    lines,
    isSelected,
    isExpanded,
    onToggleExpand,
    onToggleSelect
}: any) => (
    <div className={cn(
        "group rounded-2xl border border-white/5 bg-white/[0.01] shadow-lg transition-all",
        isSelected && "border-purple-500/20 bg-purple-500/[0.02]"
    )}>
        <div className="flex cursor-pointer items-center gap-4 px-6 py-4" onClick={onToggleExpand}>
            <div className="flex shrink-0 items-center" onClick={(e) => e.stopPropagation()}>
                <label className="relative h-5 w-5 cursor-pointer">
                    <input
                        type="checkbox"
                        className="peer sr-only"
                        checked={isSelected}
                        onChange={(e) => onToggleSelect(e.target.checked)}
                    />
                    <div className="h-full w-full rounded-md border-[1.5px] border-white/10 bg-white/5 transition-all peer-checked:border-purple-500 peer-checked:bg-purple-500" />
                    <svg className="absolute left-1.25 top-1.5 hidden h-2 w-2.5 text-white peer-checked:block" viewBox="0 0 10 8" fill="none" stroke="currentColor" strokeWidth="2.5">
                        <path d="M1 4L4 7L9 1" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                </label>
            </div>

            <span className="min-w-0 flex-1">
                <span className="block truncate text-[13px] font-medium text-white/70 transition-colors group-hover:text-white">
                    {name}
                </span>
            </span>

            <div className={cn("ml-auto shrink-0 transition-transform duration-300 opacity-30 group-hover:opacity-100", isExpanded && "rotate-180 text-purple-400 opacity-100")}>
                <svg width="12" height="8" viewBox="0 0 12 8" fill="none">
                    <path d="M1 1.5L6 6.5L11 1.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                </svg>
            </div>
        </div>
        <div className={cn("grid transition-all duration-300 ease-in-out", isExpanded ? "grid-rows-[1fr]" : "grid-rows-[0fr]")}>
            <div className="overflow-hidden bg-black/10">
                <DomainList lines={lines} isExpanded={isExpanded} />
            </div>
        </div>
    </div>
);

export const HostsModal = ({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    const [data, setData] = useState<Record<string, string[]>>({});
    const [loading, setLoading] = useState(true);
    const [expanded, setExpanded] = useState<string[]>([]);
    const [selected, setSelected] = useState<string[]>([]);
    const [saving, setSaving] = useState(false);
    const [lastUpdate, setLastUpdate] = useState<string>("");

    const loadHosts = async () => {
        setLoading(true);

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 10000);

        try {
            const [result, installed] = await Promise.all([
                invoke<{ date: string; categories: Record<string, string[]> }>("get_hosts_data"),
                invoke<string[]>("get_installed_categories"),
                new Promise(resolve => setTimeout(resolve, 600))
            ]);

            clearTimeout(timeoutId);
            setData(result.categories);
            setLastUpdate(result.date);
            setSelected(installed.filter((cat) => result.categories[cat]));
        } catch (e: any) {
            const isTimeout = e.name === 'AbortError';
            log(isTimeout ? "hosts load timeout" : "hosts err " + e);
            notify(isTimeout ? "Превышено время ожидания данных" : "Ошибка загрузки конфигурации", "error");

            if (Object.keys(data).length === 0) {
                onClose();
            }
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        if (isOpen && Object.keys(data).length === 0) loadHosts();
    }, [isOpen]);

    const sortedKeys = useMemo(() =>
        Object.keys(data).sort((a, b) => a === "Базовая" ? -1 : b === "Базовая" ? 1 : a.localeCompare(b))
        , [data]);

    const handleSave = async () => {
        setSaving(true);
        try {
            const MALW_START = "### dns.malw.link: hosts file";
            const MALW_END = "### dns.malw.link: end hosts file";
            const FLOW_START = "### flowseal: hosts file";
            const FLOW_END = "### flowseal: end hosts file";

            let malwContent = "";
            let flowContent = "";

            selected.forEach((cat) => {
                if (!data[cat]) return;

                const block = `# ${cat}\n${data[cat].join("\n")}\n`;

                if (cat === "Discord & Telegram") {
                    flowContent += block;
                } else {
                    malwContent += block;
                }
            });

            let finalBody = "";
            if (malwContent) finalBody += `${MALW_START}\n${malwContent}${MALW_END}\n`;
            if (flowContent) finalBody += `${FLOW_START}\n${flowContent}${FLOW_END}\n`;

            await invoke("save_hosts_selection", { selectedLines: finalBody.trim() });
            notify("Конфигурация hosts обновлена", "success");
            onClose();
        } catch (e) {
            notify("Ошибка при сохранении", "error");
        } finally {
            setSaving(false);
        }
    };

    if (!shouldRender) return null;

    return (
        <div className={cn("fixed inset-0 z-[1000] flex items-center justify-center bg-black/60 backdrop-blur-md transition-opacity duration-300", isAnimatingOut ? "opacity-0" : "opacity-100")} onClick={onClose}>
            <ModalBlock onClick={(e: React.MouseEvent) => e.stopPropagation()}>
                <ModalHeader
                    title="Настройка"
                    description={<span className="opacity-45">Последнее обновление: {lastUpdate}</span>}
                    status={
                        <span>
                            {Object.keys(data).length} групп
                        </span>
                    }
                    icon="📅"
                />

                <ModalContent>
                    {loading ? (
                        <div className="flex h-40 flex-col items-center justify-center gap-3 animate-pulse text-white/30 text-sm">
                            Загружаем данные..
                        </div>
                    ) : (
                        <div className="space-y-2.5 p-4">
                            {sortedKeys.map((name) => (
                                <CategoryItem
                                    key={name}
                                    name={name}
                                    lines={data[name]}
                                    isSelected={selected.includes(name)}
                                    isExpanded={expanded.includes(name)}
                                    onToggleExpand={() => setExpanded(prev => prev.includes(name) ? prev.filter(n => n !== name) : [...prev, name])}
                                    onToggleSelect={(checked: boolean) => setSelected(prev => checked ? [...prev, name] : prev.filter(n => n !== name))}
                                />
                            ))}
                        </div>
                    )}
                </ModalContent>

                <ModalFooter>
                    <div className="mr-auto flex gap-4">
                        {["Всё", "Ничего"].map((label) => (
                            <button
                                key={label}
                                onClick={() => setSelected(label === "Всё" ? Object.keys(data) : [])}
                                className="text-[10px] font-bold uppercase tracking-widest text-white/20 transition-colors hover:text-white/60"
                            >
                                {label}
                            </button>
                        ))}
                    </div>
                    <button className="text-[13px] font-medium text-zinc-500 hover:text-white transition-colors" onClick={onClose}>Отмена</button>
                    <SelectButton onClick={handleSave} front={saving ? "..." : "Применить"} back="Готово" />
                </ModalFooter>
            </ModalBlock>
        </div>
    );
};