import { Modal } from "./Modal";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { ModalContent, ModalFooter, ModalHeader, SelectButton } from "@/Buttons";
import React from "react";
import { Button } from "@/components/ui/button";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    selectedIpset: string | null;
    ipsetView: 'main' | 'custom';
    setIpsetView: (v: 'main' | 'custom') => void;
    customIpsetFiles: string[];
    onModeChange: (mode: 'Any' | 'None' | 'Custom', file?: string) => void;
    loadCustom: () => void;
    hoveredDesc: string | null;
    setHoveredDesc: (s: string | null) => void;
}

const IpsetItem = ({ label, icon, isActive, onClick, onHover }: any) => (
    <button
        className={cn(
            "group relative flex w-full items-center gap-3 rounded-xl border border-white/5 bg-white/[0.02] p-4 transition-all duration-200 hover:border-purple-500/30 hover:bg-purple-500/[0.04]",
            isActive && "border-purple-500/50 bg-purple-500/10 shadow-[0_0_20px_rgba(168,85,247,0.15)]"
        )}
        onClick={onClick}
        onMouseEnter={() => onHover(true)}
        onMouseLeave={() => onHover(false)}
    >
        <span className="text-xl">{icon}</span>
        <span className={cn(
            "text-[13px] font-medium text-white/70 transition-colors group-hover:text-white",
            isActive && "text-white"
        )}>
            {label}
        </span>
        {isActive && (
            <div className="absolute right-4 h-1.5 w-1.5 rounded-full bg-purple-500 animate-pulse" />
        )}
    </button>
);

const DESCRIPTIONS: Record<string, string> = {
    'Any': 'Правила по IP применяются ко всем адресам. Сильнее обходит, но нагружает систему.',
    'None': 'Правила по IP отключены — zapret работает только по спискам доменов.',
    'ipset-all.txt': 'Список адресов из zapret/lists/ipset-all.txt.',
    'custom': 'Фильтрация по вашему индивидуальному списку адресов.'
};

export const IpsetModal = (p: Props) => {

    const hoverTimerRef = React.useRef<NodeJS.Timeout | null>(null);

    const handleHover = (description: string | null) => {
        if (hoverTimerRef.current) {
            clearTimeout(hoverTimerRef.current);
            hoverTimerRef.current = null;
        }

        if (description) {
            p.setHoveredDesc(description);
        } else {
            hoverTimerRef.current = setTimeout(() => {
                p.setHoveredDesc(null);
            }, 500);
        }
    };

    return (
        <Modal isOpen={p.isOpen} onClose={p.onClose}>
                <ModalHeader
                    title={p.ipsetView === 'main' ? 'Фильтрация по IP' : 'Списки адресов'}
                    description={<span>Настройка обработки трафика ZAPRET</span>}
                    status={p.ipsetView === 'custom' ? `${p.customIpsetFiles.length + 1} файлов` : null}
                    icon="🛡️"
                />

                <ModalContent>
                    <div className="space-y-3 p-4">
                        {p.ipsetView === 'main' ? (
                            <div className="grid gap-2">
                                <IpsetItem
                                    label="Весь трафик (Any)"
                                    icon="🌐"
                                    isActive={p.selectedIpset?.toLowerCase() === 'any'}
                                    onClick={() => { p.onModeChange('Any'); p.onClose(); }}
                                    onHover={(h: boolean) => handleHover(h ? DESCRIPTIONS['Any'] : null)}
                                />
                                <IpsetItem
                                    label="Отключить (None)"
                                    icon="🚫"
                                    isActive={p.selectedIpset?.toLowerCase() === 'none'}
                                    onClick={() => { p.onModeChange('None'); p.onClose(); }}
                                    onHover={(h: boolean) => handleHover(h ? DESCRIPTIONS['None'] : null)}
                                />
                                <IpsetItem
                                    label="Пользовательский список"
                                    icon="📄"
                                    isActive={false}
                                    onClick={() => { p.loadCustom(); p.setIpsetView('custom'); }}
                                    onHover={(h: boolean) => handleHover(h ? DESCRIPTIONS['custom'] : null)}
                                />
                            </div>
                        ) : (
                            <div className="flex flex-col gap-2">
                                <div className="mb-2 flex gap-2 justify-start">
                                    <SelectButton
                                        onClick={() => api.openFolder('ipset-configs')}
                                        front="Открыть папку"
                                        back="С конфигами"
                                        className="w-30"
                                    />
                                    <Button variant="outline" onClick={() => p.setIpsetView('main')} size="sm" className="text-[12px] w-20 text-white/80">← Назад</Button>
                                </div>

                                <div className="max-h-[300px] space-y-2 overflow-y-auto pr-1 custom-scrollbar">
                                    <IpsetItem
                                        label="ipset-all.txt"
                                        icon="⭐"
                                        isActive={p.selectedIpset === 'ipset-all.txt'}
                                        onClick={() => { p.onModeChange('Custom', 'ipset-all.txt'); p.onClose(); }}
                                        onHover={(h: boolean) => handleHover(h ? DESCRIPTIONS['ipset-all.txt'] : null)}
                                    />
                                    {p.customIpsetFiles.map((file) => (
                                        <IpsetItem
                                            key={file}
                                            label={file.replace('.txt', '').replace(/_/g, ' ')}
                                            icon="📄"
                                            isActive={p.selectedIpset === file}
                                            onClick={() => { p.onModeChange('Custom', file); p.onClose(); }}
                                            onHover={(h: boolean) => handleHover(h ? DESCRIPTIONS['custom'] : null)}
                                        />
                                    ))}
                                </div>
                            </div>
                        )}
                    </div>
                </ModalContent>

                <ModalFooter>
                    <div className="mr-auto flex items-center gap-2 overflow-hidden px-1">
                        <div className={cn(
                            "flex items-center gap-2 text-[11px] text-white/30 transition-all duration-300 transform",
                            p.hoveredDesc ? "translate-y-0 opacity-100" : "translate-y-4 opacity-0"
                        )}>
                            <span className="text-purple-500/50">ℹ</span>
                            <span className="truncate text-white/60">{p.hoveredDesc}</span>
                        </div>
                    </div>
                    <button
                        className="text-[13px] font-medium text-zinc-500 transition-colors hover:text-white"
                        onClick={p.onClose}
                    >
                        Отмена
                    </button>
                </ModalFooter>
            </Modal>
    );
};