import { Modal, CloseButton } from "./Modal";
import { useState } from "react";
import { api, log, stripExt } from "@/lib/api";
import {
    ModalHeader,
    ModalContent,
    ModalFooter,
    SelectButton
} from "@/Buttons";
import { cn } from "@/lib/utils";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    configs: string[];
    stratName: string | null;
    onSelect: (name: string) => void;
    updatableStrats: string[];
    setUpdatableStrats: React.Dispatch<React.SetStateAction<string[]>>;
}

const UpdateButton = ({ name, onUpdated }: { name: string, onUpdated: () => void }) => {
    const [status, setStatus] = useState<'idle' | 'loading' | 'success'>('idle');

    const handleUpdate = async (e: React.MouseEvent) => {
        e.stopPropagation();
        setStatus('loading');
        try {
            await api.applyStrategyUpdate(name);
            setStatus('success');
            setTimeout(() => {
                setStatus('idle');
                onUpdated();
            }, 2000);
        } catch (e) {
            log("update error: " + e);
            setStatus('idle');
        }
    };

    if (status === 'success') return <span className="text-[#4ade80] text-sm font-bold">✓</span>;

    return (
        <button
            className={cn(
                "flex h-7 w-7 items-center justify-center rounded-lg bg-purple-500/10 text-purple-400 transition-all hover:bg-purple-500/20 active:scale-90",
                status === 'loading' && "animate-spin"
            )}
            onClick={handleUpdate}
            disabled={status === 'loading'}
        >
            <span className="text-sm leading-none">↻</span>
        </button>
    );
};

export const StrategyModal = ({
    isOpen,
    onClose,
    configs,
    stratName,
    onSelect,
    updatableStrats,
    setUpdatableStrats
}: Props) => {

    return (
        <Modal isOpen={isOpen} onClose={onClose}>
                <ModalHeader
                    title="Стратегии"
                    status={configs.length + " доступно"}
                    icon="📂"
                    description="Стратегии, для обхода DPI-блокировок (aka Zapret)."
                />

                <ModalContent>
                    {configs.map((cfg) => {
                        const isActive = stratName === cfg;
                        const canUpdate = updatableStrats.includes(cfg);

                        return (
                            <div
                                key={cfg}
                                onClick={() => onSelect(cfg)}
                                className={cn(
                                    "group flex items-center gap-4 rounded-xl border p-3.5 transition-all cursor-pointer active:scale-[0.99]",
                                    isActive
                                        ? "border-purple-500/30 bg-purple-500/10"
                                        : "border-transparent bg-white/[0.03] hover:bg-white/[0.06] hover:border-white/5"
                                )}
                            >
                                <div className={cn(
                                    "h-2 w-2 shrink-0 rounded-full transition-all duration-300 shadow-[0_0_8px]",
                                    isActive ? "bg-purple-500 shadow-purple-500" : "bg-white/20 shadow-transparent"
                                )} />

                                <span className={cn(
                                    "flex-1 font-['Onest'] text-[14px] font-medium transition-colors",
                                    isActive ? "text-white" : "text-white/60 group-hover:text-white/90"
                                )}>
                                    {stripExt(cfg)}
                                </span>

                                {isActive ? (
                                    <span className="text-[11px] font-bold uppercase tracking-wider text-purple-400/80">
                                        Активно
                                    </span>
                                ) : canUpdate && (
                                    <UpdateButton
                                        name={cfg}
                                        onUpdated={() => setUpdatableStrats(prev => prev.filter(s => s !== cfg))}
                                    />
                                )}
                            </div>
                        );
                    })}
                </ModalContent>

                <ModalFooter>
                    <SelectButton
                        onClick={() => api.openFolder('strategies')}
                        front="Открыть папку"
                        back="Открыть папку"
                    />
                    <CloseButton onClick={onClose}>Закрыть</CloseButton>
                </ModalFooter>
            </Modal>
    );
};