import { useState, useEffect } from "react";

export const useModalAnimation = (isOpen: boolean, duration: number = 300) => {
    const [shouldRender, setShouldRender] = useState(isOpen);
    const [isAnimatingOut, setIsAnimatingOut] = useState(false);

    useEffect(() => {
        if (isOpen) {
            setShouldRender(true);
            setIsAnimatingOut(false);
        } else if (shouldRender) {
            setIsAnimatingOut(true);
            const timer = setTimeout(() => {
                setShouldRender(false);
                setIsAnimatingOut(false);
            }, duration);
            return () => clearTimeout(timer);
        }
    }, [isOpen, duration, shouldRender]);

    return { shouldRender, isAnimatingOut };
};