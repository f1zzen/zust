import { CommunityButton, CreditCard, Header, SponsorsButton } from '@/Buttons';
import { CodeTabs } from '@/components/animate-ui/components/animate/code-tabs';
import { InteractiveHoverButton } from '@/components/ui/interactive-hover-button';
import { cn } from '@/lib/utils';
import { invoke } from '@tauri-apps/api/core';

export const ExtraPage = ({
    logs,
    torLogs,
    prefs
}: any) => {
    const CODES = getAllLogs(logs, torLogs);
    return (
        <div className="content credits-container">
            <Header title="Дополнительно" />
            <div className="credits-main-scroll">
                <div className="credits-section">
                    <h2 className="section-title">Благодарности</h2>
                    <SponsorsButton onClick={() => prefs.setIsSponsorsModalOpen(true)} />
                    <div className="grid grid-cols-2 gap-3 w-full">
                        <CreditCard title="zapret" author="bol-van" link="https://github.com/bol-van/zapret" />
                        <CreditCard title="zapret-youtube-discord" author="Flowseal" link="https://github.com/flowseal/zapret-discord-youtube" />
                        <CreditCard title="shadcn/ui" author="shadcn" link="https://ui.shadcn.com/" />
                        <CreditCard title="SmoothUI" author="Smooth Code" link="https://smoothui.dev/" />
                        <CreditCard title="MagicUI" author="Dillion" link="https://magicui.design/" />
                        <CreditCard title="AnimateUI" author="Skyleen" link="https://animate-ui.com/" />
                    </div>
                    <CommunityButton onClick={() => prefs.setIsCommunityModalOpen(true)} />
                    <p className="friends-text">...и моим друзьям 🥹</p>
                </div>
                <InteractiveHoverButton
                    className={cn(
                        "bg-[#0a0515] border-purple-900/50 text-purple-400 transition-all duration-300",
                        "[&>div_div]:bg-purple-500!",
                        "[&>div:last-child]:text-[#0a0515]!",
                        "group-hover:text-transparent!"
                    )}
                    onClick={async () => {
                        await invoke("open_link", { url: "https://t.me/zdotust" });
                    }}
                >
                    Подписаться на проект
                </InteractiveHoverButton>
            </div>
            <div className={cn(
                "relative mt-8 group mx-auto max-w-3xl w-full logs-container",
                "**:[[role=tablist]]:after:content-none!",
                "**:[[role=tablist]]:after:hidden",
                "**:[[role=tablist]]:bg-[#0a0515]!",
                "**:[[role=tablist]]:border-purple-900/50!",
                "**:[[role=tab][data-state=active]]:text-purple-400!",
                "**:[button:not([role=tab])]:bg-transparent!",
                "**:[button:not([role=tab]):hover]:text-purple-300!")}>
                <div className="relative flex flex-col rounded-xl border border-purple-900/50 bg-[#05010a] overflow-hidden">
                    <div className="overflow-y-auto custom-scrollbar">
                        <CodeTabs
                            lang="log"
                            codes={CODES}
                            themes={{
                                light: 'one-dark-pro',
                                dark: 'tokyo-night'
                            }}
                            className="!bg-transparent !border-none"
                        />
                    </div>
                </div>
            </div>
        </div>
    )
};

function getAllLogs(logs: any, torLogs: any) {
    let zustText = [...logs].reverse().join('\n');
    const torText = [...torLogs].reverse().join('\n');
    zustText = zustText.replace(/[\u001b\u009b][[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g, '');

    return {
        Zust: zustText,
        Tor: torText,
    };
}