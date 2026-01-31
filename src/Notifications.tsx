import React, { createContext, useContext, useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { log } from "./Logic";

let notifyStatic: (msg: string, type?: 'success' | 'error' | 'info') => void = () => { };

export const notify = (msg: string, type: 'success' | 'error' | 'info' = 'info') => {
    notifyStatic(msg, type);
};

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
        } catch (e) {
            log("failed to load settings " + e);
        }
    };

    useEffect(() => {
        loadStatus();
        const handleUpdate = () => loadStatus();
        window.addEventListener('settings-updated', handleUpdate);
        return () => window.removeEventListener('settings-updated', handleUpdate);
    }, []);

    const showNotify = useCallback((message: string, type: NotificationType = 'info') => {
        if (!enabledRef.current) return;

        const id = Date.now();
        setNotifications(prev => [...prev, { id, message, type, exiting: false }]);

        setTimeout(() => {
            setNotifications(prev => prev.map(n => n.id === id ? { ...n, exiting: true } : n));
            setTimeout(() => {
                setNotifications(prev => prev.filter(n => n.id !== id));
            }, 400);
        }, 3000);
    }, []);

    notifyStatic = showNotify;

    return (
        <NotificationContext.Provider value={{ showNotify }}>
            {children}
            <div className="internal-notifications-container">
                {notifications.map(n => (
                    <div key={n.id} className={`toast ${n.type} ${n.exiting ? 'toast-exit' : ''}`}>
                        <span className="toast-icon">
                            {n.type === 'success' ? '✦' : n.type === 'error' ? '✕' : 'ℹ'}
                        </span>
                        <span className="toast-message">{n.message}</span>
                    </div>
                ))}
            </div>
        </NotificationContext.Provider>
    );
};

export const useNotify = () => useContext(NotificationContext);