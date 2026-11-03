import React from "react";
import { cn } from "@/lib/utils";
import { ModalBlock } from "@/Buttons";
import { useModalAnimation } from "./useModalAnimation";

interface ModalProps {
    isOpen: boolean;
    onClose: () => void;
    children: React.ReactNode;
    /** классы для самого окна (например, "h-auto max-w-105") */
    className?: string;
    /** запретить закрытие кликом по фону (например, пока идёт операция) */
    locked?: boolean;
}

/** Затемнённый фон + окно + анимация появления/исчезновения. */
export const Modal = ({ isOpen, onClose, children, className, locked }: ModalProps) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    if (!shouldRender) return null;

    return (
        <div
            className={cn(
                "fixed inset-0 z-[1000] flex items-center justify-center bg-black/60 backdrop-blur-md transition-opacity duration-300",
                isAnimatingOut ? "opacity-0" : "opacity-100"
            )}
            onClick={() => !locked && onClose()}
        >
            <ModalBlock className={className} onClick={(e: React.MouseEvent) => e.stopPropagation()}>
                {children}
            </ModalBlock>
        </div>
    );
};

/** Кнопка «Закрыть/Отмена» в футере модалок. */
export const CloseButton = ({ onClick, children = "Закрыть" }: { onClick: () => void; children?: React.ReactNode }) => (
    <button
        className="rounded-lg bg-white/5 px-4 py-1 font-['Onest'] text-[13px] font-bold text-white/60 transition-all hover:bg-white/10 hover:text-white active:scale-95"
        onClick={onClick}
    >
        {children}
    </button>
);
