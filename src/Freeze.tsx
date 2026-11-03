import React, { useState, useEffect, createContext, useContext } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Freeze } from 'react-freeze';

const FreezeContext = createContext({ isFrozen: false });
export const useFreeze = () => useContext(FreezeContext);

/**
 * Когда окно скрыто в трей или свёрнуто, интерфейс «замораживается»:
 * React не перерисовывает его, анимации не тратят CPU.
 */
export function FreezeProvider({ children }: { children: React.ReactNode }) {
    const [isFrozen, setIsFrozen] = useState(false);

    useEffect(() => {
        const win = getCurrentWindow();
        let timer: ReturnType<typeof setTimeout> | undefined;
        const unlisteners: Promise<() => void>[] = [
            win.listen<boolean>('window-visible', (e) => setIsFrozen(!e.payload)),
            win.onResized(() => {
                clearTimeout(timer);
                timer = setTimeout(async () => setIsFrozen(await win.isMinimized()), 200);
            }),
        ];
        return () => {
            clearTimeout(timer);
            unlisteners.forEach((u) => u.then((f) => f()));
        };
    }, []);

    return (
        <FreezeContext.Provider value={{ isFrozen }}>
            <Freeze freeze={isFrozen}>
                <div style={{ height: '100%' }}>{children}</div>
            </Freeze>
        </FreezeContext.Provider>
    );
}
