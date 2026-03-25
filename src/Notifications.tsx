import React, { createContext, useContext, useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, AlertTriangle, Info } from 'lucide-react';

type NotificationType = 'success' | 'error' | 'info';

interface Notification {
    id: number;
    message: string;
    type: NotificationType;
    exiting: boolean;
}

interface NotificationContextType {
    showNotify: (message: string, type?: NotificationType) => void;
}

let notifyStatic: (msg: string, type?: NotificationType) => void = () => { };

export const notify = (msg: string, type: NotificationType = 'info') => notifyStatic(msg, type);

const NotificationContext = createContext<NotificationContextType | undefined>(undefined);

export const NotificationProvider = ({ children }: { children: React.ReactNode }) => {
    const [notifications, setNotifications] = useState<Notification[]>([]);
    const enabledRef = useRef(true);

    const loadStatus = async () => {
        try {
            const settings = await invoke<any>('load_settings');
            if (settings && typeof settings.notifications === 'boolean') {
                enabledRef.current = settings.notifications;
            }
        } catch (e) { }
    };

    useEffect(() => {
        loadStatus();
        const handleUpdate = () => loadStatus();
        window.addEventListener('settings-changed', handleUpdate);
        return () => window.removeEventListener('settings-changed', handleUpdate);
    }, []);

    const showNotify = useCallback((message: string, type: NotificationType = 'info') => {
        if (!enabledRef.current) return;
        const id = Date.now();
        setNotifications(prev => [{ id, message, type, exiting: false }, ...prev]);
        setTimeout(() => {
            setNotifications(prev => prev.map(n => n.id === id ? { ...n, exiting: true } : n));
            setTimeout(() => setNotifications(prev => prev.filter(n => n.id !== id)), 400);
        }, 4000);
    }, []);

    notifyStatic = showNotify;

    (window as any).testNotify = showNotify;

    const getIcon = (type: NotificationType) => {
        switch (type) {
            case 'success':
                return <CheckCircle2 className="w-5 h-5 text-purple-400 animate-icon-pop" />;
            case 'error':
                return <AlertTriangle className="w-5 h-5 text-red-400 animate-icon-shake" />;
            case 'info':
            default:
                return <Info className="w-5 h-5 text-white animate-icon-pop" />;
        }
    };

    const getColors = (type: NotificationType) => {
        switch (type) {
            case 'success': return 'border-purple-500/50 text-purple-100';
            case 'error': return 'border-red-500/50 text-red-100';
            case 'info':
            default: return 'border-gray-500/50 text-white';
        }
    };

    return (
        <NotificationContext.Provider value={{ showNotify }}>
            {children}
            <div className="fixed top-4 left-1/2 -translate-x-1/2 z-[9999] flex flex-col items-center gap-2 w-full max-w-[90vw] pointer-events-none">
                {notifications.map(n => (
                    <div
                        key={n.id}
                        className={`pointer-events-auto relative flex items-center px-4 py-2.5 rounded-xl bg-[#0a0a12]/95 backdrop-blur-sm border shadow-lg shadow-black/40 min-w-[250px] max-w-sm ${getColors(n.type).split(' ')[0]} ${n.exiting ? 'animate-base-out' : 'animate-base-in'}`}
                    >
                        <div className={`flex items-center gap-3 w-full ${n.exiting ? '' : 'animate-content-in'}`}>
                            <div className="shrink-0 flex items-center justify-center">
                                {getIcon(n.type)}
                            </div>
                            <span className={`flex-1 min-w-0 truncate text-sm font-medium leading-tight ${getColors(n.type).split(' ')[1]}`}>
                                {n.message}
                            </span>
                        </div>
                    </div>
                ))}
            </div>
        </NotificationContext.Provider>
    );
};

export const useNotify = () => useContext(NotificationContext);