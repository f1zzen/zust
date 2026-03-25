import { cn } from "@/lib/utils";
import { useModalAnimation } from "./useModalAnimation";
import { InfoHeader, ModalBlock, ModalContent, ModalFooter, ModalHeader } from "@/Buttons";
import { Button } from "@/components/ui/button";

export const CommunityModal = ({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) => {
    const COMMUNITY = [
        { name: "neton4ik", role: "Первый донат на бусти" },
        { name: "Анархист", role: "Первый обзор на Z.UST" },
        { name: "радостева", role: "Z.UST-тян" },
        { name: "ks", role: "Z.UST-тян" },
        { name: "Исчадие", role: "Z.UST-тян" },
        { name: "П.Ф", role: "Z.UST-тян" },
        { name: "T0ko", role: "Z.UST-тян" }
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
                    title="Наше Сообщество"
                    status="	(´ ∀ ` *)"
                    icon="🌐"
                    description="Люди, проявившие себя в сообществе!"
                />

                <ModalContent>
                    <div className="flex flex-col gap-4">
                        <InfoHeader
                            description="Люди в данном списке помогли и/или сделали что-то для Z.UST. В качестве благодарности все эти люди будут добавлены сюда!"
                            more="Следите за ТГК!"
                        />

                        <div className="grid grid-cols-3 gap-3 w-full">
                            {COMMUNITY.map((item, index) => (
                                <div
                                    key={index}
                                    className="p-3 rounded-lg bg-white/5 border border-white/5 flex justify-between items-center cursor-pointer hover:bg-white/10 hover:border-purple-500/30 transition-all active:scale-[0.98]"
                                >
                                    <div className="flex flex-col">
                                        <span className="text-white/80 font-medium text-[14px]">
                                            {item.name}
                                        </span>
                                        <span className="text-white/30 text-[11px]">
                                            {item.role}
                                        </span>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </ModalContent>

                <ModalFooter>
                    <Button
                        variant="outline"
                        onClick={onClose}
                        size="sm"
                        className="w-full rounded-lg text-[12px] text-white/80"
                    >
                        ← Назад
                    </Button>
                </ModalFooter>
            </ModalBlock>
        </div>
    );
};