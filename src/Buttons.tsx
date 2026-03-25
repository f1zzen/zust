
import { openUrl } from "@tauri-apps/plugin-opener";
import { FlipButton, FlipButtonBack, FlipButtonFront } from "./components/animate-ui/primitives/buttons/flip";
import AnimatedTooltip from "./components/smoothui/animated-tooltip";
import { cn } from "./lib/utils";


interface ModalHeaderProps {
    title: string;
    status?: React.ReactNode;
    description: React.ReactNode;
    icon: string;
}

export const ModalHeader = ({
    title,
    status,
    description,
    icon
}: ModalHeaderProps) => (
    <div className="flex items-start justify-between bg-white/[0.02] p-6 border-b border-white/5">
        <div className="space-y-1.5 flex-1">
            <div className="flex items-center gap-3">
                <h3 className="text-lg font-bold text-white font-['Onest']">{title}</h3>
                {status && (
                    <span className="rounded-full bg-purple-500/10 px-2.5 py-0.5 text-[11px] font-bold text-purple-300 border border-purple-500/20 transition-all">
                        {status}
                    </span>
                )}
            </div>
            <div className="flex items-center gap-2 text-white/40">
                <span className="text-sm shrink-0">{icon}</span>
                <span className="text-[13px] leading-tight">{description}</span>
            </div>
        </div>

        <div className="relative mt-2 ml-4">
            <div className="h-2 w-2 rounded-full bg-purple-500 shadow-[0_0_10px_#a855f7]" />
        </div>
    </div>
);

export const ModalContent = ({ children }: { children: React.ReactNode }) => (
    <div className="custom-scrollbar flex-1 overflow-y-auto overflow-x-hidden p-4">
        <div className="space-y-2 px-1">
            {children}
        </div>
    </div>
);

export const ModalBlock = ({ children, className, ...props }: any) => (
    <div
        className={cn(
            "relative flex h-[520px] w-full max-w-[560px] flex-col rounded-[24px] border border-white/5 bg-[#0d0d12] shadow-2xl animate-in zoom-in-90 duration-200",
            className
        )}
        {...props}>
        {children}
    </div>
);

export const DropDownItemStyle = "flex items-center w-full px-3 py-2.5 text-[14px] text-white/90 font-['Onest'] font-medium bg-transparent border border-transparent rounded-xl cursor-pointer transition-all duration-200 ease-out outline-none data-[highlighted]:bg-white/10 data-[highlighted]:text-white active:scale-[0.96]";

export const ModalFooter = ({ children }: { children: React.ReactNode }) => (
    <div className="flex items-center justify-center gap-4 border-t border-white/5 bg-white/[0.01] p-4">
        {children}
    </div>
);

export const MenuButton = ({ title, subtitle, icon, onClick, variant, ...props }: any) => (
    <button className={`wide-btn ${variant}-variant`} onClick={onClick} {...props}>
        <div className="btn-content">
            <span className="btn-icon">{icon}</span>
            <div className="btn-text">
                <span className="btn-title">{title}</span>
                <span className="btn-subtitle">{subtitle}</span>
            </div>
        </div>
        <span className="btn-arrow">→</span>
    </button>
);

export const WarningHeader = ({ title, description }: any) => (
    <div className="relative group">
        <div className="absolute inset-0 bg-red-950/50 rounded-xl border border-red-500/20" />
        <div className="relative bg-black/15 border border-red-500/10 rounded-xl p-4 space-y-2">
            <div className="flex items-center gap-2 text-red-400">
                <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                >
                    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                    <line x1="12" y1="9" x2="12" y2="13" />
                    <line x1="12" y1="17" x2="12.01" y2="17" />
                </svg>
                <span className="text-[10px] font-black uppercase tracking-[2px]">Внимание!</span>
            </div>

            <div>
                <div className="text-white font-medium text-[14px] mb-0.5">
                    {title}
                </div>
                <p className="text-[12px] text-white/40 leading-relaxed">
                    {description}
                </p>
            </div>
        </div>
    </div>
)
export const InfoHeader = ({ description, more }: any) => (
    <div className="relative group">
        <div className="absolute -inset-0.5 from-purple-500/20 to-blue-500/20 rounded-xl blur opacity-75" />
        <div className="relative bg-[#0d0816] border border-white/5 rounded-xl p-4 space-y-2">
            <div className="flex items-center gap-2 text-purple-400">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10" /><path d="M12 16v-4" /><path d="M12 8h.01" /></svg>
                <span className="text-xs font-bold uppercase tracking-wider">СПРАВКА!</span>
            </div>
            <p className="text-[13px] text-white/40 leading-relaxed">
                {description}
            </p>
            <p className="text-[11px] text-white/20 italic">
                {more}
            </p>
        </div>
    </div>
)

export const SelectButton = ({ onClick, front, back, className }: any) => (
    <FlipButton className={cn(
        "select-strat-btn",
        className
    )} onClick={onClick}>
        <FlipButtonFront>{front}</FlipButtonFront>
        <FlipButtonBack>{back}</FlipButtonBack>
    </FlipButton>
)

export const ActionButton = ({ STATUS, handleToggle, status }: any) => {
    return (
        <div className="hero-container">
            <AnimatedTooltip content={STATUS[status as keyof typeof STATUS]} placement='bottom'>
                <button
                    className={`hero-reactor ${status}`}
                    onClick={handleToggle}
                >
                    <div className="reactor-aura"></div>
                    <div className="reactor-core">
                        <div className="core-inner">
                            <div className={`status-icon stopped ${status === 'stopped' ? 'visible' : ''}`}>
                                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                            </div>

                            <div className={`status-icon loading ${status === 'loading' ? 'visible' : ''}`}>
                                <div className="spinner-arc"></div>
                            </div>

                            <div className={`status-icon running ${status === 'running' ? 'visible' : ''}`}>
                                <span className="sparkle-diamond">✦</span>
                            </div>
                        </div>
                    </div>

                    <div className="reactor-ring"></div>
                    <div className="reactor-ring var-2"></div>
                </button>
            </AnimatedTooltip>
        </div>)
}

export const Header = ({ title }: { title: string; Icon?: any }) => {
    return (
        <div className="flex flex-col items-center mb-6 select-none w-full">
            <button
                className="
          group relative flex items-center justify-center
          px-12 py-2
          bg-[#0a0514]/60 border border-white/5
          rounded-xl transition-all duration-300
          hover:bg-[#0a0514]/90 hover:border-purple-500/20
          active:scale-[0.98] w-full max-w-110
        "
            >
                <div className="flex items-center w-85">
                    <div className="h-[1px] flex-1 bg-gradient-to-r from-transparent to-white/10" />

                    <div className="px-6">
                        <h1 className="text-lg font-medium text-white/90">
                            {title}
                        </h1>
                    </div>

                    <div className="h-[1px] flex-1 bg-gradient-to-l from-transparent to-white/10" />
                </div>
            </button>
        </div>
    );
};

interface CreditCardProps {
    title: string;
    author: string;
    link: string;
}

export const CreditCard = ({ title, author, link }: CreditCardProps) => (
    <div
        onClick={() => openUrl(link)}
        className="
      group flex items-center justify-between w-full p-[14px_18px]
      bg-[#0a0514]/60 border border-white/5
      rounded-xl cursor-pointer gap-5 box-border
      transition-all duration-200 ease-out
      hover:bg-[#0a0514]/90 hover:border-purple-500/20
      active:scale-[0.96] active:bg-[#0a0514]
    "
    >
        <span className="font-['Onest'] text-[14px] text-white/90 font-medium whitespace-nowrap overflow-hidden text-ellipsis">
            {title}
        </span>
        <span className="text-[12px] shrink-0 text-right text-purple-500/80 group-hover:text-purple-500 transition-colors">
            by {author}
        </span>
    </div>
);

export const SponsorsButton = ({ onClick }: { onClick: () => void }) => (
    <div
        onClick={onClick}
        className="
      group flex items-center justify-between w-full p-[14px_18px]
      bg-purple-500/10 border border-purple-500/20
      rounded-xl cursor-pointer gap-5 box-border
      transition-all duration-200 ease-out
      hover:bg-purple-500/20 hover:border-purple-500/40
      active:scale-[0.96] animate-pulse-subtle
    "
    >
        <span className="font-['Onest'] text-[14px] text-purple-300 font-bold tracking-wider">
            СПОНСОРЫ!
        </span>
        <span className="text-[12px] shrink-0 text-right text-purple-400">
            {'<33'}
        </span>
    </div>
);

export const CommunityButton = ({ onClick }: { onClick: () => void }) => (
    <div
        onClick={onClick}
        className="
      group flex items-center justify-between w-full p-[14px_18px]
      bg-purple-500/10 border border-purple-500/20
      rounded-xl cursor-pointer gap-5 box-border
      transition-all duration-200 ease-out
      hover:bg-purple-500/20 hover:border-purple-500/40
      active:scale-[0.96] animate-pulse-subtle
    "
    >
        <span className="font-['Onest'] text-[14px] text-purple-300 font-bold tracking-wider">
            Сообщество!
        </span>
        <span className="text-[12px] shrink-0 text-right text-purple-400">
            Наше коммьюнити!
        </span>
    </div>
);