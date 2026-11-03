import { CommunityButton, CreditCard, Header, SponsorsButton } from '@/Buttons';
import { InteractiveHoverButton } from '@/components/ui/interactive-hover-button';
import { api, type AppInfo } from '@/lib/api';
import { cn } from '@/lib/utils';
import { notify } from '@/Notifications';
import type { ModalId } from '@/Logic';

const OS_NAMES: Record<string, string> = { windows: 'Windows', linux: 'Linux' };

/** Карточка версии: по клику копирует строку для баг-репорта. */
const VersionCard = ({ info, updateAvailable }: { info: AppInfo | null; updateAvailable: string | null }) => {
    if (!info) return null;
    const line = `Zust v${info.version} (${info.commit}, ${info.buildDate}) · ${OS_NAMES[info.os] ?? info.os} ${info.arch}`;
    return (
        <div
            onClick={async () => {
                await navigator.clipboard.writeText(line);
                notify("Версия скопирована", "success");
            }}
            className="group w-full cursor-pointer rounded-xl border border-white/5 bg-[#0a0514]/60 p-[14px_18px] transition-all hover:border-purple-500/20 active:scale-[0.98]"
        >
            <div className="flex items-baseline justify-between gap-3">
                <span className="font-['Onest'] text-[15px] font-bold text-white/90">Zust v{info.version}</span>
                <span className="text-[11px] text-white/30 group-hover:text-purple-400 transition-colors">копировать</span>
            </div>
            <div className="mt-1 font-mono text-[11px] text-white/40">
                {info.commit} · {info.buildDate} · {OS_NAMES[info.os] ?? info.os} {info.arch}{info.debug ? ' · debug' : ''}
            </div>
            {updateAvailable && (
                <button
                    onClick={(e) => {
                        e.stopPropagation();
                        api.openLink("https://github.com/f1zzen/zust/releases/latest");
                    }}
                    className="mt-2 text-[12px] font-bold text-blue-300 hover:underline"
                >
                    ✨ Доступна v{updateAvailable} — скачать
                </button>
            )}
        </div>
    );
};

export const ExtraPage = ({ appInfo, updateAvailable, openModal }: {
    appInfo: AppInfo | null;
    updateAvailable: string | null;
    openModal: (id: ModalId) => void;
}) => (
    <div className="content credits-container">
        <Header title="Дополнительно" />
        <div className="credits-main-scroll">
            <div className="credits-section">
                <h2 className="section-title">О программе</h2>
                <VersionCard info={appInfo} updateAvailable={updateAvailable} />

                <h2 className="section-title" style={{ marginTop: '12px' }}>Благодарности</h2>
                <SponsorsButton onClick={() => openModal('sponsors')} />
                <div className="grid grid-cols-2 gap-3 w-full">
                    <CreditCard title="zapret" author="bol-van" link="https://github.com/bol-van/zapret" />
                    <CreditCard title="zapret-discord-youtube" author="Flowseal" link="https://github.com/flowseal/zapret-discord-youtube" />
                    <CreditCard title="shadcn/ui" author="shadcn" link="https://ui.shadcn.com/" />
                    <CreditCard title="SmoothUI" author="Smooth Code" link="https://smoothui.dev/" />
                    <CreditCard title="MagicUI" author="Dillion" link="https://magicui.design/" />
                    <CreditCard title="AnimateUI" author="Skyleen" link="https://animate-ui.com/" />
                </div>
                <CommunityButton onClick={() => openModal('community')} />
                <p className="friends-text">...и моим друзьям 🥹</p>
            </div>
            <InteractiveHoverButton
                className={cn(
                    "bg-[#0a0515] border-purple-900/50 text-purple-400 transition-all duration-300",
                    "[&>div_div]:bg-purple-500!",
                    "[&>div:last-child]:text-[#0a0515]!",
                    "group-hover:text-transparent!"
                )}
                onClick={() => api.openLink("https://t.me/zdotust")}
            >
                Подписаться на проект
            </InteractiveHoverButton>
        </div>
    </div>
);
