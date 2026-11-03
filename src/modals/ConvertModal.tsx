import { Modal, CloseButton } from "./Modal";
import { ModalHeader, ModalContent, ModalFooter } from "@/Buttons";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    onPick: () => void;
}
export const ConvertModal = ({ isOpen, onClose, onPick }: Props) => {

    return (
        <Modal isOpen={isOpen} onClose={onClose} className="h-auto max-w-105">
                <ModalHeader
                    title="Конвертатор"
                    icon="👀"
                    description="Преобразование батников в стратегии"
                />

                <ModalContent>
                    <div
                        onClick={onPick}
                        className="group relative h-44 w-full overflow-hidden rounded-[20px] border border-white/5 bg-black/30 transition-all duration-500 hover:border-purple-500/40 hover:bg-white/[0.04] active:scale-[0.98] cursor-pointer"
                    >
                        <div className="relative z-20 flex h-full flex-col items-center justify-center space-y-3 pointer-events-none">
                            <div className="flex h-12 w-12 items-center justify-center rounded-full bg-white/5 text-white/80 transition-all group-hover:bg-purple-500/20 group-hover:text-purple-300">
                                <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                    <path d="M12 17V3m0 0l-4 4m4-4l4 4m-9 10a4 4 0 00-4 4h18a4 4 0 00-4-4" strokeLinecap="round" strokeLinejoin="round" />
                                </svg>
                            </div>
                            <div className="text-center">
                                <span className="block text-[14px] font-black tracking-widest text-white uppercase drop-shadow-[0_0_10px_rgba(168,85,247,0.4)]">
                                    Выбрать
                                </span>
                                <span className="text-[10px] font-bold text-white/20 uppercase tracking-[0.15em]">
                                    Только .bat файлы
                                </span>
                            </div>
                        </div>
                    </div>
                    <p className="rounded-lg border border-white/5 bg-black/30 py-2 text-[13px] leading-tight text-white/40 font-['Onest'] text-center">
                        Конвертация <span className="text-purple-400/80 font-medium">.bat</span> в <span className="text-purple-400/80 font-medium">.zapret</span>.
                    </p>
                </ModalContent>

                <ModalFooter>
                    <CloseButton onClick={onClose}>Отмена</CloseButton>
                </ModalFooter>
            </Modal>
    );
};