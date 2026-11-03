import { useMemo, useState } from 'react';
import { Header } from '@/Buttons';
import { cn } from '@/lib/utils';
import { notify } from '@/Notifications';

// убирает ANSI-цвета из вывода winws/nfqws
const ANSI = /[\u001b\u009b][[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g;

const lineColor = (line: string) => {
    const l = line.toLowerCase();
    if (l.includes('ошибк') || l.includes('error') || l.includes('[err]') || l.includes('не удалось')) return 'text-red-300/90';
    if (l.includes('[warn]') || l.includes('warning')) return 'text-yellow-200/80';
    if (l.includes('[tor]') || l.includes('[frp]')) return 'text-purple-200/80';
    return 'text-white/70';
};

export const LogsPage = ({ logs, torLogs }: { logs: string[]; torLogs: string[] }) => {
    const [tab, setTab] = useState<'zust' | 'tor'>('zust');
    const lines = useMemo(
        () => [...(tab === 'zust' ? logs : torLogs)].reverse().map((l) => l.replace(ANSI, '')),
        [tab, logs, torLogs]
    );

    const copy = async () => {
        await navigator.clipboard.writeText(lines.join('\n'));
        notify("Лог скопирован", "success");
    };

    return (
        <div className="content">
            <Header title="Логи" />
            <div className="w-full max-w-120 rounded-xl border border-purple-900/50 bg-[#05010a] overflow-hidden">
                <div className="flex items-center gap-1 border-b border-purple-900/40 p-1.5">
                    {(['zust', 'tor'] as const).map((t) => (
                        <button
                            key={t}
                            onClick={() => setTab(t)}
                            className={cn(
                                "rounded-lg px-3 py-1 text-[12px] font-medium transition-colors",
                                tab === t ? "bg-purple-500/20 text-purple-200" : "text-white/40 hover:text-white/70"
                            )}
                        >
                            {t === 'zust' ? 'Zust' : 'TOR'}
                            <span className="ml-1.5 text-white/30">{(t === 'zust' ? logs : torLogs).length}</span>
                        </button>
                    ))}
                    <button
                        onClick={copy}
                        disabled={!lines.length}
                        className="ml-auto rounded-lg px-3 py-1 text-[12px] text-white/40 transition-colors hover:text-purple-300 disabled:opacity-30"
                    >
                        Копировать
                    </button>
                </div>
                <div className="custom-scrollbar h-[380px] overflow-y-auto p-3 font-mono text-[11px] leading-relaxed select-text">
                    {lines.length === 0 ? (
                        <div className="py-20 text-center text-white/25">Пока пусто</div>
                    ) : (
                        lines.map((line, i) => (
                            <div key={i} className={cn("whitespace-pre-wrap break-all", lineColor(line))}>{line}</div>
                        ))
                    )}
                </div>
            </div>
            <p className="mt-3 text-center text-[11px] text-white/25">
                При жалобе на баг прикладывайте лог — так проще разобраться.
            </p>
        </div>
    );
};
