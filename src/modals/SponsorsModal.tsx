import { ModalBlock, ModalContent, ModalHeader, ModalFooter, InfoHeader } from "@/Buttons";
import { useModalAnimation } from "./useModalAnimation";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

export const SponsorsModal = ({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) => {
    const SPONSORS = [
        { name: "neton4ik", amount: "400₽" },
        { name: "Рустам", amount: "1063₽" }
    ];

    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);

    if (!shouldRender) return null;

    return (
        <div
            className={cn(
                "fixed inset-0 z-[1000] flex items-center justify-center bg-black/60 backdrop-blur-md transition-opacity duration-300",
                isAnimatingOut ? "opacity-0" : "opacity-100"
            )}
            onClick={onClose}
        >
            <ModalBlock onClick={(e: any) => e.stopPropagation()}>
                <ModalHeader
                    title="Спонсоры на Boosty!"
                    status="💝💝💝"
                    icon="⭐"
                    description="Люди, которые пожертвовали деньги до выхода этой версии!"
                />

                <ModalContent>
                    <InfoHeader description="Данный список обновляется после каждой версии, и при выходе новой (в случае пожертвования) вы обязательно будете сюда добавлены." more="Купив подписку на бусти, вы жертвуете 200₽*" />
                    <div className="grid grid-cols-3 gap-2 w-full">
                        {SPONSORS.map((sponsor, index) => (
                            <div
                                key={index}
                                className="
        relative group flex flex-col items-center justify-center p-4
        bg-[#ffffff]/[0.02] border border-white/5 rounded-xl
        transition-all duration-500 ease-out
        hover:border-purple-500/30 hover:bg-[#ffffff]/[0.04]
        active:scale-[0.97] overflow-hidden
      "
                            >
                                <div className="
        absolute inset-0 pointer-events-none opacity-0 group-hover:opacity-100
        transition-opacity duration-500
        bg-[radial-gradient(circle_at_center,_var(--tw-gradient-stops))]
        from-purple-500/15 via-transparent to-transparent
      " />
                                <span className="relative z-10 text-white/80 font-medium text-[14px] tracking-tight mb-1">
                                    {sponsor.name}
                                </span>

                                <span className="relative z-10 text-purple-500/80 font-mono text-[12px] font-bold tracking-tighter uppercase">
                                    {sponsor.amount}
                                </span>
                                <div className="
        absolute bottom-0 left-1/2 -translate-x-1/2 w-0 h-[1px]
        bg-gradient-to-r from-transparent via-purple-500/50 to-transparent
        group-hover:w-full transition-all duration-500
      " />
                            </div>
                        ))}
                    </div>
                </ModalContent>

                <ModalFooter>
                    <Button variant="outline" onClick={onClose} size="sm" className="w-full rounded-lg text-[12px] w-20 text-white/80">← Назад</Button>
                </ModalFooter>
            </ModalBlock>
        </div>
    );
};