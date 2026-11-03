'use client';

import { useState, useEffect } from 'react';
import { Header } from '@/Buttons';
import { api, log, type FileNode } from '@/lib/api';
import { notify } from '@/Notifications';
import {
    FileItem,
    FolderItem,
    FolderTrigger,
    FolderContent,
    Files,
    SubFiles,
} from '@/components/animate-ui/components/radix/files';

export const EditorPage = () => {
    const [loading, setLoading] = useState(true);
    const [tree, setTree] = useState<FileNode[]>([]);

    useEffect(() => { loadTree(); }, []);

    async function loadTree() {
        setLoading(true);
        try {
            setTree(await api.fileTree());
        } catch (e) {
            log("Ошибка чтения файлов: " + e);
        } finally {
            setLoading(false);
        }
    }

    async function openInEditor(path: string) {
        try {
            await api.openInEditor(path);
        } catch (e) {
            notify(`Не удалось открыть файл: ${e}`, "error");
        }
    }

    const renderTree = (nodes: FileNode[]) => {
        return nodes.map((node) => {
            if (node.is_dir) {
                return (
                    <FolderItem key={node.path} value={node.name}>
                        <FolderTrigger className="w-full flex items-center cursor-pointer transition-all duration-300 ease-in-out hover:bg-[#d8b4fe] hover:text-[#1a0b2e]">
                            <span className="truncate">{node.name}</span>
                        </FolderTrigger>
                        <FolderContent>
                            <SubFiles className="ml-4">
                                {renderTree(node.children || [])}
                            </SubFiles>
                        </FolderContent>
                    </FolderItem>
                );
            }
            return (
                <FileItem
                    key={node.path}
                    onClick={() => openInEditor(node.path)}
                    className="w-full flex items-center cursor-pointer transition-all duration-300 ease-in-out hover:bg-[#d8b4fe] hover:text-[#1a0b2e] overflow-visible"
                >
                    {node.name}
                </FileItem>
            );
        });
    };

    return (
        <div className="content">
            <Header title="Редактор" />
            <p className="mb-3 max-w-120 text-center text-[12px] text-white/35">
                Файлы откроются в редакторе по умолчанию. Свои домены добавляйте в <span className="text-purple-300/70">list-general-user.txt</span> —
                он не перезаписывается обновлениями. После правок перезапустите zapret.
            </p>
            <div className="mb-3 flex gap-2">
                <button onClick={() => api.openFolder('')} className="rounded-lg bg-white/5 px-3 py-1 text-[12px] text-white/60 transition-all hover:bg-white/10 hover:text-white">Открыть папку</button>
                <button onClick={loadTree} className="rounded-lg bg-white/5 px-3 py-1 text-[12px] text-white/60 transition-all hover:bg-white/10 hover:text-white">Обновить</button>
            </div>
            <div className="relative mx-auto rounded-2xl border border-white/10 bg-[#0a0514]/60 backdrop-blur-md overflow-hidden shadow-2xl w-full max-w-120">
                {loading ? (
                    <div className="p-20 flex flex-col items-center justify-center">
                        <div className="spinner mb-4"></div>
                        <span className="text-white/50 text-sm">Поиск файлов..</span>
                    </div>
                ) : (
                    <Files className="w-full p-4 overflow-y-auto max-h-125">
                        {renderTree(tree)}
                    </Files>
                )}
            </div>
        </div>
    );
};