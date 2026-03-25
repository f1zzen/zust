'use client';

import React, { useState, useEffect, useRef, createContext, useContext } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Freeze } from 'react-freeze';

const FreezeContext = createContext({ isFrozen: false });
export const useFreeze = () => useContext(FreezeContext);

export function FreezeProvider({ children }: { children: React.ReactNode }) {
    const [isFrozen, setIsFrozen] = useState(false);
    const debounceTimer = useRef<NodeJS.Timeout | null>(null);
    const appWindow = getCurrentWindow();

    useEffect(() => {
        let unlistenResized: (() => void) | null = null;
        let unlistenVisible: (() => void) | null = null;

        async function setup() {
            unlistenVisible = await appWindow.listen<boolean>('window-visible', (event) => {
                setIsFrozen(!event.payload);
            });

            unlistenResized = await appWindow.onResized(() => {
                if (debounceTimer.current) clearTimeout(debounceTimer.current);
                debounceTimer.current = setTimeout(async () => {
                    const minimized = await appWindow.isMinimized();
                    setIsFrozen(minimized);
                }, 200);
            });

            setTimeout(async () => {
                const checkMin = await appWindow.isMinimized();
                const checkVis = await appWindow.isVisible();
                if (!checkMin && checkVis && isFrozen) {
                    setIsFrozen(false);
                }
            }, 1000);
        }

        setup();

        return () => {
            if (debounceTimer.current) clearTimeout(debounceTimer.current);
            if (unlistenResized) unlistenResized();
            if (unlistenVisible) unlistenVisible();
        };
    }, [isFrozen]);

    return (
        <FreezeContext.Provider value={{ isFrozen }}>
            <Freeze freeze={isFrozen}>
                <div style={{ display: isFrozen ? 'none' : 'block', height: '100%' }}>
                    {children}
                </div>
            </Freeze>
        </FreezeContext.Provider>
    );
}