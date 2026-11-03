import { Modal } from "./Modal";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { InfoHeader, ModalContent, ModalFooter, ModalHeader, SelectButton } from "@/Buttons";
import { cn } from "@/lib/utils";
import { notify } from "@/Notifications";
import { api } from "@/lib/api";

export const HotspotModal = ({ isOpen, onClose, onLaunched }: {
    isOpen: boolean;
    onClose: () => void;
    onLaunched: (port: number) => void;
}) => {
    const [portInput, setPortInput] = useState("");
    const [statusText, setStatusText] = useState("Подготовка...");
    const [error, setError] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(false);

    useEffect(() => {
        if (isOpen) {
            setPortInput("");
            setError(null);
            setStatusText("Подготовка...");
        }
    }, [isOpen]);

    // показываем только строки frpc, а не весь лог приложения
    useEffect(() => {
        const unlisten = listen<string>("log-event", (event) => {
            if (event.payload.startsWith("[FRP]")) setStatusText(event.payload.replace("[FRP]", "").trim());
        });
        return () => { unlisten.then((f) => f()); };
    }, []);

    const handleStart = async () => {
        if (isLoading) return;
        const port = portInput ? Number(portInput) : null;
        if (port !== null && (!Number.isInteger(port) || port < 1024 || port > 65535)) {
            setError("Порт должен быть числом от 1024 до 65535");
            return;
        }
        setError(null);
        setIsLoading(true);
        try {
            const actualPort = await api.startHotspot(port);
            onLaunched(actualPort);
            onClose();
            notify(`Точка доступа запущена на порту ${actualPort}`, "success");
        } catch (e) {
            setError(String(e));
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <Modal isOpen={isOpen} onClose={onClose} locked={isLoading}>
            <ModalHeader
                title="Точка доступа"
                status="Конфигурация"
                icon="🛡️"
                description="Делает локальный SOCKS5-прокси доступным из интернета"
            />

            <ModalContent>
                <div className="space-y-4 py-2">
                    <InfoHeader
                        description="Точка доступа раздаёт прокси людям без Z.UST. Например, чтобы созвониться с другом в Telegram с телефона: включите точку доступа и добавьте прокси на обоих устройствах."
                        more="Прокси будет доступен любому, кто знает адрес и порт. Скорость может быть ниже, чем на ПК."
                    />
                    <div className="flex flex-col gap-2">
                        <label className="text-[11px] uppercase tracking-widest text-white/30 ml-1">Порт (пусто — случайный)</label>
                        <input
                            type="number"
                            value={portInput}
                            onChange={(e) => setPortInput(e.target.value)}
                            placeholder="Например, 34567"
                            disabled={isLoading}
                            className="w-full rounded-xl border border-white/5 bg-white/[0.03] p-3 text-sm text-white outline-none transition-all focus:border-purple-500/50 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                        />
                    </div>

                    {isLoading && (
                        <div className="flex flex-col items-center gap-2 py-2">
                            <div className="text-xs text-purple-400 animate-pulse truncate max-w-full">{statusText}</div>
                            <div className="h-1 w-full bg-white/5 rounded-full overflow-hidden">
                                <div className="h-full bg-purple-500 animate-progress-indefinite" />
                            </div>
                        </div>
                    )}

                    {error && (
                        <div className="rounded-lg bg-red-500/10 p-3 text-[12px] text-red-400 border border-red-500/20">
                            {error}
                        </div>
                    )}
                </div>
            </ModalContent>

            <ModalFooter>
                <SelectButton
                    onClick={handleStart}
                    className={cn(isLoading && "pointer-events-none opacity-80")}
                    front={isLoading ? "Секундочку.." : "Запустить"}
                    back={<div className="text-purple-300 font-bold">{isLoading ? "Запускаем..." : "Поехали!"}</div>}
                />
            </ModalFooter>
        </Modal>
    );
};
